//! WHATWG HTML §13.2.6.4.9 The "in table" insertion mode.
//!
//! Strict mode never enables foster parenting, so every branch the spec routes
//! through "anything else" (misnested content in a table) is a parse error.

use super::super::parse_state::{InsertionMode, Scope};
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.9 — handle a token in the "in table" insertion mode.
pub(crate) fn in_table(tb: &mut TreeBuilder, token: &Token) -> Result<Flow, StrictParseError> {
    match token {
        // Character tokens with a table-ish current node are gathered by the
        // "in table text" mode.
        Token::Character(_)
            if tb.current_node_has_any_tag(&[
                "table", "tbody", "template", "tfoot", "thead", "tr",
            ]) =>
        {
            tb.pending_table_text.clear();
            tb.state.original_mode = Some(tb.state.mode);
            tb.state.mode = InsertionMode::InTableText;
            Ok(Flow::Reprocess)
        }
        Token::Comment(data) => {
            tb.insert_comment(data);
            Ok(Flow::Next)
        }
        Token::Doctype(_) => Err(parse_error("unexpected-doctype-in-table")),
        Token::StartTag(tag) => match tag.name.as_str() {
            "caption" => {
                tb.clear_stack_to_table_context();
                // "Insert a marker" — no-op (no active formatting list).
                tb.insert_html_element(tag)?;
                tb.state.mode = InsertionMode::InCaption;
                Ok(Flow::Next)
            }
            "colgroup" => {
                tb.clear_stack_to_table_context();
                tb.insert_html_element(tag)?;
                tb.state.mode = InsertionMode::InColumnGroup;
                Ok(Flow::Next)
            }
            "col" => {
                tb.clear_stack_to_table_context();
                tb.insert_element_named("colgroup", &[]);
                tb.state.mode = InsertionMode::InColumnGroup;
                Ok(Flow::Reprocess)
            }
            "tbody" | "tfoot" | "thead" => {
                tb.clear_stack_to_table_context();
                tb.insert_html_element(tag)?;
                tb.state.mode = InsertionMode::InTableBody;
                Ok(Flow::Next)
            }
            "td" | "th" | "tr" => {
                tb.clear_stack_to_table_context();
                tb.insert_element_named("tbody", &[]);
                tb.state.mode = InsertionMode::InTableBody;
                Ok(Flow::Reprocess)
            }
            // A nested table closes the open one in the tolerant parser; strict
            // rejects it.
            "table" => Err(parse_error("nested-table")),
            "style" | "script" | "template" => super::in_head::in_head(tb, token),
            // input / form directly in a table need foster parenting or are
            // otherwise out of place — a parse error.
            "input" => Err(parse_error("unexpected-input-in-table")),
            "form" => Err(parse_error("unexpected-form-in-table")),
            _ => Err(parse_error("misplaced-start-tag-in-table")),
        },
        Token::EndTag(tag) => match tag.name.as_str() {
            "table" => {
                if !tb.has_tag_in_scope("table", Scope::Table) {
                    return Err(parse_error("unexpected-end-table-no-table-in-scope"));
                }
                tb.pop_until_tag("table");
                tb.reset_insertion_mode_appropriately();
                Ok(Flow::Next)
            }
            "template" => super::in_head::in_head(tb, token),
            "body" | "caption" | "col" | "colgroup" | "html" | "tbody" | "td" | "tfoot" | "th"
            | "thead" | "tr" => Err(parse_error("misplaced-end-tag-in-table")),
            _ => Err(parse_error("unexpected-end-tag-in-table")),
        },
        Token::EndOfFile => super::in_body::in_body(tb, token),
        // Anything else (a non-table-ish character, etc.) would require foster
        // parenting.
        Token::Character(_) => Err(parse_error("foster-parented-content-in-table")),
    }
}
