//! Generator tests (ES2020 §25.4).
//!
//! Scope for PR2 commit 4: `Op::Yield`-based generators (value yielding,
//! received-value forwarding via `.next(arg)`, return, iterator protocol).
//!
//! Out of scope (lands in PR2.5 — Generator spec completion):
//! - `Op::YieldDelegate` (`yield*`)
//! - `.return(v)` / `.throw(e)` abrupt-completion forwarding with finally
//! - Await / async function integration (PR2 commit 5)

use super::{eval_bool, eval_global_number, eval_number, eval_string};

// ─── Shape of the generator object ───────────────────────────────────────

#[test]
fn generator_function_call_returns_suspended_iterator() {
    // `function* g() {}` doesn't run the body on call — it returns an
    // iterator-like object.  We verify typeof + the presence of `.next`.
    assert_eq!(eval_string("function* g() {} typeof g();"), "object");
    assert_eq!(eval_string("function* g() {} typeof g().next;"), "function");
}

#[test]
fn generator_body_does_not_run_until_next() {
    // Side effects in the body must NOT fire on the call itself.
    assert_eq!(
        eval_global_number(
            "globalThis.x = 0; \
             function* g() { globalThis.x = 1; yield; } \
             g();",
            "x"
        ),
        0.0
    );
}

// ─── Basic yield / next ──────────────────────────────────────────────────

#[test]
fn generator_yields_sequence_of_values() {
    // The plan's canonical smoke test.
    assert_eq!(
        eval_number(
            "function* g() { yield 1; yield 2; } \
             var it = g(); \
             it.next().value + it.next().value;"
        ),
        3.0
    );
}

#[test]
fn generator_done_flag_after_final_yield() {
    assert!(eval_bool(
        "function* g() { yield 1; } \
         var it = g(); it.next(); it.next().done;"
    ));
}

#[test]
fn generator_final_return_value_delivered_in_last_step() {
    // Explicit `return` in a generator appears as the last `{value, done:true}`.
    assert_eq!(
        eval_number(
            "function* g() { yield 1; return 99; } \
             var it = g(); it.next(); it.next().value;"
        ),
        99.0
    );
}

#[test]
fn generator_next_after_completion_yields_undefined() {
    assert_eq!(
        eval_string(
            "function* g() {} \
             var it = g(); it.next(); typeof it.next().value;"
        ),
        "undefined"
    );
    assert!(eval_bool(
        "function* g() {} \
         var it = g(); it.next(); it.next().done;"
    ));
}

// ─── Received value forwarding via .next(arg) ────────────────────────────

#[test]
fn generator_next_arg_becomes_yield_expression_value() {
    // First `.next()` (no arg) starts the body.  The arg to the SECOND
    // `.next(arg)` replaces the value of the first `yield`.
    assert_eq!(
        eval_number(
            "function* g() { var v = yield 1; return v + 10; } \
             var it = g(); it.next(); it.next(42).value;"
        ),
        52.0
    );
}

#[test]
fn generator_multiple_yields_forward_arg_each_step() {
    assert_eq!(
        eval_string(
            "function* g() { \
                var a = yield 'p1'; \
                var b = yield 'p2'; \
                return a + ',' + b; \
             } \
             var it = g(); \
             it.next(); it.next('A'); it.next('B').value;"
        ),
        "A,B"
    );
}

// ─── Control flow inside generators ──────────────────────────────────────

#[test]
fn generator_yield_inside_for_loop() {
    assert_eq!(
        eval_number(
            "function* g() { for (var i = 0; i < 3; i++) yield i; } \
             var it = g(); \
             it.next().value + it.next().value + it.next().value;"
        ),
        3.0 // 0+1+2
    );
}

#[test]
fn generator_yield_inside_if_branch() {
    assert_eq!(
        eval_number(
            "function* g(n) { if (n > 0) yield n; else yield -1; } \
             g(5).next().value;"
        ),
        5.0
    );
    assert_eq!(
        eval_number(
            "function* g(n) { if (n > 0) yield n; else yield -1; } \
             g(-3).next().value;"
        ),
        -1.0
    );
}

#[test]
fn generator_try_catch_across_yield() {
    // A throw after a yield should land in the surrounding catch.
    assert_eq!(
        eval_number(
            "function* g() { \
                try { yield 1; throw 'boom'; } \
                catch (e) { yield 2; } \
             } \
             var it = g(); it.next(); it.next().value;"
        ),
        2.0
    );
}

// ─── for-of consumes generators ──────────────────────────────────────────

#[test]
fn generator_for_of_consumes_sequence() {
    assert_eq!(
        eval_number(
            "function* g() { yield 10; yield 20; yield 30; } \
             var s = 0; \
             for (var v of g()) { s += v; } \
             s;"
        ),
        60.0
    );
}

// ─── Iterator protocol identity ──────────────────────────────────────────

#[test]
fn generator_symbol_iterator_returns_self() {
    // `g()[Symbol.iterator]() === g()` on the same instance (iterator protocol).
    assert!(eval_bool(
        "function* g() { yield 1; } \
         var it = g(); it[Symbol.iterator]() === it;"
    ));
}

// ─── Closure captures across yield ───────────────────────────────────────

#[test]
fn generator_closure_reads_updated_local() {
    // A closure created BEFORE the first yield reads the local after each
    // yield — writes made by the generator between yields must be visible.
    assert_eq!(
        eval_number(
            "function* g() { \
                var x = 10; \
                var read = () => x; \
                yield read(); \
                x = 20; \
                yield read(); \
             } \
             var it = g(); \
             it.next().value + it.next().value;"
        ),
        30.0
    );
}

#[test]
fn generator_closure_write_between_yields_is_preserved() {
    // A closure created before yield writes to the captured local while
    // the generator is suspended.  After resume the generator sees the
    // external mutation (upvalue close→reopen must round-trip the value).
    assert_eq!(
        eval_global_number(
            "globalThis.result = 0; \
             function* g() { \
                var x = 1; \
                globalThis.ext = () => { x *= 10; }; \
                yield; \
                globalThis.result = x; \
             } \
             var it = g(); \
             it.next(); \
             globalThis.ext();  /* runs while suspended */ \
             it.next();",
            "result"
        ),
        10.0
    );
}

// ─── Generator return / throw (simplified semantics) ──────────────────────

#[test]
fn generator_return_completes_iterator() {
    // Simplified `.return(v)` (PR2.5 will add finally-block forwarding).
    assert!(eval_bool(
        "function* g() { yield 1; yield 2; } \
         var it = g(); it.next(); \
         var r = it.return(99); \
         r.done;"
    ));
    assert_eq!(
        eval_number(
            "function* g() { yield 1; yield 2; } \
             var it = g(); it.next(); it.return(99).value;"
        ),
        99.0
    );
}

#[test]
fn generator_throw_propagates_as_error() {
    // Simplified `.throw(e)` — native propagates the reason; PR2.5 adds
    // catch-block forwarding inside the generator.
    let mut vm = crate::vm::Vm::new();
    let err = vm.eval(
        "function* g() { yield 1; } \
         var it = g(); it.next(); it.throw('boom');",
    );
    assert!(err.is_err());
}

// ─── Sanity: yield outside a generator is a syntax error (compiler) ───────

#[test]
fn yield_outside_generator_function_rejected() {
    // `yield` outside a generator is a SyntaxError at compile time.
    assert!(crate::vm::Vm::new()
        .eval("function f() { yield 1; }")
        .is_err());
}
