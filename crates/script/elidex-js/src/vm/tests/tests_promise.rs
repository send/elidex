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
    // rejects.  Verify the catch handler observes the thrown reason as a
    // string (elidex-js uses a descriptive message rather than a fresh
    // TypeError instance; exact shape may tighten when Error wiring lands).
    assert_eq!(
        eval_global_string(
            "globalThis.r = ''; var captured; \
             var p = new Promise((resolve, _) => { captured = resolve; }); \
             captured(p); \
             p.catch(e => { globalThis.r = typeof e; });",
            "r"
        ),
        "string"
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
