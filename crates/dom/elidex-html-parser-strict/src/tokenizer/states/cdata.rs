//! CDATA section tokenizer states (WHATWG HTML §13.2.5.69–71).
//!
//! These states are only entered from the markup-declaration-open state
//! when the tree builder's foreign-content flag is set — i.e. the adjusted
//! current node is a non-HTML element (§13.2.6 dispatcher). In HTML content
//! the markup-declaration-open state instead rejects `<![CDATA[` as the
//! `cdata-in-html-content` parse error, so these handlers run only inside
//! inline SVG / MathML. The section's characters are emitted as character
//! tokens (a CDATA section produces a Text node, not a CDATASection node).

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
