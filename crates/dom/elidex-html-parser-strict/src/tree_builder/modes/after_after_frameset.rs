//! WHATWG HTML §13.2.6.4.21 The "after after frameset" insertion mode.

use super::super::parse_state::is_html_whitespace;
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.21 — handle a token in the "after after frameset" insertion mode.
pub(crate) fn after_after_frameset(
    tb: &mut TreeBuilder,
    token: &Token,
) -> Result<Flow, StrictParseError> {
    match token {
        Token::Comment(data) => {
            tb.insert_comment_to(tb.document, data);
            Ok(Flow::Next)
        }
        Token::Doctype(_) => super::in_body::in_body(tb, token),
        Token::Character(ch) if is_html_whitespace(*ch) => super::in_body::in_body(tb, token),
        Token::StartTag(tag) if tag.name == "html" => super::in_body::in_body(tb, token),
        Token::EndOfFile => Ok(Flow::Stop),
        Token::StartTag(tag) if tag.name == "noframes" => super::in_head::in_head(tb, token),
        _ => Err(parse_error("unexpected-token-after-after-frameset")),
    }
}
