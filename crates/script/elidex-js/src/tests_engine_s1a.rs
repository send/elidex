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

/// Open the engine's batch bracket for a test.
///
/// [`ElidexJsEngine::bind`] is `unsafe` (it stores raw pointers into `ctx`
/// until `unbind`); every test here upholds the contract — it keeps `ctx` alive
/// for the bracket and never touches `ctx.session`/`ctx.dom` directly while
/// bound, driving the VM only through the bound engine.
#[allow(unsafe_code)]
fn bind_engine(engine: &mut ElidexJsEngine, ctx: &mut ScriptContext<'_>) {
    // SAFETY: see fn doc — the bracket stays open until the paired `unbind`,
    // and no test body aliases `ctx.session`/`ctx.dom` while bound.
    unsafe { engine.bind(ctx) }
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
    bind_engine(&mut engine, &mut ctx);
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
    bind_engine(&mut engine, &mut ctx);
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
#[allow(unsafe_code)]
fn drain_timers_settles_tasks_enqueued_by_timer_callbacks() {
    // Codex PR327 R4 (boa parity): boa runs each ready timer through `eval()`,
    // which drains same-window tasks + CE reactions per callback; the VM fires
    // via the call path, so `drain_timers` must settle those queues itself. A
    // timer callback that calls `window.postMessage` enqueues a task; the
    // `message` listener must observe it within the timer turn — read via
    // `get_global` (NOT `eval`, which would self-drain and mask the gap).
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let _ = ScriptEngine::eval(
        &mut engine,
        "globalThis.msg = 0;
         window.addEventListener('message', function (e) { globalThis.msg = e.data; });
         setTimeout(function () { window.postMessage(7, '*'); }, 0);",
        &mut ctx,
    );
    // The timer has not fired yet → nothing posted.
    assert!(
        matches!(engine.vm().get_global("msg"), Some(JsValue::Number(n)) if n == 0.0),
        "no message before the timer fires"
    );

    let _ = ScriptEngine::drain_timers(&mut engine, &mut ctx);
    engine.unbind();

    assert!(
        matches!(engine.vm().get_global("msg"), Some(JsValue::Number(n)) if n == 7.0),
        "drain_timers must settle same-window tasks the timer callback enqueued (boa parity)"
    );
}

#[test]
#[allow(unsafe_code)]
fn drain_timers_delivers_prior_timer_tasks_before_next_callback() {
    // Codex PR327 R7 (Ihvwj): boa runs each ready timer through `eval`, which
    // drains that callback's queued tasks BEFORE the next callback; the VM must
    // match (per-timer, not once-after-the-whole-batch). Timer 1 `postMessage`s;
    // timer 2 must observe the delivered message. Read via globals (no `eval`
    // self-drain). With a once-at-end drain, timer 2 would see `gotMsg == false`.
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let _ = ScriptEngine::eval(
        &mut engine,
        "globalThis.gotMsg = false; globalThis.timer2SawMsg = false;
         window.addEventListener('message', function () { globalThis.gotMsg = true; });
         setTimeout(function () { window.postMessage(1, '*'); }, 0);
         setTimeout(function () { globalThis.timer2SawMsg = globalThis.gotMsg; }, 0);",
        &mut ctx,
    );
    let _ = ScriptEngine::drain_timers(&mut engine, &mut ctx);
    engine.unbind();
    assert!(
        matches!(
            engine.vm().get_global("timer2SawMsg"),
            Some(JsValue::Boolean(true))
        ),
        "the 2nd timer must observe the 1st timer's postMessage (per-timer drain, boa parity)"
    );
}

#[test]
#[allow(unsafe_code)]
fn drain_timers_settles_abort_listener_work_before_next_timer() {
    // Codex PR327 R8 (IiWhS): an `AbortSignal.timeout` abort dispatch runs JS
    // (its `abort` listeners), so its queued work must settle before a same-tick
    // user timer — the abort fire gets the same per-fire checkpoint a user
    // callback gets, not just the once-at-end drain. The signal is created first
    // (earlier deadline → fires first); its abort listener `postMessage`s; the
    // later `setTimeout` must observe the delivered message.
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let _ = ScriptEngine::eval(
        &mut engine,
        "globalThis.gotMsg = false; globalThis.timerSawMsg = false;
         window.addEventListener('message', function () { globalThis.gotMsg = true; });
         var sig = AbortSignal.timeout(0);
         sig.addEventListener('abort', function () { window.postMessage(1, '*'); });
         setTimeout(function () { globalThis.timerSawMsg = globalThis.gotMsg; }, 0);",
        &mut ctx,
    );
    let _ = ScriptEngine::drain_timers(&mut engine, &mut ctx);
    engine.unbind();
    assert!(
        matches!(
            engine.vm().get_global("timerSawMsg"),
            Some(JsValue::Boolean(true))
        ),
        "the user timer must observe the abort listener's postMessage (per-fire checkpoint on the abort path)"
    );
}

#[test]
#[allow(unsafe_code)]
fn with_bound_runs_then_unbinds() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    // SAFETY: the closure drives the VM only through the bound engine `e`; it
    // never touches `c.session`/`c.dom` directly, so the bracket's raw pointers
    // stay valid (the `with_bound` contract).
    let ok = unsafe {
        engine.with_bound(&mut ctx, |e, c| {
            ScriptEngine::eval(e, "globalThis.inside = true;", c).success
        })
    };
    assert!(ok, "with_bound returns the closure's value");
    // unbind ran (even though we didn't call it) → a fresh bracket does not
    // trip the non-nesting assert.
    bind_engine(&mut engine, &mut ctx);
    engine.unbind();
    assert!(global_true(&mut engine, "inside"));
}

#[test]
fn drain_timers_excludes_abort_signal_timeout_fires() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
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
    bind_engine(&mut engine, &mut ctx);
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
    bind_engine(&mut engine, &mut ctx);
    // Second bind without an intervening unbind → non-re-entrancy violation.
    bind_engine(&mut engine, &mut ctx);
}
