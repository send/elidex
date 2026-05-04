//! M4-12 slot #11-tags-T1 Phase 6 — `HTMLTextAreaElement.prototype` tests.
//!
//! Covers reflected attributes (autocomplete, cols, dirName,
//! disabled, maxLength, minLength, name, placeholder, readOnly,
//! required, rows, wrap), `value` / `defaultValue` / `textLength`,
//! `form` / `labels` derived getters, and the Selection API
//! (`selectionStart` / `selectionEnd` / `selectionDirection` /
//! `select()` / `setRangeText()` / `setSelectionRange()`).

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

// --- Prototype chain identity --------------------------------------

#[test]
fn textarea_wrapper_has_html_textarea_prototype() {
    let out = run("var t1 = document.createElement('textarea'); \
         var t2 = document.createElement('textarea'); \
         var proto = Object.getPrototypeOf(t1); \
         var same = Object.getPrototypeOf(t2) === proto; \
         var hasValue = Object.getOwnPropertyDescriptor(proto, 'value') !== undefined; \
         var hasSel = Object.getOwnPropertyDescriptor(proto, 'selectionStart') !== undefined; \
         (same && hasValue && hasSel) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn textarea_chains_to_html_element_prototype() {
    let out = run("var t = document.createElement('textarea'); \
         var p1 = Object.getPrototypeOf(t); \
         var p2 = Object.getPrototypeOf(p1); \
         (p2 === Object.getPrototypeOf(document.createElement('div'))) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// --- String reflected attrs ----------------------------------------

#[test]
fn textarea_string_attrs_round_trip() {
    let out = run("var t = document.createElement('textarea'); \
         t.autocomplete = 'on'; \
         t.dirName = 'comment.dir'; \
         t.name = 'msg'; \
         t.placeholder = 'enter text'; \
         t.wrap = 'soft'; \
         t.autocomplete + '|' + t.dirName + '|' + t.name + '|' + t.placeholder + '|' + t.wrap;");
    assert_eq!(out, "on|comment.dir|msg|enter text|soft");
}

#[test]
fn textarea_dir_name_uses_lowercased_attribute_name() {
    let out = run("var t = document.createElement('textarea'); \
         t.dirName = 'x.d'; \
         t.getAttribute('dirname');");
    assert_eq!(out, "x.d");
}

// --- Boolean reflects ----------------------------------------------

#[test]
fn textarea_disabled_round_trip() {
    let out = run("var t = document.createElement('textarea'); \
         t.disabled = true; \
         var on = t.disabled + '|' + t.hasAttribute('disabled'); \
         t.disabled = false; \
         var off = t.disabled + '|' + t.hasAttribute('disabled'); \
         on + '/' + off;");
    assert_eq!(out, "true|true/false|false");
}

#[test]
fn textarea_read_only_uses_lowercased_attribute_name() {
    let out = run("var t = document.createElement('textarea'); \
         t.readOnly = true; \
         t.hasAttribute('readonly') + '|' + t.readOnly;");
    assert_eq!(out, "true|true");
}

#[test]
fn textarea_required_round_trip() {
    let out = run("var t = document.createElement('textarea'); \
         t.required = true; \
         var on = t.required; \
         t.required = false; \
         on + '|' + t.required;");
    assert_eq!(out, "true|false");
}

// --- Numeric reflects ----------------------------------------------

#[test]
fn textarea_cols_default_is_20() {
    let out = run("var t = document.createElement('textarea'); \
         t.cols.toString();");
    assert_eq!(out, "20");
}

#[test]
fn textarea_rows_default_is_2() {
    let out = run("var t = document.createElement('textarea'); \
         t.rows.toString();");
    assert_eq!(out, "2");
}

#[test]
fn textarea_cols_round_trip() {
    let out = run("var t = document.createElement('textarea'); \
         t.cols = 80; \
         t.cols + '|' + t.getAttribute('cols');");
    assert_eq!(out, "80|80");
}

#[test]
fn textarea_rows_round_trip() {
    let out = run("var t = document.createElement('textarea'); \
         t.rows = 10; \
         t.rows + '|' + t.getAttribute('rows');");
    assert_eq!(out, "10|10");
}

#[test]
fn textarea_cols_zero_throws_index_size_error() {
    // HTML §4.10.11.4 — setting cols to 0 throws IndexSizeError.
    let out = run("var t = document.createElement('textarea'); \
         try { t.cols = 0; 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "IndexSizeError");
}

#[test]
fn textarea_rows_zero_throws_index_size_error() {
    // HTML §4.10.11.4 — setting rows to 0 throws IndexSizeError.
    let out = run("var t = document.createElement('textarea'); \
         try { t.rows = 0; 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "IndexSizeError");
}

#[test]
fn textarea_max_length_default_is_negative_one() {
    let out = run("var t = document.createElement('textarea'); \
         t.maxLength.toString();");
    assert_eq!(out, "-1");
}

#[test]
fn textarea_max_length_round_trip() {
    let out = run("var t = document.createElement('textarea'); \
         t.maxLength = 100; \
         t.maxLength + '|' + t.getAttribute('maxlength');");
    assert_eq!(out, "100|100");
}

#[test]
fn textarea_min_length_default_is_negative_one() {
    let out = run("var t = document.createElement('textarea'); \
         t.minLength.toString();");
    assert_eq!(out, "-1");
}

#[test]
fn textarea_max_length_negative_throws_index_size_error() {
    // HTML §6.13.1 reflect rule "limited to only non-negative
    // numbers" — negative values throw IndexSizeError.
    let out = run("var t = document.createElement('textarea'); \
         try { t.maxLength = -5; 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "IndexSizeError");
}

#[test]
fn textarea_min_length_negative_throws_index_size_error() {
    let out = run("var t = document.createElement('textarea'); \
         try { t.minLength = -2; 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "IndexSizeError");
}

// --- value / defaultValue / textLength -----------------------------

#[test]
fn textarea_default_value_falls_back_to_text_content() {
    let out = run("var t = document.createElement('textarea'); \
         var txt = document.createTextNode('hello'); \
         t.appendChild(txt); \
         t.defaultValue + '|' + t.value;");
    assert_eq!(out, "hello|hello");
}

#[test]
fn textarea_value_overrides_default_value_when_set() {
    let out = run("var t = document.createElement('textarea'); \
         var txt = document.createTextNode('default'); \
         t.appendChild(txt); \
         t.value = 'override'; \
         t.value + '|' + t.defaultValue;");
    assert_eq!(out, "override|default");
}

#[test]
fn textarea_default_value_setter_replaces_children_with_text() {
    let out = run("var t = document.createElement('textarea'); \
         t.defaultValue = 'fresh'; \
         t.defaultValue + '|' + t.childNodes.length;");
    assert_eq!(out, "fresh|1");
}

#[test]
fn textarea_text_length_counts_value() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'abcde'; \
         t.textLength.toString();");
    assert_eq!(out, "5");
}

#[test]
fn textarea_text_length_uses_utf16_units_for_supplementary_planes() {
    // U+1F600 (😀) is a supplementary-plane code point — 1 char in
    // Rust but 2 UTF-16 code units, matching the spec's `textLength`
    // length semantics (HTML §4.10.18.7).
    let out = run("var t = document.createElement('textarea'); \
         t.value = '\u{1f600}'; \
         t.textLength.toString();");
    assert_eq!(out, "2");
}

// --- form / labels -------------------------------------------------

#[test]
fn textarea_form_resolves_through_ancestor() {
    let out = run("var f = document.createElement('form'); \
         var t = document.createElement('textarea'); \
         f.appendChild(t); \
         document.body.appendChild(f); \
         (t.form === f) ? 'same' : 'other';");
    assert_eq!(out, "same");
}

#[test]
fn textarea_labels_collects_for_id_match() {
    let out = run("var t = document.createElement('textarea'); \
         t.id = 'msg'; \
         var lbl = document.createElement('label'); \
         lbl.htmlFor = 'msg'; \
         document.body.appendChild(t); \
         document.body.appendChild(lbl); \
         var nl = t.labels; \
         nl.length + '|' + (nl.item(0) === lbl ? 'same' : 'other');");
    assert_eq!(out, "1|same");
}

#[test]
fn textarea_labels_returns_empty_node_list_when_none() {
    let out = run("var t = document.createElement('textarea'); \
         var nl = t.labels; \
         nl.length + '|' + (typeof nl.item === 'function');");
    assert_eq!(out, "0|true");
}

// --- Selection API -------------------------------------------------

#[test]
fn textarea_selection_defaults_to_zero_zero_none() {
    let out = run("var t = document.createElement('textarea'); \
         t.selectionStart + '|' + t.selectionEnd + '|' + t.selectionDirection;");
    assert_eq!(out, "0|0|none");
}

#[test]
fn textarea_value_setter_resets_selection_to_end() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'hello'; \
         t.selectionStart + '|' + t.selectionEnd;");
    assert_eq!(out, "5|5");
}

#[test]
fn textarea_select_method_selects_all() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'hello world'; \
         t.select(); \
         t.selectionStart + '|' + t.selectionEnd + '|' + t.selectionDirection;");
    assert_eq!(out, "0|11|none");
}

#[test]
fn textarea_set_selection_range_basic() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'hello world'; \
         t.setSelectionRange(2, 7, 'forward'); \
         t.selectionStart + '|' + t.selectionEnd + '|' + t.selectionDirection;");
    assert_eq!(out, "2|7|forward");
}

#[test]
fn textarea_set_selection_range_clamps_start_above_end() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'hello'; \
         t.setSelectionRange(8, 3); \
         t.selectionStart + '|' + t.selectionEnd;");
    // start is clamped to value length (5), end clamped to value
    // length (3 < 5 OK), then start > end → end := start.
    assert_eq!(out, "5|5");
}

#[test]
fn textarea_selection_start_setter_clamps_end_upward() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'hello world'; \
         t.setSelectionRange(2, 5); \
         t.selectionStart = 8; \
         t.selectionStart + '|' + t.selectionEnd;");
    assert_eq!(out, "8|8");
}

#[test]
fn textarea_selection_end_setter_clamps_to_value_length() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'abc'; \
         t.selectionEnd = 100; \
         t.selectionEnd.toString();");
    assert_eq!(out, "3");
}

#[test]
fn textarea_selection_direction_setter_unknown_maps_to_none() {
    let out = run("var t = document.createElement('textarea'); \
         t.setSelectionRange(0, 0, 'forward'); \
         t.selectionDirection = 'sideways'; \
         t.selectionDirection;");
    assert_eq!(out, "none");
}

#[test]
fn textarea_set_range_text_replaces_selection_in_preserve_mode() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'hello world'; \
         t.setSelectionRange(6, 11); \
         t.setRangeText('JS'); \
         t.value + '|' + t.selectionStart + '|' + t.selectionEnd;");
    // Selection [6, 11) (= 'world') replaced with 'JS' (length 2)
    // → "hello JS".  Per HTML §4.10.18.7 "preserve":
    //   - selStart=6 is NOT > end (11), NOT > start (6) → unchanged.
    //   - selEnd=11 is NOT > end (11), IS > start (6) → set to
    //     start (6).
    // Result: collapsed selection at 6.
    assert_eq!(out, "hello JS|6|6");
}

#[test]
fn textarea_set_range_text_with_explicit_range_select_mode() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'hello world'; \
         t.setRangeText('JS', 6, 11, 'select'); \
         t.value + '|' + t.selectionStart + '|' + t.selectionEnd;");
    assert_eq!(out, "hello JS|6|8");
}

#[test]
fn textarea_set_range_text_throws_for_start_greater_than_end() {
    // HTML §4.10.18.7 step 2 — IndexSizeError when start > end.
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'hello'; \
         try { t.setRangeText('x', 4, 1); 'no throw'; } \
         catch (e) { e.name; }");
    assert_eq!(out, "IndexSizeError");
}

#[test]
fn textarea_set_range_text_explicit_end_mode() {
    let out = run("var t = document.createElement('textarea'); \
         t.value = 'hello world'; \
         t.setRangeText('JS', 6, 11, 'end'); \
         t.value + '|' + t.selectionStart + '|' + t.selectionEnd;");
    // After splice, "hello JS" — end mode places cursor at start +
    // replacement length = 8, collapsed.
    assert_eq!(out, "hello JS|8|8");
}

// --- Brand check ---------------------------------------------------

#[test]
fn textarea_value_throws_on_non_textarea_receiver() {
    let out = run("var t = document.createElement('textarea'); \
         var div = document.createElement('div'); \
         var getter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(t), 'value').get; \
         try { getter.call(div); 'no throw'; } catch (e) { e.name + ':' + (e.message.indexOf('Illegal invocation') >= 0); }");
    assert_eq!(out, "TypeError:true");
}

#[test]
fn textarea_select_method_throws_on_non_textarea_receiver() {
    let out = run("var t = document.createElement('textarea'); \
         var div = document.createElement('div'); \
         var fn = Object.getPrototypeOf(t).select; \
         try { fn.call(div); 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "TypeError");
}
