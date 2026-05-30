//! Text-content tokenizer states: Data, RCDATA, RAWTEXT, script data,
//! PLAINTEXT, and their less-than-sign / end-tag / script-escape families
//! (WHATWG HTML §13.2.5.1–5 and §13.2.5.9–31).

use super::{is_whitespace, State, Tokenizer};
use crate::StrictParseError;

impl Tokenizer {
    /// §13.2.5.1 Data state.
    pub(super) fn data_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('&') => {
                self.set_return_state(State::Data);
                self.switch_to(State::CharacterReference);
            }
            Some('<') => self.switch_to(State::TagOpen),
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            None => self.emit_eof(),
            Some(c) => self.emit_char(c),
        }
        Ok(())
    }

    /// §13.2.5.2 RCDATA state.
    pub(super) fn rcdata_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('&') => {
                self.set_return_state(State::Rcdata);
                self.switch_to(State::CharacterReference);
            }
            Some('<') => self.switch_to(State::RcdataLessThanSign),
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            None => self.emit_eof(),
            Some(c) => self.emit_char(c),
        }
        Ok(())
    }

    /// §13.2.5.3 RAWTEXT state.
    pub(super) fn rawtext_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('<') => self.switch_to(State::RawtextLessThanSign),
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            None => self.emit_eof(),
            Some(c) => self.emit_char(c),
        }
        Ok(())
    }

    /// §13.2.5.4 Script data state.
    pub(super) fn script_data_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('<') => self.switch_to(State::ScriptDataLessThanSign),
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            None => self.emit_eof(),
            Some(c) => self.emit_char(c),
        }
        Ok(())
    }

    /// §13.2.5.5 PLAINTEXT state.
    pub(super) fn plaintext_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            None => self.emit_eof(),
            Some(c) => self.emit_char(c),
        }
        Ok(())
    }

    /// §13.2.5.9 RCDATA less-than sign state.
    pub(super) fn rcdata_less_than_sign_state(&mut self) -> Result<(), StrictParseError> {
        self.less_than_sign_generic(State::RcdataEndTagOpen, State::Rcdata)
    }

    /// §13.2.5.10 RCDATA end tag open state.
    pub(super) fn rcdata_end_tag_open_state(&mut self) -> Result<(), StrictParseError> {
        self.end_tag_open_generic(State::RcdataEndTagName, State::Rcdata)
    }

    /// §13.2.5.11 RCDATA end tag name state.
    pub(super) fn rcdata_end_tag_name_state(&mut self) -> Result<(), StrictParseError> {
        self.end_tag_name_generic(State::Rcdata)
    }

    /// §13.2.5.12 RAWTEXT less-than sign state.
    pub(super) fn rawtext_less_than_sign_state(&mut self) -> Result<(), StrictParseError> {
        self.less_than_sign_generic(State::RawtextEndTagOpen, State::Rawtext)
    }

    /// §13.2.5.13 RAWTEXT end tag open state.
    pub(super) fn rawtext_end_tag_open_state(&mut self) -> Result<(), StrictParseError> {
        self.end_tag_open_generic(State::RawtextEndTagName, State::Rawtext)
    }

    /// §13.2.5.14 RAWTEXT end tag name state.
    pub(super) fn rawtext_end_tag_name_state(&mut self) -> Result<(), StrictParseError> {
        self.end_tag_name_generic(State::Rawtext)
    }

    /// §13.2.5.15 Script data less-than sign state.
    pub(super) fn script_data_less_than_sign_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('/') => {
                self.clear_temp_buffer();
                self.switch_to(State::ScriptDataEndTagOpen);
            }
            Some('!') => {
                self.switch_to(State::ScriptDataEscapeStart);
                self.emit_char('<');
                self.emit_char('!');
            }
            _ => {
                self.emit_char('<');
                self.reconsume_in(State::ScriptData);
            }
        }
        Ok(())
    }

    /// §13.2.5.16 Script data end tag open state.
    pub(super) fn script_data_end_tag_open_state(&mut self) -> Result<(), StrictParseError> {
        self.end_tag_open_generic(State::ScriptDataEndTagName, State::ScriptData)
    }

    /// §13.2.5.17 Script data end tag name state.
    pub(super) fn script_data_end_tag_name_state(&mut self) -> Result<(), StrictParseError> {
        self.end_tag_name_generic(State::ScriptData)
    }

    /// §13.2.5.18 Script data escape start state.
    pub(super) fn script_data_escape_start_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('-') => {
                self.switch_to(State::ScriptDataEscapeStartDash);
                self.emit_char('-');
            }
            _ => self.reconsume_in(State::ScriptData),
        }
        Ok(())
    }

    /// §13.2.5.19 Script data escape start dash state.
    pub(super) fn script_data_escape_start_dash_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('-') => {
                self.switch_to(State::ScriptDataEscapedDashDash);
                self.emit_char('-');
            }
            _ => self.reconsume_in(State::ScriptData),
        }
        Ok(())
    }

    /// §13.2.5.20 Script data escaped state.
    pub(super) fn script_data_escaped_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('-') => {
                self.switch_to(State::ScriptDataEscapedDash);
                self.emit_char('-');
            }
            Some('<') => self.switch_to(State::ScriptDataEscapedLessThanSign),
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            None => return Err(self.parse_error("eof-in-script-html-comment-like-text")),
            Some(c) => self.emit_char(c),
        }
        Ok(())
    }

    /// §13.2.5.21 Script data escaped dash state.
    pub(super) fn script_data_escaped_dash_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('-') => {
                self.switch_to(State::ScriptDataEscapedDashDash);
                self.emit_char('-');
            }
            Some('<') => self.switch_to(State::ScriptDataEscapedLessThanSign),
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            None => return Err(self.parse_error("eof-in-script-html-comment-like-text")),
            Some(c) => {
                self.switch_to(State::ScriptDataEscaped);
                self.emit_char(c);
            }
        }
        Ok(())
    }

    /// §13.2.5.22 Script data escaped dash dash state.
    pub(super) fn script_data_escaped_dash_dash_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('-') => self.emit_char('-'),
            Some('<') => self.switch_to(State::ScriptDataEscapedLessThanSign),
            Some('>') => {
                self.switch_to(State::ScriptData);
                self.emit_char('>');
            }
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            None => return Err(self.parse_error("eof-in-script-html-comment-like-text")),
            Some(c) => {
                self.switch_to(State::ScriptDataEscaped);
                self.emit_char(c);
            }
        }
        Ok(())
    }

    /// §13.2.5.23 Script data escaped less-than sign state.
    pub(super) fn script_data_escaped_less_than_sign_state(
        &mut self,
    ) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('/') => {
                self.clear_temp_buffer();
                self.switch_to(State::ScriptDataEscapedEndTagOpen);
            }
            Some(c) if c.is_ascii_alphabetic() => {
                self.clear_temp_buffer();
                self.emit_char('<');
                self.reconsume_in(State::ScriptDataDoubleEscapeStart);
            }
            _ => {
                self.emit_char('<');
                self.reconsume_in(State::ScriptDataEscaped);
            }
        }
        Ok(())
    }

    /// §13.2.5.24 Script data escaped end tag open state.
    pub(super) fn script_data_escaped_end_tag_open_state(
        &mut self,
    ) -> Result<(), StrictParseError> {
        self.end_tag_open_generic(State::ScriptDataEscapedEndTagName, State::ScriptDataEscaped)
    }

    /// §13.2.5.25 Script data escaped end tag name state.
    pub(super) fn script_data_escaped_end_tag_name_state(
        &mut self,
    ) -> Result<(), StrictParseError> {
        self.end_tag_name_generic(State::ScriptDataEscaped)
    }

    /// §13.2.5.26 Script data double escape start state.
    pub(super) fn script_data_double_escape_start_state(&mut self) -> Result<(), StrictParseError> {
        // "script" → double-escaped, otherwise back to escaped.
        self.script_data_double_escape_generic(
            State::ScriptDataDoubleEscaped,
            State::ScriptDataEscaped,
            State::ScriptDataEscaped,
        )
    }

    /// §13.2.5.27 Script data double escaped state.
    pub(super) fn script_data_double_escaped_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('-') => {
                self.switch_to(State::ScriptDataDoubleEscapedDash);
                self.emit_char('-');
            }
            Some('<') => {
                self.switch_to(State::ScriptDataDoubleEscapedLessThanSign);
                self.emit_char('<');
            }
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            None => return Err(self.parse_error("eof-in-script-html-comment-like-text")),
            Some(c) => self.emit_char(c),
        }
        Ok(())
    }

    /// §13.2.5.28 Script data double escaped dash state.
    pub(super) fn script_data_double_escaped_dash_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('-') => {
                self.switch_to(State::ScriptDataDoubleEscapedDashDash);
                self.emit_char('-');
            }
            Some('<') => {
                self.switch_to(State::ScriptDataDoubleEscapedLessThanSign);
                self.emit_char('<');
            }
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            None => return Err(self.parse_error("eof-in-script-html-comment-like-text")),
            Some(c) => {
                self.switch_to(State::ScriptDataDoubleEscaped);
                self.emit_char(c);
            }
        }
        Ok(())
    }

    /// §13.2.5.29 Script data double escaped dash dash state.
    pub(super) fn script_data_double_escaped_dash_dash_state(
        &mut self,
    ) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('-') => self.emit_char('-'),
            Some('<') => {
                self.switch_to(State::ScriptDataDoubleEscapedLessThanSign);
                self.emit_char('<');
            }
            Some('>') => {
                self.switch_to(State::ScriptData);
                self.emit_char('>');
            }
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            None => return Err(self.parse_error("eof-in-script-html-comment-like-text")),
            Some(c) => {
                self.switch_to(State::ScriptDataDoubleEscaped);
                self.emit_char(c);
            }
        }
        Ok(())
    }

    /// §13.2.5.30 Script data double escaped less-than sign state.
    pub(super) fn script_data_double_escaped_less_than_sign_state(
        &mut self,
    ) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('/') => {
                self.clear_temp_buffer();
                self.switch_to(State::ScriptDataDoubleEscapeEnd);
                self.emit_char('/');
            }
            _ => self.reconsume_in(State::ScriptDataDoubleEscaped),
        }
        Ok(())
    }

    /// §13.2.5.31 Script data double escape end state.
    pub(super) fn script_data_double_escape_end_state(&mut self) -> Result<(), StrictParseError> {
        // "script" → back to escaped, otherwise stay double-escaped.
        self.script_data_double_escape_generic(
            State::ScriptDataEscaped,
            State::ScriptDataDoubleEscaped,
            State::ScriptDataDoubleEscaped,
        )
    }

    // ----- shared less-than-sign / end-tag-open machinery -----

    /// Shared body for the RCDATA/RAWTEXT less-than-sign states
    /// (§13.2.5.9/12): `/` clears the temp buffer and enters
    /// `end_tag_open_state`, anything else emits `<` and reconsumes in
    /// the text-content state. (The script-data variants differ — they
    /// have extra `!` / double-escape branches — so they are not shared.)
    fn less_than_sign_generic(
        &mut self,
        end_tag_open_state: State,
        raw_state: State,
    ) -> Result<(), StrictParseError> {
        if self.consume() == Some('/') {
            self.clear_temp_buffer();
            self.switch_to(end_tag_open_state);
        } else {
            self.emit_char('<');
            self.reconsume_in(raw_state);
        }
        Ok(())
    }

    /// Shared body for the RCDATA/RAWTEXT/script-data(-escaped) end-tag-open
    /// states (§13.2.5.10/13/16/24): ASCII alpha starts a new end tag and
    /// reconsumes in `name_state`, anything else emits `</` and reconsumes
    /// in the text-content state.
    fn end_tag_open_generic(
        &mut self,
        name_state: State,
        raw_state: State,
    ) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if c.is_ascii_alphabetic() => {
                self.new_end_tag();
                self.reconsume_in(name_state);
            }
            _ => {
                self.emit_char('<');
                self.emit_char('/');
                self.reconsume_in(raw_state);
            }
        }
        Ok(())
    }

    /// Shared body for the script-data double-escape start/end states
    /// (§13.2.5.26/31): on whitespace/`/`/`>`, switch to `matched` if the
    /// temp buffer is `"script"` else `unmatched`, emitting the char;
    /// ASCII letters accumulate (lowercased) into the temp buffer and are
    /// emitted; anything else reconsumes in `reconsume_state`.
    fn script_data_double_escape_generic(
        &mut self,
        matched: State,
        unmatched: State,
        reconsume_state: State,
    ) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) || c == '/' || c == '>' => {
                if self.temp_buffer_is("script") {
                    self.switch_to(matched);
                } else {
                    self.switch_to(unmatched);
                }
                self.emit_char(c);
            }
            Some(c) if c.is_ascii_uppercase() => {
                self.push_temp_buffer(c.to_ascii_lowercase());
                self.emit_char(c);
            }
            Some(c) if c.is_ascii_lowercase() => {
                self.push_temp_buffer(c);
                self.emit_char(c);
            }
            _ => self.reconsume_in(reconsume_state),
        }
        Ok(())
    }

    // ----- shared end-tag-name machinery (§13.2.5.11/14/17/25) -----

    /// Shared body for the RCDATA / RAWTEXT / script-data(-escaped) end tag
    /// name states. `raw_state` is the text-content state to reconsume in
    /// when the end tag is not "appropriate".
    fn end_tag_name_generic(&mut self, raw_state: State) -> Result<(), StrictParseError> {
        match self.consume() {
            Some(c) if is_whitespace(c) => {
                if self.is_appropriate_end_tag() {
                    self.switch_to(State::BeforeAttributeName);
                } else {
                    self.end_tag_anything_else(raw_state);
                }
            }
            Some('/') => {
                if self.is_appropriate_end_tag() {
                    self.switch_to(State::SelfClosingStartTag);
                } else {
                    self.end_tag_anything_else(raw_state);
                }
            }
            Some('>') => {
                if self.is_appropriate_end_tag() {
                    self.switch_to(State::Data);
                    self.emit_current_tag()?;
                } else {
                    self.end_tag_anything_else(raw_state);
                }
            }
            Some(c) if c.is_ascii_uppercase() => {
                self.push_tag_name(c.to_ascii_lowercase());
                self.push_temp_buffer(c);
            }
            Some(c) if c.is_ascii_lowercase() => {
                self.push_tag_name(c);
                self.push_temp_buffer(c);
            }
            _ => self.end_tag_anything_else(raw_state),
        }
        Ok(())
    }

    /// The "anything else" branch shared by the end-tag-name states: emit
    /// `<`, `/`, and the temp buffer as character tokens, then reconsume
    /// the current input character in the text-content state.
    fn end_tag_anything_else(&mut self, raw_state: State) {
        self.emit_char('<');
        self.emit_char('/');
        let buf = self.take_temp_buffer();
        for c in buf.chars() {
            self.emit_char(c);
        }
        self.reconsume_in(raw_state);
    }
}
