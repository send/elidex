//! Tests for the type-coercion module ([`super::super::coerce`]).
//!
//! Covers ES2020 abstract operations (`ToBoolean` / `ToNumber` /
//! `ToString` / `ToInt32`) and the equality / relational /
//! arithmetic operator helpers.
//!
//! Originally an inline `#[cfg(test)] mod tests { ... }` block at
//! the bottom of `vm/coerce.rs`; extracted into the standard
//! `vm/tests/` directory to keep `coerce.rs` below the 1000-line
//! convention (cleanup tranche 2).

use super::super::coerce::{
    abstract_eq, relative_index_f64, strict_eq, string_to_number, to_boolean, to_display_string,
    to_index_u64, to_int32, to_integer_or_infinity, to_number, to_string, typeof_str,
};
use super::super::coerce_ops::*;
use super::super::value::{JsValue, NativeContext, ObjectId, StringId};
use super::super::Vm;

#[test]
fn to_boolean_values() {
    let vm = Vm::new();
    let i = &vm.inner;
    assert!(!to_boolean(i, JsValue::Undefined));
    assert!(!to_boolean(i, JsValue::Null));
    assert!(!to_boolean(i, JsValue::Boolean(false)));
    assert!(to_boolean(i, JsValue::Boolean(true)));
    assert!(!to_boolean(i, JsValue::Number(0.0)));
    assert!(!to_boolean(i, JsValue::Number(f64::NAN)));
    assert!(to_boolean(i, JsValue::Number(1.0)));
    assert!(to_boolean(i, JsValue::Number(-1.0)));
    // Empty string → false
    let empty = i.well_known.empty;
    assert!(!to_boolean(i, JsValue::String(empty)));
    // Non-empty string → true
    let hello = i.well_known.undefined; // "undefined" is non-empty
    assert!(to_boolean(i, JsValue::String(hello)));
    // Object → always true
    assert!(to_boolean(i, JsValue::Object(ObjectId(0))));
}

#[test]
fn to_number_values() {
    let vm = Vm::new();
    let i = &vm.inner;
    assert!(to_number(i, JsValue::Undefined).unwrap().is_nan());
    assert_eq!(to_number(i, JsValue::Null).unwrap(), 0.0);
    assert_eq!(to_number(i, JsValue::Boolean(true)).unwrap(), 1.0);
    assert_eq!(to_number(i, JsValue::Boolean(false)).unwrap(), 0.0);
    assert_eq!(to_number(i, JsValue::Number(42.0)).unwrap(), 42.0);
}

#[test]
fn to_number_symbol_throws() {
    let mut vm = Vm::new();
    let sid = vm.inner.alloc_symbol(None);
    let result = to_number(&vm.inner, JsValue::Symbol(sid));
    assert!(result.is_err());
}

#[test]
fn string_to_number_cases() {
    assert_eq!(string_to_number(""), 0.0);
    assert_eq!(string_to_number("  "), 0.0);
    assert_eq!(string_to_number("42"), 42.0);
    assert_eq!(string_to_number("  3.125  "), 3.125);
    assert_eq!(string_to_number("0xff"), 255.0);
    assert_eq!(string_to_number("0b1010"), 10.0);
    assert_eq!(string_to_number("0o17"), 15.0);
    assert_eq!(string_to_number("Infinity"), f64::INFINITY);
    assert_eq!(string_to_number("-Infinity"), f64::NEG_INFINITY);
    assert!(string_to_number("abc").is_nan());
    assert!(string_to_number("12abc").is_nan());
}

#[test]
fn to_string_values() {
    let mut vm = Vm::new();
    let i = &mut vm.inner;

    let id = to_string(i, JsValue::Undefined).unwrap();
    assert_eq!(i.strings.get_utf8(id), "undefined");
    let id = to_string(i, JsValue::Null).unwrap();
    assert_eq!(i.strings.get_utf8(id), "null");
    let id = to_string(i, JsValue::Boolean(true)).unwrap();
    assert_eq!(i.strings.get_utf8(id), "true");
    let id = to_string(i, JsValue::Boolean(false)).unwrap();
    assert_eq!(i.strings.get_utf8(id), "false");
    let id = to_string(i, JsValue::Number(0.0)).unwrap();
    assert_eq!(i.strings.get_utf8(id), "0");
    let id = to_string(i, JsValue::Number(42.0)).unwrap();
    assert_eq!(i.strings.get_utf8(id), "42");
    let id = to_string(i, JsValue::Number(-1.5)).unwrap();
    assert_eq!(i.strings.get_utf8(id), "-1.5");
    let id = to_string(i, JsValue::Number(f64::NAN)).unwrap();
    assert_eq!(i.strings.get_utf8(id), "NaN");
    let id = to_string(i, JsValue::Number(f64::INFINITY)).unwrap();
    assert_eq!(i.strings.get_utf8(id), "Infinity");
}

#[test]
fn to_string_symbol_throws() {
    let mut vm = Vm::new();
    let sid = vm.inner.alloc_symbol(None);
    let result = to_string(&mut vm.inner, JsValue::Symbol(sid));
    assert!(result.is_err());
}

#[test]
fn to_display_string_symbol() {
    let mut vm = Vm::new();
    let desc = vm.inner.strings.intern("foo");
    let sid = vm.inner.alloc_symbol(Some(desc));
    let id = to_display_string(&mut vm.inner, JsValue::Symbol(sid));
    assert_eq!(vm.inner.strings.get_utf8(id), "Symbol(foo)");
}

#[test]
fn to_int32_cases() {
    let vm = Vm::new();
    let i = &vm.inner;
    assert_eq!(to_int32(i, JsValue::Number(0.0)).unwrap(), 0);
    assert_eq!(to_int32(i, JsValue::Number(1.7)).unwrap(), 1);
    assert_eq!(to_int32(i, JsValue::Number(-1.7)).unwrap(), -1);
    assert_eq!(to_int32(i, JsValue::Number(f64::NAN)).unwrap(), 0);
    assert_eq!(to_int32(i, JsValue::Number(f64::INFINITY)).unwrap(), 0);
}

#[test]
fn strict_eq_cases() {
    let vm = Vm::new();
    let i = &vm.inner;
    assert!(strict_eq(i, JsValue::Undefined, JsValue::Undefined));
    assert!(strict_eq(i, JsValue::Null, JsValue::Null));
    assert!(!strict_eq(i, JsValue::Undefined, JsValue::Null));
    assert!(strict_eq(i, JsValue::Number(1.0), JsValue::Number(1.0)));
    assert!(!strict_eq(
        i,
        JsValue::Number(f64::NAN),
        JsValue::Number(f64::NAN)
    ));
    assert!(strict_eq(i, JsValue::Number(0.0), JsValue::Number(-0.0)));
    assert!(strict_eq(
        i,
        JsValue::String(StringId(0)),
        JsValue::String(StringId(0))
    ));
    assert!(!strict_eq(
        i,
        JsValue::String(StringId(0)),
        JsValue::String(StringId(1))
    ));
}

#[test]
fn abstract_eq_null_undefined() {
    let mut vm = Vm::new();
    assert!(abstract_eq(&mut vm.inner, JsValue::Null, JsValue::Undefined).unwrap());
    assert!(abstract_eq(&mut vm.inner, JsValue::Undefined, JsValue::Null).unwrap());
    assert!(!abstract_eq(&mut vm.inner, JsValue::Null, JsValue::Boolean(false)).unwrap());
}

#[test]
fn abstract_eq_coercion() {
    let mut vm = Vm::new();
    let one_str = vm.inner.strings.intern("1");
    // "1" == 1
    assert!(abstract_eq(
        &mut vm.inner,
        JsValue::String(one_str),
        JsValue::Number(1.0)
    )
    .unwrap());
    // true == 1
    assert!(abstract_eq(&mut vm.inner, JsValue::Boolean(true), JsValue::Number(1.0)).unwrap());
    // false == 0
    assert!(abstract_eq(&mut vm.inner, JsValue::Boolean(false), JsValue::Number(0.0)).unwrap());
}

#[test]
fn typeof_values() {
    let vm = Vm::new();
    let i = &vm.inner;

    let id = typeof_str(i, JsValue::Undefined);
    assert_eq!(i.strings.get_utf8(id), "undefined");
    let id = typeof_str(i, JsValue::Null);
    assert_eq!(i.strings.get_utf8(id), "object");
    let id = typeof_str(i, JsValue::Boolean(true));
    assert_eq!(i.strings.get_utf8(id), "boolean");
    let id = typeof_str(i, JsValue::Number(0.0));
    assert_eq!(i.strings.get_utf8(id), "number");
    let s = i.well_known.empty;
    let id = typeof_str(i, JsValue::String(s));
    assert_eq!(i.strings.get_utf8(id), "string");
}

#[test]
fn add_string_concat() {
    let mut vm = Vm::new();
    let hello = vm.inner.strings.intern("hello");
    let world = vm.inner.strings.intern(" world");
    let result = vm
        .inner
        .op_add(JsValue::String(hello), JsValue::String(world))
        .unwrap();
    let JsValue::String(id) = result else {
        panic!("expected string");
    };
    assert_eq!(vm.inner.strings.get_utf8(id), "hello world");
}

#[test]
fn add_number_plus_string() {
    let mut vm = Vm::new();
    let s = vm.inner.strings.intern("px");
    let result = vm
        .inner
        .op_add(JsValue::Number(42.0), JsValue::String(s))
        .unwrap();
    let JsValue::String(id) = result else {
        panic!("expected string");
    };
    assert_eq!(vm.inner.strings.get_utf8(id), "42px");
}

#[test]
fn add_numbers() {
    let mut vm = Vm::new();
    let result = vm
        .inner
        .op_add(JsValue::Number(1.0), JsValue::Number(2.0))
        .unwrap();
    assert_eq!(result, JsValue::Number(3.0));
}

#[test]
fn relational_comparison() {
    let mut vm = Vm::new();
    assert_eq!(
        abstract_relational(
            &mut vm.inner,
            JsValue::Number(1.0),
            JsValue::Number(2.0),
            true,
        )
        .unwrap(),
        Some(true)
    );
    assert_eq!(
        abstract_relational(
            &mut vm.inner,
            JsValue::Number(2.0),
            JsValue::Number(1.0),
            true,
        )
        .unwrap(),
        Some(false)
    );
    // NaN comparison → None (undefined)
    assert_eq!(
        abstract_relational(
            &mut vm.inner,
            JsValue::Number(f64::NAN),
            JsValue::Number(1.0),
            true,
        )
        .unwrap(),
        None
    );
    // String comparison (lexicographic)
    let a = vm.inner.strings.intern("abc");
    let b = vm.inner.strings.intern("abd");
    assert_eq!(
        abstract_relational(&mut vm.inner, JsValue::String(a), JsValue::String(b), true,).unwrap(),
        Some(true)
    );
}

#[test]
fn bitwise_operations() {
    let mut vm = Vm::new();
    let i = &mut vm.inner;
    assert_eq!(
        op_bitwise(
            i,
            JsValue::Number(5.0),
            JsValue::Number(3.0),
            BitwiseOp::And
        )
        .unwrap(),
        JsValue::Number(1.0)
    );
    assert_eq!(
        op_bitwise(i, JsValue::Number(5.0), JsValue::Number(3.0), BitwiseOp::Or).unwrap(),
        JsValue::Number(7.0)
    );
    assert_eq!(
        op_bitwise(
            i,
            JsValue::Number(5.0),
            JsValue::Number(3.0),
            BitwiseOp::Xor
        )
        .unwrap(),
        JsValue::Number(6.0)
    );
    assert_eq!(
        op_bitwise(
            i,
            JsValue::Number(1.0),
            JsValue::Number(2.0),
            BitwiseOp::Shl
        )
        .unwrap(),
        JsValue::Number(4.0)
    );
    assert_eq!(
        op_bitwise(
            i,
            JsValue::Number(-8.0),
            JsValue::Number(2.0),
            BitwiseOp::Shr
        )
        .unwrap(),
        JsValue::Number(-2.0)
    );
}

#[test]
fn unary_operators() {
    let mut vm = Vm::new();
    let i = &mut vm.inner;
    assert_eq!(
        op_neg(i, JsValue::Number(5.0)).unwrap(),
        JsValue::Number(-5.0)
    );
    assert_eq!(
        op_pos(i, JsValue::Boolean(true)).unwrap(),
        JsValue::Number(1.0)
    );
    assert_eq!(op_not(i, JsValue::Boolean(true)), JsValue::Boolean(false));
    assert_eq!(op_not(i, JsValue::Number(0.0)), JsValue::Boolean(true));
    assert_eq!(
        op_bitnot(i, JsValue::Number(5.0)).unwrap(),
        JsValue::Number(-6.0)
    );
    assert_eq!(op_void(), JsValue::Undefined);
}

// ---------------------------------------------------------------------------
// `to_integer_or_infinity` (ES §7.1.5) and `relative_index_f64` (the §7.1.5
// + clamp pipeline used by Array / TypedArray / ArrayBuffer / Blob slice
// methods).  Pure-arithmetic helpers, no Vm setup needed.
// ---------------------------------------------------------------------------

#[test]
fn to_integer_or_infinity_nan_returns_zero() {
    assert_eq!(to_integer_or_infinity(f64::NAN), 0.0);
}

#[test]
fn to_integer_or_infinity_preserves_infinities() {
    assert_eq!(to_integer_or_infinity(f64::INFINITY), f64::INFINITY);
    assert_eq!(to_integer_or_infinity(f64::NEG_INFINITY), f64::NEG_INFINITY);
}

#[test]
fn to_integer_or_infinity_truncates_toward_zero() {
    assert_eq!(to_integer_or_infinity(3.9), 3.0);
    assert_eq!(to_integer_or_infinity(-3.9), -3.0);
}

/// Both signs of zero, plus `NaN`, compare equal to mathematical zero on
/// the way out.  The helper does not promise anything about IEEE 754 sign
/// bits — `f64::trunc` is sign-preserving on `-0`, while the `is_nan` arm
/// returns `+0` — and downstream consumers don't observe the difference,
/// so this test pins only the numeric equality.
#[test]
fn to_integer_or_infinity_zero_inputs_compare_equal_to_zero() {
    assert_eq!(to_integer_or_infinity(0.0), 0.0);
    assert_eq!(to_integer_or_infinity(-0.0), 0.0);
    assert_eq!(to_integer_or_infinity(f64::NAN), 0.0);
}

#[test]
fn relative_index_f64_nan_is_zero() {
    assert_eq!(relative_index_f64(f64::NAN, 10.0), 0.0);
}

/// Lock the spec equivalence `(−∞) + len = −∞`, then `max(0) = 0`, so
/// `relative_index_f64(−∞, len) = 0` for every finite non-negative `len`.
/// Sister of [`relative_index_f64_pos_infinity_is_len`] — together they
/// prove the clamp algebra subsumes any explicit `±Infinity` arm.
#[test]
fn relative_index_f64_neg_infinity_is_zero() {
    assert_eq!(relative_index_f64(f64::NEG_INFINITY, 10.0), 0.0);
}

/// Lock the spec equivalence `(+∞).min(len) = len`, so
/// `relative_index_f64(+∞, len) = len` for every finite non-negative `len`.
/// Sister of [`relative_index_f64_neg_infinity_is_zero`].
#[test]
fn relative_index_f64_pos_infinity_is_len() {
    assert_eq!(relative_index_f64(f64::INFINITY, 10.0), 10.0);
}

#[test]
fn relative_index_f64_negative_counts_from_end() {
    assert_eq!(relative_index_f64(-3.0, 10.0), 7.0);
    assert_eq!(relative_index_f64(-1.5, 10.0), 9.0); // trunc(-1.5) = -1
    assert_eq!(relative_index_f64(-100.0, 10.0), 0.0); // clamps at 0
}

#[test]
fn relative_index_f64_positive_clamps_at_len() {
    assert_eq!(relative_index_f64(0.0, 10.0), 0.0);
    assert_eq!(relative_index_f64(5.7, 10.0), 5.0);
    assert_eq!(relative_index_f64(10.0, 10.0), 10.0);
    assert_eq!(relative_index_f64(100.0, 10.0), 10.0); // clamps at len
}

#[test]
fn relative_index_f64_zero_length() {
    assert_eq!(relative_index_f64(5.0, 0.0), 0.0);
    assert_eq!(relative_index_f64(-5.0, 0.0), 0.0);
    assert_eq!(relative_index_f64(f64::INFINITY, 0.0), 0.0);
    assert_eq!(relative_index_f64(f64::NEG_INFINITY, 0.0), 0.0);
}

// ---------------------------------------------------------------------------
// `to_index_u64` (ES §7.1.22) — full-width `[0, 2^53)` ToIndex.  Drives both
// the `ArrayBuffer(length)` and `BigInt.asIntN/asUintN(bits)` callers, so
// the boundary semantics are locked here rather than re-asserted at each
// caller's integration tests.
// ---------------------------------------------------------------------------

fn try_to_index_u64(vm: &mut Vm, val: JsValue) -> Result<u64, super::super::value::VmError> {
    let mut ctx = NativeContext { vm: &mut vm.inner };
    to_index_u64(&mut ctx, val, "Test", "arg")
}

#[test]
fn to_index_u64_nan_returns_zero() {
    let mut vm = Vm::new();
    assert_eq!(
        try_to_index_u64(&mut vm, JsValue::Number(f64::NAN)).unwrap(),
        0
    );
}

#[test]
fn to_index_u64_truncates_fractional() {
    let mut vm = Vm::new();
    assert_eq!(try_to_index_u64(&mut vm, JsValue::Number(3.9)).unwrap(), 3);
    // Negative fractional truncates *toward zero* before the range
    // check fires, so `-0.7 → trunc(-0) → 0` is accepted.
    assert_eq!(try_to_index_u64(&mut vm, JsValue::Number(-0.7)).unwrap(), 0);
}

#[test]
fn to_index_u64_negative_rejects_with_safe_integer_message() {
    let mut vm = Vm::new();
    let err = try_to_index_u64(&mut vm, JsValue::Number(-1.0)).unwrap_err();
    let msg = format!("{err:?}");
    assert!(
        msg.contains("must be a non-negative safe integer"),
        "expected safe-integer rejection message, got {msg}"
    );
}

#[test]
fn to_index_u64_neg_infinity_rejects() {
    let mut vm = Vm::new();
    let err = try_to_index_u64(&mut vm, JsValue::Number(f64::NEG_INFINITY)).unwrap_err();
    let msg = format!("{err:?}");
    assert!(
        msg.contains("must be a non-negative safe integer"),
        "expected non-finite rejection, got {msg}"
    );
}

#[test]
fn to_index_u64_pos_infinity_rejects() {
    let mut vm = Vm::new();
    // `+Infinity` survives `to_integer_or_infinity` (preserved
    // unchanged), so the `is_finite()` guard rejects it before the
    // bounds check.  Same path as the negative case but worth
    // pinning since the literal value is different.
    let err = try_to_index_u64(&mut vm, JsValue::Number(f64::INFINITY)).unwrap_err();
    let msg = format!("{err:?}");
    assert!(
        msg.contains("must be a non-negative safe integer"),
        "expected non-finite rejection, got {msg}"
    );
}

/// `Number.MAX_SAFE_INTEGER == 2^53 - 1` — the maximum value `ToIndex`
/// accepts.  Boundary case: this **must** succeed (the `>=` check uses
/// `2^53`, so `2^53 - 1` lands inside the open upper bound).
#[test]
fn to_index_u64_max_safe_integer_accepted() {
    let mut vm = Vm::new();
    let max_safe = (1_u64 << 53) - 1;
    #[allow(clippy::cast_precision_loss)]
    let as_f64 = max_safe as f64;
    assert_eq!(
        try_to_index_u64(&mut vm, JsValue::Number(as_f64)).unwrap(),
        max_safe
    );
}

/// `Number.MAX_SAFE_INTEGER + 1 == 2^53` — first value above the spec
/// limit.  Boundary case: this **must** reject with the
/// `"exceeds the maximum safe integer"` message so the rejection
/// distinguishes from the negative / non-finite branch.
#[test]
fn to_index_u64_two_pow_53_rejects_with_max_message() {
    let mut vm = Vm::new();
    #[allow(clippy::cast_precision_loss)]
    let two_pow_53 = (1_u64 << 53) as f64;
    let err = try_to_index_u64(&mut vm, JsValue::Number(two_pow_53)).unwrap_err();
    let msg = format!("{err:?}");
    assert!(
        msg.contains("exceeds the maximum safe integer"),
        "expected upper-bound rejection, got {msg}"
    );
}

/// `undefined` coerces to `NaN` via `ToNumber`, then the NaN-fast-path
/// in `to_integer_or_infinity` returns `0` — so `to_index_u64`
/// implicitly handles `undefined → 0` via the standard pipeline.  The
/// `BigInt.asIntN` caller still wraps with an explicit `Undefined →
/// 0` early-return for symmetry with the spec, but this test pins
/// the canonical-helper-only behaviour so future refactors of either
/// caller can drop the wrapper without breaking semantics.
#[test]
fn to_index_u64_undefined_via_to_number_pipeline_returns_zero() {
    let mut vm = Vm::new();
    assert_eq!(try_to_index_u64(&mut vm, JsValue::Undefined).unwrap(), 0);
}
