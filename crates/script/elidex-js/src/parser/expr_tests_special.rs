use crate::ast::*;
use crate::{parse_module, parse_script};

use super::expr_tests_core::parse_expr;

// ── L11: super expression ──

#[test]
fn super_member_access() {
    let out = parse_script("class C extends B { constructor() { super.method(); } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── import.meta and new.target ──

#[test]
fn import_meta() {
    let out = parse_module("import.meta.url;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn new_target() {
    let out = parse_script("function f() { new.target; }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── Dynamic import ──

#[test]
fn dynamic_import() {
    let (prog, e) = parse_expr("import('module')");
    assert!(matches!(
        prog.exprs.get(e).kind,
        ExprKind::DynamicImport { .. }
    ));
}

// ── B21: top-level await ──

#[test]
fn top_level_await_module_ok() {
    let out = parse_module("await fetch('url');");
    // In module mode, await should parse as AwaitExpression
    assert!(out.errors.is_empty(), "Errors: {:?}", out.errors);
    let stmt = &out.program.stmts.get(out.program.body[0]);
    if let StmtKind::Expression(e) = &stmt.kind {
        assert!(
            matches!(out.program.exprs.get(*e).kind, ExprKind::Await(_)),
            "Expected await expression, got {:?}",
            out.program.exprs.get(*e).kind
        );
    } else {
        panic!("Expected expression statement");
    }
}

#[test]
fn top_level_await_script_is_identifier() {
    use super::expr_tests_core::r;
    // In script mode (outside async), `await` is just an identifier
    let out = parse_script("await;");
    assert!(out.errors.is_empty(), "Errors: {:?}", out.errors);
    let stmt = &out.program.stmts.get(out.program.body[0]);
    if let StmtKind::Expression(e) = &stmt.kind {
        assert!(
            matches!(out.program.exprs.get(*e).kind, ExprKind::Identifier(name) if r(&out.program, name) == "await"),
            "Expected identifier 'await', got {:?}",
            out.program.exprs.get(*e).kind
        );
    } else {
        panic!("Expected expression statement");
    }
}

// ── B7: regexp/division ambiguity ──

#[test]
fn regexp_after_block_statement() {
    let out = parse_script("{} /abc/g;");
    assert!(out.errors.is_empty(), "Errors: {:?}", out.errors);
    // The second statement should contain a RegExp literal
    let stmt = &out.program.stmts.get(out.program.body[1]);
    if let StmtKind::Expression(e) = &stmt.kind {
        assert!(
            matches!(
                out.program.exprs.get(*e).kind,
                ExprKind::Literal(Literal::RegExp { .. })
            ),
            "Expected RegExp literal"
        );
    } else {
        panic!("Expected expression statement");
    }
}

#[test]
fn regexp_after_if_block() {
    let out = parse_script("if (true) {} /re/;");
    assert!(out.errors.is_empty(), "Errors: {:?}", out.errors);
}

#[test]
fn division_after_object_literal() {
    let out = parse_script("x = {} / 2;");
    // `{}` is an empty block, then `/2/` would be a regexp... but actually
    // `x = {}` makes `{}` an object literal.  The `/` after is division.
    // This test verifies no crash and some output.
    assert!(!out.program.body.is_empty());
}

// ── B1: import.meta module-only ──

#[test]
fn import_meta_in_module_ok() {
    let out = parse_module("import.meta.url;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn import_meta_in_script_error() {
    let out = parse_script("import.meta.url;");
    assert!(
        out.errors.iter().any(|e| e.message.contains("module")),
        "Expected module-only error: {:?}",
        out.errors
    );
}

// ── B2: new.target function-only ──

#[test]
fn new_target_in_function_ok() {
    let out = parse_script("function f() { new.target; }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn new_target_at_top_level_error() {
    let out = parse_script("new.target;");
    assert!(
        out.errors.iter().any(|e| e.message.contains("function")),
        "Expected function-only error: {:?}",
        out.errors
    );
}

// ── B5: #x in obj private field membership ──

#[test]
fn private_field_in_ok() {
    let out = parse_script("class C { #x; method(obj) { return #x in obj; } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn private_identifier_without_in_error() {
    let out = parse_script("function f() { return #x; }");
    assert!(
        !out.errors.is_empty(),
        "Expected error for lone private identifier"
    );
}

// ── Coverage: yield expressions ──

#[test]
fn yield_expression() {
    let out = parse_script("function* g() { yield 1; }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::FunctionDeclaration(f) = &out.program.stmts.get(out.program.body[0]).kind {
        let body_stmt = out.program.stmts.get(f.body[0]);
        if let StmtKind::Expression(e) = &body_stmt.kind {
            assert!(
                matches!(
                    out.program.exprs.get(*e).kind,
                    ExprKind::Yield {
                        delegate: false,
                        ..
                    }
                ),
                "Expected Yield expression"
            );
        } else {
            panic!("Expected expression statement");
        }
    }
}

#[test]
fn yield_delegate_expression() {
    let out = parse_script("function* g() { yield* other(); }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::FunctionDeclaration(f) = &out.program.stmts.get(out.program.body[0]).kind {
        let body_stmt = out.program.stmts.get(f.body[0]);
        if let StmtKind::Expression(e) = &body_stmt.kind {
            assert!(
                matches!(
                    out.program.exprs.get(*e).kind,
                    ExprKind::Yield { delegate: true, .. }
                ),
                "Expected Yield* expression"
            );
        } else {
            panic!("Expected expression statement");
        }
    }
}

// ── T3: v flag (unicodeSets) ──

#[test]
fn regexp_v_flag_accepted() {
    use crate::regexp::parse_flags;
    let result = parse_flags("v");
    assert!(result.is_ok(), "v flag should be accepted");
    assert!(result.unwrap().unicode_sets);
}

#[test]
fn regexp_uv_flags_mutually_exclusive() {
    use crate::regexp::parse_flags;
    let result = parse_flags("uv");
    assert!(result.is_err(), "u+v should be rejected");
    assert!(result.unwrap_err().message.contains("mutually exclusive"));
}

// ── T1: await in async function default params ──

#[test]
fn await_in_async_default_param_error() {
    let out = parse_script("async function f(x = await 1) {}");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("await") && e.message.contains("parameter")),
        "Expected await-in-params error: {:?}",
        out.errors
    );
}

#[test]
fn await_in_sync_default_param_ok() {
    // In a non-async function, `await` is just an identifier
    let out = parse_script("function f(x = await) {}");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn await_in_async_body_ok() {
    // `await` in async function body is fine
    let out = parse_script("async function f() { await 1; }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── P9: dynamic import with options ──

#[test]
fn dynamic_import_with_options() {
    let (prog, e) = parse_expr("import('mod', { assert: { type: 'json' } })");
    match &prog.exprs.get(e).kind {
        ExprKind::DynamicImport { options, .. } => {
            assert!(options.is_some(), "Expected options argument");
        }
        other => panic!("Expected DynamicImport, got {other:?}"),
    }
}

#[test]
fn dynamic_import_with_trailing_comma() {
    let (prog, e) = parse_expr("import('mod',)");
    match &prog.exprs.get(e).kind {
        ExprKind::DynamicImport { options, .. } => {
            assert!(
                options.is_none(),
                "Trailing comma should not produce options"
            );
        }
        other => panic!("Expected DynamicImport, got {other:?}"),
    }
}

#[test]
fn dynamic_import_single_arg() {
    let (prog, e) = parse_expr("import('mod')");
    match &prog.exprs.get(e).kind {
        ExprKind::DynamicImport { options, .. } => {
            assert!(options.is_none(), "Single arg should have no options");
        }
        other => panic!("Expected DynamicImport, got {other:?}"),
    }
}
