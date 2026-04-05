//! Main bytecode dispatch loop.
//!
//! Contains `Vm::run()` — the core opcode-dispatch loop. Bytecode reading
//! helpers, constant loading, and jump support live in `dispatch_helpers.rs`.

use crate::bytecode::opcode::Op;

use super::coerce::{
    abstract_eq, get_property, op_bitnot, op_neg, op_not, op_pos, op_void, strict_eq, to_boolean,
    to_number, to_string, typeof_str, BitwiseOp, NumericBinaryOp,
};
use super::ops::parse_array_index_u16;
use super::value::{JsValue, Object, ObjectKind, Property, PropertyKey, VmError, VmErrorKind};
use super::Vm;

// ---------------------------------------------------------------------------
// Main dispatch loop
// ---------------------------------------------------------------------------

impl Vm {
    /// Execute bytecode until the current call frame returns.
    #[allow(clippy::too_many_lines)] // single dispatch loop, splitting would hurt readability
    pub(crate) fn run(&mut self) -> Result<JsValue, VmError> {
        let entry_frame_depth = self.inner.frames.len() - 1;

        loop {
            let frame_idx = self.inner.frames.len() - 1;
            let func_id = self.inner.frames[frame_idx].func_id;
            let ip = self.inner.frames[frame_idx].ip;

            let bytecode = &self.inner.compiled_functions[func_id.0 as usize].bytecode;
            if ip >= bytecode.len() {
                // Fell off the end → implicit ReturnUndefined.
                if frame_idx == entry_frame_depth {
                    let completion = self.inner.completion_value;
                    self.inner.completion_value = JsValue::Undefined;
                    return Ok(completion);
                }
                self.pop_frame();
                continue;
            }

            let op_byte = bytecode[ip];
            let op = Op::from_byte(op_byte).ok_or_else(|| {
                VmError::internal(format!("invalid opcode: {op_byte:#x} at ip={ip}"))
            })?;
            self.inner.frames[frame_idx].ip = ip + 1;

            match op {
                // ── Stack manipulation ──────────────────────────────
                Op::PushUndefined => self.inner.stack.push(JsValue::Undefined),
                Op::PushNull => self.inner.stack.push(JsValue::Null),
                Op::PushTrue => self.inner.stack.push(JsValue::Boolean(true)),
                Op::PushFalse => self.inner.stack.push(JsValue::Boolean(false)),
                Op::PushI8 => {
                    let val = self.read_i8_op();
                    self.inner.stack.push(JsValue::Number(f64::from(val)));
                }
                Op::PushConst => {
                    let idx = self.read_u16_op();
                    let val = self.load_constant(func_id, idx)?;
                    self.inner.stack.push(val);
                }
                Op::Dup => {
                    let val = self.peek()?;
                    self.inner.stack.push(val);
                }
                Op::Pop => {
                    let val = self.pop()?;
                    // At script (entry) level, capture completion value for eval.
                    if frame_idx == entry_frame_depth {
                        self.inner.completion_value = val;
                    }
                }
                Op::Swap => {
                    let len = self.inner.stack.len();
                    if len < 2 {
                        return Err(VmError::internal("stack underflow on Swap"));
                    }
                    self.inner.stack.swap(len - 1, len - 2);
                }

                // ── Local access ────────────────────────────────────
                Op::GetLocal => {
                    let slot = self.read_u16_op() as usize;
                    let base = self.inner.frames[frame_idx].base;
                    let val = self.inner.stack[base + slot];
                    self.inner.stack.push(val);
                }
                Op::SetLocal => {
                    let slot = self.read_u16_op() as usize;
                    let val = self.peek()?;
                    let base = self.inner.frames[frame_idx].base;
                    self.inner.stack[base + slot] = val;
                }
                Op::CheckTdz => {
                    let slot = self.read_u16_op() as usize;
                    let frame = &self.inner.frames[frame_idx];
                    if frame.tdz_slots.get(slot).copied().unwrap_or(false) {
                        let err = VmError::reference_error(
                            "Cannot access variable before initialization",
                        );
                        self.throw_error(err, entry_frame_depth)?;
                    }
                }
                Op::InitLocal => {
                    let slot = self.read_u16_op() as usize;
                    let frame = &mut self.inner.frames[frame_idx];
                    if let Some(v) = frame.tdz_slots.get_mut(slot) {
                        *v = false;
                    }
                }

                // ── Upvalue access ──────────────────────────────────
                Op::GetUpvalue => {
                    let idx = self.read_u16_op() as usize;
                    let uv_id = self.inner.frames[frame_idx].upvalue_ids[idx];
                    let val = self.read_upvalue(uv_id);
                    self.inner.stack.push(val);
                }
                Op::SetUpvalue => {
                    let idx = self.read_u16_op() as usize;
                    let val = self.peek()?;
                    let uv_id = self.inner.frames[frame_idx].upvalue_ids[idx];
                    self.write_upvalue(uv_id, val);
                }

                // ── Global access ───────────────────────────────────
                Op::GetGlobal => {
                    let name_idx = self.read_u16_op();
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    if let Some(val) = self.inner.globals.get(&name_id).copied() {
                        self.inner.stack.push(val);
                    } else {
                        // Fall back to the global object (supports accessor properties
                        // defined via Object.defineProperty(globalThis, ...)).
                        let global_obj = self.inner.global_object;
                        match self.get_property_val(JsValue::Object(global_obj), name_id) {
                            Ok(JsValue::Undefined) => {
                                let name_str = self.inner.strings.get_utf8(name_id);
                                let msg = format!("{name_str} is not defined");
                                let err = VmError::reference_error(&msg);
                                self.throw_error(err, entry_frame_depth)?;
                            }
                            Ok(val) => self.inner.stack.push(val),
                            Err(e) => {
                                self.throw_error(e, entry_frame_depth)?;
                            }
                        }
                    }
                }
                Op::SetGlobal => {
                    let name_idx = self.read_u16_op();
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    let val = self.peek()?;
                    // §8.1.1.2.5: In strict mode, assigning to an undeclared
                    // variable is a ReferenceError.
                    if self.inner.compiled_functions[func_id.0 as usize].is_strict
                        && !self.inner.globals.contains_key(&name_id)
                    {
                        let name_str = self.inner.strings.get_utf8(name_id);
                        let msg = format!("{name_str} is not defined");
                        let err = VmError::reference_error(&msg);
                        self.throw_error(err, entry_frame_depth)?;
                    } else {
                        // Check for accessor setter on globalThis before
                        // writing to the globals HashMap.
                        let global_obj = self.inner.global_object;
                        if let Err(e) =
                            self.set_property_val(JsValue::Object(global_obj), name_id, val)
                        {
                            self.throw_error(e, entry_frame_depth)?;
                        }
                    }
                }

                // ── Arithmetic ──────────────────────────────────────
                Op::Add => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    match self.op_add(a, b) {
                        Ok(r) => self.inner.stack.push(r),
                        Err(e) => {
                            self.throw_error(e, entry_frame_depth)?;
                        }
                    }
                }
                Op::Sub => {
                    if let Err(e) = self.binary_numeric(NumericBinaryOp::Sub) {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }
                Op::Mul => {
                    if let Err(e) = self.binary_numeric(NumericBinaryOp::Mul) {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }
                Op::Div => {
                    if let Err(e) = self.binary_numeric(NumericBinaryOp::Div) {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }
                Op::Mod => {
                    if let Err(e) = self.binary_numeric(NumericBinaryOp::Rem) {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }
                Op::Exp => {
                    if let Err(e) = self.binary_numeric(NumericBinaryOp::Exp) {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }

                // ── Bitwise ─────────────────────────────────────────
                Op::BitAnd => {
                    if let Err(e) = self.binary_bitwise(BitwiseOp::And) {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }
                Op::BitOr => {
                    if let Err(e) = self.binary_bitwise(BitwiseOp::Or) {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }
                Op::BitXor => {
                    if let Err(e) = self.binary_bitwise(BitwiseOp::Xor) {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }
                Op::Shl => {
                    if let Err(e) = self.binary_bitwise(BitwiseOp::Shl) {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }
                Op::Shr => {
                    if let Err(e) = self.binary_bitwise(BitwiseOp::Shr) {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }
                Op::UShr => {
                    if let Err(e) = self.binary_bitwise(BitwiseOp::UShr) {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }

                // ── Comparison ──────────────────────────────────────
                Op::Eq => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    let r = abstract_eq(&mut self.inner, a, b);
                    self.inner.stack.push(JsValue::Boolean(r));
                }
                Op::NotEq => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    let r = !abstract_eq(&mut self.inner, a, b);
                    self.inner.stack.push(JsValue::Boolean(r));
                }
                Op::StrictEq => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.inner.stack.push(JsValue::Boolean(strict_eq(a, b)));
                }
                Op::StrictNotEq => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.inner.stack.push(JsValue::Boolean(!strict_eq(a, b)));
                }
                Op::Lt => {
                    if let Err(e) = self.relational_op(false, false) {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }
                Op::LtEq => {
                    if let Err(e) = self.relational_op(false, true) {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }
                Op::Gt => {
                    if let Err(e) = self.relational_op(true, false) {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }
                Op::GtEq => {
                    if let Err(e) = self.relational_op(true, true) {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }

                Op::Instanceof => {
                    let rhs = self.pop()?; // constructor
                    let lhs = self.pop()?; // object

                    // §12.10.4 step 2: Check rhs[@@hasInstance]
                    if let JsValue::Object(rhs_id) = rhs {
                        let has_instance_key =
                            PropertyKey::Symbol(self.inner.well_known_symbols.has_instance);
                        if let Some(has_instance_result) =
                            super::coerce::get_property(&self.inner, rhs_id, has_instance_key)
                        {
                            let has_instance_fn =
                                match self.resolve_property(has_instance_result, rhs) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        self.throw_error(e, entry_frame_depth)?;
                                        continue;
                                    }
                                };
                            let result = match self.call_value(has_instance_fn, rhs, &[lhs]) {
                                Ok(r) => r,
                                Err(e) => {
                                    self.throw_error(e, entry_frame_depth)?;
                                    continue;
                                }
                            };
                            let bool_result = to_boolean(&self.inner, result);
                            self.inner.stack.push(JsValue::Boolean(bool_result));
                            continue;
                        }
                    }

                    // OrdinaryHasInstance: walk lhs's prototype chain looking for rhs.prototype
                    let result =
                        if let (JsValue::Object(obj_id), JsValue::Object(ctor_id)) = (lhs, rhs) {
                            let proto_key = PropertyKey::String(self.inner.well_known.prototype);
                            let ctor_proto =
                                super::coerce::get_property(&self.inner, ctor_id, proto_key);
                            if let Some(super::coerce::PropertyResult::Data(JsValue::Object(
                                target_proto,
                            ))) = ctor_proto
                            {
                                let mut current = self.inner.get_object(obj_id).prototype;
                                let mut found = false;
                                while let Some(proto_id) = current {
                                    if proto_id == target_proto {
                                        found = true;
                                        break;
                                    }
                                    current = self.inner.get_object(proto_id).prototype;
                                }
                                found
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                    self.inner.stack.push(JsValue::Boolean(result));
                }
                Op::In => {
                    let rhs = self.pop()?; // object
                    let lhs = self.pop()?; // key
                    if let JsValue::Object(obj_id) = rhs {
                        let pk = match self.make_property_key(lhs) {
                            Ok(pk) => pk,
                            Err(e) => {
                                self.throw_error(e, entry_frame_depth)?;
                                continue;
                            }
                        };
                        let obj = self.inner.get_object(obj_id);
                        let found = match (&obj.kind, &pk) {
                            (ObjectKind::Array { ref elements }, PropertyKey::String(key_id)) => {
                                let key_units = self.inner.strings.get(*key_id);
                                if let Some(idx) = parse_array_index_u16(key_units) {
                                    idx < elements.len()
                                } else {
                                    super::coerce::get_property(&self.inner, obj_id, pk).is_some()
                                }
                            }
                            _ => super::coerce::get_property(&self.inner, obj_id, pk).is_some(),
                        };
                        self.inner.stack.push(JsValue::Boolean(found));
                    } else {
                        let err = VmError::type_error(
                            "Cannot use 'in' operator to search for property in non-object",
                        );
                        self.throw_error(err, entry_frame_depth)?;
                    }
                }

                // ── Unary ───────────────────────────────────────────
                Op::Neg => {
                    let a = self.pop()?;
                    match op_neg(&self.inner, a) {
                        Ok(r) => self.inner.stack.push(r),
                        Err(e) => self.throw_error(e, entry_frame_depth)?,
                    }
                }
                Op::Pos => {
                    let a = self.pop()?;
                    match op_pos(&self.inner, a) {
                        Ok(r) => self.inner.stack.push(r),
                        Err(e) => self.throw_error(e, entry_frame_depth)?,
                    }
                }
                Op::Not => {
                    let a = self.pop()?;
                    self.inner.stack.push(op_not(&self.inner, a));
                }
                Op::BitNot => {
                    let a = self.pop()?;
                    match op_bitnot(&self.inner, a) {
                        Ok(r) => self.inner.stack.push(r),
                        Err(e) => self.throw_error(e, entry_frame_depth)?,
                    }
                }
                Op::TypeOf => {
                    let a = self.pop()?;
                    let s = typeof_str(&self.inner, a);
                    self.inner.stack.push(JsValue::String(s));
                }
                Op::TypeOfGlobal => {
                    let name_idx = self.read_u16_op();
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    let val = self
                        .inner
                        .globals
                        .get(&name_id)
                        .copied()
                        .unwrap_or(JsValue::Undefined);
                    let s = typeof_str(&self.inner, val);
                    self.inner.stack.push(JsValue::String(s));
                }
                Op::Void => {
                    self.pop()?;
                    self.inner.stack.push(op_void());
                }

                // ── Update operations ───────────────────────────────
                Op::IncLocal | Op::DecLocal => {
                    let slot = self.read_u16_op() as usize;
                    let prefix = self.read_u8_op() != 0;
                    let base = self.inner.frames[frame_idx].base;
                    match to_number(&self.inner, self.inner.stack[base + slot]) {
                        Ok(old) => {
                            let new = if op == Op::IncLocal {
                                old + 1.0
                            } else {
                                old - 1.0
                            };
                            self.inner.stack[base + slot] = JsValue::Number(new);
                            let push_val = if prefix { new } else { old };
                            self.inner.stack.push(JsValue::Number(push_val));
                        }
                        Err(e) => self.throw_error(e, entry_frame_depth)?,
                    }
                }
                Op::IncProp | Op::DecProp => {
                    let name_idx = self.read_u16_op();
                    let prefix = self.read_u8_op() != 0;
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    let obj_val = self.pop()?;
                    let old = match self.get_property_val(obj_val, name_id) {
                        Ok(v) => v,
                        Err(e) => {
                            self.throw_error(e, entry_frame_depth)?;
                            continue;
                        }
                    };
                    match to_number(&self.inner, old) {
                        Ok(old_num) => {
                            let new_num = if op == Op::IncProp {
                                old_num + 1.0
                            } else {
                                old_num - 1.0
                            };
                            if let JsValue::Object(id) = obj_val {
                                if let Err(e) = self.set_property_val(
                                    JsValue::Object(id),
                                    name_id,
                                    JsValue::Number(new_num),
                                ) {
                                    self.throw_error(e, entry_frame_depth)?;
                                    continue;
                                }
                            }
                            self.inner.stack.push(JsValue::Number(if prefix {
                                new_num
                            } else {
                                old_num
                            }));
                        }
                        Err(e) => self.throw_error(e, entry_frame_depth)?,
                    }
                }
                Op::IncElem | Op::DecElem => {
                    let prefix = self.read_u8_op() != 0;
                    let key = self.pop()?;
                    let obj_val = self.pop()?;
                    let old = match self.get_element(obj_val, key) {
                        Ok(v) => v,
                        Err(e) => {
                            self.throw_error(e, entry_frame_depth)?;
                            continue;
                        }
                    };
                    match to_number(&self.inner, old) {
                        Ok(old_num) => {
                            let new_num = if op == Op::IncElem {
                                old_num + 1.0
                            } else {
                                old_num - 1.0
                            };
                            if let Err(e) = self.set_element(obj_val, key, JsValue::Number(new_num))
                            {
                                self.throw_error(e, entry_frame_depth)?;
                                continue;
                            }
                            self.inner.stack.push(JsValue::Number(if prefix {
                                new_num
                            } else {
                                old_num
                            }));
                        }
                        Err(e) => self.throw_error(e, entry_frame_depth)?,
                    }
                }

                // ── Control flow ────────────────────────────────────
                Op::Jump => {
                    let offset = self.read_i16_op();
                    self.jump_relative(offset);
                }
                Op::JumpIfFalse => {
                    let offset = self.read_i16_op();
                    let val = self.pop()?;
                    if !to_boolean(&self.inner, val) {
                        self.jump_relative(offset);
                    }
                }
                Op::JumpIfTrue => {
                    let offset = self.read_i16_op();
                    let val = self.pop()?;
                    if to_boolean(&self.inner, val) {
                        self.jump_relative(offset);
                    }
                }
                Op::JumpIfNullish => {
                    let offset = self.read_i16_op();
                    let val = self.peek()?;
                    if val.is_nullish() {
                        self.jump_relative(offset);
                    }
                }
                Op::JumpIfNotNullish => {
                    let offset = self.read_i16_op();
                    let val = self.peek()?;
                    if !val.is_nullish() {
                        self.jump_relative(offset);
                    }
                }

                // ── Return ──────────────────────────────────────────
                Op::Return => {
                    let val = self.pop()?;
                    if frame_idx == entry_frame_depth {
                        self.pop_frame();
                        return Ok(val);
                    }
                    self.pop_frame();
                }
                Op::ReturnUndefined => {
                    if frame_idx == entry_frame_depth {
                        let completion = self.inner.completion_value;
                        self.pop_frame();
                        self.inner.completion_value = JsValue::Undefined;
                        return Ok(completion);
                    }
                    self.pop_frame();
                }

                // ── Property access (Step 4 stubs) ──────────────────
                Op::GetProp => {
                    let name_idx = self.read_u16_op();
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    let obj_val = self.pop()?;
                    match self.get_property_val(obj_val, name_id) {
                        Ok(val) => self.inner.stack.push(val),
                        Err(e) => {
                            self.throw_error(e, entry_frame_depth)?;
                        }
                    }
                }
                Op::SetProp => {
                    let name_idx = self.read_u16_op();
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    let val = self.pop()?;
                    let obj_val = self.pop()?;
                    if let Err(e) = self.set_property_val(obj_val, name_id, val) {
                        self.throw_error(e, entry_frame_depth)?;
                        continue;
                    }
                    self.inner.stack.push(val);
                }
                Op::GetElem => {
                    let key = self.pop()?;
                    let obj = self.pop()?;
                    match self.get_element(obj, key) {
                        Ok(val) => self.inner.stack.push(val),
                        Err(e) => {
                            self.throw_error(e, entry_frame_depth)?;
                        }
                    }
                }
                Op::SetElem => {
                    let val = self.pop()?;
                    let key = self.pop()?;
                    let obj = self.pop()?;
                    if let Err(e) = self.set_element(obj, key, val) {
                        self.throw_error(e, entry_frame_depth)?;
                        continue;
                    }
                    self.inner.stack.push(val);
                }
                Op::DeleteProp => {
                    let name_idx = self.read_u16_op();
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    let pk = PropertyKey::String(name_id);
                    let obj_val = self.pop()?;
                    if let JsValue::Object(id) = obj_val {
                        // Sync global object deletes to the globals HashMap.
                        if id == self.inner.global_object {
                            self.inner.globals.remove(&name_id);
                        }
                        let obj = self.get_object_mut(id);
                        obj.properties.retain(|(k, _)| *k != pk);
                    }
                    self.inner.stack.push(JsValue::Boolean(true));
                }
                Op::DeleteElem => {
                    let key = self.pop()?;
                    let obj_val = self.pop()?;
                    if let JsValue::Object(id) = obj_val {
                        match self.make_property_key(key) {
                            Ok(pk) => {
                                // Sync global object deletes to the globals HashMap.
                                if id == self.inner.global_object {
                                    if let PropertyKey::String(sid) = pk {
                                        self.inner.globals.remove(&sid);
                                    }
                                }
                                let obj = self.get_object_mut(id);
                                obj.properties.retain(|(k, _)| *k != pk);
                            }
                            Err(e) => {
                                self.throw_error(e, entry_frame_depth)?;
                                continue;
                            }
                        }
                    }
                    self.inner.stack.push(JsValue::Boolean(true));
                }

                // ── Object/Array creation ───────────────────────────
                Op::CreateObject => self.op_create_object(),
                Op::DefineProperty => {
                    let name_idx = self.read_u16_op();
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    self.op_define_property(name_id)?;
                }
                Op::DefineComputedProperty => {
                    self.op_define_computed_property(entry_frame_depth)?;
                }
                Op::DefineComputedMethod => {
                    self.op_define_computed_method(entry_frame_depth)?;
                }
                Op::CreateArray => self.op_create_array(),
                Op::ArrayPush => self.op_array_push()?,
                Op::ArrayHole => self.op_array_hole()?,
                Op::ArraySpread => {
                    if let Err(e) = self.op_array_spread() {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }
                Op::SpreadObject => self.op_spread_object()?,

                // ── Template ────────────────────────────────────────
                Op::TemplateConcat => {
                    let count = self.read_u16_op() as usize;
                    let start = self.inner.stack.len() - count;
                    let parts: Vec<JsValue> = self.inner.stack[start..].to_vec();
                    self.inner.stack.truncate(start);
                    let mut result: Vec<u16> = Vec::new();
                    let mut err: Option<VmError> = None;
                    for val in parts {
                        match to_string(&mut self.inner, val) {
                            Ok(s_id) => result.extend_from_slice(self.inner.strings.get(s_id)),
                            Err(e) => {
                                err = Some(e);
                                break;
                            }
                        }
                    }
                    if let Some(e) = err {
                        self.throw_error(e, entry_frame_depth)?;
                        continue;
                    }
                    let id = self.inner.strings.intern_utf16(&result);
                    self.inner.stack.push(JsValue::String(id));
                }

                // ── Function call ───────────────────────────────────
                Op::Call => {
                    let argc = self.read_u8_op() as usize;
                    if let Err(e) = self.do_call(argc, JsValue::Undefined) {
                        let thrown = self.vm_error_to_thrown(&e);
                        if self.handle_exception(thrown, entry_frame_depth) {
                            continue;
                        }
                        return Err(e);
                    }
                }
                Op::CallMethod => {
                    let argc = self.read_u8_op() as usize;
                    // Stack: [receiver, callee, arg0..argN]
                    let args_start = self.inner.stack.len() - argc;
                    let callee = self.inner.stack[args_start - 1];
                    let receiver = self.inner.stack[args_start - 2];
                    // PERF: M4-11 — eliminate this allocation by restructuring call_internal
                    let call_args: Vec<JsValue> = self.inner.stack[args_start..].to_vec();
                    self.inner.stack.truncate(args_start - 2);
                    match self.call_value(callee, receiver, &call_args) {
                        Ok(result) => self.inner.stack.push(result),
                        Err(e) => {
                            let thrown = if let VmErrorKind::ThrowValue(val) = e.kind {
                                val
                            } else {
                                let msg = self.inner.strings.intern(&e.to_string());
                                JsValue::String(msg)
                            };
                            if self.handle_exception(thrown, entry_frame_depth) {
                                continue;
                            }
                            return Err(e);
                        }
                    }
                }
                Op::New => {
                    let argc = self.read_u8_op() as usize;
                    if let Err(e) = self.do_new(argc) {
                        let thrown = self.vm_error_to_thrown(&e);
                        if self.handle_exception(thrown, entry_frame_depth) {
                            continue;
                        }
                        return Err(e);
                    }
                }
                Op::PushThis => {
                    let this = self.inner.frames[frame_idx].this_value;
                    self.inner.stack.push(this);
                }
                Op::Closure => {
                    let const_idx = self.read_u16_op();
                    self.create_closure(func_id, const_idx)?;
                }

                // ── Exception handling ───────────────────────────────
                Op::PushExceptionHandler => {
                    let catch_ip = self.read_u16_op();
                    let finally_ip = self.read_u16_op();
                    let stack_depth = self.inner.stack.len();
                    let frame = self.inner.frames.last_mut().unwrap();
                    frame.exception_handlers.push(super::value::HandlerEntry {
                        catch_ip: u32::from(catch_ip),
                        finally_ip: u32::from(finally_ip),
                        stack_depth,
                    });
                }
                Op::PopExceptionHandler => {
                    let frame = self.inner.frames.last_mut().unwrap();
                    frame.exception_handlers.pop();
                }
                Op::Throw => {
                    let val = self.pop()?;
                    // Try to find an exception handler.
                    if self.handle_exception(val, entry_frame_depth) {
                        // Handler found and activated — continue the dispatch loop.
                        continue;
                    }
                    // No handler found — clean up frames/stack so subsequent
                    // `eval()` calls don't see stale state.
                    while self.inner.frames.len() > entry_frame_depth {
                        let frame = self.inner.frames.pop().unwrap();
                        self.close_upvalues(&frame.local_upvalue_ids);
                        self.inner.stack.truncate(frame.base);
                    }
                    if let Some(frame) = self.inner.frames.pop() {
                        self.close_upvalues(&frame.local_upvalue_ids);
                        self.inner.stack.truncate(frame.base);
                    }
                    return Err(VmError {
                        kind: VmErrorKind::ThrowValue(val),
                        message: "uncaught throw".into(),
                    });
                }
                Op::PushException => {
                    let exc = self.inner.current_exception;
                    self.inner.stack.push(exc);
                }

                // ── Switch ──────────────────────────────────────────
                Op::SwitchJump => {
                    let _table_idx = self.read_u16_op();
                    // Will be fully implemented later.
                    self.pop()?;
                }

                // ── Accessor property definition ────────────────────
                Op::DefineGetter => {
                    let name_idx = self.read_u16_op();
                    let flags = self.read_u8_op();
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    let enumerable = flags & 1 != 0;
                    self.op_define_accessor(name_id, true, enumerable)?;
                }
                Op::DefineSetter => {
                    let name_idx = self.read_u16_op();
                    let flags = self.read_u8_op();
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    let enumerable = flags & 1 != 0;
                    self.op_define_accessor(name_id, false, enumerable)?;
                }

                // ── Arguments object ────────────────────────────────
                Op::CreateArguments => {
                    let args = self.inner.frames[frame_idx]
                        .actual_args
                        .take()
                        .unwrap_or_default();
                    let len = args.len();
                    let args_obj = self.alloc_object(super::value::Object {
                        kind: ObjectKind::Arguments { values: args },
                        properties: Vec::new(),
                        prototype: self.inner.object_prototype,
                    });
                    // Set the `length` property.
                    let length_key = PropertyKey::String(self.inner.well_known.length);
                    #[allow(clippy::cast_precision_loss)]
                    // arguments.length is non-enumerable (§10.4.4.5).
                    self.get_object_mut(args_obj)
                        .properties
                        .push((length_key, Property::method(JsValue::Number(len as f64))));
                    self.inner.stack.push(JsValue::Object(args_obj));
                }

                // ── Stubs for remaining opcodes ─────────────────────
                Op::CallSpread | Op::NewSpread | Op::SuperCallSpread => {
                    self.pop()?; // args array
                    self.pop()?; // callee/constructor
                    self.inner.stack.push(JsValue::Undefined);
                }
                Op::TaggedTemplate => {
                    let _count = self.read_u8_op();
                    // Simplified stub.
                    self.inner.stack.push(JsValue::Undefined);
                }

                // ── Iteration ───────────────────────────────────────
                Op::GetIterator => {
                    if let Err(e) = self.op_get_iterator() {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }
                Op::IteratorNext => {
                    // Stack: [iterator] → [iterator value done]
                    let iter_val = *self
                        .inner
                        .stack
                        .last()
                        .ok_or_else(|| VmError::internal("empty stack on IteratorNext"))?;
                    match self.iter_next(iter_val) {
                        Ok(Some(value)) => {
                            self.inner.stack.push(value);
                            self.inner.stack.push(JsValue::Boolean(false));
                        }
                        Ok(None) => {
                            self.inner.stack.push(JsValue::Undefined);
                            self.inner.stack.push(JsValue::Boolean(true));
                        }
                        Err(e) => {
                            self.throw_error(e, entry_frame_depth)?;
                        }
                    }
                }
                Op::IteratorRest => {
                    // Stack: [iterator] → [rest_array]
                    // Collect remaining iterator elements into a new array.
                    let iter_val = self.pop()?;
                    let mut elements = Vec::new();
                    loop {
                        match self.iter_next(iter_val) {
                            Ok(Some(value)) => elements.push(value),
                            Ok(None) => break,
                            Err(e) => {
                                self.throw_error(e, entry_frame_depth)?;
                                break;
                            }
                        }
                    }
                    let proto = self.inner.array_prototype;
                    let arr = self.alloc_object(Object {
                        kind: ObjectKind::Array { elements },
                        properties: Vec::new(),
                        prototype: proto,
                    });
                    self.inner.stack.push(JsValue::Object(arr));
                }
                Op::IteratorClose => {
                    // Stack: [iterator] → []
                    let iter_val = self.pop()?;
                    if let JsValue::Object(iter_id) = iter_val {
                        let return_key = PropertyKey::String(self.inner.well_known.return_str);
                        if let Some(return_result) = get_property(&self.inner, iter_id, return_key)
                        {
                            let return_fn = match self.resolve_property(return_result, iter_val) {
                                Ok(v) => v,
                                Err(e) => {
                                    self.throw_error(e, entry_frame_depth)?;
                                    continue;
                                }
                            };
                            // Call .return() and route errors through exception handling.
                            if let Err(e) = self.call_value(return_fn, iter_val, &[]) {
                                self.throw_error(e, entry_frame_depth)?;
                            }
                        }
                    }
                }
                Op::DestructureElem | Op::Debugger => {
                    // Destructuring stubs / no-op instructions.
                }
                Op::DestructureProp | Op::ObjectRest | Op::DefaultIfUndefined => {
                    self.read_u16_op();
                }

                // ── Class (Step 9 stubs) ────────────────────────────
                Op::CreateClass => {
                    self.read_u16_op();
                    self.pop()?; // super
                    self.inner.stack.push(JsValue::Undefined);
                }
                Op::DefineMethod => {
                    let name_idx = self.read_u16_op();
                    let _flags = self.read_u8_op(); // flags byte (static|kind)
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    self.op_define_method(name_id)?;
                }
                Op::DefineField => {
                    self.read_u16_op();
                    self.read_u8_op();
                    self.pop()?; // value/closure
                                 // Leave class on stack.
                }
                Op::SuperCall => {
                    let _argc = self.read_u8_op();
                    self.inner.stack.push(JsValue::Undefined);
                }

                // ── For-in iteration ────────────────────────────────
                Op::ForInIterator => self.op_for_in_iterator()?,
                Op::ForInNext => self.op_for_in_next()?,

                // ── Private (Step 9 stubs) ──────────────────────────
                Op::GetPrivate | Op::SetPrivate | Op::PrivateIn => {
                    self.read_u16_op();
                    // Simplified: leave as-is or push undefined.
                    if op == Op::GetPrivate {
                        self.pop()?;
                        self.inner.stack.push(JsValue::Undefined);
                    } else if op == Op::SetPrivate {
                        // [obj val -- val]
                        let val = self.pop()?;
                        self.pop()?;
                        self.inner.stack.push(val);
                    } else {
                        self.pop()?;
                        self.inner.stack.push(JsValue::Boolean(false));
                    }
                }
                Op::GetSuperProp | Op::SetSuperProp | Op::GetSuperElem => {
                    if matches!(op, Op::GetSuperProp | Op::SetSuperProp) {
                        self.read_u16_op();
                    }
                    if op == Op::SetSuperProp {
                        let val = self.pop()?;
                        self.inner.stack.push(val);
                    } else {
                        self.inner.stack.push(JsValue::Undefined);
                    }
                }

                // ── Generator/Async (not in M4-10) ──────────────────
                Op::Yield
                | Op::YieldDelegate
                | Op::Await
                | Op::CreateGenerator
                | Op::CreateAsyncGenerator => {
                    return Err(VmError::internal("generator/async not supported in M4-10"));
                }

                // ── Misc stubs ──────────────────────────────────────
                Op::NewTarget | Op::ImportMeta => {
                    self.inner.stack.push(JsValue::Undefined);
                }
                Op::DynamicImport => {
                    self.pop()?;
                    self.inner.stack.push(JsValue::Undefined);
                }
                Op::GetModuleVar => {
                    self.read_u16_op();
                    self.inner.stack.push(JsValue::Undefined);
                }
                Op::Wide => {
                    return Err(VmError::internal("Wide prefix not yet supported"));
                }
            }
        }
    }

    // Helper methods live in dispatch_helpers.rs.
}
