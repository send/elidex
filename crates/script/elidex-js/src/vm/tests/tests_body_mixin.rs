//! Body-mixin tests (WHATWG Fetch §5 Body) — `text()` / `json()` /
//! `arrayBuffer()` / `blob()` shared between `Request` and
//! `Response`.
//!
//! Covers body round-trips for string / ArrayBuffer / Blob init,
//! `bodyUsed` transitions, double-read rejection, and Content-Type
//! propagation through `.blob()`.

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
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

fn eval_global_bool(source: &str, name: &str) -> bool {
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    match vm.get_global(name) {
        Some(JsValue::Boolean(b)) => b,
        other => panic!("expected global {name} to be a bool, got {other:?}"),
    }
}

#[test]
fn response_text_round_trip() {
    assert_eq!(
        eval_global_string(
            "globalThis.r = ''; \
             new Response('hello').text().then(v => { globalThis.r = v; });",
            "r",
        ),
        "hello"
    );
}

#[test]
fn request_text_round_trip() {
    assert_eq!(
        eval_global_string(
            "globalThis.r = ''; \
             new Request('http://x/', {method: 'POST', body: 'hi'}).text().then(v => { globalThis.r = v; });",
            "r",
        ),
        "hi"
    );
}

#[test]
fn response_json_round_trip() {
    // `Response.json({...})` serialises to a body, then `.json()`
    // parses it back.
    assert_eq!(
        eval_global_number(
            "globalThis.r = 0; \
             Response.json({a: 1, b: 'x'}).json().then(o => { globalThis.r = o.a; });",
            "r",
        ),
        1.0
    );
}

#[test]
fn response_array_buffer_round_trip() {
    assert_eq!(
        eval_global_number(
            "globalThis.r = 0; \
             new Response('abcde').arrayBuffer().then(b => { globalThis.r = b.byteLength; });",
            "r",
        ),
        5.0
    );
}

#[test]
fn response_blob_inherits_content_type() {
    // `new Response(str)` installs `content-type: text/plain;
    // charset=UTF-8` by default; `.blob()` carries that through.
    assert_eq!(
        eval_global_string(
            "globalThis.r = ''; \
             new Response('hi').blob().then(b => { globalThis.r = b.type; });",
            "r",
        ),
        "text/plain;charset=UTF-8"
    );
}

#[test]
fn body_used_flips_after_text() {
    let mut vm = Vm::new();
    // Synchronous settle means `.bodyUsed` is observable
    // immediately after the promise is constructed.
    assert!(eval_bool(
        &mut vm,
        "var r = new Response('x'); r.text(); r.bodyUsed;"
    ));
}

#[test]
fn double_read_rejects_with_type_error() {
    // Second `.text()` call rejects the returned Promise with
    // TypeError, not throws synchronously.  Observable via
    // `.catch()`.
    assert!(eval_global_bool(
        "globalThis.r = false; \
         var resp = new Response('x'); \
         resp.text(); \
         resp.text().catch(e => { globalThis.r = e instanceof TypeError; });",
        "r",
    ));
}

#[test]
fn response_array_buffer_body_round_trip() {
    // `new Response(arrayBuffer).arrayBuffer()` preserves byte length.
    assert_eq!(
        eval_global_number(
            "globalThis.r = 0; \
             new Response(new ArrayBuffer(7)).arrayBuffer().then(b => { globalThis.r = b.byteLength; });",
            "r",
        ),
        7.0
    );
}

#[test]
fn response_blob_body_text_round_trip() {
    // `new Response(new Blob(["hi"])).text()` decodes the Blob body.
    assert_eq!(
        eval_global_string(
            "globalThis.r = ''; \
             new Response(new Blob(['hi'])).text().then(v => { globalThis.r = v; });",
            "r",
        ),
        "hi"
    );
}
