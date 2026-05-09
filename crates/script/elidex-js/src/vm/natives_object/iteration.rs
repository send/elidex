//! Object iteration / transfer methods — keys, values, entries, assign,
//! fromEntries.

use super::super::coerce_format::{collect_own_keys_es_order, parse_array_index_u32};
use super::super::natives_array::create_array;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, VmError,
};
use super::{to_object_arg, to_property_key};

pub(in super::super) fn native_object_keys(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_id = to_object_arg(ctx, args)?;
    let keys: Vec<JsValue> = collect_own_keys_es_order(ctx.vm, obj_id)?
        .into_iter()
        .map(JsValue::String)
        .collect();
    Ok(create_array(ctx, keys))
}

pub(in super::super) fn native_object_values(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_id = to_object_arg(ctx, args)?;
    // §7.3.21 EnumerableOwnPropertyNames in ES key order, then Get per key.
    let keys = collect_own_keys_es_order(ctx.vm, obj_id)?;
    let mut values = Vec::with_capacity(keys.len());
    for sid in &keys {
        values.push(ctx.get_property_value(obj_id, PropertyKey::String(*sid))?);
    }
    Ok(create_array(ctx, values))
}

pub(in super::super) fn native_object_assign(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let target = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(target_id) = target else {
        return Ok(target);
    };
    let is_global = target_id == ctx.vm.global_object;
    for &source in args.iter().skip(1) {
        let JsValue::Object(src_id) = source else {
            continue;
        };
        // §19.1.2.1: OwnPropertyKeys in ES order, then Get per key.
        // Array element indices (ascending) come before named properties.
        let keys: Vec<PropertyKey> = {
            let elem_indices: Vec<usize> = match &ctx.get_object(src_id).kind {
                ObjectKind::Array { ref elements } => elements
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| !e.is_empty())
                    .map(|(i, _)| i)
                    .collect(),
                _ => Vec::new(),
            };
            let mut ks = Vec::new();
            for i in elem_indices {
                let sid = ctx.vm.strings.intern(&i.to_string());
                ks.push(PropertyKey::String(sid));
            }
            // Named properties: partition numeric-index vs other, sort indices.
            let obj = ctx.get_object(src_id);
            let mut idx_keys: Vec<(u32, PropertyKey)> = Vec::new();
            let mut other_keys: Vec<PropertyKey> = Vec::new();
            let mut sym_keys: Vec<PropertyKey> = Vec::new();
            for (k, attrs) in obj.storage.iter_keys(&ctx.vm.shapes) {
                if !attrs.enumerable {
                    continue;
                }
                match k {
                    PropertyKey::Symbol(_) => sym_keys.push(k),
                    PropertyKey::String(sid) => {
                        let units = ctx.vm.strings.get(sid);
                        if let Some(idx) = parse_array_index_u32(units) {
                            idx_keys.push((idx, k));
                        } else {
                            other_keys.push(k);
                        }
                    }
                }
            }
            idx_keys.sort_by_key(|(idx, _)| *idx);
            ks.extend(idx_keys.into_iter().map(|(_, k)| k));
            ks.extend(other_keys);
            ks.extend(sym_keys);
            ks
        };
        let target_is_array = matches!(ctx.get_object(target_id).kind, ObjectKind::Array { .. });
        for key in keys {
            let value = ctx.get_property_value(src_id, key)?;
            // §19.1.2.1 step 5.c.iii.2: Set(O, nextKey, propValue, true).
            // Throw TypeError if target is non-extensible and key is new.
            let target_obj = ctx.get_object(target_id);
            if !target_obj.extensible && !target_obj.storage.has(key, &ctx.vm.shapes) {
                return Err(VmError::type_error(
                    "Cannot add property to a non-extensible object",
                ));
            }
            // Route numeric index keys on Array targets through set_element
            // so elements storage and length stay coherent.
            if target_is_array {
                if let PropertyKey::String(sid) = key {
                    let units = ctx.vm.strings.get(sid);
                    if let Some(idx) = parse_array_index_u32(units) {
                        ctx.vm.set_element(
                            JsValue::Object(target_id),
                            JsValue::Number(f64::from(idx)),
                            value,
                        )?;
                        continue;
                    }
                }
            }
            if is_global {
                if let PropertyKey::String(sid) = key {
                    ctx.vm.globals.insert(sid, value);
                }
            }
            ctx.vm.upsert_data_property(
                target_id,
                key,
                value,
                super::super::shape::PropertyAttrs::DATA,
            );
        }
    }
    Ok(target)
}

/// `Object.entries(obj)` — ES2020 §19.1.2.5
pub(in super::super) fn native_object_entries(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_id = to_object_arg(ctx, args)?;
    let keys = collect_own_keys_es_order(ctx.vm, obj_id)?;
    let mut entries = Vec::with_capacity(keys.len());
    for sid in &keys {
        let val = ctx.get_property_value(obj_id, PropertyKey::String(*sid))?;
        let pair = create_array(ctx, vec![JsValue::String(*sid), val]);
        entries.push(pair);
    }
    Ok(create_array(ctx, entries))
}

/// `Object.fromEntries(iterable)` — ES2020 §19.1.2.7
pub(in super::super) fn native_object_from_entries(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let iterable = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(iter_id) = iterable else {
        return Err(VmError::type_error(
            "Object.fromEntries requires an iterable",
        ));
    };
    let obj_id = ctx.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: ctx.vm.object_prototype,
        extensible: true,
    });
    // Fast path: array of [key, value] pairs — index-based to avoid cloning.
    let len = match &ctx.get_object(iter_id).kind {
        ObjectKind::Array { elements } => Some(elements.len()),
        _ => None,
    };
    let Some(len) = len else {
        return Err(VmError::type_error(
            "Object.fromEntries requires an iterable",
        ));
    };
    for i in 0..len {
        let pair_val = {
            let elems = match &ctx.get_object(iter_id).kind {
                ObjectKind::Array { elements } => elements[i].or_undefined(),
                _ => JsValue::Undefined,
            };
            elems
        };
        let JsValue::Object(pair_id) = pair_val else {
            return Err(VmError::type_error(
                "Iterator value is not an entry-like object",
            ));
        };
        let pair_obj = ctx.get_object(pair_id);
        let (key_val, val_val) = match &pair_obj.kind {
            ObjectKind::Array { elements: elems } => (
                elems
                    .first()
                    .copied()
                    .unwrap_or(JsValue::Undefined)
                    .or_undefined(),
                elems
                    .get(1)
                    .copied()
                    .unwrap_or(JsValue::Undefined)
                    .or_undefined(),
            ),
            _ => {
                return Err(VmError::type_error(
                    "Iterator value is not an entry-like object",
                ))
            }
        };
        let pk = to_property_key(ctx, key_val)?;
        ctx.vm.upsert_data_property(
            obj_id,
            pk,
            val_val,
            super::super::shape::PropertyAttrs::DATA,
        );
    }
    Ok(JsValue::Object(obj_id))
}
