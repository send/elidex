//! WHATWG HTML §13.2.6.4.15 The "in cell" insertion mode.

use super::super::parse_state::{InsertionMode, Scope};
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.15 — handle a token in the "in cell" insertion mode.
pub(crate) fn in_cell(tb: &mut TreeBuilder, token: &Token) -> Result<Flow, StrictParseError> {
    match token {
        Token::EndTag(tag) if matches!(tag.name.as_str(), "td" | "th") => {
            let name = tag.name.as_str();
            if !tb.has_tag_in_scope(name, Scope::Table) {
                return Err(parse_error("unexpected-end-cell"));
            }
            tb.generate_implied_end_tags();
            if !tb.current_node_has_tag(name) {
                return Err(parse_error("misnested-end-cell"));
            }
            tb.pop_until_tag(name);
            // "Clear the list of active formatting elements up to the last
            // marker" — no-op (no list).
            tb.state.mode = InsertionMode::InRow;
            Ok(Flow::Next)
        }
        Token::StartTag(tag)
            if matches!(
                tag.name.as_str(),
                "caption" | "col" | "colgroup" | "tbody" | "td" | "tfoot" | "th" | "thead" | "tr"
            ) =>
        {
            if !tb.has_any_tag_in_scope(&["td", "th"], Scope::Table) {
                return Err(parse_error("cell-start-without-cell-in-scope"));
            }
            close_cell(tb)?;
            Ok(Flow::Reprocess)
        }
        Token::EndTag(tag)
            if matches!(
                tag.name.as_str(),
                "body" | "caption" | "col" | "colgroup" | "html"
            ) =>
        {
            Err(parse_error("misplaced-end-tag-in-cell"))
        }
        Token::EndTag(tag)
            if matches!(
                tag.name.as_str(),
                "table" | "tbody" | "tfoot" | "thead" | "tr"
            ) =>
        {
            if !tb.has_tag_in_scope(tag.name.as_str(), Scope::Table) {
                return Err(parse_error("unexpected-end-tag-in-cell"));
            }
            close_cell(tb)?;
            Ok(Flow::Reprocess)
        }
        _ => super::in_body::in_body(tb, token),
    }
}

/// §13.2.6.4.15 "close the cell": pop to the open `td`/`th` and return to
/// "in row".
fn close_cell(tb: &mut TreeBuilder) -> Result<(), StrictParseError> {
    tb.generate_implied_end_tags();
    if !tb.current_node_has_any_tag(&["td", "th"]) {
        return Err(parse_error("misnested-cell"));
    }
    tb.pop_until_any(&["td", "th"]);
    // "Clear the list of active formatting elements up to the last marker" —
    // no-op (no list).
    tb.state.mode = InsertionMode::InRow;
    Ok(())
}
