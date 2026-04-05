//! Native Number.prototype methods.

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
    let s = format!("{n:.digits$}");
    let id = ctx.intern(&s);
    Ok(JsValue::String(id))
}
