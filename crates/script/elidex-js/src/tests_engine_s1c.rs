//! S1c (boa→VM cutover): the navigation back-channel — the shell-facing
//! inherent methods on [`ElidexJsEngine`] (`set_current_url` / `current_url` /
//! `take_pending_navigation` / `take_pending_history` / `set_history_length` /
//! `history_length`), driven through the engine's PUBLIC API (not the internal
//! `vm.inner.navigation` the `vm/host` unit tests poke).
//!
//! See `memory/boa-vm-cutover-s1c-plan.md` §6. Like S1a/S1b these exercise the
//! VM while boa stays live; the S5 flip rewrites the shell's `runtime.X()` /
//! `runtime.bridge().X()` to these.

#![cfg(feature = "engine")]

use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::{HistoryAction, ScriptContext, ScriptEngine, SessionCore};
use url::Url;

use crate::engine::ElidexJsEngine;
use crate::vm::host_data::HostData;

/// Construct an unbound engine + session + dom with a fresh `document_root`
/// (mirrors `tests_engine_s1b::fresh_unbound`).
fn fresh_unbound() -> (ElidexJsEngine, SessionCore, EcsDom, Entity) {
    let mut engine = ElidexJsEngine::new();
    engine.vm().install_host_data(HostData::new());
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (engine, session, dom, doc)
}

/// Open the engine's batch bracket (see `tests_engine_s1a::bind_engine`).
#[allow(unsafe_code)]
fn bind_engine(engine: &mut ElidexJsEngine, ctx: &mut ScriptContext<'_>) {
    // SAFETY: the bracket stays open until the paired `unbind`, and no test body
    // aliases `ctx.session`/`ctx.dom` while bound.
    unsafe { engine.bind(ctx) }
}

fn url(s: &str) -> Url {
    Url::parse(s).expect("valid test URL")
}

// ---------------------------------------------------------------------------
// current_url / set_current_url — the shell-commit path
// ---------------------------------------------------------------------------

#[test]
fn set_current_url_round_trips_and_none_resets_to_about_blank() {
    let mut engine = ElidexJsEngine::new();
    // Default: the VM always has an active document → Some(about:blank) (unlike
    // boa's None-when-unset; this is the documented S5-integrator divergence).
    assert_eq!(engine.current_url(), Some(url("about:blank")));

    engine.set_current_url(Some(url("https://example.com/a?x=1#y")));
    assert_eq!(
        engine.current_url(),
        Some(url("https://example.com/a?x=1#y"))
    );

    // None → reset to about:blank (the spec's "no active document").
    engine.set_current_url(None);
    assert_eq!(engine.current_url(), Some(url("about:blank")));
}

// ---------------------------------------------------------------------------
// history_length — shell-pushed
// ---------------------------------------------------------------------------

#[test]
fn history_length_round_trips_with_default_one() {
    let mut engine = ElidexJsEngine::new();
    assert_eq!(engine.history_length(), 1); // spec-minimum current entry
    engine.set_history_length(7);
    assert_eq!(engine.history_length(), 7);
}

// ---------------------------------------------------------------------------
// take_pending_navigation — drained after the location setters enqueue
// ---------------------------------------------------------------------------

#[test]
fn eval_location_assign_then_take_pending_navigation() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let r = ScriptEngine::eval(
        &mut engine,
        "location.assign('https://a.example/b');",
        &mut ctx,
    );
    assert!(r.success);
    engine.unbind();

    let nav = engine
        .take_pending_navigation()
        .expect("assign enqueued a navigation");
    assert_eq!(nav.url, "https://a.example/b");
    assert!(!nav.replace);
    // Drain-once: a second take is empty.
    assert!(engine.take_pending_navigation().is_none());
}

#[test]
fn eval_location_assign_last_wins_single_slot() {
    // The pending slot is single-Option last-wins (boa parity) — a second
    // un-drained enqueue overwrites the first.
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let r = ScriptEngine::eval(
        &mut engine,
        "location.assign('https://first.example/'); location.assign('https://second.example/');",
        &mut ctx,
    );
    assert!(r.success);
    engine.unbind();

    let nav = engine.take_pending_navigation().expect("a navigation");
    assert_eq!(nav.url, "https://second.example/");
}

// ---------------------------------------------------------------------------
// take_pending_history — drained after the history methods enqueue
// ---------------------------------------------------------------------------

#[test]
fn eval_history_push_state_then_take_pending_history() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    // pushState needs a hierarchical, same-origin base.
    engine.set_current_url(Some(url("https://localhost/")));
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let r = ScriptEngine::eval(
        &mut engine,
        "history.pushState({n: 1}, '', '/x');",
        &mut ctx,
    );
    assert!(r.success);
    engine.unbind();

    let actions = engine.take_pending_history();
    assert_eq!(
        actions.len(),
        1,
        "pushState enqueued one history action, got {actions:?}"
    );
    match &actions[0] {
        HistoryAction::PushState { url, .. } => {
            assert_eq!(url.as_deref(), Some("https://localhost/x"));
        }
        other => panic!("expected PushState, got {other:?}"),
    }
    // Drain-once: a second take is empty.
    assert!(engine.take_pending_history().is_empty());
}

#[test]
fn eval_history_back_then_take_pending_history() {
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let r = ScriptEngine::eval(&mut engine, "history.back();", &mut ctx);
    assert!(r.success);
    engine.unbind();
    assert!(matches!(
        engine.take_pending_history().as_slice(),
        [HistoryAction::Back]
    ));
}

#[test]
fn eval_multiple_push_states_drain_in_fifo_order() {
    // Two synchronous pushStates in one turn each commit an independent
    // session-history entry; the back-channel must hand the shell both, in order.
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    engine.set_current_url(Some(url("https://localhost/")));
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let r = ScriptEngine::eval(
        &mut engine,
        "history.pushState(null, '', '/a'); history.pushState(null, '', '/b');",
        &mut ctx,
    );
    assert!(r.success);
    engine.unbind();

    let actions = engine.take_pending_history();
    assert_eq!(
        actions.len(),
        2,
        "both pushStates preserved, got {actions:?}"
    );
    match (&actions[0], &actions[1]) {
        (HistoryAction::PushState { url: a, .. }, HistoryAction::PushState { url: b, .. }) => {
            assert_eq!(a.as_deref(), Some("https://localhost/a"));
            assert_eq!(b.as_deref(), Some("https://localhost/b"));
        }
        other => panic!("expected two PushState actions in order, got {other:?}"),
    }
}

#[test]
fn eval_push_state_grows_history_length_synchronously() {
    // `pushState` bumps `history.length` in the same turn (§7.4.4); the shell's
    // later `set_history_length` reconciles (and overwrites, so no double-count).
    let (mut engine, mut session, mut dom, doc) = fresh_unbound();
    engine.set_current_url(Some(url("https://localhost/")));
    engine.set_history_length(2); // shell-pushed starting count
    let mut ctx = ScriptContext::new(&mut session, &mut dom, doc);
    bind_engine(&mut engine, &mut ctx);
    let r = ScriptEngine::eval(
        &mut engine,
        "history.pushState(null, '', '/x'); history.replaceState(null, '', '/y');",
        &mut ctx,
    );
    assert!(r.success);
    engine.unbind();
    // 2 (start) + 1 (push); replaceState does not grow the count.
    assert_eq!(engine.history_length(), 3);
}
