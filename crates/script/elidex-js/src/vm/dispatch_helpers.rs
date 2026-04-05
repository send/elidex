//! Bytecode reading helpers, constant loading, and jump support for the
//! dispatch loop. Extracted from `dispatch.rs` to keep that file focused
//! on the main opcode dispatch.

use crate::bytecode::compiled::Constant;

use super::value::{FuncId, JsValue, Object, ObjectKind, Property, PropertyKey, StringId, VmError};
use super::Vm;

// ---------------------------------------------------------------------------
// Bytecode reading (free functions, used by methods below)
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

/// Convert a JS flags string (e.g. "gi") into `regress::Flags`.
pub(super) fn regress_flags_from_str(flags: &str) -> regress::Flags {
    let mut f = regress::Flags::default();
    for ch in flags.chars() {
        match ch {
            'i' => f.icase = true,
            'm' => f.multiline = true,
            's' => f.dot_all = true,
            'u' => f.unicode = true,
            _ => {}
        }
    }
    f
}

// ---------------------------------------------------------------------------
// Vm helper methods
// ---------------------------------------------------------------------------

impl Vm {
    pub(crate) fn read_u8_op(&mut self) -> u8 {
        let frame = self.inner.frames.last_mut().unwrap();
        let bc = &self.inner.compiled_functions[frame.func_id.0 as usize].bytecode;
        read_u8(bc, &mut frame.ip)
    }

    pub(crate) fn read_i8_op(&mut self) -> i8 {
        let frame = self.inner.frames.last_mut().unwrap();
        let bc = &self.inner.compiled_functions[frame.func_id.0 as usize].bytecode;
        read_i8(bc, &mut frame.ip)
    }

    pub(crate) fn read_u16_op(&mut self) -> u16 {
        let frame = self.inner.frames.last_mut().unwrap();
        let bc = &self.inner.compiled_functions[frame.func_id.0 as usize].bytecode;
        read_u16(bc, &mut frame.ip)
    }

    pub(crate) fn read_i16_op(&mut self) -> i16 {
        let frame = self.inner.frames.last_mut().unwrap();
        let bc = &self.inner.compiled_functions[frame.func_id.0 as usize].bytecode;
        read_i16(bc, &mut frame.ip)
    }

    pub(crate) fn jump_relative(&mut self, offset: i16) {
        let frame = self.inner.frames.last_mut().unwrap();
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

    pub(crate) fn load_constant(&mut self, func_id: FuncId, idx: u16) -> Result<JsValue, VmError> {
        let constant = self.inner.compiled_functions[func_id.0 as usize]
            .constants
            .get(idx as usize)
            .ok_or_else(|| VmError::internal("constant index out of bounds"))?;
        match constant {
            Constant::Number(n) => Ok(JsValue::Number(*n)),
            Constant::Wtf16(v) => {
                let id = self.inner.strings.intern_utf16(v);
                Ok(JsValue::String(id))
            }
            Constant::RegExp { pattern, flags } => {
                let regex_flags = regress_flags_from_str(flags);
                let compiled = regress::Regex::with_flags(pattern, regex_flags)
                    .map_err(|e| VmError::type_error(format!("Invalid RegExp: {e}")))?;
                let pat_id = self.inner.strings.intern(pattern);
                let flags_id = self.inner.strings.intern(flags);
                let proto = self.inner.regexp_prototype;
                let obj_id = self.alloc_object(Object {
                    kind: ObjectKind::RegExp {
                        pattern: pat_id,
                        flags: flags_id,
                        compiled: Box::new(compiled),
                    },
                    properties: Vec::new(),
                    prototype: proto,
                });
                // source and flags are non-enumerable, non-writable (§21.2.5.10, §21.2.5.3).
                let source_key = PropertyKey::String(self.inner.strings.intern("source"));
                self.get_object_mut(obj_id).properties.push((
                    source_key,
                    Property::builtin(JsValue::String(pat_id)),
                ));
                let flags_key = PropertyKey::String(self.inner.strings.intern("flags"));
                self.get_object_mut(obj_id).properties.push((
                    flags_key,
                    Property::builtin(JsValue::String(flags_id)),
                ));
                // lastIndex is writable but non-enumerable (§21.2.5.3).
                let last_index_key = PropertyKey::String(self.inner.strings.intern("lastIndex"));
                self.get_object_mut(obj_id).properties.push((
                    last_index_key,
                    Property::method(JsValue::Number(0.0)),
                ));
                Ok(JsValue::Object(obj_id))
            }
            Constant::BigInt(_) // deferred to M4-12
            | Constant::Function(_) // loaded via Closure opcode, not PushConst
            | Constant::TemplateObject { .. } => Ok(JsValue::Undefined),
        }
    }

    pub(crate) fn constant_to_string_id(
        &mut self,
        func_id: FuncId,
        idx: u16,
    ) -> Result<StringId, VmError> {
        let constant = self.inner.compiled_functions[func_id.0 as usize]
            .constants
            .get(idx as usize)
            .ok_or_else(|| VmError::internal("constant index out of bounds"))?;
        match constant {
            Constant::Wtf16(v) => {
                let id = self.inner.strings.intern_utf16(v);
                Ok(id)
            }
            _ => Err(VmError::internal("expected string constant")),
        }
    }
}
