//! Class compilation: class declarations and expressions.

#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::bytecode::compiled::Constant;
use crate::bytecode::opcode::Op;

use super::function::FunctionCompiler;
use super::resolve::FunctionScope;
use super::CompileError;
use crate::scope::{ScopeAnalysis, ScopeKind};

use super::expr::compile_expr;
use super::expr_function::compile_nested_function;

// ── Class compilation ──────────────────────────────────────────────

/// Compile a class declaration or expression.
///
/// Desugars the class into existing opcodes:
/// 1. Compile the constructor (or create a default one) as a function → `Closure`
/// 2. Create a prototype object and attach it to the constructor via `DefineProperty`
/// 3. Define prototype methods on the prototype object
/// 4. Define static methods on the constructor object
/// 5. Set up the `constructor` back-link on the prototype
///
/// Stack effect: pushes the constructor (class) value.
#[allow(clippy::too_many_lines)]
pub(super) fn compile_class(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    class: &Class,
) -> Result<(), CompileError> {
    // 1. Find and compile the constructor method, or create a default one.
    let constructor = class.body.iter().find(|m| {
        matches!(
            m.kind,
            ClassMemberKind::Method {
                kind: MethodKind::Constructor,
                ..
            }
        )
    });

    if let Some(ctor_member) = constructor {
        if let ClassMemberKind::Method { function, .. } = &ctor_member.kind {
            let child = compile_nested_function(fc, prog, analysis, func_scopes, function, false)?;
            let idx = fc.add_constant(Constant::Function(Box::new(child)));
            fc.emit_u16(Op::Closure, idx);
        }
    } else {
        // Default constructor: an empty function that returns undefined.
        // Use PushUndefined + Return (not ReturnUndefined) because ReturnUndefined
        // has special completion-value semantics for script-level eval.
        let default_ctor = crate::bytecode::compiled::CompiledFunction {
            bytecode: vec![Op::PushUndefined as u8, Op::Return as u8],
            name: class.name.map(|a| prog.interner.get_utf8(a)),
            ..Default::default()
        };
        let idx = fc.add_constant(Constant::Function(Box::new(default_ctor)));
        fc.emit_u16(Op::Closure, idx);
    }
    // Stack: [constructor]

    // 2. Create a prototype object and link it:
    //    proto.constructor = ctor; ctor.prototype = proto
    fc.emit(Op::Dup); //                          [ctor ctor]
    fc.emit(Op::CreateObject); //                 [ctor ctor proto]
    fc.emit(Op::Swap); //                         [ctor proto ctor]
    let constructor_name = fc.add_name("constructor");
    fc.emit_u16(Op::DefineProperty, constructor_name); // [ctor proto]  (proto.constructor = ctor)
    let prototype_name = fc.add_name("prototype");
    fc.emit_u16(Op::DefineProperty, prototype_name); // [ctor]  (ctor.prototype = proto)

    // 3. Define prototype methods.
    for member in &class.body {
        match &member.kind {
            ClassMemberKind::Method {
                key,
                function,
                kind,
                is_static,
                computed,
            } => {
                if matches!(kind, MethodKind::Constructor) {
                    continue;
                }

                let child =
                    compile_nested_function(fc, prog, analysis, func_scopes, function, false)?;
                let const_idx = fc.add_constant(Constant::Function(Box::new(child)));

                if *is_static {
                    // Static method: define on the constructor itself.
                    // Stack: [ctor]
                    fc.emit(Op::Dup); // [ctor ctor]
                    emit_class_method_define(
                        fc,
                        prog,
                        analysis,
                        func_scopes,
                        key,
                        *computed,
                        *kind,
                        const_idx,
                    )?;
                    // After: [ctor target]
                    fc.emit(Op::Pop); // [ctor]
                } else {
                    // Prototype method: define on constructor.prototype.
                    // Stack: [ctor]
                    fc.emit(Op::Dup); // [ctor ctor]
                    let proto_name = fc.add_name("prototype");
                    fc.emit_u16(Op::GetProp, proto_name); // [ctor proto]
                    emit_class_method_define(
                        fc,
                        prog,
                        analysis,
                        func_scopes,
                        key,
                        *computed,
                        *kind,
                        const_idx,
                    )?;
                    // After: [ctor proto]
                    fc.emit(Op::Pop); // [ctor]
                }
            }
            ClassMemberKind::PrivateMethod {
                name,
                function,
                kind,
                is_static,
            } => {
                let child =
                    compile_nested_function(fc, prog, analysis, func_scopes, function, false)?;
                let const_idx = fc.add_constant(Constant::Function(Box::new(child)));
                let name_u16 = prog.interner.get(*name);

                if *is_static {
                    fc.emit(Op::Dup); // [ctor ctor]
                    fc.emit_u16(Op::Closure, const_idx); // [ctor ctor method]
                    let define_op = match kind {
                        MethodKind::Get => Op::DefineGetter,
                        MethodKind::Set => Op::DefineSetter,
                        _ => Op::DefineProperty,
                    };
                    let idx = fc.add_name_u16(name_u16);
                    fc.emit_u16(define_op, idx);
                    if matches!(define_op, Op::DefineGetter | Op::DefineSetter) {
                        fc.bytecode.push(0); // non-enumerable for class
                    }
                    fc.emit(Op::Pop); // [ctor]
                } else {
                    fc.emit(Op::Dup); // [ctor ctor]
                    let proto_name = fc.add_name("prototype");
                    fc.emit_u16(Op::GetProp, proto_name); // [ctor proto]
                    fc.emit_u16(Op::Closure, const_idx); // [ctor proto method]
                    let define_op = match kind {
                        MethodKind::Get => Op::DefineGetter,
                        MethodKind::Set => Op::DefineSetter,
                        _ => Op::DefineProperty,
                    };
                    let idx = fc.add_name_u16(name_u16);
                    fc.emit_u16(define_op, idx);
                    if matches!(define_op, Op::DefineGetter | Op::DefineSetter) {
                        fc.bytecode.push(0); // non-enumerable for class
                    }
                    fc.emit(Op::Pop); // [ctor]
                }
            }
            ClassMemberKind::Property {
                key,
                value,
                is_static,
                computed,
            } => {
                // Static properties are defined on the constructor.
                // Instance properties would be initialized in the constructor, but
                // for simplicity we skip instance field initializers for now.
                if *is_static {
                    fc.emit(Op::Dup); // [ctor ctor]
                    if *computed {
                        // [ctor ctor] → compile key → [ctor ctor key]
                        if let PropertyKey::Computed(expr) = key {
                            compile_expr(fc, prog, analysis, func_scopes, *expr)?;
                        }
                        // → compile value → [ctor ctor key value]
                        if let Some(val_expr) = value {
                            compile_expr(fc, prog, analysis, func_scopes, *val_expr)?;
                        } else {
                            fc.emit(Op::PushUndefined);
                        }
                        fc.emit(Op::DefineComputedProperty); // [ctor ctor]
                    } else {
                        if let Some(val_expr) = value {
                            compile_expr(fc, prog, analysis, func_scopes, *val_expr)?;
                        } else {
                            fc.emit(Op::PushUndefined);
                        }
                        // [ctor ctor value]
                        emit_class_member_name_op(fc, prog, key, false, Op::DefineProperty)?;
                    }
                    // [ctor ctor]
                    fc.emit(Op::Pop); // [ctor]
                }
                // Non-static properties: skip (would need field initializer injection).
            }
            ClassMemberKind::PrivateField {
                name,
                value,
                is_static,
            } => {
                if *is_static {
                    fc.emit(Op::Dup); // [ctor ctor]
                    if let Some(val_expr) = value {
                        compile_expr(fc, prog, analysis, func_scopes, *val_expr)?;
                    } else {
                        fc.emit(Op::PushUndefined);
                    }
                    let name_u16 = prog.interner.get(*name);
                    let idx = fc.add_name_u16(name_u16);
                    fc.emit_u16(Op::DefineProperty, idx); // [ctor ctor]
                    fc.emit(Op::Pop); // [ctor]
                }
            }
            ClassMemberKind::StaticBlock(stmts) => {
                // Static blocks execute as strict, with `this` bound to the class.
                // Compile as a separate function scope (IIFE) for correct
                // var-hoisting and scope isolation, then immediately call
                // with the constructor as `this`.
                // Stack: [ctor]
                fc.emit(Op::Dup); // [ctor ctor] — keep ctor, use copy as this

                let mut child_fc =
                    FunctionCompiler::new(fc.func_scope_idx, fc.current_scope_idx, true);
                for &stmt_id in stmts {
                    super::stmt::compile_stmt(&mut child_fc, prog, analysis, func_scopes, stmt_id)?;
                }
                child_fc.emit(Op::PushUndefined);
                child_fc.emit(Op::Return);

                let child = child_fc.finish(&func_scopes[fc.func_scope_idx]);
                let const_idx = fc.add_constant(Constant::Function(Box::new(child)));
                fc.emit_u16(Op::Closure, const_idx); // [ctor ctor closure]
                fc.emit_u8(Op::CallMethod, 0); // [ctor result] — this=ctor
                fc.emit(Op::Pop); // [ctor] — discard result
            }
            ClassMemberKind::Empty => {}
        }
    }

    // Stack: [constructor]. This is the class value.

    // Initialize the inner class name binding so that methods can reference
    // the class via upvalue capture. The scope analysis creates a Block scope
    // with the class span that contains a Class binding for the name.
    if let Some(name) = class.name {
        // Find the inner block scope for this class (ScopeKind::Block with class.span).
        let inner_scope_idx = analysis.scopes.iter().enumerate().find_map(|(idx, scope)| {
            if scope.kind == ScopeKind::Block && scope.span == class.span {
                Some(idx)
            } else {
                None
            }
        });
        if let Some(inner_idx) = inner_scope_idx {
            if let Some(info) = func_scopes[fc.func_scope_idx]
                .locals
                .get(&(inner_idx, name))
            {
                let slot = info.slot;
                fc.emit(Op::Dup); // keep class on stack
                fc.emit_u16(Op::InitLocal, slot);
                fc.emit_u16(Op::SetLocal, slot);
                fc.emit(Op::Pop); // pop the dup
            }
        }
    }

    Ok(())
}

/// Emit a method definition on a target object (class prototype or constructor).
///
/// Stack on entry: `[target]`. Stack on exit: `[target]`.
/// Handles both computed and static keys, and getter/setter kinds.
#[allow(clippy::too_many_arguments)]
fn emit_class_method_define(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    key: &PropertyKey,
    computed: bool,
    kind: MethodKind,
    closure_const_idx: u16,
) -> Result<(), CompileError> {
    if computed {
        if !matches!(kind, MethodKind::Method) {
            return Err(CompileError {
                message: "computed getter/setter keys not yet supported (deferred to M4-10.2)"
                    .into(),
            });
        }
        // Stack: [target]
        // → compile key → [target key]
        if let PropertyKey::Computed(expr) = key {
            compile_expr(fc, prog, analysis, func_scopes, *expr)?;
        }
        // → closure → [target key closure]
        fc.emit_u16(Op::Closure, closure_const_idx);
        // → DefineComputedMethod → [target] (non-enumerable per §14.3.8)
        fc.emit(Op::DefineComputedMethod);
        return Ok(());
    }
    // Non-computed: push closure then emit named define.
    fc.emit_u16(Op::Closure, closure_const_idx); // [target closure]
    let op = match kind {
        MethodKind::Get => Op::DefineGetter,
        MethodKind::Set => Op::DefineSetter,
        // Class methods are non-enumerable (§14.3.8).
        // Use DefineMethod (u16 name + u8 flags) for the correct descriptor.
        _ => Op::DefineMethod,
    };
    if op == Op::DefineMethod {
        // DefineMethod has u16 name + u8 flags format.
        let name_idx = match key {
            PropertyKey::Identifier(name) | PropertyKey::PrivateIdentifier(name) => {
                fc.add_name_u16(prog.interner.get(*name))
            }
            PropertyKey::Literal(Literal::String(s)) => fc.add_name_u16(prog.interner.get(*s)),
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            PropertyKey::Literal(Literal::Number(n)) => {
                let key_str = if *n == (*n as i64) as f64 && *n >= 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                };
                fc.add_name(&key_str)
            }
            _ => {
                return Err(CompileError {
                    message: "unsupported class member key type".into(),
                })
            }
        };
        fc.emit_u16_u8(Op::DefineMethod, name_idx, 0); // flags byte (unused)
        return Ok(());
    }
    emit_class_member_name_op(fc, prog, key, false, op)
}

/// Emit a DefineProperty/DefineGetter/DefineSetter with the appropriate key.
///
/// For computed keys, falls back to `DefineComputedProperty`.
fn emit_class_member_name_op(
    fc: &mut FunctionCompiler,
    prog: &Program,
    key: &PropertyKey,
    computed: bool,
    op: Op,
) -> Result<(), CompileError> {
    if computed {
        return Err(CompileError {
            message: "computed class member keys not yet supported".into(),
        });
    }
    let is_accessor = matches!(op, Op::DefineGetter | Op::DefineSetter);
    match key {
        PropertyKey::Identifier(name) | PropertyKey::PrivateIdentifier(name) => {
            let name_u16 = prog.interner.get(*name);
            let idx = fc.add_name_u16(name_u16);
            fc.emit_u16(op, idx);
        }
        PropertyKey::Literal(Literal::String(s)) => {
            let name_u16 = prog.interner.get(*s);
            let idx = fc.add_name_u16(name_u16);
            fc.emit_u16(op, idx);
        }
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        PropertyKey::Literal(Literal::Number(n)) => {
            let key_str = if *n == (*n as i64) as f64 && *n >= 0.0 {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            };
            let idx = fc.add_name(&key_str);
            fc.emit_u16(op, idx);
        }
        _ => {
            return Err(CompileError {
                message: "unsupported class member key type".into(),
            });
        }
    }
    // DefineGetter/DefineSetter have a flags byte: class accessors are non-enumerable.
    if is_accessor {
        fc.bytecode.push(0);
    }
    Ok(())
}
