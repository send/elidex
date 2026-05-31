//! WHATWG HTML §13.2.6.4.16 The "in template" insertion mode.

use super::super::parse_state::InsertionMode;
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.16 — handle a token in the "in template" insertion mode.
pub(crate) fn in_template(tb: &mut TreeBuilder, token: &Token) -> Result<Flow, StrictParseError> {
    match token {
        Token::Character(_) | Token::Comment(_) | Token::Doctype(_) => {
            super::in_body::in_body(tb, token)
        }
        Token::StartTag(tag)
            if matches!(
                tag.name.as_str(),
                "base"
                    | "basefont"
                    | "bgsound"
                    | "link"
                    | "meta"
                    | "noframes"
                    | "script"
                    | "style"
                    | "template"
                    | "title"
            ) =>
        {
            super::in_head::in_head(tb, token)
        }
        Token::EndTag(tag) if tag.name == "template" => super::in_head::in_head(tb, token),
        Token::StartTag(tag)
            if matches!(
                tag.name.as_str(),
                "caption" | "colgroup" | "tbody" | "tfoot" | "thead"
            ) =>
        {
            Ok(switch_template_mode(tb, InsertionMode::InTable))
        }
        Token::StartTag(tag) if tag.name == "col" => {
            Ok(switch_template_mode(tb, InsertionMode::InColumnGroup))
        }
        Token::StartTag(tag) if tag.name == "tr" => {
            Ok(switch_template_mode(tb, InsertionMode::InTableBody))
        }
        Token::StartTag(tag) if matches!(tag.name.as_str(), "td" | "th") => {
            Ok(switch_template_mode(tb, InsertionMode::InRow))
        }
        Token::StartTag(_) => Ok(switch_template_mode(tb, InsertionMode::InBody)),
        Token::EndTag(_) => Err(parse_error("unexpected-end-tag-in-template")),
        Token::EndOfFile => {
            if !tb.has_template_on_stack() {
                // Fragment case: stop parsing.
                return Ok(Flow::Stop);
            }
            // A template left open at EOF is non-conforming.
            Err(parse_error("eof-with-open-template"))
        }
    }
}

/// Swap the current template insertion mode and the insertion mode for `mode`,
/// then reprocess the token (the spec's "pop the current template insertion
/// mode … push _m_ … switch the insertion mode to _m_ … reprocess").
fn switch_template_mode(tb: &mut TreeBuilder, mode: InsertionMode) -> Flow {
    tb.state.template_modes.pop();
    tb.state.template_modes.push(mode);
    tb.state.mode = mode;
    Flow::Reprocess
}
