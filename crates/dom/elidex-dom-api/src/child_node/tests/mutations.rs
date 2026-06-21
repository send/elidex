use super::setup;
use crate::child_node::{
    viable_next_sibling, viable_prev_sibling, After, Append, Before, ChildNodeRemove, Prepend,
    ReplaceChildren, ReplaceWith,
};
use elidex_ecs::{Attributes, EcsDom, TextContent};
use elidex_plugin::JsValue;
use elidex_script_session::{ComponentKind, DomApiHandler, SessionCore};

// ---- before ----

#[test]
fn before_single() {
    let (mut dom, body, div, span, _p, mut session) = setup();
    let new_el = dom.create_element("em", Attributes::default());
    let new_ref = session
        .get_or_create_wrapper(new_el, ComponentKind::Element)
        .to_raw();

    let handler = Before;
    handler
        .invoke(span, &[JsValue::ObjectRef(new_ref)], &mut session, &mut dom)
        .unwrap();

    let children = dom.children(body);
    assert_eq!(children.len(), 4);
    assert_eq!(children[0], div);
    assert_eq!(children[1], new_el);
    assert_eq!(children[2], span);
}

#[test]
fn before_multiple() {
    let (mut dom, body, div, span, _p, mut session) = setup();
    let a = dom.create_element("a", Attributes::default());
    let a_ref = session
        .get_or_create_wrapper(a, ComponentKind::Element)
        .to_raw();
    let b = dom.create_element("b", Attributes::default());
    let b_ref = session
        .get_or_create_wrapper(b, ComponentKind::Element)
        .to_raw();

    let handler = Before;
    handler
        .invoke(
            span,
            &[JsValue::ObjectRef(a_ref), JsValue::ObjectRef(b_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let children = dom.children(body);
    assert_eq!(children[0], div);
    assert_eq!(children[1], a);
    assert_eq!(children[2], b);
    assert_eq!(children[3], span);
}

#[test]
fn before_string_creates_text() {
    let (mut dom, body, _div, span, _p, mut session) = setup();

    let handler = Before;
    handler
        .invoke(
            span,
            &[JsValue::String("hello".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let children = dom.children(body);
    assert_eq!(children.len(), 4);
    // The text node should be before span.
    let text_entity = children[1];
    let tc = dom.world().get::<&TextContent>(text_entity).unwrap();
    assert_eq!(tc.0, "hello");
}

#[test]
fn before_orphan_noop() {
    let mut dom = EcsDom::new();
    let orphan = dom.create_element("div", Attributes::default());
    let mut session = SessionCore::new();

    let handler = Before;
    let result = handler.invoke(
        orphan,
        &[JsValue::String("text".into())],
        &mut session,
        &mut dom,
    );
    assert!(result.is_ok());
}

#[test]
fn before_self_in_nodes() {
    let (mut dom, body, div, span, _p, mut session) = setup();
    // Insert a new element before span.
    let new_el = dom.create_element("em", Attributes::default());
    let new_ref = session
        .get_or_create_wrapper(new_el, ComponentKind::Element)
        .to_raw();

    let handler = Before;
    handler
        .invoke(span, &[JsValue::ObjectRef(new_ref)], &mut session, &mut dom)
        .unwrap();

    let children = dom.children(body);
    assert_eq!(children[0], div);
    assert_eq!(children[1], new_el);
    assert_eq!(children[2], span);
}

// ---- after ----

#[test]
fn after_single() {
    let (mut dom, body, div, span, p, mut session) = setup();
    let new_el = dom.create_element("em", Attributes::default());
    let new_ref = session
        .get_or_create_wrapper(new_el, ComponentKind::Element)
        .to_raw();

    let handler = After;
    handler
        .invoke(span, &[JsValue::ObjectRef(new_ref)], &mut session, &mut dom)
        .unwrap();

    let children = dom.children(body);
    assert_eq!(children.len(), 4);
    assert_eq!(children[0], div);
    assert_eq!(children[1], span);
    assert_eq!(children[2], new_el);
    assert_eq!(children[3], p);
}

#[test]
fn after_multiple() {
    let (mut dom, body, div, span, p, mut session) = setup();
    let a = dom.create_element("a", Attributes::default());
    let a_ref = session
        .get_or_create_wrapper(a, ComponentKind::Element)
        .to_raw();
    let b = dom.create_element("b", Attributes::default());
    let b_ref = session
        .get_or_create_wrapper(b, ComponentKind::Element)
        .to_raw();

    let handler = After;
    handler
        .invoke(
            div,
            &[JsValue::ObjectRef(a_ref), JsValue::ObjectRef(b_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let children = dom.children(body);
    assert_eq!(children[0], div);
    assert_eq!(children[1], a);
    assert_eq!(children[2], b);
    assert_eq!(children[3], span);
    assert_eq!(children[4], p);
}

#[test]
fn after_validates_insertion() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("span", Attributes::default());
    dom.append_child(parent, child);
    let mut session = SessionCore::new();

    // Try to insert parent after child (would create cycle).
    let parent_ref = session
        .get_or_create_wrapper(parent, ComponentKind::Element)
        .to_raw();
    let handler = After;
    let result = handler.invoke(
        child,
        &[JsValue::ObjectRef(parent_ref)],
        &mut session,
        &mut dom,
    );
    assert!(result.is_err());
}

// ---- remove ----

#[test]
fn remove_attached() {
    let (mut dom, body, div, span, p, mut session) = setup();

    let handler = ChildNodeRemove;
    handler.invoke(span, &[], &mut session, &mut dom).unwrap();

    let children = dom.children(body);
    assert_eq!(children, vec![div, p]);
}

#[test]
fn remove_orphan_noop() {
    let mut dom = EcsDom::new();
    let orphan = dom.create_element("div", Attributes::default());
    let mut session = SessionCore::new();

    let handler = ChildNodeRemove;
    let result = handler.invoke(orphan, &[], &mut session, &mut dom);
    assert!(result.is_ok());
}

// ---- replaceWith ----

#[test]
fn replace_with_single() {
    let (mut dom, body, div, span, p, mut session) = setup();
    let new_el = dom.create_element("em", Attributes::default());
    let new_ref = session
        .get_or_create_wrapper(new_el, ComponentKind::Element)
        .to_raw();

    let handler = ReplaceWith;
    handler
        .invoke(span, &[JsValue::ObjectRef(new_ref)], &mut session, &mut dom)
        .unwrap();

    let children = dom.children(body);
    assert_eq!(children, vec![div, new_el, p]);
}

#[test]
fn replace_with_multiple() {
    let (mut dom, body, div, span, p, mut session) = setup();
    let a = dom.create_element("a", Attributes::default());
    let a_ref = session
        .get_or_create_wrapper(a, ComponentKind::Element)
        .to_raw();
    let b = dom.create_element("b", Attributes::default());
    let b_ref = session
        .get_or_create_wrapper(b, ComponentKind::Element)
        .to_raw();

    let handler = ReplaceWith;
    handler
        .invoke(
            span,
            &[JsValue::ObjectRef(a_ref), JsValue::ObjectRef(b_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let children = dom.children(body);
    assert_eq!(children, vec![div, a, b, p]);
}

#[test]
fn replace_with_empty_removes() {
    let (mut dom, body, div, span, p, mut session) = setup();

    let handler = ReplaceWith;
    handler.invoke(span, &[], &mut session, &mut dom).unwrap();

    let children = dom.children(body);
    assert_eq!(children, vec![div, p]);
}

// ---- prepend ----

#[test]
fn prepend_single() {
    let (mut dom, body, div, _span, _p, mut session) = setup();
    let new_el = dom.create_element("em", Attributes::default());
    let new_ref = session
        .get_or_create_wrapper(new_el, ComponentKind::Element)
        .to_raw();

    let handler = Prepend;
    handler
        .invoke(body, &[JsValue::ObjectRef(new_ref)], &mut session, &mut dom)
        .unwrap();

    let children = dom.children(body);
    assert_eq!(children[0], new_el);
    assert_eq!(children[1], div);
}

#[test]
fn prepend_multiple() {
    let (mut dom, body, div, _span, _p, mut session) = setup();
    let a = dom.create_element("a", Attributes::default());
    let a_ref = session
        .get_or_create_wrapper(a, ComponentKind::Element)
        .to_raw();
    let b = dom.create_element("b", Attributes::default());
    let b_ref = session
        .get_or_create_wrapper(b, ComponentKind::Element)
        .to_raw();

    let handler = Prepend;
    handler
        .invoke(
            body,
            &[JsValue::ObjectRef(a_ref), JsValue::ObjectRef(b_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let children = dom.children(body);
    assert_eq!(children[0], a);
    assert_eq!(children[1], b);
    assert_eq!(children[2], div);
}

#[test]
fn prepend_empty() {
    let (mut dom, body, div, span, p, mut session) = setup();

    let handler = Prepend;
    handler.invoke(body, &[], &mut session, &mut dom).unwrap();

    let children = dom.children(body);
    assert_eq!(children, vec![div, span, p]);
}

// ---- append ----

#[test]
fn append_single() {
    let (mut dom, body, _div, _span, _p, mut session) = setup();
    let new_el = dom.create_element("em", Attributes::default());
    let new_ref = session
        .get_or_create_wrapper(new_el, ComponentKind::Element)
        .to_raw();

    let handler = Append;
    handler
        .invoke(body, &[JsValue::ObjectRef(new_ref)], &mut session, &mut dom)
        .unwrap();

    let children = dom.children(body);
    assert_eq!(children.last(), Some(&new_el));
    assert_eq!(children.len(), 4);
}

#[test]
fn append_multiple() {
    let (mut dom, body, _div, _span, _p, mut session) = setup();
    let a = dom.create_element("a", Attributes::default());
    let a_ref = session
        .get_or_create_wrapper(a, ComponentKind::Element)
        .to_raw();
    let b = dom.create_element("b", Attributes::default());
    let b_ref = session
        .get_or_create_wrapper(b, ComponentKind::Element)
        .to_raw();

    let handler = Append;
    handler
        .invoke(
            body,
            &[JsValue::ObjectRef(a_ref), JsValue::ObjectRef(b_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let children = dom.children(body);
    assert_eq!(children.len(), 5);
    assert_eq!(children[3], a);
    assert_eq!(children[4], b);
}

// ---- replaceChildren ----

#[test]
fn replace_children() {
    let (mut dom, body, _div, _span, _p, mut session) = setup();
    let a = dom.create_element("a", Attributes::default());
    let a_ref = session
        .get_or_create_wrapper(a, ComponentKind::Element)
        .to_raw();
    let b = dom.create_element("b", Attributes::default());
    let b_ref = session
        .get_or_create_wrapper(b, ComponentKind::Element)
        .to_raw();

    let handler = ReplaceChildren;
    handler
        .invoke(
            body,
            &[JsValue::ObjectRef(a_ref), JsValue::ObjectRef(b_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap();

    let children = dom.children(body);
    assert_eq!(children, vec![a, b]);
}

#[test]
fn replace_children_validates_before_removing() {
    let mut dom = EcsDom::new();
    let text = dom.create_text("hello");
    // Cannot insert nodes under a text node.
    let child = dom.create_element("span", Attributes::default());
    let mut session = SessionCore::new();
    let child_ref = session
        .get_or_create_wrapper(child, ComponentKind::Element)
        .to_raw();
    // Calling replaceChildren on text should fail validation.
    let handler = ReplaceChildren;
    let result = handler.invoke(
        text,
        &[JsValue::ObjectRef(child_ref)],
        &mut session,
        &mut dom,
    );
    assert!(result.is_err());
}

// ---- viable_next_sibling ----

#[test]
fn viable_sibling_basic() {
    let (dom, _body, div, span, p, _session) = setup();

    // Next sibling of div, excluding nothing.
    assert_eq!(viable_next_sibling(div, &[], &dom), Some(span));

    // Next sibling of div, excluding span -> p.
    assert_eq!(viable_next_sibling(div, &[span], &dom), Some(p));

    // Next sibling of div, excluding span and p -> None.
    assert_eq!(viable_next_sibling(div, &[span, p], &dom), None);
}

#[test]
fn self_in_args_skipped() {
    // When `before` is called on a node that is also in the args,
    // viable_next_sibling should skip it.
    let (dom, _body, div, span, p, _session) = setup();
    assert_eq!(viable_next_sibling(div, &[span], &dom), Some(p));
}

// ---- viable_prev_sibling ----

#[test]
fn viable_prev_basic() {
    let (dom, _body, div, span, p, _session) = setup();
    // Previous of p, excluding nothing -> span.
    assert_eq!(viable_prev_sibling(p, &[], &dom), Some(span));
    // Previous of p, excluding span -> div.
    assert_eq!(viable_prev_sibling(p, &[span], &dom), Some(div));
    // Previous of p, excluding span and div -> None.
    assert_eq!(viable_prev_sibling(p, &[span, div], &dom), None);
    // Previous of div -> None (first child).
    assert_eq!(viable_prev_sibling(div, &[], &dom), None);
}

// ---- transient wrapper cleanup (no entity leak) ----

/// Count live `DocumentFragment` entities in the world.
fn count_document_fragments(dom: &EcsDom) -> usize {
    dom.world()
        .iter()
        .filter(|e| dom.is_document_fragment(e.entity()))
        .count()
}

#[test]
fn append_multiple_frees_transient_wrapper_fragment() {
    let (mut dom, _body, _div, span, _p, mut session) = setup();
    let a = dom.create_element("a", Attributes::default());
    let a_ref = session
        .get_or_create_wrapper(a, ComponentKind::Element)
        .to_raw();
    let b = dom.create_element("b", Attributes::default());
    let b_ref = session
        .get_or_create_wrapper(b, ComponentKind::Element)
        .to_raw();

    let frags_before = count_document_fragments(&dom);
    Append
        .invoke(
            span,
            &[JsValue::ObjectRef(a_ref), JsValue::ObjectRef(b_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap();

    // a, b are appended to span; the transient multi-arg wrapper DocumentFragment
    // that `convert_nodes_into_node` built must be destroyed once `apply_*` expanded
    // it (regression guard: the VM's `finalize_pair` cleanup was dropped in the
    // B1.2b convergence, so without `destroy_temp_fragment` every multi-arg call
    // would orphan a fragment entity).
    assert_eq!(dom.children(span), vec![a, b]);
    assert_eq!(
        count_document_fragments(&dom),
        frags_before,
        "transient wrapper DocumentFragment must be freed (no entity leak)"
    );
}

#[test]
fn replace_children_multiple_frees_transient_wrapper_fragment() {
    let (mut dom, _body, _div, span, _p, mut session) = setup();
    let a = dom.create_element("a", Attributes::default());
    let a_ref = session
        .get_or_create_wrapper(a, ComponentKind::Element)
        .to_raw();
    let b = dom.create_element("b", Attributes::default());
    let b_ref = session
        .get_or_create_wrapper(b, ComponentKind::Element)
        .to_raw();

    let frags_before = count_document_fragments(&dom);
    ReplaceChildren
        .invoke(
            span,
            &[JsValue::ObjectRef(a_ref), JsValue::ObjectRef(b_ref)],
            &mut session,
            &mut dom,
        )
        .unwrap();

    assert_eq!(dom.children(span), vec![a, b]);
    assert_eq!(
        count_document_fragments(&dom),
        frags_before,
        "replaceChildren transient wrapper DocumentFragment must be freed"
    );
}

// ---- shadow-root reference-child exposure ----

#[test]
fn prepend_on_shadow_host_does_not_leak_shadow_root_as_reference() {
    use elidex_ecs::ShadowRootMode;
    let (mut dom, _body, _div, _span, _p, mut session) = setup();
    // A shadow host with a shadow root but NO light children: `get_first_child`
    // returns the internal ShadowRoot, but `prepend` must use the first EXPOSED child
    // (none → append) so the ShadowRoot never becomes the reference child / leaks into
    // the record's `nextSibling`. (Codex PR393 R2.)
    let host = dom.create_element("div", Attributes::default());
    let _sr = dom
        .attach_shadow(host, ShadowRootMode::Open)
        .expect("attach_shadow");
    let x = dom.create_element("x", Attributes::default());
    let x_ref = session
        .get_or_create_wrapper(x, ComponentKind::Element)
        .to_raw();

    Prepend
        .invoke(host, &[JsValue::ObjectRef(x_ref)], &mut session, &mut dom)
        .unwrap();

    let records = session.take_notify_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].added_nodes, vec![x]);
    assert_eq!(
        records[0].next_sibling, None,
        "shadow root must not leak as the record's nextSibling"
    );
    assert_eq!(
        dom.children(host),
        vec![x],
        "x is the only (first) light child"
    );
}

#[test]
fn append_multiarg_rejects_shadow_root_arg_without_corruption() {
    use elidex_ecs::ShadowRootMode;
    let (mut dom, _body, _div, _span, _p, mut session) = setup();
    let host = dom.create_element("div", Attributes::default());
    let sr = dom
        .attach_shadow(host, ShadowRootMode::Open)
        .expect("attach_shadow");
    let target = dom.create_element("div", Attributes::default());
    let x = dom.create_element("x", Attributes::default());
    let x_ref = session
        .get_or_create_wrapper(x, ComponentKind::Element)
        .to_raw();
    let sr_ref = session
        .get_or_create_wrapper(sr, ComponentKind::DocumentFragment)
        .to_raw();

    // `target.append(x, host.shadowRoot)` — a multi-arg list with a ShadowRoot. The
    // shadow-root arg is rejected at `collect_nodes`, BEFORE `convert_nodes_into_node`
    // wraps the variadic args via the raw `EcsDom::append_child` that would reparent it
    // (the apply_* guard only sees the temp fragment, not its shadow-root child). So the
    // op fails atomically — no reparenting, no partial mutation. (Codex PR393 R6.)
    let result = Append.invoke(
        target,
        &[JsValue::ObjectRef(x_ref), JsValue::ObjectRef(sr_ref)],
        &mut session,
        &mut dom,
    );
    assert!(
        result.is_err(),
        "multi-arg append with a ShadowRoot must fail"
    );
    assert!(dom.is_shadow_root(sr), "shadow root still a shadow root");
    assert!(
        dom.children(target).is_empty(),
        "atomic: neither the shadow root nor x was inserted"
    );
}
