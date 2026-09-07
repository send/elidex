//! Assignment and identifier variable access compilation.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::atom::Atom;
use crate::bytecode::compiled::Constant;
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
        // Slot: `#11-vm-assignment-target-completeness`.
        VarLocation::Module(_) => {
            emit_unsupported(fc, "assignment to an imported binding is not supported");
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
    // Short-circuit path: drop the reference slots under the retained value in
    // one instruction rather than an `n`-fold `Swap; Pop` dance.  An identifier
    // target has no reference slots, so it needs neither the cleanup nor a jump
    // over it.
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
                // Parenthesized and call targets (`(x) += 1`, `f() = v`).  The
                // arm used to compile the RHS and emit no store at all, so the
                // assignment silently did nothing and evaluated to the RHS —
                // the same silent-wrong shape the private-name guard below
                // rejects.  Loud and scoped until
                // `#11-vm-assignment-target-completeness` normalises
                // `ExprKind::Paren` away.
                _ => {
                    emit_unsupported(fc, "assignment to this target is not yet supported");
                }
            }
        }
        AssignTarget::Pattern(_pattern_id) => {
            // Destructuring assignment (`[a,b] = [b,a]`, `({x} = o)`) is not
            // lowered yet.  This used to compile the RHS and `Op::Pop` it,
            // which the comment called "fail explicitly" while in fact failing
            // silently — and left ZERO values where every other expression
            // leaves one, so a statement-position destructure underflowed the
            // following discard.
            // Owned by `#11-vm-assignment-target-completeness`.
            emit_unsupported(fc, "destructuring assignment is not yet supported");
        }
    }
    Ok(())
}

/// The single admissibility gate for a member-assignment target: the message
/// for [`Op::ThrowUnsupported`] when the target has no store path yet, or `None`
/// when it can be lowered.
///
/// Deliberately **one** decision site, evaluated *before* the
/// computed/non-computed split. Deciding admissibility inside a lowering branch
/// has been wrong twice in this slice, and once more one file away:
///
///   1. the private-name check was gated on `logical_assign_jump(op).is_some()`,
///      so `this.#x ??= 1` was rejected while `this.#x = 1` and `+= 1` emitted a
///      store that fell to an `Op::Pop` tail — the write silently lost and the
///      expression evaluating to the *object*;
///   2. the `super` guard sat below the computed branch, so `super.x = v` threw
///      the scoped error while `super[k] = v` fell through to `PushUndefined`
///      and a misleading "cannot read property of undefined" — *after* running
///      both operands;
///   3. `compile_update_expr` decided separately again, so `this.#x++` stayed a
///      silent no-op after (1) was fixed. It now calls this gate.
///
/// Whether a target can be lowered does not depend on which lowering it would
/// take, so it must not be decided inside one — which is also why update
/// expressions share this gate rather than carrying their own.
///
/// ⚠ **`None` means "the assignment/update lowerings accept this shape", not
/// "every lowering does".** `compile_forin_left_binding` calls this as a
/// *diagnosis* lookup rather than an admissibility decision: no for-in/of member
/// store exists at all, so it rejects every member head and uses whatever
/// message this returns only so that `for (this.#x of a)` reports the same cause
/// as `this.#x = v`. When `#11-vm-assignment-target-completeness` gives member
/// heads a store, that call site is the one that has to change with it — there
/// is no compile-time link.
///
/// Rejection is a scoped runtime throw, not a `CompileError`: umbrella I-1 wants
/// loud **and scoped**, and the umbrella's decision 5 reserves `CompileError`
/// for what the compiler already rejects. A `CompileError` emits no bytecode for the whole
/// script, so one `this.#x = 1` would take every unrelated statement down with
/// it. The throw precedes operand evaluation — the construct is unsupported in
/// full, so running half its side effects first would be a second, subtler
/// divergence.
pub(super) fn unsupported_member_target(
    prog: &Program,
    object: NodeId<Expr>,
    property: &MemberProp,
    computed: bool,
) -> Option<&'static str> {
    // No `Op::SetPrivate` emit path yet, and every form failed differently:
    // `this.#x ??= 1` aborted the process, while `this.#x = 1` / `+= 1` emitted a
    // store that fell to an `Op::Pop` tail — the write silently lost and the
    // expression evaluating to the *object*.
    // Slot: `#11-vm-class-private-fields`.  Kept out of the message itself —
    // it reaches page script as `e.message`, and a ledger rename would drift a
    // web-observable string with no compiler signal.
    if matches!(property, MemberProp::PrivateIdentifier(_)) {
        return Some("assignment to a private name is not yet supported");
    }
    // Both lowerings emit the base with `compile_expr`, which turns
    // `ExprKind::Super` into `Op::PushUndefined`; there is no `SetSuperProp` or
    // `SetSuperElem` emit path, so the store would go to `undefined`.
    // Slot: `#11-step9-class-extras`.
    if matches!(prog.exprs.get(object).kind, ExprKind::Super) {
        return Some("assignment to a super property is not yet supported");
    }
    // Shape/flag mismatches: each lowering below is total on exactly one
    // `MemberProp` variant, so anything else would silently mis-lower.
    match (computed, property) {
        (true, MemberProp::Expression(_)) | (false, MemberProp::Identifier(_)) => None,
        // Defensive, not a deferral: the parser produces no other
        // (computed, MemberProp) pairing, so this arm exists so a future variant
        // cannot silently mis-lower instead of announcing itself.
        _ => Some("this member target shape is not supported"),
    }
}

/// Emit a scoped runtime rejection for a construct the compiler cannot lower.
///
/// One helper so every rejection site produces the same shape — a `TypeError`
/// raised where the construct executes rather than failing the whole compile.
///
/// The message is deliberately **slot-free**: it reaches page script as
/// `e.message`, and a ledger rename would drift a web-observable string with no
/// compiler signal. The owning `#11-*` slot is named in a comment near the
/// rejection instead — either at the call site or on the gate arm that supplied
/// the message.
pub(super) fn emit_unsupported(fc: &mut FunctionCompiler, message: &str) {
    let idx = fc.add_constant(Constant::Wtf16(message.encode_utf16().collect()));
    fc.emit_u16(Op::ThrowUnsupported, idx);
}

/// Compile assignment to a member target (`o.p = v`, `o[k] += v`, `o.p ??= v`).
///
/// The two read-modify-write ECMA-262 §13.15.2 productions evaluate the
/// LeftHandSideExpression **once** and reuse that reference for both `GetValue`
/// and `PutValue`, so compound and logical forms keep the reference on the stack
/// rather than re-evaluating the object or key — re-evaluation would run user
/// getters and `valueOf` twice. Step indices differ per production: `LHS
/// AssignmentOperator AssignmentExpression` is steps 1 / 3 / 9, the logical
/// forms steps 1 / 2 / 6.  Simple `=` evaluates `leftRef` at step 1.a and
/// `PutValue`s at step 1.e without a `GetValue`, so it needs no reference kept
/// across a load — which is why only the other two reach `Op::GetElemRef`.
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
    // ONE admissibility decision, before the computed/non-computed split.
    if let Some(message) = unsupported_member_target(prog, object, property, computed) {
        emit_unsupported(fc, message);
        return Ok(());
    }

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

    // Only `Identifier` reaches here — every other shape was rejected by the
    // admissibility gate above.
    let MemberProp::Identifier(prop_name) = property else {
        return Err(CompileError {
            message: "member assignment target passed the admissibility gate but has no lowering"
                .into(),
        });
    };

    // Named-member logical assignment: `o.p ||= v`. `[obj]` → `[obj old]`.
    if let (Some(jump_op), name) = (logical_assign_jump(op), prop_name) {
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
    let name_u16 = prog.interner.get(*prop_name);
    let idx = fc.add_name_u16(name_u16);
    let ic = fc.alloc_ic_slot();
    fc.emit_u16_u16(Op::SetProp, idx, ic);
    Ok(())
}
