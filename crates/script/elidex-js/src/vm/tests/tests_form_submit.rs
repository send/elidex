//! Slot `#11-form-submission-dispatcher` (D-29) — `form.submit()` /
//! `form.requestSubmit(submitter?)` coverage per WHATWG HTML
//! §4.10.21.3 + §4.10.21.4.
//!
//! Test catalogue (JS-observable surface):
//!
//! - E-1  `form.submit()` fires neither `submit` nor `formdata` event
//!        (HTML §4.10.21.3 step 5: from-submit-method=true).
//! - E-2  `form.requestSubmit()` (no submitter) fires `submit` event
//!        (`submitter === null`) then `formdata` event.
//! - E-3  `form.requestSubmit(button)` propagates the submitter into
//!        the `submit` event slot.
//! - E-4  `form.requestSubmit(otherFormButton)` throws `NotFoundError`
//!        DOMException (§4.10.21.4 step 2.2).
//! - E-5  Calling `preventDefault()` in the `submit` listener
//!        suppresses the subsequent `formdata` event.
//! - E-6  `FormDataEvent.formData` carries an entry list populated
//!        from the form's submittable controls (exercises the
//!        `name` IDL setter → `FormControlState.name` sync added in
//!        D-29 — see `html_input_proto::native_input_set_name`).
//! - E-7  `new FormData(formEl)` populates the entry list from the
//!        form's controls via the §C-3 single-source-of-truth path
//!        (existing FormData ctor TODO closed).
//!
//! Cross-tree submitter validation (`form="id"` attribute path) and
//! `FormControlKind::Button` rejection (non-submit `type=button`) are
//! covered at the Rust level in
//! `crates/dom/elidex-form/src/submit.rs::tests` (`is_form_owner_*`
//! and `is_submit_button_*`).  JS-level coverage would require either
//! the elidex `innerHTML` parser path to attach `FormControlState`
//! to parsed elements (currently a `<style>`-grade hook missing per
//! `vm/host/dom_inner_html.rs`) or an attribute-mutation observer
//! that re-runs `from_element` on `form`/`type` content-attribute
//! changes — both broader infrastructure work outside D-29 scope.

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

// E-1 — form.submit() fires no events per spec step 5.
#[test]
fn submit_does_not_fire_submit_or_formdata_events() {
    let out = run(
        "var f = document.createElement('form'); \
         var fired = []; \
         f.addEventListener('submit', function() { fired.push('submit'); }); \
         f.addEventListener('formdata', function() { fired.push('formdata'); }); \
         var r = f.submit(); \
         (r === undefined ? 'undef' : 'other') + '|' + fired.join(',');",
    );
    assert_eq!(out, "undef|");
}

// E-2 — requestSubmit() with no submitter fires submit (submitter=null)
// then formdata.
#[test]
fn request_submit_no_submitter_fires_submit_then_formdata() {
    let out = run(
        "var f = document.createElement('form'); \
         var trace = []; \
         f.addEventListener('submit', function(e) { \
             trace.push('submit/' + (e.submitter === null ? 'null' : 'other')); \
         }); \
         f.addEventListener('formdata', function(e) { \
             trace.push('formdata/' + (e.formData instanceof FormData ? 'fd' : 'other')); \
         }); \
         var r = f.requestSubmit(); \
         (r === undefined ? 'undef' : 'other') + '|' + trace.join(',');",
    );
    assert_eq!(out, "undef|submit/null,formdata/fd");
}

// E-3 — requestSubmit(button) propagates the submitter into the
// submit event.  `<button>` created via `createElement` defaults to
// `type=submit` per HTML §4.10.6 (FormControlKind::SubmitButton),
// no explicit type setter needed.
#[test]
fn request_submit_with_valid_submitter_propagates_to_event() {
    let out = run(
        "var f = document.createElement('form'); \
         var b = document.createElement('button'); \
         f.appendChild(b); \
         var got = ''; \
         f.addEventListener('submit', function(e) { \
             got = (e.submitter === b) ? 'same' : 'other'; \
         }); \
         f.requestSubmit(b); \
         got;",
    );
    assert_eq!(out, "same");
}

// E-4 — requestSubmit(otherFormButton) → NotFoundError per
// §4.10.21.4 step 2.2.  Tree-ancestor of `btn` is form `b`, not `a`,
// and the button has no `form` IDREF — both ownership paths fail.
#[test]
fn request_submit_wrong_form_throws_not_found_error() {
    let out = run(
        "var a = document.createElement('form'); \
         var b = document.createElement('form'); \
         var btn = document.createElement('button'); \
         b.appendChild(btn); \
         try { a.requestSubmit(btn); 'no-throw'; } \
         catch (e) { e.name === 'NotFoundError' ? 'ok' : ('other:' + e.name); }",
    );
    assert_eq!(out, "ok");
}

// E-5 — preventDefault() in submit listener suppresses formdata event.
#[test]
fn request_submit_prevent_default_suppresses_formdata() {
    let out = run(
        "var f = document.createElement('form'); \
         var fired = []; \
         f.addEventListener('submit', function(e) { \
             fired.push('submit'); \
             e.preventDefault(); \
         }); \
         f.addEventListener('formdata', function() { fired.push('formdata'); }); \
         var r = f.requestSubmit(); \
         (r === undefined ? 'undef' : 'other') + '|' + fired.join(',');",
    );
    assert_eq!(out, "undef|submit");
}

// E-6 — FormDataEvent.formData entries populated from form controls.
// Exercises the `input.name` IDL setter → `FormControlState.name`
// sync added in D-29 (`html_input_proto::native_input_set_name`);
// without it `collect_form_data` would see `fcs.name == ""` and
// silently drop the entry.
#[test]
fn formdata_event_carries_populated_entries() {
    let out = run(
        "var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         i.name = 'q'; \
         i.value = 'hello'; \
         f.appendChild(i); \
         var entries = []; \
         f.addEventListener('formdata', function(e) { \
             e.formData.forEach(function(v, k) { entries.push(k + '=' + v); }); \
         }); \
         f.requestSubmit(); \
         entries.join('&');",
    );
    assert_eq!(out, "q=hello");
}

// E-7 — `new FormData(formEl)` populates entries from form controls.
// Validates the §C-3 single-source-of-truth path that requestSubmit
// reuses (existing `form_data.rs:203-207` TODO closed).
#[test]
fn new_form_data_with_form_populates_entries() {
    let out = run(
        "var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         i.name = 'q'; \
         i.value = 'world'; \
         f.appendChild(i); \
         var fd = new FormData(f); \
         var entries = []; \
         fd.forEach(function(v, k) { entries.push(k + '=' + v); }); \
         entries.join('&');",
    );
    assert_eq!(out, "q=world");
}
