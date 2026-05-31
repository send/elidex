//! WHATWG HTML §13.2.6.4.18 The "in frameset" insertion mode.

use super::super::parse_state::{is_html_whitespace, InsertionMode};
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.18 — handle a token in the "in frameset" insertion mode.
pub(crate) fn in_frameset(tb: &mut TreeBuilder, token: &Token) -> Result<Flow, StrictParseError> {
    match token {
        Token::Character(ch) if is_html_whitespace(*ch) => {
            tb.insert_character(*ch);
            Ok(Flow::Next)
        }
        Token::Comment(data) => {
            tb.insert_comment(data);
            Ok(Flow::Next)
        }
        Token::Doctype(_) => Err(parse_error("unexpected-doctype-in-frameset")),
        Token::StartTag(tag) if tag.name == "html" => super::in_body::in_body(tb, token),
        Token::StartTag(tag) if tag.name == "frameset" => {
            tb.insert_html_element(tag)?;
            Ok(Flow::Next)
        }
        Token::EndTag(tag) if tag.name == "frameset" => {
            // The root html element as current node is the fragment case.
            if tb.current_node_has_tag("html") {
                return Err(parse_error("unexpected-end-frameset-at-root"));
            }
            tb.pop();
            if !tb.current_node_has_tag("frameset") {
                tb.state.mode = InsertionMode::AfterFrameset;
            }
            Ok(Flow::Next)
        }
        Token::StartTag(tag) if tag.name == "frame" => {
            tb.insert_void_element(tag);
            Ok(Flow::Next)
        }
        Token::StartTag(tag) if tag.name == "noframes" => super::in_head::in_head(tb, token),
        Token::EndOfFile => {
            // The current node can only be the root html element in the
            // fragment case; otherwise a frameset is left open.
            if !tb.current_node_has_tag("html") {
                return Err(parse_error("eof-in-frameset"));
            }
            Ok(Flow::Stop)
        }
        _ => Err(parse_error("unexpected-token-in-frameset")),
    }
}
