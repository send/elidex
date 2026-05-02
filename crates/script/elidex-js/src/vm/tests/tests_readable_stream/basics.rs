//! Constructor, brand-check, source callbacks, and the
//! `enqueue` / `read` / `close` / `cancel` core wiring.

use crate::vm::value::JsValue;
use crate::vm::Vm;

use super::{eval_bool, eval_global_bool, eval_global_string, eval_number};

// ---------------------------------------------------------------------------
// Constructor / brand-check
// ---------------------------------------------------------------------------

#[test]
fn readable_stream_constructor_yields_brand_check_passing_instance() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "new ReadableStream() instanceof ReadableStream",
    ));
}

#[test]
fn readable_stream_default_controller_illegal_constructor() {
    let mut vm = Vm::new();
    let result = vm.eval("new ReadableStreamDefaultController()");
    assert!(result.is_err(), "expected illegal-constructor TypeError");
}

#[test]
fn readable_stream_locked_initially_false() {
    let mut vm = Vm::new();
    assert!(!eval_bool(&mut vm, "new ReadableStream().locked"));
}

// ---------------------------------------------------------------------------
// start callback
// ---------------------------------------------------------------------------

#[test]
fn start_callback_receives_controller() {
    let source = r#"
        let isCtrl = false;
        new ReadableStream({
            start(c) {
                isCtrl = c instanceof ReadableStreamDefaultController;
            }
        });
        globalThis.result = isCtrl;
    "#;
    assert!(eval_global_bool(source, "result"));
}

#[test]
fn start_throw_propagates_to_constructor() {
    let mut vm = Vm::new();
    let result = vm.eval(r#"new ReadableStream({ start() { throw new TypeError("nope"); } })"#);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// enqueue / read pairing
// ---------------------------------------------------------------------------

#[test]
fn enqueue_then_read_resolves_with_value_done_false() {
    let source = r#"
        const s = new ReadableStream({
            start(c) { c.enqueue("hello"); c.close(); }
        });
        const r = s.getReader();
        r.read().then(v => { globalThis.result = v.value; });
    "#;
    assert_eq!(eval_global_string(source, "result"), "hello");
}

#[test]
fn read_after_close_returns_done_true() {
    let source = r#"
        const s = new ReadableStream({ start(c) { c.close(); } });
        const r = s.getReader();
        r.read().then(v => { globalThis.result = v.done; });
    "#;
    assert!(eval_global_bool(source, "result"));
}

#[test]
fn read_pending_resolves_on_later_enqueue() {
    let mut vm = Vm::new();
    vm.eval(
        r#"
        let ctrl;
        const s = new ReadableStream({ start(c) { ctrl = c; } });
        const r = s.getReader();
        r.read().then(v => { globalThis.result = v.value; });
        ctrl.enqueue(42);
        "#,
    )
    .unwrap();
    let result = vm.get_global("result").unwrap();
    match result {
        JsValue::Number(n) => assert_eq!(n, 42.0),
        other => panic!("expected number 42, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// locked / getReader / releaseLock
// ---------------------------------------------------------------------------

#[test]
fn get_reader_locks_stream() {
    let source = r#"
        const s = new ReadableStream();
        s.getReader();
        globalThis.result = s.locked;
    "#;
    assert!(eval_global_bool(source, "result"));
}

#[test]
fn get_reader_twice_throws() {
    let mut vm = Vm::new();
    let result = vm.eval(
        r#"
        const s = new ReadableStream();
        s.getReader();
        s.getReader();
        "#,
    );
    assert!(result.is_err());
}

#[test]
fn release_lock_unlocks_stream() {
    let source = r#"
        const s = new ReadableStream();
        const r = s.getReader();
        r.releaseLock();
        globalThis.result = s.locked;
    "#;
    assert!(!eval_global_bool(source, "result"));
}

// ---------------------------------------------------------------------------
// error path
// ---------------------------------------------------------------------------

#[test]
fn controller_error_rejects_pending_read() {
    let source = r#"
        let ctrl;
        const s = new ReadableStream({ start(c) { ctrl = c; } });
        const r = s.getReader();
        r.read().then(_ => { globalThis.result = false; },
                      _ => { globalThis.result = true; });
        ctrl.error("boom");
    "#;
    assert!(eval_global_bool(source, "result"));
}

// ---------------------------------------------------------------------------
// desiredSize
// ---------------------------------------------------------------------------

#[test]
fn desired_size_starts_at_high_water_mark() {
    let mut vm = Vm::new();
    let v = eval_number(
        &mut vm,
        r#"
        let ds;
        new ReadableStream({ start(c) { ds = c.desiredSize; } });
        ds;
        "#,
    );
    assert_eq!(v, 1.0);
}

#[test]
fn desired_size_decreases_on_enqueue() {
    let mut vm = Vm::new();
    let v = eval_number(
        &mut vm,
        r#"
        let ds;
        new ReadableStream({
            start(c) { c.enqueue("x"); ds = c.desiredSize; }
        });
        ds;
        "#,
    );
    assert_eq!(v, 0.0);
}

// ---------------------------------------------------------------------------
// cancel
// ---------------------------------------------------------------------------

#[test]
fn stream_cancel_invokes_source_cancel_with_reason() {
    let source = r#"
        let observed;
        const s = new ReadableStream({ cancel(r) { observed = r; } });
        s.cancel("user-reason");
        globalThis.result = observed;
    "#;
    assert_eq!(eval_global_string(source, "result"), "user-reason");
}
