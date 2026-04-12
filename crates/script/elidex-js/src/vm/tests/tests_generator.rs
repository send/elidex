//! Generator tests (ES2020 §25.4).
//!
//! - `Op::Yield`-based generators (value yielding, received-value
//!   forwarding via `.next(arg)`, return, iterator protocol).
//! - `.return(v)` / `.throw(e)` with `finally` forwarding (PR2.5).
//!
//! Out of scope (future milestones):
//! - `Op::YieldDelegate` (`yield*` via bytecode expansion — PR2.5 1.2)

use super::{eval_bool, eval_global_number, eval_global_string, eval_number, eval_string};

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

#[test]
fn generator_try_finally_runs_finally_after_normal_completion() {
    // Normal exit from a try block inside a generator: finally runs exactly
    // once when the body completes, observable via an external side-effect
    // string.  Guards against regressing the "finally-runs-twice" bug fixed
    // in commit a5205be — we specifically check the generator path since the
    // fix touched the compiler's Throw lowering, and we want coverage across
    // yield suspension boundaries too.
    assert_eq!(
        eval_global_number(
            "globalThis.n = 0; \
             function* g() { \
                 try { yield 1; yield 2; } \
                 finally { globalThis.n += 1; } \
             } \
             var it = g(); it.next(); it.next(); it.next();",
            "n"
        ),
        1.0
    );
}

#[test]
fn generator_try_catch_finally_ordering_across_yield() {
    // `try { yield; throw }` followed by `catch { ... }` and `finally`
    // must run each branch exactly once, in order T/C/F.  The throw takes
    // the runtime handler path (compile-time inline finally emission was
    // removed from Op::Throw in commit a5205be), so this test would catch a
    // regression that re-introduces the double-finally bug via the
    // generator resume path.
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; \
             function* g() { \
                 try { \
                     yield 1; \
                     globalThis.log += 'T'; \
                     throw 'boom'; \
                 } \
                 catch (e) { globalThis.log += 'C'; } \
                 finally { globalThis.log += 'F'; } \
             } \
             var it = g(); it.next(); it.next();",
            "log"
        ),
        "TCF"
    );
}

#[test]
fn generator_for_of_completion_runs_finally_once() {
    // Driving a generator through `for (...of g())` to normal completion
    // still runs the finally block exactly once, exercising the iterator
    // protocol's value/done transition while a finally is pending.
    assert_eq!(
        eval_global_number(
            "globalThis.n = 0; \
             function* g() { \
                 try { yield 1; yield 2; } \
                 finally { globalThis.n += 1; } \
             } \
             var sum = 0; \
             for (var v of g()) { sum += v; } \
             globalThis.n;",
            "n"
        ),
        1.0
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
fn generator_closure_survives_many_yield_resume_cycles() {
    // Regression guard for the resume-side upvalue bookkeeping fix
    // (Copilot PR #71 #2/#3): `resume_generator` previously re-pushed
    // upvalue ids into `frame.local_upvalue_ids` on every resume, so the
    // list grew unboundedly and each subsequent suspend closed the same
    // upvalue multiple times.  A tight yield loop with a closure reading
    // the captured local keeps the state machine exercised; if
    // accumulation re-appears, each close-reopen round would compound
    // the work and (in pathological cases) corrupt the value written
    // back at resume.
    assert_eq!(
        eval_number(
            "function* g() { \
                 var x = 0; \
                 var read = () => x; \
                 for (var i = 0; i < 50; i++) { \
                     x = i; \
                     yield read(); \
                 } \
             } \
             var s = 0; \
             for (var v of g()) { s += v; } \
             s;"
        ),
        1225.0 // 0 + 1 + ... + 49
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

// ─── Generator return / throw ────────────────────────────────────────────

#[test]
fn generator_return_completes_iterator_no_finally() {
    // No try/finally in scope → `.return(v)` marks the generator Completed
    // with `{value: v, done: true}` without running any user code.
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
fn generator_throw_uncaught_propagates_as_error() {
    // No catch in scope → `.throw(e)` propagates the reason as a VmError.
    let mut vm = crate::vm::Vm::new();
    let err = vm.eval(
        "function* g() { yield 1; } \
         var it = g(); it.next(); it.throw('boom');",
    );
    assert!(err.is_err());
}

#[test]
fn generator_throw_caught_by_in_scope_catch() {
    // `.throw(e)` inside an active try routes through `catch(e)` which
    // yields the replacement value; the generator then completes normally.
    //
    // Sequence: .next() enters try, yields 1 → user calls .throw('boom'),
    // catch binds 'boom' and yields 'caught:boom' → .next() drains to
    // the function end → {undefined, done:true}.
    assert_eq!(
        eval_string(
            "function* g() { \
               try { yield 1; } catch(e) { yield 'caught:' + e; } \
             } \
             var it = g(); it.next(); it.throw('boom').value;"
        ),
        "caught:boom"
    );
}

#[test]
fn generator_throw_on_completed_throws_reason() {
    // §25.4.1.4: throw on a completed iterator surfaces the reason
    // synchronously (the body is already gone — no handler can see it).
    let mut vm = crate::vm::Vm::new();
    let err = vm.eval(
        "function* g() {} \
         var it = g(); it.next(); it.throw('boom');",
    );
    assert!(err.is_err());
}

// ─── Finally forwarding for `.return` / `.throw` ─────────────────────────

#[test]
fn generator_return_runs_in_scope_finally_then_completes() {
    // `try { yield 1 } finally { yield 2 }` + `.return(99)`:
    // - .next() → yield 1
    // - .return(99) enters finally, yields 2 (pending=Return(99))
    // - .next() reaches EndFinally → completes with 99.
    assert_eq!(
        eval_global_string(
            "globalThis.log = []; \
             function* g() { try { yield 1; } finally { globalThis.log.push('f'); yield 2; } } \
             var it = g(); \
             globalThis.log.push(it.next().value); \
             globalThis.log.push(it.return(99).value); \
             var r = it.next(); \
             globalThis.log.push(r.value + '/' + r.done); \
             globalThis.out = globalThis.log.join(',');",
            "out"
        ),
        // log: [1, 'f', 2, '99/true']
        "1,f,2,99/true"
    );
}

#[test]
fn generator_throw_caught_and_finally_also_runs() {
    // `try { yield } catch(e) { yield } finally { yield }` + `.throw`
    // routes to catch first (via handle_exception), then finally runs.
    assert_eq!(
        eval_global_string(
            "globalThis.log = []; \
             function* g() { \
               try { yield 'T'; } \
               catch(e) { globalThis.log.push('C:'+e); yield 'Cy'; } \
               finally { globalThis.log.push('F'); yield 'Fy'; } \
             } \
             var it = g(); \
             globalThis.log.push(it.next().value); \
             globalThis.log.push(it.throw('boom').value); \
             globalThis.log.push(it.next().value); \
             globalThis.out = globalThis.log.join(',');",
            "out"
        ),
        "T,C:boom,Cy,F,Fy"
    );
}

#[test]
fn return_inside_finally_overrides_pending_return() {
    // `try { return 1 } finally { return 2 }` — finally's own return
    // overrides the try's pending return per §13.15.
    assert_eq!(
        eval_number("function f() { try { return 1; } finally { return 2; } } f();"),
        2.0
    );
}

#[test]
fn throw_inside_finally_overrides_try_throw() {
    // `try { throw 'x' } finally { throw 'y' }` — finally's throw
    // overrides the try's throw.  The outer try/catch sees 'y'.
    assert_eq!(
        eval_string(
            "var r; try { try { throw 'x'; } finally { throw 'y'; } } catch(e) { r = e; } r;"
        ),
        "y"
    );
}

#[test]
fn for_of_break_runs_inner_generator_finally() {
    // `for (const v of g()) { break }` — for-of abrupt completion
    // calls inner.return(undefined), which must run the generator's
    // finally block.
    assert_eq!(
        eval_global_string(
            "globalThis.log = []; \
             function* g() { try { yield 1; yield 2; } finally { globalThis.log.push('cleanup'); } } \
             for (var v of g()) { globalThis.log.push(v); break; } \
             globalThis.out = globalThis.log.join(',');",
            "out"
        ),
        "1,cleanup"
    );
}

// ─── yield* (delegate) ────────────────────────────────────────────────────

#[test]
fn yield_star_iterates_array() {
    // `yield* [1,2,3]` yields 1, 2, 3 in sequence then completes.
    assert_eq!(
        eval_number(
            "function* g() { yield* [1, 2, 3]; } \
             var it = g(); it.next().value + it.next().value + it.next().value;"
        ),
        6.0
    );
    assert!(eval_bool(
        "function* g() { yield* [1, 2, 3]; } \
         var it = g(); it.next(); it.next(); it.next(); it.next().done;"
    ));
}

#[test]
fn yield_star_empty_iterable_completes_immediately() {
    // `yield* []` done=true on the first .next().
    assert!(eval_bool("function* g() { yield* []; } g().next().done;"));
}

#[test]
fn yield_star_delegates_to_inner_generator() {
    // Outer drives inner through yield*.  Each outer.next() advances
    // inner.next() exactly once while inner has more values.
    assert_eq!(
        eval_number(
            "function* inner() { yield 10; yield 20; } \
             function* outer() { yield 1; yield* inner(); yield 2; } \
             var it = outer(); \
             it.next().value + it.next().value + it.next().value + it.next().value;"
        ),
        33.0 // 1 + 10 + 20 + 2
    );
}

#[test]
fn yield_star_expression_value_is_inner_return_value() {
    // Per §14.4.14, the value of a `yield* iter` expression is the inner
    // iterator's return value (from `{done:true, value}`).  Here outer
    // yields the captured value afterwards so we can observe it.
    //
    // Call trace:
    //   it.next() #1  → inner.next() yields 1, outer re-yields 1.
    //   it.next() #2  → inner.next() returns 42 (done=true), yield*
    //                   expression value = 42, outer binds r=42 and
    //                   then runs `yield r`, yielding 42.
    assert_eq!(
        eval_number(
            "function* inner() { yield 1; return 42; } \
             function* outer() { var r = yield* inner(); yield r; } \
             var it = outer(); it.next(); it.next().value;"
        ),
        42.0
    );
}

#[test]
fn yield_star_forwards_next_arg_to_inner() {
    // `.next(x)` while suspended inside yield* passes `x` to the inner
    // iterator's `.next(x)` — inner observes it as the value of its own
    // `yield` expression.
    assert_eq!(
        eval_number(
            "function* inner() { var a = yield 1; var b = yield a + 10; return b + 100; } \
             function* outer() { return yield* inner(); } \
             var it = outer(); \
             it.next();       /* inner yields 1 */ \
             it.next(5);       /* a = 5, inner yields 15 */ \
             it.next(7).value; /* b = 7, inner returns 107 */"
        ),
        107.0
    );
}

#[test]
fn yield_star_forwards_return_via_iterator_close() {
    // `.return(v)` on outer while suspended inside yield* runs the
    // inner iterator's `.return()` (via IteratorClose), then completes
    // with `v`.
    assert_eq!(
        eval_global_string(
            "globalThis.log = []; \
             var inner = { \
               next() { return { value: 1, done: false }; }, \
               return(v) { globalThis.log.push('inner.return'); return { value: v, done: true }; }, \
               [Symbol.iterator]() { return this; }, \
             }; \
             function* outer() { yield* inner; } \
             var it = outer(); \
             it.next(); \
             globalThis.log.push(it.return(99).value); \
             globalThis.out = globalThis.log.join(',');",
            "out"
        ),
        "inner.return,99"
    );
}

#[test]
fn yield_star_forwards_throw_closes_inner() {
    // `.throw(e)` on outer while inside yield*: close inner then rethrow.
    // (Proper `iter.throw` method forwarding is a future spec-alignment
    // task — this verifies the close-and-rethrow fallback path.)
    assert_eq!(
        eval_global_string(
            "globalThis.log = []; \
             var inner = { \
               next() { return { value: 1, done: false }; }, \
               return(v) { globalThis.log.push('inner.return'); return { value: v, done: true }; }, \
               [Symbol.iterator]() { return this; }, \
             }; \
             function* outer() { try { yield* inner; } catch(e) { globalThis.log.push('caught:' + e); } } \
             var it = outer(); \
             it.next(); \
             it.throw('boom'); \
             globalThis.out = globalThis.log.join(',');",
            "out"
        ),
        "inner.return,caught:boom"
    );
}

// ─── Sanity: yield outside a generator is a syntax error (compiler) ───────

#[test]
fn yield_outside_generator_function_rejected() {
    // `yield` outside a generator is a SyntaxError at compile time.
    assert!(crate::vm::Vm::new()
        .eval("function f() { yield 1; }")
        .is_err());
}
