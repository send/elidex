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
    let raw = args.first().copied().unwrap_or(JsValue::Undefined);
    // §21.2.1.1 step 2: if `value` is Object, run `ToPrimitive(value,
    // number)`.  BigIntWrapper's `@@toPrimitive`/`valueOf` returns
    // the primitive BigInt; any user-defined hook likewise has its
    // result coerced through the primitive switch below.
    let val = if matches!(raw, JsValue::Object(_)) {
        ctx.vm.to_primitive(raw, "number")?
    } else {
        raw
    };
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
            let s = super::coerce::trim_js(&s);
            crate::vm::dispatch_helpers::parse_bigint_literal(s).ok_or_else(|| {
                VmError::syntax_error(format!(
                    "Cannot convert \"{}\" to a BigInt",
                    ctx.vm.strings.get_utf8(id)
                ))
            })?
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

/// Coerce a value to a non-negative integer index (ES §7.1.22
/// `ToIndex`).  `Undefined → 0` is the per-method default the
/// `BigInt.asIntN` / `asUintN` callers rely on (their `bits` arg
/// is the only consumer); the spec arithmetic itself routes through
/// [`super::coerce::to_index_u64`] with the V8-shape error prefix
/// `"BigInt"` so the rejection message matches the rest of the
/// engine's `ToIndex` surface.
fn to_index(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<u64, VmError> {
    if matches!(val, JsValue::Undefined) {
        return Ok(0);
    }
    super::coerce::to_index_u64(ctx, val, "BigInt", "bits")
}

/// Coerce a value to BigInt via ToBigInt (§7.1.13).
fn to_bigint(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<JsValue, VmError> {
    match val {
        JsValue::BigInt(_) => Ok(val),
        other => native_bigint_constructor(ctx, JsValue::Undefined, &[other]),
    }
}

/// ES §7.1.13 `ToBigInt` — strict variant that rejects `Number`
/// input with TypeError (vs the inner `to_bigint` helper above
/// which mirrors the `BigInt()` constructor and coerces integer
/// numbers).  Required by TypedArray / DataView BigInt setter
/// paths: `bi64[0] = 1` must throw TypeError per spec §10.4.5.16
/// step 1 (ToBigInt rejects the Number argument), even though
/// `BigInt(1) === 1n` succeeds.
///
/// Step 1 of the abstract operation runs `ToPrimitive(argument,
/// number)` — so `BigInt64Array`-style writes accept a
/// `BigIntWrapper` (whose `@@toPrimitive` / `valueOf` returns the
/// primitive BigInt) and any custom object whose hook returns a
/// BigInt.
#[cfg(feature = "engine")]
fn to_bigint_strict(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<JsValue, VmError> {
    let prim = if matches!(val, JsValue::Object(_)) {
        ctx.vm.to_primitive(val, "number")?
    } else {
        val
    };
    match prim {
        JsValue::BigInt(_) => Ok(prim),
        JsValue::Number(_) => Err(VmError::type_error("Cannot convert a Number to a BigInt")),
        other => to_bigint(ctx, other),
    }
}

/// ES §7.1.15 `ToBigInt64` — coerce `val` to BigInt (strict, so
/// Number rejects), then reduce modulo 2^64 and reinterpret the
/// high-bit-set half as negative.  Used by `BigInt64Array` indexed
/// writes + `DataView.setBigInt64`.  Strings / booleans / bigints
/// coerce successfully; numbers / null / undefined / symbols
/// throw TypeError.
#[cfg(feature = "engine")]
pub(crate) fn to_bigint64(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<i64, VmError> {
    let bi_val = to_bigint_strict(ctx, val)?;
    let JsValue::BigInt(bi_id) = bi_val else {
        unreachable!("to_bigint_strict always returns a BigInt");
    };
    let bi = ctx.vm.bigints.get(bi_id);
    let modulus = BigIntValue::from(1u64) << 64u32;
    let half = BigIntValue::from(1u64) << 63u32;
    // Fold `bi` into `[0, 2^64)` then, if the high bit is set,
    // subtract 2^64 to recover the signed representation in
    // `[-2^63, 2^63)`.  `signed` then fits exactly in `i64`.
    let folded = ((bi % &modulus) + &modulus) % &modulus;
    let signed = if folded >= half {
        folded - modulus
    } else {
        folded
    };
    Ok(signed_to_i64(&signed))
}

/// ES §7.1.16 `ToBigUint64` — coerce `val` to BigInt (strict, so
/// Number rejects) then reduce modulo 2^64.  Used by
/// `BigUint64Array` indexed writes + `DataView.setBigUint64`.
#[cfg(feature = "engine")]
pub(crate) fn to_biguint64(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<u64, VmError> {
    let bi_val = to_bigint_strict(ctx, val)?;
    let JsValue::BigInt(bi_id) = bi_val else {
        unreachable!("to_bigint_strict always returns a BigInt");
    };
    let bi = ctx.vm.bigints.get(bi_id);
    let modulus = BigIntValue::from(1u64) << 64u32;
    let folded = ((bi % &modulus) + &modulus) % &modulus;
    // folded ∈ [0, 2^64) — take low 8 bytes big-endian.
    let (_, bytes_be) = folded.to_bytes_be();
    let mut buf = [0u8; 8];
    let copy_len = bytes_be.len().min(8);
    if copy_len > 0 {
        buf[8 - copy_len..].copy_from_slice(&bytes_be[bytes_be.len() - copy_len..]);
    }
    Ok(u64::from_be_bytes(buf))
}

/// Convert a `BigIntValue` known to fit in `i64` into its raw i64.
/// Uses the signed two's-complement representation: magnitude via
/// big-endian bytes, then sign-negate.
#[cfg(feature = "engine")]
fn signed_to_i64(bi: &BigIntValue) -> i64 {
    use num_bigint::Sign;
    let (sign, bytes_be) = bi.to_bytes_be();
    let mut buf = [0u8; 8];
    let copy_len = bytes_be.len().min(8);
    if copy_len > 0 {
        buf[8 - copy_len..].copy_from_slice(&bytes_be[bytes_be.len() - copy_len..]);
    }
    let magnitude = u64::from_be_bytes(buf);
    match sign {
        Sign::Minus => {
            // `magnitude` is the absolute value, ≤ 2^63 (caller
            // guarantees the fold keeps `bi` in [-2^63, 2^63)).
            // -2^63 is the only case where negation overflows; handle
            // via wrapping.
            magnitude.cast_signed().wrapping_neg()
        }
        _ => magnitude.cast_signed(),
    }
}

/// Maximum bit-width for asIntN/asUintN to prevent OOM from `1 << bits`.
const MAX_AS_INT_BITS: u64 = 1_000_000;

/// `BigInt.asIntN(bits, bigint)`
pub(super) fn native_bigint_as_int_n(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let bits = to_index(ctx, args.first().copied().unwrap_or(JsValue::Undefined))?;
    if bits > MAX_AS_INT_BITS {
        return Err(VmError::range_error("bit width too large"));
    }
    let bi_val = to_bigint(ctx, args.get(1).copied().unwrap_or(JsValue::Undefined))?;
    let JsValue::BigInt(bi_id) = bi_val else {
        unreachable!()
    };
    let bi = ctx.vm.bigints.get(bi_id);
    if bits == 0 {
        // Reuse the pool's canonical 0n — no alloc, no dedup check.
        return Ok(JsValue::BigInt(ctx.vm.bigints.zero));
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
    if bits > MAX_AS_INT_BITS {
        return Err(VmError::range_error("bit width too large"));
    }
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
        for &d in &digits {
            let c = if d < 10 {
                (b'0' + d) as char
            } else {
                (b'a' + d - 10) as char
            };
            s.push(c);
        }
    }
    s
}
