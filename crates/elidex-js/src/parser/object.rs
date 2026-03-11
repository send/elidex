//! Object literal, property key, template literal, and method function parsing.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::atom::Atom;
use crate::error::JsParseErrorKind;
use crate::token::TokenKind;

use super::Parser;

impl Parser<'_> {
    /// Parse object literal.
    #[allow(clippy::too_many_lines)]
    pub(super) fn parse_object_literal(&mut self) -> NodeId<Expr> {
        let start = self.span();
        self.advance(); // skip {
        let mut properties = Vec::new();
        let mut has_proto = false; // P6: track duplicate __proto__

        while !matches!(self.at(), TokenKind::RBrace | TokenKind::Eof) && !self.aborted {
            let prop_start = self.span();

            // Spread property: `...expr`
            if matches!(self.at(), TokenKind::Ellipsis) {
                self.advance();
                let expr = self.parse_assignment_expression();
                let end = self.exprs.get(expr).span;
                let mut flags = PropertyFlags::default();
                flags.set_spread();
                properties.push(Property {
                    kind: PropertyKind::Init,
                    key: PropertyKey::Computed(expr),
                    value: None,
                    flags,
                    span: prop_start.merge(end),
                });
                self.expect_comma_unless(&TokenKind::RBrace);
                continue;
            }

            // R5: shared method prefix parsing (get/set/async/*)
            let prefixes = self.parse_method_prefixes(false);
            let method_kind = prefixes.method_kind;
            let is_async = prefixes.is_async;
            let is_generator = prefixes.is_generator;

            // Parse property key
            let (key, computed) = self.parse_property_key();

            // Method shorthand: `key(params) { body }`
            if matches!(self.at(), TokenKind::LParen) {
                let func = self.parse_method_function(is_async, is_generator, false, false);
                self.validate_accessor_params(method_kind, &func.params);
                let property_kind = match method_kind {
                    MethodKind::Get => PropertyKind::Get,
                    MethodKind::Set => PropertyKind::Set,
                    MethodKind::Method | MethodKind::Constructor => PropertyKind::Init,
                };
                let func_span = func.span;
                let value = self.exprs.alloc(Expr {
                    kind: ExprKind::Function(Box::new(func)),
                    span: func_span,
                });
                let mut flags = PropertyFlags::default();
                if computed {
                    flags.set_computed();
                }
                if matches!(method_kind, MethodKind::Method | MethodKind::Constructor) {
                    flags.set_method();
                }
                properties.push(Property {
                    kind: property_kind,
                    key,
                    value: Some(value),
                    flags,
                    span: prop_start.merge(func_span),
                });
            } else if matches!(self.at(), TokenKind::Colon) {
                // `key: value`
                self.advance();
                let val = self.parse_assignment_expression();
                let val_span = self.exprs.get(val).span;
                // P6: duplicate __proto__ is a syntax error (§13.2.5.1)
                if !computed && matches!(method_kind, MethodKind::Method | MethodKind::Constructor)
                {
                    let is_proto = match &key {
                        PropertyKey::Identifier(name) => *name == self.atoms.proto,
                        PropertyKey::Literal(Literal::String(s)) => *s == self.atoms.proto,
                        _ => false,
                    };
                    if is_proto {
                        if has_proto {
                            self.error(
                                JsParseErrorKind::UnexpectedToken,
                                "Duplicate __proto__ fields are not allowed in object literals"
                                    .into(),
                            );
                        }
                        has_proto = true;
                    }
                }
                let mut flags = PropertyFlags::default();
                if computed {
                    flags.set_computed();
                }
                properties.push(Property {
                    kind: PropertyKind::Init,
                    key,
                    value: Some(val),
                    flags,
                    span: prop_start.merge(val_span),
                });
            } else {
                // Shorthand property `{ x }` or `{ x = default }`
                let shorthand_value = self.parse_optional_initializer();
                let end = shorthand_value.map_or(prop_start, |v| self.exprs.get(v).span);
                // T2: track CoverInitializedName for later validation
                if shorthand_value.is_some() {
                    self.cover_init_span = Some(prop_start.merge(end));
                }

                if let PropertyKey::Identifier(name) = key {
                    let ident = self.exprs.alloc(Expr {
                        kind: ExprKind::Identifier(name),
                        span: prop_start,
                    });
                    let value = if let Some(default_val) = shorthand_value {
                        let span = prop_start.merge(self.exprs.get(default_val).span);
                        self.exprs.alloc(Expr {
                            kind: ExprKind::Assignment {
                                left: AssignTarget::Simple(ident),
                                op: AssignOp::Assign,
                                right: default_val,
                            },
                            span,
                        })
                    } else {
                        ident
                    };
                    let mut flags = PropertyFlags::default();
                    flags.set_shorthand();
                    properties.push(Property {
                        kind: PropertyKind::Init,
                        key,
                        value: Some(value),
                        flags,
                        span: prop_start.merge(end),
                    });
                } else {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "Shorthand property must be an identifier".into(),
                    );
                }
            }

            if self.check_list_limit(properties.len(), "object properties") {
                break;
            }
            self.expect_comma_unless(&TokenKind::RBrace);
        }

        let end = self.span();
        let _ = self.expect(&TokenKind::RBrace);

        self.exprs.alloc(Expr {
            kind: ExprKind::Object(properties),
            span: start.merge(end),
        })
    }

    /// Parse a property key.
    pub(crate) fn parse_property_key(&mut self) -> (PropertyKey, bool) {
        match *self.at() {
            TokenKind::LBracket => {
                self.advance();
                let expr = self.parse_assignment_expression();
                let _ = self.expect(&TokenKind::RBracket);
                (PropertyKey::Computed(expr), true)
            }
            TokenKind::Identifier(name) => {
                self.advance();
                (PropertyKey::Identifier(name), false)
            }
            TokenKind::StringLiteral(s) => {
                self.advance();
                (PropertyKey::Literal(Literal::String(s)), false)
            }
            TokenKind::NumericLiteral(n) => {
                self.advance();
                (PropertyKey::Literal(Literal::Number(n)), false)
            }
            TokenKind::BigIntLiteral(n) => {
                self.advance();
                (PropertyKey::Literal(Literal::BigInt(n)), false)
            }
            TokenKind::PrivateIdentifier(name) => {
                self.advance();
                (PropertyKey::PrivateIdentifier(name), false)
            }
            TokenKind::Keyword(kw) => {
                let name = self.intern(kw.as_str());
                self.advance();
                (PropertyKey::Identifier(name), false)
            }
            _ => {
                self.error(
                    JsParseErrorKind::UnexpectedToken,
                    format!("Expected property name, got {:?}", self.at()),
                );
                self.advance();
                (PropertyKey::Identifier(Atom::EMPTY), false)
            }
        }
    }

    /// Parse a template literal (no-sub or head+middles+tail).
    pub(crate) fn parse_template_literal(&mut self) -> TemplateLiteral {
        let start = self.span();
        match *self.at() {
            TokenKind::TemplateNoSub { cooked, raw } => {
                self.advance();
                TemplateLiteral {
                    quasis: vec![TemplateElement {
                        raw,
                        cooked,
                        tail: true,
                        span: start,
                    }],
                    expressions: Vec::new(),
                    span: start,
                }
            }
            TokenKind::TemplateHead { cooked, raw } => {
                let head_span = self.span();
                self.advance();
                let mut quasis = vec![TemplateElement {
                    raw,
                    cooked,
                    tail: false,
                    span: head_span,
                }];
                let mut expressions = Vec::new();

                loop {
                    if self.aborted {
                        break;
                    }
                    if self.check_list_limit(expressions.len(), "template expressions") {
                        break;
                    }
                    // Parse expression inside ${ ... }
                    let expr = self.parse_expression();
                    expressions.push(expr);

                    // Expect } and lex template part
                    if !matches!(self.at(), TokenKind::RBrace) {
                        self.error(
                            JsParseErrorKind::UnterminatedTemplate,
                            "Expected '}' in template literal".into(),
                        );
                        break;
                    }
                    let part = self.lex_template_part();
                    let part_span = part.span;

                    match part.kind {
                        TokenKind::TemplateTail { cooked, raw } => {
                            quasis.push(TemplateElement {
                                raw,
                                cooked,
                                tail: true,
                                span: part_span,
                            });
                            self.advance_after_template_part();
                            break;
                        }
                        TokenKind::TemplateMiddle { cooked, raw } => {
                            quasis.push(TemplateElement {
                                raw,
                                cooked,
                                tail: false,
                                span: part_span,
                            });
                            self.advance_after_template_part();
                        }
                        _ => {
                            self.error(
                                JsParseErrorKind::UnterminatedTemplate,
                                "Unexpected template token".into(),
                            );
                            break;
                        }
                    }
                }

                let end = quasis.last().map_or(start, |q| q.span);
                TemplateLiteral {
                    quasis,
                    expressions,
                    span: start.merge(end),
                }
            }
            _ => {
                self.error(
                    JsParseErrorKind::UnexpectedToken,
                    "Expected template literal".into(),
                );
                TemplateLiteral {
                    quasis: Vec::new(),
                    expressions: Vec::new(),
                    span: start,
                }
            }
        }
    }

    /// Advance the parser's current token after `lex_template_part()` has been called.
    /// `lex_template_part()` consumes the template span from the lexer but leaves
    /// `self.current` stale; this fetches the next real token.
    fn advance_after_template_part(&mut self) {
        let (next, nl) = self.lexer.next_token();
        self.current = next;
        self.had_newline_before = nl;
    }

    /// Parse method function body (for object/class methods).
    /// A7: when `is_constructor` is true, sets `in_constructor` for `super()` validation.
    /// V11: when `is_derived_constructor` is true, `super()` is allowed.
    #[allow(clippy::fn_params_excessive_bools)]
    pub(crate) fn parse_method_function(
        &mut self,
        is_async: bool,
        is_generator: bool,
        is_constructor: bool,
        is_derived_constructor: bool,
    ) -> Function {
        let start = self.span();
        let params = self.parse_formal_params(is_async, is_generator);
        let body = self.with_function_context(is_async, is_generator, |this| {
            // S4: methods have [[HomeObject]] — super.prop is valid
            this.context.in_method = true;
            if is_constructor {
                this.context.in_constructor = true;
            }
            // V11: only derived constructors allow super()
            this.context.in_derived_constructor = is_derived_constructor;
            this.parse_block_body()
        });

        let end = self.span();
        Function {
            name: None,
            params,
            body,
            is_async,
            is_generator,
            span: start.merge(end),
        }
    }
}
