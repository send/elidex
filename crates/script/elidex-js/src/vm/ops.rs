//! VM operation helpers: property access, function calls, exception handling,
//! upvalue management, and operator helpers.

use super::coerce::{
    abstract_relational, get_property, op_bitwise, op_numeric_binary, to_string, BitwiseOp,
    NumericBinaryOp,
};
use super::value::{
    FuncId, JsValue, ObjectKind, Property, StringId, Upvalue, UpvalueState, VmError,
};
use super::Vm;
use crate::bytecode::compiled::Constant;

// ---------------------------------------------------------------------------
// Operator helpers
// ---------------------------------------------------------------------------

impl Vm {
    pub(crate) fn binary_numeric(&mut self, op: NumericBinaryOp) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        let r = op_numeric_binary(&self.inner, a, b, op);
        self.inner.stack.push(r);
        Ok(())
    }

    pub(crate) fn binary_bitwise(&mut self, op: BitwiseOp) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        let r = op_bitwise(&self.inner, a, b, op);
        self.inner.stack.push(r);
        Ok(())
    }

    pub(crate) fn relational_op(&mut self, swap: bool, eq: bool) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = if eq {
            // x <= y  ===  !(y < x)
            // x >= y  ===  !(x < y)  (with swap)
            let (lhs, rhs) = if swap { (a, b) } else { (b, a) };
            match abstract_relational(&mut self.inner, lhs, rhs, swap) {
                Some(false) => true,        // !(y < x) → <=
                Some(true) | None => false, // y < x, or NaN
            }
        } else {
            // x < y  or  x > y (with swap)
            let (lhs, rhs) = if swap { (b, a) } else { (a, b) };
            abstract_relational(&mut self.inner, lhs, rhs, !swap).unwrap_or(false)
        };
        self.inner.stack.push(JsValue::Boolean(result));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Exception handling & frame management
// ---------------------------------------------------------------------------

impl Vm {
    /// Try to handle an exception by finding a handler in the current or parent frames.
    /// Returns `true` if a handler was found and ip was redirected, `false` if unhandled.
    pub(crate) fn handle_exception(
        &mut self,
        thrown_value: JsValue,
        entry_frame_depth: usize,
    ) -> bool {
        self.inner.current_exception = thrown_value;

        // Search from the current frame outward.
        loop {
            if self.inner.frames.is_empty() {
                return false;
            }
            let frame_idx = self.inner.frames.len() - 1;

            // Check if this frame has a handler.
            if let Some(handler) = self.inner.frames[frame_idx].exception_handlers.pop() {
                // Unwind stack to the handler's recorded depth.
                self.inner.stack.truncate(handler.stack_depth);

                // Jump to catch block if present, otherwise finally.
                if handler.catch_ip != u32::MAX {
                    self.inner.frames[frame_idx].ip = handler.catch_ip as usize;
                } else if handler.finally_ip != u32::MAX {
                    self.inner.frames[frame_idx].ip = handler.finally_ip as usize;
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
            let frame = self.inner.frames.pop().unwrap();
            self.close_upvalues(&frame.local_upvalue_ids);
            self.inner.stack.truncate(frame.base);
        }
    }

    pub(crate) fn pop_frame(&mut self) {
        if let Some(frame) = self.inner.frames.pop() {
            // Close open upvalues that capture this frame's local slots.
            self.close_upvalues(&frame.local_upvalue_ids);
            // Truncate stack to frame base.
            self.inner.stack.truncate(frame.base);
        }
    }

    pub(crate) fn close_upvalues(&mut self, upvalue_ids: &[super::value::UpvalueId]) {
        for &uv_id in upvalue_ids {
            if let UpvalueState::Open { frame_base, slot } =
                self.inner.upvalues[uv_id.0 as usize].state
            {
                let val = self.inner.stack[frame_base + slot as usize];
                self.inner.upvalues[uv_id.0 as usize].state = UpvalueState::Closed(val);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Upvalue read/write
// ---------------------------------------------------------------------------

impl Vm {
    pub(crate) fn read_upvalue(&self, uv_id: super::value::UpvalueId) -> JsValue {
        match self.inner.upvalues[uv_id.0 as usize].state {
            UpvalueState::Open { frame_base, slot } => self.inner.stack[frame_base + slot as usize],
            UpvalueState::Closed(val) => val,
        }
    }

    pub(crate) fn write_upvalue(&mut self, uv_id: super::value::UpvalueId, val: JsValue) {
        match self.inner.upvalues[uv_id.0 as usize].state {
            UpvalueState::Open { frame_base, slot } => {
                self.inner.stack[frame_base + slot as usize] = val;
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

impl Vm {
    pub(crate) fn get_property_val(
        &mut self,
        obj: JsValue,
        key: StringId,
    ) -> Result<JsValue, VmError> {
        match obj {
            JsValue::Object(id) => {
                Ok(get_property(&self.inner, id, key).unwrap_or(JsValue::Undefined))
            }
            JsValue::String(sid) => {
                // String.length
                if key == self.inner.well_known.length {
                    let s = self.inner.strings.get(sid);
                    #[allow(clippy::cast_precision_loss)]
                    Ok(JsValue::Number(s.len() as f64))
                } else if let Some(proto_id) = self.inner.string_prototype {
                    // Look up method on String.prototype.
                    Ok(get_property(&self.inner, proto_id, key).unwrap_or(JsValue::Undefined))
                } else {
                    Ok(JsValue::Undefined)
                }
            }
            _ => Ok(JsValue::Undefined),
        }
    }

    pub(crate) fn set_property_val(
        &mut self,
        obj: JsValue,
        key: StringId,
        val: JsValue,
    ) -> Result<(), VmError> {
        if let JsValue::Object(id) = obj {
            let obj = self.get_object_mut(id);
            // Check if property exists.
            for prop in &mut obj.properties {
                if prop.0 == key {
                    prop.1.value = val;
                    return Ok(());
                }
            }
            obj.properties.push((key, Property::data(val)));
        }
        Ok(())
    }

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
                    if let ObjectKind::Array { ref elements } = obj_ref.kind {
                        return Ok(elements.get(idx).copied().unwrap_or(JsValue::Undefined));
                    }
                }
            }
            // Fall back to string key property lookup.
            let key_id = to_string(&mut self.inner, key);
            Ok(get_property(&self.inner, id, key_id).unwrap_or(JsValue::Undefined))
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
                    if let ObjectKind::Array { ref mut elements } = obj_ref.kind {
                        if idx >= elements.len() {
                            elements.resize(idx + 1, JsValue::Undefined);
                        }
                        elements[idx] = val;
                        return Ok(());
                    }
                }
            }
            let key_id = to_string(&mut self.inner, key);
            self.set_property_val(JsValue::Object(id), key_id, val)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Function calls & closures
// ---------------------------------------------------------------------------

impl Vm {
    pub(crate) fn do_call(&mut self, argc: usize, default_this: JsValue) -> Result<(), VmError> {
        let args_start = self.inner.stack.len() - argc;
        let callee = self.inner.stack[args_start - 1];
        // PERF: M4-11 — eliminate this allocation by restructuring call_internal
        let call_args: Vec<JsValue> = self.inner.stack[args_start..].to_vec();
        self.inner.stack.truncate(args_start - 1);
        let result = self.call_value(callee, default_this, &call_args)?;
        self.inner.stack.push(result);
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
        let args_start = self.inner.stack.len() - argc;
        let constructor = self.inner.stack[args_start - 1];
        // PERF: M4-11 — eliminate this allocation by restructuring call_internal
        let ctor_args: Vec<JsValue> = self.inner.stack[args_start..].to_vec();
        self.inner.stack.truncate(args_start - 1);

        if let JsValue::Object(ctor_id) = constructor {
            // Look up constructor.prototype for the new instance's [[Prototype]].
            let proto_key = self.inner.well_known.prototype;
            let proto_id = get_property(&self.inner, ctor_id, proto_key).and_then(|v| {
                if let JsValue::Object(id) = v {
                    Some(id)
                } else {
                    None
                }
            });
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
            self.inner.stack.push(final_val);
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
        let constant = self.inner.compiled_functions[parent_func_id.0 as usize]
            .constants
            .get(const_idx as usize)
            .ok_or_else(|| VmError::internal("closure constant out of bounds"))?;

        let compiled = match constant {
            Constant::Function(f) => (**f).clone(),
            _ => return Err(VmError::internal("expected function constant for Closure")),
        };

        let upvalue_descs = compiled.upvalues.clone();
        let is_arrow = compiled.is_arrow;
        let name = compiled.name.clone();

        let func_id = self.register_function(compiled);

        // Build upvalue IDs from descriptors.
        let frame = self.inner.frames.last().unwrap();
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
                self.inner
                    .frames
                    .last_mut()
                    .unwrap()
                    .local_upvalue_ids
                    .push(id);
                id
            } else {
                // Capture from parent's upvalues.
                parent_upvalues[desc.index as usize]
            };
            upvalue_ids.push(uv_id);
        }

        let this_mode = if is_arrow {
            super::value::ThisMode::Lexical
        } else {
            super::value::ThisMode::Global
        };

        // Arrow functions capture the enclosing `this` at closure-creation time.
        let captured_this = if is_arrow {
            Some(self.inner.frames.last().unwrap().this_value)
        } else {
            None
        };

        let name_id = name.map(|n| self.inner.strings.intern(&n));

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
            let obj_proto = self.inner.object_prototype;
            let proto_obj = self.alloc_object(super::value::Object {
                kind: ObjectKind::Ordinary,
                properties: Vec::new(),
                prototype: obj_proto,
            });
            // Set constructor back-reference on the prototype.
            let ctor_key = self.inner.well_known.constructor;
            self.get_object_mut(proto_obj)
                .properties
                .push((ctor_key, Property::method(JsValue::Object(func_obj))));
            // Set .prototype on the function object.
            let proto_key = self.inner.well_known.prototype;
            self.get_object_mut(func_obj)
                .properties
                .push((proto_key, Property::data(JsValue::Object(proto_obj))));
        }

        self.inner.stack.push(JsValue::Object(func_obj));
        Ok(())
    }
}
