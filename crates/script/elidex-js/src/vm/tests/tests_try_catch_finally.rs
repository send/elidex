//! `try` / `catch` / `finally` + `throw` tests (ES2020 §13.15).
//!
//! Includes ordering regression tests that catch the "finally runs
//! before catch and then again after" bug that value-overwriting tests
//! could miss.

use super::{eval, eval_global_string, eval_number};

#[test]
fn eval_throw_uncaught() {
    let result = eval("throw 42;");
    assert!(result.is_err());
}

#[test]
fn eval_try_catch_basic() {
    assert_eq!(
        eval_number("var r = 0; try { throw 42; } catch(e) { r = e; } r;"),
        42.0
    );
}

#[test]
fn eval_try_catch_no_throw() {
    assert_eq!(
        eval_number("var r = 0; try { r = 1; } catch(e) { r = 99; } r;"),
        1.0
    );
}

#[test]
fn eval_try_finally() {
    assert_eq!(
        eval_number("var r = 0; try { r = 1; } finally { r = 2; } r;"),
        2.0
    );
}

#[test]
fn eval_try_catch_finally() {
    assert_eq!(
        eval_number(
            "var r = 0; try { throw 1; } catch(e) { r = e + 10; } finally { r = r + 100; } r;"
        ),
        111.0
    );
}

#[test]
fn eval_nested_try_catch() {
    assert_eq!(
        eval_number(
            "var r = 0; try { try { throw 1; } catch(e) { r = e; } throw 2; } catch(e) { r = r + e; } r;"
        ),
        3.0 // inner catch: r=1, outer catch: r=1+2=3
    );
}

#[test]
fn eval_finally_runs_on_return() {
    // finally must execute even when try block returns
    assert_eq!(
        eval_number(
            "var x = 0; function f() { try { x = 1; return 42; } finally { x = 2; } } f(); x;"
        ),
        2.0
    );
}

#[test]
fn eval_finally_runs_on_break() {
    assert_eq!(
        eval_number("var x = 0; while (true) { try { x = 1; break; } finally { x = 2; } } x;"),
        2.0
    );
}

#[test]
fn eval_finally_runs_on_catch_return() {
    // `return` inside a catch block must still inline the enclosing
    // try's finally body before the abrupt return takes effect.
    // Regression for the `finally_stack` pop-too-early bug (Copilot
    // review, PR2.5): the pop must happen before the finally body
    // itself is compiled — NOT before the catch body.
    //
    // Expected: try throws → catch runs (x=2, would return 43) →
    // return 43 inlines finally (x = x*10 = 20) → but finally body
    // is also a `try {} finally {}` participant on the outer f(),
    // which the inline form covers.  f() returns 43, x === 20.
    assert_eq!(
        eval_number(
            "var x = 0; \
             function f() { \
               try { throw 0; } \
               catch (e) { x = 2; return 43; } \
               finally { x = x * 10; } \
             } \
             f(); x;"
        ),
        20.0
    );
}

#[test]
fn eval_catch_return_overridden_by_finally_return() {
    // §13.15: a `return` from finally overrides a `return` from catch.
    // Requires the catch-body's `return` to inline the enclosing
    // finally — that finally's own `return` fires first, bypassing
    // catch's `return 43`.
    assert_eq!(
        eval_number(
            "function f() { \
               try { throw 0; } \
               catch (e) { return 43; } \
               finally { return 99; } \
             } \
             f();"
        ),
        99.0
    );
}

#[test]
fn eval_finally_runs_on_catch_break() {
    // `break` inside catch must inline the enclosing try's finally.
    assert_eq!(
        eval_number(
            "var x = 0; \
             while (true) { \
               try { throw 0; } \
               catch (e) { x = 2; break; } \
               finally { x = x * 10; } \
             } \
             x;"
        ),
        20.0
    );
}

#[test]
fn eval_finally_runs_on_catch_throw() {
    // throw inside catch must still execute finally
    assert_eq!(
        eval_number(
            "var x = 0; try { try { throw 1; } catch(e) { x = 1; throw 2; } finally { x = 2; } } catch(e) {} x;"
        ),
        2.0
    );
}

// ── Ordering: throw → catch → finally ──
//
// If the compiler ever pre-emits the finally body inline before
// `Op::Throw`, these tests fail: the handler mechanism already runs
// finally on catch fall-through, so an inline copy would run finally
// twice.  String concatenation makes the ordering observable (the older
// `eval_try_catch_finally` above used `+=` with numbers, which masked
// the bug because the final sum was the same regardless of order).

#[test]
fn eval_tcf_order_throw_with_catch_and_finally() {
    // Spec §13.15: try throws → catch runs → finally runs.  Sequence "TCF".
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; \
             try { globalThis.log += 'T'; throw 'boom'; } \
             catch (e) { globalThis.log += 'C' + e; } \
             finally { globalThis.log += 'F'; }",
            "log"
        ),
        "TCboomF"
    );
}

#[test]
fn eval_tcf_order_empty_catch_runs_finally_once() {
    // Even with no catch body code, finally should run exactly once.
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; \
             try { globalThis.log += 'T'; throw 1; } \
             catch(e) {} \
             finally { globalThis.log += 'F'; }",
            "log"
        ),
        "TF"
    );
}

#[test]
fn eval_tcf_order_no_throw_still_runs_finally() {
    assert_eq!(
        eval_global_string(
            "globalThis.log = ''; \
             try { globalThis.log += 'T'; } \
             catch (e) { globalThis.log += 'C'; } \
             finally { globalThis.log += 'F'; }",
            "log"
        ),
        "TF"
    );
}
