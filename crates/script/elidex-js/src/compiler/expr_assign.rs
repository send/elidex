//! Assignment and identifier variable access compilation.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::atom::Atom;
use crate::bytecode::opcode::Op;
use crate::scope::{BindingKind, ScopeAnalysis};

use super::expr::compile_expr;
use super::expr_member::compile_member_property;
use super::expr_ops::compound_op_to_opcode;
use super::function::FunctionCompiler;
use super::resolve::{resolve_identifier, FunctionScope, VarLocation};
use super::CompileError;

/// Compile an identifier load (read).
pub(super) fn compile_identifier_load(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    atom: Atom,
) {
    let loc = resolve_identifier(
        atom,
        fc.func_scope_idx,
        fc.current_scope_idx,
        func_scopes,
        analysis,
    );
    match loc {
        VarLocation::Local(slot) => {
            // Check TDZ if needed — use scope-aware lookup to respect shadowing.
            if let Some(info) = func_scopes[fc.func_scope_idx].get_local_from_scope(
                atom,
                fc.current_scope_idx,
                analysis,
            ) {
                if info.needs_tdz {
                    fc.emit_u16(Op::CheckTdz, slot);
                }
            }
            fc.emit_u16(Op::GetLocal, slot);
        }
        VarLocation::Upvalue(idx) => fc.emit_u16(Op::GetUpvalue, idx),
        VarLocation::Global => {
            let name = prog.interner.get(atom);
            let idx = fc.add_name_u16(name);
            fc.emit_u16(Op::GetGlobal, idx);
        }
        VarLocation::Module(idx) => fc.emit_u16(Op::GetModuleVar, idx),
    }
}

/// Compile an identifier store (write).
fn compile_identifier_store(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    atom: Atom,
) -> Result<(), CompileError> {
    let loc = resolve_identifier(
        atom,
        fc.func_scope_idx,
        fc.current_scope_idx,
        func_scopes,
        analysis,
    );
    match loc {
        VarLocation::Local(slot) => {
            // Use scope-aware lookup to respect shadowing.
            if let Some(info) = func_scopes[fc.func_scope_idx].get_local_from_scope(
                atom,
                fc.current_scope_idx,
                analysis,
            ) {
                // Check for const assignment (ECMA-262 §13.15.2 — TypeError).
                if info.kind == BindingKind::Const {
                    return Err(CompileError {
                        message: format!(
                            "Assignment to constant variable '{}'",
                            prog.interner.get_utf8(atom)
                        ),
                    });
                }
                // Check TDZ for let/const bindings before writing.
                if info.needs_tdz {
                    fc.emit_u16(Op::CheckTdz, slot);
                }
            }
            fc.emit_u16(Op::SetLocal, slot);
        }
        VarLocation::Upvalue(idx) => fc.emit_u16(Op::SetUpvalue, idx),
        VarLocation::Global => {
            let name = prog.interner.get(atom);
            let idx = fc.add_name_u16(name);
            fc.emit_u16(Op::SetGlobal, idx);
        }
        VarLocation::Module(_) => {
            unreachable!("assignment to import binding is not allowed (ECMA-262 §16.2.3.7)");
        }
    }
    Ok(())
}

/// The short-circuit jump for a logical assignment operator (`&&=`, `||=`,
/// `??=`), or `None` for plain/arithmetic compound assignment.
///
/// ECMA-262 §13.15.2 gives the logical forms their own evaluation: the RHS is
/// evaluated **only** when the short-circuit test fails, so they cannot route
/// through [`compound_op_to_opcode`].
fn logical_assign_jump(op: AssignOp) -> Option<Op> {
    match op {
        AssignOp::AndAssign => Some(Op::JumpIfFalse),
        AssignOp::OrAssign => Some(Op::JumpIfTrue),
        AssignOp::NullCoalAssign => Some(Op::JumpIfNotNullish),
        _ => None,
    }
}

/// Where a logical assignment's short-circuit tail stores its result, and how
/// many reference slots sit under the old value on the stack.
#[derive(Clone, Copy)]
enum LogicalStore {
    /// Identifier: `[value -- value]`, no reference slots.
    Ident(Atom),
    /// Computed member: `[object key value -- value]`, 2 reference slots.
    Elem,
    /// Named member: `[object value -- value]`, 1 reference slot.
    Prop { name_idx: u16, ic: u16 },
}

impl LogicalStore {
    /// Reference slots beneath the old value that the short-circuit path must
    /// discard to leave the assignment's value alone on the stack.
    fn ref_slots(self) -> u8 {
        match self {
            Self::Ident(_) => 0,
            Self::Elem => 2,
            Self::Prop { .. } => 1,
        }
    }
}

/// Emit the short-circuit tail of a logical assignment (`&&=`, `||=`, `??=`).
///
/// This is the single lowering for **every** target shape — identifier, named
/// member and computed member.  The pop-versus-peek rule below is subtle enough
/// that a second copy diverged from it once already, so the identifier form
/// routes here rather than encoding the sequence again.
///
/// On entry the stack is `[<ref slots> old]`; on exit exactly one value remains
/// on **both** paths — the value of the `AssignmentExpression` per ECMA-262
/// §13.15.2.  The RHS is compiled only into the assign path, so it is not
/// evaluated when the assignment short-circuits.
fn emit_logical_assign_tail(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    right: NodeId<Expr>,
    jump_op: Op,
    store: LogicalStore,
) -> Result<(), CompileError> {
    // `JumpIfFalse`/`JumpIfTrue` pop their condition; `JumpIfNotNullish` peeks.
    // Dup only for the popping variants so both paths leave `[<refs> old]`; an
    // unconditional `Dup` leaks a slot per evaluation under `??=`.
    if jump_op != Op::JumpIfNotNullish {
        fc.emit(Op::Dup);
    }
    let short_circuit = fc.emit_jump(jump_op);
    // Assign path: discard the old value, evaluate the RHS, store.
    fc.emit(Op::Pop);
    compile_expr(fc, prog, analysis, func_scopes, right)?;
    match store {
        LogicalStore::Ident(atom) => {
            compile_identifier_store(fc, prog, analysis, func_scopes, atom)?;
        }
        LogicalStore::Elem => fc.emit(Op::SetElem),
        LogicalStore::Prop { name_idx, ic } => fc.emit_u16_u16(Op::SetProp, name_idx, ic),
    }
    // Short-circuit path: drop the reference slots under the retained value.
    // `Op::PopUnder`, not `Swap; Pop` — the latter discards through the
    // completion-recording opcode and would overwrite the script's completion
    // value with the base object.  An identifier target has no reference slots,
    // so it needs neither the cleanup nor a jump over it.
    let refs = store.ref_slots();
    let end = (refs > 0).then(|| fc.emit_jump(Op::Jump));
    fc.patch_jump(short_circuit);
    if refs > 0 {
        fc.emit_u8(Op::PopUnder, refs);
    }
    if let Some(end) = end {
        fc.patch_jump(end);
    }
    Ok(())
}

/// Compile an assignment expression.
pub(super) fn compile_assignment(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    left: &AssignTarget,
    op: AssignOp,
    right: NodeId<Expr>,
) -> Result<(), CompileError> {
    match left {
        AssignTarget::Simple(target_id) => {
            let target = prog.exprs.get(*target_id);
            match &target.kind {
                ExprKind::Identifier(atom) => {
                    // Logical assignment short-circuits (ECMA-262 §13.15.2) —
                    // one lowering for every target shape.
                    if let Some(jump_op) = logical_assign_jump(op) {
                        compile_identifier_load(fc, prog, analysis, func_scopes, *atom);
                        return emit_logical_assign_tail(
                            fc,
                            prog,
                            analysis,
                            func_scopes,
                            right,
                            jump_op,
                            LogicalStore::Ident(*atom),
                        );
                    }

                    if op != AssignOp::Assign {
                        // Compound: load current value first.
                        compile_identifier_load(fc, prog, analysis, func_scopes, *atom);
                    }
                    compile_expr(fc, prog, analysis, func_scopes, right)?;
                    if op != AssignOp::Assign {
                        fc.emit(compound_op_to_opcode(op));
                    }
                    compile_identifier_store(fc, prog, analysis, func_scopes, *atom)?;
                }
                ExprKind::Member {
                    object,
                    property,
                    computed,
                } => {
                    compile_member_assignment(
                        fc,
                        prog,
                        analysis,
                        func_scopes,
                        *object,
                        property,
                        *computed,
                        op,
                        right,
                    )?;
                }
                _ => {
                    compile_expr(fc, prog, analysis, func_scopes, right)?;
                }
            }
        }
        AssignTarget::Pattern(_pattern_id) => {
            // Destructuring assignment not yet implemented — pop RHS to keep
            // stack balanced and fail explicitly.
            compile_expr(fc, prog, analysis, func_scopes, right)?;
            fc.emit(Op::Pop);
        }
    }
    Ok(())
}

/// Compile assignment to a member target (`o.p = v`, `o[k] += v`, `o.p ??= v`).
///
/// ECMA-262 §13.15.2 evaluates the LeftHandSideExpression **once** (step 1) and
/// reuses that reference for both `GetValue` (step 3) and `PutValue` (step 9),
/// so compound and logical forms duplicate the reference on the stack rather
/// than re-evaluating the object or key — re-evaluation would run user getters
/// and `valueOf` twice.
#[allow(clippy::too_many_arguments)]
fn compile_member_assignment(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    object: NodeId<Expr>,
    property: &MemberProp,
    computed: bool,
    op: AssignOp,
    right: NodeId<Expr>,
) -> Result<(), CompileError> {
    if computed {
        // `Op::SetElem` consumes `[object key value -- value]`.
        compile_expr(fc, prog, analysis, func_scopes, object)?;
        if let MemberProp::Expression(key_expr) = property {
            compile_expr(fc, prog, analysis, func_scopes, *key_expr)?;
        }
        // `Op::GetElemRef` turns `[obj key]` into `[obj key' old]`, keeping the
        // reference for the store and memoizing the converted key (§6.2.5.5
        // step 3.c.i) so a stateful key's `toString` runs once.  Plain `=` needs
        // neither: it evaluates the reference once and stores through it, and
        // `Op::SetElem`'s own conversion is then the single conversion.
        if let Some(jump_op) = logical_assign_jump(op) {
            fc.emit(Op::GetElemRef);
            return emit_logical_assign_tail(
                fc,
                prog,
                analysis,
                func_scopes,
                right,
                jump_op,
                LogicalStore::Elem,
            );
        }
        if op != AssignOp::Assign {
            fc.emit(Op::GetElemRef);
        }
        compile_expr(fc, prog, analysis, func_scopes, right)?;
        if op != AssignOp::Assign {
            fc.emit(compound_op_to_opcode(op));
        }
        fc.emit(Op::SetElem);
        return Ok(());
    }

    // There is no `Op::SetPrivate` emit path yet (Slice 5 /
    // `#11-vm-class-private-fields`), so **no** store to a private name can be
    // compiled — and the failure mode differs per form, which is why this is one
    // guard rather than a per-form check:
    //
    //   `this.#x ??= 1`  reached `compound_op_to_opcode`'s `unreachable!` and
    //                    aborted the process;
    //   `this.#x += 1`   emitted `GetPrivate`, computed the sum, then discarded
    //                    it at the `Op::Pop` tail below — the write was silently
    //                    lost and the expression evaluated to the *object*;
    //   `this.#x = 1`    same silent loss, without the arithmetic.
    //
    // Both outcomes are banned by the umbrella's no-silent-stub invariant (I-1:
    // connect or reject, never a silently-wrong path), so reject every form
    // loudly until Slice 5 gives `#x` a store opcode.
    if matches!(property, MemberProp::PrivateIdentifier(_)) {
        return Err(CompileError {
            message: "assignment to a private name is not yet supported \
                      (#11-vm-class-private-fields)"
                .into(),
        });
    }

    // Named-member logical assignment: `o.p ||= v`. `[obj]` → `[obj old]`.
    if let (Some(jump_op), MemberProp::Identifier(name)) = (logical_assign_jump(op), property) {
        compile_expr(fc, prog, analysis, func_scopes, object)?;
        fc.emit(Op::Dup);
        let name_u16 = prog.interner.get(*name);
        let get_idx = fc.add_name_u16(name_u16);
        let get_ic = fc.alloc_ic_slot();
        fc.emit_u16_u16(Op::GetProp, get_idx, get_ic);
        let store = LogicalStore::Prop {
            name_idx: fc.add_name_u16(name_u16),
            ic: fc.alloc_ic_slot(),
        };
        return emit_logical_assign_tail(fc, prog, analysis, func_scopes, right, jump_op, store);
    }

    // Named property assignment. `Op::SetProp` consumes `[object value -- value]`.
    compile_expr(fc, prog, analysis, func_scopes, object)?;
    if op != AssignOp::Assign {
        fc.emit(Op::Dup);
        compile_member_property(fc, prog, analysis, func_scopes, property, false)?;
    }
    compile_expr(fc, prog, analysis, func_scopes, right)?;
    if op != AssignOp::Assign {
        fc.emit(compound_op_to_opcode(op));
    }
    match property {
        MemberProp::Identifier(name) => {
            let name_u16 = prog.interner.get(*name);
            let idx = fc.add_name_u16(name_u16);
            let ic = fc.alloc_ic_slot();
            fc.emit_u16_u16(Op::SetProp, idx, ic);
        }
        // `PrivateIdentifier` is rejected above and `Expression` only reaches
        // this function with `computed`, handled in the branch at the top.  A
        // future `MemberProp` variant must not silently reach the old
        // `Op::Pop` tail, which discarded the write and left the *object* as
        // the expression's value, so refuse rather than emit.
        _ => {
            return Err(CompileError {
                message: "unsupported assignment target".into(),
            })
        }
    }
    Ok(())
}
