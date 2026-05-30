//! Attribute name/value tokenizer states (WHATWG HTML §13.2.5.32–39).

use super::{is_whitespace, State, Tokenizer};
use crate::StrictParseError;

impl Tokenizer {
    /// §13.2.5.32 Before attribute name state.
    pub(super) fn before_attribute_name_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => {}
            Some('/' | '>') | None => self.reconsume_in(State::AfterAttributeName),
            Some('=') => {
                return Err(self.parse_error("unexpected-equals-sign-before-attribute-name"))
            }
            Some(_) => {
                self.start_attribute()?;
                self.reconsume_in(State::AttributeName);
            }
        }
        Ok(())
    }

    /// §13.2.5.33 Attribute name state.
    pub(super) fn attribute_name_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => self.reconsume_in(State::AfterAttributeName),
            Some('/' | '>') | None => self.reconsume_in(State::AfterAttributeName),
            Some('=') => self.switch_to(State::BeforeAttributeValue),
            Some(c) if c.is_ascii_uppercase() => self.push_attr_name(c.to_ascii_lowercase()),
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            Some('"' | '\'' | '<') => {
                return Err(self.parse_error("unexpected-character-in-attribute-name"))
            }
            Some(c) => self.push_attr_name(c),
        }
        Ok(())
    }

    /// §13.2.5.34 After attribute name state.
    pub(super) fn after_attribute_name_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => {}
            Some('/') => self.switch_to(State::SelfClosingStartTag),
            Some('=') => self.switch_to(State::BeforeAttributeValue),
            Some('>') => {
                self.switch_to(State::Data);
                self.emit_current_tag()?;
            }
            None => return Err(self.parse_error("eof-in-tag")),
            Some(_) => {
                self.start_attribute()?;
                self.reconsume_in(State::AttributeName);
            }
        }
        Ok(())
    }

    /// §13.2.5.35 Before attribute value state.
    pub(super) fn before_attribute_value_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => {}
            Some('"') => self.switch_to(State::AttributeValueDoubleQuoted),
            Some('\'') => self.switch_to(State::AttributeValueSingleQuoted),
            Some('>') => return Err(self.parse_error("missing-attribute-value")),
            _ => self.reconsume_in(State::AttributeValueUnquoted),
        }
        Ok(())
    }

    /// §13.2.5.36 Attribute value (double-quoted) state.
    pub(super) fn attribute_value_double_quoted_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('"') => self.switch_to(State::AfterAttributeValueQuoted),
            Some('&') => {
                self.set_return_state(State::AttributeValueDoubleQuoted);
                self.switch_to(State::CharacterReference);
            }
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            None => return Err(self.parse_error("eof-in-tag")),
            Some(c) => self.push_attr_value(c),
        }
        Ok(())
    }

    /// §13.2.5.37 Attribute value (single-quoted) state.
    pub(super) fn attribute_value_single_quoted_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('\'') => self.switch_to(State::AfterAttributeValueQuoted),
            Some('&') => {
                self.set_return_state(State::AttributeValueSingleQuoted);
                self.switch_to(State::CharacterReference);
            }
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            None => return Err(self.parse_error("eof-in-tag")),
            Some(c) => self.push_attr_value(c),
        }
        Ok(())
    }

    /// §13.2.5.38 Attribute value (unquoted) state.
    pub(super) fn attribute_value_unquoted_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => self.switch_to(State::BeforeAttributeName),
            Some('&') => {
                self.set_return_state(State::AttributeValueUnquoted);
                self.switch_to(State::CharacterReference);
            }
            Some('>') => {
                self.switch_to(State::Data);
                self.emit_current_tag()?;
            }
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            Some('"' | '\'' | '<' | '=' | '`') => {
                return Err(self.parse_error("unexpected-character-in-unquoted-attribute-value"))
            }
            None => return Err(self.parse_error("eof-in-tag")),
            Some(c) => self.push_attr_value(c),
        }
        Ok(())
    }

    /// §13.2.5.39 After attribute value (quoted) state.
    pub(super) fn after_attribute_value_quoted_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => self.switch_to(State::BeforeAttributeName),
            Some('/') => self.switch_to(State::SelfClosingStartTag),
            Some('>') => {
                self.switch_to(State::Data);
                self.emit_current_tag()?;
            }
            None => return Err(self.parse_error("eof-in-tag")),
            Some(_) => return Err(self.parse_error("missing-whitespace-between-attributes")),
        }
        Ok(())
    }
}
