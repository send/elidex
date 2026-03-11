//! Byte-oriented JavaScript lexer.
//!
//! Produces tokens from source bytes with support for:
//! - Identifiers and keywords (ASCII fast path + Unicode)
//! - All ES2020+ punctuators (maximal munch)
//! - String/numeric/template/regexp literals
//! - Comments and whitespace
//! - Line terminator tracking for ASI
//!
//! Always strict mode. No Annex B (see crate-level docs).

mod ident;
mod literal;
mod regexp_lit;

use crate::atom::StringInterner;
use crate::error::{JsParseError, JsParseErrorKind};
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// The lexer state.
pub(crate) struct Lexer<'a> {
    source: &'a [u8],
    // B7: pub(crate) for parser-directed rescan
    pub(crate) pos: usize,
    /// After expression-end tokens, `/` is division; otherwise regexp.
    pub(crate) prev_allows_regexp: bool,
    /// Byte offsets of line starts (for `SourceLocation` computation).
    pub(crate) line_starts: Vec<u32>,
    /// Errors encountered during lexing.
    pub(crate) errors: Vec<JsParseError>,
    /// Shared string interner for all identifier/literal strings.
    pub(crate) interner: StringInterner,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        Self {
            source: source.as_bytes(),
            pos: 0,
            prev_allows_regexp: true,
            line_starts: vec![0],
            errors: Vec::new(),
            interner: StringInterner::new(),
        }
    }

    /// B7: Reset lexer position and rescan `/` or `/=` as a regexp literal.
    /// Returns `None` if `target_pos` is out of bounds.
    pub(crate) fn rescan_as_regexp(&mut self, target_pos: usize) -> Option<TokenKind> {
        if target_pos > self.source.len() {
            return None;
        }
        self.pos = target_pos;
        let kind = self.lex_regexp();
        self.prev_allows_regexp = !kind.is_expression_end();
        Some(kind)
    }

    /// Peek current byte without consuming.
    pub(super) fn peek(&self) -> Option<u8> {
        self.source.get(self.pos).copied()
    }

    /// Peek byte at offset from current position.
    pub(super) fn peek_at(&self, offset: usize) -> Option<u8> {
        self.source.get(self.pos + offset).copied()
    }

    /// Advance one byte.
    pub(super) fn advance(&mut self) -> Option<u8> {
        let b = self.source.get(self.pos).copied()?;
        self.pos += 1;
        Some(b)
    }

    /// Check if at end of input.
    pub(super) fn at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    /// Record a line start offset (capped to prevent unbounded growth on pathological input).
    pub(super) fn record_line_start(&mut self) {
        // Cap at 16M entries (~64 MiB) — only affects source location reporting.
        if self.line_starts.len() < 16 * 1024 * 1024 {
            self.line_starts.push(self.pos as u32);
        }
    }

    /// Check if current position is a LS U+2028 (E2 80 A8) or PS U+2029 (E2 80 A9).
    pub(super) fn is_ls_ps(&self) -> bool {
        self.peek() == Some(0xE2)
            && self.peek_at(1) == Some(0x80)
            && matches!(self.peek_at(2), Some(0xA8 | 0xA9))
    }

    /// B5: Check if current position is a Unicode Zs whitespace character (non-line-terminator).
    /// Covers: U+00A0 NBSP, U+1680 OGHAM, U+2000-200A, U+202F, U+205F, U+3000.
    pub(super) fn is_unicode_whitespace(&self) -> bool {
        match self.peek() {
            Some(0xC2) if self.peek_at(1) == Some(0xA0) => true, // U+00A0 NBSP
            Some(0xE1) if self.peek_at(1) == Some(0x9A) && self.peek_at(2) == Some(0x80) => true, // U+1680 OGHAM
            Some(0xE2) => matches!(
                (self.peek_at(1), self.peek_at(2)),
                (Some(0x80), Some(0x80..=0x8A | 0xAF)) // U+2000-200A, U+202F
                | (Some(0x81), Some(0x9F)) // U+205F
            ),
            Some(0xE3) if self.peek_at(1) == Some(0x80) && self.peek_at(2) == Some(0x80) => true, // U+3000 全角空白
            _ => false,
        }
    }

    /// M7: Push an error with limit check to prevent unbounded accumulation.
    pub(super) fn push_error(&mut self, error: JsParseError) {
        if self.errors.len() < crate::error::MAX_ERRORS {
            self.errors.push(error);
        }
    }

    /// Emit a "Numeric separator in invalid position" error.
    pub(super) fn separator_error(&mut self, pos: u32) {
        self.push_error(JsParseError {
            kind: JsParseErrorKind::InvalidNumber,
            span: Span::new(pos, pos + 1),
            message: "Numeric separator in invalid position".into(),
        });
    }

    /// Produce the next token and whether a line terminator preceded it.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn next_token(&mut self) -> (Token, bool) {
        // S1: outer loop replaces recursion for unexpected bytes (stack safety)
        let mut had_newline = self.skip_whitespace_and_comments();
        loop {
            let start = self.pos as u32;

            if self.at_end() {
                let span = Span::empty(start);
                return (
                    Token {
                        kind: TokenKind::Eof,
                        span,
                    },
                    had_newline,
                );
            }

            // S4: bounds-checked access (at_end() guard above should prevent OOB,
            // but use .get() for defense-in-depth)
            let Some(&b) = self.source.get(self.pos) else {
                return (
                    Token {
                        kind: TokenKind::Eof,
                        span: Span::empty(start),
                    },
                    had_newline,
                );
            };
            let kind = match b {
                // String literals
                b'\'' | b'"' => self.lex_string(b),
                // Template literals
                b'`' => self.lex_template(),
                // Numeric literals
                b'0'..=b'9' => self.lex_number(),
                // Dot / Ellipsis / Number starting with .
                b'.' => {
                    if matches!(self.peek_at(1), Some(b'0'..=b'9')) {
                        self.lex_number()
                    } else if self.peek_at(1) == Some(b'.') && self.peek_at(2) == Some(b'.') {
                        self.pos += 3;
                        TokenKind::Ellipsis
                    } else {
                        self.pos += 1;
                        TokenKind::Dot
                    }
                }
                // Identifiers and keywords
                b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'$' => self.lex_identifier(),
                // Private identifier
                b'#' => self.lex_private_identifier(),
                // Punctuators
                b'(' => {
                    self.pos += 1;
                    TokenKind::LParen
                }
                b')' => {
                    self.pos += 1;
                    TokenKind::RParen
                }
                b'{' => {
                    self.pos += 1;
                    TokenKind::LBrace
                }
                b'}' => {
                    self.pos += 1;
                    TokenKind::RBrace
                }
                b'[' => {
                    self.pos += 1;
                    TokenKind::LBracket
                }
                b']' => {
                    self.pos += 1;
                    TokenKind::RBracket
                }
                b';' => {
                    self.pos += 1;
                    TokenKind::Semicolon
                }
                b',' => {
                    self.pos += 1;
                    TokenKind::Comma
                }
                b':' => {
                    self.pos += 1;
                    TokenKind::Colon
                }
                b'~' => {
                    self.pos += 1;
                    TokenKind::Tilde
                }
                b'?' => self.lex_question(),
                b'+' => self.lex_plus(),
                b'-' => self.lex_minus(),
                b'*' => self.lex_star(),
                b'/' => self.lex_slash(),
                b'%' => self.lex_percent(),
                b'<' => self.lex_lt(),
                b'>' => self.lex_gt(),
                b'=' => self.lex_eq(),
                b'!' => self.lex_excl(),
                b'&' => self.lex_amp(),
                b'|' => self.lex_pipe(),
                b'^' => self.lex_caret(),
                // B4: \u escape as identifier start
                b'\\' if self.peek_at(1) == Some(b'u') => self.lex_identifier(),
                // Unicode identifier start
                _ => {
                    if self.is_unicode_id_start() {
                        self.lex_identifier()
                    } else {
                        // V24: skip the full UTF-8 character, not just one byte,
                        // to avoid cascading errors from landing mid-sequence.
                        let char_len = match b {
                            0xC0..=0xDF => 2,
                            0xE0..=0xEF => 3,
                            0xF0..=0xF7 => 4,
                            _ => 1,
                        };
                        let skip = char_len.min(self.source.len() - self.pos);
                        self.pos += skip;
                        self.push_error(JsParseError {
                            kind: JsParseErrorKind::UnexpectedToken,
                            span: Span::new(start, self.pos as u32),
                            message: format!("Unexpected character: 0x{b:02x}"),
                        });
                        // S1: loop to get the next valid token (replaces recursion)
                        had_newline |= self.skip_whitespace_and_comments();
                        continue;
                    }
                }
            };

            let span = Span::new(start, self.pos as u32);
            self.prev_allows_regexp = !kind.is_expression_end();
            return (Token { kind, span }, had_newline);
        } // end of S1 loop
    }

    /// Lex a template continuation part (called by parser after `}`).
    pub(crate) fn lex_template_part(&mut self) -> Token {
        let start = self.pos as u32;
        let kind = self.lex_template_inner(false);
        let span = Span::new(start, self.pos as u32);
        self.prev_allows_regexp = !kind.is_expression_end();
        Token { kind, span }
    }

    // ── Punctuators (maximal munch) ──────────────────────────────────

    fn lex_question(&mut self) -> TokenKind {
        self.pos += 1; // skip ?
        match self.peek() {
            Some(b'?') => {
                self.pos += 1;
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    TokenKind::NullCoalEq
                } else {
                    TokenKind::NullCoal
                }
            }
            Some(b'.') => {
                // ?. but not ?.digit (which would be ? followed by number)
                if matches!(self.peek_at(1), Some(b'0'..=b'9')) {
                    TokenKind::Question
                } else {
                    self.pos += 1;
                    TokenKind::OptChain
                }
            }
            _ => TokenKind::Question,
        }
    }

    fn lex_plus(&mut self) -> TokenKind {
        self.pos += 1;
        match self.peek() {
            Some(b'+') => {
                self.pos += 1;
                TokenKind::PlusPlus
            }
            Some(b'=') => {
                self.pos += 1;
                TokenKind::PlusEq
            }
            _ => TokenKind::Plus,
        }
    }

    fn lex_minus(&mut self) -> TokenKind {
        self.pos += 1;
        match self.peek() {
            Some(b'-') => {
                self.pos += 1;
                TokenKind::MinusMinus
            }
            Some(b'=') => {
                self.pos += 1;
                TokenKind::MinusEq
            }
            _ => TokenKind::Minus,
        }
    }

    fn lex_star(&mut self) -> TokenKind {
        self.pos += 1;
        match self.peek() {
            Some(b'*') => {
                self.pos += 1;
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    TokenKind::ExpEq
                } else {
                    TokenKind::Exp
                }
            }
            Some(b'=') => {
                self.pos += 1;
                TokenKind::StarEq
            }
            _ => TokenKind::Star,
        }
    }

    fn lex_slash(&mut self) -> TokenKind {
        // Comments already handled in skip_whitespace_and_comments
        if self.prev_allows_regexp {
            self.lex_regexp()
        } else {
            self.pos += 1;
            if self.peek() == Some(b'=') {
                self.pos += 1;
                TokenKind::SlashEq
            } else {
                TokenKind::Slash
            }
        }
    }

    fn lex_percent(&mut self) -> TokenKind {
        self.pos += 1;
        if self.peek() == Some(b'=') {
            self.pos += 1;
            TokenKind::PercentEq
        } else {
            TokenKind::Percent
        }
    }

    fn lex_lt(&mut self) -> TokenKind {
        self.pos += 1;
        match self.peek() {
            Some(b'<') => {
                self.pos += 1;
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    TokenKind::ShlEq
                } else {
                    TokenKind::Shl
                }
            }
            Some(b'=') => {
                self.pos += 1;
                TokenKind::LtEq
            }
            _ => TokenKind::Lt,
        }
    }

    fn lex_gt(&mut self) -> TokenKind {
        self.pos += 1;
        match self.peek() {
            Some(b'>') => {
                self.pos += 1;
                match self.peek() {
                    Some(b'>') => {
                        self.pos += 1;
                        if self.peek() == Some(b'=') {
                            self.pos += 1;
                            TokenKind::UShrEq
                        } else {
                            TokenKind::UShr
                        }
                    }
                    Some(b'=') => {
                        self.pos += 1;
                        TokenKind::ShrEq
                    }
                    _ => TokenKind::Shr,
                }
            }
            Some(b'=') => {
                self.pos += 1;
                TokenKind::GtEq
            }
            _ => TokenKind::Gt,
        }
    }

    fn lex_eq(&mut self) -> TokenKind {
        self.pos += 1;
        match self.peek() {
            Some(b'=') => {
                self.pos += 1;
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    TokenKind::StrictEq
                } else {
                    TokenKind::EqEq
                }
            }
            Some(b'>') => {
                self.pos += 1;
                TokenKind::Arrow
            }
            _ => TokenKind::Eq,
        }
    }

    fn lex_excl(&mut self) -> TokenKind {
        self.pos += 1;
        match self.peek() {
            Some(b'=') => {
                self.pos += 1;
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    TokenKind::StrictNe
                } else {
                    TokenKind::NotEq
                }
            }
            _ => TokenKind::Not,
        }
    }

    fn lex_amp(&mut self) -> TokenKind {
        self.pos += 1;
        match self.peek() {
            Some(b'&') => {
                self.pos += 1;
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    TokenKind::AndEq
                } else {
                    TokenKind::And
                }
            }
            Some(b'=') => {
                self.pos += 1;
                TokenKind::AmpEq
            }
            _ => TokenKind::Amp,
        }
    }

    fn lex_pipe(&mut self) -> TokenKind {
        self.pos += 1;
        match self.peek() {
            Some(b'|') => {
                self.pos += 1;
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    TokenKind::OrEq
                } else {
                    TokenKind::Or
                }
            }
            Some(b'=') => {
                self.pos += 1;
                TokenKind::PipeEq
            }
            _ => TokenKind::Pipe,
        }
    }

    fn lex_caret(&mut self) -> TokenKind {
        self.pos += 1;
        if self.peek() == Some(b'=') {
            self.pos += 1;
            TokenKind::CaretEq
        } else {
            TokenKind::Caret
        }
    }

    /// R2: Shared `\u{HHHH...}` braced unicode escape parser.
    /// Caller must have already consumed past `{`. Returns `Some(codepoint)` on success.
    /// Per §12.9.4, any number of hex digits is allowed as long as value ≤ 0x10FFFF.
    pub(super) fn read_braced_unicode_codepoint(&mut self) -> Option<u32> {
        let mut val = 0u32;
        let mut count = 0;
        let mut overflow = false;
        let mut terminated = false;
        while let Some(b) = self.peek() {
            if b == b'}' {
                self.pos += 1;
                terminated = true;
                break;
            }
            if let Some(d) = hex_digit(b) {
                count += 1;
                self.pos += 1;
                if !overflow {
                    val = val
                        .checked_mul(16)
                        .and_then(|v| v.checked_add(u32::from(d)))
                        .unwrap_or_else(|| {
                            overflow = true;
                            u32::MAX
                        });
                }
            } else {
                break;
            }
        }
        if count > 0 && terminated {
            Some(if overflow { u32::MAX } else { val })
        } else {
            None
        }
    }
}

/// R8: ASCII subset of ES `IdentifierPartChar` (`[0-9a-zA-Z_$]`).
#[inline]
pub(super) fn is_ascii_id_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

/// Convert a hex digit byte to its numeric value.
pub(super) fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
