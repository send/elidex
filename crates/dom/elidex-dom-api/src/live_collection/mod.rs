//! Live DOM collections (`HTMLCollection`, `NodeList`).
//!
//! A `LiveCollection` lazily evaluates against the current DOM tree state,
//! caching results until the subtree version changes. This mirrors the
//! WHATWG DOM specification's live collection semantics.

use elidex_ecs::{Attributes, EcsDom, Entity, ShadowRoot, TagType};

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
    /// Match descendant *listed* form-control elements (HTML
    /// §4.10.2): `<button>`, `<fieldset>`, `<input>`, `<object>`,
    /// `<output>`, `<select>`, `<textarea>`.  Backs
    /// `HTMLFormElement.elements` / `HTMLFieldSetElement.elements`.
    /// Cross-tree `form="<id>"` association is **not** modelled here
    /// (deferred — would require a cross-tree walk, which the live
    /// collection's "descendants of root" model is not designed for).
    FormControls,
    /// Match descendant `<option>` elements.  Backs
    /// `HTMLSelectElement.options` (HTML §4.10.10.2).  `<optgroup>`
    /// nesting is handled implicitly by the descendant traversal.
    Options,
    /// Match descendant `<option>` elements that are *effectively*
    /// selected per HTML §4.10.10.2 ("ask for a reset"): any option
    /// with the `selected` content attribute set is included; if no
    /// option carries `selected`, the **first non-disabled option**
    /// is included as the implicit default — but only when the
    /// owning `<select>` is non-multiple AND its display size is 1
    /// (parsed from the `size` attribute, missing / "0" / invalid →
    /// default 1).  Listbox-style selects (`size > 1`) and `multiple`
    /// selects yield an empty collection when no option has
    /// `selected`.  Backs `HTMLSelectElement.selectedOptions` (HTML
    /// §4.10.7.4) — must be live so callers holding the collection
    /// across mutations see updated state.  Implementation lives in
    /// the private `populate_selected_options` walker (the
    /// per-entity matcher path can't express this rule because it
    /// requires whole-list inspection to find the implicit default).
    SelectedOptions,
    /// Match *direct* children of the root whose tag matches any of
    /// the given names (ASCII-case-insensitive).  Empty vec yields
    /// an empty result.  Backs `<table>.tBodies`
    /// (`vec!["tbody"]`), section.rows / `<tr>.cells`
    /// (`vec!["tr"]` and `vec!["td","th"]` respectively).  Strings
    /// are owned (`Vec<String>`) to match the `ByClassNames` precedent
    /// — filter is moved into the `LiveCollection`, so `&'static str`
    /// slices won't compile across the dispatch surface.  Per-call
    /// allocation is at filter **construction** (one-time per
    /// `LiveCollection::new`), not on match (hot path).
    DirectChildrenByTagName(Vec<String>),
    /// Match `<table>.rows` per HTML §4.9.1: thead's tr (in tree
    /// order) → table-direct tr / tbody's tr (interleaved in tree
    /// order) → tfoot's tr.  Bespoke walker because the algorithm
    /// requires whole-tree inspection of the root `<table>`.
    TableRows,
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
///
/// `cached_version` is `Option<u64>` rather than a `u64` sentinel: a sentinel
/// value (e.g. `u64::MAX`) could legally collide with a real subtree version
/// since `EcsDom::rev_version` increments via `wrapping_add(1)`, so a
/// collection created at exact wraparound would false-hit the cache check
/// and silently surface an empty snapshot. `None` is structurally distinct.
#[derive(Debug)]
pub struct LiveCollection {
    root: Option<Entity>,
    filter: CollectionFilter,
    kind: CollectionKind,
    cached_version: Option<u64>,
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
    /// `Snapshot` is **not** a valid filter for `new` — the entity list of
    /// a static collection lives in [`Self::new_snapshot`]'s in-place
    /// buffer (no per-filter Vec), so passing `CollectionFilter::Snapshot`
    /// here would yield a permanently empty collection. All builds panic
    /// to surface the misuse immediately; callers must use
    /// [`Self::new_snapshot`] for static lists.
    #[must_use]
    pub fn new(root: Entity, filter: CollectionFilter, kind: CollectionKind) -> Self {
        assert!(
            !matches!(filter, CollectionFilter::Snapshot),
            "LiveCollection::new called with Snapshot filter — use new_snapshot instead"
        );
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
            cached_version: None,
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
            // Refresh bypass for Snapshot is the
            // `CollectionFilter::Snapshot` early return in
            // `refresh_if_stale`; this field is observably unused
            // here. `None` matches the structural "no refresh has
            // run" state.
            cached_version: None,
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

    /// The root entity that bounds this collection's walk, or `None`
    /// for `Snapshot` collections (whose entity list is frozen and
    /// has no live root).
    #[must_use]
    pub fn root(&self) -> Option<Entity> {
        self.root
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
        if self.cached_version != Some(current_version) {
            self.refresh(dom);
            self.cached_version = Some(current_version);
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
            CollectionFilter::SelectedOptions => populate_selected_options(root, dom, out),
            CollectionFilter::DirectChildrenByTagName(tags) => {
                populate_direct_children_with_tags(root, dom, tags, out);
            }
            CollectionFilter::TableRows => populate_table_rows(root, dom, out),
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

/// Populate `out` with the selected option descendants of `root`,
/// applying HTML §4.10.10 selectedness — both the explicit
/// `selected` attribute path AND the implicit default for
/// `<select size=1>` / non-multiple selects.
///
/// `<select><option>A</option></select>` (no explicit `selected`)
/// must report `selectedOptions[0] === optionA` to match the
/// outcome of `select.selectedIndex` (= 0) and `select.value`
/// (= "A"); the previous attribute-only filter returned an empty
/// collection and broke this consistency invariant.
fn populate_selected_options(root: Entity, dom: &EcsDom, out: &mut Vec<Entity>) {
    let mut options: Vec<Entity> = Vec::new();
    dom.traverse_descendants(root, |entity| {
        if matches_tag_ascii_ci(entity, "option", dom) {
            options.push(entity);
        }
        true
    });
    let any_explicit = options.iter().any(|opt| {
        dom.world()
            .get::<&Attributes>(*opt)
            .is_ok_and(|a| a.contains("selected"))
    });
    if any_explicit {
        for opt in &options {
            if dom
                .world()
                .get::<&Attributes>(*opt)
                .is_ok_and(|a| a.contains("selected"))
            {
                out.push(*opt);
            }
        }
        return;
    }
    // Implicit default per HTML §4.10.10.2 ("ask for a reset"):
    // only non-multiple selects with `display size == 1` pick the
    // first non-disabled option.  Multi-select OR `<select
    // size="N">` (N > 1, listbox style) yields an empty
    // `selectedOptions`.  "Display size" is the parsed `size`
    // attribute (positive integer) — missing / "0" / invalid →
    // default 1 (matching `elidex_form::init_select_options`).
    let multiple = dom
        .world()
        .get::<&Attributes>(root)
        .is_ok_and(|a| a.contains("multiple"));
    if multiple {
        return;
    }
    let display_size = dom
        .world()
        .get::<&Attributes>(root)
        .ok()
        .and_then(|a| a.get("size").and_then(|s| s.parse::<u32>().ok()))
        .filter(|&n| n > 0)
        .unwrap_or(1);
    if display_size > 1 {
        return;
    }
    for opt in &options {
        if !crate::element::is_option_disabled(dom, *opt) {
            out.push(*opt);
            return;
        }
    }
}

/// `DirectChildrenByTagName` walker — direct children of `root`
/// whose tag matches any of `tags` (ASCII-case-insensitive).  Empty
/// `tags` yields nothing (matching the empty `ByClassNames` semantic).
fn populate_direct_children_with_tags(
    root: Entity,
    dom: &EcsDom,
    tags: &[String],
    out: &mut Vec<Entity>,
) {
    if tags.is_empty() {
        return;
    }
    for child in dom.children_iter(root) {
        let Ok(tt) = dom.world().get::<&TagType>(child) else {
            continue;
        };
        let child_tag = tt.0.as_str();
        if tags.iter().any(|t| child_tag.eq_ignore_ascii_case(t)) {
            out.push(child);
        }
    }
}

/// `<table>.rows` walker per HTML §4.9.1: append all `<tr>` direct
/// children of the **first** `<thead>` direct child (in tree order),
/// then walk the table's direct children appending direct `<tr>`s and
/// expanding `<tbody>` direct children's `<tr>`s, then append all
/// `<tr>` direct children of the **first** `<tfoot>` direct child.
///
/// `thead`/`tfoot` themselves do NOT appear in the output — only
/// their direct `<tr>` children (HTML §4.9.1 explicitly walks
/// "rows of the table").  Multiple `<thead>` / `<tfoot>` direct
/// children: HTML §4.9.1 specifies only the **first** `<thead>` and
/// the **first** `<tfoot>` participate (additional ones are
/// non-conforming and ignored).
fn populate_table_rows(root: Entity, dom: &EcsDom, out: &mut Vec<Entity>) {
    // Collect thead's direct <tr> children first.
    let thead = dom.first_child_with_tag(root, "thead");
    if let Some(thead) = thead {
        for tr in dom.children_iter(thead) {
            if matches_tag_ascii_ci(tr, "tr", dom) {
                out.push(tr);
            }
        }
    }
    // Walk root's direct children: collect direct <tr>, expand <tbody>'s direct <tr>.
    // <thead>/<tfoot> are skipped here (handled separately).
    for child in dom.children_iter(root) {
        if matches_tag_ascii_ci(child, "tr", dom) {
            out.push(child);
        } else if matches_tag_ascii_ci(child, "tbody", dom) {
            for tr in dom.children_iter(child) {
                if matches_tag_ascii_ci(tr, "tr", dom) {
                    out.push(tr);
                }
            }
        }
    }
    // Collect tfoot's direct <tr> children last.
    let tfoot = dom.first_child_with_tag(root, "tfoot");
    if let Some(tfoot) = tfoot {
        for tr in dom.children_iter(tfoot) {
            if matches_tag_ascii_ci(tr, "tr", dom) {
                out.push(tr);
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
        CollectionFilter::FormControls => matches_tag_ascii_ci_listed(entity, dom),
        CollectionFilter::Options => matches_tag_ascii_ci(entity, "option", dom),
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
        // SelectedOptions / ChildNodes / ElementChildren / Snapshot
        // are dispatched in `populate_into` directly — SelectedOptions
        // because its HTML §4.10.10.2 implicit-default rule needs to
        // see the whole option list, the rest because they walk the
        // direct-child / cached-snapshot fast path.  The `false`
        // return makes the per-entity matcher path a no-op for these,
        // matching the populate-time short-circuit.
        // T2c `DirectChildrenByTagName` / `TableRows` are also
        // direct-walk filters that bypass the descendant-traversal
        // matcher path.
        CollectionFilter::ChildNodes
        | CollectionFilter::ElementChildren
        | CollectionFilter::Snapshot
        | CollectionFilter::SelectedOptions
        | CollectionFilter::DirectChildrenByTagName(_)
        | CollectionFilter::TableRows => false,
    }
}

// FormControls / Options arms above use `matches_tag_ascii_ci` and a
// flat-match against the listed-element tag set; both are handled
// inside `matches_filter` directly so no special branch in
// `populate_into` is needed.

/// `FormControls` filter — tests `entity` against the HTML §4.10.2
/// listed-elements set (`button` / `fieldset` / `input` / `object` /
/// `output` / `select` / `textarea`) ASCII-case-insensitively.  Same
/// rationale as [`matches_tag_ascii_ci`]: tags reach this matcher
/// from non-parser paths that may not have lowercased them.
fn matches_tag_ascii_ci_listed(entity: Entity, dom: &EcsDom) -> bool {
    let Ok(tt) = dom.world().get::<&TagType>(entity) else {
        return false;
    };
    let tag = tt.0.as_str();
    [
        "button", "fieldset", "input", "object", "output", "select", "textarea",
    ]
    .iter()
    .any(|listed| tag.eq_ignore_ascii_case(listed))
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

#[cfg(test)]
mod tests;
