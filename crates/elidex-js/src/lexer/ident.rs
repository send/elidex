//! Identifier, keyword, and whitespace/comment lexing.

use crate::error::{JsParseError, JsParseErrorKind};
use crate::span::Span;
use crate::token::{Keyword, TokenKind};

use super::Lexer;

// ── R4: Unicode identifier predicates ─────────────────────────────

/// ES spec `ID_Start`: `XID_Start || '_' || '$'`
fn is_id_start(c: char) -> bool {
    unicode_ident::is_xid_start(c) || c == '_' || c == '$'
}

/// ES spec `ID_Continue`: `XID_Continue || '$' || U+200C || U+200D`
pub(super) fn is_id_continue(c: char) -> bool {
    unicode_ident::is_xid_continue(c) || c == '$' || c == '\u{200C}' || c == '\u{200D}'
}

/// R3: Decode first char from a byte slice with error recovery for invalid UTF-8.
/// L1: Only examines up to 4 bytes (max UTF-8 char length) to avoid O(n) validation.
fn decode_first_char(bytes: &[u8]) -> Option<char> {
    let len = bytes.len().min(4);
    match std::str::from_utf8(&bytes[..len]) {
        Ok(s) => s.chars().next(),
        Err(e) => {
            let valid = e.valid_up_to();
            if valid > 0 {
                std::str::from_utf8(&bytes[..valid])
                    .ok()
                    .and_then(|s| s.chars().next())
            } else {
                None
            }
        }
    }
}

impl Lexer<'_> {
    // ── Whitespace & Comments ────────────────────────────────────────

    /// Skip whitespace and comments, returning true if any line terminators seen.
    pub(super) fn skip_whitespace_and_comments(&mut self) -> bool {
        // B5: Hashbang comment `#!` at position 0
        if self.pos == 0 && self.peek() == Some(b'#') && self.peek_at(1) == Some(b'!') {
            self.pos += 2;
            while let Some(b) = self.peek() {
                if b == b'\n' || b == b'\r' {
                    break;
                }
                if self.is_ls_ps() {
                    break;
                }
                self.pos += 1;
            }
        }

        let mut saw_newline = false;
        loop {
            // R19: Fast-path ASCII whitespace (space, tab, VT, FF) and newlines
            // before falling through to multi-byte Unicode whitespace checks.
            match self.peek() {
                Some(b' ' | b'\t' | 0x0B | 0x0C) => {
                    self.pos += 1;
                }
                Some(b'\n') => {
                    self.pos += 1;
                    self.record_line_start();
                    saw_newline = true;
                }
                Some(b'\r') => {
                    self.pos += 1;
                    if self.peek() == Some(b'\n') {
                        self.pos += 1;
                    }
                    self.record_line_start();
                    saw_newline = true;
                }
                Some(b'/') => {
                    if self.peek_at(1) == Some(b'/') {
                        self.skip_line_comment();
                    } else if self.peek_at(1) == Some(b'*') {
                        if self.skip_block_comment() {
                            saw_newline = true;
                        }
                    } else {
                        break;
                    }
                }
                // Multi-byte whitespace: only reached for non-ASCII leading bytes
                // BOM (UTF-8: EF BB BF)
                Some(0xEF) if self.peek_at(1) == Some(0xBB) && self.peek_at(2) == Some(0xBF) => {
                    self.pos += 3;
                }
                // LS U+2028 (E2 80 A8) or PS U+2029 (E2 80 A9)
                Some(0xE2) if self.is_ls_ps() => {
                    self.pos += 3;
                    self.record_line_start();
                    saw_newline = true;
                }
                // B5: Unicode Zs whitespace (multi-byte, non-LS/PS)
                Some(0xE1..=0xE3) if self.is_unicode_whitespace() => {
                    self.pos += 3;
                }
                // B5: U+00A0 NBSP (C2 A0)
                Some(0xC2) if self.is_unicode_whitespace() => {
                    self.pos += 2;
                }
                _ => break,
            }
        }
        saw_newline
    }

    fn skip_line_comment(&mut self) {
        self.pos += 2; // skip //
        while let Some(b) = self.peek() {
            if b == b'\n' || b == b'\r' {
                break;
            }
            // LS/PS
            if self.is_ls_ps() {
                break;
            }
            self.pos += 1;
        }
    }

    /// Skip block comment. Returns true if it contained a line terminator.
    fn skip_block_comment(&mut self) -> bool {
        let start = self.pos as u32;
        self.pos += 2; // skip /*
        let mut saw_newline = false;
        loop {
            match self.peek() {
                None => {
                    self.push_error(JsParseError {
                        kind: JsParseErrorKind::UnterminatedComment,
                        span: Span::new(start, self.pos as u32),
                        message: "Unterminated block comment".into(),
                    });
                    break;
                }
                Some(b'*') if self.peek_at(1) == Some(b'/') => {
                    self.pos += 2;
                    break;
                }
                Some(b'\n') => {
                    self.pos += 1;
                    self.record_line_start();
                    saw_newline = true;
                }
                Some(b'\r') => {
                    self.pos += 1;
                    if self.peek() == Some(b'\n') {
                        self.pos += 1;
                    }
                    self.record_line_start();
                    saw_newline = true;
                }
                Some(0xE2) if self.is_ls_ps() => {
                    self.pos += 3;
                    self.record_line_start();
                    saw_newline = true;
                }
                _ => {
                    self.pos += 1;
                }
            }
        }
        saw_newline
    }

    // ── Identifiers & Keywords ───────────────────────────────────────

    pub(super) fn is_unicode_id_start(&self) -> bool {
        if self.at_end() {
            return false;
        }
        decode_first_char(&self.source[self.pos..]).is_some_and(is_id_start)
    }

    /// Maximum identifier length (bytes) to prevent pathological memory usage.
    const MAX_IDENTIFIER_LEN: usize = 65536;

    pub(super) fn lex_identifier(&mut self) -> TokenKind {
        let start = self.pos;
        // R3 fast path: scan ahead for pure ASCII identifiers (no escapes, no multi-byte).
        // This avoids building an intermediate String for the overwhelmingly common case.
        while let Some(b) = self.peek() {
            if super::is_ascii_id_continue(b) {
                self.pos += 1;
            } else {
                break;
            }
        }
        // Check if we hit a non-ASCII or escape — if so, fall back to slow path.
        let needs_slow_path = matches!(self.peek(), Some(b'\\' | 0x80..=0xFF));
        if !needs_slow_path {
            // Pure ASCII — use source slice directly.
            let s = std::str::from_utf8(&self.source[start..self.pos])
                .expect("ASCII bytes are valid UTF-8");
            if let Some(kw) = Keyword::from_reserved(s) {
                return TokenKind::Keyword(kw);
            }
            let atom = self.interner.intern(s);
            return TokenKind::Identifier(atom);
        }

        // R20: Slow path — preserve already-scanned ASCII bytes instead of resetting.
        let span_start = start as u32;
        let ascii_prefix = std::str::from_utf8(&self.source[start..self.pos])
            .expect("ASCII bytes are valid UTF-8");
        let mut buf = String::from(ascii_prefix);
        let mut has_escape = false;
        self.lex_identifier_chars(&mut buf, span_start, &mut has_escape);

        // B4: escaped identifier must not resolve to a keyword
        if has_escape {
            if Keyword::from_reserved(&buf).is_some() {
                self.push_error(JsParseError {
                    kind: JsParseErrorKind::UnexpectedToken,
                    span: Span::new(span_start, self.pos as u32),
                    message: format!("Keyword '{buf}' must not contain Unicode escapes"),
                });
            }
            let atom = self.interner.intern(&buf);
            TokenKind::Identifier(atom)
        } else if let Some(kw) = Keyword::from_reserved(&buf) {
            TokenKind::Keyword(kw)
        } else {
            let atom = self.interner.intern(&buf);
            TokenKind::Identifier(atom)
        }
    }

    /// R2: Validate a Unicode escape character as `ID_Start` (if first) or `ID_Continue`.
    fn validate_unicode_escape_char(&mut self, c: char, is_start: bool, span_start: u32) {
        if is_start {
            if !is_id_start(c) {
                self.push_error(JsParseError {
                    kind: JsParseErrorKind::InvalidEscape,
                    span: Span::new(span_start, self.pos as u32),
                    message: format!(
                        "Unicode escape \\u{{{:04X}}} is not a valid identifier start",
                        c as u32
                    ),
                });
            }
        } else if !is_id_continue(c) {
            self.push_error(JsParseError {
                kind: JsParseErrorKind::InvalidEscape,
                span: Span::new(span_start, self.pos as u32),
                message: format!(
                    "Unicode escape \\u{{{:04X}}} is not a valid identifier part",
                    c as u32
                ),
            });
        }
    }

    /// Parse `\uHHHH` or `\u{HHHH}` inside an identifier.
    fn lex_unicode_escape_in_ident(&mut self, ident_start: u32) -> Option<char> {
        self.pos += 2; // skip \u
        let val = if self.peek() == Some(b'{') {
            // R2: shared braced unicode escape
            self.pos += 1;
            if let Some(v) = self.read_braced_unicode_codepoint() {
                v
            } else {
                self.push_error(JsParseError {
                    kind: JsParseErrorKind::InvalidEscape,
                    span: Span::new(ident_start, self.pos as u32),
                    message: "Invalid Unicode escape in identifier".into(),
                });
                return None;
            }
        } else if let Some(v) = self.read_hex_digits(4) {
            v
        } else {
            self.push_error(JsParseError {
                kind: JsParseErrorKind::InvalidEscape,
                span: Span::new(ident_start, self.pos as u32),
                message: "Invalid Unicode escape in identifier".into(),
            });
            return None;
        };
        char::from_u32(val).or_else(|| {
            self.push_error(JsParseError {
                kind: JsParseErrorKind::InvalidEscape,
                span: Span::new(ident_start, self.pos as u32),
                message: "Invalid Unicode code point in identifier escape".into(),
            });
            None
        })
    }

    /// Consume a Unicode character if it satisfies `pred`. Returns true if consumed.
    fn advance_if_unicode(&mut self, pred: fn(char) -> bool) -> bool {
        let remaining = &self.source[self.pos..];
        if let Some(c) = decode_first_char(remaining) {
            if pred(c) {
                self.pos += c.len_utf8();
                return true;
            }
        }
        false
    }

    /// A9: Consume a Unicode `ID_Start` character. Returns true if consumed.
    fn advance_unicode_id_start(&mut self) -> bool {
        self.advance_if_unicode(is_id_start)
    }

    /// Consume a Unicode `ID_Continue` character. Returns true if consumed.
    fn advance_unicode_id_continue(&mut self) -> bool {
        self.advance_if_unicode(is_id_continue)
    }

    /// R1: Shared identifier body loop for regular, private, and escaped identifiers.
    fn lex_identifier_chars(&mut self, buf: &mut String, span_start: u32, has_escape: &mut bool) {
        loop {
            if buf.len() >= Self::MAX_IDENTIFIER_LEN {
                self.push_error(JsParseError {
                    kind: JsParseErrorKind::UnexpectedToken,
                    span: Span::new(span_start, self.pos as u32),
                    message: "Identifier exceeds maximum length".into(),
                });
                // M4: skip remaining identifier chars to avoid orphaning tokens
                while let Some(b) = self.peek() {
                    if super::is_ascii_id_continue(b) {
                        self.pos += 1;
                    } else if b >= 0x80 {
                        if !self.advance_unicode_id_continue() {
                            break;
                        }
                    } else if b == b'\\' && self.peek_at(1) == Some(b'u') {
                        let _ = self.lex_unicode_escape_in_ident(span_start);
                    } else {
                        break;
                    }
                }
                break;
            }
            match self.peek() {
                Some(b) if super::is_ascii_id_continue(b) => {
                    buf.push(b as char);
                    self.pos += 1;
                }
                Some(b'\\') if self.peek_at(1) == Some(b'u') => {
                    *has_escape = true;
                    if let Some(c) = self.lex_unicode_escape_in_ident(span_start) {
                        self.validate_unicode_escape_char(c, buf.is_empty(), span_start);
                        buf.push(c);
                    }
                }
                Some(b) if b >= 0x80 => {
                    let prev_pos = self.pos;
                    // A9: first character must be ID_Start, subsequent must be ID_Continue
                    let consumed = if buf.is_empty() {
                        self.advance_unicode_id_start()
                    } else {
                        self.advance_unicode_id_continue()
                    };
                    if consumed {
                        if let Ok(s) = std::str::from_utf8(&self.source[prev_pos..self.pos]) {
                            buf.push_str(s);
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
    }

    pub(super) fn lex_private_identifier(&mut self) -> TokenKind {
        let hash_pos = self.pos as u32;
        self.pos += 1; // skip #
        let mut name = String::new();
        let mut has_escape = false;
        self.lex_identifier_chars(&mut name, hash_pos, &mut has_escape);
        // L4: empty private identifier `#` with no name
        if name.is_empty() {
            self.push_error(JsParseError {
                kind: JsParseErrorKind::UnexpectedToken,
                span: Span::new(hash_pos, self.pos as u32),
                message: "Empty private identifier".into(),
            });
        }
        let atom = self.interner.intern(&name);
        TokenKind::PrivateIdentifier(atom)
    }
}
