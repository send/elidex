//! WHATWG HTML §13.2.6.4.14 The "in row" insertion mode.

use super::super::parse_state::{InsertionMode, Scope};
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.14 — handle a token in the "in row" insertion mode.
pub(crate) fn in_row(tb: &mut TreeBuilder, token: &Token) -> Result<Flow, StrictParseError> {
    match token {
        Token::StartTag(tag) if matches!(tag.name.as_str(), "th" | "td") => {
            tb.clear_stack_to_table_row_context();
            tb.insert_html_element(tag)?;
            tb.state.mode = InsertionMode::InCell;
            // "Insert a marker" — no-op (no active formatting list).
            Ok(Flow::Next)
        }
        Token::EndTag(tag) if tag.name == "tr" => {
            if !tb.has_tag_in_scope("tr", Scope::Table) {
                return Err(parse_error("unexpected-end-tr"));
            }
            close_row(tb);
            Ok(Flow::Next)
        }
        Token::StartTag(tag)
            if matches!(
                tag.name.as_str(),
                "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead" | "tr"
            ) =>
        {
            if !tb.has_tag_in_scope("tr", Scope::Table) {
                return Err(parse_error("no-tr-in-table-scope"));
            }
            close_row(tb);
            Ok(Flow::Reprocess)
        }
        Token::EndTag(tag) if tag.name == "table" => {
            if !tb.has_tag_in_scope("tr", Scope::Table) {
                return Err(parse_error("no-tr-in-table-scope"));
            }
            close_row(tb);
            Ok(Flow::Reprocess)
        }
        Token::EndTag(tag) if matches!(tag.name.as_str(), "tbody" | "tfoot" | "thead") => {
            if !tb.has_tag_in_scope(tag.name.as_str(), Scope::Table) {
                return Err(parse_error("unexpected-end-table-section-in-row"));
            }
            // No tr in scope: ignore (fragment case), no parse error.
            if !tb.has_tag_in_scope("tr", Scope::Table) {
                return Ok(Flow::Next);
            }
            close_row(tb);
            Ok(Flow::Reprocess)
        }
        Token::EndTag(tag)
            if matches!(
                tag.name.as_str(),
                "body" | "caption" | "col" | "colgroup" | "html" | "td" | "th"
            ) =>
        {
            Err(parse_error("misplaced-end-tag-in-row"))
        }
        _ => super::in_table::in_table(tb, token),
    }
}

/// Close the current row and return to "in table body". Callers must have
/// verified that a `tr` is in table scope.
fn close_row(tb: &mut TreeBuilder) {
    tb.clear_stack_to_table_row_context();
    tb.pop();
    tb.state.mode = InsertionMode::InTableBody;
}
