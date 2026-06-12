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

/// Drain the FIFO history queue (S1c — `pending_history` is a bounded `VecDeque`,
/// so a turn's synchronous `pushState`/`replaceState` mutations are preserved in
/// order up to the cap).
fn drain_history(vm: &mut Vm) -> Vec<HistoryAction> {
    std::mem::take(&mut vm.inner.navigation.pending_history).into()
}

/// Drain and assert exactly one enqueued history action.
fn take_one_history(vm: &mut Vm) -> HistoryAction {
    let mut actions = drain_history(vm);
    assert_eq!(
        actions.len(),
        1,
        "expected exactly one enqueued history action, got {actions:?}"
    );
    actions.remove(0)
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
    // count via `set_session_history`).
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
fn push_state_grows_length_via_index_replace_does_not() {
    // pushState advances the current index and sets `history.length = index + 1`
    // synchronously (§7.4.4); replaceState changes neither.
    let mut vm = new_vm_with_base();
    assert_eq!(eval_number(&mut vm, "history.length;"), 1.0); // default: index 0, length 1
    vm.eval("history.pushState(null, '', '/a'); history.pushState(null, '', '/b');")
        .unwrap();
    assert_eq!(eval_number(&mut vm, "history.length;"), 3.0); // index 2 → length 3
    vm.eval("history.replaceState(null, '', '/c');").unwrap();
    assert_eq!(eval_number(&mut vm, "history.length;"), 3.0); // replace: no growth
}

#[test]
fn push_state_length_uses_index_not_plus_one_after_back() {
    // The reason the length is computed from the index, not `length + 1`: when the
    // current entry is not the last (the shell pushed `(index 1, length 3)` after
    // a `back` left a forward entry), pushState discards the forward entry and the
    // new length is `index + 1 = 3` — a naive `length + 1` would over-count to 4.
    let mut vm = new_vm_with_base();
    vm.inner.navigation.current_index = 1; // mid-history (shell-pushed)
    vm.inner.navigation.history_length = 3;
    vm.eval("history.pushState(null, '', '/d');").unwrap();
    assert_eq!(eval_number(&mut vm, "history.length;"), 3.0); // index 2 → length 3, not 4
}

#[test]
fn history_go_uses_webidl_modular_long_conversion() {
    // WebIDL `long` is ConvertToInt(V, 32, "signed") = ECMA-262 ToInt32, which
    // wraps modulo 2^32 — NOT a saturating clamp.  `go(4294967295)` → -1 (a
    // one-step back), `go(4294967296)` → 0, `go(-1)` → -1; a large in-range value
    // passes through.
    let mut vm = new_vm_with_base();
    vm.eval("history.go(4294967295);").unwrap();
    assert!(matches!(take_one_history(&mut vm), HistoryAction::Go(-1)));
    vm.eval("history.go(4294967296);").unwrap();
    assert!(matches!(take_one_history(&mut vm), HistoryAction::Go(0)));
    vm.eval("history.go(-1);").unwrap();
    assert!(matches!(take_one_history(&mut vm), HistoryAction::Go(-1)));
    vm.eval("history.go(2000000000);").unwrap();
    assert!(matches!(
        take_one_history(&mut vm),
        HistoryAction::Go(2_000_000_000)
    ));
}

#[test]
fn pending_history_queue_is_bounded_drop_oldest() {
    // A runaway `pushState` loop must not grow the queue without limit: it is
    // capped (drop-oldest), and the newest entry — matching the synchronously
    // updated `current_url` — is always retained.
    let mut vm = new_vm_with_base();
    vm.eval("for (let i = 0; i < 5000; i++) { history.pushState(null, '', '/p' + i); }")
        .unwrap();
    // current_url tracked every push synchronously (independent of the queue cap).
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/p4999");
    let actions = drain_history(&mut vm);
    assert_eq!(
        actions.len(),
        1024,
        "queue capped at MAX_PENDING_HISTORY_ACTIONS"
    );
    match actions.last() {
        Some(HistoryAction::PushState { url, .. }) => {
            assert_eq!(url.as_deref(), Some("http://localhost/p4999")); // newest kept
        }
        other => panic!("expected newest PushState /p4999 retained, got {other:?}"),
    }
}

#[test]
fn push_state_syncs_url_and_state_and_enqueues() {
    let mut vm = new_vm_with_base();
    vm.eval("history.pushState({step: 1}, '', '/a');").unwrap();
    // §7.4.4 synchronous update — observable in the same script turn.
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/a");
    assert_eq!(eval_number(&mut vm, "history.state.step;"), 1.0);
    // Enqueued for the shell to persist.
    match take_one_history(&mut vm) {
        HistoryAction::PushState { url, .. } => {
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
        take_one_history(&mut vm),
        HistoryAction::ReplaceState { .. }
    ));
}

#[test]
fn push_state_without_url_keeps_current_url() {
    let mut vm = new_vm_with_base();
    vm.eval("history.pushState({step: 5}, '');").unwrap();
    // No URL arg → current_url unchanged, state updated, PushState{url: None}.
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/");
    assert_eq!(eval_number(&mut vm, "history.state.step;"), 5.0);
    match take_one_history(&mut vm) {
        HistoryAction::PushState { url, .. } => assert!(url.is_none()),
        other => panic!("expected PushState, got {other:?}"),
    }
}

#[test]
fn multiple_push_states_enqueue_in_fifo_order() {
    // Two synchronous `pushState`s in one turn each commit an independent
    // session-history entry, so both must reach the shell in order — the queue is
    // FIFO, not a last-wins single slot (a single slot would drop `/a`).
    let mut vm = new_vm_with_base();
    vm.eval("history.pushState(null, '', '/a'); history.pushState(null, '', '/b');")
        .unwrap();
    let actions = drain_history(&mut vm);
    assert_eq!(
        actions.len(),
        2,
        "both pushStates preserved, got {actions:?}"
    );
    match (&actions[0], &actions[1]) {
        (HistoryAction::PushState { url: a, .. }, HistoryAction::PushState { url: b, .. }) => {
            assert_eq!(a.as_deref(), Some("http://localhost/a"));
            assert_eq!(b.as_deref(), Some("http://localhost/b"));
        }
        other => panic!("expected two PushState actions in order, got {other:?}"),
    }
}

#[test]
fn push_state_then_traversal_enqueue_in_order() {
    // A synchronous `pushState` followed by an async `back()` must reach the shell
    // as [PushState, Back] — the FIFO queue mixes both intent kinds in order so
    // the shell applies the push before the traversal (a single slot would drop
    // the push).
    let mut vm = new_vm_with_base();
    vm.eval("history.pushState(null, '', '/a'); history.back();")
        .unwrap();
    let actions = drain_history(&mut vm);
    assert!(
        matches!(
            actions.as_slice(),
            [HistoryAction::PushState { .. }, HistoryAction::Back]
        ),
        "expected [PushState, Back], got {actions:?}"
    );
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
    assert!(vm.inner.navigation.pending_history.is_empty());
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
    assert!(vm.inner.navigation.pending_history.is_empty());
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
    assert!(matches!(take_one_history(&mut vm), HistoryAction::Go(0)));
}

#[test]
fn back_forward_go_enqueue_actions() {
    let mut vm = new_vm_with_base();
    vm.eval("history.back();").unwrap();
    assert!(matches!(take_one_history(&mut vm), HistoryAction::Back));
    vm.eval("history.forward();").unwrap();
    assert!(matches!(take_one_history(&mut vm), HistoryAction::Forward));
    vm.eval("history.go(-2);").unwrap();
    assert!(matches!(take_one_history(&mut vm), HistoryAction::Go(-2)));
    // Traversals do NOT mutate `current_url` — the shell commits it after the
    // target entry loads.
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/");
}

#[test]
fn traversal_preserves_current_state_until_commit() {
    // A traversal is async (the shell loads the target entry), so it leaves
    // `history.state` untouched — a same-turn read still sees the current entry's
    // state, and a no-op traversal (`go(0)`) changes nothing.  Restoring the
    // *target* entry's state on commit is the shell's job (slot
    // `#11-history-state-traversal-popstate-fidelity`).
    let mut vm = new_vm_with_base();
    vm.eval("history.pushState({step: 9}, '', '/a');").unwrap();
    assert_eq!(eval_number(&mut vm, "history.state.step;"), 9.0);
    vm.eval("history.back();").unwrap();
    assert_eq!(eval_number(&mut vm, "history.state.step;"), 9.0);
    vm.eval("history.go(0);").unwrap();
    assert_eq!(eval_number(&mut vm, "history.state.step;"), 9.0);
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
