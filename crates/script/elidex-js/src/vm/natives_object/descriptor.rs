//! Object descriptor methods — defineProperty, getOwnPropertyDescriptor,
//! getOwnPropertyNames, getOwnPropertySymbols.

use super::super::coerce_format::parse_array_index_u32;
use super::super::natives_array::create_array;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectKind, Property, PropertyKey, PropertyStorage, VmError,
};
use super::{to_object_arg, to_property_key};

#[allow(clippy::too_many_lines)]
pub(in super::super) fn native_object_define_property(
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
        let enumerable = has_enumerable.map(|v| super::super::coerce::to_boolean(ctx.vm, v));
        let configurable = has_configurable.map(|v| super::super::coerce::to_boolean(ctx.vm, v));

        let has_accessor = has_get.is_some() || has_set.is_some();
        let has_data = has_value.is_some() || has_writable.is_some();

        // §10.1.6.3 step 2: mixing accessor and data fields is a TypeError.
        if has_accessor && has_data {
            return Err(VmError::type_error(
                "Invalid property descriptor. Cannot both specify accessors and a value or writable attribute",
            ));
        }

        // §10.1.6.3 step 7-8: get/set must be callable or undefined.
        let validate_accessor =
            |v: JsValue, role: &str| -> Result<Option<super::super::value::ObjectId>, VmError> {
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
                    super::super::value::PropertyValue::Accessor { getter, .. } => getter,
                    super::super::value::PropertyValue::Data(_) => None,
                }),
            };
            let setter = match has_set {
                Some(v) => validate_accessor(v, "set")?,
                None => existing.and_then(|p| match p.slot {
                    super::super::value::PropertyValue::Accessor { setter, .. } => setter,
                    super::super::value::PropertyValue::Data(_) => None,
                }),
            };
            Property {
                slot: super::super::value::PropertyValue::Accessor { getter, setter },
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
                |v| super::super::coerce::to_boolean(ctx.vm, v),
            );
            Property {
                slot: super::super::value::PropertyValue::Data(value),
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

    // §10.1.6.3: Validate attribute changes against existing non-configurable property.
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
            let existing_is_accessor = matches!(
                existing.slot,
                super::super::value::PropertyValue::Accessor { .. }
            );
            let new_is_accessor = matches!(
                new_prop.slot,
                super::super::value::PropertyValue::Accessor { .. }
            );
            if existing_is_accessor != new_is_accessor {
                return Err(VmError::type_error(
                    "Cannot redefine property: cannot convert between data and accessor",
                ));
            }
            match (existing.slot, new_prop.slot) {
                // Non-configurable data: cannot change writable false→true
                // or value if non-writable.
                (
                    super::super::value::PropertyValue::Data(existing_val),
                    super::super::value::PropertyValue::Data(new_val),
                ) => {
                    if !existing.writable && new_prop.writable {
                        return Err(VmError::type_error(
                            "Cannot redefine property: cannot make non-writable property writable",
                        ));
                    }
                    if !existing.writable && !super::super::value::same_value(existing_val, new_val)
                    {
                        return Err(VmError::type_error(
                            "Cannot redefine property: cannot change value of non-writable, non-configurable property",
                        ));
                    }
                }
                // Non-configurable accessor: cannot change getter or setter
                // unless SameValue (§10.1.6.3 step 11).
                (
                    super::super::value::PropertyValue::Accessor {
                        getter: eg,
                        setter: es,
                    },
                    super::super::value::PropertyValue::Accessor {
                        getter: ng,
                        setter: ns,
                    },
                ) => {
                    let obj_or_undef = |o: Option<super::super::value::ObjectId>| {
                        o.map_or(JsValue::Undefined, JsValue::Object)
                    };
                    if !super::super::value::same_value(obj_or_undef(eg), obj_or_undef(ng)) {
                        return Err(VmError::type_error(
                            "Cannot redefine property: cannot change getter of non-configurable accessor",
                        ));
                    }
                    if !super::super::value::same_value(obj_or_undef(es), obj_or_undef(ns)) {
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
                super::super::value::PropertyValue::Data(v) => {
                    ctx.vm.globals.insert(sid, v);
                }
                super::super::value::PropertyValue::Accessor { .. } => {
                    ctx.vm.globals.remove(&sid);
                }
            }
        }
    }
    // Write the property using shape transitions (preserves IC caching).
    let new_attrs = super::super::shape::PropertyAttrs {
        writable: new_prop.writable,
        enumerable: new_prop.enumerable,
        configurable: new_prop.configurable,
        is_accessor: matches!(
            new_prop.slot,
            super::super::value::PropertyValue::Accessor { .. }
        ),
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
            if let super::super::value::PropertyStorage::Dictionary(vec) = &mut obj.storage {
                if let Some((_, p)) = vec.iter_mut().find(|(k, _)| *k == key) {
                    *p = new_prop;
                }
            }
        }
    } else {
        // New property — reject if non-extensible (§10.1.6.3 step 2.a).
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

pub(in super::super) fn native_object_get_own_property_symbols(
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
) -> super::super::value::ObjectId {
    let desc_id = ctx.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: ctx.vm.object_prototype,
        extensible: true,
    });
    let value_key = PropertyKey::String(ctx.vm.well_known.value);
    let writable_key = PropertyKey::String(ctx.vm.well_known.writable);
    let enumerable_key = PropertyKey::String(ctx.vm.well_known.enumerable);
    let configurable_key = PropertyKey::String(ctx.vm.well_known.configurable);
    ctx.vm.define_shaped_property(
        desc_id,
        value_key,
        super::super::value::PropertyValue::Data(value),
        super::super::shape::PropertyAttrs::DATA,
    );
    ctx.vm.define_shaped_property(
        desc_id,
        writable_key,
        super::super::value::PropertyValue::Data(JsValue::Boolean(writable)),
        super::super::shape::PropertyAttrs::DATA,
    );
    ctx.vm.define_shaped_property(
        desc_id,
        enumerable_key,
        super::super::value::PropertyValue::Data(JsValue::Boolean(enumerable)),
        super::super::shape::PropertyAttrs::DATA,
    );
    ctx.vm.define_shaped_property(
        desc_id,
        configurable_key,
        super::super::value::PropertyValue::Data(JsValue::Boolean(configurable)),
        super::super::shape::PropertyAttrs::DATA,
    );
    desc_id
}

/// `Object.getOwnPropertyDescriptor(obj, prop)` — ECMA-262 §20.1.2.8
pub(in super::super) fn native_object_get_own_property_descriptor(
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
        if let Some(result) = super::super::host::dataset::try_get(ctx.vm, obj_id, prop) {
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
        if let Some(result) = super::super::host::storage::try_get(ctx.vm, obj_id, prop) {
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
        storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: ctx.vm.object_prototype,
        extensible: true,
    });
    let configurable_key = PropertyKey::String(ctx.vm.well_known.configurable);
    let enumerable_key = PropertyKey::String(ctx.vm.well_known.enumerable);
    match slot {
        super::super::value::PropertyValue::Data(v) => {
            let value_key = PropertyKey::String(ctx.vm.well_known.value);
            let writable_key = PropertyKey::String(ctx.vm.well_known.writable);
            ctx.vm.define_shaped_property(
                desc_id,
                value_key,
                super::super::value::PropertyValue::Data(v),
                super::super::shape::PropertyAttrs::DATA,
            );
            ctx.vm.define_shaped_property(
                desc_id,
                writable_key,
                super::super::value::PropertyValue::Data(JsValue::Boolean(attrs.writable)),
                super::super::shape::PropertyAttrs::DATA,
            );
        }
        super::super::value::PropertyValue::Accessor { getter, setter } => {
            let get_key = PropertyKey::String(ctx.vm.well_known.get);
            let set_key = PropertyKey::String(ctx.vm.well_known.set);
            let get_val = getter.map_or(JsValue::Undefined, JsValue::Object);
            let set_val = setter.map_or(JsValue::Undefined, JsValue::Object);
            ctx.vm.define_shaped_property(
                desc_id,
                get_key,
                super::super::value::PropertyValue::Data(get_val),
                super::super::shape::PropertyAttrs::DATA,
            );
            ctx.vm.define_shaped_property(
                desc_id,
                set_key,
                super::super::value::PropertyValue::Data(set_val),
                super::super::shape::PropertyAttrs::DATA,
            );
        }
    }
    ctx.vm.define_shaped_property(
        desc_id,
        enumerable_key,
        super::super::value::PropertyValue::Data(JsValue::Boolean(attrs.enumerable)),
        super::super::shape::PropertyAttrs::DATA,
    );
    ctx.vm.define_shaped_property(
        desc_id,
        configurable_key,
        super::super::value::PropertyValue::Data(JsValue::Boolean(attrs.configurable)),
        super::super::shape::PropertyAttrs::DATA,
    );
    Ok(JsValue::Object(desc_id))
}

/// `Object.getOwnPropertyNames(obj)` — ECMA-262 §20.1.2.10
pub(in super::super) fn native_object_get_own_property_names(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let obj_id = to_object_arg(ctx, args)?;
    // §10.1.11.1 OrdinaryOwnPropertyKeys: array element indices (ascending),
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
    let mut index_keys: Vec<(u32, super::super::value::StringId)> = Vec::new();
    let mut other_keys: Vec<super::super::value::StringId> = Vec::new();
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
