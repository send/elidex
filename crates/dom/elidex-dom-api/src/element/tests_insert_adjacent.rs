//! Handler-direct tests for `InsertAdjacentElement` / `InsertAdjacentText`
//! (WHATWG DOM §4.9). Split out of `tests_tree.rs` (B1.2b-2) so the canonical
//! tree-handler test surface stays under the 1000-line convention and the
//! insert-adjacent cases live in one focused module.
//!
//! These exercise the engine-independent handlers directly (boa/wasm-style); the
//! VM has its own end-to-end coverage in `elidex-js`
//! `vm::tests::tests_insert_adjacent` / `..::tests_mutation_observer`.

use super::*;
use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_plugin::JsValue;
use elidex_script_session::{ComponentKind, DomApiErrorKind, DomApiHandler, SessionCore};

fn setup() -> (EcsDom, Entity, Entity, SessionCore) {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("span", Attributes::default());
    let mut session = SessionCore::new();
    // Pre-register entities so we can pass ObjectRef args.
    session.get_or_create_wrapper(parent, ComponentKind::Element);
    session.get_or_create_wrapper(child, ComponentKind::Element);
    (dom, parent, child, session)
}

#[test]
fn insert_adjacent_element_beforebegin() {
    let (mut dom, parent, _child, mut session) = setup();
    let root = dom.create_element("body", Attributes::default());
    dom.append_child(root, parent);

    let new_elem = dom.create_element("p", Attributes::default());
    session.get_or_create_wrapper(new_elem, ComponentKind::Element);
    let new_ref = session
        .get_or_create_wrapper(new_elem, ComponentKind::Element)
        .to_raw();

    let result = InsertAdjacentElement
        .invoke(
            parent,
            &[
                JsValue::String("beforebegin".into()),
                JsValue::ObjectRef(new_ref),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::ObjectRef(new_ref));
    let children = dom.children(root);
    assert_eq!(children, vec![new_elem, parent]);
}

#[test]
fn insert_adjacent_element_afterbegin() {
    let (mut dom, parent, child, mut session) = setup();
    dom.append_child(parent, child);

    let new_elem = dom.create_element("p", Attributes::default());
    session.get_or_create_wrapper(new_elem, ComponentKind::Element);
    let new_ref = session
        .get_or_create_wrapper(new_elem, ComponentKind::Element)
        .to_raw();

    InsertAdjacentElement
        .invoke(
            parent,
            &[
                JsValue::String("afterbegin".into()),
                JsValue::ObjectRef(new_ref),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let children = dom.children(parent);
    assert_eq!(children[0], new_elem);
    assert_eq!(children[1], child);
}

#[test]
fn insert_adjacent_element_beforeend() {
    let (mut dom, parent, child, mut session) = setup();
    dom.append_child(parent, child);

    let new_elem = dom.create_element("p", Attributes::default());
    session.get_or_create_wrapper(new_elem, ComponentKind::Element);
    let new_ref = session
        .get_or_create_wrapper(new_elem, ComponentKind::Element)
        .to_raw();

    InsertAdjacentElement
        .invoke(
            parent,
            &[
                JsValue::String("beforeend".into()),
                JsValue::ObjectRef(new_ref),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let children = dom.children(parent);
    assert_eq!(children[0], child);
    assert_eq!(children[1], new_elem);
}

#[test]
fn insert_adjacent_element_afterend() {
    let (mut dom, parent, _, mut session) = setup();
    let root = dom.create_element("body", Attributes::default());
    dom.append_child(root, parent);

    let new_elem = dom.create_element("p", Attributes::default());
    session.get_or_create_wrapper(new_elem, ComponentKind::Element);
    let new_ref = session
        .get_or_create_wrapper(new_elem, ComponentKind::Element)
        .to_raw();

    InsertAdjacentElement
        .invoke(
            parent,
            &[
                JsValue::String("afterend".into()),
                JsValue::ObjectRef(new_ref),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let children = dom.children(root);
    assert_eq!(children, vec![parent, new_elem]);
}

#[test]
fn insert_adjacent_element_invalid_position() {
    let (mut dom, parent, child, mut session) = setup();
    let child_ref = session
        .get_or_create_wrapper(child, ComponentKind::Element)
        .to_raw();
    let err = InsertAdjacentElement
        .invoke(
            parent,
            &[
                JsValue::String("invalid".into()),
                JsValue::ObjectRef(child_ref),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::SyntaxError);
}

#[test]
fn insert_adjacent_element_case_insensitive() {
    let (mut dom, parent, child, mut session) = setup();
    dom.append_child(parent, child);

    let new_elem = dom.create_element("p", Attributes::default());
    session.get_or_create_wrapper(new_elem, ComponentKind::Element);
    let new_ref = session
        .get_or_create_wrapper(new_elem, ComponentKind::Element)
        .to_raw();

    InsertAdjacentElement
        .invoke(
            parent,
            &[
                JsValue::String("BeforeEnd".into()),
                JsValue::ObjectRef(new_ref),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let children = dom.children(parent);
    assert_eq!(children[1], new_elem);
}

#[test]
fn insert_adjacent_text_beforeend() {
    let (mut dom, parent, _, mut session) = setup();
    InsertAdjacentText
        .invoke(
            parent,
            &[
                JsValue::String("beforeend".into()),
                JsValue::String("hello".into()),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap();
    let text = collect_text_content(parent, &dom);
    assert_eq!(text, "hello");
}

// B1.2b-2 — handler-direct coverage of the behaviours that the convergence
// relocated into the canonical (engine-independent) layer, so the boa/wasm
// runtimes inherit them (the VM has its own end-to-end coverage in
// `vm::tests::tests_insert_adjacent` / `..::tests_mutation_observer`).

#[test]
fn insert_adjacent_element_no_parent_returns_null_not_error() {
    // DOM `#insert-adjacent`: "If element's parent is null, return null" — a
    // silent no-op for `beforebegin`/`afterend`, NOT a HierarchyRequestError.
    // The handler wrongly threw before B1.2b-2 (boa-reachable). `parent` from
    // `setup()` has no parent of its own.
    let (mut dom, parent, _child, mut session) = setup();
    let new_elem = dom.create_element("p", Attributes::default());
    let new_ref = session
        .get_or_create_wrapper(new_elem, ComponentKind::Element)
        .to_raw();

    let result = InsertAdjacentElement
        .invoke(
            parent,
            &[
                JsValue::String("beforebegin".into()),
                JsValue::ObjectRef(new_ref),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Null);
    // Nothing was inserted anywhere.
    assert!(dom.get_parent(new_elem).is_none());
}

#[test]
fn insert_adjacent_element_rejects_non_element_arg_at_canonical_layer() {
    // WebIDL `Element element`: a non-Element node (here a Text) resolved through
    // the identity map must be rejected, NOT relinked. This guard is the sole
    // protection on the boa/wasm path (the VM additionally brand-checks the arg);
    // before B1.2b-2 the handler had no kind check and would insert the Text.
    let (mut dom, parent, _child, mut session) = setup();
    let text = dom.create_text("t");
    let text_ref = session
        .get_or_create_wrapper(text, ComponentKind::TextNode)
        .to_raw();

    let err = InsertAdjacentElement
        .invoke(
            parent,
            &[
                JsValue::String("beforeend".into()),
                JsValue::ObjectRef(text_ref),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap_err();
    assert_eq!(err.kind, DomApiErrorKind::TypeError);
    // No insertion occurred.
    assert!(dom.children(parent).is_empty());
    assert!(dom.get_parent(text).is_none());
}

#[test]
fn insert_adjacent_text_no_parent_noop_does_not_allocate_text() {
    // `beforebegin`/`afterend` on a parent-less receiver is a void no-op; the
    // handler MUST resolve the (no-op) site BEFORE allocating the Text, else an
    // unreferenced Text leaks into the ECS (no handle ⇒ never GC'd).
    use elidex_ecs::TextContent;
    let (mut dom, parent, _child, mut session) = setup();
    let before = dom.world().query::<&TextContent>().iter().count();

    let result = InsertAdjacentText
        .invoke(
            parent,
            &[
                JsValue::String("afterend".into()),
                JsValue::String("ghost".into()),
            ],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Undefined);
    let after = dom.world().query::<&TextContent>().iter().count();
    assert_eq!(
        before, after,
        "parent-null insertAdjacentText leaked a Text node"
    );
}
