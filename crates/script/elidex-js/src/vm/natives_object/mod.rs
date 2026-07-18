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

/// `Object ( value )` — ECMA-262 §20.1.1.1. The `Object` constructor: callable
/// (`Object(x)`) and constructable (`new Object(x)`, subclass `new`).
pub(crate) fn native_object_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let value = args.first().copied().unwrap_or(JsValue::Undefined);
    // Step 1: a subclass `new` (NewTarget ≠ %Object%) returns the `do_new`-provided
    // instance (which carries the subclass prototype); `value` is ignored.
    // `new_target()` is `Some` only in construct mode, so it doubles as the
    // construct gate; the `%Object%` id compare is what makes `new Object(5)`
    // (NewTarget === %Object%) fall through to step 3. `Date`/`Number` return
    // `this` the same way (`natives_date/mod.rs`, `natives_number.rs`).
    if let Some(nt) = ctx.new_target() {
        if Some(nt) != ctx.vm.object_constructor {
            return Ok(this);
        }
    }
    // Step 2: `value` is undefined or null → a fresh ordinary object with
    // %Object.prototype% (construct mode returns the `do_new` `this`, call mode
    // allocs fresh — `ensure_instance_or_alloc` folds both, as the sibling
    // Error/DOMException/… ctors do).
    if value.is_nullish() {
        return Ok(ctx
            .vm
            .ensure_instance_or_alloc(this, ctx.vm.object_prototype, ctx.mode));
    }
    // Step 3: Return ! ToObject(value) — boxes a primitive into its wrapper. The
    // `do_new` receiver is discarded here, so `new Object(5)` yields a Number
    // wrapper (§7.1.19 ToObject).
    Ok(JsValue::Object(super::coerce::to_object(ctx.vm, value)?))
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
