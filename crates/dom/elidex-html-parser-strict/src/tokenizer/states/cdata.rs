//! CDATA section tokenizer states (WHATWG HTML §13.2.5.69–71).
//!
//! These states are only entered from the markup-declaration-open state
//! when the adjusted current node is a non-HTML element (foreign content).
//! The strict tokenizer has no foreign-content support until A5
//! (`#11-html-parser-strict-foreign-content`), so the markup-declaration-
//! open state rejects `<![CDATA[` in HTML content and these handlers are
//! unreachable in A2. They are implemented spec-faithfully so that A5 can
//! wire them in without revisiting the tokenizer core.

use super::{State, Tokenizer};
use crate::StrictParseError;

impl Tokenizer {
    /// §13.2.5.69 CDATA section state.
    pub(super) fn cdata_section_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(']') => self.switch_to(State::CdataSectionBracket),
            None => return Err(self.parse_error("eof-in-cdata")),
            Some(c) => self.emit_char(c),
        }
        Ok(())
    }

    /// §13.2.5.70 CDATA section bracket state.
    pub(super) fn cdata_section_bracket_state(&mut self) -> Result<(), StrictParseError> {
        if self.consume() == Some(']') {
            self.switch_to(State::CdataSectionEnd);
        } else {
            self.emit_char(']');
            self.reconsume_in(State::CdataSection);
        }
        Ok(())
    }

    /// §13.2.5.71 CDATA section end state.
    pub(super) fn cdata_section_end_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(']') => self.emit_char(']'),
            Some('>') => self.switch_to(State::Data),
            _ => {
                self.emit_char(']');
                self.emit_char(']');
                self.reconsume_in(State::CdataSection);
            }
        }
        Ok(())
    }
}
