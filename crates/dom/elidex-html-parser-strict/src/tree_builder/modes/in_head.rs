//! WHATWG HTML §13.2.6.4.4 The "in head" insertion mode.

use super::super::parse_state::{is_html_whitespace, InsertionMode};
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.4 — handle a token in the "in head" insertion mode.
pub(crate) fn in_head(tb: &mut TreeBuilder, token: &Token) -> Result<Flow, StrictParseError> {
    match token {
        Token::Character(ch) if is_html_whitespace(*ch) => {
            tb.insert_character(*ch);
            Ok(Flow::Next)
        }
        Token::Comment(data) => {
            tb.insert_comment(data);
            Ok(Flow::Next)
        }
        Token::Doctype(_) => Err(parse_error("unexpected-doctype-in-head")),
        Token::StartTag(tag) => match tag.name.as_str() {
            "html" => super::in_body::in_body(tb, token),
            // `meta` is grouped here: the strict parser takes `&str` input and
            // does no charset detection, so the spec's meta charset handling is a
            // no-op and meta is just another void head element.
            "base" | "basefont" | "bgsound" | "link" | "meta" => {
                tb.insert_void_element(tag);
                Ok(Flow::Next)
            }
            "title" => {
                tb.parse_generic_rcdata(tag)?;
                Ok(Flow::Next)
            }
            "noframes" | "style" => {
                tb.parse_generic_rawtext(tag)?;
                Ok(Flow::Next)
            }
            "noscript" => {
                if tb.state.scripting {
                    // Scripting enabled: noscript content is raw text.
                    tb.parse_generic_rawtext(tag)?;
                } else {
                    tb.insert_html_element(tag)?;
                    tb.state.mode = InsertionMode::InHeadNoscript;
                }
                Ok(Flow::Next)
            }
            "script" => {
                tb.parse_script_element(tag)?;
                Ok(Flow::Next)
            }
            "template" => {
                tb.template_start_tag(tag)?;
                Ok(Flow::Next)
            }
            "head" => Err(parse_error("unexpected-head-start-tag-in-head")),
            _ => Ok(anything_else(tb)),
        },
        Token::EndTag(tag) => match tag.name.as_str() {
            "head" => {
                // Pop the head element off the stack.
                tb.pop();
                tb.state.mode = InsertionMode::AfterHead;
                Ok(Flow::Next)
            }
            "body" | "html" | "br" => Ok(anything_else(tb)),
            "template" => {
                tb.template_end_tag()?;
                Ok(Flow::Next)
            }
            _ => Err(parse_error("unexpected-end-tag-in-head")),
        },
        // A non-whitespace character or EOF: anything else.
        _ => Ok(anything_else(tb)),
    }
}

/// "Anything else": pop the head element, switch to "after head", and
/// reprocess the current token.
fn anything_else(tb: &mut TreeBuilder) -> Flow {
    tb.pop();
    tb.state.mode = InsertionMode::AfterHead;
    Flow::Reprocess
}
