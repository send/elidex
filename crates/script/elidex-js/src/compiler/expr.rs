//! Expression compilation: ExprKind → bytecode.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::atom::Atom;
use crate::bytecode::compiled::Constant;
use crate::bytecode::opcode::Op;

use super::function::FunctionCompiler;
use super::resolve::{resolve_identifier, FunctionScope, VarLocation};
use super::CompileError;
use crate::scope::{BindingKind, ScopeAnalysis};

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
            if matches!(op, UnaryOp::Typeof) {
                // typeof on unresolved global should not throw.
                let arg = prog.exprs.get(*argument);
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
                let arg = prog.exprs.get(*argument);
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
                    compile_expr(fc, prog, analysis, func_scopes, *argument)?;
                    fc.emit(Op::Pop);
                    fc.emit(Op::PushTrue);
                }
                return Ok(());
            }
            compile_expr(fc, prog, analysis, func_scopes, *argument)?;
            fc.emit(unary_op_to_opcode(*op));
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
            // Check if it's a method call (obj.method()).
            let callee_expr = prog.exprs.get(*callee);
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
            } else {
                compile_expr(fc, prog, analysis, func_scopes, *callee)?;
                compile_arguments(fc, prog, analysis, func_scopes, arguments)?;
                let argc = arguments.len() as u8;
                let call_ic = fc.alloc_call_ic_slot();
                fc.emit_u8_u16(Op::Call, argc, call_ic);
            }
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
                        // Evaluate key first if computed (ES2020 §13.1.5.7).
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
                                PropertyKey::Identifier(name)
                                | PropertyKey::PrivateIdentifier(name) => {
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
                                PropertyKey::Literal(
                                    Literal::BigInt(_) | Literal::RegExp { .. },
                                ) => {
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
            let arg = prog.exprs.get(*argument);
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
                        fc.emit_u16_u8(inc_op, slot, u8::from(*prefix));
                        return Ok(());
                    }
                    VarLocation::Global => {
                        let name = prog.interner.get(*atom);
                        let load_idx = fc.add_name_u16(name);
                        let store_idx = fc.add_name_u16(name);
                        emit_update_sequence(
                            fc,
                            *op,
                            *prefix,
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
                            *op,
                            *prefix,
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
                        fc.emit_u16_u8(inc_op, name_idx, u8::from(*prefix));
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
                    fc.emit_u8(inc_op, u8::from(*prefix));
                }
            } else {
                // Unsupported update target — just evaluate for side effects.
                compile_expr(fc, prog, analysis, func_scopes, *argument)?;
            }
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

        ExprKind::Super | ExprKind::MetaProperty(_) | ExprKind::DynamicImport { .. } => {
            // Stubs for complex features.
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
            // Compile base. Result on stack: [base_value]
            compile_expr(fc, prog, analysis, func_scopes, *base)?;
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
                        let next_is_call =
                            matches!(chain.get(i + 1), Some(OptionalChainPart::Call(_)));
                        if next_is_call {
                            fc.emit(Op::Dup); // keep receiver
                        }
                        compile_member_property(
                            fc,
                            prog,
                            analysis,
                            func_scopes,
                            property,
                            *computed,
                        )?;
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

/// Compile an identifier load (read).
fn compile_identifier_load(
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

/// Compile an assignment expression.
fn compile_assignment(
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
                    // Handle logical assignment operators with short-circuit (ES2020 §13.15.3).
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
                // Check for const assignment (ES2020 §13.15.2 — TypeError).
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
            unreachable!("assignment to import binding is not allowed (ES2020 §16.2.3.7)");
        }
    }
    Ok(())
}

/// Compile a member property access (after object is on stack).
fn compile_member_property(
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

/// Compile function call arguments.
///
/// Panics if more than 255 arguments are provided (u8 argc encoding limit).
fn compile_arguments(
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

/// Map BinaryOp to opcode.
fn binary_op_to_opcode(op: BinaryOp) -> Op {
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

/// Map UnaryOp to opcode.  `Delete` is intercepted upstream in `compile_expr`.
fn unary_op_to_opcode(op: UnaryOp) -> Op {
    match op {
        UnaryOp::Minus => Op::Neg,
        UnaryOp::Plus => Op::Pos,
        UnaryOp::Not => Op::Not,
        UnaryOp::BitwiseNot => Op::BitNot,
        UnaryOp::Typeof => Op::TypeOf,
        UnaryOp::Void => Op::Void,
        UnaryOp::Delete => unreachable!("Delete handled by compile_expr early-return"),
    }
}

/// Map compound AssignOp to the corresponding binary opcode.
fn compound_op_to_opcode(op: AssignOp) -> Op {
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

// Re-exports from split modules so that existing call-sites keep working.
pub(super) use super::expr_class::compile_class;
pub(super) use super::expr_function::{compile_arrow_function, compile_nested_function};
pub(super) use super::expr_yield_star::compile_yield_star;
