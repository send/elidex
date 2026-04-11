//! Math built-in methods and constants (ES2020 §20.2).

use super::coerce::f64_to_uint32;
use super::value::{JsValue, NativeContext, VmError};

pub(super) fn native_math_abs(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.abs()))
}

pub(super) fn native_math_ceil(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.ceil()))
}

pub(super) fn native_math_floor(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.floor()))
}

pub(super) fn native_math_round(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    // ES2020 §20.2.2.28: if n is in [-0.5, 0), result is -0.
    let result = if (-0.5..0.0).contains(&n) {
        -0.0_f64
    } else {
        (n + 0.5).floor()
    };
    Ok(JsValue::Number(result))
}

pub(super) fn native_math_max(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if args.is_empty() {
        return Ok(JsValue::Number(f64::NEG_INFINITY));
    }
    let mut result = f64::NEG_INFINITY;
    for &arg in args {
        let n = ctx.to_number(arg)?;
        if n.is_nan() {
            return Ok(JsValue::Number(f64::NAN));
        }
        // §20.2.2.24: +0 is greater than -0
        if n > result || (n == 0.0 && result == 0.0 && result.is_sign_negative()) {
            result = n;
        }
    }
    Ok(JsValue::Number(result))
}

pub(super) fn native_math_min(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if args.is_empty() {
        return Ok(JsValue::Number(f64::INFINITY));
    }
    let mut result = f64::INFINITY;
    for &arg in args {
        let n = ctx.to_number(arg)?;
        if n.is_nan() {
            return Ok(JsValue::Number(f64::NAN));
        }
        // §20.2.2.25: -0 is less than +0
        if n < result || (n == 0.0 && result == 0.0 && n.is_sign_negative()) {
            result = n;
        }
    }
    Ok(JsValue::Number(result))
}

pub(super) fn native_math_random(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // xorshift64 PRNG — not cryptographically secure but sufficient for
    // Math.random(). State is stored in VmInner so successive calls produce
    // distinct values.
    let mut s = ctx.vm.rng_state;
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    ctx.vm.rng_state = s;
    // The shift produces a 53-bit value that fits in f64's mantissa exactly.
    #[allow(clippy::cast_precision_loss)]
    let n = (s >> 11) as f64 / (1u64 << 53) as f64;
    Ok(JsValue::Number(n))
}

pub(super) fn native_math_sqrt(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.sqrt()))
}

pub(super) fn native_math_pow(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let base = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let exp = ctx.to_number(args.get(1).copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(base.powf(exp)))
}

pub(super) fn native_math_log(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.ln()))
}

pub(super) fn native_math_trunc(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.trunc()))
}

pub(super) fn native_math_sign(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let result = if n.is_nan() {
        f64::NAN
    } else if n == 0.0 {
        n // preserve sign of zero
    } else {
        n.signum()
    };
    Ok(JsValue::Number(result))
}

pub(super) fn native_math_sin(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.sin()))
}

pub(super) fn native_math_cos(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.cos()))
}

pub(super) fn native_math_tan(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.tan()))
}

pub(super) fn native_math_asin(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.asin()))
}

pub(super) fn native_math_acos(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.acos()))
}

pub(super) fn native_math_atan(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.atan()))
}

pub(super) fn native_math_atan2(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let y = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let x = ctx.to_number(args.get(1).copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(y.atan2(x)))
}

pub(super) fn native_math_log2(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.log2()))
}

pub(super) fn native_math_log10(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.log10()))
}

pub(super) fn native_math_exp(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.exp()))
}

pub(super) fn native_math_cbrt(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(n.cbrt()))
}

pub(super) fn native_math_hypot(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if args.is_empty() {
        return Ok(JsValue::Number(0.0));
    }
    // ES2020 §20.2.2.18: coerce all, infinity takes precedence over NaN.
    // Use scaled-sum to avoid intermediate overflow for large finite values.
    let mut has_nan = false;
    let mut max_abs = 0.0_f64;
    let mut values = Vec::with_capacity(args.len());
    for &arg in args {
        let n = ctx.to_number(arg)?;
        if n.is_infinite() {
            return Ok(JsValue::Number(f64::INFINITY));
        }
        if n.is_nan() {
            has_nan = true;
        } else {
            let a = n.abs();
            if a > max_abs {
                max_abs = a;
            }
            values.push(a);
        }
    }
    if has_nan {
        return Ok(JsValue::Number(f64::NAN));
    }
    if max_abs == 0.0 {
        return Ok(JsValue::Number(0.0));
    }
    let mut sum = 0.0_f64;
    for &v in &values {
        let scaled = v / max_abs;
        sum += scaled * scaled;
    }
    Ok(JsValue::Number(max_abs * sum.sqrt()))
}

/// ES2020 §20.2.2.11 — Math.clz32(x)
pub(super) fn native_math_clz32(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let i = f64_to_uint32(n);
    Ok(JsValue::Number(f64::from(i.leading_zeros())))
}

/// ES2020 §20.2.2.19 — Math.imul(x, y)
pub(super) fn native_math_imul(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let a = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    let b = ctx.to_number(args.get(1).copied().unwrap_or(JsValue::Undefined))?;
    #[allow(clippy::cast_possible_wrap)]
    let result = (f64_to_uint32(a) as i32).wrapping_mul(f64_to_uint32(b) as i32);
    Ok(JsValue::Number(f64::from(result)))
}

/// ES2020 §20.2.2.17 — Math.fround(x)
pub(super) fn native_math_fround(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = ctx.to_number(args.first().copied().unwrap_or(JsValue::Undefined))?;
    Ok(JsValue::Number(f64::from(n as f32)))
}
