//! UTF-16 code-unit selection offsets (HTML §4.10.20) for `<input>`.
//!
//! Selection offsets crossing the IDL surface are measured in UTF-16 code
//! units of the relevant value, while `FormControlState` stores byte offsets
//! internally.  These exercise the byte↔UTF-16 conversion at the
//! `selection_api.rs` boundary for BMP-multibyte ("café", `é` = 2 bytes /
//! 1 unit), 3-byte CJK ("あいう", 3 bytes / 1 unit each) and astral ("𠮷"
//! U+20BB7, 4 bytes / 2 units) text.  Split out of `tests_html_input_proto.rs`
//! to keep that file under the 1000-line threshold (Codex PR#362 R1 P3).

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
fn input_selection_getter_returns_utf16_units_bmp() {
    // "café" = 5 bytes / 4 UTF-16 units.  Caret at the end is unit 4
    // (NOT byte 5).  Pre-conversion this returned the byte offset 5.
    let out = run("var i = document.createElement('input'); \
         i.value = 'café'; \
         i.setSelectionRange(4, 4); \
         '' + i.selectionStart + '/' + i.selectionEnd;");
    assert_eq!(out, "4/4");
}

#[test]
fn input_selection_range_whole_value_utf16_units_bmp() {
    // Whole-value selection of "café": end is unit 4, not byte 5.
    let out = run("var i = document.createElement('input'); \
         i.value = 'café'; \
         i.setSelectionRange(0, 4); \
         '' + i.selectionStart + '/' + i.selectionEnd;");
    assert_eq!(out, "0/4");
}

#[test]
fn input_selection_range_cjk_units() {
    // "あいう" = 9 bytes / 3 units (each char 3 bytes, 1 unit).
    // setSelectionRange(1, 2) selects 'い'; getters report units 1/2.
    let out = run("var i = document.createElement('input'); \
         i.value = 'あいう'; \
         i.setSelectionRange(1, 2); \
         '' + i.selectionStart + '/' + i.selectionEnd;");
    assert_eq!(out, "1/2");
}

#[test]
fn input_selection_range_astral_surrogate_pair_units() {
    // "𠮷" (U+20BB7) = 4 bytes / 2 UTF-16 units (surrogate pair).
    // The whole char spans units 0..2.
    let out = run("var i = document.createElement('input'); \
         i.value = '𠮷'; \
         i.setSelectionRange(0, 2); \
         '' + i.selectionStart + '/' + i.selectionEnd;");
    assert_eq!(out, "0/2");
}

#[test]
fn input_set_range_text_splices_at_utf16_offsets() {
    // "café": replace unit range [3,4) ('é') with "X" → "cafX".  The
    // resulting collapsed caret is reported as unit 4.
    let out = run("var i = document.createElement('input'); \
         i.value = 'café'; \
         i.setRangeText('X', 3, 4); \
         i.value + '|' + i.selectionStart + '/' + i.selectionEnd;");
    assert_eq!(out, "cafX|4/4");
}

#[test]
fn input_set_selection_range_clamps_past_end_in_units() {
    // "café" is 4 UTF-16 units; an over-length offset clamps to the
    // end in units (4), not to the byte length (5).
    let out = run("var i = document.createElement('input'); \
         i.value = 'café'; \
         i.setSelectionRange(99, 99); \
         '' + i.selectionStart + '/' + i.selectionEnd;");
    assert_eq!(out, "4/4");
}

#[test]
fn input_selection_offset_inside_surrogate_pair_snaps_to_char_start() {
    // §4.10.20 offsets are UTF-16 code units, but the internal store is
    // byte-based: an offset that splits a surrogate pair ('𠮷' spans units
    // 1..3 of "a𠮷b") is not representable as a byte offset, so a caret set
    // to the mid-pair unit 2 snaps DOWN to the char start (unit 1) — a
    // deterministic, documented consequence of byte-internal storage.
    // Full mid-surrogate fidelity is deferred to UTF-16-internal selection
    // storage (slot #11-selection-mid-surrogate-fidelity).
    let out = run("var i = document.createElement('input'); \
         i.value = 'a𠮷b'; \
         i.setSelectionRange(2, 2); \
         '' + i.selectionStart + '/' + i.selectionEnd;");
    assert_eq!(out, "1/1");
}

#[test]
fn input_set_range_text_no_args_uses_byte_fallback_selection() {
    // §4.10.20 mixed-unit hazard: setRangeText() with no start/end uses
    // the CURRENT selection (already byte offsets internally) — it must
    // not be double-converted.  setSelectionRange(1, 3) selects "af"
    // (units == bytes here for the ASCII prefix); setRangeText("Z")
    // replaces it → "cZé".
    let out = run("var i = document.createElement('input'); \
         i.value = 'café'; \
         i.setSelectionRange(1, 3); \
         i.setRangeText('Z'); \
         i.value;");
    assert_eq!(out, "cZé");
}
