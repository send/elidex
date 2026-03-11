use crate::ast::*;
use crate::parse_script;

fn parse(src: &str) -> crate::error::ParseOutput {
    parse_script(src)
}

#[test]
fn variable_declarations() {
    let out = parse("let x = 1; const y = 2; var z = 3;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert_eq!(out.program.body.len(), 3);
    for (i, kind) in [VarKind::Let, VarKind::Const, VarKind::Var]
        .iter()
        .enumerate()
    {
        match &out.program.stmts.get(out.program.body[i]).kind {
            StmtKind::VariableDeclaration { kind: k, .. } => assert_eq!(k, kind),
            other => panic!("Expected var decl, got {other:?}"),
        }
    }
}

#[test]
fn if_else() {
    let out = parse("if (true) { 1 } else { 2 }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    match &out.program.stmts.get(out.program.body[0]).kind {
        StmtKind::If {
            alternate: Some(_), ..
        } => {}
        other => panic!("Expected if/else, got {other:?}"),
    }
}

#[test]
fn while_loop() {
    let out = parse("while (true) { break; }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert!(matches!(
        out.program.stmts.get(out.program.body[0]).kind,
        StmtKind::While { .. }
    ));
}

#[test]
fn for_loop() {
    let out = parse("for (let i = 0; i < 10; i++) {}");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert!(matches!(
        out.program.stmts.get(out.program.body[0]).kind,
        StmtKind::For { .. }
    ));
}

#[test]
fn for_in() {
    let out = parse("for (let k in obj) {}");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert!(matches!(
        out.program.stmts.get(out.program.body[0]).kind,
        StmtKind::ForIn { .. }
    ));
}

#[test]
fn for_of() {
    let out = parse("for (const x of arr) {}");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert!(matches!(
        out.program.stmts.get(out.program.body[0]).kind,
        StmtKind::ForOf { .. }
    ));
}

#[test]
fn switch_statement() {
    let out = parse("switch (x) { case 1: break; default: break; }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::Switch { cases, .. } = &out.program.stmts.get(out.program.body[0]).kind {
        assert_eq!(cases.len(), 2);
    } else {
        panic!("Expected switch");
    }
}

#[test]
fn try_catch_finally() {
    let out = parse("try { 1 } catch (e) { 2 } finally { 3 }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::Try {
        handler, finalizer, ..
    } = &out.program.stmts.get(out.program.body[0]).kind
    {
        assert!(handler.is_some());
        assert!(finalizer.is_some());
    } else {
        panic!("Expected try");
    }
}

#[test]
fn optional_catch_binding() {
    let out = parse("try { 1 } catch { 2 }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    if let StmtKind::Try {
        handler: Some(h), ..
    } = &out.program.stmts.get(out.program.body[0]).kind
    {
        assert!(h.param.is_none());
    } else {
        panic!("Expected try with optional catch");
    }
}

#[test]
fn labeled_statement() {
    let out = parse("outer: for (;;) { break outer; }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert!(matches!(
        out.program.stmts.get(out.program.body[0]).kind,
        StmtKind::Labeled { .. }
    ));
}

#[test]
fn do_while() {
    let out = parse("do { x++ } while (x < 10);");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert!(matches!(
        out.program.stmts.get(out.program.body[0]).kind,
        StmtKind::DoWhile { .. }
    ));
}

#[test]
fn asi_after_return() {
    let out = parse("function f() { return\n42 }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    // return should have no argument (ASI inserts ; after return)
    let func_stmt = out.program.body[0];
    if let StmtKind::FunctionDeclaration(f) = &out.program.stmts.get(func_stmt).kind {
        if let StmtKind::Return(arg) = &out.program.stmts.get(f.body[0]).kind {
            assert!(arg.is_none(), "return should have no argument due to ASI");
        }
    }
}

#[test]
fn empty_statement() {
    let out = parse(";;;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert_eq!(out.program.body.len(), 3);
}

#[test]
fn debugger_statement() {
    let out = parse("debugger;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert!(matches!(
        out.program.stmts.get(out.program.body[0]).kind,
        StmtKind::Debugger
    ));
}

#[test]
fn error_recovery_multiple_errors() {
    let out = parse("let = ; if { } let x = 1;");
    // Should produce errors but still parse some statements
    assert!(!out.errors.is_empty());
    // The last valid declaration should be present
    assert!(!out.program.body.is_empty());
}

#[test]
fn error_recovery_unclosed_block() {
    let out = parse("{ let x = 1;");
    assert!(!out.errors.is_empty());
    assert!(!out.program.body.is_empty());
}

// ── M7: for-in no_in disambiguation ──

#[test]
fn for_in_with_var_init() {
    // `for (var x in obj)` should parse as for-in, not classic for
    let out = parse("for (var x in obj) {}");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert!(matches!(
        out.program.stmts.get(out.program.body[0]).kind,
        StmtKind::ForIn { .. }
    ));
}

#[test]
fn for_classic_with_no_in() {
    // With no_in: `for (var x = a + b; ...)` — `a + b` should work normally
    let out = parse("for (var x = a + b; x < 10; x++) {}");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert!(matches!(
        out.program.stmts.get(out.program.body[0]).kind,
        StmtKind::For { .. }
    ));
}

// ── Step 3: A4 — for expression init no_in ──

#[test]
fn for_expr_init_no_in() {
    // `for (x in obj)` should parse as for-in, not classic for
    let out = parse("for (x in obj) {}");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert!(matches!(
        out.program.stmts.get(out.program.body[0]).kind,
        StmtKind::ForIn { .. }
    ));
}

// ── Step 3: A5 — label validation ──

#[test]
fn break_unknown_label_error() {
    let out = parse("while (true) { break unknown; }");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Undefined label")),
        "Expected undefined label error: {:?}",
        out.errors
    );
}

#[test]
fn continue_unknown_label_error() {
    let out = parse("while (true) { continue unknown; }");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Undefined label")),
        "Expected undefined label error: {:?}",
        out.errors
    );
}

#[test]
fn break_outside_loop_error() {
    let out = parse("break;");
    assert!(
        out.errors.iter().any(|e| e.message.contains("outside")),
        "Expected break outside loop error: {:?}",
        out.errors
    );
}

#[test]
fn labeled_break_ok() {
    let out = parse("outer: for (;;) { inner: for (;;) { break outer; } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── Step 3: B18 — with statement always error ──

// ── B3: duplicate label detection ──

#[test]
fn duplicate_label_error() {
    let out = parse("outer: outer: for (;;) {}");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Duplicate label")),
        "Expected duplicate label error: {:?}",
        out.errors
    );
}

#[test]
fn nested_same_label_error() {
    let out = parse("foo: { foo: for (;;) {} }");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Duplicate label")),
        "Expected duplicate label error for nested: {:?}",
        out.errors
    );
}

#[test]
fn different_labels_ok() {
    let out = parse("outer: inner: for (;;) { break outer; }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn with_statement_error() {
    let out = parse("with (obj) { x; }");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("with") || e.message.contains("strict")),
        "Expected with statement error: {:?}",
        out.errors
    );
}

// ── H3: for await validation ──

#[test]
fn for_await_of_ok() {
    let out = crate::parse_module("async function f() { for await (const x of stream) {} }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    let func = &out.program.stmts.get(out.program.body[0]);
    if let StmtKind::FunctionDeclaration(f) = &func.kind {
        assert!(matches!(
            out.program.stmts.get(f.body[0]).kind,
            StmtKind::ForOf { is_await: true, .. }
        ));
    } else {
        panic!("Expected function declaration");
    }
}

#[test]
fn for_await_in_error() {
    let out = crate::parse_module("async function f() { for await (const x in obj) {} }");
    assert!(
        out.errors.iter().any(|e| e.message.contains("for await")),
        "Expected for await error: {:?}",
        out.errors
    );
}

#[test]
fn for_await_classic_error() {
    let out = crate::parse_module("async function f() { for await (let i = 0; i < 10; i++) {} }");
    assert!(
        out.errors.iter().any(|e| e.message.contains("for await")),
        "Expected for await error: {:?}",
        out.errors
    );
}

// ── Coverage: throw newline restriction ──

#[test]
fn throw_newline_error() {
    let out = parse("function f() { throw\nnew Error(); }");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("line break") || e.message.contains("No line")),
        "Expected throw newline error: {:?}",
        out.errors
    );
}

// ── Coverage: try without catch or finally ──

#[test]
fn try_without_catch_or_finally_error() {
    let out = parse("try {}");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("catch") || e.message.contains("finally")),
        "Expected missing catch/finally error: {:?}",
        out.errors
    );
}

// ── Coverage: continue outside loop ──

#[test]
fn continue_outside_loop_error() {
    let out = parse("continue;");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("outside") || e.message.contains("loop")),
        "Expected continue outside loop error: {:?}",
        out.errors
    );
}

// ── Coverage: empty input ──

#[test]
fn empty_input() {
    let out = parse("");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert!(out.program.body.is_empty());
}

// ── H1: deep new expression recursion ──

#[test]
fn deep_new_recursion_no_crash() {
    // 2000 nested `new` — must not stack overflow, should produce NestingTooDeep error
    let src = "new ".repeat(2000) + "X";
    let out = parse(&src);
    assert!(
        out.errors
            .iter()
            .any(|e| e.kind == crate::error::JsParseErrorKind::NestingTooDeep),
        "Expected NestingTooDeep error: {:?}",
        out.errors
    );
}

// ── M1: label stack isolation across function boundary ──

#[test]
fn label_does_not_leak_into_nested_function() {
    let out = parse("outer: for (;;) { function f() { break outer; } break outer; }");
    // `break outer` inside f() should be an error (label not visible)
    // `break outer` outside f() should be fine
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Undefined label")),
        "Expected label error in nested function: {:?}",
        out.errors
    );
}

#[test]
fn label_still_works_outside_nested_function() {
    let out = parse("outer: for (;;) { function f() {} break outer; }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── A4: for-in/of initializer error ──

#[test]
fn for_in_initializer_error() {
    let out = parse("for (let x = 0 in obj) {}");
    assert!(
        out.errors.iter().any(|e| e.message.contains("initializer")),
        "Expected for-in initializer error: {:?}",
        out.errors
    );
}

#[test]
fn for_of_initializer_error() {
    let out = parse("for (const x = 0 of arr) {}");
    assert!(
        out.errors.iter().any(|e| e.message.contains("initializer")),
        "Expected for-of initializer error: {:?}",
        out.errors
    );
}

#[test]
fn for_in_no_initializer_ok() {
    let out = parse("for (let x in obj) {}");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── A5: import/export only at module top level ──

#[test]
fn import_inside_function_error() {
    let out = crate::parse_module("async function f() { import x from 'mod'; }");
    assert!(
        out.errors.iter().any(|e| e.message.contains("top level")),
        "Expected import top-level error: {:?}",
        out.errors
    );
}

#[test]
fn export_inside_block_error() {
    let out = crate::parse_module("{ export const x = 1; }");
    assert!(
        out.errors.iter().any(|e| e.message.contains("top level")),
        "Expected export top-level error: {:?}",
        out.errors
    );
}

#[test]
fn import_at_top_level_ok() {
    let out = crate::parse_module("import x from 'mod';");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn export_at_top_level_ok() {
    let out = crate::parse_module("export const x = 1;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn dynamic_import_inside_function_ok() {
    // import() expressions are not import declarations — they're allowed anywhere
    let out = crate::parse_module("async function f() { await import('mod'); }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── B2: await in sync function within module ──

#[test]
fn await_in_sync_function_in_module_not_unary() {
    // Inside a sync function in a module, `await x` should NOT be parsed as await expression
    // — `await` is reserved in modules but not a unary operator in sync functions
    let out = crate::parse_module("function f() { await; }");
    // Should parse `await` as an identifier expression (with possible reserved word error)
    // but NOT as an await expression
    let stmt = out.program.body[0];
    if let StmtKind::FunctionDeclaration(f) = &out.program.stmts.get(stmt).kind {
        if let StmtKind::Expression(e) = &out.program.stmts.get(f.body[0]).kind {
            assert!(
                matches!(out.program.exprs.get(*e).kind, ExprKind::Identifier(_)),
                "Expected identifier, got {:?}",
                out.program.exprs.get(*e).kind
            );
        }
    }
}

#[test]
fn await_at_module_top_level_ok() {
    let out = crate::parse_module("await fetch('url');");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn for_await_in_sync_function_in_module_error() {
    // for await should not be allowed inside sync functions even in module mode
    let out = crate::parse_module("function f() { for await (const x of stream) {} }");
    // `await` should not be recognized as for-await; it will be parsed as identifier
    // and cause a syntax error
    assert!(
        !out.errors.is_empty(),
        "Expected error for for-await in sync function"
    );
}

// ── B3: labeled function declaration error (strict mode) ──

#[test]
fn labeled_function_declaration_error() {
    let out = parse("foo: function f() {}");
    assert!(
        out.errors.iter().any(|e| {
            e.message.contains("function declarations") || e.message.contains("strict mode")
        }),
        "Expected labeled function declaration error: {:?}",
        out.errors
    );
}

// ── B4: lexical declarations in sub-statement position ──

#[test]
fn let_in_if_body_error() {
    let out = parse("if (true) let x = 1;");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("top level of a block")),
        "Expected lexical declaration error: {:?}",
        out.errors
    );
}

#[test]
fn const_in_while_body_error() {
    let out = parse("while (true) const x = 1;");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("top level of a block")),
        "Expected lexical declaration error: {:?}",
        out.errors
    );
}

#[test]
fn class_in_for_body_error() {
    let out = parse("for (;;) class C {}");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("top level of a block")),
        "Expected class declaration error: {:?}",
        out.errors
    );
}

#[test]
fn let_in_block_ok() {
    // Inside a block `{ }`, lexical declarations are fine
    let out = parse("if (true) { let x = 1; }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn function_in_block_ok() {
    // Function declarations inside blocks are fine in strict mode
    let out = parse("if (true) { function f() {} }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}
