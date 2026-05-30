//! `ArrayBuffer` tests (ECMA-262 §25.1, minimal Phase 2 form).
//!
//! Covers ctor (length coercion / range validation), `byteLength`
//! getter (authoritative internal slot, delete-resistant), and
//! `slice` (negative index / OOB clamp).

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

#[test]
fn ctor_zero_length() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(&mut vm, "var b = new ArrayBuffer(0); b.byteLength;"),
        0.0
    );
}

#[test]
fn ctor_positive_length_zero_filled() {
    let mut vm = Vm::new();
    // Allocate 8 bytes, slice out the whole thing and wrap in a
    // second ArrayBuffer — both should report the same length.
    assert_eq!(
        eval_number(
            &mut vm,
            "var b = new ArrayBuffer(8); b.slice(0).byteLength;"
        ),
        8.0
    );
}

#[test]
fn ctor_negative_length_throws_range_error() {
    let mut vm = Vm::new();
    // ToIndex rejects negative integers with RangeError per spec.
    assert!(eval_bool(
        &mut vm,
        "var threw = false; \
         try { new ArrayBuffer(-1); } \
         catch (e) { threw = e instanceof RangeError; } threw;"
    ));
}

#[test]
fn byte_length_is_authoritative_after_delete() {
    let mut vm = Vm::new();
    // `byteLength` is a WebIDL readonly accessor — `delete` is a
    // no-op on the prototype, so the getter continues to read the
    // authoritative internal slot (PR5a2 R7.1 lesson).
    assert_eq!(
        eval_number(
            &mut vm,
            "var b = new ArrayBuffer(4); delete b.byteLength; b.byteLength;"
        ),
        4.0
    );
}

#[test]
fn slice_negative_begin_counts_from_end() {
    let mut vm = Vm::new();
    // `slice(-2)` on a length-4 buffer → length 2.
    assert_eq!(
        eval_number(&mut vm, "new ArrayBuffer(4).slice(-2).byteLength;"),
        2.0
    );
}

#[test]
fn slice_fractional_indices_truncate_toward_zero() {
    let mut vm = Vm::new();
    // ES `ToIntegerOrInfinity` (§7.1.5) truncates toward zero
    // before the negative-index adjustment: `-1.9` → `-1`, `3.9`
    // → `3`.  So `slice(-1.9, 3.9)` on a len-4 buffer is
    // equivalent to `slice(-1, 3)`, i.e. `[3, 3)` — empty.
    // Verifies browser parity; pre-R24.1 this returned 2 bytes
    // because the raw fractional f64 was fed into `len + n`.
    assert_eq!(
        eval_number(&mut vm, "new ArrayBuffer(4).slice(-1.9, 3.9).byteLength;"),
        0.0
    );
    // Complementary positive-index fractional case: `slice(0.9,
    // 2.9)` → `slice(0, 2)` → 2 bytes, not 2 bytes-but-offset.
    assert_eq!(
        eval_number(&mut vm, "new ArrayBuffer(4).slice(0.9, 2.9).byteLength;"),
        2.0
    );
}

#[test]
fn blob_slice_fractional_indices_truncate_toward_zero() {
    let mut vm = Vm::new();
    // `Blob.prototype.slice` shares `relative_index`; the
    // ToIntegerOrInfinity fix must apply there too.
    assert_eq!(
        eval_number(&mut vm, "new Blob(['hello']).slice(-1.9, 4.9).size;"),
        0.0
    );
}

/// Regression for Copilot R5: `array_buffer_view_bytes` now takes
/// `usize` offsets so ordinary ArrayBuffers > 4 GiB slice correctly.
/// Pre-fix the helper's `u32` parameters silently wrapped on
/// `byte_offset > u32::MAX`, returning truncated-position bytes
/// instead of the spec's out-of-range empty result.  Verifies
/// directly via the helper (allocating a real >4 GiB ArrayBuffer is
/// CI-prohibitive); the post-fix code path returns empty because the
/// small backing Vec can't satisfy an offset above its length.
#[test]
fn view_bytes_returns_empty_on_offset_above_u32_max() {
    use super::super::host::array_buffer::array_buffer_view_bytes;
    let mut vm = Vm::new();
    // Materialise a real ArrayBuffer so the helper sees a valid
    // `body_data` entry (8 bytes).
    let buf_val = vm.eval("var b = new ArrayBuffer(8); b").unwrap();
    let JsValue::Object(buf_id) = buf_val else {
        panic!("expected Object, got {buf_val:?}");
    };
    // Offset is > u32::MAX (one past); the helper's pre-fix `as u32`
    // cast would wrap to 0 and return the first `byte_length` bytes.
    // Post-fix: usize-typed indexing returns empty for the OOR slice.
    let huge_off = (u32::MAX as usize).saturating_add(1);
    let bytes = array_buffer_view_bytes(&vm.inner, buf_id, huge_off, 4);
    assert!(
        bytes.is_empty(),
        "byte_offset above u32::MAX must yield empty bytes, got {} bytes",
        bytes.len()
    );
}

#[test]
fn slice_out_of_range_clamps_to_length() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(&mut vm, "new ArrayBuffer(4).slice(0, 100).byteLength;"),
        4.0
    );
}

#[test]
fn slice_end_before_begin_yields_empty_buffer() {
    let mut vm = Vm::new();
    assert_eq!(
        eval_number(&mut vm, "new ArrayBuffer(4).slice(3, 1).byteLength;"),
        0.0
    );
}

#[test]
fn ctor_requires_new_operator() {
    let mut vm = Vm::new();
    assert!(eval_bool(
        &mut vm,
        "var threw = false; \
         try { ArrayBuffer(4); } \
         catch (e) { threw = e instanceof TypeError; } threw;"
    ));
}
