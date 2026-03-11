//! `RegExpParser` — internal regexp pattern parser.
//!
//! Implements ES2025 §22.2.1 (main spec) WITHOUT Annex B extensions.
//! Non-unicode mode does NOT relax identity escapes, control escapes,
//! backreference validation, or `\0` + digit handling. See `docs/design`.

use std::collections::HashSet;

use super::escape::class_atom_char;
use super::{
    AssertionKind, CharClassAtom, CharRange, EscapeKind, GroupKind, RegExpError, RegExpFlags,
    RegExpNode,
};

/// ES §12.7 `IdentifierStartChar`: `ID_Start` or `$` or `_`.
fn is_id_start(c: char) -> bool {
    unicode_ident::is_xid_start(c) || c == '$' || c == '_'
}

/// Maximum nesting depth for regexp groups.
const MAX_REGEXP_DEPTH: u32 = 1024;

/// Maximum number of elements (alternatives, terms, ranges) in a single regexp construct.
pub(super) const MAX_REGEXP_PARTS: usize = 65536;

pub(super) struct RegExpParser<'a> {
    pub(super) source: &'a str,
    pub(super) pos: usize, // byte offset into source
    pub(super) group_count: u32,
    pub(super) flags: &'a RegExpFlags,
    /// A14: track seen group names for duplicate detection.
    pub(super) seen_group_names: HashSet<String>,
    /// B3: collect numeric backreferences for post-parse validation.
    pub(super) backreferences: Vec<(u32, usize)>,
    /// B4: collect named backreferences for post-parse validation.
    pub(super) named_backreferences: Vec<(String, usize)>,
    depth: u32,
}

impl<'a> RegExpParser<'a> {
    pub(super) fn new(pattern: &'a str, flags: &'a RegExpFlags) -> Self {
        Self {
            source: pattern,
            pos: 0,
            group_count: 0,
            flags,
            seen_group_names: HashSet::new(),
            backreferences: Vec::new(),
            named_backreferences: Vec::new(),
            depth: 0,
        }
    }

    pub(super) fn source_len(&self) -> usize {
        self.source.len()
    }

    pub(super) fn is_unicode(&self) -> bool {
        self.flags.unicode || self.flags.unicode_sets
    }

    pub(super) fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    /// Peek at the next char after the current one (skipping one char).
    pub(super) fn peek_next(&self) -> Option<char> {
        let mut chars = self.source[self.pos..].chars();
        chars.next(); // skip current
        chars.next()
    }

    pub(super) fn advance(&mut self) -> Option<char> {
        let c = self.source[self.pos..].chars().next()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    pub(super) fn eat(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.pos += expected.len_utf8();
            true
        } else {
            false
        }
    }

    /// R5: Convenience constructor for `RegExpError` at current position.
    pub(super) fn err(&self, message: impl Into<String>) -> RegExpError {
        RegExpError {
            message: message.into(),
            offset: self.pos,
        }
    }

    /// Parse a disjunction: alternative (| alternative)*
    pub(super) fn parse_disjunction(&mut self) -> Result<RegExpNode, RegExpError> {
        self.depth += 1;
        if self.depth > MAX_REGEXP_DEPTH {
            self.depth -= 1;
            return Err(self.err("RegExp pattern nesting too deep"));
        }
        let result = self.parse_disjunction_inner();
        self.depth -= 1;
        result
    }

    fn parse_disjunction_inner(&mut self) -> Result<RegExpNode, RegExpError> {
        // B30: ES2025 — duplicate named groups allowed across different alternatives.
        // Snapshot names *before* first alternative so each alt starts from the same base.
        let outer_names = self.seen_group_names.clone();

        let first = self.parse_alternative()?;

        if !self.eat('|') {
            return Ok(first);
        }

        // R11: Merge names from first alternative.
        let mut merged = self.seen_group_names.clone();
        let mut alternatives = vec![first];

        loop {
            // Reset to outer scope for each alternative.
            if outer_names.is_empty() {
                self.seen_group_names.clear();
            } else {
                self.seen_group_names.clone_from(&outer_names);
            }
            if alternatives.len() >= MAX_REGEXP_PARTS {
                return Err(self.err("Too many elements in regexp"));
            }
            alternatives.push(self.parse_alternative()?);
            merged.extend(self.seen_group_names.iter().cloned());
            if !self.eat('|') {
                break;
            }
        }

        self.seen_group_names = merged;
        Ok(RegExpNode::Disjunction(alternatives))
    }

    /// Parse an alternative: term*
    fn parse_alternative(&mut self) -> Result<RegExpNode, RegExpError> {
        let mut terms = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            if terms.len() >= MAX_REGEXP_PARTS {
                return Err(self.err("Too many elements in regexp"));
            }
            terms.push(self.parse_term()?);
        }
        if terms.len() == 1 {
            Ok(terms.remove(0))
        } else {
            Ok(RegExpNode::Alternative(terms))
        }
    }

    /// Parse a term: atom quantifier?
    fn parse_term(&mut self) -> Result<RegExpNode, RegExpError> {
        let atom = self.parse_atom()?;
        self.parse_quantifier(atom)
    }

    /// Parse an atom.
    fn parse_atom(&mut self) -> Result<RegExpNode, RegExpError> {
        let c = self
            .peek()
            .ok_or_else(|| self.err("Unexpected end of pattern"))?;

        match c {
            '.' => {
                self.advance();
                Ok(RegExpNode::Dot)
            }
            '^' => {
                self.advance();
                Ok(RegExpNode::Assertion(AssertionKind::Start))
            }
            '$' => {
                self.advance();
                Ok(RegExpNode::Assertion(AssertionKind::End))
            }
            '[' => self.parse_char_class(),
            '(' => self.parse_group(),
            '\\' => self.parse_escape(),
            // Quantifiers (pipe and closing paren are handled by callers)
            '*' | '+' | '?' => Err(self.err(format!("Nothing to repeat ('{c}')"))),
            // A10/B8: `{` — in unicode mode, lone `{` is an error;
            // in non-unicode mode, `{` not forming a valid quantifier is a literal
            '{' => {
                if self.is_unicode() {
                    Err(self.err("Nothing to repeat ('{')"))
                } else {
                    self.advance();
                    Ok(RegExpNode::Literal('{'))
                }
            }
            // R4: lone `}` and `]` are syntax errors in unicode mode
            '}' | ']' if self.is_unicode() => {
                Err(self.err(format!("Lone '{c}' not allowed in unicode mode")))
            }
            _ => {
                self.advance();
                Ok(RegExpNode::Literal(c))
            }
        }
    }

    /// Parse a quantifier suffix (if present).
    fn parse_quantifier(&mut self, atom: RegExpNode) -> Result<RegExpNode, RegExpError> {
        let (min, max) = match self.peek() {
            Some('*') => {
                self.advance();
                (0, None)
            }
            Some('+') => {
                self.advance();
                (1, None)
            }
            Some('?') => {
                self.advance();
                (0, Some(1))
            }
            Some('{') => {
                if let Some((lo, hi)) = self.try_parse_braced_quantifier()? {
                    (lo, hi)
                } else {
                    return Ok(atom);
                }
            }
            _ => return Ok(atom),
        };

        let greedy = if self.peek() == Some('?') {
            self.advance();
            false
        } else {
            true
        };

        // B25: {n,m} where n > m is an error
        if let Some(m) = max {
            if min > m {
                return Err(self.err(format!("Quantifier range out of order: {{{min},{m}}}")));
            }
        }

        // E5/§22.2.1: Only non-quantifiable assertions are rejected.
        // Lookahead (?=...) and (?!...) are QuantifiableAssertion — allowed.
        if let RegExpNode::Assertion(ref kind) = atom {
            let is_quantifiable = matches!(
                kind,
                AssertionKind::Lookahead(_) | AssertionKind::NegativeLookahead(_)
            );
            if !is_quantifiable {
                return Err(self.err("Cannot quantify assertion"));
            }
        }

        Ok(RegExpNode::Quantifier {
            body: Box::new(atom),
            min,
            max,
            greedy,
        })
    }

    fn try_parse_braced_quantifier(&mut self) -> Result<Option<(u32, Option<u32>)>, RegExpError> {
        let save = self.pos;
        self.advance(); // skip {
        let Some(min) = self.parse_digits()? else {
            self.pos = save; // A10: restore on failed parse
            return Ok(None);
        };
        let max = if self.eat(',') {
            if self.peek() == Some('}') {
                None
            } else {
                let Some(hi) = self.parse_digits()? else {
                    self.pos = save; // A10: restore on failed parse
                    return Ok(None);
                };
                Some(hi)
            }
        } else {
            Some(min)
        };
        if self.eat('}') {
            Ok(Some((min, max)))
        } else {
            self.pos = save;
            Ok(None)
        }
    }

    /// R16: Compute integer inline instead of building a String.
    fn parse_digits(&mut self) -> Result<Option<u32>, RegExpError> {
        let mut val: u32 = 0;
        let mut has_digit = false;
        while let Some(c) = self.peek() {
            if let Some(d) = c.to_digit(10) {
                has_digit = true;
                self.advance();
                val = val
                    .checked_mul(10)
                    .and_then(|v| v.checked_add(d))
                    .ok_or_else(|| self.err("Quantifier value too large"))?;
            } else {
                break;
            }
        }
        Ok(if has_digit { Some(val) } else { None })
    }

    /// Parse a character class `[...]`.
    pub(super) fn parse_char_class(&mut self) -> Result<RegExpNode, RegExpError> {
        self.depth += 1;
        if self.depth > MAX_REGEXP_DEPTH {
            self.depth -= 1;
            return Err(self.err("Character class nesting too deep"));
        }
        self.advance(); // skip [
        let negated = self.eat('^');

        // T3: v-flag uses ClassSetExpression with set operations
        if self.flags.unicode_sets {
            let result = self.parse_class_set_expression(negated);
            self.depth -= 1;
            return result;
        }

        let result = self.parse_char_class_inner(negated);
        self.depth -= 1;
        result
    }

    fn parse_char_class_inner(&mut self, negated: bool) -> Result<RegExpNode, RegExpError> {
        let mut ranges = Vec::new();

        loop {
            match self.peek() {
                None => {
                    return Err(self.err("Unterminated character class"));
                }
                Some(']') => {
                    self.advance();
                    break;
                }
                _ => {
                    if ranges.len() >= MAX_REGEXP_PARTS {
                        return Err(self.err("Too many elements in regexp"));
                    }
                    let atom = self.parse_class_atom()?;
                    if self.peek() == Some('-') && self.peek_next() != Some(']') {
                        self.advance(); // skip -
                        let end = self.parse_class_atom()?;
                        // A12: validate character class range
                        if let (Some(start_c), Some(end_c)) =
                            (class_atom_char(&atom), class_atom_char(&end))
                        {
                            if start_c > end_c {
                                return Err(self.err("Range out of order in character class"));
                            }
                        } else {
                            // A12: class escapes (\d, \w, etc.) cannot be range endpoints
                            return Err(self.err("Invalid range in character class"));
                        }
                        ranges.push(CharRange::Range(atom, end));
                    } else {
                        ranges.push(CharRange::Single(atom));
                    }
                }
            }
        }

        Ok(RegExpNode::CharClass { negated, ranges })
    }

    /// R8: Handle `\b` (backspace) and `\B` (error) inside character classes.
    /// Shared by `parse_class_atom` and `parse_v_class_atom`.
    pub(super) fn parse_class_escape_prefix(
        &mut self,
    ) -> Result<Option<CharClassAtom>, RegExpError> {
        if self.peek() == Some('b') {
            self.advance();
            return Ok(Some(CharClassAtom::Escape(EscapeKind::Identity('\x08'))));
        }
        if self.peek() == Some('B') {
            return Err(self.err("\\B is not valid inside a character class"));
        }
        Ok(None)
    }

    fn parse_class_atom(&mut self) -> Result<CharClassAtom, RegExpError> {
        match self.peek() {
            Some('\\') => {
                self.advance();
                if let Some(atom) = self.parse_class_escape_prefix()? {
                    return Ok(atom);
                }
                let esc = self.parse_escape_kind()?;
                Ok(CharClassAtom::Escape(esc))
            }
            Some(c) => {
                self.advance();
                Ok(CharClassAtom::Literal(c))
            }
            None => Err(self.err("Unexpected end in character class")),
        }
    }

    /// Parse a group `(...)`.
    fn parse_group(&mut self) -> Result<RegExpNode, RegExpError> {
        self.advance(); // skip (

        let kind = if self.eat('?') {
            match self.peek() {
                Some(':') => {
                    self.advance();
                    GroupKind::NonCapturing
                }
                Some('=') => {
                    return self.parse_lookaround(AssertionKind::Lookahead, "lookahead");
                }
                Some('!') => {
                    return self
                        .parse_lookaround(AssertionKind::NegativeLookahead, "negative lookahead");
                }
                Some('<') => {
                    self.advance();
                    match self.peek() {
                        Some('=') => {
                            return self.parse_lookaround(AssertionKind::Lookbehind, "lookbehind");
                        }
                        Some('!') => {
                            return self.parse_lookaround(
                                AssertionKind::NegativeLookbehind,
                                "negative lookbehind",
                            );
                        }
                        _ => {
                            // Named group (?<name>...)
                            let name = self.parse_group_name_definition()?;
                            self.group_count += 1;
                            GroupKind::Named(name)
                        }
                    }
                }
                _ => {
                    return Err(self.err("Invalid group specifier"));
                }
            }
        } else {
            self.group_count += 1;
            GroupKind::Capturing
        };

        let body = self.parse_disjunction()?;
        if !self.eat(')') {
            return Err(self.err("Unterminated group"));
        }

        Ok(RegExpNode::Group {
            kind,
            body: Box::new(body),
        })
    }

    /// R8: Shared parser for lookahead/lookbehind assertions.
    fn parse_lookaround(
        &mut self,
        make_kind: fn(Box<RegExpNode>) -> AssertionKind,
        label: &str,
    ) -> Result<RegExpNode, RegExpError> {
        self.advance();
        let body = self.parse_disjunction()?;
        if !self.eat(')') {
            return Err(self.err(format!("Unterminated {label}")));
        }
        Ok(RegExpNode::Assertion(make_kind(Box::new(body))))
    }

    /// Parse a group name (for definitions). Validates and tracks duplicates.
    fn parse_group_name_definition(&mut self) -> Result<String, RegExpError> {
        let name = self.parse_group_name_raw()?;
        // M6: §22.2.1 — first char must be IdentifierStartChar (ID_Start, $, _)
        if let Some(first) = name.chars().next() {
            if !is_id_start(first) {
                return Err(self.err(format!(
                    "Group name '{name}' must start with a letter, '$', or '_'"
                )));
            }
        }
        // A14: check for duplicate group names
        if !self.seen_group_names.insert(name.clone()) {
            return Err(self.err(format!("Duplicate group name '{name}'")));
        }
        Ok(name)
    }

    pub(super) fn parse_group_name_raw(&mut self) -> Result<String, RegExpError> {
        let mut name = String::new();
        loop {
            // S11: length limit to prevent OOM
            if name.len() >= 1024 {
                return Err(self.err("Group name exceeds maximum length"));
            }
            match self.peek() {
                Some('>') => {
                    self.advance();
                    if name.is_empty() {
                        return Err(self.err("Empty group name"));
                    }
                    return Ok(name);
                }
                // T4: Unicode escapes in group names per §22.2.1 RegExpIdentifierName
                Some('\\') => {
                    self.advance();
                    let c = self.parse_group_name_unicode_escape()?;
                    // First char must be ID_Start or '$' or '_'
                    if name.is_empty() && !is_id_start(c) {
                        return Err(self.err(format!(
                            "Invalid identifier start character U+{:04X} in group name",
                            c as u32
                        )));
                    }
                    name.push(c);
                }
                // M6: ASCII path — use exact ID_Start/ID_Continue instead of is_alphanumeric()
                Some(c) if c.is_ascii_alphanumeric() || c == '_' || c == '$' => {
                    self.advance();
                    name.push(c);
                }
                // L3: §12.7 — ZWJ/ZWNJ as IdentifierPartChar (continuation only)
                Some(c @ ('\u{200C}' | '\u{200D}')) if !name.is_empty() => {
                    self.advance();
                    name.push(c);
                }
                // T4: Non-ASCII Unicode ID_Start/ID_Continue chars
                Some(c) if !c.is_ascii() => {
                    if name.is_empty() {
                        if !is_id_start(c) {
                            return Err(self.err("Invalid group name"));
                        }
                    } else if !unicode_ident::is_xid_continue(c) {
                        return Err(self.err("Invalid group name"));
                    }
                    self.advance();
                    name.push(c);
                }
                _ => {
                    return Err(self.err("Invalid group name"));
                }
            }
        }
    }

    /// Parse `\uHHHH` or `\u{HHHH}` inside a group name, returning the decoded char.
    fn parse_group_name_unicode_escape(&mut self) -> Result<char, RegExpError> {
        match self.peek() {
            Some('u') => {
                self.advance();
                // R5: delegate to shared unicode escape parser
                let val = self
                    .parse_unicode_escape_value()
                    .ok_or_else(|| self.err("Invalid unicode escape in group name"))?;
                char::from_u32(val)
                    .ok_or_else(|| self.err("Invalid unicode code point in group name"))
            }
            _ => Err(self.err("Expected 'u' after '\\' in group name")),
        }
    }
}
