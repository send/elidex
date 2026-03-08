//! Pratt expression parser.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::error::JsParseErrorKind;
use crate::span::Span;
use crate::token::{Keyword, TokenKind};

use super::Parser;

/// Binding power (precedence) levels.
/// Higher = tighter binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Bp(pub(crate) u8);

impl Bp {
    const ASSIGN: Self = Self(2);
    const CONDITIONAL: Self = Self(3);
    const NULL_COAL: Self = Self(4);
    const OR: Self = Self(5);
    const AND: Self = Self(6);
    const BIT_OR: Self = Self(7);
    const BIT_XOR: Self = Self(8);
    const BIT_AND: Self = Self(9);
    const EQUALITY: Self = Self(10);
    const RELATIONAL: Self = Self(11);
    const SHIFT: Self = Self(12);
    const ADDITIVE: Self = Self(13);
    const MULTIPLICATIVE: Self = Self(14);
    const EXPONENTIAL: Self = Self(15);
    const UNARY: Self = Self(16);
    const POSTFIX: Self = Self(17);
}

impl Parser<'_> {
    // ── Public entry points ──────────────────────────────────────────

    /// Parse an expression (assignment level, no comma).
    pub(crate) fn parse_assignment_expression(&mut self) -> NodeId<Expr> {
        let result = self.parse_expr_bp(Bp::ASSIGN);
        // T2: CoverInitializedName — if `{x = 1}` was not consumed by `=`, it's an error
        if let Some(span) = self.cover_init_span.take() {
            self.error_at(
                JsParseErrorKind::UnexpectedToken,
                "Shorthand property initializer is only valid in destructuring assignment".into(),
                span,
            );
        }
        result
    }

    /// Parse an expression including comma sequences.
    pub(crate) fn parse_expression(&mut self) -> NodeId<Expr> {
        let first = self.parse_assignment_expression();
        if !matches!(self.at(), TokenKind::Comma) {
            return first;
        }
        let start = self.exprs.get(first).span;
        let mut exprs = vec![first];
        while matches!(self.at(), TokenKind::Comma) && !self.aborted {
            self.advance();
            exprs.push(self.parse_assignment_expression());
            if self.check_list_limit(exprs.len(), "sequence expressions") {
                break;
            }
        }
        let end = self
            .exprs
            .get(*exprs.last().expect("sequence has at least two elements"))
            .span;
        self.exprs.alloc(Expr {
            kind: ExprKind::Sequence(exprs),
            span: start.merge(end),
        })
    }

    // ── Pratt parser core ────────────────────────────────────────────

    fn parse_expr_bp(&mut self, min_bp: Bp) -> NodeId<Expr> {
        if !self.enter_recursion() {
            return self.error_expr(self.span());
        }
        let lhs = self.parse_prefix();
        let result = self.parse_expr_bp_from(lhs, min_bp);
        self.leave_recursion();
        result
    }

    /// Continue Pratt parsing from an already-parsed LHS expression.
    /// This is the infix loop extracted for reuse by `continue_expression_from`.
    pub(crate) fn parse_expr_bp_from(&mut self, mut lhs: NodeId<Expr>, min_bp: Bp) -> NodeId<Expr> {
        loop {
            if self.aborted {
                break;
            }

            // Postfix operators
            if matches!(self.at(), TokenKind::PlusPlus | TokenKind::MinusMinus)
                && !self.had_newline_before
                && min_bp <= Bp::POSTFIX
            {
                // V1/V9: validate update target
                self.validate_assign_target(
                    lhs,
                    false,
                    "Invalid left-hand side in postfix operation",
                );
                let op = if matches!(self.at(), TokenKind::PlusPlus) {
                    UpdateOp::Increment
                } else {
                    UpdateOp::Decrement
                };
                let op_span = self.span();
                self.advance();
                let lhs_span = self.exprs.get(lhs).span;
                lhs = self.exprs.alloc(Expr {
                    kind: ExprKind::Update {
                        op,
                        prefix: false,
                        argument: lhs,
                    },
                    span: lhs_span.merge(op_span),
                });
                continue;
            }

            // Infix operators
            if let Some((op_bp, assoc)) = self.infix_bp() {
                if op_bp < min_bp {
                    break;
                }
                let next_bp = if assoc == Assoc::Right {
                    Bp(op_bp.0) // right-associative: same bp
                } else {
                    Bp(op_bp.0 + 1) // left-associative: higher bp
                };

                lhs = self.parse_infix(lhs, next_bp);
                continue;
            }

            break;
        }

        lhs
    }

    /// Parse a prefix expression (unary, primary, or prefix update).
    #[allow(clippy::too_many_lines)]
    fn parse_prefix(&mut self) -> NodeId<Expr> {
        let start = self.span();

        match *self.at() {
            // Prefix unary
            TokenKind::Not => self.parse_unary_prefix(UnaryOp::Not, start),
            TokenKind::Tilde => self.parse_unary_prefix(UnaryOp::BitwiseNot, start),
            TokenKind::Plus => self.parse_unary_prefix(UnaryOp::Plus, start),
            TokenKind::Minus => self.parse_unary_prefix(UnaryOp::Minus, start),
            TokenKind::Keyword(Keyword::Typeof) => self.parse_unary_prefix(UnaryOp::Typeof, start),
            TokenKind::Keyword(Keyword::Void) => self.parse_unary_prefix(UnaryOp::Void, start),
            TokenKind::Keyword(Keyword::Delete) => {
                self.advance();
                let arg = self.parse_expr_bp(Bp::UNARY);
                // B19: `delete identifier` is a syntax error in strict mode (elidex always strict)
                if matches!(self.exprs.get(arg).kind, ExprKind::Identifier(_)) {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "Cannot delete an unqualified identifier in strict mode".into(),
                    );
                }
                let end = self.exprs.get(arg).span;
                self.exprs.alloc(Expr {
                    kind: ExprKind::Unary {
                        op: UnaryOp::Delete,
                        argument: arg,
                    },
                    span: start.merge(end),
                })
            }
            // Prefix update
            TokenKind::PlusPlus | TokenKind::MinusMinus => {
                let op = if matches!(self.at(), TokenKind::PlusPlus) {
                    UpdateOp::Increment
                } else {
                    UpdateOp::Decrement
                };
                self.advance();
                let arg = self.parse_expr_bp(Bp::UNARY);
                // V1/V9: validate update target
                self.validate_assign_target(
                    arg,
                    false,
                    "Invalid left-hand side in prefix operation",
                );
                let end = self.exprs.get(arg).span;
                self.exprs.alloc(Expr {
                    kind: ExprKind::Update {
                        op,
                        prefix: true,
                        argument: arg,
                    },
                    span: start.merge(end),
                })
            }
            // Await — B21: also valid at top level in modules (not inside sync functions)
            TokenKind::Identifier(s)
                if s == self.atoms.r#await && self.context.await_is_keyword() =>
            {
                // T1: §15.8.3.1 — `await` in async function default parameters is a syntax error
                if self.context.in_async_params {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "'await' expression is not allowed in async function parameters".into(),
                    );
                }
                self.advance();
                let arg = self.parse_expr_bp(Bp::UNARY);
                let end = self.exprs.get(arg).span;
                self.exprs.alloc(Expr {
                    kind: ExprKind::Await(arg),
                    span: start.merge(end),
                })
            }
            // S3: yield is a reserved keyword in strict mode; parse as expression in generators
            TokenKind::Keyword(Keyword::Yield) if self.context.in_generator => {
                self.parse_yield_expression(start)
            }
            // new — apply suffix loop so `new Foo().bar` works
            TokenKind::Keyword(Keyword::New) => {
                let new_expr = self.parse_new_expression();
                self.parse_suffix_loop(new_expr, true)
            }
            // B5: #x in obj — private field membership test (ES2022)
            TokenKind::PrivateIdentifier(name) => {
                self.advance();
                if !self.context.no_in && self.at_keyword(Keyword::In) {
                    self.advance();
                    let right = self.parse_expr_bp(Bp::RELATIONAL);
                    let end = self.exprs.get(right).span;
                    self.exprs.alloc(Expr {
                        kind: ExprKind::PrivateIn { name, right },
                        span: start.merge(end),
                    })
                } else {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "Private identifier '#' can only appear in 'in' expression".into(),
                    );
                    self.error_expr(start)
                }
            }
            // Spread (in arguments/array context)
            TokenKind::Ellipsis => {
                self.advance();
                let arg = self.parse_assignment_expression();
                let end = self.exprs.get(arg).span;
                self.exprs.alloc(Expr {
                    kind: ExprKind::Spread(arg),
                    span: start.merge(end),
                })
            }
            _ => self.parse_primary_and_suffix(),
        }
    }

    fn parse_yield_expression(&mut self, start: Span) -> NodeId<Expr> {
        // E3: §14.4 — yield expression is not allowed in generator parameter defaults
        if self.context.in_generator_params {
            self.error(
                JsParseErrorKind::UnexpectedToken,
                "'yield' expression is not allowed in generator function parameters".into(),
            );
        }
        self.advance();
        let delegate = matches!(self.at(), TokenKind::Star) && !self.had_newline_before;
        if delegate {
            self.advance();
        }
        let argument = if !self.had_newline_before
            && !matches!(
                self.at(),
                TokenKind::Semicolon
                    | TokenKind::RBrace
                    | TokenKind::RParen
                    | TokenKind::RBracket
                    | TokenKind::Comma
                    | TokenKind::Colon
                    | TokenKind::Eof
            ) {
            Some(self.parse_assignment_expression())
        } else if delegate {
            self.error(
                JsParseErrorKind::UnexpectedToken,
                "Expected expression after yield*".into(),
            );
            Some(self.error_expr(start))
        } else {
            None
        };
        let end = argument.map_or(start, |a| self.exprs.get(a).span);
        self.exprs.alloc(Expr {
            kind: ExprKind::Yield { argument, delegate },
            span: start.merge(end),
        })
    }

    // ── Infix parsing ────────────────────────────────────────────────

    fn infix_bp(&self) -> Option<(Bp, Assoc)> {
        match self.at() {
            // Assignment operators (right-associative)
            TokenKind::Eq
            | TokenKind::PlusEq
            | TokenKind::MinusEq
            | TokenKind::StarEq
            | TokenKind::SlashEq
            | TokenKind::PercentEq
            | TokenKind::ExpEq
            | TokenKind::AmpEq
            | TokenKind::PipeEq
            | TokenKind::CaretEq
            | TokenKind::ShlEq
            | TokenKind::ShrEq
            | TokenKind::UShrEq
            | TokenKind::AndEq
            | TokenKind::OrEq
            | TokenKind::NullCoalEq => Some((Bp::ASSIGN, Assoc::Right)),

            // Conditional (ternary) — handled specially
            TokenKind::Question => Some((Bp::CONDITIONAL, Assoc::Right)),

            // Null coalescing
            TokenKind::NullCoal => Some((Bp::NULL_COAL, Assoc::Left)),

            // Logical OR
            TokenKind::Or => Some((Bp::OR, Assoc::Left)),
            // Logical AND
            TokenKind::And => Some((Bp::AND, Assoc::Left)),
            // Bitwise OR
            TokenKind::Pipe => Some((Bp::BIT_OR, Assoc::Left)),
            // Bitwise XOR
            TokenKind::Caret => Some((Bp::BIT_XOR, Assoc::Left)),
            // Bitwise AND
            TokenKind::Amp => Some((Bp::BIT_AND, Assoc::Left)),

            // Equality
            TokenKind::EqEq | TokenKind::NotEq | TokenKind::StrictEq | TokenKind::StrictNe => {
                Some((Bp::EQUALITY, Assoc::Left))
            }

            // Relational
            TokenKind::Lt
            | TokenKind::Gt
            | TokenKind::LtEq
            | TokenKind::GtEq
            | TokenKind::Keyword(Keyword::Instanceof) => Some((Bp::RELATIONAL, Assoc::Left)),
            // `in` operator — suppressed in no_in context (for-in disambiguation)
            TokenKind::Keyword(Keyword::In) if !self.context.no_in => {
                Some((Bp::RELATIONAL, Assoc::Left))
            }

            // Shift
            TokenKind::Shl | TokenKind::Shr | TokenKind::UShr => Some((Bp::SHIFT, Assoc::Left)),

            // Additive
            TokenKind::Plus | TokenKind::Minus => Some((Bp::ADDITIVE, Assoc::Left)),

            // Multiplicative
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => {
                Some((Bp::MULTIPLICATIVE, Assoc::Left))
            }

            // Exponentiation (right-associative)
            TokenKind::Exp => Some((Bp::EXPONENTIAL, Assoc::Right)),

            _ => None,
        }
    }

    #[allow(clippy::too_many_lines)]
    fn parse_infix(&mut self, lhs: NodeId<Expr>, next_bp: Bp) -> NodeId<Expr> {
        let lhs_span = self.exprs.get(lhs).span;

        match *self.at() {
            // Assignment
            TokenKind::Eq => {
                // A10/V9: validate assignment target (= allows destructuring cover grammar)
                self.validate_assign_target(
                    lhs,
                    true,
                    "Invalid left-hand side in assignment",
                );
                // T2: object used as destructuring target — clear CoverInitializedName
                if matches!(self.exprs.get(lhs).kind, ExprKind::Object(_)) {
                    self.cover_init_span = None;
                }
                self.advance();
                let rhs = self.parse_expr_bp(next_bp);
                let end = self.exprs.get(rhs).span;
                self.exprs.alloc(Expr {
                    kind: ExprKind::Assignment {
                        left: AssignTarget::Simple(lhs),
                        op: AssignOp::Assign,
                        right: rhs,
                    },
                    span: lhs_span.merge(end),
                })
            }
            tok @ (TokenKind::PlusEq
            | TokenKind::MinusEq
            | TokenKind::StarEq
            | TokenKind::SlashEq
            | TokenKind::PercentEq
            | TokenKind::ExpEq
            | TokenKind::AmpEq
            | TokenKind::PipeEq
            | TokenKind::CaretEq
            | TokenKind::ShlEq
            | TokenKind::ShrEq
            | TokenKind::UShrEq
            | TokenKind::AndEq
            | TokenKind::OrEq
            | TokenKind::NullCoalEq) => {
                // A10/V9: compound assignment requires simple target (no destructuring)
                self.validate_assign_target(
                    lhs,
                    false,
                    "Invalid left-hand side in assignment",
                );
                let op = assign_op_from_token(&tok);
                self.advance();
                let rhs = self.parse_expr_bp(next_bp);
                let end = self.exprs.get(rhs).span;
                self.exprs.alloc(Expr {
                    kind: ExprKind::Assignment {
                        left: AssignTarget::Simple(lhs),
                        op,
                        right: rhs,
                    },
                    span: lhs_span.merge(end),
                })
            }
            // Conditional (ternary)
            TokenKind::Question => {
                self.advance();
                let consequent = self.parse_assignment_expression();
                let _ = self.expect(&TokenKind::Colon);
                let alternate = self.parse_assignment_expression();
                let end = self.exprs.get(alternate).span;
                self.exprs.alloc(Expr {
                    kind: ExprKind::Conditional {
                        test: lhs,
                        consequent,
                        alternate,
                    },
                    span: lhs_span.merge(end),
                })
            }
            // Logical operators
            TokenKind::And => self.parse_logical_infix(lhs, LogicalOp::And, next_bp, lhs_span),
            TokenKind::Or => self.parse_logical_infix(lhs, LogicalOp::Or, next_bp, lhs_span),
            TokenKind::NullCoal => {
                self.parse_logical_infix(lhs, LogicalOp::NullCoal, next_bp, lhs_span)
            }
            // Binary operators
            tok => {
                let op = binary_op_from_token(&tok);
                // A6/A9: unary/await expression ** exponentiation is a syntax error
                if op == BinaryOp::Exp
                    && matches!(
                        self.exprs.get(lhs).kind,
                        ExprKind::Unary { .. } | ExprKind::Await(_)
                    )
                {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "Unary operator or 'await' before '**' requires parentheses".into(),
                    );
                }
                self.advance();
                let rhs = self.parse_expr_bp(next_bp);
                let end = self.exprs.get(rhs).span;
                self.exprs.alloc(Expr {
                    kind: ExprKind::Binary {
                        left: lhs,
                        op,
                        right: rhs,
                    },
                    span: lhs_span.merge(end),
                })
            }
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────

    /// Validate that `id` is a valid assignment/update target and check eval/arguments restriction.
    /// `allow_destructuring`: true for `=` (Object/Array cover grammar), false for compound/update.
    pub(super) fn validate_assign_target(
        &mut self,
        id: NodeId<Expr>,
        allow_destructuring: bool,
        msg: &str,
    ) {
        if !self.is_valid_assign_target(id, allow_destructuring) {
            self.error(JsParseErrorKind::InvalidAssignmentTarget, msg.into());
        }
        self.check_eval_arguments_target(id);
    }

    /// V9: Check if an identifier expression is `eval` or `arguments` (strict mode restriction).
    /// elidex is always strict, so these are never valid assignment/update targets.
    fn check_eval_arguments_target(&mut self, id: NodeId<Expr>) {
        // Unwrap parens to reach the inner identifier
        let mut current = id;
        loop {
            match &self.exprs.get(current).kind {
                ExprKind::Identifier(name)
                    if *name == self.atoms.eval || *name == self.atoms.arguments =>
                {
                    let name_str = self.resolve(*name);
                    self.error(
                        JsParseErrorKind::StrictModeViolation,
                        format!("Cannot assign to '{name_str}' in strict mode"),
                    );
                    return;
                }
                ExprKind::Paren(inner) => current = *inner,
                _ => return,
            }
        }
    }

    /// A10: Check if an expression is a valid assignment target.
    /// `allow_destructuring`: true for `=` (Object/Array as cover grammar), false for compound `+=` etc.
    pub(super) fn is_valid_assign_target(
        &self,
        id: NodeId<Expr>,
        allow_destructuring: bool,
    ) -> bool {
        // M2: iterative Paren unwrap to avoid stack overflow on deep nesting
        let mut current = id;
        let mut seen_paren = false;
        loop {
            match &self.exprs.get(current).kind {
                ExprKind::Identifier(_) | ExprKind::Member { .. } => return true,
                ExprKind::Paren(inner) => {
                    seen_paren = true;
                    current = *inner;
                }
                // S1: Object/Array destructuring is only valid as a direct assignment target,
                // not through parentheses — `({x} = obj)` is valid but `(({x})) = obj` is not.
                ExprKind::Object(_) | ExprKind::Array(_) if allow_destructuring && !seen_paren => {
                    return true;
                }
                _ => return false,
            }
        }
    }

    /// Parse a unary prefix expression (`!x`, `~x`, `+x`, `-x`, `typeof x`, `void x`).
    fn parse_unary_prefix(&mut self, op: UnaryOp, start: Span) -> NodeId<Expr> {
        self.advance();
        let arg = self.parse_expr_bp(Bp::UNARY);
        let end = self.exprs.get(arg).span;
        self.exprs.alloc(Expr {
            kind: ExprKind::Unary { op, argument: arg },
            span: start.merge(end),
        })
    }

    /// Parse a logical infix expression (`&&`, `||`, `??`) with B8 mixing check.
    fn parse_logical_infix(
        &mut self,
        lhs: NodeId<Expr>,
        op: LogicalOp,
        next_bp: Bp,
        lhs_span: Span,
    ) -> NodeId<Expr> {
        // B8: check LHS for forbidden mixing
        let lhs_forbidden = match op {
            LogicalOp::And | LogicalOp::Or => matches!(
                self.exprs.get(lhs).kind,
                ExprKind::Logical {
                    op: LogicalOp::NullCoal,
                    ..
                }
            ),
            LogicalOp::NullCoal => matches!(
                self.exprs.get(lhs).kind,
                ExprKind::Logical {
                    op: LogicalOp::And | LogicalOp::Or,
                    ..
                }
            ),
        };
        if lhs_forbidden {
            self.error(
                JsParseErrorKind::UnexpectedToken,
                format!(
                    "Cannot mix '??' with '{}' without parentheses",
                    match op {
                        LogicalOp::And => "&&",
                        LogicalOp::Or => "||",
                        LogicalOp::NullCoal => "&&'/'||",
                    }
                ),
            );
        }
        self.advance();
        let rhs = self.parse_expr_bp(next_bp);
        // B8: NullCoal also checks RHS (higher-precedence && / || binds first)
        if op == LogicalOp::NullCoal
            && matches!(
                self.exprs.get(rhs).kind,
                ExprKind::Logical {
                    op: LogicalOp::And | LogicalOp::Or,
                    ..
                }
            )
        {
            self.error(
                JsParseErrorKind::UnexpectedToken,
                "Cannot mix '??' with '&&'/'||' without parentheses".into(),
            );
        }
        let end = self.exprs.get(rhs).span;
        self.exprs.alloc(Expr {
            kind: ExprKind::Logical {
                left: lhs,
                op,
                right: rhs,
            },
            span: lhs_span.merge(end),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Assoc {
    Left,
    Right,
}

fn assign_op_from_token(tok: &TokenKind) -> AssignOp {
    match tok {
        TokenKind::PlusEq => AssignOp::AddAssign,
        TokenKind::MinusEq => AssignOp::SubAssign,
        TokenKind::StarEq => AssignOp::MulAssign,
        TokenKind::SlashEq => AssignOp::DivAssign,
        TokenKind::PercentEq => AssignOp::ModAssign,
        TokenKind::ExpEq => AssignOp::ExpAssign,
        TokenKind::AmpEq => AssignOp::BitAndAssign,
        TokenKind::PipeEq => AssignOp::BitOrAssign,
        TokenKind::CaretEq => AssignOp::BitXorAssign,
        TokenKind::ShlEq => AssignOp::ShlAssign,
        TokenKind::ShrEq => AssignOp::ShrAssign,
        TokenKind::UShrEq => AssignOp::UShrAssign,
        TokenKind::AndEq => AssignOp::AndAssign,
        TokenKind::OrEq => AssignOp::OrAssign,
        TokenKind::NullCoalEq => AssignOp::NullCoalAssign,
        _ => AssignOp::Assign,
    }
}

fn binary_op_from_token(tok: &TokenKind) -> BinaryOp {
    match tok {
        TokenKind::Minus => BinaryOp::Sub,
        TokenKind::Star => BinaryOp::Mul,
        TokenKind::Slash => BinaryOp::Div,
        TokenKind::Percent => BinaryOp::Mod,
        TokenKind::Exp => BinaryOp::Exp,
        TokenKind::Shl => BinaryOp::Shl,
        TokenKind::Shr => BinaryOp::Shr,
        TokenKind::UShr => BinaryOp::UShr,
        TokenKind::Amp => BinaryOp::BitAnd,
        TokenKind::Pipe => BinaryOp::BitOr,
        TokenKind::Caret => BinaryOp::BitXor,
        TokenKind::EqEq => BinaryOp::Eq,
        TokenKind::NotEq => BinaryOp::NotEq,
        TokenKind::StrictEq => BinaryOp::StrictEq,
        TokenKind::StrictNe => BinaryOp::StrictNotEq,
        TokenKind::Lt => BinaryOp::Lt,
        TokenKind::LtEq => BinaryOp::LtEq,
        TokenKind::Gt => BinaryOp::Gt,
        TokenKind::GtEq => BinaryOp::GtEq,
        TokenKind::Keyword(Keyword::In) => BinaryOp::In,
        TokenKind::Keyword(Keyword::Instanceof) => BinaryOp::Instanceof,
        // S5: Plus + defensive fallback (caller ensures only binary op tokens reach here)
        _ => BinaryOp::Add,
    }
}

#[cfg(test)]
#[path = "expr_tests.rs"]
mod tests;
