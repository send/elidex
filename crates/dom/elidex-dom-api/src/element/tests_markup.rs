//! Handler-direct tests for the HTML §8.5 parsing and serialization surface
//! (`GetInnerHtml` and the `serialize_*` functions in `markup.rs`). Split out
//! of `tests_tree.rs` together with the code they test (`element/markup.rs`).

use super::*;
use elidex_ecs::{Attributes, EcsDom, ShadowRootMode};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiHandler, SessionCore};

use super::test_support::setup;

#[test]
fn inner_html_serialization() {
    let (mut dom, parent, _, mut session) = setup();
    let text = dom.create_text("hello <world>");
    dom.append_child(parent, text);

    let handler = GetInnerHtml;
    let result = handler.invoke(parent, &[], &mut session, &mut dom).unwrap();
    assert_eq!(result, JsValue::String("hello &lt;world&gt;".into()));
}

#[test]
fn inner_html_void_elements() {
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    let br = dom.create_element("br", Attributes::default());
    let mut img_attrs = Attributes::default();
    img_attrs.set("src", "test.png");
    let img = dom.create_element("img", img_attrs);
    dom.append_child(div, br);
    dom.append_child(div, img);

    let mut session = SessionCore::new();
    let handler = GetInnerHtml;
    let result = handler.invoke(div, &[], &mut session, &mut dom).unwrap();
    assert_eq!(result, JsValue::String("<br><img src=\"test.png\">".into()));
}

#[test]
fn inner_html_preserves_inter_element_whitespace() {
    // §11.3 whitespace unify: with the tolerant parser retaining inter-element
    // whitespace text nodes, serialization must round-trip them — a
    // whitespace-only text child is emitted verbatim, not dropped (outerHTML /
    // innerHTML fidelity for indented markup).
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    let p1 = dom.create_element("p", Attributes::default());
    let a = dom.create_text("A");
    dom.append_child(p1, a);
    let ws = dom.create_text("\n  ");
    let p2 = dom.create_element("p", Attributes::default());
    let b = dom.create_text("B");
    dom.append_child(p2, b);
    dom.append_child(div, p1);
    dom.append_child(div, ws);
    dom.append_child(div, p2);

    let mut session = SessionCore::new();
    let result = GetInnerHtml
        .invoke(div, &[], &mut session, &mut dom)
        .unwrap();
    assert_eq!(
        result,
        JsValue::String("<p>A</p>\n  <p>B</p>".into()),
        "inter-element whitespace must round-trip through innerHTML serialization"
    );
}

#[test]
fn inner_html_nested() {
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    let mut p_attrs = Attributes::default();
    p_attrs.set("class", "intro");
    let p = dom.create_element("p", p_attrs);
    let text = dom.create_text("hi");
    dom.append_child(div, p);
    dom.append_child(p, text);

    let mut session = SessionCore::new();
    let handler = GetInnerHtml;
    let result = handler.invoke(div, &[], &mut session, &mut dom).unwrap();
    assert_eq!(result, JsValue::String("<p class=\"intro\">hi</p>".into()));
}

#[test]
fn serialize_inner_html_excludes_shadow_root_subtree() {
    // WHATWG DOM §4.8 + HTML §2.7.3: `host.innerHTML` MUST NOT leak
    // shadow content (encapsulation).  Without the `ShadowRoot` skip
    // in `serialize_node`, the shadow root child entity (which carries
    // no TagType) would recurse into its own children + serialize them
    // as if they belonged to the host's light DOM — breaching
    // encapsulation for both open and closed shadows.  This regression
    // test locks the skip introduced in PR `#11-shadow-dom-surface` (D-15).
    let mut dom = EcsDom::new();
    let host = dom.create_element("div", Attributes::default());
    // Light-DOM child + shadow root with its own content.
    let light = dom.create_element("span", Attributes::default());
    assert!(dom.append_child(host, light));
    let shadow = dom
        .attach_shadow(host, ShadowRootMode::Open)
        .expect("attach_shadow on <div> succeeds");
    let shadow_child = dom.create_element("b", Attributes::default());
    assert!(dom.append_child(shadow, shadow_child));

    let html = serialize_inner_html(host, &dom);
    assert!(
        html.contains("<span>"),
        "light DOM child must serialize: {html}"
    );
    assert!(
        !html.contains("<b>"),
        "shadow root content must NOT serialize: {html}"
    );
}

// ---------------------------------------------------------------------------
// HTML §13.3 "Serializing HTML fragments" — is-value compensation step
// ---------------------------------------------------------------------------

#[test]
fn serialize_emits_is_for_customized_builtin_without_attr() {
    // createElement(tag, {is}) sets no `is` content attribute (DOM
    // §4.5); the serializer must append ` is="..."` from the
    // CustomElementState (the is-value slot) so the markup
    // round-trips. The round-trip itself: element_init re-derives the
    // same component from the emitted attribute.
    let mut dom = EcsDom::new();
    let button = dom.create_element("button", Attributes::default());
    dom.world_mut()
        .insert_one(
            button,
            elidex_custom_elements::CustomElementState::for_created_element(
                "button",
                Some("my-btn"),
                elidex_ecs::Namespace::Html,
            )
            .unwrap(),
        )
        .unwrap();
    let html = serialize_outer_html(button, &dom);
    assert_eq!(html, r#"<button is="my-btn"></button>"#);
}

#[test]
fn serialize_no_double_is_when_attr_present() {
    // Author markup `<button is="my-btn">` carries the attribute in
    // Attributes — the compensation step must not emit a second copy.
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("is", "my-btn");
    let button = dom.create_element("button", attrs);
    dom.world_mut()
        .insert_one(
            button,
            elidex_custom_elements::CustomElementState::undefined("my-btn"),
        )
        .unwrap();
    let html = serialize_outer_html(button, &dom);
    assert_eq!(html.matches("is=").count(), 1);
}

#[test]
fn serialize_no_is_for_autonomous_custom_element() {
    // Autonomous CE: definition_name == local name — nothing to
    // compensate (the tag itself carries the identity).
    let mut dom = EcsDom::new();
    let el = dom.create_element("my-widget", Attributes::default());
    dom.world_mut()
        .insert_one(
            el,
            elidex_custom_elements::CustomElementState::undefined("my-widget"),
        )
        .unwrap();
    let html = serialize_outer_html(el, &dom);
    assert_eq!(html, "<my-widget></my-widget>");
}

#[test]
fn serialize_escapes_synthetic_is_value() {
    // The is value is an arbitrary author string (step 6.3 imposes no
    // validity) — raw emission would inject markup.
    let mut dom = EcsDom::new();
    let button = dom.create_element("button", Attributes::default());
    dom.world_mut()
        .insert_one(
            button,
            elidex_custom_elements::CustomElementState::for_created_element(
                "button",
                Some(r#"x" onclick="evil"#),
                elidex_ecs::Namespace::Html,
            )
            .unwrap(),
        )
        .unwrap();
    let html = serialize_outer_html(button, &dom);
    assert!(
        html.contains("is=\"x&quot; onclick=&quot;evil\""),
        "synthetic is value must route through escape_attr: {html}"
    );
}

#[test]
fn serialize_emits_is_for_autonomous_element_with_is_value() {
    // Codex PR331 R1: `createElement('my-el', {is: 'my-other'})` — the
    // autonomous branch keys the definition on the tag but the
    // non-null is value lives in its own slot and must serialize
    // (DOM §4.9 sets the is value independently of step 6.3).
    let mut dom = EcsDom::new();
    let el = dom.create_element("my-el", Attributes::default());
    dom.world_mut()
        .insert_one(
            el,
            elidex_custom_elements::CustomElementState::for_created_element(
                "my-el",
                Some("my-other"),
                elidex_ecs::Namespace::Html,
            )
            .unwrap(),
        )
        .unwrap();
    let html = serialize_outer_html(el, &dom);
    assert_eq!(html, r#"<my-el is="my-other"></my-el>"#);
}

#[test]
fn serialize_emits_is_equal_to_local_name() {
    // Codex PR331 R2: an is value equal to the local name is still
    // non-null — HTML §13.3's condition is nullness, not inequality.
    let mut dom = EcsDom::new();
    let el = dom.create_element("button", Attributes::default());
    dom.world_mut()
        .insert_one(
            el,
            elidex_custom_elements::CustomElementState::for_created_element(
                "button",
                Some("button"),
                elidex_ecs::Namespace::Html,
            )
            .unwrap(),
        )
        .unwrap();
    let html = serialize_outer_html(el, &dom);
    assert_eq!(html, r#"<button is="button"></button>"#);
}
