//! Async function tests (ES2020 §14.6 / §25.5).
//!
//! Commit 5 of PR2 lands async function + `await` on top of the generator
//! infrastructure: an async function call compiles like a generator body
//! but returns a wrapper Promise, and each `await` suspends the coroutine
//! until the awaited Promise settles.
//!
//! Out of scope (later work):
//! - Async generator functions (`async function*`)
//! - Top-level `await` in modules
//! - Abrupt-completion forwarding via `.throw()` on a paused coroutine

use super::{eval_bool, eval_global_number, eval_global_string};

// ─── Return shape ────────────────────────────────────────────────────────

#[test]
fn async_function_returns_promise() {
    // Calling an async function returns a Promise even with an empty body.
    assert!(eval_bool("async function f() {} f() instanceof Promise;"));
}

#[test]
fn async_function_return_value_fulfills_wrapper() {
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             async function f() { return 42; } \
             f().then(v => { globalThis.x = v; });",
            "x"
        ),
        42.0
    );
}

#[test]
fn async_function_throw_rejects_wrapper() {
    assert_eq!(
        eval_global_number(
            "globalThis.err = 0; \
             async function f() { throw 7; } \
             f().catch(r => { globalThis.err = r; });",
            "err"
        ),
        7.0
    );
}

// ─── Await ───────────────────────────────────────────────────────────────

#[test]
fn await_fulfilled_promise_resumes_with_value() {
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             async function f() { \
                 var v = await Promise.resolve(10); \
                 return v + 1; \
             } \
             f().then(v => { globalThis.x = v; });",
            "x"
        ),
        11.0
    );
}

#[test]
fn await_rejected_promise_throws_inside_body() {
    // A rejected awaited Promise makes the `await` expression throw —
    // the body's try/catch should observe the reason.
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             async function f() { \
                 try { await Promise.reject(5); } \
                 catch (e) { globalThis.x = e; } \
             } \
             f();",
            "x"
        ),
        5.0
    );
}

#[test]
fn await_plain_value_acts_like_resolve() {
    // `await v` where v is not a Promise: Promise.resolve(v) first, then
    // continue with v as the await result.
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             async function f() { \
                 var v = await 99; \
                 return v; \
             } \
             f().then(v => { globalThis.x = v; });",
            "x"
        ),
        99.0
    );
}

#[test]
fn multiple_awaits_sequence_correctly() {
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             async function f() { \
                 var a = await 1; \
                 var b = await Promise.resolve(2); \
                 var c = await 4; \
                 return a + b + c; \
             } \
             f().then(v => { globalThis.x = v; });",
            "x"
        ),
        7.0
    );
}

#[test]
fn await_chained_promise_unwraps() {
    // `await Promise.resolve(Promise.resolve(v))` still yields `v`
    // because the outer resolve assimilates the inner Promise.
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             async function f() { \
                 var v = await Promise.resolve(Promise.resolve(42)); \
                 return v; \
             } \
             f().then(v => { globalThis.x = v; });",
            "x"
        ),
        42.0
    );
}

// ─── Timing ───────────────────────────────────────────────────────────────

#[test]
fn async_function_body_is_asynchronous_from_first_await() {
    // Statements between the call and the first await run synchronously.
    // Statements after the first await run in a microtask.
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; \
             async function f() { \
                 globalThis.log = globalThis.log + 'A'; \
                 await 0; \
                 globalThis.log = globalThis.log + 'C'; \
             } \
             f(); \
             globalThis.log = globalThis.log + 'B';",
            "log"
        ),
        "ABC"
    );
}

// ─── Control flow ────────────────────────────────────────────────────────

#[test]
fn await_inside_for_loop() {
    assert_eq!(
        eval_global_number(
            "globalThis.sum = 0; \
             async function f() { \
                 var total = 0; \
                 for (var i = 0; i < 4; i++) { \
                     total += await i; \
                 } \
                 return total; \
             } \
             f().then(v => { globalThis.sum = v; });",
            "sum"
        ),
        6.0 // 0+1+2+3
    );
}

#[test]
fn await_inside_try_catch_catches_rejection() {
    // await + try/catch: a rejected awaited Promise should land in the
    // surrounding catch block.
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; \
             async function f() { \
                 try { \
                     await Promise.reject('boom'); \
                 } \
                 catch (e) { globalThis.log = 'caught:' + e; } \
             } \
             f();",
            "log"
        ),
        "caught:boom"
    );
}

#[test]
fn await_inside_try_finally_runs_finally_on_normal_completion() {
    // Normal completion path: await → finally → return.
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; \
             async function f() { \
                 try { await 1; globalThis.log += 'T'; } \
                 finally { globalThis.log += 'F'; } \
             } \
             f();",
            "log"
        ),
        "TF"
    );
}

#[test]
fn await_inside_try_catch_finally_runs_all_branches_in_order() {
    // Throw after await lands in catch, then finally runs.  Sequence
    // "TCboomF" matches spec §13.15.  This test depends on the compile-
    // side fix that removed the inline finally emission before Op::Throw.
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; \
             async function f() { \
                 try { \
                     await 1; \
                     globalThis.log += 'T'; \
                     throw 'boom'; \
                 } \
                 catch (e) { globalThis.log += 'C' + e; } \
                 finally { globalThis.log += 'F'; } \
             } \
             f();",
            "log"
        ),
        "TCboomF"
    );
}

// ─── Async arrow functions ───────────────────────────────────────────────

#[test]
fn async_arrow_function_works() {
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             var f = async (v) => await v + 1; \
             f(5).then(v => { globalThis.x = v; });",
            "x"
        ),
        6.0
    );
}

// ─── Rejection propagation ────────────────────────────────────────────────

#[test]
fn uncaught_reject_inside_async_rejects_wrapper() {
    // If a rejected await isn't caught, the wrapper rejects with the same reason.
    assert_eq!(
        eval_global_number(
            "globalThis.err = 0; \
             async function f() { await Promise.reject(8); } \
             f().catch(r => { globalThis.err = r; });",
            "err"
        ),
        8.0
    );
}

#[test]
fn async_function_composition() {
    // Compose two async functions — the outer awaits the inner's Promise.
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             async function inner(n) { return n * 2; } \
             async function outer() { \
                 var a = await inner(3); \
                 var b = await inner(5); \
                 return a + b; \
             } \
             outer().then(v => { globalThis.x = v; });",
            "x"
        ),
        16.0 // 6 + 10
    );
}
