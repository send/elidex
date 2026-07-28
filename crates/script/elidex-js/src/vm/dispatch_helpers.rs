//! Bytecode reading helpers, constant loading, and jump support for the
//! dispatch loop. Extracted from `dispatch.rs` to keep that file focused
//! on the main opcode dispatch.

use num_bigint::BigInt;

use crate::bytecode::compiled::Constant;

use super::coerce::to_number;
use super::ops::{parse_array_index_u16, try_as_array_index};
use super::value::{
    FuncId, JsValue, Object, ObjectId, ObjectKind, PropertyKey, PropertyValue, StringId, VmError,
    VmErrorKind,
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
    /// `Op::TemplateConcat` — `[p1 … pn -- string]`, `ToString` each part
    /// (ECMA-262 §13.2.8.6 `SubstitutionTemplate` step 4) and concatenate.
    ///
    /// Reads the parts **in place** and truncates only once every conversion has
    /// finished, for the same reason as the element-access family below: user
    /// `toString` / `@@toPrimitive` can allocate enough to trigger a collection,
    /// and the GC roots the VM stack (`gc/roots.rs`) but not Rust locals.
    /// Copying the parts into a `Vec` and truncating first left every part after
    /// the one being converted reachable **only** from that `Vec` — so a
    /// temporary object part was collected mid-loop and its `ToString` then ran
    /// against a recycled slot (or panicked in `get_object`).
    pub(crate) fn op_template_concat(&mut self, count: usize) -> Result<(), super::value::VmError> {
        let len = self.stack.len();
        if len < count {
            return Err(VmError::internal("stack underflow on TemplateConcat"));
        }
        let start = len - count;
        let mut result: Vec<u16> = Vec::new();
        // Converted eagerly into the accumulator rather than collected into a
        // `Vec<StringId>`: interned string ids are never collected (the GC sweeps
        // objects and upvalues only), so the accumulator needs no rooting.
        let mut outcome = Ok(());
        for i in 0..count {
            // Re-read each part by index: the slots stay on the stack for the
            // whole loop, so a part not yet converted cannot be collected by an
            // earlier part's user code.
            let val = self.stack[start + i];
            match super::coerce::to_string(self, val) {
                Ok(sid) => result.extend_from_slice(self.strings.get(sid)),
                Err(e) => {
                    outcome = Err(e);
                    break;
                }
            }
        }
        // Consume the parts on both paths: `[p1 … pn -- string]` on success,
        // `[p1 … pn -- ]` before the caller's error routing.
        self.stack.truncate(start);
        outcome?;
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

// ── Stack + member-access opcode bodies ──────────────────────────────
//
// Extracted rather than written inline: `dispatch.rs` is over the 1000-line
// threshold, and the umbrella's forward rule for slices that add or edit arms is
// to put the body in a `dispatch_*.rs` sibling so the dispatch loop stays a
// table.

/// ECMA-262 §13.5.1.2 Runtime Semantics: Evaluation, `UnaryExpression : delete
/// UnaryExpression` step 4.f: strict-mode TypeError message when `[[Delete]]`
/// returns `false`.
pub(super) const NON_CONFIGURABLE_DELETE_MSG: &str =
    "Cannot delete property: property is not configurable";

/// ECMA-262 §13.5.1.2 `UnaryExpression : delete UnaryExpression` step 4.c
/// `? ToObject(ref.[[Base]])`.  Null/undefined throw TypeError (via ToObject);
/// other primitives are boxed to their wrapper so their [[Delete]] applies to
/// the (temporary) wrapper.
pub(super) fn resolve_delete_base(vm: &mut VmInner, obj: JsValue) -> Result<ObjectId, VmError> {
    match obj {
        JsValue::Object(id) => Ok(id),
        _ => super::coerce::to_object(vm, obj),
    }
}

impl VmInner {
    /// Route an opcode body's error out of the dispatch loop.
    ///
    /// Two dispositions, chosen by the error's own kind rather than by which arm
    /// noticed it:
    ///
    /// - [`VmErrorKind::InternalError`] reports a **broken VM invariant** —
    ///   `error.rs` documents it as "should not occur in correct programs", and
    ///   every producer in the dispatch path is a stack-shape guard or a
    ///   malformed-bytecode check. Handing it to
    ///   [`VmInner::throw_error`](super::VmInner::throw_error) would materialise
    ///   a plain, catchable JS `Error`, so a user `try`/`catch` could swallow
    ///   the evidence and execution would carry on over a stack the VM has
    ///   already agreed is wrong. It leaves by the hard path instead, straight
    ///   out of `run()`, matching what `Op::Swap` has always done inline for the
    ///   identical condition.
    /// - Everything else is a JS-level error (a `TypeError` from a coercion, a
    ///   `ReferenceError`, a user `throw` value) and goes to `throw_error` so
    ///   handler search and `catch` work normally. `Op::ThrowUnsupported` is the
    ///   pointed case: it reports an unimplemented *language construct*, not a
    ///   broken invariant, so its `TypeError` stays catchable.
    pub(super) fn raise(
        &mut self,
        error: VmError,
        entry_frame_depth: usize,
    ) -> Result<(), VmError> {
        if matches!(error.kind, VmErrorKind::InternalError) {
            return Err(error);
        }
        self.throw_error(error, entry_frame_depth)
    }

    /// Run a two-operand opcode over the top two stack slots **in place**.
    ///
    /// The shared rooting primitive for every binary arm whose body takes
    /// `&mut VmInner` and can therefore reach user JS: `ToPrimitive` /
    /// `ToPropertyKey` / `ToNumber` on an operand, or an accessor on one of
    /// them, can allocate enough to trigger a collection. The GC roots the VM
    /// stack (`gc/roots.rs`) but **not** Rust locals, so popping the pair first
    /// leaves the operand that has not been consumed yet reachable from nowhere
    /// — it can be collected mid-op and the rest of the op then runs against a
    /// recycled slot, or panics in `get_object`'s "object already freed".
    ///
    /// The test for "does this arm need it" is deliberately mechanical — *is the
    /// callee `&mut` and can it reach user JS* — not "does some operand
    /// demonstrably outlive the user-code window". The latter has to be redone
    /// from scratch every time a callee changes, it got the boundary wrong
    /// twice already in this PR, and the two per-arm arguments that survived
    /// those rounds were both false:
    ///
    /// - *"the operand is re-rooted as the receiver of the user code that
    ///   runs"* — not for an **arrow or bound** callee, whose `this` comes from
    ///   its closure or its binding rather than from the caller. This is what
    ///   exposed `SpreadObject`'s source and `ArraySpread`'s iterator.
    /// - *"the value is rooted as the callee's argument"* — not for a
    ///   **zero-parameter** callee: `call_internal` copies only
    ///   `args[..min(argc, param_count)]` into the callee's stack slots. This is
    ///   what exposed `Op::SetProp`'s stored value.
    ///
    /// `Op::StrictEq` / `Op::StrictNotEq` stay on `pop()`, and not by a derived
    /// argument: `strict_eq` takes `&VmInner`, so the compiler — not a reviewer
    /// — proves they run no user code.
    ///
    /// Both slots are consumed on the success path and on every error path, so
    /// the arm's stack effect is `[lhs rhs -- result]` / `[lhs rhs -- ]`
    /// exactly as it was under `pop()`.
    pub(super) fn binary_op_rooted(
        &mut self,
        what: &'static str,
        op: impl FnOnce(&mut Self, JsValue, JsValue) -> Result<JsValue, VmError>,
    ) -> Result<(), VmError> {
        let len = self.stack.len();
        if len < 2 {
            return Err(VmError::internal(format!("stack underflow on {what}")));
        }
        let lhs = self.stack[len - 2];
        let rhs = self.stack[len - 1];
        let result = op(self, lhs, rhs);
        self.stack.truncate(len - 2);
        let val = result?;
        // Nothing between the truncate and the push allocates, so `val` — which
        // the op produced before the operands were dropped — cannot be collected
        // in the gap.
        self.stack.push(val);
        Ok(())
    }

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

    /// `Op::GetProp` — `[object -- value]`, static-key member read, plus a
    /// name-constant index and an IC slot index read from the bytecode.
    ///
    /// Reads the operand **in place** by the same mechanical test as the rest of
    /// the family — `ic_get_prop` is `&mut` and reaches a user getter. This arm
    /// used to be documented as a proven exception ("the base is re-rooted as
    /// the accessor's receiver"), and for the read side that argument does hold
    /// today: `get_prop_slow` collects its IC info *before* the accessor runs
    /// and touches only `compiled_functions` after it. The argument was wrong
    /// for its sibling [`Self::op_set_prop`], though, and it has to be re-derived
    /// from scratch on every edit to either callee — so the arm now satisfies
    /// the rule structurally instead, at the cost of one index read.
    pub(super) fn op_get_prop(&mut self, func_id: FuncId) -> Result<(), VmError> {
        let name_idx = self.read_u16_op();
        let ic_idx = self.read_u16_op() as usize;
        let len = self.stack.len();
        if len < 1 {
            return Err(VmError::internal("stack underflow on GetProp"));
        }
        let obj_val = self.stack[len - 1];
        let result = self.ic_get_prop(func_id, name_idx, ic_idx, obj_val);
        // `[object -- value]` on success; consumed and nothing pushed before the
        // caller's error routing.
        self.stack.truncate(len - 1);
        let val = result?;
        self.stack.push(val);
        Ok(())
    }

    /// `Op::SetProp` — `[object value -- value]`, static-key member store.
    ///
    /// Reads both operands **in place**. The stored value is handed to a user
    /// setter as its argument, which is where it was previously argued to be
    /// rooted — but `call_internal` copies only `args[..min(argc, param_count)]`
    /// into the callee's stack slots, so a **zero-parameter** setter copies
    /// nothing and the value spends the whole body in a Rust local. Pair that
    /// with an arrow setter (its `this` is lexical, so the base is unrooted too)
    /// and both operands are exposed; the arm then pushes the value back as the
    /// assignment's result, publishing a dangling id to the rest of the program.
    pub(super) fn op_set_prop(&mut self, func_id: FuncId) -> Result<(), VmError> {
        let name_idx = self.read_u16_op();
        let ic_idx = self.read_u16_op() as usize;
        let len = self.stack.len();
        if len < 2 {
            return Err(VmError::internal("stack underflow on SetProp"));
        }
        let obj_val = self.stack[len - 2];
        let val = self.stack[len - 1];
        let result = self.ic_set_prop(func_id, name_idx, ic_idx, obj_val, val);
        // `[object value -- value]` on success; both consumed and nothing
        // pushed before the caller's error routing.
        self.stack.truncate(len - 2);
        let stored = result?;
        self.stack.push(stored);
        Ok(())
    }

    /// `Op::GetElemRef` — `[object key -- object key' value]`, load through a
    /// computed member reference while keeping the reference for a later store.
    ///
    /// Reads the `[object key]` pair **in place** rather than popping it. The
    /// key conversion runs user `toString`/`@@toPrimitive`, which can allocate
    /// enough to trigger a collection, and the GC roots the VM stack
    /// (`gc/roots.rs`) but not Rust locals. Popping first would let a temporary
    /// base be collected mid-conversion and then hand a dangling `ObjectId` to
    /// the `SetElem` that follows — a store into a recycled slot, or
    /// `get_object_mut`'s "object already freed" panic. Keeping both slots on
    /// the stack makes this opcode rooted **by construction** — the shape the
    /// rest of the element-access family below now shares.
    ///
    /// `key'` is the **converted** key: §6.2.5.5 GetValue step 3.c.i writes
    /// `ToPropertyKey`'s result back into the Reference Record, so a later
    /// `PutValue` through the same reference does not re-run user code.
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

    /// `Op::GetElem` — `[object key -- value]`, ordinary computed member read.
    ///
    /// Reads the operand pair **in place** for the same reason as
    /// [`Self::op_get_elem_ref`]: `ToPropertyKey` on an object key runs user
    /// `toString`/`@@toPrimitive`, which can allocate enough to collect, and the
    /// GC roots the VM stack but not Rust locals. The slots are dropped only
    /// once the access has finished, so a temporary base cannot be collected out
    /// from under it and yield a value read from a recycled object.
    pub(super) fn op_get_elem(&mut self) -> Result<(), VmError> {
        let len = self.stack.len();
        if len < 2 {
            return Err(VmError::internal("stack underflow on GetElem"));
        }
        let obj = self.stack[len - 2];
        let key = self.stack[len - 1];
        let result = self.get_element(obj, key);
        // Consume the operands on both paths: `[object key -- value]` on
        // success, `[object key -- ]` before the caller's `throw_error`.
        self.stack.truncate(len - 2);
        let val = result?;
        self.stack.push(val);
        Ok(())
    }

    /// `Op::SetElem` — `[object key value -- value]`, computed member store.
    ///
    /// Reads all three operands **in place**, which matters more here than on
    /// the read side: `set_element` hand-rolls `ToPropertyKey`, so a base popped
    /// into a (non-rooted) Rust local could be collected by an allocating user
    /// `toString` and the store would then go through a dangling `ObjectId` —
    /// into whatever object recycled the slot, or `get_object_mut`'s "object
    /// already freed" panic. Leaving the slots in place also roots `value`,
    /// which is otherwise equally exposed to the key conversion.
    pub(super) fn op_set_elem(&mut self) -> Result<(), VmError> {
        let len = self.stack.len();
        if len < 3 {
            return Err(VmError::internal("stack underflow on SetElem"));
        }
        let obj = self.stack[len - 3];
        let key = self.stack[len - 2];
        let val = self.stack[len - 1];
        let result = self.set_element(obj, key, val);
        // `[object key value -- value]` on success; all three consumed and
        // nothing pushed before the caller's `throw_error`.
        self.stack.truncate(len - 3);
        result?;
        self.stack.push(val);
        Ok(())
    }

    /// `Op::DeleteElem` — `[object key -- boolean]`, `delete object[key]`.
    ///
    /// Reads the operand pair **in place** because this arm *mutates*:
    /// `make_property_key` runs user `toString`/`@@toPrimitive` and
    /// `try_delete_property` then deletes through the base, so a base held only
    /// in a Rust local could be collected mid-conversion and the delete would
    /// hit a recycled object (or panic in `get_object_mut`). The boxed wrapper a
    /// primitive base coerces to is written back into the base slot for the same
    /// reason — after `resolve_delete_base` *it* is the object being mutated,
    /// and nothing else references it.
    pub(super) fn op_delete_elem(&mut self) -> Result<(), VmError> {
        let len = self.stack.len();
        if len < 2 {
            return Err(VmError::internal("stack underflow on DeleteElem"));
        }
        let obj_val = self.stack[len - 2];
        let key = self.stack[len - 1];
        let id = match resolve_delete_base(self, obj_val) {
            Ok(id) => id,
            Err(e) => {
                self.stack.truncate(len - 2);
                return Err(e);
            }
        };
        // Root the resolved base in the slot it came from — for a primitive base
        // this is a wrapper nothing else holds.  Both slots are dropped below,
        // so the arm's stack effect is unchanged.
        self.stack[len - 2] = JsValue::Object(id);
        // Resolve array index from Number or String key.
        let arr_idx = match key {
            JsValue::Number(n) => try_as_array_index(n),
            JsValue::String(sid) => parse_array_index_u16(self.strings.get(sid)),
            _ => None,
        };
        // Fast path: array element present → set to Empty.
        let fast = arr_idx.and_then(|idx| match &self.get_object(id).kind {
            ObjectKind::Array { elements } if idx < elements.len() && !elements[idx].is_empty() => {
                Some(idx)
            }
            _ => None,
        });
        let deleted = if let Some(idx) = fast {
            if let ObjectKind::Array { elements } = &mut self.get_object_mut(id).kind {
                elements[idx] = JsValue::Empty;
            }
            Ok(true)
        } else {
            self.make_property_key(key)
                .and_then(|pk| self.try_delete_property(id, pk))
        };
        self.stack.truncate(len - 2);
        if deleted? {
            self.stack.push(JsValue::Boolean(true));
            Ok(())
        } else {
            // §13.5.1.2 step 4.f: `delete` throws TypeError in strict mode when
            // [[Delete]] returns false.  All code is strict, so we always throw.
            Err(VmError::type_error(NON_CONFIGURABLE_DELETE_MSG))
        }
    }

    /// `Op::IncElem` / `Op::DecElem` — `[object key -- number]`, plus a `prefix`
    /// operand byte read from the bytecode.
    ///
    /// Reads the operand pair **in place** because this arm runs user code
    /// twice before it stores: `ToPropertyKey` on the key and `ToNumber` on the
    /// old value can each allocate enough to collect, and the following
    /// `set_element` writes through the *same* base. A base popped into a Rust
    /// local — which the GC does not root — would make that a store through a
    /// dangling `ObjectId`; keeping both slots on the stack makes the whole
    /// read-modify-write rooted by construction.
    pub(super) fn op_inc_dec_elem(&mut self, increment: bool) -> Result<(), VmError> {
        let prefix = self.read_u8_op() != 0;
        let len = self.stack.len();
        if len < 2 {
            return Err(VmError::internal("stack underflow on IncElem/DecElem"));
        }
        let obj_val = self.stack[len - 2];
        let raw_key = self.stack[len - 1];
        let result = self.inc_dec_elem_value(obj_val, raw_key, increment, prefix);
        // `[object key -- number]` on success; both consumed and nothing pushed
        // before the caller's `throw_error`.
        self.stack.truncate(len - 2);
        let val = result?;
        self.stack.push(val);
        Ok(())
    }

    /// Read-modify-write body of [`Self::op_inc_dec_elem`], split out so the
    /// caller drops the operand slots on exactly one path — they must stay
    /// rooted for the whole of this function.
    ///
    /// `o[k]++` evaluates its reference once (§13.4.2.1 step 1) and reuses it
    /// for GetValue (step 3) and PutValue (step 6), so the key is converted
    /// once — §6.2.5.5 step 3.c.i.  All four update productions this arm serves
    /// share that shape (§13.4.2.1/§13.4.3.1 postfix, §13.4.4.1/§13.4.5.1
    /// prefix).  Passing the raw key to both accessors ran a stateful
    /// `toString` twice, reading one property and writing another.
    fn inc_dec_elem_value(
        &mut self,
        obj: JsValue,
        raw_key: JsValue,
        increment: bool,
        prefix: bool,
    ) -> Result<JsValue, VmError> {
        let (key, old) = self.get_element_keeping_key(obj, raw_key)?;
        let old_num = to_number(self, old)?;
        let new_num = if increment {
            old_num + 1.0
        } else {
            old_num - 1.0
        };
        self.set_element(obj, key, JsValue::Number(new_num))?;
        Ok(JsValue::Number(if prefix { new_num } else { old_num }))
    }

    /// `Op::IncProp` / `Op::DecProp` — `[object -- number]`, plus a name-constant
    /// index and a `prefix` operand byte read from the bytecode.
    ///
    /// The static-key sibling of [`Self::op_inc_dec_elem`], and rooted the same
    /// way for a strictly worse reason: this arm crosses **two** user-code
    /// windows before it stores. `get_property_val` may run a user **getter**
    /// and `to_number` a user `valueOf`, and the `set_property_val` that follows
    /// writes through the *same* base. Held in a Rust local — which the GC does
    /// not root — that base could be collected by either window, making the
    /// store land in whatever object recycled the slot or panic in
    /// `get_object_mut`. Reading the operand in place makes the whole
    /// read-modify-write rooted by construction.
    ///
    /// One reference, evaluated once: §13.4.2.1 (postfix `++`) step 1 evaluates
    /// it and reuses it for `GetValue` (§6.2.5.5) at step 3 and `PutValue`
    /// (§6.2.5.6) at step 6; §13.4.3.1 / §13.4.4.1 / §13.4.5.1 are the same
    /// shape for the other three update productions this arm serves.
    pub(super) fn op_inc_dec_prop(
        &mut self,
        func_id: FuncId,
        increment: bool,
    ) -> Result<(), VmError> {
        let name_idx = self.read_u16_op();
        let prefix = self.read_u8_op() != 0;
        // Ahead of the operand read, as it was under `pop()`: a bad constant
        // index is a malformed-bytecode invariant break, and leaving the operand
        // in place for it keeps that path's stack shape unchanged.
        let name_id = self.constant_to_string_id(func_id, name_idx)?;
        let len = self.stack.len();
        if len < 1 {
            return Err(VmError::internal("stack underflow on IncProp/DecProp"));
        }
        let obj_val = self.stack[len - 1];
        let result = self.inc_dec_prop_value(obj_val, name_id, increment, prefix);
        // `[object -- number]` on success; consumed and nothing pushed before
        // the caller's error routing.
        self.stack.truncate(len - 1);
        let val = result?;
        self.stack.push(val);
        Ok(())
    }

    /// Read-modify-write body of [`Self::op_inc_dec_prop`], split out so the
    /// caller drops the operand slot on exactly one path — it must stay rooted
    /// for the whole of this function.
    ///
    /// The primitive-base store is skipped rather than thrown, preserving the
    /// behaviour this arm had before the operand was rooted. (§6.2.5.6 PutValue
    /// step 5.e would `throw` for a strict primitive base; that divergence is
    /// pre-existing and orthogonal to rooting.)
    fn inc_dec_prop_value(
        &mut self,
        obj: JsValue,
        name_id: StringId,
        increment: bool,
        prefix: bool,
    ) -> Result<JsValue, VmError> {
        let old = self.get_property_val(obj, name_id)?;
        let old_num = to_number(self, old)?;
        let new_num = if increment {
            old_num + 1.0
        } else {
            old_num - 1.0
        };
        if matches!(obj, JsValue::Object(_)) {
            self.set_property_val(obj, name_id, JsValue::Number(new_num))?;
        }
        Ok(JsValue::Number(if prefix { new_num } else { old_num }))
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
