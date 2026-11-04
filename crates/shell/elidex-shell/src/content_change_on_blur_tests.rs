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
use super::focus::set_focus;
use super::ime::handle_ime;
use super::test_support::{build_test_content_state, probe_body_attr};
use super::ContentState;
use crate::ipc::{BrowserToContent, ContentToBrowser, ImeKind, LocalChannel, ModifierState};

type Browser = LocalChannel<BrowserToContent, ContentToBrowser>;

/// The events every logged control reports. `input` carries its `inputType`.
/// (`listen` takes `t` as a parameter so each listener closes over its own type.)
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

fn value(state: &ContentState, id: &str) -> String {
    let e = entity(state, id);
    state
        .pipeline
        .dom
        .world()
        .get::<&FormControlState>(e)
        .map(|fcs| fcs.value().to_owned())
        .expect("form control state")
}

fn has_change(state: &ContentState) -> bool {
    log(state).split(',').any(|t| t == "change")
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
    eval(&mut s, "document.getElementById('b').value = 'third';");
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
    eval(&mut s, "document.getElementById('b').value = '';");
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
    eval(&mut s, "document.getElementById('b').value = 'x';");
    key(&mut s, "a");
    eval(&mut s, "document.getElementById('b').value = 'y';");
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
    eval(&mut s, "document.getElementById('b').value = 'x';");
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
