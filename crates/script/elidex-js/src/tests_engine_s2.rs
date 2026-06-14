//! S2 (boa→VM cutover): the page-visibility + scroll transport method-group on
//! the `HostDriver` contract ([`ElidexJsEngine`]). Covers `set_visibility` →
//! `document.hidden` / `document.visibilityState` (WHATWG HTML §6.2), and the
//! scroll read-back round-trip — `window.scrollTo` records a pending offset the
//! shell drains via `take_pending_scroll`, and `set_scroll_offset` syncs the
//! applied offset back so `window.scrollX` / `scrollY` read it (CSSOM View §4).
//!
//! Like S1a–S1d these drive the VM while boa stays live (the S5 flip wires the
//! shell to call these instead). See `memory/boa-vm-cutover-s2-plan.md` §5-U1/U2.

#![cfg(feature = "engine")]

use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::{HostDriver, ScriptContext, ScriptEngine, SessionCore};

use crate::engine::ElidexJsEngine;
use crate::vm::host_data::HostData;
use crate::vm::value::JsValue;

/// Construct an unbound engine + session + dom with a fresh `document_root`
/// (mirrors `tests_engine_s1d::fresh_unbound`).
fn fresh_unbound() -> (ElidexJsEngine, SessionCore, EcsDom, Entity) {
    let mut engine = ElidexJsEngine::new();
    engine.vm().install_host_data(HostData::new());
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (engine, session, dom, doc)
}

#[allow(unsafe_code)]
fn bind_engine(engine: &mut ElidexJsEngine, ctx: &mut ScriptContext<'_>) {
    // SAFETY: the bracket stays open until the paired `unbind`, and no test body
    // aliases `ctx.session`/`ctx.dom` while bound.
    unsafe { engine.bind(ctx) }
}

fn eval_ok(engine: &mut ElidexJsEngine, ctx: &mut ScriptContext<'_>, script: &str) {
    assert!(
        ScriptEngine::eval(engine, script, ctx).success,
        "script eval failed: {script}"
    );
}

fn global_bool(engine: &mut ElidexJsEngine, name: &str) -> bool {
    match engine.vm().get_global(name) {
        Some(JsValue::Boolean(b)) => b,
        other => panic!("expected boolean global `{name}`, got {other:?}"),
    }
}

fn global_string(engine: &mut ElidexJsEngine, name: &str) -> String {
    match engine.vm().get_global(name) {
        Some(JsValue::String(id)) => engine.vm().get_string(id),
        other => panic!("expected string global `{name}`, got {other:?}"),
    }
}

fn global_number(engine: &mut ElidexJsEngine, name: &str) -> f64 {
    match engine.vm().get_global(name) {
        Some(JsValue::Number(n)) => n,
        other => panic!("expected number global `{name}`, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Page visibility (U1) — set_visibility → document.hidden / visibilityState
// ---------------------------------------------------------------------------

#[test]
fn visibility_defaults_to_visible() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    eval_ok(
        &mut engine,
        &mut ctx,
        "globalThis.h = document.hidden; globalThis.vs = document.visibilityState;",
    );
    engine.unbind();
    assert!(
        !global_bool(&mut engine, "h"),
        "default document.hidden is false"
    );
    assert_eq!(
        global_string(&mut engine, "vs"),
        "visible",
        "default visibilityState is visible"
    );
}

#[test]
fn set_visibility_false_drives_hidden() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    // The shell drives visibility; `set_visibility` is valid before binding (it
    // is a `HostData` setter, like the other security-context setters).
    engine.set_visibility(false);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    eval_ok(
        &mut engine,
        &mut ctx,
        "globalThis.h = document.hidden; globalThis.vs = document.visibilityState;",
    );
    engine.unbind();
    assert!(
        global_bool(&mut engine, "h"),
        "hidden after set_visibility(false)"
    );
    assert_eq!(global_string(&mut engine, "vs"), "hidden");
}

#[test]
fn has_focus_is_false_when_tab_hidden_but_active_element_retained() {
    // Codex S2 final pass: `hasFocus()` is window-system-focus based — a hidden
    // (background / minimized) tab has no system focus, so `hasFocus() === false`
    // even while an element retains the focused area. `activeElement` still
    // reports that element (focus is preserved across tab switches), so ONLY the
    // hasFocus read is visibility-gated.
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    engine.set_visibility(false);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    eval_ok(
        &mut engine,
        &mut ctx,
        // The engine-contract harness builds only a Document root (no html/body),
        // so append the focusable element directly as the document element to
        // connect it.
        "var d = document.createElement('div'); \
         d.setAttribute('tabindex', '0'); \
         document.appendChild(d); \
         d.focus(); \
         globalThis.hf = document.hasFocus(); \
         globalThis.activeIsD = document.activeElement === d;",
    );
    engine.unbind();
    assert!(
        !global_bool(&mut engine, "hf"),
        "hasFocus() is false in a hidden tab"
    );
    assert!(
        global_bool(&mut engine, "activeIsD"),
        "activeElement still reports the retained focused element when hidden"
    );
}

#[test]
fn visibility_getters_scope_to_bound_document() {
    // Codex R2 F2: page visibility is a fact of the *bound* document — a
    // cloned/non-bound `Document` receiver must NOT leak the bound tab's hidden
    // state.
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    engine.set_visibility(false);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    eval_ok(
        &mut engine,
        &mut ctx,
        "globalThis.boundHidden = document.hidden; \
         var clone = document.cloneNode(false); \
         globalThis.cloneHidden = clone.hidden; \
         globalThis.cloneVis = clone.visibilityState;",
    );
    engine.unbind();
    assert!(
        global_bool(&mut engine, "boundHidden"),
        "the bound document reflects the hidden tab"
    );
    assert!(
        !global_bool(&mut engine, "cloneHidden"),
        "a cloned (non-bound) document does not leak the bound hidden state"
    );
    assert_eq!(
        global_string(&mut engine, "cloneVis"),
        "visible",
        "cloned document visibilityState defaults to visible"
    );
}

#[test]
fn set_visibility_toggles_back_to_visible() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    engine.set_visibility(false);
    engine.set_visibility(true);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    eval_ok(
        &mut engine,
        &mut ctx,
        "globalThis.vs = document.visibilityState;",
    );
    engine.unbind();
    assert_eq!(
        global_string(&mut engine, "vs"),
        "visible",
        "set_visibility(true) returns to visible"
    );
}

// ---------------------------------------------------------------------------
// Scroll read-back (U2) — scrollTo pending drain + set_scroll_offset sync-in
// ---------------------------------------------------------------------------

#[test]
fn scroll_to_records_pending_offset_drained_once() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    // Nothing pending before any script scroll.
    assert!(engine.take_pending_scroll().is_none());
    eval_ok(&mut engine, &mut ctx, "window.scrollTo(10, 20);");
    engine.unbind();

    // The shell drains the script-requested offset post-batch.
    assert_eq!(
        engine.take_pending_scroll(),
        Some((10.0, 20.0)),
        "scrollTo records a pending offset for the shell to apply"
    );
    // Drained — a second take yields nothing.
    assert!(
        engine.take_pending_scroll().is_none(),
        "take_pending_scroll drains (does not repeat)"
    );
}

#[test]
fn scroll_by_accumulates_into_pending_offset() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    eval_ok(
        &mut engine,
        &mut ctx,
        "window.scrollTo(10, 20); window.scrollBy(5, 3);",
    );
    engine.unbind();
    assert_eq!(
        engine.take_pending_scroll(),
        Some((15.0, 23.0)),
        "scrollBy accumulates onto the current offset"
    );
}

#[test]
fn set_scroll_offset_syncs_into_scroll_x_y() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    // The shell pushes the applied (e.g. user wheel) offset into the engine.
    engine.set_scroll_offset(5.0, 7.0);
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    eval_ok(
        &mut engine,
        &mut ctx,
        "globalThis.sx = window.scrollX; globalThis.sy = window.scrollY;",
    );
    engine.unbind();
    assert_eq!(
        global_number(&mut engine, "sx"),
        5.0,
        "scrollX reads synced offset"
    );
    assert_eq!(
        global_number(&mut engine, "sy"),
        7.0,
        "scrollY reads synced offset"
    );
}
