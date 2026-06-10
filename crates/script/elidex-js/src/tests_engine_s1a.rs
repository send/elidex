//! S1a (boa→VM cutover): `ElidexJsEngine` shell-facing engine — the
//! BATCH-BIND bracket primitive + the `scripts_allowed` eval gate +
//! per-callback `drain_timers` results.
//!
//! These drive the `ScriptEngine` trait through the engine's own batch
//! bracket (`bind`/`unbind` / `with_bound`), mirroring how the shell will
//! drive the VM at S5.  See `memory/boa-vm-cutover-s1-plan.md` §2/§6/§7.

#![cfg(feature = "engine")]

use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::{ScriptContext, ScriptEngine, SessionCore};

use crate::engine::ElidexJsEngine;
use crate::vm::host_data::HostData;
use crate::vm::value::JsValue;

/// Construct an unbound engine + session + dom with a fresh `document_root`,
/// matching `tests_dispatch_integration::fresh_unbound`.
fn fresh_unbound() -> (ElidexJsEngine, SessionCore, EcsDom, Entity) {
    let mut engine = ElidexJsEngine::new();
    engine.vm().install_host_data(HostData::new());
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (engine, session, dom, doc)
}

/// Read a `globalThis.<name> === true` sentinel a script was meant to set.
fn global_true(engine: &mut ElidexJsEngine, name: &str) -> bool {
    matches!(engine.vm().get_global(name), Some(JsValue::Boolean(true)))
}

#[test]
fn eval_runs_when_unsandboxed() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    let r = ScriptEngine::eval(&mut engine, "globalThis.ran = true;", &mut ctx);
    assert!(r.success);
    assert!(global_true(&mut engine, "ran"));
}

#[test]
fn eval_gate_silent_success_when_scripts_disabled() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    // A sandboxed document WITHOUT `allow-scripts` → scripting is disabled
    // (HTML §8.1.3.4); `run a classic script` (§8.1.4.4) is a silent no-op.
    engine
        .vm()
        .host_data()
        .expect("host data installed")
        .set_sandbox_flags(Some(elidex_plugin::IframeSandboxFlags::empty()));
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    let r = ScriptEngine::eval(&mut engine, "globalThis.ran = true;", &mut ctx);
    assert!(
        r.success,
        "scripting-disabled eval is a silent success, not an error"
    );
    assert!(
        !global_true(&mut engine, "ran"),
        "the script must NOT run when scripting is disabled"
    );
}

#[test]
fn eval_runs_when_sandbox_grants_allow_scripts() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    engine
        .vm()
        .host_data()
        .expect("host data installed")
        .set_sandbox_flags(Some(elidex_plugin::IframeSandboxFlags::ALLOW_SCRIPTS));
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    let r = ScriptEngine::eval(&mut engine, "globalThis.ran = true;", &mut ctx);
    assert!(r.success);
    assert!(
        global_true(&mut engine, "ran"),
        "allow-scripts re-enables scripting"
    );
}

#[test]
fn bracket_allows_dom_touching_eval() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    // The batch bracket binds the VM; a DOM-reading script then resolves
    // `document` against the bound `EcsDom`.
    engine.bind(&mut ctx);
    let r = ScriptEngine::eval(
        &mut engine,
        "globalThis.hasDoc = (typeof document === 'object' && document !== null);",
        &mut ctx,
    );
    engine.unbind();
    assert!(r.success);
    assert!(global_true(&mut engine, "hasDoc"));
}

#[test]
fn drain_timers_returns_one_result_per_callback() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    engine.bind(&mut ctx);
    // One callback succeeds, one throws — drain_timers must surface BOTH as
    // distinct `EvalResult`s (the per-callback split, plan §7c).
    let _ = ScriptEngine::eval(
        &mut engine,
        "globalThis.ticks = 0;
         setTimeout(function () { globalThis.ticks += 1; }, 0);
         setTimeout(function () { throw new Error('boom'); }, 0);",
        &mut ctx,
    );
    let results = ScriptEngine::drain_timers(&mut engine, &mut ctx);
    engine.unbind();

    assert_eq!(results.len(), 2, "one EvalResult per fired callback");
    assert!(
        results.iter().any(|r| r.success),
        "the non-throwing callback reports success"
    );
    assert!(
        results.iter().any(|r| !r.success && r.error.is_some()),
        "the throwing callback reports an error"
    );
    assert!(
        matches!(engine.vm().get_global("ticks"), Some(JsValue::Number(n)) if n == 1.0),
        "the ok callback's side-effect ran exactly once"
    );
}

#[test]
fn with_bound_runs_then_unbinds() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    let ok = engine.with_bound(&mut ctx, |e, c| {
        ScriptEngine::eval(e, "globalThis.inside = true;", c).success
    });
    assert!(ok, "with_bound returns the closure's value");
    // unbind ran (even though we didn't call it) → a fresh bracket does not
    // trip the non-nesting assert.
    engine.bind(&mut ctx);
    engine.unbind();
    assert!(global_true(&mut engine, "inside"));
}

#[test]
fn drain_timers_excludes_abort_signal_timeout_fires() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    engine.bind(&mut ctx);
    // One user setTimeout callback + one AbortSignal.timeout (an *internal*
    // abort fire, NOT a user callback). drain_timers must surface only the
    // former — the abort fire is excluded from the per-callback results
    // (plan §7c / natives_timer drain doc).
    let _ = ScriptEngine::eval(
        &mut engine,
        "globalThis.sig = AbortSignal.timeout(0);
         setTimeout(function () {}, 0);",
        &mut ctx,
    );
    let results = ScriptEngine::drain_timers(&mut engine, &mut ctx);
    engine.unbind();
    assert_eq!(
        results.len(),
        1,
        "AbortSignal.timeout internal fire is excluded; only the user setTimeout yields an EvalResult"
    );
    assert!(results[0].success);
}

#[test]
fn drain_reactions_composes_and_is_noop_on_empty_queue() {
    // `drain_reactions` wires to `VmInner::flush_ce_reactions`, whose firing
    // path (connected/disconnected/attributeChanged callbacks) is covered by
    // `tests_custom_elements` via `eval`'s internal flush. S1a is VM-side only,
    // so the shell-mutation → reaction-enqueue path that makes the *trait*
    // `drain_reactions` distinct from `eval`'s flush is exercised at S5
    // integration; here we verify composition + the empty-queue no-op.
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    engine.bind(&mut ctx);
    let _ = ScriptEngine::eval(
        &mut engine,
        "customElements.define('s1a-el', class extends HTMLElement {});",
        &mut ctx,
    );
    // Empty reaction queue (no pending shell mutation) → clean no-op.
    ScriptEngine::drain_reactions(&mut engine, &mut ctx);
    // Engine stays usable after the drain.
    let after = ScriptEngine::eval(&mut engine, "globalThis.ok = true;", &mut ctx);
    engine.unbind();
    assert!(after.success);
    assert!(global_true(&mut engine, "ok"));
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "batch brackets must not nest")]
fn nested_bind_trips_non_reentrancy_assert() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    engine.bind(&mut ctx);
    // Second bind without an intervening unbind → non-re-entrancy violation.
    engine.bind(&mut ctx);
}
