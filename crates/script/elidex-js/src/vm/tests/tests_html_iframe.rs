//! PR4f C8: `HTMLIFrameElement.prototype` + per-tag accessor surface.
//!
//! Verifies that `<iframe>` wrappers pick up the new tag-specific
//! prototype, its nine string-reflect attrs, the `allowFullscreen`
//! boolean reflect, and the `contentDocument` / `contentWindow` null
//! stubs (the last pair is the PR5d upgrade anchor — see
//! `vm/host/html_iframe_proto.rs` "PR5b CHECKLIST").

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_empty_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

fn run(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_empty_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let result = vm.eval(script).unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}")
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    out
}

// --- Prototype chain identity --------------------------------------

#[test]
fn iframe_wrapper_has_html_iframe_prototype() {
    let out = run("var iframe = document.createElement('iframe'); \
         var proto = Object.getPrototypeOf(iframe); \
         var anotherIframe = document.createElement('iframe'); \
         var same = Object.getPrototypeOf(anotherIframe) === proto; \
         var hasSrcGetter = Object.getOwnPropertyDescriptor(proto, 'src') !== undefined; \
         (same && hasSrcGetter) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn non_iframe_element_does_not_see_src_accessor() {
    // Per-tag install: `.src` must only appear on the iframe
    // prototype chain, not on a <div>'s.
    let out = run("var d = document.createElement('div'); \
         typeof d.src;");
    assert_eq!(out, "undefined");
}

// --- String reflect attrs ------------------------------------------

#[test]
fn iframe_src_getter_initial_is_empty_string() {
    let out = run("var iframe = document.createElement('iframe'); \
         typeof iframe.src + ':' + iframe.src;");
    assert_eq!(out, "string:");
}

#[test]
fn iframe_src_set_then_get_reflects_attribute() {
    let out = run("var iframe = document.createElement('iframe'); \
         iframe.src = 'https://example.com/x'; \
         iframe.src + '|' + iframe.getAttribute('src');");
    assert_eq!(out, "https://example.com/x|https://example.com/x");
}

#[test]
fn iframe_string_attrs_roundtrip() {
    // One assertion per attribute to keep the failure localised.
    let out = run("var iframe = document.createElement('iframe'); \
         iframe.srcdoc = 'A'; \
         iframe.name = 'B'; \
         iframe.referrerPolicy = 'no-referrer'; \
         iframe.allow = 'camera'; \
         iframe.loading = 'lazy'; \
         iframe.sandbox = 'allow-scripts'; \
         iframe.srcdoc + '|' + iframe.name + '|' + iframe.referrerPolicy \
           + '|' + iframe.allow + '|' + iframe.loading + '|' + iframe.sandbox;");
    assert_eq!(out, "A|B|no-referrer|camera|lazy|allow-scripts");
}

#[test]
fn iframe_width_height_preserve_non_numeric_round_trip() {
    // `width` / `height` are string-reflect (not long), so
    // unusual values ("100px") must survive verbatim.
    let out = run("var iframe = document.createElement('iframe'); \
         iframe.width = '100px'; \
         iframe.height = '50%'; \
         iframe.width + '|' + iframe.height;");
    assert_eq!(out, "100px|50%");
}

#[test]
fn iframe_referrer_policy_property_name_is_camel_case() {
    // The IDL property is camelCase; the HTML attribute lowercase.
    let out = run("var iframe = document.createElement('iframe'); \
         iframe.setAttribute('referrerpolicy', 'origin'); \
         iframe.referrerPolicy;");
    assert_eq!(out, "origin");
}

// --- allowFullscreen boolean reflect --------------------------------

#[test]
fn iframe_allow_fullscreen_initial_false() {
    let out = run("var iframe = document.createElement('iframe'); \
         typeof iframe.allowFullscreen + ':' + iframe.allowFullscreen;");
    assert_eq!(out, "boolean:false");
}

#[test]
fn iframe_allow_fullscreen_setter_toggles_attribute() {
    let out = run("var iframe = document.createElement('iframe'); \
         iframe.allowFullscreen = true; \
         var hasAfterSet = iframe.hasAttribute('allowfullscreen'); \
         iframe.allowFullscreen = false; \
         var hasAfterUnset = iframe.hasAttribute('allowfullscreen'); \
         hasAfterSet + '|' + hasAfterUnset;");
    assert_eq!(out, "true|false");
}

#[test]
fn iframe_allow_fullscreen_truthy_string_is_truthy() {
    // WHATWG: setter applies ToBoolean — "0" is truthy because it's
    // a non-empty string, despite being numerically zero.
    let out = run("var iframe = document.createElement('iframe'); \
         iframe.allowFullscreen = '0'; \
         String(iframe.allowFullscreen);");
    assert_eq!(out, "true");
}

// --- contentDocument / contentWindow null-stub lock-in (S5) ---------

#[test]
fn iframe_content_document_is_null_until_pr5d() {
    let out = run("var iframe = document.createElement('iframe'); \
         iframe.contentDocument === null ? 'null' : 'not-null';");
    assert_eq!(out, "null");
}

#[test]
fn iframe_content_window_is_null_until_pr5d() {
    let out = run("var iframe = document.createElement('iframe'); \
         iframe.contentWindow === null ? 'null' : 'not-null';");
    assert_eq!(out, "null");
}

// --- Brand check ----------------------------------------------------

#[test]
fn iframe_src_on_non_iframe_host_throws() {
    let out = run("var iframe = document.createElement('iframe'); \
         var getSrc = Object.getOwnPropertyDescriptor( \
             Object.getPrototypeOf(iframe), 'src').get; \
         var div = document.createElement('div'); \
         try { getSrc.call(div); 'no-throw'; } \
         catch (e) { (e && e.message && e.message.indexOf('Illegal') >= 0) ? 'ok' : 'bad'; }");
    assert_eq!(out, "ok");
}
