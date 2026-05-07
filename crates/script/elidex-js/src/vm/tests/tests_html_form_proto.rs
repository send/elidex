//! Slot `#11-tags-T1-v2` Phase 4 — `HTMLFormElement.prototype`
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
fn form_action_round_trip() {
    let out = run("var f = document.createElement('form'); \
         f.action = '/submit'; \
         f.action + '/' + f.getAttribute('action');");
    assert_eq!(out, "/submit//submit");
}

#[test]
fn form_method_round_trip() {
    let out = run("var f = document.createElement('form'); \
         f.method = 'post'; \
         f.method;");
    assert_eq!(out, "post");
}

#[test]
fn form_name_round_trip() {
    let out = run("var f = document.createElement('form'); \
         f.name = 'my-form'; \
         f.name;");
    assert_eq!(out, "my-form");
}

#[test]
fn form_enctype_round_trip() {
    let out = run("var f = document.createElement('form'); \
         f.enctype = 'multipart/form-data'; \
         f.enctype;");
    assert_eq!(out, "multipart/form-data");
}

#[test]
fn form_encoding_aliases_enctype() {
    let out = run("var f = document.createElement('form'); \
         f.encoding = 'multipart/form-data'; \
         f.enctype + '/' + f.encoding;");
    assert_eq!(out, "multipart/form-data/multipart/form-data");
}

#[test]
fn form_no_validate_default_false() {
    let out = run("var f = document.createElement('form'); '' + f.noValidate;");
    assert_eq!(out, "false");
}

#[test]
fn form_no_validate_round_trip() {
    let out = run("var f = document.createElement('form'); \
         f.noValidate = true; \
         '' + f.hasAttribute('novalidate');");
    assert_eq!(out, "true");
}

#[test]
fn form_target_default_empty() {
    let out = run("var f = document.createElement('form'); f.target;");
    assert_eq!(out, "");
}

#[test]
fn form_accept_charset_round_trip() {
    let out = run("var f = document.createElement('form'); \
         f.acceptCharset = 'UTF-8'; \
         f.acceptCharset + '/' + f.getAttribute('accept-charset');");
    assert_eq!(out, "UTF-8/UTF-8");
}

#[test]
fn form_rel_round_trip() {
    let out = run("var f = document.createElement('form'); \
         f.rel = 'noopener'; \
         f.rel;");
    assert_eq!(out, "noopener");
}

#[test]
fn form_length_returns_zero_phase4_stub() {
    let out = run("var f = document.createElement('form'); '' + f.length;");
    assert_eq!(out, "0");
}

#[test]
fn form_elements_returns_collection() {
    let out = run("var f = document.createElement('form'); \
         (f.elements != null && typeof f.elements.length === 'number') ? 'ok' : 'bad';");
    assert_eq!(out, "ok");
}

#[test]
fn form_elements_is_same_object() {
    // F5 regression — WebIDL `[SameObject]`: successive reads of
    // `form.elements` return the same wrapper id.
    let out = run("var f = document.createElement('form'); \
         (f.elements === f.elements) ? 'same' : 'diff';");
    assert_eq!(out, "same");
}

#[test]
fn form_submit_throws_not_supported_error() {
    let out = run("var f = document.createElement('form'); \
         try { f.submit(); 'no-throw'; } \
         catch (e) { (e.name === 'NotSupportedError') ? 'ok' : ('other:' + e.name); }");
    assert_eq!(out, "ok");
}

#[test]
fn form_request_submit_throws_not_supported_error() {
    let out = run("var f = document.createElement('form'); \
         try { f.requestSubmit(); 'no-throw'; } \
         catch (e) { (e.name === 'NotSupportedError') ? 'ok' : ('other:' + e.name); }");
    assert_eq!(out, "ok");
}

#[test]
fn form_reset_returns_undefined_no_op_when_empty() {
    let out = run("var f = document.createElement('form'); \
         var r = f.reset(); \
         (r === undefined) ? 'undef' : 'other';");
    assert_eq!(out, "undef");
}

#[test]
fn form_reset_dispatches_reset_event_and_resets_state() {
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         f.appendChild(i); \
         i.defaultValue = 'init'; \
         i.value = 'typed'; \
         var fired = ''; \
         f.addEventListener('reset', function(e) { \
            fired = e.type + '/' + e.bubbles + '/' + e.cancelable; \
         }); \
         f.reset(); \
         fired + '|' + i.value;");
    assert_eq!(out, "reset/true/true|init");
}

#[test]
fn form_reset_event_cancellable_via_prevent_default() {
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         f.appendChild(i); \
         i.defaultValue = 'init'; \
         i.value = 'typed'; \
         f.addEventListener('reset', function(e) { e.preventDefault(); }); \
         f.reset(); \
         i.value;");
    assert_eq!(out, "typed");
}

#[test]
fn form_check_validity_returns_true_when_all_controls_valid() {
    // R2 F2 regression — HTML §4.10.20.4 form.checkValidity()
    // returns true when every listed element is a valid candidate.
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         f.appendChild(i); \
         '' + f.checkValidity();");
    assert_eq!(out, "true");
}

#[test]
fn form_check_validity_returns_false_when_a_control_is_invalid() {
    // R2 F2 regression — any failing constraint flips the form's
    // checkValidity to false.
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         i.required = true; \
         f.appendChild(i); \
         '' + f.checkValidity();");
    assert_eq!(out, "false");
}

#[test]
fn form_check_validity_fires_invalid_on_each_invalid_control() {
    // R2 F2 regression — per-control `invalid` event fires for
    // every invalid candidate, even after the first failure
    // (HTML §4.10.20.4 step 2).
    let out = run("var f = document.createElement('form'); \
         var i1 = document.createElement('input'); i1.required = true; \
         var i2 = document.createElement('input'); i2.required = true; \
         f.appendChild(i1); f.appendChild(i2); \
         var fired = 0; \
         i1.addEventListener('invalid', function() { fired++; }); \
         i2.addEventListener('invalid', function() { fired++; }); \
         var v = f.checkValidity(); \
         '' + v + '/' + fired;");
    assert_eq!(out, "false/2");
}

#[test]
fn form_report_validity_aliases_check_validity_in_headless_mode() {
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); i.required = true; \
         f.appendChild(i); \
         '' + f.reportValidity() + '/' + f.checkValidity();");
    assert_eq!(out, "false/false");
}

#[test]
fn form_check_validity_skips_controls_in_disabled_fieldset() {
    // R7 F2 regression — controls inside a disabled `<fieldset>`
    // are barred from constraint validation per HTML §4.10.20.3.
    // The form-level checkValidity must skip them: `invalid` event
    // does not fire and the form aggregates as valid.
    let out = run("var f = document.createElement('form'); \
         var fs = document.createElement('fieldset'); fs.disabled = true; \
         var i = document.createElement('input'); i.required = true; \
         fs.appendChild(i); f.appendChild(fs); \
         var fired = false; \
         i.addEventListener('invalid', function() { fired = true; }); \
         var v = f.checkValidity(); \
         '' + v + '/' + fired;");
    assert_eq!(out, "true/false");
}

#[test]
fn form_check_validity_skips_disabled_controls() {
    // HTML §4.10.20.3 — disabled controls are barred from
    // constraint validation; the form's check must skip them.
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         i.required = true; i.disabled = true; \
         f.appendChild(i); \
         '' + f.checkValidity();");
    assert_eq!(out, "true");
}

#[test]
fn form_brand_check_throws_on_non_form_receiver() {
    let out = run("var d = document.createElement('div'); \
         var f = document.createElement('form'); \
         var getter = Object.getOwnPropertyDescriptor(\
             Object.getPrototypeOf(f), 'action').get; \
         try { getter.call(d); 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'type' : 'other'; }");
    assert_eq!(out, "type");
}
