//! WHATWG HTML §13.2.6.4.17 The "after body" insertion mode.

use super::super::parse_state::{is_html_whitespace, InsertionMode};
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.17 — handle a token in the "after body" insertion mode.
pub(crate) fn after_body(tb: &mut TreeBuilder, token: &Token) -> Result<Flow, StrictParseError> {
    match token {
        Token::Character(ch) if is_html_whitespace(*ch) => super::in_body::in_body(tb, token),
        Token::Comment(data) => {
            // Insert as the last child of the html element (the first element
            // on the stack).
            let html = tb
                .state
                .open_elements
                .first()
                .copied()
                .unwrap_or(tb.document);
            tb.insert_comment_to(html, data);
            Ok(Flow::Next)
        }
        Token::Doctype(_) => Err(parse_error("unexpected-doctype-after-body")),
        Token::StartTag(tag) if tag.name == "html" => super::in_body::in_body(tb, token),
        Token::EndTag(tag) if tag.name == "html" => {
            tb.state.mode = InsertionMode::AfterAfterBody;
            Ok(Flow::Next)
        }
        Token::EndOfFile => Ok(Flow::Stop),
        // Non-whitespace content after `</body>` is non-conforming (the spec
        // switches back to "in body" as recovery).
        _ => Err(parse_error("unexpected-content-after-body")),
    }
}
