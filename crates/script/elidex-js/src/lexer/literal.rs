//! String, numeric, template, and regexp literal lexing.

use crate::error::{JsParseError, JsParseErrorKind};
use crate::span::Span;
use crate::token::TokenKind;

use super::{hex_digit, Lexer};

/// Extract a numeric text slice from source bytes, removing `_` separators.
/// Returns `Cow::Borrowed` when no separators are present (common case).
fn number_text(source: &[u8], range: std::ops::Range<usize>) -> std::borrow::Cow<'_, str> {
    let s = source
        .get(range)
        .and_then(|b| std::str::from_utf8(b).ok())
        .unwrap_or("0");
    if s.contains('_') {
        std::borrow::Cow::Owned(s.replace('_', ""))
    } else {
        std::borrow::Cow::Borrowed(s)
    }
}

impl Lexer<'_> {
    /// L1: Maximum string/template literal length (bytes) to prevent pathological memory usage.
    pub(super) const MAX_LITERAL_LEN: usize = 1 << 24; // 16 MiB

    // ── String Literals ──────────────────────────────────────────────

    pub(super) fn lex_string(&mut self, quote: u8) -> TokenKind {
        let start = self.pos as u32;
        self.pos += 1; // skip opening quote
        let content_start = self.pos;

        // R4 fast path: scan for simple ASCII strings with no escapes.
        loop {
            match self.peek() {
                Some(b) if b == quote => {
                    // Simple string — each ASCII byte becomes a u16.
                    let bytes = &self.source[content_start..self.pos];
                    let atom = if bytes.len() <= 64 {
                        let mut stack = [0u16; 64];
                        for (i, &b) in bytes.iter().enumerate() {
                            stack[i] = u16::from(b);
                        }
                        self.interner.intern_wtf16(&stack[..bytes.len()])
                    } else {
                        let units: Vec<u16> = bytes.iter().map(|&b| u16::from(b)).collect();
                        self.interner.intern_wtf16(&units)
                    };
                    self.pos += 1; // skip closing quote
                    return TokenKind::StringLiteral(atom);
                }
                Some(b'\\' | b'\n' | b'\r') | None => break, // need slow path
                Some(b) if b >= 0x80 => break,               // multi-byte
                Some(_) => self.pos += 1,
            }
        }

        // Slow path: reset and build Vec<u16> char-by-char.
        self.pos = content_start;
        let mut units: Vec<u16> = Vec::new();

        loop {
            if units.len() >= Self::MAX_LITERAL_LEN {
                self.push_error(JsParseError {
                    kind: JsParseErrorKind::InvalidString,
                    span: Span::new(start, self.pos as u32),
                    message: "String literal exceeds maximum length".into(),
                });
                // Skip to closing quote
                while let Some(b) = self.peek() {
                    self.pos += 1;
                    if b == quote {
                        break;
                    }
                }
                break;
            }
            match self.peek() {
                Some(b) if b == quote => {
                    self.pos += 1;
                    break;
                }
                Some(b'\\') => {
                    self.pos += 1;
                    self.lex_escape_sequence_u16(&mut units, start);
                }
                // Line terminators are not allowed in string literals (except LS/PS per ES2019)
                None | Some(b'\n' | b'\r') => {
                    self.push_error(JsParseError {
                        kind: JsParseErrorKind::InvalidString,
                        span: Span::new(start, self.pos as u32),
                        message: "Unterminated string literal".into(),
                    });
                    break;
                }
                // LS U+2028 / PS U+2029 are allowed in strings (ES2019 JSON superset)
                Some(0xE2) if self.is_ls_ps() => {
                    if self.pos + 3 <= self.source.len() {
                        let bytes = &self.source[self.pos..self.pos + 3];
                        if let Ok(s) = std::str::from_utf8(bytes) {
                            let mut buf = [0u16; 2];
                            for ch in s.chars() {
                                let encoded = ch.encode_utf16(&mut buf);
                                units.extend_from_slice(encoded);
                            }
                        }
                    }
                    self.pos += 3;
                }
                Some(b) if b < 0x80 => {
                    units.push(u16::from(b));
                    self.pos += 1;
                }
                _ => {
                    // Multi-byte UTF-8
                    if let Some(c) = self.read_utf8_char() {
                        let mut buf = [0u16; 2];
                        let encoded = c.encode_utf16(&mut buf);
                        units.extend_from_slice(encoded);
                    }
                }
            }
        }

        let atom = self.interner.intern_wtf16(&units);
        TokenKind::StringLiteral(atom)
    }

    /// WTF-16 variant: push a Unicode code point to a `Vec<u16>`.
    /// Lone surrogates are preserved directly (valid in WTF-16).
    fn push_unicode_codepoint_u16(&mut self, val: u32, out: &mut Vec<u16>, literal_start: u32) {
        if (0xD800..=0xDFFF).contains(&val) {
            // Lone surrogate — push directly as u16
            #[allow(clippy::cast_possible_truncation)]
            out.push(val as u16);
        } else if val <= 0xFFFF {
            // BMP
            #[allow(clippy::cast_possible_truncation)]
            out.push(val as u16);
        } else if val <= 0x10_FFFF {
            // Supplementary — encode as surrogate pair
            let v = val - 0x1_0000;
            #[allow(clippy::cast_possible_truncation)]
            {
                out.push((0xD800 + (v >> 10)) as u16);
                out.push((0xDC00 + (v & 0x3FF)) as u16);
            }
        } else {
            self.push_error(JsParseError {
                kind: JsParseErrorKind::InvalidEscape,
                span: Span::new(literal_start, self.pos as u32),
                message: format!("Unicode code point U+{val:X} is out of range"),
            });
        }
    }

    /// WTF-16 variant of `lex_escape_sequence`: pushes u16 code units.
    #[allow(clippy::too_many_lines)]
    pub(super) fn lex_escape_sequence_u16(&mut self, out: &mut Vec<u16>, literal_start: u32) {
        match self.advance() {
            None => {
                self.push_error(JsParseError {
                    kind: JsParseErrorKind::InvalidEscape,
                    span: Span::new(literal_start, self.pos as u32),
                    message: "Unterminated escape sequence".into(),
                });
            }
            Some(b'n') => out.push(u16::from(b'\n')),
            Some(b'r') => out.push(u16::from(b'\r')),
            Some(b't') => out.push(u16::from(b'\t')),
            Some(b'\\') => out.push(u16::from(b'\\')),
            Some(b'\'') => out.push(u16::from(b'\'')),
            Some(b'"') => out.push(u16::from(b'"')),
            Some(b'0') => {
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.push_error(JsParseError {
                        kind: JsParseErrorKind::InvalidEscape,
                        span: Span::new(literal_start, self.pos as u32),
                        message: "Octal escape sequences are not allowed in strict mode".into(),
                    });
                } else {
                    out.push(0);
                }
            }
            Some(b'b') => out.push(0x0008),
            Some(b'f') => out.push(0x000C),
            Some(b'v') => out.push(0x000B),
            Some(b'x') => {
                if let Some(val) = self.read_hex_digits(2) {
                    #[allow(clippy::cast_possible_truncation)]
                    out.push(val as u16);
                } else {
                    self.push_error(JsParseError {
                        kind: JsParseErrorKind::InvalidEscape,
                        span: Span::new(literal_start, self.pos as u32),
                        message: "Invalid hex escape sequence".into(),
                    });
                }
            }
            Some(b'u') => {
                let val = if self.peek() == Some(b'{') {
                    self.pos += 1;
                    self.read_braced_unicode_codepoint()
                } else {
                    self.read_hex_digits(4)
                };
                if let Some(v) = val {
                    self.push_unicode_codepoint_u16(v, out, literal_start);
                } else {
                    self.push_error(JsParseError {
                        kind: JsParseErrorKind::InvalidEscape,
                        span: Span::new(literal_start, self.pos as u32),
                        message: "Invalid Unicode escape sequence".into(),
                    });
                }
            }
            // Line continuation
            Some(b'\n') => {
                self.record_line_start();
            }
            Some(b'\r') => {
                if self.peek() == Some(b'\n') {
                    self.pos += 1;
                }
                self.record_line_start();
            }
            Some(0xE2)
                if self.peek() == Some(0x80) && matches!(self.peek_at(1), Some(0xA8 | 0xA9)) =>
            {
                self.pos += 2;
                self.record_line_start();
            }
            Some(b) if matches!(b, b'1'..=b'9') => {
                self.push_error(JsParseError {
                    kind: JsParseErrorKind::InvalidEscape,
                    span: Span::new(literal_start, self.pos as u32),
                    message: "Numeric escape sequences are not allowed in strict mode".into(),
                });
                out.push(u16::from(b));
            }
            Some(b) => {
                if b < 0x80 {
                    self.push_error(JsParseError {
                        kind: JsParseErrorKind::InvalidEscape,
                        span: Span::new(literal_start, self.pos as u32),
                        message: format!(
                            "Invalid escape sequence '\\{}' in strict mode",
                            b as char
                        ),
                    });
                    out.push(u16::from(b));
                } else {
                    self.pos -= 1;
                    if let Some(c) = self.read_utf8_char() {
                        self.push_error(JsParseError {
                            kind: JsParseErrorKind::InvalidEscape,
                            span: Span::new(literal_start, self.pos as u32),
                            message: format!("Invalid escape sequence '\\{c}' in strict mode"),
                        });
                        let mut buf = [0u16; 2];
                        let encoded = c.encode_utf16(&mut buf);
                        out.extend_from_slice(encoded);
                    }
                }
            }
        }
    }

    pub(super) fn read_hex_digits(&mut self, count: usize) -> Option<u32> {
        let mut val = 0u32;
        for _ in 0..count {
            let b = self.peek()?;
            let d = hex_digit(b)?;
            val = val * 16 + u32::from(d);
            self.pos += 1;
        }
        Some(val)
    }

    /// L2: Only examines up to 4 bytes (max UTF-8 char length) to avoid O(n) validation.
    pub(super) fn read_utf8_char(&mut self) -> Option<char> {
        let remaining = &self.source[self.pos..];
        if remaining.is_empty() {
            return None;
        }
        let len = remaining.len().min(4);
        let s = match std::str::from_utf8(&remaining[..len]) {
            Ok(s) => s,
            Err(e) => {
                let valid = e.valid_up_to();
                if valid > 0 {
                    // Safety: valid_up_to guarantees this range is valid UTF-8
                    std::str::from_utf8(&remaining[..valid]).ok()?
                } else {
                    // Invalid UTF-8 at current position — skip the byte to avoid infinite loop
                    self.pos += 1;
                    return None;
                }
            }
        };
        let c = s.chars().next()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    // ── Numeric Literals ─────────────────────────────────────────────

    pub(super) fn lex_number(&mut self) -> TokenKind {
        let start = self.pos;

        if self.peek() == Some(b'0') {
            match self.peek_at(1) {
                Some(b'x' | b'X') => return self.lex_hex_number(start),
                Some(b'b' | b'B') => return self.lex_binary_number(start),
                Some(b'o' | b'O') => return self.lex_octal_number(start),
                // L1: Leading zero followed by separator — reject (§12.9.3)
                Some(b'_') => {
                    self.push_error(JsParseError {
                        kind: JsParseErrorKind::InvalidNumber,
                        span: Span::new(start as u32, start as u32 + 2),
                        message: "Numeric separator not allowed after leading zero".into(),
                    });
                }
                // A1: Leading zero followed by digit — reject in strict mode
                // V16: distinguish octal (00-07) from non-octal (08-09) in message
                Some(d @ b'0'..=b'9') => {
                    let msg = if d <= b'7' {
                        "Legacy octal literals are not allowed in strict mode"
                    } else {
                        "Decimals with leading zeros are not allowed in strict mode"
                    };
                    self.push_error(JsParseError {
                        kind: JsParseErrorKind::InvalidNumber,
                        span: Span::new(start as u32, start as u32 + 2),
                        message: msg.into(),
                    });
                    // Continue parsing as decimal for error recovery
                }
                _ => {}
            }
        }

        // Decimal integer part
        self.skip_decimal_digits();

        // Fractional part
        if self.peek() == Some(b'.') {
            self.pos += 1;
            self.skip_decimal_digits();
        }

        // Exponent
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            // A3: empty exponent
            let before = self.pos;
            self.skip_decimal_digits();
            if self.pos == before {
                self.push_error(JsParseError {
                    kind: JsParseErrorKind::InvalidNumber,
                    span: Span::new(start as u32, self.pos as u32),
                    message: "Exponent requires at least one digit".into(),
                });
            }
        }

        // BigInt suffix — only valid for integers (no . or e/E before n)
        if self.peek() == Some(b'n') {
            let raw = &self.source[start..self.pos];
            let has_dot = raw.contains(&b'.');
            let has_exp = raw.iter().any(|&b| b == b'e' || b == b'E');
            if has_dot || has_exp {
                self.push_error(JsParseError {
                    kind: JsParseErrorKind::InvalidNumber,
                    span: Span::new(start as u32, self.pos as u32 + 1),
                    message: "BigInt literal cannot have decimal point or exponent".into(),
                });
            }
            self.pos += 1;
            let text = number_text(self.source, start..self.pos.saturating_sub(1));
            let atom = self.interner.intern(&text);
            self.check_identifier_after_number(start);
            return TokenKind::BigIntLiteral(atom);
        }

        let text = number_text(self.source, start..self.pos);
        let val = text.parse::<f64>().unwrap_or(f64::NAN);
        self.check_identifier_after_number(start);
        TokenKind::NumericLiteral(val)
    }

    /// V18a: §12.9.3 — `NumericLiteral` immediately followed by `IdentifierStart` is a syntax error.
    fn check_identifier_after_number(&mut self, start: usize) {
        let is_id_start = match self.peek() {
            // L2: `\` is only an identifier start when followed by `u` (Unicode escape)
            Some(b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'$') => true,
            Some(b'\\') => self.peek_at(1) == Some(b'u'),
            Some(b) if b >= 0x80 => self.is_unicode_id_start(),
            _ => false,
        };
        if is_id_start {
            self.push_error(JsParseError {
                kind: JsParseErrorKind::InvalidNumber,
                span: Span::new(start as u32, self.pos as u32 + 1),
                message: "Identifier starts immediately after numeric literal".into(),
            });
        }
    }

    // R3: hex now routed through lex_prefixed_number
    fn lex_hex_number(&mut self, start: usize) -> TokenKind {
        self.lex_prefixed_number(start, 16, |b: u8| b.is_ascii_hexdigit())
    }

    fn lex_binary_number(&mut self, start: usize) -> TokenKind {
        self.lex_prefixed_number(start, 2, |b| matches!(b, b'0' | b'1'))
    }

    fn lex_octal_number(&mut self, start: usize) -> TokenKind {
        self.lex_prefixed_number(start, 8, |b| matches!(b, b'0'..=b'7'))
    }

    /// Unified parser for binary (0b), octal (0o) literals with separator validation.
    fn lex_prefixed_number(
        &mut self,
        start: usize,
        radix: u32,
        is_digit: impl Fn(u8) -> bool,
    ) -> TokenKind {
        self.pos += 2; // skip prefix (0b/0o)
        let before = self.pos;
        self.skip_digits(is_digit);
        // A4: empty digits after prefix
        if self.pos == before {
            self.push_error(JsParseError {
                kind: JsParseErrorKind::InvalidNumber,
                span: Span::new(start as u32, self.pos as u32),
                message: "Expected digits after numeric prefix".into(),
            });
        }

        if self.peek() == Some(b'n') {
            self.pos += 1;
            let text = number_text(self.source, start..self.pos.saturating_sub(1));
            let atom = self.interner.intern(&text);
            self.check_identifier_after_number(start);
            return TokenKind::BigIntLiteral(atom);
        }

        let digits = number_text(self.source, start + 2..self.pos);
        // L2: fallback to f64 parsing for values > u64::MAX
        #[allow(clippy::cast_precision_loss)]
        let val = match u64::from_str_radix(&digits, radix) {
            Ok(v) => v as f64,
            Err(_) => parse_large_radix(&digits, radix),
        };
        self.check_identifier_after_number(start);
        TokenKind::NumericLiteral(val)
    }

    /// Skip digits matching `is_digit`, validating numeric separators.
    fn skip_digits(&mut self, is_digit: impl Fn(u8) -> bool) {
        let start = self.pos;
        let mut prev_sep = false;
        while let Some(b) = self.peek() {
            if is_digit(b) {
                prev_sep = false;
                self.pos += 1;
            } else if b == b'_' {
                if self.pos == start || prev_sep {
                    self.separator_error(self.pos as u32);
                }
                prev_sep = true;
                self.pos += 1;
            } else {
                break;
            }
        }
        if prev_sep && self.pos > start {
            self.separator_error(self.pos as u32 - 1);
        }
    }

    pub(super) fn skip_decimal_digits(&mut self) {
        self.skip_digits(|b| b.is_ascii_digit());
    }

    // ── Template Literals ────────────────────────────────────────────

    /// Build the appropriate template token kind based on position (head/tail vs middle/nosub).
    fn template_end_token(
        &mut self,
        is_head: bool,
        is_end: bool,
        cooked_valid: bool,
        cooked: &[u16],
        raw: &[u16],
    ) -> TokenKind {
        let raw_atom = self.interner.intern_wtf16(raw);
        let cooked_opt = if cooked_valid {
            Some(self.interner.intern_wtf16(cooked))
        } else {
            None
        };
        match (is_head, is_end) {
            (true, true) => TokenKind::TemplateNoSub {
                cooked: cooked_opt,
                raw: raw_atom,
            },
            (true, false) => TokenKind::TemplateHead {
                cooked: cooked_opt,
                raw: raw_atom,
            },
            (false, true) => TokenKind::TemplateTail {
                cooked: cooked_opt,
                raw: raw_atom,
            },
            (false, false) => TokenKind::TemplateMiddle {
                cooked: cooked_opt,
                raw: raw_atom,
            },
        }
    }

    pub(super) fn lex_template(&mut self) -> TokenKind {
        self.pos += 1; // skip `
        self.lex_template_inner(true)
    }

    #[allow(clippy::too_many_lines)]
    // Single match dispatcher over token/AST variants.
    pub(super) fn lex_template_inner(&mut self, is_head: bool) -> TokenKind {
        let start_pos = self.pos as u32;
        let mut cooked: Vec<u16> = Vec::new();
        let mut raw: Vec<u16> = Vec::new();
        let mut cooked_valid = true;

        loop {
            // S9: check both cooked and raw for size limits
            if cooked.len() >= Self::MAX_LITERAL_LEN || raw.len() >= Self::MAX_LITERAL_LEN {
                self.push_error(JsParseError {
                    kind: JsParseErrorKind::UnterminatedTemplate,
                    span: Span::new(start_pos, self.pos as u32),
                    message: "Template literal exceeds maximum length".into(),
                });
                // Skip to closing backtick
                while let Some(b) = self.peek() {
                    self.pos += 1;
                    if b == b'`' {
                        break;
                    }
                }
                return self.template_end_token(is_head, true, cooked_valid, &cooked, &raw);
            }
            match self.peek() {
                None => {
                    self.push_error(JsParseError {
                        kind: JsParseErrorKind::UnterminatedTemplate,
                        span: Span::new(start_pos, self.pos as u32),
                        message: "Unterminated template literal".into(),
                    });
                    return self.template_end_token(is_head, true, cooked_valid, &cooked, &raw);
                }
                Some(b'`') => {
                    self.pos += 1;
                    return self.template_end_token(is_head, true, cooked_valid, &cooked, &raw);
                }
                Some(b'$') if self.peek_at(1) == Some(b'{') => {
                    self.pos += 2; // skip ${
                    return self.template_end_token(is_head, false, cooked_valid, &cooked, &raw);
                }
                Some(b'\\') => {
                    // E2/§12.9.6: capture raw escape text. Invalid escapes in templates
                    // must NOT produce errors — they set cooked to None (for tagged templates).
                    // Errors are only emitted by the parser for untagged templates.
                    let esc_start = self.pos;
                    let err_count = self.errors.len();
                    self.pos += 1;
                    self.lex_escape_sequence_u16(&mut cooked, start_pos);
                    if self.errors.len() > err_count {
                        cooked_valid = false;
                        // Remove errors generated during template escape processing
                        self.errors.truncate(err_count);
                    }
                    // Raw: capture source bytes, normalize CR/CRLF → LF
                    let esc_bytes = &self.source[esc_start..self.pos];
                    let esc_raw = std::str::from_utf8(esc_bytes).unwrap_or("\\");
                    let mut buf = [0u16; 2];
                    for c in esc_raw.chars() {
                        if c == '\r' {
                            // skip — \n follows (CRLF) or we push \n below
                        } else {
                            let encoded = c.encode_utf16(&mut buf);
                            raw.extend_from_slice(encoded);
                        }
                    }
                    // If CR was present without following LF, push LF
                    if esc_bytes.contains(&b'\r') && !esc_bytes.contains(&b'\n') {
                        raw.push(u16::from(b'\n'));
                    }
                }
                Some(b'\n') => {
                    cooked.push(u16::from(b'\n'));
                    raw.push(u16::from(b'\n'));
                    self.pos += 1;
                    self.record_line_start();
                }
                Some(b'\r') => {
                    cooked.push(u16::from(b'\n')); // normalize CR/CRLF to LF
                    raw.push(u16::from(b'\n'));
                    self.pos += 1;
                    if self.peek() == Some(b'\n') {
                        self.pos += 1;
                    }
                    self.record_line_start();
                }
                // A3/A8: LS U+2028 / PS U+2029 are line terminators in templates
                // TV preserves actual LS/PS (ES2023 §12.9.6); TRV preserves original
                Some(0xE2) if self.is_ls_ps() => {
                    // A8: push actual LS/PS char, not '\n' (CR/CRLF normalize, LS/PS do not)
                    let ls_ps_u16 = if self.peek_at(2) == Some(0xA8) {
                        0x2028u16
                    } else {
                        0x2029u16
                    };
                    cooked.push(ls_ps_u16);
                    // Raw: encode the LS/PS char to u16 (BMP, single code unit)
                    raw.push(ls_ps_u16);
                    self.pos += 3;
                    self.record_line_start();
                }
                Some(b) if b < 0x80 => {
                    cooked.push(u16::from(b));
                    raw.push(u16::from(b));
                    self.pos += 1;
                }
                _ => {
                    if let Some(c) = self.read_utf8_char() {
                        let mut buf = [0u16; 2];
                        let encoded = c.encode_utf16(&mut buf);
                        cooked.extend_from_slice(encoded);
                        raw.extend_from_slice(encoded);
                    }
                }
            }
        }
    }
}

/// L2: fallback for numbers that overflow u64 — compute as f64 via repeated multiply.
fn parse_large_radix(s: &str, radix: u32) -> f64 {
    let mut val = 0.0_f64;
    let base = f64::from(radix);
    for c in s.chars() {
        let d = c.to_digit(radix).unwrap_or(0);
        val = val * base + f64::from(d);
    }
    val
}
