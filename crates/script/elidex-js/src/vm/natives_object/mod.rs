//! Object built-in methods (ECMA-262 §20.1).
//!
//! Covers Object static methods (keys, values, entries, assign, create,
//! defineProperty, freeze, seal, etc.) and Object.prototype methods
//! (hasOwnProperty, valueOf, isPrototypeOf, propertyIsEnumerable).
//!
//! Split into category sibling modules:
//! - [`descriptor`] — defineProperty, getOwnPropertyDescriptor / Names / Symbols
//! - [`iteration`] — keys, values, entries, assign, fromEntries
//! - [`integrity`] — freeze, seal, preventExtensions, isFrozen, isSealed, isExtensible
//! - [`prototype`] — create, is, get/setPrototypeOf, Object.prototype methods

mod descriptor;
mod integrity;
mod iteration;
mod prototype;

use super::value::{JsValue, NativeContext, ObjectId, PropertyKey, VmError};

/// §7.1.19 ToObject on first argument — throw TypeError for null/undefined,
/// wrap primitives into wrapper objects, pass through objects.
fn to_object_arg(ctx: &mut NativeContext<'_>, args: &[JsValue]) -> Result<ObjectId, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    super::coerce::to_object(ctx.vm, val)
}

/// Convert a JS value to a `PropertyKey` (ECMA-262 §7.1.20 ToPropertyKey).
fn to_property_key(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<PropertyKey, VmError> {
    if let JsValue::Symbol(s) = val {
        return Ok(PropertyKey::Symbol(s));
    }
    let sid = ctx.to_string_val(val)?;
    Ok(PropertyKey::String(sid))
}

pub(super) use descriptor::{
    native_object_define_property, native_object_get_own_property_descriptor,
    native_object_get_own_property_names, native_object_get_own_property_symbols,
};
pub(super) use integrity::{
    native_object_freeze, native_object_is_extensible, native_object_is_frozen,
    native_object_is_sealed, native_object_prevent_extensions, native_object_seal,
};
pub(super) use iteration::{
    native_object_assign, native_object_entries, native_object_from_entries, native_object_keys,
    native_object_values,
};
pub(super) use prototype::{
    native_object_create, native_object_get_prototype_of, native_object_has_own_property,
    native_object_is, native_object_is_prototype_of, native_object_property_is_enumerable,
    native_object_set_prototype_of, native_object_value_of,
};
