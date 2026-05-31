//! WHATWG HTML §13.2.6.4.10 The "in table text" insertion mode.

use super::super::parse_state::is_html_whitespace;
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.10 — handle a token in the "in table text" insertion mode.
pub(crate) fn in_table_text(tb: &mut TreeBuilder, token: &Token) -> Result<Flow, StrictParseError> {
    // U+0000 NULL is impossible (the strict tokenizer rejects it). Other
    // characters accumulate in the pending table character run.
    if let Token::Character(ch) = token {
        tb.pending_table_text.push(*ch);
        return Ok(Flow::Next);
    }

    // Anything else: decide what to do with the accumulated run.
    let run = std::mem::take(&mut tb.pending_table_text);
    if run.chars().any(|ch| !is_html_whitespace(ch)) {
        // Non-whitespace character data in a table would be foster-parented —
        // a parse error in strict mode.
        return Err(parse_error("non-whitespace-in-table-text"));
    }
    // Pure whitespace is inserted as a text node (at the table, which is still
    // the current node), then the original mode resumes.
    if !run.is_empty() {
        tb.insert_text_run(&run);
    }
    tb.state.mode = tb
        .state
        .original_mode
        .take()
        .expect("original insertion mode is set whenever the parser is in in-table-text mode");
    Ok(Flow::Reprocess)
}
