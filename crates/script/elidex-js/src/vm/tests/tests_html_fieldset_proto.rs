//! Slot `#11-tags-T1-v2` Phase 3 — `HTMLFieldSetElement.prototype`
//! coverage.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

fn run(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);
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

#[test]
fn fieldset_disabled_default_false() {
    let out = run("var f = document.createElement('fieldset'); '' + f.disabled;");
    assert_eq!(out, "false");
}

#[test]
fn fieldset_disabled_round_trip() {
    let out = run("var f = document.createElement('fieldset'); \
         f.disabled = true; \
         '' + f.hasAttribute('disabled');");
    assert_eq!(out, "true");
}

#[test]
fn fieldset_name_round_trip() {
    let out = run("var f = document.createElement('fieldset'); \
         f.name = 'group1'; \
         f.name + '/' + f.getAttribute('name');");
    assert_eq!(out, "group1/group1");
}

#[test]
fn fieldset_type_returns_fieldset() {
    let out = run("var f = document.createElement('fieldset'); f.type;");
    assert_eq!(out, "fieldset");
}

#[test]
fn fieldset_form_null_without_form_ancestor() {
    let out = run("var f = document.createElement('fieldset'); \
         (f.form === null) ? 'null' : 'non-null';");
    assert_eq!(out, "null");
}

#[test]
fn fieldset_form_resolves_via_form_ancestor() {
    let out = run("var f = document.createElement('form'); \
         var fs = document.createElement('fieldset'); \
         f.appendChild(fs); \
         (fs.form === f) ? 'same' : 'diff';");
    assert_eq!(out, "same");
}

#[test]
fn fieldset_elements_returns_collection() {
    let out = run("var fs = document.createElement('fieldset'); \
         (fs.elements != null && typeof fs.elements.length === 'number') ? 'ok' : 'bad';");
    assert_eq!(out, "ok");
}

#[test]
fn fieldset_brand_check_throws_on_non_fieldset_receiver() {
    let out = run("var d = document.createElement('div'); \
         var fs = document.createElement('fieldset'); \
         var getter = Object.getOwnPropertyDescriptor(\
             Object.getPrototypeOf(fs), 'name').get; \
         try { getter.call(d); 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'type' : 'other'; }");
    assert_eq!(out, "type");
}
