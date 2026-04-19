//! PR4f C6: Document simple accessors — `title` setter +
//! `compatMode` / `defaultView` / `doctype`.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_html_with_head_body(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let head = dom.create_element("head", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, head));
    assert!(dom.append_child(html, body));
    doc
}

fn build_no_head(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    assert!(dom.append_child(doc, html));
    doc
}

fn build_doctype_fixture(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let doctype = dom.create_document_type("html", "", "");
    let html = dom.create_element("html", Attributes::default());
    assert!(dom.append_child(doc, doctype));
    assert!(dom.append_child(doc, html));
    doc
}

fn run(script: &str, fixture: impl FnOnce(&mut EcsDom) -> elidex_ecs::Entity) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let result = vm.eval(script).unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}");
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    out
}

#[test]
fn title_setter_creates_title_when_absent() {
    let out = run(
        "document.title = 'Hello';\
         document.head.firstChild.tagName + ':' + document.title;",
        build_html_with_head_body,
    );
    assert_eq!(out, "TITLE:Hello");
}

#[test]
fn title_setter_replaces_existing_title_text() {
    let out = run(
        "var t = document.createElement('title');\
         t.appendChild(document.createTextNode('old'));\
         document.head.appendChild(t);\
         document.title = 'new';\
         document.title + '|' + document.head.getElementsByTagName('title').length;",
        build_html_with_head_body,
    );
    assert_eq!(out, "new|1");
}

#[test]
fn title_setter_noop_without_head() {
    // Per spec: if the document has no <head>, the setter is a no-op.
    let out = run(
        "document.title = 'x'; \
         var headChildren = document.documentElement.getElementsByTagName('head').length; \
         document.title + ':' + headChildren;",
        build_no_head,
    );
    assert_eq!(out, ":0");
}

#[test]
fn title_setter_coerces_to_string() {
    let out = run(
        "document.title = 42;\
         document.title;",
        build_html_with_head_body,
    );
    assert_eq!(out, "42");
}

#[test]
fn compat_mode_is_css1_compat() {
    let out = run("document.compatMode;", build_html_with_head_body);
    assert_eq!(out, "CSS1Compat");
}

#[test]
fn compat_mode_on_plain_object_returns_empty_not_css1_compat() {
    // Copilot R2 F5 lock-in: `compatMode` used to return the cached
    // "CSS1Compat" string even when called on a non-HostObject
    // receiver (e.g. via `Object.getOwnPropertyDescriptor(...).get.call({})`).
    // The unbound-receiver policy for Document accessors is to fall
    // through silently with the kind's default — empty string here —
    // so the getter does not leak a plausible answer for an
    // invalid receiver.
    let out = run(
        "var getCompat = Object.getOwnPropertyDescriptor(document, 'compatMode').get; \
         getCompat.call({}) + ':' + typeof getCompat.call({});",
        build_html_with_head_body,
    );
    assert_eq!(out, ":string");
}

#[test]
fn default_view_equals_global_this() {
    let out = run(
        "document.defaultView === globalThis ? 'same' : 'diff';",
        build_html_with_head_body,
    );
    assert_eq!(out, "same");
}

#[test]
fn default_view_is_null_for_detached_cloned_document() {
    // WHATWG: a Document without a browsing context (e.g. a clone)
    // has no `defaultView`.
    let out = run(
        "var cloneDoc = document.cloneNode(true);\
         cloneDoc.defaultView === null ? 'null' : 'not-null';",
        build_html_with_head_body,
    );
    assert_eq!(out, "null");
}

#[test]
fn doctype_returns_document_type_child() {
    // PR4f C6 verifies the accessor returns a non-null wrapper with the
    // correct nodeType (10 = DOCUMENT_TYPE_NODE). The DocumentType-specific
    // `name` / `publicId` / `systemId` members land with
    // `DocumentType.prototype` in C7.
    let out = run(
        "var dt = document.doctype; \
         dt === null ? 'missing' : String(dt.nodeType);",
        build_doctype_fixture,
    );
    assert_eq!(out, "10");
}

#[test]
fn doctype_is_null_when_absent() {
    let out = run(
        "document.doctype === null ? 'null' : 'present';",
        build_html_with_head_body,
    );
    assert_eq!(out, "null");
}
