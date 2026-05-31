//! WHATWG HTML §13.2.6.4.13 The "in table body" insertion mode.

use super::super::parse_state::{InsertionMode, Scope};
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.13 — handle a token in the "in table body" insertion mode.
pub(crate) fn in_table_body(tb: &mut TreeBuilder, token: &Token) -> Result<Flow, StrictParseError> {
    match token {
        Token::StartTag(tag) if tag.name == "tr" => {
            tb.clear_stack_to_table_body_context();
            tb.insert_html_element(tag)?;
            tb.state.mode = InsertionMode::InRow;
            Ok(Flow::Next)
        }
        // A cell directly inside a table section (without a row) is
        // non-conforming.
        Token::StartTag(tag) if matches!(tag.name.as_str(), "th" | "td") => {
            Err(parse_error("cell-without-row-in-table-body"))
        }
        Token::EndTag(tag) if matches!(tag.name.as_str(), "tbody" | "tfoot" | "thead") => {
            if !tb.has_tag_in_scope(tag.name.as_str(), Scope::Table) {
                return Err(parse_error("unexpected-end-table-section"));
            }
            tb.clear_stack_to_table_body_context();
            tb.pop();
            tb.state.mode = InsertionMode::InTable;
            Ok(Flow::Next)
        }
        Token::StartTag(tag)
            if matches!(
                tag.name.as_str(),
                "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead"
            ) =>
        {
            leave_table_body(tb)
        }
        Token::EndTag(tag) if tag.name == "table" => leave_table_body(tb),
        Token::EndTag(tag)
            if matches!(
                tag.name.as_str(),
                "body" | "caption" | "col" | "colgroup" | "html" | "td" | "th" | "tr"
            ) =>
        {
            Err(parse_error("misplaced-end-tag-in-table-body"))
        }
        _ => super::in_table::in_table(tb, token),
    }
}

/// Close the current table section and return to "in table", then reprocess
/// the token (shared by the table-structure start tags and `</table>`).
fn leave_table_body(tb: &mut TreeBuilder) -> Result<Flow, StrictParseError> {
    if !tb.has_any_tag_in_scope(&["tbody", "thead", "tfoot"], Scope::Table) {
        return Err(parse_error("no-table-body-in-table-scope"));
    }
    tb.clear_stack_to_table_body_context();
    tb.pop();
    tb.state.mode = InsertionMode::InTable;
    Ok(Flow::Reprocess)
}
