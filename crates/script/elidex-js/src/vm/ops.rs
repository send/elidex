//! VM operation helpers: property access, function calls, exception handling,
//! upvalue management, and operator helpers.

use super::coerce::{
    abstract_relational, find_inherited_property, get_property, op_bitwise, op_numeric_binary,
    to_boolean, to_number, to_string, BitwiseOp, InheritedProperty, NumericBinaryOp,
    PropertyResult,
};
use super::value::{
    FuncId, JsValue, ObjectId, ObjectKind, Property, PropertyKey, PropertyValue, StringId, Upvalue,
    UpvalueState, VmError, VmErrorKind,
};
use super::VmInner;
use crate::bytecode::compiled::Constant;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a WTF-16 string as a non-negative integer array index (e.g. "0", "42").
/// Returns `None` for empty strings, leading zeros (except "0"), non-digit chars,
/// or overflow.
pub(crate) fn parse_array_index_u16(units: &[u16]) -> Option<usize> {
    if units.is_empty() {
        return None;
    }
    // Reject leading zeros (except "0" itself).
    if units.len() > 1 && units[0] == u16::from(b'0') {
        return None;
    }
    let mut n: usize = 0;
    for &u in units {
        let digit = u.wrapping_sub(u16::from(b'0'));
        if digit > 9 {
            return None;
        }
        n = n.checked_mul(10)?.checked_add(digit as usize)?;
    }
    Some(n)
}

// ---------------------------------------------------------------------------
// Error-to-thrown-value conversion
// ---------------------------------------------------------------------------

impl VmInner {
    /// Create a thrown JS error object from a `VmError` and attempt to dispatch
    /// it through the exception handling chain.  Returns `Ok(())` if the
    /// exception was caught (caller should `continue` the dispatch loop) or
    /// `Err(error)` if no handler exists (caller should `return Err`).
    pub(crate) fn throw_error(
        &mut self,
        error: VmError,
        entry_frame_depth: usize,
    ) -> Result<(), VmError> {
        let thrown = self.vm_error_to_thrown(&error);
        if self.handle_exception(thrown, entry_frame_depth) {
            Ok(()) // caught — caller continues
        } else {
            Err(error) // uncaught
        }
    }

    /// Convert a `VmError` into a `JsValue` suitable for `handle_exception`.
    /// `ThrowValue` errors pass through; other runtime errors are wrapped
    /// in a proper Error object (TypeError, ReferenceError, etc.).
    pub(crate) fn vm_error_to_thrown(&mut self, error: &VmError) -> JsValue {
        match &error.kind {
            VmErrorKind::ThrowValue(val) => *val,
            kind => {
                let error_name = match kind {
                    VmErrorKind::TypeError => "TypeError",
                    VmErrorKind::ReferenceError => "ReferenceError",
                    VmErrorKind::RangeError => "RangeError",
                    VmErrorKind::SyntaxError => "SyntaxError",
                    _ => "Error",
                };
                let name_id = self.strings.intern(error_name);
                let msg_id = self.strings.intern(&error.message);
                let error_obj = self.alloc_object(super::value::Object {
                    kind: ObjectKind::Error { name: name_id },
                    properties: vec![
                        (
                            PropertyKey::String(self.well_known.name),
                            Property::data(JsValue::String(name_id)),
                        ),
                        (
                            PropertyKey::String(self.well_known.message),
                            Property::data(JsValue::String(msg_id)),
                        ),
                    ],
                    prototype: self.object_prototype,
                });
                JsValue::Object(error_obj)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ToPrimitive / op_add (ES2020 §7.1.1, §12.8.3)
// ---------------------------------------------------------------------------

impl VmInner {
    /// ToPrimitive (ES2020 §7.1.1). Checks `@@toPrimitive` on objects and calls
    /// it if present. Falls back to OrdinaryToPrimitive (simplified: returns
    /// `"[object Object]"`).
    #[allow(clippy::wrong_self_convention)] // matches ES2020 abstract operation name
    pub(crate) fn to_primitive(&mut self, val: JsValue, hint: &str) -> Result<JsValue, VmError> {
        match val {
            JsValue::Object(obj_id) => {
                // Unwrap primitive wrapper objects first.
                match self.get_object(obj_id).kind {
                    ObjectKind::NumberWrapper(n) => return Ok(JsValue::Number(n)),
                    ObjectKind::StringWrapper(s) => return Ok(JsValue::String(s)),
                    ObjectKind::BooleanWrapper(b) => return Ok(JsValue::Boolean(b)),
                    _ => {}
                }
                // §7.1.1 step 2d: Check @@toPrimitive
                let to_prim_key = PropertyKey::Symbol(self.well_known_symbols.to_primitive);
                let exotic_to_prim = match get_property(self, obj_id, to_prim_key) {
                    Some(PropertyResult::Data(v)) => Some(v),
                    Some(PropertyResult::Getter(g)) => Some(self.call(g, val, &[])?),
                    None => None,
                };
                if let Some(exotic_to_prim) = exotic_to_prim {
                    let hint_id = self.strings.intern(hint);
                    let result =
                        self.call_value(exotic_to_prim, val, &[JsValue::String(hint_id)])?;
                    if matches!(result, JsValue::Object(_)) {
                        return Err(VmError::type_error(
                            "Cannot convert object to primitive value",
                        ));
                    }
                    return Ok(result);
                }
                // OrdinaryToPrimitive: simplified — return "[object Object]"
                Ok(JsValue::String(self.well_known.object_to_string))
            }
            // Symbols (and all other primitives) are already primitive.
            other => Ok(other),
        }
    }

    /// The `+` operator (ES2020 §12.8.3). Handles both addition and string
    /// concatenation, calling `ToPrimitive` which may invoke `@@toPrimitive`.
    pub(crate) fn op_add(&mut self, lhs: JsValue, rhs: JsValue) -> Result<JsValue, VmError> {
        let lhs = self.to_primitive(lhs, "default")?;
        let rhs = self.to_primitive(rhs, "default")?;
        // If either operand is a string, concatenate.
        if matches!(lhs, JsValue::String(_)) || matches!(rhs, JsValue::String(_)) {
            let ls = to_string(self, lhs)?;
            let rs = to_string(self, rhs)?;
            let left = self.strings.get(ls);
            let right = self.strings.get(rs);
            let mut concat: Vec<u16> = Vec::with_capacity(left.len() + right.len());
            concat.extend_from_slice(left);
            concat.extend_from_slice(right);
            let id = self.strings.intern_utf16(&concat);
            return Ok(JsValue::String(id));
        }
        let a = to_number(self, lhs)?;
        let b = to_number(self, rhs)?;
        Ok(JsValue::Number(a + b))
    }
}

// ---------------------------------------------------------------------------
// Operator helpers
// ---------------------------------------------------------------------------

impl VmInner {
    pub(crate) fn binary_numeric(&mut self, op: NumericBinaryOp) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        let r = op_numeric_binary(self, a, b, op)?;
        self.stack.push(r);
        Ok(())
    }

    pub(crate) fn binary_bitwise(&mut self, op: BitwiseOp) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        let r = op_bitwise(self, a, b, op)?;
        self.stack.push(r);
        Ok(())
    }

    pub(crate) fn relational_op(&mut self, swap: bool, eq: bool) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = if eq {
            // x <= y  ===  !(y < x)
            // x >= y  ===  !(x < y)  (with swap)
            let (lhs, rhs) = if swap { (a, b) } else { (b, a) };
            match abstract_relational(self, lhs, rhs, swap)? {
                Some(false) => true,        // !(y < x) → <=
                Some(true) | None => false, // y < x, or NaN
            }
        } else {
            // x < y  or  x > y (with swap)
            let (lhs, rhs) = if swap { (b, a) } else { (a, b) };
            abstract_relational(self, lhs, rhs, !swap)?.unwrap_or(false)
        };
        self.stack.push(JsValue::Boolean(result));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Property key conversion
// ---------------------------------------------------------------------------

impl VmInner {
    /// Convert a `JsValue` to a `PropertyKey`, preserving symbols (ES2020 §7.1.14 ToPropertyKey).
    pub(crate) fn make_property_key(&mut self, key: JsValue) -> Result<PropertyKey, VmError> {
        match key {
            JsValue::Symbol(sid) => Ok(PropertyKey::Symbol(sid)),
            other => Ok(PropertyKey::String(to_string(self, other)?)),
        }
    }
}

// ---------------------------------------------------------------------------
// Exception handling & frame management
// ---------------------------------------------------------------------------

impl VmInner {
    /// Try to handle an exception by finding a handler in the current or parent frames.
    /// Returns `true` if a handler was found and ip was redirected, `false` if unhandled.
    pub(crate) fn handle_exception(
        &mut self,
        thrown_value: JsValue,
        entry_frame_depth: usize,
    ) -> bool {
        self.current_exception = thrown_value;

        // Search from the current frame outward.
        loop {
            if self.frames.is_empty() {
                return false;
            }
            let frame_idx = self.frames.len() - 1;

            // Check if this frame has a handler.
            if let Some(handler) = self.frames[frame_idx].exception_handlers.pop() {
                // Unwind stack to the handler's recorded depth.
                self.stack.truncate(handler.stack_depth);

                // Jump to catch block if present, otherwise finally.
                if handler.catch_ip != u32::MAX {
                    self.frames[frame_idx].ip = handler.catch_ip as usize;
                } else if handler.finally_ip != u32::MAX {
                    self.frames[frame_idx].ip = handler.finally_ip as usize;
                } else {
                    // Neither catch nor finally — shouldn't happen but continue unwinding.
                    continue;
                }
                return true;
            }

            // No handler in this frame — pop it and try the parent.
            if frame_idx <= entry_frame_depth {
                return false;
            }
            let frame = self.frames.pop().unwrap();
            self.close_upvalues(&frame.local_upvalue_ids);
            self.stack.truncate(frame.base);
        }
    }

    pub(crate) fn pop_frame(&mut self) {
        if let Some(frame) = self.frames.pop() {
            // Close open upvalues that capture this frame's local slots.
            self.close_upvalues(&frame.local_upvalue_ids);
            // Truncate stack to frame base.
            self.stack.truncate(frame.base);
        }
    }

    pub(crate) fn close_upvalues(&mut self, upvalue_ids: &[super::value::UpvalueId]) {
        for &uv_id in upvalue_ids {
            if let UpvalueState::Open { frame_base, slot } = self.upvalues[uv_id.0 as usize].state {
                let val = self.stack[frame_base + slot as usize];
                self.upvalues[uv_id.0 as usize].state = UpvalueState::Closed(val);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Upvalue read/write
// ---------------------------------------------------------------------------

impl VmInner {
    pub(crate) fn read_upvalue(&self, uv_id: super::value::UpvalueId) -> JsValue {
        match self.upvalues[uv_id.0 as usize].state {
            UpvalueState::Open { frame_base, slot } => self.stack[frame_base + slot as usize],
            UpvalueState::Closed(val) => val,
        }
    }

    pub(crate) fn write_upvalue(&mut self, uv_id: super::value::UpvalueId, val: JsValue) {
        match self.upvalues[uv_id.0 as usize].state {
            UpvalueState::Open { frame_base, slot } => {
                self.stack[frame_base + slot as usize] = val;
            }
            UpvalueState::Closed(ref mut v) => {
                *v = val;
            }
        }
    }
}

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
        let obj = self.get_object_mut(id);
        if let Some(pos) = obj.properties.iter().position(|(k, _)| *k == pk) {
            if !obj.properties[pos].1.configurable {
                if self.is_strict_mode() {
                    return Err(VmError::type_error(
                        "Cannot delete property: property is not configurable",
                    ));
                }
                return Ok(false);
            }
            obj.properties.swap_remove(pos);
            // Sync global object deletes to the globals HashMap.
            if id == self.global_object {
                if let PropertyKey::String(sid) = pk {
                    self.globals.remove(&sid);
                }
            }
            Ok(true)
        } else {
            Ok(true) // Property doesn't exist — delete succeeds.
        }
    }

    pub(crate) fn set_property_val(
        &mut self,
        obj: JsValue,
        key: StringId,
        val: JsValue,
    ) -> Result<(), VmError> {
        let pk = PropertyKey::String(key);
        if let JsValue::Object(id) = obj {
            let is_strict = self.is_strict_mode();
            // §9.1.9 OrdinarySet: check prototype chain for inherited properties.
            match find_inherited_property(self, id, pk) {
                InheritedProperty::Setter(setter_id) => {
                    self.call(setter_id, obj, &[val])?;
                    return Ok(());
                }
                InheritedProperty::WritableFalse | InheritedProperty::AccessorNoSetter => {
                    if is_strict {
                        return Err(VmError::type_error(
                            "Cannot set property: inherited descriptor prevents it",
                        ));
                    }
                    return Ok(());
                }
                InheritedProperty::None => {}
            }
            let is_global = id == self.global_object;
            let obj = self.get_object_mut(id);
            for prop in &mut obj.properties {
                if prop.0 == pk {
                    match &prop.1.slot {
                        PropertyValue::Data(_) if prop.1.writable => {
                            prop.1.slot = PropertyValue::Data(val);
                            if is_global {
                                self.globals.insert(key, val);
                            }
                        }
                        PropertyValue::Data(_) => {
                            if is_strict {
                                return Err(VmError::type_error(
                                    "Cannot assign to read only property",
                                ));
                            }
                        }
                        PropertyValue::Accessor {
                            setter: Some(s), ..
                        } => {
                            // Own accessor with setter: invoke it.
                            let setter_id = *s;
                            let receiver = JsValue::Object(id);
                            self.call(setter_id, receiver, &[val])?;
                        }
                        PropertyValue::Accessor { setter: None, .. } => {
                            // Own accessor without setter: reject.
                            if is_strict {
                                return Err(VmError::type_error(
                                    "Cannot set property which has only a getter",
                                ));
                            }
                        }
                    }
                    return Ok(());
                }
            }
            obj.properties.push((pk, Property::data(val)));
            if is_global {
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
            // Symbol key → direct property lookup.
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
            // Symbol key → direct property set.
            if let JsValue::Symbol(sid) = key {
                let pk = PropertyKey::Symbol(sid);
                let is_strict = self.is_strict_mode();
                // §9.1.9: check prototype chain for inherited constraints.
                match find_inherited_property(self, id, pk) {
                    InheritedProperty::Setter(setter_id) => {
                        self.call(setter_id, obj, &[val])?;
                        return Ok(());
                    }
                    InheritedProperty::WritableFalse | InheritedProperty::AccessorNoSetter => {
                        if is_strict {
                            return Err(VmError::type_error(
                                "Cannot set property: inherited descriptor prevents it",
                            ));
                        }
                        return Ok(());
                    }
                    InheritedProperty::None => {}
                }
                let obj_ref = self.get_object_mut(id);
                for prop in &mut obj_ref.properties {
                    if prop.0 == pk {
                        match &prop.1.slot {
                            PropertyValue::Data(_) if prop.1.writable => {
                                prop.1.slot = PropertyValue::Data(val);
                            }
                            PropertyValue::Data(_) if is_strict => {
                                return Err(VmError::type_error(
                                    "Cannot assign to read only property",
                                ));
                            }
                            PropertyValue::Accessor {
                                setter: Some(s), ..
                            } => {
                                let setter_id = *s;
                                self.call(setter_id, obj, &[val])?;
                            }
                            PropertyValue::Accessor { setter: None, .. } if is_strict => {
                                return Err(VmError::type_error(
                                    "Cannot set property which has only a getter",
                                ));
                            }
                            _ => {}
                        }
                        return Ok(());
                    }
                }
                obj_ref.properties.push((pk, Property::data(val)));
                return Ok(());
            }
            let key_id = to_string(self, key)?;
            self.set_property_val(JsValue::Object(id), key_id, val)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Iterator helper
// ---------------------------------------------------------------------------

impl VmInner {
    /// Call `iterator.next()` and return `Some(value)` if not done,
    /// or `None` when the iterator is exhausted (`done=true`).
    /// Returns `Err(TypeError)` for protocol violations.
    pub(crate) fn iter_next(&mut self, iter_val: JsValue) -> Result<Option<JsValue>, VmError> {
        let JsValue::Object(iter_id) = iter_val else {
            return Err(VmError::type_error("iterator value is not an object"));
        };

        // Generic iterator protocol (.next() call + {value, done}).
        // No fast paths — optimise in M4-11 (inline caches) where we can
        // safely guard against `.next` override.
        let next_key = PropertyKey::String(self.well_known.next);
        let next_fn = match get_property(self, iter_id, next_key) {
            Some(result) => self.resolve_property(result, iter_val)?,
            None => return Err(VmError::type_error("iterator.next is not defined")),
        };
        let result = self.call_value(next_fn, iter_val, &[])?;
        let JsValue::Object(result_id) = result else {
            return Err(VmError::type_error("iterator.next() must return an object"));
        };
        let done_key = PropertyKey::String(self.well_known.done);
        let done = match get_property(self, result_id, done_key) {
            Some(result) => self.resolve_property(result, JsValue::Object(result_id))?,
            None => JsValue::Boolean(false),
        };
        if to_boolean(self, done) {
            return Ok(None);
        }
        let value_key = PropertyKey::String(self.well_known.value);
        let value = match get_property(self, result_id, value_key) {
            Some(result) => self.resolve_property(result, JsValue::Object(result_id))?,
            None => JsValue::Undefined,
        };
        Ok(Some(value))
    }
}

// ---------------------------------------------------------------------------
// Function calls & closures
// ---------------------------------------------------------------------------

impl VmInner {
    pub(crate) fn do_call(&mut self, argc: usize, default_this: JsValue) -> Result<(), VmError> {
        let args_start = self.stack.len() - argc;
        let callee = self.stack[args_start - 1];
        // PERF: M4-11 — eliminate this allocation by restructuring call_internal
        let call_args: Vec<JsValue> = self.stack[args_start..].to_vec();
        self.stack.truncate(args_start - 1);
        let result = self.call_value(callee, default_this, &call_args)?;
        self.stack.push(result);
        Ok(())
    }

    pub(crate) fn call_value(
        &mut self,
        callee: JsValue,
        this: JsValue,
        args: &[JsValue],
    ) -> Result<JsValue, VmError> {
        match callee {
            JsValue::Object(id) => self.call(id, this, args),
            _ => Err(VmError::type_error("not a function")),
        }
    }

    pub(crate) fn do_new(&mut self, argc: usize) -> Result<(), VmError> {
        let args_start = self.stack.len() - argc;
        let constructor = self.stack[args_start - 1];
        // PERF: M4-11 — eliminate this allocation by restructuring call_internal
        let ctor_args: Vec<JsValue> = self.stack[args_start..].to_vec();
        self.stack.truncate(args_start - 1);

        if let JsValue::Object(ctor_id) = constructor {
            // Non-constructable native functions (e.g. Symbol) must reject `new`.
            if let ObjectKind::NativeFunction(ref nf) = self.get_object(ctor_id).kind {
                if !nf.constructable {
                    let name_str = self.strings.get_utf8(nf.name);
                    return Err(VmError::type_error(format!(
                        "{name_str} is not a constructor"
                    )));
                }
            }
            // Look up constructor.prototype for the new instance's [[Prototype]].
            let proto_key = PropertyKey::String(self.well_known.prototype);
            let proto_id = match get_property(self, ctor_id, proto_key) {
                Some(PropertyResult::Data(JsValue::Object(id))) => Some(id),
                _ => None,
            };
            // Create new instance with prototype chain.
            let instance = self.alloc_object(super::value::Object {
                kind: ObjectKind::Ordinary,
                properties: Vec::new(),
                prototype: proto_id,
            });
            let result = self.call(ctor_id, JsValue::Object(instance), &ctor_args)?;
            // If constructor returns an object, use that; otherwise use instance.
            let final_val = if matches!(result, JsValue::Object(_)) {
                result
            } else {
                JsValue::Object(instance)
            };
            self.stack.push(final_val);
            Ok(())
        } else {
            Err(VmError::type_error("not a constructor"))
        }
    }

    pub(crate) fn create_closure(
        &mut self,
        parent_func_id: FuncId,
        const_idx: u16,
    ) -> Result<(), VmError> {
        // Get the CompiledFunction from the parent's constant pool.
        let constant = self.compiled_functions[parent_func_id.0 as usize]
            .constants
            .get(const_idx as usize)
            .ok_or_else(|| VmError::internal("closure constant out of bounds"))?;

        let compiled = match constant {
            Constant::Function(f) => (**f).clone(),
            _ => return Err(VmError::internal("expected function constant for Closure")),
        };

        let upvalue_descs = compiled.upvalues.clone();
        let is_arrow = compiled.is_arrow;
        let is_strict = compiled.is_strict;
        let name = compiled.name.clone();

        let func_id = self.register_function(compiled);

        // Build upvalue IDs from descriptors.
        let frame = self.frames.last().unwrap();
        let frame_base = frame.base;
        let parent_upvalues = frame.upvalue_ids.clone();

        let mut upvalue_ids = Vec::with_capacity(upvalue_descs.len());
        for desc in &upvalue_descs {
            let uv_id = if desc.is_local {
                // Capture from parent's locals.
                let id = self.alloc_upvalue(Upvalue {
                    state: UpvalueState::Open {
                        frame_base,
                        slot: desc.index,
                    },
                });
                // Track on the current frame so pop_frame can close it.
                self.frames.last_mut().unwrap().local_upvalue_ids.push(id);
                id
            } else {
                // Capture from parent's upvalues.
                parent_upvalues[desc.index as usize]
            };
            upvalue_ids.push(uv_id);
        }

        let this_mode = if is_arrow {
            super::value::ThisMode::Lexical
        } else if is_strict {
            super::value::ThisMode::Strict
        } else {
            super::value::ThisMode::Global
        };

        // Arrow functions capture the enclosing `this` at closure-creation time.
        let captured_this = if is_arrow {
            Some(self.frames.last().unwrap().this_value)
        } else {
            None
        };

        let name_id = name.map(|n| self.strings.intern(&n));

        let func_obj = self.alloc_object(super::value::Object {
            kind: ObjectKind::Function(super::value::FunctionObject {
                func_id,
                upvalue_ids,
                this_mode,
                name: name_id,
                captured_this,
            }),
            properties: Vec::new(),
            prototype: None,
        });

        // Non-arrow functions get a `.prototype` property (a plain object with
        // a `.constructor` back-reference), matching ES2020 §9.2.5.
        if !is_arrow {
            let obj_proto = self.object_prototype;
            let proto_obj = self.alloc_object(super::value::Object {
                kind: ObjectKind::Ordinary,
                properties: Vec::new(),
                prototype: obj_proto,
            });
            // Set constructor back-reference on the prototype.
            let ctor_key = PropertyKey::String(self.well_known.constructor);
            self.get_object_mut(proto_obj)
                .properties
                .push((ctor_key, Property::method(JsValue::Object(func_obj))));
            // Set .prototype on the function object (writable, non-enumerable,
            // non-configurable per ES2020 §9.2.5).
            let proto_key = PropertyKey::String(self.well_known.prototype);
            self.get_object_mut(func_obj).properties.push((
                proto_key,
                Property {
                    slot: PropertyValue::Data(JsValue::Object(proto_obj)),
                    writable: true,
                    enumerable: false,
                    configurable: false,
                },
            ));
        }

        self.stack.push(JsValue::Object(func_obj));
        Ok(())
    }
}
