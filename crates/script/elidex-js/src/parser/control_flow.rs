//! Control flow statement parsing (if/while/do-while/for/switch/return/throw/try/break/continue/with).

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
// AST module exports 50+ node types used pervasively in parser.
use crate::ast::*;
use crate::error::JsParseErrorKind;
use crate::span::Span;
use crate::token::{Keyword, TokenKind};

use super::Parser;

impl Parser<'_> {
    pub(super) fn parse_if_statement(&mut self) -> NodeId<Stmt> {
        let start = self.span();
        self.advance(); // skip `if`
        let _ = self.expect(&TokenKind::LParen);
        let test = self.parse_expression();
        let _ = self.expect(&TokenKind::RParen);
        // B4: use parse_sub_statement to reject lexical declarations in single-statement position
        let consequent = self.parse_sub_statement();
        let alternate = if self.at_keyword(Keyword::Else) {
            self.advance();
            Some(self.parse_sub_statement())
        } else {
            None
        };
        let end = alternate.map_or_else(
            || self.stmts.get(consequent).span,
            |a| self.stmts.get(a).span,
        );
        self.stmts.alloc(Stmt {
            kind: StmtKind::If {
                test,
                consequent,
                alternate,
            },
            span: start.merge(end),
        })
    }

    pub(super) fn parse_while_statement(&mut self) -> NodeId<Stmt> {
        let start = self.span();
        self.advance(); // skip `while`
        let _ = self.expect(&TokenKind::LParen);
        let test = self.parse_expression();
        let _ = self.expect(&TokenKind::RParen);
        let body = self.parse_loop_body();
        let end = self.stmts.get(body).span;
        self.stmts.alloc(Stmt {
            kind: StmtKind::While { test, body },
            span: start.merge(end),
        })
    }

    pub(super) fn parse_do_while_statement(&mut self) -> NodeId<Stmt> {
        let start = self.span();
        self.advance(); // skip `do`
        let body = self.parse_loop_body();
        let _ = self.expect_keyword(Keyword::While);
        let _ = self.expect(&TokenKind::LParen);
        let test = self.parse_expression();
        let _ = self.expect(&TokenKind::RParen);
        self.expect_semicolon();
        let end = self.exprs.get(test).span;
        self.stmts.alloc(Stmt {
            kind: StmtKind::DoWhile { body, test },
            span: start.merge(end),
        })
    }

    pub(super) fn parse_for_statement(&mut self) -> NodeId<Stmt> {
        let start = self.span();
        self.advance(); // skip `for`

        // for await (x of y) — async iteration (also allowed at module top-level per ES2022)
        // B2: not allowed inside sync functions even in module mode
        let is_await =
            self.at_contextual_atom(self.atoms.r#await) && self.context.await_is_keyword();
        if is_await {
            self.advance();
        }

        let _ = self.expect(&TokenKind::LParen);

        // Determine init
        if matches!(self.at(), TokenKind::Semicolon) {
            self.check_for_await(is_await, "a classic for loop");
            return self.parse_for_classic(start, None);
        }

        if matches!(
            self.at(),
            TokenKind::Keyword(Keyword::Var | Keyword::Let | Keyword::Const)
        ) {
            let var_kind = self.parse_var_kind();
            let decls = self.parse_declarators_no_in();

            let is_for_in = self.at_keyword(Keyword::In);
            let is_for_of = !is_for_in && self.at_contextual_atom(self.atoms.of);
            if is_for_in || is_for_of {
                // A5: for-in/of requires exactly one declarator
                if decls.len() != 1 {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "for-in/of requires exactly one variable declarator".into(),
                    );
                }
                // A4: for-in/of declarator must not have an initializer
                if decls.first().is_some_and(|d| d.init.is_some()) {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "for-in/of variable declaration must not have an initializer".into(),
                    );
                }
                self.advance();
                // H1: safe access — decls could be empty after error recovery
                let pattern = decls
                    .first()
                    .map_or_else(|| self.error_pattern(self.span()), |d| d.pattern);
                let left = ForInOfLeft::Declaration {
                    kind: var_kind,
                    pattern,
                };
                if is_for_of {
                    let right = self.parse_assignment_expression();
                    return self.finish_for_in_of(start, left, right, true, is_await);
                }
                self.check_for_await(is_await, "'in'");
                let right = self.parse_expression();
                return self.finish_for_in_of(start, left, right, false, false);
            }

            self.check_for_await(is_await, "a classic for loop");
            let init = ForInit::Declaration {
                kind: var_kind,
                declarators: decls,
            };
            return self.parse_for_classic(start, Some(init));
        }

        // A4: Expression init — suppress `in` operator for for-in disambiguation
        let saved_no_in = self.context.no_in;
        self.context.no_in = true;
        let expr = self.parse_expression();
        self.context.no_in = saved_no_in;

        if self.at_keyword(Keyword::In) {
            // E10/§14.7.5: validate LHS is a valid assignment target
            if !self.is_valid_assign_target(expr, true) {
                self.error(
                    JsParseErrorKind::InvalidAssignmentTarget,
                    "Invalid left-hand side in for-in".into(),
                );
            }
            self.check_for_await(is_await, "'in'");
            self.advance();
            let right = self.parse_expression();
            return self.finish_for_in_of(start, ForInOfLeft::Pattern(expr), right, false, false);
        }
        if self.at_contextual_atom(self.atoms.of) {
            // E10/§14.7.5: validate LHS is a valid assignment target
            if !self.is_valid_assign_target(expr, true) {
                self.error(
                    JsParseErrorKind::InvalidAssignmentTarget,
                    "Invalid left-hand side in for-of".into(),
                );
            }
            self.advance();
            let right = self.parse_assignment_expression();
            return self.finish_for_in_of(start, ForInOfLeft::Pattern(expr), right, true, is_await);
        }

        self.check_for_await(is_await, "a classic for loop");
        self.parse_for_classic(start, Some(ForInit::Expression(expr)))
    }

    /// Finish parsing `for-in` or `for-of`: expect `)`, parse body, alloc stmt.
    fn finish_for_in_of(
        &mut self,
        start: Span,
        left: ForInOfLeft,
        right: NodeId<Expr>,
        is_of: bool,
        is_await: bool,
    ) -> NodeId<Stmt> {
        let _ = self.expect(&TokenKind::RParen);
        let body = self.parse_loop_body();
        let end = self.stmts.get(body).span;
        let kind = if is_of {
            StmtKind::ForOf {
                is_await,
                left,
                right,
                body,
            }
        } else {
            StmtKind::ForIn { left, right, body }
        };
        self.stmts.alloc(Stmt {
            kind,
            span: start.merge(end),
        })
    }

    fn parse_for_classic(&mut self, start: Span, init: Option<ForInit>) -> NodeId<Stmt> {
        let _ = self.expect(&TokenKind::Semicolon);
        let test = if matches!(self.at(), TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expression())
        };
        let _ = self.expect(&TokenKind::Semicolon);
        let update = if matches!(self.at(), TokenKind::RParen) {
            None
        } else {
            Some(self.parse_expression())
        };
        let _ = self.expect(&TokenKind::RParen);
        let body = self.parse_loop_body();
        let end = self.stmts.get(body).span;
        self.stmts.alloc(Stmt {
            kind: StmtKind::For {
                init,
                test,
                update,
                body,
            },
            span: start.merge(end),
        })
    }

    /// Parse declarators without allowing `in` operator (for `for-in` disambiguation).
    fn parse_declarators_no_in(&mut self) -> Vec<Declarator> {
        let saved_no_in = self.context.no_in;
        self.context.no_in = true;
        let decls = self.parse_declarators();
        self.context.no_in = saved_no_in;
        decls
    }

    pub(super) fn parse_switch_statement(&mut self) -> NodeId<Stmt> {
        let start = self.span();
        self.advance(); // skip `switch`
        let _ = self.expect(&TokenKind::LParen);
        let discriminant = self.parse_expression();
        let _ = self.expect(&TokenKind::RParen);
        let _ = self.expect(&TokenKind::LBrace);

        let mut cases = Vec::new();
        let saved = self.context;
        self.context.in_switch = true;
        let mut has_default = false; // A6: track duplicate default

        while !matches!(self.at(), TokenKind::RBrace | TokenKind::Eof) && !self.aborted {
            let case_start = self.span();
            let test = if self.at_keyword(Keyword::Case) {
                self.advance();
                Some(self.parse_expression())
            } else if self.at_keyword(Keyword::Default) {
                // A6: reject multiple default clauses
                if has_default {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "Multiple 'default' clauses in switch statement".into(),
                    );
                }
                has_default = true;
                self.advance();
                None
            } else {
                self.error(
                    JsParseErrorKind::UnexpectedToken,
                    "Expected 'case' or 'default'".into(),
                );
                self.recover_to_statement_start();
                continue;
            };
            let _ = self.expect(&TokenKind::Colon);
            let mut consequent = Vec::new();
            while !self.aborted
                && !matches!(
                    self.at(),
                    TokenKind::Keyword(Keyword::Case | Keyword::Default)
                        | TokenKind::RBrace
                        | TokenKind::Eof
                )
            {
                consequent.push(self.parse_statement_or_declaration());
                if self.check_list_limit(consequent.len(), "statements in switch case") {
                    break;
                }
            }
            let end = consequent
                .last()
                .map_or(case_start, |s| self.stmts.get(*s).span);
            cases.push(SwitchCase {
                test,
                consequent,
                span: case_start.merge(end),
            });
            if self.check_list_limit(cases.len(), "switch cases") {
                break;
            }
        }

        self.context = saved;
        let end = self.span();
        let _ = self.expect(&TokenKind::RBrace);
        self.stmts.alloc(Stmt {
            kind: StmtKind::Switch {
                discriminant,
                cases,
            },
            span: start.merge(end),
        })
    }

    pub(super) fn parse_return_statement(&mut self) -> NodeId<Stmt> {
        let start = self.span();
        if !self.context.in_function {
            self.error(
                JsParseErrorKind::IllegalReturn,
                "'return' outside function".into(),
            );
        }
        self.advance(); // skip `return`

        // No-LineTerminator-here restriction
        let argument = if self.had_newline_before
            || matches!(
                self.at(),
                TokenKind::Semicolon | TokenKind::RBrace | TokenKind::Eof
            ) {
            None
        } else {
            Some(self.parse_expression())
        };
        self.expect_semicolon();
        let end = argument.map_or(start, |a| self.exprs.get(a).span);
        self.stmts.alloc(Stmt {
            kind: StmtKind::Return(argument),
            span: start.merge(end),
        })
    }

    pub(super) fn parse_throw_statement(&mut self) -> NodeId<Stmt> {
        let start = self.span();
        self.advance(); // skip `throw`

        // No-LineTerminator-here restriction
        if self.had_newline_before {
            self.error(
                JsParseErrorKind::UnexpectedToken,
                "No line break allowed after 'throw'".into(),
            );
            return self.error_stmt(start);
        }
        let argument = self.parse_expression();
        self.expect_semicolon();
        let end = self.exprs.get(argument).span;
        self.stmts.alloc(Stmt {
            kind: StmtKind::Throw(argument),
            span: start.merge(end),
        })
    }

    pub(super) fn parse_try_statement(&mut self) -> NodeId<Stmt> {
        let start = self.span();
        self.advance(); // skip `try`

        let block = self.parse_block_body();

        let handler = if self.at_keyword(Keyword::Catch) {
            let catch_start = self.span();
            self.advance();
            let param = if matches!(self.at(), TokenKind::LParen) {
                self.advance();
                let p = self.parse_binding_pattern_direct();
                let _ = self.expect(&TokenKind::RParen);
                Some(p)
            } else {
                None // Optional catch binding (ES2019)
            };
            let body = self.parse_block_body();
            let end = self.span();
            Some(CatchClause {
                param,
                body,
                span: catch_start.merge(end),
            })
        } else {
            None
        };

        let finalizer = if self.at_keyword(Keyword::Finally) {
            self.advance();
            Some(self.parse_block_body())
        } else {
            None
        };

        if handler.is_none() && finalizer.is_none() {
            self.error(
                JsParseErrorKind::UnexpectedToken,
                "Missing catch or finally clause".into(),
            );
        }

        let end = self.span();
        self.stmts.alloc(Stmt {
            kind: StmtKind::Try {
                block,
                handler,
                finalizer,
            },
            span: start.merge(end),
        })
    }

    pub(super) fn parse_break_statement(&mut self) -> NodeId<Stmt> {
        self.parse_break_or_continue(true)
    }

    pub(super) fn parse_continue_statement(&mut self) -> NodeId<Stmt> {
        self.parse_break_or_continue(false)
    }

    fn parse_break_or_continue(&mut self, is_break: bool) -> NodeId<Stmt> {
        let start = self.span();
        self.advance(); // skip `break` / `continue`

        // No-LineTerminator-here + optional label
        let label = if self.had_newline_before {
            None
        } else if let TokenKind::Identifier(name) = *self.at() {
            self.advance();
            Some(name)
        } else {
            None
        };

        // A5: validate label exists, or statement is inside loop(/switch for break)
        // A10: continue with label must refer to an iteration statement
        if let Some(ref lbl) = label {
            let found = self.labels.iter().find(|(l, _)| l == lbl);
            if let Some((_, is_iteration)) = found {
                if !is_break && !is_iteration {
                    // V8: resolve Atom to string for user-facing error message
                    let name = self.resolve(*lbl);
                    self.error(
                        JsParseErrorKind::IllegalBreak,
                        format!(
                            "'continue' label '{name}' does not refer to an iteration statement"
                        ),
                    );
                }
            } else {
                let name = self.resolve(*lbl);
                self.error(
                    JsParseErrorKind::IllegalBreak,
                    format!("Undefined label '{name}'"),
                );
            }
        } else if is_break {
            if !self.context.in_loop && !self.context.in_switch {
                self.error(
                    JsParseErrorKind::IllegalBreak,
                    "'break' outside loop or switch".into(),
                );
            }
        } else if !self.context.in_loop {
            self.error(
                JsParseErrorKind::IllegalBreak,
                "'continue' outside loop".into(),
            );
        }

        self.expect_semicolon();
        let kind = if is_break {
            StmtKind::Break(label)
        } else {
            StmtKind::Continue(label)
        };
        self.stmts.alloc(Stmt {
            kind,
            span: start.merge(self.span()),
        })
    }

    pub(super) fn parse_with_statement(&mut self) -> NodeId<Stmt> {
        // B18: elidex is always strict — `with` is a syntax error
        let start = self.span();
        self.error(
            JsParseErrorKind::UnexpectedToken,
            "'with' statement is not allowed in strict mode".into(),
        );
        self.advance(); // skip `with`
        let _ = self.expect(&TokenKind::LParen);
        let object = self.parse_expression();
        let _ = self.expect(&TokenKind::RParen);
        let body = self.parse_sub_statement();
        let end = self.stmts.get(body).span;
        self.stmts.alloc(Stmt {
            kind: StmtKind::With { object, body },
            span: start.merge(end),
        })
    }
}
