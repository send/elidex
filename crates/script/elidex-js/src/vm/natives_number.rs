//! Native Number.prototype and Number static methods.

use super::value::{JsValue, NativeContext, ObjectKind, VmError};

/// Extract the numeric value from `this`: either a Number primitive or
/// a NumberWrapper object.
fn this_number_value(ctx: &NativeContext<'_>, this: JsValue) -> Result<f64, VmError> {
    match this {
        JsValue::Number(n) => Ok(n),
        JsValue::Object(id) => match ctx.get_object(id).kind {
            ObjectKind::NumberWrapper(n) => Ok(n),
            _ => Err(VmError::type_error(
                "Number.prototype method called on non-number",
            )),
        },
        _ => Err(VmError::type_error(
            "Number.prototype method called on non-number",
        )),
    }
}

pub(super) fn native_number_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = this_number_value(ctx, this)?;
    let s = if n.is_nan() {
        "NaN".to_string()
    } else if n.is_infinite() {
        if n.is_sign_positive() {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        }
    } else if n == 0.0 {
        "0".to_string()
    } else if n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 {
        // 2^53: maximum safe integer representable in f64
        format!("{}", n as i64)
    } else {
        format!("{n}")
    };
    let id = ctx.intern(&s);
    Ok(JsValue::String(id))
}

pub(super) fn native_number_value_of(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = this_number_value(ctx, this)?;
    Ok(JsValue::Number(n))
}

pub(super) fn native_number_to_fixed(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let n = this_number_value(ctx, this)?;
    // §20.1.3.3 step 4: ToIntegerOrInfinity then range-check.
    let digits = {
        let arg = args.first().copied().unwrap_or(JsValue::Number(0.0));
        let d = super::coerce::to_number(ctx.vm, arg)?;
        let raw = if d.is_nan() {
            0.0
        } else if d.is_infinite() {
            d
        } else {
            d.trunc()
        };
        if !(0.0..=100.0).contains(&raw) {
            return Err(VmError::range_error(
                "toFixed() digits argument must be between 0 and 100",
            ));
        }
        #[allow(clippy::cast_sign_loss)]
        {
            raw as usize
        }
    };
    // Non-finite values return their toString representation (§20.1.3.3 step 5-6).
    let s = if n.is_nan() {
        "NaN".to_string()
    } else if n.is_infinite() {
        if n.is_sign_positive() {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        }
    } else {
        format!("{n:.digits$}")
    };
    let id = ctx.intern(&s);
    Ok(JsValue::String(id))
}

// -- Number.prototype.toExponential -------------------------------------------

pub(super) fn native_number_to_exponential(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // §20.1.3.2: step 1 — ThisNumberValue
    let n = this_number_value(ctx, this)?;
    // step 2 — ToInteger(fractionDigits) before non-finite check
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let fraction_digits = if arg == JsValue::Undefined {
        None
    } else {
        let digits = super::coerce::to_number(ctx.vm, arg)?;
        let digits = digits.trunc();
        if !(0.0..=100.0).contains(&digits) {
            return Err(VmError::range_error(
                "toExponential() argument must be between 0 and 100",
            ));
        }
        #[allow(clippy::cast_sign_loss)]
        Some(digits as usize)
    };
    // step 3-4 — non-finite
    if n.is_nan() || n.is_infinite() {
        return native_number_to_string(ctx, this, &[]);
    }
    let raw = match fraction_digits {
        None => format!("{n:e}"),
        Some(d) => format!("{n:.d$e}"),
    };
    // Rust uses "e" without "+" for positive exponents; ES2020 requires "e+".
    let s = fix_exponential_sign(&raw);
    let id = ctx.intern(&s);
    Ok(JsValue::String(id))
}

// -- Number.prototype.toPrecision ---------------------------------------------

pub(super) fn native_number_to_precision(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // §20.1.3.5: step 1 — ThisNumberValue
    let n = this_number_value(ctx, this)?;
    // step 2 — if undefined, return ToString(x)
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    if arg == JsValue::Undefined {
        return native_number_to_string(ctx, this, &[]);
    }
    // step 3 — ToIntegerOrInfinity(precision) + RangeError
    let precision = super::coerce::to_number(ctx.vm, arg)?;
    let precision = precision.trunc();
    if !(1.0..=100.0).contains(&precision) {
        return Err(VmError::range_error(
            "toPrecision() argument must be between 1 and 100",
        ));
    }
    // step 4-5 — non-finite
    if n.is_nan() || n.is_infinite() {
        return native_number_to_string(ctx, this, &[]);
    }
    #[allow(clippy::cast_sign_loss)]
    let p = precision as usize;
    let s = format_significant_digits(n, p);
    let id = ctx.intern(&s);
    Ok(JsValue::String(id))
}

// -- Number static methods (ES2015+) ------------------------------------------

/// Number.isFinite(value) — §20.1.2.1
pub(super) fn native_number_is_finite(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let result = matches!(val, JsValue::Number(n) if n.is_finite());
    Ok(JsValue::Boolean(result))
}

/// Number.isInteger(value) — §20.1.2.3
pub(super) fn native_number_is_integer(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let result = matches!(val, JsValue::Number(n) if n.is_finite() && n.trunc() == n);
    Ok(JsValue::Boolean(result))
}

/// Number.isNaN(value) — §20.1.2.4
pub(super) fn native_number_is_nan(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let result = matches!(val, JsValue::Number(n) if n.is_nan());
    Ok(JsValue::Boolean(result))
}

/// Number.isSafeInteger(value) — §20.1.2.5
pub(super) fn native_number_is_safe_integer(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    let result = matches!(val, JsValue::Number(n)
        if n.is_finite() && n.trunc() == n && n.abs() <= 9_007_199_254_740_991.0);
    Ok(JsValue::Boolean(result))
}

// -- Helpers ------------------------------------------------------------------

/// Insert "+" after "e" for positive exponents (Rust omits it, ES2020 requires it).
fn fix_exponential_sign(s: &str) -> String {
    if let Some(pos) = s.find('e') {
        if s.as_bytes().get(pos + 1) != Some(&b'-') {
            return format!("{}e+{}", &s[..pos], &s[pos + 1..]);
        }
    }
    s.to_string()
}

/// Format a number with a given number of significant digits (for toPrecision).
/// Implements ES2020 §20.1.3.5 steps 7-12.
/// Uses Rust's {:e} formatter to robustly extract the exponent (avoids log10
/// floating-point imprecision near powers of 10).
fn format_significant_digits(n: f64, precision: usize) -> String {
    if n == 0.0 {
        if precision <= 1 {
            return "0".to_string();
        }
        let mut s = "0.".to_string();
        for _ in 0..precision - 1 {
            s.push('0');
        }
        return s;
    }
    let negative = n < 0.0;
    let abs_n = n.abs();

    // Use Rust's exponential formatter to get correct significant digits.
    // Format with (precision-1) decimal places in exponential notation, then
    // decide whether to emit in fixed or exponential form.
    let p = precision;
    let exp_str = format!("{abs_n:.prec$e}", prec = p - 1);
    // Parse exponent from the formatted string (e.g., "1.235e5" or "1.235e-7")
    let e_pos = exp_str.find('e').unwrap();
    let e: i32 = exp_str[e_pos + 1..].parse().unwrap();
    // Extract the significant digits (remove '.') from mantissa
    let mantissa = &exp_str[..e_pos];
    let digits: String = mantissa.chars().filter(|&c| c != '.').collect();

    // §20.1.3.5 step 10-12: exponential notation if e >= p or e < -6
    #[allow(clippy::cast_possible_wrap)]
    let p_i32 = p as i32;
    let formatted = if e >= p_i32 || e < -6 {
        if digits.len() == 1 {
            format!("{}e{}{}", &digits, if e >= 0 { "+" } else { "" }, e)
        } else {
            format!(
                "{}.{}e{}{}",
                &digits[..1],
                &digits[1..],
                if e >= 0 { "+" } else { "" },
                e
            )
        }
    } else if e >= 0 {
        // Fixed notation: e+1 digits before decimal, rest after
        #[allow(clippy::cast_sign_loss)]
        let int_digits = (e + 1) as usize;
        if int_digits >= digits.len() {
            // All digits are before the decimal; pad with zeros if needed
            let mut s = digits.clone();
            for _ in 0..(int_digits - digits.len()) {
                s.push('0');
            }
            s
        } else {
            format!("{}.{}", &digits[..int_digits], &digits[int_digits..])
        }
    } else {
        // e < 0: number is 0.000...digits
        #[allow(clippy::cast_sign_loss)]
        let leading_zeros = (-e - 1) as usize;
        let mut s = "0.".to_string();
        for _ in 0..leading_zeros {
            s.push('0');
        }
        s.push_str(&digits);
        s
    };

    if negative {
        format!("-{formatted}")
    } else {
        formatted
    }
}
