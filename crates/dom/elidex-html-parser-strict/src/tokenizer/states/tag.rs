//! Tag-opening, tag-name, self-closing, and markup-declaration-open
//! states (WHATWG HTML §13.2.5.6–8 and §13.2.5.40, 42; the §13.2.5.41
//! bogus comment recovery state is omitted — strict rejects its entry).

use super::{is_whitespace, State, Tokenizer};
use crate::StrictParseError;

impl Tokenizer {
    /// §13.2.5.6 Tag open state.
    pub(super) fn tag_open_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('!') => self.switch_to(State::MarkupDeclarationOpen),
            Some('/') => self.switch_to(State::EndTagOpen),
            Some(c) if c.is_ascii_alphabetic() => {
                self.new_start_tag();
                self.reconsume_in(State::TagName);
            }
            Some('?') => {
                return Err(self.parse_error("unexpected-question-mark-instead-of-tag-name"))
            }
            None => return Err(self.parse_error("eof-before-tag-name")),
            Some(_) => return Err(self.parse_error("invalid-first-character-of-tag-name")),
        }
        Ok(())
    }

    /// §13.2.5.7 End tag open state.
    pub(super) fn end_tag_open_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if c.is_ascii_alphabetic() => {
                self.new_end_tag();
                self.reconsume_in(State::TagName);
            }
            Some('>') => return Err(self.parse_error("missing-end-tag-name")),
            None => return Err(self.parse_error("eof-before-tag-name")),
            Some(_) => return Err(self.parse_error("invalid-first-character-of-tag-name")),
        }
        Ok(())
    }

    /// §13.2.5.8 Tag name state.
    pub(super) fn tag_name_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => self.switch_to(State::BeforeAttributeName),
            Some('/') => self.switch_to(State::SelfClosingStartTag),
            Some('>') => {
                self.switch_to(State::Data);
                self.emit_current_tag()?;
            }
            Some(c) if c.is_ascii_uppercase() => self.push_tag_name(c.to_ascii_lowercase()),
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            None => return Err(self.parse_error("eof-in-tag")),
            Some(c) => self.push_tag_name(c),
        }
        Ok(())
    }

    /// §13.2.5.40 Self-closing start tag state.
    pub(super) fn self_closing_start_tag_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('>') => {
                self.set_self_closing();
                self.switch_to(State::Data);
                self.emit_current_tag()?;
            }
            None => return Err(self.parse_error("eof-in-tag")),
            Some(_) => return Err(self.parse_error("unexpected-solidus-in-tag")),
        }
        Ok(())
    }

    /// §13.2.5.42 Markup declaration open state.
    ///
    /// Recognizes `<!--` (comment), `<!DOCTYPE` (ASCII-case-insensitive),
    /// and `<![CDATA[`. The CDATA opener is valid only in foreign content:
    /// when the tree builder's foreign-content flag is set (the adjusted
    /// current node is in a non-HTML namespace, §13.2.6 dispatcher) it opens
    /// a CDATA section; in HTML content it is the `cdata-in-html-content`
    /// parse error. Any other sequence is `incorrectly-opened-comment` —
    /// both error sequences are rejected.
    pub(super) fn markup_declaration_open_state(&mut self) -> Result<(), StrictParseError> {
        if self.matches_ahead("--", false) {
            self.advance(2);
            self.new_comment();
            self.switch_to(State::CommentStart);
        } else if self.matches_ahead("DOCTYPE", true) {
            self.advance(7);
            self.switch_to(State::Doctype);
        } else if self.matches_ahead("[CDATA[", false) {
            if self.foreign_content {
                self.advance(7);
                self.switch_to(State::CdataSection);
            } else {
                // §13.2.5.42: outside foreign content this is a parse error.
                return Err(self.parse_error("cdata-in-html-content"));
            }
        } else {
            return Err(self.parse_error("incorrectly-opened-comment"));
        }
        Ok(())
    }
}
