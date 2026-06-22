//! Member access, call, and optional chain compilation.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::bytecode::opcode::Op;
use crate::scope::ScopeAnalysis;

use super::expr::compile_expr;
use super::function::FunctionCompiler;
use super::resolve::FunctionScope;
use super::CompileError;

/// Compile a member property access (after object is on stack).
pub(super) fn compile_member_property(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    property: &MemberProp,
    computed: bool,
) -> Result<(), CompileError> {
    match property {
        MemberProp::Identifier(name) if !computed => {
            let name_u16 = prog.interner.get(*name);
            let idx = fc.add_name_u16(name_u16);
            let ic = fc.alloc_ic_slot();
            fc.emit_u16_u16(Op::GetProp, idx, ic);
        }
        MemberProp::Expression(e) => {
            compile_expr(fc, prog, analysis, func_scopes, *e)?;
            fc.emit(Op::GetElem);
        }
        MemberProp::Identifier(name) => {
            // computed identifier — compile as string key
            let name_u16 = prog.interner.get(*name);
            let idx = fc.add_name_u16(name_u16);
            fc.emit_u16(Op::PushConst, idx);
            fc.emit(Op::GetElem);
        }
        MemberProp::PrivateIdentifier(name) => {
            let name_u16 = prog.interner.get(*name);
            let idx = fc.add_name_u16(name_u16);
            fc.emit_u16(Op::GetPrivate, idx);
        }
    }
    Ok(())
}

/// Compile function call arguments.
///
/// Panics if more than 255 arguments are provided (u8 argc encoding limit).
pub(super) fn compile_arguments(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    arguments: &[Argument],
) -> Result<(), CompileError> {
    assert!(
        arguments.len() <= 255,
        "too many arguments ({}) — maximum 255 supported",
        arguments.len()
    );
    for arg in arguments {
        match arg {
            Argument::Expression(e) => {
                compile_expr(fc, prog, analysis, func_scopes, *e)?;
            }
            Argument::Spread(e) => {
                // Spread arguments are not yet supported — the expression is
                // compiled as a normal argument. The stack remains balanced since
                // the spread expression produces one value, matching the argc count.
                compile_expr(fc, prog, analysis, func_scopes, *e)?;
            }
        }
    }
    Ok(())
}

/// Compile a call expression (`callee(args)`, method calls, and `super(...args)`).
pub(super) fn compile_call_expr(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    callee: NodeId<Expr>,
    arguments: &[Argument],
) -> Result<(), CompileError> {
    // Check if it's a method call (obj.method()).
    let callee_expr = prog.exprs.get(callee);
    if let ExprKind::Member {
        object,
        property,
        computed,
    } = &callee_expr.kind
    {
        // Method call: push receiver, then callee.
        compile_expr(fc, prog, analysis, func_scopes, *object)?;
        fc.emit(Op::Dup); // keep receiver for CallMethod
        compile_member_property(fc, prog, analysis, func_scopes, property, *computed)?;
        compile_arguments(fc, prog, analysis, func_scopes, arguments)?;
        let argc = arguments.len() as u8;
        let call_ic = fc.alloc_call_ic_slot();
        fc.emit_u8_u16(Op::CallMethod, argc, call_ic);
    } else if matches!(callee_expr.kind, ExprKind::Super) {
        // `super(...args)` ([C13] SuperCall) — emit `Op::SuperCall`
        // (or `SuperCallSpread` when any argument is a spread,
        // [C19] ArgumentListEvaluation spread variant). The
        // VM resolves the super constructor via the current
        // frame's `home_class.[[Prototype]]`; no callee is
        // pushed onto the stack.
        let has_spread = arguments.iter().any(|a| matches!(a, Argument::Spread(_)));
        if has_spread {
            // Build an Array of all args, then SuperCallSpread.
            fc.emit(Op::CreateArray);
            for arg in arguments {
                match arg {
                    Argument::Expression(e) => {
                        compile_expr(fc, prog, analysis, func_scopes, *e)?;
                        fc.emit(Op::ArrayPush);
                    }
                    Argument::Spread(e) => {
                        compile_expr(fc, prog, analysis, func_scopes, *e)?;
                        fc.emit(Op::ArraySpread);
                    }
                }
            }
            fc.emit(Op::SuperCallSpread);
        } else {
            compile_arguments(fc, prog, analysis, func_scopes, arguments)?;
            let argc = arguments.len() as u8;
            fc.emit_u8(Op::SuperCall, argc);
        }
    } else {
        compile_expr(fc, prog, analysis, func_scopes, callee)?;
        compile_arguments(fc, prog, analysis, func_scopes, arguments)?;
        let argc = arguments.len() as u8;
        let call_ic = fc.alloc_call_ic_slot();
        fc.emit_u8_u16(Op::Call, argc, call_ic);
    }
    Ok(())
}

/// Compile an optional chain expression (`base?.chain`).
pub(super) fn compile_optional_chain_expr(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    base: NodeId<Expr>,
    chain: &[OptionalChainPart],
) -> Result<(), CompileError> {
    // Compile base. Result on stack: [base_value]
    compile_expr(fc, prog, analysis, func_scopes, base)?;
    // Dup for nullish check: [base_value base_value]
    fc.emit(Op::Dup);
    // JumpIfNullish peeks (does not pop). If nullish → jump to null_path.
    let null_path_patch = fc.emit_jump(Op::JumpIfNullish);
    // Not nullish — pop the dup, leaving original value: [base_value]
    fc.emit(Op::Pop);
    // Compile chain parts on the base value.
    for (i, part) in chain.iter().enumerate() {
        match part {
            OptionalChainPart::Member { property, computed } => {
                // If the next part is a Call, keep the receiver for
                // CallMethod so that `obj?.method()` binds `this`
                // correctly to `obj`.
                let next_is_call = matches!(chain.get(i + 1), Some(OptionalChainPart::Call(_)));
                if next_is_call {
                    fc.emit(Op::Dup); // keep receiver
                }
                compile_member_property(fc, prog, analysis, func_scopes, property, *computed)?;
            }
            OptionalChainPart::Call(arguments) => {
                compile_arguments(fc, prog, analysis, func_scopes, arguments)?;
                let argc = arguments.len() as u8;
                // Use CallMethod when preceded by a Member access
                // (receiver is on the stack below the callee).
                let prev_is_member =
                    i > 0 && matches!(chain[i - 1], OptionalChainPart::Member { .. });
                let call_ic = fc.alloc_call_ic_slot();
                if prev_is_member {
                    fc.emit_u8_u16(Op::CallMethod, argc, call_ic);
                } else {
                    fc.emit_u8_u16(Op::Call, argc, call_ic);
                }
            }
        }
    }
    // Result is on stack: [chain_result]. Jump to end.
    let end_patch = fc.emit_jump(Op::Jump);
    // Null path: stack is [base_value base_value] (dup still there).
    fc.patch_jump(null_path_patch);
    // Pop both copies, push undefined.
    fc.emit(Op::Pop);
    fc.emit(Op::Pop);
    fc.emit(Op::PushUndefined);
    // End: stack has exactly 1 value.
    fc.patch_jump(end_patch);
    Ok(())
}
