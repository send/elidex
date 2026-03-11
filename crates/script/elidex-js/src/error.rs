//! Parse error types and output container.

use crate::span::Span;
use std::fmt;

/// A single parse error with location information.
#[derive(Debug, Clone)]
pub struct JsParseError {
    pub kind: JsParseErrorKind,
    pub span: Span,
    pub message: String,
}

impl fmt::Display for JsParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.span, self.kind, self.message)
    }
}

impl std::error::Error for JsParseError {}

/// Categories of parse errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsParseErrorKind {
    /// Unexpected token encountered.
    UnexpectedToken,
    /// Unexpected end of input.
    UnexpectedEof,
    /// Invalid or unterminated string literal.
    InvalidString,
    /// Invalid numeric literal.
    InvalidNumber,
    /// Invalid regular expression.
    InvalidRegExp,
    /// Invalid escape sequence.
    InvalidEscape,
    /// Unterminated template literal.
    UnterminatedTemplate,
    /// Unterminated comment.
    UnterminatedComment,
    /// Invalid destructuring target.
    InvalidDestructuring,
    /// Invalid assignment target.
    InvalidAssignmentTarget,
    /// Duplicate binding in same scope.
    DuplicateBinding,
    /// Strict mode restriction violated (e.g. eval/arguments as binding name).
    StrictModeViolation,
    /// `break`/`continue` outside loop.
    IllegalBreak,
    /// `return` outside function.
    IllegalReturn,
    /// Exceeded max error count (parser aborted).
    TooManyErrors,
    /// Nesting depth exceeded.
    NestingTooDeep,
    /// Resource limit exceeded (source size, AST node count, etc.).
    ResourceLimit,
}

impl fmt::Display for JsParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Maximum number of errors before the parser aborts.
pub const MAX_ERRORS: usize = 100;

/// Maximum nesting depth for recursive parsing.
pub const MAX_NESTING_DEPTH: u32 = 1024;

/// Parser output: partial AST + error list (error recovery result).
/// `errors` is empty iff the program is syntactically valid.
#[derive(Debug)]
#[must_use]
pub struct ParseOutput {
    pub program: crate::ast::Program,
    pub errors: Vec<JsParseError>,
}
