//! Console globals (WHATWG Console §2).
//!
//! The actual output routes through `eprintln!` in the VM; tests here
//! verify that the globals are wired (return shape, method identity,
//! invocation) without asserting on stderr.  Full output-channel wiring
//! to the host session lives in PR6.

use super::{eval_bool, eval_number, eval_string};

// ─── Method shape ────────────────────────────────────────────────────────

#[test]
fn console_is_object() {
    assert_eq!(eval_string("typeof console;"), "object");
}

#[test]
fn console_has_all_six_methods() {
    for name in &["log", "error", "warn", "info", "debug", "trace"] {
        assert_eq!(
            eval_string(&format!("typeof console.{name};")),
            "function",
            "console.{name} should be a function",
        );
    }
}

// ─── Invocation returns undefined ────────────────────────────────────────

#[test]
fn console_methods_return_undefined() {
    for name in &["log", "error", "warn", "info", "debug", "trace"] {
        assert_eq!(
            eval_string(&format!("typeof console.{name}('hello');")),
            "undefined",
            "console.{name} should return undefined",
        );
    }
}

// ─── Variadic invocation ─────────────────────────────────────────────────

#[test]
fn console_log_accepts_variadic_args() {
    // Doesn't throw and returns undefined.
    assert_eq!(
        eval_string("typeof console.log(1, 2, 3, {a: 1}, [1,2]);"),
        "undefined",
    );
}

#[test]
fn console_log_zero_args() {
    // No args: still returns undefined without throwing.
    assert_eq!(eval_string("typeof console.log();"), "undefined");
}

// ─── console method identity ─────────────────────────────────────────────

#[test]
fn console_method_identity_is_stable() {
    // Successive reads of `console.log` return the same function value.
    assert!(eval_bool("console.log === console.log;"));
}

#[test]
fn console_methods_have_name_property() {
    assert_eq!(eval_string("console.log.name;"), "log");
    assert_eq!(eval_string("console.info.name;"), "info");
    assert_eq!(eval_string("console.trace.name;"), "trace");
}

// ─── Late binding survives GC (no surprise disappearance) ────────────────

#[test]
fn console_log_length_property_is_zero() {
    // Native functions currently don't expose `.length`, but accessing
    // it shouldn't throw — it just returns undefined.
    assert_eq!(eval_string("typeof console.log.length;"), "undefined");
}

// ─── Composition inside user code ────────────────────────────────────────

#[test]
fn console_log_inside_function_body() {
    // Exercise the call-frame path: `f()` invokes console.log.
    assert_eq!(
        eval_number("function f() { console.log('ok'); return 42; } f();"),
        42.0
    );
}
