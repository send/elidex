//! Regexp escape sequence parsing and backreference validation.
//!
//! Handles `\d`, `\w`, `\s`, `\uHHHH`, `\u{...}`, `\xHH`, `\cX`, `\p{...}`,
//! identity escapes, backreferences (`\1`), and named backreferences (`\k<name>`).

use super::{AssertionKind, CharClassAtom, EscapeKind, RegExpError, RegExpNode};

use super::parser::RegExpParser;

impl RegExpParser<'_> {
    /// R5: Parse `\uHHHH` or `\u{HHHH}`, returning the code point value.
    /// The caller has already consumed the `u`. Returns `None` on invalid escape.
    pub(super) fn parse_unicode_escape_value(&mut self) -> Option<u32> {
        if self.peek() == Some('{') {
            self.advance();
            let mut val = 0u32;
            let mut count = 0u32;
            let mut found_close = false;
            while let Some(c) = self.peek() {
                if c == '}' {
                    self.advance();
                    found_close = true;
                    break;
                }
                if let Some(d) = hex_val(c) {
                    // M4: limit to 6 hex digits (max valid codepoint 10FFFF)
                    if count >= 6 {
                        val = u32::MAX;
                    } else {
                        val = val * 16 + d;
                    }
                    count += 1;
                    self.advance();
                } else {
                    break;
                }
            }
            if !found_close || count == 0 {
                return None;
            }
            Some(val)
        } else {
            let mut val = 0u32;
            for _ in 0..4 {
                let d = self.advance().and_then(hex_val)?;
                val = val * 16 + d;
            }
            Some(val)
        }
    }

    /// R9: Reject surrogate code points in unicode mode.
    fn check_surrogate(&self, val: u32) -> Result<(), RegExpError> {
        if self.is_unicode() && (0xD800..=0xDFFF).contains(&val) {
            return Err(self.err(format!("Surrogate U+{val:04X} not allowed in unicode mode")));
        }
        Ok(())
    }

    /// Parse escape sequence (after `\`).
    pub(super) fn parse_escape(&mut self) -> Result<RegExpNode, RegExpError> {
        self.advance(); // skip backslash
        match self.peek() {
            Some('b') => {
                self.advance();
                Ok(RegExpNode::Assertion(AssertionKind::WordBoundary))
            }
            Some('B') => {
                self.advance();
                Ok(RegExpNode::Assertion(AssertionKind::NonWordBoundary))
            }
            Some('k') => {
                self.advance();
                if !self.eat('<') {
                    return Err(self.err("Expected '<' after \\k"));
                }
                let name = self.parse_group_name_raw()?;
                // B4: record for post-parse validation
                self.named_backreferences.push((name.clone(), self.pos));
                Ok(RegExpNode::NamedBackreference(name))
            }
            Some('p' | 'P') if self.is_unicode() => {
                let negated = self.peek() == Some('P');
                self.advance();
                if !self.eat('{') {
                    return Err(self.err("Expected '{' after \\p/\\P"));
                }
                let (name, value) = self.parse_unicode_property()?;
                // E1: §22.2.2.4 — negated sequence properties (\P{Basic_Emoji} etc.)
                // are a SyntaxError because negation of multi-codepoint strings is undefined.
                if negated
                    && value.is_none()
                    && self.flags.unicode_sets
                    && super::unicode_property::is_sequence_property_name(&name)
                {
                    return Err(self.err("\\P{...} cannot be used with sequence properties"));
                }
                Ok(RegExpNode::UnicodeProperty {
                    name,
                    value,
                    negated,
                })
            }
            Some(c @ '1'..='9') => {
                self.advance();
                // R12: accumulate u32 directly instead of building a String
                let mut num = (c as u32) - ('0' as u32);
                while let Some(d @ '0'..='9') = self.peek() {
                    self.advance();
                    num = num
                        .saturating_mul(10)
                        .saturating_add((d as u32) - ('0' as u32));
                }
                // B3: record for post-parse validation
                self.backreferences.push((num, self.pos));
                Ok(RegExpNode::Backreference(num))
            }
            _ => {
                let esc = self.parse_escape_kind()?;
                Ok(RegExpNode::Escape(esc))
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    // Single match dispatcher over token/AST variants.
    pub(super) fn parse_escape_kind(&mut self) -> Result<EscapeKind, RegExpError> {
        let c = self
            .advance()
            .ok_or_else(|| self.err("Unexpected end after escape"))?;

        Ok(match c {
            'd' => EscapeKind::Digit,
            'D' => EscapeKind::NonDigit,
            'w' => EscapeKind::Word,
            'W' => EscapeKind::NonWord,
            's' => EscapeKind::Whitespace,
            'S' => EscapeKind::NonWhitespace,
            't' => EscapeKind::Tab,
            'n' => EscapeKind::Newline,
            'r' => EscapeKind::CarriageReturn,
            'f' => EscapeKind::FormFeed,
            'v' => EscapeKind::VerticalTab,
            // B6: \cX control escape (A-Z, a-z)
            'c' => {
                if let Some(c @ ('A'..='Z' | 'a'..='z')) = self.peek() {
                    self.advance();
                    let code = (c as u32) % 32;
                    EscapeKind::Identity(char::from_u32(code).unwrap_or('\0'))
                } else {
                    // R3: \c without valid control letter is always an error (no Annex B)
                    return Err(self.err("Invalid control escape \\c"));
                }
            }
            '0' => {
                // R2: \0 followed by digit is always an error (no Annex B octal support)
                if matches!(self.peek(), Some('0'..='9')) {
                    return Err(self.err("\\0 followed by digit is not allowed"));
                }
                EscapeKind::Null
            }
            'x' => {
                let h1 = self
                    .advance()
                    .and_then(hex_val)
                    .ok_or_else(|| self.err("Invalid hex escape"))?;
                let h2 = self
                    .advance()
                    .and_then(hex_val)
                    .ok_or_else(|| self.err("Invalid hex escape"))?;
                let val = h1 * 16 + h2;
                EscapeKind::Hex(char::from_u32(val).unwrap_or('\0'))
            }
            'u' => {
                let is_braced = self.peek() == Some('{');
                // B6: \u{...} is only valid in unicode mode
                if is_braced && !self.is_unicode() {
                    return Err(self.err("\\u{...} escape is only valid with unicode flag"));
                }
                // R5: shared unicode escape parser
                let val = self.parse_unicode_escape_value().ok_or_else(|| {
                    if is_braced {
                        self.err("Unterminated unicode escape \\u{...}")
                    } else {
                        self.err("Invalid unicode escape")
                    }
                })?;
                // B26: value must be <= 0x10FFFF
                if val > 0x10_FFFF {
                    return Err(self.err(format!(
                        "Unicode escape value {val:#X} exceeds maximum (0x10FFFF)"
                    )));
                }
                self.check_surrogate(val)?;
                EscapeKind::Unicode(char::from_u32(val).unwrap_or('\u{FFFD}'))
            }
            _ => {
                // A20: in unicode mode, only SyntaxCharacter and `/` may be identity-escaped
                if self.is_unicode() && !is_regexp_syntax_char(c) && c != '/' {
                    return Err(
                        self.err(format!("Invalid identity escape '\\{c}' in unicode mode"))
                    );
                }
                EscapeKind::Identity(c)
            }
        })
    }

    /// B3/B4/R7: Validate backreferences after full parse (supports forward references).
    /// Always validated — no Annex B relaxation.
    pub(super) fn validate_backreferences(&self) -> Result<(), RegExpError> {
        for &(num, offset) in &self.backreferences {
            if num > self.group_count {
                return Err(RegExpError {
                    message: format!(
                        "Invalid backreference \\{num}: only {gc} capturing group(s)",
                        gc = self.group_count
                    ),
                    offset,
                });
            }
        }
        // Named backreferences are always validated (no Annex B exception)
        for (name, offset) in &self.named_backreferences {
            if !self.seen_group_names.contains(name.as_str()) {
                return Err(RegExpError {
                    message: format!("Invalid named backreference \\k<{name}>: group not found"),
                    offset: *offset,
                });
            }
        }
        Ok(())
    }

    /// §22.2.2.9 — Parse and validate `\p{Name}` or `\p{Name=Value}`.
    fn parse_unicode_property(&mut self) -> Result<(String, Option<String>), RegExpError> {
        let mut name = String::new();
        let mut value = None;
        let mut reading_value = false;

        loop {
            // S11: length limit to prevent OOM
            let total_len = name.len() + value.as_ref().map_or(0, String::len);
            if total_len >= 256 {
                return Err(self.err("Unicode property name/value exceeds maximum length"));
            }
            match self.peek() {
                Some('}') => {
                    self.advance();
                    let parsed_value = if reading_value {
                        Some(value.unwrap_or_default())
                    } else {
                        None
                    };
                    // R4: validate against Unicode property tables
                    let is_v = self.flags.unicode_sets;
                    if let Err(msg) =
                        super::unicode_property::validate(&name, parsed_value.as_deref(), is_v)
                    {
                        return Err(self.err(msg));
                    }
                    return Ok((name, parsed_value));
                }
                Some('=') if !reading_value => {
                    self.advance();
                    reading_value = true;
                    value = Some(String::new());
                }
                Some(c) if c.is_alphanumeric() || c == '_' => {
                    self.advance();
                    if reading_value {
                        // Safety: `value` is set to `Some` when `reading_value` becomes true
                        value
                            .as_mut()
                            .expect("value initialized with reading_value")
                            .push(c);
                    } else {
                        name.push(c);
                    }
                }
                _ => {
                    return Err(self.err("Invalid unicode property escape"));
                }
            }
        }
    }
}

/// A20: `SyntaxCharacter` set per ES spec 12.8.5.1
fn is_regexp_syntax_char(c: char) -> bool {
    matches!(
        c,
        '^' | '$' | '\\' | '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|'
    )
}

pub(super) fn hex_val(c: char) -> Option<u32> {
    match c {
        '0'..='9' => Some(c as u32 - '0' as u32),
        'a'..='f' => Some(c as u32 - 'a' as u32 + 10),
        'A'..='F' => Some(c as u32 - 'A' as u32 + 10),
        _ => None,
    }
}

/// Extract a single character from a class atom (for range validation).
/// Returns None for class escapes like \d, \w, \s that aren't single chars.
pub(super) fn class_atom_char(atom: &CharClassAtom) -> Option<char> {
    match atom {
        CharClassAtom::Literal(c) => Some(*c),
        CharClassAtom::Escape(esc) => esc.to_char(),
        // Nested classes and string alternatives are not single chars
        CharClassAtom::NestedClass(_) | CharClassAtom::StringAlternative(_) => None,
    }
}
