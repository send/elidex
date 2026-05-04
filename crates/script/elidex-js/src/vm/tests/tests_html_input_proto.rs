//! M4-12 slot #11-tags-T1 Phase 8 — `HTMLInputElement.prototype` tests.
//!
//! Covers reflected attributes (~30 of them), `type` enumerated
//! reflection with the `"text"` invalid-value/missing-value default,
//! `value` / `defaultValue` / `checked` / `defaultChecked`,
//! `valueAsNumber` / `valueAsDate` per-type gating, `stepUp` /
//! `stepDown`, the Selection API gated by `supports_selection`, and
//! the `files` / `showPicker` / `list` deferred stubs.

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

// --- Prototype identity --------------------------------------------

#[test]
fn input_wrapper_has_html_input_prototype() {
    let out = run("var i1 = document.createElement('input'); \
         var i2 = document.createElement('input'); \
         var proto = Object.getPrototypeOf(i1); \
         var same = Object.getPrototypeOf(i2) === proto; \
         var hasValue = Object.getOwnPropertyDescriptor(proto, 'value') !== undefined; \
         var hasType = Object.getOwnPropertyDescriptor(proto, 'type') !== undefined; \
         var hasFiles = Object.getOwnPropertyDescriptor(proto, 'files') !== undefined; \
         (same && hasValue && hasType && hasFiles) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// --- type enumerated reflect --------------------------------------

#[test]
fn input_type_default_is_text() {
    let out = run("var i = document.createElement('input'); \
         i.type;");
    assert_eq!(out, "text");
}

#[test]
fn input_type_invalid_value_falls_back_to_text() {
    let out = run("var i = document.createElement('input'); \
         i.setAttribute('type', 'frobozz'); \
         i.type;");
    assert_eq!(out, "text");
}

#[test]
fn input_type_known_keywords_round_trip() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'checkbox'; var c = i.type; \
         i.type = 'radio'; var r = i.type; \
         i.type = 'number'; var n = i.type; \
         i.type = 'email'; var e = i.type; \
         c + '|' + r + '|' + n + '|' + e;");
    assert_eq!(out, "checkbox|radio|number|email");
}

#[test]
fn input_type_case_insensitive_normalises_to_lowercase() {
    let out = run("var i = document.createElement('input'); \
         i.setAttribute('type', 'NUMBER'); \
         i.type;");
    assert_eq!(out, "number");
}

// --- value / defaultValue ------------------------------------------

#[test]
fn input_default_value_reflects_value_attribute() {
    let out = run("var i = document.createElement('input'); \
         i.setAttribute('value', 'hello'); \
         i.defaultValue + '|' + i.value;");
    assert_eq!(out, "hello|hello");
}

#[test]
fn input_value_setter_overrides_default_value() {
    let out = run("var i = document.createElement('input'); \
         i.setAttribute('value', 'default'); \
         i.value = 'override'; \
         i.value + '|' + i.defaultValue;");
    assert_eq!(out, "override|default");
}

#[test]
fn input_default_value_setter_writes_attribute() {
    let out = run("var i = document.createElement('input'); \
         i.defaultValue = 'fresh'; \
         i.getAttribute('value');");
    assert_eq!(out, "fresh");
}

// --- checked / defaultChecked --------------------------------------

#[test]
fn input_checked_reflects_attribute() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'checkbox'; \
         i.checked = true; \
         var on = i.checked + '|' + i.hasAttribute('checked'); \
         i.checked = false; \
         on + '/' + i.checked;");
    assert_eq!(out, "true|true/false");
}

#[test]
fn input_default_checked_reflects_attribute() {
    let out = run("var i = document.createElement('input'); \
         i.defaultChecked = true; \
         i.defaultChecked + '|' + i.hasAttribute('checked');");
    assert_eq!(out, "true|true");
}

// --- Boolean reflects ----------------------------------------------

#[test]
fn input_disabled_round_trip() {
    let out = run("var i = document.createElement('input'); \
         i.disabled = true; \
         i.disabled + '|' + i.hasAttribute('disabled');");
    assert_eq!(out, "true|true");
}

#[test]
fn input_required_round_trip() {
    let out = run("var i = document.createElement('input'); \
         i.required = true; \
         i.required.toString();");
    assert_eq!(out, "true");
}

#[test]
fn input_read_only_lowercased_attribute_name() {
    let out = run("var i = document.createElement('input'); \
         i.readOnly = true; \
         i.hasAttribute('readonly').toString();");
    assert_eq!(out, "true");
}

// --- String reflects ----------------------------------------------

#[test]
fn input_string_attrs_round_trip() {
    let out = run("var i = document.createElement('input'); \
         i.accept = 'image/*'; \
         i.alt = 'pic'; \
         i.placeholder = 'enter'; \
         i.pattern = '\\\\d+'; \
         i.src = '/img.png'; \
         i.accept + '|' + i.alt + '|' + i.placeholder + '|' + i.pattern + '|' + i.src;");
    assert_eq!(out, "image/*|pic|enter|\\d+|/img.png");
}

#[test]
fn input_form_overrides_round_trip() {
    let out = run("var i = document.createElement('input'); \
         i.formAction = '/x'; \
         i.formMethod = 'post'; \
         i.formAction + '|' + i.formMethod + '|' + i.getAttribute('formaction');");
    assert_eq!(out, "/x|post|/x");
}

// --- Numeric reflects ---------------------------------------------

#[test]
fn input_size_default_is_20() {
    let out = run("var i = document.createElement('input'); \
         i.size.toString();");
    assert_eq!(out, "20");
}

#[test]
fn input_size_round_trip() {
    let out = run("var i = document.createElement('input'); \
         i.size = 30; \
         i.size + '|' + i.getAttribute('size');");
    assert_eq!(out, "30|30");
}

#[test]
fn input_max_length_default_is_negative_one() {
    let out = run("var i = document.createElement('input'); \
         i.maxLength.toString();");
    assert_eq!(out, "-1");
}

#[test]
fn input_max_length_round_trip() {
    let out = run("var i = document.createElement('input'); \
         i.maxLength = 50; \
         i.maxLength + '|' + i.getAttribute('maxlength');");
    assert_eq!(out, "50|50");
}

#[test]
fn input_max_length_negative_throws_index_size_error() {
    // HTML §6.13.1 reflect rule "limited to only non-negative
    // numbers" — negative values throw IndexSizeError.
    let out = run("var i = document.createElement('input'); \
         try { i.maxLength = -3; 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "IndexSizeError");
}

#[test]
fn input_min_length_negative_throws_index_size_error() {
    let out = run("var i = document.createElement('input'); \
         try { i.minLength = -1; 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "IndexSizeError");
}

#[test]
fn input_width_height_round_trip() {
    let out = run("var i = document.createElement('input'); \
         i.width = 100; i.height = 50; \
         i.width + '|' + i.height;");
    assert_eq!(out, "100|50");
}

// --- valueAsNumber -------------------------------------------------

#[test]
fn input_value_as_number_for_numeric_type() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'number'; \
         i.value = '42'; \
         i.valueAsNumber.toString();");
    assert_eq!(out, "42");
}

#[test]
fn input_value_as_number_is_nan_for_text_type() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'text'; \
         i.value = 'hello'; \
         isNaN(i.valueAsNumber) + '|' + (i.valueAsNumber + '');");
    assert_eq!(out, "true|NaN");
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
fn input_value_as_number_setter_throws_for_non_numeric_type() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'text'; \
         try { i.valueAsNumber = 5; 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "InvalidStateError");
}

// --- valueAsDate (deferred) ----------------------------------------

#[test]
fn input_value_as_date_returns_null_for_date_type() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'date'; \
         (i.valueAsDate === null) ? 'null' : 'wrong';");
    assert_eq!(out, "null");
}

#[test]
fn input_value_as_date_setter_accepts_null() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'date'; \
         i.valueAsDate = null; \
         'ok';");
    assert_eq!(out, "ok");
}

#[test]
fn input_value_as_date_setter_throws_for_non_date_type() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'text'; \
         try { i.valueAsDate = null; 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "InvalidStateError");
}

// --- stepUp / stepDown ---------------------------------------------

#[test]
fn input_step_up_default_step_is_one() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'number'; \
         i.value = '10'; \
         i.stepUp(); \
         i.value;");
    assert_eq!(out, "11");
}

#[test]
fn input_step_up_with_count() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'number'; \
         i.value = '10'; \
         i.stepUp(5); \
         i.value;");
    assert_eq!(out, "15");
}

#[test]
fn input_step_down_default() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'number'; \
         i.value = '10'; \
         i.stepDown(); \
         i.value;");
    assert_eq!(out, "9");
}

#[test]
fn input_step_up_uses_step_attribute() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'number'; \
         i.step = '5'; \
         i.value = '10'; \
         i.stepUp(); \
         i.value;");
    assert_eq!(out, "15");
}

#[test]
fn input_step_up_throws_for_non_numeric_type() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'text'; \
         try { i.stepUp(); 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "InvalidStateError");
}

#[test]
fn input_step_up_with_step_any_throws() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'number'; \
         i.step = 'any'; \
         try { i.stepUp(); 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "InvalidStateError");
}

// --- Selection API gated by supports_selection ---------------------

#[test]
fn input_selection_works_for_text_type() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'text'; \
         i.value = 'hello'; \
         i.setSelectionRange(1, 4); \
         i.selectionStart + '|' + i.selectionEnd;");
    assert_eq!(out, "1|4");
}

#[test]
fn input_selection_throws_for_non_text_type() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'number'; \
         try { i.setSelectionRange(0, 1); 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "InvalidStateError");
}

#[test]
fn input_selection_start_throws_for_checkbox() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'checkbox'; \
         try { var x = i.selectionStart; 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "InvalidStateError");
}

#[test]
fn input_select_method_works_for_password() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'password'; \
         i.value = 'secret'; \
         i.select(); \
         i.selectionStart + '|' + i.selectionEnd;");
    assert_eq!(out, "0|6");
}

#[test]
fn input_set_range_text_works_for_email_type() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'email'; \
         i.value = 'user@example.com'; \
         i.setSelectionRange(0, 4); \
         i.setRangeText('admin'); \
         i.value;");
    assert_eq!(out, "admin@example.com");
}

// --- form / labels -------------------------------------------------

#[test]
fn input_form_resolves_through_ancestor() {
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         f.appendChild(i); \
         document.body.appendChild(f); \
         (i.form === f) ? 'same' : 'other';");
    assert_eq!(out, "same");
}

#[test]
fn input_form_with_empty_form_attribute_suppresses_ancestor_fallback() {
    // HTML §4.10.18.3 — once the `form` content attribute is
    // PRESENT (any value, including empty), the ancestor-fallback
    // path is suppressed.  `form=""` cannot resolve by id-equality
    // so `.form` is null even when an ancestor `<form>` exists.
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         f.appendChild(i); \
         document.body.appendChild(f); \
         i.setAttribute('form', ''); \
         (i.form === null) ? 'null' : 'wrong';");
    assert_eq!(out, "null");
}

#[test]
fn input_label_with_empty_for_attribute_does_not_match_wrapping() {
    // A wrapping `<label for="">` should NOT count as a wrapping
    // ancestor — per HTML §4.10.4 once `for=` is present (any
    // value), the wrapping-ancestor association is suppressed.
    let out = run("var lbl = document.createElement('label'); \
         var i = document.createElement('input'); \
         lbl.setAttribute('for', ''); \
         lbl.appendChild(i); \
         document.body.appendChild(lbl); \
         i.labels.length.toString();");
    assert_eq!(out, "0");
}

#[test]
fn input_labels_only_innermost_wrapping_label_when_nested() {
    // HTML §4.10.4 — for nested `<label>` ancestors, only the
    // INNERMOST wrapping label (no `for=`) claims the control.
    // The outer label's "labeled control" walk is blocked by the
    // inner label.
    let out = run("var outer = document.createElement('label'); \
         var inner = document.createElement('label'); \
         var i = document.createElement('input'); \
         inner.appendChild(i); \
         outer.appendChild(inner); \
         document.body.appendChild(outer); \
         i.labels.length + '|' + (i.labels.item(0) === inner);");
    assert_eq!(out, "1|true");
}

#[test]
fn input_labels_outer_label_with_for_still_associates_via_id() {
    // Nested labels: outer has `for=` matching the input's id,
    // inner is a wrapping label with no `for=`.  Outer associates
    // by id-route, inner via wrapping → both in `.labels`, in
    // tree order.
    let out = run("var outer = document.createElement('label'); \
         outer.htmlFor = 'x'; \
         var inner = document.createElement('label'); \
         var i = document.createElement('input'); \
         i.id = 'x'; \
         inner.appendChild(i); \
         outer.appendChild(inner); \
         document.body.appendChild(outer); \
         i.labels.length + '|' + (i.labels.item(0) === outer) + '|' + (i.labels.item(1) === inner);");
    assert_eq!(out, "2|true|true");
}

#[test]
fn input_labels_collects_for_id_match() {
    let out = run("var i = document.createElement('input'); \
         i.id = 'x'; \
         var lbl = document.createElement('label'); \
         lbl.htmlFor = 'x'; \
         document.body.appendChild(i); \
         document.body.appendChild(lbl); \
         var nl = i.labels; \
         nl.length + '|' + (nl.item(0) === lbl);");
    assert_eq!(out, "1|true");
}

#[test]
fn input_labels_in_document_order_when_id_match_precedes_wrapping() {
    // HTML §4.10.4 — `.labels` is in tree order regardless of which
    // association form (id-based vs wrapping ancestor) matched.  The
    // id-matched label appears first in the document, so it must
    // come before the wrapping ancestor label in `.labels.item(0)`.
    let out = run("var i = document.createElement('input'); \
         i.id = 'x'; \
         var first = document.createElement('label'); \
         first.htmlFor = 'x'; \
         document.body.appendChild(first); \
         var wrap = document.createElement('label'); \
         wrap.appendChild(i); \
         document.body.appendChild(wrap); \
         var nl = i.labels; \
         nl.length + '|' + (nl.item(0) === first) + '|' + (nl.item(1) === wrap);");
    assert_eq!(out, "2|true|true");
}

#[test]
fn input_labels_in_document_order_when_wrapping_precedes_id_match() {
    // Reverse: wrapping label is earlier in the tree, so it should
    // come first in `.labels`.
    let out = run("var i = document.createElement('input'); \
         i.id = 'x'; \
         var wrap = document.createElement('label'); \
         wrap.appendChild(i); \
         document.body.appendChild(wrap); \
         var second = document.createElement('label'); \
         second.htmlFor = 'x'; \
         document.body.appendChild(second); \
         var nl = i.labels; \
         nl.length + '|' + (nl.item(0) === wrap) + '|' + (nl.item(1) === second);");
    assert_eq!(out, "2|true|true");
}

// --- Deferred stubs -----------------------------------------------

#[test]
fn input_files_returns_null_for_non_file_type() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'text'; \
         (i.files === null) ? 'null' : 'wrong';");
    assert_eq!(out, "null");
}

#[test]
fn input_files_throws_invalid_state_for_file_type() {
    let out = run("var i = document.createElement('input'); \
         i.type = 'file'; \
         try { var x = i.files; 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "InvalidStateError");
}

#[test]
fn input_list_returns_null_stub() {
    let out = run("var i = document.createElement('input'); \
         i.setAttribute('list', 'somedl'); \
         (i.list === null) ? 'null' : 'wrong';");
    assert_eq!(out, "null");
}

#[test]
fn input_show_picker_throws_not_supported() {
    let out = run("var i = document.createElement('input'); \
         try { i.showPicker(); 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "NotSupportedError");
}

// --- Brand check ---------------------------------------------------

#[test]
fn input_value_throws_on_non_input_receiver() {
    let out = run("var i = document.createElement('input'); \
         var div = document.createElement('div'); \
         var getter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(i), 'value').get; \
         try { getter.call(div); 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "TypeError");
}
