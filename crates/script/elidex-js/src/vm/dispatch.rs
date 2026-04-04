//! Main bytecode dispatch loop.
//!
//! Contains `Vm::run()` — the core opcode-dispatch loop — along with the
//! bytecode reading helpers it uses.

use crate::bytecode::compiled::Constant;
use crate::bytecode::opcode::Op;

use super::coerce::{
    abstract_eq, op_add, op_bitnot, op_neg, op_not, op_pos, op_void, strict_eq, to_boolean,
    to_number, to_string, typeof_str, BitwiseOp, NumericBinaryOp,
};
use super::value::{
    ArrayIterState, ForInState, FuncId, JsValue, Object, ObjectKind, Property, StringId, VmError,
    VmErrorKind,
};
use super::Vm;

// ---------------------------------------------------------------------------
// Bytecode reading helpers (free functions)
// ---------------------------------------------------------------------------

/// Read a u8 from bytecode at `ip`, advancing ip.
#[inline]
fn read_u8(bytecode: &[u8], ip: &mut usize) -> u8 {
    let val = bytecode[*ip];
    *ip += 1;
    val
}

/// Read a u16 (little-endian) from bytecode at `ip`, advancing ip.
#[inline]
fn read_u16(bytecode: &[u8], ip: &mut usize) -> u16 {
    let lo = u16::from(bytecode[*ip]);
    let hi = u16::from(bytecode[*ip + 1]);
    *ip += 2;
    lo | (hi << 8)
}

/// Read an i16 (little-endian) from bytecode at `ip`, advancing ip.
#[inline]
fn read_i16(bytecode: &[u8], ip: &mut usize) -> i16 {
    read_u16(bytecode, ip).cast_signed()
}

/// Read an i8 from bytecode at `ip`, advancing ip.
#[inline]
fn read_i8(bytecode: &[u8], ip: &mut usize) -> i8 {
    read_u8(bytecode, ip).cast_signed()
}

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
                        // Create a ReferenceError object and throw it through
                        // the JS exception handling path (try/catch).
                        let msg = "Cannot access variable before initialization";
                        let msg_id = self.inner.strings.intern(msg);
                        let err_name_id = self.inner.strings.intern("ReferenceError");
                        let err_obj = self.alloc_object(Object {
                            kind: ObjectKind::Error { name: err_name_id },
                            properties: vec![
                                (
                                    self.inner.well_known.message,
                                    Property::data(JsValue::String(msg_id)),
                                ),
                                (
                                    self.inner.well_known.name,
                                    Property::data(JsValue::String(err_name_id)),
                                ),
                            ],
                            prototype: None,
                        });
                        let thrown = JsValue::Object(err_obj);
                        if self.handle_exception(thrown, entry_frame_depth) {
                            continue;
                        }
                        return Err(VmError::reference_error(msg));
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
                    let val = self
                        .inner
                        .globals
                        .get(&name_id)
                        .copied()
                        .unwrap_or(JsValue::Undefined);
                    self.inner.stack.push(val);
                }
                Op::SetGlobal => {
                    let name_idx = self.read_u16_op();
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    let val = self.peek()?;
                    self.inner.globals.insert(name_id, val);
                }

                // ── Arithmetic ──────────────────────────────────────
                Op::Add => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    let r = op_add(&mut self.inner, a, b);
                    self.inner.stack.push(r);
                }
                Op::Sub => self.binary_numeric(NumericBinaryOp::Sub)?,
                Op::Mul => self.binary_numeric(NumericBinaryOp::Mul)?,
                Op::Div => self.binary_numeric(NumericBinaryOp::Div)?,
                Op::Mod => self.binary_numeric(NumericBinaryOp::Rem)?,
                Op::Exp => self.binary_numeric(NumericBinaryOp::Exp)?,

                // ── Bitwise ─────────────────────────────────────────
                Op::BitAnd => self.binary_bitwise(BitwiseOp::And)?,
                Op::BitOr => self.binary_bitwise(BitwiseOp::Or)?,
                Op::BitXor => self.binary_bitwise(BitwiseOp::Xor)?,
                Op::Shl => self.binary_bitwise(BitwiseOp::Shl)?,
                Op::Shr => self.binary_bitwise(BitwiseOp::Shr)?,
                Op::UShr => self.binary_bitwise(BitwiseOp::UShr)?,

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
                Op::Lt => self.relational_op(false, false)?,
                Op::LtEq => self.relational_op(false, true)?,
                Op::Gt => self.relational_op(true, false)?,
                Op::GtEq => self.relational_op(true, true)?,

                Op::Instanceof => {
                    let rhs = self.pop()?; // constructor
                    let lhs = self.pop()?; // object
                                           // Walk lhs's prototype chain looking for rhs.prototype
                    let result =
                        if let (JsValue::Object(obj_id), JsValue::Object(ctor_id)) = (lhs, rhs) {
                            let proto_key = self.inner.well_known.prototype;
                            let ctor_proto =
                                super::coerce::get_property(&self.inner, ctor_id, proto_key);
                            if let Some(JsValue::Object(target_proto)) = ctor_proto {
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
                        let key_id = to_string(&mut self.inner, lhs);
                        let obj = self.inner.get_object(obj_id);
                        let found = if let ObjectKind::Array { ref elements } = obj.kind {
                            let key_str = self.inner.strings.get(key_id);
                            if let Ok(idx) = key_str.parse::<usize>() {
                                idx < elements.len()
                            } else {
                                super::coerce::get_property(&self.inner, obj_id, key_id).is_some()
                            }
                        } else {
                            super::coerce::get_property(&self.inner, obj_id, key_id).is_some()
                        };
                        self.inner.stack.push(JsValue::Boolean(found));
                    } else {
                        let msg_str = self.inner.strings.intern(
                            "Cannot use 'in' operator to search for property in non-object",
                        );
                        let type_error_name = self.inner.strings.intern("TypeError");
                        let error_obj = self.alloc_object(Object {
                            kind: ObjectKind::Error {
                                name: type_error_name,
                            },
                            properties: vec![
                                (
                                    self.inner.well_known.message,
                                    Property::data(JsValue::String(msg_str)),
                                ),
                                (
                                    self.inner.well_known.name,
                                    Property::data(JsValue::String(type_error_name)),
                                ),
                            ],
                            prototype: None,
                        });
                        let val = JsValue::Object(error_obj);
                        if self.handle_exception(val, entry_frame_depth) {
                            continue;
                        }
                        return Err(VmError::type_error(
                            "Cannot use 'in' operator to search for property in non-object",
                        ));
                    }
                }

                // ── Unary ───────────────────────────────────────────
                Op::Neg => {
                    let a = self.pop()?;
                    self.inner.stack.push(op_neg(&self.inner, a));
                }
                Op::Pos => {
                    let a = self.pop()?;
                    self.inner.stack.push(op_pos(&self.inner, a));
                }
                Op::Not => {
                    let a = self.pop()?;
                    self.inner.stack.push(op_not(&self.inner, a));
                }
                Op::BitNot => {
                    let a = self.pop()?;
                    self.inner.stack.push(op_bitnot(&self.inner, a));
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
                    let old = to_number(&self.inner, self.inner.stack[base + slot]);
                    let new = if op == Op::IncLocal {
                        old + 1.0
                    } else {
                        old - 1.0
                    };
                    self.inner.stack[base + slot] = JsValue::Number(new);
                    let push_val = if prefix { new } else { old };
                    self.inner.stack.push(JsValue::Number(push_val));
                }
                Op::IncProp | Op::DecProp => {
                    let name_idx = self.read_u16_op();
                    let prefix = self.read_u8_op() != 0;
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    let obj_val = self.pop()?;
                    let old = if let JsValue::Object(id) = obj_val {
                        super::coerce::get_property(&self.inner, id, name_id)
                            .unwrap_or(JsValue::Undefined)
                    } else {
                        JsValue::Undefined
                    };
                    let old_num = to_number(&self.inner, old);
                    let new_num = if op == Op::IncProp {
                        old_num + 1.0
                    } else {
                        old_num - 1.0
                    };
                    if let JsValue::Object(id) = obj_val {
                        self.set_property_val(
                            JsValue::Object(id),
                            name_id,
                            JsValue::Number(new_num),
                        )?;
                    }
                    self.inner
                        .stack
                        .push(JsValue::Number(if prefix { new_num } else { old_num }));
                }
                Op::IncElem | Op::DecElem => {
                    let prefix = self.read_u8_op() != 0;
                    let key = self.pop()?;
                    let obj_val = self.pop()?;
                    let old = self.get_element(obj_val, key)?;
                    let old_num = to_number(&self.inner, old);
                    let new_num = if op == Op::IncElem {
                        old_num + 1.0
                    } else {
                        old_num - 1.0
                    };
                    self.set_element(obj_val, key, JsValue::Number(new_num))?;
                    self.inner
                        .stack
                        .push(JsValue::Number(if prefix { new_num } else { old_num }));
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
                    let val = self.get_property_val(obj_val, name_id)?;
                    self.inner.stack.push(val);
                }
                Op::SetProp => {
                    let name_idx = self.read_u16_op();
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    let val = self.pop()?;
                    let obj_val = self.pop()?;
                    self.set_property_val(obj_val, name_id, val)?;
                    self.inner.stack.push(val);
                }
                Op::GetElem => {
                    let key = self.pop()?;
                    let obj = self.pop()?;
                    let val = self.get_element(obj, key)?;
                    self.inner.stack.push(val);
                }
                Op::SetElem => {
                    let val = self.pop()?;
                    let key = self.pop()?;
                    let obj = self.pop()?;
                    self.set_element(obj, key, val)?;
                    self.inner.stack.push(val);
                }
                Op::DeleteProp => {
                    let name_idx = self.read_u16_op();
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    let obj_val = self.pop()?;
                    if let JsValue::Object(id) = obj_val {
                        let obj = self.get_object_mut(id);
                        obj.properties.retain(|(k, _)| *k != name_id);
                    }
                    self.inner.stack.push(JsValue::Boolean(true));
                }
                Op::DeleteElem => {
                    let key = self.pop()?;
                    let obj_val = self.pop()?;
                    if let JsValue::Object(id) = obj_val {
                        let key_id = to_string(&mut self.inner, key);
                        let obj = self.get_object_mut(id);
                        obj.properties.retain(|(k, _)| *k != key_id);
                    }
                    self.inner.stack.push(JsValue::Boolean(true));
                }

                // ── Object/Array creation ───────────────────────────
                Op::CreateObject => {
                    let proto = self.inner.object_prototype;
                    let id = self.alloc_object(super::value::Object {
                        kind: ObjectKind::Ordinary,
                        properties: Vec::new(),
                        prototype: proto,
                    });
                    self.inner.stack.push(JsValue::Object(id));
                }
                Op::DefineProperty => {
                    let name_idx = self.read_u16_op();
                    let name_id = self.constant_to_string_id(func_id, name_idx)?;
                    let val = self.pop()?;
                    let obj_val = self.peek()?;
                    if let JsValue::Object(id) = obj_val {
                        let obj = self.get_object_mut(id);
                        // Overwrite if key already exists (e.g. after spread).
                        if let Some(existing) =
                            obj.properties.iter_mut().find(|(k, _)| *k == name_id)
                        {
                            existing.1 = Property::data(val);
                        } else {
                            obj.properties.push((name_id, Property::data(val)));
                        }
                    }
                }
                Op::DefineComputedProperty => {
                    let val = self.pop()?;
                    let key = self.pop()?;
                    let obj_val = self.peek()?;
                    if let JsValue::Object(id) = obj_val {
                        let key_id = to_string(&mut self.inner, key);
                        self.get_object_mut(id)
                            .properties
                            .push((key_id, Property::data(val)));
                    }
                }
                Op::CreateArray => {
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
                Op::ArrayPush => {
                    let val = self.pop()?;
                    let arr_val = self.peek()?;
                    if let JsValue::Object(id) = arr_val {
                        if let ObjectKind::Array { ref mut elements } = self.get_object_mut(id).kind
                        {
                            elements.push(val);
                        }
                    }
                }
                Op::ArrayHole => {
                    let arr_val = self.peek()?;
                    if let JsValue::Object(id) = arr_val {
                        if let ObjectKind::Array { ref mut elements } = self.get_object_mut(id).kind
                        {
                            elements.push(JsValue::Undefined);
                        }
                    }
                }
                Op::ArraySpread => {
                    let source = self.pop()?;
                    let arr_val = self.peek()?;
                    if let (JsValue::Object(src_id), JsValue::Object(arr_id)) = (source, arr_val) {
                        let src = self.inner.get_object(src_id);
                        if let ObjectKind::Array { elements } = &src.kind {
                            let elems: Vec<JsValue> = elements.clone();
                            let arr = self.inner.get_object_mut(arr_id);
                            if let ObjectKind::Array {
                                elements: ref mut target,
                            } = arr.kind
                            {
                                target.extend(elems);
                            }
                        }
                    }
                }
                Op::SpreadObject => {
                    let source = self.pop()?;
                    let obj_val = self.peek()?;
                    if let (JsValue::Object(src_id), JsValue::Object(dst_id)) = (source, obj_val) {
                        let src = self.inner.get_object(src_id);
                        let props: Vec<(StringId, Property)> = src
                            .properties
                            .iter()
                            .filter(|(_, p)| p.enumerable)
                            .map(|(k, p)| (*k, Property::data(p.value)))
                            .collect();
                        let dst = self.inner.get_object_mut(dst_id);
                        for (k, p) in props {
                            if let Some(existing) =
                                dst.properties.iter_mut().find(|(ek, _)| *ek == k)
                            {
                                existing.1 = p;
                            } else {
                                dst.properties.push((k, p));
                            }
                        }
                    }
                }

                // ── Template ────────────────────────────────────────
                Op::TemplateConcat => {
                    let count = self.read_u16_op() as usize;
                    let start = self.inner.stack.len() - count;
                    let parts: Vec<JsValue> = self.inner.stack[start..].to_vec();
                    self.inner.stack.truncate(start);
                    let mut result = String::new();
                    for val in parts {
                        let s_id = to_string(&mut self.inner, val);
                        result.push_str(self.inner.strings.get(s_id));
                    }
                    let id = self.inner.strings.intern(&result);
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

                // ── Stubs for remaining opcodes ─────────────────────
                Op::DefineGetter | Op::DefineSetter => {
                    self.read_u16_op();
                    self.pop()?; // closure
                                 // Leave object on stack.
                }
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
                    let val = self.pop()?;
                    // For arrays, create an ArrayIterator.
                    if let JsValue::Object(obj_id) = val {
                        let obj_ref = self.inner.objects[obj_id.0 as usize]
                            .as_ref()
                            .ok_or_else(|| VmError::type_error("cannot iterate freed object"))?;
                        if matches!(obj_ref.kind, ObjectKind::Array { .. }) {
                            let iter_obj = self.alloc_object(Object {
                                kind: ObjectKind::ArrayIterator(ArrayIterState {
                                    array_id: obj_id,
                                    index: 0,
                                }),
                                properties: Vec::new(),
                                prototype: None,
                            });
                            self.inner.stack.push(JsValue::Object(iter_obj));
                        } else {
                            // Non-array objects: not iterable for now.
                            self.inner.stack.push(JsValue::Undefined);
                        }
                    } else {
                        self.inner.stack.push(JsValue::Undefined);
                    }
                }
                Op::IteratorNext => {
                    // Stack: [iterator] → [iterator value done]
                    let iter_val = *self
                        .inner
                        .stack
                        .last()
                        .ok_or_else(|| VmError::internal("empty stack on IteratorNext"))?;
                    if let JsValue::Object(iter_id) = iter_val {
                        let (array_id, idx) = {
                            let iter_obj = self.inner.objects[iter_id.0 as usize]
                                .as_ref()
                                .ok_or_else(|| VmError::internal("freed iterator"))?;
                            if let ObjectKind::ArrayIterator(state) = &iter_obj.kind {
                                (state.array_id, state.index)
                            } else {
                                self.inner.stack.push(JsValue::Undefined);
                                self.inner.stack.push(JsValue::Boolean(true));
                                continue;
                            }
                        };
                        // Read element from array.
                        let (value, done) = {
                            let arr_obj = self.inner.objects[array_id.0 as usize]
                                .as_ref()
                                .ok_or_else(|| VmError::internal("freed array in iterator"))?;
                            if let ObjectKind::Array { elements } = &arr_obj.kind {
                                if idx < elements.len() {
                                    (elements[idx], false)
                                } else {
                                    (JsValue::Undefined, true)
                                }
                            } else {
                                (JsValue::Undefined, true)
                            }
                        };
                        // Advance index.
                        if !done {
                            let iter_obj = self.inner.objects[iter_id.0 as usize]
                                .as_mut()
                                .ok_or_else(|| VmError::internal("freed iterator"))?;
                            if let ObjectKind::ArrayIterator(state) = &mut iter_obj.kind {
                                state.index += 1;
                            }
                        }
                        self.inner.stack.push(value);
                        self.inner.stack.push(JsValue::Boolean(done));
                    } else {
                        self.inner.stack.push(JsValue::Undefined);
                        self.inner.stack.push(JsValue::Boolean(true));
                    }
                }
                Op::IteratorRest => {
                    // Stack: [iterator] → [rest_array]
                    // Collect remaining iterator elements into a new array.
                    let iter_val = self.pop()?;
                    let mut elements = Vec::new();
                    if let JsValue::Object(iter_id) = iter_val {
                        loop {
                            let (array_id, idx) = {
                                let iter_obj = self.inner.objects[iter_id.0 as usize]
                                    .as_ref()
                                    .ok_or_else(|| VmError::internal("freed iterator"))?;
                                if let ObjectKind::ArrayIterator(state) = &iter_obj.kind {
                                    (state.array_id, state.index)
                                } else {
                                    break;
                                }
                            };
                            let value = {
                                let arr_obj = self.inner.objects[array_id.0 as usize]
                                    .as_ref()
                                    .ok_or_else(|| VmError::internal("freed array"))?;
                                if let ObjectKind::Array {
                                    elements: arr_elems,
                                } = &arr_obj.kind
                                {
                                    if idx >= arr_elems.len() {
                                        break;
                                    }
                                    arr_elems[idx]
                                } else {
                                    break;
                                }
                            };
                            elements.push(value);
                            // Advance index.
                            if let Some(obj) = self.inner.objects[iter_id.0 as usize].as_mut() {
                                if let ObjectKind::ArrayIterator(state) = &mut obj.kind {
                                    state.index += 1;
                                }
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
                Op::IteratorClose | Op::DestructureElem | Op::Debugger => {
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
                    let val = self.pop()?;
                    let obj_val = self.peek()?;
                    if let JsValue::Object(id) = obj_val {
                        // Class methods are non-enumerable per ES2020 spec.
                        self.get_object_mut(id)
                            .properties
                            .push((name_id, Property::method(val)));
                    }
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
                Op::ForInIterator => {
                    let obj = self.pop()?;
                    // Collect enumerable string keys from the object and its
                    // prototype chain, skipping shadowed properties.
                    let keys = if let JsValue::Object(obj_id) = obj {
                        let mut keys = Vec::new();
                        let mut seen = std::collections::HashSet::new();
                        let mut current = Some(obj_id);
                        while let Some(id) = current {
                            let obj_ref =
                                self.inner.objects[id.0 as usize].as_ref().ok_or_else(|| {
                                    VmError::type_error("cannot iterate freed object")
                                })?;
                            for (key, prop) in &obj_ref.properties {
                                if prop.enumerable && seen.insert(*key) {
                                    keys.push(*key);
                                }
                            }
                            current = obj_ref.prototype;
                        }
                        keys
                    } else {
                        Vec::new()
                    };
                    let iter_obj = self.alloc_object(Object {
                        kind: ObjectKind::ForInIterator(ForInState { keys, index: 0 }),
                        properties: Vec::new(),
                        prototype: None,
                    });
                    self.inner.stack.push(JsValue::Object(iter_obj));
                }
                Op::ForInNext => {
                    // Stack: [iterator] → [iterator key done]
                    let iter_val = *self
                        .inner
                        .stack
                        .last()
                        .ok_or_else(|| VmError::internal("empty stack on ForInNext"))?;
                    if let JsValue::Object(iter_id) = iter_val {
                        let iter_obj = self.inner.objects[iter_id.0 as usize]
                            .as_mut()
                            .ok_or_else(|| VmError::internal("freed for-in iterator"))?;
                        if let ObjectKind::ForInIterator(state) = &mut iter_obj.kind {
                            if state.index < state.keys.len() {
                                let key_sid = state.keys[state.index];
                                state.index += 1;
                                let key_val = JsValue::String(key_sid);
                                self.inner.stack.push(key_val);
                                self.inner.stack.push(JsValue::Boolean(false)); // not done
                            } else {
                                self.inner.stack.push(JsValue::Undefined);
                                self.inner.stack.push(JsValue::Boolean(true)); // done
                            }
                        } else {
                            self.inner.stack.push(JsValue::Undefined);
                            self.inner.stack.push(JsValue::Boolean(true));
                        }
                    } else {
                        self.inner.stack.push(JsValue::Undefined);
                        self.inner.stack.push(JsValue::Boolean(true));
                    }
                }

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

    // -- Helper methods -------------------------------------------------------

    fn read_u8_op(&mut self) -> u8 {
        let frame = self.inner.frames.last_mut().unwrap();
        let bc = &self.inner.compiled_functions[frame.func_id.0 as usize].bytecode;
        read_u8(bc, &mut frame.ip)
    }

    fn read_i8_op(&mut self) -> i8 {
        let frame = self.inner.frames.last_mut().unwrap();
        let bc = &self.inner.compiled_functions[frame.func_id.0 as usize].bytecode;
        read_i8(bc, &mut frame.ip)
    }

    fn read_u16_op(&mut self) -> u16 {
        let frame = self.inner.frames.last_mut().unwrap();
        let bc = &self.inner.compiled_functions[frame.func_id.0 as usize].bytecode;
        read_u16(bc, &mut frame.ip)
    }

    fn read_i16_op(&mut self) -> i16 {
        let frame = self.inner.frames.last_mut().unwrap();
        let bc = &self.inner.compiled_functions[frame.func_id.0 as usize].bytecode;
        read_i16(bc, &mut frame.ip)
    }

    fn jump_relative(&mut self, offset: i16) {
        let frame = self.inner.frames.last_mut().unwrap();
        // offset is relative to the ip AFTER reading the operand
        let new_ip = frame.ip.wrapping_add_signed(offset as isize);
        let bytecode_len = self.inner.compiled_functions[frame.func_id.0 as usize]
            .bytecode
            .len();
        debug_assert!(
            new_ip <= bytecode_len,
            "invalid jump: ip={}, offset={offset}, bytecode_len={bytecode_len}",
            frame.ip
        );
        frame.ip = new_ip;
    }

    fn load_constant(&mut self, func_id: FuncId, idx: u16) -> Result<JsValue, VmError> {
        let constant = self.inner.compiled_functions[func_id.0 as usize]
            .constants
            .get(idx as usize)
            .ok_or_else(|| VmError::internal("constant index out of bounds"))?;
        match constant {
            Constant::Number(n) => Ok(JsValue::Number(*n)),
            Constant::String(s) => {
                let id = self.inner.strings.intern(s);
                Ok(JsValue::String(id))
            }
            Constant::BigInt(_) // deferred to M4-12
            | Constant::RegExp { .. } // deferred to M4-10.2
            | Constant::Function(_) // loaded via Closure opcode, not PushConst
            | Constant::TemplateObject { .. } => Ok(JsValue::Undefined),
        }
    }

    fn constant_to_string_id(&mut self, func_id: FuncId, idx: u16) -> Result<StringId, VmError> {
        let constant = self.inner.compiled_functions[func_id.0 as usize]
            .constants
            .get(idx as usize)
            .ok_or_else(|| VmError::internal("constant index out of bounds"))?;
        match constant {
            Constant::String(s) => {
                let id = self.inner.strings.intern(s);
                Ok(id)
            }
            _ => Err(VmError::internal("expected string constant")),
        }
    }
}
