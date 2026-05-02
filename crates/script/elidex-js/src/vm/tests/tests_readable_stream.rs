//! `ReadableStream` + DefaultReader + DefaultController tests
//! (WHATWG Streams §4, Phase-2 read-output-only).

#![cfg(feature = "engine")]

use super::super::value::JsValue;
use super::super::Vm;

fn eval_bool(vm: &mut Vm, source: &str) -> bool {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(b) => b,
        other => panic!("expected bool, got {other:?}"),
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

fn eval_global_bool(source: &str, name: &str) -> bool {
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    match vm.get_global(name) {
        Some(JsValue::Boolean(b)) => b,
        other => panic!("expected global {name} to be a bool, got {other:?}"),
    }
}

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

// ---------------------------------------------------------------------------
// Queuing strategies (§6.1 / §6.2)
// ---------------------------------------------------------------------------

#[test]
fn count_queuing_strategy_size_returns_one() {
    let mut vm = Vm::new();
    let v = eval_number(
        &mut vm,
        "new CountQueuingStrategy({highWaterMark: 5}).size()",
    );
    assert_eq!(v, 1.0);
}

#[test]
fn count_queuing_strategy_high_water_mark_own_property() {
    let mut vm = Vm::new();
    let v = eval_number(
        &mut vm,
        "new CountQueuingStrategy({highWaterMark: 7}).highWaterMark",
    );
    assert_eq!(v, 7.0);
}

#[test]
fn byte_length_queuing_strategy_size_reads_byte_length() {
    let mut vm = Vm::new();
    let v = eval_number(
        &mut vm,
        "new ByteLengthQueuingStrategy({highWaterMark: 1024}).size(new Uint8Array(42))",
    );
    assert_eq!(v, 42.0);
}

#[test]
fn high_water_mark_negative_throws() {
    let mut vm = Vm::new();
    let result = vm.eval("new ReadableStream(undefined, {highWaterMark: -1})");
    assert!(result.is_err());
}

#[test]
fn high_water_mark_nan_throws() {
    let mut vm = Vm::new();
    let result = vm.eval("new ReadableStream(undefined, {highWaterMark: NaN})");
    assert!(result.is_err());
}

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
        r#"
        new ReadableStream({
            start(c) {
                c.close();
                c.close();
            }
        });
        "#,
    );
    assert!(result.is_err());
}

#[test]
fn release_lock_then_get_reader_again_works() {
    let source = r#"
        const s = new ReadableStream();
        s.getReader().releaseLock();
        const r2 = s.getReader();
        globalThis.result = r2 instanceof ReadableStreamDefaultReader;
    "#;
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

// ---------------------------------------------------------------------------
// R1 regression: spec-edge bugs caught in Copilot review round 1
// ---------------------------------------------------------------------------

/// R1.1 regression: `controller.close()` issued while the queue
/// still carries chunks must finalise the stream once the queue
/// drains via reads.  Without the post-drain `finalize_close`
/// hook in `deliver_pending_reads`, `reader.closed` stayed
/// pending forever.  We exercise this by enqueuing+closing
/// inside `start` (queue carries chunks at close time) and then
/// reading them out — `r.closed` must resolve synchronously
/// during the eval microtask drain.
#[test]
fn close_with_pending_chunks_finalises_after_drain() {
    let source = r#"
        const s = new ReadableStream({
            start(c) { c.enqueue("a"); c.enqueue("b"); c.close(); }
        });
        const r = s.getReader();
        // Queue a closed-resolves marker first; reads come after.
        // The read+close finalisation only happens once
        // deliver_pending_reads drains both queued chunks AND the
        // close_requested gate fires from inside the same drain.
        r.closed.then(() => { globalThis.closed_resolved = true; });
        r.read();  // delivers "a"
        r.read();  // delivers "b" → queue empties → close finalises
    "#;
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    let v = vm.get_global("closed_resolved");
    assert!(
        matches!(v, Some(JsValue::Boolean(true))),
        "expected reader.closed to resolve once queue drained, got {v:?}"
    );
}

/// R1.2 regression: with `ByteLengthQueuingStrategy` the queue's
/// stored size is `chunk.byteLength`, not 1.  After a read, the
/// stream's `desiredSize` must reflect the **actual** size
/// reclaimed — not a hard-coded `1.0` decrement.
#[test]
fn dequeue_decrements_by_recorded_chunk_size() {
    let source = r#"
        let ctrl;
        const s = new ReadableStream(
            { start(c) { ctrl = c; c.enqueue(new Uint8Array(10)); } },
            new ByteLengthQueuingStrategy({highWaterMark: 100})
        );
        // Before read: hwm=100, queue_total=10 → desired=90.
        // After read: queue_total back to 0 → desired=100.  A
        // hard-coded -1.0 decrement would produce 91, not 100.
        const r = s.getReader();
        r.read().then(_ => {
            globalThis.result = ctrl.desiredSize;
        });
    "#;
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    let v = vm.get_global("result");
    match v {
        Some(JsValue::Number(n)) => assert_eq!(n, 100.0),
        other => panic!("expected number 100, got {other:?}"),
    }
}

/// R1.4 regression: `getReader({mode})` accepted any non-empty
/// non-"byob" string + rejected empty string.  Spec: `undefined`
/// → default reader; `"byob"` → BYOB (Phase 2 unsupported);
/// every other value → TypeError.
#[test]
fn get_reader_accepts_undefined_mode() {
    let source = r#"
        const s = new ReadableStream();
        const r = s.getReader({});
        globalThis.result = r instanceof ReadableStreamDefaultReader;
    "#;
    assert!(eval_global_bool(source, "result"));
}

#[test]
fn get_reader_rejects_unknown_mode() {
    let mut vm = Vm::new();
    let result = vm.eval(r#"new ReadableStream().getReader({mode: "default"})"#);
    assert!(result.is_err(), "expected TypeError on mode=\"default\"");
}

#[test]
fn get_reader_rejects_byob_mode() {
    let mut vm = Vm::new();
    let result = vm.eval(r#"new ReadableStream().getReader({mode: "byob"})"#);
    assert!(result.is_err(), "expected TypeError on mode=\"byob\"");
}

/// R1.5 regression: `new ReadableStreamDefaultReader(stream)`
/// must promote the pre-allocated `this` (so identity is
/// preserved across the ctor) instead of allocating a fresh
/// `Object` and returning it.  Verified by checking that the
/// reader instance is brand-correct + locks the source stream.
#[test]
fn reader_constructor_locks_stream_and_brand_checks() {
    let source = r#"
        const s = new ReadableStream();
        const r = new ReadableStreamDefaultReader(s);
        globalThis.result =
            r instanceof ReadableStreamDefaultReader && s.locked === true;
    "#;
    assert!(eval_global_bool(source, "result"));
}

/// Companion: cannot construct a reader for an already-locked
/// stream — checks the second `acquire_default_reader`-or-ctor
/// path explicitly.
#[test]
fn reader_constructor_rejects_locked_stream() {
    let mut vm = Vm::new();
    let result = vm.eval(
        r#"
        const s = new ReadableStream();
        s.getReader();
        new ReadableStreamDefaultReader(s);
        "#,
    );
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// R2 regression: spec-edge bugs caught in Copilot review round 2
// ---------------------------------------------------------------------------

/// R2.1: `null` is treated as `undefined` per WebIDL dict-from-null
/// for both underlyingSource and queuingStrategy positions.
#[test]
fn ctor_accepts_null_underlying_source_and_strategy() {
    let mut vm = Vm::new();
    let v = eval_bool(
        &mut vm,
        "new ReadableStream(null, null) instanceof ReadableStream",
    );
    assert!(v);
}

/// R2.2: `stream.cancel()` on a locked stream returns a rejected
/// Promise per spec §4.2.6 step 1 (IsReadableStreamLocked check).
#[test]
fn stream_cancel_on_locked_rejects_with_typeerror() {
    let source = r#"
        const s = new ReadableStream();
        s.getReader();
        s.cancel("nope").then(_ => { globalThis.result = "ok"; },
                              _ => { globalThis.result = "rejected"; });
    "#;
    assert_eq!(eval_global_string(source, "result"), "rejected");
}

/// R2.3: `.body` identity is preserved across body-mixin
/// consumption.  Reading `.text()` first then `.body` must NOT
/// flip the slot back to `null` — spec models `.body` as the
/// (now disturbed) stream regardless of observation order.
#[test]
fn body_returns_disturbed_stream_after_text_consumes_first() {
    let source = r#"
        const r = new Response("hi");
        r.text();
        // After mixin consumed body_data, .body must still
        // present a (now closed) ReadableStream — not null.
        globalThis.result = r.body instanceof ReadableStream;
    "#;
    assert!(eval_global_bool(source, "result"));
}

/// Companion: receiver that never had a body still returns null
/// from `.body` so the no-body case isn't masked.
#[test]
fn body_returns_null_for_no_body_response() {
    let source = r#"
        const r = new Response();
        globalThis.result = r.body === null;
    "#;
    assert!(eval_global_bool(source, "result"));
}

// ---------------------------------------------------------------------------
// R4 regression: spec-edge bugs caught in Copilot review round 4
// ---------------------------------------------------------------------------

/// R4.1: a size() that returns a negative number must error the
/// stream with RangeError per spec §4.5.4 step 4 (negatives would
/// invert `desiredSize` arithmetic).
#[test]
fn negative_size_algorithm_errors_stream() {
    let source = r#"
        let ctrl;
        const s = new ReadableStream(
            { start(c) { ctrl = c; } },
            { highWaterMark: 1, size: () => -1 }
        );
        const r = s.getReader();
        r.read().then(_ => { globalThis.result = "ok"; },
                      _ => { globalThis.result = "rejected"; });
        try { ctrl.enqueue("x"); } catch (_) {}
    "#;
    assert_eq!(eval_global_string(source, "result"), "rejected");
}

/// R4.3: empty fetch responses (or any non-opaque body-carrying
/// response with zero bytes) must expose `.body` as a closed
/// ReadableStream per spec §4.1, not `null`.  Verified
/// indirectly via `new Response("")` since fetch tests need a
/// broker.
#[test]
fn body_for_empty_response_is_a_readable_stream() {
    let source = r#"
        const r = new Response("");
        globalThis.result = r.body instanceof ReadableStream;
    "#;
    assert!(eval_global_bool(source, "result"));
}

// ---------------------------------------------------------------------------
// R5 regression: spec-edge bugs caught in Copilot review round 5
// ---------------------------------------------------------------------------

/// R5.1: `start(controller)` is invoked with `this` bound to the
/// underlyingSource object per spec InvokeOrNoop semantics, so
/// `start() { this.x = 1 }` shapes work.
#[test]
fn start_callback_this_is_underlying_source() {
    let source = r#"
        const src = { tag: "marker" };
        let observedTag;
        src.start = function() { observedTag = this.tag; };
        new ReadableStream(src);
        globalThis.result = observedTag;
    "#;
    assert_eq!(eval_global_string(source, "result"), "marker");
}

/// R5.2: `pull(controller)` is invoked with `this` bound to the
/// underlyingSource — `pull() { this.enqueue(1) }` style works.
#[test]
fn pull_callback_this_is_underlying_source() {
    let source = r#"
        const src = {
            tag: "pull-marker",
            pull(c) { c.enqueue(this.tag); }
        };
        const s = new ReadableStream(src);
        s.getReader().read().then(v => { globalThis.result = v.value; });
    "#;
    assert_eq!(eval_global_string(source, "result"), "pull-marker");
}

/// R5.3: `cancel(reason)` is invoked with `this` bound to the
/// underlyingSource.
#[test]
fn cancel_callback_this_is_underlying_source() {
    let source = r#"
        const src = { tag: "cancel-marker" };
        let observedTag;
        src.cancel = function() { observedTag = this.tag; };
        const s = new ReadableStream(src);
        s.cancel("nope");
        globalThis.result = observedTag;
    "#;
    assert_eq!(eval_global_string(source, "result"), "cancel-marker");
}

/// R5.4: getReader rejects non-object non-null non-undefined
/// options per WebIDL dict-conversion semantics.
#[test]
fn get_reader_rejects_number_options() {
    let mut vm = Vm::new();
    let result = vm.eval("new ReadableStream().getReader(1)");
    assert!(result.is_err());
}

/// R5.7: oversize-body case must produce an *errored* stream
/// whose `read()` rejects, not a closed stream whose `read()`
/// returns `{done:true}`.  Phase 2 cannot easily construct a
/// >4GiB body in a unit test — verified instead by inspecting
/// the order-of-operations contract via doc + manual review;
/// the smaller invariant we *can* test is that `error()` after
/// `close()` is a no-op (so the fix path matters).
#[test]
fn error_after_close_is_no_op() {
    // Sanity: this confirms that `error_stream`'s "early-return
    // unless Readable" guard is in place — the underlying
    // contract that R5.7's reorder relies on.
    let source = r#"
        let ctrl;
        const s = new ReadableStream({ start(c) { ctrl = c; c.close(); } });
        try { ctrl.error("late"); } catch (_) {}
        const r = s.getReader();
        r.read().then(v => { globalThis.result = v.done; },
                      _ => { globalThis.result = false; });
    "#;
    // Closed wins — read resolves done=true rather than rejecting.
    assert!(eval_global_bool(source, "result"));
}

// ---------------------------------------------------------------------------
// R7 regression: spec / GC bugs caught in Copilot review round 7
// ---------------------------------------------------------------------------

/// R7.1: pull-completion only re-fires `pull_if_needed` when
/// `pull_again` was set during the in-flight pull.  Without that
/// gate, a `pull()` returning `undefined` would loop forever
/// (desiredSize stays positive, every pull-step microtask
/// schedules another pull).  Verified: a simple non-enqueueing
/// `pull()` quiesces after one fire — read stays pending.
#[test]
fn pull_returning_undefined_does_not_infinite_loop() {
    // Tracks pull invocation count.  Without R7.1 fix this would
    // grow unboundedly during the single eval microtask drain.
    let source = r#"
        let pullCount = 0;
        const s = new ReadableStream({
            pull() { pullCount++; globalThis.result = pullCount; }
        });
        s.getReader().read();
        // An infinite loop in pull would prevent eval from
        // returning at all (test would hang / panic in test
        // harness).  R7.1 bug: each pull-step microtask would
        // schedule another pull.  After the fix, pull fires once
        // and `pullCount` settles at 1.
    "#;
    let mut vm = Vm::new();
    vm.eval(source).unwrap();
    let v = vm.get_global("result");
    let count = match v {
        Some(JsValue::Number(n)) => n,
        other => panic!("expected number, got {other:?}"),
    };
    assert!(
        count >= 1.0 && count <= 2.0,
        "pull should fire ~1× then quiesce, got {count}"
    );
}

// ---------------------------------------------------------------------------
// R8 regression: null-body status check
// ---------------------------------------------------------------------------

/// R8.2 (proxy): `new Response(null, {status: 204})` produces
/// a Response whose `.body` must be `null` per WHATWG Fetch
/// §4.1 null-body-status rule.  The same rule applies to fetch
/// responses with status 204/205/304 — Copilot R8 caught that
/// my R4.3 unconditional `body_data.insert` for non-opaque
/// fetched responses violated this; the fetch path is now
/// gated on `null_body_status`.  This test exercises the
/// Response-ctor side which shares the `.body === null` invariant.
#[test]
fn null_body_status_response_has_null_body() {
    let source = r#"
        const r = new Response(null, { status: 204 });
        globalThis.result = r.body === null;
    "#;
    assert!(eval_global_bool(source, "result"));
}

// ---------------------------------------------------------------------------
// R9 regression: spec edges caught in Copilot review round 9
// ---------------------------------------------------------------------------

/// R9.1: a null-body receiver stays `.body === null` even after
/// the body mixin sets `disturbed`.  Previously my R8.2
/// `disturbed`-gated path materialised an empty stream there,
/// flipping null to non-null.
#[test]
fn null_body_receiver_stays_null_body_after_mixin_consumed() {
    let source = r#"
        const r = new Response();   // no body, .body === null
        r.text();
        globalThis.result = r.body === null;
    "#;
    assert!(eval_global_bool(source, "result"));
}

/// R9.1 companion: receivers WITH a body (even empty string body)
/// must keep `.body` non-null after consumption — distinguish
/// "had a body" (presence in body_data, possibly empty) from
/// "no body, ever" (absent).
#[test]
fn empty_body_receiver_keeps_stream_after_mixin_consumed() {
    let source = r#"
        const r = new Response("");
        r.text();
        globalThis.result = r.body instanceof ReadableStream;
    "#;
    assert!(eval_global_bool(source, "result"));
}

/// R9.2: queuing strategy ctors accept undefined/null init per
/// WebIDL dict-from-null and surface the missing-highWaterMark
/// error, not "init must be an object".
#[test]
fn queuing_strategy_undefined_init_throws_highwatermark_required() {
    let mut vm = Vm::new();
    let result = vm.eval("new CountQueuingStrategy()");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("highWaterMark is required"),
        "expected highWaterMark-missing error, got: {err}"
    );
}

#[test]
fn queuing_strategy_null_init_throws_highwatermark_required() {
    let mut vm = Vm::new();
    let result = vm.eval("new ByteLengthQueuingStrategy(null)");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("highWaterMark is required"),
        "expected highWaterMark-missing error, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// R10 regression: spec edges caught in Copilot review round 10
// ---------------------------------------------------------------------------

/// R10.1: `releaseLock()` rejects the previously-issued
/// `closed` Promise (not just the new one), so `const p =
/// reader.closed; reader.releaseLock()` makes `p` reject —
/// previously `p` stayed pending forever.
#[test]
fn release_lock_rejects_previously_captured_closed_promise() {
    let source = r#"
        const s = new ReadableStream();
        const r = s.getReader();
        const p = r.closed;
        p.then(_ => {}, _ => { globalThis.result = "rejected"; });
        r.releaseLock();
    "#;
    assert_eq!(eval_global_string(source, "result"), "rejected");
}

/// R10.2: `strategy.highWaterMark` is a WebIDL readonly
/// attribute; assignments are silently ignored in non-strict
/// mode (and throw under "use strict").  Verified via "use
/// strict" — strict-mode write on readonly throws TypeError.
#[test]
fn strategy_high_water_mark_is_readonly() {
    let mut vm = Vm::new();
    let result = vm.eval(
        r#"
        "use strict";
        const s = new CountQueuingStrategy({highWaterMark: 5});
        s.highWaterMark = 99;
        "#,
    );
    assert!(
        result.is_err(),
        "expected strict-mode TypeError on readonly write"
    );
}

/// R10.3: primitive init value throws "init must be an
/// object" per WebIDL dict-from-non-object rule.
#[test]
fn strategy_primitive_init_throws_must_be_object() {
    let mut vm = Vm::new();
    let result = vm.eval("new CountQueuingStrategy(1)");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("init must be an object"),
        "expected 'init must be an object' error, got: {err}"
    );
}

#[test]
fn stream_tee_method_not_installed() {
    // Phase 2: `tee` is intentionally absent — `'tee' in stream`
    // returns `false` rather than throwing a stub error.
    // Re-installs land with M4-13.1 PR-streams-tee.
    let mut vm = Vm::new();
    let v = eval_bool(&mut vm, "'tee' in (new ReadableStream())");
    assert!(!v, "tee must not be installed in Phase 2");
}

/// PR-file-split-a Copilot R3 regression: `pull_should_fire`
/// must honour spec §4.5.13 step 4 — pull is required while a
/// locked reader has at least one pending read request, even
/// when `desiredSize` is 0.  With `highWaterMark: 0`, an
/// `enqueue`-on-`read` source pattern would otherwise leave
/// `pull` un-fired and `read()` permanently pending.
#[test]
fn pull_fires_for_pending_read_with_high_water_mark_zero() {
    let source = r#"
        let pullCount = 0;
        const s = new ReadableStream(
            {
                pull(c) {
                    pullCount++;
                    c.enqueue("chunk-" + pullCount);
                }
            },
            { highWaterMark: 0 }
        );
        const r = s.getReader();
        r.read().then(v => { globalThis.result = v.value; });
    "#;
    assert_eq!(
        eval_global_string(source, "result"),
        "chunk-1",
        "with highWaterMark=0, pull() must fire on read() to satisfy the pending request"
    );
}
