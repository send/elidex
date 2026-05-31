//! WHATWG HTML §13.2.6.4.20 The "after after body" insertion mode.

use super::super::parse_state::is_html_whitespace;
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.20 — handle a token in the "after after body" insertion mode.
pub(crate) fn after_after_body(
    tb: &mut TreeBuilder,
    token: &Token,
) -> Result<Flow, StrictParseError> {
    match token {
        Token::Comment(data) => {
            tb.insert_comment_to(tb.document, data);
            Ok(Flow::Next)
        }
        // A DOCTYPE, whitespace, or an `html` start tag is processed with the
        // "in body" rules (whitespace is appended; a DOCTYPE there is a parse
        // error).
        Token::Doctype(_) => super::in_body::in_body(tb, token),
        Token::Character(ch) if is_html_whitespace(*ch) => super::in_body::in_body(tb, token),
        Token::StartTag(tag) if tag.name == "html" => super::in_body::in_body(tb, token),
        Token::EndOfFile => Ok(Flow::Stop),
        _ => Err(parse_error("unexpected-content-after-after-body")),
    }
}
