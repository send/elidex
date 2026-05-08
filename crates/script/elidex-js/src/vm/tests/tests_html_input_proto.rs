//! Slot `#11-tags-T1-v2` Phase 8 — `HTMLInputElement.prototype` coverage.

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
// type — enumerated keyword
// ---------------------------------------------------------------------------

#[test]
fn input_type_default_text() {
    let out = run("var i = document.createElement('input'); i.type;");
    assert_eq!(out, "text");
}

#[test]
fn input_type_invalid_falls_back_to_text() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'whatever'; \
         i.type;");
    assert_eq!(out, "text");
}

#[test]
fn input_type_known_keyword_round_trip() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'checkbox'; \
         i.type;");
    assert_eq!(out, "checkbox");
}

// ---------------------------------------------------------------------------
// value / defaultValue — IDL state via FormControlState
// ---------------------------------------------------------------------------

#[test]
fn input_value_default_empty() {
    let out = run("var i = document.createElement('input'); i.value;");
    assert_eq!(out, "");
}

#[test]
fn input_value_setter_round_trip() {
    let out = run("var i = document.createElement('input'); \
         i.value = 'hello'; \
         i.value;");
    assert_eq!(out, "hello");
}

#[test]
fn input_default_value_reflects_value_attribute() {
    let out = run("var i = document.createElement('input'); \
         i.defaultValue = 'init'; \
         i.defaultValue + '/' + i.getAttribute('value');");
    assert_eq!(out, "init/init");
}

#[test]
fn input_default_value_setter_updates_value_when_not_dirty() {
    let out = run("var i = document.createElement('input'); \
         i.defaultValue = 'foo'; \
         i.value;");
    assert_eq!(out, "foo");
}

#[test]
fn input_default_value_setter_does_not_overwrite_dirty_value() {
    let out = run("var i = document.createElement('input'); \
         i.value = 'user-typed'; \
         i.defaultValue = 'reset-target'; \
         i.value;");
    assert_eq!(out, "user-typed");
}

// ---------------------------------------------------------------------------
// checked / defaultChecked
// ---------------------------------------------------------------------------

#[test]
fn input_checked_default_false() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'checkbox'; \
         '' + i.checked;");
    assert_eq!(out, "false");
}

#[test]
fn input_checked_round_trip() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'checkbox'; \
         i.checked = true; \
         '' + i.checked;");
    assert_eq!(out, "true");
}

#[test]
fn input_default_checked_reflects_checked_attribute() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'checkbox'; \
         i.defaultChecked = true; \
         '' + i.hasAttribute('checked');");
    assert_eq!(out, "true");
}

// ---------------------------------------------------------------------------
// indeterminate (HTML §4.10.5.1.16) — IDL-only, independent of `checked`
// ---------------------------------------------------------------------------

#[test]
fn input_indeterminate_default_false() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'checkbox'; \
         '' + i.indeterminate;");
    assert_eq!(out, "false");
}

#[test]
fn input_indeterminate_round_trip() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'checkbox'; \
         i.indeterminate = true; \
         '' + i.indeterminate;");
    assert_eq!(out, "true");
}

#[test]
fn input_indeterminate_independent_of_checked() {
    // Setting indeterminate must not toggle `checked`, and vice versa.
    let out = run("var i = document.createElement('input'); \
         i.type = 'checkbox'; \
         i.checked = true; \
         i.indeterminate = true; \
         '' + i.checked + '/' + i.indeterminate;");
    assert_eq!(out, "true/true");
}

#[test]
fn input_indeterminate_does_not_reflect_to_attribute() {
    // Pure IDL bit — no content attribute mirror.
    let out = run("var i = document.createElement('input'); \
         i.type = 'checkbox'; \
         i.indeterminate = true; \
         '' + i.hasAttribute('indeterminate');");
    assert_eq!(out, "false");
}

// ---------------------------------------------------------------------------
// Reflected primitives
// ---------------------------------------------------------------------------

#[test]
fn input_disabled_round_trip() {
    let out = run("var i = document.createElement('input'); \
         i.disabled = true; \
         '' + i.hasAttribute('disabled');");
    assert_eq!(out, "true");
}

#[test]
fn input_required_round_trip() {
    let out = run("var i = document.createElement('input'); \
         i.required = true; \
         '' + i.hasAttribute('required');");
    assert_eq!(out, "true");
}

#[test]
fn input_max_length_negative_removes_attribute() {
    // R19 F1 regression — HTML §6.13.1 reflection rule for
    // unsigned-long length attrs: negative values clear the
    // content attribute (IDL getter falls back to default -1)
    // rather than persist `maxlength="-1"`.
    let out = run("var i = document.createElement('input'); \
         i.maxLength = 10; \
         i.maxLength = -1; \
         '' + i.hasAttribute('maxlength') + '/' + i.maxLength;");
    assert_eq!(out, "false/-1");
}

#[test]
fn input_set_range_text_coerces_string_start_end() {
    // R19 F2 regression — WebIDL `unsigned long` coercion: string
    // arguments flow through ToInt32 (`"2"` → 2), not the old
    // `try_to_int_or_zero` fallback that defaulted everything
    // non-finite-Number to 0.
    let out = run("var i = document.createElement('input'); \
         i.value = 'abcdef'; \
         i.setRangeText('XY', '1', '4'); \
         i.value;");
    assert_eq!(out, "aXYef");
}

#[test]
fn input_max_length_round_trip() {
    let out = run("var i = document.createElement('input'); \
         i.maxLength = 32; \
         '' + i.maxLength;");
    assert_eq!(out, "32");
}

#[test]
fn input_pattern_round_trip() {
    let out = run("var i = document.createElement('input'); \
         i.pattern = '[a-z]+'; \
         i.pattern;");
    assert_eq!(out, "[a-z]+");
}

#[test]
fn input_placeholder_round_trip() {
    let out = run("var i = document.createElement('input'); \
         i.placeholder = 'enter…'; \
         i.placeholder;");
    assert_eq!(out, "enter…");
}

// ---------------------------------------------------------------------------
// valueAsNumber
// ---------------------------------------------------------------------------

#[test]
fn input_value_as_number_for_text_returns_nan() {
    let out = run("var i = document.createElement('input'); \
         i.value = '42'; \
         '' + i.valueAsNumber;");
    // Default type is "text" — valueAsNumber returns NaN.
    assert_eq!(out, "NaN");
}

#[test]
fn input_value_as_number_for_number_type() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'number'; \
         i.value = '42'; \
         '' + i.valueAsNumber;");
    assert_eq!(out, "42");
}

#[test]
fn input_value_as_number_setter_writes_value() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'number'; \
         i.valueAsNumber = 7.5; \
         i.value;");
    assert_eq!(out, "7.5");
}

#[test]
fn input_value_as_number_setter_rejects_non_finite_infinity() {
    // HTML §4.10.5.1.4 step 5 — non-finite values throw TypeError.
    let out = run("var i = document.createElement('input'); \
         i.type = 'number'; \
         try { i.valueAsNumber = Infinity; 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'type' : 'other'; }");
    assert_eq!(out, "type");
}

#[test]
fn input_value_as_number_setter_rejects_non_finite_nan() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'number'; \
         try { i.valueAsNumber = NaN; 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'type' : 'other'; }");
    assert_eq!(out, "type");
}

#[test]
fn input_value_as_number_setter_rejects_non_number_string() {
    // Non-Number argument: throws TypeError before the finite check.
    let out = run("var i = document.createElement('input'); \
         i.type = 'number'; \
         try { i.valueAsNumber = '5'; 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'type' : 'other'; }");
    assert_eq!(out, "type");
}

// ---------------------------------------------------------------------------
// Selection API
// ---------------------------------------------------------------------------

#[test]
fn input_select_method_marks_full_range() {
    let out = run("var i = document.createElement('input'); \
         i.value = 'hello'; \
         i.select(); \
         '' + i.selectionStart + '/' + i.selectionEnd;");
    assert_eq!(out, "0/5");
}

#[test]
fn input_set_selection_range_updates_state() {
    let out = run("var i = document.createElement('input'); \
         i.value = 'abcdef'; \
         i.setSelectionRange(2, 5); \
         '' + i.selectionStart + '/' + i.selectionEnd;");
    assert_eq!(out, "2/5");
}

#[test]
fn input_set_range_text_replaces_selection() {
    let out = run("var i = document.createElement('input'); \
         i.value = 'abcdef'; \
         i.setSelectionRange(1, 4); \
         i.setRangeText('XYZ'); \
         i.value;");
    assert_eq!(out, "aXYZef");
}

#[test]
fn input_selection_throws_for_non_text_type() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'number'; \
         try { i.select(); 'no-throw'; } \
         catch (e) { (e.name === 'InvalidStateError') ? 'ok' : 'other:' + e.name; }");
    assert_eq!(out, "ok");
}

#[test]
fn input_selection_direction_default_none() {
    let out = run("var i = document.createElement('input'); \
         i.value = 'x'; \
         i.selectionDirection;");
    assert_eq!(out, "none");
}

// ---------------------------------------------------------------------------
// stepUp / stepDown
// ---------------------------------------------------------------------------

#[test]
fn input_step_up_increments_number_value() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'number'; \
         i.value = '10'; \
         i.stepUp(); \
         i.value;");
    assert_eq!(out, "11");
}

#[test]
fn input_step_down_decrements_number_value() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'number'; \
         i.value = '10'; \
         i.stepDown(2); \
         i.value;");
    assert_eq!(out, "8");
}

#[test]
fn input_step_up_throws_for_non_steppable_type() {
    let out = run("var i = document.createElement('input'); \
         try { i.stepUp(); 'no-throw'; } \
         catch (e) { (e.name === 'InvalidStateError') ? 'ok' : 'other:' + e.name; }");
    assert_eq!(out, "ok");
}

// ---------------------------------------------------------------------------
// Stubs / form / labels
// ---------------------------------------------------------------------------

#[test]
fn input_show_picker_throws_not_supported() {
    let out = run("var i = document.createElement('input'); \
         try { i.showPicker(); 'no-throw'; } \
         catch (e) { (e.name === 'NotSupportedError') ? 'ok' : 'other'; }");
    assert_eq!(out, "ok");
}

#[test]
fn input_files_returns_null_stub() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'file'; \
         (i.files === null) ? 'null' : 'non-null';");
    assert_eq!(out, "null");
}

#[test]
fn input_list_returns_null_stub() {
    let out = run("var i = document.createElement('input'); \
         (i.list === null) ? 'null' : 'non-null';");
    assert_eq!(out, "null");
}

#[test]
fn input_form_resolves_via_form_ancestor() {
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         f.appendChild(i); \
         (i.form === f) ? 'same' : 'diff';");
    assert_eq!(out, "same");
}

#[test]
fn input_brand_check_throws_on_non_input_receiver() {
    let out = run("var d = document.createElement('div'); \
         var i = document.createElement('input'); \
         var getter = Object.getOwnPropertyDescriptor(\
             Object.getPrototypeOf(i), 'value').get; \
         try { getter.call(d); 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'type' : 'other'; }");
    assert_eq!(out, "type");
}

// ---------------------------------------------------------------------------
// R25 regressions — WebIDL `unsigned long` (ToUint32) coercion for
// the Selection API mixin (HTML §4.10.5.2.10).  Negative inputs
// must wrap to (2³² + n) and then clamp to value.len(), NOT clamp
// to 0 immediately (which would change the user-visible
// behaviour from "selection at end of value" to "selection at
// start of value").
// ---------------------------------------------------------------------------

#[test]
fn input_set_selection_range_negative_wraps_then_clamps_to_length() {
    // -1 ToUint32 = 4294967295, which clamps to value.len() = 5.
    // Spec: the resulting range is [5, 5], i.e. a collapsed
    // caret at end of value.  Buggy `to_int32(...)?.max(0)` would
    // produce [0, 5] = the whole-string selection.
    let out = run("var i = document.createElement('input'); \
         i.value = 'hello'; \
         i.setSelectionRange(-1, -1); \
         i.selectionStart + '/' + i.selectionEnd;");
    assert_eq!(out, "5/5");
}

#[test]
fn input_selection_start_setter_negative_wraps_then_clamps() {
    // i.selectionStart = -1 → ToUint32 = u32::MAX → clamps to len.
    let out = run("var i = document.createElement('input'); \
         i.value = 'abc'; \
         i.setSelectionRange(0, 3); \
         i.selectionStart = -1; \
         '' + i.selectionStart;");
    assert_eq!(out, "3");
}

#[test]
fn input_set_range_text_negative_start_wraps_then_clamps() {
    // setRangeText('X', -1, -1) → start/end coerce to u32::MAX,
    // both clamp to value.len() = 3, so the empty range [3,3)
    // gets replaced by "X" — the result is "abcX" (insert at
    // end).  Buggy `to_int32(...)?.max(0)` would clamp to [0,0)
    // and produce "Xabc" (insert at start).
    let out = run("var i = document.createElement('input'); \
         i.value = 'abc'; \
         i.setRangeText('X', -1, -1); \
         i.value;");
    assert_eq!(out, "abcX");
}

#[test]
fn input_set_selection_range_large_positive_clamps_to_length() {
    // 2^32 - 5 ToUint32 = 2^32 - 5; clamps to len=3 → [3,3].
    // (Buggy `to_int32` would wrap to -5 then clamp to 0,
    // yielding [0,0] = caret-at-start.)
    let out = run("var i = document.createElement('input'); \
         i.value = 'abc'; \
         i.setSelectionRange(4294967291, 4294967291); \
         i.selectionStart + '/' + i.selectionEnd;");
    assert_eq!(out, "3/3");
}

// `<input>.formMethod` / `<input>.formEnctype` enumerated reflection
// (HTML §4.10.5.4) — missing- and invalid-value defaults are both
// `""` (the no-override sentinel), distinct from `<form>.method` /
// `<form>.enctype` whose defaults are keywords.
#[test]
fn input_form_method_default_when_missing_is_empty_string() {
    let out = run("var i = document.createElement('input'); i.formMethod;");
    assert_eq!(out, "");
}

#[test]
fn input_form_method_canonicalises_uppercase_to_lowercase() {
    let out = run("var i = document.createElement('input'); \
         i.setAttribute('formmethod', 'POST'); i.formMethod;");
    assert_eq!(out, "post");
}

#[test]
fn input_form_method_invalid_falls_back_to_empty_string() {
    let out = run("var i = document.createElement('input'); \
         i.setAttribute('formmethod', 'bogus'); i.formMethod;");
    assert_eq!(out, "");
}

#[test]
fn input_form_enctype_default_when_missing_is_empty_string() {
    let out = run("var i = document.createElement('input'); i.formEnctype;");
    assert_eq!(out, "");
}

#[test]
fn input_form_enctype_canonicalises_multipart() {
    let out = run("var i = document.createElement('input'); \
         i.setAttribute('formenctype', 'MULTIPART/FORM-DATA'); i.formEnctype;");
    assert_eq!(out, "multipart/form-data");
}

#[test]
fn input_form_enctype_invalid_falls_back_to_empty_string() {
    let out = run("var i = document.createElement('input'); \
         i.setAttribute('formenctype', 'application/json'); i.formEnctype;");
    assert_eq!(out, "");
}
