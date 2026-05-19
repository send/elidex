//! Slot `#11-form-submission-dispatcher` (D-29) — `form.submit()` /
//! `form.requestSubmit(submitter?)` coverage per WHATWG HTML
//! §4.10.21.3 + §4.10.21.4.
//!
//! Test catalogue (JS-observable surface):
//!
//! - E-1 `form.submit()` fires neither `submit` nor `formdata` event
//!   (HTML §4.10.21.3 step 5: from-submit-method=true).
//! - E-2 `form.requestSubmit()` (no submitter) fires `submit` event
//!   (`submitter === null`) then `formdata` event.
//! - E-3 `form.requestSubmit(button)` propagates the submitter into
//!   the `submit` event slot.
//! - E-4 `form.requestSubmit(otherFormButton)` throws `NotFoundError`
//!   DOMException (§4.10.21.4 step 2.2).
//! - E-5 Calling `preventDefault()` in the `submit` listener
//!   suppresses the subsequent `formdata` event.
//! - E-6 `FormDataEvent.formData` carries an entry list populated
//!   from the form's submittable controls (exercises the `name` IDL
//!   setter → content-attribute reflection →
//!   `MutationEvent::AttributeChange` → `FormControlReconciler`
//!   sync chain landed in PR #208 D-31 successor).
//! - E-7 `new FormData(formEl)` populates the entry list from the
//!   form's controls via the §C-3 single-source-of-truth path
//!   (existing FormData ctor TODO closed).
//! - E-8 `form.requestSubmit(divEl)` with a non-submit-button
//!   wrapper throws `TypeError` per HTML §4.10.21.4 step 2.1.
//! - E-9 Interactive validation arm (§4.10.21.3 substep): a form
//!   with a single `<input required>` empty control fires `invalid`
//!   on the control and aborts before `submit` / `formdata`.
//! - E-10 `form.noValidate = true` (and `submitter.formNoValidate`)
//!   skip the interactive-validation substep so `submit` /
//!   `formdata` fire even when constraints would otherwise fail.
//!
//! Cross-tree submitter validation (`form="id"` attribute path) and
//! `FormControlKind::Button` rejection (non-submit `type=button`) are
//! covered at the Rust level in
//! `crates/dom/elidex-form/src/submit.rs::tests` (`is_form_owner_*`
//! and `is_submit_button_*`).  JS-level coverage of the parser-path
//! `form="id"` attribute scenario is now structurally feasible
//! post-#208 (`FormControlReconciler::handle_insert` attaches
//! `FormControlState` to parser-created form-control entities), but
//! is deferred to slot `#11-form-navigation` (D-33 paired sweep —
//! registered in the D-29 landing memo Defer ledger) so this PR's
//! scope stays on the `submit()` / `requestSubmit()` / `formdata`
//! dispatcher surface.

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
    let out = run("var f = document.createElement('form'); \
         var fired = []; \
         f.addEventListener('submit', function() { fired.push('submit'); }); \
         f.addEventListener('formdata', function() { fired.push('formdata'); }); \
         var r = f.submit(); \
         (r === undefined ? 'undef' : 'other') + '|' + fired.join(',');");
    assert_eq!(out, "undef|");
}

// E-2 — requestSubmit() with no submitter fires submit (submitter=null)
// then formdata.
#[test]
fn request_submit_no_submitter_fires_submit_then_formdata() {
    let out = run("var f = document.createElement('form'); \
         var trace = []; \
         f.addEventListener('submit', function(e) { \
             trace.push('submit/' + (e.submitter === null ? 'null' : 'other')); \
         }); \
         f.addEventListener('formdata', function(e) { \
             trace.push('formdata/' + (e.formData instanceof FormData ? 'fd' : 'other')); \
         }); \
         var r = f.requestSubmit(); \
         (r === undefined ? 'undef' : 'other') + '|' + trace.join(',');");
    assert_eq!(out, "undef|submit/null,formdata/fd");
}

// E-3 — requestSubmit(button) propagates the submitter into the
// submit event.  `<button>` created via `createElement` defaults to
// `type=submit` per HTML §4.10.6 (FormControlKind::SubmitButton),
// no explicit type setter needed.
#[test]
fn request_submit_with_valid_submitter_propagates_to_event() {
    let out = run("var f = document.createElement('form'); \
         var b = document.createElement('button'); \
         f.appendChild(b); \
         var got = ''; \
         f.addEventListener('submit', function(e) { \
             got = (e.submitter === b) ? 'same' : 'other'; \
         }); \
         f.requestSubmit(b); \
         got;");
    assert_eq!(out, "same");
}

// E-4 — requestSubmit(otherFormButton) → NotFoundError per
// §4.10.21.4 step 2.2.  Tree-ancestor of `btn` is form `b`, not `a`,
// and the button has no `form` IDREF — both ownership paths fail.
#[test]
fn request_submit_wrong_form_throws_not_found_error() {
    let out = run("var a = document.createElement('form'); \
         var b = document.createElement('form'); \
         var btn = document.createElement('button'); \
         b.appendChild(btn); \
         try { a.requestSubmit(btn); 'no-throw'; } \
         catch (e) { e.name === 'NotFoundError' ? 'ok' : ('other:' + e.name); }");
    assert_eq!(out, "ok");
}

// E-5 — preventDefault() in submit listener suppresses formdata event.
#[test]
fn request_submit_prevent_default_suppresses_formdata() {
    let out = run("var f = document.createElement('form'); \
         var fired = []; \
         f.addEventListener('submit', function(e) { \
             fired.push('submit'); \
             e.preventDefault(); \
         }); \
         f.addEventListener('formdata', function() { fired.push('formdata'); }); \
         var r = f.requestSubmit(); \
         (r === undefined ? 'undef' : 'other') + '|' + fired.join(',');");
    assert_eq!(out, "undef|submit");
}

// E-6 — FormDataEvent.formData entries populated from form controls.
// Exercises the post-PR #208 sync chain `input.name = "..."` →
// content-attribute reflection (`set_attribute("name", …)`) →
// `MutationEvent::AttributeChange` → `FormControlReconciler` →
// `FormControlState.name` update.  Without the reconciler path,
// `collect_form_data` would see `fcs.name == ""` and silently drop
// the entry.
#[test]
fn formdata_event_carries_populated_entries() {
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         i.name = 'q'; \
         i.value = 'hello'; \
         f.appendChild(i); \
         var entries = []; \
         f.addEventListener('formdata', function(e) { \
             e.formData.forEach(function(v, k) { entries.push(k + '=' + v); }); \
         }); \
         f.requestSubmit(); \
         entries.join('&');");
    assert_eq!(out, "q=hello");
}

// E-7 — `new FormData(formEl)` populates entries from form controls.
// Validates the §C-3 single-source-of-truth path that requestSubmit
// reuses (existing `form_data.rs:203-207` TODO closed).
#[test]
fn new_form_data_with_form_populates_entries() {
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         i.name = 'q'; \
         i.value = 'world'; \
         f.appendChild(i); \
         var fd = new FormData(f); \
         var entries = []; \
         fd.forEach(function(v, k) { entries.push(k + '=' + v); }); \
         entries.join('&');");
    assert_eq!(out, "q=world");
}

// E-8 — requestSubmit(non-submit-button) → TypeError per HTML
// §4.10.21.4 step 2.1.  `<div>` passes `entity_from_this` (it has a
// HostObject wrapper) but fails `is_submit_button` because it has
// no `FormControlState`.
#[test]
fn request_submit_non_submit_button_throws_type_error() {
    let out = run("var f = document.createElement('form'); \
         var d = document.createElement('div'); \
         try { f.requestSubmit(d); 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'ok' : ('other:' + e.name); }");
    assert_eq!(out, "ok");
}

// E-9 — Interactive validation arm: a form with `<input required>`
// empty must NOT fire `submit` / `formdata`, and must fire `invalid`
// on the failing control.  Per HTML §4.10.21.3 the "interactively
// validate the constraints" substep runs before `submit` and aborts
// on failure.
#[test]
fn request_submit_invalid_form_skips_submit_and_fires_invalid() {
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         i.name = 'q'; i.required = true; \
         f.appendChild(i); \
         var trace = []; \
         i.addEventListener('invalid', function() { trace.push('invalid'); }); \
         f.addEventListener('submit', function() { trace.push('submit'); }); \
         f.addEventListener('formdata', function() { trace.push('formdata'); }); \
         f.requestSubmit(); \
         trace.join(',');");
    assert_eq!(out, "invalid");
}

// E-10 — `form.noValidate = true` skips the interactive-validation
// substep so `submit` / `formdata` still fire even when constraints
// would otherwise fail.  Matches HTML §4.10.21.3 "if form's
// no-validate state is true" carve-out.
#[test]
fn request_submit_novalidate_skips_validation() {
    let out = run("var f = document.createElement('form'); \
         f.noValidate = true; \
         var i = document.createElement('input'); \
         i.name = 'q'; i.required = true; \
         f.appendChild(i); \
         var trace = []; \
         i.addEventListener('invalid', function() { trace.push('invalid'); }); \
         f.addEventListener('submit', function() { trace.push('submit'); }); \
         f.addEventListener('formdata', function() { trace.push('formdata'); }); \
         f.requestSubmit(); \
         trace.join(',');");
    assert_eq!(out, "submit,formdata");
}
