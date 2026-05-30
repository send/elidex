//! Comment tokenizer states (WHATWG HTML §13.2.5.43–52). Reachable in
//! valid HTML5 via `<!-- … -->` (markup-declaration-open `--` branch).

use super::{State, Tokenizer};
use crate::StrictParseError;

impl Tokenizer {
    /// §13.2.5.43 Comment start state.
    pub(super) fn comment_start_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('-') => self.switch_to(State::CommentStartDash),
            Some('>') => return Err(self.parse_error("abrupt-closing-of-empty-comment")),
            _ => self.reconsume_in(State::Comment),
        }
        Ok(())
    }

    /// §13.2.5.44 Comment start dash state.
    pub(super) fn comment_start_dash_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('-') => self.switch_to(State::CommentEnd),
            Some('>') => return Err(self.parse_error("abrupt-closing-of-empty-comment")),
            None => return Err(self.parse_error("eof-in-comment")),
            _ => {
                self.push_comment('-');
                self.reconsume_in(State::Comment);
            }
        }
        Ok(())
    }

    /// §13.2.5.45 Comment state.
    pub(super) fn comment_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('<') => {
                self.push_comment('<');
                self.switch_to(State::CommentLessThanSign);
            }
            Some('-') => self.switch_to(State::CommentEndDash),
            Some('\u{0000}') => return Err(self.parse_error("unexpected-null-character")),
            None => return Err(self.parse_error("eof-in-comment")),
            Some(c) => self.push_comment(c),
        }
        Ok(())
    }

    /// §13.2.5.46 Comment less-than sign state.
    pub(super) fn comment_less_than_sign_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('!') => {
                self.push_comment('!');
                self.switch_to(State::CommentLessThanSignBang);
            }
            Some('<') => self.push_comment('<'),
            _ => self.reconsume_in(State::Comment),
        }
        Ok(())
    }

    /// §13.2.5.47 Comment less-than sign bang state.
    pub(super) fn comment_less_than_sign_bang_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('-') => self.switch_to(State::CommentLessThanSignBangDash),
            _ => self.reconsume_in(State::Comment),
        }
        Ok(())
    }

    /// §13.2.5.48 Comment less-than sign bang dash state.
    pub(super) fn comment_less_than_sign_bang_dash_state(
        &mut self,
    ) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('-') => self.switch_to(State::CommentLessThanSignBangDashDash),
            _ => self.reconsume_in(State::CommentEndDash),
        }
        Ok(())
    }

    /// §13.2.5.49 Comment less-than sign bang dash dash state.
    pub(super) fn comment_less_than_sign_bang_dash_dash_state(
        &mut self,
    ) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('>') | None => self.reconsume_in(State::CommentEnd),
            _ => return Err(self.parse_error("nested-comment")),
        }
        Ok(())
    }

    /// §13.2.5.50 Comment end dash state.
    pub(super) fn comment_end_dash_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('-') => self.switch_to(State::CommentEnd),
            None => return Err(self.parse_error("eof-in-comment")),
            _ => {
                self.push_comment('-');
                self.reconsume_in(State::Comment);
            }
        }
        Ok(())
    }

    /// §13.2.5.51 Comment end state.
    pub(super) fn comment_end_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('>') => {
                self.switch_to(State::Data);
                self.emit_comment();
            }
            Some('!') => self.switch_to(State::CommentEndBang),
            Some('-') => self.push_comment('-'),
            None => return Err(self.parse_error("eof-in-comment")),
            _ => {
                self.push_comment_str("--");
                self.reconsume_in(State::Comment);
            }
        }
        Ok(())
    }

    /// §13.2.5.52 Comment end bang state.
    pub(super) fn comment_end_bang_state(&mut self) -> Result<(), StrictParseError> {
        match self.consume() {
            Some('-') => {
                self.push_comment_str("--!");
                self.switch_to(State::CommentEndDash);
            }
            Some('>') => return Err(self.parse_error("incorrectly-closed-comment")),
            None => return Err(self.parse_error("eof-in-comment")),
            _ => {
                self.push_comment_str("--!");
                self.reconsume_in(State::Comment);
            }
        }
        Ok(())
    }
}
