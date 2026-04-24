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

use super::ops::{parse_array_index_u16, try_as_array_index, DENSE_ARRAY_LEN_LIMIT};

/// Classification of a WTF-16 string key against the TypedArray
/// integer-indexed exotic object contract (ES §10.4.5 + §7.1.16.1
/// `CanonicalNumericIndexString`).
#[cfg(feature = "engine")]
enum TypedArrayStringKey {
    /// Non-negative integer in [0, u32::MAX].  Dispatches to
    /// `read_element_raw` / `write_element_raw`.  If the caller's
    /// length check fails, Get returns `undefined` and Set is a
    /// silent no-op.
    IntegerIndex(u32),
    /// Canonical numeric string that is NOT a valid integer index
    /// (`"-0"`, `"Infinity"`, `"-Infinity"`, `"NaN"`, negative
    /// integer, fractional, exponential with canonical round-trip).
    /// TypedArray Get returns `undefined`; Set is a silent no-op;
    /// neither creates an ordinary property (§10.4.5.15 step 3 /
    /// §10.4.5.16 step 1).
    CanonicalNonInteger,
    /// Not a canonical numeric string — falls through to ordinary
    /// property storage.
    NotNumeric,
}

/// Classify a Number key against the TypedArray integer-indexed
/// exotic contract.  A Number key `n` is treated as if ToString'd —
/// non-negative integers up to `u32::MAX` map to their index; all
/// other numeric forms (`NaN`, ±`Infinity`, negative, fractional,
/// out-of-u32-range) are canonical-numeric-but-not-integer, which
/// §10.4.5.15/16 short-circuit to `undefined` / silent no-op.
#[cfg(feature = "engine")]
fn classify_typed_array_number_key(n: f64) -> TypedArrayStringKey {
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    {
        if n.is_finite() && n >= 0.0 && n <= f64::from(u32::MAX) {
            let as_u32 = n as u32;
            if f64::from(as_u32) == n {
                return TypedArrayStringKey::IntegerIndex(as_u32);
            }
        }
    }
    TypedArrayStringKey::CanonicalNonInteger
}

/// Parse a WTF-16 string as a TypedArray integer index (0..=u32::MAX).
/// Distinct from `parse_array_index_u16` (capped at 2^32−2 per the
/// Array `[[HasOwnProperty]]` contract); TypedArray permits the full
/// u32 range.
#[cfg(feature = "engine")]
fn parse_typed_array_index_u32(units: &[u16]) -> Option<u32> {
    if units.is_empty() {
        return None;
    }
    if units.len() > 1 && units[0] == u16::from(b'0') {
        return None;
    }
    let mut n: u64 = 0;
    for &u in units {
        let digit = u.wrapping_sub(u16::from(b'0'));
        if digit > 9 {
            return None;
        }
        n = n.checked_mul(10)?.checked_add(u64::from(digit))?;
    }
    u32::try_from(n).ok()
}

#[cfg(feature = "engine")]
fn classify_typed_array_string_key(vm: &VmInner, sid: StringId) -> TypedArrayStringKey {
    let units = vm.strings.get(sid);
    if let Some(idx) = parse_typed_array_index_u32(units) {
        return TypedArrayStringKey::IntegerIndex(idx);
    }
    // ES §7.1.16.1 step 1 hard-codes `"-0"` as canonical numeric
    // (returns -0) even though ToString(-0) = "0" — the round-trip
    // check below would otherwise miss it.
    if units == [u16::from(b'-'), u16::from(b'0')] {
        return TypedArrayStringKey::CanonicalNonInteger;
    }
    // Slow path: round-trip via ES Number::toString.  If
    // ToString(ToNumber(key)) == key, the key is canonical numeric
    // (the fast path above already handles non-negative integers, so
    // any remaining canonical form — `"Infinity"`, `"NaN"`, negative
    // integer, fractional — is non-integer-valid).
    let n = super::coerce::to_number(vm, JsValue::String(sid)).unwrap_or(f64::NAN);
    let mut roundtrip = String::new();
    if n.is_nan() {
        roundtrip.push_str("NaN");
    } else if n.is_infinite() {
        roundtrip.push_str(if n > 0.0 { "Infinity" } else { "-Infinity" });
    } else {
        super::coerce_format::write_number_es(n, &mut roundtrip);
    }
    if units.len() == roundtrip.len()
        && units
            .iter()
            .zip(roundtrip.as_bytes())
            .all(|(&u, &b)| u == u16::from(b))
    {
        TypedArrayStringKey::CanonicalNonInteger
    } else {
        TypedArrayStringKey::NotNumeric
    }
}

// ---------------------------------------------------------------------------
// Property access
// ---------------------------------------------------------------------------

/// Whether `ordinary_set` wrote or created an own data property.
/// Used by `set_property_val` to decide whether to sync `globals`.
/// Note: setter calls are ES2020-successful but do NOT produce a
/// `DataWritten` result, because the setter controls its own writes.
enum SetOutcome {
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
        // §6.2.4.5 RequireObjectCoercible: property reads on null/undefined
        // must throw TypeError before any prototype-chain walk.
        super::coerce::require_object_coercible(obj)?;
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
    fn ordinary_set(
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

    #[allow(clippy::too_many_lines)]
    pub(crate) fn get_element(&mut self, obj: JsValue, key: JsValue) -> Result<JsValue, VmError> {
        // §6.2.4.5 RequireObjectCoercible: `null[key]` / `undefined[key]` throw.
        super::coerce::require_object_coercible(obj)?;
        if let JsValue::Object(id) = obj {
            // TypedArray integer-indexed get (ES §10.4.5.15).  Must
            // run ahead of the generic numeric fast path so any
            // CanonicalNumericIndexString Number key — including
            // `NaN` / ±`Infinity` / negative / fractional /
            // out-of-u32-range values rejected by `try_as_array_index`
            // — short-circuits to `undefined` without consulting
            // ordinary properties or the prototype chain.
            #[cfg(feature = "engine")]
            if let JsValue::Number(n) = key {
                if let ObjectKind::TypedArray {
                    buffer_id,
                    byte_offset,
                    byte_length,
                    element_kind,
                } = self.get_object(id).kind
                {
                    match classify_typed_array_number_key(n) {
                        TypedArrayStringKey::IntegerIndex(i) => {
                            let len_elem =
                                byte_length / u32::from(element_kind.bytes_per_element());
                            if i < len_elem {
                                return Ok(super::host::typed_array::read_element_raw(
                                    self,
                                    buffer_id,
                                    byte_offset,
                                    i,
                                    element_kind,
                                ));
                            }
                            return Ok(JsValue::Undefined);
                        }
                        TypedArrayStringKey::CanonicalNonInteger => return Ok(JsValue::Undefined),
                        TypedArrayStringKey::NotNumeric => {}
                    }
                }
            }
            // Numeric index for arrays / Arguments / StringWrapper.
            if let JsValue::Number(n) = key {
                if let Some(idx) = try_as_array_index(n) {
                    let obj_ref = self.get_object(id);
                    match &obj_ref.kind {
                        ObjectKind::Array { ref elements } => {
                            let elem = elements.get(idx).copied().unwrap_or(JsValue::Empty);
                            if !elem.is_empty() {
                                return Ok(elem);
                            }
                            // Hole or out-of-range: fall through to property/prototype lookup.
                        }
                        ObjectKind::Arguments { ref values } if idx < values.len() => {
                            return Ok(values[idx]);
                        }
                        ObjectKind::StringWrapper(sid) => {
                            if let Some(&unit) = self.strings.get(*sid).get(idx) {
                                let ch_id = self.strings.intern_utf16(&[unit]);
                                return Ok(JsValue::String(ch_id));
                            }
                        }
                        _ => {}
                    }
                }
            }

            // PR5b §C3: HTMLCollection / NodeList indexed + legacy
            // named property access.  Delegates to a shared helper
            // that re-traverses the backing filter and resolves
            // both numeric indices and (HTMLCollection-only)
            // `id` / tag-allowlisted `name` lookups.  Falls through
            // to the standard property / prototype lookup on miss
            // so that `.length` / `.item` still see the prototype
            // accessor / method.
            #[cfg(feature = "engine")]
            {
                let kind_snapshot = &self.get_object(id).kind;
                let is_live_collection = matches!(
                    kind_snapshot,
                    ObjectKind::HtmlCollection | ObjectKind::NodeList
                );
                let is_named_node_map = matches!(kind_snapshot, ObjectKind::NamedNodeMap);
                if is_live_collection || is_named_node_map {
                    // Two-phase lookup:
                    //
                    //   1. With a shared `&EcsDom` borrow (obtained
                    //      through a raw-pointer detach from
                    //      `HostData::dom_shared`), call the typed
                    //      helper to produce an `Entity` (for live
                    //      collections) or `(owner, qname_sid)`
                    //      (for NamedNodeMap).  The helper must not
                    //      itself allocate wrappers — doing so
                    //      would mutably reborrow `host_data`
                    //      (`wrapper_cache` / `attr_states`) while
                    //      the `&EcsDom` derived from the same
                    //      `HostData` reborrow chain is still live,
                    //      a Stacked Borrows violation.
                    //   2. Drop the `&EcsDom` borrow, then allocate
                    //      the wrapper on the clean `&mut VmInner`.
                    //
                    // `dom_shared()` panics when `HostData` is
                    // unbound.  Collection wrappers can outlive
                    // `Vm::unbind()` when they remain reachable
                    // from ordinary JS roots (e.g. `globalThis.hc
                    // = ...`); the side tables
                    // (`live_collection_states` /
                    // `named_node_map_states`) are NOT GC roots —
                    // they are pruned after the mark phase based on
                    // whether the key `ObjectId` was itself marked.
                    // Post-unbind indexed access on a retained
                    // wrapper therefore falls through to normal
                    // prototype lookup rather than panicking.
                    let entity_hit: Option<elidex_ecs::Entity>;
                    let nnm_hit: Option<(elidex_ecs::Entity, super::value::StringId)>;
                    if let Some(hd) = self.host_data.as_deref().filter(|h| h.is_bound()) {
                        #[allow(unsafe_code)]
                        let dom_ptr: *const elidex_ecs::EcsDom = hd.dom_shared();
                        #[allow(unsafe_code)]
                        let dom = unsafe { &*dom_ptr };
                        if is_live_collection {
                            entity_hit =
                                super::host::dom_collection::try_indexed_get(self, dom, id, key);
                            nnm_hit = None;
                        } else {
                            entity_hit = None;
                            nnm_hit =
                                super::host::named_node_map::try_indexed_get(self, dom, id, key);
                        }
                        // `dom` / `dom_ptr` fall out of scope here
                        // — subsequent wrapper allocation runs with
                        // no outstanding DOM borrow aliasing
                        // `host_data`.
                    } else {
                        entity_hit = None;
                        nnm_hit = None;
                    }
                    if let Some(e) = entity_hit {
                        return Ok(JsValue::Object(self.create_element_wrapper(e)));
                    }
                    if let Some((owner, qname_sid)) = nnm_hit {
                        let attr_id = self.alloc_attr(super::host::attr_proto::AttrState {
                            owner,
                            qualified_name: qname_sid,
                            detached_value: None,
                        });
                        return Ok(JsValue::Object(attr_id));
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
            // StringWrapper: index access and length
            if let ObjectKind::StringWrapper(sid) = self.get_object(id).kind {
                if key_id == self.well_known.length {
                    #[allow(clippy::cast_precision_loss)]
                    let len = self.strings.get(sid).len() as f64;
                    return Ok(JsValue::Number(len));
                }
                let key_units = self.strings.get(key_id);
                if let Some(idx) = parse_array_index_u16(key_units) {
                    if let Some(&unit) = self.strings.get(sid).get(idx) {
                        let ch_id = self.strings.intern_utf16(&[unit]);
                        return Ok(JsValue::String(ch_id));
                    }
                }
            }
            // String numeric key on TypedArray — ES §10.4.5 integer-
            // indexed exotic dispatch.  Any CanonicalNumericIndexString
            // (§7.1.16.1) — including `"-0"` / `"Infinity"` / `"NaN"` /
            // negative integer / fractional — short-circuits to
            // `undefined` rather than falling through to ordinary
            // property access.
            #[cfg(feature = "engine")]
            {
                if let ObjectKind::TypedArray {
                    buffer_id,
                    byte_offset,
                    byte_length,
                    element_kind,
                } = self.get_object(id).kind
                {
                    match classify_typed_array_string_key(self, key_id) {
                        TypedArrayStringKey::IntegerIndex(i) => {
                            let len_elem =
                                byte_length / u32::from(element_kind.bytes_per_element());
                            if i < len_elem {
                                return Ok(super::host::typed_array::read_element_raw(
                                    self,
                                    buffer_id,
                                    byte_offset,
                                    i,
                                    element_kind,
                                ));
                            }
                            return Ok(JsValue::Undefined);
                        }
                        TypedArrayStringKey::CanonicalNonInteger => {
                            return Ok(JsValue::Undefined);
                        }
                        TypedArrayStringKey::NotNumeric => {}
                    }
                }
            }
            // String key that parses as array index → check elements first.
            if matches!(self.get_object(id).kind, ObjectKind::Array { .. }) {
                let key_units = self.strings.get(key_id);
                if let Some(idx) = parse_array_index_u16(key_units) {
                    let elem = {
                        let obj_ref = self.get_object(id);
                        if let ObjectKind::Array { ref elements } = obj_ref.kind {
                            elements.get(idx).copied().unwrap_or(JsValue::Empty)
                        } else {
                            JsValue::Empty
                        }
                    };
                    if !elem.is_empty() {
                        return Ok(elem);
                    }
                    // Hole: fall through to property/prototype lookup.
                }
            }
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
        } else if matches!(
            obj,
            JsValue::Number(_) | JsValue::Boolean(_) | JsValue::BigInt(_)
        ) {
            let proto = match obj {
                JsValue::Number(_) => self.number_prototype,
                JsValue::BigInt(_) => self.bigint_prototype,
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

    /// Check whether an array element write at `idx` should be rejected
    /// due to non-extensible / frozen constraints. Returns `Some(result)`
    /// to early-return from the caller, or `None` to proceed.
    fn check_array_element_write(
        &self,
        obj_id: super::value::ObjectId,
        idx: usize,
    ) -> Option<Result<(), VmError>> {
        let obj = self.get_object(obj_id);
        if !matches!(obj.kind, ObjectKind::Array { .. }) || obj.extensible {
            return None;
        }
        let is_new = match &obj.kind {
            ObjectKind::Array { elements } => {
                idx >= elements.len() || elements.get(idx).is_some_and(|v| v.is_empty())
            }
            _ => false,
        };
        // Frozen = non-extensible + all named properties are non-writable+non-configurable.
        // Requires at least one named property to distinguish from preventExtensions.
        let mut has_named_props = false;
        let is_frozen = !is_new
            && obj.storage.iter_keys(&self.shapes).all(|(_, attrs)| {
                has_named_props = true;
                !attrs.configurable && (attrs.is_accessor || !attrs.writable)
            })
            && has_named_props;
        if is_new || is_frozen {
            return Some(Err(VmError::type_error(
                "Cannot assign to read only property",
            )));
        }
        None
    }

    /// TypedArray integer-indexed-write fast path (ES §10.4.5.16
    /// `IntegerIndexedElementSet`).  Returns `Some(Ok(()))` when the
    /// receiver is a TypedArray and `key` resolves to a canonical
    /// integer index (in-range write or silent out-of-range no-op);
    /// `Some(Err(…))` on coercion failure; `None` to defer to the
    /// ordinary property path.  Keeps `set_element` under the
    /// 100-line clippy limit while preserving the required
    /// precedence ahead of Array / Arguments dispatch.
    #[cfg(feature = "engine")]
    fn try_typed_array_element_set(
        &mut self,
        id: super::value::ObjectId,
        key: JsValue,
        val: JsValue,
    ) -> Option<Result<(), VmError>> {
        let (buffer_id, byte_offset, byte_length, element_kind) = match self.get_object(id).kind {
            ObjectKind::TypedArray {
                buffer_id,
                byte_offset,
                byte_length,
                element_kind,
            } => (buffer_id, byte_offset, byte_length, element_kind),
            _ => return None,
        };
        // Resolve a canonical integer index.  Non-canonical strings
        // (`"01"`, `"1.5e2"`) fall through to ordinary property
        // storage; canonical forms that are NOT valid integer
        // indices — `NaN` / ±`Infinity` / negative / fractional /
        // out-of-u32-range — are silent no-ops per §10.4.5.16 step 1
        // and must NOT surface as ordinary own properties.  Objects
        // with a custom `toString` routing to a canonical numeric
        // index string flow through the generic `ToString` branch
        // below; Symbols bypass the TypedArray exotic path and land
        // on ordinary own properties (§10.4.5 only specialises
        // Strings).
        let idx: u32 = match key {
            JsValue::Number(n) => match classify_typed_array_number_key(n) {
                TypedArrayStringKey::IntegerIndex(i) => i,
                TypedArrayStringKey::CanonicalNonInteger => return Some(Ok(())),
                TypedArrayStringKey::NotNumeric => return None,
            },
            JsValue::String(sid) => match classify_typed_array_string_key(self, sid) {
                TypedArrayStringKey::IntegerIndex(i) => i,
                TypedArrayStringKey::CanonicalNonInteger => return Some(Ok(())),
                TypedArrayStringKey::NotNumeric => return None,
            },
            JsValue::Symbol(_) => return None,
            other => {
                let sid = match to_string(self, other) {
                    Ok(sid) => sid,
                    Err(err) => return Some(Err(err)),
                };
                match classify_typed_array_string_key(self, sid) {
                    TypedArrayStringKey::IntegerIndex(i) => i,
                    TypedArrayStringKey::CanonicalNonInteger => return Some(Ok(())),
                    TypedArrayStringKey::NotNumeric => return None,
                }
            }
        };
        let len_elem = byte_length / u32::from(element_kind.bytes_per_element());
        if idx >= len_elem {
            // Canonical integer but out-of-range → silent no-op
            // (§10.4.5.16 step 1).  Does NOT create an own
            // ordinary property.
            return Some(Ok(()));
        }
        // In-range: coerce through `write_element_raw` (handles
        // ToBigInt / ToInt* / float encoding per `element_kind`).
        let mut ctx = super::value::NativeContext { vm: self };
        Some(super::host::typed_array::write_element_raw(
            &mut ctx,
            buffer_id,
            byte_offset,
            idx,
            element_kind,
            val,
        ))
    }

    pub(crate) fn set_element(
        &mut self,
        obj: JsValue,
        key: JsValue,
        val: JsValue,
    ) -> Result<(), VmError> {
        // §6.2.4.5 RequireObjectCoercible: `null[k] = v` / `undefined[k] = v` throw.
        super::coerce::require_object_coercible(obj)?;
        if let JsValue::Object(id) = obj {
            // TypedArray integer-indexed write dispatches ahead of
            // the Array / Arguments fast path — see
            // `try_typed_array_element_set` for rationale.
            #[cfg(feature = "engine")]
            if let Some(result) = self.try_typed_array_element_set(id, key, val) {
                return result;
            }
            // Numeric key → Array/Arguments dense-storage fast path.
            if let JsValue::Number(n) = key {
                if let Some(idx) = try_as_array_index(n) {
                    // Check extensible/frozen before taking mutable borrow.
                    if let Some(reject) = self.check_array_element_write(id, idx) {
                        return reject;
                    }
                    let obj_ref = self.get_object_mut(id);
                    match &mut obj_ref.kind {
                        ObjectKind::Array { ref mut elements } => {
                            if idx >= elements.len() {
                                if idx >= DENSE_ARRAY_LEN_LIMIT {
                                    return Err(VmError::range_error("Array allocation failed"));
                                }
                                let new_len = idx + 1;
                                elements
                                    .try_reserve(new_len - elements.len())
                                    .map_err(|_| VmError::range_error("Array allocation failed"))?;
                                elements.resize(new_len, JsValue::Empty);
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
            // Symbol key → §9.1.9 OrdinarySet directly (no string conversion).
            if let JsValue::Symbol(sid) = key {
                let pk = PropertyKey::Symbol(sid);
                self.ordinary_set(id, pk, val, obj)?;
                return Ok(());
            }
            let key_id = to_string(self, key)?;
            // Numeric-string key on Array → dense-storage fast path.
            if matches!(self.get_object(id).kind, ObjectKind::Array { .. }) {
                let key_units = self.strings.get(key_id);
                if let Some(idx) = parse_array_index_u16(key_units) {
                    if let Some(reject) = self.check_array_element_write(id, idx) {
                        return reject;
                    }
                    let obj_ref = self.get_object_mut(id);
                    if let ObjectKind::Array { ref mut elements } = obj_ref.kind {
                        if idx >= elements.len() {
                            if idx >= DENSE_ARRAY_LEN_LIMIT {
                                return Err(VmError::range_error("Array allocation failed"));
                            }
                            let new_len = idx + 1;
                            elements
                                .try_reserve(new_len - elements.len())
                                .map_err(|_| VmError::range_error("Array allocation failed"))?;
                            elements.resize(new_len, JsValue::Empty);
                        }
                        elements[idx] = val;
                        return Ok(());
                    }
                }
            }
            return self.set_property_val(obj, key_id, val);
        }

        // Primitive base (after RequireObjectCoercible): box for descriptor
        // lookup per §6.2.4.8 PutValue step 5.a, keeping the original base
        // as Receiver so `ordinary_set` rejects data writes via §9.1.9.2
        // step 2.b.  (Array-style fast paths don't apply: primitive
        // wrappers are never `ObjectKind::Array`.)
        if let JsValue::Symbol(sid) = key {
            let target = super::coerce::to_object(self, obj)?;
            self.ordinary_set(target, PropertyKey::Symbol(sid), val, obj)?;
            return Ok(());
        }
        let key_id = to_string(self, key)?;
        self.set_property_val(obj, key_id, val)
    }
}
