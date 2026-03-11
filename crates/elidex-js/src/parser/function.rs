//! Function, arrow, and class declarations/expressions.

use std::collections::HashSet;

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::atom::Atom;
use crate::error::JsParseErrorKind;
use crate::span::Span;
use crate::token::{Keyword, TokenKind};

use super::Parser;

/// R7: Check if a property key matches a given name (identifier or string literal).
fn key_matches_name(key: &PropertyKey, name: Atom) -> bool {
    match key {
        PropertyKey::Identifier(n) => *n == name,
        PropertyKey::Literal(Literal::String(s)) => *s == name,
        _ => false,
    }
}

impl Parser<'_> {
    /// R1: Validate getter (0 params) / setter (exactly 1 param) parameter count.
    pub(super) fn validate_accessor_params(&mut self, kind: MethodKind, params: &[Param]) {
        match kind {
            MethodKind::Get if !params.is_empty() => {
                self.error(
                    JsParseErrorKind::UnexpectedToken,
                    "Getter must have no parameters".into(),
                );
            }
            MethodKind::Set => {
                if params.len() != 1 {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        format!(
                            "Setter must have exactly one parameter, got {}",
                            params.len()
                        ),
                    );
                } else if params[0].rest {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "Setter parameter must not be a rest element".into(),
                    );
                }
            }
            _ => {}
        }
    }

    // ── Function Declaration / Expression ─────────────────────────────

    pub(crate) fn parse_function_declaration(&mut self) -> NodeId<Stmt> {
        self.parse_function_declaration_inner(false)
    }

    pub(crate) fn parse_async_function_declaration(&mut self) -> NodeId<Stmt> {
        self.parse_function_declaration_inner(true)
    }

    fn parse_function_declaration_inner(&mut self, is_async: bool) -> NodeId<Stmt> {
        let start = self.span();
        if is_async {
            self.advance(); // skip `async`
        }
        let func = self.parse_function_inner(is_async);
        // B11: function declarations require a name (except export default)
        if func.name.is_none() {
            self.error(
                JsParseErrorKind::UnexpectedToken,
                "Function declaration requires a name".into(),
            );
        }
        let end = func.span;
        self.stmts.alloc(Stmt {
            kind: StmtKind::FunctionDeclaration(Box::new(func)),
            span: start.merge(end),
        })
    }

    pub(crate) fn parse_function_expression_node(&mut self) -> NodeId<Expr> {
        let start = self.span();
        self.parse_function_expression_inner(false, start)
    }

    pub(crate) fn parse_function_expression_inner(
        &mut self,
        is_async: bool,
        start: Span,
    ) -> NodeId<Expr> {
        let func = self.parse_function_inner(is_async);
        let end = func.span;
        self.exprs.alloc(Expr {
            kind: ExprKind::Function(Box::new(func)),
            span: start.merge(end),
        })
    }

    pub(crate) fn parse_function_inner(&mut self, is_async: bool) -> Function {
        let start = self.span();
        let _ = self.expect_keyword(Keyword::Function);

        let is_generator = if matches!(self.at(), TokenKind::Star) {
            self.advance();
            true
        } else {
            false
        };

        let name = if let TokenKind::Identifier(n) = *self.at() {
            self.check_reserved_binding(n, "a function name");
            self.advance();
            Some(n)
        } else {
            None
        };

        let params = self.parse_formal_params(is_async, is_generator);
        let body = self.with_function_context(is_async, is_generator, Self::parse_block_body);

        let end = self.span();
        Function {
            name,
            params,
            body,
            is_async,
            is_generator,
            span: start.merge(end),
        }
    }

    /// Parse formal parameters `(param, param = default, ...rest)`.
    /// `is_async`: when true, `await` is recognized as keyword but forbidden as expression (T1).
    /// `is_generator`: when true, `yield` is a keyword but forbidden as expression (E3).
    pub(crate) fn parse_formal_params(&mut self, is_async: bool, is_generator: bool) -> Vec<Param> {
        let saved_in_async_params = self.context.in_async_params;
        let saved_in_generator_params = self.context.in_generator_params;
        if is_async {
            self.context.in_async_params = true;
        }
        if is_generator {
            self.context.in_generator_params = true;
        }
        let _ = self.expect(&TokenKind::LParen);
        let mut params = Vec::new();

        while !matches!(self.at(), TokenKind::RParen | TokenKind::Eof) && !self.aborted {
            let param_start = self.span();

            let rest = if matches!(self.at(), TokenKind::Ellipsis) {
                self.advance();
                true
            } else {
                false
            };

            let pattern = self.parse_binding_pattern_direct();

            let default = if matches!(self.at(), TokenKind::Eq) {
                // B12: rest parameter must not have a default value
                if rest {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "Rest parameter must not have a default value".into(),
                    );
                }
                self.advance();
                Some(self.parse_assignment_expression())
            } else {
                None
            };

            let end = default.map_or_else(
                || self.patterns.get(pattern).span,
                |d| self.exprs.get(d).span,
            );

            params.push(Param {
                pattern,
                default,
                rest,
                span: param_start.merge(end),
            });

            if rest {
                // Rest must be last
                if matches!(self.at(), TokenKind::Comma) {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "Rest parameter must be last".into(),
                    );
                }
                break;
            }

            if self.check_list_limit(params.len(), "parameters") {
                break;
            }

            self.expect_comma_unless(&TokenKind::RParen);
        }

        let _ = self.expect(&TokenKind::RParen);
        self.context.in_async_params = saved_in_async_params;
        self.context.in_generator_params = saved_in_generator_params;
        // R8/B10: check for duplicate parameter names after collecting all params
        self.check_duplicate_params(&params);
        params
    }

    /// R8: Check for duplicate parameter names across a parameter list.
    pub(crate) fn check_duplicate_params(&mut self, params: &[Param]) {
        let mut seen_names: HashSet<Atom> = HashSet::new();
        for param in params {
            // Fast path: simple identifier params (the common case) avoid Vec allocation
            if let PatternKind::Identifier(name) = self.patterns.get(param.pattern).kind {
                if !seen_names.insert(name) {
                    self.error(
                        JsParseErrorKind::DuplicateBinding,
                        format!("Duplicate parameter name '{}'", self.resolve(name)),
                    );
                }
                continue;
            }
            let mut param_names = Vec::new();
            Self::collect_binding_names(&self.patterns, param.pattern, &mut param_names);
            for &name in &param_names {
                if !seen_names.insert(name) {
                    self.error(
                        JsParseErrorKind::DuplicateBinding,
                        format!("Duplicate parameter name '{}'", self.resolve(name)),
                    );
                }
            }
        }
    }

    /// A18: Recursively collect all binding names from a pattern (handles destructuring).
    /// S3: iterative to avoid stack overflow on deeply nested patterns.
    /// R7: Also used by scope analysis for export name collection.
    pub(crate) fn collect_binding_names(
        patterns: &crate::arena::Arena<Pattern>,
        id: NodeId<Pattern>,
        names: &mut Vec<Atom>,
    ) {
        let mut stack = vec![id];
        while let Some(current) = stack.pop() {
            match &patterns.get(current).kind {
                PatternKind::Identifier(name) => names.push(*name),
                PatternKind::Array { elements, rest } => {
                    for elem in elements.iter().flatten() {
                        stack.push(elem.pattern);
                    }
                    if let Some(r) = rest {
                        stack.push(*r);
                    }
                }
                PatternKind::Object { properties, rest } => {
                    for prop in properties {
                        stack.push(prop.value);
                    }
                    if let Some(r) = rest {
                        stack.push(*r);
                    }
                }
                PatternKind::Assign { left, .. } => {
                    stack.push(*left);
                }
                PatternKind::Expression(_) | PatternKind::Error => {}
            }
        }
    }

    /// E8/§15.7.1: Check for duplicate private names in a class.
    /// `sets[0]` = general, `sets[1]` = getters, `sets[2]` = setters.
    /// Allows matching get/set pairs for the same private name.
    fn check_private_name_dup(&mut self, member: &ClassMember, sets: &mut [HashSet<Atom>; 3]) {
        let (name, accessor_idx) = match &member.kind {
            ClassMemberKind::PrivateField { name, .. } => (*name, 0),
            ClassMemberKind::PrivateMethod { name, kind, .. } => {
                let idx = match kind {
                    MethodKind::Get => 1,
                    MethodKind::Set => 2,
                    _ => 0,
                };
                (*name, idx)
            }
            _ => return,
        };
        let name_str = self.resolve(name);
        if accessor_idx == 0 {
            if !sets[0].insert(name) {
                self.error(
                    JsParseErrorKind::DuplicateBinding,
                    format!("Duplicate private name '#{name_str}'"),
                );
            }
        } else {
            let other_idx = 3 - accessor_idx;
            let is_dup = !sets[accessor_idx].insert(name)
                || (sets[0].contains(&name) && !sets[other_idx].contains(&name));
            if is_dup {
                self.error(
                    JsParseErrorKind::DuplicateBinding,
                    format!("Duplicate private name '#{name_str}'"),
                );
            }
            sets[0].insert(name);
        }
    }

    // ── Class Declaration / Expression ────────────────────────────────

    pub(crate) fn parse_class_declaration(&mut self) -> NodeId<Stmt> {
        let start = self.span();
        let class = self.parse_class_inner();
        // B11: class declarations require a name (except export default)
        if class.name.is_none() {
            self.error(
                JsParseErrorKind::UnexpectedToken,
                "Class declaration requires a name".into(),
            );
        }
        let end = class.span;
        self.stmts.alloc(Stmt {
            kind: StmtKind::ClassDeclaration(Box::new(class)),
            span: start.merge(end),
        })
    }

    pub(crate) fn parse_class_expression_node(&mut self) -> NodeId<Expr> {
        let start = self.span();
        let class = self.parse_class_inner();
        let end = class.span;
        self.exprs.alloc(Expr {
            kind: ExprKind::Class(Box::new(class)),
            span: start.merge(end),
        })
    }

    pub(crate) fn parse_class_inner(&mut self) -> Class {
        let start = self.span();
        let _ = self.expect_keyword(Keyword::Class);

        let name = if let TokenKind::Identifier(n) = *self.at() {
            self.check_reserved_binding(n, "a class name");
            self.advance();
            Some(n)
        } else {
            None
        };

        let super_class = if self.at_keyword(Keyword::Extends) {
            self.advance();
            // H1: spec §15.7.1 — ClassHeritage: extends LeftHandSideExpression
            Some(self.parse_primary_and_suffix())
        } else {
            None
        };

        let _ = self.expect(&TokenKind::LBrace);
        let mut body = Vec::new();
        let mut seen_constructor = false;
        // E8/§15.7.1: track private names for duplicate detection
        // Indexed by: 0 = general, 1 = getter, 2 = setter
        let mut private_names: [HashSet<Atom>; 3] =
            [HashSet::new(), HashSet::new(), HashSet::new()];

        while !matches!(self.at(), TokenKind::RBrace | TokenKind::Eof) && !self.aborted {
            if matches!(self.at(), TokenKind::Semicolon) {
                let span = self.span();
                self.advance();
                body.push(ClassMember {
                    kind: ClassMemberKind::Empty,
                    span,
                });
                continue;
            }
            let member = self.parse_class_member(&mut seen_constructor, super_class.is_some());
            // E8: check duplicate private names
            self.check_private_name_dup(&member, &mut private_names);
            body.push(member);
            if self.check_list_limit(body.len(), "class members") {
                break;
            }
        }

        let end = self.span();
        let _ = self.expect(&TokenKind::RBrace);

        Class {
            name,
            super_class,
            body,
            span: start.merge(end),
        }
    }

    fn parse_class_member(&mut self, seen_constructor: &mut bool, has_super: bool) -> ClassMember {
        let start = self.span();

        let is_static = self.at_keyword(Keyword::Static);

        if is_static {
            self.advance(); // consume `static`

            // E3/§15.7.1: `static` is a field name only when followed by `=`, `;`, or `}`.
            if matches!(
                self.at(),
                TokenKind::Eq | TokenKind::Semicolon | TokenKind::RBrace
            ) {
                let value = self.parse_optional_initializer();
                self.expect_semicolon();
                let end = value.map_or(start, |v| self.exprs.get(v).span);
                return ClassMember {
                    kind: ClassMemberKind::Property {
                        key: PropertyKey::Identifier(self.intern("static")),
                        value,
                        is_static: false,
                        computed: false,
                    },
                    span: start.merge(end),
                };
            }

            // Static block: `static { ... }` (ES2022)
            // B22: static blocks are NOT function contexts — return is not allowed
            if matches!(self.at(), TokenKind::LBrace) {
                let saved = self.context;
                self.context.in_function = false;
                self.context.in_async = false;
                self.context.in_generator = false;
                self.context.in_loop = false;
                self.context.in_switch = false;
                self.context.in_static_block = true;
                let body = self.parse_block_body();
                self.context = saved;
                let end = self.span();
                return ClassMember {
                    kind: ClassMemberKind::StaticBlock(body),
                    span: start.merge(end),
                };
            }
        }

        // R5: shared method prefix parsing (async/*/get/set)
        let prefixes = self.parse_method_prefixes(true);
        let is_async = prefixes.is_async;
        let is_generator = prefixes.is_generator;
        let mut method_kind = prefixes.method_kind;

        // Private field/method
        if let TokenKind::PrivateIdentifier(name) = *self.at() {
            // B15: #constructor is not allowed
            if name == self.atoms.constructor {
                self.error(
                    JsParseErrorKind::UnexpectedToken,
                    "Class cannot have a private member named '#constructor'".into(),
                );
            }
            self.advance();

            if matches!(self.at(), TokenKind::LParen) {
                let func = self.parse_method_function(is_async, is_generator, false, false);
                self.validate_accessor_params(method_kind, &func.params);
                let end = func.span;
                return ClassMember {
                    kind: ClassMemberKind::PrivateMethod {
                        name,
                        function: func,
                        kind: method_kind,
                        is_static,
                    },
                    span: start.merge(end),
                };
            }

            let value = self.parse_optional_initializer();
            self.expect_semicolon();
            let end = value.map_or(start, |v| self.exprs.get(v).span);
            return ClassMember {
                kind: ClassMemberKind::PrivateField {
                    name,
                    value,
                    is_static,
                },
                span: start.merge(end),
            };
        }

        self.parse_class_method_or_field(
            start,
            is_static,
            is_async,
            is_generator,
            &mut method_kind,
            seen_constructor,
            has_super,
        )
    }

    /// Parse a class method or field after prefixes (static/async/*/get/set) and key.
    #[allow(clippy::fn_params_excessive_bools, clippy::too_many_arguments)]
    fn parse_class_method_or_field(
        &mut self,
        start: Span,
        is_static: bool,
        is_async: bool,
        is_generator: bool,
        method_kind: &mut MethodKind,
        seen_constructor: &mut bool,
        has_super: bool,
    ) -> ClassMember {
        let (key, computed) = self.parse_property_key();

        // Check for constructor (identifier or string literal key)
        if !is_static && !computed && key_matches_name(&key, self.atoms.constructor) {
            if *seen_constructor {
                self.error(
                    JsParseErrorKind::UnexpectedToken,
                    "A class may only have one constructor".into(),
                );
            }
            *seen_constructor = true;
            if *method_kind == MethodKind::Get || *method_kind == MethodKind::Set {
                self.error(
                    JsParseErrorKind::UnexpectedToken,
                    "Class constructor cannot be a getter or setter".into(),
                );
            }
            if is_generator {
                self.error(
                    JsParseErrorKind::UnexpectedToken,
                    "Constructor cannot be a generator".into(),
                );
            }
            if is_async {
                self.error(
                    JsParseErrorKind::UnexpectedToken,
                    "Constructor cannot be async".into(),
                );
            }
            *method_kind = MethodKind::Constructor;
        }

        // B16: static prototype (method or field) is not allowed
        if is_static && !computed && key_matches_name(&key, self.atoms.prototype) {
            self.error(
                JsParseErrorKind::UnexpectedToken,
                "Class cannot have a static member named 'prototype'".into(),
            );
        }

        // Method
        if matches!(self.at(), TokenKind::LParen) {
            let is_ctor = *method_kind == MethodKind::Constructor;
            // V11: super() only allowed in constructors of classes with extends
            let func =
                self.parse_method_function(is_async, is_generator, is_ctor, is_ctor && has_super);
            self.validate_accessor_params(*method_kind, &func.params);
            let end = func.span;
            return ClassMember {
                kind: ClassMemberKind::Method {
                    key,
                    function: func,
                    kind: *method_kind,
                    is_static,
                    computed,
                },
                span: start.merge(end),
            };
        }

        // Property (field)
        let value = self.parse_optional_initializer();
        self.expect_semicolon();
        let end = value.map_or(start, |v| self.exprs.get(v).span);
        ClassMember {
            kind: ClassMemberKind::Property {
                key,
                value,
                is_static,
                computed,
            },
            span: start.merge(end),
        }
    }
}

#[cfg(test)]
#[path = "function_tests.rs"]
mod tests;
