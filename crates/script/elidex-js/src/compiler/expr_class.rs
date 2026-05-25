//! Class compilation: class declarations and expressions.

use crate::arena::NodeId;
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

/// Class-heritage classification (ECMA-262 §15.7.14
/// ClassDefinitionEvaluation step 6).
///
/// Three syntactic forms, three runtime shapes:
/// - `None`: no `extends` clause → base class. `ctor.__proto__ =
///   %Function.prototype%` (default), `ctor.prototype.__proto__ =
///   %Object.prototype%` (default), default ctor is BASE.
/// - `Null`: `extends null` (literal). `ctor.__proto__ =
///   %Function.prototype%` (default), `ctor.prototype.__proto__ =
///   null`, default ctor is BASE (no super call). A user `super()`
///   call resolves to %Function.prototype%, which is not constructable
///   → spec-mandated TypeError at runtime.
/// - `Expr(super_id)`: derived class with constructor heritage.
///   `ctor.__proto__ = super`, `ctor.prototype.__proto__ =
///   super.prototype`, default ctor is DERIVED (`super(...args)`).
enum ClassHeritage {
    None,
    Null,
    Expr(NodeId<Expr>),
}

fn classify_heritage(prog: &Program, super_class: Option<NodeId<Expr>>) -> ClassHeritage {
    let Some(super_id) = super_class else {
        return ClassHeritage::None;
    };
    match &prog.exprs.get(super_id).kind {
        ExprKind::Literal(Literal::Null) => ClassHeritage::Null,
        _ => ClassHeritage::Expr(super_id),
    }
}

/// Build a synthesized default-class-constructor `CompiledFunction`
/// with the spec-required common attributes (class-ctor flag, strict
/// mode) auto-filled. Callers supply only the body bytecode + the
/// 3 shape parameters that differ between DERIVED (1 rest param) and
/// BASE (no params) (D-17b R9 G9-helper extraction — Copilot /review
/// Sug#4 close).
///
/// `is_class_ctor: true` makes the dispatch class-ctor carve-outs fire
/// (see `dispatch.rs` Op::ReturnUndefined). `is_strict: true` reflects
/// ECMA-262 §15.7 ClassBody strict-mode region — synthesized bodies
/// today have no strict-vs-loose-divergent ops but the flag keeps the
/// spec invariant explicit (D-17b R8 G8-1/G8-2 alignment).
fn synthesize_default_class_ctor(
    bytecode: Vec<u8>,
    name: Option<String>,
    param_count: u16,
    local_count: u16,
    has_rest_param: bool,
) -> crate::bytecode::compiled::CompiledFunction {
    crate::bytecode::compiled::CompiledFunction {
        bytecode,
        name,
        param_count,
        local_count,
        has_rest_param,
        is_class_ctor: true,
        is_strict: true,
        ..Default::default()
    }
}

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

    let heritage = classify_heritage(prog, class.super_class);
    if let Some(ctor_member) = constructor {
        if let ClassMemberKind::Method { function, .. } = &ctor_member.kind {
            let mut child =
                compile_nested_function(fc, prog, analysis, func_scopes, function, false)?;
            // User-written constructor: mark for home_class threading
            // ([C16] ClassDefinitionEvaluation — class ctor frames get
            // `CallFrame::home_class = closure_obj_id`).
            child.is_class_ctor = true;
            let idx = fc.add_constant(Constant::Function(Box::new(child)));
            fc.emit_u16(Op::Closure, idx);
        }
    } else if matches!(heritage, ClassHeritage::Expr(_)) {
        // Default DERIVED constructor synthesis ([C16] §15.7.14
        // default-constructor branch when ClassHeritage is present):
        // equivalent of `constructor(...args) { super(...args); }`.
        // Emitted as direct bytecode (no AST round-trip) so we don't
        // duplicate the parser's NodeId arena. Rest-param packing at
        // frame entry (Stage 0) materializes the `args` array in
        // slot 0; SuperCallSpread consumes it and propagates the
        // outer NewTarget via the dispatch-class core.
        // NOTE on the explicit `PushUndefined; Return` tail (instead
        // of `Pop; ReturnUndefined`): the script-level
        // completion-value-capture path in `Op::Pop` at the entry
        // frame would otherwise stash the super-call return value as
        // the frame's completion (dispatch.rs Op::Pop entry-frame
        // branch), and `Op::ReturnUndefined` at the entry frame
        // returns that completion rather than Undefined. invoke_upgrade
        // then sees the super-call's NEW wrapper as the ctor's
        // return value, the SameValue check at upgrade.rs §4.13.5
        // step 12.2 fails, and the upgrade aborts. `Pop; PushUndefined;
        // Return` discards the super result explicitly + returns a
        // literal Undefined that `Op::Return` (which uses the popped
        // value, not completion_value) propagates faithfully.
        let bytecode = vec![
            Op::GetLocal as u8,
            0,
            0, // u16 LE slot 0
            Op::SuperCallSpread as u8,
            Op::Pop as u8,
            Op::PushUndefined as u8,
            Op::Return as u8,
        ];
        let default_derived_ctor = synthesize_default_class_ctor(
            bytecode,
            class.name.map(|a| prog.interner.get_utf8(a)),
            /* param_count */ 1, // (...args)
            /* local_count */ 1, // slot 0 for args
            /* has_rest_param */ true,
        );
        let idx = fc.add_constant(Constant::Function(Box::new(default_derived_ctor)));
        fc.emit_u16(Op::Closure, idx);
    } else {
        // Default BASE constructor ([C16] default-constructor branch
        // when ClassHeritage is absent): empty body. Use PushUndefined
        // + Return (not ReturnUndefined) because ReturnUndefined has
        // special completion-value semantics for script-level eval.
        let default_ctor = synthesize_default_class_ctor(
            vec![Op::PushUndefined as u8, Op::Return as u8],
            class.name.map(|a| prog.interner.get_utf8(a)),
            /* param_count */ 0,
            /* local_count */ 0,
            /* has_rest_param */ false,
        );
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

    // 2b. Super-class chain setup ([C16] ClassDefinitionEvaluation —
    // `constructorParent` = super_class for derived classes (so
    // `MyEl.__proto__ === HTMLElement` & static-method inheritance
    // works) + `protoParent` = `super_class.prototype` (so
    // `MyEl.prototype.__proto__ === HTMLElement.prototype` & instance
    // method inheritance works).
    match heritage {
        ClassHeritage::None => {} // default proto chain inherited from %Object.prototype%.
        ClassHeritage::Null => {
            // `extends null` (ECMA-262 §15.7.14 step 6.f): protoParent
            // = null, constructorParent = %Function.prototype%. The
            // closure's `[[Prototype]]` already defaults to
            // %Function.prototype%, so we only splice
            // `ctor.prototype.[[Prototype]] = null`. A user-written
            // `super()` then resolves to %Function.prototype% via
            // GetSuperConstructor and throws TypeError on Construct —
            // spec-mandated.
            fc.emit(Op::Dup); //                          [ctor ctor]
            let proto_name = fc.add_name("prototype");
            let ic1 = fc.alloc_ic_slot();
            fc.emit_u16_u16(Op::GetProp, proto_name, ic1); // [ctor ctor.prototype]
            fc.emit(Op::PushNull); //                     [ctor ctor.prototype null]
            fc.emit(Op::SetPrototype); //                 [ctor ctor.prototype]
            fc.emit(Op::Pop); //                          [ctor]
        }
        ClassHeritage::Expr(super_id) => {
            // ECMA-262 §15.7.14 step 6 ClassDefinitionEvaluation: the
            // ClassHeritage expression is evaluated **exactly once**.
            // Stash the super value in a temp local so both prototype
            // splices below read from it instead of re-emitting (and
            // re-evaluating) the expression — fixes the duplicate
            // side-effect bug Copilot R6-1 flagged.
            let super_slot = func_scopes[fc.func_scope_idx].next_local;
            func_scopes[fc.func_scope_idx].next_local += 1;

            // Stack: [ctor]
            compile_expr(fc, prog, analysis, func_scopes, super_id)?; // [ctor super]
                                                                      // `Op::SetLocal` is peek-then-store (stack effect
                                                                      // `[value -- value]`), so an explicit `Pop` is needed to
                                                                      // discard the duplicate.
            fc.emit_u16(Op::SetLocal, super_slot); //                    [ctor super]
                                                   // ECMA-262 §15.7.14 step 6.f ClassDefinitionEvaluation:
                                                   // if the heritage value is not a constructor (callable but
                                                   // [[Construct]]-less, e.g. Symbol / BigInt / arrow fn),
                                                   // throw TypeError at class-definition time — NOT later at
                                                   // super() dispatch (D-17b R17 G17-1). `Op::AssertConstructor`
                                                   // pops the duplicate left by `SetLocal` so the splice phase
                                                   // below reads the validated value via `GetLocal super_slot`.
            fc.emit(Op::AssertConstructor); //                            [ctor]

            // First splice: ctor.prototype.__proto__ = super.prototype.
            fc.emit(Op::Dup); //                          [ctor ctor]
            let proto_name = fc.add_name("prototype");
            let ic1 = fc.alloc_ic_slot();
            fc.emit_u16_u16(Op::GetProp, proto_name, ic1); // [ctor ctor.prototype]
            fc.emit_u16(Op::GetLocal, super_slot); //       [ctor ctor.prototype super]
            let proto_name2 = fc.add_name("prototype");
            let ic2 = fc.alloc_ic_slot();
            fc.emit_u16_u16(Op::GetProp, proto_name2, ic2); // [ctor ctor.prototype super.prototype]
            fc.emit(Op::SetPrototype); //                 [ctor ctor.prototype]
            fc.emit(Op::Pop); //                          [ctor]

            // Second splice: ctor.__proto__ = super.
            fc.emit(Op::Dup); //                          [ctor ctor]
            fc.emit_u16(Op::GetLocal, super_slot); //       [ctor ctor super]
            fc.emit(Op::SetPrototype); //                 [ctor ctor]
            fc.emit(Op::Pop); //                          [ctor]
        }
    }

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
                    {
                        let ic = fc.alloc_ic_slot();
                        fc.emit_u16_u16(Op::GetProp, proto_name, ic);
                    } // [ctor proto]
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
                    {
                        let ic = fc.alloc_ic_slot();
                        fc.emit_u16_u16(Op::GetProp, proto_name, ic);
                    } // [ctor proto]
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
                {
                    let call_ic = fc.alloc_call_ic_slot();
                    fc.emit_u8_u16(Op::CallMethod, 0, call_ic);
                } // [ctor result] — this=ctor
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
        if let PropertyKey::Computed(expr) = key {
            compile_expr(fc, prog, analysis, func_scopes, *expr)?;
        }
        fc.emit_u16(Op::Closure, closure_const_idx);
        let op = match kind {
            MethodKind::Get => Op::DefineComputedGetter,
            MethodKind::Set => Op::DefineComputedSetter,
            // Constructor is never emitted here (parser places it on the class itself).
            MethodKind::Method | MethodKind::Constructor => Op::DefineComputedMethod,
        };
        fc.emit(op);
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
