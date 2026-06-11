//! `history` host-global tests — S1c enqueue + synchronous-pushState model.
//!
//! The shell's `NavigationController` is the single session-history source of
//! truth; the VM holds a current-document view + drain-once intent buffers.
//! `back`/`forward`/`go` *enqueue* a `HistoryAction`; `pushState`/`replaceState`
//! update `current_url` + `history.state` synchronously (§7.4.4) AND enqueue.

#![cfg(feature = "engine")]

use elidex_script_session::HistoryAction;

use super::super::value::JsValue;
use super::super::Vm;

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(id) => vm.get_string(id),
        other => panic!("expected string, got {other:?}"),
    }
}

fn eval_number(vm: &mut Vm, source: &str) -> f64 {
    match vm.eval(source).unwrap() {
        JsValue::Number(n) => n,
        other => panic!("expected number, got {other:?}"),
    }
}

/// Commit a hierarchical base URL via the shell's `set_current_url` path.  The
/// enqueue-only `location.href=` setter no longer mutates `current_url`, so a
/// test that needs a concrete base (e.g. to resolve a relative `pushState` URL,
/// which the WHATWG parser refuses against `about:blank`) installs it directly —
/// simulating the shell committing a load.
fn new_vm_with_base() -> Vm {
    let mut vm = Vm::new();
    vm.inner
        .navigation
        .set_current_url(Some(url::Url::parse("http://localhost/").unwrap()));
    vm
}

#[test]
fn history_is_object() {
    let mut vm = Vm::new();
    assert_eq!(eval_string(&mut vm, "typeof history;"), "object");
}

#[test]
fn history_initial_length_is_one() {
    let mut vm = Vm::new();
    // Default `history_length` = 1 (the current entry; the shell pushes the real
    // count via `set_history_length`).
    assert_eq!(eval_number(&mut vm, "history.length;"), 1.0);
}

#[test]
fn history_initial_state_is_null() {
    let mut vm = Vm::new();
    match vm.eval("history.state;").unwrap() {
        JsValue::Null => {}
        other => panic!("expected null, got {other:?}"),
    }
}

#[test]
fn history_scroll_restoration_is_auto() {
    let mut vm = Vm::new();
    assert_eq!(eval_string(&mut vm, "history.scrollRestoration;"), "auto");
}

#[test]
fn history_length_is_shell_pushed_not_vm_grown() {
    // `pushState` does NOT grow the VM's `history.length` — the shell's
    // `NavigationController` owns the count.
    let mut vm = new_vm_with_base();
    vm.eval("history.pushState(null, '', '/a'); history.pushState(null, '', '/b');")
        .unwrap();
    assert_eq!(eval_number(&mut vm, "history.length;"), 1.0);
    // The shell-facing setter is what moves it.
    vm.inner.navigation.history_length = 7;
    assert_eq!(eval_number(&mut vm, "history.length;"), 7.0);
}

#[test]
fn push_state_syncs_url_and_state_and_enqueues() {
    let mut vm = new_vm_with_base();
    vm.eval("history.pushState({step: 1}, '', '/a');").unwrap();
    // §7.4.4 synchronous update — observable in the same script turn.
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/a");
    assert_eq!(eval_number(&mut vm, "history.state.step;"), 1.0);
    // Enqueued for the shell to persist.
    match vm.inner.navigation.pending_history.take() {
        Some(HistoryAction::PushState { url, .. }) => {
            assert_eq!(url.as_deref(), Some("http://localhost/a"));
        }
        other => panic!("expected PushState, got {other:?}"),
    }
}

#[test]
fn replace_state_syncs_url_and_enqueues_replace() {
    let mut vm = new_vm_with_base();
    vm.eval("history.replaceState({step: 2}, '', '/b');")
        .unwrap();
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/b");
    assert_eq!(eval_number(&mut vm, "history.state.step;"), 2.0);
    assert!(matches!(
        vm.inner.navigation.pending_history.take(),
        Some(HistoryAction::ReplaceState { .. })
    ));
}

#[test]
fn push_state_without_url_keeps_current_url() {
    let mut vm = new_vm_with_base();
    vm.eval("history.pushState({step: 5}, '');").unwrap();
    // No URL arg → current_url unchanged, state updated, PushState{url: None}.
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/");
    assert_eq!(eval_number(&mut vm, "history.state.step;"), 5.0);
    match vm.inner.navigation.pending_history.take() {
        Some(HistoryAction::PushState { url, .. }) => assert!(url.is_none()),
        other => panic!("expected PushState, got {other:?}"),
    }
}

#[test]
fn push_state_cross_origin_throws_security_error() {
    // §7.2.5 step 6.3: a cross-origin target → SecurityError, synchronously,
    // before any update or enqueue.
    let mut vm = new_vm_with_base(); // http://localhost/
    let check = vm
        .eval(
            "var thrown = null;\
             try { history.pushState(null, '', 'https://evil.example/x'); } \
             catch (e) { thrown = e; }\
             thrown && thrown.name === 'SecurityError' \
             && thrown instanceof DOMException;",
        )
        .unwrap();
    assert!(matches!(check, JsValue::Boolean(true)));
    // Neither updated nor enqueued.
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/");
    assert!(vm.inner.navigation.pending_history.is_none());
}

#[test]
fn replace_state_cross_origin_throws_security_error() {
    // §7.2.5 step 6.3 applies to replaceState too (the gate lives in the shared
    // state_mutate body) — guards both arms of the same-origin check.
    let mut vm = new_vm_with_base(); // http://localhost/
    let check = vm
        .eval(
            "var thrown = null;\
             try { history.replaceState(null, '', 'https://evil.example/x'); } \
             catch (e) { thrown = e; }\
             thrown && thrown.name === 'SecurityError' \
             && thrown instanceof DOMException;",
        )
        .unwrap();
    assert!(matches!(check, JsValue::Boolean(true)));
    assert!(vm.inner.navigation.pending_history.is_none());
}

#[test]
fn history_state_round_trips_through_push_state() {
    let mut vm = new_vm_with_base();
    vm.eval("history.pushState({step: 42}, '', '/x');").unwrap();
    assert_eq!(eval_number(&mut vm, "history.state.step;"), 42.0);
}

#[test]
fn history_go_zero_enqueues_go_zero() {
    // §7.2.5: `go(0)` reloads the current entry — the VM enqueues `Go(0)` (the
    // shell's NavigationController.go(0) re-fetches), NOT a no-op.
    let mut vm = new_vm_with_base();
    vm.eval("history.go(0);").unwrap();
    assert!(matches!(
        vm.inner.navigation.pending_history.take(),
        Some(HistoryAction::Go(0))
    ));
}

#[test]
fn back_forward_go_enqueue_actions() {
    let mut vm = new_vm_with_base();
    vm.eval("history.back();").unwrap();
    assert!(matches!(
        vm.inner.navigation.pending_history.take(),
        Some(HistoryAction::Back)
    ));
    vm.eval("history.forward();").unwrap();
    assert!(matches!(
        vm.inner.navigation.pending_history.take(),
        Some(HistoryAction::Forward)
    ));
    vm.eval("history.go(-2);").unwrap();
    assert!(matches!(
        vm.inner.navigation.pending_history.take(),
        Some(HistoryAction::Go(-2))
    ));
    // Traversals do NOT mutate `current_url` — the shell commits it after the
    // target entry loads.
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/");
}

#[test]
fn traversal_resets_history_state() {
    // A traversal moves to a different entry whose state the VM cannot know
    // until the shell restores it (slot `#11-history-state-traversal-popstate-fidelity`);
    // the VM conservatively resets `history.state` to null.
    let mut vm = new_vm_with_base();
    vm.eval("history.pushState({step: 9}, '', '/a');").unwrap();
    assert_eq!(eval_number(&mut vm, "history.state.step;"), 9.0);
    vm.eval("history.back();").unwrap();
    match vm.eval("history.state;").unwrap() {
        JsValue::Null => {}
        other => panic!("expected null after traversal, got {other:?}"),
    }
}

#[test]
fn history_state_survives_gc() {
    // Regression: `NavigationState.current_state` is a GC root (S1c — one value,
    // replacing the old per-entry `history_entries[*].state` iteration).  Pushing
    // an object + forcing GC + reading it back must preserve the value.
    let mut vm = new_vm_with_base();
    vm.eval(
        "history.pushState({step: 7, nested: {v: 99}}, '', '/x');
         // Many allocations to raise GC pressure; if `current_state`'s nested
         // object were unrooted, GC would have claimed it.
         var filler = [];
         for (var i = 0; i < 5000; i++) { filler.push({k: i}); }
         filler = null;",
    )
    .unwrap();
    assert_eq!(eval_number(&mut vm, "history.state.step;"), 7.0);
    assert_eq!(eval_number(&mut vm, "history.state.nested.v;"), 99.0);
}
