//! Shape-transition and property-operation helpers for [`VmInner`].
//!
//! Split out of [`super::mod`] to keep that file under the
//! project's 1000-line convention.  All APIs remain `pub(crate)` —
//! call sites are unaffected by the move.

use std::collections::HashMap;

use super::value::{self, JsValue, NativeContext, Object, ObjectId, ObjectKind, VmError};
use super::{coerce, shape, VmInner};
use crate::bytecode::compiled::CompiledFunction;
use value::{FuncId, NativeFunction, UpvalueId};

impl VmInner {
    // -- Shape helpers --------------------------------------------------------

    /// Add-transition: add a new property to a Shape, returning the child ShapeId.
    /// Reuses an existing transition if the same (key, attrs) was added before.
    pub(crate) fn shape_add_transition(
        &mut self,
        parent: shape::ShapeId,
        key: value::PropertyKey,
        attrs: shape::PropertyAttrs,
    ) -> shape::ShapeId {
        let tk = shape::TransitionKey::Add(key, attrs);
        if let Some(&child) = self.shapes[parent as usize].transitions.get(&tk) {
            return child;
        }
        let parent_shape = &self.shapes[parent as usize];
        debug_assert!(
            !parent_shape.property_map.contains_key(&key),
            "shape_add_transition called for existing key; use shape_reconfigure_transition instead"
        );
        let mut property_map = parent_shape.property_map.clone();
        let slot_index = parent_shape.ordered_entries.len() as u16;
        property_map.insert(key, slot_index);
        let mut ordered_entries = parent_shape.ordered_entries.clone();
        ordered_entries.push((key, attrs));
        let child_id = self.shapes.len() as shape::ShapeId;
        self.shapes.push(shape::Shape {
            transitions: HashMap::new(),
            property_map,
            ordered_entries,
        });
        self.shapes[parent as usize]
            .transitions
            .insert(tk, child_id);
        child_id
    }

    /// Reconfigure-transition: change the attributes of an existing property.
    /// Slot index is unchanged; only attrs in ordered_entries are updated.
    pub(crate) fn shape_reconfigure_transition(
        &mut self,
        parent: shape::ShapeId,
        key: value::PropertyKey,
        attrs: shape::PropertyAttrs,
    ) -> shape::ShapeId {
        let tk = shape::TransitionKey::Reconfigure(key, attrs);
        if let Some(&child) = self.shapes[parent as usize].transitions.get(&tk) {
            return child;
        }
        let parent_shape = &self.shapes[parent as usize];
        debug_assert!(
            parent_shape.property_map.contains_key(&key),
            "shape_reconfigure_transition called for non-existent key"
        );
        let slot_index = parent_shape.property_map[&key];
        let property_map = parent_shape.property_map.clone();
        let mut ordered_entries = parent_shape.ordered_entries.clone();
        ordered_entries[slot_index as usize].1 = attrs;
        let child_id = self.shapes.len() as shape::ShapeId;
        self.shapes.push(shape::Shape {
            transitions: HashMap::new(),
            property_map,
            ordered_entries,
        });
        self.shapes[parent as usize]
            .transitions
            .insert(tk, child_id);
        child_id
    }

    /// Reconfigure an existing property's attributes on a Shaped object.
    /// Updates the shape via reconfigure transition and optionally writes a new slot value.
    pub(crate) fn reconfigure_property(
        &mut self,
        obj_id: ObjectId,
        key: value::PropertyKey,
        new_attrs: shape::PropertyAttrs,
        new_value: Option<value::PropertyValue>,
    ) {
        let current_shape = match &self.objects[obj_id.0 as usize].as_ref().unwrap().storage {
            value::PropertyStorage::Shaped { shape, .. } => *shape,
            value::PropertyStorage::Dictionary(_) => return, // no-op for dictionary
        };
        let new_shape = self.shape_reconfigure_transition(current_shape, key, new_attrs);
        let obj = self.objects[obj_id.0 as usize].as_mut().unwrap();
        if let value::PropertyStorage::Shaped { shape, slots } = &mut obj.storage {
            *shape = new_shape;
            if let Some(val) = new_value {
                let slot_idx = self.shapes[new_shape as usize].property_map[&key];
                slots[slot_idx as usize] = val;
            }
        }
    }

    /// Install a pre-built shape and its matching slot values on an
    /// object in a single operation — skipping the per-property
    /// transition walk.
    ///
    /// Used by hot paths where the final property layout is fixed at
    /// VM creation time (e.g. event objects via `PrecomputedEventShapes`):
    /// allocate the object at `ROOT_SHAPE` with an empty slot vec,
    /// then call this API once with the precomputed terminal shape and
    /// the pre-assembled slot values.  Replaces ~N `define_shaped_property`
    /// calls with a single `PropertyStorage` replacement.
    ///
    /// `slots` is consumed by value and **moved** into the object
    /// (the caller's `Vec` becomes the object's slot storage
    /// directly) — no intermediate `collect()` allocates a second
    /// vector.  Callers that need accessor properties on the fast
    /// path must fall back to `define_shaped_property` (a design
    /// trade-off — this API is optimised for the event-object case,
    /// where every own property is a data property and accessors
    /// live on the shared `event_methods_prototype`).
    ///
    /// # Panics
    ///
    /// Debug-only asserts the slot count matches the shape's property
    /// count; mismatch means the caller assembled the slot Vec in a
    /// different order than the shape was built with — a structural
    /// bug that would otherwise silently write values into the wrong
    /// JS-visible property names.
    ///
    /// Also panics if the object is in `Dictionary` storage mode —
    /// caller should only route objects that have never left
    /// `Shaped` (freshly-allocated event objects never transition to
    /// Dictionary).
    //
    // Engine-feature gated — the sole consumer is
    // `host::events::create_event_object`, which is itself engine-only
    // (no DOM events to dispatch in non-engine builds).  A future
    // non-engine caller can relax this, but for now it keeps the
    // non-engine build free of dead-code warnings.
    #[cfg(feature = "engine")]
    pub(crate) fn define_with_precomputed_shape(
        &mut self,
        obj_id: ObjectId,
        shape_id: shape::ShapeId,
        slots: Vec<value::PropertyValue>,
    ) {
        debug_assert_eq!(
            self.shapes[shape_id as usize].property_count() as usize,
            slots.len(),
            "define_with_precomputed_shape: slot count ({}) does not match shape property count ({}) — caller built the slot Vec in a different order than the shape",
            slots.len(),
            self.shapes[shape_id as usize].property_count(),
        );
        let obj = self.objects[obj_id.0 as usize].as_mut().unwrap();
        match &mut obj.storage {
            value::PropertyStorage::Shaped { shape, slots: s } => {
                *shape = shape_id;
                *s = slots;
            }
            value::PropertyStorage::Dictionary(_) => {
                panic!("define_with_precomputed_shape requires Shaped storage; got Dictionary");
            }
        }
    }

    /// Define a new property on a Shaped object: transition + slot push.
    /// If the object is in Dictionary mode, pushes directly.
    pub(crate) fn define_shaped_property(
        &mut self,
        obj_id: ObjectId,
        key: value::PropertyKey,
        value: value::PropertyValue,
        attrs: shape::PropertyAttrs,
    ) {
        // Read current shape.
        let current_shape = match &self.objects[obj_id.0 as usize].as_ref().unwrap().storage {
            value::PropertyStorage::Shaped { shape, .. } => *shape,
            value::PropertyStorage::Dictionary(_) => {
                let prop = value::Property::from_attrs(value, attrs);
                self.get_object_mut(obj_id).storage.push_dict(key, prop);
                return;
            }
        };
        // Transition shape.
        let new_shape = self.shape_add_transition(current_shape, key, attrs);
        // Update object.
        let obj = self.objects[obj_id.0 as usize].as_mut().unwrap();
        if let value::PropertyStorage::Shaped { shape, slots } = &mut obj.storage {
            *shape = new_shape;
            slots.push(value);
        }
    }

    /// Convert a Shaped object to Dictionary mode (for delete).
    pub(crate) fn convert_to_dictionary(&mut self, obj_id: ObjectId) {
        let obj = self.objects[obj_id.0 as usize].as_mut().unwrap();
        let new_storage = match &obj.storage {
            value::PropertyStorage::Dictionary(_) => return, // already dictionary
            value::PropertyStorage::Shaped { shape, slots } => {
                let s = &self.shapes[*shape as usize];
                let vec: Vec<(value::PropertyKey, value::Property)> = s
                    .ordered_entries
                    .iter()
                    .enumerate()
                    .map(|(i, (key, attrs))| {
                        (
                            *key,
                            value::Property {
                                slot: slots[i],
                                writable: attrs.writable,
                                enumerable: attrs.enumerable,
                                configurable: attrs.configurable,
                            },
                        )
                    })
                    .collect();
                value::PropertyStorage::Dictionary(vec)
            }
        };
        obj.storage = new_storage;
    }

    // -- Compiled functions --------------------------------------------------

    /// Register a compiled function in the VM, returning its `FuncId`.
    pub(crate) fn register_function(&mut self, func: CompiledFunction) -> FuncId {
        let id = FuncId(self.compiled_functions.len() as u32);
        self.compiled_functions.push(func);
        id
    }

    /// Get a reference to a compiled function.
    #[inline]
    pub(crate) fn get_compiled(&self, id: FuncId) -> &CompiledFunction {
        &self.compiled_functions[id.0 as usize]
    }

    // -- Upvalues ------------------------------------------------------------

    /// Allocate an upvalue, returning its `UpvalueId`.
    pub(crate) fn alloc_upvalue(&mut self, uv: value::Upvalue) -> UpvalueId {
        if let Some(idx) = self.free_upvalues.pop() {
            self.upvalues[idx as usize] = uv;
            UpvalueId(idx)
        } else {
            let id = UpvalueId(self.upvalues.len() as u32);
            self.upvalues.push(uv);
            id
        }
    }

    // -- Native function helpers ---------------------------------------------

    /// Helper: create a native function object (non-constructable by default,
    /// matching the ES2020 spec for most built-in functions).
    pub(crate) fn create_native_function(
        &mut self,
        name: &str,
        func: fn(&mut NativeContext<'_>, JsValue, &[JsValue]) -> Result<JsValue, VmError>,
    ) -> ObjectId {
        self.create_native_function_impl(name, func, false)
    }

    /// Helper: create a constructable native function object (for Error, etc.).
    pub(crate) fn create_constructable_function(
        &mut self,
        name: &str,
        func: fn(&mut NativeContext<'_>, JsValue, &[JsValue]) -> Result<JsValue, VmError>,
    ) -> ObjectId {
        self.create_native_function_impl(name, func, true)
    }

    fn create_native_function_impl(
        &mut self,
        name: &str,
        func: fn(&mut NativeContext<'_>, JsValue, &[JsValue]) -> Result<JsValue, VmError>,
        constructable: bool,
    ) -> ObjectId {
        let name_id = self.strings.intern(name);
        let obj = self.alloc_object(Object {
            kind: ObjectKind::NativeFunction(NativeFunction {
                name: name_id,
                func,
                constructable,
            }),
            storage: value::PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: self.function_prototype,
            extensible: true,
        });
        // §19.2.4.2: `name` is a non-enumerable, non-writable, configurable
        // data property on every built-in function.
        let name_key = value::PropertyKey::String(self.well_known.name);
        self.define_shaped_property(
            obj,
            name_key,
            value::PropertyValue::Data(JsValue::String(name_id)),
            shape::PropertyAttrs {
                writable: false,
                enumerable: false,
                configurable: true,
                is_accessor: false,
            },
        );
        obj
    }

    /// Update an existing data property or define a new one.
    pub(crate) fn upsert_data_property(
        &mut self,
        obj_id: ObjectId,
        key: value::PropertyKey,
        val: JsValue,
        attrs: shape::PropertyAttrs,
    ) {
        let existing_attrs = {
            let shapes = &self.shapes;
            let obj = self.objects[obj_id.0 as usize].as_ref().unwrap();
            obj.storage.get(key, shapes).map(|(_, a)| a)
        };
        match existing_attrs {
            Some(current_attrs) if current_attrs == attrs => {
                // Same attrs — just update the slot value.
                let shapes = &self.shapes;
                let obj = self.objects[obj_id.0 as usize].as_mut().unwrap();
                if let Some((slot, _)) = obj.storage.get_mut(key, shapes) {
                    *slot = value::PropertyValue::Data(val);
                }
            }
            Some(_) => {
                // Attrs differ — update both value and attrs.
                let new_val = value::PropertyValue::Data(val);
                let is_shaped = matches!(
                    self.objects[obj_id.0 as usize].as_ref().unwrap().storage,
                    value::PropertyStorage::Shaped { .. }
                );
                if is_shaped {
                    // Shaped: write value then reconfigure shape.
                    {
                        let shapes = &self.shapes;
                        let obj = self.objects[obj_id.0 as usize].as_mut().unwrap();
                        if let Some((slot, _)) = obj.storage.get_mut(key, shapes) {
                            *slot = new_val;
                        }
                    }
                    self.reconfigure_property(obj_id, key, attrs, None);
                } else {
                    // Dictionary: replace the entire Property.
                    let obj = self.objects[obj_id.0 as usize].as_mut().unwrap();
                    if let value::PropertyStorage::Dictionary(vec) = &mut obj.storage {
                        if let Some((_, prop)) = vec.iter_mut().find(|(k, _)| *k == key) {
                            *prop = value::Property::from_attrs(new_val, attrs);
                        }
                    }
                }
            }
            None => {
                // Non-extensible objects cannot gain new properties.
                if !self.get_object(obj_id).extensible {
                    return;
                }
                self.define_shaped_property(obj_id, key, value::PropertyValue::Data(val), attrs);
            }
        }
    }

    /// Resolve a `PropertyValue` slot to a `JsValue`, invoking the getter
    /// if the slot is an accessor.
    pub(crate) fn resolve_slot(
        &mut self,
        slot: value::PropertyValue,
        this: JsValue,
    ) -> Result<JsValue, VmError> {
        match slot {
            value::PropertyValue::Data(v) => Ok(v),
            value::PropertyValue::Accessor {
                getter: Some(g), ..
            } => self.call(g, this, &[]),
            value::PropertyValue::Accessor { getter: None, .. } => Ok(JsValue::Undefined),
        }
    }

    /// Perform a fresh `Get` (§7.3.1) on an object by `PropertyKey`.
    pub(crate) fn get_property_value(
        &mut self,
        obj_id: value::ObjectId,
        key: value::PropertyKey,
    ) -> Result<JsValue, VmError> {
        let result = coerce::get_property(self, obj_id, key);
        match result {
            Some(coerce::PropertyResult::Data(v)) => Ok(v),
            Some(coerce::PropertyResult::Getter(g)) => self.call(g, JsValue::Object(obj_id), &[]),
            None => Ok(JsValue::Undefined),
        }
    }
}
