//! Operator-to-opcode mappings and unary/update expression compilation.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::bytecode::opcode::Op;
use crate::scope::ScopeAnalysis;

use super::expr::compile_expr;
use super::function::FunctionCompiler;
use super::resolve::{resolve_identifier, FunctionScope, VarLocation};
use super::CompileError;

/// Map compound AssignOp to the corresponding binary opcode.
pub(super) fn compound_op_to_opcode(op: AssignOp) -> Op {
    match op {
        AssignOp::AddAssign => Op::Add,
        AssignOp::SubAssign => Op::Sub,
        AssignOp::MulAssign => Op::Mul,
        AssignOp::DivAssign => Op::Div,
        AssignOp::ModAssign => Op::Mod,
        AssignOp::ExpAssign => Op::Exp,
        AssignOp::ShlAssign => Op::Shl,
        AssignOp::ShrAssign => Op::Shr,
        AssignOp::UShrAssign => Op::UShr,
        AssignOp::BitAndAssign => Op::BitAnd,
        AssignOp::BitOrAssign => Op::BitOr,
        AssignOp::AndAssign | AssignOp::OrAssign | AssignOp::NullCoalAssign => {
            unreachable!(
                "logical assignment operators use short-circuit, not compound_op_to_opcode"
            )
        }
        AssignOp::BitXorAssign => Op::BitXor,
        AssignOp::Assign => unreachable!("plain assign should not call compound_op_to_opcode"),
    }
}

/// Map BinaryOp to opcode.
pub(super) fn binary_op_to_opcode(op: BinaryOp) -> Op {
    match op {
        BinaryOp::Add => Op::Add,
        BinaryOp::Sub => Op::Sub,
        BinaryOp::Mul => Op::Mul,
        BinaryOp::Div => Op::Div,
        BinaryOp::Mod => Op::Mod,
        BinaryOp::Exp => Op::Exp,
        BinaryOp::BitAnd => Op::BitAnd,
        BinaryOp::BitOr => Op::BitOr,
        BinaryOp::BitXor => Op::BitXor,
        BinaryOp::Shl => Op::Shl,
        BinaryOp::Shr => Op::Shr,
        BinaryOp::UShr => Op::UShr,
        BinaryOp::Eq => Op::Eq,
        BinaryOp::NotEq => Op::NotEq,
        BinaryOp::StrictEq => Op::StrictEq,
        BinaryOp::StrictNotEq => Op::StrictNotEq,
        BinaryOp::Lt => Op::Lt,
        BinaryOp::LtEq => Op::LtEq,
        BinaryOp::Gt => Op::Gt,
        BinaryOp::GtEq => Op::GtEq,
        BinaryOp::In => Op::In,
        BinaryOp::Instanceof => Op::Instanceof,
    }
}

/// Map UnaryOp to opcode. `Delete` is intercepted upstream in `compile_unary_expr`.
fn unary_op_to_opcode(op: UnaryOp) -> Op {
    match op {
        UnaryOp::Minus => Op::Neg,
        UnaryOp::Plus => Op::Pos,
        UnaryOp::Not => Op::Not,
        UnaryOp::BitwiseNot => Op::BitNot,
        UnaryOp::Typeof => Op::TypeOf,
        UnaryOp::Void => Op::Void,
        UnaryOp::Delete => unreachable!("Delete handled by compile_unary_expr early-return"),
    }
}

/// Emit a prefix/postfix update sequence (++/--) for non-local targets.
///
/// `emit_load` pushes the current value; `emit_store` pops and stores the new value.
fn emit_update_sequence(
    fc: &mut FunctionCompiler,
    op: UpdateOp,
    prefix: bool,
    emit_load: impl FnOnce(&mut FunctionCompiler),
    emit_store: impl FnOnce(&mut FunctionCompiler),
) {
    emit_load(fc);
    if !prefix {
        fc.emit(Op::Dup); // postfix: keep old value under new
    }
    fc.emit_u8(Op::PushI8, 1);
    fc.emit(match op {
        UpdateOp::Increment => Op::Add,
        UpdateOp::Decrement => Op::Sub,
    });
    if prefix {
        fc.emit(Op::Dup); // prefix: keep new value
    }
    emit_store(fc);
}

/// Compile a unary expression (typeof, delete, and standard unary ops).
pub(super) fn compile_unary_expr(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    op: UnaryOp,
    argument: NodeId<Expr>,
) -> Result<(), CompileError> {
    if matches!(op, UnaryOp::Typeof) {
        // typeof on unresolved global should not throw.
        let arg = prog.exprs.get(argument);
        if let ExprKind::Identifier(atom) = &arg.kind {
            let loc = resolve_identifier(
                *atom,
                fc.func_scope_idx,
                fc.current_scope_idx,
                func_scopes,
                analysis,
            );
            if loc == VarLocation::Global {
                let name = prog.interner.get(*atom);
                let idx = fc.add_name_u16(name);
                fc.emit_u16(Op::TypeOfGlobal, idx);
                return Ok(());
            }
        }
    }
    if matches!(op, UnaryOp::Delete) {
        let arg = prog.exprs.get(argument);
        if let ExprKind::Member {
            object,
            property,
            computed,
        } = &arg.kind
        {
            compile_expr(fc, prog, analysis, func_scopes, *object)?;
            if !computed {
                if let MemberProp::Identifier(prop_atom) = property {
                    let prop_name = prog.interner.get(*prop_atom);
                    let name_idx = fc.add_name_u16(prop_name);
                    fc.emit_u16(Op::DeleteProp, name_idx);
                } else {
                    // Private field delete — always returns true per spec stub.
                    fc.emit(Op::Pop);
                    fc.emit(Op::PushTrue);
                }
            } else if let MemberProp::Expression(prop_expr_id) = property {
                compile_expr(fc, prog, analysis, func_scopes, *prop_expr_id)?;
                fc.emit(Op::DeleteElem);
            }
        } else {
            // `delete identifier` or `delete literal` — evaluate and return true.
            compile_expr(fc, prog, analysis, func_scopes, argument)?;
            fc.emit(Op::Pop);
            fc.emit(Op::PushTrue);
        }
        return Ok(());
    }
    compile_expr(fc, prog, analysis, func_scopes, argument)?;
    fc.emit(unary_op_to_opcode(op));
    Ok(())
}

/// Compile a prefix/postfix update expression (++/--).
pub(super) fn compile_update_expr(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    op: UpdateOp,
    prefix: bool,
    argument: NodeId<Expr>,
) -> Result<(), CompileError> {
    let arg = prog.exprs.get(argument);
    if let ExprKind::Identifier(atom) = &arg.kind {
        let loc = resolve_identifier(
            *atom,
            fc.func_scope_idx,
            fc.current_scope_idx,
            func_scopes,
            analysis,
        );
        match loc {
            VarLocation::Local(slot) => {
                let inc_op = match op {
                    UpdateOp::Increment => Op::IncLocal,
                    UpdateOp::Decrement => Op::DecLocal,
                };
                fc.emit_u16_u8(inc_op, slot, u8::from(prefix));
                return Ok(());
            }
            VarLocation::Global => {
                let name = prog.interner.get(*atom);
                let load_idx = fc.add_name_u16(name);
                let store_idx = fc.add_name_u16(name);
                emit_update_sequence(
                    fc,
                    op,
                    prefix,
                    |f| f.emit_u16(Op::GetGlobal, load_idx),
                    |f| {
                        f.emit_u16(Op::SetGlobal, store_idx);
                        f.emit(Op::Pop);
                    },
                );
                return Ok(());
            }
            VarLocation::Upvalue(uv_idx) => {
                emit_update_sequence(
                    fc,
                    op,
                    prefix,
                    |f| f.emit_u16(Op::GetUpvalue, uv_idx),
                    |f| {
                        f.emit_u16(Op::SetUpvalue, uv_idx);
                        f.emit(Op::Pop);
                    },
                );
                return Ok(());
            }
            VarLocation::Module(_) => {
                // Module imports are immutable — fall through to push undefined.
            }
        }
    }
    // Member expression updates (++obj.prop, obj[key]++).
    if let ExprKind::Member {
        object,
        property,
        computed,
    } = &arg.kind
    {
        compile_expr(fc, prog, analysis, func_scopes, *object)?;
        if !computed {
            if let MemberProp::Identifier(prop_atom) = property {
                // Static property: use IncProp/DecProp.
                let prop_name = prog.interner.get(*prop_atom);
                let name_idx = fc.add_name_u16(prop_name);
                let inc_op = match op {
                    UpdateOp::Increment => Op::IncProp,
                    UpdateOp::Decrement => Op::DecProp,
                };
                fc.emit_u16_u8(inc_op, name_idx, u8::from(prefix));
            } else {
                // PrivateIdentifier — unsupported for now, just keep value.
            }
        } else if let MemberProp::Expression(prop_expr_id) = property {
            // Computed property: use IncElem/DecElem.
            compile_expr(fc, prog, analysis, func_scopes, *prop_expr_id)?;
            let inc_op = match op {
                UpdateOp::Increment => Op::IncElem,
                UpdateOp::Decrement => Op::DecElem,
            };
            fc.emit_u8(inc_op, u8::from(prefix));
        }
    } else {
        // Unsupported update target — just evaluate for side effects.
        compile_expr(fc, prog, analysis, func_scopes, argument)?;
    }
    Ok(())
}
