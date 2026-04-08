//! Native BigInt() function and BigInt.prototype methods.

use num_bigint::BigInt as BigIntValue;

use super::value::{JsValue, NativeContext, VmError};

/// `BigInt(value)` — convert a value to BigInt (§21.2.1.1).
/// Not a constructor: `new BigInt()` throws TypeError.
pub(super) fn native_bigint_constructor(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let bi = match val {
        JsValue::BigInt(_) => return Ok(val),
        JsValue::Boolean(b) => BigIntValue::from(i64::from(b)),
        JsValue::Number(n) => {
            if !n.is_finite() || n.fract() != 0.0 {
                return Err(VmError::range_error(
                    "The number is not safe to convert to a BigInt",
                ));
            }
            BigIntValue::from(n as i64)
        }
        JsValue::String(id) => {
            let s = ctx.vm.strings.get_utf8(id);
            let s = s.trim();
            if s.is_empty() {
                BigIntValue::from(0)
            } else {
                s.parse::<BigIntValue>().map_err(|_| {
                    VmError::syntax_error(format!("Cannot convert \"{s}\" to a BigInt"))
                })?
            }
        }
        _ => {
            return Err(VmError::type_error("Cannot convert value to a BigInt"));
        }
    };
    let id = ctx.vm.bigints.alloc(bi);
    Ok(JsValue::BigInt(id))
}

/// `BigInt.prototype.toString(radix?)`
pub(super) fn native_bigint_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let bi = this_bigint_value(ctx, this)?;
    let radix = match args.first() {
        Some(&JsValue::Number(n)) => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let r = n as u32;
            if !(2..=36).contains(&r) || (f64::from(r) - n).abs() > f64::EPSILON {
                return Err(VmError::range_error("radix must be between 2 and 36"));
            }
            r
        }
        Some(JsValue::Undefined) | None => 10,
        _ => {
            return Err(VmError::range_error("radix must be between 2 and 36"));
        }
    };
    let s = if radix == 10 {
        bi.to_string()
    } else {
        bigint_to_radix_string(&bi, radix)
    };
    let id = ctx.intern(&s);
    Ok(JsValue::String(id))
}

/// `BigInt.prototype.valueOf()`
pub(super) fn native_bigint_value_of(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    match this {
        JsValue::BigInt(_) => Ok(this),
        JsValue::Object(id) => match &ctx.get_object(id).kind {
            super::value::ObjectKind::BigIntWrapper(bi_id) => Ok(JsValue::BigInt(*bi_id)),
            _ => Err(VmError::type_error(
                "BigInt.prototype.valueOf called on non-bigint",
            )),
        },
        _ => Err(VmError::type_error(
            "BigInt.prototype.valueOf called on non-bigint",
        )),
    }
}

/// `BigInt.asIntN(bits, bigint)`
pub(super) fn native_bigint_as_int_n(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let bits = match args.first() {
        Some(&JsValue::Number(n)) if n >= 0.0 && n < 2.0f64.powi(53) => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                n as u64
            }
        }
        _ => return Err(VmError::type_error("expected non-negative number for bits")),
    };
    let bi_id = match args.get(1) {
        Some(JsValue::BigInt(id)) => *id,
        _ => return Err(VmError::type_error("expected BigInt")),
    };
    let bi = ctx.vm.bigints.get(bi_id);
    let modulus = BigIntValue::from(1) << bits;
    let result = bi % &modulus;
    // Sign-extend: if result >= 2^(bits-1), subtract modulus
    let half = BigIntValue::from(1) << (bits - 1);
    let result = if result >= half {
        result - modulus
    } else {
        result
    };
    let id = ctx.vm.bigints.alloc(result);
    Ok(JsValue::BigInt(id))
}

/// `BigInt.asUintN(bits, bigint)`
pub(super) fn native_bigint_as_uint_n(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let bits = match args.first() {
        Some(&JsValue::Number(n)) if n >= 0.0 && n < 2.0f64.powi(53) => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                n as u64
            }
        }
        _ => return Err(VmError::type_error("expected non-negative number for bits")),
    };
    let bi_id = match args.get(1) {
        Some(JsValue::BigInt(id)) => *id,
        _ => return Err(VmError::type_error("expected BigInt")),
    };
    let bi = ctx.vm.bigints.get(bi_id);
    let modulus = BigIntValue::from(1) << bits;
    let result = ((bi % &modulus) + &modulus) % &modulus;
    let id = ctx.vm.bigints.alloc(result);
    Ok(JsValue::BigInt(id))
}

// -- Helpers ------------------------------------------------------------------

fn this_bigint_value(ctx: &NativeContext<'_>, this: JsValue) -> Result<BigIntValue, VmError> {
    match this {
        JsValue::BigInt(id) => Ok(ctx.vm.bigints.get(id).clone()),
        JsValue::Object(id) => match &ctx.get_object(id).kind {
            super::value::ObjectKind::BigIntWrapper(bi_id) => {
                Ok(ctx.vm.bigints.get(*bi_id).clone())
            }
            _ => Err(VmError::type_error(
                "BigInt.prototype method called on non-bigint",
            )),
        },
        _ => Err(VmError::type_error(
            "BigInt.prototype method called on non-bigint",
        )),
    }
}

fn bigint_to_radix_string(bi: &BigIntValue, radix: u32) -> String {
    use num_bigint::Sign;
    let (sign, digits) = bi.to_radix_be(radix);
    let mut s = String::new();
    if sign == Sign::Minus {
        s.push('-');
    }
    if digits.is_empty() {
        s.push('0');
    } else {
        for d in &digits {
            let c = if *d < 10 {
                (b'0' + d) as char
            } else {
                (b'a' + d - 10) as char
            };
            s.push(c);
        }
    }
    s
}
