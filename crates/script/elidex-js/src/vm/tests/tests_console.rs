//! Console globals (WHATWG Console §2).
//!
//! The printed output routes through `eprintln!` in the VM; the shape
//! tests here verify that the globals are wired (return shape, method
//! identity, invocation) without asserting on stderr.  The capture tests
//! assert on the bounded per-VM tee buffer behind
//! [`Vm::console_messages`] (the S5-6 B26 test-oracle accessor).

use super::super::natives::CONSOLE_CAPTURE_LIMIT;
use super::super::Vm;
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

// ─── Capture buffer (B26 test-oracle accessor) ───────────────────────────

#[test]
fn console_messages_captures_log_in_order() {
    let mut vm = Vm::new();
    vm.eval("console.log('hello', 42); console.warn('careful'); console.error('boom');")
        .unwrap();
    assert_eq!(
        vm.console_messages(),
        vec![
            ("log".to_string(), "hello 42".to_string()),
            ("warn".to_string(), "careful".to_string()),
            ("error".to_string(), "boom".to_string()),
        ]
    );
}

#[test]
fn console_messages_persist_across_evals() {
    let mut vm = Vm::new();
    vm.eval("console.log('first');").unwrap();
    vm.eval("console.log('second');").unwrap();
    let msgs = vm.console_messages();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].1, "first");
    assert_eq!(msgs[1].1, "second");
}

#[test]
fn console_capture_is_bounded_dropping_oldest() {
    let mut vm = Vm::new();
    let n = CONSOLE_CAPTURE_LIMIT + 5;
    vm.eval(&format!(
        "for (var i = 0; i < {n}; i++) {{ console.log('m' + i); }}"
    ))
    .unwrap();
    let msgs = vm.console_messages();
    assert_eq!(msgs.len(), CONSOLE_CAPTURE_LIMIT, "bound respected");
    // Oldest entries dropped: the buffer starts at m5 and ends at the last.
    assert_eq!(msgs[0].1, "m5");
    assert_eq!(msgs[msgs.len() - 1].1, format!("m{}", n - 1));
}
