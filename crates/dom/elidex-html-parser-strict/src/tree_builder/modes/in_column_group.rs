//! WHATWG HTML §13.2.6.4.12 The "in column group" insertion mode.

use super::super::parse_state::{is_html_whitespace, InsertionMode};
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.12 — handle a token in the "in column group" insertion mode.
pub(crate) fn in_column_group(
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
        Token::Doctype(_) => Err(parse_error("unexpected-doctype-in-column-group")),
        Token::StartTag(tag) if tag.name == "html" => super::in_body::in_body(tb, token),
        Token::StartTag(tag) if tag.name == "col" => {
            tb.insert_void_element(tag);
            Ok(Flow::Next)
        }
        Token::StartTag(tag) if tag.name == "template" => super::in_head::in_head(tb, token),
        Token::EndTag(tag) if tag.name == "template" => super::in_head::in_head(tb, token),
        Token::EndTag(tag) if tag.name == "colgroup" => {
            if !tb.current_node_has_tag("colgroup") {
                return Err(parse_error("unexpected-end-colgroup"));
            }
            tb.pop();
            tb.state.mode = InsertionMode::InTable;
            Ok(Flow::Next)
        }
        Token::EndTag(tag) if tag.name == "col" => Err(parse_error("unexpected-end-col")),
        Token::EndOfFile => super::in_body::in_body(tb, token),
        // Anything else: a `colgroup` can only contain `col`, so leave it.
        _ => {
            if !tb.current_node_has_tag("colgroup") {
                return Err(parse_error("unexpected-token-in-column-group"));
            }
            tb.pop();
            tb.state.mode = InsertionMode::InTable;
            Ok(Flow::Reprocess)
        }
    }
}
