//! V-flag (`unicodeSets`) class set expression parsing (ES2024 §22.2.2.4).
//!
//! Handles `ClassSetExpression` with set operations (`&&`, `--`),
//! nested character classes, and `\q{...}` string alternatives.

use super::{CharClassAtom, CharRange, ClassSetOp, RegExpError, RegExpNode};

use super::escape::class_atom_char;
use super::parser::{RegExpParser, MAX_REGEXP_PARTS};

impl RegExpParser<'_> {
    /// R10: Flush accumulated ranges as an operand and set (or validate) the set operator.
    fn flush_set_operator(
        &mut self,
        set_op: &mut Option<ClassSetOp>,
        operands: &mut Vec<RegExpNode>,
        ranges: &mut Vec<CharRange>,
        new_op: ClassSetOp,
    ) -> Result<(), RegExpError> {
        operands.push(RegExpNode::CharClass {
            negated: false,
            ranges: std::mem::take(ranges),
        });
        self.advance();
        self.advance();
        if let Some(existing) = set_op {
            if *existing != new_op {
                return Err(self.err("Cannot mix '&&' and '--' in the same character class"));
            }
        }
        *set_op = Some(new_op);
        Ok(())
    }

    /// T3: Parse a `ClassSetExpression` (v flag).
    /// Handles nested classes `[[a-z]]`, set operations `[A&&B]` / `[A--B]`, and `\q{...}`.
    pub(super) fn parse_class_set_expression(
        &mut self,
        negated: bool,
    ) -> Result<RegExpNode, RegExpError> {
        let mut ranges = Vec::new();
        let mut set_op: Option<ClassSetOp> = None;
        let mut operands: Vec<RegExpNode> = Vec::new();

        loop {
            match self.peek() {
                None => return Err(self.err("Unterminated character class")),
                Some(']') => {
                    self.advance();
                    break;
                }
                // Nested character class
                Some('[') => {
                    let nested = self.parse_char_class()?;
                    ranges.push(CharRange::Single(CharClassAtom::NestedClass(Box::new(
                        nested,
                    ))));
                }
                // Check for set operators `&&` or `--`
                Some('&') if self.peek_next() == Some('&') => {
                    self.flush_set_operator(
                        &mut set_op,
                        &mut operands,
                        &mut ranges,
                        ClassSetOp::Intersection,
                    )?;
                }
                Some('-') if self.peek_next() == Some('-') => {
                    self.flush_set_operator(
                        &mut set_op,
                        &mut operands,
                        &mut ranges,
                        ClassSetOp::Subtraction,
                    )?;
                }
                _ => {
                    let atom = self.parse_v_class_atom()?;
                    // Range check: `a-z` style
                    if self.peek() == Some('-')
                        && self.peek_next() != Some(']')
                        && self.peek_next() != Some('-')
                    {
                        self.advance(); // skip -
                        let end = self.parse_v_class_atom()?;
                        if let (Some(start_c), Some(end_c)) =
                            (class_atom_char(&atom), class_atom_char(&end))
                        {
                            if start_c > end_c {
                                return Err(self.err("Range out of order in character class"));
                            }
                        } else {
                            return Err(self.err("Invalid range in character class"));
                        }
                        ranges.push(CharRange::Range(atom, end));
                    } else {
                        ranges.push(CharRange::Single(atom));
                    }
                    if ranges.len() >= MAX_REGEXP_PARTS {
                        return Err(self.err("Too many elements in regexp"));
                    }
                }
            }
        }

        if let Some(op) = set_op {
            // Flush remaining ranges as last operand
            if !ranges.is_empty() {
                operands.push(RegExpNode::CharClass {
                    negated: false,
                    ranges,
                });
            }
            let mut result = RegExpNode::ClassSetExpression { op, operands };
            if negated {
                // Wrap in a negated CharClass containing the set expression
                result = RegExpNode::CharClass {
                    negated: true,
                    ranges: vec![CharRange::Single(CharClassAtom::NestedClass(Box::new(
                        result,
                    )))],
                };
            }
            Ok(result)
        } else {
            Ok(RegExpNode::CharClass { negated, ranges })
        }
    }

    /// Parse a class atom in v-flag mode (supports `\q{...}` and nested classes).
    fn parse_v_class_atom(&mut self) -> Result<CharClassAtom, RegExpError> {
        match self.peek() {
            Some('\\') => {
                self.advance();
                if let Some(atom) = self.parse_class_escape_prefix()? {
                    return Ok(atom);
                }
                // T3: \q{...} string alternative
                if self.peek() == Some('q') {
                    self.advance();
                    if self.peek() != Some('{') {
                        return Err(self.err("Expected '{' after '\\q'"));
                    }
                    self.advance(); // skip {
                    let alternatives = self.parse_string_alternative()?;
                    return Ok(CharClassAtom::StringAlternative(alternatives));
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

    /// Parse `\q{abc|def}` — string alternatives separated by `|`, terminated by `}`.
    fn parse_string_alternative(&mut self) -> Result<Vec<Vec<char>>, RegExpError> {
        let mut alternatives = Vec::new();
        let mut current = Vec::new();
        loop {
            match self.peek() {
                None => return Err(self.err("Unterminated \\q{...} string alternative")),
                Some('}') => {
                    self.advance();
                    alternatives.push(current);
                    return Ok(alternatives);
                }
                Some('|') => {
                    self.advance();
                    if alternatives.len() >= MAX_REGEXP_PARTS {
                        return Err(self.err("Too many elements in regexp"));
                    }
                    alternatives.push(std::mem::take(&mut current));
                }
                Some('\\') => {
                    self.advance();
                    let esc = self.parse_escape_kind()?;
                    if let Some(c) = esc.to_char() {
                        current.push(c);
                    } else {
                        return Err(self.err("Invalid escape in \\q{...} string alternative"));
                    }
                }
                Some(c) => {
                    self.advance();
                    current.push(c);
                    if current.len() >= MAX_REGEXP_PARTS {
                        return Err(self.err("Too many elements in regexp"));
                    }
                }
            }
        }
    }
}
