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

/// ECMA-262 §20.3.1.1 `Boolean([value])`. Call form returns the Boolean
/// primitive (`Boolean()` → `false`); construct form boxes it in a `BooleanWrapper`.
pub(crate) fn native_boolean_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let b = !args.is_empty() && ctx.to_boolean(args[0]);
    if ctx.is_construct() {
        let JsValue::Object(instance_id) = this else {
            let wrapper = ctx.vm.create_boolean_wrapper(b);
            return Ok(JsValue::Object(wrapper));
        };
        ctx.vm.promote_to_boolean_wrapper(instance_id, b);
        Ok(JsValue::Object(instance_id))
    } else {
        Ok(JsValue::Boolean(b))
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
