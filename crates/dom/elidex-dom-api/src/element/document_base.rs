//! Base URL maintenance for `<base>` elements (D-31 PR Phase B).
//!
//! Owns 3-layer ECS state per WHATWG HTML §2.4.3 (Document base
//! URLs) + §4.2.3 (The base element):
//!
//! - **Layer 1**: per-element `BaseFrozenUrl` component on each
//!   `<base>` (frozen URL invariant per HTML §4.2.3 "set the frozen
//!   base URL" algorithm).
//! - **Layer 2**: per-document `DocumentBaseUrl` derived cache +
//!   `DocumentFirstBase` positional index (HTML §2.4.3 first `<base>`
//!   rule).
//! - **Layer 3**: per-document `DocumentBaseUrlVersion` monotonic
//!   counter (HTML §2.4.3 "Respond to base URL changes" plug-in
//!   point for future reactive consumers; synchronous-drain
//!   semantics preclude late subscription).
//!
//! [`BaseUrlMaintainer`] is the [`MutationEvent`] consumer that
//! maintains all 3 layers, composed by
//! [`crate::ConsumerDispatcher`].
//!
//! Phase A scaffolding: only the `BaseUrlMaintainer` skeleton + the
//! `compute_frozen_url` algorithm are present; layer maintenance
//! lands in Phase B together with the component definitions in
//! `elidex_ecs::components`.

use elidex_ecs::{
    BaseFrozenUrl, DocumentBaseUrl, DocumentBaseUrlVersion, DocumentFirstBase, EcsDom, Entity,
    MutationEvent,
};
use url::Url;

// TODO swap fallback source to `dom.document_url(doc)` when
// `#11-document-url-real-navigation` slot lands.  The "about:blank"
// const here is placeholder until that slot provides a real
// `DocumentUrl` component reader.
const FALLBACK_BASE_URL: &str = "about:blank";

fn fallback_url() -> Url {
    Url::parse(FALLBACK_BASE_URL).expect("about:blank parses")
}

/// Compute the frozen base URL per HTML §4.2.3 "set the frozen base
/// URL" algorithm:
///
/// 1. Let urlRecord be the result of parsing `href` against
///    `fallback` (URL spec §4.4 Basic URL parser via
///    [`Url::join`]).
/// 2. (step 3 "if any of the following are true" three-part
///    disjunction): If urlRecord is failure OR urlRecord's scheme
///    is `data` / `javascript` OR `Is base allowed for Document?`
///    (CSP base-uri directive) returns "Blocked", set the frozen
///    base URL to `fallback` and return.
/// 3. Otherwise set the frozen base URL to urlRecord and return.
///
/// CSP `Is base allowed for Document?` is currently always-allow
/// stub; CSP wiring deferred to `#11-csp-base-uri` defer slot.
/// Scheme blocklist is implemented inline.
#[must_use]
pub fn compute_frozen_url(href: &str, fallback: &Url) -> Url {
    let parsed = fallback.join(href).ok();
    match parsed {
        Some(url) if matches!(url.scheme(), "data" | "javascript") => fallback.clone(),
        Some(url) => url,
        None => fallback.clone(),
    }
}

/// Reader for `document.baseURI`, `<a>.href` resolution, and
/// `node.baseURI`.  Returns the cached [`DocumentBaseUrl`] (O(1) hit)
/// — populated eagerly at [`EcsDom::create_document_root`] and
/// maintained by [`BaseUrlMaintainer`].
///
/// Returns the placeholder `about:blank` fallback if `doc` carries
/// no [`DocumentBaseUrl`] (shouldn't happen for properly-constructed
/// documents).
#[must_use]
pub fn document_base_url(dom: &EcsDom, doc: Entity) -> Url {
    dom.world()
        .get::<&DocumentBaseUrl>(doc)
        .ok()
        .map_or_else(fallback_url, |c| c.0.clone())
}

/// Walk the document subtree (light-tree, pre-order DFS) and return
/// the first `<base>` element with an `href` attribute (HTML §2.4.3
/// step 1 — "the first base element ... that has an href attribute,
/// in tree order").  Returns `None` if no qualifying `<base>` exists.
fn find_first_base_with_href(dom: &EcsDom, root: Entity) -> Option<Entity> {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if dom.is_base_element(node) && dom.has_attribute(node, "href") {
            return Some(node);
        }
        // Push children in reverse so we visit first-child first.
        let children: Vec<_> = dom.children_iter(node).collect();
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }
    None
}

/// Walk a subtree and attach [`BaseFrozenUrl`] to each `<base>`
/// element it contains, computing the frozen URL from the element's
/// current `href` attribute (or no-op if the element has no `href`).
fn attach_frozen_urls_in_subtree(dom: &mut EcsDom, root: Entity, fallback: &Url) {
    // Collect first (avoid borrowing dom mutably while iterating).
    let mut targets = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if dom.is_base_element(node) {
            if let Some(href) = dom.get_attribute(node, "href") {
                targets.push((node, href));
            }
        }
        let children: Vec<_> = dom.children_iter(node).collect();
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }
    for (node, href) in targets {
        let frozen = compute_frozen_url(&href, fallback);
        let _ = dom.world_mut().insert_one(node, BaseFrozenUrl(frozen));
    }
}

/// Recompute the document's first-base + DocumentBaseUrl +
/// DocumentBaseUrlVersion based on the current tree state.  Idempotent
/// no-op when first-base entity is unchanged AND the resolved URL is
/// unchanged (avoids spurious version bumps).
fn recompute_document_base(dom: &mut EcsDom, doc: Entity) {
    let fallback = fallback_url();
    let new_first = dom
        .document_root()
        .and_then(|root| find_first_base_with_href(dom, root));
    let new_url = match new_first {
        Some(base) => dom
            .world()
            .get::<&BaseFrozenUrl>(base)
            .ok()
            .map_or_else(|| fallback.clone(), |c| c.0.clone()),
        None => fallback.clone(),
    };
    let prev_first = dom.world().get::<&DocumentFirstBase>(doc).ok().map(|c| c.0);
    let prev_url = dom
        .world()
        .get::<&DocumentBaseUrl>(doc)
        .ok()
        .map(|c| c.0.clone());
    let url_changed = prev_url.as_ref() != Some(&new_url);
    let first_changed = prev_first.flatten() != new_first;
    if !url_changed && !first_changed {
        return;
    }
    let _ = dom
        .world_mut()
        .insert_one(doc, DocumentFirstBase(new_first));
    if url_changed {
        let _ = dom.world_mut().insert_one(doc, DocumentBaseUrl(new_url));
        // Layer 3 version bump on URL diff only.
        let prev_version = dom
            .world()
            .get::<&DocumentBaseUrlVersion>(doc)
            .map_or(0, |c| c.0);
        let _ = dom
            .world_mut()
            .insert_one(doc, DocumentBaseUrlVersion(prev_version.wrapping_add(1)));
    }
}

/// Find the doc entity that owns a given node (or fall back to the
/// EcsDom's document_root).  Used to scope the per-doc Layer 2+3
/// recompute after a mutation event.
fn owner_doc_or_root(dom: &EcsDom, _node: Entity) -> Option<Entity> {
    // D-31 Phase B uses the single document_root for the recompute
    // scope; multi-document support follows when AssociatedDocument
    // is wired into per-node owner resolution (separate concern from
    // this PR — `OwnerDocument` handler in `node_methods` already
    // does it, but routing through that mid-dispatch would require
    // additional &mut access plumbing).  Acceptable for D-31 scope
    // since the elidex test corpus uses a single document per
    // EcsDom.
    dom.document_root()
}

/// [`MutationEvent`] consumer for the D-31 3-layer base URL state.
///
/// Plain unit struct (no state) — all state lives in ECS components
/// on entities. Composed as a typed field of
/// [`crate::ConsumerDispatcher`].
pub struct BaseUrlMaintainer;

impl BaseUrlMaintainer {
    /// Single-method dispatch entry invoked by
    /// [`crate::ConsumerDispatcher`].  Maintains Layer 1+2+3 base URL
    /// state in response to Insert / Remove / AttributeChange events.
    pub fn handle(&mut self, event: &MutationEvent<'_>, dom: &mut EcsDom) {
        match *event {
            MutationEvent::Insert { node, .. } => {
                let fallback = fallback_url();
                attach_frozen_urls_in_subtree(dom, node, &fallback);
                if let Some(doc) = owner_doc_or_root(dom, node) {
                    recompute_document_base(dom, doc);
                }
            }
            MutationEvent::Remove { descendants, .. } => {
                // Layer 1 BaseFrozenUrl will be naturally absent on
                // re-insert (re-attached by Insert handler); for
                // detached-but-alive subtrees the component lingers
                // but is harmless since the entity is no longer in
                // the document tree.  Recompute Layer 2+3 to drop
                // the doc's first-base if it was inside the removed
                // subtree.
                let _ = descendants;
                if let Some(doc) = dom.document_root() {
                    recompute_document_base(dom, doc);
                }
            }
            MutationEvent::AttributeChange {
                node,
                name,
                new_value,
                ..
            } => {
                if !name.eq_ignore_ascii_case("href") || !dom.is_base_element(node) {
                    return;
                }
                let fallback = fallback_url();
                match new_value {
                    Some(href) => {
                        let frozen = compute_frozen_url(href, &fallback);
                        let _ = dom.world_mut().insert_one(node, BaseFrozenUrl(frozen));
                    }
                    None => {
                        let _ = dom.world_mut().remove_one::<BaseFrozenUrl>(node);
                    }
                }
                if let Some(doc) = owner_doc_or_root(dom, node) {
                    recompute_document_base(dom, doc);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fallback() -> Url {
        Url::parse(FALLBACK_BASE_URL).unwrap()
    }

    #[test]
    fn compute_frozen_url_returns_parsed_url_when_valid() {
        let url = compute_frozen_url("https://example.com/page", &fallback());
        assert_eq!(url.as_str(), "https://example.com/page");
    }

    #[test]
    fn compute_frozen_url_returns_fallback_on_data_scheme() {
        let url = compute_frozen_url("data:text/plain,hello", &fallback());
        assert_eq!(url.as_str(), FALLBACK_BASE_URL);
    }

    #[test]
    fn compute_frozen_url_returns_fallback_on_javascript_scheme() {
        let url = compute_frozen_url("javascript:alert(1)", &fallback());
        assert_eq!(url.as_str(), FALLBACK_BASE_URL);
    }

    #[test]
    fn compute_frozen_url_resolves_relative_against_fallback() {
        let base = Url::parse("https://example.com/page/").unwrap();
        let url = compute_frozen_url("sub/path", &base);
        assert_eq!(url.as_str(), "https://example.com/page/sub/path");
    }
}
