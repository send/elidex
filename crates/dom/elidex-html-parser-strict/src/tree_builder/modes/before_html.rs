//! WHATWG HTML §13.2.6.4.2 The "before html" insertion mode.

use super::super::parse_state::{is_html_whitespace, InsertionMode};
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.2 — handle a token in the "before html" insertion mode.
pub(crate) fn before_html(tb: &mut TreeBuilder, token: &Token) -> Result<Flow, StrictParseError> {
    match token {
        Token::Doctype(_) => Err(parse_error("unexpected-doctype-before-html")),
        Token::Comment(data) => {
            tb.insert_comment_to(tb.document, data);
            Ok(Flow::Next)
        }
        Token::Character(ch) if is_html_whitespace(*ch) => Ok(Flow::Next),
        Token::StartTag(tag) if tag.name == "html" => {
            // Create the html element, append it to the Document, and push it
            // onto the (empty) stack of open elements.
            tb.insert_html_element(tag)?;
            tb.state.mode = InsertionMode::BeforeHead;
            Ok(Flow::Next)
        }
        Token::EndTag(tag) if matches!(tag.name.as_str(), "head" | "body" | "html" | "br") => {
            Ok(anything_else(tb))
        }
        Token::EndTag(_) => Err(parse_error("unexpected-end-tag-before-html")),
        _ => Ok(anything_else(tb)),
    }
}

/// "Anything else": create an `html` element, push it, switch to "before
/// head", and reprocess the current token.
fn anything_else(tb: &mut TreeBuilder) -> Flow {
    tb.insert_element_named("html", &[]);
    tb.state.mode = InsertionMode::BeforeHead;
    Flow::Reprocess
}
