//! Inline cache fast paths for property access and call dispatch.
//!
//! Extracted from `dispatch.rs` to keep the main dispatch loop concise.
//! Each method encapsulates the IC hit/miss logic for a single opcode.

use super::value::{FuncId, JsCalleeInfo, JsValue, ObjectKind, VmError};
use super::VmInner;

impl VmInner {
    /// GetProp with IC. Attempts the IC fast path for object receivers, falls
    /// back to the slow path on miss, and handles non-object receivers
    /// (primitives with prototype lookup).
    pub(super) fn ic_get_prop(
        &mut self,
        func_id: FuncId,
        name_idx: u16,
        ic_idx: usize,
        obj_val: JsValue,
    ) -> Result<JsValue, VmError> {
        if let JsValue::Object(obj_id) = obj_val {
            // IC fast path: object receiver + Shaped storage + shape guard
            let ic_hit = {
                let obj = self.objects[obj_id.0 as usize].as_ref().unwrap();
                if let super::value::PropertyStorage::Shaped { shape, .. } = &obj.storage {
                    self.compiled_functions[func_id.0 as usize]
                        .ic_slots
                        .get(ic_idx)
                        .and_then(|s| s.as_ref())
                        .filter(|ic| ic.receiver_shape == *shape)
                        .map(|ic| (ic.slot, ic.holder))
                } else {
                    None
                }
            };

            if let Some((slot, holder)) = ic_hit {
                let val = match &holder {
                    super::ic::ICHolder::Own => {
                        let obj = self.objects[obj_id.0 as usize].as_ref().unwrap();
                        if let super::value::PropertyStorage::Shaped { slots, .. } = &obj.storage {
                            match slots[slot as usize] {
                                super::value::PropertyValue::Data(v) => v,
                                super::value::PropertyValue::Accessor {
                                    getter: Some(g), ..
                                } => self.call(g, obj_val, &[])?,
                                super::value::PropertyValue::Accessor { .. } => JsValue::Undefined,
                            }
                        } else {
                            unreachable!()
                        }
                    }
                    super::ic::ICHolder::Proto {
                        proto_shape,
                        proto_slot,
                        proto_id,
                    } => {
                        // Prototype pointer + shape double guard
                        let obj = self.objects[obj_id.0 as usize].as_ref().unwrap();
                        let proto_ok = obj.prototype == Some(*proto_id);
                        if proto_ok {
                            let proto_obj = self.objects[proto_id.0 as usize].as_ref().unwrap();
                            if let super::value::PropertyStorage::Shaped { shape: ps, slots } =
                                &proto_obj.storage
                            {
                                if *ps == *proto_shape {
                                    match slots[*proto_slot as usize] {
                                        super::value::PropertyValue::Data(v) => v,
                                        super::value::PropertyValue::Accessor {
                                            getter: Some(g),
                                            ..
                                        } => self.call(g, obj_val, &[])?,
                                        super::value::PropertyValue::Accessor { .. } => {
                                            JsValue::Undefined
                                        }
                                    }
                                } else {
                                    // Proto shape mismatch -> slow path
                                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                                    self.get_prop_slow(obj_val, obj_id, name_id, func_id, ic_idx)?
                                }
                            } else {
                                let name_id = self.constant_to_string_id(func_id, name_idx)?;
                                self.get_prop_slow(obj_val, obj_id, name_id, func_id, ic_idx)?
                            }
                        } else {
                            let name_id = self.constant_to_string_id(func_id, name_idx)?;
                            self.get_prop_slow(obj_val, obj_id, name_id, func_id, ic_idx)?
                        }
                    }
                };
                return Ok(val);
            }

            // IC miss -> slow path + IC update
            let name_id = self.constant_to_string_id(func_id, name_idx)?;
            let val = self.get_prop_slow(obj_val, obj_id, name_id, func_id, ic_idx)?;
            return Ok(val);
        }

        // Non-object receiver -> existing slow path
        let name_id = self.constant_to_string_id(func_id, name_idx)?;
        self.get_property_val(obj_val, name_id)
    }

    /// SetProp with IC. Returns the value that was set (to push on stack).
    pub(super) fn ic_set_prop(
        &mut self,
        func_id: FuncId,
        name_idx: u16,
        ic_idx: usize,
        obj_val: JsValue,
        val: JsValue,
    ) -> Result<JsValue, VmError> {
        if let JsValue::Object(obj_id) = obj_val {
            // IC fast path: own writable data property on Shaped object
            let ic_hit = {
                let obj = self.objects[obj_id.0 as usize].as_ref().unwrap();
                if let super::value::PropertyStorage::Shaped { shape, .. } = &obj.storage {
                    self.compiled_functions[func_id.0 as usize]
                        .ic_slots
                        .get(ic_idx)
                        .and_then(|s| s.as_ref())
                        .filter(|ic| {
                            ic.receiver_shape == *shape
                                && matches!(ic.holder, super::ic::ICHolder::Own)
                        })
                        .map(|ic| ic.slot)
                } else {
                    None
                }
            };

            if let Some(slot) = ic_hit {
                // Verify writable + data at the cached slot
                let shapes = &self.shapes;
                let obj = self.objects[obj_id.0 as usize].as_mut().unwrap();
                if let super::value::PropertyStorage::Shaped { shape, slots, .. } = &mut obj.storage
                {
                    let entry = &shapes[*shape as usize].ordered_entries[slot as usize];
                    if entry.1.writable && !entry.1.is_accessor {
                        slots[slot as usize] = super::value::PropertyValue::Data(val);
                        // Sync global object
                        if obj_id == self.global_object {
                            let name_id = self.constant_to_string_id(func_id, name_idx)?;
                            self.globals.insert(name_id, val);
                        }
                        return Ok(val);
                    }
                }
            }

            // IC miss -> slow path + IC update
            let name_id = self.constant_to_string_id(func_id, name_idx)?;
            self.set_prop_slow(obj_val, obj_id, name_id, val, func_id, ic_idx)?;
            return Ok(val);
        }

        // Non-object receiver -> existing slow path
        let name_id = self.constant_to_string_id(func_id, name_idx)?;
        self.set_property_val(obj_val, name_id, val)?;
        Ok(val)
    }

    /// Extract JS function info from a callee object.
    /// Returns `Some(JsCalleeInfo)` for JS functions, `None` for native/other.
    pub(super) fn extract_js_callee(
        &self,
        callee_id: super::value::ObjectId,
    ) -> Option<JsCalleeInfo> {
        let obj = self.get_object(callee_id);
        if let ObjectKind::Function(fo) = &obj.kind {
            Some(JsCalleeInfo {
                func_id: fo.func_id,
                upvalue_ids: fo.upvalue_ids.clone(),
                this_mode: fo.this_mode,
                captured_this: fo.captured_this,
            })
        } else {
            None
        }
    }

    /// Compute the effective `this` for a call, applying §9.2.1.2
    /// OrdinaryCallBindThis: Global mode coerces null/undefined to globalThis
    /// Resolve `this` for a call given the callee's `ThisMode`.  Arrow
    /// functions use their captured `this`; strict functions receive the
    /// caller-provided receiver verbatim.
    #[inline]
    pub(super) fn compute_this_for_call(
        this_mode: super::value::ThisMode,
        receiver: JsValue,
        captured_this: Option<JsValue>,
    ) -> JsValue {
        match this_mode {
            super::value::ThisMode::Lexical => captured_this.unwrap_or(JsValue::Undefined),
            super::value::ThisMode::Strict => receiver,
        }
    }

    /// Try the call IC fast path. Returns `Some(JsCalleeInfo)` on hit.
    fn try_call_ic(
        &self,
        caller_func_id: FuncId,
        call_ic_idx: usize,
        callee_id: super::value::ObjectId,
    ) -> Option<JsCalleeInfo> {
        let ic = self.compiled_functions[caller_func_id.0 as usize]
            .call_ic_slots
            .get(call_ic_idx)?
            .as_ref()
            .filter(|ic| ic.callee == callee_id)?;
        Some(JsCalleeInfo {
            func_id: ic.func_id,
            upvalue_ids: ic.upvalue_ids.clone(),
            this_mode: ic.this_mode,
            captured_this: ic.captured_this,
        })
    }

    /// Call with IC. For JS callees, pushes a frame inline (single dispatcher).
    /// For native callees, calls synchronously via `self.call()`.
    pub(super) fn ic_call(
        &mut self,
        caller_func_id: FuncId,
        argc: usize,
        call_ic_idx: usize,
    ) -> Result<(), VmError> {
        let args_start = self.stack.len() - argc;
        let callee_val = self.stack[args_start - 1];

        let JsValue::Object(callee_id) = callee_val else {
            return Err(VmError::type_error("not a function"));
        };

        // IC hit: skip object-table lookup entirely.
        if let Some(callee) = self.try_call_ic(caller_func_id, call_ic_idx, callee_id) {
            let this = Self::compute_this_for_call(
                callee.this_mode,
                JsValue::Undefined,
                callee.captured_this,
            );
            self.push_js_call_frame(callee, this, argc, 1, None);
            return Ok(());
        }

        // IC miss: extract callee info from object.
        if let Some(callee) = self.extract_js_callee(callee_id) {
            let this = Self::compute_this_for_call(
                callee.this_mode,
                JsValue::Undefined,
                callee.captured_this,
            );
            self.populate_call_ic(caller_func_id, call_ic_idx, callee_val);
            self.push_js_call_frame(callee, this, argc, 1, None);
            return Ok(());
        }

        // Native function or non-callable: delegate to call() (synchronous).
        let call_args: Vec<JsValue> = self.stack[args_start..].to_vec();
        self.stack.truncate(args_start - 1);
        let result = self.call(callee_id, JsValue::Undefined, &call_args)?;
        self.stack.push(result);
        self.populate_call_ic(caller_func_id, call_ic_idx, callee_val);
        Ok(())
    }

    /// CallMethod with IC. For JS callees, pushes a frame inline.
    pub(super) fn ic_call_method(
        &mut self,
        caller_func_id: FuncId,
        argc: usize,
        call_ic_idx: usize,
    ) -> Result<(), VmError> {
        // Stack: [receiver, callee, arg0..argN]
        let args_start = self.stack.len() - argc;
        let callee = self.stack[args_start - 1];
        let receiver = self.stack[args_start - 2];

        let JsValue::Object(callee_id) = callee else {
            return Err(VmError::type_error("not a function"));
        };

        // IC hit: skip object-table lookup entirely.
        if let Some(callee_info) = self.try_call_ic(caller_func_id, call_ic_idx, callee_id) {
            let this = Self::compute_this_for_call(
                callee_info.this_mode,
                receiver,
                callee_info.captured_this,
            );
            self.push_js_call_frame(callee_info, this, argc, 2, None);
            return Ok(());
        }

        // IC miss: extract callee info from object.
        if let Some(callee_info) = self.extract_js_callee(callee_id) {
            let this = Self::compute_this_for_call(
                callee_info.this_mode,
                receiver,
                callee_info.captured_this,
            );
            self.populate_call_ic(caller_func_id, call_ic_idx, callee);
            self.push_js_call_frame(callee_info, this, argc, 2, None);
            return Ok(());
        }

        // Native function or non-callable: delegate to call() (synchronous).
        let call_args: Vec<JsValue> = self.stack[args_start..].to_vec();
        self.stack.truncate(args_start - 2);
        let result = self.call(callee_id, receiver, &call_args)?;
        self.stack.push(result);
        self.populate_call_ic(caller_func_id, call_ic_idx, callee);
        Ok(())
    }

    /// Populate a call IC slot after a successful slow-path call.
    fn populate_call_ic(&mut self, func_id: FuncId, call_ic_idx: usize, callee_val: JsValue) {
        if let JsValue::Object(callee_id) = callee_val {
            if let Some(ic) = self.collect_call_ic(callee_id) {
                if let Some(slot) = self.compiled_functions[func_id.0 as usize]
                    .call_ic_slots
                    .get_mut(call_ic_idx)
                {
                    *slot = Some(ic);
                }
            }
        }
    }
}
