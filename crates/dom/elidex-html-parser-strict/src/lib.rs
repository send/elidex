//! Strict HTML parser for elidex.
//!
//! This crate is the SoT for "strict mode" HTML parsing in elidex:
//! valid HTML5 only, no error recovery (adoption agency / foster
//! parenting / misnested-formatting reconstruction are not implemented
//! by design). The first parse error encountered aborts with
//! [`StrictParseError`].
//!
//! # Spec coverage
//!
//! WHATWG HTML §13.2 "Parsing HTML documents":
//!
//! - `§13.2.5` Tokenization — the strict-reachable states (the two
//!   bogus *recovery* states are omitted: their entry conditions are
//!   rejected, not recovered) + `§13.5` named character reference table.
//! - `§13.2.4` Parse state + `§13.2.6` Tree construction — 21 insertion
//!   modes (error branches excluded by the strict no-recovery contract),
//!   `§13.2.6.5` inline foreign content (SVG / MathML, handled as a
//!   `§13.2.6` dispatcher branch rather than a 22nd insertion mode), and
//!   HTML `§4.12.3` / DOM `§4.9` declarative shadow root attach.
//! - `§13.2.7` The end — stop parsing (the deferred / async script
//!   timing steps 5 / 7 are the consumer's concern, not the parser's).
//!
//! Foreign-content attribute-namespace binding (XLink / xml / xmlns
//! prefixes → namespace URIs) is deferred to the attribute-namespace data
//! model (`#11-xml-namespace`); the prefixed attribute names are retained
//! verbatim under the current flat attribute map.
//!
//! # Engine independence
//!
//! Per the Layering mandate (`CLAUDE.md` "Layering check —
//! Engine-indep API mapping" section of the Phase A plan), this crate
//! depends only on [`elidex_ecs`]. No VM / script-session / DOM-API
//! coupling. The tree builder calls `EcsDom::create_element`,
//! `EcsDom::create_text`, `EcsDom::create_comment`,
//! `EcsDom::create_document_root`, `EcsDom::append_child`, and
//! `EcsDom::attach_shadow_with_init` directly — same API surface as
//! the existing compat path in `crates/dom/elidex-html-parser/src/convert.rs`.

mod error;
mod result;
mod tokenizer;
mod tree_builder;

pub use error::StrictParseError;
pub use result::{ParseFragmentOptions, ParseResult, ParseTier};

/// Parse an HTML5 document in strict mode.
///
/// Returns `Ok(ParseResult)` if the input is fully-conforming HTML5 (no
/// WHATWG HTML §13.2.2 parse error), otherwise `Err(StrictParseError)`
/// at the first parse error encountered — strict mode performs no error
/// recovery (no foster parenting, adoption agency, or misnested-tag
/// reconstruction).
///
/// The returned [`ParseResult`] owns the populated `EcsDom` and the
/// document root entity. `errors` is always empty on the `Ok` path and
/// `encoding` is always `None` (strict mode takes `&str` input — no
/// charset detection).
///
/// # Spec reference
///
/// WHATWG HTML §13.2 "Parsing HTML documents": §13.2.5 tokenization →
/// §13.2.6 tree construction → §13.2.7 "The end" (stop parsing). See the
/// crate-level docstring for the full per-section coverage map.
pub fn parse_strict(html: &str) -> Result<ParseResult, StrictParseError> {
    tree_builder::TreeBuilder::build(html)
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::{EcsDom, Entity, TextContent};

    /// Recursively find the first element with the given tag.
    fn find_tag(dom: &EcsDom, parent: Entity, tag: &str) -> Option<Entity> {
        for child in dom.children_iter(parent) {
            if dom.has_tag(child, tag) {
                return Some(child);
            }
            if let Some(found) = find_tag(dom, child, tag) {
                return Some(found);
            }
        }
        None
    }

    /// Concatenate text content of an element's direct children.
    fn child_text(dom: &EcsDom, parent: Entity) -> String {
        let mut out = String::new();
        for child in dom.children_iter(parent) {
            if let Ok(tc) = dom.world().get::<&TextContent>(child) {
                out.push_str(&tc.0);
            }
        }
        out
    }

    #[test]
    fn strict_valid_html_produces_populated_dom() {
        // End-to-end through the public API: a valid document yields a
        // *populated* `ParseResult` — the delegation actually builds a
        // tree, not just an empty `Ok`. The full tree-construction shape
        // is golden-tested in `tree_builder/tests.rs`; here we only need
        // one walk to prove the public seam delivers a real DOM.
        let html = "<!DOCTYPE html><html><head></head><body><p>Hello</p></body></html>";
        let result = parse_strict(html).expect("valid HTML5");
        let p = find_tag(&result.dom, result.document, "p").expect("p in tree");
        assert_eq!(child_text(&result.dom, p), "Hello");
    }

    #[test]
    fn strict_rejects_misnested_tags_with_messages() {
        // `</div>` closes while `<span>` is the current node — a
        // §13.2.6.4.7 "any other end tag" parse error that strict mode
        // rejects (no recovery). `unwrap_err` proves rejection; the loop
        // checks every reported message is non-empty.
        let html = "<!DOCTYPE html><html><head></head><body><div><span></div></body></html>";
        let err = parse_strict(html).unwrap_err();
        assert!(!err.errors.is_empty());
        for msg in &err.errors {
            assert!(!msg.is_empty());
        }
    }

    #[test]
    fn strict_requires_doctype() {
        // Strict mode requires a document to begin with `<!DOCTYPE html>`
        // (§13.2.6.4.1): EOF or any tag before a DOCTYPE is the
        // "missing-doctype" parse error — empty input and a bare
        // `<html>` are both rejected, not coerced into quirks mode.
        assert!(parse_strict("").is_err());
        assert!(parse_strict("<html></html>").is_err());
    }

    #[test]
    fn strict_success_contract() {
        // The `ParseResult` Ok-path contract: an empty `errors` list and
        // no detected encoding (`&str` input — no charset detection).
        let html = "<!DOCTYPE html><html><head></head><body></body></html>";
        let result = parse_strict(html).expect("valid HTML5");
        assert!(
            result.errors.is_empty(),
            "Ok path reports an empty error list"
        );
        assert!(
            result.encoding.is_none(),
            "strict mode never detects an encoding"
        );
    }
}
