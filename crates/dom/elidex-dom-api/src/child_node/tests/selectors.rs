use super::setup;
use crate::child_node::{Closest, Matches};
use elidex_ecs::{Attributes, EcsDom};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiErrorKind, DomApiHandler, JsObjectRef, SessionCore};

// ---- matches ----

#[test]
fn matches_tag() {
    let (mut dom, _body, div, _span, _p, mut session) = setup();

    let handler = Matches;
    let result = handler
        .invoke(
            div,
            &[JsValue::String("div".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(true));
}

#[test]
fn matches_class() {
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("class", "active");
    let el = dom.create_element("div", attrs);
    let mut session = SessionCore::new();

    let handler = Matches;
    let result = handler
        .invoke(
            el,
            &[JsValue::String(".active".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(true));
}

#[test]
fn matches_no_match() {
    let (mut dom, _body, div, _span, _p, mut session) = setup();

    let handler = Matches;
    let result = handler
        .invoke(
            div,
            &[JsValue::String("span".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Bool(false));
}

#[test]
fn matches_invalid_selector() {
    let (mut dom, _body, div, _span, _p, mut session) = setup();

    let handler = Matches;
    let result = handler.invoke(
        div,
        &[JsValue::String(">>>".into())],
        &mut session,
        &mut dom,
    );
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind, DomApiErrorKind::SyntaxError);
}

// ---- closest ----

#[test]
fn closest_self() {
    let (mut dom, _body, div, _span, _p, mut session) = setup();

    let handler = Closest;
    let result = handler
        .invoke(
            div,
            &[JsValue::String("div".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    // Should return an ObjectRef for `div` itself.
    assert!(matches!(result, JsValue::ObjectRef(_)));
}

#[test]
fn closest_ancestor() {
    let (mut dom, body, _div, span, _p, mut session) = setup();

    let handler = Closest;
    let result = handler
        .invoke(
            span,
            &[JsValue::String("body".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    match result {
        JsValue::ObjectRef(id) => {
            let (entity, _) = session
                .identity_map()
                .get(JsObjectRef::from_raw(id))
                .unwrap();
            assert_eq!(entity, body);
        }
        _ => panic!("expected ObjectRef"),
    }
}

#[test]
fn closest_none() {
    let (mut dom, _body, div, _span, _p, mut session) = setup();

    let handler = Closest;
    let result = handler
        .invoke(
            div,
            &[JsValue::String("article".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert_eq!(result, JsValue::Null);
}

#[test]
fn closest_skips_text() {
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    let text = dom.create_text("hello");
    dom.append_child(div, text);
    let mut session = SessionCore::new();

    let handler = Closest;
    // Starting from text node, closest("div") should find the parent div.
    let result = handler
        .invoke(
            text,
            &[JsValue::String("div".into())],
            &mut session,
            &mut dom,
        )
        .unwrap();
    assert!(matches!(result, JsValue::ObjectRef(_)));
}
