//! Base URL maintenance for `<base>` elements.
//!
//! Owns 2-layer ECS state per WHATWG HTML §2.4.3 (Document base
//! URLs) + §4.2.3 (The base element):
//!
//! - **Layer 1**: per-element [`BaseFrozenUrl`] component on each
//!   `<base>` (frozen URL invariant per HTML §4.2.3 "set the frozen
//!   base URL" algorithm).
//! - **Layer 2**: per-document [`DocumentBaseUrl`] derived cache
//!   (HTML §2.4.3 first `<base>` rule).
//!
//! [`BaseUrlMaintainer`] is the [`MutationEvent`] consumer that
//! maintains both layers, composed by `elidex_js::vm::consumer_dispatcher::ConsumerDispatcher`.

use std::ops::ControlFlow;

use elidex_ecs::{
    about_blank_url, BaseFrozenUrl, DocumentBaseUrl, EcsDom, Entity, MutationEvent, NodeKind,
};
use url::Url;

use crate::subtree_walk::walk_inclusive_filtered_until;

/// Compute the frozen base URL per WHATWG HTML §4.2.3 "set the
/// frozen base URL" algorithm:
///
/// 1. Let urlRecord be the result of parsing `href` against the
///    fallback base URL (URL spec §4.4 Basic URL parser via
///    [`Url::join`]).
/// 2. Step 3 — if any of three disjuncts holds, set the frozen base
///    URL to the fallback and return:
///    - 3.1: urlRecord is failure (parse error)
///    - 3.2: urlRecord's scheme is `data` or `javascript`
///    - 3.3: `Is base allowed for Document?` (CSP `base-uri`
///      directive) returns "Blocked" — currently always-allow stub,
///      CSP wiring deferred to `#11-csp-base-uri` defer slot
/// 3. Otherwise set the frozen base URL to urlRecord and return.
///
/// Disjuncts 3.1 and 3.2 are implemented inline; 3.3 is stubbed.
#[must_use]
pub fn compute_frozen_url(href: &str, fallback: &Url) -> Url {
    match fallback.join(href).ok() {
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
/// Returns [`about_blank_url`] if `doc` carries no [`DocumentBaseUrl`]
/// (caller misuse — `create_document_root` attaches the component
/// eagerly).
#[must_use]
pub fn document_base_url(dom: &EcsDom, doc: Entity) -> Url {
    dom.world()
        .get::<&DocumentBaseUrl>(doc)
        .ok()
        .map_or_else(about_blank_url, |c| c.0.clone())
}

/// Walk a subtree and attach [`BaseFrozenUrl`] to each `<base>`
/// element with an `href` attribute.  Returns `true` iff at least one
/// `<base>` element was attached — caller uses this to skip the
/// downstream `recompute_document_base` when the subtree contained
/// no `<base>` (the common case for typical pages).
///
/// `<template>` element children are skipped per HTML §2.4.3 —
/// template contents form a separate document and a `<base>` inside
/// must not affect the host document's base URL.
fn attach_frozen_urls_in_subtree(dom: &mut EcsDom, root: Entity, fallback: &Url) -> bool {
    let mut targets: Vec<(Entity, String)> = Vec::new();
    walk_inclusive_filtered_until(
        dom,
        root,
        |node| !dom.is_template_element(node),
        |node| {
            if dom.is_base_element(node) {
                if let Some(href) = dom.get_attribute(node, "href") {
                    targets.push((node, href));
                }
            }
            ControlFlow::<()>::Continue(())
        },
    );
    let any = !targets.is_empty();
    for (node, href) in targets {
        let frozen = compute_frozen_url(&href, fallback);
        let _ = dom.world_mut().insert_one(node, BaseFrozenUrl(frozen));
    }
    any
}

/// Recompute the [`DocumentBaseUrl`] for `doc` based on the current
/// tree state rooted at `doc` itself — locates the first `<base href>`
/// in tree order (WHATWG HTML §2.4.3 step 1 — "the first base element
/// ... that has an href attribute, in tree order") and adopts its
/// frozen URL, or falls back to `fallback`.  Idempotent no-op
/// when the resolved URL is unchanged.
///
/// `fallback` is the document's URL used when no `<base href>` is
/// present (HTML §2.4.3: "otherwise, the document base URL is the
/// document's URL"). The live-page [`BaseUrlMaintainer`] arms pass
/// [`about_blank_url`] (the page document's URL is tracked elsewhere);
/// the inert-`DOMParser` path passes the CALLER document's URL (HTML
/// §8.5.1 step 2 — the new Document's URL is the caller's), so a
/// `<base>`-less parsed document resolves relative URLs against the
/// calling page, not `about:blank`.
///
/// Walks the subtree rooted at `doc` (not the EcsDom's cached document
/// root), keeping the function self-consistent: callers pick the
/// document the recompute is for, the function uses that same tree
/// to derive `<base>` candidates and writes back to that document.
/// Today the only multi-document context is per-EcsDom (single
/// `document_root`), so in production `doc == dom.document_root()`,
/// but multi-document / DocumentFragment owner-document support would
/// hit a write/walk mismatch under the old `dom.document_root()`-
/// only formulation.
///
/// `<template>` element children are skipped per HTML §2.4.3 —
/// template contents form a separate document and a `<base>` inside
/// must not be selected as the host document's first base.
fn recompute_document_base(dom: &mut EcsDom, doc: Entity, fallback: &Url) {
    let new_first = walk_inclusive_filtered_until(
        dom,
        doc,
        |node| !dom.is_template_element(node),
        |node| {
            if dom.is_base_element(node) && dom.has_attribute(node, "href") {
                ControlFlow::Break(node)
            } else {
                ControlFlow::Continue(())
            }
        },
    );
    let new_url = match new_first {
        Some(base) => dom
            .world()
            .get::<&BaseFrozenUrl>(base)
            .ok()
            .map_or_else(|| fallback.clone(), |c| c.0.clone()),
        None => fallback.clone(),
    };
    let unchanged = dom
        .world()
        .get::<&DocumentBaseUrl>(doc)
        .ok()
        .is_some_and(|c| c.0 == new_url);
    if unchanged {
        return;
    }
    let _ = dom.world_mut().insert_one(doc, DocumentBaseUrl(new_url));
}

/// The document `node` belongs to, for routing a base-URL recompute.
///
/// Returns `node` itself when it IS a `Document` — the Remove arm
/// anchors on `parent`, which can be the `DOMParser` document when its
/// `documentElement`-level subtree is removed, and
/// [`EcsDom::owner_document`] returns `None` for a `Document` receiver.
/// Otherwise its `owner_document` (which, for a node connected to a
/// document's light tree, resolves to that document — the page OR a
/// `DOMParser` document).
///
/// Returns `None` for a node attached to no document, and the caller
/// then skips maintenance: a `document_root()` fallback here would
/// mis-route a detached / second-document mutation onto the PAGE
/// document and clobber its base URL (Codex R5). In practice the
/// [`in_document_light_tree`] guard already filters detached / shadow
/// anchors before this is reached, so a `None` is defensive.
fn owner_doc(dom: &EcsDom, node: Entity) -> Option<Entity> {
    if matches!(dom.node_kind_inferred(node), Some(NodeKind::Document)) {
        return Some(node);
    }
    dom.owner_document(node)
}

/// Initialize the 2-layer base URL state for an ARBITRARY document
/// entity `doc` from its current subtree — the document-scoped
/// primitive shared by the bind path ([`BaseUrlMaintainer::
/// initialize_from_tree`], which calls this with `dom.document_root()`)
/// and any out-of-band tree population that bypasses the live
/// [`MutationEvent`] dispatcher (e.g. an inert `DOMParser` document
/// built with the dispatcher suppressed — HTML §8.5.1).
///
/// Attaches [`BaseFrozenUrl`] to every `<base href>` reachable from
/// `doc` (resolving each relative `href` against `fallback`) and derives
/// `doc`'s [`DocumentBaseUrl`] from the first such element in tree order
/// (HTML §2.4.3), or from `fallback` itself when the document has no
/// `<base href>`. `fallback` is the document's own URL: the bind path
/// passes [`about_blank_url`]; the inert-`DOMParser` path passes the
/// CALLER document's URL (HTML §8.5.1 step 2 — the new Document's URL is
/// the caller's). Sharing one finalizer keeps the eager-init path and
/// the live-mutation path in agreement (one-issue-one-way: a single
/// base-url finalizer, not a per-call-site reimplementation).
/// Idempotent — re-running on an already-initialized subtree is a no-op
/// (each `<base href>` already carries [`BaseFrozenUrl`];
/// `recompute_document_base`'s `unchanged` short-circuit absorbs the
/// [`DocumentBaseUrl`] re-write).
pub fn initialize_base_url_for_document(dom: &mut EcsDom, doc: Entity, fallback: &Url) {
    let _ = attach_frozen_urls_in_subtree(dom, doc, fallback);
    recompute_document_base(dom, doc, fallback);
}

/// Returns `true` iff `entity`'s tree root is a **`Document`** — i.e.
/// `entity` lives in some document's light tree (the bound page
/// document OR a `DOMParser`-created document), not inside a shadow
/// tree and not in a detached fragment.  Used by the
/// [`BaseUrlMaintainer`] arms to short-circuit mutations that cannot
/// legitimately change any document's base URL:
///
///   * **shadow-tree-internal** mutations — per WHATWG HTML §2.4.3 a
///     `<base>` inside a shadow tree must not affect the host
///     document's base URL; `find_tree_root` (non-composed) returns the
///     `ShadowRoot`, not a `Document`, so this filters them out.
///   * **detached-fragment** mutations (a `<base>` inside a
///     `createDocumentFragment()` / `<template>` content / unattached
///     subtree) — there is no document whose base URL to maintain, and
///     letting them through would route via `owner_doc`'s
///     `document_root()` fallback and wrongly clobber the PAGE base.
///
/// Keyed on "is the tree root a `Document`" rather than "is the tree
/// root THE page `document_root`" so a `DOMParser` document — a second
/// live light-tree `Document` in the same `EcsDom` — keeps its OWN base
/// URL maintained as `<base>` elements are inserted / mutated / removed
/// after `parseFromString`; the arms then route by the mutation's
/// `owner_doc`, so each document maintains its own base (Codex R5).
///
/// `EcsDom`'s fire-site filter only suppresses events where the
/// `node` or `parent` IS a `ShadowRoot`; deeper shadow-tree
/// mutations (`<base>` 2+ levels into a shadow tree) still dispatch
/// here.  Without this guard the maintainer would (a) walk the
/// shadow subtree in `attach_frozen_urls_in_subtree`, (b) attach
/// `BaseFrozenUrl` to shadow-internal `<base>` elements (which then
/// silently leak resolved URLs through `<base>.href` getter for
/// those receivers), and (c) burn cycles in a `recompute` that
/// `children_iter` would harmlessly skip — all wasted work for a
/// codepath that cannot legitimately change a document base URL.
fn in_document_light_tree(dom: &EcsDom, entity: Entity) -> bool {
    matches!(
        dom.node_kind_inferred(dom.find_tree_root(entity)),
        Some(NodeKind::Document)
    )
}

/// [`MutationEvent`] consumer maintaining the 2-layer base URL state.
///
/// Plain unit struct (no state) — all state lives in the
/// [`BaseFrozenUrl`] and [`DocumentBaseUrl`] ECS components on
/// entities. Composed as a typed field of
/// `elidex_js::vm::consumer_dispatcher::ConsumerDispatcher`.
pub struct BaseUrlMaintainer;

impl BaseUrlMaintainer {
    /// Initialize the 2-layer base URL state from the current DOM
    /// tree.  Required when the tree was populated BEFORE the
    /// dispatcher was installed (e.g. parser-created `<base href>`
    /// prior to first `Vm::bind`): pre-bind nodes never went through
    /// [`MutationEvent::Insert`] dispatch, so [`BaseFrozenUrl`] was
    /// never attached and [`DocumentBaseUrl`] remained at its
    /// [`about_blank_url`] default.  Without this initialization
    /// `document.baseURI` / `Node.baseURI` / relative URL resolution
    /// stay stuck on the fallback until a subsequent mutation fires,
    /// and removing a pre-bind `<base>` would not trigger recompute
    /// (no [`BaseFrozenUrl`] to detach).
    ///
    /// Idempotent: re-running on an already-initialized tree is a
    /// no-op (each `<base href>` already has [`BaseFrozenUrl`];
    /// recompute's `unchanged` short-circuit absorbs the
    /// [`DocumentBaseUrl`] re-write).  Invoked by
    /// `ConsumerDispatcher::initialize_consumers` from the
    /// bind path.
    pub fn initialize_from_tree(&mut self, dom: &mut EcsDom) {
        let Some(root) = dom.document_root() else {
            return;
        };
        // Delegate to the document-scoped primitive so the bind path and
        // the out-of-band population path (inert `DOMParser` document)
        // share ONE base-url finalizer (one-issue-one-way). The bind path's
        // fallback stays `about_blank_url()` — unchanged from before the
        // fallback parameter was threaded for the §8.5.1 DOMParser path.
        initialize_base_url_for_document(dom, root, &about_blank_url());
    }

    /// Single-method dispatch entry invoked by
    /// `elidex_js::vm::consumer_dispatcher::ConsumerDispatcher`.  Maintains both layers in
    /// response to Insert / Remove / AttributeChange events.
    pub fn handle(&mut self, event: &MutationEvent<'_>, dom: &mut EcsDom) {
        match *event {
            MutationEvent::Insert { node, .. } => {
                // Shadow-tree carve-out: skip work for mutations
                // landing inside a shadow tree — see [`in_document_light_tree`].
                if !in_document_light_tree(dom, node) {
                    return;
                }
                let fallback = about_blank_url();
                let attached = attach_frozen_urls_in_subtree(dom, node, &fallback);
                // Short-circuit: inserting a subtree with no
                // qualifying `<base href>` cannot change which
                // `<base href>` is first-in-tree-order (HTML §2.4.3
                // step 1 — "first base element ... that has an href
                // attribute, in tree order").  Inserting non-`<base>`
                // nodes adds nothing to the candidate set, so the
                // current first-base remains first regardless of
                // insertion position.  Holds even when the document
                // already has other `<base href>` elements.
                if !attached {
                    return;
                }
                if let Some(doc) = owner_doc(dom, node) {
                    recompute_document_base(dom, doc, &fallback);
                }
            }
            MutationEvent::Remove {
                parent,
                descendants,
                ..
            } => {
                // Shadow-tree carve-out: skip when the removal
                // happened inside a shadow tree.  `parent` is the
                // former parent (still alive); its tree root tells
                // us where the mutation was.  See
                // [`in_document_light_tree`].
                if !in_document_light_tree(dom, parent) {
                    return;
                }
                // ECS hygiene + precise recompute trigger: track
                // whether any removed `<base>` actually carried
                // [`BaseFrozenUrl`] (the marker for "had a valid
                // href" — set by `attach_frozen_urls_in_subtree`).
                // Detach the component from any such removed entity
                // so it doesn't linger on the orphan.
                let mut removed_a_qualifying_base = false;
                for &n in descendants {
                    if dom.is_base_element(n)
                        && dom.world_mut().remove_one::<BaseFrozenUrl>(n).is_ok()
                    {
                        removed_a_qualifying_base = true;
                    }
                }
                // Short-circuit: removing nodes that include no
                // qualifying `<base href>` cannot change the first-
                // in-tree-order selection (symmetric to the Insert
                // arm's reasoning).  `<base>` elements without href,
                // and any other removed nodes, are filtered out by
                // the `BaseFrozenUrl` presence check above.
                if !removed_a_qualifying_base {
                    return;
                }
                // Route through `owner_doc(parent)` for symmetry with
                // the Insert + AttributeChange arms (both use
                // `owner_doc(node)`).  `parent` is the appropriate
                // anchor here since `node` has already been detached
                // — `owner_document(parent)` resolves the same
                // document the removed subtree previously belonged
                // to.  Today `owner_doc` and `dom.document_root()`
                // return the same value (single-`document_root`
                // per-EcsDom), but keeping the helper consistent
                // avoids forward-compat drift if multi-document
                // support lands and `in_document_light_tree` / root
                // selection semantics shift.
                if let Some(doc) = owner_doc(dom, parent) {
                    recompute_document_base(dom, doc, &about_blank_url());
                }
            }
            MutationEvent::AttributeChange { node, .. } => {
                // Shadow-tree carve-out FIRST (cheapest check, and
                // applies regardless of element kind): a `<base>`
                // inside a shadow tree must not affect the document
                // base URL.  See [`in_document_light_tree`].
                if !in_document_light_tree(dom, node) {
                    return;
                }
                // ECS-state-driven filter: any attribute change on a
                // <base> element may have changed its href (or any
                // other attr — recompute is cheap and idempotent).
                // No name-string check needed — `is_base_element`
                // predicate IS the structural identifier.
                if !dom.is_base_element(node) {
                    return;
                }
                let fallback = about_blank_url();
                match dom.get_attribute(node, "href") {
                    Some(href) => {
                        let frozen = compute_frozen_url(&href, &fallback);
                        let _ = dom.world_mut().insert_one(node, BaseFrozenUrl(frozen));
                    }
                    None => {
                        let _ = dom.world_mut().remove_one::<BaseFrozenUrl>(node);
                    }
                }
                if let Some(doc) = owner_doc(dom, node) {
                    recompute_document_base(dom, doc, &fallback);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_frozen_url_returns_parsed_url_when_valid() {
        let url = compute_frozen_url("https://example.com/page", &about_blank_url());
        assert_eq!(url.as_str(), "https://example.com/page");
    }

    #[test]
    fn compute_frozen_url_returns_fallback_on_data_scheme() {
        let url = compute_frozen_url("data:text/plain,hello", &about_blank_url());
        assert_eq!(url, about_blank_url());
    }

    #[test]
    fn compute_frozen_url_returns_fallback_on_javascript_scheme() {
        let url = compute_frozen_url("javascript:alert(1)", &about_blank_url());
        assert_eq!(url, about_blank_url());
    }

    #[test]
    fn compute_frozen_url_resolves_relative_against_fallback() {
        let base = Url::parse("https://example.com/page/").unwrap();
        let url = compute_frozen_url("sub/path", &base);
        assert_eq!(url.as_str(), "https://example.com/page/sub/path");
    }

    #[test]
    fn recompute_walks_from_doc_param_not_cached_root() {
        // Regression for R3: `recompute_document_base(dom, doc)` must
        // walk from `doc` itself, NOT from `dom.document_root()`.
        // Old formulation would pick up a `<base href>` reachable from
        // the cached root and write its URL onto `doc` even when `doc`
        // is a different / detached subtree — write/walk mismatch.
        use elidex_ecs::{Attributes, EcsDom};
        let mut dom = EcsDom::new();
        let doc_root = dom.create_document_root();
        let body = dom.create_element("body", Attributes::default());
        assert!(dom.append_child(doc_root, body));

        // Attach a `<base href>` reachable from the cached root.  The
        // `BaseFrozenUrl` is hand-attached because no dispatcher is
        // installed in this raw-ECS test fixture.
        let base = dom.create_element("base", Attributes::default());
        assert!(
            dom.set_attribute(base, "href", "https://outer.example/")
                .did_set
        );
        assert!(dom.append_child(body, base));
        let outer = Url::parse("https://outer.example/").unwrap();
        dom.world_mut()
            .insert_one(base, BaseFrozenUrl(outer.clone()))
            .expect("attach BaseFrozenUrl on test base element");

        // A separate, detached "would-be document" entity with NO base
        // in its (empty) subtree.
        let alt_doc = dom.create_element("html", Attributes::default());

        // Walk must originate at `alt_doc` — empty subtree → fallback.
        recompute_document_base(&mut dom, alt_doc, &about_blank_url());
        let recorded = dom
            .world()
            .get::<&DocumentBaseUrl>(alt_doc)
            .expect("recompute writes DocumentBaseUrl to its `doc` param")
            .0
            .clone();
        assert_eq!(
            recorded,
            about_blank_url(),
            "recompute_document_base must walk from its `doc` param \
             (alt_doc here), not from dom.document_root() — otherwise \
             outer.example would leak in via the cached-root walk",
        );

        // Sanity: recomputing for `doc_root` (which IS the cached root)
        // still picks up `<base href>` correctly.
        recompute_document_base(&mut dom, doc_root, &about_blank_url());
        let doc_root_url = dom
            .world()
            .get::<&DocumentBaseUrl>(doc_root)
            .expect("recompute writes DocumentBaseUrl to doc_root")
            .0
            .clone();
        assert_eq!(doc_root_url, outer);
    }

    #[test]
    fn remove_arm_routes_to_owning_document_not_page_root() {
        // Codex R5 follow-on (adversarial completeness check): the Remove arm
        // anchors on `parent`, which IS a `Document` when a second live
        // document's (e.g. a `DOMParser` document) `documentElement`-level
        // subtree is removed. `EcsDom::owner_document` returns `None` for a
        // `Document` receiver, so the old `owner_doc` `.or_else(document_root)`
        // fallback mis-routed the recompute to the PAGE root — leaving the
        // second document's `DocumentBaseUrl` stale. `owner_doc` now resolves a
        // `Document` anchor to itself, so the recompute lands on the right
        // document and the page base is untouched.
        use elidex_ecs::{Attributes, EcsDom};
        let mut dom = EcsDom::new();
        let page = dom.create_document_root();

        // A second live Document with a `<base href>` (mirrors a DOMParser doc).
        let doc = dom.create_document_node();
        let html = dom.create_element("html", Attributes::default());
        assert!(dom.append_child(doc, html));
        let base = dom.create_element("base", Attributes::default());
        assert!(
            dom.set_attribute(base, "href", "https://throwaway.example/x/")
                .did_set
        );
        assert!(dom.append_child(html, base));
        initialize_base_url_for_document(&mut dom, doc, &about_blank_url());
        initialize_base_url_for_document(&mut dom, page, &about_blank_url());
        let page_base_before = document_base_url(&dom, page);
        assert_eq!(
            document_base_url(&dom, doc).as_str(),
            "https://throwaway.example/x/"
        );

        // Remove the `documentElement` subtree: the former `parent` is `doc`
        // itself (a `Document`). Detach first (fire-before-despawn), then fire.
        let descendants = [html, base];
        assert!(dom.remove_child(doc, html));
        BaseUrlMaintainer.handle(
            &MutationEvent::Remove {
                node: html,
                parent: doc,
                removed_index: 0,
                descendants: &descendants,
                was_connected: true,
            },
            &mut dom,
        );

        // `doc` recomputed to the fallback (its only `<base>` is gone) — i.e.
        // the recompute routed to `doc`, NOT to the page root (which would
        // strand `doc` at the stale throwaway URL).
        assert_eq!(
            document_base_url(&dom, doc),
            about_blank_url(),
            "Remove arm must route the recompute to the owning Document, not document_root",
        );
        // The page base must be untouched by a second document's removal.
        assert_eq!(
            document_base_url(&dom, page),
            page_base_before,
            "removing a second document's subtree must not clobber the page base",
        );
    }
}
