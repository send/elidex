//! Bytecode reading helpers, constant loading, and jump support for the
//! dispatch loop. Extracted from `dispatch.rs` to keep that file focused
//! on the main opcode dispatch.

use num_bigint::BigInt;

use crate::bytecode::compiled::Constant;

use super::value::{
    FuncId, JsValue, Object, ObjectKind, PropertyKey, PropertyValue, StringId, VmError,
};
use super::VmInner;

/// Parse a BigInt literal string that may have an optional sign and
/// 0x/0b/0o prefix (e.g., "-0xFF", "+0b101").
pub(super) fn parse_bigint_literal(s: &str) -> Option<BigInt> {
    let (negative, magnitude) = if let Some(rest) = s.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = s.strip_prefix('+') {
        (false, rest)
    } else {
        (false, s)
    };

    let value = if let Some(hex) = magnitude
        .strip_prefix("0x")
        .or_else(|| magnitude.strip_prefix("0X"))
    {
        BigInt::parse_bytes(hex.as_bytes(), 16)
    } else if let Some(bin) = magnitude
        .strip_prefix("0b")
        .or_else(|| magnitude.strip_prefix("0B"))
    {
        BigInt::parse_bytes(bin.as_bytes(), 2)
    } else if let Some(oct) = magnitude
        .strip_prefix("0o")
        .or_else(|| magnitude.strip_prefix("0O"))
    {
        BigInt::parse_bytes(oct.as_bytes(), 8)
    } else {
        magnitude.parse::<BigInt>().ok()
    }?;

    Some(if negative { -value } else { value })
}

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

impl VmInner {
    pub(crate) fn read_u8_op(&mut self) -> u8 {
        let frame = self.frames.last_mut().unwrap();
        let bc = &self.compiled_functions[frame.func_id.0 as usize].bytecode;
        read_u8(bc, &mut frame.ip)
    }

    pub(crate) fn read_i8_op(&mut self) -> i8 {
        let frame = self.frames.last_mut().unwrap();
        let bc = &self.compiled_functions[frame.func_id.0 as usize].bytecode;
        read_i8(bc, &mut frame.ip)
    }

    pub(crate) fn read_u16_op(&mut self) -> u16 {
        let frame = self.frames.last_mut().unwrap();
        let bc = &self.compiled_functions[frame.func_id.0 as usize].bytecode;
        read_u16(bc, &mut frame.ip)
    }

    pub(crate) fn read_i16_op(&mut self) -> i16 {
        let frame = self.frames.last_mut().unwrap();
        let bc = &self.compiled_functions[frame.func_id.0 as usize].bytecode;
        read_i16(bc, &mut frame.ip)
    }

    pub(crate) fn jump_relative(&mut self, offset: i16) {
        let frame = self.frames.last_mut().unwrap();
        let new_ip = frame.ip.wrapping_add_signed(offset as isize);
        let bytecode_len = self.compiled_functions[frame.func_id.0 as usize]
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
        let constant = self.compiled_functions[func_id.0 as usize]
            .constants
            .get(idx as usize)
            .ok_or_else(|| VmError::internal("constant index out of bounds"))?;
        match constant {
            Constant::Number(n) => Ok(JsValue::Number(*n)),
            Constant::Wtf16(v) => {
                let id = self.strings.intern_utf16(v);
                Ok(JsValue::String(id))
            }
            Constant::RegExp { pattern, flags } => {
                let regex_flags = regress_flags_from_str(flags);
                let compiled = regress::Regex::with_flags(pattern, regex_flags)
                    .map_err(|e| VmError::type_error(format!("Invalid RegExp: {e}")))?;
                let pat_id = self.strings.intern(pattern);
                let flags_id = self.strings.intern(flags);
                let proto = self.regexp_prototype;
                let obj_id = self.alloc_object(Object {
                    kind: ObjectKind::RegExp {
                        pattern: pat_id,
                        flags: flags_id,
                        compiled: Box::new(compiled),
                    },
                    storage: super::value::PropertyStorage::shaped(super::shape::ROOT_SHAPE),
                    prototype: proto,
                    extensible: true,
                });
                // source and flags are non-enumerable, non-writable (§21.2.5.10, §21.2.5.3).
                let source_key = PropertyKey::String(self.strings.intern("source"));
                self.define_shaped_property(
                    obj_id,
                    source_key,
                    PropertyValue::Data(JsValue::String(pat_id)),
                    super::shape::PropertyAttrs::BUILTIN,
                );
                let flags_key = PropertyKey::String(self.strings.intern("flags"));
                self.define_shaped_property(
                    obj_id,
                    flags_key,
                    PropertyValue::Data(JsValue::String(flags_id)),
                    super::shape::PropertyAttrs::BUILTIN,
                );
                // lastIndex: writable, non-enumerable, non-configurable (§21.2.5.3).
                let last_index_key = PropertyKey::String(self.strings.intern("lastIndex"));
                self.define_shaped_property(
                    obj_id,
                    last_index_key,
                    PropertyValue::Data(JsValue::Number(0.0)),
                    super::shape::PropertyAttrs::WRITABLE_HIDDEN,
                );
                Ok(JsValue::Object(obj_id))
            }
            Constant::BigInt(ref s) => {
                let bi = parse_bigint_literal(s)
                    .ok_or_else(|| VmError::internal("invalid bigint constant"))?;
                let id = self.bigints.alloc(bi);
                Ok(JsValue::BigInt(id))
            }
            Constant::Function(_) // loaded via Closure opcode, not PushConst
            | Constant::TemplateObject { .. } => Ok(JsValue::Undefined),
        }
    }

    pub(crate) fn constant_to_string_id(
        &mut self,
        func_id: FuncId,
        idx: u16,
    ) -> Result<StringId, VmError> {
        let constant = self.compiled_functions[func_id.0 as usize]
            .constants
            .get(idx as usize)
            .ok_or_else(|| VmError::internal("constant index out of bounds"))?;
        match constant {
            Constant::Wtf16(v) => {
                let id = self.strings.intern_utf16(v);
                Ok(id)
            }
            _ => Err(VmError::internal("expected string constant")),
        }
    }
}
