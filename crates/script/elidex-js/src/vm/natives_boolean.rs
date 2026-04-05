//! Native Boolean.prototype methods.

use super::value::{JsValue, NativeContext, ObjectKind, VmError};

/// Extract the boolean value from `this`: either a Boolean primitive or
/// a BooleanWrapper object.
fn this_boolean_value(ctx: &NativeContext<'_>, this: JsValue) -> Result<bool, VmError> {
    match this {
        JsValue::Boolean(b) => Ok(b),
        JsValue::Object(id) => {
            if let ObjectKind::BooleanWrapper(b) = ctx.get_object(id).kind {
                Ok(b)
            } else {
                Err(VmError::type_error(
                    "Boolean.prototype method called on non-boolean",
                ))
            }
        }
        _ => Err(VmError::type_error(
            "Boolean.prototype method called on non-boolean",
        )),
    }
}

pub(super) fn native_boolean_to_string(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let b = this_boolean_value(ctx, this)?;
    let id = if b {
        ctx.vm.well_known.r#true
    } else {
        ctx.vm.well_known.r#false
    };
    Ok(JsValue::String(id))
}

pub(super) fn native_boolean_value_of(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let b = this_boolean_value(ctx, this)?;
    Ok(JsValue::Boolean(b))
}
