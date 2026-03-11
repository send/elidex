//! Statement parser with ASI support and error recovery.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
// AST module exports 50+ node types used pervasively in parser.
use crate::ast::*;
use crate::atom::Atom;
use crate::error::JsParseErrorKind;
use crate::token::{Keyword, TokenKind};

use super::Parser;

impl Parser<'_> {
    /// B3/B4: Parse a sub-statement (body of if/while/for/do/with/labeled).
    /// Rejects lexical declarations, class declarations, and function declarations
    /// in single-statement positions (ES2023 strict mode).
    pub(crate) fn parse_sub_statement(&mut self) -> NodeId<Stmt> {
        match self.at() {
            TokenKind::Keyword(Keyword::Let | Keyword::Const) => {
                self.error(
                    JsParseErrorKind::UnexpectedToken,
                    "Lexical declarations can only appear at the top level of a block".into(),
                );
            }
            TokenKind::Keyword(Keyword::Class) => {
                self.error(
                    JsParseErrorKind::UnexpectedToken,
                    "Class declarations can only appear at the top level of a block".into(),
                );
            }
            // B3: function declarations in sub-statement position are not allowed in strict mode
            // (Annex B relaxes this for non-strict, but elidex is always strict)
            TokenKind::Keyword(Keyword::Function) => {
                self.error(
                    JsParseErrorKind::UnexpectedToken,
                    "In strict mode, function declarations can only appear at the top level of a block".into(),
                );
            }
            _ => {}
        }
        self.parse_statement_or_declaration()
    }

    /// Parse a statement (not a declaration — called from `parse_statement_or_declaration`).
    pub(crate) fn parse_statement(&mut self) -> NodeId<Stmt> {
        match *self.at() {
            TokenKind::LBrace => self.parse_block_statement(),
            TokenKind::Semicolon => {
                let span = self.span();
                self.advance();
                self.stmts.alloc(Stmt {
                    kind: StmtKind::Empty,
                    span,
                })
            }
            TokenKind::Keyword(Keyword::If) => self.parse_if_statement(),
            TokenKind::Keyword(Keyword::While) => self.parse_while_statement(),
            TokenKind::Keyword(Keyword::Do) => self.parse_do_while_statement(),
            TokenKind::Keyword(Keyword::For) => self.parse_for_statement(),
            TokenKind::Keyword(Keyword::Switch) => self.parse_switch_statement(),
            TokenKind::Keyword(Keyword::Return) => self.parse_return_statement(),
            TokenKind::Keyword(Keyword::Throw) => self.parse_throw_statement(),
            TokenKind::Keyword(Keyword::Try) => self.parse_try_statement(),
            TokenKind::Keyword(Keyword::Break) => self.parse_break_statement(),
            TokenKind::Keyword(Keyword::Continue) => self.parse_continue_statement(),
            TokenKind::Keyword(Keyword::With) => self.parse_with_statement(),
            TokenKind::Keyword(Keyword::Debugger) => {
                let span = self.span();
                self.advance();
                self.expect_semicolon();
                self.stmts.alloc(Stmt {
                    kind: StmtKind::Debugger,
                    span,
                })
            }
            // Labeled statement: `label: stmt`
            TokenKind::Identifier(name) => {
                // A9/B21: `await`/`yield` in async/module/generator context are prefix unary —
                // must go through parse_prefix() to be recognized.
                if (name == self.atoms.r#await && self.context.await_is_keyword())
                    || (name == self.atoms.r#yield && self.context.in_generator)
                {
                    return self.parse_expression_statement();
                }
                // Need to check if this is `ident:` (labeled) or an expression statement
                // We can peek ahead but the lexer doesn't have a peek.
                // Instead, parse as expression. If it's just an identifier followed by `:`,
                // we handle it by checking.
                self.parse_possible_labeled_or_expression(name)
            }
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_possible_labeled_or_expression(&mut self, name: Atom) -> NodeId<Stmt> {
        let start = self.span();

        // Try: if current is identifier and next might be colon
        // We already know current is Identifier(name)
        self.advance(); // consume the identifier

        if matches!(self.at(), TokenKind::Colon) {
            // Labeled statement
            self.advance(); // consume :
                            // B3: reject duplicate labels at same nesting level
            if self.labels.iter().any(|(l, _)| *l == name) {
                self.error(
                    JsParseErrorKind::UnexpectedToken,
                    format!("Duplicate label '{}'", self.resolve(name)),
                );
            }
            // A10: track whether label is for an iteration statement
            let is_iteration = matches!(
                self.at(),
                TokenKind::Keyword(Keyword::For | Keyword::While | Keyword::Do)
            );
            self.labels.push((name, is_iteration));
            // B3/B4: use parse_sub_statement to reject declarations in labeled position
            let body = self.parse_sub_statement();
            self.labels.pop();
            let end = self.stmts.get(body).span;
            return self.stmts.alloc(Stmt {
                kind: StmtKind::Labeled { label: name, body },
                span: start.merge(end),
            });
        }

        // Not a label — it's an expression starting with this identifier
        // Create identifier expression and continue parsing as expression statement
        let ident_expr = self.exprs.alloc(Expr {
            kind: ExprKind::Identifier(name),
            span: start,
        });

        // Check for arrow: `name =>`
        if matches!(self.at(), TokenKind::Arrow) && !self.had_newline_before {
            let arrow = self.parse_arrow_from_single_param(name, start);
            self.expect_semicolon();
            return self.stmts.alloc(Stmt {
                kind: StmtKind::Expression(arrow),
                span: start.merge(self.span()),
            });
        }

        // Continue parsing suffix and infix of the expression
        let expr = self.continue_expression_from(ident_expr);
        self.expect_semicolon();
        let end = self.exprs.get(expr).span;
        self.stmts.alloc(Stmt {
            kind: StmtKind::Expression(expr),
            span: start.merge(end),
        })
    }

    /// Continue parsing an expression that started with a primary already parsed.
    /// Applies suffix operators (member, call, etc.) then delegates to the Pratt
    /// parser for correct precedence handling of infix operators (H1-H3 fix).
    pub(crate) fn continue_expression_from(&mut self, primary: NodeId<Expr>) -> NodeId<Expr> {
        use super::expr::Bp;
        let expr = self.parse_suffix_loop(primary, true);
        // Bp(1) is below Bp::ASSIGN(2), so all operators will be parsed correctly.
        self.parse_expr_bp_from(expr, Bp(1))
    }

    /// Parse expression statement (expression `;`).
    pub(crate) fn parse_expression_statement(&mut self) -> NodeId<Stmt> {
        let start = self.span();
        let expr = self.parse_expression();
        self.expect_semicolon();
        let end = self.exprs.get(expr).span;
        self.stmts.alloc(Stmt {
            kind: StmtKind::Expression(expr),
            span: start.merge(end),
        })
    }

    // ── Variable Declaration ─────────────────────────────────────────

    /// R10: Consume a `var`/`let`/`const` keyword and return the corresponding `VarKind`.
    pub(super) fn parse_var_kind(&mut self) -> VarKind {
        let kind = match self.at() {
            TokenKind::Keyword(Keyword::Let) => VarKind::Let,
            TokenKind::Keyword(Keyword::Const) => VarKind::Const,
            _ => VarKind::Var,
        };
        self.advance();
        kind
    }

    pub(crate) fn parse_variable_declaration_stmt(&mut self) -> NodeId<Stmt> {
        let start = self.span();
        let kind = self.parse_var_kind();

        let declarators = self.parse_declarators();

        // B7: const declarations must have an initializer
        if kind == VarKind::Const {
            for d in &declarators {
                if d.init.is_none() {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "Missing initializer in const declaration".into(),
                    );
                }
            }
        }

        self.expect_semicolon();
        let end = declarators.last().map_or(start, |d| d.span);
        self.stmts.alloc(Stmt {
            kind: StmtKind::VariableDeclaration { kind, declarators },
            span: start.merge(end),
        })
    }

    pub(crate) fn parse_declarators(&mut self) -> Vec<Declarator> {
        let mut declarators = Vec::new();
        loop {
            if self.aborted {
                break;
            }
            let decl_start = self.span();
            let pattern = self.parse_binding_pattern_direct();
            let init = if matches!(self.at(), TokenKind::Eq) {
                self.advance();
                Some(self.parse_assignment_expression())
            } else {
                None
            };
            let end = init.map_or_else(
                || self.patterns.get(pattern).span,
                |i| self.exprs.get(i).span,
            );
            declarators.push(Declarator {
                pattern,
                init,
                span: decl_start.merge(end),
            });
            if self.check_list_limit(declarators.len(), "declarators") {
                break;
            }
            if matches!(self.at(), TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        declarators
    }

    // ── Block ────────────────────────────────────────────────────────

    pub(crate) fn parse_block_statement(&mut self) -> NodeId<Stmt> {
        let start = self.span();
        let body = self.parse_block_body();
        let end = self.span();
        self.stmts.alloc(Stmt {
            kind: StmtKind::Block(body),
            span: start.merge(end),
        })
    }

    pub(crate) fn parse_block_body(&mut self) -> Vec<NodeId<Stmt>> {
        let _ = self.expect(&TokenKind::LBrace);
        let mut stmts = Vec::new();
        while !matches!(self.at(), TokenKind::RBrace | TokenKind::Eof) && !self.aborted {
            stmts.push(self.parse_statement_or_declaration());
            if self.check_list_limit(stmts.len(), "block statements") {
                break;
            }
        }
        let _ = self.expect(&TokenKind::RBrace);
        stmts
    }
}

#[cfg(test)]
#[path = "stmt_tests.rs"]
mod tests;
