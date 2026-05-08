//! Slot `#11-tags-T1-v2` Phase 5 — `HTMLButtonElement.prototype`
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
fn button_type_default_submit() {
    let out = run("var b = document.createElement('button'); b.type;");
    assert_eq!(out, "submit");
}

#[test]
fn button_type_reset_round_trip() {
    let out = run("var b = document.createElement('button'); \
         b.type = 'reset'; \
         b.type;");
    assert_eq!(out, "reset");
}

#[test]
fn button_type_invalid_falls_back_to_submit() {
    let out = run("var b = document.createElement('button'); \
         b.type = 'whatever'; \
         b.type;");
    assert_eq!(out, "submit");
}

#[test]
fn button_type_button_round_trip() {
    let out = run("var b = document.createElement('button'); \
         b.type = 'button'; \
         b.type;");
    assert_eq!(out, "button");
}

#[test]
fn button_disabled_round_trip() {
    let out = run("var b = document.createElement('button'); \
         b.disabled = true; \
         '' + b.hasAttribute('disabled');");
    assert_eq!(out, "true");
}

#[test]
fn button_name_round_trip() {
    let out = run("var b = document.createElement('button'); \
         b.name = 'btn1'; \
         b.name;");
    assert_eq!(out, "btn1");
}

#[test]
fn button_value_round_trip() {
    let out = run("var b = document.createElement('button'); \
         b.value = 'send'; \
         b.value;");
    assert_eq!(out, "send");
}

#[test]
fn button_form_action_round_trip() {
    let out = run("var b = document.createElement('button'); \
         b.formAction = '/x'; \
         b.formAction + '/' + b.getAttribute('formaction');");
    assert_eq!(out, "/x//x");
}

#[test]
fn button_form_no_validate_round_trip() {
    let out = run("var b = document.createElement('button'); \
         b.formNoValidate = true; \
         '' + b.hasAttribute('formnovalidate');");
    assert_eq!(out, "true");
}

#[test]
fn button_autofocus_round_trip() {
    let out = run("var b = document.createElement('button'); \
         b.autofocus = true; \
         '' + b.hasAttribute('autofocus');");
    assert_eq!(out, "true");
}

#[test]
fn button_form_null_without_form_ancestor() {
    let out = run("var b = document.createElement('button'); \
         (b.form === null) ? 'null' : 'non-null';");
    assert_eq!(out, "null");
}

#[test]
fn button_form_resolves_via_form_ancestor() {
    let out = run("var f = document.createElement('form'); \
         var b = document.createElement('button'); \
         f.appendChild(b); \
         (b.form === f) ? 'same' : 'diff';");
    assert_eq!(out, "same");
}

#[test]
fn button_labels_returns_collection() {
    let out = run("var b = document.createElement('button'); \
         (b.labels != null && typeof b.labels.length === 'number') ? 'ok' : 'bad';");
    assert_eq!(out, "ok");
}

#[test]
fn button_brand_check_throws_on_non_button_receiver() {
    let out = run("var d = document.createElement('div'); \
         var b = document.createElement('button'); \
         var getter = Object.getOwnPropertyDescriptor(\
             Object.getPrototypeOf(b), 'value').get; \
         try { getter.call(d); 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'type' : 'other'; }");
    assert_eq!(out, "type");
}

// `<button>.formMethod` enumerated reflection (HTML §4.10.5.4) —
// missing- and invalid-value defaults are both `""` (the no-override
// sentinel), distinct from `<form>.method` whose default is `"get"`.
#[test]
fn button_form_method_default_when_missing_is_empty_string() {
    let out = run("var b = document.createElement('button'); b.formMethod;");
    assert_eq!(out, "");
}

#[test]
fn button_form_method_canonicalises_uppercase_to_lowercase() {
    let out = run("var b = document.createElement('button'); \
         b.setAttribute('formmethod', 'POST'); b.formMethod;");
    assert_eq!(out, "post");
}

#[test]
fn button_form_method_invalid_falls_back_to_empty_string() {
    let out = run("var b = document.createElement('button'); \
         b.setAttribute('formmethod', 'bogus'); b.formMethod;");
    assert_eq!(out, "");
}

#[test]
fn button_form_enctype_default_when_missing_is_empty_string() {
    let out = run("var b = document.createElement('button'); b.formEnctype;");
    assert_eq!(out, "");
}

#[test]
fn button_form_enctype_canonicalises_multipart() {
    let out = run("var b = document.createElement('button'); \
         b.setAttribute('formenctype', 'MULTIPART/FORM-DATA'); b.formEnctype;");
    assert_eq!(out, "multipart/form-data");
}

#[test]
fn button_form_enctype_invalid_falls_back_to_empty_string() {
    let out = run("var b = document.createElement('button'); \
         b.setAttribute('formenctype', 'application/json'); b.formEnctype;");
    assert_eq!(out, "");
}
