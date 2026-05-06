//! Live DOM collections (`HTMLCollection`, `NodeList`).
//!
//! A `LiveCollection` lazily evaluates against the current DOM tree state,
//! caching results until the subtree version changes. This mirrors the
//! WHATWG DOM specification's live collection semantics.

use elidex_ecs::{Attributes, EcsDom, Entity, ShadowRoot, TagType};

/// Sentinel version that never matches any real subtree version, ensuring the
/// first access always triggers a refresh. Equivalent to WHATWG's "created with
/// an empty snapshot" semantics.
const UNINITIALIZED_VERSION: u64 = u64::MAX;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Filter criterion for a live collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionFilter {
    /// Match elements by tag name (lowercased at creation for HTML documents).
    /// `"*"` matches all elements.
    ByTagName(String),
    /// Match elements that have *all* of the given class names.
    /// An empty vec always produces an empty result (WHATWG spec).
    ByClassNames(Vec<String>),
    /// Match elements whose `name` attribute equals the given value.
    ByName(String),
    /// Match `<img>` elements.
    Images,
    /// Match `<form>` elements.
    Forms,
    /// Match `<a>` and `<area>` elements that have an `href` attribute.
    Links,
    /// All direct child nodes (including text nodes) — `NodeList` semantics.
    ChildNodes,
    /// All direct child *elements* (excluding text nodes) — `HTMLCollection` semantics.
    ElementChildren,
    /// Static, pre-captured entity list (`querySelectorAll` result).
    /// Per WHATWG DOM §4.2.6, the single non-live `NodeList` case.
    /// The entities themselves live in the collection's internal
    /// snapshot buffer directly — populated at construction by
    /// [`LiveCollection::new_snapshot`] and never refreshed, so no
    /// second buffer holds them.
    Snapshot,
}

/// Whether the collection behaves as an `HTMLCollection` or a `NodeList`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionKind {
    HtmlCollection,
    NodeList,
}

/// A lazily-evaluated, cached live collection over a DOM subtree.
///
/// The snapshot is invalidated when the subtree rooted at `root` is mutated
/// (detected via `EcsDom::inclusive_descendants_version`).
///
/// `root` is `None` for `Snapshot` collections — their entity list is
/// frozen at construction so there is no subtree version to track.
#[derive(Debug)]
pub struct LiveCollection {
    root: Option<Entity>,
    filter: CollectionFilter,
    kind: CollectionKind,
    cached_version: u64,
    cached_snapshot: Vec<Entity>,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl LiveCollection {
    /// Create a new live collection rooted at `root`.
    ///
    /// `ByTagName` filters are lowercased at creation (WHATWG HTML spec: tag
    /// names are ASCII-lowercased for HTML documents).
    ///
    /// Use [`Self::new_snapshot`] for `Snapshot` collections — both paths
    /// yield identical observable behaviour (`refresh_if_stale` short-
    /// circuits on the filter variant), but `new_snapshot` is the
    /// conventional constructor and stores `root: None`, making the
    /// "no subtree to track" intent explicit.
    #[must_use]
    pub fn new(root: Entity, filter: CollectionFilter, kind: CollectionKind) -> Self {
        let filter = match filter {
            CollectionFilter::ByTagName(tag) => {
                CollectionFilter::ByTagName(tag.to_ascii_lowercase())
            }
            other => other,
        };
        Self {
            root: Some(root),
            filter,
            kind,
            cached_version: UNINITIALIZED_VERSION,
            cached_snapshot: Vec::new(),
        }
    }

    /// Create a static (`Snapshot`) collection from a pre-captured entity list.
    ///
    /// Used for `querySelectorAll` results — the entity list is frozen at
    /// construction and never re-walks the DOM. `querySelectorAll` callers
    /// should pass `CollectionKind::NodeList` per WHATWG DOM §4.2.6 ("a
    /// non-live `NodeList`"). The parameter is explicit (not defaulted)
    /// to keep future `HTMLCollection`-shaped snapshot users
    /// (e.g. potential XPath bindings) able to opt in.
    ///
    /// The captured entities are moved into `cached_snapshot` at
    /// construction; subsequent `length` / `item` / `snapshot` calls
    /// read directly from that buffer without a per-access refresh
    /// or a duplicated filter-side copy.
    #[must_use]
    pub fn new_snapshot(entities: Vec<Entity>, kind: CollectionKind) -> Self {
        Self {
            root: None,
            filter: CollectionFilter::Snapshot,
            kind,
            // Any non-`UNINITIALIZED_VERSION` value disables the
            // version-check refresh path. Snapshot collections
            // never refresh, so the field is observably unused; `0`
            // is the conventional "fresh" marker.
            cached_version: 0,
            cached_snapshot: entities,
        }
    }

    /// Number of entities in the collection.
    pub fn length(&mut self, dom: &EcsDom) -> usize {
        self.refresh_if_stale(dom);
        self.cached_snapshot.len()
    }

    /// Return the entity at `index`, or `None` if out of bounds.
    pub fn item(&mut self, index: usize, dom: &EcsDom) -> Option<Entity> {
        self.refresh_if_stale(dom);
        self.cached_snapshot.get(index).copied()
    }

    /// Return the full snapshot slice.
    pub fn snapshot(&mut self, dom: &EcsDom) -> &[Entity] {
        self.refresh_if_stale(dom);
        &self.cached_snapshot
    }

    /// The filter this collection was created with.
    #[must_use]
    pub fn filter(&self) -> &CollectionFilter {
        &self.filter
    }

    /// The kind of this collection.
    #[must_use]
    pub fn kind(&self) -> CollectionKind {
        self.kind
    }

    // -- private -------------------------------------------------------------

    fn refresh_if_stale(&mut self, dom: &EcsDom) {
        // Snapshot collections are frozen — `cached_snapshot` was
        // populated by `new_snapshot` and never re-walks.
        if matches!(self.filter, CollectionFilter::Snapshot) {
            return;
        }
        let Some(root) = self.root else {
            return;
        };
        let current_version = dom.inclusive_descendants_version(root);
        if current_version != self.cached_version {
            self.refresh(dom);
            self.cached_version = current_version;
        }
    }

    /// Refresh the cached snapshot in place, reusing both the
    /// `Vec`'s capacity and the underlying allocation.
    ///
    /// `populate_into` writes directly into `self.cached_snapshot`
    /// after `clear()`, avoiding any intermediate `Vec<Entity>`.
    /// Once the result set stabilises at its high-water mark,
    /// subsequent miss-path refreshes are allocation-free.
    fn refresh(&mut self, dom: &EcsDom) {
        let LiveCollection {
            filter,
            root,
            cached_snapshot,
            ..
        } = self;
        cached_snapshot.clear();
        Self::populate_into(filter, *root, dom, cached_snapshot);
    }

    fn populate_into(
        filter: &CollectionFilter,
        root: Option<Entity>,
        dom: &EcsDom,
        out: &mut Vec<Entity>,
    ) {
        // Snapshot is never refreshed (see `refresh_if_stale`), so
        // `populate_into` never receives one. Every other variant
        // needs `root` to walk from.
        let Some(root) = root else {
            return;
        };
        match filter {
            CollectionFilter::Snapshot => {
                unreachable!("Snapshot is populated at construction; never re-walks")
            }
            CollectionFilter::ChildNodes => collect_direct_children(dom, root, out, true),
            CollectionFilter::ElementChildren => collect_direct_children(dom, root, out, false),
            // ByClassNames with empty vec always returns empty.
            CollectionFilter::ByClassNames(names) if names.is_empty() => {}
            // All other filters: pre-order traversal of the subtree.
            // Shadow boundaries are respected because the child
            // iterators used by `traverse_descendants` skip
            // ShadowRoot entities, so shadow subtrees are unreachable.
            f => {
                dom.traverse_descendants(root, |entity| {
                    if matches_filter(entity, f, dom) {
                        out.push(entity);
                    }
                    true
                });
            }
        }
    }
}

/// Collect direct children of `parent`. If `include_text` is true, text nodes
/// are included; otherwise only element nodes are returned.
/// Shadow root entities are excluded (`EcsDom::children_iter` already excludes them).
fn collect_direct_children(
    dom: &EcsDom,
    parent: Entity,
    out: &mut Vec<Entity>,
    include_text: bool,
) {
    for child in dom.children_iter(parent) {
        if dom.world().get::<&ShadowRoot>(child).is_ok() {
            continue;
        }
        if include_text || dom.is_element(child) {
            out.push(child);
        }
    }
}

// ---------------------------------------------------------------------------
// Filter matching
// ---------------------------------------------------------------------------

/// Check whether `entity` matches the given `filter`.
fn matches_filter(entity: Entity, filter: &CollectionFilter, dom: &EcsDom) -> bool {
    match filter {
        CollectionFilter::ByTagName(tag) => {
            if tag == "*" {
                return dom.is_element(entity);
            }
            // ASCII case-insensitive comparison matches WHATWG DOM
            // §4.2.6.2 ("the qualified name is matched ASCII case-
            // insensitively for HTML documents") and the pre-hoist VM
            // walker's behaviour. The constructor still lowercases
            // `tag`, so only the element side needs the CI compare.
            match dom.world().get::<&TagType>(entity) {
                Ok(tt) => tt.0.eq_ignore_ascii_case(tag),
                Err(_) => false,
            }
        }
        CollectionFilter::ByClassNames(names) => {
            // WHATWG §4.2.6.2 "descendant elements" — non-Element entities
            // that happen to carry `Attributes` (parser fixtures can attach
            // a stray `class` to Text/Comment via direct
            // `EcsDom::set_attribute`) must not surface here.
            if names.is_empty() || !dom.is_element(entity) {
                return false;
            }
            match dom.world().get::<&Attributes>(entity) {
                Ok(attrs) => {
                    let Some(class_str) = attrs.get("class") else {
                        return false;
                    };
                    // Iterator-based containment check — re-splits
                    // `class_str` per needle, but avoids the per-entity
                    // `Vec<&str>` allocation a `.collect()` would pay
                    // on every visited descendant.
                    names.iter().all(|name| {
                        class_str
                            .split_ascii_whitespace()
                            .any(|tok| tok == name.as_str())
                    })
                }
                Err(_) => false,
            }
        }
        CollectionFilter::ByName(name) => {
            // WHATWG HTML §3.1.5 — `getElementsByName` is a list-of-elements
            // query, mirroring the ByClassNames Element-only guard.
            if !dom.is_element(entity) {
                return false;
            }
            match dom.world().get::<&Attributes>(entity) {
                Ok(attrs) => attrs.get("name") == Some(name.as_str()),
                Err(_) => false,
            }
        }
        CollectionFilter::Images => matches_tag_ascii_ci(entity, "img", dom),
        CollectionFilter::Forms => matches_tag_ascii_ci(entity, "form", dom),
        CollectionFilter::Links => {
            let is_link_tag =
                matches_tag_ascii_ci(entity, "a", dom) || matches_tag_ascii_ci(entity, "area", dom);
            if !is_link_tag {
                return false;
            }
            match dom.world().get::<&Attributes>(entity) {
                Ok(attrs) => attrs.get("href").is_some(),
                Err(_) => false,
            }
        }
        // ChildNodes / ElementChildren / Snapshot are handled in populate() directly.
        CollectionFilter::ChildNodes
        | CollectionFilter::ElementChildren
        | CollectionFilter::Snapshot => false,
    }
}

/// Case-insensitive tag-name match (ASCII). Mirrors the WHATWG HTML
/// canonicalisation that `Document.{forms,images,links}` perform — the
/// HTML parser already lowercases tags, but `EcsDom::create_element` is
/// also reachable from non-parser paths (e.g. JS `document.createElementNS`),
/// so the matcher tolerates uppercase input rather than relying on the
/// parser's lowercase guarantee.
fn matches_tag_ascii_ci(entity: Entity, tag: &str, dom: &EcsDom) -> bool {
    match dom.world().get::<&TagType>(entity) {
        Ok(tt) => tt.0.eq_ignore_ascii_case(tag),
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
mod tests {
    use super::*;

    fn setup_dom() -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let html = dom.create_element("html", Attributes::default());
        let body = dom.create_element("body", Attributes::default());
        dom.append_child(doc, html);
        dom.append_child(html, body);
        (dom, doc)
    }

    #[test]
    fn by_tag_name_match() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByTagName("div".into()),
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.length(&dom), 1);
        assert_eq!(coll.item(0, &dom), Some(div));
    }

    #[test]
    fn by_tag_name_wildcard() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let div = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        dom.append_child(body, div);
        dom.append_child(body, span);

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByTagName("*".into()),
            CollectionKind::HtmlCollection,
        );
        // html, body, div, span
        assert_eq!(coll.length(&dom), 4);
    }

    #[test]
    fn by_tag_name_parser_lowercase() {
        // In real usage, the HTML parser lowercases tags. This test verifies
        // that lowercase element tags match lowercase filters (exact match).
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByTagName("div".into()),
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.length(&dom), 1);
    }

    #[test]
    fn by_tag_name_filter_lowercased_at_creation() {
        // Even if caller passes uppercase filter, it is lowercased at creation.
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByTagName("DIV".into()),
            CollectionKind::HtmlCollection,
        );
        // Filter "DIV" lowercased to "div", element tag is "div" → match.
        assert_eq!(coll.length(&dom), 1);
    }

    #[test]
    fn by_tag_name_uppercase_element_matches_via_ascii_ci() {
        // Per WHATWG DOM §4.2.6.2, tag matching for HTML documents is
        // ASCII case-insensitive. Elements with non-canonical TagType
        // (`EcsDom::create_element("DIV", _)`) match a filter whose
        // needle was constructor-lowercased to "div".
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let div = dom.create_element("DIV", Attributes::default());
        dom.append_child(body, div);

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByTagName("DIV".into()),
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.length(&dom), 1);
        assert_eq!(coll.item(0, &dom), Some(div));
    }

    #[test]
    fn by_tag_name_mixed_case_element_matches_via_ascii_ci() {
        // Mixed-case TagType ("Div") still matches a lowercased filter
        // ("div") via ASCII-CI comparison.
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let mixed = dom.create_element("Div", Attributes::default());
        dom.append_child(body, mixed);

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByTagName("div".into()),
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.length(&dom), 1);
        assert_eq!(coll.item(0, &dom), Some(mixed));
    }

    #[test]
    fn by_tag_name_empty() {
        let (dom, doc) = setup_dom();
        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByTagName("article".into()),
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.length(&dom), 0);
    }

    #[test]
    fn by_class_names_single() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let mut attrs = Attributes::default();
        attrs.set("class", "foo bar");
        let div = dom.create_element("div", attrs);
        dom.append_child(body, div);

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByClassNames(vec!["foo".into()]),
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.length(&dom), 1);
        assert_eq!(coll.item(0, &dom), Some(div));
    }

    #[test]
    fn by_class_names_multi() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let mut attrs = Attributes::default();
        attrs.set("class", "foo bar baz");
        let div = dom.create_element("div", attrs);
        dom.append_child(body, div);

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByClassNames(vec!["foo".into(), "baz".into()]),
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.length(&dom), 1);
    }

    #[test]
    fn by_class_names_partial_no_match() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let mut attrs = Attributes::default();
        attrs.set("class", "foo");
        let div = dom.create_element("div", attrs);
        dom.append_child(body, div);

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByClassNames(vec!["foo".into(), "missing".into()]),
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.length(&dom), 0);
    }

    #[test]
    fn by_class_names_empty_vec() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let mut attrs = Attributes::default();
        attrs.set("class", "foo");
        let div = dom.create_element("div", attrs);
        dom.append_child(body, div);

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByClassNames(vec![]),
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.length(&dom), 0);
    }

    #[test]
    fn by_name() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let mut attrs = Attributes::default();
        attrs.set("name", "myfield");
        let input = dom.create_element("input", attrs);
        dom.append_child(body, input);

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByName("myfield".into()),
            CollectionKind::NodeList,
        );
        assert_eq!(coll.length(&dom), 1);
        assert_eq!(coll.item(0, &dom), Some(input));
    }

    #[test]
    fn images_filter() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let img = dom.create_element("img", Attributes::default());
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, img);
        dom.append_child(body, div);

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::Images,
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.length(&dom), 1);
        assert_eq!(coll.item(0, &dom), Some(img));
    }

    #[test]
    fn forms_filter() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let form = dom.create_element("form", Attributes::default());
        dom.append_child(body, form);

        let mut coll =
            LiveCollection::new(doc, CollectionFilter::Forms, CollectionKind::HtmlCollection);
        assert_eq!(coll.length(&dom), 1);
        assert_eq!(coll.item(0, &dom), Some(form));
    }

    #[test]
    fn links_filter() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let mut attrs = Attributes::default();
        attrs.set("href", "https://example.com");
        let a = dom.create_element("a", attrs);
        // An <a> without href should NOT match.
        let a_no_href = dom.create_element("a", Attributes::default());
        dom.append_child(body, a);
        dom.append_child(body, a_no_href);

        let mut coll =
            LiveCollection::new(doc, CollectionFilter::Links, CollectionKind::HtmlCollection);
        assert_eq!(coll.length(&dom), 1);
        assert_eq!(coll.item(0, &dom), Some(a));
    }

    #[test]
    fn child_nodes_includes_text() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let div = dom.create_element("div", Attributes::default());
        let text = dom.create_text("hello");
        dom.append_child(body, div);
        dom.append_child(body, text);

        let mut coll =
            LiveCollection::new(body, CollectionFilter::ChildNodes, CollectionKind::NodeList);
        assert_eq!(coll.length(&dom), 2);
        let snap = coll.snapshot(&dom);
        assert!(snap.contains(&div));
        assert!(snap.contains(&text));
    }

    #[test]
    fn element_children_excludes_text() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let div = dom.create_element("div", Attributes::default());
        let text = dom.create_text("hello");
        dom.append_child(body, div);
        dom.append_child(body, text);

        let mut coll = LiveCollection::new(
            body,
            CollectionFilter::ElementChildren,
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.length(&dom), 1);
        assert_eq!(coll.item(0, &dom), Some(div));
    }

    #[test]
    fn cache_invalidation_same_subtree() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByTagName("div".into()),
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.length(&dom), 1);

        // Add another div — cache should invalidate.
        let div2 = dom.create_element("div", Attributes::default());
        dom.append_child(body, div2);
        assert_eq!(coll.length(&dom), 2);
    }

    #[test]
    fn cache_preserved_different_subtree() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        // Collection rooted at div — mutations outside div should not invalidate.
        let mut coll = LiveCollection::new(
            div,
            CollectionFilter::ByTagName("span".into()),
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.length(&dom), 0);

        // Store the version after first access.
        let version_after_first = coll.cached_version;

        // Add a span as sibling of div (under body, not under div).
        let span = dom.create_element("span", Attributes::default());
        dom.append_child(body, span);

        // The version on `div` should not have changed.
        assert_eq!(dom.inclusive_descendants_version(div), version_after_first);
        assert_eq!(coll.length(&dom), 0);
    }

    #[test]
    fn cache_reuse() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByTagName("div".into()),
            CollectionKind::HtmlCollection,
        );
        // First access populates cache.
        assert_eq!(coll.length(&dom), 1);
        let v1 = coll.cached_version;

        // Second access without mutation should reuse cache.
        assert_eq!(coll.length(&dom), 1);
        assert_eq!(coll.cached_version, v1);
    }

    #[test]
    fn item_out_of_bounds() {
        let (dom, doc) = setup_dom();
        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByTagName("div".into()),
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.item(0, &dom), None);
        assert_eq!(coll.item(99, &dom), None);
    }

    #[test]
    fn shadow_tree_excluded() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];

        // Create a shadow host and attach a shadow root.
        let host = dom.create_element("div", Attributes::default());
        dom.append_child(body, host);
        let shadow_root = dom
            .attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
            .unwrap();

        // Add a span inside the shadow tree.
        let shadow_span = dom.create_element("span", Attributes::default());
        dom.append_child(shadow_root, shadow_span);

        // Add a span in the light DOM for comparison.
        let light_span = dom.create_element("span", Attributes::default());
        dom.append_child(body, light_span);

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByTagName("span".into()),
            CollectionKind::HtmlCollection,
        );
        // Only the light DOM span should be found.
        assert_eq!(coll.length(&dom), 1);
        assert_eq!(coll.item(0, &dom), Some(light_span));
    }

    // -- Snapshot variant -----------------------------------------------------

    #[test]
    fn snapshot_returns_stored_entities() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let div = dom.create_element("div", Attributes::default());
        let span = dom.create_element("span", Attributes::default());
        dom.append_child(body, div);
        dom.append_child(body, span);

        let mut coll = LiveCollection::new_snapshot(vec![div, span], CollectionKind::NodeList);
        assert_eq!(coll.length(&dom), 2);
        assert_eq!(coll.item(0, &dom), Some(div));
        assert_eq!(coll.item(1, &dom), Some(span));
    }

    #[test]
    fn snapshot_unaffected_by_dom_mutation() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);

        let mut coll = LiveCollection::new_snapshot(vec![div], CollectionKind::NodeList);
        assert_eq!(coll.length(&dom), 1);

        // Add another sibling and mutate the existing one.
        let div2 = dom.create_element("div", Attributes::default());
        dom.append_child(body, div2);

        // Snapshot stays fixed at the original capture.
        assert_eq!(coll.length(&dom), 1);
        assert_eq!(coll.item(0, &dom), Some(div));
    }

    #[test]
    fn snapshot_empty_vec() {
        let (dom, _doc) = setup_dom();
        let mut coll = LiveCollection::new_snapshot(Vec::new(), CollectionKind::NodeList);
        assert_eq!(coll.length(&dom), 0);
        assert_eq!(coll.item(0, &dom), None);
    }

    #[test]
    fn snapshot_kind_is_node_list_per_spec() {
        // WHATWG DOM §4.2.6 — querySelectorAll returns a non-live NodeList.
        let coll = LiveCollection::new_snapshot(Vec::new(), CollectionKind::NodeList);
        assert_eq!(coll.kind(), CollectionKind::NodeList);
        assert!(matches!(coll.filter(), CollectionFilter::Snapshot));
    }

    // -- Case-insensitive Forms / Images / Links ------------------------------

    #[test]
    fn forms_case_insensitive() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let upper = dom.create_element("FORM", Attributes::default());
        let lower = dom.create_element("form", Attributes::default());
        dom.append_child(body, upper);
        dom.append_child(body, lower);

        let mut coll =
            LiveCollection::new(doc, CollectionFilter::Forms, CollectionKind::HtmlCollection);
        assert_eq!(coll.length(&dom), 2);
    }

    #[test]
    fn images_case_insensitive() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let upper = dom.create_element("IMG", Attributes::default());
        let mixed = dom.create_element("Img", Attributes::default());
        dom.append_child(body, upper);
        dom.append_child(body, mixed);

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::Images,
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.length(&dom), 2);
    }

    #[test]
    fn links_case_insensitive_a_and_area() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let mut href_attrs = Attributes::default();
        href_attrs.set("href", "https://example.com");
        let a_upper = dom.create_element("A", href_attrs.clone());
        let area_upper = dom.create_element("AREA", href_attrs);
        // Without href — must NOT match even with case match.
        let a_no_href = dom.create_element("A", Attributes::default());
        dom.append_child(body, a_upper);
        dom.append_child(body, area_upper);
        dom.append_child(body, a_no_href);

        let mut coll =
            LiveCollection::new(doc, CollectionFilter::Links, CollectionKind::HtmlCollection);
        assert_eq!(coll.length(&dom), 2);
    }

    #[test]
    fn forms_mixed_case_siblings() {
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let f1 = dom.create_element("form", Attributes::default());
        let f2 = dom.create_element("FORM", Attributes::default());
        let f3 = dom.create_element("Form", Attributes::default());
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, f1);
        dom.append_child(body, f2);
        dom.append_child(body, f3);
        dom.append_child(body, div);

        let mut coll =
            LiveCollection::new(doc, CollectionFilter::Forms, CollectionKind::HtmlCollection);
        assert_eq!(coll.length(&dom), 3);
        assert_eq!(coll.item(3, &dom), None);
    }

    // -- cloneNode rev_version invalidation regression ------------------------

    #[test]
    fn clone_subtree_does_not_invalidate_external_collection() {
        // Live collection rooted in a sibling subtree is unaffected
        // by cloneNode of an unrelated source — `clone_subtree` only
        // bumps rev_version on the new subtree's ancestors (during
        // its append_child internals), not on the source's siblings.
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let target = dom.create_element("div", Attributes::default());
        let target_child = dom.create_element("span", Attributes::default());
        dom.append_child(body, target);
        dom.append_child(target, target_child);

        let source = dom.create_element("section", Attributes::default());
        dom.append_child(body, source);

        let mut coll = LiveCollection::new(
            target,
            CollectionFilter::ByTagName("span".into()),
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.length(&dom), 1);
        let pre_version = coll.cached_version;

        let _clone = dom.clone_subtree(source).expect("source exists");

        // The orphan clone has not been attached anywhere — `target`'s
        // subtree is untouched, so the cache stays valid.
        assert_eq!(coll.length(&dom), 1);
        assert_eq!(coll.cached_version, pre_version);
    }

    #[test]
    fn live_collection_sees_appended_clone_descendants() {
        // After attaching a clone, the parent's live collection
        // observes the clone's descendants — `append_child` bumps
        // rev_version on the parent, invalidating the cache.
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];

        let source = dom.create_element("section", Attributes::default());
        let inner = dom.create_element("span", Attributes::default());
        dom.append_child(body, source);
        dom.append_child(source, inner);

        let target = dom.create_element("div", Attributes::default());
        dom.append_child(body, target);

        let mut coll = LiveCollection::new(
            target,
            CollectionFilter::ByTagName("span".into()),
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.length(&dom), 0);

        let clone = dom.clone_subtree(source).expect("source exists");
        dom.append_child(target, clone);

        // The clone's <span> descendant is now under target.
        assert_eq!(coll.length(&dom), 1);
    }

    // -- Buffer reuse regression ----------------------------------------------

    #[test]
    fn refresh_reuses_cache_buffer_capacity() {
        // Mutation-heavy refresh path must not re-allocate `cached_snapshot`
        // each time. Repeated refresh cycles preserve the high-water-mark
        // capacity AND the underlying allocation (clear()+extend_from_slice,
        // not assignment).
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];

        // Pre-populate with several divs so the buffer grows.
        for _ in 0..5 {
            let d = dom.create_element("div", Attributes::default());
            dom.append_child(body, d);
        }

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByTagName("div".into()),
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.length(&dom), 5);
        let cap_after_first = coll.cached_snapshot.capacity();
        let ptr_after_first = coll.cached_snapshot.as_ptr();
        assert!(cap_after_first >= 5);

        // Trigger a refresh that doesn't grow past the high-water mark.
        let extra = dom.create_element("span", Attributes::default());
        dom.append_child(body, extra);
        assert_eq!(coll.length(&dom), 5); // div count unchanged

        // Capacity preserved AND allocation reused — no reallocation
        // when the refreshed result fits in the existing buffer.
        assert_eq!(coll.cached_snapshot.capacity(), cap_after_first);
        assert_eq!(coll.cached_snapshot.as_ptr(), ptr_after_first);
    }
}
