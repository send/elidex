//! Object and array creation opcode handlers extracted from the main dispatch loop.

use super::coerce;
use super::ops::{parse_array_index_u16, DENSE_ARRAY_LEN_LIMIT};
use super::value::{JsValue, ObjectKind, PropertyKey, PropertyValue, VmError};
use super::VmInner;

impl VmInner {
    /// `CreateObject` — allocate an ordinary object with `Object.prototype`.
    pub(crate) fn op_create_object(&mut self) {
        let proto = self.object_prototype;
        let id = self.alloc_object(super::value::Object {
            kind: ObjectKind::Ordinary,
            storage: super::value::PropertyStorage::shaped(super::shape::ROOT_SHAPE),
            prototype: proto,
            extensible: true,
        });
        self.stack.push(JsValue::Object(id));
    }

    /// `DefineProperty` — define a named data property on the object at TOS.
    pub(crate) fn op_define_property(
        &mut self,
        name_id: super::value::StringId,
    ) -> Result<(), VmError> {
        let pk = PropertyKey::String(name_id);
        let val = self.pop()?;
        let obj_val = self.peek()?;
        if let JsValue::Object(id) = obj_val {
            // Sync global object writes to the globals HashMap.
            if id == self.global_object {
                self.globals.insert(name_id, val);
            }
            self.upsert_data_property(id, pk, val, super::shape::PropertyAttrs::DATA);
        }
        Ok(())
    }

    /// `DefineComputedProperty` — define a computed-key data property.
    pub(crate) fn op_define_computed_property(
        &mut self,
        entry_frame_depth: usize,
    ) -> Result<(), VmError> {
        let val = self.pop()?;
        let key = self.pop()?;
        let obj_val = self.peek()?;
        if let JsValue::Object(id) = obj_val {
            match self.make_property_key(key) {
                Ok(pk) => {
                    // Sync global object writes to globals HashMap.
                    if id == self.global_object {
                        if let PropertyKey::String(sid) = pk {
                            self.globals.insert(sid, val);
                        }
                    }
                    self.upsert_data_property(id, pk, val, super::shape::PropertyAttrs::DATA);
                }
                Err(e) => {
                    self.throw_error(e, entry_frame_depth)?;
                }
            }
        }
        Ok(())
    }

    /// `DefineComputedMethod` — like `DefineComputedProperty` but non-enumerable (section 14.3.8).
    pub(crate) fn op_define_computed_method(
        &mut self,
        entry_frame_depth: usize,
    ) -> Result<(), VmError> {
        let val = self.pop()?;
        let key = self.pop()?;
        let obj_val = self.peek()?;
        if let JsValue::Object(id) = obj_val {
            match self.make_property_key(key) {
                Ok(pk) => {
                    if id == self.global_object {
                        if let PropertyKey::String(sid) = pk {
                            self.globals.insert(sid, val);
                        }
                    }
                    self.upsert_data_property(id, pk, val, super::shape::PropertyAttrs::METHOD);
                }
                Err(e) => {
                    self.throw_error(e, entry_frame_depth)?;
                }
            }
        }
        Ok(())
    }

    /// `CreateArray` — allocate an array with `Array.prototype`.
    pub(crate) fn op_create_array(&mut self) {
        let id = self.create_array_object(Vec::new());
        self.stack.push(JsValue::Object(id));
    }

    /// `ArrayPush` — push a value onto the array at TOS.
    pub(crate) fn op_array_push(&mut self) -> Result<(), VmError> {
        let val = self.pop()?;
        let arr_val = self.peek()?;
        if let JsValue::Object(id) = arr_val {
            if let ObjectKind::Array { ref mut elements } = self.get_object_mut(id).kind {
                if elements.len() >= DENSE_ARRAY_LEN_LIMIT {
                    return Err(VmError::range_error("Array allocation failed"));
                }
                elements.push(val);
            }
        }
        Ok(())
    }

    /// `ArrayHole` — push a sparse hole (`Empty`) onto the array at TOS (elision).
    pub(crate) fn op_array_hole(&mut self) -> Result<(), VmError> {
        let arr_val = self.peek()?;
        if let JsValue::Object(id) = arr_val {
            if let ObjectKind::Array { ref mut elements } = self.get_object_mut(id).kind {
                if elements.len() >= DENSE_ARRAY_LEN_LIMIT {
                    return Err(VmError::range_error("Array allocation failed"));
                }
                elements.push(JsValue::Empty);
            }
        }
        Ok(())
    }

    /// `SpreadObject` — copy all enumerable own properties from source to target.
    /// Accessor properties invoke their getter via Get (§12.2.6.8).
    pub(crate) fn op_spread_object(&mut self) -> Result<(), VmError> {
        let source = self.pop()?;
        let obj_val = self.peek()?;
        if let (JsValue::Object(src_id), JsValue::Object(dst_id)) = (source, obj_val) {
            let is_global = dst_id == self.global_object;
            // §12.2.6.8 CopyDataProperties: snapshot keys in ES order, then Get per key.
            // Array element indices (ascending) come before string keys.
            let keys: Vec<PropertyKey> = {
                let elem_indices: Vec<usize> = match &self.get_object(src_id).kind {
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
                    let sid = self.strings.intern(&i.to_string());
                    ks.push(PropertyKey::String(sid));
                }
                let obj = self.get_object(src_id);
                for (k, attrs) in obj.storage.iter_keys(&self.shapes) {
                    if attrs.enumerable {
                        ks.push(k);
                    }
                }
                ks
            };
            // GC safety: root resolved values on the stack while getters
            // for subsequent properties may trigger allocations / GC.
            let stack_base = self.stack.len();
            for key in &keys {
                let val = self.get_property_value(src_id, *key)?;
                self.stack.push(val);
            }
            let values: Vec<JsValue> = self.stack.drain(stack_base..).collect();
            let props: Vec<(PropertyKey, JsValue)> = keys.into_iter().zip(values).collect();
            // Apply resolved values to destination.
            if is_global {
                for (k, v) in &props {
                    if let PropertyKey::String(sid) = k {
                        self.globals.insert(*sid, *v);
                    }
                }
            }
            for (k, v) in props {
                self.upsert_data_property(dst_id, k, v, super::shape::PropertyAttrs::DATA);
            }
        }
        Ok(())
    }

    /// `DefineMethod` — define a named non-enumerable method (class method).
    pub(crate) fn op_define_method(
        &mut self,
        name_id: super::value::StringId,
    ) -> Result<(), VmError> {
        let val = self.pop()?;
        let obj_val = self.peek()?;
        if let JsValue::Object(id) = obj_val {
            let key = PropertyKey::String(name_id);
            self.upsert_data_property(id, key, val, super::shape::PropertyAttrs::METHOD);
        }
        Ok(())
    }

    /// Define a getter or setter accessor on the object at TOS.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn op_define_accessor(
        &mut self,
        name_id: super::value::StringId,
        is_getter: bool,
        enumerable: bool,
    ) -> Result<(), VmError> {
        enum AccessorAction {
            Updated,
            ReconfigureData,
            New,
        }
        let closure = self.pop()?;
        let obj_val = self.peek()?;
        if let (JsValue::Object(obj_id), JsValue::Object(fn_id)) = (obj_val, closure) {
            let pk = PropertyKey::String(name_id);
            let (init_get, init_set) = if is_getter {
                (Some(fn_id), None)
            } else {
                (None, Some(fn_id))
            };
            let accessor_attrs = super::shape::PropertyAttrs {
                writable: false,
                enumerable,
                configurable: true,
                is_accessor: true,
            };
            // Determine existing property state using split borrow.
            let action = {
                let shapes = &self.shapes;
                let obj = self.objects[obj_id.0 as usize].as_mut().unwrap();
                if let Some((slot, attrs)) = obj.storage.get_mut(pk, shapes) {
                    if attrs.is_accessor {
                        // Existing accessor — update getter or setter in place.
                        if let PropertyValue::Accessor { getter, setter } = slot {
                            if is_getter {
                                *getter = Some(fn_id);
                            } else {
                                *setter = Some(fn_id);
                            }
                        }
                        AccessorAction::Updated
                    } else {
                        // Existing data property — need reconfigure transition.
                        AccessorAction::ReconfigureData
                    }
                } else {
                    AccessorAction::New
                }
            };
            match action {
                AccessorAction::Updated => {
                    // Slot was updated in place.  If enumerability changed, reconfigure.
                    let needs_reconfigure = {
                        let shapes = &self.shapes;
                        let obj = self.objects[obj_id.0 as usize].as_ref().unwrap();
                        if let Some((_, attrs)) = obj.storage.get(pk, shapes) {
                            attrs.enumerable != enumerable
                        } else {
                            false
                        }
                    };
                    if needs_reconfigure {
                        // reconfigure_property handles Shaped mode; for Dictionary
                        // we must update the Property flags directly.
                        let obj = self.objects[obj_id.0 as usize].as_mut().unwrap();
                        if let super::value::PropertyStorage::Dictionary(vec) = &mut obj.storage {
                            if let Some((_, prop)) = vec.iter_mut().find(|(k, _)| *k == pk) {
                                prop.enumerable = accessor_attrs.enumerable;
                                prop.configurable = accessor_attrs.configurable;
                            }
                        } else {
                            self.reconfigure_property(obj_id, pk, accessor_attrs, None);
                        }
                    }
                }
                AccessorAction::ReconfigureData => {
                    // Data → accessor: reconfigure transition + replace slot value.
                    let accessor_slot = PropertyValue::Accessor {
                        getter: init_get,
                        setter: init_set,
                    };
                    let obj = self.objects[obj_id.0 as usize].as_ref().unwrap();
                    let is_dict =
                        matches!(&obj.storage, super::value::PropertyStorage::Dictionary(_));
                    if is_dict {
                        // Dictionary mode: replace the full Property entry
                        // (slot + flags) directly since reconfigure_property is
                        // a no-op for Dictionary.
                        let obj = self.objects[obj_id.0 as usize].as_mut().unwrap();
                        if let super::value::PropertyStorage::Dictionary(vec) = &mut obj.storage {
                            if let Some((_, prop)) = vec.iter_mut().find(|(k, _)| *k == pk) {
                                prop.slot = accessor_slot;
                                prop.writable = accessor_attrs.writable;
                                prop.enumerable = accessor_attrs.enumerable;
                                prop.configurable = accessor_attrs.configurable;
                            }
                        }
                    } else {
                        // Shaped mode: reconfigure_property handles transition.
                        self.reconfigure_property(obj_id, pk, accessor_attrs, Some(accessor_slot));
                    }
                }
                AccessorAction::New => {
                    // New accessor property.
                    self.define_shaped_property(
                        obj_id,
                        pk,
                        PropertyValue::Accessor {
                            getter: init_get,
                            setter: init_set,
                        },
                        accessor_attrs,
                    );
                }
            }
        }
        Ok(())
    }

    /// `instanceof` operator (§12.10.4).
    pub(crate) fn op_instanceof(&mut self, lhs: JsValue, rhs: JsValue) -> Result<bool, VmError> {
        let JsValue::Object(rhs_id) = rhs else {
            return Err(VmError::type_error(
                "Right-hand side of 'instanceof' is not an object",
            ));
        };

        // Step 2: Check rhs[@@hasInstance]
        let has_instance_key = PropertyKey::Symbol(self.well_known_symbols.has_instance);
        if let Some(has_instance_result) = coerce::get_property(self, rhs_id, has_instance_key) {
            let has_instance_fn = self.resolve_property(has_instance_result, rhs)?;
            let result = self.call_value(has_instance_fn, rhs, &[lhs])?;
            return Ok(coerce::to_boolean(self, result));
        }

        // OrdinaryHasInstance: walk lhs's prototype chain looking for rhs.prototype
        let JsValue::Object(obj_id) = lhs else {
            return Ok(false);
        };
        let proto_key = PropertyKey::String(self.well_known.prototype);
        let ctor_proto = coerce::get_property(self, rhs_id, proto_key);
        if let Some(coerce::PropertyResult::Data(JsValue::Object(target_proto))) = ctor_proto {
            let mut current = self.get_object(obj_id).prototype;
            while let Some(proto_id) = current {
                if proto_id == target_proto {
                    return Ok(true);
                }
                current = self.get_object(proto_id).prototype;
            }
        }
        Ok(false)
    }

    /// `in` operator (§13.10.1).
    pub(crate) fn op_in(&mut self, lhs: JsValue, rhs: JsValue) -> Result<bool, VmError> {
        let JsValue::Object(obj_id) = rhs else {
            return Err(VmError::type_error(
                "Cannot use 'in' operator to search for property in non-object",
            ));
        };
        let pk = self.make_property_key(lhs)?;
        let obj = self.get_object(obj_id);
        Ok(match (&obj.kind, &pk) {
            (ObjectKind::Array { ref elements }, PropertyKey::String(key_id)) => {
                let key_units = self.strings.get(*key_id);
                if let Some(idx) = parse_array_index_u16(key_units) {
                    if idx < elements.len() && !elements[idx].is_empty() {
                        true
                    } else {
                        // Index beyond elements or hole — fall back to property storage.
                        coerce::get_property(self, obj_id, pk).is_some()
                    }
                } else {
                    coerce::get_property(self, obj_id, pk).is_some()
                }
            }
            _ => coerce::get_property(self, obj_id, pk).is_some(),
        })
    }
}
