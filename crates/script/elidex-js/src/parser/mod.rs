//! Recursive-descent parser with Pratt expression parsing.
//!
//! Always strict mode. No Annex B (see crate-level docs).

mod arrow;
mod control_flow;
mod expr;
mod function;
mod module;
mod object;
mod pattern;
mod primary;
mod stmt;

use crate::arena::{Arena, NodeId};
#[allow(clippy::wildcard_imports)]
// AST module exports 50+ node types used pervasively in parser.
use crate::ast::*;
use crate::atom::{Atom, WellKnownAtoms};
use crate::error::{JsParseError, JsParseErrorKind, ParseOutput, MAX_ERRORS, MAX_NESTING_DEPTH};
use crate::lexer::Lexer;
use crate::span::Span;
use crate::token::{Keyword, Token, TokenKind};

/// Maximum number of items in parser-collected lists (args, params, cases, etc.).
/// Prevents unbounded `Vec` growth on pathological input.
pub(crate) const MAX_LIST_ITEMS: usize = 65536;

/// Parse context flags (bitfield-style struct for readability).
#[allow(clippy::struct_excessive_bools)] // Parser context tracks independent boolean flags per spec grammar.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ParseContext {
    pub(crate) in_function: bool,
    pub(crate) in_async: bool,
    pub(crate) in_generator: bool,
    pub(crate) in_loop: bool,
    pub(crate) in_switch: bool,
    pub(crate) is_module: bool,
    pub(crate) no_in: bool,
    /// A7: true when inside a class constructor (allows `super()` calls).
    pub(crate) in_constructor: bool,
    /// A5: true when at the top level of a module (import/export allowed).
    pub(crate) at_top_level: bool,
    /// P3: true when inside a class static block (allows `super.prop` but not `super()` or `arguments`).
    pub(crate) in_static_block: bool,
    /// T1: true when parsing formal parameters of an async function.
    /// `await` is a keyword but cannot be used as an expression in this context.
    pub(crate) in_async_params: bool,
    /// E3: true when parsing formal parameters of a generator function.
    /// `yield` is a keyword but cannot be used as an expression in this context.
    pub(crate) in_generator_params: bool,
    /// S4: true when inside a method (class or object literal) — allows `super.prop`.
    pub(crate) in_method: bool,
    /// V11: true when inside a constructor of a class with `extends`.
    pub(crate) in_derived_constructor: bool,
}

impl ParseContext {
    /// B21: `await` is a keyword (not an identifier) in async contexts
    /// and at the top level of modules.
    pub(crate) fn await_is_keyword(&self) -> bool {
        self.in_async || self.in_async_params || (self.is_module && !self.in_function)
    }
}

pub(crate) struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    had_newline_before: bool,
    /// One-token lookahead buffer for `peek_kind()`.
    peeked: Option<(Token, bool)>,
    pub(crate) context: ParseContext,
    pub(crate) errors: Vec<JsParseError>,
    pub(crate) stmts: Arena<Stmt>,
    pub(crate) exprs: Arena<Expr>,
    pub(crate) patterns: Arena<Pattern>,
    /// Pre-interned atoms for fast contextual keyword comparison.
    pub(crate) atoms: WellKnownAtoms,
    /// A5: Active labels for break/continue validation. (name, `is_iteration_label`)
    labels: Vec<(Atom, bool)>,
    /// T2: Span of a `CoverInitializedName` (`{x = 1}`) in an object literal.
    /// Set during parse, cleared when consumed by destructuring assignment.
    cover_init_span: Option<Span>,
    aborted: bool,
    /// Recursion depth counter for stack overflow prevention.
    depth: u32,
}

impl<'a> Parser<'a> {
    /// Maximum source size (`u32::MAX` bytes) to prevent span truncation.
    const MAX_SOURCE_LEN: usize = u32::MAX as usize;

    pub(crate) fn new(source: &'a str, kind: ProgramKind) -> Self {
        let mut lexer = Lexer::new(source);
        let atoms = WellKnownAtoms::new(&mut lexer.interner);
        let (first_token, had_newline) = lexer.next_token();
        let mut parser = Self {
            lexer,
            current: first_token,
            had_newline_before: had_newline,
            peeked: None,
            atoms,
            context: ParseContext {
                is_module: kind == ProgramKind::Module,
                at_top_level: kind == ProgramKind::Module,
                ..Default::default()
            },
            errors: Vec::new(),
            stmts: Arena::new(),
            exprs: Arena::new(),
            patterns: Arena::new(),
            labels: Vec::new(),
            cover_init_span: None,
            aborted: false,
            depth: 0,
        };
        // S13: reject oversized input that would cause span truncation
        if source.len() > Self::MAX_SOURCE_LEN {
            parser.error(
                JsParseErrorKind::ResourceLimit,
                "Source exceeds maximum supported size (4 GiB)".into(),
            );
            parser.aborted = true;
        }
        parser
    }

    pub(crate) fn parse(mut self) -> ParseOutput {
        let kind = if self.context.is_module {
            ProgramKind::Module
        } else {
            ProgramKind::Script
        };

        let mut body = Vec::new();

        // R14: "use strict" detection is handled in scope analysis (ScopeAnalyzer::has_use_strict).

        while !matches!(self.current.kind, TokenKind::Eof) && !self.aborted {
            // S12: abort before arena panic on pathological input
            if self.stmts.is_full()
                || self.exprs.is_full()
                || self.patterns.is_full()
                || self.stmts.has_overflowed()
                || self.exprs.has_overflowed()
                || self.patterns.has_overflowed()
            {
                self.error(
                    JsParseErrorKind::ResourceLimit,
                    "AST node limit exceeded, aborting".into(),
                );
                self.aborted = true;
                break;
            }
            let stmt_id = self.parse_statement_or_declaration();
            body.push(stmt_id);
        }

        // Move lexer errors into parser errors
        let mut all_errors = std::mem::take(&mut self.lexer.errors);
        all_errors.append(&mut self.errors);

        let program = Program {
            kind,
            body,
            stmts: self.stmts,
            exprs: self.exprs,
            patterns: self.patterns,
            interner: self.lexer.interner,
            atoms: self.atoms,
        };

        ParseOutput {
            program,
            errors: all_errors,
        }
    }

    // ── Token helpers ────────────────────────────────────────────────

    /// Current token kind reference.
    pub(crate) fn at(&self) -> &TokenKind {
        &self.current.kind
    }

    /// Current token span.
    pub(crate) fn span(&self) -> Span {
        self.current.span
    }

    /// Advance to the next token, returning the consumed token.
    pub(crate) fn advance(&mut self) -> Token {
        let prev = std::mem::replace(
            &mut self.current,
            Token {
                kind: TokenKind::Eof,
                span: Span::empty(0),
            },
        );
        if let Some((tok, had_nl)) = self.peeked.take() {
            self.current = tok;
            self.had_newline_before = had_nl;
        } else {
            let (next, had_nl) = self.lexer.next_token();
            self.current = next;
            self.had_newline_before = had_nl;
        }
        prev
    }

    /// Peek at the next token's kind without consuming the current token.
    pub(crate) fn peek_kind(&mut self) -> &TokenKind {
        if self.peeked.is_none() {
            self.peeked = Some(self.lexer.next_token());
        }
        &self.peeked.as_ref().expect("peeked token set above").0.kind
    }

    /// Consume a template continuation part (after `}`).
    pub(crate) fn lex_template_part(&mut self) -> Token {
        self.lexer.lex_template_part()
    }

    /// Check if current token is a specific keyword.
    pub(crate) fn at_keyword(&self, kw: Keyword) -> bool {
        matches!(&self.current.kind, TokenKind::Keyword(k) if *k == kw)
    }

    /// Intern a string, returning its `Atom`.
    pub(crate) fn intern(&mut self, s: &str) -> Atom {
        self.lexer.interner.intern(s)
    }

    /// Resolve an `Atom` to its UTF-8 string (lossy for lone surrogates).
    pub(crate) fn resolve(&self, atom: Atom) -> String {
        self.lexer.interner.get_utf8(atom)
    }

    /// Check if current is a contextual keyword by atom comparison (u32 == u32).
    pub(crate) fn at_contextual_atom(&self, atom: Atom) -> bool {
        matches!(&self.current.kind, TokenKind::Identifier(a) if *a == atom)
    }

    /// Consume a specific punctuator or return error.
    pub(crate) fn expect(&mut self, expected: &TokenKind) -> Result<Token, ()> {
        if std::mem::discriminant(&self.current.kind) == std::mem::discriminant(expected) {
            Ok(self.advance())
        } else {
            self.error(
                JsParseErrorKind::UnexpectedToken,
                format!("Expected {expected:?}, got {:?}", self.current.kind),
            );
            Err(())
        }
    }

    /// Consume a keyword or return error.
    pub(crate) fn expect_keyword(&mut self, kw: Keyword) -> Result<Token, ()> {
        if self.at_keyword(kw) {
            Ok(self.advance())
        } else {
            self.error(
                JsParseErrorKind::UnexpectedToken,
                format!("Expected keyword {kw:?}, got {:?}", self.current.kind),
            );
            Err(())
        }
    }

    /// R13: Consume a contextual keyword by atom comparison (u32 == u32).
    pub(crate) fn expect_contextual_atom(&mut self, atom: Atom) -> Result<Token, ()> {
        if self.at_contextual_atom(atom) {
            Ok(self.advance())
        } else {
            let name = self.resolve(atom);
            self.error(
                JsParseErrorKind::UnexpectedToken,
                format!("Expected '{name}', got {:?}", self.current.kind),
            );
            Err(())
        }
    }

    /// Expect a comma separator unless the next token is the list closer.
    /// Used for argument lists, array/object literals, imports/exports, formal params, etc.
    pub(crate) fn expect_comma_unless(&mut self, closer: &TokenKind) {
        if std::mem::discriminant(self.at()) != std::mem::discriminant(closer) {
            let _ = self.expect(&TokenKind::Comma);
        }
    }

    /// ASI: Automatic Semicolon Insertion.
    pub(crate) fn expect_semicolon(&mut self) {
        match self.current.kind {
            TokenKind::Semicolon => {
                self.advance();
            }
            TokenKind::RBrace | TokenKind::Eof => {
                // Virtual semicolon
            }
            _ if self.had_newline_before => {
                // Virtual semicolon (newline before current token)
            }
            _ => {
                self.error(
                    JsParseErrorKind::UnexpectedToken,
                    format!("Expected ';', got {:?}", self.current.kind),
                );
            }
        }
    }

    /// R6: Emit error if `for await` is used with non-`for-of` loop.
    pub(crate) fn check_for_await(&mut self, is_await: bool, context: &str) {
        if is_await {
            self.error(
                JsParseErrorKind::UnexpectedToken,
                format!("for await requires 'of', not {context}"),
            );
        }
    }

    /// R4: Parse optional `= expr` initializer.
    pub(crate) fn parse_optional_initializer(&mut self) -> Option<NodeId<Expr>> {
        if matches!(self.at(), TokenKind::Eq) {
            self.advance();
            Some(self.parse_assignment_expression())
        } else {
            None
        }
    }

    /// Returns true if no line terminator appears between the current token
    /// and the peeked token. Used for `[no LineTerminator here]` productions.
    /// Must be called after `peek_kind()` so that `peeked` is populated.
    pub(crate) fn peek_no_newline(&self) -> bool {
        debug_assert!(
            self.peeked.is_some(),
            "peek_no_newline called without prior peek_kind()"
        );
        !self.peeked.as_ref().is_some_and(|(_, nl)| *nl)
    }

    /// Returns true if the next token could start a property name,
    /// indicating the current identifier is a get/set/async modifier prefix.
    pub(crate) fn peek_is_property_name(&mut self) -> bool {
        matches!(
            self.peek_kind(),
            TokenKind::Identifier(_)
                | TokenKind::StringLiteral(_)
                | TokenKind::NumericLiteral(_)
                | TokenKind::BigIntLiteral(_)
                | TokenKind::LBracket
                | TokenKind::Star
                | TokenKind::PrivateIdentifier(_)
                | TokenKind::Keyword(_)
        )
    }

    /// R5: Parse method prefix keywords (async, *, get, set) shared by object and class.
    ///
    /// Object syntax: `get`/`set` checked before `async`, `*` can come before get/set.
    /// Class syntax: `async` checked first, `get`/`set` only if not async/generator.
    /// Both check `peek_is_property_name()` to distinguish prefix from property name.
    pub(crate) fn parse_method_prefixes(&mut self, class_mode: bool) -> MethodPrefixes {
        let mut p = MethodPrefixes::default();

        if class_mode {
            // Class: async first
            if self.at_contextual_atom(self.atoms.r#async)
                && self.peek_is_property_name()
                && self.peek_no_newline()
            {
                p.is_async = true;
                self.advance();
                if matches!(self.at(), TokenKind::Star) {
                    p.is_generator = true;
                    self.advance();
                }
            }

            if matches!(self.at(), TokenKind::Star) && !p.is_async {
                p.is_generator = true;
                self.advance();
            }

            if self.at_contextual_atom(self.atoms.get)
                && self.peek_is_property_name()
                && !p.is_async
                && !p.is_generator
            {
                p.method_kind = MethodKind::Get;
                self.advance();
            } else if self.at_contextual_atom(self.atoms.set)
                && self.peek_is_property_name()
                && !p.is_async
                && !p.is_generator
            {
                p.method_kind = MethodKind::Set;
                self.advance();
            }
        } else {
            // Object: get/set first, then async
            if self.at_contextual_atom(self.atoms.get) && self.peek_is_property_name() {
                p.method_kind = MethodKind::Get;
                self.advance();
            } else if self.at_contextual_atom(self.atoms.set) && self.peek_is_property_name() {
                p.method_kind = MethodKind::Set;
                self.advance();
            } else if self.at_contextual_atom(self.atoms.r#async)
                && self.peek_is_property_name()
                && self.peek_no_newline()
            {
                p.is_async = true;
                self.advance();
                if matches!(self.at(), TokenKind::Star) {
                    p.is_generator = true;
                    self.advance();
                }
            }

            if matches!(self.at(), TokenKind::Star)
                && p.method_kind == MethodKind::Method
                && !p.is_async
            {
                p.is_generator = true;
                self.advance();
            }
        }
        p
    }

    /// Parse a loop body with `in_loop = true`, restoring context after.
    /// B4: Uses `parse_sub_statement` to reject lexical declarations in single-statement positions.
    pub(crate) fn parse_loop_body(&mut self) -> NodeId<Stmt> {
        let saved = self.context;
        self.context.in_loop = true;
        let body = self.parse_sub_statement();
        self.context = saved;
        body
    }

    /// Run a closure with function context flags, restoring context after.
    pub(crate) fn with_function_context<R>(
        &mut self,
        is_async: bool,
        is_generator: bool,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let saved = self.context;
        // M1: labels from enclosing scopes must not leak into nested functions
        let saved_labels = std::mem::take(&mut self.labels);
        self.context.in_function = true;
        self.context.in_async = is_async;
        self.context.in_generator = is_generator;
        self.context.in_loop = false;
        self.context.in_switch = false;
        self.context.in_constructor = false;
        self.context.in_static_block = false;
        self.context.in_method = false;
        self.context.in_derived_constructor = false;
        let result = f(self);
        self.context = saved;
        self.labels = saved_labels;
        result
    }

    // ── Recursion depth tracking ──────────────────────────────────────

    /// Enter a recursive parsing level. Returns false if max depth exceeded.
    pub(crate) fn enter_recursion(&mut self) -> bool {
        self.depth += 1;
        if self.depth > MAX_NESTING_DEPTH {
            self.error(
                JsParseErrorKind::NestingTooDeep,
                "Maximum nesting depth exceeded".into(),
            );
            self.depth -= 1;
            return false;
        }
        true
    }

    /// Leave a recursive parsing level.
    pub(crate) fn leave_recursion(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    // ── Error handling & recovery ────────────────────────────────────

    pub(crate) fn error(&mut self, kind: JsParseErrorKind, message: String) {
        self.error_at(kind, message, self.current.span);
    }

    pub(crate) fn error_at(&mut self, kind: JsParseErrorKind, message: String, span: Span) {
        self.errors.push(JsParseError {
            kind,
            span,
            message,
        });
        if self.errors.len() >= MAX_ERRORS {
            self.errors.push(JsParseError {
                kind: JsParseErrorKind::TooManyErrors,
                span: self.current.span,
                message: "Too many errors, aborting".into(),
            });
            self.aborted = true;
        }
    }

    /// Allocate an error expression node.
    pub(crate) fn error_expr(&mut self, span: Span) -> NodeId<Expr> {
        self.exprs.alloc(Expr {
            kind: ExprKind::Error,
            span,
        })
    }

    /// Allocate an error statement node.
    pub(crate) fn error_stmt(&mut self, span: Span) -> NodeId<Stmt> {
        self.stmts.alloc(Stmt {
            kind: StmtKind::Error,
            span,
        })
    }

    /// Allocate an error pattern node.
    pub(crate) fn error_pattern(&mut self, span: Span) -> NodeId<Pattern> {
        self.patterns.alloc(Pattern {
            kind: PatternKind::Error,
            span,
        })
    }

    /// R1: Check if a name is `eval` or `arguments` (forbidden as binding in strict mode)
    /// or `await` (forbidden when it's a keyword).
    pub(crate) fn check_reserved_binding(&mut self, name: Atom, context: &str) {
        if name == self.atoms.eval || name == self.atoms.arguments {
            self.error(
                JsParseErrorKind::StrictModeViolation,
                format!("Cannot use 'eval' or 'arguments' as {context} in strict mode"),
            );
        }
        if name == self.atoms.r#await && self.context.await_is_keyword() {
            self.error(
                JsParseErrorKind::UnexpectedToken,
                format!("'await' cannot be used as {context} in this context"),
            );
        }
    }

    /// R2: Check if a list has reached the max item limit; emit error and return true to break.
    pub(crate) fn check_list_limit(&mut self, len: usize, what: &str) -> bool {
        if len >= MAX_LIST_ITEMS {
            self.error(
                JsParseErrorKind::UnexpectedToken,
                format!("Too many {what}"),
            );
            return true;
        }
        false
    }

    /// Panic-mode recovery: skip tokens until a statement-start sync point.
    pub(crate) fn recover_to_statement_start(&mut self) {
        // L7: safety limit to guarantee progress (Eof always terminates, but be defensive)
        for i in 0..100_000 {
            match &self.current.kind {
                TokenKind::Eof
                | TokenKind::RBrace
                | TokenKind::Semicolon
                | TokenKind::Keyword(
                    Keyword::Let
                    | Keyword::Const
                    | Keyword::Var
                    | Keyword::Function
                    | Keyword::Class
                    | Keyword::If
                    | Keyword::For
                    | Keyword::While
                    | Keyword::Do
                    | Keyword::Return
                    | Keyword::Throw
                    | Keyword::Try
                    | Keyword::Switch
                    | Keyword::Break
                    | Keyword::Continue
                    | Keyword::Import
                    | Keyword::Export,
                )
                | TokenKind::LBrace => break,
                _ => {
                    self.advance();
                }
            }
            // S15: abort if safety limit reached (should never happen with well-behaved lexer)
            if i == 99_999 {
                self.aborted = true;
            }
        }
    }

    // ── Top-level dispatch ───────────────────────────────────────────

    /// Parse a statement or declaration (top-level dispatch).
    pub(crate) fn parse_statement_or_declaration(&mut self) -> NodeId<Stmt> {
        if !self.enter_recursion() {
            // S1: advance to prevent repeated errors on the same token
            self.advance();
            return self.error_stmt(self.span());
        }
        let result = self.parse_statement_or_declaration_inner();
        self.leave_recursion();
        result
    }

    fn parse_statement_or_declaration_inner(&mut self) -> NodeId<Stmt> {
        // A5: save and clear at_top_level so child statements see false
        let was_top = self.context.at_top_level;
        self.context.at_top_level = false;

        let result = match &self.current.kind {
            // Declarations
            TokenKind::Keyword(Keyword::Let | Keyword::Const | Keyword::Var) => {
                self.parse_variable_declaration_stmt()
            }
            TokenKind::Keyword(Keyword::Function) => self.parse_function_declaration(),
            TokenKind::Keyword(Keyword::Class) => self.parse_class_declaration(),

            // Module declarations
            TokenKind::Keyword(Keyword::Import) => {
                if self.context.is_module {
                    // Could be import declaration or dynamic import expression
                    // Peek ahead to disambiguate
                    match self.lexer_peek_after_import() {
                        ImportLookahead::Declaration => {
                            // A5: import declarations only at module top level
                            if !was_top {
                                self.error(
                                    JsParseErrorKind::UnexpectedToken,
                                    "'import' declarations can only appear at top level of a module"
                                        .into(),
                                );
                            }
                            self.parse_import_declaration()
                        }
                        ImportLookahead::Expression => self.parse_expression_statement(),
                    }
                } else {
                    // In script mode, `import(...)` and `import.meta` are expressions
                    self.parse_expression_statement()
                }
            }
            TokenKind::Keyword(Keyword::Export) => {
                if self.context.is_module {
                    // A5: export declarations only at module top level
                    if !was_top {
                        self.error(
                            JsParseErrorKind::UnexpectedToken,
                            "'export' declarations can only appear at top level of a module".into(),
                        );
                    }
                    self.parse_export_declaration()
                } else {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "'export' only allowed in modules".into(),
                    );
                    let span = self.span();
                    self.advance();
                    self.error_stmt(span)
                }
            }

            // Async function declaration
            TokenKind::Identifier(s) if *s == self.atoms.r#async => {
                // Could be `async function` or expression
                self.parse_async_or_expression()
            }

            // Statements
            _ => self.parse_statement(),
        };

        self.context.at_top_level = was_top;
        result
    }

    /// Disambiguate `import` — declaration vs expression.
    /// H4: Use `peek_kind()` to correctly skip comments and Unicode whitespace.
    fn lexer_peek_after_import(&mut self) -> ImportLookahead {
        match self.peek_kind() {
            TokenKind::LParen | TokenKind::Dot => ImportLookahead::Expression,
            _ => ImportLookahead::Declaration,
        }
    }

    /// Parse `async function` declaration or fall back to expression statement.
    fn parse_async_or_expression(&mut self) -> NodeId<Stmt> {
        // M13: Use peek_kind to check if next token is `function` (with no line terminator).
        // We must check `had_newline_before` AFTER consuming `async` (i.e. on the peeked token).
        // But since `async` is the current token, we peek to see the next token.
        if matches!(self.peek_kind(), TokenKind::Keyword(Keyword::Function)) {
            // Check no newline between `async` and `function` by inspecting peeked newline flag
            let no_newline = self.peek_no_newline();
            if no_newline {
                return self.parse_async_function_declaration();
            }
        }
        self.parse_expression_statement()
    }
}

enum ImportLookahead {
    Declaration,
    Expression,
}

/// R5: Parsed method prefix flags, shared by object and class member parsing.
#[derive(Debug, Default)]
pub(crate) struct MethodPrefixes {
    pub(crate) is_async: bool,
    pub(crate) is_generator: bool,
    pub(crate) method_kind: MethodKind,
}
