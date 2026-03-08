//! Regexp literal lexing (separated from `literal.rs` for file size).

use crate::error::{JsParseError, JsParseErrorKind};
use crate::span::Span;
use crate::token::TokenKind;

use super::Lexer;

impl Lexer<'_> {
    fn regexp_unterminated(&mut self, start: u32, pattern: &str) -> TokenKind {
        self.push_error(JsParseError {
            kind: JsParseErrorKind::InvalidRegExp,
            span: Span::new(start, self.pos as u32),
            message: "Unterminated regular expression".into(),
        });
        let pat_atom = self.interner.intern(pattern);
        let flags_atom = self.interner.intern("");
        TokenKind::RegExpLiteral {
            pattern: pat_atom,
            flags: flags_atom,
        }
    }

    // B7: pub(crate) so parser can rescan slash as regexp
    pub(crate) fn lex_regexp(&mut self) -> TokenKind {
        let start = self.pos as u32;
        self.pos += 1; // skip opening /
        let mut pattern = String::new();
        let mut in_class = false;

        loop {
            // S8: size limit to prevent OOM from huge regexp patterns
            if pattern.len() >= Self::MAX_LITERAL_LEN {
                self.push_error(JsParseError {
                    kind: JsParseErrorKind::InvalidRegExp,
                    span: Span::new(start, self.pos as u32),
                    message: "Regular expression exceeds maximum length".into(),
                });
                // L6: Skip to closing / for recovery (reset in_class for reliable termination)
                while let Some(b) = self.peek() {
                    self.pos += 1;
                    if b == b'/' {
                        break;
                    }
                    if matches!(b, b'\n' | b'\r') {
                        break;
                    }
                }
                let pat_atom = self.interner.intern(&pattern);
                let flags_atom = self.interner.intern("");
                return TokenKind::RegExpLiteral {
                    pattern: pat_atom,
                    flags: flags_atom,
                };
            }
            match self.peek() {
                // B3: LS/PS are also line terminators in regexp literals
                None | Some(b'\n' | b'\r') => return self.regexp_unterminated(start, &pattern),
                Some(0xE2) if self.is_ls_ps() => return self.regexp_unterminated(start, &pattern),
                Some(b'/') if !in_class => {
                    self.pos += 1;
                    break;
                }
                Some(b'[') => {
                    in_class = true;
                    pattern.push('[');
                    self.pos += 1;
                }
                Some(b']') => {
                    in_class = false;
                    pattern.push(']');
                    self.pos += 1;
                }
                Some(b'\\') => {
                    pattern.push('\\');
                    self.pos += 1;
                    match self.peek() {
                        // Line terminators end the regexp (backslash sequence incomplete)
                        None | Some(b'\n' | b'\r') => {}
                        // LS/PS are also line terminators
                        Some(0xE2) if self.is_ls_ps() => {}
                        // Multi-byte: consume full character
                        Some(b) if b >= 0x80 => {
                            if let Some(c) = self.read_utf8_char() {
                                pattern.push(c);
                            }
                        }
                        Some(b) => {
                            pattern.push(b as char);
                            self.pos += 1;
                        }
                    }
                }
                Some(b) if b < 0x80 => {
                    pattern.push(b as char);
                    self.pos += 1;
                }
                _ => {
                    if let Some(c) = self.read_utf8_char() {
                        pattern.push(c);
                    }
                }
            }
        }

        let flags = self.lex_regexp_flags();

        // A2: Validate regexp flags (invalid chars, duplicates, u+v mutual exclusion)
        if let Err(e) = crate::regexp::parse_flags(&flags) {
            self.push_error(JsParseError {
                kind: JsParseErrorKind::InvalidRegExp,
                span: Span::new(start, self.pos as u32),
                message: e.message,
            });
        }

        let pat_atom = self.interner.intern(&pattern);
        let flags_atom = self.interner.intern(&flags);
        TokenKind::RegExpLiteral {
            pattern: pat_atom,
            flags: flags_atom,
        }
    }

    /// S6: Consume regexp flags — `IdentifierPartChar` per spec (ASCII alphanumeric,
    /// Unicode `ID_Continue`, `\uHHHH`/`\u{HHHH}` escapes). Invalid flags are caught
    /// by the caller's flag validator.
    fn lex_regexp_flags(&mut self) -> String {
        let mut flags = String::new();
        while let Some(b) = self.peek() {
            if super::is_ascii_id_continue(b) {
                flags.push(b as char);
                self.pos += 1;
            } else if b == b'\\' && self.peek_at(1) == Some(b'u') {
                let esc_start = self.pos;
                self.pos += 2;
                let val = if self.peek() == Some(b'{') {
                    self.read_braced_unicode_codepoint()
                } else {
                    self.read_hex_digits(4)
                };
                if let Some(v) = val {
                    if let Some(c) = char::from_u32(v) {
                        flags.push(c);
                    }
                } else {
                    self.pos = esc_start;
                    break;
                }
            } else if b >= 0x80 {
                if let Some(c) = self.read_utf8_char() {
                    if super::ident::is_id_continue(c) {
                        flags.push(c);
                    } else {
                        self.pos -= c.len_utf8();
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        flags
    }
}
