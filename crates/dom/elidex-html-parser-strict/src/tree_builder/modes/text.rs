//! WHATWG HTML §13.2.6.4.8 The "text" insertion mode.
//!
//! Entered via the generic raw text / RCDATA / script-data algorithms
//! (§13.2.6.2). While here the tokenizer is in a raw-text state, so it only
//! ever emits character tokens, the element's appropriate end tag, and EOF —
//! never a start tag, comment, or DOCTYPE.

use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.8 — handle a token in the "text" insertion mode.
pub(crate) fn text(tb: &mut TreeBuilder, token: &Token) -> Result<Flow, StrictParseError> {
    match token {
        Token::Character(ch) => {
            tb.insert_character(*ch);
            Ok(Flow::Next)
        }
        // EOF inside raw text (an unterminated title/style/script/textarea) is
        // a parse error; strict mode does not recover.
        Token::EndOfFile => Err(parse_error("eof-in-text")),
        // The appropriate end tag (`</script>` or any other): pop the raw-text
        // element and return to the original insertion mode. The strict parser
        // has no scripting engine, so `</script>` needs no special handling.
        Token::EndTag(_) => {
            tb.pop();
            tb.state.mode = tb
                .state
                .original_mode
                .take()
                .expect("original insertion mode is set whenever the parser is in text mode");
            Ok(Flow::Next)
        }
        // The raw-text tokenizer states cannot emit start tags, comments, or
        // DOCTYPEs, so these are unreachable by construction.
        Token::StartTag(_) | Token::Comment(_) | Token::Doctype(_) => {
            unreachable!("the tokenizer emits only characters and the appropriate end tag in a raw-text state")
        }
    }
}
