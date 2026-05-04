//! Body integration: `Response.body`, body-input rejection on
//! `new Request`, `Blob.prototype.stream()`, and stream-level
//! invariants for the body adapter (post-close enqueue, double
//! close, reader.closed resolve/reject).

use crate::vm::value::JsValue;
use crate::vm::Vm;

use super::{eval_global_bool, eval_global_string};

// ---------------------------------------------------------------------------
// Body integration: Response.body
// ---------------------------------------------------------------------------

#[test]
fn response_body_returns_readable_stream() {
    let source = r#"
        const r = new Response("hello");
        globalThis.result = r.body instanceof ReadableStream;
    "#;
    assert!(eval_global_bool(source, "result"));
}

#[test]
fn response_body_identity_preserved_across_reads() {
    let source = r#"
        const r = new Response("hello");
        globalThis.result = r.body === r.body;
    "#;
    assert!(eval_global_bool(source, "result"));
}

#[test]
fn response_body_chunks_uint8array() {
    let source = r#"
        const r = new Response("hi");
        const reader = r.body.getReader();
        reader.read().then(v => {
            globalThis.result = v.value instanceof Uint8Array && v.value.length === 2;
        });
    "#;
    assert!(eval_global_bool(source, "result"));
}

#[test]
fn body_mixin_after_body_access_throws_disturbed() {
    let mut vm = Vm::new();
    let result = vm.eval(
        r#"
        const r = new Response("hi");
        r.body;
        r.text().then(_ => {}, _ => { globalThis.result = true; });
        "#,
    );
    let _ = result;
    let v = vm.get_global("result");
    assert!(matches!(v, Some(JsValue::Boolean(true))));
}

// ---------------------------------------------------------------------------
// Body input rejection
// ---------------------------------------------------------------------------

#[test]
fn new_request_with_readable_stream_body_throws() {
    let mut vm = Vm::new();
    let result = vm.eval(
        r#"
        const s = new ReadableStream();
        new Request("https://example.com/", {method: "POST", body: s});
        "#,
    );
    assert!(
        result.is_err(),
        "expected TypeError on ReadableStream body input"
    );
}

// ---------------------------------------------------------------------------
// Blob.prototype.stream()
// ---------------------------------------------------------------------------

#[test]
fn blob_stream_returns_readable_stream() {
    let source = r#"
        const b = new Blob(["chunk"]);
        globalThis.result = b.stream() instanceof ReadableStream;
    "#;
    assert!(eval_global_bool(source, "result"));
}

#[test]
fn blob_stream_emits_uint8array_chunk() {
    let source = r#"
        const b = new Blob(["abc"]);
        const reader = b.stream().getReader();
        reader.read().then(v => {
            globalThis.result = v.value instanceof Uint8Array && v.value.length === 3;
        });
    "#;
    assert!(eval_global_bool(source, "result"));
}

#[test]
fn blob_stream_independent_per_call() {
    // Blob is immutable; stream() returns a fresh stream each
    // call.  Two streams should not collide on locked-state.
    let source = r#"
        const b = new Blob(["x"]);
        const s1 = b.stream();
        const s2 = b.stream();
        globalThis.result = s1 !== s2;
    "#;
    assert!(eval_global_bool(source, "result"));
}

// ---------------------------------------------------------------------------
// Stream-level invariants
// ---------------------------------------------------------------------------

#[test]
fn enqueue_after_close_throws() {
    let mut vm = Vm::new();
    let result = vm.eval(
        r#"
        new ReadableStream({
            start(c) {
                c.close();
                c.enqueue("late");
            }
        });
        "#,
    );
    assert!(result.is_err());
}

#[test]
fn double_close_throws() {
    let mut vm = Vm::new();
    let result = vm.eval(
        r"
        new ReadableStream({
            start(c) {
                c.close();
                c.close();
            }
        });
        ",
    );
    assert!(result.is_err());
}

#[test]
fn release_lock_then_get_reader_again_works() {
    let source = r"
        const s = new ReadableStream();
        s.getReader().releaseLock();
        const r2 = s.getReader();
        globalThis.result = r2 instanceof ReadableStreamDefaultReader;
    ";
    assert!(eval_global_bool(source, "result"));
}

#[test]
fn reader_closed_resolves_on_close() {
    let source = r#"
        let ctrl;
        const s = new ReadableStream({ start(c) { ctrl = c; } });
        const r = s.getReader();
        r.closed.then(_ => { globalThis.result = "ok"; });
        ctrl.close();
    "#;
    assert_eq!(eval_global_string(source, "result"), "ok");
}

#[test]
fn reader_closed_rejects_on_error() {
    let source = r#"
        let ctrl;
        const s = new ReadableStream({ start(c) { ctrl = c; } });
        const r = s.getReader();
        r.closed.then(_ => {}, e => { globalThis.result = e; });
        ctrl.error("boom");
    "#;
    assert_eq!(eval_global_string(source, "result"), "boom");
}
