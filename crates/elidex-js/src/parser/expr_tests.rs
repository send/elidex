use crate::atom::Atom;
use crate::{ast::*, parse_module, parse_script, NodeId};

fn r(prog: &Program, atom: Atom) -> &str {
    prog.interner.get(atom)
}

fn parse_expr(src: &str) -> (Program, NodeId<Expr>) {
    let out = parse_script(src);
    assert!(out.errors.is_empty(), "Parse errors: {:?}", out.errors);
    let stmt = out.program.body[0];
    let expr_id = {
        let s = out.program.stmts.get(stmt);
        match &s.kind {
            StmtKind::Expression(e) => *e,
            _ => panic!("Expected expression statement"),
        }
    };
    (out.program, expr_id)
}

#[test]
fn simple_binary() {
    let (prog, e) = parse_expr("1 + 2");
    match &prog.exprs.get(e).kind {
        ExprKind::Binary {
            op: BinaryOp::Add, ..
        } => {}
        other => panic!("Expected Binary Add, got {other:?}"),
    }
}

#[test]
fn precedence_mul_over_add() {
    let (prog, e) = parse_expr("1 + 2 * 3");
    if let ExprKind::Binary {
        left,
        op: BinaryOp::Add,
        right,
    } = &prog.exprs.get(e).kind
    {
        assert!(matches!(
            prog.exprs.get(*left).kind,
            ExprKind::Literal(Literal::Number(_))
        ));
        assert!(matches!(
            prog.exprs.get(*right).kind,
            ExprKind::Binary {
                op: BinaryOp::Mul,
                ..
            }
        ));
    } else {
        panic!("Wrong structure");
    }
}

#[test]
fn ternary() {
    let (prog, e) = parse_expr("a ? b : c");
    assert!(matches!(
        prog.exprs.get(e).kind,
        ExprKind::Conditional { .. }
    ));
}

#[test]
fn member_and_call() {
    let (prog, e) = parse_expr("a.b(c)");
    assert!(matches!(prog.exprs.get(e).kind, ExprKind::Call { .. }));
}

#[test]
fn arrow_no_parens() {
    let (prog, e) = parse_expr("x => x + 1");
    assert!(matches!(prog.exprs.get(e).kind, ExprKind::Arrow(_)));
}

#[test]
fn arrow_with_parens() {
    let (prog, e) = parse_expr("(x, y) => x + y");
    if let ExprKind::Arrow(arrow) = &prog.exprs.get(e).kind {
        assert_eq!(arrow.params.len(), 2);
    } else {
        panic!("Expected arrow");
    }
}

#[test]
fn array_literal() {
    let (prog, e) = parse_expr("[1, 2, 3]");
    if let ExprKind::Array(elems) = &prog.exprs.get(e).kind {
        assert_eq!(elems.len(), 3);
    } else {
        panic!("Expected array");
    }
}

#[test]
fn object_literal() {
    let (prog, e) = parse_expr("({ a: 1, b: 2 })");
    if let ExprKind::Paren(inner) = &prog.exprs.get(e).kind {
        if let ExprKind::Object(props) = &prog.exprs.get(*inner).kind {
            assert_eq!(props.len(), 2);
        } else {
            panic!("Expected object");
        }
    } else {
        panic!("Expected paren");
    }
}

#[test]
fn template_literal() {
    let (prog, e) = parse_expr("`hello`");
    assert!(matches!(prog.exprs.get(e).kind, ExprKind::Template(_)));
}

#[test]
fn new_expression() {
    let (prog, e) = parse_expr("new Foo(1)");
    if let ExprKind::New { arguments, .. } = &prog.exprs.get(e).kind {
        assert_eq!(arguments.len(), 1);
    } else {
        panic!("Expected new");
    }
}

#[test]
fn optional_chain() {
    let (prog, e) = parse_expr("a?.b");
    assert!(matches!(
        prog.exprs.get(e).kind,
        ExprKind::OptionalChain { .. }
    ));
}

#[test]
fn optional_chain_keyword_member() {
    // L8: `a?.class` — keyword after `?.` should work as property name
    let (prog, e) = parse_expr("a?.class");
    assert!(matches!(
        prog.exprs.get(e).kind,
        ExprKind::OptionalChain { .. }
    ));
}

#[test]
fn null_coalescing() {
    let (prog, e) = parse_expr("a ?? b");
    assert!(matches!(
        prog.exprs.get(e).kind,
        ExprKind::Logical {
            op: LogicalOp::NullCoal,
            ..
        }
    ));
}

#[test]
fn unary_prefix() {
    let (prog, e) = parse_expr("typeof x");
    assert!(matches!(
        prog.exprs.get(e).kind,
        ExprKind::Unary {
            op: UnaryOp::Typeof,
            ..
        }
    ));
}

#[test]
fn update_expressions() {
    let (prog, e) = parse_expr("++x");
    assert!(matches!(
        prog.exprs.get(e).kind,
        ExprKind::Update {
            op: UpdateOp::Increment,
            prefix: true,
            ..
        }
    ));
}

#[test]
fn assignment() {
    let (prog, e) = parse_expr("x = 5");
    assert!(matches!(
        prog.exprs.get(e).kind,
        ExprKind::Assignment {
            op: AssignOp::Assign,
            ..
        }
    ));
}

#[test]
fn dynamic_import() {
    let (prog, e) = parse_expr("import('module')");
    assert!(matches!(
        prog.exprs.get(e).kind,
        ExprKind::DynamicImport { .. }
    ));
}

#[test]
fn comma_sequence() {
    let (prog, e) = parse_expr("1, 2, 3");
    if let ExprKind::Sequence(items) = &prog.exprs.get(e).kind {
        assert_eq!(items.len(), 3);
    } else {
        panic!("Expected sequence");
    }
}

#[test]
fn exponentiation_right_assoc() {
    let (prog, e) = parse_expr("2 ** 3 ** 4");
    if let ExprKind::Binary {
        right,
        op: BinaryOp::Exp,
        ..
    } = &prog.exprs.get(e).kind
    {
        assert!(matches!(
            prog.exprs.get(*right).kind,
            ExprKind::Binary {
                op: BinaryOp::Exp,
                ..
            }
        ));
    } else {
        panic!("Expected right-assoc exp");
    }
}

#[test]
fn spread_in_array() {
    let (prog, e) = parse_expr("[...a, 1]");
    if let ExprKind::Array(elems) = &prog.exprs.get(e).kind {
        assert_eq!(elems.len(), 2);
        assert!(matches!(elems[0], Some(ArrayElement::Spread(_))));
    } else {
        panic!("Expected array");
    }
}

// ── H1-H3 regression: binary operators in expression statements ──

#[test]
fn binary_mul_expression_stmt() {
    let (prog, e) = parse_expr("x * 2");
    assert!(matches!(
        prog.exprs.get(e).kind,
        ExprKind::Binary {
            op: BinaryOp::Mul,
            ..
        }
    ));
}

#[test]
fn binary_strict_eq_expression_stmt() {
    let (prog, e) = parse_expr("x === y");
    assert!(matches!(
        prog.exprs.get(e).kind,
        ExprKind::Binary {
            op: BinaryOp::StrictEq,
            ..
        }
    ));
}

#[test]
fn logical_and_or_expression_stmt() {
    let (prog, e) = parse_expr("x && y || z");
    // Should be: (x && y) || z
    if let ExprKind::Logical {
        op: LogicalOp::Or,
        left,
        ..
    } = &prog.exprs.get(e).kind
    {
        assert!(matches!(
            prog.exprs.get(*left).kind,
            ExprKind::Logical {
                op: LogicalOp::And,
                ..
            }
        ));
    } else {
        panic!("Expected Logical Or, got {:?}", prog.exprs.get(e).kind);
    }
}

// ── H1-H3: expression-then-binary via labeled/identifier path ──

#[test]
fn ident_binary_via_continue_expression() {
    // This goes through parse_possible_labeled_or_expression → continue_expression_from
    let out = parse_script("x * 2;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    let stmt = &out.program.stmts.get(out.program.body[0]);
    if let StmtKind::Expression(e) = &stmt.kind {
        assert!(matches!(
            out.program.exprs.get(*e).kind,
            ExprKind::Binary {
                op: BinaryOp::Mul,
                ..
            }
        ));
    } else {
        panic!("Expected expression statement");
    }
}

// ── H8-H9: object literal get/set as property name ──

#[test]
fn object_get_as_property_name() {
    let (prog, e) = parse_expr("({ get: 1 })");
    if let ExprKind::Paren(inner) = &prog.exprs.get(e).kind {
        if let ExprKind::Object(props) = &prog.exprs.get(*inner).kind {
            assert_eq!(props.len(), 1);
            assert!(
                matches!(props[0].key, PropertyKey::Identifier(name) if r(&prog, name) == "get")
            );
            assert!(props[0].value.is_some());
        } else {
            panic!("Expected object");
        }
    } else {
        panic!("Expected paren");
    }
}

#[test]
fn object_set_as_property_name() {
    let (prog, e) = parse_expr("({ set: \"x\" })");
    if let ExprKind::Paren(inner) = &prog.exprs.get(e).kind {
        if let ExprKind::Object(props) = &prog.exprs.get(*inner).kind {
            assert_eq!(props.len(), 1);
            assert!(
                matches!(props[0].key, PropertyKey::Identifier(name) if r(&prog, name) == "set")
            );
        } else {
            panic!("Expected object");
        }
    } else {
        panic!("Expected paren");
    }
}

#[test]
fn object_async_as_property_name() {
    let (prog, e) = parse_expr("({ async: true })");
    if let ExprKind::Paren(inner) = &prog.exprs.get(e).kind {
        if let ExprKind::Object(props) = &prog.exprs.get(*inner).kind {
            assert_eq!(props.len(), 1);
            assert!(
                matches!(props[0].key, PropertyKey::Identifier(name) if r(&prog, name) == "async")
            );
        } else {
            panic!("Expected object");
        }
    } else {
        panic!("Expected paren");
    }
}

// ── L11: super expression ──

#[test]
fn super_member_access() {
    let out = parse_script("class C extends B { constructor() { super.method(); } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── import.meta and new.target ──

#[test]
fn import_meta() {
    let out = crate::parse_module("import.meta.url;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn new_target() {
    let out = parse_script("function f() { new.target; }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── undefined as identifier (L10) ──

#[test]
fn undefined_is_identifier() {
    let (prog, e) = parse_expr("undefined");
    assert!(matches!(
        prog.exprs.get(e).kind,
        ExprKind::Identifier(name) if r(&prog, name) == "undefined"
    ));
}

#[test]
fn undefined_is_rebindable() {
    let out = parse_script("var undefined = 5;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── A6/A9: unary/await before ** requires parens ──

#[test]
fn unary_before_exp_error() {
    let out = parse_script("-2 ** 3;");
    assert!(
        out.errors.iter().any(|e| e.message.contains("parentheses")),
        "Expected unary before ** error: {:?}",
        out.errors
    );
}

#[test]
fn paren_unary_exp_ok() {
    let out = parse_script("(-2) ** 3;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn await_exponentiation_error() {
    let out = parse_script("async function f() { await x ** 2; }");
    assert!(
        out.errors.iter().any(|e| e.message.contains("parentheses")),
        "Expected await ** error: {:?}",
        out.errors
    );
}

// ── A5: for-in/of multiple declarators ──

#[test]
fn for_in_multiple_declarators_error() {
    let out = parse_script("for (let x, y in obj) {}");
    assert!(
        out.errors.iter().any(|e| e.message.contains("exactly one")),
        "Expected for-in declarator error: {:?}",
        out.errors
    );
}

#[test]
fn for_of_multiple_declarators_error() {
    let out = parse_script("for (const a, b of arr) {}");
    assert!(
        out.errors.iter().any(|e| e.message.contains("exactly one")),
        "Expected for-of declarator error: {:?}",
        out.errors
    );
}

// ── A6: switch multiple default ──

#[test]
fn switch_multiple_default_error() {
    let out = parse_script("switch (x) { default: break; default: break; }");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Multiple 'default'")),
        "Expected multiple default error: {:?}",
        out.errors
    );
}

#[test]
fn switch_single_default_ok() {
    let out = parse_script("switch (x) { case 1: break; default: break; }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── Step 3: B8 — ?? cannot mix with && / || ──

#[test]
fn nullcoal_and_mixing_error() {
    let out = parse_script("a ?? b && c;");
    assert!(
        out.errors.iter().any(|e| e.message.contains("Cannot mix")),
        "Expected ?? mixing error: {:?}",
        out.errors
    );
}

#[test]
fn nullcoal_or_mixing_error() {
    let out = parse_script("a || b ?? c;");
    assert!(
        out.errors.iter().any(|e| e.message.contains("Cannot mix")),
        "Expected ?? mixing error: {:?}",
        out.errors
    );
}

#[test]
fn nullcoal_with_parens_ok() {
    let out = parse_script("(a ?? b) || c;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── Step 3: B19 — delete identifier in strict mode ──

#[test]
fn delete_identifier_strict_error() {
    let out = parse_script("\"use strict\"; delete x;");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("delete") && e.message.contains("identifier")),
        "Expected delete identifier error: {:?}",
        out.errors
    );
}

#[test]
fn delete_member_ok() {
    let out = parse_script("delete obj.prop;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── A10: invalid assignment target ──

#[test]
fn invalid_assignment_target_literal() {
    let out = parse_script("1 = 2;");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Invalid left-hand side")),
        "Expected invalid assignment target error: {:?}",
        out.errors
    );
}

#[test]
fn invalid_assignment_target_call() {
    let out = parse_script("f() = x;");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Invalid left-hand side")),
        "Expected invalid assignment target error: {:?}",
        out.errors
    );
}

#[test]
fn valid_assignment_destructuring() {
    // Object destructuring cover grammar
    let out = parse_script("({x} = obj);");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn valid_assignment_paren() {
    let out = parse_script("(x) = 1;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── A11: tagged template after optional chain ──

#[test]
fn tagged_template_after_optional_chain_error() {
    let out = parse_script("a?.b`template`;");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Tagged template")),
        "Expected tagged template error: {:?}",
        out.errors
    );
}

// ── B21: top-level await ──

#[test]
fn top_level_await_module_ok() {
    let out = crate::parse_module("await fetch('url');");
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

// ── Coverage: postfix update ──

#[test]
fn postfix_increment() {
    let (prog, e) = parse_expr("x++");
    match &prog.exprs.get(e).kind {
        ExprKind::Update {
            op: UpdateOp::Increment,
            prefix: false,
            ..
        } => {}
        other => panic!("Expected postfix increment, got {other:?}"),
    }
}

#[test]
fn postfix_decrement() {
    let (prog, e) = parse_expr("x--");
    match &prog.exprs.get(e).kind {
        ExprKind::Update {
            op: UpdateOp::Decrement,
            prefix: false,
            ..
        } => {}
        other => panic!("Expected postfix decrement, got {other:?}"),
    }
}

// ── Coverage: template literal with multiple substitutions ──

#[test]
fn template_multi_substitution() {
    let (prog, e) = parse_expr("`a${b}c${d}e`");
    match &prog.exprs.get(e).kind {
        ExprKind::Template(tpl) => {
            assert_eq!(tpl.quasis.len(), 3, "Expected 3 quasis");
            assert_eq!(tpl.expressions.len(), 2, "Expected 2 expressions");
        }
        other => panic!("Expected Template, got {other:?}"),
    }
}

// ── A7: super() only in constructors ──

#[test]
fn super_call_in_constructor_ok() {
    let out = crate::parse_script("class A extends B { constructor() { super(); } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn super_call_in_method_error() {
    let out = crate::parse_script("class A extends B { method() { super(); } }");
    assert!(
        out.errors.iter().any(|e| e.message.contains("super()")),
        "Expected super() error in method: {:?}",
        out.errors
    );
}

#[test]
fn super_member_in_method_ok() {
    // super.prop is allowed in any method
    let out = crate::parse_script("class A extends B { method() { super.foo(); } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── E10: for-in/of invalid LHS ──

#[test]
fn for_in_literal_lhs_error() {
    let out = crate::parse_script("for (1 in obj) {}");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("assignment target") || e.message.contains("left-hand")),
        "Expected invalid LHS error: {:?}",
        out.errors
    );
}

#[test]
fn for_of_call_lhs_error() {
    let out = crate::parse_script("for (f() of arr) {}");
    assert!(
        !out.errors.is_empty(),
        "Expected error for call expression as for-of LHS"
    );
}

#[test]
fn for_in_identifier_ok() {
    let out = crate::parse_script("for (x in obj) {}");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── E11: tagged template allow_call guard removed ──

#[test]
fn tagged_template_in_new_ok() {
    // Tagged templates are part of MemberExpression, should work after new
    let out = crate::parse_script("new foo`template`;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── P3: super in static block ──

#[test]
fn super_call_in_static_block_error() {
    // super() (SuperCall) is forbidden in static blocks — only valid in constructors
    let out = crate::parse_script("class C extends B { static { super(); } }");
    assert!(
        out.errors.iter().any(|e| e.message.contains("super")),
        "Expected super() error in static block: {:?}",
        out.errors
    );
}

#[test]
fn super_property_in_static_block_ok() {
    // super.x (SuperProperty) is allowed in static blocks — refers to parent static members
    let out = crate::parse_script("class C extends B { static { super.x; } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn super_in_method_ok() {
    let out = crate::parse_script("class C extends B { method() { super.x; } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── P4: string import name requires 'as' ──

#[test]
fn string_import_name_requires_as() {
    let out = crate::parse_module("import { 'foo' } from 'mod';");
    assert!(
        !out.errors.is_empty(),
        "String import name without 'as' should error"
    );
}

#[test]
fn string_import_name_with_as_ok() {
    let out = crate::parse_module("import { 'foo' as bar } from 'mod';");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── P5: export string local name requires 'from' ──

#[test]
fn export_string_local_requires_from() {
    let out = crate::parse_module("export { 'foo' };");
    assert!(
        !out.errors.is_empty(),
        "Export with string local name without 'from' should error"
    );
}

#[test]
fn export_string_local_with_from_ok() {
    let out = crate::parse_module("export { 'foo' } from 'mod';");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── P6: duplicate __proto__ ──

#[test]
fn duplicate_proto_error() {
    let out = crate::parse_script("({ __proto__: {}, __proto__: {} });");
    assert!(
        out.errors.iter().any(|e| e.message.contains("__proto__")),
        "Expected duplicate __proto__ error: {:?}",
        out.errors
    );
}

#[test]
fn single_proto_ok() {
    let out = crate::parse_script("({ __proto__: {} });");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn proto_string_key_duplicate_error() {
    let out = crate::parse_script("({ '__proto__': {}, __proto__: {} });");
    assert!(
        out.errors.iter().any(|e| e.message.contains("__proto__")),
        "Expected duplicate __proto__ error with string key: {:?}",
        out.errors
    );
}

#[test]
fn proto_computed_no_error() {
    // Computed __proto__ doesn't count
    let out = crate::parse_script("({ ['__proto__']: {}, __proto__: {} });");
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

// ── T2: CoverInitializedName validation ──

#[test]
fn cover_init_name_in_destructuring_ok() {
    // `{x = 1}` as destructuring assignment target is valid
    let out = parse_script("({x = 1} = obj);");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn cover_init_name_in_expression_error() {
    // `{x = 1}` as an object literal expression is invalid
    let out = parse_script("({x = 1});");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Shorthand property initializer")),
        "Expected CoverInitializedName error: {:?}",
        out.errors
    );
}

#[test]
fn cover_init_name_in_call_error() {
    // `{x = 1}` passed as argument is invalid
    let out = parse_script("f({x = 1});");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Shorthand property initializer")),
        "Expected CoverInitializedName error in call: {:?}",
        out.errors
    );
}

#[test]
fn shorthand_without_init_ok() {
    // `{x}` without initializer is a valid object literal
    let out = parse_script("({x});");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── V1: Update expression target validation ──

#[test]
fn postfix_increment_literal_error() {
    let out = parse_script("5++;");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("left-hand side")),
        "Expected invalid LHS error: {:?}",
        out.errors
    );
}

#[test]
fn prefix_decrement_call_error() {
    let out = parse_script("--f();");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("left-hand side")),
        "Expected invalid LHS error: {:?}",
        out.errors
    );
}

#[test]
fn postfix_increment_identifier_ok() {
    let out = parse_script("x++;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn prefix_increment_member_ok() {
    let out = parse_script("++obj.x;");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── V9: eval/arguments as assignment targets ──

#[test]
fn eval_assignment_error() {
    let out = parse_script("eval = 1;");
    assert!(
        out.errors.iter().any(|e| e.message.contains("eval")),
        "Expected strict mode eval error: {:?}",
        out.errors
    );
}

#[test]
fn arguments_increment_error() {
    let out = parse_script("arguments++;");
    assert!(
        out.errors.iter().any(|e| e.message.contains("arguments")),
        "Expected strict mode arguments error: {:?}",
        out.errors
    );
}

#[test]
fn eval_compound_assign_error() {
    let out = parse_script("eval += 1;");
    assert!(
        out.errors.iter().any(|e| e.message.contains("eval")),
        "Expected strict mode eval error: {:?}",
        out.errors
    );
}

// ── V12: Arrow function inherits super context ──

#[test]
fn arrow_in_method_super_ok() {
    // super.prop inside an arrow inside a method should be valid
    let out = parse_script("class C { m() { const f = () => super.x; } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

#[test]
fn arrow_in_constructor_super_call_ok() {
    // super() inside an arrow inside a derived constructor
    let out = parse_script("class C extends B { constructor() { const f = () => super(); } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── V11: super() in non-derived constructor ──

#[test]
fn super_call_non_derived_error() {
    let out = parse_script("class C { constructor() { super(); } }");
    assert!(
        out.errors.iter().any(|e| e.message.contains("derived")),
        "Expected derived class error: {:?}",
        out.errors
    );
}

#[test]
fn super_call_derived_ok() {
    let out = parse_script("class C extends B { constructor() { super(); } }");
    assert!(out.errors.is_empty(), "{:?}", out.errors);
}

// ── V18b: Array rest trailing elements ──

#[test]
fn array_rest_trailing_arrow_error() {
    // Arrow param reinterpretation: expr→pattern detects rest-not-last
    let out = parse_script("([...a, b]) => {};");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Rest") || e.message.contains("rest")),
        "Expected rest element error: {:?}",
        out.errors
    );
}

// ── V10: Parenthesized destructuring in arrow params ──

#[test]
fn parenthesized_destructuring_arrow_error() {
    let out = parse_script("(([a])) => {};");
    assert!(
        out.errors
            .iter()
            .any(|e| e.message.contains("Parenthesized")),
        "Expected parenthesized destructuring error: {:?}",
        out.errors
    );
}

// ── V8: Break/continue label display ──

#[test]
fn continue_non_iteration_label_error_display() {
    // Label error should display the label name, not Atom(N)
    let out = parse_script("myLabel: if (true) { continue myLabel; }");
    let err = out
        .errors
        .iter()
        .find(|e| e.message.contains("continue"))
        .expect("Expected continue error");
    assert!(
        err.message.contains("myLabel"),
        "Error should contain label name 'myLabel', got: {}",
        err.message
    );
    assert!(
        !err.message.contains("Atom("),
        "Error should not contain raw Atom display: {}",
        err.message
    );
}

#[test]
fn undefined_label_error_display() {
    let out = parse_script("break foo;");
    let err = out
        .errors
        .iter()
        .find(|e| e.message.contains("Undefined"))
        .expect("Expected undefined label error");
    assert!(
        err.message.contains("foo"),
        "Error should contain label name 'foo', got: {}",
        err.message
    );
}
