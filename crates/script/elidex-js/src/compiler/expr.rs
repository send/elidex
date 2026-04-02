//! Expression compilation: ExprKind → bytecode.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::atom::Atom;
use crate::bytecode::compiled::Constant;
use crate::bytecode::opcode::Op;

use super::function::FunctionCompiler;
use super::resolve::{resolve_identifier, FunctionScope, VarLocation};
use crate::scope::{BindingKind, ScopeAnalysis};

/// Compile an expression, leaving its value on the stack.
#[allow(clippy::too_many_lines)]
pub fn compile_expr(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    expr_id: NodeId<Expr>,
) {
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
            compile_expr(fc, prog, analysis, func_scopes, *left);
            compile_expr(fc, prog, analysis, func_scopes, *right);
            fc.emit(binary_op_to_opcode(*op));
        }

        ExprKind::Unary { op, argument } => {
            if matches!(op, UnaryOp::Typeof) {
                // typeof on unresolved global should not throw.
                let arg = prog.exprs.get(*argument);
                if let ExprKind::Identifier(atom) = &arg.kind {
                    let loc = resolve_identifier(*atom, fc.func_scope_idx, func_scopes, analysis);
                    if loc == VarLocation::Global {
                        let name = prog.interner.get(*atom);
                        let idx = fc.add_name(name);
                        fc.emit_u16(Op::TypeOfGlobal, idx);
                        return;
                    }
                }
            }
            compile_expr(fc, prog, analysis, func_scopes, *argument);
            fc.emit(unary_op_to_opcode(*op));
        }

        ExprKind::Logical { left, op, right } => {
            compile_expr(fc, prog, analysis, func_scopes, *left);
            let jump_op = match op {
                LogicalOp::And => Op::JumpIfFalse,
                LogicalOp::Or => Op::JumpIfTrue,
                LogicalOp::NullCoal => Op::JumpIfNotNullish,
            };
            // Short-circuit: duplicate TOS, conditional jump over RHS.
            fc.emit(Op::Dup);
            let patch = fc.emit_jump(jump_op);
            fc.emit(Op::Pop); // discard LHS
            compile_expr(fc, prog, analysis, func_scopes, *right);
            fc.patch_jump(patch);
        }

        ExprKind::Assignment { left, op, right } => {
            compile_assignment(fc, prog, analysis, func_scopes, left, *op, *right);
        }

        ExprKind::Conditional {
            test,
            consequent,
            alternate,
        } => {
            compile_expr(fc, prog, analysis, func_scopes, *test);
            let else_patch = fc.emit_jump(Op::JumpIfFalse);
            compile_expr(fc, prog, analysis, func_scopes, *consequent);
            let end_patch = fc.emit_jump(Op::Jump);
            fc.patch_jump(else_patch);
            compile_expr(fc, prog, analysis, func_scopes, *alternate);
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
                compile_expr(fc, prog, analysis, func_scopes, *object);
                fc.emit(Op::Dup); // keep receiver for CallMethod
                compile_member_property(fc, prog, analysis, func_scopes, property, *computed);
                compile_arguments(fc, prog, analysis, func_scopes, arguments);
                let argc = arguments.len() as u8;
                fc.emit_u8(Op::CallMethod, argc);
            } else {
                compile_expr(fc, prog, analysis, func_scopes, *callee);
                compile_arguments(fc, prog, analysis, func_scopes, arguments);
                let argc = arguments.len() as u8;
                fc.emit_u8(Op::Call, argc);
            }
        }

        ExprKind::New { callee, arguments } => {
            compile_expr(fc, prog, analysis, func_scopes, *callee);
            compile_arguments(fc, prog, analysis, func_scopes, arguments);
            let argc = arguments.len() as u8;
            fc.emit_u8(Op::New, argc);
        }

        ExprKind::Member {
            object,
            property,
            computed,
        } => {
            compile_expr(fc, prog, analysis, func_scopes, *object);
            compile_member_property(fc, prog, analysis, func_scopes, property, *computed);
        }

        ExprKind::Array(elements) => {
            fc.emit(Op::CreateArray);
            for elem in elements {
                match elem {
                    Some(ArrayElement::Expression(e)) => {
                        compile_expr(fc, prog, analysis, func_scopes, *e);
                        fc.emit(Op::ArrayPush);
                    }
                    Some(ArrayElement::Spread(e)) => {
                        compile_expr(fc, prog, analysis, func_scopes, *e);
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
                    if let Some(value) = prop.value {
                        compile_expr(fc, prog, analysis, func_scopes, value);
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
                                compile_expr(fc, prog, analysis, func_scopes, *e);
                            }
                        }
                        // Then evaluate value.
                        if let Some(value) = prop.value {
                            compile_expr(fc, prog, analysis, func_scopes, value);
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
                        compile_accessor(fc, prog, analysis, func_scopes, prop, Op::DefineGetter);
                    }
                    PropertyKind::Set => {
                        compile_accessor(fc, prog, analysis, func_scopes, prop, Op::DefineSetter);
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
                    compile_expr(fc, prog, analysis, func_scopes, tpl.expressions[i]);
                }
            }
            fc.emit_u16(Op::TemplateConcat, total as u16);
        }

        ExprKind::Sequence(exprs) => {
            for (i, &e) in exprs.iter().enumerate() {
                compile_expr(fc, prog, analysis, func_scopes, e);
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
                let loc = resolve_identifier(*atom, fc.func_scope_idx, func_scopes, analysis);
                match loc {
                    VarLocation::Local(slot) => {
                        let inc_op = match op {
                            UpdateOp::Increment => Op::IncLocal,
                            UpdateOp::Decrement => Op::DecLocal,
                        };
                        fc.emit_u16_u8(inc_op, slot, u8::from(*prefix));
                        return;
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
                        return;
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
                        return;
                    }
                    VarLocation::Module(_) => {
                        // Module imports are immutable — fall through to push undefined.
                    }
                }
            }
            // Fallback: member expression updates (++obj.prop, obj[key]++).
            // TODO: emit proper GetProp/SetProp sequence for member expressions.
            // For now, compile the argument for its side effects and keep stack balanced.
            compile_expr(fc, prog, analysis, func_scopes, *argument);
        }

        ExprKind::Await(arg) => {
            compile_expr(fc, prog, analysis, func_scopes, *arg);
            fc.emit(Op::Await);
        }

        ExprKind::Yield { argument, delegate } => {
            if let Some(arg) = argument {
                compile_expr(fc, prog, analysis, func_scopes, *arg);
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
            compile_expr(fc, prog, analysis, func_scopes, *inner);
        }

        ExprKind::Paren(inner) => {
            compile_expr(fc, prog, analysis, func_scopes, *inner);
        }

        ExprKind::Super | ExprKind::MetaProperty(_) | ExprKind::DynamicImport { .. } => {
            // Stubs for complex features.
            fc.emit(Op::PushUndefined);
        }

        // Function/Arrow/Class expressions compiled as nested functions.
        ExprKind::Function(_)
        | ExprKind::Arrow(_)
        | ExprKind::Class(_)
        | ExprKind::TaggedTemplate { .. }
        | ExprKind::OptionalChain { .. }
        | ExprKind::PrivateIn { .. } => {
            // TODO: implement in subsequent steps
            fc.emit(Op::PushUndefined);
        }

        ExprKind::Error => fc.emit(Op::PushUndefined),
    }
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
    let loc = resolve_identifier(atom, fc.func_scope_idx, func_scopes, analysis);
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
) {
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
                        compile_expr(fc, prog, analysis, func_scopes, right);
                        compile_identifier_store(fc, prog, analysis, func_scopes, *atom);
                        fc.patch_jump(patch);
                        return;
                    }

                    if op != AssignOp::Assign {
                        // Compound: load current value first.
                        compile_identifier_load(fc, prog, analysis, func_scopes, *atom);
                    }
                    compile_expr(fc, prog, analysis, func_scopes, right);
                    if op != AssignOp::Assign {
                        fc.emit(compound_op_to_opcode(op));
                    }
                    compile_identifier_store(fc, prog, analysis, func_scopes, *atom);
                }
                ExprKind::Member {
                    object,
                    property,
                    computed,
                } => {
                    if *computed {
                        // Computed member assignment: obj[key] = value
                        // SetElem expects [object key value -- value]
                        compile_expr(fc, prog, analysis, func_scopes, *object);
                        if let MemberProp::Expression(key_expr) = property {
                            compile_expr(fc, prog, analysis, func_scopes, *key_expr);
                        }
                        // Compound computed assignment (obj[key] += val) requires
                        // preserving object+key while loading the old value. Not yet
                        // supported — reject to avoid miscompilation.
                        assert!(
                            op == AssignOp::Assign,
                            "compound assignments to computed members are not yet supported"
                        );
                        compile_expr(fc, prog, analysis, func_scopes, right);
                        fc.emit(Op::SetElem);
                    } else {
                        // Named property assignment: obj.prop = value
                        // SetProp expects [object value -- value]
                        compile_expr(fc, prog, analysis, func_scopes, *object);
                        if op != AssignOp::Assign {
                            fc.emit(Op::Dup);
                            compile_member_property(
                                fc,
                                prog,
                                analysis,
                                func_scopes,
                                property,
                                false,
                            );
                        }
                        compile_expr(fc, prog, analysis, func_scopes, right);
                        if op != AssignOp::Assign {
                            fc.emit(compound_op_to_opcode(op));
                        }
                        if let MemberProp::Identifier(name) = property {
                            let name_str = prog.interner.get(*name);
                            let idx = fc.add_name(name_str);
                            fc.emit_u16(Op::SetProp, idx);
                        }
                    }
                }
                _ => {
                    compile_expr(fc, prog, analysis, func_scopes, right);
                }
            }
        }
        AssignTarget::Pattern(_pattern_id) => {
            // Destructuring assignment not yet implemented — pop RHS to keep
            // stack balanced and fail explicitly.
            compile_expr(fc, prog, analysis, func_scopes, right);
            fc.emit(Op::Pop);
        }
    }
}

/// Compile an identifier store (write).
fn compile_identifier_store(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    atom: Atom,
) {
    let loc = resolve_identifier(atom, fc.func_scope_idx, func_scopes, analysis);
    match loc {
        VarLocation::Local(slot) => {
            if let Some(info) = func_scopes[fc.func_scope_idx].get_local(atom) {
                // Check for const assignment (ES2020 §13.15.2 — TypeError).
                assert!(
                    info.kind != BindingKind::Const,
                    "assignment to constant variable"
                );
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
}

/// Compile a member property access (after object is on stack).
fn compile_member_property(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    property: &MemberProp,
    computed: bool,
) {
    match property {
        MemberProp::Identifier(name) if !computed => {
            let name_str = prog.interner.get(*name);
            let idx = fc.add_name(name_str);
            fc.emit_u16(Op::GetProp, idx);
        }
        MemberProp::Expression(e) => {
            compile_expr(fc, prog, analysis, func_scopes, *e);
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
}

/// Compile a getter or setter property definition.
fn compile_accessor(
    fc: &mut FunctionCompiler,
    prog: &Program,
    analysis: &ScopeAnalysis,
    func_scopes: &mut [FunctionScope],
    prop: &Property,
    define_op: Op,
) {
    if let Some(value) = prop.value {
        compile_expr(fc, prog, analysis, func_scopes, value);
        match &prop.key {
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
) {
    assert!(
        arguments.len() <= 255,
        "too many arguments ({}) — maximum 255 supported",
        arguments.len()
    );
    for arg in arguments {
        match arg {
            Argument::Expression(e) => compile_expr(fc, prog, analysis, func_scopes, *e),
            Argument::Spread(e) => {
                // Spread arguments are not yet supported — the expression is
                // compiled as a normal argument. The stack remains balanced since
                // the spread expression produces one value, matching the argc count.
                compile_expr(fc, prog, analysis, func_scopes, *e);
            }
        }
    }
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
