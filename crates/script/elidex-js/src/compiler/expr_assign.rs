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
                    // Handle logical assignment operators with short-circuit (ECMA-262 §13.15.2).
                    if matches!(
                        op,
                        AssignOp::AndAssign | AssignOp::OrAssign | AssignOp::NullCoalAssign
                    ) {
                        compile_identifier_load(fc, prog, analysis, func_scopes, *atom);
                        let jump_op = match op {
                            AssignOp::AndAssign => Op::JumpIfFalse,
                            AssignOp::OrAssign => Op::JumpIfTrue,
                            AssignOp::NullCoalAssign => Op::JumpIfNotNullish,
                            _ => unreachable!(),
                        };
                        // Dup + conditional jump: if short-circuit, keep current value.
                        fc.emit(Op::Dup);
                        let patch = fc.emit_jump(jump_op);
                        fc.emit(Op::Pop); // discard old value
                        compile_expr(fc, prog, analysis, func_scopes, right)?;
                        compile_identifier_store(fc, prog, analysis, func_scopes, *atom)?;
                        fc.patch_jump(patch);
                        return Ok(());
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
                    if *computed {
                        // Computed member assignment: obj[key] = value
                        // SetElem expects [object key value -- value]
                        compile_expr(fc, prog, analysis, func_scopes, *object)?;
                        if let MemberProp::Expression(key_expr) = property {
                            compile_expr(fc, prog, analysis, func_scopes, *key_expr)?;
                        }
                        // Compound computed assignment (obj[key] += val) requires
                        // preserving object+key while loading the old value. Not yet
                        // supported — reject to avoid miscompilation.
                        assert!(
                            op == AssignOp::Assign,
                            "compound assignments to computed members are not yet supported"
                        );
                        compile_expr(fc, prog, analysis, func_scopes, right)?;
                        fc.emit(Op::SetElem);
                    } else {
                        // Named property assignment: obj.prop = value
                        // SetProp expects [object value -- value]
                        compile_expr(fc, prog, analysis, func_scopes, *object)?;
                        if op != AssignOp::Assign {
                            fc.emit(Op::Dup);
                            compile_member_property(
                                fc,
                                prog,
                                analysis,
                                func_scopes,
                                property,
                                false,
                            )?;
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
                            _ => {
                                // PrivateIdentifier or computed (shouldn't reach here
                                // for computed — handled above). Pop to keep stack balanced.
                                fc.emit(Op::Pop);
                            }
                        }
                    }
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
