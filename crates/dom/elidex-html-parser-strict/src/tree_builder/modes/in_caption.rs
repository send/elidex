//! WHATWG HTML §13.2.6.4.11 The "in caption" insertion mode.

use super::super::parse_state::{InsertionMode, Scope};
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.11 — handle a token in the "in caption" insertion mode.
pub(crate) fn in_caption(tb: &mut TreeBuilder, token: &Token) -> Result<Flow, StrictParseError> {
    match token {
        Token::EndTag(tag) if tag.name == "caption" => {
            close_caption(tb)?;
            Ok(Flow::Next)
        }
        Token::StartTag(tag)
            if matches!(
                tag.name.as_str(),
                "caption" | "col" | "colgroup" | "tbody" | "td" | "tfoot" | "th" | "thead" | "tr"
            ) =>
        {
            close_caption(tb)?;
            Ok(Flow::Reprocess)
        }
        Token::EndTag(tag) if tag.name == "table" => {
            close_caption(tb)?;
            Ok(Flow::Reprocess)
        }
        Token::EndTag(tag)
            if matches!(
                tag.name.as_str(),
                "body"
                    | "col"
                    | "colgroup"
                    | "html"
                    | "tbody"
                    | "td"
                    | "tfoot"
                    | "th"
                    | "thead"
                    | "tr"
            ) =>
        {
            Err(parse_error("misplaced-end-tag-in-caption"))
        }
        _ => super::in_body::in_body(tb, token),
    }
}

/// Close the open caption (shared by `</caption>`, `</table>`, and the
/// table-structure start tags), switching back to "in table".
fn close_caption(tb: &mut TreeBuilder) -> Result<(), StrictParseError> {
    if !tb.has_tag_in_scope("caption", Scope::Table) {
        return Err(parse_error("no-caption-in-table-scope"));
    }
    tb.generate_implied_end_tags();
    if !tb.current_node_has_tag("caption") {
        return Err(parse_error("misnested-caption"));
    }
    tb.pop_until_tag("caption");
    // "Clear the list of active formatting elements up to the last marker" —
    // no-op (no list).
    tb.state.mode = InsertionMode::InTable;
    Ok(())
}
