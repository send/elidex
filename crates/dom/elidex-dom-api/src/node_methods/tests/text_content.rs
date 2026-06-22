//! Handler-direct tests for `SetTextContentNodeKind` Element/DocumentFragment
//! "string replace all" (WHATWG DOM §4.4 → §4.2.3). B1.2c.
//!
//! These exercise the engine-independent handler directly (boa/wasm-style). The
//! VM end-to-end coverage (real JS + delivered `MutationRecord`s) lives in
//! `elidex-js` `vm::tests::tests_mutation_observer::text_content`.

use super::*;

#[test]
fn textcontent_replaces_children_with_one_text() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    let s1 = dom.create_element("span", Attributes::default());
    let s2 = dom.create_element("span", Attributes::default());
    dom.append_child(div, s1);
    dom.append_child(div, s2);

    SetTextContentNodeKind
        .invoke(div, &[JsValue::String("hi".into())], &mut session, &mut dom)
        .unwrap();

    let children = dom.children(div);
    assert_eq!(children.len(), 1);
    assert_eq!(
        dom.world().get::<&TextContent>(children[0]).unwrap().0,
        "hi"
    );
}

#[test]
fn textcontent_empty_string_clears_children() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());
    let s1 = dom.create_element("span", Attributes::default());
    dom.append_child(div, s1);

    SetTextContentNodeKind
        .invoke(
            div,
            &[JsValue::String(String::new())],
            &mut session,
            &mut dom,
        )
        .unwrap();

    assert!(dom.children(div).is_empty());
}

#[test]
fn textcontent_empty_string_on_empty_element_is_noop() {
    let (mut dom, mut session) = setup();
    let div = dom.create_element("div", Attributes::default());

    // No children, empty value → no-op (and, per §4.2.3 step 7, no record — the
    // VM e2e test asserts the no-record; here we assert it neither panics nor
    // spuriously creates a child).
    SetTextContentNodeKind
        .invoke(
            div,
            &[JsValue::String(String::new())],
            &mut session,
            &mut dom,
        )
        .unwrap();

    assert!(dom.children(div).is_empty());
}
