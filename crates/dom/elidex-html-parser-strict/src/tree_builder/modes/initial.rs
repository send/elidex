//! WHATWG HTML §13.2.6.4.1 The "initial" insertion mode.

use super::super::parse_state::{is_html_whitespace, InsertionMode};
use super::super::{parse_error, Flow, TreeBuilder};
use crate::tokenizer::token::{DoctypeToken, Token};
use crate::StrictParseError;

/// §13.2.6.4.1 — handle a token in the "initial" insertion mode.
pub(crate) fn initial(tb: &mut TreeBuilder, token: &Token) -> Result<Flow, StrictParseError> {
    match token {
        // Whitespace is ignored.
        Token::Character(ch) if is_html_whitespace(*ch) => Ok(Flow::Next),
        // A comment is inserted as the last child of the Document.
        Token::Comment(data) => {
            tb.insert_comment_to(tb.document, data);
            Ok(Flow::Next)
        }
        Token::Doctype(dt) => {
            // A non-conforming DOCTYPE is a parse error. Every conforming
            // DOCTYPE is, by construction, a no-quirks DOCTYPE (the quirks
            // conditions are a subset of the parse-error conditions), so no
            // document-mode state is set.
            if !is_conforming_doctype(dt) {
                return Err(parse_error("non-conforming-doctype"));
            }
            tb.insert_doctype(dt);
            tb.state.mode = InsertionMode::BeforeHtml;
            Ok(Flow::Next)
        }
        // Anything else (a non-whitespace character, any tag, or EOF) before a
        // DOCTYPE is the "missing DOCTYPE" parse error: strict mode requires a
        // document to begin with `<!DOCTYPE html>` (after optional whitespace
        // and comments).
        _ => Err(parse_error("missing-doctype")),
    }
}

/// Whether `dt` is a conforming HTML5 DOCTYPE (§13.2.6.4.1): name `html`, no
/// public identifier, a missing or `about:legacy-compat` system identifier,
/// and the force-quirks flag off.
fn is_conforming_doctype(dt: &DoctypeToken) -> bool {
    dt.name.as_deref() == Some("html")
        && dt.public_id.is_none()
        && matches!(dt.system_id.as_deref(), None | Some("about:legacy-compat"))
        && !dt.force_quirks
}
