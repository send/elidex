//! `ArrayBuffer` detach state tests (ECMA-262 §25.1.3.4 + §25.1.3.5).
//!
//! `array_buffer_detach` is a Rust-side helper invoked by the
//! eventual D-16 `WebAssembly.Memory.grow` path; no JS-visible
//! `.detach()` method exists in v1 (the spec ES2024 `.transfer` API
//! that detaches the source buffer remains deferred — see
//! `vm/host/array_buffer.rs` module-doc Deferred list).  So Stage 1
//! tests drive the helper directly through `vm.inner.*`; the
//! JS-visible surfaces (`.byteLength` short-circuit, `.slice`
//! TypeError, `.detached` getter, TypedArray / DataView wire-ins,
//! BufferSource boundary checks) live in later-stage tests.
//!
//! Coverage in this file (incrementally appended through F3 stages):
//!
//! - Stage 1: storage shape + helper idempotency + GC prune +
//!   `Vm::unbind` persistence (deviation from plan §2.4 + DR-6 — see
//!   the test's inline NOTE for rationale).

#![cfg(feature = "engine")]

use super::super::host::array_buffer::{
    array_buffer_detach, create_array_buffer_from_bytes, is_detached_buffer,
};
use super::super::value::{JsValue, ObjectId, ObjectKind};
use super::super::Vm;

/// Allocate `new ArrayBuffer(len)` from JS, then resolve its
/// underlying `ObjectId` so the test can drive `array_buffer_detach`
/// directly on it.  The buffer is parked on `globalThis.buf` so JS-side
/// observations can keep referring to it after the Rust-side detach.
fn alloc_js_buffer_and_park(vm: &mut Vm, len: u32) -> ObjectId {
    let src = format!("globalThis.buf = new ArrayBuffer({len}); globalThis.buf;");
    match vm.eval(&src).expect("ArrayBuffer ctor") {
        JsValue::Object(id) => {
            assert!(
                matches!(vm.inner.get_object(id).kind, ObjectKind::ArrayBuffer),
                "expected ArrayBuffer ObjectKind"
            );
            id
        }
        other => panic!("expected ArrayBuffer object, got {other:?}"),
    }
}

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

fn eval_string(vm: &mut Vm, source: &str) -> String {
    match vm.eval(source).unwrap() {
        JsValue::String(sid) => vm.inner.strings.get_utf8(sid).clone(),
        other => panic!("expected string, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Stage 1 — storage + helpers + GC + unbind
// ---------------------------------------------------------------------------

#[test]
fn detach_inserts_into_set_and_drops_body_data() {
    let mut vm = Vm::new();
    let id = create_array_buffer_from_bytes(&mut vm.inner, vec![1, 2, 3, 4]);
    assert!(!is_detached_buffer(&vm.inner, id));
    assert!(vm.inner.body_data.contains_key(&id));

    array_buffer_detach(&mut vm.inner, id);
    assert!(is_detached_buffer(&vm.inner, id));
    // ECMA-262 §25.1.3.5 step 5: `[[ArrayBufferByteLength]]` is set
    // to 0.  In elidex this is realised by dropping the `body_data`
    // entry; subsequent `array_buffer_byte_length` then naturally
    // returns 0 via the missing-entry path.
    assert!(!vm.inner.body_data.contains_key(&id));
}

#[test]
fn re_detach_is_idempotent() {
    let mut vm = Vm::new();
    let id = create_array_buffer_from_bytes(&mut vm.inner, vec![0xAA; 8]);
    array_buffer_detach(&mut vm.inner, id);
    let before = vm.inner.detached_buffers.len();
    // Second detach: no spec-visible effect (the buffer is already
    // detached); `body_data.remove` second call is also a no-op.
    array_buffer_detach(&mut vm.inner, id);
    assert_eq!(vm.inner.detached_buffers.len(), before);
    assert!(is_detached_buffer(&vm.inner, id));
}

#[test]
fn detached_state_persists_across_unbind() {
    // NOTE: This test enforces a deliberate deviation from plan §2.4
    // + DR-6 (which proposed clearing `detached_buffers` on
    // `Vm::unbind`).  The plan's rationale cited a "mirrors
    // `disturbed` / `body_data` cross-DOM scrub pattern", but grep
    // verification (2026-05-29) confirms neither `body_data` nor
    // `disturbed` is actually cleared on `unbind` — both are
    // ObjectId-keyed (not Entity-keyed) and survive bind cycles.
    //
    // Clearing `detached_buffers` on `unbind` would let a JS-visible
    // ArrayBuffer that was detached pre-unbind silently transition
    // back to attached post-bind — divergent from ECMA-262 §25.1.3.5
    // (detach is permanent — "Set arrayBuffer.[[ArrayBufferData]] to
    // null" with no spec-prescribed re-attach path).  So
    // `detached_buffers` follows the `body_data` ObjectId-keyed
    // pattern: kept across unbind, pruned only by GC sweep.
    let mut vm = Vm::new();
    let id = create_array_buffer_from_bytes(&mut vm.inner, vec![0; 4]);
    array_buffer_detach(&mut vm.inner, id);
    vm.unbind();
    assert!(
        is_detached_buffer(&vm.inner, id),
        "detach is permanent per ECMA-262 §25.1.3.5; must survive Vm::unbind"
    );
}

#[test]
fn detached_entry_pruned_on_arraybuffer_gc() {
    // Hold no JS-side reference to the buffer so the next GC pass
    // can sweep it; the `detached_buffers` entry's ObjectId then
    // becomes invalid, and the sweep tail in `gc/collect.rs` is
    // required to prune the membership so a recycled ObjectId slot
    // can't inherit a stale detach flag.
    let mut vm = Vm::new();
    let id = create_array_buffer_from_bytes(&mut vm.inner, vec![1, 2, 3]);
    array_buffer_detach(&mut vm.inner, id);
    assert!(is_detached_buffer(&vm.inner, id));
    vm.inner.collect_garbage();
    assert!(
        !is_detached_buffer(&vm.inner, id),
        "GC sweep tail must prune `detached_buffers` entries whose key was collected"
    );
}

// ---------------------------------------------------------------------------
// Stage 2 — ArrayBuffer JS-visible surface
//   `.byteLength` zero short-circuit (§25.1.6.1 step 4)
//   `.slice` TypeError (§25.1.6.7 step 4)
//   `.detached` getter (§25.1.6.3)
// ---------------------------------------------------------------------------

#[test]
fn arraybuffer_byte_length_after_detach_is_zero() {
    let mut vm = Vm::new();
    let id = alloc_js_buffer_and_park(&mut vm, 16);
    assert_eq!(eval_number(&mut vm, "buf.byteLength;"), 16.0);
    array_buffer_detach(&mut vm.inner, id);
    assert_eq!(eval_number(&mut vm, "buf.byteLength;"), 0.0);
}

#[test]
fn arraybuffer_slice_after_detach_throws_typeerror() {
    let mut vm = Vm::new();
    let id = alloc_js_buffer_and_park(&mut vm, 8);
    // Pre-detach: `.slice(0, 4)` produces a fresh 4-byte buffer.
    assert_eq!(eval_number(&mut vm, "buf.slice(0, 4).byteLength;"), 4.0);
    array_buffer_detach(&mut vm.inner, id);
    let probe = "
        var caught = null;
        try { buf.slice(0, 4); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn arraybuffer_detached_getter_reports_state() {
    let mut vm = Vm::new();
    let id = alloc_js_buffer_and_park(&mut vm, 4);
    // Fresh buffer is not detached.
    assert!(!eval_bool(&mut vm, "buf.detached;"));
    array_buffer_detach(&mut vm.inner, id);
    assert!(eval_bool(&mut vm, "buf.detached;"));
}

// ---------------------------------------------------------------------------
// Stage 3 — TypedArray indexed access + 3 getters
//   §10.4.5.16 IsValidIntegerIndex / §10.4.5.17 TypedArrayGetElement
//   / §10.4.5.18 TypedArraySetElement
//   §23.2.3.3 byteLength / §23.2.3.4 byteOffset / §23.2.3.21 length
// ---------------------------------------------------------------------------

#[test]
fn typed_array_indexed_read_after_buffer_detach_returns_undefined() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 8);
    let _ = vm
        .eval("globalThis.u8 = new Uint8Array(buf); u8[0] = 0xAB; u8[0];")
        .unwrap();
    assert_eq!(eval_number(&mut vm, "u8[0];"), f64::from(0xAB_u8));
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var v = u8[0];
        v === undefined ? 'undefined' : String(v);
    ";
    assert_eq!(eval_string(&mut vm, probe), "undefined");
}

#[test]
fn typed_array_indexed_write_after_buffer_detach_is_silent_noop() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 4);
    let _ = vm.eval("globalThis.u8 = new Uint8Array(buf);").unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    // Per §10.4.5.18 the write must NOT throw — silent no-op success.
    let probe = "
        var threw = false;
        try { u8[0] = 99; } catch (e) { threw = true; }
        threw;
    ";
    assert!(!eval_bool(&mut vm, probe));
    // The corresponding read remains `undefined` (no write took
    // effect, and the buffer's `body_data` entry was dropped at
    // detach time anyway).
    let probe_read = "u8[0] === undefined;";
    assert!(eval_bool(&mut vm, probe_read));
}

#[test]
fn typed_array_indexed_write_after_buffer_detach_still_coerces_value() {
    // ECMA-262 §10.4.5.18 steps 1-2: ToNumber / ToBigInt runs
    // BEFORE step 3 IsValidIntegerIndex.  So a thrown `valueOf` on
    // the value side must surface even when the backing buffer is
    // already detached — F3 must NOT short-circuit before coercion.
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 4);
    let _ = vm.eval("globalThis.u8 = new Uint8Array(buf);").unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var threw = false;
        try { u8[0] = { valueOf: function () { throw new Error('coerced'); } }; }
        catch (e) { threw = (e && e.message === 'coerced'); }
        threw;
    ";
    assert!(eval_bool(&mut vm, probe));
}

#[test]
fn typed_array_byte_length_after_detach_is_zero() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 16);
    let _ = vm.eval("globalThis.u32 = new Uint32Array(buf);").unwrap();
    assert_eq!(eval_number(&mut vm, "u32.byteLength;"), 16.0);
    array_buffer_detach(&mut vm.inner, buf_id);
    assert_eq!(eval_number(&mut vm, "u32.byteLength;"), 0.0);
}

#[test]
fn typed_array_byte_offset_after_detach_is_zero() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 16);
    // A view at offset 4: `byteOffset` reads 4 pre-detach, 0 post.
    let _ = vm
        .eval("globalThis.u8 = new Uint8Array(buf, 4, 8);")
        .unwrap();
    assert_eq!(eval_number(&mut vm, "u8.byteOffset;"), 4.0);
    array_buffer_detach(&mut vm.inner, buf_id);
    assert_eq!(eval_number(&mut vm, "u8.byteOffset;"), 0.0);
}

#[test]
fn typed_array_length_after_detach_is_zero() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 16);
    let _ = vm.eval("globalThis.u32 = new Uint32Array(buf);").unwrap();
    assert_eq!(eval_number(&mut vm, "u32.length;"), 4.0);
    array_buffer_detach(&mut vm.inner, buf_id);
    assert_eq!(eval_number(&mut vm, "u32.length;"), 0.0);
}

// ---------------------------------------------------------------------------
// Stage 3.5 — TypedArray ctor (§23.2.5.1.3 step 6) +
//             ValidateTypedArray sweep (§23.2.4.4)
// ---------------------------------------------------------------------------

#[test]
fn typed_array_ctor_with_detached_array_buffer_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 8);
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { new Uint8Array(buf); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn typed_array_ctor_with_detached_buffer_and_offset_throws_typeerror_not_rangeerror() {
    // Without the spec-step-6 detach check before the
    // `byte_offset > buf_len` range check, a detached buffer
    // (`buf_len == 0`) + nonzero offset would surface RangeError
    // instead of the spec-mandated TypeError.
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 16);
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { new Uint8Array(buf, 4); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn typed_array_ctor_from_detached_typed_array_throws_typeerror() {
    // ECMA-262 §23.2.5.1.2 step 8 (`InitializeTypedArrayFromTypedArray`):
    // the source TypedArray's backing buffer must be attached at
    // construction time.  Without this check, `read_element_raw`'s
    // Stage 3 silent-undefined-on-detach path would copy `0`s into
    // the fresh destination instead of throwing TypeError.
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 8);
    let _ = vm.eval("globalThis.src = new Uint8Array(buf);").unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { new Uint16Array(src); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn typed_array_fill_on_detached_buffer_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 4);
    let _ = vm.eval("globalThis.u8 = new Uint8Array(buf);").unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { u8.fill(0xFF); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn typed_array_set_on_detached_buffer_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 4);
    let _ = vm.eval("globalThis.u8 = new Uint8Array(buf);").unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { u8.set([1, 2, 3]); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn typed_array_for_each_on_detached_buffer_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 4);
    let _ = vm.eval("globalThis.u8 = new Uint8Array(buf);").unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { u8.forEach(function () {}); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

// ---------------------------------------------------------------------------
// Stage 4 — DataView surface
//   GetViewValue §25.3.1.5 step 8 / SetViewValue §25.3.1.6 step 10
//   byteLength §25.3.4.2 / byteOffset §25.3.4.3
// ---------------------------------------------------------------------------

#[test]
fn dataview_get_int8_after_buffer_detach_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 4);
    let _ = vm.eval("globalThis.dv = new DataView(buf);").unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { dv.getInt8(0); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn dataview_get_int32_after_buffer_detach_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 8);
    let _ = vm.eval("globalThis.dv = new DataView(buf);").unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { dv.getInt32(0); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn dataview_get_float64_after_buffer_detach_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 8);
    let _ = vm.eval("globalThis.dv = new DataView(buf);").unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { dv.getFloat64(0); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn dataview_get_bigint64_after_buffer_detach_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 8);
    let _ = vm.eval("globalThis.dv = new DataView(buf);").unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { dv.getBigInt64(0); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn dataview_set_int8_after_buffer_detach_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 4);
    let _ = vm.eval("globalThis.dv = new DataView(buf);").unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { dv.setInt8(0, 0xAB); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn dataview_byte_length_after_buffer_detach_throws_typeerror() {
    // Contrast with `typed_array_byte_length_after_detach_is_zero` —
    // DataView's `byteLength` getter THROWS rather than returning 0
    // (different witness AO: §25.3.1.2 MakeDataViewWithBufferWitness
    // + §25.3.1.4 IsViewOutOfBounds vs. TypedArray's
    // §10.4.5.x TypedArrayLength path).
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 4);
    let _ = vm.eval("globalThis.dv = new DataView(buf);").unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { dv.byteLength; } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn dataview_byte_offset_after_buffer_detach_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 4);
    let _ = vm.eval("globalThis.dv = new DataView(buf);").unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { dv.byteOffset; } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn dataview_buffer_getter_after_detach_returns_the_buffer() {
    // ECMA-262 §25.3.4.1 `get DataView.prototype.buffer` has no
    // detach branch — JS code uses this to discover that the
    // backing buffer became detached, so throwing here would be
    // observably wrong.
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 4);
    let _ = vm.eval("globalThis.dv = new DataView(buf);").unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    assert!(eval_bool(&mut vm, "dv.buffer === buf;"));
    assert!(eval_bool(&mut vm, "dv.buffer.detached;"));
}

// ---------------------------------------------------------------------------
// Stage 4.5 — DataView ctor (§25.3.2.1 step 4)
// ---------------------------------------------------------------------------

#[test]
fn dataview_ctor_with_detached_array_buffer_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 8);
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { new DataView(buf); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn dataview_ctor_with_detached_buffer_and_offset_throws_typeerror_not_rangeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 16);
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { new DataView(buf, 4); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

// ---------------------------------------------------------------------------
// Stage 5 — D-16 consumer simulation
//   Exercises the F3 API surface as D-16's eventual
//   `WebAssembly.Memory.grow` caller will: alloc → detach → observe
//   every spec-prescribed surface, no actual wasm involvement.
// ---------------------------------------------------------------------------

#[test]
fn d16_consumer_simulation_full_surface_sweep_post_detach() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 16);
    // Build TypedArray + DataView views BEFORE detach so the JS
    // references survive — this mirrors the D-16 path where the
    // detach happens against an already-published `Memory.buffer`.
    let _ = vm
        .eval(
            "globalThis.u8 = new Uint8Array(buf); \
             globalThis.dv = new DataView(buf);",
        )
        .unwrap();

    array_buffer_detach(&mut vm.inner, buf_id);

    // (a) ArrayBuffer surface.
    assert_eq!(eval_number(&mut vm, "buf.byteLength;"), 0.0);
    assert!(eval_bool(&mut vm, "buf.detached;"));
    assert_eq!(
        eval_string(
            &mut vm,
            "var c=null; try { buf.slice(0); } catch (e) { c=e.name; } c;"
        ),
        "TypeError"
    );

    // (b) TypedArray surface — getter zeros + indexed undefined +
    // ValidateTypedArray sweep TypeError on prototype methods.
    assert_eq!(eval_number(&mut vm, "u8.length;"), 0.0);
    assert_eq!(eval_number(&mut vm, "u8.byteLength;"), 0.0);
    assert!(eval_bool(&mut vm, "u8[0] === undefined;"));
    assert_eq!(
        eval_string(
            &mut vm,
            "var c=null; try { u8.fill(1); } catch (e) { c=e.name; } c;"
        ),
        "TypeError"
    );

    // (c) DataView surface — getters + getInt32 throw TypeError.
    assert_eq!(
        eval_string(
            &mut vm,
            "var c=null; try { dv.byteLength; } catch (e) { c=e.name; } c;"
        ),
        "TypeError"
    );
    assert_eq!(
        eval_string(
            &mut vm,
            "var c=null; try { dv.getInt32(0); } catch (e) { c=e.name; } c;"
        ),
        "TypeError"
    );

    // (d) Ctor wire-in — fresh `new Uint8Array(buf)` /
    // `new DataView(buf)` over the already-detached buffer throw.
    assert_eq!(
        eval_string(
            &mut vm,
            "var c=null; try { new Uint8Array(buf); } catch (e) { c=e.name; } c;"
        ),
        "TypeError"
    );
    assert_eq!(
        eval_string(
            &mut vm,
            "var c=null; try { new DataView(buf); } catch (e) { c=e.name; } c;"
        ),
        "TypeError"
    );
}

// ---------------------------------------------------------------------------
// Stage 6 — BufferSource WebIDL boundary sweep
//   Cluster S (3 callers of `extract_buffer_source_bytes`):
//     TextDecoder.decode / SubtleCrypto.digest / WebSocket.send
//   Per-site wire-in (4 modules):
//     Blob ctor / File ctor (delegates to Blob) /
//     Request body / Response body / Crypto.getRandomValues /
//     ImageData ctor / putImageData
// ---------------------------------------------------------------------------

#[test]
fn text_decoder_decode_with_detached_buffer_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 4);
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { new TextDecoder().decode(buf); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn text_decoder_decode_with_detached_view_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 4);
    let _ = vm.eval("globalThis.u8 = new Uint8Array(buf);").unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { new TextDecoder().decode(u8); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

// NOTE: `SubtleCrypto.digest` and `WebSocket.send` are the other 2
// callers of `extract_buffer_source_bytes` (Cluster S), but they
// require either microtask-drain (digest returns a Promise that
// rejects asynchronously) or full bound-VM + mock NetworkHandle
// setup (WebSocket ctor demands HostData session).  The
// `text_decoder_decode_with_detached_*` tests above already
// exercise the shared helper's detach path — adding per-caller
// duplicates would be `three similar lines vs premature
// abstraction` redundancy without coverage gain (CLAUDE.md
// "don't add ... validation for scenarios that can't happen").  If
// the helper's detach path is broken, the TextDecoder tests fail
// loudly.

#[test]
fn blob_ctor_with_detached_buffer_part_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 4);
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { new Blob([buf]); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn blob_ctor_with_detached_view_part_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 4);
    let _ = vm.eval("globalThis.u8 = new Uint8Array(buf);").unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { new Blob([u8]); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn file_ctor_with_detached_buffer_part_throws_typeerror() {
    // File ctor delegates to the same `append_blob_part_bytes`
    // path that Blob ctor uses — regression coverage to confirm
    // the wire-in propagates via delegation.
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 4);
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { new File([buf], 'x'); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn response_with_detached_buffer_body_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 4);
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { new Response(buf); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn response_with_detached_view_body_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 4);
    let _ = vm.eval("globalThis.u8 = new Uint8Array(buf);").unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { new Response(u8); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn crypto_get_random_values_with_detached_view_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 4);
    let _ = vm.eval("globalThis.u8 = new Uint8Array(buf);").unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { crypto.getRandomValues(u8); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn image_data_ctor_with_detached_clamped_view_throws_typeerror() {
    let mut vm = Vm::new();
    let buf_id = alloc_js_buffer_and_park(&mut vm, 16);
    let _ = vm
        .eval("globalThis.clamped = new Uint8ClampedArray(buf);")
        .unwrap();
    array_buffer_detach(&mut vm.inner, buf_id);
    let probe = "
        var caught = null;
        try { new ImageData(clamped, 2); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}

#[test]
fn arraybuffer_detached_getter_is_brand_checked() {
    let mut vm = Vm::new();
    let _ = alloc_js_buffer_and_park(&mut vm, 1);
    // Calling `.detached` on a non-ArrayBuffer receiver must throw
    // per the standard `require_array_buffer_this` brand check.
    let probe = "
        var d = Object.getOwnPropertyDescriptor(ArrayBuffer.prototype, 'detached');
        var caught = null;
        try { d.get.call({}); } catch (e) { caught = e.name; }
        caught;
    ";
    assert_eq!(eval_string(&mut vm, probe), "TypeError");
}
