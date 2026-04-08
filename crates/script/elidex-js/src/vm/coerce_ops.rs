//! Arithmetic, comparison, bitwise, and unary operator semantics.
//!
//! Extracted from `coerce.rs` to keep that module focused on type coercions.

use super::coerce::{to_boolean, to_int32, to_number, to_uint32};
use super::value::{BigIntId, JsValue, VmError};
use super::VmInner;
use num_bigint::BigInt as BigIntValue;
use num_bigint::Sign;

// ---------------------------------------------------------------------------
// Arithmetic operators
// ---------------------------------------------------------------------------

/// Binary numeric operator (-, *, /, %, **).
pub(crate) fn op_numeric_binary(
    vm: &mut VmInner,
    lhs: JsValue,
    rhs: JsValue,
    op: NumericBinaryOp,
) -> Result<JsValue, VmError> {
    // BigInt path
    if let (JsValue::BigInt(ai), JsValue::BigInt(bi)) = (lhs, rhs) {
        return bigint_binary(vm, ai, bi, op);
    }
    let a = to_number(vm, lhs)?;
    let b = to_number(vm, rhs)?;
    let result = match op {
        NumericBinaryOp::Sub => a - b,
        NumericBinaryOp::Mul => a * b,
        NumericBinaryOp::Div => a / b,
        NumericBinaryOp::Rem => a % b,
        NumericBinaryOp::Exp => {
            // ES2020 §6.1.6.1.4: deviations from IEEE 754 pow
            if b.is_nan() || (a.abs() == 1.0 && b.is_infinite()) {
                f64::NAN
            } else {
                a.powf(b)
            }
        }
    };
    Ok(JsValue::Number(result))
}

/// BigInt binary arithmetic.
fn bigint_binary(
    vm: &mut VmInner,
    ai: BigIntId,
    bi: BigIntId,
    op: NumericBinaryOp,
) -> Result<JsValue, VmError> {
    let a = vm.bigints.get(ai);
    let b = vm.bigints.get(bi);
    let result = match op {
        NumericBinaryOp::Sub => a - b,
        NumericBinaryOp::Mul => a * b,
        NumericBinaryOp::Div => {
            if b.sign() == Sign::NoSign {
                return Err(VmError::range_error("Division by zero"));
            }
            a / b
        }
        NumericBinaryOp::Rem => {
            if b.sign() == Sign::NoSign {
                return Err(VmError::range_error("Division by zero"));
            }
            a % b
        }
        NumericBinaryOp::Exp => {
            if b.sign() == Sign::Minus {
                return Err(VmError::range_error("Exponent must be positive for BigInt"));
            }
            let exp: u32 = b
                .try_into()
                .map_err(|_| VmError::range_error("BigInt exponent too large"))?;
            a.pow(exp)
        }
    };
    let id = vm.bigints.alloc(result);
    Ok(JsValue::BigInt(id))
}

/// Exact comparison of a BigInt against a Number (§6.1.6.2.14).
/// Returns `None` if the Number is NaN, otherwise the ordering.
pub(crate) fn compare_bigint_number(bi: &BigIntValue, n: f64) -> Option<std::cmp::Ordering> {
    use num_traits::FromPrimitive;
    use std::cmp::Ordering;

    if n.is_nan() {
        return None;
    }
    if n == f64::INFINITY {
        return Some(Ordering::Less);
    }
    if n == f64::NEG_INFINITY {
        return Some(Ordering::Greater);
    }

    let n_floor = n.floor();
    let n_bi = BigIntValue::from_f64(n_floor).unwrap();

    match bi.cmp(&n_bi) {
        Ordering::Less => Some(Ordering::Less),
        Ordering::Greater => Some(Ordering::Greater),
        Ordering::Equal => {
            if n == n_floor {
                Some(Ordering::Equal)
            } else {
                Some(Ordering::Less)
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum NumericBinaryOp {
    Sub,
    Mul,
    Div,
    Rem,
    Exp,
}

// ---------------------------------------------------------------------------
// Comparison operators (ES2020 §7.2.14)
// ---------------------------------------------------------------------------

/// Abstract relational comparison. Returns `Ok(Some(true))` if x < y,
/// `Ok(Some(false))` if x >= y, `Ok(None)` if undefined (NaN involved),
/// or `Err` if ToNumber throws (e.g. Symbol).
pub(crate) fn abstract_relational(
    vm: &mut VmInner,
    x: JsValue,
    y: JsValue,
    left_first: bool,
) -> Result<Option<bool>, VmError> {
    if let (JsValue::String(a), JsValue::String(b)) = (x, y) {
        return Ok(Some(vm.strings.get(a) < vm.strings.get(b)));
    }

    if let (JsValue::BigInt(a), JsValue::BigInt(b)) = (x, y) {
        return Ok(Some(vm.bigints.get(a) < vm.bigints.get(b)));
    }
    if let (JsValue::BigInt(bi), JsValue::Number(n)) | (JsValue::Number(n), JsValue::BigInt(bi)) =
        (x, y)
    {
        use std::cmp::Ordering;
        let cmp = compare_bigint_number(vm.bigints.get(bi), n);
        return if matches!(x, JsValue::BigInt(_)) {
            Ok(cmp.map(|o| o == Ordering::Less))
        } else {
            Ok(cmp.map(|o| o == Ordering::Greater))
        };
    }

    // BigInt vs non-Number (String/Boolean/etc.): coerce the non-BigInt
    // to Number, then compare BigInt vs Number.
    if matches!(x, JsValue::BigInt(_)) {
        let ny = to_number(vm, y)?;
        return abstract_relational(vm, x, JsValue::Number(ny), left_first);
    }
    if matches!(y, JsValue::BigInt(_)) {
        let nx = to_number(vm, x)?;
        return abstract_relational(vm, JsValue::Number(nx), y, left_first);
    }

    let (nx, ny) = if left_first {
        (to_number(vm, x)?, to_number(vm, y)?)
    } else {
        let b = to_number(vm, y)?;
        let a = to_number(vm, x)?;
        (a, b)
    };

    if nx.is_nan() || ny.is_nan() {
        return Ok(None);
    }
    Ok(Some(nx < ny))
}

// ---------------------------------------------------------------------------
// Bitwise operators
// ---------------------------------------------------------------------------

/// Bitwise binary operator.
pub(crate) fn op_bitwise(
    vm: &mut VmInner,
    lhs: JsValue,
    rhs: JsValue,
    op: BitwiseOp,
) -> Result<JsValue, VmError> {
    if let (JsValue::BigInt(ai), JsValue::BigInt(bi)) = (lhs, rhs) {
        let a = vm.bigints.get(ai);
        let b = vm.bigints.get(bi);
        let result = match op {
            BitwiseOp::And => a & b,
            BitwiseOp::Or => a | b,
            BitwiseOp::Xor => a ^ b,
            BitwiseOp::Shl => {
                let shift: i64 = b
                    .try_into()
                    .map_err(|_| VmError::range_error("BigInt shift amount too large"))?;
                if shift >= 0 {
                    a.clone() << shift.cast_unsigned()
                } else {
                    a.clone() >> (-shift).cast_unsigned()
                }
            }
            BitwiseOp::Shr => {
                let shift: i64 = b
                    .try_into()
                    .map_err(|_| VmError::range_error("BigInt shift amount too large"))?;
                if shift >= 0 {
                    a.clone() >> shift.cast_unsigned()
                } else {
                    a.clone() << (-shift).cast_unsigned()
                }
            }
            BitwiseOp::UShr => {
                return Err(VmError::type_error(
                    "BigInt does not support unsigned right shift",
                ));
            }
        };
        let id = vm.bigints.alloc(result);
        return Ok(JsValue::BigInt(id));
    }
    if matches!(lhs, JsValue::BigInt(_)) || matches!(rhs, JsValue::BigInt(_)) {
        return Err(VmError::type_error(
            "Cannot mix BigInt and other types, use explicit conversions",
        ));
    }
    Ok(match op {
        BitwiseOp::And => JsValue::Number(f64::from(to_int32(vm, lhs)? & to_int32(vm, rhs)?)),
        BitwiseOp::Or => JsValue::Number(f64::from(to_int32(vm, lhs)? | to_int32(vm, rhs)?)),
        BitwiseOp::Xor => JsValue::Number(f64::from(to_int32(vm, lhs)? ^ to_int32(vm, rhs)?)),
        BitwiseOp::Shl => {
            let x = to_int32(vm, lhs)?;
            let count = to_uint32(vm, rhs)? & 0x1f;
            JsValue::Number(f64::from(x << count))
        }
        BitwiseOp::Shr => {
            let x = to_int32(vm, lhs)?;
            let count = to_uint32(vm, rhs)? & 0x1f;
            JsValue::Number(f64::from(x >> count))
        }
        BitwiseOp::UShr => {
            let x = to_uint32(vm, lhs)?;
            let count = to_uint32(vm, rhs)? & 0x1f;
            JsValue::Number(f64::from(x >> count))
        }
    })
}

#[derive(Clone, Copy)]
pub(crate) enum BitwiseOp {
    And,
    Or,
    Xor,
    Shl,
    Shr,
    UShr,
}

// ---------------------------------------------------------------------------
// Unary operators
// ---------------------------------------------------------------------------

/// Unary `-` (negate).
pub(crate) fn op_neg(vm: &mut VmInner, val: JsValue) -> Result<JsValue, VmError> {
    if let JsValue::BigInt(id) = val {
        let v = -vm.bigints.get(id).clone();
        let new_id = vm.bigints.alloc(v);
        return Ok(JsValue::BigInt(new_id));
    }
    Ok(JsValue::Number(-to_number(vm, val)?))
}

/// Unary `+` (ToNumber).
pub(crate) fn op_pos(vm: &VmInner, val: JsValue) -> Result<JsValue, VmError> {
    if matches!(val, JsValue::BigInt(_)) {
        return Err(VmError::type_error(
            "Cannot convert a BigInt value to a number",
        ));
    }
    Ok(JsValue::Number(to_number(vm, val)?))
}

/// Unary `!` (logical NOT).
pub(crate) fn op_not(vm: &VmInner, val: JsValue) -> JsValue {
    JsValue::Boolean(!to_boolean(vm, val))
}

/// Unary `~` (bitwise NOT).
pub(crate) fn op_bitnot(vm: &mut VmInner, val: JsValue) -> Result<JsValue, VmError> {
    if let JsValue::BigInt(id) = val {
        let v = !vm.bigints.get(id).clone();
        let new_id = vm.bigints.alloc(v);
        return Ok(JsValue::BigInt(new_id));
    }
    Ok(JsValue::Number(f64::from(!to_int32(vm, val)?)))
}

/// Unary `void` — always returns undefined.
pub(crate) fn op_void() -> JsValue {
    JsValue::Undefined
}
