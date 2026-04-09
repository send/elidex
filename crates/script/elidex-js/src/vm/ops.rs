//! VM operation helpers: function calls, exception handling, upvalue management,
//! and operator helpers. Property access methods live in `ops_property.rs`.

use super::coerce::{get_property, to_boolean, to_number, to_string, PropertyResult};
use super::coerce_ops::{
    abstract_relational, op_bitwise, op_numeric_binary, BitwiseOp, NumericBinaryOp,
};
use super::value::{
    FuncId, JsValue, ObjectKind, PropertyKey, PropertyValue, Upvalue, UpvalueState, VmError,
    VmErrorKind,
};
use super::VmInner;
use crate::bytecode::compiled::Constant;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// ES array index upper bound: 2^32 − 2 (§6.1.7, max valid array index).
pub(crate) const MAX_ES_ARRAY_INDEX: usize = (u32::MAX as usize) - 1;

/// Practical dense array length cap: 2^27 = 128M entries ≈ 2 GiB at 16 B/JsValue.
/// Applied to both `Array(n)` constructor and `set_element` resize to prevent OOM.
pub(crate) const MAX_DENSE_ARRAY_LEN: usize = 1 << 27;

/// Try to interpret an `f64` as a valid ES array index (0..=2^32−2).
/// Returns `None` for negative, non-integer, or out-of-range values.
#[inline]
pub(crate) fn try_as_array_index(n: f64) -> Option<usize> {
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation
    )]
    let i = n as usize;
    #[allow(clippy::cast_precision_loss)]
    if n >= 0.0 && (i as f64) == n && i <= MAX_ES_ARRAY_INDEX {
        Some(i)
    } else {
        None
    }
}

/// Parse a WTF-16 string as a valid ES array index (0..=2^32−2).
/// Returns `None` for empty strings, leading zeros (except "0"), non-digit chars,
/// overflow, or values beyond the ES array index range.
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
    if n > MAX_ES_ARRAY_INDEX {
        return None;
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
                    storage: super::value::PropertyStorage::shaped(super::shape::ROOT_SHAPE),
                    prototype: self.object_prototype,
                });
                self.define_shaped_property(
                    error_obj,
                    PropertyKey::String(self.well_known.name),
                    PropertyValue::Data(JsValue::String(name_id)),
                    super::shape::PropertyAttrs::DATA,
                );
                self.define_shaped_property(
                    error_obj,
                    PropertyKey::String(self.well_known.message),
                    PropertyValue::Data(JsValue::String(msg_id)),
                    super::shape::PropertyAttrs::DATA,
                );
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
                    ObjectKind::BigIntWrapper(id) => return Ok(JsValue::BigInt(id)),
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
        // BigInt + BigInt → BigInt addition.
        if let (JsValue::BigInt(ai), JsValue::BigInt(bi)) = (lhs, rhs) {
            let result = self.bigints.get(ai) + self.bigints.get(bi);
            let id = self.bigints.alloc(result);
            return Ok(JsValue::BigInt(id));
        }
        // Mixed BigInt + Number → TypeError (to_number will throw for BigInt).
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
            self.completion_value = frame.saved_completion;
            self.stack.truncate(frame.cleanup_base);
        }
    }

    pub(crate) fn pop_frame(&mut self) {
        if let Some(frame) = self.frames.pop() {
            self.close_upvalues(&frame.local_upvalue_ids);
            self.stack.truncate(frame.cleanup_base);
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

// Property access methods live in ops_property.rs.

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

    /// `new` constructor call. For JS constructors, pushes a frame inline
    /// with `new_instance` set (single dispatcher); the Return opcode handles
    /// the "if constructor returns non-object, use instance" logic.
    /// For native constructors, calls synchronously.
    pub(crate) fn do_new(&mut self, argc: usize) -> Result<(), VmError> {
        let args_start = self.stack.len() - argc;
        let constructor = self.stack[args_start - 1];

        let JsValue::Object(ctor_id) = constructor else {
            return Err(VmError::type_error("not a constructor"));
        };

        let js_callee = self.extract_js_callee(ctor_id);

        // Arrow functions are not constructors (§9.2.1 [[Construct]]).
        if let Some(ref callee) = js_callee {
            if callee.this_mode == super::value::ThisMode::Lexical {
                return Err(VmError::type_error("not a constructor"));
            }
        }

        // For non-JS callees, validate native constructability.
        if js_callee.is_none() {
            let obj = self.get_object(ctor_id);
            match &obj.kind {
                ObjectKind::NativeFunction(ref nf) if !nf.constructable => {
                    let name_str = self.strings.get_utf8(nf.name);
                    return Err(VmError::type_error(format!(
                        "{name_str} is not a constructor"
                    )));
                }
                ObjectKind::NativeFunction(_) => {}
                _ => return Err(VmError::type_error("not a constructor")),
            }
        }

        // Look up constructor.prototype for the new instance's [[Prototype]].
        let proto_key = PropertyKey::String(self.well_known.prototype);
        let proto_id = match get_property(self, ctor_id, proto_key) {
            Some(PropertyResult::Data(JsValue::Object(id))) => Some(id),
            _ => None,
        };

        // GC safety: suppress GC during alloc_object. The instance is rooted
        // via CallFrame.new_instance (JS) or this_value (JS) / Rust local (native)
        // immediately after allocation.
        let saved_gc = self.gc_enabled;
        self.gc_enabled = false;
        let instance = self.alloc_object(super::value::Object {
            kind: ObjectKind::Ordinary,
            storage: super::value::PropertyStorage::shaped(super::shape::ROOT_SHAPE),
            prototype: proto_id,
        });
        self.gc_enabled = saved_gc;

        if let Some(callee) = js_callee {
            self.push_js_call_frame(callee, JsValue::Object(instance), argc, 1, Some(instance));
            Ok(())
        } else {
            // Native constructor: call synchronously.
            let ctor_args: Vec<JsValue> = self.stack[args_start..].to_vec();
            self.stack.truncate(args_start - 1);
            let result = self.call(ctor_id, JsValue::Object(instance), &ctor_args)?;
            let final_val = if matches!(result, JsValue::Object(_)) {
                result
            } else {
                JsValue::Object(instance)
            };
            self.stack.push(final_val);
            Ok(())
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
                upvalue_ids: upvalue_ids.into(),
                this_mode,
                name: name_id,
                captured_this,
            }),
            storage: super::value::PropertyStorage::shaped(super::shape::ROOT_SHAPE),
            prototype: None,
        });

        // Non-arrow functions get a `.prototype` property (a plain object with
        // a `.constructor` back-reference), matching ES2020 §9.2.5.
        if !is_arrow {
            // Push func_obj onto the stack to protect it from GC during
            // proto_obj allocation (alloc_object may trigger collection).
            self.stack.push(JsValue::Object(func_obj));
            let obj_proto = self.object_prototype;
            let proto_obj = self.alloc_object(super::value::Object {
                kind: ObjectKind::Ordinary,
                storage: super::value::PropertyStorage::shaped(super::shape::ROOT_SHAPE),
                prototype: obj_proto,
            });
            // Set constructor back-reference on the prototype.
            let ctor_key = PropertyKey::String(self.well_known.constructor);
            self.define_shaped_property(
                proto_obj,
                ctor_key,
                PropertyValue::Data(JsValue::Object(func_obj)),
                super::shape::PropertyAttrs::METHOD,
            );
            // Set .prototype on the function object (writable, non-enumerable,
            // non-configurable per ES2020 §9.2.5).
            let proto_key = PropertyKey::String(self.well_known.prototype);
            self.define_shaped_property(
                func_obj,
                proto_key,
                PropertyValue::Data(JsValue::Object(proto_obj)),
                super::shape::PropertyAttrs::WRITABLE_HIDDEN,
            );
        }

        if is_arrow {
            // Arrow functions have no prototype — push func_obj now.
            self.stack.push(JsValue::Object(func_obj));
        }
        // Non-arrow: func_obj was already pushed above for GC protection
        // during proto_obj allocation; it's at the correct stack position.
        Ok(())
    }
}
