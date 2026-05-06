//! Named-property access operations: get, set, delete, IC collection,
//! and the `ordinary_set` / `lookup_on_proto` primitives shared with
//! [`super::ops_element`] for `obj[key]` reads and writes.
//!
//! Extracted from `ops.rs` to keep that file focused on operator
//! helpers, exception handling, and function call mechanics.  Element
//! access (`get_element` / `set_element` plus the TypedArray
//! integer-indexed exotic dispatch and Array / Arguments dense
//! fast paths) was further split out into [`super::ops_element`]
//! during the 1000-line cleanup tranche 2.

use super::coerce::{find_inherited_property, get_property, InheritedProperty, PropertyResult};
use super::value::{
    FuncId, JsValue, ObjectId, ObjectKind, PropertyKey, PropertyValue, StringId, VmError,
};
use super::VmInner;

// ---------------------------------------------------------------------------
// Property access
// ---------------------------------------------------------------------------

/// Whether `ordinary_set` wrote or created an own data property.
/// Used by `set_property_val` to decide whether to sync `globals`.
/// Note: setter calls are ES2020-successful but do NOT produce a
/// `DataWritten` result, because the setter controls its own writes.
pub(super) enum SetOutcome {
    DataWritten,
    NoDataWrite,
}

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
    pub(super) fn lookup_on_proto(
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
        // §6.2.4.5 RequireObjectCoercible: property reads on null/undefined
        // must throw TypeError before any prototype-chain walk.
        super::coerce::require_object_coercible(obj)?;
        let pk = PropertyKey::String(key);
        match obj {
            JsValue::Object(id) => {
                // DOMStringMap (HTMLElement.dataset) named-property
                // exotic [[Get]] — `dataset.fooBar` reads the
                // `data-foo-bar` attribute via the `dataset.get`
                // handler.  Returning `Some(_)` short-circuits
                // before the ordinary prototype walk; `None`
                // (absent attribute) falls through so
                // `dataset.toString` still resolves to
                // `Object.prototype.toString`.
                #[cfg(feature = "engine")]
                if matches!(self.get_object(id).kind, ObjectKind::DOMStringMap { .. }) {
                    if let Some(result) =
                        super::host::dataset::try_get(self, id, JsValue::String(key))
                    {
                        return result;
                    }
                }
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
            // §6.2.4.1 step 4.b: prototype lookup for primitive base values
            // uses `GetThisValue(V)` (the original primitive) as Receiver,
            // independent of any boxing for own-property lookup.  An invoked
            // accessor observes the raw primitive as `this` per §9.4.3
            // step 5 — matches V8/SpiderMonkey.
            JsValue::Symbol(_) => self.lookup_on_proto(self.symbol_prototype, pk, obj),
            JsValue::Number(_) => self.lookup_on_proto(self.number_prototype, pk, obj),
            JsValue::Boolean(_) => self.lookup_on_proto(self.boolean_prototype, pk, obj),
            JsValue::BigInt(_) => self.lookup_on_proto(self.bigint_prototype, pk, obj),
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
    /// Only caches own writable data properties — SetProp never uses the
    /// `Proto` IC holder (writes don't walk the prototype chain for
    /// caching purposes).  GetProp does use `Proto` for inherited reads.
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
                upvalue_ids: fo.upvalue_ids.clone(),
                captured_this: fo.captured_this,
            })
        } else {
            None
        }
    }

    /// §9.1.10 [[Delete]] — delete a named property from an object.
    /// Returns `Ok(true)` if deleted or absent, `Ok(false)` if the property
    /// exists but is non-configurable.  Spec `[[Delete]]` never throws
    /// TypeError on non-configurable; the strict-mode throw is the `delete`
    /// operator's responsibility (§12.5.3.2), applied by the DeleteProp /
    /// DeleteElem opcodes.  Callers that implement spec abstract ops
    /// (e.g. `JSON.parse` reviver §24.5.1.3 step 7.c.i) must honor the
    /// `false` return rather than treating it as an error.
    pub(crate) fn try_delete_property(
        &mut self,
        id: ObjectId,
        pk: PropertyKey,
    ) -> Result<bool, VmError> {
        // DOMStringMap (HTMLElement.dataset) named-property exotic
        // [[Delete]] — `delete dataset.fooBar` removes the backing
        // `data-foo-bar` attribute via the `dataset.delete` handler
        // (WHATWG HTML §3.2.6 named deleter).  String keys only;
        // Symbol keys fall through to the ordinary path.
        #[cfg(feature = "engine")]
        if matches!(
            self.get_object(id).kind,
            super::value::ObjectKind::DOMStringMap { .. }
        ) {
            if let PropertyKey::String(sid) = pk {
                let result = super::host::dataset::try_delete(self, id, JsValue::String(sid));
                if let Some(r) = result {
                    return r;
                }
            }
        }
        // Check existence and configurability while still in Shaped mode
        // to avoid unnecessary Dictionary conversion.
        {
            let obj = self.objects[id.0 as usize].as_ref().unwrap();
            match obj.storage.get(pk, &self.shapes) {
                None => return Ok(true), // Property doesn't exist — delete succeeds.
                Some((_, attrs)) if !attrs.configurable => return Ok(false),
                Some(_) => {} // configurable — proceed with delete
            }
        }
        // Only convert to Dictionary when we're actually going to remove.
        self.convert_to_dictionary(id);
        let obj = self.get_object_mut(id);
        if let Some(pos) = obj.storage.dict_position(pk) {
            obj.storage.remove_dict(pos);
            if id == self.global_object {
                if let PropertyKey::String(sid) = pk {
                    self.globals.remove(&sid);
                }
            }
        }
        Ok(true)
    }

    /// §9.1.9.2 OrdinarySetWithOwnDescriptor: dispatch a set based on the own
    /// descriptor on the target, falling back to the prototype chain.  Writes
    /// funnel through `write_data_to_receiver`, which enforces §9.1.9.2
    /// step 2.b / 2.e (Receiver must be an Object).
    ///
    /// `id` is the target `O` (the lookup object, possibly a primitive
    /// wrapper from `ToObject`); `receiver` is the original base value flowing
    /// through PutValue.  For primitive receivers the two differ — data
    /// writes fail with TypeError in that case.
    pub(super) fn ordinary_set(
        &mut self,
        id: ObjectId,
        pk: PropertyKey,
        val: JsValue,
        receiver: JsValue,
    ) -> Result<SetOutcome, VmError> {
        enum OwnDesc {
            DataWritable,
            DataReadOnly,
            Setter(ObjectId),
            NoSetter,
        }

        // Step 1: read own descriptor on target (no mutation yet).
        let own = {
            let obj_ref = self.objects[id.0 as usize].as_ref().unwrap();
            obj_ref
                .storage
                .get(pk, &self.shapes)
                .map(|(slot, attrs)| match slot {
                    PropertyValue::Data(_) if attrs.writable => OwnDesc::DataWritable,
                    PropertyValue::Data(_) => OwnDesc::DataReadOnly,
                    PropertyValue::Accessor {
                        setter: Some(s), ..
                    } => OwnDesc::Setter(*s),
                    PropertyValue::Accessor { setter: None, .. } => OwnDesc::NoSetter,
                })
        };

        if let Some(desc) = own {
            return match desc {
                OwnDesc::DataWritable => self.write_data_to_receiver(pk, val, receiver),
                OwnDesc::DataReadOnly => {
                    Err(VmError::type_error("Cannot assign to read only property"))
                }
                OwnDesc::Setter(s) => {
                    self.call(s, receiver, &[val])?;
                    Ok(SetOutcome::NoDataWrite)
                }
                OwnDesc::NoSetter => Err(VmError::type_error(
                    "Cannot set property which has only a getter",
                )),
            };
        }

        // Step 2: walk prototype chain.
        match find_inherited_property(self, id, pk) {
            InheritedProperty::Setter(setter_id) => {
                self.call(setter_id, receiver, &[val])?;
                Ok(SetOutcome::NoDataWrite)
            }
            InheritedProperty::WritableFalse | InheritedProperty::AccessorNoSetter => Err(
                VmError::type_error("Cannot set property: inherited descriptor prevents it"),
            ),
            InheritedProperty::None => {
                // Step 3: nothing blocks — create an own data property on
                // Receiver (§9.1.9.2 step 2.e CreateDataProperty).
                self.write_data_to_receiver(pk, val, receiver)
            }
        }
    }

    /// Write `val` onto `receiver` as a data property (§9.1.9.2 step 2.b-e).
    /// Rejects non-Object receivers (covers the primitive-base case where
    /// PutValue routed `O := ToObject(primitive)` into `ordinary_set` while
    /// keeping `receiver` as the primitive).
    ///
    /// Updates an existing own data slot in place; otherwise creates a new
    /// own data property after checking the extensibility invariant.
    fn write_data_to_receiver(
        &mut self,
        pk: PropertyKey,
        val: JsValue,
        receiver: JsValue,
    ) -> Result<SetOutcome, VmError> {
        let JsValue::Object(recv_id) = receiver else {
            return Err(VmError::type_error(
                "Cannot set property on non-object receiver",
            ));
        };
        // Fast path: existing own data slot → write in place.  (An accessor
        // slot on the receiver diverging from the target's data descriptor
        // is a Proxy/recursive-[[Set]] scenario not yet wired up; falling
        // through to create-own keeps behavior well-defined.)
        {
            let shapes = &self.shapes;
            let obj_ref = self.objects[recv_id.0 as usize].as_mut().unwrap();
            if let Some((PropertyValue::Data(v), _)) = obj_ref.storage.get_mut(pk, shapes) {
                *v = val;
                return Ok(SetOutcome::DataWritten);
            }
        }
        if !self.get_object(recv_id).extensible {
            return Err(VmError::type_error(
                "Cannot add property to a non-extensible object",
            ));
        }
        self.define_shaped_property(
            recv_id,
            pk,
            PropertyValue::Data(val),
            super::shape::PropertyAttrs::DATA,
        );
        Ok(SetOutcome::DataWritten)
    }

    pub(crate) fn set_property_val(
        &mut self,
        obj: JsValue,
        key: StringId,
        val: JsValue,
    ) -> Result<(), VmError> {
        // §6.2.4.5 RequireObjectCoercible: writes on null/undefined throw.
        super::coerce::require_object_coercible(obj)?;
        let pk = PropertyKey::String(key);
        // §6.2.4.8 PutValue step 5.a: `? ToObject(base)` for the lookup
        // target; the original base flows through as Receiver so
        // `ordinary_set` rejects data writes when Receiver is primitive.
        let target_id = match obj {
            JsValue::Object(id) => id,
            _ => super::coerce::to_object(self, obj)?,
        };
        // DOMStringMap (HTMLElement.dataset) named-property exotic
        // [[Set]] — `dataset.fooBar = x` writes the
        // `data-foo-bar` attribute via the `dataset.set` handler.
        // Bypasses the ordinary set path because the wrapper is
        // sealed (`extensible: false`) — `ordinary_set` would
        // otherwise reject the write with "non-extensible".
        #[cfg(feature = "engine")]
        if matches!(
            self.get_object(target_id).kind,
            ObjectKind::DOMStringMap { .. }
        ) {
            if let Some(result) =
                super::host::dataset::try_set(self, target_id, JsValue::String(key), val)
            {
                return result;
            }
        }
        let is_global = target_id == self.global_object;
        let outcome = self.ordinary_set(target_id, pk, val, obj)?;
        // Sync the global variable table only when a data property was
        // actually written or created.  Accessor calls (setter / no-setter)
        // and non-writable rejections must NOT desynchronize the table.
        if is_global && matches!(outcome, SetOutcome::DataWritten) {
            self.globals.insert(key, val);
        }
        Ok(())
    }
}
