//! Main bytecode dispatch loop.
//!
//! Contains `Vm::run()` — the core opcode-dispatch loop. Bytecode reading
//! helpers, constant loading, and jump support live in `dispatch_helpers.rs`.

use crate::bytecode::opcode::Op;

use super::coerce::{
    abstract_eq, op_bitnot, op_neg, op_not, op_pos, op_void, strict_eq, to_boolean, to_number,
    to_string, typeof_str, BitwiseOp, NumericBinaryOp,
};
use super::ops::parse_array_index_u16;
use super::value::{JsValue, ObjectKind, PropertyKey, VmError, VmErrorKind};
use super::VmInner;

// ---------------------------------------------------------------------------
// Main dispatch loop
// ---------------------------------------------------------------------------

impl VmInner {
    /// Execute bytecode until the current call frame returns.
    #[allow(clippy::too_many_lines)] // single dispatch loop, splitting would hurt readability
    pub(crate) fn run(&mut self) -> Result<JsValue, VmError> {
        let entry_frame_depth = self.frames.len() - 1;

        loop {
            let frame_idx = self.frames.len() - 1;
            let func_id = self.frames[frame_idx].func_id;
            let ip = self.frames[frame_idx].ip;

            let bytecode = &self.compiled_functions[func_id.0 as usize].bytecode;
            if ip >= bytecode.len() {
                // Fell off the end → implicit ReturnUndefined.
                if frame_idx == entry_frame_depth {
                    let completion = self.completion_value;
                    self.completion_value = JsValue::Undefined;
                    return Ok(completion);
                }
                self.complete_inline_frame(JsValue::Undefined);
                continue;
            }

            let op_byte = bytecode[ip];
            let op = Op::from_byte(op_byte).ok_or_else(|| {
                VmError::internal(format!("invalid opcode: {op_byte:#x} at ip={ip}"))
            })?;
            self.frames[frame_idx].ip = ip + 1;

            match op {
                // ── Stack manipulation ──────────────────────────────
                Op::PushUndefined => self.stack.push(JsValue::Undefined),
                Op::PushNull => self.stack.push(JsValue::Null),
                Op::PushTrue => self.stack.push(JsValue::Boolean(true)),
                Op::PushFalse => self.stack.push(JsValue::Boolean(false)),
                Op::PushI8 => {
                    let val = self.read_i8_op();
                    self.stack.push(JsValue::Number(f64::from(val)));
                }
                Op::PushConst => {
                    let idx = self.read_u16_op();
                    let val = self.load_constant(func_id, idx)?;
                    self.stack.push(val);
                }
                Op::Dup => {
                    let val = self.peek()?;
                    self.stack.push(val);
                }
                Op::Pop => {
                    let val = self.pop()?;
                    // At script (entry) level, capture completion value for eval.
                    if frame_idx == entry_frame_depth {
                        self.completion_value = val;
                    }
                }
                Op::Swap => {
                    let len = self.stack.len();
                    if len < 2 {
                        return Err(VmError::internal("stack underflow on Swap"));
                    }
                    self.stack.swap(len - 1, len - 2);
                }

                // ── Local access ────────────────────────────────────
                Op::GetLocal => {
                    let slot = self.read_u16_op() as usize;
                    let base = self.frames[frame_idx].base;
                    let val = self.stack[base + slot];
                    self.stack.push(val);
                }
                Op::SetLocal => {
                    let slot = self.read_u16_op() as usize;
                    let val = self.peek()?;
                    let base = self.frames[frame_idx].base;
                    self.stack[base + slot] = val;
                }
                Op::CheckTdz => {
                    let slot = self.read_u16_op() as usize;
                    if self.frames[frame_idx].is_in_tdz(slot) {
                        let err = VmError::reference_error(
                            "Cannot access variable before initialization",
                        );
                        self.throw_error(err, entry_frame_depth)?;
                    }
                }
                Op::InitLocal => {
                    let slot = self.read_u16_op() as usize;
                    self.frames[frame_idx].clear_tdz(slot);
                }

                // ── Upvalue access ──────────────────────────────────
                Op::GetUpvalue => {
                    let idx = self.read_u16_op() as usize;
                    let uv_id = self.frames[frame_idx].upvalue_ids[idx];
                    let val = self.read_upvalue(uv_id);
                    self.stack.push(val);
                }
                Op::SetUpvalue => {
                    let idx = self.read_u16_op() as usize;
                    let val = self.peek()?;
                    let uv_id = self.frames[frame_idx].upvalue_ids[idx];
                    self.write_upvalue(uv_id, val);
                }

                // ── Global access ───────────────────────────────────
                Op::GetGlobal => {
                    let name_idx = self.read_u16_op();
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    if let Some(val) = self.globals.get(&name_id).copied() {
                        self.stack.push(val);
                    } else {
                        // Fall back to the global object (supports accessor properties
                        // defined via Object.defineProperty(globalThis, ...)).
                        // Check property existence on the global object, then resolve.
                        let global_obj = self.global_object;
                        let pk = PropertyKey::String(name_id);
                        if let Some(result) = super::coerce::get_property(self, global_obj, pk) {
                            match self.resolve_property(result, JsValue::Object(global_obj)) {
                                Ok(val) => self.stack.push(val),
                                Err(e) => {
                                    self.throw_error(e, entry_frame_depth)?;
                                }
                            }
                        } else {
                            let name_str = self.strings.get_utf8(name_id);
                            let msg = format!("{name_str} is not defined");
                            let err = VmError::reference_error(&msg);
                            self.throw_error(err, entry_frame_depth)?;
                        }
                    }
                }
                Op::SetGlobal => {
                    let name_idx = self.read_u16_op();
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    let val = self.peek()?;
                    // §8.1.1.2.5: In strict mode, assigning to an undeclared
                    // variable is a ReferenceError.
                    let exists_on_global = {
                        let pk = PropertyKey::String(name_id);
                        self.globals.contains_key(&name_id)
                            || super::coerce::get_property(self, self.global_object, pk).is_some()
                    };
                    if self.compiled_functions[func_id.0 as usize].is_strict && !exists_on_global {
                        let name_str = self.strings.get_utf8(name_id);
                        let msg = format!("{name_str} is not defined");
                        let err = VmError::reference_error(&msg);
                        self.throw_error(err, entry_frame_depth)?;
                    } else {
                        // Check for accessor setter on globalThis before
                        // writing to the globals HashMap.
                        let global_obj = self.global_object;
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
                        Ok(r) => self.stack.push(r),
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
                    let r = abstract_eq(self, a, b);
                    self.stack.push(JsValue::Boolean(r));
                }
                Op::NotEq => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    let r = !abstract_eq(self, a, b);
                    self.stack.push(JsValue::Boolean(r));
                }
                Op::StrictEq => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(JsValue::Boolean(strict_eq(a, b)));
                }
                Op::StrictNotEq => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(JsValue::Boolean(!strict_eq(a, b)));
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
                            PropertyKey::Symbol(self.well_known_symbols.has_instance);
                        if let Some(has_instance_result) =
                            super::coerce::get_property(self, rhs_id, has_instance_key)
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
                            let bool_result = to_boolean(self, result);
                            self.stack.push(JsValue::Boolean(bool_result));
                            continue;
                        }
                    }

                    // OrdinaryHasInstance: walk lhs's prototype chain looking for rhs.prototype
                    let result =
                        if let (JsValue::Object(obj_id), JsValue::Object(ctor_id)) = (lhs, rhs) {
                            let proto_key = PropertyKey::String(self.well_known.prototype);
                            let ctor_proto = super::coerce::get_property(self, ctor_id, proto_key);
                            if let Some(super::coerce::PropertyResult::Data(JsValue::Object(
                                target_proto,
                            ))) = ctor_proto
                            {
                                let mut current = self.get_object(obj_id).prototype;
                                let mut found = false;
                                while let Some(proto_id) = current {
                                    if proto_id == target_proto {
                                        found = true;
                                        break;
                                    }
                                    current = self.get_object(proto_id).prototype;
                                }
                                found
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                    self.stack.push(JsValue::Boolean(result));
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
                        let obj = self.get_object(obj_id);
                        let found = match (&obj.kind, &pk) {
                            (ObjectKind::Array { ref elements }, PropertyKey::String(key_id)) => {
                                let key_units = self.strings.get(*key_id);
                                if let Some(idx) = parse_array_index_u16(key_units) {
                                    idx < elements.len()
                                } else {
                                    super::coerce::get_property(self, obj_id, pk).is_some()
                                }
                            }
                            _ => super::coerce::get_property(self, obj_id, pk).is_some(),
                        };
                        self.stack.push(JsValue::Boolean(found));
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
                    match op_neg(self, a) {
                        Ok(r) => self.stack.push(r),
                        Err(e) => self.throw_error(e, entry_frame_depth)?,
                    }
                }
                Op::Pos => {
                    let a = self.pop()?;
                    match op_pos(self, a) {
                        Ok(r) => self.stack.push(r),
                        Err(e) => self.throw_error(e, entry_frame_depth)?,
                    }
                }
                Op::Not => {
                    let a = self.pop()?;
                    self.stack.push(op_not(self, a));
                }
                Op::BitNot => {
                    let a = self.pop()?;
                    match op_bitnot(self, a) {
                        Ok(r) => self.stack.push(r),
                        Err(e) => self.throw_error(e, entry_frame_depth)?,
                    }
                }
                Op::TypeOf => {
                    let a = self.pop()?;
                    let s = typeof_str(self, a);
                    self.stack.push(JsValue::String(s));
                }
                Op::TypeOfGlobal => {
                    let name_idx = self.read_u16_op();
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    let val = if let Some(&v) = self.globals.get(&name_id) {
                        v
                    } else {
                        // Fall back to global object (supports accessor properties).
                        let global_obj = self.global_object;
                        let pk = PropertyKey::String(name_id);
                        match super::coerce::get_property(self, global_obj, pk) {
                            Some(result) => {
                                match self.resolve_property(result, JsValue::Object(global_obj)) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        self.throw_error(e, entry_frame_depth)?;
                                        continue;
                                    }
                                }
                            }
                            None => JsValue::Undefined,
                        }
                    };
                    let s = typeof_str(self, val);
                    self.stack.push(JsValue::String(s));
                }
                Op::Void => {
                    self.pop()?;
                    self.stack.push(op_void());
                }

                // ── Update operations ───────────────────────────────
                Op::IncLocal | Op::DecLocal => {
                    let slot = self.read_u16_op() as usize;
                    let prefix = self.read_u8_op() != 0;
                    let base = self.frames[frame_idx].base;
                    match to_number(self, self.stack[base + slot]) {
                        Ok(old) => {
                            let new = if op == Op::IncLocal {
                                old + 1.0
                            } else {
                                old - 1.0
                            };
                            self.stack[base + slot] = JsValue::Number(new);
                            let push_val = if prefix { new } else { old };
                            self.stack.push(JsValue::Number(push_val));
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
                    match to_number(self, old) {
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
                            self.stack.push(JsValue::Number(if prefix {
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
                    match to_number(self, old) {
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
                            self.stack.push(JsValue::Number(if prefix {
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
                    if !to_boolean(self, val) {
                        self.jump_relative(offset);
                    }
                }
                Op::JumpIfTrue => {
                    let offset = self.read_i16_op();
                    let val = self.pop()?;
                    if to_boolean(self, val) {
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
                    self.complete_inline_frame(val);
                }
                Op::ReturnUndefined => {
                    if frame_idx == entry_frame_depth {
                        let completion = self.completion_value;
                        self.pop_frame();
                        self.completion_value = JsValue::Undefined;
                        return Ok(completion);
                    }
                    self.complete_inline_frame(JsValue::Undefined);
                }

                // ── Property access (Step 4 stubs) ──────────────────
                Op::GetProp => {
                    let name_idx = self.read_u16_op();
                    let ic_idx = self.read_u16_op() as usize;
                    let obj_val = self.pop()?;
                    match self.ic_get_prop(func_id, name_idx, ic_idx, obj_val) {
                        Ok(val) => self.stack.push(val),
                        Err(e) => {
                            self.throw_error(e, entry_frame_depth)?;
                        }
                    }
                }
                Op::SetProp => {
                    let name_idx = self.read_u16_op();
                    let ic_idx = self.read_u16_op() as usize;
                    let val = self.pop()?;
                    let obj_val = self.pop()?;
                    match self.ic_set_prop(func_id, name_idx, ic_idx, obj_val, val) {
                        Ok(v) => self.stack.push(v),
                        Err(e) => {
                            self.throw_error(e, entry_frame_depth)?;
                        }
                    }
                }
                Op::GetElem => {
                    let key = self.pop()?;
                    let obj = self.pop()?;
                    match self.get_element(obj, key) {
                        Ok(val) => self.stack.push(val),
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
                    self.stack.push(val);
                }
                Op::DeleteProp => {
                    let name_idx = self.read_u16_op();
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    let pk = PropertyKey::String(name_id);
                    let obj_val = self.pop()?;
                    let deleted = if let JsValue::Object(id) = obj_val {
                        match self.try_delete_property(id, pk) {
                            Ok(d) => d,
                            Err(e) => {
                                self.throw_error(e, entry_frame_depth)?;
                                continue;
                            }
                        }
                    } else {
                        true
                    };
                    self.stack.push(JsValue::Boolean(deleted));
                }
                Op::DeleteElem => {
                    let key = self.pop()?;
                    let obj_val = self.pop()?;
                    let deleted = if let JsValue::Object(id) = obj_val {
                        match self.make_property_key(key) {
                            Ok(pk) => match self.try_delete_property(id, pk) {
                                Ok(d) => d,
                                Err(e) => {
                                    self.throw_error(e, entry_frame_depth)?;
                                    continue;
                                }
                            },
                            Err(e) => {
                                self.throw_error(e, entry_frame_depth)?;
                                continue;
                            }
                        }
                    } else {
                        true
                    };
                    self.stack.push(JsValue::Boolean(deleted));
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
                Op::SpreadObject => {
                    if let Err(e) = self.op_spread_object() {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }

                // ── Template ────────────────────────────────────────
                Op::TemplateConcat => {
                    let count = self.read_u16_op() as usize;
                    let start = self.stack.len() - count;
                    let parts: Vec<JsValue> = self.stack[start..].to_vec();
                    self.stack.truncate(start);
                    let mut result: Vec<u16> = Vec::new();
                    let mut err: Option<VmError> = None;
                    for val in parts {
                        match to_string(self, val) {
                            Ok(s_id) => result.extend_from_slice(self.strings.get(s_id)),
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
                    let id = self.strings.intern_utf16(&result);
                    self.stack.push(JsValue::String(id));
                }

                // ── Function call ───────────────────────────────────
                Op::Call => {
                    let argc = self.read_u8_op() as usize;
                    let call_ic_idx = self.read_u16_op() as usize;
                    if let Err(e) = self.ic_call(func_id, argc, call_ic_idx) {
                        let thrown = self.vm_error_to_thrown(&e);
                        if self.handle_exception(thrown, entry_frame_depth) {
                            continue;
                        }
                        return Err(e);
                    }
                }
                Op::CallMethod => {
                    let argc = self.read_u8_op() as usize;
                    let call_ic_idx = self.read_u16_op() as usize;
                    if let Err(e) = self.ic_call_method(func_id, argc, call_ic_idx) {
                        let thrown = if let VmErrorKind::ThrowValue(val) = e.kind {
                            val
                        } else {
                            let msg = self.strings.intern(&e.to_string());
                            JsValue::String(msg)
                        };
                        if self.handle_exception(thrown, entry_frame_depth) {
                            continue;
                        }
                        return Err(e);
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
                    let this = self.frames[frame_idx].this_value;
                    self.stack.push(this);
                }
                Op::Closure => {
                    let const_idx = self.read_u16_op();
                    self.create_closure(func_id, const_idx)?;
                }

                // ── Exception handling ───────────────────────────────
                Op::PushExceptionHandler => {
                    let catch_ip = self.read_u16_op();
                    let finally_ip = self.read_u16_op();
                    let stack_depth = self.stack.len();
                    let frame = self.frames.last_mut().unwrap();
                    frame.exception_handlers.push(super::value::HandlerEntry {
                        catch_ip: u32::from(catch_ip),
                        finally_ip: u32::from(finally_ip),
                        stack_depth,
                    });
                }
                Op::PopExceptionHandler => {
                    let frame = self.frames.last_mut().unwrap();
                    frame.exception_handlers.pop();
                }
                Op::Throw => {
                    let val = self.pop()?;
                    // Try to find an exception handler.
                    if self.handle_exception(val, entry_frame_depth) {
                        // Handler found and activated — continue the dispatch loop.
                        continue;
                    }
                    // No handler found — clean up frames above the entry
                    // frame so subsequent `eval()` calls don't see stale
                    // state.  The entry frame itself is NOT popped here:
                    // for nested `run()` calls (re-entrant native → JS
                    // callbacks), the caller's frames must survive so that
                    // the outer dispatch loop can still catch the exception.
                    while self.frames.len() > entry_frame_depth + 1 {
                        let frame = self.frames.pop().unwrap();
                        self.close_upvalues(&frame.local_upvalue_ids);
                        self.stack.truncate(frame.base);
                    }
                    return Err(VmError {
                        kind: VmErrorKind::ThrowValue(val),
                        message: "uncaught throw".into(),
                    });
                }
                Op::PushException => {
                    let exc = self.current_exception;
                    self.stack.push(exc);
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
                    let args = self.frames[frame_idx]
                        .actual_args
                        .take()
                        .unwrap_or_default();
                    let len = args.len();
                    // GC safety: args (taken from frame) are in a Rust-local Vec.
                    let saved_gc = self.gc_enabled;
                    self.gc_enabled = false;
                    let args_obj = self.alloc_object(super::value::Object {
                        kind: ObjectKind::Arguments { values: args },
                        storage: super::value::PropertyStorage::shaped(super::shape::ROOT_SHAPE),
                        prototype: self.object_prototype,
                    });
                    self.gc_enabled = saved_gc;
                    // Set the `length` property.
                    let length_key = PropertyKey::String(self.well_known.length);
                    #[allow(clippy::cast_precision_loss)]
                    // arguments.length is non-enumerable (§10.4.4.5).
                    self.define_shaped_property(
                        args_obj,
                        length_key,
                        super::value::PropertyValue::Data(JsValue::Number(len as f64)),
                        super::shape::PropertyAttrs::METHOD,
                    );
                    self.stack.push(JsValue::Object(args_obj));
                }

                // ── Stubs for remaining opcodes ─────────────────────
                Op::CallSpread | Op::NewSpread | Op::SuperCallSpread => {
                    self.pop()?; // args array
                    self.pop()?; // callee/constructor
                    self.stack.push(JsValue::Undefined);
                }
                Op::TaggedTemplate => {
                    let _count = self.read_u8_op();
                    // Simplified stub.
                    self.stack.push(JsValue::Undefined);
                }

                // ── Iteration ───────────────────────────────────────
                Op::GetIterator => {
                    if let Err(e) = self.op_get_iterator() {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }
                Op::IteratorNext => {
                    if let Err(e) = self.op_iterator_next() {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }
                Op::IteratorRest => {
                    if let Err(e) = self.op_iterator_rest() {
                        self.throw_error(e, entry_frame_depth)?;
                    }
                }
                Op::IteratorClose => {
                    if let Err(e) = self.op_iterator_close() {
                        self.throw_error(e, entry_frame_depth)?;
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
                    self.stack.push(JsValue::Undefined);
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
                    self.stack.push(JsValue::Undefined);
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
                        self.stack.push(JsValue::Undefined);
                    } else if op == Op::SetPrivate {
                        // [obj val -- val]
                        let val = self.pop()?;
                        self.pop()?;
                        self.stack.push(val);
                    } else {
                        self.pop()?;
                        self.stack.push(JsValue::Boolean(false));
                    }
                }
                Op::GetSuperProp | Op::SetSuperProp | Op::GetSuperElem => {
                    if matches!(op, Op::GetSuperProp | Op::SetSuperProp) {
                        self.read_u16_op();
                    }
                    if op == Op::SetSuperProp {
                        let val = self.pop()?;
                        self.stack.push(val);
                    } else {
                        self.stack.push(JsValue::Undefined);
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
                    self.stack.push(JsValue::Undefined);
                }
                Op::DynamicImport => {
                    self.pop()?;
                    self.stack.push(JsValue::Undefined);
                }
                Op::GetModuleVar => {
                    self.read_u16_op();
                    self.stack.push(JsValue::Undefined);
                }
                Op::Wide => {
                    return Err(VmError::internal("Wide prefix not yet supported"));
                }
            }
        }
    }

    /// Pop a non-entry call frame pushed by the single dispatcher, restore
    /// parent state, and push the return value onto the caller's stack.
    ///
    /// Handles constructor semantics: if `new_target` is set and `return_value`
    /// is not an object, the instance is returned instead.
    fn complete_inline_frame(&mut self, return_value: JsValue) {
        let frame = self.frames.pop().unwrap();
        self.close_upvalues(&frame.local_upvalue_ids);
        self.completion_value = frame.saved_completion;
        let final_val = if let Some(instance_id) = frame.new_target {
            if matches!(return_value, JsValue::Object(_)) {
                return_value
            } else {
                JsValue::Object(instance_id)
            }
        } else {
            return_value
        };
        self.stack.truncate(frame.cleanup_base);
        self.stack.push(final_val);
    }

    // Helper methods live in dispatch_helpers.rs.
}
