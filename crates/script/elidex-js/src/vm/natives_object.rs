//! Object built-in methods (ES2020 §19.1).
//!
//! Covers Object static methods (keys, values, entries, assign, create,
//! defineProperty, freeze, seal, etc.) and Object.prototype methods
//! (hasOwnProperty, valueOf, isPrototypeOf, propertyIsEnumerable).

use super::coerce_format::parse_array_index_u32;
use super::natives_array::create_array;
use super::value::{
    JsValue, NativeContext, Object, ObjectKind, Property, PropertyKey, PropertyStorage, VmError,
};

// -- Object static methods --------------------------------------------------

/// §7.1.13 ToObject on first argument — throw TypeError for null/undefined,
/// wrap primitives into wrapper objects, pass through objects.
fn to_object_arg(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
) -> Result<super::value::ObjectId, VmError> {
    let val = args.first().copied().unwrap_or(JsValue::Undefined);
    super::coerce::to_object(ctx.vm, val)
}

pub(super) fn native_object_keys(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_id = to_object_arg(ctx, args)?;
    let keys: Vec<JsValue> = super::coerce_format::collect_own_keys_es_order(ctx.vm, obj_id)?
        .into_iter()
        .map(JsValue::String)
        .collect();
    Ok(create_array(ctx, keys))
}

pub(super) fn native_object_values(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_id = to_object_arg(ctx, args)?;
    // §7.3.21 EnumerableOwnPropertyNames in ES key order, then Get per key.
    let keys = super::coerce_format::collect_own_keys_es_order(ctx.vm, obj_id)?;
    let mut values = Vec::with_capacity(keys.len());
    for sid in &keys {
        values.push(ctx.get_property_value(obj_id, PropertyKey::String(*sid))?);
    }
    Ok(create_array(ctx, values))
}

pub(super) fn native_object_assign(
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
            ctx.vm
                .upsert_data_property(target_id, key, value, super::shape::PropertyAttrs::DATA);
        }
    }
    Ok(target)
}

pub(super) fn native_object_create(
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
        storage: PropertyStorage::shaped(super::shape::ROOT_SHAPE),
        prototype,
        extensible: true,
    });
    Ok(JsValue::Object(obj_id))
}

#[allow(clippy::too_many_lines)]
pub(super) fn native_object_define_property(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let prop_val = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let desc_val = args.get(2).copied().unwrap_or(JsValue::Undefined);

    let JsValue::Object(obj_id) = obj_val else {
        return Err(VmError::type_error(
            "Object.defineProperty called on non-object",
        ));
    };
    let key = match prop_val {
        JsValue::Symbol(sid) => PropertyKey::Symbol(sid),
        other => PropertyKey::String(ctx.to_string_val(other)?),
    };

    // Extract descriptor fields.
    let new_prop = if let JsValue::Object(desc_id) = desc_val {
        let get_key = PropertyKey::String(ctx.vm.well_known.get);
        let set_key = PropertyKey::String(ctx.vm.well_known.set);
        let value_key = PropertyKey::String(ctx.vm.well_known.value);
        let enumerable_key = PropertyKey::String(ctx.vm.well_known.enumerable);
        let configurable_key = PropertyKey::String(ctx.vm.well_known.configurable);
        let writable_key = PropertyKey::String(ctx.vm.well_known.writable);

        // §6.2.5.5 ToPropertyDescriptor: HasProperty + Get per field in
        // spec order (enumerable, configurable, value, writable, get, set).
        // No up-front snapshot — each Get sees the current state, including
        // mutations from earlier getters and inherited fields.
        let has_enumerable = ctx.try_get_property_value(desc_id, enumerable_key)?;
        let has_configurable = ctx.try_get_property_value(desc_id, configurable_key)?;
        let has_value = ctx.try_get_property_value(desc_id, value_key)?;
        let has_writable = ctx.try_get_property_value(desc_id, writable_key)?;
        let has_get = ctx.try_get_property_value(desc_id, get_key)?;
        let has_set = ctx.try_get_property_value(desc_id, set_key)?;
        // ToBoolean coercion for boolean descriptor fields (§6.2.5.1).
        let enumerable = has_enumerable.map(|v| super::coerce::to_boolean(ctx.vm, v));
        let configurable = has_configurable.map(|v| super::coerce::to_boolean(ctx.vm, v));

        let has_accessor = has_get.is_some() || has_set.is_some();
        let has_data = has_value.is_some() || has_writable.is_some();

        // §9.1.6.3 step 2: mixing accessor and data fields is a TypeError.
        if has_accessor && has_data {
            return Err(VmError::type_error(
                "Invalid property descriptor. Cannot both specify accessors and a value or writable attribute",
            ));
        }

        // §9.1.6.3 step 7-8: get/set must be callable or undefined.
        let validate_accessor =
            |v: JsValue, role: &str| -> Result<Option<super::value::ObjectId>, VmError> {
                match v {
                    JsValue::Undefined => Ok(None),
                    JsValue::Object(id) => {
                        if ctx.get_object(id).kind.is_callable() {
                            Ok(Some(id))
                        } else {
                            Err(VmError::type_error(format!(
                                "Property descriptor {role} must be a function or undefined"
                            )))
                        }
                    }
                    _ => Err(VmError::type_error(format!(
                        "Property descriptor {role} must be a function or undefined"
                    ))),
                }
            };

        // Look up the existing property to merge partial descriptors.
        let existing = ctx
            .get_object(obj_id)
            .storage
            .get(key, &ctx.vm.shapes)
            .map(|(pv, attrs)| Property::from_attrs(*pv, attrs));

        if has_accessor {
            let getter = match has_get {
                Some(v) => validate_accessor(v, "get")?,
                None => existing.and_then(|p| match p.slot {
                    super::value::PropertyValue::Accessor { getter, .. } => getter,
                    super::value::PropertyValue::Data(_) => None,
                }),
            };
            let setter = match has_set {
                Some(v) => validate_accessor(v, "set")?,
                None => existing.and_then(|p| match p.slot {
                    super::value::PropertyValue::Accessor { setter, .. } => setter,
                    super::value::PropertyValue::Data(_) => None,
                }),
            };
            Property {
                slot: super::value::PropertyValue::Accessor { getter, setter },
                writable: false,
                enumerable: enumerable.unwrap_or_else(|| existing.is_some_and(|p| p.enumerable)),
                configurable: configurable
                    .unwrap_or_else(|| existing.is_some_and(|p| p.configurable)),
            }
        } else {
            let value = has_value
                .unwrap_or_else(|| existing.map_or(JsValue::Undefined, |p| p.data_value()));
            let writable = has_writable.map_or_else(
                || existing.is_some_and(|p| p.writable),
                |v| super::coerce::to_boolean(ctx.vm, v),
            );
            Property {
                slot: super::value::PropertyValue::Data(value),
                writable,
                enumerable: enumerable.unwrap_or_else(|| existing.is_some_and(|p| p.enumerable)),
                configurable: configurable
                    .unwrap_or_else(|| existing.is_some_and(|p| p.configurable)),
            }
        }
    } else {
        return Err(VmError::type_error(
            "Property description must be an object",
        ));
    };

    // §9.1.6.3: Validate attribute changes against existing non-configurable property.
    if let Some(existing) = ctx
        .get_object(obj_id)
        .storage
        .get(key, &ctx.vm.shapes)
        .map(|(pv, attrs)| Property::from_attrs(*pv, attrs))
    {
        if !existing.configurable {
            // Cannot change configurable from false to true.
            if new_prop.configurable {
                return Err(VmError::type_error(
                    "Cannot redefine property: configurable is false",
                ));
            }
            // Cannot change enumerable on non-configurable property.
            if new_prop.enumerable != existing.enumerable {
                return Err(VmError::type_error(
                    "Cannot redefine property: cannot change enumerable",
                ));
            }
            // Cannot change from data to accessor or vice versa.
            let existing_is_accessor =
                matches!(existing.slot, super::value::PropertyValue::Accessor { .. });
            let new_is_accessor =
                matches!(new_prop.slot, super::value::PropertyValue::Accessor { .. });
            if existing_is_accessor != new_is_accessor {
                return Err(VmError::type_error(
                    "Cannot redefine property: cannot convert between data and accessor",
                ));
            }
            match (existing.slot, new_prop.slot) {
                // Non-configurable data: cannot change writable false→true
                // or value if non-writable.
                (
                    super::value::PropertyValue::Data(existing_val),
                    super::value::PropertyValue::Data(new_val),
                ) => {
                    if !existing.writable && new_prop.writable {
                        return Err(VmError::type_error(
                            "Cannot redefine property: cannot make non-writable property writable",
                        ));
                    }
                    if !existing.writable && !super::value::same_value(existing_val, new_val) {
                        return Err(VmError::type_error(
                            "Cannot redefine property: cannot change value of non-writable, non-configurable property",
                        ));
                    }
                }
                // Non-configurable accessor: cannot change getter or setter
                // unless SameValue (§9.1.6.3 step 11).
                (
                    super::value::PropertyValue::Accessor {
                        getter: eg,
                        setter: es,
                    },
                    super::value::PropertyValue::Accessor {
                        getter: ng,
                        setter: ns,
                    },
                ) => {
                    let obj_or_undef = |o: Option<super::value::ObjectId>| {
                        o.map_or(JsValue::Undefined, JsValue::Object)
                    };
                    if !super::value::same_value(obj_or_undef(eg), obj_or_undef(ng)) {
                        return Err(VmError::type_error(
                            "Cannot redefine property: cannot change getter of non-configurable accessor",
                        ));
                    }
                    if !super::value::same_value(obj_or_undef(es), obj_or_undef(ns)) {
                        return Err(VmError::type_error(
                            "Cannot redefine property: cannot change setter of non-configurable accessor",
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    // Sync globals HashMap: data properties are cached for GetGlobal fast path;
    // accessor properties remove any stale data entry so GetGlobal falls back
    // to the global object.
    if obj_id == ctx.vm.global_object {
        if let PropertyKey::String(sid) = key {
            match new_prop.slot {
                super::value::PropertyValue::Data(v) => {
                    ctx.vm.globals.insert(sid, v);
                }
                super::value::PropertyValue::Accessor { .. } => {
                    ctx.vm.globals.remove(&sid);
                }
            }
        }
    }
    // Write the property using shape transitions (preserves IC caching).
    let new_attrs = super::shape::PropertyAttrs {
        writable: new_prop.writable,
        enumerable: new_prop.enumerable,
        configurable: new_prop.configurable,
        is_accessor: matches!(new_prop.slot, super::value::PropertyValue::Accessor { .. }),
    };
    let existing_info = {
        let obj = ctx.vm.objects[obj_id.0 as usize].as_ref().unwrap();
        obj.storage.get(key, &ctx.vm.shapes).map(|(_, attrs)| attrs)
    };
    if let Some(existing_attrs) = existing_info {
        if existing_attrs == new_attrs {
            // Attrs unchanged — just write the slot value.
            let obj = ctx.vm.objects[obj_id.0 as usize].as_mut().unwrap();
            if let Some((slot, _)) = obj.storage.get_mut(key, &ctx.vm.shapes) {
                *slot = new_prop.slot;
            }
        } else {
            // Attrs changed — reconfigure transition + slot write.
            ctx.vm
                .reconfigure_property(obj_id, key, new_attrs, Some(new_prop.slot));
            // Dictionary mode: reconfigure_property is a no-op, update in place.
            let obj = ctx.vm.objects[obj_id.0 as usize].as_mut().unwrap();
            if let super::value::PropertyStorage::Dictionary(vec) = &mut obj.storage {
                if let Some((_, p)) = vec.iter_mut().find(|(k, _)| *k == key) {
                    *p = new_prop;
                }
            }
        }
    } else {
        // New property — reject if non-extensible (§9.1.6.3 step 2.a).
        if !ctx.get_object(obj_id).extensible {
            return Err(VmError::type_error(
                "Cannot define property on a non-extensible object",
            ));
        }
        ctx.vm
            .define_shaped_property(obj_id, key, new_prop.slot, new_attrs);
    }
    Ok(obj_val)
}

// -- Object.getOwnPropertySymbols ---------------------------------------------

pub(super) fn native_object_get_own_property_symbols(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_id = to_object_arg(ctx, args)?;
    let syms: Vec<JsValue> = ctx
        .get_object(obj_id)
        .storage
        .iter_keys(&ctx.vm.shapes)
        .filter_map(|(k, _)| {
            if let PropertyKey::Symbol(sid) = k {
                Some(JsValue::Symbol(sid))
            } else {
                None
            }
        })
        .collect();
    Ok(create_array(ctx, syms))
}

// -- Object.entries / is / getPrototypeOf / setPrototypeOf / descriptors ------

/// `Object.entries(obj)` — ES2020 §19.1.2.5
pub(super) fn native_object_entries(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_id = to_object_arg(ctx, args)?;
    let keys = super::coerce_format::collect_own_keys_es_order(ctx.vm, obj_id)?;
    let mut entries = Vec::with_capacity(keys.len());
    for sid in &keys {
        let val = ctx.get_property_value(obj_id, PropertyKey::String(*sid))?;
        let pair = create_array(ctx, vec![JsValue::String(*sid), val]);
        entries.push(pair);
    }
    Ok(create_array(ctx, entries))
}

/// `Object.is(a, b)` — ES2020 §19.1.2.10
pub(super) fn native_object_is(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let a = args.first().copied().unwrap_or(JsValue::Undefined);
    let b = args.get(1).copied().unwrap_or(JsValue::Undefined);
    Ok(JsValue::Boolean(super::value::same_value(a, b)))
}

/// `Object.getPrototypeOf(obj)` — ES2020 §19.1.2.9
pub(super) fn native_object_get_prototype_of(
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
pub(super) fn native_object_set_prototype_of(
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

/// Build a plain `{value, writable, enumerable, configurable}`
/// descriptor object for a data property — shared between the
/// ordinary `[[GetOwnProperty]]` path below and the WebIDL named-
/// property exotic branches (currently DOMStringMap supported
/// names; other exotics can route through this helper as they
/// land).
#[cfg(feature = "engine")]
fn build_data_descriptor(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    writable: bool,
    enumerable: bool,
    configurable: bool,
) -> super::value::ObjectId {
    let desc_id = ctx.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(super::shape::ROOT_SHAPE),
        prototype: ctx.vm.object_prototype,
        extensible: true,
    });
    let value_key = PropertyKey::String(ctx.vm.well_known.value);
    let writable_key = PropertyKey::String(ctx.vm.well_known.writable);
    let enumerable_key = PropertyKey::String(ctx.vm.well_known.enumerable);
    let configurable_key = PropertyKey::String(ctx.vm.well_known.configurable);
    for (key, val) in [
        (value_key, value),
        (writable_key, JsValue::Boolean(writable)),
        (enumerable_key, JsValue::Boolean(enumerable)),
        (configurable_key, JsValue::Boolean(configurable)),
    ] {
        ctx.vm.define_shaped_property(
            desc_id,
            key,
            super::value::PropertyValue::Data(val),
            super::shape::PropertyAttrs::DATA,
        );
    }
    desc_id
}

/// `Object.getOwnPropertyDescriptor(obj, prop)` — ES2020 §19.1.2.6
pub(super) fn native_object_get_own_property_descriptor(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_id = to_object_arg(ctx, args)?;
    let prop = args.get(1).copied().unwrap_or(JsValue::Undefined);
    // DOMStringMap (HTMLElement.dataset) named-property exotic
    // [[GetOwnProperty]] — supported names ARE own properties at
    // the WebIDL level (writable, enumerable, configurable), so
    // `Object.getOwnPropertyDescriptor(dataset, 'fooBar')` must
    // return a real data descriptor when `data-foo-bar` is set.
    // Without this branch, we'd return `undefined` (sealed wrapper
    // has no entries in `storage`), which breaks Reflect-style
    // introspection and structuredClone-of-descriptors.
    #[cfg(feature = "engine")]
    if matches!(ctx.get_object(obj_id).kind, ObjectKind::DOMStringMap { .. }) {
        if let Some(result) = super::host::dataset::try_get(ctx.vm, obj_id, prop) {
            let value = result?;
            return Ok(JsValue::Object(build_data_descriptor(
                ctx, value, true, true, true,
            )));
        }
    }
    // Storage named-property exotic [[GetOwnProperty]] — supported
    // names are own properties (writable/enumerable/configurable per
    // WebIDL §3.10) so `Object.getOwnPropertyDescriptor(localStorage,
    // 'k')` reflects the stored entry.
    #[cfg(feature = "engine")]
    if matches!(ctx.get_object(obj_id).kind, ObjectKind::Storage { .. }) {
        if let Some(result) = super::host::storage::try_get(ctx.vm, obj_id, prop) {
            let value = result?;
            return Ok(JsValue::Object(build_data_descriptor(
                ctx, value, true, true, true,
            )));
        }
    }
    let key = to_property_key(ctx, prop)?;
    let result = ctx.get_object(obj_id).storage.get(key, &ctx.vm.shapes);
    let Some((slot, attrs)) = result else {
        return Ok(JsValue::Undefined);
    };
    // Build the descriptor object.
    let slot = *slot;
    let desc_id = ctx.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(super::shape::ROOT_SHAPE),
        prototype: ctx.vm.object_prototype,
        extensible: true,
    });
    let configurable_key = PropertyKey::String(ctx.vm.well_known.configurable);
    let enumerable_key = PropertyKey::String(ctx.vm.well_known.enumerable);
    match slot {
        super::value::PropertyValue::Data(v) => {
            let value_key = PropertyKey::String(ctx.vm.well_known.value);
            let writable_key = PropertyKey::String(ctx.vm.well_known.writable);
            ctx.vm.define_shaped_property(
                desc_id,
                value_key,
                super::value::PropertyValue::Data(v),
                super::shape::PropertyAttrs::DATA,
            );
            ctx.vm.define_shaped_property(
                desc_id,
                writable_key,
                super::value::PropertyValue::Data(JsValue::Boolean(attrs.writable)),
                super::shape::PropertyAttrs::DATA,
            );
        }
        super::value::PropertyValue::Accessor { getter, setter } => {
            let get_key = PropertyKey::String(ctx.vm.well_known.get);
            let set_key = PropertyKey::String(ctx.vm.well_known.set);
            let get_val = getter.map_or(JsValue::Undefined, JsValue::Object);
            let set_val = setter.map_or(JsValue::Undefined, JsValue::Object);
            ctx.vm.define_shaped_property(
                desc_id,
                get_key,
                super::value::PropertyValue::Data(get_val),
                super::shape::PropertyAttrs::DATA,
            );
            ctx.vm.define_shaped_property(
                desc_id,
                set_key,
                super::value::PropertyValue::Data(set_val),
                super::shape::PropertyAttrs::DATA,
            );
        }
    }
    ctx.vm.define_shaped_property(
        desc_id,
        enumerable_key,
        super::value::PropertyValue::Data(JsValue::Boolean(attrs.enumerable)),
        super::shape::PropertyAttrs::DATA,
    );
    ctx.vm.define_shaped_property(
        desc_id,
        configurable_key,
        super::value::PropertyValue::Data(JsValue::Boolean(attrs.configurable)),
        super::shape::PropertyAttrs::DATA,
    );
    Ok(JsValue::Object(desc_id))
}

/// `Object.getOwnPropertyNames(obj)` — ES2020 §19.1.2.8
pub(super) fn native_object_get_own_property_names(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_id = to_object_arg(ctx, args)?;
    // §9.1.11.1 OrdinaryOwnPropertyKeys: array element indices (ascending),
    // then named property indices (ascending), then other string keys
    // (insertion order). Non-string keys (symbols) are excluded.
    let elem_indices: Vec<usize> = match &ctx.get_object(obj_id).kind {
        ObjectKind::Array { elements } => elements
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.is_empty())
            .map(|(i, _)| i)
            .collect(),
        _ => Vec::new(),
    };
    // Collect named properties, partitioning numeric-index keys from others.
    let obj = ctx.get_object(obj_id);
    let mut index_keys: Vec<(u32, super::value::StringId)> = Vec::new();
    let mut other_keys: Vec<super::value::StringId> = Vec::new();
    for (k, _) in obj.storage.iter_keys(&ctx.vm.shapes) {
        if let PropertyKey::String(sid) = k {
            let units = ctx.vm.strings.get(sid);
            if let Some(idx) = parse_array_index_u32(units) {
                index_keys.push((idx, sid));
            } else {
                other_keys.push(sid);
            }
        }
    }
    index_keys.sort_by_key(|(idx, _)| *idx);

    let mut names = Vec::new();
    // 1. Array element indices (ascending).
    for i in elem_indices {
        let sid = ctx.vm.strings.intern(&i.to_string());
        names.push(JsValue::String(sid));
    }
    // 2. Named numeric-index keys (ascending).
    for (_, sid) in index_keys {
        names.push(JsValue::String(sid));
    }
    // 3. Other string keys (insertion order).
    for sid in other_keys {
        names.push(JsValue::String(sid));
    }
    Ok(create_array(ctx, names))
}

/// `Object.fromEntries(iterable)` — ES2019 §22.1.2.1
pub(super) fn native_object_from_entries(
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
        storage: PropertyStorage::shaped(super::shape::ROOT_SHAPE),
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
        ctx.vm
            .upsert_data_property(obj_id, pk, val_val, super::shape::PropertyAttrs::DATA);
    }
    Ok(JsValue::Object(obj_id))
}

// -- Object.freeze/seal/preventExtensions --------------------------------------

/// Mark an object non-extensible and collect its property keys+attrs.
/// Shared by `freeze` and `seal`.
fn lock_and_collect_keys(
    ctx: &mut NativeContext<'_>,
    obj_id: super::value::ObjectId,
) -> Vec<(PropertyKey, super::shape::PropertyAttrs)> {
    ctx.get_object_mut(obj_id).extensible = false;
    ctx.get_object(obj_id)
        .storage
        .iter_keys(&ctx.vm.shapes)
        .collect()
}

/// `Object.freeze(obj)` — ES2020 §19.1.2.6
pub(super) fn native_object_freeze(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_val = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_id) = obj_val else {
        return Ok(obj_val);
    };
    for (key, attrs) in lock_and_collect_keys(ctx, obj_id) {
        let new_attrs = super::shape::PropertyAttrs {
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

/// `Object.seal(obj)` — ES2020 §19.1.2.20
pub(super) fn native_object_seal(
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
            let new_attrs = super::shape::PropertyAttrs {
                configurable: false,
                ..attrs
            };
            ctx.vm.reconfigure_property(obj_id, key, new_attrs, None);
        }
    }
    Ok(obj_val)
}

/// `Object.preventExtensions(obj)` — ES2020 §19.1.2.18
pub(super) fn native_object_prevent_extensions(
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

/// `Object.isFrozen(obj)` — ES2020 §19.1.2.13
pub(super) fn native_object_is_frozen(
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

/// `Object.isSealed(obj)` — ES2020 §19.1.2.14
pub(super) fn native_object_is_sealed(
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

/// `Object.isExtensible(obj)` — ES2020 §19.1.2.11
pub(super) fn native_object_is_extensible(
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

// -- Object.prototype methods -------------------------------------------------

/// Convert a JS value to a `PropertyKey` (ES2020 §7.1.14 ToPropertyKey).
fn to_property_key(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<PropertyKey, VmError> {
    if let JsValue::Symbol(s) = val {
        return Ok(PropertyKey::Symbol(s));
    }
    let sid = ctx.to_string_val(val)?;
    Ok(PropertyKey::String(sid))
}

/// `Object.prototype.hasOwnProperty(prop)` — ES2020 §19.1.3.2
pub(super) fn native_object_has_own_property(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_id = super::coerce::to_object(ctx.vm, this)?;
    let prop = args.first().copied().unwrap_or(JsValue::Undefined);
    // DOMStringMap (HTMLElement.dataset) named-property exotic
    // [[GetOwnProperty]] — supported names ARE own properties at
    // the WebIDL level, so `el.dataset.hasOwnProperty('fooBar')`
    // must reflect `data-foo-bar` presence.  Pre-coercion (we
    // pass the raw `prop` JsValue so Symbol keys fall through to
    // the ordinary `to_property_key` path below).
    #[cfg(feature = "engine")]
    if matches!(ctx.get_object(obj_id).kind, ObjectKind::DOMStringMap { .. }) {
        if let Some(result) = super::host::dataset::try_has(ctx.vm, obj_id, prop) {
            return result.map(JsValue::Boolean);
        }
    }
    // Storage `hasOwnProperty` — stored keys are own properties at
    // the WebIDL level.
    #[cfg(feature = "engine")]
    if matches!(ctx.get_object(obj_id).kind, ObjectKind::Storage { .. }) {
        if let Some(result) = super::host::storage::try_has(ctx.vm, obj_id, prop) {
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
            if let Some(idx) = super::coerce_format::parse_array_index_u32(key_units) {
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
pub(super) fn native_object_value_of(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_id = super::coerce::to_object(ctx.vm, this)?;
    Ok(JsValue::Object(obj_id))
}

/// `Object.prototype.isPrototypeOf(obj)` — ES2020 §19.1.3.4
pub(super) fn native_object_is_prototype_of(
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
pub(super) fn native_object_property_is_enumerable(
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
