//! WHATWG HTML §13.2.6.4.19 The "after frameset" insertion mode.

use super::super::parse_state::{is_html_whitespace, InsertionMode};
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.19 — handle a token in the "after frameset" insertion mode.
pub(crate) fn after_frameset(
    tb: &mut TreeBuilder,
    token: &Token,
) -> Result<Flow, StrictParseError> {
    match token {
        Token::Character(ch) if is_html_whitespace(*ch) => {
            tb.insert_character(*ch);
            Ok(Flow::Next)
        }
        Token::Comment(data) => {
            tb.insert_comment(data);
            Ok(Flow::Next)
        }
        Token::Doctype(_) => Err(parse_error("unexpected-doctype-after-frameset")),
        Token::StartTag(tag) if tag.name == "html" => super::in_body::in_body(tb, token),
        Token::EndTag(tag) if tag.name == "html" => {
            tb.state.mode = InsertionMode::AfterAfterFrameset;
            Ok(Flow::Next)
        }
        Token::StartTag(tag) if tag.name == "noframes" => super::in_head::in_head(tb, token),
        Token::EndOfFile => Ok(Flow::Stop),
        _ => Err(parse_error("unexpected-token-after-frameset")),
    }
}
