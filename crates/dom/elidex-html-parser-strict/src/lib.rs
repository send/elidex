//! Strict HTML parser for elidex — Phase A1 skeleton.
//!
//! This crate is the SoT for "strict mode" HTML parsing in elidex:
//! valid HTML5 only, no error recovery (adoption agency / foster
//! parenting / misnested-formatting reconstruction are not implemented
//! by design). The first parse error encountered aborts with
//! [`StrictParseError`].
//!
//! # Phase A staging
//!
//! Per the Phase A plan
//! (`m4-12-pr-html-parser-strict-phase-a-plan.md`), the strict parser
//! lands in 4 sub-PRs:
//!
//! - **A1 (this PR)**: crate skeleton + SoT types ([`ParseResult`], [`ParseFragmentOptions`], [`StrictParseError`]) + [`parse_strict`] stub. Skeleton tests `#[ignore]` until A4 activation.
//! - **A2**: WHATWG HTML `§13.2.5` tokenizer (80 states) + `§13.5` named character reference table.
//! - **A3**: WHATWG HTML `§13.2.4` parse state + `§13.2.6` tree construction (21 insertion modes, error branches excluded) + HTML `§4.12.3` + DOM `§4.9` declarative shadow root attach.
//! - **A4**: [`parse_strict`] wiring + `§13.2.7` stopping parsing + delete legacy `elidex-html-parser::parse_strict` (caller-zero dead code) + type SoT facade re-export on `elidex-html-parser`.
//!
//! # Engine independence
//!
//! Per the Layering mandate (`CLAUDE.md` "Layering check —
//! Engine-indep API mapping" section of the Phase A plan), this crate
//! depends only on [`elidex_ecs`]. No VM / script-session / DOM-API
//! coupling. Tree builder (A3) calls `EcsDom::create_element`,
//! `EcsDom::create_text`, `EcsDom::create_comment`,
//! `EcsDom::create_document_root`, `EcsDom::append_child`, and
//! `EcsDom::attach_shadow_with_init` directly — same API surface as
//! the existing compat path in `crates/dom/elidex-html-parser/src/convert.rs`.

mod error;
mod result;
mod tokenizer;
mod tree_builder;

pub use error::StrictParseError;
pub use result::{ParseFragmentOptions, ParseResult};

/// Parse an HTML5 document in strict mode.
///
/// Returns `Ok(ParseResult)` if the input is valid HTML5 (no parse
/// errors per WHATWG HTML §13.2.2), otherwise `Err(StrictParseError)`
/// at the first error encountered (no recovery).
///
/// # Phase A1 skeleton stub
///
/// This skeleton implementation returns
/// `Err(StrictParseError { errors: vec!["unimplemented: …"] })` for
/// any input. Real tokenization + tree construction land in A2-A4 per
/// the Phase A plan.
///
/// # Spec reference
///
/// WHATWG HTML §13.2 "Parsing HTML documents". See the crate-level
/// docstring for the per-sub-PR coverage map.
pub fn parse_strict(_html: &str) -> Result<ParseResult, StrictParseError> {
    Err(StrictParseError {
        errors: vec![
            "unimplemented: tokenizer pending A2, tree builder pending A3, wiring pending A4"
                .to_string(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Skeleton placeholder: A4 will activate this as a valid HTML5
    /// round-trip test (`<!DOCTYPE html><html><head></head><body><p>Hello</p></body></html>`).
    #[test]
    #[ignore = "skeleton stage (A1) — activate in A4"]
    fn strict_valid_html() {
        let html = "<!DOCTYPE html><html><head></head><body><p>Hello</p></body></html>";
        let result = parse_strict(html);
        assert!(result.is_ok());
    }

    /// Skeleton placeholder: A4 will activate this as a strict-reject
    /// test for mismatched tags (`<div><span></div>` → first error
    /// aborts with `Err(StrictParseError)`).
    #[test]
    #[ignore = "skeleton stage (A1) — activate in A4"]
    fn strict_invalid_html() {
        let html = "<div><span></div>";
        let result = parse_strict(html);
        assert!(result.is_err());
    }

    /// Skeleton placeholder: A4 will activate this as a parse-error
    /// message presence check (each error string non-empty).
    #[test]
    #[ignore = "skeleton stage (A1) — activate in A4"]
    fn strict_error_messages() {
        let html = "<div><span></div>";
        let err = parse_strict(html).unwrap_err();
        assert!(!err.errors.is_empty());
        for msg in &err.errors {
            assert!(!msg.is_empty());
        }
    }

    /// Skeleton placeholder: A4 will activate this as a `ParseResult`
    /// invariant check — strict mode always returns `encoding: None`
    /// (no charset detection, `&str` input).
    #[test]
    #[ignore = "skeleton stage (A1) — activate in A4"]
    fn strict_encoding_is_none() {
        let html = "<!DOCTYPE html><html></html>";
        let result = parse_strict(html).expect("valid HTML5");
        assert!(result.encoding.is_none());
    }

    /// A1-active sanity check: the skeleton stub returns Err for any
    /// input, including empty. Verifies the stub is wired.
    #[test]
    fn skeleton_stub_returns_err() {
        let result = parse_strict("");
        let err = result.expect_err("A1 skeleton always errs");
        assert!(!err.errors.is_empty());
        assert!(err.errors[0].contains("unimplemented"));
    }

    /// A1-active sanity check: the skeleton stub error mentions the
    /// pending sub-PR stages (A2 / A3 / A4) so failures during the
    /// A2-A4 transition are easy to trace.
    #[test]
    fn skeleton_stub_error_cites_stages() {
        let result = parse_strict("<html></html>");
        let err = result.expect_err("A1 skeleton always errs");
        let msg = &err.errors[0];
        assert!(msg.contains("A2"));
        assert!(msg.contains("A3"));
        assert!(msg.contains("A4"));
    }
}
