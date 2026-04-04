//! Arrow function parsing and async expression disambiguation.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
// AST module exports 50+ node types used pervasively in parser.
use crate::ast::*;
use crate::atom::Atom;

use crate::error::JsParseErrorKind;
use crate::span::Span;
use crate::token::{Keyword, TokenKind};

use super::Parser;

impl Parser<'_> {
    /// Parse `(expr)` or arrow function `(params) => body`.
    pub(super) fn parse_paren_or_arrow(&mut self) -> NodeId<Expr> {
        let start = self.span();
        self.advance(); // skip (

        // Empty parens: `() =>`
        if matches!(self.at(), TokenKind::RParen) {
            self.advance();
            if matches!(self.at(), TokenKind::Arrow) && !self.had_newline_before {
                return self.parse_arrow_body(Vec::new(), false, start);
            }
            // Empty grouping — error
            self.error(JsParseErrorKind::UnexpectedToken, "Unexpected ')'".into());
            return self.error_expr(start);
        }

        // `(...rest) =>`
        if matches!(self.at(), TokenKind::Ellipsis) {
            // This must be arrow params
            let params = self.parse_arrow_params_from_rest(start);
            if matches!(self.at(), TokenKind::Arrow) && !self.had_newline_before {
                return self.parse_arrow_body(params, false, start);
            }
            self.error(
                JsParseErrorKind::UnexpectedToken,
                "Expected '=>' after rest parameter".into(),
            );
            return self.error_expr(start);
        }

        // Parse as expression — may reinterpret as arrow params
        let expr = self.parse_expression();
        let end = self.span();
        let _ = self.expect(&TokenKind::RParen);

        // Check for arrow: `(expr) =>`
        if matches!(self.at(), TokenKind::Arrow) && !self.had_newline_before {
            let params = self.reinterpret_as_arrow_params(expr);
            return self.parse_arrow_body(params, false, start);
        }

        // Regular parenthesized expression
        self.exprs.alloc(Expr {
            kind: ExprKind::Paren(expr),
            span: start.merge(end),
        })
    }

    fn parse_arrow_params_from_rest(&mut self, _start: Span) -> Vec<Param> {
        self.advance(); // skip ...
        let rest_start = self.span();
        let rest_pattern = self.parse_binding_pattern_direct();
        let rest_span = self.patterns.get(rest_pattern).span;
        let _ = self.expect(&TokenKind::RParen);
        vec![Param {
            pattern: rest_pattern,
            default: None,
            rest: true,
            span: rest_start.merge(rest_span),
        }]
    }

    pub(crate) fn parse_arrow_from_single_param(
        &mut self,
        name: Atom,
        start: Span,
    ) -> NodeId<Expr> {
        // E4: §15.3 — arrow parameter cannot be eval/arguments/await in strict mode
        self.check_reserved_binding(name, "a parameter name");
        let pat = self.patterns.alloc(Pattern {
            kind: PatternKind::Identifier(name),
            span: start,
        });
        let params = vec![Param::simple(pat, start)];
        self.parse_arrow_body(params, false, start)
    }

    fn parse_arrow_body(
        &mut self,
        params: Vec<Param>,
        is_async: bool,
        start: Span,
    ) -> NodeId<Expr> {
        let _ = self.expect(&TokenKind::Arrow); // skip =>

        // R8/E9/§15.3.1: check for duplicate parameter names
        self.check_duplicate_params(&params);

        // V12: Arrow functions inherit super context (in_method, in_constructor, in_static_block)
        // from the enclosing scope, unlike regular functions. Save and restore these flags
        // across with_function_context which would otherwise clear them.
        let saved_in_method = self.context.in_method;
        let saved_in_constructor = self.context.in_constructor;
        let saved_in_static_block = self.context.in_static_block;
        let saved_in_derived_constructor = self.context.in_derived_constructor;

        // Use with_function_context to also reset in_loop/in_switch (arrows create a new function
        // scope, so break/continue from an outer loop must not cross the arrow boundary).
        let body = self.with_function_context(is_async, false, |this| {
            // Restore super context that arrows inherit
            this.context.in_method = saved_in_method;
            this.context.in_constructor = saved_in_constructor;
            this.context.in_static_block = saved_in_static_block;
            this.context.in_derived_constructor = saved_in_derived_constructor;
            if matches!(this.at(), TokenKind::LBrace) {
                ArrowBody::Block(this.parse_block_body())
            } else {
                ArrowBody::Expression(this.parse_assignment_expression())
            }
        });

        let end = match &body {
            ArrowBody::Expression(e) => self.exprs.get(*e).span,
            ArrowBody::Block(stmts) => stmts.last().map_or(start, |s| self.stmts.get(*s).span),
        };

        self.exprs.alloc(Expr {
            kind: ExprKind::Arrow(Box::new(ArrowFunction {
                params,
                body,
                is_async,
                span: start.merge(end),
            })),
            span: start.merge(end),
        })
    }

    /// Reinterpret a parsed expression as arrow function parameters.
    fn reinterpret_as_arrow_params(&mut self, expr: NodeId<Expr>) -> Vec<Param> {
        // Clone only the Vec<NodeId<Expr>> (NodeId is Copy), not the whole Expr.
        if let ExprKind::Sequence(items) = &self.exprs.get(expr).kind {
            let items = items.clone();
            items
                .into_iter()
                .map(|item| self.expr_to_param(item))
                .collect()
        } else {
            vec![self.expr_to_param(expr)]
        }
    }

    fn expr_to_param(&mut self, expr: NodeId<Expr>) -> Param {
        // S2: depth guard for recursive cover grammar reinterpretation
        if !self.enter_recursion() {
            let span = self.exprs.get(expr).span;
            return Param::simple(self.error_pattern(span), span);
        }
        let span = self.exprs.get(expr).span;

        // Fast path for destructuring (Object/Array): no ExprKind ownership needed — just
        // convert to pattern directly. This avoids cloning the entire Property/Element Vec.
        if matches!(
            &self.exprs.get(expr).kind,
            ExprKind::Object(_) | ExprKind::Array(_)
        ) {
            let pat = self.expr_to_pattern(expr);
            self.leave_recursion();
            return Param::simple(pat, span);
        }

        // For remaining (small) variants, clone ExprKind to release the borrow before
        // using &mut self in the match arms.
        let e = self.exprs.get(expr).clone();
        let result = match e.kind {
            ExprKind::Identifier(name) => {
                self.check_reserved_binding(name, "a parameter name");
                let pat = self.patterns.alloc(Pattern {
                    kind: PatternKind::Identifier(name),
                    span,
                });
                Param::simple(pat, span)
            }
            ExprKind::Assignment {
                left: AssignTarget::Simple(lhs),
                op: AssignOp::Assign,
                right,
            } => {
                let pat_id = self.expr_to_pattern(lhs);
                Param {
                    pattern: pat_id,
                    default: Some(right),
                    rest: false,
                    span,
                }
            }
            ExprKind::Spread(inner) => {
                let pat = self.expr_to_pattern(inner);
                Param {
                    pattern: pat,
                    default: None,
                    rest: true,
                    span,
                }
            }
            ExprKind::Paren(inner) => {
                // V10: parenthesized destructuring patterns are not valid in arrow params
                if matches!(
                    self.exprs.get(inner).kind,
                    ExprKind::Object(_) | ExprKind::Array(_)
                ) {
                    self.error(
                        JsParseErrorKind::InvalidDestructuring,
                        "Parenthesized destructuring pattern is not allowed".into(),
                    );
                }
                self.expr_to_param(inner)
            }
            _ => {
                let pat = self.expr_to_pattern(expr);
                Param::simple(pat, span)
            }
        };
        self.leave_recursion();
        result
    }

    /// Parse async expression: could be `async function`, `async () =>`, or identifier.
    pub(super) fn parse_async_expression_or_identifier(&mut self) -> NodeId<Expr> {
        let start = self.span();
        self.advance(); // skip 'async'

        // `async function`
        if matches!(self.at(), TokenKind::Keyword(Keyword::Function)) && !self.had_newline_before {
            return self.parse_function_expression_inner(true, start);
        }

        // `async (params) =>` or `async x =>`
        if !self.had_newline_before {
            if matches!(self.at(), TokenKind::LParen) {
                return self.parse_async_arrow_or_call(start);
            }

            if let TokenKind::Identifier(name) = *self.at() {
                let param_start = self.span();
                self.advance();
                if matches!(self.at(), TokenKind::Arrow) && !self.had_newline_before {
                    // E4: §15.3 — arrow parameter cannot be eval/arguments/await
                    self.check_reserved_binding(name, "a parameter name");
                    let pat = self.patterns.alloc(Pattern {
                        kind: PatternKind::Identifier(name),
                        span: param_start,
                    });
                    let params = vec![Param::simple(pat, param_start)];
                    return self.parse_arrow_body(params, true, start);
                }
                // H4: Not an arrow — `async` is an identifier, `name` consumed
                // but no operator between them. Continue `name` as expression for
                // error recovery (the caller will likely hit a syntax error).
                let name_expr = self.exprs.alloc(Expr {
                    kind: ExprKind::Identifier(name),
                    span: param_start,
                });
                return self.continue_expression_from(name_expr);
            }
        }

        // Just `async` as identifier
        let async_atom = self.atoms.r#async;
        self.exprs.alloc(Expr {
            kind: ExprKind::Identifier(async_atom),
            span: start,
        })
    }

    /// Handle `async(...)` disambiguation: async arrow function or async call expression.
    fn parse_async_arrow_or_call(&mut self, start: Span) -> NodeId<Expr> {
        self.advance(); // skip (

        // `async() =>` or `async()` call
        if matches!(self.at(), TokenKind::RParen) {
            self.advance();
            if matches!(self.at(), TokenKind::Arrow) && !self.had_newline_before {
                return self.parse_arrow_body(Vec::new(), true, start);
            }
            // A8: `async()` — not an arrow, treat as call expression
            return self.make_async_call(start, Vec::new());
        }

        // `async(...rest) =>` or `async(...rest)` call
        if matches!(self.at(), TokenKind::Ellipsis) {
            let params = self.parse_arrow_params_from_rest(start);
            if matches!(self.at(), TokenKind::Arrow) && !self.had_newline_before {
                return self.parse_arrow_body(params, true, start);
            }
            // E4: Not an arrow — construct `async(...args)` call from consumed params
            let args: Vec<Argument> = params
                .into_iter()
                .map(|p| {
                    let pat = self.patterns.get(p.pattern);
                    let span = pat.span;
                    let expr = if let PatternKind::Identifier(name) = pat.kind {
                        self.exprs.alloc(Expr {
                            kind: ExprKind::Identifier(name),
                            span,
                        })
                    } else {
                        self.error_expr(span)
                    };
                    if p.rest {
                        Argument::Spread(expr)
                    } else {
                        Argument::Expression(expr)
                    }
                })
                .collect();
            return self.make_async_call(start, args);
        }

        // `async(expr, ...)` — parse as expression, may reinterpret as arrow params
        let expr = self.parse_expression();
        let _ = self.expect(&TokenKind::RParen);

        if matches!(self.at(), TokenKind::Arrow) && !self.had_newline_before {
            let params = self.reinterpret_as_arrow_params(expr);
            return self.parse_arrow_body(params, true, start);
        }

        // H5: It was `async(expr)` — a call to `async` as identifier.
        let args = if let ExprKind::Sequence(items) = &self.exprs.get(expr).kind {
            let items = items.clone(); // Vec<NodeId<Expr>>: cheap, NodeId is Copy
            items.into_iter().map(Argument::Expression).collect()
        } else {
            vec![Argument::Expression(expr)]
        };
        self.make_async_call(start, args)
    }

    /// Build a call expression `async(args...)`.
    fn make_async_call(&mut self, start: Span, arguments: Vec<Argument>) -> NodeId<Expr> {
        let async_atom = self.atoms.r#async;
        let callee = self.exprs.alloc(Expr {
            kind: ExprKind::Identifier(async_atom),
            span: start,
        });
        let end = self.span();
        self.exprs.alloc(Expr {
            kind: ExprKind::Call { callee, arguments },
            span: start.merge(end),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::*;
    use crate::parse_script;

    // ── Step 5: A8 — async() without arrow ──

    #[test]
    fn async_call_no_arrow() {
        // `async() + 1` should parse as `async()` call + `+ 1`
        let out = parse_script("async() + 1;");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        let stmt = &out.program.stmts.get(out.program.body[0]);
        if let StmtKind::Expression(e) = &stmt.kind {
            // Should be a binary Add expression
            assert!(
                matches!(
                    out.program.exprs.get(*e).kind,
                    ExprKind::Binary {
                        op: BinaryOp::Add,
                        ..
                    }
                ),
                "Expected binary add, got {:?}",
                out.program.exprs.get(*e).kind
            );
        } else {
            panic!("Expected expression statement");
        }
    }

    #[test]
    fn async_empty_arrow_ok() {
        // `async() => {}` should parse as async arrow
        let out = parse_script("async() => {};");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        let stmt = &out.program.stmts.get(out.program.body[0]);
        if let StmtKind::Expression(e) = &stmt.kind {
            if let ExprKind::Arrow(arrow) = &out.program.exprs.get(*e).kind {
                assert!(arrow.is_async);
                assert!(arrow.params.is_empty());
            } else {
                panic!("Expected arrow");
            }
        } else {
            panic!("Expected expression statement");
        }
    }

    // ── E4: async(...args) without arrow is a call ──

    #[test]
    fn async_spread_call_no_arrow() {
        // `async(...args)` without `=>` should parse as a call expression
        let out = parse_script("async(...args);");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        let stmt = &out.program.stmts.get(out.program.body[0]);
        if let StmtKind::Expression(e) = &stmt.kind {
            assert!(
                matches!(out.program.exprs.get(*e).kind, ExprKind::Call { .. }),
                "Expected call expression, got {:?}",
                out.program.exprs.get(*e).kind
            );
        } else {
            panic!("Expected expression statement");
        }
    }

    // ── E9: arrow duplicate parameters ──

    #[test]
    fn arrow_duplicate_param_error() {
        // E9/§15.3.1: arrow functions always strict — no duplicate params
        let out = parse_script("(a, a) => {};");
        assert!(
            out.errors.iter().any(|e| e.message.contains("Duplicate")),
            "Expected duplicate param error: {:?}",
            out.errors
        );
    }

    #[test]
    fn arrow_destructured_dup_param_error() {
        let out = parse_script("({x}, {x}) => {};");
        assert!(
            out.errors.iter().any(|e| e.message.contains("Duplicate")),
            "Expected duplicate param error: {:?}",
            out.errors
        );
    }

    #[test]
    fn arrow_unique_params_ok() {
        let out = parse_script("(a, b, c) => {};");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }

    // ── E4: single-param arrow reserved binding check ──

    #[test]
    fn arrow_single_param_eval_error() {
        let out = parse_script("eval => 1;");
        assert!(
            out.errors.iter().any(|e| e.message.contains("eval")),
            "Expected eval binding error: {:?}",
            out.errors
        );
    }

    #[test]
    fn arrow_single_param_arguments_error() {
        let out = parse_script("arguments => 1;");
        assert!(
            out.errors.iter().any(|e| e.message.contains("arguments")),
            "Expected arguments binding error: {:?}",
            out.errors
        );
    }

    #[test]
    fn async_arrow_single_param_eval_error() {
        let out = parse_script("async eval => 1;");
        assert!(
            out.errors.iter().any(|e| e.message.contains("eval")),
            "Expected eval binding error: {:?}",
            out.errors
        );
    }
}
