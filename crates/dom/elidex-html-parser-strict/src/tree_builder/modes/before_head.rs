//! WHATWG HTML §13.2.6.4.3 The "before head" insertion mode.

use super::super::parse_state::{is_html_whitespace, InsertionMode};
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.3 — handle a token in the "before head" insertion mode.
pub(crate) fn before_head(tb: &mut TreeBuilder, token: &Token) -> Result<Flow, StrictParseError> {
    match token {
        Token::Character(ch) if is_html_whitespace(*ch) => Ok(Flow::Next),
        Token::Comment(data) => {
            tb.insert_comment(data);
            Ok(Flow::Next)
        }
        Token::Doctype(_) => Err(parse_error("unexpected-doctype-before-head")),
        Token::StartTag(tag) if tag.name == "html" => super::in_body::in_body(tb, token),
        Token::StartTag(tag) if tag.name == "head" => {
            let head = tb.insert_html_element(tag)?;
            tb.state.head_pointer = Some(head);
            tb.state.mode = InsertionMode::InHead;
            Ok(Flow::Next)
        }
        Token::EndTag(tag) if matches!(tag.name.as_str(), "head" | "body" | "html" | "br") => {
            Ok(anything_else(tb))
        }
        Token::EndTag(_) => Err(parse_error("unexpected-end-tag-before-head")),
        _ => Ok(anything_else(tb)),
    }
}

/// "Anything else": insert an implied `head`, set the head element pointer,
/// switch to "in head", and reprocess the current token.
fn anything_else(tb: &mut TreeBuilder) -> Flow {
    let head = tb.insert_element_named("head", &[]);
    tb.state.head_pointer = Some(head);
    tb.state.mode = InsertionMode::InHead;
    Flow::Reprocess
}
