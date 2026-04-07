//! Property access operations: get, set, delete, element access, IC collection.
//!
//! Extracted from `ops.rs` to keep that file focused on operator helpers,
//! exception handling, and function call mechanics.

use super::coerce::{
    find_inherited_property, get_property, to_string, InheritedProperty, PropertyResult,
};
use super::value::{
    FuncId, JsValue, ObjectId, ObjectKind, PropertyKey, PropertyValue, StringId, VmError,
};
use super::VmInner;

use super::ops::parse_array_index_u16;

// ---------------------------------------------------------------------------
// Property access
// ---------------------------------------------------------------------------

impl VmInner {
    /// Resolve a `PropertyResult` to a `JsValue`, invoking the getter if needed.
    pub(crate) fn resolve_property(
        &mut self,
        result: PropertyResult,
        receiver: JsValue,
    ) -> Result<JsValue, VmError> {
        match result {
            PropertyResult::Data(v) => Ok(v),
            PropertyResult::Getter(g) => self.call(g, receiver, &[]),
        }
    }

    /// Look up `pk` on a prototype object and resolve (invoke getter if accessor).
    /// Returns `Undefined` if the prototype is `None` or the property is not found.
    fn lookup_on_proto(
        &mut self,
        proto: Option<super::value::ObjectId>,
        pk: PropertyKey,
        receiver: JsValue,
    ) -> Result<JsValue, VmError> {
        if let Some(proto_id) = proto {
            match get_property(self, proto_id, pk) {
                Some(result) => self.resolve_property(result, receiver),
                None => Ok(JsValue::Undefined),
            }
        } else {
            Ok(JsValue::Undefined)
        }
    }

    pub(crate) fn get_property_val(
        &mut self,
        obj: JsValue,
        key: StringId,
    ) -> Result<JsValue, VmError> {
        let pk = PropertyKey::String(key);
        match obj {
            JsValue::Object(id) => {
                if id == self.global_object {
                    if let Some(result) = get_property(self, id, pk) {
                        return self.resolve_property(result, obj);
                    }
                    if let Some(&val) = self.globals.get(&key) {
                        return Ok(val);
                    }
                    return Ok(JsValue::Undefined);
                }
                match get_property(self, id, pk) {
                    Some(result) => self.resolve_property(result, obj),
                    None => Ok(JsValue::Undefined),
                }
            }
            JsValue::String(sid) => {
                if key == self.well_known.length {
                    #[allow(clippy::cast_precision_loss)]
                    let len = self.strings.get(sid).len() as f64;
                    Ok(JsValue::Number(len))
                } else {
                    self.lookup_on_proto(self.string_prototype, pk, obj)
                }
            }
            // TODO(M4-11): strict-mode getters on primitive prototypes should
            // receive a ToObject wrapper as `this`, not the raw primitive.
            // Requires VM single dispatcher for correct receiver boxing.
            JsValue::Symbol(_) => self.lookup_on_proto(self.symbol_prototype, pk, obj),
            JsValue::Number(_) => self.lookup_on_proto(self.number_prototype, pk, obj),
            JsValue::Boolean(_) => self.lookup_on_proto(self.boolean_prototype, pk, obj),
            _ => Ok(JsValue::Undefined),
        }
    }

    /// GetProp slow path: full property lookup (via `get_property_val`) + IC update.
    pub(crate) fn get_prop_slow(
        &mut self,
        obj_val: JsValue,
        obj_id: ObjectId,
        name_id: StringId,
        func_id: FuncId,
        ic_idx: usize,
    ) -> Result<JsValue, VmError> {
        // Collect IC info BEFORE the full lookup (which may trigger getters).
        let ic_info = self.collect_get_prop_ic(obj_id, PropertyKey::String(name_id));

        // Full property lookup including global object special path, prototype chain, etc.
        let val = self.get_property_val(obj_val, name_id)?;

        // Update IC
        if let Some(ic) = ic_info {
            if let Some(slot) = self.compiled_functions[func_id.0 as usize]
                .ic_slots
                .get_mut(ic_idx)
            {
                *slot = Some(ic);
            }
        }

        Ok(val)
    }

    /// Collect IC info for a GetProp operation without modifying state.
    fn collect_get_prop_ic(
        &self,
        obj_id: ObjectId,
        pk: PropertyKey,
    ) -> Option<super::ic::PropertyIC> {
        let obj = self.objects[obj_id.0 as usize].as_ref()?;
        let receiver_shape = match &obj.storage {
            super::value::PropertyStorage::Shaped { shape, .. } => *shape,
            super::value::PropertyStorage::Dictionary(_) => return None,
        };

        // Check own property
        if let Some(slot) = self.shapes[receiver_shape as usize].property_map.get(&pk) {
            return Some(super::ic::PropertyIC {
                receiver_shape,
                slot: *slot,
                holder: super::ic::ICHolder::Own,
            });
        }

        // Check immediate prototype
        let proto_id = obj.prototype?;
        let proto_obj = self.objects[proto_id.0 as usize].as_ref()?;
        let proto_shape = match &proto_obj.storage {
            super::value::PropertyStorage::Shaped { shape, .. } => *shape,
            super::value::PropertyStorage::Dictionary(_) => return None,
        };
        let proto_slot = self.shapes[proto_shape as usize].property_map.get(&pk)?;
        Some(super::ic::PropertyIC {
            receiver_shape,
            slot: *proto_slot,
            holder: super::ic::ICHolder::Proto {
                proto_shape,
                proto_slot: *proto_slot,
                proto_id,
            },
        })
    }

    /// SetProp slow path: full property set + IC update (own property only).
    pub(crate) fn set_prop_slow(
        &mut self,
        obj_val: JsValue,
        obj_id: ObjectId,
        name_id: StringId,
        val: JsValue,
        func_id: FuncId,
        ic_idx: usize,
    ) -> Result<(), VmError> {
        // Collect IC info for own property before the set.
        let ic_info = self.collect_set_prop_ic(obj_id, PropertyKey::String(name_id));

        // Full set_property_val (handles prototype chain, strict mode, accessors, etc.)
        self.set_property_val(obj_val, name_id, val)?;

        // Update IC
        if let Some(ic) = ic_info {
            if let Some(slot) = self.compiled_functions[func_id.0 as usize]
                .ic_slots
                .get_mut(ic_idx)
            {
                *slot = Some(ic);
            }
        }

        Ok(())
    }

    /// Collect IC info for a SetProp operation.
    /// Only caches own writable data properties (no prototype IC for writes).
    fn collect_set_prop_ic(
        &self,
        obj_id: ObjectId,
        pk: PropertyKey,
    ) -> Option<super::ic::PropertyIC> {
        let obj = self.objects[obj_id.0 as usize].as_ref()?;
        let receiver_shape = match &obj.storage {
            super::value::PropertyStorage::Shaped { shape, .. } => *shape,
            super::value::PropertyStorage::Dictionary(_) => return None,
        };
        let slot = *self.shapes[receiver_shape as usize].property_map.get(&pk)?;
        let attrs = self.shapes[receiver_shape as usize].ordered_entries[slot as usize].1;
        // Only cache writable data properties
        if attrs.writable && !attrs.is_accessor {
            Some(super::ic::PropertyIC {
                receiver_shape,
                slot,
                holder: super::ic::ICHolder::Own,
            })
        } else {
            None
        }
    }

    /// Collect Call IC info from a callee ObjectId.
    pub(crate) fn collect_call_ic(&self, callee_id: ObjectId) -> Option<super::ic::CallIC> {
        let obj = self.objects[callee_id.0 as usize].as_ref()?;
        if let ObjectKind::Function(fo) = &obj.kind {
            Some(super::ic::CallIC {
                callee: callee_id,
                func_id: fo.func_id,
                this_mode: fo.this_mode,
            })
        } else {
            None
        }
    }

    /// Check if the current call frame is in strict mode.
    pub(crate) fn is_strict_mode(&self) -> bool {
        self.frames
            .last()
            .is_some_and(|f| self.compiled_functions[f.func_id.0 as usize].is_strict)
    }

    /// Delete a named property from an object (single-pass).
    /// Returns `Ok(true)` if deleted, `Ok(false)` if non-configurable in
    /// sloppy mode, or `Err(TypeError)` if non-configurable in strict mode.
    pub(crate) fn try_delete_property(
        &mut self,
        id: ObjectId,
        pk: PropertyKey,
    ) -> Result<bool, VmError> {
        self.convert_to_dictionary(id);
        let obj = self.get_object_mut(id);
        if let Some(pos) = obj.storage.dict_position(pk) {
            if !obj.storage.dict_get(pos).configurable {
                if self.is_strict_mode() {
                    return Err(VmError::type_error(
                        "Cannot delete property: property is not configurable",
                    ));
                }
                return Ok(false);
            }
            obj.storage.remove_dict(pos);
            // Sync global object deletes to the globals HashMap.
            if id == self.global_object {
                if let PropertyKey::String(sid) = pk {
                    self.globals.remove(&sid);
                }
            }
            Ok(true)
        } else {
            Ok(true) // Property doesn't exist -- delete succeeds.
        }
    }

    /// §9.1.9 OrdinarySet: set a property on an object, checking own/inherited
    /// descriptors.  Shared by `set_property_val` (string keys) and
    /// `set_element` (symbol keys).
    fn ordinary_set(
        &mut self,
        id: ObjectId,
        pk: PropertyKey,
        val: JsValue,
        receiver: JsValue,
    ) -> Result<bool, VmError> {
        /// Action determined from own property in a single `get_mut` lookup.
        enum OwnAction {
            Written,
            NonWritable,
            CallSetter(ObjectId),
            NoSetter,
            NotFound,
        }

        let is_strict = self.is_strict_mode();

        // Step 1: check own property (single mutable lookup).
        let own_action = {
            let shapes = &self.shapes;
            let obj_ref = self.objects[id.0 as usize].as_mut().unwrap();
            match obj_ref.storage.get_mut(pk, shapes) {
                Some((slot, attrs)) => match slot {
                    PropertyValue::Data(_) if attrs.writable => {
                        *slot = PropertyValue::Data(val);
                        OwnAction::Written
                    }
                    PropertyValue::Data(_) => OwnAction::NonWritable,
                    PropertyValue::Accessor {
                        setter: Some(s), ..
                    } => OwnAction::CallSetter(*s),
                    PropertyValue::Accessor { setter: None, .. } => OwnAction::NoSetter,
                },
                None => OwnAction::NotFound,
            }
        };

        match own_action {
            OwnAction::Written => return Ok(true),
            OwnAction::NonWritable => {
                if is_strict {
                    return Err(VmError::type_error("Cannot assign to read only property"));
                }
                return Ok(false);
            }
            OwnAction::CallSetter(s) => {
                self.call(s, receiver, &[val])?;
                return Ok(false);
            }
            OwnAction::NoSetter => {
                if is_strict {
                    return Err(VmError::type_error(
                        "Cannot set property which has only a getter",
                    ));
                }
                return Ok(false);
            }
            OwnAction::NotFound => {} // fall through to prototype chain
        }
        // Step 2: no own property -- check prototype chain.
        match find_inherited_property(self, id, pk) {
            InheritedProperty::Setter(setter_id) => {
                self.call(setter_id, receiver, &[val])?;
                return Ok(false);
            }
            InheritedProperty::WritableFalse | InheritedProperty::AccessorNoSetter => {
                if is_strict {
                    return Err(VmError::type_error(
                        "Cannot set property: inherited descriptor prevents it",
                    ));
                }
                return Ok(false);
            }
            InheritedProperty::None => {}
        }
        // Step 3: create own data property.
        self.define_shaped_property(
            id,
            pk,
            PropertyValue::Data(val),
            super::shape::PropertyAttrs::DATA,
        );
        Ok(true)
    }

    pub(crate) fn set_property_val(
        &mut self,
        obj: JsValue,
        key: StringId,
        val: JsValue,
    ) -> Result<(), VmError> {
        let pk = PropertyKey::String(key);
        if let JsValue::Object(id) = obj {
            let is_global = id == self.global_object;
            let written = self.ordinary_set(id, pk, val, obj)?;
            // Sync the global variable table only when a data property was
            // actually written or created.  Accessor calls (setter / no-setter)
            // and non-writable rejections must NOT desynchronize the table.
            if is_global && written {
                self.globals.insert(key, val);
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn get_element(&mut self, obj: JsValue, key: JsValue) -> Result<JsValue, VmError> {
        if let JsValue::Object(id) = obj {
            // Numeric index for arrays.
            if let JsValue::Number(n) = key {
                #[allow(clippy::cast_sign_loss, clippy::cast_precision_loss)]
                let (idx, is_index) = {
                    let i = n as usize;
                    (i, n >= 0.0 && (i as f64) == n)
                };
                if is_index {
                    let obj_ref = self.get_object(id);
                    match &obj_ref.kind {
                        ObjectKind::Array { ref elements } => {
                            return Ok(elements.get(idx).copied().unwrap_or(JsValue::Undefined));
                        }
                        ObjectKind::Arguments { ref values } if idx < values.len() => {
                            return Ok(values[idx]);
                        }
                        _ => {}
                    }
                }
            }
            // Symbol key -> direct property lookup.
            if let JsValue::Symbol(sid) = key {
                let pk = PropertyKey::Symbol(sid);
                return match get_property(self, id, pk) {
                    Some(result) => self.resolve_property(result, obj),
                    None => Ok(JsValue::Undefined),
                };
            }
            // Fall back to string key property lookup.
            let key_id = to_string(self, key)?;
            let pk = PropertyKey::String(key_id);
            match get_property(self, id, pk) {
                Some(result) => self.resolve_property(result, obj),
                None => Ok(JsValue::Undefined),
            }
        } else if let JsValue::String(sid) = obj {
            // String bracket access: str[index] returns a single UTF-16 code unit.
            if let JsValue::Number(n) = key {
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let idx = n as usize;
                #[allow(clippy::cast_precision_loss)]
                if n >= 0.0 && (idx as f64) == n {
                    let unit = self.strings.get(sid).get(idx).copied();
                    if let Some(u) = unit {
                        let id = self.strings.intern_utf16(&[u]);
                        return Ok(JsValue::String(id));
                    }
                }
            } else if let JsValue::String(key_sid) = key {
                let unit = {
                    let key_units = self.strings.get(key_sid);
                    parse_array_index_u16(key_units)
                        .and_then(|idx| self.strings.get(sid).get(idx).copied())
                };
                if let Some(u) = unit {
                    let ch_id = self.strings.intern_utf16(&[u]);
                    return Ok(JsValue::String(ch_id));
                }
            }
            let pk = match key {
                JsValue::Symbol(sym) => PropertyKey::Symbol(sym),
                other => PropertyKey::String(to_string(self, other)?),
            };
            if pk == PropertyKey::String(self.well_known.length) {
                #[allow(clippy::cast_precision_loss)]
                let len = self.strings.get(sid).len() as f64;
                return Ok(JsValue::Number(len));
            }
            if let Some(proto_id) = self.string_prototype {
                match get_property(self, proto_id, pk) {
                    Some(result) => self.resolve_property(result, obj),
                    None => Ok(JsValue::Undefined),
                }
            } else {
                Ok(JsValue::Undefined)
            }
        } else if matches!(obj, JsValue::Number(_) | JsValue::Boolean(_)) {
            let proto = match obj {
                JsValue::Number(_) => self.number_prototype,
                _ => self.boolean_prototype,
            };
            let pk = match key {
                JsValue::Symbol(sym) => PropertyKey::Symbol(sym),
                other => PropertyKey::String(to_string(self, other)?),
            };
            self.lookup_on_proto(proto, pk, obj)
        } else {
            Ok(JsValue::Undefined)
        }
    }

    pub(crate) fn set_element(
        &mut self,
        obj: JsValue,
        key: JsValue,
        val: JsValue,
    ) -> Result<(), VmError> {
        if let JsValue::Object(id) = obj {
            if let JsValue::Number(n) = key {
                #[allow(clippy::cast_sign_loss, clippy::cast_precision_loss)]
                let (idx, is_index) = {
                    let i = n as usize;
                    (i, n >= 0.0 && (i as f64) == n)
                };
                if is_index {
                    let obj_ref = self.get_object_mut(id);
                    match &mut obj_ref.kind {
                        ObjectKind::Array { ref mut elements } => {
                            if idx >= elements.len() {
                                elements.resize(idx + 1, JsValue::Undefined);
                            }
                            elements[idx] = val;
                            return Ok(());
                        }
                        ObjectKind::Arguments { ref mut values } if idx < values.len() => {
                            values[idx] = val;
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }
            // Symbol key -> §9.1.9 OrdinarySet via shared helper.
            if let JsValue::Symbol(sid) = key {
                let pk = PropertyKey::Symbol(sid);
                self.ordinary_set(id, pk, val, obj)?;
                return Ok(());
            }
            let key_id = to_string(self, key)?;
            self.set_property_val(JsValue::Object(id), key_id, val)?;
        }
        Ok(())
    }
}
