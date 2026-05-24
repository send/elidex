//! Object integrity methods — freeze, seal, preventExtensions and the
//! corresponding query predicates (isFrozen, isSealed, isExtensible).

use super::super::value::{JsValue, NativeContext, ObjectId, PropertyKey, VmError};

/// Mark an object non-extensible and collect its property keys+attrs.
/// Shared by `freeze` and `seal`.
fn lock_and_collect_keys(
    ctx: &mut NativeContext<'_>,
    obj_id: ObjectId,
) -> Vec<(PropertyKey, super::super::shape::PropertyAttrs)> {
    ctx.get_object_mut(obj_id).extensible = false;
    ctx.get_object(obj_id)
        .storage
        .iter_keys(&ctx.vm.shapes)
        .collect()
}

/// `Object.freeze(obj)` — ECMA-262 §20.1.2.6
pub(in super::super) fn native_object_freeze(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_id) = obj_val else {
        return Ok(obj_val);
    };
    for (key, attrs) in lock_and_collect_keys(ctx, obj_id) {
        let new_attrs = super::super::shape::PropertyAttrs {
            writable: false,
            configurable: false,
            enumerable: attrs.enumerable,
            is_accessor: attrs.is_accessor,
        };
        if new_attrs != attrs {
            ctx.vm.reconfigure_property(obj_id, key, new_attrs, None);
        }
    }
    Ok(obj_val)
}

/// `Object.seal(obj)` — ECMA-262 §20.1.2.22
pub(in super::super) fn native_object_seal(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_id) = obj_val else {
        return Ok(obj_val);
    };
    for (key, attrs) in lock_and_collect_keys(ctx, obj_id) {
        if attrs.configurable {
            let new_attrs = super::super::shape::PropertyAttrs {
                configurable: false,
                ..attrs
            };
            ctx.vm.reconfigure_property(obj_id, key, new_attrs, None);
        }
    }
    Ok(obj_val)
}

/// `Object.preventExtensions(obj)` — ECMA-262 §20.1.2.20
pub(in super::super) fn native_object_prevent_extensions(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_val = args.first().copied().unwrap_or(JsValue::Undefined);
    if let JsValue::Object(obj_id) = obj_val {
        ctx.get_object_mut(obj_id).extensible = false;
    }
    Ok(obj_val)
}

/// `Object.isFrozen(obj)` — ECMA-262 §20.1.2.17
pub(in super::super) fn native_object_is_frozen(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_id) = obj_val else {
        return Ok(JsValue::Boolean(true));
    };
    let obj = ctx.get_object(obj_id);
    if obj.extensible {
        return Ok(JsValue::Boolean(false));
    }
    // All named properties must be non-writable + non-configurable.
    // An empty non-extensible object is vacuously frozen per spec.
    let frozen = obj
        .storage
        .iter_keys(&ctx.vm.shapes)
        .all(|(_, attrs)| !attrs.configurable && (attrs.is_accessor || !attrs.writable));
    Ok(JsValue::Boolean(frozen))
}

/// `Object.isSealed(obj)` — ECMA-262 §20.1.2.18
pub(in super::super) fn native_object_is_sealed(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_id) = obj_val else {
        return Ok(JsValue::Boolean(true));
    };
    let obj = ctx.get_object(obj_id);
    if obj.extensible {
        return Ok(JsValue::Boolean(false));
    }
    let sealed = obj
        .storage
        .iter_keys(&ctx.vm.shapes)
        .all(|(_, attrs)| !attrs.configurable);
    Ok(JsValue::Boolean(sealed))
}

/// `Object.isExtensible(obj)` — ECMA-262 §20.1.2.16
pub(in super::super) fn native_object_is_extensible(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_id) = obj_val else {
        return Ok(JsValue::Boolean(false));
    };
    Ok(JsValue::Boolean(ctx.get_object(obj_id).extensible))
}
