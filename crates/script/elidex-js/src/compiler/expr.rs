//! Expression compilation: ExprKind → bytecode.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::bytecode::compiled::Constant;
use crate::bytecode::opcode::Op;

use super::function::FunctionCompiler;
use super::resolve::FunctionScope;
use super::CompileError;
use crate::scope::ScopeAnalysis;

use super::expr_assign::{compile_assignment, compile_identifier_load};
use super::expr_member::{
    compile_arguments, compile_call_expr, compile_member_property, compile_optional_chain_expr,
};
use super::expr_object::compile_object_expr;
use super::expr_ops::{binary_op_to_opcode, compile_unary_expr, compile_update_expr};

/// Compile an expression, leaving its value on the stack.
#[allow(clippy::too_many_lines)]
pub fn compile_expr(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    expr_id: NodeId<Expr>,
) -> Result<(), CompileError> {
    let expr = prog.exprs.get(expr_id);
    let span = expr.span;
    fc.source_map.add(fc.pc(), span);

    match &expr.kind {
        ExprKind::Literal(lit) => compile_literal(fc, prog, lit),
        ExprKind::Identifier(atom) => {
            compile_identifier_load(fc, prog, analysis, func_scopes, *atom);
        }
        ExprKind::This => fc.emit(Op::PushThis),

        ExprKind::Binary { left, op, right } => {
            compile_expr(fc, prog, analysis, func_scopes, *left)?;
            compile_expr(fc, prog, analysis, func_scopes, *right)?;
            fc.emit(binary_op_to_opcode(*op));
        }

        ExprKind::Unary { op, argument } => {
            compile_unary_expr(fc, prog, analysis, func_scopes, *op, *argument)?;
        }

        ExprKind::Logical { left, op, right } => {
            compile_expr(fc, prog, analysis, func_scopes, *left)?;
            let jump_op = match op {
                LogicalOp::And => Op::JumpIfFalse,
                LogicalOp::Or => Op::JumpIfTrue,
                LogicalOp::NullCoal => Op::JumpIfNotNullish,
            };
            // Short-circuit: duplicate TOS, conditional jump over RHS.
            fc.emit(Op::Dup);
            let patch = fc.emit_jump(jump_op);
            fc.emit(Op::Pop); // discard LHS
            compile_expr(fc, prog, analysis, func_scopes, *right)?;
            fc.patch_jump(patch);
        }

        ExprKind::Assignment { left, op, right } => {
            compile_assignment(fc, prog, analysis, func_scopes, left, *op, *right)?;
        }

        ExprKind::Conditional {
            test,
            consequent,
            alternate,
        } => {
            compile_expr(fc, prog, analysis, func_scopes, *test)?;
            let else_patch = fc.emit_jump(Op::JumpIfFalse);
            compile_expr(fc, prog, analysis, func_scopes, *consequent)?;
            let end_patch = fc.emit_jump(Op::Jump);
            fc.patch_jump(else_patch);
            compile_expr(fc, prog, analysis, func_scopes, *alternate)?;
            fc.patch_jump(end_patch);
        }

        ExprKind::Call { callee, arguments } => {
            compile_call_expr(fc, prog, analysis, func_scopes, *callee, arguments)?;
        }

        ExprKind::New { callee, arguments } => {
            compile_expr(fc, prog, analysis, func_scopes, *callee)?;
            compile_arguments(fc, prog, analysis, func_scopes, arguments)?;
            let argc = arguments.len() as u8;
            fc.emit_u8(Op::New, argc);
        }

        ExprKind::Member {
            object,
            property,
            computed,
        } => {
            compile_expr(fc, prog, analysis, func_scopes, *object)?;
            compile_member_property(fc, prog, analysis, func_scopes, property, *computed)?;
        }

        ExprKind::Array(elements) => {
            fc.emit(Op::CreateArray);
            for elem in elements {
                match elem {
                    Some(ArrayElement::Expression(e)) => {
                        compile_expr(fc, prog, analysis, func_scopes, *e)?;
                        fc.emit(Op::ArrayPush);
                    }
                    Some(ArrayElement::Spread(e)) => {
                        compile_expr(fc, prog, analysis, func_scopes, *e)?;
                        fc.emit(Op::ArraySpread);
                    }
                    None => fc.emit(Op::ArrayHole),
                }
            }
        }

        ExprKind::Object(properties) => {
            compile_object_expr(fc, prog, analysis, func_scopes, properties)?;
        }

        ExprKind::Template(tpl) => {
            // Push quasis and expressions interleaved, then concat.
            let total = tpl.quasis.len() + tpl.expressions.len();
            for (i, quasi) in tpl.quasis.iter().enumerate() {
                let cooked_u16: Vec<u16> = quasi
                    .cooked
                    .as_ref()
                    .map_or_else(Vec::new, |a| prog.interner.get(*a).to_vec());
                let idx = fc.add_constant(Constant::Wtf16(cooked_u16));
                fc.emit_u16(Op::PushConst, idx);
                if i < tpl.expressions.len() {
                    compile_expr(fc, prog, analysis, func_scopes, tpl.expressions[i])?;
                }
            }
            fc.emit_u16(Op::TemplateConcat, total as u16);
        }

        ExprKind::Sequence(exprs) => {
            for (i, &e) in exprs.iter().enumerate() {
                compile_expr(fc, prog, analysis, func_scopes, e)?;
                if i < exprs.len() - 1 {
                    fc.emit(Op::Pop); // discard intermediate values
                }
            }
        }

        ExprKind::Update {
            op,
            prefix,
            argument,
        } => {
            compile_update_expr(fc, prog, analysis, func_scopes, *op, *prefix, *argument)?;
        }

        ExprKind::Await(arg) => {
            compile_expr(fc, prog, analysis, func_scopes, *arg)?;
            fc.emit(Op::Await);
        }

        ExprKind::Yield { argument, delegate } => {
            if *delegate {
                // `yield* expr` is expanded inline into a loop that
                // drives the iterator's `.next(received)`, yielding each
                // `value` and using `result.value` as the yield*
                // expression's own value once `done` is true.  See
                // `compile_yield_star` for the full layout + abrupt
                // completion (return / throw) forwarding.
                let arg_id = argument.ok_or_else(|| CompileError {
                    message: "yield* requires an operand".into(),
                })?;
                compile_yield_star(fc, prog, analysis, func_scopes, arg_id)?;
            } else {
                if let Some(arg) = argument {
                    compile_expr(fc, prog, analysis, func_scopes, *arg)?;
                } else {
                    fc.emit(Op::PushUndefined);
                }
                fc.emit(Op::Yield);
            }
        }

        ExprKind::Spread(inner) => {
            // Spread is context-dependent; just compile the inner expression.
            compile_expr(fc, prog, analysis, func_scopes, *inner)?;
        }

        ExprKind::Paren(inner) => {
            compile_expr(fc, prog, analysis, func_scopes, *inner)?;
        }

        ExprKind::MetaProperty(MetaPropertyKind::NewTarget) => {
            // `new.target` ([C11] [[Construct]] step 4) — emits the
            // runtime read of `CallFrame::new_target`.
            fc.emit(Op::NewTarget);
        }
        ExprKind::Super | ExprKind::MetaProperty(_) | ExprKind::DynamicImport { .. } => {
            // Stubs for complex features. `Super` alone (without a
            // call wrapper) is a syntax error in ECMA-262 but the
            // parser may surface it for `super.method()` (Step-9
            // deferred, slot `#11-step9-class-extras`); leave as
            // Undefined so the GetSuperProp stub still observes the
            // existing shape.
            fc.emit(Op::PushUndefined);
        }

        ExprKind::Function(func) => {
            let child_func = compile_nested_function(fc, prog, analysis, func_scopes, func, false)?;
            let idx = fc.add_constant(Constant::Function(Box::new(child_func)));
            fc.emit_u16(Op::Closure, idx);
        }

        ExprKind::Arrow(arrow) => {
            let child_func = compile_arrow_function(fc, prog, analysis, func_scopes, arrow)?;
            let idx = fc.add_constant(Constant::Function(Box::new(child_func)));
            fc.emit_u16(Op::Closure, idx);
        }

        ExprKind::OptionalChain { base, chain } => {
            compile_optional_chain_expr(fc, prog, analysis, func_scopes, *base, chain)?;
        }

        ExprKind::PrivateIn { name, right } => {
            compile_expr(fc, prog, analysis, func_scopes, *right)?;
            let name_u16 = prog.interner.get(*name);
            let idx = fc.add_name_u16(name_u16);
            fc.emit_u16(Op::PrivateIn, idx);
        }

        ExprKind::Class(class) => {
            compile_class(fc, prog, analysis, func_scopes, class)?;
        }

        // Tagged templates — not yet implemented.
        ExprKind::TaggedTemplate { .. } => {
            fc.emit(Op::PushUndefined);
        }

        ExprKind::Error => fc.emit(Op::PushUndefined),
    }

    Ok(())
}

/// Compile a literal value.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn compile_literal(fc: &mut FunctionCompiler, prog: &Program, lit: &Literal) {
    match lit {
        Literal::Number(n) => {
            // Optimize small integers.
            let ni = *n as i64;
            if (ni as f64).to_bits() == n.to_bits()
                && (i64::from(i8::MIN)..=i64::from(i8::MAX)).contains(&ni)
            {
                fc.emit_u8(Op::PushI8, ni as u8);
            } else {
                let idx = fc.add_constant(Constant::Number(*n));
                fc.emit_u16(Op::PushConst, idx);
            }
        }
        Literal::String(atom) => {
            let units = prog.interner.get(*atom).to_vec();
            let idx = fc.add_constant(Constant::Wtf16(units));
            fc.emit_u16(Op::PushConst, idx);
        }
        Literal::Boolean(true) => fc.emit(Op::PushTrue),
        Literal::Boolean(false) => fc.emit(Op::PushFalse),
        Literal::Null => fc.emit(Op::PushNull),
        Literal::BigInt(atom) => {
            let s = prog.interner.get_utf8(*atom);
            let idx = fc.add_constant(Constant::BigInt(s));
            fc.emit_u16(Op::PushConst, idx);
        }
        Literal::RegExp { pattern, flags } => {
            let p = prog.interner.get_utf8(*pattern);
            let f = prog.interner.get_utf8(*flags);
            let idx = fc.add_constant(Constant::RegExp {
                pattern: p,
                flags: f,
            });
            fc.emit_u16(Op::PushConst, idx);
        }
    }
}

// Re-exports from split modules so that existing call-sites keep working.
pub(super) use super::expr_class::compile_class;
pub(super) use super::expr_function::{compile_arrow_function, compile_nested_function};
pub(super) use super::expr_yield_star::compile_yield_star;
