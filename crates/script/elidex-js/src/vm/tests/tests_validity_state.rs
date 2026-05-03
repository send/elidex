//! M4-12 slot #11-tags-T1 Phase 9 — `ValidityState` interface +
//! `ConstraintValidation` mixin tests.
//!
//! Covers ValidityState exposure on `<form-control>.validity`,
//! identity caching across repeated reads, the 11 boolean
//! accessors (Phase 9 approximation: only `customError` /
//! `valid` reflect the per-element custom-validity message; other
//! flags return `false`), `setCustomValidity()` / `validationMessage`
//! / `willValidate` / `checkValidity()` / `reportValidity()` across
//! the 5 host element types (input / select / textarea / button /
//! fieldset).

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

// --- ValidityState exposure ----------------------------------------

#[test]
fn validity_state_global_is_present() {
    let out = run("(typeof ValidityState === 'function') ? 'fn' : 'wrong';");
    assert_eq!(out, "fn");
}

#[test]
fn validity_state_constructor_throws_on_new() {
    let out = run("try { new ValidityState(); 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "TypeError");
}

#[test]
fn validity_state_constructor_throws_on_call() {
    let out = run("try { ValidityState(); 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "TypeError");
}

#[test]
fn validity_accessor_returns_validity_state() {
    let out = run("var i = document.createElement('input'); \
         var v = i.validity; \
         (v instanceof ValidityState) + '|' + (typeof v.valid);");
    assert_eq!(out, "true|boolean");
}

#[test]
fn validity_accessor_caches_identity_per_control() {
    let out = run("var i = document.createElement('input'); \
         (i.validity === i.validity) ? 'same' : 'fresh';");
    assert_eq!(out, "same");
}

#[test]
fn validity_distinct_per_control() {
    let out = run("var a = document.createElement('input'); \
         var b = document.createElement('input'); \
         (a.validity !== b.validity) ? 'distinct' : 'shared';");
    assert_eq!(out, "distinct");
}

// --- 11 boolean accessors (approximation: customError / valid only) -

#[test]
fn validity_state_default_all_flags_false_and_valid_true() {
    let out = run("var i = document.createElement('input'); \
         var v = i.validity; \
         v.valueMissing + '|' + v.typeMismatch + '|' + v.patternMismatch + '|' + \
         v.tooLong + '|' + v.tooShort + '|' + v.rangeUnderflow + '|' + \
         v.rangeOverflow + '|' + v.stepMismatch + '|' + v.badInput + '|' + \
         v.customError + '|' + v.valid;");
    assert_eq!(
        out,
        "false|false|false|false|false|false|false|false|false|false|true"
    );
}

#[test]
fn set_custom_validity_flips_custom_error_and_valid() {
    let out = run("var i = document.createElement('input'); \
         i.setCustomValidity('bad'); \
         i.validity.customError + '|' + i.validity.valid;");
    assert_eq!(out, "true|false");
}

#[test]
fn set_custom_validity_empty_string_clears_custom_error() {
    let out = run("var i = document.createElement('input'); \
         i.setCustomValidity('bad'); \
         i.setCustomValidity(''); \
         i.validity.customError + '|' + i.validity.valid;");
    assert_eq!(out, "false|true");
}

// --- validationMessage --------------------------------------------

#[test]
fn validation_message_default_is_empty() {
    let out = run("var i = document.createElement('input'); \
         i.validationMessage;");
    assert_eq!(out, "");
}

#[test]
fn validation_message_returns_custom_validity_when_set() {
    let out = run("var i = document.createElement('input'); \
         i.setCustomValidity('Please enter a value'); \
         i.validationMessage;");
    assert_eq!(out, "Please enter a value");
}

#[test]
fn validation_message_clears_when_custom_validity_cleared() {
    let out = run("var i = document.createElement('input'); \
         i.setCustomValidity('error'); \
         i.setCustomValidity(''); \
         i.validationMessage + '|' + (i.validationMessage === '');");
    assert_eq!(out, "|true");
}

// --- willValidate -------------------------------------------------

#[test]
fn will_validate_is_true_for_default_input() {
    let out = run("var i = document.createElement('input'); \
         i.willValidate.toString();");
    assert_eq!(out, "true");
}

#[test]
fn will_validate_is_false_for_disabled_input() {
    let out = run("var i = document.createElement('input'); \
         i.disabled = true; \
         i.willValidate.toString();");
    assert_eq!(out, "false");
}

#[test]
fn will_validate_is_false_inside_disabled_fieldset() {
    let out = run("var fs = document.createElement('fieldset'); \
         fs.disabled = true; \
         var i = document.createElement('input'); \
         fs.appendChild(i); \
         document.body.appendChild(fs); \
         i.willValidate.toString();");
    assert_eq!(out, "false");
}

// --- checkValidity / reportValidity --------------------------------

#[test]
fn check_validity_true_when_no_custom_error() {
    let out = run("var i = document.createElement('input'); \
         i.checkValidity().toString();");
    assert_eq!(out, "true");
}

#[test]
fn check_validity_false_when_custom_error_set() {
    let out = run("var i = document.createElement('input'); \
         i.setCustomValidity('bad'); \
         i.checkValidity().toString();");
    assert_eq!(out, "false");
}

#[test]
fn report_validity_matches_check_validity_in_headless_mode() {
    let out = run("var i = document.createElement('input'); \
         i.setCustomValidity('bad'); \
         (i.reportValidity() === i.checkValidity()) ? 'same' : 'diff';");
    assert_eq!(out, "same");
}

// --- Mixin installed across 5 host elements ------------------------

#[test]
fn select_has_constraint_validation_mixin() {
    let out = run("var s = document.createElement('select'); \
         (typeof s.checkValidity === 'function') + '|' + \
         (typeof s.setCustomValidity === 'function') + '|' + \
         (s.validity instanceof ValidityState);");
    assert_eq!(out, "true|true|true");
}

#[test]
fn textarea_has_constraint_validation_mixin() {
    let out = run("var t = document.createElement('textarea'); \
         (typeof t.checkValidity === 'function') + '|' + \
         (t.validity instanceof ValidityState);");
    assert_eq!(out, "true|true");
}

#[test]
fn button_has_constraint_validation_mixin() {
    let out = run("var b = document.createElement('button'); \
         (typeof b.checkValidity === 'function') + '|' + \
         (b.validity instanceof ValidityState);");
    assert_eq!(out, "true|true");
}

#[test]
fn fieldset_has_constraint_validation_mixin() {
    let out = run("var fs = document.createElement('fieldset'); \
         (typeof fs.checkValidity === 'function') + '|' + \
         (fs.validity instanceof ValidityState);");
    assert_eq!(out, "true|true");
}

#[test]
fn select_set_custom_validity_round_trip() {
    let out = run("var s = document.createElement('select'); \
         s.setCustomValidity('pick one'); \
         s.validationMessage + '|' + s.checkValidity();");
    assert_eq!(out, "pick one|false");
}

// --- Brand check ---------------------------------------------------

#[test]
fn validity_accessor_throws_on_non_form_control_receiver() {
    let out = run("var i = document.createElement('input'); \
         var div = document.createElement('div'); \
         var getter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(i), 'validity').get; \
         try { getter.call(div); 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "TypeError");
}

#[test]
fn validity_state_value_missing_throws_on_non_validity_receiver() {
    let out = run("var i = document.createElement('input'); \
         var v = i.validity; \
         var getter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(v), 'valueMissing').get; \
         try { getter.call({}); 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "TypeError");
}

// --- Form does not have the full mixin (only checkValidity / reportValidity) ---

#[test]
fn form_has_check_validity_but_no_validity_accessor() {
    let out = run("var f = document.createElement('form'); \
         (typeof f.checkValidity === 'function') + '|' + \
         (f.validity === undefined);");
    assert_eq!(out, "true|true");
}
