//! Object and array creation opcode handlers extracted from the main dispatch loop.

use super::value::{JsValue, ObjectKind, Property, PropertyKey, PropertyValue, VmError};
use super::Vm;

impl Vm {
    /// `CreateObject` — allocate an ordinary object with `Object.prototype`.
    pub(crate) fn op_create_object(&mut self) {
        let proto = self.inner.object_prototype;
        let id = self.alloc_object(super::value::Object {
            kind: ObjectKind::Ordinary,
            properties: Vec::new(),
            prototype: proto,
        });
        self.inner.stack.push(JsValue::Object(id));
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
            if id == self.inner.global_object {
                self.inner.globals.insert(name_id, val);
            }
            let obj = self.get_object_mut(id);
            // Overwrite if key already exists (e.g. after spread).
            if let Some(existing) = obj.properties.iter_mut().find(|(k, _)| *k == pk) {
                existing.1 = Property::data(val);
            } else {
                obj.properties.push((pk, Property::data(val)));
            }
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
                    if id == self.inner.global_object {
                        if let PropertyKey::String(sid) = pk {
                            self.inner.globals.insert(sid, val);
                        }
                    }
                    let obj = self.get_object_mut(id);
                    if let Some(existing) = obj.properties.iter_mut().find(|(k, _)| *k == pk) {
                        existing.1 = Property::data(val);
                    } else {
                        obj.properties.push((pk, Property::data(val)));
                    }
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
                    if id == self.inner.global_object {
                        if let PropertyKey::String(sid) = pk {
                            self.inner.globals.insert(sid, val);
                        }
                    }
                    let obj = self.get_object_mut(id);
                    if let Some(existing) = obj.properties.iter_mut().find(|(k, _)| *k == pk) {
                        existing.1 = Property::method(val);
                    } else {
                        obj.properties.push((pk, Property::method(val)));
                    }
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
        let proto = self.inner.array_prototype;
        let id = self.alloc_object(super::value::Object {
            kind: ObjectKind::Array {
                elements: Vec::new(),
            },
            properties: Vec::new(),
            prototype: proto,
        });
        self.inner.stack.push(JsValue::Object(id));
    }

    /// `ArrayPush` — push a value onto the array at TOS.
    pub(crate) fn op_array_push(&mut self) -> Result<(), VmError> {
        let val = self.pop()?;
        let arr_val = self.peek()?;
        if let JsValue::Object(id) = arr_val {
            if let ObjectKind::Array { ref mut elements } = self.get_object_mut(id).kind {
                elements.push(val);
            }
        }
        Ok(())
    }

    /// `ArrayHole` — push `undefined` onto the array at TOS (elision).
    pub(crate) fn op_array_hole(&mut self) -> Result<(), VmError> {
        let arr_val = self.peek()?;
        if let JsValue::Object(id) = arr_val {
            if let ObjectKind::Array { ref mut elements } = self.get_object_mut(id).kind {
                elements.push(JsValue::Undefined);
            }
        }
        Ok(())
    }

    /// `SpreadObject` — copy all enumerable own properties from source to target.
    pub(crate) fn op_spread_object(&mut self) -> Result<(), VmError> {
        let source = self.pop()?;
        let obj_val = self.peek()?;
        if let (JsValue::Object(src_id), JsValue::Object(dst_id)) = (source, obj_val) {
            let is_global = dst_id == self.inner.global_object;
            let src = self.inner.get_object(src_id);
            // TODO(M4-11): spread should invoke getters via Get for accessor properties.
            // Requires VM single dispatcher (NativeContext re-entrancy).
            let props: Vec<(PropertyKey, Property)> = src
                .properties
                .iter()
                .filter(|(_, p)| p.enumerable)
                .map(|(k, p)| (*k, Property::data(p.data_value())))
                .collect();
            // Sync global object writes to the globals HashMap.
            if is_global {
                for (k, p) in &props {
                    if let PropertyKey::String(sid) = k {
                        self.inner.globals.insert(*sid, p.data_value());
                    }
                }
            }
            let dst = self.inner.get_object_mut(dst_id);
            for (k, p) in props {
                if let Some(existing) = dst.properties.iter_mut().find(|(ek, _)| *ek == k) {
                    existing.1 = p;
                } else {
                    dst.properties.push((k, p));
                }
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
            let obj = self.get_object_mut(id);
            if let Some(existing) = obj.properties.iter_mut().find(|(k, _)| *k == key) {
                existing.1 = Property::method(val);
            } else {
                obj.properties.push((key, Property::method(val)));
            }
        }
        Ok(())
    }

    /// Define a getter or setter accessor on the object at TOS.
    pub(crate) fn op_define_accessor(
        &mut self,
        name_id: super::value::StringId,
        is_getter: bool,
        enumerable: bool,
    ) -> Result<(), VmError> {
        let closure = self.pop()?;
        let obj_val = self.peek()?;
        if let (JsValue::Object(obj_id), JsValue::Object(fn_id)) = (obj_val, closure) {
            let pk = PropertyKey::String(name_id);
            let (init_get, init_set) = if is_getter {
                (Some(fn_id), None)
            } else {
                (None, Some(fn_id))
            };
            let obj = self.get_object_mut(obj_id);
            if let Some(existing) = obj.properties.iter_mut().find(|(k, _)| *k == pk) {
                match &mut existing.1.slot {
                    PropertyValue::Accessor { getter, setter } => {
                        if is_getter {
                            *getter = Some(fn_id);
                        } else {
                            *setter = Some(fn_id);
                        }
                        // Update enumerability to match the latest definition.
                        existing.1.enumerable = enumerable;
                    }
                    PropertyValue::Data(_) => {
                        existing.1 = Property::accessor(init_get, init_set, enumerable);
                    }
                }
            } else {
                obj.properties
                    .push((pk, Property::accessor(init_get, init_set, enumerable)));
            }
        }
        Ok(())
    }
}
