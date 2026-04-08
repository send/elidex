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
                    "Cannot convert non-finite or non-integer number to a BigInt",
                ));
            }
            // Use i64 for safe integers, string roundtrip for larger values.
            #[allow(clippy::cast_possible_truncation)]
            if n.abs() < 9_007_199_254_740_992.0 {
                BigIntValue::from(n as i64)
            } else {
                format!("{n:.0}")
                    .parse::<BigIntValue>()
                    .map_err(|_| VmError::range_error("Cannot convert to BigInt"))?
            }
        }
        JsValue::String(id) => {
            let s = ctx.vm.strings.get_utf8(id);
            let s = s.trim();
            if s.is_empty() {
                BigIntValue::from(0)
            } else {
                crate::vm::dispatch_helpers::parse_bigint_literal(s).ok_or_else(|| {
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
    let radix = match args.first().copied().unwrap_or(JsValue::Undefined) {
        JsValue::Undefined => 10,
        val => {
            let n = super::coerce::to_number(ctx.vm, val)?;
            let n = n.trunc();
            if !n.is_finite() || !(2.0..=36.0).contains(&n) {
                return Err(VmError::range_error("radix must be between 2 and 36"));
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                n as u32
            }
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
    // Validate this is a BigInt (reuses this_bigint_value for type checking).
    match this {
        JsValue::BigInt(_) => Ok(this),
        JsValue::Object(id) => match &ctx.get_object(id).kind {
            super::value::ObjectKind::BigIntWrapper(bi_id) => Ok(JsValue::BigInt(*bi_id)),
            _ => Err(VmError::type_error(
                "BigInt.prototype.valueOf requires a BigInt",
            )),
        },
        _ => Err(VmError::type_error(
            "BigInt.prototype.valueOf requires a BigInt",
        )),
    }
}

/// Coerce a value to a non-negative integer index (§7.1.22 ToIndex).
fn to_index(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<u64, VmError> {
    let n = super::coerce::to_number(ctx.vm, val)?;
    let n = n.trunc();
    if !n.is_finite() || !(0.0..9_007_199_254_740_992.0).contains(&n) {
        return Err(VmError::range_error("index out of range"));
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok(n as u64)
}

/// Coerce a value to BigInt via ToBigInt (§7.1.13).
fn to_bigint(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<JsValue, VmError> {
    match val {
        JsValue::BigInt(_) => Ok(val),
        other => native_bigint_constructor(ctx, JsValue::Undefined, &[other]),
    }
}

/// `BigInt.asIntN(bits, bigint)`
pub(super) fn native_bigint_as_int_n(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let bits = to_index(ctx, args.first().copied().unwrap_or(JsValue::Undefined))?;
    let bi_val = to_bigint(ctx, args.get(1).copied().unwrap_or(JsValue::Undefined))?;
    let JsValue::BigInt(bi_id) = bi_val else {
        unreachable!()
    };
    let bi = ctx.vm.bigints.get(bi_id);
    if bits == 0 {
        let id = ctx.vm.bigints.alloc(BigIntValue::from(0));
        return Ok(JsValue::BigInt(id));
    }
    let modulus = BigIntValue::from(1) << bits;
    let result = bi % &modulus;
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
    let bits = to_index(ctx, args.first().copied().unwrap_or(JsValue::Undefined))?;
    let bi_val = to_bigint(ctx, args.get(1).copied().unwrap_or(JsValue::Undefined))?;
    let JsValue::BigInt(bi_id) = bi_val else {
        unreachable!()
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
