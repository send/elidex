//! Slot `#11-tags-T1-v2` Phase 6 — `HTMLTextAreaElement.prototype`
//! coverage (incl. Selection API mixin folded in for B-1).

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
fn textarea_type_returns_textarea() {
    let out = run("var t = document.createElement('textarea'); t.type;");
    assert_eq!(out, "textarea");
}

#[test]
fn textarea_cols_default_20() {
    let out = run("var t = document.createElement('textarea'); '' + t.cols;");
    assert_eq!(out, "20");
}

#[test]
fn textarea_rows_default_2() {
    let out = run("var t = document.createElement('textarea'); '' + t.rows;");
    assert_eq!(out, "2");
}

#[test]
fn textarea_cols_round_trip() {
    let out = run("var t = document.createElement('textarea'); \
         t.cols = 80; \
         '' + t.cols + '/' + t.getAttribute('cols');");
    assert_eq!(out, "80/80");
}

#[test]
fn textarea_max_length_default_minus_one() {
    let out = run("var t = document.createElement('textarea'); '' + t.maxLength;");
    assert_eq!(out, "-1");
}

#[test]
fn textarea_disabled_round_trip() {
    let out = run("var t = document.createElement('textarea'); \
         t.disabled = true; \
         '' + t.hasAttribute('disabled');");
    assert_eq!(out, "true");
}

#[test]
fn textarea_readonly_round_trip() {
    let out = run("var t = document.createElement('textarea'); \
         t.readOnly = true; \
         '' + t.hasAttribute('readonly');");
    assert_eq!(out, "true");
}

#[test]
fn textarea_required_round_trip() {
    let out = run("var t = document.createElement('textarea'); \
         t.required = true; \
         '' + t.hasAttribute('required');");
    assert_eq!(out, "true");
}

#[test]
fn textarea_name_round_trip() {
    let out = run("var t = document.createElement('textarea'); \
         t.name = 'msg'; \
         t.name;");
    assert_eq!(out, "msg");
}

#[test]
fn textarea_placeholder_round_trip() {
    let out = run("var t = document.createElement('textarea'); \
         t.placeholder = 'enter…'; \
         t.placeholder;");
    assert_eq!(out, "enter…");
}

#[test]
fn textarea_wrap_round_trip() {
    let out = run("var t = document.createElement('textarea'); \
         t.wrap = 'soft'; \
         t.wrap;");
    assert_eq!(out, "soft");
}

#[test]
fn textarea_form_resolves_via_form_ancestor() {
    let out = run("var f = document.createElement('form'); \
         var t = document.createElement('textarea'); \
         f.appendChild(t); \
         (t.form === f) ? 'same' : 'diff';");
    assert_eq!(out, "same");
}

#[test]
fn textarea_default_value_reflects_text_content() {
    let out = run("var t = document.createElement('textarea'); \
         t.defaultValue = 'hello'; \
         t.defaultValue + '/' + t.textContent;");
    assert_eq!(out, "hello/hello");
}

#[test]
fn textarea_value_round_trip_state_backed() {
    // Setting `.value` writes into FormControlState and is observable
    // independently of `defaultValue` / `textContent`.
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'hi'; \
         t.value;");
    assert_eq!(out, "hi");
}

#[test]
fn textarea_default_value_setter_updates_value_when_not_dirty() {
    let out = run("var t = document.createElement('textarea'); \
         t.defaultValue = 'init'; \
         t.value;");
    assert_eq!(out, "init");
}

#[test]
fn textarea_default_value_setter_does_not_overwrite_dirty_value() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'user-typed'; \
         t.defaultValue = 'reset-target'; \
         t.value;");
    assert_eq!(out, "user-typed");
}

#[test]
fn textarea_text_length_counts_utf16() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'abcd'; \
         '' + t.textLength;");
    assert_eq!(out, "4");
}

#[test]
fn textarea_brand_check_throws_on_non_textarea_receiver() {
    let out = run("var d = document.createElement('div'); \
         var t = document.createElement('textarea'); \
         var getter = Object.getOwnPropertyDescriptor(\
             Object.getPrototypeOf(t), 'rows').get; \
         try { getter.call(d); 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'type' : 'other'; }");
    assert_eq!(out, "type");
}

// ---------------------------------------------------------------------------
// Selection API mixin (B-1) — same surface as HTMLInputElement
// ---------------------------------------------------------------------------

#[test]
fn textarea_selection_default_zero() {
    let out = run("var t = document.createElement('textarea'); \
         '' + t.selectionStart + '/' + t.selectionEnd;");
    assert_eq!(out, "0/0");
}

#[test]
fn textarea_selection_direction_default_none() {
    let out = run("var t = document.createElement('textarea'); \
         t.selectionDirection;");
    assert_eq!(out, "none");
}

#[test]
fn textarea_select_method_marks_full_range() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'hello'; \
         t.select(); \
         '' + t.selectionStart + '/' + t.selectionEnd;");
    assert_eq!(out, "0/5");
}

#[test]
fn textarea_set_selection_range_updates_state() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'abcdef'; \
         t.setSelectionRange(2, 5); \
         '' + t.selectionStart + '/' + t.selectionEnd;");
    assert_eq!(out, "2/5");
}

#[test]
fn textarea_set_selection_range_with_direction() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'abcdef'; \
         t.setSelectionRange(1, 4, 'backward'); \
         t.selectionDirection;");
    assert_eq!(out, "backward");
}

#[test]
fn textarea_selection_start_setter_round_trip() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'abcdef'; \
         t.selectionStart = 3; \
         '' + t.selectionStart;");
    assert_eq!(out, "3");
}

#[test]
fn textarea_selection_end_setter_round_trip() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'abcdef'; \
         t.selectionEnd = 4; \
         '' + t.selectionEnd;");
    assert_eq!(out, "4");
}

#[test]
fn textarea_max_length_negative_removes_attribute() {
    // R19 F1 regression — same as input.maxLength: negative
    // values clear the `maxlength` content attribute per
    // HTML §6.13.1 reflection rules.
    let out = run("var t = document.createElement('textarea'); \
         t.maxLength = 5; \
         t.maxLength = -1; \
         '' + t.hasAttribute('maxlength') + '/' + t.maxLength;");
    assert_eq!(out, "false/-1");
}

#[test]
fn textarea_set_range_text_coerces_boolean_start_end() {
    // R19 F2 regression — WebIDL `unsigned long` coercion:
    // boolean true → 1 / false → 0 via ToNumber → ToInt32, not
    // the old "non-Number defaults to 0" fallback.
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'abcdef'; \
         t.setRangeText('Z', true, true); \
         t.value;");
    // start=1 (true→1), end=1 (true→1) → insertion point at 1, no chars replaced
    assert_eq!(out, "aZbcdef");
}

#[test]
fn textarea_set_range_text_replaces_selection() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'abcdef'; \
         t.setSelectionRange(1, 4); \
         t.setRangeText('XYZ'); \
         t.value;");
    assert_eq!(out, "aXYZef");
}

#[test]
fn textarea_set_range_text_with_explicit_bounds() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'abcdef'; \
         t.setRangeText('Q', 2, 4); \
         t.value;");
    assert_eq!(out, "abQef");
}

#[test]
fn textarea_selection_brand_check_throws_on_non_textarea_receiver() {
    let out = run("var d = document.createElement('div'); \
         var t = document.createElement('textarea'); \
         try { t.setSelectionRange.call(d, 0, 0); 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'type' : 'other'; }");
    assert_eq!(out, "type");
}

// ---------------------------------------------------------------------------
// R25 regressions — WebIDL `unsigned long` (ToUint32) coercion for
// the Selection API mixin on textarea, mirroring
// `tests_html_input_proto::input_*_negative_wraps_then_clamps_*`.
// ---------------------------------------------------------------------------

#[test]
fn textarea_set_selection_range_negative_wraps_then_clamps_to_length() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'hello'; \
         t.setSelectionRange(-1, -1); \
         t.selectionStart + '/' + t.selectionEnd;");
    assert_eq!(out, "5/5");
}

#[test]
fn textarea_selection_start_setter_negative_wraps_then_clamps() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'abc'; \
         t.setSelectionRange(0, 3); \
         t.selectionStart = -1; \
         '' + t.selectionStart;");
    assert_eq!(out, "3");
}

#[test]
fn textarea_set_range_text_negative_start_wraps_then_clamps() {
    // -1 → ToUint32 = u32::MAX → clamps to len = 3 → empty range
    // [3,3) replaced by "X" → "abcX".  See input_set_range_text_*.
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'abc'; \
         t.setRangeText('X', -1, -1); \
         t.value;");
    assert_eq!(out, "abcX");
}
