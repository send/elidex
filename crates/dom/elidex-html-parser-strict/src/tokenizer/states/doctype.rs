//! DOCTYPE tokenizer states (WHATWG HTML §13.2.5.53–68). The common path
//! `<!DOCTYPE html>` is valid HTML5; the public/system-identifier states
//! are spec-complete but only the `>`-terminated forms are reachable
//! without a parse error.

use super::{is_whitespace, State, Tokenizer};
use crate::StrictParseError;

impl Tokenizer {
    /// §13.2.5.53 DOCTYPE state.
    pub(super) fn doctype_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => self.switch_to(State::BeforeDoctypeName),
            Some('>') => self.reconsume_in(State::BeforeDoctypeName),
            None => return Err(self.parse_error("eof-in-doctype")),
            Some(_) => return Err(self.parse_error("missing-whitespace-before-doctype-name")),
        }
        Ok(())
    }

    /// §13.2.5.54 Before DOCTYPE name state.
    pub(super) fn before_doctype_name_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => {}
            Some(c) if c.is_ascii_uppercase() => {
                self.new_doctype();
                self.push_doctype_name(c.to_ascii_lowercase());
                self.switch_to(State::DoctypeName);
            }
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            Some('>') => return Err(self.parse_error("missing-doctype-name")),
            None => return Err(self.parse_error("eof-in-doctype")),
            Some(c) => {
                self.new_doctype();
                self.push_doctype_name(c);
                self.switch_to(State::DoctypeName);
            }
        }
        Ok(())
    }

    /// §13.2.5.55 DOCTYPE name state.
    pub(super) fn doctype_name_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => self.switch_to(State::AfterDoctypeName),
            Some('>') => {
                self.switch_to(State::Data);
                self.emit_doctype();
            }
            Some(c) if c.is_ascii_uppercase() => self.push_doctype_name(c.to_ascii_lowercase()),
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            None => return Err(self.parse_error("eof-in-doctype")),
            Some(c) => self.push_doctype_name(c),
        }
        Ok(())
    }

    /// §13.2.5.56 After DOCTYPE name state.
    pub(super) fn after_doctype_name_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => {}
            Some('>') => {
                self.switch_to(State::Data);
                self.emit_doctype();
            }
            None => return Err(self.parse_error("eof-in-doctype")),
            Some(_) => {
                // The six-character keyword match starts at the current
                // input character, so step back over the char we consumed.
                self.pos -= 1;
                if self.matches_ahead("PUBLIC", true) {
                    self.advance(6);
                    self.switch_to(State::AfterDoctypePublicKeyword);
                } else if self.matches_ahead("SYSTEM", true) {
                    self.advance(6);
                    self.switch_to(State::AfterDoctypeSystemKeyword);
                } else {
                    return Err(self.parse_error("invalid-character-sequence-after-doctype-name"));
                }
            }
        }
        Ok(())
    }

    /// §13.2.5.57 After DOCTYPE public keyword state.
    pub(super) fn after_doctype_public_keyword_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => self.switch_to(State::BeforeDoctypePublicIdentifier),
            Some('"' | '\'') => {
                return Err(self.parse_error("missing-whitespace-after-doctype-public-keyword"))
            }
            Some('>') => return Err(self.parse_error("missing-doctype-public-identifier")),
            None => return Err(self.parse_error("eof-in-doctype")),
            Some(_) => {
                return Err(self.parse_error("missing-quote-before-doctype-public-identifier"))
            }
        }
        Ok(())
    }

    /// §13.2.5.58 Before DOCTYPE public identifier state.
    pub(super) fn before_doctype_public_identifier_state(
        &mut self,
    ) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => {}
            Some('"') => {
                self.doctype.public_id = Some(String::new());
                self.switch_to(State::DoctypePublicIdentifierDoubleQuoted);
            }
            Some('\'') => {
                self.doctype.public_id = Some(String::new());
                self.switch_to(State::DoctypePublicIdentifierSingleQuoted);
            }
            Some('>') => return Err(self.parse_error("missing-doctype-public-identifier")),
            None => return Err(self.parse_error("eof-in-doctype")),
            Some(_) => {
                return Err(self.parse_error("missing-quote-before-doctype-public-identifier"))
            }
        }
        Ok(())
    }

    /// §13.2.5.59 DOCTYPE public identifier (double-quoted) state.
    pub(super) fn doctype_public_identifier_double_quoted_state(
        &mut self,
    ) -> Result<(), StrictParseError> {
        self.doctype_public_identifier_quoted('"')
    }

    /// §13.2.5.60 DOCTYPE public identifier (single-quoted) state.
    pub(super) fn doctype_public_identifier_single_quoted_state(
        &mut self,
    ) -> Result<(), StrictParseError> {
        self.doctype_public_identifier_quoted('\'')
    }

    /// §13.2.5.61 After DOCTYPE public identifier state.
    pub(super) fn after_doctype_public_identifier_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => {
                self.switch_to(State::BetweenDoctypePublicAndSystemIdentifiers);
            }
            Some('>') => {
                self.switch_to(State::Data);
                self.emit_doctype();
            }
            Some('"' | '\'') => {
                return Err(self.parse_error(
                    "missing-whitespace-between-doctype-public-and-system-identifiers",
                ))
            }
            None => return Err(self.parse_error("eof-in-doctype")),
            Some(_) => {
                return Err(self.parse_error("missing-quote-before-doctype-system-identifier"))
            }
        }
        Ok(())
    }

    /// §13.2.5.62 Between DOCTYPE public and system identifiers state.
    pub(super) fn between_doctype_public_and_system_identifiers_state(
        &mut self,
    ) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => {}
            Some('>') => {
                self.switch_to(State::Data);
                self.emit_doctype();
            }
            Some('"') => {
                self.doctype.system_id = Some(String::new());
                self.switch_to(State::DoctypeSystemIdentifierDoubleQuoted);
            }
            Some('\'') => {
                self.doctype.system_id = Some(String::new());
                self.switch_to(State::DoctypeSystemIdentifierSingleQuoted);
            }
            None => return Err(self.parse_error("eof-in-doctype")),
            Some(_) => {
                return Err(self.parse_error("missing-quote-before-doctype-system-identifier"))
            }
        }
        Ok(())
    }

    /// §13.2.5.63 After DOCTYPE system keyword state.
    pub(super) fn after_doctype_system_keyword_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => self.switch_to(State::BeforeDoctypeSystemIdentifier),
            Some('"' | '\'') => {
                return Err(self.parse_error("missing-whitespace-after-doctype-system-keyword"))
            }
            Some('>') => return Err(self.parse_error("missing-doctype-system-identifier")),
            None => return Err(self.parse_error("eof-in-doctype")),
            Some(_) => {
                return Err(self.parse_error("missing-quote-before-doctype-system-identifier"))
            }
        }
        Ok(())
    }

    /// §13.2.5.64 Before DOCTYPE system identifier state.
    pub(super) fn before_doctype_system_identifier_state(
        &mut self,
    ) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => {}
            Some('"') => {
                self.doctype.system_id = Some(String::new());
                self.switch_to(State::DoctypeSystemIdentifierDoubleQuoted);
            }
            Some('\'') => {
                self.doctype.system_id = Some(String::new());
                self.switch_to(State::DoctypeSystemIdentifierSingleQuoted);
            }
            Some('>') => return Err(self.parse_error("missing-doctype-system-identifier")),
            None => return Err(self.parse_error("eof-in-doctype")),
            Some(_) => {
                return Err(self.parse_error("missing-quote-before-doctype-system-identifier"))
            }
        }
        Ok(())
    }

    /// §13.2.5.65 DOCTYPE system identifier (double-quoted) state.
    pub(super) fn doctype_system_identifier_double_quoted_state(
        &mut self,
    ) -> Result<(), StrictParseError> {
        self.doctype_system_identifier_quoted('"')
    }

    /// §13.2.5.66 DOCTYPE system identifier (single-quoted) state.
    pub(super) fn doctype_system_identifier_single_quoted_state(
        &mut self,
    ) -> Result<(), StrictParseError> {
        self.doctype_system_identifier_quoted('\'')
    }

    /// §13.2.5.67 After DOCTYPE system identifier state.
    pub(super) fn after_doctype_system_identifier_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => {}
            Some('>') => {
                self.switch_to(State::Data);
                self.emit_doctype();
            }
            None => return Err(self.parse_error("eof-in-doctype")),
            Some(_) => {
                return Err(self.parse_error("unexpected-character-after-doctype-system-identifier"))
            }
        }
        Ok(())
    }

    /// §13.2.5.68 Bogus DOCTYPE state.
    ///
    /// Strict mode never enters this state (every transition to it is a
    /// rejected parse error). Implemented spec-faithfully for completeness:
    /// it consumes to the next `>` and emits the DOCTYPE token.
    pub(super) fn bogus_doctype_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('>') => {
                self.switch_to(State::Data);
                self.emit_doctype();
            }
            None => {
                self.emit_doctype();
                self.emit_eof();
            }
            // U+0000 (unexpected-null-character) and any other character
            // are both ignored per §13.2.5.68.
            Some(_) => {}
        }
        Ok(())
    }

    // ----- shared quoted-identifier bodies -----

    /// Shared body for the double/single-quoted public identifier states
    /// (§13.2.5.59/60), parameterized by the closing quote.
    fn doctype_public_identifier_quoted(&mut self, quote: char) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if c == quote => self.switch_to(State::AfterDoctypePublicIdentifier),
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            Some('>') => return Err(self.parse_error("abrupt-doctype-public-identifier")),
            None => return Err(self.parse_error("eof-in-doctype")),
            Some(c) => self.push_doctype_public_id(c),
        }
        Ok(())
    }

    /// Shared body for the double/single-quoted system identifier states
    /// (§13.2.5.65/66), parameterized by the closing quote.
    fn doctype_system_identifier_quoted(&mut self, quote: char) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if c == quote => self.switch_to(State::AfterDoctypeSystemIdentifier),
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            Some('>') => return Err(self.parse_error("abrupt-doctype-system-identifier")),
            None => return Err(self.parse_error("eof-in-doctype")),
            Some(c) => self.push_doctype_system_id(c),
        }
        Ok(())
    }
}
