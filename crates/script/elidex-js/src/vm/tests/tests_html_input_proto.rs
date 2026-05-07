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
