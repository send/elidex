//! IDBKeyRange (W3C IndexedDB §4.7) — static constructors + `includes` +
//! the `lower` / `upper` / `lowerOpen` / `upperOpen` accessors.  The range
//! algorithm (comparison, SQL clause) lives in the backend `key_range.rs`.

#![cfg(feature = "engine")]

use super::super::super::shape;
use super::super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage, VmError,
};
use super::super::super::VmInner;
use super::value;

/// Allocate an `IDBKeyRange` wrapper holding the backend range value.
fn create_key_range(vm: &mut VmInner, range: elidex_indexeddb::IdbKeyRange) -> ObjectId {
    let id = vm.alloc_object(Object {
        kind: ObjectKind::IdbKeyRange,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.idb_key_range_prototype,
        extensible: true,
    });
    vm.idb_key_range_states.insert(id, range);
    id
}

fn require_key_range_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    member: &str,
) -> Result<ObjectId, VmError> {
    if let JsValue::Object(id) = this {
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::IdbKeyRange) {
            return Ok(id);
        }
    }
    Err(VmError::type_error(format!(
        "IDBKeyRange.prototype.{member} called on non-IDBKeyRange"
    )))
}

/// `IDBKeyRange.only(value)` (§4.7).
pub(crate) fn native_key_range_only(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let arg = value::require_arg(args, 0, "IDBKeyRange", "only", 1)?;
    let key = value::js_to_idb_key(ctx, arg)?;
    Ok(JsValue::Object(create_key_range(
        ctx.vm,
        elidex_indexeddb::IdbKeyRange::only(key),
    )))
}

/// `IDBKeyRange.lowerBound(lower, open?)` (§4.7).
pub(crate) fn native_key_range_lower_bound(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let arg = value::require_arg(args, 0, "IDBKeyRange", "lowerBound", 1)?;
    let key = value::js_to_idb_key(ctx, arg)?;
    let open = ctx.to_boolean(args.get(1).copied().unwrap_or(JsValue::Undefined));
    Ok(JsValue::Object(create_key_range(
        ctx.vm,
        elidex_indexeddb::IdbKeyRange::lower_bound(key, open),
    )))
}

/// `IDBKeyRange.upperBound(upper, open?)` (§4.7).
pub(crate) fn native_key_range_upper_bound(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let arg = value::require_arg(args, 0, "IDBKeyRange", "upperBound", 1)?;
    let key = value::js_to_idb_key(ctx, arg)?;
    let open = ctx.to_boolean(args.get(1).copied().unwrap_or(JsValue::Undefined));
    Ok(JsValue::Object(create_key_range(
        ctx.vm,
        elidex_indexeddb::IdbKeyRange::upper_bound(key, open),
    )))
}

/// `IDBKeyRange.bound(lower, upper, lowerOpen?, upperOpen?)` (§4.7).
/// `DataError` when `lower > upper` (or equal with an open bound).
pub(crate) fn native_key_range_bound(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let lower_arg = value::require_arg(args, 0, "IDBKeyRange", "bound", 2)?;
    let upper_arg = value::require_arg(args, 1, "IDBKeyRange", "bound", 2)?;
    let lower = value::js_to_idb_key(ctx, lower_arg)?;
    let upper = value::js_to_idb_key(ctx, upper_arg)?;
    let lower_open = ctx.to_boolean(args.get(2).copied().unwrap_or(JsValue::Undefined));
    let upper_open = ctx.to_boolean(args.get(3).copied().unwrap_or(JsValue::Undefined));
    match elidex_indexeddb::IdbKeyRange::bound(lower, upper, lower_open, upper_open) {
        Some(range) => Ok(JsValue::Object(create_key_range(ctx.vm, range))),
        None => Err(value::dom_exc(
            ctx,
            "DataError",
            "IDBKeyRange.bound: lower must be less than or equal to upper",
        )),
    }
}

/// `range.includes(key)` (§4.7).
pub(crate) fn native_key_range_includes(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_key_range_this(ctx, this, "includes")?;
    let arg = value::require_arg(args, 0, "IDBKeyRange", "includes", 1)?;
    let key = value::js_to_idb_key(ctx, arg)?;
    let included = ctx
        .vm
        .idb_key_range_states
        .get(&id)
        .is_some_and(|r| r.includes(&key));
    Ok(JsValue::Boolean(included))
}

/// `range.lower` accessor.
pub(crate) fn native_key_range_get_lower(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_key_range_this(ctx, this, "lower")?;
    let lower = ctx
        .vm
        .idb_key_range_states
        .get(&id)
        .and_then(|r| r.lower.clone());
    Ok(lower.map_or(JsValue::Undefined, |k| value::idb_key_to_js(ctx.vm, &k)))
}

/// `range.upper` accessor.
pub(crate) fn native_key_range_get_upper(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_key_range_this(ctx, this, "upper")?;
    let upper = ctx
        .vm
        .idb_key_range_states
        .get(&id)
        .and_then(|r| r.upper.clone());
    Ok(upper.map_or(JsValue::Undefined, |k| value::idb_key_to_js(ctx.vm, &k)))
}

/// `range.lowerOpen` accessor.
pub(crate) fn native_key_range_get_lower_open(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_key_range_this(ctx, this, "lowerOpen")?;
    let open = ctx
        .vm
        .idb_key_range_states
        .get(&id)
        .is_some_and(|r| r.lower_open);
    Ok(JsValue::Boolean(open))
}

/// `range.upperOpen` accessor.
pub(crate) fn native_key_range_get_upper_open(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_key_range_this(ctx, this, "upperOpen")?;
    let open = ctx
        .vm
        .idb_key_range_states
        .get(&id)
        .is_some_and(|r| r.upper_open);
    Ok(JsValue::Boolean(open))
}
