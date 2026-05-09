//! Tests for `LiveCollection` (HTMLCollection / NodeList semantics).

#![allow(unused_must_use)] // setup helpers (`dom.append_child`, …) do not need their return values checked.

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
    assert_eq!(
        Some(dom.inclusive_descendants_version(div)),
        version_after_first
    );
    assert_eq!(coll.length(&dom), 0);
}

#[test]
fn cache_uninitialized_state_distinct_from_any_real_version() {
    // Regression: `cached_version` was a `u64` with `u64::MAX` as
    // an "uninitialized" sentinel, which could legally collide
    // with `EcsDom::rev_version`'s `wrapping_add(1)` value at
    // wraparound. `Option<u64>` makes the "no refresh has run"
    // state structurally distinct from any real version.
    let (dom, doc) = setup_dom();
    let coll = LiveCollection::new(
        doc,
        CollectionFilter::ByTagName("div".into()),
        CollectionKind::HtmlCollection,
    );
    // Pre-first-access state is `None`, not a sentinel u64.
    assert_eq!(coll.cached_version, None);
    let _ = &dom; // Touch so the imports stay used in this test.
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
#[should_panic(expected = "use new_snapshot instead")]
fn new_with_snapshot_filter_panics() {
    // The Snapshot filter routes through `new_snapshot` only;
    // calling `new` with it would silently produce an empty
    // collection because `cached_snapshot` starts empty and
    // `refresh_if_stale` skips Snapshot. All builds panic
    // (release-safe assertion).
    let (_dom, doc) = setup_dom();
    let _coll = LiveCollection::new(doc, CollectionFilter::Snapshot, CollectionKind::NodeList);
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

// -- SelectedOptions implicit-default rule (HTML §4.10.10.2) -------------

fn make_select(dom: &mut EcsDom, multiple: bool) -> Entity {
    let mut attrs = Attributes::default();
    if multiple {
        attrs.set("multiple", "");
    }
    dom.create_element("select", attrs)
}

fn make_option(dom: &mut EcsDom, selected: bool, disabled: bool) -> Entity {
    let mut attrs = Attributes::default();
    if selected {
        attrs.set("selected", "");
    }
    if disabled {
        attrs.set("disabled", "");
    }
    dom.create_element("option", attrs)
}

#[test]
fn selected_options_explicit_attribute_only() {
    let mut dom = EcsDom::new();
    let s = make_select(&mut dom, /*multiple=*/ false);
    let o1 = make_option(&mut dom, /*selected=*/ false, /*disabled=*/ false);
    let o2 = make_option(&mut dom, /*selected=*/ true, /*disabled=*/ false);
    dom.append_child(s, o1);
    dom.append_child(s, o2);
    let mut coll = LiveCollection::new(
        s,
        CollectionFilter::SelectedOptions,
        CollectionKind::HtmlCollection,
    );
    assert_eq!(coll.length(&dom), 1);
    assert_eq!(coll.item(0, &dom), Some(o2));
}

#[test]
fn selected_options_implicit_default_for_size_one_select() {
    let mut dom = EcsDom::new();
    let s = make_select(&mut dom, /*multiple=*/ false);
    let o1 = make_option(&mut dom, /*selected=*/ false, /*disabled=*/ false);
    dom.append_child(s, o1);
    let mut coll = LiveCollection::new(
        s,
        CollectionFilter::SelectedOptions,
        CollectionKind::HtmlCollection,
    );
    // Implicit default: with no `selected` attribute and a
    // non-multiple select, the first non-disabled option is the
    // implicit selection.
    assert_eq!(coll.length(&dom), 1);
    assert_eq!(coll.item(0, &dom), Some(o1));
}

#[test]
fn selected_options_implicit_default_skips_disabled() {
    let mut dom = EcsDom::new();
    let s = make_select(&mut dom, /*multiple=*/ false);
    let o1 = make_option(&mut dom, /*selected=*/ false, /*disabled=*/ true);
    let o2 = make_option(&mut dom, /*selected=*/ false, /*disabled=*/ false);
    dom.append_child(s, o1);
    dom.append_child(s, o2);
    let mut coll = LiveCollection::new(
        s,
        CollectionFilter::SelectedOptions,
        CollectionKind::HtmlCollection,
    );
    assert_eq!(coll.length(&dom), 1);
    assert_eq!(coll.item(0, &dom), Some(o2));
}

#[test]
fn selected_options_implicit_default_skips_optgroup_disabled() {
    let mut dom = EcsDom::new();
    let s = make_select(&mut dom, /*multiple=*/ false);
    let mut og_attrs = Attributes::default();
    og_attrs.set("disabled", "");
    let og = dom.create_element("optgroup", og_attrs);
    let o1 = make_option(&mut dom, /*selected=*/ false, /*disabled=*/ false);
    let o2 = make_option(&mut dom, /*selected=*/ false, /*disabled=*/ false);
    dom.append_child(s, og);
    dom.append_child(og, o1);
    dom.append_child(s, o2);
    let mut coll = LiveCollection::new(
        s,
        CollectionFilter::SelectedOptions,
        CollectionKind::HtmlCollection,
    );
    // o1 is disabled-via-optgroup; the implicit default falls
    // through to o2.
    assert_eq!(coll.length(&dom), 1);
    assert_eq!(coll.item(0, &dom), Some(o2));
}

#[test]
fn selected_options_multiple_no_implicit_default() {
    let mut dom = EcsDom::new();
    let s = make_select(&mut dom, /*multiple=*/ true);
    let o1 = make_option(&mut dom, /*selected=*/ false, /*disabled=*/ false);
    dom.append_child(s, o1);
    let mut coll = LiveCollection::new(
        s,
        CollectionFilter::SelectedOptions,
        CollectionKind::HtmlCollection,
    );
    // Multi-select with no explicit selectedness yields an empty
    // collection — there's no implicit default in this case.
    assert_eq!(coll.length(&dom), 0);
}

#[test]
fn selected_options_explicit_overrides_implicit() {
    let mut dom = EcsDom::new();
    let s = make_select(&mut dom, /*multiple=*/ false);
    let o1 = make_option(&mut dom, /*selected=*/ false, /*disabled=*/ false);
    let o2 = make_option(&mut dom, /*selected=*/ true, /*disabled=*/ false);
    dom.append_child(s, o1);
    dom.append_child(s, o2);
    let mut coll = LiveCollection::new(
        s,
        CollectionFilter::SelectedOptions,
        CollectionKind::HtmlCollection,
    );
    // Explicit selectedness on o2 short-circuits the implicit
    // default — only o2 is in the collection, even though o1
    // would be the implicit default if no option had `selected`.
    assert_eq!(coll.length(&dom), 1);
    assert_eq!(coll.item(0, &dom), Some(o2));
}

#[test]
fn selected_options_listbox_size_gt_one_no_implicit_default() {
    // R28 regression — `<select size="3">` is a listbox-style
    // select (display size > 1).  HTML §4.10.10.2 "ask for a
    // reset" only auto-selects when display size == 1, so a
    // listbox with no explicit `selected` attr must yield an
    // empty `selectedOptions`.
    let mut dom = EcsDom::new();
    let mut s_attrs = Attributes::default();
    s_attrs.set("size", "3");
    let s = dom.create_element("select", s_attrs);
    let o1 = make_option(&mut dom, /*selected=*/ false, /*disabled=*/ false);
    dom.append_child(s, o1);
    let mut coll = LiveCollection::new(
        s,
        CollectionFilter::SelectedOptions,
        CollectionKind::HtmlCollection,
    );
    assert_eq!(coll.length(&dom), 0);
}

#[test]
fn selected_options_size_zero_falls_to_implicit_default() {
    // `size="0"` is invalid per HTML; display size defaults to
    // 1 for non-multiple selects, so implicit default still
    // applies.  Mirrors `elidex_form::init_select_options`'s
    // `state.size <= 1` gate after the parsed value falls back
    // to the missing-default of 1.
    let mut dom = EcsDom::new();
    let mut s_attrs = Attributes::default();
    s_attrs.set("size", "0");
    let s = dom.create_element("select", s_attrs);
    let o1 = make_option(&mut dom, /*selected=*/ false, /*disabled=*/ false);
    dom.append_child(s, o1);
    let mut coll = LiveCollection::new(
        s,
        CollectionFilter::SelectedOptions,
        CollectionKind::HtmlCollection,
    );
    assert_eq!(coll.length(&dom), 1);
    assert_eq!(coll.item(0, &dom), Some(o1));
}

#[test]
fn selected_options_size_one_implicit_default() {
    // Explicit `size="1"` is the default for non-multiple
    // selects — implicit default applies.
    let mut dom = EcsDom::new();
    let mut s_attrs = Attributes::default();
    s_attrs.set("size", "1");
    let s = dom.create_element("select", s_attrs);
    let o1 = make_option(&mut dom, /*selected=*/ false, /*disabled=*/ false);
    dom.append_child(s, o1);
    let mut coll = LiveCollection::new(
        s,
        CollectionFilter::SelectedOptions,
        CollectionKind::HtmlCollection,
    );
    assert_eq!(coll.length(&dom), 1);
    assert_eq!(coll.item(0, &dom), Some(o1));
}

#[test]
fn selected_options_implicit_default_skips_nested_optgroup_disabled() {
    // R27 regression — `is_option_disabled_local` must walk the
    // full ancestor chain, not just the direct parent, so that
    // malformed trees with a wrapper between option and optgroup
    // (constructible via JS `appendChild`) still observe the
    // disabled propagation.  Tree: select > optgroup[disabled] >
    // div > o1  /  select > o2.
    let mut dom = EcsDom::new();
    let s = make_select(&mut dom, /*multiple=*/ false);
    let mut og_attrs = Attributes::default();
    og_attrs.set("disabled", "");
    let og = dom.create_element("optgroup", og_attrs);
    let wrapper = dom.create_element("div", Attributes::default());
    let o1 = make_option(&mut dom, /*selected=*/ false, /*disabled=*/ false);
    let o2 = make_option(&mut dom, /*selected=*/ false, /*disabled=*/ false);
    dom.append_child(s, og);
    dom.append_child(og, wrapper);
    dom.append_child(wrapper, o1);
    dom.append_child(s, o2);
    let mut coll = LiveCollection::new(
        s,
        CollectionFilter::SelectedOptions,
        CollectionKind::HtmlCollection,
    );
    // o1 is disabled-via-ancestor-optgroup (with a div wrapper);
    // implicit default falls through to o2.
    assert_eq!(coll.length(&dom), 1);
    assert_eq!(coll.item(0, &dom), Some(o2));
}
