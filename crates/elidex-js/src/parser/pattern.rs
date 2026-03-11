//! Destructuring pattern parsing and expression→pattern reinterpretation.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
// AST module exports 50+ node types used pervasively in parser.
use crate::ast::*;
use crate::error::JsParseErrorKind;
use crate::token::TokenKind;

use super::Parser;

impl Parser<'_> {
    /// Parse a binding pattern directly (for declarations, params).
    pub(crate) fn parse_binding_pattern_direct(&mut self) -> NodeId<Pattern> {
        if !self.enter_recursion() {
            return self.error_pattern(self.span());
        }
        let result = self.parse_binding_pattern_inner();
        self.leave_recursion();
        result
    }

    fn parse_binding_pattern_inner(&mut self) -> NodeId<Pattern> {
        let start = self.span();
        match *self.at() {
            TokenKind::Identifier(name) => {
                self.check_reserved_binding(name, "a binding name");
                self.advance();
                self.patterns.alloc(Pattern {
                    kind: PatternKind::Identifier(name),
                    span: start,
                })
            }
            TokenKind::LBracket => self.parse_array_pattern(),
            TokenKind::LBrace => self.parse_object_pattern(),
            _ => {
                self.error(
                    JsParseErrorKind::UnexpectedToken,
                    format!("Expected binding pattern, got {:?}", self.at()),
                );
                self.advance();
                self.error_pattern(start)
            }
        }
    }

    /// Parse array destructuring pattern `[a, , b, ...rest]`.
    fn parse_array_pattern(&mut self) -> NodeId<Pattern> {
        let start = self.span();
        self.advance(); // skip [
        let mut elements = Vec::new();
        let mut rest = None;

        while !matches!(self.at(), TokenKind::RBracket | TokenKind::Eof) && !self.aborted {
            if matches!(self.at(), TokenKind::Comma) {
                // Elision
                elements.push(None);
                self.advance();
                continue;
            }

            if matches!(self.at(), TokenKind::Ellipsis) {
                self.advance();
                let pat = self.parse_binding_pattern_direct();
                rest = Some(pat);
                // Trailing comma after rest is not allowed
                break;
            }

            let pat = self.parse_binding_pattern_direct();
            let default = if matches!(self.at(), TokenKind::Eq) {
                self.advance();
                Some(self.parse_assignment_expression())
            } else {
                None
            };

            elements.push(Some(ArrayPatternElement {
                pattern: pat,
                default,
            }));
            if self.check_list_limit(elements.len(), "array pattern elements") {
                break;
            }

            self.expect_comma_unless(&TokenKind::RBracket);
        }

        let end = self.span();
        let _ = self.expect(&TokenKind::RBracket);

        self.patterns.alloc(Pattern {
            kind: PatternKind::Array { elements, rest },
            span: start.merge(end),
        })
    }

    /// Parse object destructuring pattern `{ a, b: c, ...rest }`.
    fn parse_object_pattern(&mut self) -> NodeId<Pattern> {
        let start = self.span();
        self.advance(); // skip {
        let mut properties = Vec::new();
        let mut rest = None;

        while !matches!(self.at(), TokenKind::RBrace | TokenKind::Eof) && !self.aborted {
            let prop_start = self.span();

            if matches!(self.at(), TokenKind::Ellipsis) {
                self.advance();
                let pat = self.parse_binding_pattern_direct();
                // E12/§13.15.5.3: object rest element must be a simple identifier
                if !matches!(self.patterns.get(pat).kind, PatternKind::Identifier(_)) {
                    self.error(
                        JsParseErrorKind::InvalidDestructuring,
                        "Object rest element must be an identifier".into(),
                    );
                }
                rest = Some(pat);
                break;
            }

            // Parse key
            let (key, computed) = self.parse_property_key();
            let is_shorthand;
            let value;

            if matches!(self.at(), TokenKind::Colon) {
                // `key: pattern`
                self.advance();
                value = self.parse_binding_pattern_direct();
                is_shorthand = false;
            } else {
                // Shorthand `{ x }` or `{ x = default }`
                is_shorthand = true;
                if let PropertyKey::Identifier(name) = key {
                    value = self.patterns.alloc(Pattern {
                        kind: PatternKind::Identifier(name),
                        span: prop_start,
                    });
                } else {
                    self.error(
                        JsParseErrorKind::InvalidDestructuring,
                        "Shorthand property must be identifier".into(),
                    );
                    value = self.error_pattern(prop_start);
                }
            }

            // Default value
            let final_value = if matches!(self.at(), TokenKind::Eq) {
                self.advance();
                let default_expr = self.parse_assignment_expression();
                let span = self
                    .patterns
                    .get(value)
                    .span
                    .merge(self.exprs.get(default_expr).span);
                self.patterns.alloc(Pattern {
                    kind: PatternKind::Assign {
                        left: value,
                        right: default_expr,
                    },
                    span,
                })
            } else {
                value
            };

            let end = self.patterns.get(final_value).span;
            properties.push(ObjectPatternProp {
                key,
                value: final_value,
                computed,
                shorthand: is_shorthand,
                span: prop_start.merge(end),
            });
            if self.check_list_limit(properties.len(), "object pattern properties") {
                break;
            }

            self.expect_comma_unless(&TokenKind::RBrace);
        }

        let end = self.span();
        let _ = self.expect(&TokenKind::RBrace);

        self.patterns.alloc(Pattern {
            kind: PatternKind::Object { properties, rest },
            span: start.merge(end),
        })
    }

    /// Convert an expression to a pattern (cover grammar reinterpretation).
    #[allow(clippy::too_many_lines)]
    pub(crate) fn expr_to_pattern(&mut self, expr: NodeId<Expr>) -> NodeId<Pattern> {
        // S2: depth guard for recursive cover grammar reinterpretation
        if !self.enter_recursion() {
            return self.error_pattern(self.exprs.get(expr).span);
        }
        // NOTE: clone is intentional. We cannot use `mem::take` on the Arena entry because
        // scope analysis later visits the same Expr nodes (e.g. `({x = f()} = obj)` — the
        // Object expr is visited via AssignTarget::Simple to resolve references in default
        // values). Emptying the Vec<Property>/Vec<ArrayElement> here would cause scope
        // analysis to silently miss those inner expressions.
        let e = self.exprs.get(expr).clone();
        let span = e.span;

        let result = match e.kind {
            ExprKind::Identifier(name) => self.patterns.alloc(Pattern {
                kind: PatternKind::Identifier(name),
                span,
            }),
            ExprKind::Array(elements) => {
                let mut pat_elements = Vec::new();
                let mut rest_pat = None;
                for elem in elements {
                    // V18b: elements after rest (...) are not allowed
                    if rest_pat.is_some() {
                        self.error(
                            JsParseErrorKind::InvalidDestructuring,
                            "Rest element must be last element".into(),
                        );
                        break;
                    }
                    match elem {
                        None => pat_elements.push(None),
                        Some(ArrayElement::Expression(e)) => {
                            let pat = self.expr_to_pattern(e);
                            pat_elements.push(Some(ArrayPatternElement {
                                pattern: pat,
                                default: None,
                            }));
                        }
                        Some(ArrayElement::Spread(e)) => {
                            let pat = self.expr_to_pattern(e);
                            rest_pat = Some(pat);
                        }
                    }
                }
                self.patterns.alloc(Pattern {
                    kind: PatternKind::Array {
                        elements: pat_elements,
                        rest: rest_pat,
                    },
                    span,
                })
            }
            ExprKind::Object(properties) => {
                let mut pat_props = Vec::new();
                let mut rest_pat = None;
                for prop in properties {
                    // Spread property → rest pattern
                    // H5: spread stores expr in key (Computed), not value
                    if prop.flags.is_spread() {
                        if let Some(val) = prop.value {
                            let pat = self.expr_to_pattern(val);
                            rest_pat = Some(pat);
                        } else if let PropertyKey::Computed(expr) = prop.key {
                            let pat = self.expr_to_pattern(expr);
                            rest_pat = Some(pat);
                        }
                        continue;
                    }
                    if prop.flags.shorthand() {
                        if let PropertyKey::Identifier(name) = prop.key {
                            // S2: CoverInitializedName `{x = default}` — preserve the default
                            // value. The value is an Assignment expr `x = default`; extract
                            // the right side as the default initializer.
                            let pat = if let Some(val) = prop.value {
                                if let ExprKind::Assignment { right, .. } =
                                    &self.exprs.get(val).kind
                                {
                                    let ident = self.patterns.alloc(Pattern {
                                        kind: PatternKind::Identifier(name),
                                        span: self.exprs.get(val).span,
                                    });
                                    self.patterns.alloc(Pattern {
                                        kind: PatternKind::Assign {
                                            left: ident,
                                            right: *right,
                                        },
                                        span: prop.span,
                                    })
                                } else {
                                    self.patterns.alloc(Pattern {
                                        kind: PatternKind::Identifier(name),
                                        span: prop.span,
                                    })
                                }
                            } else {
                                self.patterns.alloc(Pattern {
                                    kind: PatternKind::Identifier(name),
                                    span: prop.span,
                                })
                            };
                            pat_props.push(ObjectPatternProp {
                                key: prop.key,
                                value: pat,
                                computed: prop.flags.computed(),
                                shorthand: true,
                                span: prop.span,
                            });
                        }
                    } else if let Some(val) = prop.value {
                        let pat = self.expr_to_pattern(val);
                        pat_props.push(ObjectPatternProp {
                            key: prop.key,
                            value: pat,
                            computed: prop.flags.computed(),
                            shorthand: false,
                            span: prop.span,
                        });
                    }
                }
                self.patterns.alloc(Pattern {
                    kind: PatternKind::Object {
                        properties: pat_props,
                        rest: rest_pat,
                    },
                    span,
                })
            }
            ExprKind::Assignment {
                left: AssignTarget::Simple(lhs),
                op: AssignOp::Assign,
                right,
            } => {
                let pat = self.expr_to_pattern(lhs);
                self.patterns.alloc(Pattern {
                    kind: PatternKind::Assign { left: pat, right },
                    span,
                })
            }
            ExprKind::Paren(inner) => self.expr_to_pattern(inner),
            ExprKind::Member { .. } => {
                // Member expressions are valid assignment targets
                self.patterns.alloc(Pattern {
                    kind: PatternKind::Expression(expr),
                    span,
                })
            }
            _ => {
                // B21: invalid destructuring target
                self.error(
                    JsParseErrorKind::InvalidDestructuring,
                    "Invalid destructuring assignment target".into(),
                );
                self.patterns.alloc(Pattern {
                    kind: PatternKind::Expression(expr),
                    span,
                })
            }
        };
        self.leave_recursion();
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::*;
    use crate::parse_script;

    #[test]
    fn array_destructuring() {
        let out = parse_script("let [a, , b, ...rest] = arr;");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        if let StmtKind::VariableDeclaration { declarators, .. } =
            &out.program.stmts.get(out.program.body[0]).kind
        {
            let pat = out.program.patterns.get(declarators[0].pattern);
            assert!(matches!(pat.kind, PatternKind::Array { .. }));
        } else {
            panic!("Expected variable declaration");
        }
    }

    #[test]
    fn object_destructuring() {
        let out = parse_script("let { a, b: c, ...rest } = obj;");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        if let StmtKind::VariableDeclaration { declarators, .. } =
            &out.program.stmts.get(out.program.body[0]).kind
        {
            let pat = out.program.patterns.get(declarators[0].pattern);
            assert!(matches!(pat.kind, PatternKind::Object { .. }));
        }
    }

    #[test]
    fn nested_destructuring() {
        let out = parse_script("let { a: [b, c] } = obj;");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }

    #[test]
    fn default_values() {
        let out = parse_script("let [a = 1, b = 2] = arr;");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }

    #[test]
    fn assignment_destructuring() {
        // This tests the cover grammar: `({ a, b } = obj);` is valid
        let out = parse_script("({ a, b } = obj);");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }

    // ── L15: object rest pattern via cover grammar ──

    #[test]
    fn object_rest_destructuring() {
        let out = parse_script("const { a, ...rest } = obj;");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        if let StmtKind::VariableDeclaration { declarators, .. } =
            &out.program.stmts.get(out.program.body[0]).kind
        {
            let pat = out.program.patterns.get(declarators[0].pattern);
            if let PatternKind::Object { rest, .. } = &pat.kind {
                assert!(rest.is_some(), "Expected rest pattern");
            } else {
                panic!("Expected object pattern");
            }
        }
    }

    #[test]
    fn object_rest_assignment_destructuring() {
        // `({ a, ...rest } = obj);` — cover grammar should convert spread to rest
        let out = parse_script("({ a, ...rest } = obj);");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }

    // ── Step 3: B20 — yield/await as binding names ──

    #[test]
    fn yield_binding_in_generator_error() {
        // S3: yield is a reserved keyword in strict mode — cannot be used as binding
        let out = parse_script("function* g() { let [yield] = arr; }");
        assert!(
            out.errors.iter().any(|e| e.message.contains("Yield")),
            "Expected yield binding error: {:?}",
            out.errors
        );
    }

    #[test]
    fn await_binding_in_async_error() {
        let out = parse_script("async function f() { let [await] = arr; }");
        assert!(
            out.errors.iter().any(|e| e.message.contains("await")),
            "Expected await binding error: {:?}",
            out.errors
        );
    }

    // ── H5: object spread → rest pattern via assignment cover grammar ──

    #[test]
    fn object_spread_rest_in_arrow_params() {
        // H5: arrow param destructuring triggers expr_to_pattern, which must
        // convert spread to rest pattern.
        let out = parse_script("const f = ({a, ...rest}) => rest;");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        // The arrow's first param should be an Object pattern with rest
        if let StmtKind::VariableDeclaration { declarators, .. } =
            &out.program.stmts.get(out.program.body[0]).kind
        {
            let init = declarators[0].init.expect("Expected init");
            let expr = out.program.exprs.get(init);
            if let ExprKind::Arrow(arrow) = &expr.kind {
                assert_eq!(arrow.params.len(), 1);
                let pat = out.program.patterns.get(arrow.params[0].pattern);
                if let PatternKind::Object { rest, .. } = &pat.kind {
                    assert!(rest.is_some(), "Spread should be converted to rest pattern");
                } else {
                    panic!("Expected object pattern, got {:?}", pat.kind);
                }
            } else {
                panic!("Expected Arrow, got {:?}", expr.kind);
            }
        }
    }
}
