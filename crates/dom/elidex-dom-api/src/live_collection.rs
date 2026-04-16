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
#[derive(Debug)]
pub struct LiveCollection {
    root: Entity,
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
    #[must_use]
    pub fn new(root: Entity, filter: CollectionFilter, kind: CollectionKind) -> Self {
        let filter = match filter {
            CollectionFilter::ByTagName(tag) => {
                CollectionFilter::ByTagName(tag.to_ascii_lowercase())
            }
            other => other,
        };
        Self {
            root,
            filter,
            kind,
            cached_version: UNINITIALIZED_VERSION,
            cached_snapshot: Vec::new(),
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
        let current_version = dom.inclusive_descendants_version(self.root);
        if current_version != self.cached_version {
            self.refresh(dom);
            self.cached_version = current_version;
        }
    }

    fn refresh(&mut self, dom: &EcsDom) {
        self.cached_snapshot = self.populate(dom);
    }

    fn populate(&self, dom: &EcsDom) -> Vec<Entity> {
        match &self.filter {
            // ChildNodes and ElementChildren only look at direct children.
            CollectionFilter::ChildNodes => {
                let mut result = Vec::new();
                collect_direct_children(dom, self.root, &mut result, true);
                result
            }
            CollectionFilter::ElementChildren => {
                let mut result = Vec::new();
                collect_direct_children(dom, self.root, &mut result, false);
                result
            }
            // ByClassNames with empty vec always returns empty.
            CollectionFilter::ByClassNames(names) if names.is_empty() => Vec::new(),
            // All other filters: pre-order traversal of the subtree.
            // Shadow boundaries are respected because `children_iter`
            // (used internally by `traverse_descendants`) skips
            // ShadowRoot entities, so shadow subtrees are unreachable.
            filter => {
                let mut result = Vec::new();
                dom.traverse_descendants(self.root, |entity| {
                    if matches_filter(entity, filter, dom) {
                        result.push(entity);
                    }
                    true
                });
                result
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
            match dom.world().get::<&TagType>(entity) {
                Ok(tt) => tt.0 == *tag,
                Err(_) => false,
            }
        }
        CollectionFilter::ByClassNames(names) => {
            // All class names must be present on the element.
            if names.is_empty() {
                return false;
            }
            match dom.world().get::<&Attributes>(entity) {
                Ok(attrs) => {
                    let Some(class_str) = attrs.get("class") else {
                        return false;
                    };
                    let element_classes: Vec<&str> = class_str.split_ascii_whitespace().collect();
                    names
                        .iter()
                        .all(|name| element_classes.contains(&name.as_str()))
                }
                Err(_) => false,
            }
        }
        CollectionFilter::ByName(name) => match dom.world().get::<&Attributes>(entity) {
            Ok(attrs) => attrs.get("name") == Some(name.as_str()),
            Err(_) => false,
        },
        CollectionFilter::Images => dom.has_tag(entity, "img"),
        CollectionFilter::Forms => dom.has_tag(entity, "form"),
        CollectionFilter::Links => {
            let is_link_tag = dom.has_tag(entity, "a") || dom.has_tag(entity, "area");
            if !is_link_tag {
                return false;
            }
            match dom.world().get::<&Attributes>(entity) {
                Ok(attrs) => attrs.get("href").is_some(),
                Err(_) => false,
            }
        }
        // ChildNodes / ElementChildren are handled in populate() directly.
        CollectionFilter::ChildNodes | CollectionFilter::ElementChildren => false,
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
    fn by_tag_name_uppercase_element_no_match() {
        // An element created with uppercase tag won't match a lowercased filter
        // (exact match). In real usage, the parser always lowercases tags.
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let div = dom.create_element("DIV", Attributes::default());
        dom.append_child(body, div);

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByTagName("DIV".into()),
            CollectionKind::HtmlCollection,
        );
        // Filter "DIV" lowercased to "div", element tag is "DIV" → no match.
        assert_eq!(coll.length(&dom), 0);
    }

    #[test]
    fn by_tag_name_exact_match_semantics() {
        // Verify exact match: "Div" (mixed case) element won't match "div" filter.
        let (mut dom, doc) = setup_dom();
        let body = dom.children(dom.children(doc)[0])[0];
        let mixed = dom.create_element("Div", Attributes::default());
        dom.append_child(body, mixed);

        let mut coll = LiveCollection::new(
            doc,
            CollectionFilter::ByTagName("div".into()),
            CollectionKind::HtmlCollection,
        );
        assert_eq!(coll.length(&dom), 0);
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
}
