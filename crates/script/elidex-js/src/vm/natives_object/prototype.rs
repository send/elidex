//! Object prototype-related methods — both Object static methods that
//! manipulate the [[Prototype]] (create, getPrototypeOf, setPrototypeOf, is)
//! and Object.prototype methods (hasOwnProperty, valueOf, isPrototypeOf,
//! propertyIsEnumerable).

use super::super::coerce_format::parse_array_index_u32;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, VmError,
};
use super::{to_object_arg, to_property_key};

pub(crate) fn native_object_create(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let proto = args.first().copied().unwrap_or(JsValue::Null);
    let prototype = match proto {
        JsValue::Object(id) => Some(id),
        JsValue::Null => None,
        _ => {
            return Err(VmError::type_error(
                "Object prototype may only be an Object or null",
            ))
        }
    };
    let obj_id = ctx.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype,
        extensible: true,
    });
    Ok(JsValue::Object(obj_id))
}

/// `Object.is(a, b)` — ES2020 §19.1.2.10
pub(crate) fn native_object_is(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let a = args.first().copied().unwrap_or(JsValue::Undefined);
    let b = args.get(1).copied().unwrap_or(JsValue::Undefined);
    Ok(JsValue::Boolean(super::super::value::same_value(a, b)))
}

/// `Object.getPrototypeOf(obj)` — ES2020 §19.1.2.9
pub(crate) fn native_object_get_prototype_of(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_id = to_object_arg(ctx, args)?;
    match ctx.get_object(obj_id).prototype {
        Some(pid) => Ok(JsValue::Object(pid)),
        None => Ok(JsValue::Null),
    }
}

/// `Object.setPrototypeOf(obj, proto)` — ES2020 §19.1.2.21
pub(crate) fn native_object_set_prototype_of(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let proto_val = args.get(1).copied().unwrap_or(JsValue::Undefined);
    // §19.1.2.21 step 1: RequireObjectCoercible(O)
    if matches!(obj_val, JsValue::Null | JsValue::Undefined) {
        return Err(VmError::type_error(
            "Cannot convert undefined or null to object",
        ));
    }
    let JsValue::Object(obj_id) = obj_val else {
        // §19.1.2.21 step 3: Type(O) is not Object → return O
        return Ok(obj_val);
    };
    let new_proto = match proto_val {
        JsValue::Object(id) => Some(id),
        JsValue::Null => None,
        _ => {
            return Err(VmError::type_error(
                "Object prototype may only be an Object or null",
            ))
        }
    };
    // §9.1.2 OrdinarySetPrototypeOf step 3: non-extensible objects cannot
    // change their prototype (unless it's the same value).
    let obj = ctx.get_object(obj_id);
    if !obj.extensible && new_proto != obj.prototype {
        return Err(VmError::type_error(
            "Cannot set prototype of a non-extensible object",
        ));
    }
    // Cycle check: walk `new_proto` chain to ensure `obj_id` is not in it.
    // Capped at 10,000 iterations to guard against corrupted state.
    if let Some(mut cursor) = new_proto {
        let mut found_end = false;
        for _ in 0..10_000 {
            if cursor == obj_id {
                return Err(VmError::type_error("Cyclic __proto__ value"));
            }
            if let Some(p) = ctx.get_object(cursor).prototype {
                cursor = p;
            } else {
                found_end = true;
                break;
            }
        }
        if !found_end {
            return Err(VmError::type_error("Cyclic __proto__ value"));
        }
    }
    ctx.get_object_mut(obj_id).prototype = new_proto;
    Ok(obj_val)
}

/// `Object.prototype.hasOwnProperty(prop)` — ES2020 §19.1.3.2
pub(crate) fn native_object_has_own_property(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_id = super::super::coerce::to_object(ctx.vm, this)?;
    let prop = args.first().copied().unwrap_or(JsValue::Undefined);
    // DOMStringMap (HTMLElement.dataset) named-property exotic
    // [[GetOwnProperty]] — supported names ARE own properties at
    // the WebIDL level, so `el.dataset.hasOwnProperty('fooBar')`
    // must reflect `data-foo-bar` presence.  Pre-coercion (we
    // pass the raw `prop` JsValue so Symbol keys fall through to
    // the ordinary `to_property_key` path below).
    #[cfg(feature = "engine")]
    if matches!(ctx.get_object(obj_id).kind, ObjectKind::DOMStringMap { .. }) {
        if let Some(result) = super::super::host::dataset::try_has(ctx.vm, obj_id, prop) {
            return result.map(JsValue::Boolean);
        }
    }
    // Storage `hasOwnProperty` — stored keys are own properties at
    // the WebIDL level.
    #[cfg(feature = "engine")]
    if matches!(ctx.get_object(obj_id).kind, ObjectKind::Storage { .. }) {
        if let Some(result) = super::super::host::storage::try_has(ctx.vm, obj_id, prop) {
            return result.map(JsValue::Boolean);
        }
    }
    let key = to_property_key(ctx, prop)?;
    // Check storage first
    if ctx.get_object(obj_id).storage.has(key, &ctx.vm.shapes) {
        return Ok(JsValue::Boolean(true));
    }
    // StringWrapper: virtual index properties + "length"
    if let ObjectKind::StringWrapper(sid) = ctx.get_object(obj_id).kind {
        if let PropertyKey::String(key_sid) = key {
            if key_sid == ctx.vm.well_known.length {
                return Ok(JsValue::Boolean(true));
            }
            let key_units = ctx.vm.strings.get(key_sid);
            if let Some(idx) = parse_array_index_u32(key_units) {
                let str_len = ctx.vm.strings.get(sid).len();
                if (idx as usize) < str_len {
                    return Ok(JsValue::Boolean(true));
                }
            }
        }
    }
    Ok(JsValue::Boolean(false))
}

/// `Object.prototype.valueOf()` — ES2020 §19.1.3.7: return ToObject(this).
pub(crate) fn native_object_value_of(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_id = super::super::coerce::to_object(ctx.vm, this)?;
    Ok(JsValue::Object(obj_id))
}

/// `Object.prototype.isPrototypeOf(obj)` — ES2020 §19.1.3.4
pub(crate) fn native_object_is_prototype_of(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let v = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(mut current_id) = v else {
        return Ok(JsValue::Boolean(false));
    };
    let JsValue::Object(proto_id) = this else {
        return Ok(JsValue::Boolean(false));
    };
    // Walk the prototype chain of `v` looking for `this`.
    // Cap at 10,000 iterations to guard against cyclic chains.
    for _ in 0..10_000 {
        let obj = ctx.get_object(current_id);
        match obj.prototype {
            Some(parent) => {
                if parent == proto_id {
                    return Ok(JsValue::Boolean(true));
                }
                current_id = parent;
            }
            None => return Ok(JsValue::Boolean(false)),
        }
    }
    Ok(JsValue::Boolean(false))
}

/// `Object.prototype.propertyIsEnumerable(prop)` — ES2020 §19.1.3.5
pub(crate) fn native_object_property_is_enumerable(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(obj_id) = this else {
        return Ok(JsValue::Boolean(false));
    };
    let prop = args.first().copied().unwrap_or(JsValue::Undefined);
    let key = to_property_key(ctx, prop)?;
    let result = ctx
        .get_object(obj_id)
        .storage
        .get(key, &ctx.vm.shapes)
        .is_some_and(|(_, attrs)| attrs.enumerable);
    Ok(JsValue::Boolean(result))
}
