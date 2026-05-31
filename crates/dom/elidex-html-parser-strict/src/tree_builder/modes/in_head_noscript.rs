//! WHATWG HTML §13.2.6.4.5 The "in head noscript" insertion mode.
//!
//! Only reachable when the scripting flag is disabled: with scripting enabled
//! (the v1 baseline), `<noscript>` in "in head" follows the generic raw text
//! algorithm and never switches here. The mode is implemented in full for
//! completeness.

use super::super::parse_state::{is_html_whitespace, InsertionMode};
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::Token;
use crate::StrictParseError;

/// §13.2.6.4.5 — handle a token in the "in head noscript" insertion mode.
pub(crate) fn in_head_noscript(
    tb: &mut TreeBuilder,
    token: &Token,
) -> Result<Flow, StrictParseError> {
    match token {
        Token::Doctype(_) => Err(parse_error("unexpected-doctype-in-head-noscript")),
        Token::StartTag(tag) if tag.name == "html" => super::in_body::in_body(tb, token),
        Token::EndTag(tag) if tag.name == "noscript" => {
            // Pop the noscript element; the new current node is head.
            tb.pop();
            tb.state.mode = InsertionMode::InHead;
            Ok(Flow::Next)
        }
        // Whitespace, comments, and the head-content start tags are handled
        // with the "in head" rules.
        Token::Character(ch) if is_html_whitespace(*ch) => super::in_head::in_head(tb, token),
        Token::Comment(_) => super::in_head::in_head(tb, token),
        Token::StartTag(tag)
            if matches!(
                tag.name.as_str(),
                "basefont" | "bgsound" | "link" | "meta" | "noframes" | "style"
            ) =>
        {
            super::in_head::in_head(tb, token)
        }
        // Everything else (end `br`, `head`/`noscript` start, any other end
        // tag, a non-whitespace character, EOF) is a parse error — strict mode
        // does not perform the spec's "pop noscript and reprocess" recovery.
        _ => Err(parse_error("unexpected-token-in-head-noscript")),
    }
}
