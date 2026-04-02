//! Per-function compilation state and bytecode emit helpers.

use std::collections::HashMap;

use crate::atom::Atom;
use crate::bytecode::compiled::{CompiledFunction, Constant, ExceptionHandler, UpvalueDesc};
use crate::bytecode::opcode::Op;
use crate::bytecode::source_map::SourceMap;
use crate::span::Span;

use super::resolve::FunctionScope;

/// Per-function compilation state.
#[allow(clippy::struct_excessive_bools)]
pub struct FunctionCompiler {
    /// Bytecode being built.
    pub bytecode: Vec<u8>,
    /// Constant pool being built.
    pub constants: Vec<Constant>,
    /// Source map entries.
    pub source_map: SourceMap,
    /// Exception handler entries.
    pub exception_handlers: Vec<ExceptionHandler>,
    /// Break/continue label targets for loops.
    pub loop_stack: Vec<LoopContext>,
    /// Label → loop stack index mapping for labeled statements.
    pub label_map: HashMap<Atom, usize>,
    /// Function metadata.
    pub name: Option<String>,
    pub is_async: bool,
    pub is_generator: bool,
    pub is_arrow: bool,
    pub is_strict: bool,
    /// Reference to the function scope (local slot assignments).
    pub func_scope_idx: usize,
}

/// Loop context for break/continue jump patching.
pub struct LoopContext {
    /// Bytecode offset to jump to for `continue`.
    pub continue_target: u32,
    /// Placeholder offsets that need patching for `break`.
    pub break_patches: Vec<u32>,
    /// Placeholder offsets that need patching for `continue` (used when
    /// the continue target is not yet known, e.g. do-while test, for-loop update).
    pub continue_patches: Vec<u32>,
}

impl FunctionCompiler {
    /// Create a new function compiler.
    pub fn new(func_scope_idx: usize, is_strict: bool) -> Self {
        Self {
            bytecode: Vec::new(),
            constants: Vec::new(),
            source_map: SourceMap::new(),
            exception_handlers: Vec::new(),
            loop_stack: Vec::new(),
            label_map: HashMap::new(),
            name: None,
            is_async: false,
            is_generator: false,
            is_arrow: false,
            is_strict,
            func_scope_idx,
        }
    }

    /// Current bytecode offset (program counter).
    #[must_use]
    pub fn pc(&self) -> u32 {
        self.bytecode.len() as u32
    }

    // ── Emit helpers ────────────────────────────────────────────────

    /// Emit a zero-operand opcode.
    pub fn emit(&mut self, op: Op) {
        self.bytecode.push(op.to_byte());
    }

    /// Emit an opcode with source span tracking.
    pub fn emit_span(&mut self, op: Op, span: Span) {
        self.source_map.add(self.pc(), span);
        self.emit(op);
    }

    /// Emit an opcode with a u8 operand.
    pub fn emit_u8(&mut self, op: Op, val: u8) {
        self.bytecode.push(op.to_byte());
        self.bytecode.push(val);
    }

    /// Emit an opcode with a u16 operand (little-endian).
    pub fn emit_u16(&mut self, op: Op, val: u16) {
        self.bytecode.push(op.to_byte());
        self.bytecode.extend_from_slice(&val.to_le_bytes());
    }

    /// Emit an opcode with a u16 + u8 operand.
    pub fn emit_u16_u8(&mut self, op: Op, val: u16, flag: u8) {
        self.bytecode.push(op.to_byte());
        self.bytecode.extend_from_slice(&val.to_le_bytes());
        self.bytecode.push(flag);
    }

    /// Emit an opcode with two u16 operands.
    pub fn emit_u16_u16(&mut self, op: Op, a: u16, b: u16) {
        self.bytecode.push(op.to_byte());
        self.bytecode.extend_from_slice(&a.to_le_bytes());
        self.bytecode.extend_from_slice(&b.to_le_bytes());
    }

    // ── Jump helpers ────────────────────────────────────────────────

    /// Emit a jump opcode with a placeholder offset. Returns the
    /// bytecode position of the offset bytes for later patching.
    pub fn emit_jump(&mut self, op: Op) -> u32 {
        self.bytecode.push(op.to_byte());
        let patch_pos = self.pc();
        self.bytecode.extend_from_slice(&0i16.to_le_bytes());
        patch_pos
    }

    /// Patch a previously emitted jump offset to point to the current PC.
    #[allow(clippy::cast_possible_wrap)]
    pub fn patch_jump(&mut self, patch_pos: u32) {
        let target = self.pc();
        // Offset is relative to the byte AFTER the jump instruction
        // (opcode byte + 2 offset bytes = 3 bytes total).
        let offset = (target as i32) - (patch_pos as i32) - 2;
        assert!(
            (i32::from(i16::MIN)..=i32::from(i16::MAX)).contains(&offset),
            "jump offset {offset} out of i16 range"
        );
        let bytes = (offset as i16).to_le_bytes();
        self.bytecode[patch_pos as usize] = bytes[0];
        self.bytecode[(patch_pos + 1) as usize] = bytes[1];
    }

    /// Emit a backward jump to `target`.
    #[allow(clippy::cast_possible_wrap)]
    pub fn emit_jump_to(&mut self, op: Op, target: u32) {
        self.bytecode.push(op.to_byte());
        let offset = (target as i32) - (self.pc() as i32) - 2;
        assert!(
            (i32::from(i16::MIN)..=i32::from(i16::MAX)).contains(&offset),
            "jump offset {offset} out of i16 range"
        );
        self.bytecode
            .extend_from_slice(&(offset as i16).to_le_bytes());
    }

    // ── Constant pool ───────────────────────────────────────────────

    /// Add a constant to the pool, returning its index.
    /// Deduplicates numbers and strings.
    #[allow(clippy::cast_possible_truncation)]
    pub fn add_constant(&mut self, constant: Constant) -> u16 {
        // Dedup for numbers and strings.
        match &constant {
            Constant::Number(n) => {
                for (i, c) in self.constants.iter().enumerate() {
                    if let Constant::Number(existing) = c {
                        if n.to_bits() == existing.to_bits() {
                            return i as u16;
                        }
                    }
                }
            }
            Constant::String(s) => {
                for (i, c) in self.constants.iter().enumerate() {
                    if let Constant::String(existing) = c {
                        if s == existing {
                            return i as u16;
                        }
                    }
                }
            }
            _ => {}
        }
        let idx = self.constants.len();
        self.constants.push(constant);
        idx as u16
    }

    /// Add a name (identifier) constant and return its index.
    pub fn add_name(&mut self, name: &str) -> u16 {
        self.add_constant(Constant::String(name.to_string()))
    }

    // ── Loop management ─────────────────────────────────────────────

    /// Push a new loop context. `continue_target` is the PC for `continue`.
    pub fn push_loop(&mut self, continue_target: u32) {
        self.loop_stack.push(LoopContext {
            continue_target,
            break_patches: Vec::new(),
            continue_patches: Vec::new(),
        });
    }

    /// Record a break jump that needs patching when the loop ends.
    pub fn add_break_patch(&mut self, patch_pos: u32) {
        if let Some(ctx) = self.loop_stack.last_mut() {
            ctx.break_patches.push(patch_pos);
        }
    }

    /// Record a continue jump that needs patching (target not yet known).
    pub fn add_continue_patch(&mut self, patch_pos: u32) {
        if let Some(ctx) = self.loop_stack.last_mut() {
            ctx.continue_patches.push(patch_pos);
        }
    }

    /// Patch all pending continue jumps to the current PC.
    pub fn patch_continue_jumps(&mut self) {
        if let Some(ctx) = self.loop_stack.last_mut() {
            let patches: Vec<u32> = ctx.continue_patches.drain(..).collect();
            for patch in patches {
                self.patch_jump(patch);
            }
        }
    }

    /// Patch all pending continue jumps to a specific target PC.
    #[allow(clippy::cast_possible_wrap)]
    pub fn patch_continue_jumps_to(&mut self, target: u32) {
        if let Some(ctx) = self.loop_stack.last_mut() {
            let patches: Vec<u32> = ctx.continue_patches.drain(..).collect();
            for patch_pos in patches {
                let offset = (target as i32) - (patch_pos as i32) - 2;
                debug_assert!(
                    (i32::from(i16::MIN)..=i32::from(i16::MAX)).contains(&offset),
                    "jump offset {offset} out of i16 range"
                );
                let bytes = (offset as i16).to_le_bytes();
                self.bytecode[patch_pos as usize] = bytes[0];
                self.bytecode[(patch_pos + 1) as usize] = bytes[1];
            }
        }
    }

    /// Pop the loop context and patch all break jumps to current PC.
    pub fn pop_loop(&mut self) {
        if let Some(ctx) = self.loop_stack.pop() {
            for patch in ctx.break_patches {
                self.patch_jump(patch);
            }
        }
    }

    // ── Finalization ────────────────────────────────────────────────

    /// Finalize into a `CompiledFunction`.
    pub fn finish(self, func_scope: &FunctionScope) -> CompiledFunction {
        CompiledFunction {
            bytecode: self.bytecode,
            constants: self.constants,
            local_count: func_scope.next_local,
            param_count: 0, // set by caller
            upvalues: func_scope
                .upvalues
                .iter()
                .map(|uv| UpvalueDesc {
                    is_local: uv.is_local,
                    index: uv.index,
                })
                .collect(),
            source_map: self.source_map,
            name: self.name,
            exception_handlers: self.exception_handlers,
            is_async: self.is_async,
            is_generator: self.is_generator,
            is_arrow: self.is_arrow,
            is_strict: self.is_strict,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_basic() {
        let mut fc = FunctionCompiler::new(0, false);
        fc.emit(Op::PushUndefined);
        fc.emit(Op::Return);
        assert_eq!(fc.bytecode, vec![Op::PushUndefined as u8, Op::Return as u8]);
    }

    #[test]
    fn emit_u16() {
        let mut fc = FunctionCompiler::new(0, false);
        fc.emit_u16(Op::PushConst, 0x0102);
        assert_eq!(fc.bytecode, vec![Op::PushConst as u8, 0x02, 0x01]);
    }

    #[test]
    fn jump_patching() {
        let mut fc = FunctionCompiler::new(0, false);
        let patch = fc.emit_jump(Op::JumpIfFalse);
        fc.emit(Op::Pop);
        fc.emit(Op::Pop);
        fc.patch_jump(patch);
        // After patching, the jump offset should skip the two Pop instructions.
        let offset = i16::from_le_bytes([fc.bytecode[1], fc.bytecode[2]]);
        assert_eq!(offset, 2); // skip 2 bytes (2 x Pop)
    }

    #[test]
    fn constant_dedup() {
        let mut fc = FunctionCompiler::new(0, false);
        let a = fc.add_constant(Constant::Number(42.0));
        let b = fc.add_constant(Constant::Number(42.0));
        assert_eq!(a, b);
        assert_eq!(fc.constants.len(), 1);

        let c = fc.add_constant(Constant::String("hello".into()));
        let d = fc.add_constant(Constant::String("hello".into()));
        assert_eq!(c, d);
        assert_eq!(fc.constants.len(), 2);
    }

    #[test]
    fn loop_break_patching() {
        let mut fc = FunctionCompiler::new(0, false);
        fc.push_loop(0);
        let break_patch = fc.emit_jump(Op::Jump);
        fc.add_break_patch(break_patch);
        fc.emit(Op::Pop); // loop body
        fc.pop_loop(); // patches break to after Pop
        let offset = i16::from_le_bytes([fc.bytecode[1], fc.bytecode[2]]);
        assert_eq!(offset, 1); // skip 1 byte (Pop)
    }
}
