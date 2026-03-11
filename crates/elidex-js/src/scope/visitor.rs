//! AST visitor free functions for scope analysis.
//!
//! All visitor functions take `(&Program, &mut ScopeState)` separately,
//! allowing shared borrows on AST nodes while mutating scope state.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::span::Span;

use super::{has_use_strict, BindingKind, ScopeKind, ScopeState};

/// Visit a statement for scope analysis.
#[allow(clippy::too_many_lines)]
pub(super) fn visit_stmt(prog: &Program, state: &mut ScopeState, stmt_id: NodeId<Stmt>) {
    if !state.enter_recursion() {
        return;
    }

    let stmt = prog.stmts.get(stmt_id);
    let span = stmt.span;

    match &stmt.kind {
        StmtKind::Error
        | StmtKind::Empty
        | StmtKind::Debugger
        | StmtKind::Break(_)
        | StmtKind::Continue(_) => {}

        StmtKind::Expression(e) | StmtKind::Throw(e) => visit_expr(prog, state, *e),
        StmtKind::Return(arg) => {
            if let Some(a) = arg {
                visit_expr(prog, state, *a);
            }
        }

        StmtKind::If {
            test,
            consequent,
            alternate,
        } => {
            visit_expr(prog, state, *test);
            visit_stmt(prog, state, *consequent);
            if let Some(alt) = alternate {
                visit_stmt(prog, state, *alt);
            }
        }

        StmtKind::While { test, body } | StmtKind::DoWhile { body, test } => {
            visit_expr(prog, state, *test);
            visit_stmt(prog, state, *body);
        }
        StmtKind::With { object, body } => {
            visit_expr(prog, state, *object);
            visit_stmt(prog, state, *body);
        }
        StmtKind::Labeled { body, .. } => visit_stmt(prog, state, *body),

        StmtKind::VariableDeclaration { kind, declarators } => {
            let binding_kind = BindingKind::from(*kind);
            for d in declarators {
                visit_pattern_binding(prog, state, d.pattern, binding_kind);
                if let Some(i) = d.init {
                    visit_expr(prog, state, i);
                }
            }
        }

        StmtKind::Block(body) => {
            state.push_scope(ScopeKind::Block, state.is_strict(), span);
            for &s in body {
                visit_stmt(prog, state, s);
            }
            state.pop_scope();
        }

        StmtKind::For {
            init,
            test,
            update,
            body,
        } => {
            state.push_scope(ScopeKind::Block, state.is_strict(), span);
            if let Some(init) = init {
                match init {
                    ForInit::Declaration { kind, declarators } => {
                        let bk = BindingKind::from(*kind);
                        for d in declarators {
                            visit_pattern_binding(prog, state, d.pattern, bk);
                            if let Some(i) = d.init {
                                visit_expr(prog, state, i);
                            }
                        }
                    }
                    ForInit::Expression(e) => visit_expr(prog, state, *e),
                }
            }
            if let Some(t) = test {
                visit_expr(prog, state, *t);
            }
            if let Some(u) = update {
                visit_expr(prog, state, *u);
            }
            visit_stmt(prog, state, *body);
            state.pop_scope();
        }

        StmtKind::ForIn { left, right, body }
        | StmtKind::ForOf {
            left, right, body, ..
        } => {
            state.push_scope(ScopeKind::Block, state.is_strict(), span);
            match left {
                ForInOfLeft::Declaration { kind, pattern } => {
                    visit_pattern_binding(prog, state, *pattern, (*kind).into());
                }
                ForInOfLeft::Pattern(e) => visit_expr(prog, state, *e),
            }
            visit_expr(prog, state, *right);
            visit_stmt(prog, state, *body);
            state.pop_scope();
        }

        StmtKind::Switch {
            discriminant,
            cases,
        } => {
            visit_expr(prog, state, *discriminant);
            state.push_scope(ScopeKind::Block, state.is_strict(), span);
            for case in cases {
                if let Some(t) = case.test {
                    visit_expr(prog, state, t);
                }
                for &s in &case.consequent {
                    visit_stmt(prog, state, s);
                }
            }
            state.pop_scope();
        }

        StmtKind::Try {
            block,
            handler,
            finalizer,
        } => {
            state.push_scope(ScopeKind::Block, state.is_strict(), span);
            for &s in block {
                visit_stmt(prog, state, s);
            }
            state.pop_scope();

            if let Some(h) = handler {
                state.push_scope(ScopeKind::Catch, state.is_strict(), h.span);
                if let Some(p) = h.param {
                    visit_pattern_binding(prog, state, p, BindingKind::CatchParam);
                }
                for &s in &h.body {
                    visit_stmt(prog, state, s);
                }
                state.pop_scope();
            }

            if let Some(f) = finalizer {
                state.push_scope(ScopeKind::Block, state.is_strict(), span);
                for &s in f {
                    visit_stmt(prog, state, s);
                }
                state.pop_scope();
            }
        }

        StmtKind::ImportDeclaration(decl) => {
            let scope = state.current_scope();
            for spec in &decl.specifiers {
                let (name, spec_span) = match spec {
                    ImportSpecifier::Default(n, s) | ImportSpecifier::Namespace(n, s) => (*n, *s),
                    ImportSpecifier::Named { local, span, .. } => (*local, *span),
                };
                state.add_binding(prog, scope, name, BindingKind::Import, spec_span, false);
            }
        }

        StmtKind::ExportDeclaration(decl) => {
            visit_export(prog, state, decl, span);
        }

        StmtKind::FunctionDeclaration(func) => {
            bind_and_visit_function(prog, state, func, span);
        }
        StmtKind::ClassDeclaration(class) => {
            bind_and_visit_class(prog, state, class, span);
        }
    }

    state.leave_recursion();
}

/// Visit an export declaration.
fn visit_export(prog: &Program, state: &mut ScopeState, decl: &ExportDecl, span: Span) {
    match &decl.kind {
        ExportKind::Declaration(s) => {
            register_export_declaration_names(prog, state, *s);
            visit_stmt(prog, state, *s);
        }
        ExportKind::Default(e) => {
            state.check_duplicate_export(prog, state.default_atom, span);
            visit_expr(prog, state, *e);
        }
        ExportKind::DefaultFunction(f) => {
            state.check_duplicate_export(prog, state.default_atom, span);
            bind_and_visit_function(prog, state, f, span);
        }
        ExportKind::DefaultClass(c) => {
            state.check_duplicate_export(prog, state.default_atom, span);
            bind_and_visit_class(prog, state, c, span);
        }
        ExportKind::Named { specifiers, .. } => {
            for s in specifiers {
                state.check_duplicate_export(prog, s.exported, s.span);
            }
        }
        ExportKind::AllFrom { .. } => {}
        ExportKind::NamespaceFrom { exported, .. } => {
            state.check_duplicate_export(prog, *exported, span);
        }
    }
}

/// Visit an expression for scope analysis.
#[allow(clippy::too_many_lines)]
pub(super) fn visit_expr(prog: &Program, state: &mut ScopeState, expr_id: NodeId<Expr>) {
    if !state.enter_recursion() {
        return;
    }

    let expr = prog.exprs.get(expr_id);
    match &expr.kind {
        // M3: §15.7.14 — `arguments` is a Syntax Error in ClassStaticBlock
        ExprKind::Identifier(name) if *name == prog.atoms.arguments => {
            if !state.at_error_limit() {
                for &idx in state.scope_stack.iter().rev() {
                    match state.scopes[idx].kind {
                        ScopeKind::StaticBlock => {
                            state.errors.push(crate::error::JsParseError {
                                kind: crate::error::JsParseErrorKind::UnexpectedToken,
                                span: expr.span,
                                message: "'arguments' is not allowed in class static blocks".into(),
                            });
                            break;
                        }
                        // Stop at function boundary — function has its own `arguments`
                        ScopeKind::Function => break,
                        _ => {}
                    }
                }
            }
        }

        ExprKind::Error
        | ExprKind::Identifier(_)
        | ExprKind::Literal(_)
        | ExprKind::This
        | ExprKind::Super
        | ExprKind::MetaProperty(_) => {}

        ExprKind::Unary { argument, .. }
        | ExprKind::Update { argument, .. }
        | ExprKind::Await(argument)
        | ExprKind::Spread(argument)
        | ExprKind::Paren(argument)
        | ExprKind::PrivateIn {
            right: argument, ..
        } => visit_expr(prog, state, *argument),

        ExprKind::Yield { argument, .. } => {
            if let Some(a) = argument {
                visit_expr(prog, state, *a);
            }
        }

        ExprKind::Binary { left, right, .. } | ExprKind::Logical { left, right, .. } => {
            visit_expr(prog, state, *left);
            visit_expr(prog, state, *right);
        }

        ExprKind::DynamicImport { source, options } => {
            visit_expr(prog, state, *source);
            if let Some(o) = options {
                visit_expr(prog, state, *o);
            }
        }

        ExprKind::Conditional {
            test,
            consequent,
            alternate,
        } => {
            visit_expr(prog, state, *test);
            visit_expr(prog, state, *consequent);
            visit_expr(prog, state, *alternate);
        }

        ExprKind::Assignment { left, right, .. } => {
            if let AssignTarget::Simple(e) = left {
                visit_expr(prog, state, *e);
            }
            visit_expr(prog, state, *right);
        }

        ExprKind::Member {
            object, property, ..
        } => {
            visit_expr(prog, state, *object);
            if let MemberProp::Expression(e) = property {
                visit_expr(prog, state, *e);
            }
        }

        ExprKind::Array(elems) => {
            for elem in elems.iter().flatten() {
                match elem {
                    ArrayElement::Expression(e) | ArrayElement::Spread(e) => {
                        visit_expr(prog, state, *e);
                    }
                }
            }
        }

        ExprKind::Object(props) => {
            for prop in props {
                if let PropertyKey::Computed(e) = &prop.key {
                    visit_expr(prog, state, *e);
                }
                if let Some(v) = prop.value {
                    visit_expr(prog, state, v);
                }
            }
        }

        ExprKind::Sequence(exprs) => {
            for &e in exprs {
                visit_expr(prog, state, e);
            }
        }

        ExprKind::Template(tl) => {
            for &e in &tl.expressions {
                visit_expr(prog, state, e);
            }
        }

        ExprKind::TaggedTemplate { tag, template } => {
            visit_expr(prog, state, *tag);
            for &e in &template.expressions {
                visit_expr(prog, state, e);
            }
        }

        ExprKind::OptionalChain { base, chain } => {
            visit_expr(prog, state, *base);
            for part in chain {
                match part {
                    OptionalChainPart::Member {
                        property: MemberProp::Expression(e),
                        ..
                    } => visit_expr(prog, state, *e),
                    OptionalChainPart::Call(args) => {
                        for arg in args {
                            match arg {
                                Argument::Expression(e) | Argument::Spread(e) => {
                                    visit_expr(prog, state, *e);
                                }
                            }
                        }
                    }
                    OptionalChainPart::Member { .. } => {}
                }
            }
        }

        ExprKind::Call { callee, arguments } | ExprKind::New { callee, arguments } => {
            visit_expr(prog, state, *callee);
            for arg in arguments {
                match arg {
                    Argument::Expression(e) | Argument::Spread(e) => {
                        visit_expr(prog, state, *e);
                    }
                }
            }
        }

        ExprKind::Function(f) => visit_function(prog, state, f, true),
        ExprKind::Class(c) => visit_class(prog, state, c),
        ExprKind::Arrow(arrow) => {
            state.push_scope(ScopeKind::Function, state.is_strict(), arrow.span);
            for param in &arrow.params {
                visit_pattern_binding(prog, state, param.pattern, BindingKind::Param);
                if let Some(d) = param.default {
                    visit_expr(prog, state, d);
                }
            }
            match &arrow.body {
                ArrowBody::Expression(e) => visit_expr(prog, state, *e),
                ArrowBody::Block(stmts) => {
                    for &s in stmts {
                        visit_stmt(prog, state, s);
                    }
                }
            }
            state.pop_scope();
        }
    }

    state.leave_recursion();
}

/// Bind a function declaration name in the current scope and visit its body.
fn bind_and_visit_function(prog: &Program, state: &mut ScopeState, func: &Function, span: Span) {
    if let Some(name) = func.name {
        let scope = state.current_scope();
        state.add_binding(prog, scope, name, BindingKind::Function, span, false);
    }
    visit_function(prog, state, func, false);
}

/// Bind a class declaration name in the current scope and visit its body.
fn bind_and_visit_class(prog: &Program, state: &mut ScopeState, class: &Class, span: Span) {
    if let Some(name) = class.name {
        let scope = state.current_scope();
        state.add_binding(prog, scope, name, BindingKind::Class, span, false);
    }
    visit_class(prog, state, class);
}

/// E13: `is_expression` controls inner name binding.
/// Function declarations bind the name in the outer scope (done by `visit_stmt`).
/// Function expressions bind the name in their own inner scope (for recursion).
fn visit_function(prog: &Program, state: &mut ScopeState, func: &Function, is_expression: bool) {
    let has_strict = has_use_strict(prog, &func.body);
    let is_strict = state.is_strict() || has_strict;

    // S4: "use strict" directive is illegal with non-simple parameters
    if has_strict
        && !state.is_strict()
        && has_non_simple_params(prog, func)
        && !state.at_error_limit()
    {
        state.errors.push(crate::error::JsParseError {
            kind: crate::error::JsParseErrorKind::StrictModeViolation,
            span: func.span,
            message: "\"use strict\" not allowed in function with non-simple parameters".into(),
        });
    }

    state.push_scope(ScopeKind::Function, is_strict, func.span);

    if is_expression {
        if let Some(name) = func.name {
            let scope = state.current_scope();
            state.add_binding(prog, scope, name, BindingKind::Function, func.span, false);
        }
    }

    // ES2023 §10.2.3: non-arrow functions have an implicit `arguments` binding.
    {
        let scope = state.current_scope();
        let arguments_atom = prog.interner.lookup("arguments");
        state.add_binding(
            prog,
            scope,
            arguments_atom,
            BindingKind::Implicit,
            func.span,
            false,
        );
    }

    for param in &func.params {
        visit_pattern_binding(prog, state, param.pattern, BindingKind::Param);
        if let Some(d) = param.default {
            visit_expr(prog, state, d);
        }
    }

    for &s in &func.body {
        visit_stmt(prog, state, s);
    }

    state.pop_scope();
}

fn visit_class(prog: &Program, state: &mut ScopeState, class: &Class) {
    // super_class is evaluated in outer scope
    if let Some(sc) = class.super_class {
        visit_expr(prog, state, sc);
    }

    // B26: create inner scope for class name (visible within class body).
    // B9: class bodies are always strict (ES2023 §15.7.1).
    state.push_scope(ScopeKind::Block, true, class.span);
    if let Some(name) = class.name {
        let scope = state.current_scope();
        state.add_binding(prog, scope, name, BindingKind::Class, class.span, false);
    }
    visit_class_body(prog, state, &class.body);
    state.pop_scope();
}

fn visit_class_body(prog: &Program, state: &mut ScopeState, body: &[ClassMember]) {
    for member in body {
        match &member.kind {
            ClassMemberKind::Method { function: f, .. }
            | ClassMemberKind::PrivateMethod { function: f, .. } => {
                visit_function(prog, state, f, false);
            }
            ClassMemberKind::Property { value: Some(v), .. }
            | ClassMemberKind::PrivateField { value: Some(v), .. } => {
                visit_expr(prog, state, *v);
            }
            ClassMemberKind::StaticBlock(stmts) => {
                // B9: class body is always strict
                state.push_scope(ScopeKind::StaticBlock, true, member.span);
                for &s in stmts {
                    visit_stmt(prog, state, s);
                }
                state.pop_scope();
            }
            _ => {}
        }
    }
}

/// S4: Check if a function has non-simple parameters (destructuring, default, rest).
fn has_non_simple_params(prog: &Program, func: &Function) -> bool {
    func.params.iter().any(|p| {
        p.rest
            || p.default.is_some()
            || !matches!(
                prog.patterns.get(p.pattern).kind,
                PatternKind::Identifier(_)
            )
    })
}

/// Visit a pattern for binding registration.
fn visit_pattern_binding(
    prog: &Program,
    state: &mut ScopeState,
    pat_id: NodeId<Pattern>,
    kind: BindingKind,
) {
    // S4: depth guard for recursive pattern visiting
    if !state.enter_recursion() {
        return;
    }

    let pat = prog.patterns.get(pat_id);
    match &pat.kind {
        PatternKind::Error => {}
        PatternKind::Identifier(name) => {
            let name = *name;
            let pat_span = pat.span;
            let scope = if kind == BindingKind::Var {
                let vs = state.var_scope();
                // S7: var declarations must not shadow catch parameters when hoisted
                state.check_var_catch_conflict(prog, name, pat_span, vs);
                vs
            } else {
                state.current_scope()
            };
            state.add_binding(prog, scope, name, kind, pat_span, kind == BindingKind::Var);
        }
        PatternKind::Expression(e) => visit_expr(prog, state, *e),
        PatternKind::Assign { left, right } => {
            let left = *left;
            let right = *right;
            visit_pattern_binding(prog, state, left, kind);
            visit_expr(prog, state, right);
        }
        PatternKind::Array { elements, rest } => {
            for ae in elements.iter().flatten() {
                visit_pattern_binding(prog, state, ae.pattern, kind);
                if let Some(d) = ae.default {
                    visit_expr(prog, state, d);
                }
            }
            if let Some(r) = rest {
                visit_pattern_binding(prog, state, *r, kind);
            }
        }
        PatternKind::Object { properties, rest } => {
            for prop in properties {
                // M2: visit computed key expressions
                if let PropertyKey::Computed(e) = &prop.key {
                    visit_expr(prog, state, *e);
                }
                visit_pattern_binding(prog, state, prop.value, kind);
            }
            if let Some(r) = rest {
                visit_pattern_binding(prog, state, *r, kind);
            }
        }
    }

    state.leave_recursion();
}

/// A2: Register all declared names from `export const/let/var/function/class` as exported.
fn register_export_declaration_names(
    prog: &Program,
    state: &mut ScopeState,
    stmt_id: NodeId<Stmt>,
) {
    let stmt = prog.stmts.get(stmt_id);
    match &stmt.kind {
        StmtKind::VariableDeclaration { declarators, .. } => {
            for d in declarators {
                collect_pattern_export_names(prog, state, d.pattern, stmt.span);
            }
        }
        StmtKind::FunctionDeclaration(f) => {
            if let Some(n) = f.name {
                state.check_duplicate_export(prog, n, stmt.span);
            }
        }
        StmtKind::ClassDeclaration(c) => {
            if let Some(n) = c.name {
                state.check_duplicate_export(prog, n, stmt.span);
            }
        }
        _ => {}
    }
}

/// R7: Collect binding names from a pattern and register as exports.
fn collect_pattern_export_names(
    prog: &Program,
    state: &mut ScopeState,
    pat_id: NodeId<Pattern>,
    span: Span,
) {
    let mut names = Vec::new();
    crate::parser::Parser::collect_binding_names(&prog.patterns, pat_id, &mut names);
    for name in names {
        state.check_duplicate_export(prog, name, span);
    }
}
