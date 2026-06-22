//! Object literal and accessor (getter/setter) compilation.

#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::bytecode::opcode::Op;
use crate::scope::ScopeAnalysis;

use super::expr::compile_expr;
use super::function::FunctionCompiler;
use super::resolve::FunctionScope;
use super::CompileError;

/// Compile a getter or setter property definition.
fn compile_accessor(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    property: &Property,
    define_op: Op,
    enumerable: bool,
) -> Result<(), CompileError> {
    if let Some(value) = property.value {
        compile_expr(fc, prog, analysis, func_scopes, value)?;
        let flags: u8 = u8::from(enumerable);
        match &property.key {
            PropertyKey::Identifier(name) => {
                let idx = fc.add_name_u16(prog.interner.get(*name));
                fc.emit_u16(define_op, idx);
                fc.bytecode.push(flags);
            }
            PropertyKey::Literal(Literal::String(s)) => {
                let idx = fc.add_name_u16(prog.interner.get(*s));
                fc.emit_u16(define_op, idx);
                fc.bytecode.push(flags);
            }
            _ => {
                fc.emit(Op::Pop);
            }
        }
    }
    Ok(())
}

/// Compile an object literal expression.
pub(super) fn compile_object_expr(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    properties: &[Property],
) -> Result<(), CompileError> {
    fc.emit(Op::CreateObject);
    for prop in properties {
        if prop.flags.is_spread() {
            // The spread expression is stored in `key` (as Computed(expr)),
            // not in `value` (which is None for spread properties).
            if let PropertyKey::Computed(expr_id) = &prop.key {
                compile_expr(fc, prog, analysis, func_scopes, *expr_id)?;
                fc.emit(Op::SpreadObject);
            }
            continue;
        }
        match prop.kind {
            PropertyKind::Init => {
                let is_computed = matches!(&prop.key, PropertyKey::Computed(_));
                // Evaluate key first if computed (ECMA-262 §13.2.5.6).
                if is_computed {
                    if let PropertyKey::Computed(e) = &prop.key {
                        compile_expr(fc, prog, analysis, func_scopes, *e)?;
                    }
                }
                // Then evaluate value.
                if let Some(value) = prop.value {
                    compile_expr(fc, prog, analysis, func_scopes, value)?;
                } else {
                    fc.emit(Op::PushUndefined);
                }
                // Define property.
                if is_computed {
                    fc.emit(Op::DefineComputedProperty);
                } else {
                    match &prop.key {
                        PropertyKey::Identifier(name) | PropertyKey::PrivateIdentifier(name) => {
                            let name_u16 = prog.interner.get(*name);
                            let idx = fc.add_name_u16(name_u16);
                            fc.emit_u16(Op::DefineProperty, idx);
                        }
                        PropertyKey::Literal(Literal::String(s)) => {
                            let name_u16 = prog.interner.get(*s);
                            let idx = fc.add_name_u16(name_u16);
                            fc.emit_u16(Op::DefineProperty, idx);
                        }
                        PropertyKey::Literal(Literal::Number(n)) => {
                            // ES2020: property key is ToString(number).
                            #[allow(
                                clippy::cast_possible_truncation,
                                clippy::cast_sign_loss,
                                clippy::cast_precision_loss
                            )]
                            let key_str = if *n == (*n as i64) as f64 && *n >= 0.0 {
                                format!("{}", *n as i64)
                            } else {
                                format!("{n}")
                            };
                            let idx = fc.add_name(&key_str);
                            fc.emit_u16(Op::DefineProperty, idx);
                        }
                        PropertyKey::Literal(Literal::Boolean(b)) => {
                            let idx = fc.add_name(if *b { "true" } else { "false" });
                            fc.emit_u16(Op::DefineProperty, idx);
                        }
                        PropertyKey::Literal(Literal::Null) => {
                            let idx = fc.add_name("null");
                            fc.emit_u16(Op::DefineProperty, idx);
                        }
                        PropertyKey::Literal(Literal::BigInt(_) | Literal::RegExp { .. }) => {
                            // BigInt/RegExp property keys are rare — emit empty string
                            // as a conservative fallback.
                            let idx = fc.add_name("");
                            fc.emit_u16(Op::DefineProperty, idx);
                        }
                        PropertyKey::Computed(_) => unreachable!(),
                    }
                }
            }
            PropertyKind::Get => {
                compile_accessor(
                    fc,
                    prog,
                    analysis,
                    func_scopes,
                    prop,
                    Op::DefineGetter,
                    true,
                )?;
            }
            PropertyKind::Set => {
                compile_accessor(
                    fc,
                    prog,
                    analysis,
                    func_scopes,
                    prop,
                    Op::DefineSetter,
                    true,
                )?;
            }
        }
    }
    Ok(())
}
