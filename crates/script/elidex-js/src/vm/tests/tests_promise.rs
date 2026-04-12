//! Promise tests (ES2020 §25.6).
//!
//! Covers the commit-1 surface: Promise constructor, static `resolve`/`reject`,
//! `prototype.then`/`catch`, and microtask-driven reaction dispatch at
//! end-of-eval.  Generators / async / static combinators come in later
//! commits of PR2.
//!
//! Reactions fire asynchronously, so assertions on values set from handlers
//! use [`super::eval_global_number`] / [`super::eval_global_string`] which
//! read the relevant `globalThis.<name>` **after** `eval`'s microtask drain
//! completes.  Assignments to `globalThis.<name>` inside handlers route
//! through the VM's globals HashMap (see `eval_global_object_set_property_syncs_to_globals`);
//! top-level `var` declarations in the script body stay as script-frame
//! locals and are not observable via `get_global`.

use super::{eval_bool, eval_global_number, eval_global_string, eval_string};

// ─── Basic identity / typeof ─────────────────────────────────────────────

#[test]
fn promise_typeof_is_object() {
    assert_eq!(eval_string("typeof Promise.resolve(1);"), "object");
}

#[test]
fn promise_instanceof() {
    assert!(eval_bool("Promise.resolve(1) instanceof Promise;"));
}

// ─── Promise.resolve ─────────────────────────────────────────────────────

#[test]
fn promise_resolve_fires_microtask() {
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; Promise.resolve(42).then(v => { globalThis.x = v; });",
            "x"
        ),
        42.0
    );
}

#[test]
fn promise_resolve_pass_through_preserves_identity() {
    // Promise.resolve(p) === p when p is already a Promise.
    assert!(eval_bool(
        "var p = Promise.resolve(1); Promise.resolve(p) === p;"
    ));
}

#[test]
fn promise_resolve_is_asynchronous() {
    // The handler MUST run AFTER the synchronous statements below it — so
    // `log` is set to "A" first, then "AB" during microtask drain.
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; \
             Promise.resolve(1).then(() => { globalThis.log = globalThis.log + 'B'; }); \
             globalThis.log = globalThis.log + 'A';",
            "log"
        ),
        "AB"
    );
}

// ─── Promise.reject / catch ───────────────────────────────────────────────

#[test]
fn promise_reject_invokes_catch_handler() {
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; Promise.reject(7).catch(r => { globalThis.x = r; });",
            "x"
        ),
        7.0
    );
}

#[test]
fn promise_then_onrejected_equivalent_to_catch() {
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; Promise.reject(9).then(undefined, r => { globalThis.x = r; });",
            "x"
        ),
        9.0
    );
}

// ─── Chaining ────────────────────────────────────────────────────────────

#[test]
fn promise_then_returns_new_promise() {
    assert!(eval_bool(
        "var p = Promise.resolve(1); var q = p.then(v => v); q !== p;"
    ));
}

#[test]
fn promise_chain_propagates_value() {
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             Promise.resolve(1).then(v => v + 1).then(v => v * 3).then(v => { globalThis.x = v; });",
            "x"
        ),
        6.0
    );
}

#[test]
fn promise_chain_propagates_reject_through_gaps() {
    // When `.then` has no reject handler, the rejection skips it and
    // reaches the next `.catch`.
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             Promise.reject(5).then(v => v + 1).catch(r => { globalThis.x = r; });",
            "x"
        ),
        5.0
    );
}

#[test]
fn promise_handler_throw_becomes_rejection() {
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             Promise.resolve(1).then(() => { throw 11; }).catch(r => { globalThis.x = r; });",
            "x"
        ),
        11.0
    );
}

// ─── Constructor executor ─────────────────────────────────────────────────

#[test]
fn promise_constructor_resolve() {
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             new Promise((resolve, _) => { resolve(3); }).then(v => { globalThis.x = v; });",
            "x"
        ),
        3.0
    );
}

#[test]
fn promise_constructor_reject() {
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             new Promise((_, reject) => { reject(4); }).catch(r => { globalThis.x = r; });",
            "x"
        ),
        4.0
    );
}

#[test]
fn promise_constructor_executor_throw_rejects() {
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             new Promise(() => { throw 'bang'; }).catch(r => { globalThis.x = r.length; });",
            "x"
        ),
        4.0 // "bang".length
    );
}

#[test]
fn promise_idempotent_settle() {
    // Spec §25.6.1.3: once settled, subsequent resolve/reject are no-ops.
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             new Promise((resolve, reject) => { resolve(1); resolve(2); reject(3); }) \
               .then(v => { globalThis.x = v; });",
            "x"
        ),
        1.0
    );
}

#[test]
fn promise_self_resolution_rejects_with_typeerror() {
    // §25.6.1.3.2 step 7: resolving a promise with itself ⇒ the promise
    // rejects with a fresh TypeError.  Verify both the `.name` and the
    // message shape.
    assert_eq!(
        eval_global_string(
            "globalThis.out = ''; var captured; \
             var p = new Promise((resolve, _) => { captured = resolve; }); \
             captured(p); \
             p.catch(e => { globalThis.out = e.name + ':' + e.message; });",
            "out"
        ),
        "TypeError:Chaining cycle detected for promise"
    );
}

// ─── Non-callable then handlers are ignored ───────────────────────────────

#[test]
fn promise_then_non_callable_fulfill_is_passthrough() {
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             Promise.resolve(7).then(42).then(v => { globalThis.x = v; });",
            "x"
        ),
        7.0
    );
}

#[test]
fn promise_then_non_callable_reject_is_passthrough() {
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             Promise.reject(8).then(undefined, 'not callable').catch(r => { globalThis.x = r; });",
            "x"
        ),
        8.0
    );
}

// ─── Already-settled reactions still fire asynchronously ──────────────────

#[test]
fn promise_already_settled_then_still_async() {
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; var p = Promise.resolve(1); \
             p.then(() => { globalThis.log = globalThis.log + 'B'; }); \
             globalThis.log = globalThis.log + 'A';",
            "log"
        ),
        "AB"
    );
}

// ─── Promise used as resolve() argument forwards settlement ───────────────

#[test]
fn promise_resolve_with_pending_promise_forwards() {
    // resolve(innerPromise) waits for innerPromise to settle, then
    // propagates its result to the outer promise.
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             var inner = Promise.resolve(5); \
             new Promise((r, _) => { r(inner); }).then(v => { globalThis.x = v; });",
            "x"
        ),
        5.0
    );
}

// ─── queueMicrotask + drain ordering ─────────────────────────────────────

#[test]
fn queue_microtask_runs_after_script() {
    // Bare callback: synchronous statements finish first, then microtasks drain.
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; \
             queueMicrotask(() => { globalThis.log = globalThis.log + 'B'; }); \
             globalThis.log = globalThis.log + 'A';",
            "log"
        ),
        "AB"
    );
}

#[test]
fn queue_microtask_fifo_order() {
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; \
             queueMicrotask(() => { globalThis.log = globalThis.log + '1'; }); \
             queueMicrotask(() => { globalThis.log = globalThis.log + '2'; }); \
             queueMicrotask(() => { globalThis.log = globalThis.log + '3'; });",
            "log"
        ),
        "123"
    );
}

#[test]
fn queue_microtask_nested_enqueues_run_in_same_drain() {
    // A microtask that enqueues another microtask — both must fire within
    // the same drain (HTML §8.1.4.2 step 7: drain continues until empty).
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; \
             queueMicrotask(() => { \
                 globalThis.log = globalThis.log + 'A'; \
                 queueMicrotask(() => { globalThis.log = globalThis.log + 'B'; }); \
             });",
            "log"
        ),
        "AB"
    );
}

#[test]
fn queue_microtask_non_callable_throws() {
    // TypeError when the argument isn't a function.  The throw is
    // synchronous (from the native call), not async.
    let result = super::eval("queueMicrotask(42);");
    assert!(result.is_err());
}

#[test]
fn queue_microtask_callback_error_is_swallowed() {
    // A throwing callback must not abort the rest of the drain.
    // The later callback should still run.
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; \
             queueMicrotask(() => { throw 'boom'; }); \
             queueMicrotask(() => { globalThis.log = globalThis.log + 'survived'; });",
            "log"
        ),
        "survived"
    );
}

#[test]
fn queue_microtask_interleaves_with_promise_reactions_fifo() {
    // Microtasks are one global FIFO queue — queueMicrotask callbacks and
    // Promise reactions are dispatched in the order they were enqueued.
    //
    // Script order: enqueue 'A', then Promise.resolve(…).then enqueues 'B',
    // then enqueue 'C'.  Expected drain order: A, B, C.
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; \
             queueMicrotask(() => { globalThis.log = globalThis.log + 'A'; }); \
             Promise.resolve(0).then(() => { globalThis.log = globalThis.log + 'B'; }); \
             queueMicrotask(() => { globalThis.log = globalThis.log + 'C'; });",
            "log"
        ),
        "ABC"
    );
}

// ─── Promise.all ──────────────────────────────────────────────────────────

#[test]
fn promise_all_resolves_with_array_of_values() {
    assert_eq!(
        eval_global_number(
            "globalThis.sum = 0; \
             Promise.all([1, Promise.resolve(2), 3]).then(arr => { \
                 globalThis.sum = arr[0] + arr[1] + arr[2]; \
             });",
            "sum"
        ),
        6.0
    );
}

#[test]
fn promise_all_preserves_input_order() {
    // Values must appear at the same index as their input, even when
    // inner promises fulfil in a different order (here: chained .then
    // enqueues happen at microtask time).
    assert_eq!(
        eval_global_string(
            "globalThis.out = ''; \
             var p1 = Promise.resolve('a'); \
             var p2 = Promise.resolve('b'); \
             var p3 = Promise.resolve('c'); \
             Promise.all([p1, p2, p3]).then(arr => { globalThis.out = arr.join(','); });",
            "out"
        ),
        "a,b,c"
    );
}

#[test]
fn promise_all_empty_resolves_with_empty_array() {
    assert_eq!(
        eval_global_number(
            "globalThis.len = -1; \
             Promise.all([]).then(arr => { globalThis.len = arr.length; });",
            "len"
        ),
        0.0
    );
}

#[test]
fn promise_all_rejects_on_first_rejection() {
    assert_eq!(
        eval_global_number(
            "globalThis.err = 0; \
             Promise.all([1, Promise.reject(42), 3]).catch(r => { globalThis.err = r; });",
            "err"
        ),
        42.0
    );
}

// ─── Promise.allSettled ───────────────────────────────────────────────────

#[test]
fn promise_all_settled_reports_each_outcome() {
    assert_eq!(
        eval_global_string(
            "globalThis.out = ''; \
             Promise.allSettled([Promise.resolve(1), Promise.reject(2), 3]).then(arr => { \
                 globalThis.out = arr[0].status + ',' + arr[0].value + '|' \
                                 + arr[1].status + ',' + arr[1].reason + '|' \
                                 + arr[2].status + ',' + arr[2].value; \
             });",
            "out"
        ),
        "fulfilled,1|rejected,2|fulfilled,3"
    );
}

#[test]
fn promise_all_settled_empty() {
    assert_eq!(
        eval_global_number(
            "globalThis.len = -1; \
             Promise.allSettled([]).then(arr => { globalThis.len = arr.length; });",
            "len"
        ),
        0.0
    );
}

// ─── Promise.race ─────────────────────────────────────────────────────────

#[test]
fn promise_race_resolves_with_first_fulfilled() {
    // Already-settled plain values in the iterable settle synchronously
    // (as microtasks), so the first item wins.
    assert_eq!(
        eval_global_number(
            "globalThis.winner = 0; \
             Promise.race([1, Promise.resolve(2), 3]).then(v => { globalThis.winner = v; });",
            "winner"
        ),
        1.0
    );
}

#[test]
fn promise_race_rejects_with_first_rejection() {
    assert_eq!(
        eval_global_number(
            "globalThis.err = 0; \
             Promise.race([Promise.reject(9), Promise.resolve(1)]).catch(r => { globalThis.err = r; });",
            "err"
        ),
        9.0
    );
}

// ─── Promise.any ──────────────────────────────────────────────────────────

#[test]
fn promise_any_resolves_with_first_fulfilled() {
    assert_eq!(
        eval_global_number(
            "globalThis.val = 0; \
             Promise.any([Promise.reject(1), Promise.resolve(7), Promise.reject(2)]).then(v => { \
                 globalThis.val = v; \
             });",
            "val"
        ),
        7.0
    );
}

#[test]
fn promise_any_rejects_with_aggregate_when_all_reject() {
    // AggregateError: we expect `.errors` to be an array of the rejection
    // reasons in input order, and `.message` to be non-empty.
    assert_eq!(
        eval_global_string(
            "globalThis.out = ''; \
             Promise.any([Promise.reject('a'), Promise.reject('b')]).catch(e => { \
                 globalThis.out = e.name + ':' + e.errors.join(','); \
             });",
            "out"
        ),
        "AggregateError:a,b"
    );
}

#[test]
fn promise_any_empty_rejects_immediately() {
    assert_eq!(
        eval_global_string(
            "globalThis.out = ''; \
             Promise.any([]).catch(e => { globalThis.out = e.name; });",
            "out"
        ),
        "AggregateError"
    );
}

// ─── Promise.prototype.finally ────────────────────────────────────────────

#[test]
fn promise_finally_runs_on_fulfill_and_passes_value() {
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; \
             Promise.resolve(5).finally(() => { globalThis.log += 'F'; }).then(v => { \
                 globalThis.log += 'v' + v; \
             });",
            "log"
        ),
        "Fv5"
    );
}

#[test]
fn promise_finally_runs_on_reject_and_preserves_reason() {
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; \
             Promise.reject(7).finally(() => { globalThis.log += 'F'; }).catch(r => { \
                 globalThis.log += 'r' + r; \
             });",
            "log"
        ),
        "Fr7"
    );
}

#[test]
fn promise_finally_throw_overrides_reason() {
    // Spec §25.6.5.3: if `onFinally` itself throws, the derived promise
    // rejects with that error, overriding the original outcome.
    assert_eq!(
        eval_global_number(
            "globalThis.err = 0; \
             Promise.resolve(1).finally(() => { throw 99; }).catch(r => { globalThis.err = r; });",
            "err"
        ),
        99.0
    );
}

// ─── Unhandled-rejection tracking ─────────────────────────────────────────
//
// The warning output itself is an `eprintln!` stream (PR3 will swap it for
// a proper PromiseRejectionEvent); here we verify the *state machine* —
// that attaching a reject handler marks the promise handled, and that
// already-handled rejections do not appear in `pending_rejections`.

#[test]
fn promise_catch_clears_pending_rejection() {
    use crate::vm::Vm;

    let mut vm = Vm::new();
    // Rejected promise with a .catch: must NOT remain in pending_rejections
    // after the drain (the .catch microtask marks handled=true).
    vm.eval("Promise.reject(1).catch(() => {});").unwrap();
    assert!(
        vm.inner.pending_rejections.is_empty(),
        "handled rejection must not leave a pending entry"
    );
}

#[test]
fn promise_unhandled_rejection_marks_handled_after_warning() {
    use crate::vm::Vm;

    // A bare Promise.reject with no handler: after eval's drain, the
    // end-of-drain scan emits a warning and marks the promise handled so
    // subsequent drains don't re-warn.
    let mut vm = Vm::new();
    vm.eval("Promise.reject('bare');").unwrap();
    // pending_rejections is cleared at drain end.
    assert!(vm.inner.pending_rejections.is_empty());
    // A second drain (no new rejections) is a no-op.
    vm.inner.drain_microtasks();
    assert!(vm.inner.pending_rejections.is_empty());
}

#[test]
fn promise_late_catch_still_clears_handled() {
    // A .catch attached AFTER the rejecting promise settles still marks
    // the source promise handled — the tracker tolerates this pattern
    // (it's common in real code).
    use crate::vm::Vm;

    let mut vm = Vm::new();
    vm.eval(
        "globalThis.x = 0; \
         var p = Promise.reject(42); \
         p.catch(r => { globalThis.x = r; });",
    )
    .unwrap();
    assert_eq!(
        vm.get_global("x"),
        Some(crate::vm::value::JsValue::Number(42.0))
    );
    assert!(vm.inner.pending_rejections.is_empty());
}

#[test]
fn promise_chain_dispatched_in_separate_microtasks() {
    // Each link in a promise chain settles in its own microtask.  A chain
    // of three .then()s interleaved with two queueMicrotask()s demonstrates
    // the FIFO flow: microtasks enqueued before a chained .then() run
    // before the derived promise's reaction fires.
    //
    //   p = Promise.resolve()
    //   p.then(A) — enqueues reaction for A immediately
    //   queueMicrotask(B) — enqueues B
    //   p.then(C) — enqueues reaction for C (p is already fulfilled)
    //
    // Queue order: [A, B, C]
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; \
             var p = Promise.resolve(); \
             p.then(() => { globalThis.log = globalThis.log + 'A'; }); \
             queueMicrotask(() => { globalThis.log = globalThis.log + 'B'; }); \
             p.then(() => { globalThis.log = globalThis.log + 'C'; });",
            "log"
        ),
        "ABC"
    );
}
