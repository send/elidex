//! Bytecode reading helpers, constant loading, and jump support for the
//! dispatch loop. Extracted from `dispatch.rs` to keep that file focused
//! on the main opcode dispatch.

use num_bigint::BigInt;

use crate::bytecode::compiled::Constant;

use super::coerce::to_number;
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
    /// `Op::TemplateConcat`: pop `count` values from the stack, `ToString`
    /// each, concatenate their WTF-16 units, intern the result, and push
    /// the resulting `JsValue::String` back.  Extracted from the dispatch
    /// loop so the opcode arm stays compact.
    pub(crate) fn op_template_concat(&mut self, count: usize) -> Result<(), super::value::VmError> {
        let start = self.stack.len() - count;
        let parts: Vec<super::value::JsValue> = self.stack[start..].to_vec();
        self.stack.truncate(start);
        let mut result: Vec<u16> = Vec::new();
        for val in parts {
            let sid = super::coerce::to_string(self, val)?;
            result.extend_from_slice(self.strings.get(sid));
        }
        let id = self.strings.intern_utf16(&result);
        self.stack.push(super::value::JsValue::String(id));
        Ok(())
    }

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
                let source_key = PropertyKey::String(self.well_known.source);
                self.define_shaped_property(
                    obj_id,
                    source_key,
                    PropertyValue::Data(JsValue::String(pat_id)),
                    super::shape::PropertyAttrs::BUILTIN,
                );
                let flags_key = PropertyKey::String(self.well_known.flags);
                self.define_shaped_property(
                    obj_id,
                    flags_key,
                    PropertyValue::Data(JsValue::String(flags_id)),
                    super::shape::PropertyAttrs::BUILTIN,
                );
                // lastIndex: writable, non-enumerable, non-configurable (§21.2.5.3).
                let last_index_key = PropertyKey::String(self.well_known.last_index);
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

// ── Opcode bodies added by this slice ────────────────────────────────
//
// Extracted rather than written inline: `dispatch.rs` is over the 1000-line
// threshold, so the umbrella's forward rule is to put an added arm's body in a
// `dispatch_*.rs` sibling and leave the dispatch loop a table.  The rule covers
// bodies that need only the VM and their own bytecode operands; `Op::PopCompletion`
// stays inline because it reads `frame_idx` / `entry_frame_depth` — loop state
// that is not the opcode's own — and threading both through a helper would trade
// one line of table for two of plumbing.
//
// **Error disposition** is keyed on what a body can return, which the signature
// fixes at compile time rather than leaving to whichever arm noticed:
//
// - `op_pop_under` can fail *only* by breaking a stack invariant, so its arm
//   propagates with `?` and leaves `run()` by the hard path — the same exit
//   `Op::Swap` has always taken inline for the identical condition.
// - `op_get_elem_ref` / `op_inc_dec_elem` / `op_throw_unsupported` can also
//   raise a JS-level error, so their arms hand everything to `throw_error` and
//   handler search works normally.  That leaves their (unreachable-by-
//   construction) invariant guards catchable, which is what every pre-existing
//   `op_*` helper in this crate already does; unifying the two dispositions on
//   the error's *kind* engine-wide is `#11-vm-internal-error-hard-exit`.

impl VmInner {
    /// `Op::PopUnder` — `[a1 … an v -- v]`, discard `n` slots from beneath the
    /// top while keeping the top.
    ///
    /// Drops a member target's reference slots when a logical assignment
    /// short-circuits, in one instruction rather than an `n`-fold `Swap; Pop`
    /// dance. Like `Op::Pop`, and unlike `Op::PopCompletion`, it does not write
    /// `completion_value`: only an ExpressionStatement's value belongs there
    /// (ECMA-262 §14.5.1), and recording internal housekeeping there made
    /// `eval("if (globalThis.Math ||= 2) {}")` return the global object instead
    /// of `undefined`.
    pub(super) fn op_pop_under(&mut self) -> Result<(), VmError> {
        let count = self.read_u8_op() as usize;
        let len = self.stack.len();
        if len < count + 1 {
            return Err(VmError::internal("stack underflow on PopUnder"));
        }
        let top = self.stack[len - 1];
        self.stack.truncate(len - count - 1);
        self.stack.push(top);
        Ok(())
    }

    /// `Op::GetElemRef` — `[object key -- object key' value]`, load through a
    /// computed member reference while keeping the reference for a later store.
    ///
    /// The `[object key]` pair is read **in place** rather than popped and
    /// pushed back.  The stack effect alone does not force that — a
    /// pop-then-repush produces the identical `[object key -- object key' value]`
    /// and every test still passes — but the rooting does: key conversion runs
    /// user `toString` / `@@toPrimitive`, which can allocate enough to trigger a
    /// collection, and `gc/roots.rs` walks the VM stack but **not** Rust locals.
    /// A base popped into a local could therefore be collected mid-conversion,
    /// and this opcode hands that base straight to the following `Op::SetElem` —
    /// a store through a dangling `ObjectId`, into whatever object recycled the
    /// slot, or `get_object_mut`'s "object already freed" panic.  Leaving both
    /// slots on the stack makes that unrepresentable here.  Pinned by
    /// `get_elem_ref_keeps_temporary_base_rooted`.
    ///
    /// The reference is preserved for the same §13.15.2 reason the compiler
    /// emits this opcode at all: the LeftHandSideExpression is evaluated once
    /// and that one reference serves both `GetValue` and `PutValue`.
    ///
    /// `key'` is the **converted** key: §6.2.5.5 GetValue step 3.c.i writes
    /// `ToPropertyKey`'s result back into the Reference Record, so a later
    /// `PutValue` through the same reference does not re-run user
    /// `toString`/`@@toPrimitive`. Converting inside this opcode also keeps step
    /// 3.a ahead of step 3.c, so `null[k] += 1` is a `TypeError` even when
    /// `k.toString()` would also throw.
    pub(super) fn op_get_elem_ref(&mut self) -> Result<(), VmError> {
        let len = self.stack.len();
        if len < 2 {
            return Err(VmError::internal("stack underflow on GetElemRef"));
        }
        let obj = self.stack[len - 2];
        let key = self.stack[len - 1];
        match self.get_element_keeping_key(obj, key) {
            Ok((key, val)) => {
                // The base slot is already correct; overwrite only the key with
                // its converted form, then push the value.
                self.stack[len - 1] = key;
                self.stack.push(val);
                Ok(())
            }
            Err(e) => {
                // Consume the reference slots, as every other element opcode
                // does, before the caller unwinds.
                self.stack.truncate(len - 2);
                Err(e)
            }
        }
    }

    /// `Op::IncElem` / `Op::DecElem` — `[object key -- number]`, plus a `prefix`
    /// operand byte read from the bytecode.
    ///
    /// `o[k]++` evaluates its reference once (§13.4.2.1 step 1) and reuses it
    /// for GetValue (step 3) and PutValue (step 6), so the key is converted
    /// once — §6.2.5.5 step 3.c.i.  All four update productions this arm serves
    /// share that shape (§13.4.2.1/§13.4.3.1 postfix, §13.4.4.1/§13.4.5.1
    /// prefix).  Passing the raw key to both accessors ran a stateful
    /// `toString` twice, reading one property and writing another — the same
    /// defect `Op::GetElemRef` removes for compound assignment, fixed here by
    /// sharing its [`Self::get_element_keeping_key`].
    ///
    /// ⚠ Unlike `Op::GetElemRef` this arm **pops** its operands, so `obj` spends
    /// the key conversion and the `to_number` of the old value in a Rust local
    /// that `gc/roots.rs` does not walk, and the `set_element` below writes
    /// through it.  That is merge-base behaviour, unchanged by this slice and
    /// shared with roughly twenty other arms; the boundary was drawn there
    /// deliberately, because five successive audits each declared a different
    /// per-site sweep complete and each was wrong.  Owned by
    /// `#11-vm-operand-rooting-by-construction`, whose deliverable is an
    /// invariant making an unrooted hold unrepresentable rather than a sixth
    /// sweep.
    pub(super) fn op_inc_dec_elem(&mut self, increment: bool) -> Result<(), VmError> {
        let prefix = self.read_u8_op() != 0;
        let raw_key = self.pop()?;
        let obj = self.pop()?;
        let (key, old) = self.get_element_keeping_key(obj, raw_key)?;
        let old_num = to_number(self, old)?;
        let new_num = if increment {
            old_num + 1.0
        } else {
            old_num - 1.0
        };
        self.set_element(obj, key, JsValue::Number(new_num))?;
        self.stack
            .push(JsValue::Number(if prefix { new_num } else { old_num }));
        Ok(())
    }

    /// `Op::ThrowUnsupported` — build the `TypeError` for a construct the
    /// compiler cannot lower yet. Always throws; control never falls through.
    ///
    /// Throwing here rather than refusing to compile keeps the failure scoped to
    /// the statement that runs it (umbrella I-1: loud **and** scoped) and
    /// catchable, mirroring how `Op::CheckTdz` raises the TDZ `ReferenceError`.
    pub(super) fn op_throw_unsupported(&mut self, func_id: FuncId) -> VmError {
        let idx = self.read_u16_op();
        match self.load_constant(func_id, idx) {
            Ok(JsValue::String(id)) => {
                VmError::type_error(String::from_utf16_lossy(self.strings.get(id)))
            }
            Ok(other) => VmError::type_error(format!("unsupported construct ({other:?})")),
            Err(e) => e,
        }
    }
}
