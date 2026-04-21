//! `Blob` tests (File API §3, minimal Phase 2 form).
//!
//! Covers ctor (parts / options), `size` + `type` IDL attrs,
//! `slice` (range + type override), and the Promise-returning
//! body reads (`text()` / `arrayBuffer()`).

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

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

fn eval_global_string(source: &str, name: &str) -> String {
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    match vm.get_global(name) {
        Some(JsValue::String(id)) => vm.get_string(id),
        other => panic!("expected global {name} to be a string, got {other:?}"),
    }
}

fn eval_global_number(source: &str, name: &str) -> f64 {
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    match vm.get_global(name) {
        Some(JsValue::Number(n)) => n,
        other => panic!("expected global {name} to be a number, got {other:?}"),
    }
}

#[test]
fn ctor_empty_size_zero() {
    let mut vm = Vm::new();
    assert_eq!(eval_number(&mut vm, "new Blob().size;"), 0.0);
}

#[test]
fn ctor_from_string_parts_concatenates() {
    let mut vm = Vm::new();
    // "hi" (2) + " " (1) + "world" (5) = 8 bytes.
    assert_eq!(
        eval_number(&mut vm, "new Blob(['hi', ' ', 'world']).size;"),
        8.0
    );
}

#[test]
fn ctor_options_type_lowercased() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(&mut vm, "new Blob(['x'], {type: 'Text/Plain'}).type;"),
        "text/plain"
    );
}

#[test]
fn ctor_missing_type_defaults_to_empty_string() {
    let mut vm = Vm::new();
    assert_eq!(eval_string(&mut vm, "new Blob(['x']).type;"), "");
}

#[test]
fn blob_slice_returns_new_blob() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(&mut vm, "new Blob(['hello world']).slice(0, 5).size;"),
        5.0
    );
}

#[test]
fn blob_slice_content_type_override() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_string(
            &mut vm,
            "new Blob(['hi'], {type: 'text/plain'}).slice(0, 1, 'Text/Html').type;"
        ),
        "text/html"
    );
}

#[test]
fn blob_text_async_decodes_utf8() {
    assert_eq!(
        eval_global_string(
            "globalThis.r = ''; \
             new Blob(['hello']).text().then(v => { globalThis.r = v; });",
            "r",
        ),
        "hello"
    );
}

#[test]
fn blob_array_buffer_round_trip() {
    assert_eq!(
        eval_global_number(
            "globalThis.size = 0; \
             new Blob(['abcde']).arrayBuffer().then(buf => { globalThis.size = buf.byteLength; });",
            "size",
        ),
        5.0
    );
}

#[test]
fn blob_ctor_requires_new_operator() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var threw = false; \
         try { Blob(['x']); } \
         catch (e) { threw = e instanceof TypeError; } threw;"
    ));
}
