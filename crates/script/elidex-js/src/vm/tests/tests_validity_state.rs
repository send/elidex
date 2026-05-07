//! Slot `#11-tags-T1-v2` Phase 9 — `ValidityState` wrapper +
//! `ConstraintValidation` mixin (`validity` / `validationMessage` /
//! `willValidate` / `checkValidity` / `reportValidity` /
//! `setCustomValidity`) coverage.

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

// ---------------------------------------------------------------------------
// validity — [SameObject] + accessors
// ---------------------------------------------------------------------------

#[test]
fn input_validity_returns_validity_state_object() {
    let out = run("var i = document.createElement('input'); \
         (typeof i.validity === 'object') ? 'ok' : 'bad';");
    assert_eq!(out, "ok");
}

#[test]
fn input_validity_same_object() {
    let out = run("var i = document.createElement('input'); \
         (i.validity === i.validity) ? 'same' : 'diff';");
    assert_eq!(out, "same");
}

#[test]
fn input_validity_value_missing_for_required_empty() {
    let out = run("var i = document.createElement('input'); \
         i.required = true; \
         '' + i.validity.valueMissing;");
    assert_eq!(out, "true");
}

#[test]
fn input_validity_valid_when_no_constraints_violated() {
    let out = run("var i = document.createElement('input'); \
         '' + i.validity.valid;");
    assert_eq!(out, "true");
}

#[test]
fn input_validity_too_long_when_max_length_exceeded() {
    let out = run("var i = document.createElement('input'); \
         i.maxLength = 3; \
         i.value = 'abcdef'; \
         '' + i.validity.tooLong;");
    assert_eq!(out, "true");
}

#[test]
fn textarea_validity_too_long_when_max_length_exceeded() {
    // Regression test for C-2 — textarea.maxLength setter must
    // mirror into FormControlState.maxlength so validate_control()
    // observes the constraint without re-attach.
    let out = run("var t = document.createElement('textarea'); \
         t.maxLength = 3; \
         t.value = 'abcdef'; \
         '' + t.validity.tooLong;");
    assert_eq!(out, "true");
}

#[test]
fn textarea_max_length_negative_resets_constraint() {
    // Regression test for C-2 — `maxLength = -1` must clear the
    // constraint (state.maxlength = None), matching HTML
    // §4.10.5.1.13 reflection rules.
    let out = run("var t = document.createElement('textarea'); \
         t.maxLength = 3; \
         t.maxLength = -1; \
         t.value = 'abcdef'; \
         '' + t.validity.tooLong;");
    assert_eq!(out, "false");
}

#[test]
fn textarea_validity_value_missing_when_required_sync_works() {
    // Regression test for I-3 — `required = true` via JS must
    // round-trip into FormControlState.required (through
    // bool_attr_with_state_sync) so an empty required textarea is
    // valueMissing.
    let out = run("var t = document.createElement('textarea'); \
         t.required = true; \
         '' + t.validity.valueMissing;");
    assert_eq!(out, "true");
}

// ---------------------------------------------------------------------------
// setCustomValidity / validationMessage / customError
// ---------------------------------------------------------------------------

#[test]
fn input_set_custom_validity_marks_custom_error() {
    let out = run("var i = document.createElement('input'); \
         i.setCustomValidity('Bad'); \
         '' + i.validity.customError + '/' + i.validity.valid;");
    assert_eq!(out, "true/false");
}

#[test]
fn input_validation_message_returns_custom_error() {
    let out = run("var i = document.createElement('input'); \
         i.setCustomValidity('Bad data'); \
         i.validationMessage;");
    assert_eq!(out, "Bad data");
}

#[test]
fn input_set_custom_validity_clear_with_empty_string() {
    let out = run("var i = document.createElement('input'); \
         i.setCustomValidity('Bad'); \
         i.setCustomValidity(''); \
         '' + i.validity.customError + '/' + i.validity.valid;");
    assert_eq!(out, "false/true");
}

// ---------------------------------------------------------------------------
// willValidate / checkValidity / reportValidity
// ---------------------------------------------------------------------------

#[test]
fn input_will_validate_true_for_text_default() {
    let out = run("var i = document.createElement('input'); \
         '' + i.willValidate;");
    assert_eq!(out, "true");
}

#[test]
fn input_will_validate_false_when_disabled() {
    let out = run("var i = document.createElement('input'); \
         i.disabled = true; \
         '' + i.willValidate;");
    assert_eq!(out, "false");
}

#[test]
fn input_will_validate_false_for_hidden_type() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'hidden'; \
         '' + i.willValidate;");
    assert_eq!(out, "false");
}

#[test]
fn input_check_validity_returns_true_when_valid() {
    let out = run("var i = document.createElement('input'); \
         '' + i.checkValidity();");
    assert_eq!(out, "true");
}

#[test]
fn input_check_validity_returns_false_when_required_empty() {
    let out = run("var i = document.createElement('input'); \
         i.required = true; \
         '' + i.checkValidity();");
    assert_eq!(out, "false");
}

#[test]
fn input_report_validity_aliases_check_validity() {
    let out = run("var i = document.createElement('input'); \
         i.required = true; \
         '' + i.reportValidity();");
    assert_eq!(out, "false");
}

// ---------------------------------------------------------------------------
// Mixin install verification on other prototypes
// ---------------------------------------------------------------------------

#[test]
fn select_validity_returns_object() {
    let out = run("var s = document.createElement('select'); \
         (typeof s.validity === 'object') ? 'ok' : 'bad';");
    assert_eq!(out, "ok");
}

#[test]
fn textarea_validity_returns_object() {
    let out = run("var t = document.createElement('textarea'); \
         (typeof t.validity === 'object') ? 'ok' : 'bad';");
    assert_eq!(out, "ok");
}

#[test]
fn button_validity_returns_object() {
    let out = run("var b = document.createElement('button'); \
         (typeof b.validity === 'object') ? 'ok' : 'bad';");
    assert_eq!(out, "ok");
}

#[test]
fn fieldset_check_validity_returns_true() {
    let out = run("var f = document.createElement('fieldset'); \
         '' + f.checkValidity();");
    assert_eq!(out, "true");
}

#[test]
fn check_validity_fires_invalid_event_when_control_is_invalid() {
    let out = run("var i = document.createElement('input'); \
         i.required = true; \
         var fired = ''; \
         i.addEventListener('invalid', function(e) { \
            fired = e.type + '/' + e.bubbles + '/' + e.cancelable; \
         }); \
         var v = i.checkValidity(); \
         fired + '|' + v;");
    assert_eq!(out, "invalid/false/true|false");
}

#[test]
fn check_validity_does_not_fire_invalid_event_when_valid() {
    let out = run("var i = document.createElement('input'); \
         var fired = false; \
         i.addEventListener('invalid', function() { fired = true; }); \
         var v = i.checkValidity(); \
         '' + fired + '/' + v;");
    assert_eq!(out, "false/true");
}

#[test]
fn check_validity_returns_false_even_when_invalid_event_is_default_prevented() {
    let out = run("var i = document.createElement('input'); \
         i.required = true; \
         i.addEventListener('invalid', function(e) { e.preventDefault(); }); \
         '' + i.checkValidity();");
    // checkValidity always returns the validity result; preventDefault
    // only suppresses the UA's reporting, not the boolean return.
    assert_eq!(out, "false");
}

#[test]
fn validity_state_brand_check_throws_on_non_validity_receiver() {
    let out = run("var i = document.createElement('input'); \
         var v = i.validity; \
         var getter = Object.getOwnPropertyDescriptor(\
             Object.getPrototypeOf(v), 'valueMissing').get; \
         try { getter.call({}); 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'type' : 'other'; }");
    assert_eq!(out, "type");
}

#[test]
fn check_validity_brand_check_rejects_non_form_control_receiver() {
    // F8 regression — `HTMLInputElement.prototype.checkValidity.call(div)`
    // must throw TypeError "Illegal invocation" because <div> is not
    // one of the constraint-validation host tags.
    let out = run("var i = document.createElement('input'); \
         var d = document.createElement('div'); \
         var fn = Object.getPrototypeOf(i).checkValidity; \
         try { fn.call(d); 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'type' : 'other:' + e.name; }");
    assert_eq!(out, "type");
}

#[test]
fn validity_brand_check_rejects_cross_element_validity_getter() {
    // F8 regression — `validity` getter on a non-form-control
    // receiver must throw.
    let out = run("var i = document.createElement('input'); \
         var d = document.createElement('div'); \
         var getter = Object.getOwnPropertyDescriptor(\
             Object.getPrototypeOf(i), 'validity').get; \
         try { getter.call(d); 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'type' : 'other:' + e.name; }");
    assert_eq!(out, "type");
}

#[test]
fn check_validity_returns_true_for_hidden_input_even_when_required() {
    // F9 regression — HTML §4.10.20.3 bars `<input type=hidden>`
    // from constraint validation regardless of `required`.
    // checkValidity must return true (no candidate, no validation
    // ran), and no `invalid` event must be dispatched.
    let out = run("var i = document.createElement('input'); \
         i.type = 'hidden'; \
         i.required = true; \
         var fired = false; \
         i.addEventListener('invalid', function() { fired = true; }); \
         var v = i.checkValidity(); \
         '' + v + '/' + fired;");
    assert_eq!(out, "true/false");
}
