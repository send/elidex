//! Change-on-blur user provenance (HTML §6.6.4 focus update steps step 2.1,
//! §4.10.5.5 Common event behaviors): `change` fires at a blurred text control
//! only for a value the **user** changed since focus, and `input` fires for a
//! user action only when it changed the value.
//!
//! Plan: `docs/plans/2026-11-change-on-blur-user-provenance.md` §6 (cells
//! C1–C29, model H of §4.3). User input is driven through `handle_key` /
//! `handle_ime`, UA focus moves through `content::focus::set_focus` /
//! `blur_current`, script steps through `eval_script`. Events are observed by
//! listeners that append to a log mirrored into `<body data-log>`, read back
//! Rust-side with `probe_body_attr` (no extra script bracket).

use elidex_form::FormControlState;

use super::event_handlers::handle_key;
use super::focus::{blur_current, set_focus};
use super::ime::handle_ime;
use super::test_support::{build_test_content_state, probe_body_attr, set_test_clipboard};
use super::ContentState;
use crate::ipc::{BrowserToContent, ContentToBrowser, ImeKind, LocalChannel, ModifierState};

type Browser = LocalChannel<BrowserToContent, ContentToBrowser>;

/// The events every logged control reports. `input` carries its `inputType`.
/// (`listen` takes `t` as a parameter so each listener closes over its own type: the VM's
/// `for (const …)` loop shares one binding across iterations, `#11-vm-per-iteration-loop-environment`;
/// inline the loop body again when that slot lands.)
const LOGGER: &str = "globalThis.__log = []; \
    function listen(el, t) { \
      el.addEventListener(t, (e) => { \
        __log.push(t === 'input' ? 'input:' + e.inputType : t); \
        document.body.setAttribute('data-log', __log.join(',')); \
      }); \
    } \
    for (const el of document.querySelectorAll('input, textarea')) { \
      for (const t of ['input', 'change', 'blur', 'focusout', 'compositionend']) { \
        listen(el, t); \
      } \
    }";

/// Build a content state over `body` (inner HTML of `<body>`) and install the
/// event logger on every `<input>` / `<textarea>`.
fn setup(body: &str) -> (ContentState, Browser) {
    let html = format!("<!DOCTYPE html><html><body>{body}</body></html>");
    let (mut state, browser) = build_test_content_state(&html, "");
    eval(&mut state, LOGGER);
    (state, browser)
}

/// `eval_script`, asserting the script completed.
fn eval(state: &mut ContentState, src: &str) {
    let r = state.pipeline.eval_script(src);
    assert!(r.success, "eval failed: {src}: {:?}", r.error);
}

fn entity(state: &ContentState, id: &str) -> elidex_ecs::Entity {
    state
        .pipeline
        .dom
        .find_by_id(state.pipeline.document, id)
        .unwrap_or_else(|| panic!("no element #{id}"))
}

/// The UA focus path (`set_focus`, the shell sink's canonical transition).
fn ua_focus(state: &mut ContentState, id: &str) {
    let e = entity(state, id);
    set_focus(&mut state.pipeline, e);
}

/// A user key press (`keydown`, no modifiers) at the focused element.
fn key(state: &mut ContentState, k: &str) {
    handle_key(state, "keydown", k, "", false, ModifierState::default());
}

/// A user Cmd+`k` (clipboard / select-all chords) at the focused element.
fn cmd_key(state: &mut ContentState, k: &str) {
    let mods = ModifierState {
        meta: true,
        ..ModifierState::default()
    };
    handle_key(state, "keydown", k, "", false, mods);
}

/// The logged events so far, comma-joined (`""` when none fired).
fn log(state: &ContentState) -> String {
    probe_body_attr(&state.pipeline, "data-log").unwrap_or_default()
}

/// Clear the event log.
fn clear_log(state: &mut ContentState) {
    eval(
        state,
        "__log = []; document.body.setAttribute('data-log', '');",
    );
}

/// Read `#id`'s [`FormControlState`] through `f`.
fn with_fcs<T>(state: &ContentState, id: &str, f: impl FnOnce(&FormControlState) -> T) -> T {
    let e = entity(state, id);
    let fcs = state
        .pipeline
        .dom
        .world()
        .get::<&FormControlState>(e)
        .expect("form control state");
    f(&fcs)
}

fn value(state: &ContentState, id: &str) -> String {
    with_fcs(state, id, |f| f.value().to_owned())
}

/// Whether the log holds an event named `name` (an `input:<inputType>` entry
/// counts as `input`).
fn has_event(state: &ContentState, name: &str) -> bool {
    log(state)
        .split(',')
        .any(|t| t.split(':').next() == Some(name))
}

/// A script `#b.value = v`.
fn script_set_value(state: &mut ContentState, v: &str) {
    eval(
        state,
        &format!("document.getElementById('b').value = '{v}';"),
    );
}

/// A script `#b.setSelectionRange(a, b)`.
fn select(state: &mut ContentState, a: usize, b: usize) {
    eval(
        state,
        &format!("document.getElementById('b').setSelectionRange({a}, {b});"),
    );
}

fn has_change(state: &ContentState) -> bool {
    has_event(state, "change")
}

fn has_input(state: &ContentState) -> bool {
    has_event(state, "input")
}

/// `<b>` (the control under test, `tag`) followed by a plain `<input id=c>`.
fn pair(tag: &str, attrs: &str, content: &str) -> String {
    if tag == "textarea" {
        format!("<textarea id=b {attrs}>{content}</textarea><input id=c>")
    } else {
        format!("<input id=b {attrs} value=\"{content}\"><input id=c>")
    }
}

// ---- Commit (1): controls green at HEAD ----------------------------------

/// C2: a user edit then a UA blur fires `input`, `change`, `blur`, `focusout`.
#[test]
fn c2_ua_blur_change_for_user_edit() {
    let (mut s, _b) = setup(&pair("input", "", ""));
    ua_focus(&mut s, "b");
    clear_log(&mut s);
    key(&mut s, "a");
    ua_focus(&mut s, "c");
    assert_eq!(log(&s), "input:insertText,change,blur,focusout");
}

/// C4 (M1): user edit, then a script write; `change` fires.
fn c4(tag: &str) {
    let (mut s, _b) = setup(&pair(tag, "", ""));
    ua_focus(&mut s, "b");
    key(&mut s, "a");
    script_set_value(&mut s, "third");
    clear_log(&mut s);
    ua_focus(&mut s, "c");
    assert!(has_change(&s), "M1: change fires ({})", log(&s));
}

#[test]
fn c4_ua_blur_after_user_edit_then_script_write_input() {
    c4("input");
}

#[test]
fn c4_ua_blur_after_user_edit_then_script_write_textarea() {
    c4("textarea");
}

/// C5 (M2): user edit, then a script write restoring the focus-time value; no `change`.
fn c5(tag: &str) {
    let (mut s, _b) = setup(&pair(tag, "", ""));
    ua_focus(&mut s, "b");
    key(&mut s, "a");
    script_set_value(&mut s, "");
    clear_log(&mut s);
    ua_focus(&mut s, "c");
    assert!(!has_change(&s), "M2: no change ({})", log(&s));
}

#[test]
fn c5_ua_blur_after_user_edit_then_script_restore_input() {
    c5("input");
}

#[test]
fn c5_ua_blur_after_user_edit_then_script_restore_textarea() {
    c5("textarea");
}

/// C10: an IME commit fires `compositionend`, `input`, and marks (blur → `change`).
#[test]
fn c10_ime_commit_marks_and_fires_input() {
    let (mut s, _b) = setup(&pair("input", "", ""));
    ua_focus(&mut s, "b");
    clear_log(&mut s);
    handle_ime(&mut s, ImeKind::Commit("あ".to_string()));
    assert_eq!(log(&s), "compositionend,input:insertFromComposition");
    ua_focus(&mut s, "c");
    assert!(has_change(&s), "IME commit is a user change ({})", log(&s));
}

/// C13: VM `focus()` seeds; a user edit then a UA blur fires `change`.
#[test]
fn c13_vm_focus_then_user_edit_fires_change() {
    let (mut s, _b) = setup(&pair("input", "", ""));
    eval(&mut s, "document.getElementById('b').focus();");
    key(&mut s, "a");
    clear_log(&mut s);
    ua_focus(&mut s, "c");
    assert!(has_change(&s), "{}", log(&s));
}

/// C15: C2 on `<textarea>` with Enter.
#[test]
fn c15_textarea_user_edit_fires_change() {
    let (mut s, _b) = setup(&pair("textarea", "", ""));
    ua_focus(&mut s, "b");
    clear_log(&mut s);
    key(&mut s, "Enter");
    ua_focus(&mut s, "c");
    assert_eq!(log(&s), "input:insertLineBreak,change,blur,focusout");
}

/// C17 (M6): script `x`; type `a`; script `y`; blur → `change`.
fn c17(tag: &str) {
    let (mut s, _b) = setup(&pair(tag, "", ""));
    ua_focus(&mut s, "b");
    script_set_value(&mut s, "x");
    key(&mut s, "a");
    script_set_value(&mut s, "y");
    clear_log(&mut s);
    ua_focus(&mut s, "c");
    assert!(has_change(&s), "M6: change ({})", log(&s));
}

#[test]
fn c17_script_write_user_edit_script_write_input() {
    c17("input");
}

#[test]
fn c17_script_write_user_edit_script_write_textarea() {
    c17("textarea");
}

/// C18 (M7): type `a`; Backspace; blur → no `change`.
fn c18(tag: &str) {
    let (mut s, _b) = setup(&pair(tag, "", ""));
    ua_focus(&mut s, "b");
    key(&mut s, "a");
    key(&mut s, "Backspace");
    assert_eq!(value(&s, "b"), "");
    clear_log(&mut s);
    ua_focus(&mut s, "c");
    assert!(!has_change(&s), "M7: no change ({})", log(&s));
}

#[test]
fn c18_user_edit_then_revert_input() {
    c18("input");
}

#[test]
fn c18_user_edit_then_revert_textarea() {
    c18("textarea");
}

/// C19 (M8): type `a`; blur (`change`); focus; script `x`; type `b`; blur (`change`).
fn c19(tag: &str) {
    let (mut s, _b) = setup(&pair(tag, "", ""));
    ua_focus(&mut s, "b");
    key(&mut s, "a");
    clear_log(&mut s);
    ua_focus(&mut s, "c");
    assert!(has_change(&s), "M8 first blur ({})", log(&s));
    ua_focus(&mut s, "b");
    script_set_value(&mut s, "x");
    key(&mut s, "b");
    clear_log(&mut s);
    ua_focus(&mut s, "c");
    assert!(has_change(&s), "M8 second blur ({})", log(&s));
}

#[test]
fn c19_second_focus_rebaselines_before_user_edit_input() {
    c19("input");
}

#[test]
fn c19_second_focus_rebaselines_before_user_edit_textarea() {
    c19("textarea");
}

// ---- Commit (2): `input` iff the user changed the value -------------------

/// The caret position of `#id`.
fn cursor(state: &ContentState, id: &str) -> usize {
    with_fcs(state, id, FormControlState::cursor_pos)
}

/// A script `value = "ab"` before focus leaves the caret at the end (the
/// setter's cursor move), so ArrowLeft has somewhere to go — the setup of the
/// Chromium-measured N1 / M5 cases. (A collapsed VM `setSelectionRange(1, 1)`
/// does not move elidex's caret, `cursor_pos`, so it cannot set this up:
/// `#11-text-control-set-selection-range-caret`.)
fn focus_with_caret_at_end(state: &mut ContentState) {
    script_set_value(state, "ab");
    ua_focus(state, "b");
    assert_eq!(cursor(state, "b"), 2, "caret at the end");
}

/// C8 (N1): ArrowLeft moves the caret but leaves the value; no `input`.
#[test]
fn c8_arrow_key_fires_no_input() {
    let (mut s, _b) = setup(&pair("input", "", ""));
    focus_with_caret_at_end(&mut s);
    clear_log(&mut s);
    key(&mut s, "ArrowLeft");
    assert_eq!(cursor(&s, "b"), 1, "the arrow moved the caret");
    assert_eq!(value(&s, "b"), "ab");
    assert!(!has_input(&s), "no input for a caret move ({})", log(&s));
}

/// C11: an IME commit into a full `maxlength` control inserts nothing; no `input`.
#[test]
fn c11_ime_commit_with_full_maxlength_fires_no_input() {
    let (mut s, _b) = setup(&pair("input", "maxlength=1", "z"));
    ua_focus(&mut s, "b");
    select(&mut s, 1, 1);
    clear_log(&mut s);
    handle_ime(&mut s, ImeKind::Commit("あ".to_string()));
    assert_eq!(value(&s, "b"), "z");
    assert_eq!(log(&s), "compositionend", "compositionend only");
}

/// C12 (N2): ArrowLeft in a `readonly` control moves the caret; no `input`.
#[test]
fn c12_readonly_navigation_fires_no_input() {
    let (mut s, _b) = setup(&pair("input", "readonly", ""));
    focus_with_caret_at_end(&mut s);
    clear_log(&mut s);
    key(&mut s, "ArrowLeft");
    assert_eq!(cursor(&s, "b"), 1, "readonly navigation moved the caret");
    assert_eq!(value(&s, "b"), "ab");
    assert!(!has_input(&s), "no input ({})", log(&s));
}

/// C27: paste into a `readonly` control leaves the value; no `input`.
fn c27(tag: &str) {
    let (mut s, _b) = setup(&pair(tag, "readonly", "ab"));
    set_test_clipboard("q");
    ua_focus(&mut s, "b");
    clear_log(&mut s);
    cmd_key(&mut s, "v");
    assert_eq!(value(&s, "b"), "ab");
    assert!(!has_input(&s), "no input ({})", log(&s));
}

#[test]
fn c27_paste_into_readonly_fires_no_input_input() {
    c27("input");
}

#[test]
fn c27_paste_into_readonly_fires_no_input_textarea() {
    c27("textarea");
}

/// C28: paste into a full `maxlength` control leaves the value; no `input`.
fn c28(tag: &str) {
    let (mut s, _b) = setup(&pair(tag, "maxlength=1", "z"));
    set_test_clipboard("q");
    ua_focus(&mut s, "b");
    select(&mut s, 1, 1);
    clear_log(&mut s);
    cmd_key(&mut s, "v");
    assert_eq!(value(&s, "b"), "z");
    assert!(!has_input(&s), "no input ({})", log(&s));
}

#[test]
fn c28_paste_into_full_maxlength_fires_no_input_input() {
    c28("input");
}

#[test]
fn c28_paste_into_full_maxlength_fires_no_input_textarea() {
    c28("textarea");
}

/// C30: the positive control for §4.2 row 3 (C27 / C28 are negative only) — a
/// paste that changes the value fires `input` (`insertFromPaste`), marks, and the
/// following blur fires `change`.
#[test]
fn c30_paste_changes_value_fires_input_and_change() {
    let (mut s, _b) = setup(&pair("input", "", ""));
    set_test_clipboard("hi");
    ua_focus(&mut s, "b");
    clear_log(&mut s);
    cmd_key(&mut s, "v");
    assert_eq!(value(&s, "b"), "hi");
    assert_eq!(log(&s), "input:insertFromPaste");
    ua_focus(&mut s, "c");
    assert!(has_change(&s), "paste is a user change ({})", log(&s));
}

/// C29: cut writes the clipboard, fires `input` (`deleteByCut`) and marks.
#[test]
fn c29_cut_marks_and_fires_input() {
    let (mut s, _b) = setup(&pair("input", "", "ab"));
    set_test_clipboard("");
    ua_focus(&mut s, "b");
    select(&mut s, 0, 1);
    clear_log(&mut s);
    cmd_key(&mut s, "x");
    assert_eq!(
        super::test_support::test_clipboard_text().as_deref(),
        Some("a")
    );
    assert_eq!(value(&s, "b"), "b");
    assert_eq!(log(&s), "input:deleteByCut");
    ua_focus(&mut s, "c");
    assert!(has_change(&s), "cut is a user change ({})", log(&s));
}

// ---- Commit (3): the mark decides, and the first mark re-baselines --------

/// C1 (M0) / C14: a script write to a UA-focused control, then a UA blur; no `change`.
fn c1(tag: &str) {
    let (mut s, _b) = setup(&pair(tag, "", ""));
    ua_focus(&mut s, "b");
    script_set_value(&mut s, "x");
    clear_log(&mut s);
    ua_focus(&mut s, "c");
    assert!(has_event(&s, "blur"), "{}", log(&s));
    assert!(!has_change(&s), "M0: no change ({})", log(&s));
}

#[test]
fn c1_ua_blur_no_change_for_script_written_value() {
    c1("input");
}

/// C14: C1 on `<textarea>`.
#[test]
fn c14_textarea_ua_blur_no_change_for_script_written_value() {
    c1("textarea");
}

/// C3: C1 with the blur through `blur_current` (the unfocusing steps).
#[test]
fn c3_blur_current_no_change_for_script_written_value() {
    let (mut s, _b) = setup(&pair("input", "", ""));
    ua_focus(&mut s, "b");
    script_set_value(&mut s, "x");
    clear_log(&mut s);
    blur_current(&mut s.pipeline);
    assert!(has_event(&s, "blur"), "{}", log(&s));
    assert!(!has_change(&s), "M0: no change ({})", log(&s));
}

/// C6 (M4): script `x`; user Backspace (value back to `""`); blur → `change`.
fn c6(tag: &str) {
    let (mut s, _b) = setup(&pair(tag, "", ""));
    ua_focus(&mut s, "b");
    script_set_value(&mut s, "x");
    key(&mut s, "Backspace");
    assert_eq!(value(&s, "b"), "");
    clear_log(&mut s);
    ua_focus(&mut s, "c");
    assert!(has_change(&s), "M4: change ({})", log(&s));
}

#[test]
fn c6_ua_blur_after_script_write_then_user_revert_input() {
    c6("input");
}

#[test]
fn c6_ua_blur_after_script_write_then_user_revert_textarea() {
    c6("textarea");
}

/// C9 (M5): ArrowLeft, then a script write, then a UA blur; no `change`.
#[test]
fn c9_arrow_key_then_script_write_no_change() {
    let (mut s, _b) = setup(&pair("input", "", ""));
    focus_with_caret_at_end(&mut s);
    key(&mut s, "ArrowLeft");
    assert_eq!(cursor(&s, "b"), 1, "the arrow moved the caret");
    script_set_value(&mut s, "x");
    clear_log(&mut s);
    ua_focus(&mut s, "c");
    assert!(!has_change(&s), "M5: no change ({})", log(&s));
}

/// C16: a mark left by VM `blur()` does not survive the VM `focus()` re-seed.
#[test]
fn c16_stale_mark_does_not_survive_refocus() {
    let (mut s, _b) = setup(&pair("input", "", ""));
    ua_focus(&mut s, "b");
    key(&mut s, "a");
    eval(
        &mut s,
        "const b = document.getElementById('b'); b.blur(); b.focus(); b.value = 'x';",
    );
    clear_log(&mut s);
    ua_focus(&mut s, "c");
    assert!(!has_change(&s), "no change ({})", log(&s));
}

/// C20 (M9, W1 / W2): the user returns to the script-written value; no second `change`.
fn c20(tag: &str) {
    let (mut s, _b) = setup(&pair(tag, "", ""));
    ua_focus(&mut s, "b");
    key(&mut s, "a");
    clear_log(&mut s);
    ua_focus(&mut s, "c");
    assert!(has_change(&s), "M9 first blur ({})", log(&s));
    ua_focus(&mut s, "b");
    script_set_value(&mut s, "x");
    key(&mut s, "y");
    key(&mut s, "Backspace");
    assert_eq!(value(&s, "b"), "x");
    clear_log(&mut s);
    ua_focus(&mut s, "c");
    assert!(!has_change(&s), "M9 second blur ({})", log(&s));
}

#[test]
fn c20_user_returns_to_script_written_value_input() {
    c20("input");
}

#[test]
fn c20_user_returns_to_script_written_value_textarea() {
    c20("textarea");
}

/// The W3–W8 pattern: UA focus; `writer` changes the value to `expected`;
/// user `z`, Backspace (back to `expected`); UA blur → no `change`.
fn writer_rebaselines(body: &str, before_focus: &str, writer: &str, expected: &str) {
    let (mut s, _b) = setup(body);
    if !before_focus.is_empty() {
        eval(&mut s, before_focus);
    }
    ua_focus(&mut s, "b");
    eval(&mut s, writer);
    assert_eq!(value(&s, "b"), expected, "the writer ran: {writer}");
    key(&mut s, "z");
    key(&mut s, "Backspace");
    assert_eq!(value(&s, "b"), expected);
    clear_log(&mut s);
    ua_focus(&mut s, "c");
    assert!(!has_change(&s), "{writer}: no change ({})", log(&s));
}

/// C21 (W3).
#[test]
fn c21_textarea_default_value_write_rebaselines() {
    writer_rebaselines(
        "<textarea id=b>p</textarea><input id=c>",
        "",
        "document.getElementById('b').defaultValue = 'q';",
        "q",
    );
}

/// C22 (W4).
#[test]
fn c22_value_attribute_write_rebaselines() {
    writer_rebaselines(
        "<input id=b value=\"p\"><input id=c>",
        "",
        "document.getElementById('b').setAttribute('value', 'q');",
        "q",
    );
}

/// C23 (W5).
#[test]
fn c23_type_change_sanitize_rebaselines() {
    writer_rebaselines(
        "<input id=b value=\" a\"><input id=c>",
        "",
        "document.getElementById('b').type = 'url';",
        "a",
    );
}

/// C24 (W6; the accepted Chromium divergence, plan §6 / PM 307).
#[test]
fn c24_email_multiple_toggle_rebaselines() {
    writer_rebaselines(
        "<input id=b type=email value=\"a@b, c@d\"><input id=c>",
        "",
        "document.getElementById('b').setAttribute('multiple', '');",
        "a@b,c@d",
    );
}

/// C25 (W7).
#[test]
fn c25_set_range_text_rebaselines() {
    writer_rebaselines(
        "<input id=b value=\"p\"><input id=c>",
        "",
        "document.getElementById('b').setRangeText('q', 0, 0);",
        "qp",
    );
}

/// C26 (W8).
#[test]
fn c26_form_reset_rebaselines() {
    writer_rebaselines(
        "<form id=f><input id=b value=\"d\"></form><input id=c>",
        "document.getElementById('b').value = 'x';",
        "document.getElementById('f').reset();",
        "d",
    );
}
