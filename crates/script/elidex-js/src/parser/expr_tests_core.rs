use crate::atom::Atom;
use crate::{ast::*, parse_script, NodeId};

pub(super) fn r(prog: &Program, atom: Atom) -> String {
    prog.interner.get_utf8(atom)
}

pub(super) fn parse_expr(src: &str) -> (Program, NodeId<Expr>) {
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
fn comma_sequence() {
    let (prog, e) = parse_expr("1, 2, 3");
    if let ExprKind::Sequence(items) = &prog.exprs.get(e).kind {
        assert_eq!(items.len(), 3);
    } else {
        panic!("Expected sequence");
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
