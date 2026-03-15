use crate::ast::*;
use crate::parse_script;

use super::expr_tests_core::parse_expr;

// ── Basic binary/logical operators ──

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

// ── Postfix update ──

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

// ── B8: ?? cannot mix with && / || ──

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
