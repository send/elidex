//! Primary expressions, member access, call arguments, and compound literals.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
// AST module exports 50+ node types used pervasively in parser.
use crate::ast::*;
use crate::error::JsParseErrorKind;
use crate::token::{Keyword, TokenKind};

use super::Parser;

impl Parser<'_> {
    /// Parse a primary expression and any member/call suffixes.
    pub(super) fn parse_primary_and_suffix(&mut self) -> NodeId<Expr> {
        let expr = self.parse_primary();
        self.parse_suffix_loop(expr, true)
    }

    /// Parse primary + member access (no call — for `new` callee).
    pub(super) fn parse_member_expression(&mut self) -> NodeId<Expr> {
        let expr = self.parse_primary();
        self.parse_suffix_loop(expr, false)
    }

    /// Apply suffix operators (member, call, optional chain, tagged template).
    /// When `allow_call` is false, `LParen` and templates are not consumed (for `new` callee).
    // Single match dispatcher over token/AST variants.
    pub(crate) fn parse_suffix_loop(
        &mut self,
        mut expr: NodeId<Expr>,
        allow_call: bool,
    ) -> NodeId<Expr> {
        loop {
            if self.aborted {
                break;
            }
            match self.at() {
                // Member access: .prop
                TokenKind::Dot => {
                    self.advance();
                    let start = self.exprs.get(expr).span;
                    if let Some((prop, end)) = self.try_parse_dot_property() {
                        expr = self.exprs.alloc(Expr {
                            kind: ExprKind::Member {
                                object: expr,
                                property: prop,
                                computed: false,
                            },
                            span: start.merge(end),
                        });
                    } else {
                        if allow_call {
                            self.error(
                                JsParseErrorKind::UnexpectedToken,
                                "Expected property name after '.'".into(),
                            );
                        }
                        break;
                    }
                }
                // Computed member access: [expr]
                TokenKind::LBracket => {
                    self.advance();
                    let start = self.exprs.get(expr).span;
                    let prop = self.parse_expression();
                    let end = self.span();
                    let _ = self.expect(&TokenKind::RBracket);
                    expr = self.exprs.alloc(Expr {
                        kind: ExprKind::Member {
                            object: expr,
                            property: MemberProp::Expression(prop),
                            computed: true,
                        },
                        span: start.merge(end),
                    });
                }
                // Function call: (args)
                TokenKind::LParen if allow_call => {
                    // A7/V11: super() is only valid inside derived constructors
                    if matches!(self.exprs.get(expr).kind, ExprKind::Super) {
                        if !self.context.in_constructor {
                            self.error(
                                JsParseErrorKind::UnexpectedToken,
                                "'super()' is only valid inside a constructor".into(),
                            );
                        } else if !self.context.in_derived_constructor {
                            self.error(
                                JsParseErrorKind::UnexpectedToken,
                                "'super()' is only valid in a derived class constructor".into(),
                            );
                        }
                    }
                    let start = self.exprs.get(expr).span;
                    let args = self.parse_arguments();
                    let end = self.span();
                    expr = self.exprs.alloc(Expr {
                        kind: ExprKind::Call {
                            callee: expr,
                            arguments: args,
                        },
                        span: start.merge(end),
                    });
                }
                // E11/§13.3.5: Tagged template is part of MemberExpression — NOT gated by allow_call.
                // This ensures `new foo\`t\`` parses as `new (foo\`t\`)`, not `(new foo)\`t\``.
                TokenKind::TemplateNoSub { .. } | TokenKind::TemplateHead { .. } => {
                    // A11: tagged templates forbidden after optional chain
                    if matches!(self.exprs.get(expr).kind, ExprKind::OptionalChain { .. }) {
                        self.error(
                            JsParseErrorKind::UnexpectedToken,
                            "Tagged template cannot follow optional chain".into(),
                        );
                        break;
                    }
                    let start = self.exprs.get(expr).span;
                    let template = self.parse_template_literal();
                    let end = template.span;
                    expr = self.exprs.alloc(Expr {
                        kind: ExprKind::TaggedTemplate {
                            tag: expr,
                            template,
                        },
                        span: start.merge(end),
                    });
                }
                // Optional chain: ?.
                TokenKind::OptChain if allow_call => {
                    expr = self.parse_optional_chain(expr);
                }
                _ => break,
            }
        }

        expr
    }

    /// Parse an optional chain starting from `?.` (already peeked).
    fn parse_optional_chain(&mut self, base: NodeId<Expr>) -> NodeId<Expr> {
        let start = self.exprs.get(base).span;
        self.advance(); // consume ?.
        let mut chain = Vec::new();

        // First part of chain
        match self.at() {
            TokenKind::LParen => {
                let args = self.parse_arguments();
                chain.push(OptionalChainPart::Call(args));
            }
            TokenKind::LBracket => {
                self.advance();
                let prop = self.parse_expression();
                let _ = self.expect(&TokenKind::RBracket);
                chain.push(OptionalChainPart::Member {
                    property: MemberProp::Expression(prop),
                    computed: true,
                });
            }
            _ => {
                if let Some((prop, _span)) = self.try_parse_dot_property() {
                    chain.push(OptionalChainPart::Member {
                        property: prop,
                        computed: false,
                    });
                } else {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "Expected property name, '[', or '(' after '?.'".into(),
                    );
                }
            }
        }

        // Continue chain: .prop, [expr], (args)
        self.parse_chain_continuation(&mut chain);

        let end = self.span();
        self.exprs.alloc(Expr {
            kind: ExprKind::OptionalChain { base, chain },
            span: start.merge(end),
        })
    }

    /// Maximum parts in a single optional chain (prevents unbounded Vec growth).
    const MAX_CHAIN_PARTS: usize = 65536;

    /// Try to parse a dot-property name (identifier, keyword, or private identifier).
    /// Returns the property and its span, or `None` if not a valid property name.
    fn try_parse_dot_property(&mut self) -> Option<(MemberProp, crate::span::Span)> {
        let span = self.span();
        match *self.at() {
            TokenKind::Identifier(name) => {
                self.advance();
                Some((MemberProp::Identifier(name), span))
            }
            TokenKind::Keyword(kw) => {
                let name = self.intern(kw.as_str());
                self.advance();
                Some((MemberProp::Identifier(name), span))
            }
            TokenKind::PrivateIdentifier(name) => {
                self.advance();
                Some((MemberProp::PrivateIdentifier(name), span))
            }
            _ => None,
        }
    }

    /// Continue an optional chain with `.prop`, `[expr]`, `(args)` parts.
    fn parse_chain_continuation(&mut self, chain: &mut Vec<OptionalChainPart>) {
        loop {
            if self.aborted || chain.len() >= Self::MAX_CHAIN_PARTS {
                break;
            }
            match self.at() {
                TokenKind::Dot => {
                    self.advance();
                    if let Some((prop, _span)) = self.try_parse_dot_property() {
                        chain.push(OptionalChainPart::Member {
                            property: prop,
                            computed: false,
                        });
                    } else {
                        self.error(
                            JsParseErrorKind::UnexpectedToken,
                            "Expected property name after '.'".into(),
                        );
                        break;
                    }
                }
                TokenKind::LBracket => {
                    self.advance();
                    let prop = self.parse_expression();
                    let _ = self.expect(&TokenKind::RBracket);
                    chain.push(OptionalChainPart::Member {
                        property: MemberProp::Expression(prop),
                        computed: true,
                    });
                }
                TokenKind::LParen => {
                    let args = self.parse_arguments();
                    chain.push(OptionalChainPart::Call(args));
                }
                _ => break,
            }
        }
    }

    /// Parse call arguments `(a, b, ...c)`.
    pub(crate) fn parse_arguments(&mut self) -> Vec<Argument> {
        let _ = self.expect(&TokenKind::LParen);
        let mut args = Vec::new();
        while !matches!(self.at(), TokenKind::RParen | TokenKind::Eof) && !self.aborted {
            if matches!(self.at(), TokenKind::Ellipsis) {
                self.advance();
                let expr = self.parse_assignment_expression();
                args.push(Argument::Spread(expr));
            } else {
                let expr = self.parse_assignment_expression();
                args.push(Argument::Expression(expr));
            }
            if self.check_list_limit(args.len(), "arguments") {
                break;
            }
            self.expect_comma_unless(&TokenKind::RParen);
        }
        let _ = self.expect(&TokenKind::RParen);
        args
    }

    /// B7: If the current token is `/` or `/=`, re-lex as regexp literal.
    /// Called at expression-start positions where `/` must begin a regexp.
    fn rescan_slash_as_regexp(&mut self) {
        if !matches!(self.current.kind, TokenKind::Slash | TokenKind::SlashEq) {
            return;
        }
        let target = self.current.span.start as usize;
        if let Some(kind) = self.lexer.rescan_as_regexp(target) {
            let span = crate::span::Span::new(self.current.span.start, self.lexer.pos as u32);
            self.peeked = None;
            self.current = crate::token::Token { kind, span };
        }
    }

    /// R7: Advance past a literal token and allocate the corresponding expression node.
    fn alloc_literal(&mut self, lit: Literal, span: crate::span::Span) -> NodeId<Expr> {
        self.advance();
        self.exprs.alloc(Expr {
            kind: ExprKind::Literal(lit),
            span,
        })
    }

    /// Parse a primary expression.
    #[allow(clippy::too_many_lines)]
    // Single match dispatcher over token/AST variants.
    pub(super) fn parse_primary(&mut self) -> NodeId<Expr> {
        self.rescan_slash_as_regexp(); // B7
        let start = self.span();

        match *self.at() {
            TokenKind::Identifier(name) => {
                // Check for async arrow: `async (params) =>`
                if name == self.atoms.r#async {
                    return self.parse_async_expression_or_identifier();
                }
                self.advance();

                // Check for arrow function: `x =>`
                if matches!(self.at(), TokenKind::Arrow) && !self.had_newline_before {
                    return self.parse_arrow_from_single_param(name, start);
                }

                self.exprs.alloc(Expr {
                    kind: ExprKind::Identifier(name),
                    span: start,
                })
            }
            TokenKind::NumericLiteral(val) => self.alloc_literal(Literal::Number(val), start),
            TokenKind::BigIntLiteral(val) => self.alloc_literal(Literal::BigInt(val), start),
            TokenKind::StringLiteral(val) => self.alloc_literal(Literal::String(val), start),
            TokenKind::RegExpLiteral { pattern, flags } => {
                self.alloc_literal(Literal::RegExp { pattern, flags }, start)
            }
            TokenKind::Keyword(Keyword::True) => self.alloc_literal(Literal::Boolean(true), start),
            TokenKind::Keyword(Keyword::False) => {
                self.alloc_literal(Literal::Boolean(false), start)
            }
            TokenKind::Keyword(Keyword::Null) => self.alloc_literal(Literal::Null, start),
            // `undefined` is now lexed as Identifier, not Keyword (L3/L10)
            TokenKind::Keyword(Keyword::This) => {
                self.advance();
                self.exprs.alloc(Expr {
                    kind: ExprKind::This,
                    span: start,
                })
            }
            // Parenthesized expression or arrow params
            TokenKind::LParen => self.parse_paren_or_arrow(),
            // Array literal
            TokenKind::LBracket => self.parse_array_literal(),
            // Object literal
            TokenKind::LBrace => self.parse_object_literal(),
            // Function expression
            TokenKind::Keyword(Keyword::Function) => self.parse_function_expression_node(),
            // Class expression
            TokenKind::Keyword(Keyword::Class) => self.parse_class_expression_node(),
            // Template literal
            TokenKind::TemplateNoSub { .. } | TokenKind::TemplateHead { .. } => {
                let tl = self.parse_template_literal();
                let span = tl.span;
                self.exprs.alloc(Expr {
                    kind: ExprKind::Template(tl),
                    span,
                })
            }
            // import(...) or import.meta
            TokenKind::Keyword(Keyword::Import) => {
                self.advance();
                if matches!(self.at(), TokenKind::LParen) {
                    // Dynamic import — ES2025: optional second argument
                    self.advance();
                    let source = self.parse_assignment_expression();
                    let options = if matches!(self.at(), TokenKind::Comma) {
                        self.advance();
                        // Allow trailing comma: `import(source,)`
                        if matches!(self.at(), TokenKind::RParen) {
                            None
                        } else {
                            let opts = self.parse_assignment_expression();
                            // Consume optional trailing comma
                            if matches!(self.at(), TokenKind::Comma) {
                                self.advance();
                            }
                            Some(opts)
                        }
                    } else {
                        None
                    };
                    let end = self.span();
                    let _ = self.expect(&TokenKind::RParen);
                    self.exprs.alloc(Expr {
                        kind: ExprKind::DynamicImport { source, options },
                        span: start.merge(end),
                    })
                } else if matches!(self.at(), TokenKind::Dot) {
                    // import.meta
                    self.advance();
                    if self.at_contextual_atom(self.atoms.meta) {
                        // B1: import.meta is only valid in module context
                        if !self.context.is_module {
                            self.error(
                                JsParseErrorKind::UnexpectedToken,
                                "'import.meta' is only valid in module context".into(),
                            );
                        }
                        let end = self.span();
                        self.advance();
                        self.exprs.alloc(Expr {
                            kind: ExprKind::MetaProperty(MetaPropertyKind::ImportMeta),
                            span: start.merge(end),
                        })
                    } else {
                        self.error(
                            JsParseErrorKind::UnexpectedToken,
                            "Expected 'meta' after 'import.'".into(),
                        );
                        self.error_expr(start)
                    }
                } else {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "Expected '(' or '.' after 'import'".into(),
                    );
                    self.error_expr(start)
                }
            }
            TokenKind::Keyword(Keyword::Super) => {
                // S4: super.prop requires [[HomeObject]] — only valid in methods, constructors,
                // and static blocks. super() is validated separately at call site (A7).
                if !self.context.in_method
                    && !self.context.in_constructor
                    && !self.context.in_static_block
                {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "'super' keyword unexpected here".into(),
                    );
                }
                self.advance();
                // H2: §13.3.5/§13.3.6 — bare `super` is not valid; must be
                // followed by `.`, `[`, or `(` (call validated in suffix loop).
                if !matches!(
                    self.at(),
                    TokenKind::Dot | TokenKind::LBracket | TokenKind::LParen
                ) {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "'super' must be followed by '.', '[', or '()'".into(),
                    );
                }
                self.exprs.alloc(Expr {
                    kind: ExprKind::Super,
                    span: start,
                })
            }
            _ => {
                self.error(
                    JsParseErrorKind::UnexpectedToken,
                    format!("Unexpected token {:?}", self.at()),
                );
                let span = self.span();
                self.advance();
                self.error_expr(span)
            }
        }
    }

    /// Parse `new` expression (handles `new.target` too).
    pub(super) fn parse_new_expression(&mut self) -> NodeId<Expr> {
        // H1: depth guard for `new new new...` recursion
        if !self.enter_recursion() {
            return self.error_expr(self.span());
        }
        let start = self.span();
        self.advance(); // consume `new`

        // new.target
        if matches!(self.at(), TokenKind::Dot) {
            self.advance();
            if self.at_contextual_atom(self.atoms.target) {
                // B2: new.target is only valid inside functions
                if !self.context.in_function {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "'new.target' is only valid inside functions".into(),
                    );
                }
                let end = self.span();
                self.advance();
                self.leave_recursion();
                return self.exprs.alloc(Expr {
                    kind: ExprKind::MetaProperty(MetaPropertyKind::NewTarget),
                    span: start.merge(end),
                });
            }
            // Not target — error, fall through
            self.error(
                JsParseErrorKind::UnexpectedToken,
                "Expected 'target' after 'new.'".into(),
            );
        }

        // new Expr(args) — parse callee at member level (no call)
        let callee = if matches!(self.at(), TokenKind::Keyword(Keyword::New)) {
            self.parse_new_expression()
        } else {
            self.parse_member_expression()
        };

        // Arguments are optional for `new`
        let arguments = if matches!(self.at(), TokenKind::LParen) {
            self.parse_arguments()
        } else {
            Vec::new()
        };

        let end = self.span();
        self.leave_recursion();
        self.exprs.alloc(Expr {
            kind: ExprKind::New { callee, arguments },
            span: start.merge(end),
        })
    }

    /// Parse array literal.
    pub(super) fn parse_array_literal(&mut self) -> NodeId<Expr> {
        let start = self.span();
        self.advance(); // skip [
        let mut elements = Vec::new();

        while !matches!(self.at(), TokenKind::RBracket | TokenKind::Eof) && !self.aborted {
            if matches!(self.at(), TokenKind::Comma) {
                // Elision
                elements.push(None);
                self.advance();
                continue;
            }
            if matches!(self.at(), TokenKind::Ellipsis) {
                self.advance();
                let expr = self.parse_assignment_expression();
                elements.push(Some(ArrayElement::Spread(expr)));
            } else {
                let expr = self.parse_assignment_expression();
                elements.push(Some(ArrayElement::Expression(expr)));
            }
            if self.check_list_limit(elements.len(), "array elements") {
                break;
            }
            self.expect_comma_unless(&TokenKind::RBracket);
        }
        let end = self.span();
        let _ = self.expect(&TokenKind::RBracket);

        self.exprs.alloc(Expr {
            kind: ExprKind::Array(elements),
            span: start.merge(end),
        })
    }
}
