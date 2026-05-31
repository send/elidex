//! WHATWG HTML §13.2.6.4.6 The "after head" insertion mode.

use super::super::parse_state::{is_html_whitespace, InsertionMode};
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.6 — handle a token in the "after head" insertion mode.
pub(crate) fn after_head(tb: &mut TreeBuilder, token: &Token) -> Result<Flow, StrictParseError> {
    match token {
        Token::Character(ch) if is_html_whitespace(*ch) => {
            tb.insert_character(*ch);
            Ok(Flow::Next)
        }
        Token::Comment(data) => {
            tb.insert_comment(data);
            Ok(Flow::Next)
        }
        Token::Doctype(_) => Err(parse_error("unexpected-doctype-after-head")),
        Token::StartTag(tag) => match tag.name.as_str() {
            "html" => super::in_body::in_body(tb, token),
            "body" => {
                tb.insert_html_element(tag)?;
                tb.state.frameset_ok = false;
                tb.state.mode = InsertionMode::InBody;
                Ok(Flow::Next)
            }
            "frameset" => {
                tb.insert_html_element(tag)?;
                tb.state.mode = InsertionMode::InFrameset;
                Ok(Flow::Next)
            }
            // Head-content start tags after `</head>` are out of place: a
            // parse error in strict mode (the spec re-opens the head as
            // recovery).
            "base" | "basefont" | "bgsound" | "link" | "meta" | "noframes" | "script" | "style"
            | "template" | "title" => Err(parse_error("unexpected-head-element-after-head")),
            "head" => Err(parse_error("unexpected-head-start-tag-after-head")),
            _ => Ok(anything_else(tb)),
        },
        Token::EndTag(tag) => match tag.name.as_str() {
            "template" => super::in_head::in_head(tb, token),
            "body" | "html" | "br" => Ok(anything_else(tb)),
            _ => Err(parse_error("unexpected-end-tag-after-head")),
        },
        _ => Ok(anything_else(tb)),
    }
}

/// "Anything else": insert an implied `body`, switch to "in body", and
/// reprocess the current token.
fn anything_else(tb: &mut TreeBuilder) -> Flow {
    tb.insert_element_named("body", &[]);
    tb.state.mode = InsertionMode::InBody;
    Flow::Reprocess
}
