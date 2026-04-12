//! `try` / `catch` / `finally` + `throw` tests (ES2020 §13.15).
//!
//! Extracted from `tests/mod.rs` to keep that file under the 1000-line
//! project convention.  Includes the ordering regression tests added
//! alongside the PR2 bytecode-compile fix (previously the compiler
//! pre-emitted the finally body inline before `Op::Throw`, causing
//! double-execution masked by value-overwriting tests).

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
fn eval_finally_runs_on_catch_throw() {
    // throw inside catch must still execute finally
    assert_eq!(
        eval_number(
            "var x = 0; try { try { throw 1; } catch(e) { x = 1; throw 2; } finally { x = 2; } } catch(e) {} x;"
        ),
        2.0
    );
}

// ── try/catch/finally ordering (regression tests for PR2 bug fix) ──
//
// The previous compile path pre-emitted the finally body inline before
// `Op::Throw`, causing it to run once before the catch handler AND again
// after (via the handler fall-through), a double execution hidden in
// value-overwriting tests like `eval_try_catch_finally` above.  These
// tests observe execution ORDER via string concatenation to catch
// regressions should that path ever be reintroduced.

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
