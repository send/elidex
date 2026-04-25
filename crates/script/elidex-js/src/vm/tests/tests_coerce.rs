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
    to_int32, to_integer_or_infinity, to_number, to_string, typeof_str,
};
use super::super::coerce_ops::*;
use super::super::value::{JsValue, ObjectId, StringId};
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

/// `f64::trunc` would leak the input's IEEE 754 sign bit (`(-0.0).trunc()
/// == -0.0`); ES §7.1.5 returns the mathematical value 0 for both `+0` and
/// `-0`.  `assert_eq!` compares `±0` as equal, so use `is_sign_negative`
/// to lock the canonicalisation.
#[test]
fn to_integer_or_infinity_canonicalises_negative_zero() {
    let from_pos_zero = to_integer_or_infinity(0.0);
    assert_eq!(from_pos_zero, 0.0);
    assert!(!from_pos_zero.is_sign_negative());

    let from_neg_zero = to_integer_or_infinity(-0.0);
    assert_eq!(from_neg_zero, 0.0);
    assert!(!from_neg_zero.is_sign_negative());

    // NaN-fast-path also returns +0.
    let from_nan = to_integer_or_infinity(f64::NAN);
    assert!(!from_nan.is_sign_negative());
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
