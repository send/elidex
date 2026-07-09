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
fn push_state_length_saturates_at_session_history_cap() {
    // R7-#3: a tight pushState loop reports the *capped* history.length (matching
    // the shell's eviction at MAX_HISTORY_ENTRIES = 50), not an unbounded count
    // that collapses to the cap the moment the shell drains.
    let mut vm = new_vm_with_base();
    vm.eval("for (let i = 0; i < 200; i++) { history.pushState(null, '', '/p' + i); }")
        .unwrap();
    assert_eq!(eval_number(&mut vm, "history.length;"), 50.0); // SESSION_HISTORY_CAP
}

#[test]
fn push_state_invalid_url_throws_security_error_not_syntax_error() {
    // §7.2.5 step 6.2: a non-empty `url` that fails to parse → SecurityError —
    // NOT SyntaxError (which is what the `location.href =` setter throws).
    let mut vm = new_vm_with_base(); // http://localhost/
    assert_eq!(
        eval_string(
            &mut vm,
            "var name = '';\
             try { history.pushState(null, '', 'http://'); }\
             catch (e) { name = e instanceof DOMException ? e.name : 'not-dom'; }\
             name;"
        ),
        "SecurityError"
    );
    // Nothing updated or enqueued (the throw is before the synchronous update).
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/");
    assert!(vm.inner.navigation.pending_history.is_empty());
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
fn bounded_queue_retains_self_contained_no_url_actions() {
    // R4-#2: a URL-bearing push followed by enough no-URL pushes to overflow the
    // cap.  When drop-oldest evicts the URL-bearing `/a` action, the retained
    // no-URL actions must each still carry the effective URL (`/a`) — otherwise
    // the shell would apply them against its stale current URL and diverge.
    let mut vm = new_vm_with_base();
    vm.eval(
        "history.pushState(null, '', '/a');\
         for (let i = 0; i < 1100; i++) { history.pushState(null, ''); }",
    )
    .unwrap();
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/a"); // no-URL pushes kept /a
    let actions = drain_history(&mut vm);
    assert_eq!(actions.len(), 1024); // capped; the `/a`-bearing first action evicted
    for a in &actions {
        match a {
            HistoryAction::PushState { url, .. } => {
                assert_eq!(url.as_deref(), Some("http://localhost/a")); // self-contained
            }
            other => panic!("expected self-contained PushState, got {other:?}"),
        }
    }
}

#[test]
fn bounded_queue_preserves_traversal_intents() {
    // R9-#2: a traversal (`back`) followed by enough pushes to overflow the cap
    // must NOT lose the traversal — the cap evicts the oldest *evictable*
    // pushState (which the shell's session cap drops anyway), never a traversal,
    // so the shell still replays `back` before the pushes.
    let mut vm = new_vm_with_base();
    vm.eval("history.back();").unwrap();
    vm.eval("for (let i = 0; i < 1100; i++) { history.pushState(null, '', '/p' + i); }")
        .unwrap();
    let actions = drain_history(&mut vm);
    assert_eq!(actions.len(), 1024); // capped
    assert!(
        matches!(actions.first(), Some(HistoryAction::Back)),
        "the Back traversal is preserved at the front, got {:?}",
        actions.first()
    );
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
fn push_state_without_url_keeps_current_url_self_contained() {
    let mut vm = new_vm_with_base();
    vm.eval("history.pushState({step: 5}, '');").unwrap();
    // No URL arg → current_url unchanged, state updated.  The enqueued action
    // carries the effective (current) URL — not None — so it stays correct even
    // if the per-turn cap later drops an earlier URL-bearing action.
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/");
    assert_eq!(eval_number(&mut vm, "history.state.step;"), 5.0);
    match take_one_history(&mut vm) {
        HistoryAction::PushState { url, .. } => {
            assert_eq!(url.as_deref(), Some("http://localhost/"));
        }
        other => panic!("expected PushState, got {other:?}"),
    }
}

#[test]
fn push_state_empty_url_keeps_document_url_with_fragment() {
    // §7.2.5 step 6: the empty string is the historical special case — it is NOT
    // parsed (unlike `location.href = ""`), so the document URL, including a
    // trailing `#fragment`, is preserved (parsing "" would resolve it away).
    let mut vm = Vm::new();
    vm.inner
        .navigation
        .set_current_url(Some(url::Url::parse("http://localhost/p#frag").unwrap()));
    vm.eval("history.pushState(null, '', '');").unwrap();
    assert_eq!(
        eval_string(&mut vm, "location.href;"),
        "http://localhost/p#frag"
    );
    match take_one_history(&mut vm) {
        HistoryAction::PushState { url, .. } => {
            assert_eq!(url.as_deref(), Some("http://localhost/p#frag"));
        }
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
fn history_state_is_a_serialized_snapshot_not_the_live_object() {
    // §7.4.4 restores `history.state` from the NEW entry AFTER serialization — so it
    // is the serialized SNAPSHOT (a structured clone), NOT the live object passed to
    // pushState. Mutating the passed object afterward must NOT be observed, and the
    // value must match what a traversal/reload restores (Codex R2-F2).
    let mut vm = new_vm_with_base();
    vm.eval("var o = { n: 1 }; history.pushState(o, '', '/a'); o.n = 2;")
        .unwrap();
    assert_eq!(
        eval_number(&mut vm, "history.state.n;"),
        1.0,
        "history.state is the snapshot (1), not the live mutated object (2)"
    );
    // A JSON-unrepresentable state (BigInt) degrades to null immediately — CONSISTENT
    // with the traversal/reload restore (not the live BigInt), the interim D1 gap.
    vm.eval("history.pushState({ v: 10n }, '', '/b');").unwrap();
    match vm.eval("history.state;").unwrap() {
        JsValue::Null => {}
        other => panic!("expected null (degraded snapshot), got {other:?}"),
    }
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
    // A traversal is async (the shell loads the target entry), so the ENQUEUE
    // leaves `history.state` untouched — a same-turn read still sees the current
    // entry's state, and a no-op traversal (`go(0)`) changes nothing. Restoring the
    // *target* entry's state is the shell's job at commit, via
    // `deliver_history_step_events` (5c) — see `tests_history_events`.
    let mut vm = new_vm_with_base();
    vm.eval("history.pushState({step: 9}, '', '/a');").unwrap();
    assert_eq!(eval_number(&mut vm, "history.state.step;"), 9.0);
    vm.eval("history.back();").unwrap();
    assert_eq!(eval_number(&mut vm, "history.state.step;"), 9.0);
    vm.eval("history.go(0);").unwrap();
    assert_eq!(eval_number(&mut vm, "history.state.step;"), 9.0);
}

#[test]
fn push_state_enqueues_serialized_state_bytes() {
    // §7.2.5 step 3: the VM serializes the state object (JSON-shortcut interim) to
    // storage bytes on the enqueued action, so a later cross-document traversal can
    // restore `history.state`. The VM ALWAYS serializes (`Some(bytes)`); the `None`
    // variant is boa's light-touch.
    let mut vm = new_vm_with_base();
    vm.eval("history.pushState({step: 3}, '', '/a');").unwrap();
    match take_one_history(&mut vm) {
        HistoryAction::PushState {
            serialized_state, ..
        } => {
            let bytes = serialized_state.expect("VM always serializes (Some), never boa-None");
            assert_eq!(String::from_utf8(bytes).unwrap(), "{\"step\":3}");
        }
        other => panic!("expected PushState with serialized_state, got {other:?}"),
    }
}

#[test]
fn push_state_null_serializes_to_json_null() {
    // A null (or undefined) state round-trips as JSON `null` bytes — so the entry
    // carries a restorable `Some(b"null")`, deserializing back to `null`, not an
    // ambiguous `None` (which would be indistinguishable from a plain-nav entry).
    let mut vm = new_vm_with_base();
    vm.eval("history.pushState(null, '', '/a');").unwrap();
    match take_one_history(&mut vm) {
        HistoryAction::PushState {
            serialized_state, ..
        } => {
            assert_eq!(
                String::from_utf8(serialized_state.unwrap()).unwrap(),
                "null"
            );
        }
        other => panic!("expected PushState, got {other:?}"),
    }
}

#[test]
fn cyclic_and_bigint_state_succeed_and_degrade_to_null(// CR-3
) {
    // `StructuredSerializeForStorage` SUCCEEDS for cyclic graphs + BigInt (both
    // structured-cloneable); only `JSON.stringify` throws. The interim JSON-shortcut
    // must therefore NOT throw DataCloneError — it degrades to no restorable state
    // (`serialized_state: None`), so the pushState succeeds (URL updates, entry
    // enqueued) and a later cross-document traversal restores `null`. Full-fidelity
    // restore is D1 (`#11-history-state-structured-serialize-fidelity`).
    let mut vm = new_vm_with_base(); // http://localhost/
                                     // A cyclic state: pushState succeeds (no throw).
    vm.eval("var o = {}; o.self = o; history.pushState(o, '', '/cyclic');")
        .unwrap();
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/cyclic");
    match take_one_history(&mut vm) {
        HistoryAction::PushState {
            serialized_state, ..
        } => assert_eq!(
            serialized_state, None,
            "cyclic degrades to None, not a throw"
        ),
        other => panic!("expected PushState, got {other:?}"),
    }
    // A BigInt state: likewise succeeds + degrades.
    vm.eval("history.pushState({v: 10n}, '', '/bigint');")
        .unwrap();
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/bigint");
    match take_one_history(&mut vm) {
        HistoryAction::PushState {
            serialized_state, ..
        } => assert_eq!(serialized_state, None, "BigInt degrades to None"),
        other => panic!("expected PushState, got {other:?}"),
    }
}

#[test]
fn function_state_succeeds_with_null_state_interim(// CR-3 opposite deviation (D1-owned)
) {
    // INTERIM behavior pin: the spec REQUIRES `pushState(function(){})` to throw
    // DataCloneError (a function is non-cloneable), but the JSON shortcut renders it
    // as `undefined` → `serialized_state: None` → the call SUCCEEDS with null state.
    // This is the opposite-direction interim gap the JSON shortcut cannot fix; D1
    // (the full structured-clone walker) turns this back into a DataCloneError throw
    // — at which point this test flips (a visible D1 landing signal).
    let mut vm = new_vm_with_base();
    vm.eval("history.pushState(function () {}, '', '/fn');")
        .unwrap();
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/fn");
    match take_one_history(&mut vm) {
        HistoryAction::PushState {
            serialized_state, ..
        } => assert_eq!(serialized_state, None),
        other => panic!("expected PushState, got {other:?}"),
    }
}

#[test]
fn throwing_tojson_degrades_to_null_interim() {
    // `StructuredSerializeInternal` (WHATWG HTML §2.7.3 step 24) serializes ordinary
    // objects via enumerable-property `Get` and NEVER invokes JSON's `toJSON` hook,
    // so a throwing `toJSON` does NOT abort real structured serialization. The interim
    // JSON shortcut *does* call it and `JSON.stringify` throws — a JSON-only exception
    // that must DEGRADE to no restorable state (like BigInt/cyclic, CR-3), NOT
    // propagate and lose the history entry (Codex R5). The pushState SUCCEEDS: the URL
    // applies and the entry is enqueued with `serialized_state: None`. (A throwing
    // property *getter* — which real clone WOULD propagate via `? Get` — also degrades
    // here, an interim gap the full walker restores; the JSON shortcut cannot tell the
    // two apart.)
    let mut vm = new_vm_with_base(); // http://localhost/
    vm.eval(
        "var o = { toJSON: function () { throw new Error('boom'); } };
         history.pushState(o, '', '/applies');",
    )
    .unwrap();
    // URL side-effect happened (no throw), entry enqueued with null state.
    assert_eq!(eval_string(&mut vm, "location.pathname;"), "/applies");
    match take_one_history(&mut vm) {
        HistoryAction::PushState {
            serialized_state, ..
        } => assert_eq!(
            serialized_state, None,
            "throwing toJSON degrades to None, no throw"
        ),
        other => panic!("expected PushState, got {other:?}"),
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
