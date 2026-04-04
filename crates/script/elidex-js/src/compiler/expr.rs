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
use crate::scope::{BindingKind, ScopeAnalysis, ScopeKind};
use crate::span::Span;

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
                        let idx = fc.add_name(name);
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
                            let name_idx = fc.add_name(prop_name);
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
                fc.emit_u8(Op::CallMethod, argc);
            } else {
                compile_expr(fc, prog, analysis, func_scopes, *callee)?;
                compile_arguments(fc, prog, analysis, func_scopes, arguments)?;
                let argc = arguments.len() as u8;
                fc.emit_u8(Op::Call, argc);
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
                                    let name_str = prog.interner.get(*name);
                                    let idx = fc.add_name(name_str);
                                    fc.emit_u16(Op::DefineProperty, idx);
                                }
                                PropertyKey::Literal(Literal::String(s)) => {
                                    let name_str = prog.interner.get(*s);
                                    let idx = fc.add_name(name_str);
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
                        compile_accessor(fc, prog, analysis, func_scopes, prop, Op::DefineGetter)?;
                    }
                    PropertyKind::Set => {
                        compile_accessor(fc, prog, analysis, func_scopes, prop, Op::DefineSetter)?;
                    }
                }
            }
        }

        ExprKind::Template(tpl) => {
            // Push quasis and expressions interleaved, then concat.
            let total = tpl.quasis.len() + tpl.expressions.len();
            for (i, quasi) in tpl.quasis.iter().enumerate() {
                let cooked = quasi.cooked.as_ref().map_or("", |a| prog.interner.get(*a));
                let idx = fc.add_constant(Constant::String(cooked.to_string()));
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
                        let load_idx = fc.add_name(name);
                        let store_idx = fc.add_name(name);
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
                        let name_idx = fc.add_name(prop_name);
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
            if let Some(arg) = argument {
                compile_expr(fc, prog, analysis, func_scopes, *arg)?;
            } else {
                fc.emit(Op::PushUndefined);
            }
            if *delegate {
                fc.emit(Op::YieldDelegate);
            } else {
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
            for part in chain {
                match part {
                    OptionalChainPart::Member { property, computed } => {
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
                        fc.emit_u8(Op::Call, argc);
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
            let name_str = prog.interner.get(*name);
            let idx = fc.add_name(name_str);
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
            let s = prog.interner.get(*atom).to_string();
            let idx = fc.add_constant(Constant::String(s));
            fc.emit_u16(Op::PushConst, idx);
        }
        Literal::Boolean(true) => fc.emit(Op::PushTrue),
        Literal::Boolean(false) => fc.emit(Op::PushFalse),
        Literal::Null => fc.emit(Op::PushNull),
        Literal::BigInt(atom) => {
            let s = prog.interner.get(*atom).to_string();
            let idx = fc.add_constant(Constant::BigInt(s));
            fc.emit_u16(Op::PushConst, idx);
        }
        Literal::RegExp { pattern, flags } => {
            let p = prog.interner.get(*pattern).to_string();
            let f = prog.interner.get(*flags).to_string();
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
            // Check TDZ if needed.
            if let Some(info) = func_scopes[fc.func_scope_idx].get_local(atom) {
                if info.needs_tdz {
                    fc.emit_u16(Op::CheckTdz, slot);
                }
            }
            fc.emit_u16(Op::GetLocal, slot);
        }
        VarLocation::Upvalue(idx) => fc.emit_u16(Op::GetUpvalue, idx),
        VarLocation::Global => {
            let name = prog.interner.get(atom);
            let idx = fc.add_name(name);
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
                                let name_str = prog.interner.get(*name);
                                let idx = fc.add_name(name_str);
                                fc.emit_u16(Op::SetProp, idx);
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
            if let Some(info) = func_scopes[fc.func_scope_idx].get_local(atom) {
                // Check for const assignment (ES2020 §13.15.2 — TypeError).
                if info.kind == BindingKind::Const {
                    return Err(CompileError {
                        message: format!(
                            "Assignment to constant variable '{}'",
                            prog.interner.get(atom)
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
            let idx = fc.add_name(name);
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
            let name_str = prog.interner.get(*name);
            let idx = fc.add_name(name_str);
            fc.emit_u16(Op::GetProp, idx);
        }
        MemberProp::Expression(e) => {
            compile_expr(fc, prog, analysis, func_scopes, *e)?;
            fc.emit(Op::GetElem);
        }
        MemberProp::Identifier(name) => {
            // computed identifier — compile as string key
            let name_str = prog.interner.get(*name);
            let idx = fc.add_name(name_str);
            fc.emit_u16(Op::PushConst, idx);
            fc.emit(Op::GetElem);
        }
        MemberProp::PrivateIdentifier(name) => {
            let name_str = prog.interner.get(*name);
            let idx = fc.add_name(name_str);
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
) -> Result<(), CompileError> {
    if let Some(value) = property.value {
        compile_expr(fc, prog, analysis, func_scopes, value)?;
        match &property.key {
            PropertyKey::Identifier(name) => {
                let idx = fc.add_name(prog.interner.get(*name));
                fc.emit_u16(define_op, idx);
            }
            PropertyKey::Literal(Literal::String(s)) => {
                let idx = fc.add_name(prog.interner.get(*s));
                fc.emit_u16(define_op, idx);
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

/// Map UnaryOp to opcode.
fn unary_op_to_opcode(op: UnaryOp) -> Op {
    match op {
        UnaryOp::Minus => Op::Neg,
        UnaryOp::Plus => Op::Pos,
        UnaryOp::Not => Op::Not,
        UnaryOp::BitwiseNot => Op::BitNot,
        UnaryOp::Typeof => Op::TypeOf,
        UnaryOp::Void | UnaryOp::Delete => Op::Void, // TODO: proper delete
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
            name: class.name.map(|a| prog.interner.get(a).to_string()),
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
                    fc.emit_u16(Op::Closure, const_idx); // [ctor ctor method]
                    match kind {
                        MethodKind::Get => {
                            emit_class_member_name_op(fc, prog, key, *computed, Op::DefineGetter);
                        }
                        MethodKind::Set => {
                            emit_class_member_name_op(fc, prog, key, *computed, Op::DefineSetter);
                        }
                        _ => {
                            emit_class_member_name_op(fc, prog, key, *computed, Op::DefineProperty);
                        }
                    }
                    // After DefineProperty/Getter/Setter: [ctor ctor_with_method]
                    fc.emit(Op::Pop); // [ctor]
                } else {
                    // Prototype method: define on constructor.prototype.
                    // Stack: [ctor]
                    fc.emit(Op::Dup); // [ctor ctor]
                    let proto_name = fc.add_name("prototype");
                    fc.emit_u16(Op::GetProp, proto_name); // [ctor proto]
                    fc.emit_u16(Op::Closure, const_idx); // [ctor proto method]
                    match kind {
                        MethodKind::Get => {
                            emit_class_member_name_op(fc, prog, key, *computed, Op::DefineGetter);
                        }
                        MethodKind::Set => {
                            emit_class_member_name_op(fc, prog, key, *computed, Op::DefineSetter);
                        }
                        _ => {
                            emit_class_member_name_op(fc, prog, key, *computed, Op::DefineProperty);
                        }
                    }
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
                let name_str = prog.interner.get(*name);

                if *is_static {
                    fc.emit(Op::Dup); // [ctor ctor]
                    fc.emit_u16(Op::Closure, const_idx); // [ctor ctor method]
                    let define_op = match kind {
                        MethodKind::Get => Op::DefineGetter,
                        MethodKind::Set => Op::DefineSetter,
                        _ => Op::DefineProperty,
                    };
                    let idx = fc.add_name(name_str);
                    fc.emit_u16(define_op, idx); // [ctor ctor]
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
                    let idx = fc.add_name(name_str);
                    fc.emit_u16(define_op, idx); // [ctor proto]
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
                    if let Some(val_expr) = value {
                        compile_expr(fc, prog, analysis, func_scopes, *val_expr)?;
                    } else {
                        fc.emit(Op::PushUndefined);
                    }
                    // [ctor ctor value]
                    emit_class_member_name_op(fc, prog, key, *computed, Op::DefineProperty);
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
                    let name_str = prog.interner.get(*name);
                    let idx = fc.add_name(name_str);
                    fc.emit_u16(Op::DefineProperty, idx); // [ctor ctor]
                    fc.emit(Op::Pop); // [ctor]
                }
            }
            ClassMemberKind::StaticBlock(stmts) => {
                // Compile static block statements. They execute with `this` = class.
                // For simplicity, just compile the statements inline.
                for &stmt_id in stmts {
                    super::stmt::compile_stmt(fc, prog, analysis, func_scopes, stmt_id)?;
                }
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

/// Emit a DefineProperty/DefineGetter/DefineSetter with the appropriate key.
///
/// For computed keys, falls back to `DefineComputedProperty`.
fn emit_class_member_name_op(
    fc: &mut FunctionCompiler,
    prog: &Program,
    key: &PropertyKey,
    computed: bool,
    op: Op,
) {
    if computed {
        // For computed keys, we would need to evaluate the key expression first.
        // For now, use a placeholder. DefineComputedProperty only works for Init.
        // Since we don't support computed method keys yet, just use DefineProperty
        // with an empty name as a fallback.
        let idx = fc.add_name("");
        fc.emit_u16(op, idx);
    } else {
        match key {
            PropertyKey::Identifier(name) | PropertyKey::PrivateIdentifier(name) => {
                let name_str = prog.interner.get(*name);
                let idx = fc.add_name(name_str);
                fc.emit_u16(op, idx);
            }
            PropertyKey::Literal(Literal::String(s)) => {
                let name_str = prog.interner.get(*s);
                let idx = fc.add_name(name_str);
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
                let idx = fc.add_name("");
                fc.emit_u16(op, idx);
            }
        }
    }
}

// ── Nested function compilation ────────────────────────────────────

/// Find the `func_scopes` index for a function/arrow with the given span.
///
/// Searches `analysis.scopes` for a `Function` scope whose span matches,
/// then maps that scope index back to its owning `func_scopes` entry.
fn find_func_scope_for_span(
    analysis: &ScopeAnalysis,
    func_scopes: &[FunctionScope],
    span: Span,
) -> Option<usize> {
    for (scope_idx, scope) in analysis.scopes.iter().enumerate() {
        if scope.kind == ScopeKind::Function && scope.span == span {
            for (fi, fs) in func_scopes.iter().enumerate() {
                if fs.scope_indices.first() == Some(&scope_idx) {
                    return Some(fi);
                }
            }
        }
    }
    None
}

/// Compile a nested `Function` (declaration or expression) into a `CompiledFunction`.
///
/// This is `pub(super)` so that `stmt.rs` can call it for `FunctionDeclaration`.
pub(super) fn compile_nested_function(
    _parent_fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    func: &Function,
    _is_declaration: bool,
) -> Result<crate::bytecode::compiled::CompiledFunction, CompileError> {
    let child_func_idx =
        find_func_scope_for_span(analysis, func_scopes, func.span).ok_or_else(|| CompileError {
            message: format!("no function scope found for span {:?}", func.span),
        })?;

    let root_scope_idx = func_scopes[child_func_idx].scope_indices[0];
    let is_strict = analysis.scopes[root_scope_idx].is_strict;

    let mut child_fc = FunctionCompiler::new(child_func_idx, root_scope_idx, is_strict);
    child_fc.name = func.name.map(|a| prog.interner.get(a).to_string());
    child_fc.is_async = func.is_async;
    child_fc.is_generator = func.is_generator;

    // Initialize var-declared locals to undefined (same pattern as top-level in mod.rs).
    let mut var_slots: Vec<u16> = func_scopes[child_func_idx]
        .locals
        .values()
        .filter(|info| matches!(info.kind, BindingKind::Var | BindingKind::Function))
        .map(|info| info.slot)
        .collect();
    var_slots.sort_unstable();
    // Skip param slots (they are filled by the caller).
    let param_count = func.params.len() as u16;
    for slot in var_slots {
        if slot < param_count {
            continue;
        }
        child_fc.emit(Op::PushUndefined);
        child_fc.emit_u16(Op::SetLocal, slot);
        child_fc.emit(Op::Pop);
    }

    // Compile default parameter values.
    for (i, param) in func.params.iter().enumerate() {
        if let Some(default_expr) = param.default {
            let slot = i as u16;
            child_fc.emit_u16(Op::GetLocal, slot);
            child_fc.emit(Op::PushUndefined);
            child_fc.emit(Op::StrictEq);
            let skip = child_fc.emit_jump(Op::JumpIfFalse);
            compile_expr(&mut child_fc, prog, analysis, func_scopes, default_expr)?;
            child_fc.emit_u16(Op::SetLocal, slot);
            child_fc.emit(Op::Pop);
            child_fc.patch_jump(skip);
        }
    }

    // Compile body statements.
    for &stmt_id in &func.body {
        super::stmt::compile_stmt(&mut child_fc, prog, analysis, func_scopes, stmt_id)?;
    }

    // Ensure the function ends with a return.
    if child_fc.bytecode.last() != Some(&(Op::Return as u8))
        && child_fc.bytecode.last() != Some(&(Op::ReturnUndefined as u8))
    {
        child_fc.emit(Op::ReturnUndefined);
    }

    let mut compiled = child_fc.finish(&func_scopes[child_func_idx]);
    compiled.param_count = func.params.len() as u16;
    Ok(compiled)
}

/// Compile an arrow function expression into a `CompiledFunction`.
fn compile_arrow_function(
    _parent_fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    arrow: &ArrowFunction,
) -> Result<crate::bytecode::compiled::CompiledFunction, CompileError> {
    let child_func_idx =
        find_func_scope_for_span(analysis, func_scopes, arrow.span).ok_or_else(|| {
            CompileError {
                message: format!("no function scope found for arrow span {:?}", arrow.span),
            }
        })?;

    let root_scope_idx = func_scopes[child_func_idx].scope_indices[0];
    let is_strict = analysis.scopes[root_scope_idx].is_strict;

    let mut child_fc = FunctionCompiler::new(child_func_idx, root_scope_idx, is_strict);
    child_fc.is_arrow = true;
    child_fc.is_async = arrow.is_async;

    // Initialize var-declared locals to undefined, skipping params.
    let mut var_slots: Vec<u16> = func_scopes[child_func_idx]
        .locals
        .values()
        .filter(|info| matches!(info.kind, BindingKind::Var | BindingKind::Function))
        .map(|info| info.slot)
        .collect();
    var_slots.sort_unstable();
    let param_count = arrow.params.len() as u16;
    for slot in var_slots {
        if slot < param_count {
            continue;
        }
        child_fc.emit(Op::PushUndefined);
        child_fc.emit_u16(Op::SetLocal, slot);
        child_fc.emit(Op::Pop);
    }

    // Compile default parameter values.
    for (i, param) in arrow.params.iter().enumerate() {
        if let Some(default_expr) = param.default {
            let slot = i as u16;
            child_fc.emit_u16(Op::GetLocal, slot);
            child_fc.emit(Op::PushUndefined);
            child_fc.emit(Op::StrictEq);
            let skip = child_fc.emit_jump(Op::JumpIfFalse);
            compile_expr(&mut child_fc, prog, analysis, func_scopes, default_expr)?;
            child_fc.emit_u16(Op::SetLocal, slot);
            child_fc.emit(Op::Pop);
            child_fc.patch_jump(skip);
        }
    }

    match &arrow.body {
        ArrowBody::Expression(expr_id) => {
            compile_expr(&mut child_fc, prog, analysis, func_scopes, *expr_id)?;
            child_fc.emit(Op::Return); // implicit return of expression value
        }
        ArrowBody::Block(stmts) => {
            for &stmt_id in stmts {
                super::stmt::compile_stmt(&mut child_fc, prog, analysis, func_scopes, stmt_id)?;
            }
            // Ensure block-body arrow ends with return.
            if child_fc.bytecode.last() != Some(&(Op::Return as u8))
                && child_fc.bytecode.last() != Some(&(Op::ReturnUndefined as u8))
            {
                child_fc.emit(Op::ReturnUndefined);
            }
        }
    }

    let mut compiled = child_fc.finish(&func_scopes[child_func_idx]);
    compiled.param_count = arrow.params.len() as u16;
    Ok(compiled)
}
