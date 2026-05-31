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
//!   modes (error branches excluded by the strict no-recovery contract)
//!   + HTML `§4.12.3` / DOM `§4.9` declarative shadow root attach.
//! - `§13.2.7` The end — stop parsing (the deferred / async script
//!   timing steps 5 / 7 are the consumer's concern, not the parser's).
//!
//! Foreign content (`§13.2.6.5` SVG / MathML inline) is out of scope; it
//! is tracked separately (`#11-html-parser-strict-foreign-content`).
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
pub use result::{ParseFragmentOptions, ParseResult};

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
    use elidex_ecs::{EcsDom, Entity, TagType, TextContent};

    /// Recursively find the first element with the given tag.
    fn find_tag(dom: &EcsDom, parent: Entity, tag: &str) -> Option<Entity> {
        for child in dom.children_iter(parent) {
            if let Ok(t) = dom.world().get::<&TagType>(child) {
                if t.0 == tag {
                    return Some(child);
                }
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
    fn strict_valid_html() {
        let html = "<!DOCTYPE html><html><head></head><body><p>Hello</p></body></html>";
        let result = parse_strict(html);
        assert!(result.is_ok());
    }

    #[test]
    fn strict_invalid_html() {
        // Mismatched tags: `</div>` closes while `<span>` is the current
        // node — a §13.2.6.4.7 "any other end tag" parse error that strict
        // mode rejects (no recovery).
        let html = "<!DOCTYPE html><html><head></head><body><div><span></div></body></html>";
        let result = parse_strict(html);
        assert!(result.is_err());
    }

    #[test]
    fn strict_error_messages() {
        let html = "<!DOCTYPE html><html><head></head><body><div><span></div></body></html>";
        let err = parse_strict(html).unwrap_err();
        assert!(!err.errors.is_empty());
        for msg in &err.errors {
            assert!(!msg.is_empty());
        }
    }

    #[test]
    fn strict_encoding_is_none() {
        let html = "<!DOCTYPE html><html></html>";
        let result = parse_strict(html).expect("valid HTML5");
        assert!(result.encoding.is_none());
    }

    #[test]
    fn strict_success_has_no_errors() {
        let html = "<!DOCTYPE html><html><head></head><body></body></html>";
        let result = parse_strict(html).expect("valid HTML5");
        assert!(
            result.errors.is_empty(),
            "strict success must report an empty error list"
        );
    }

    #[test]
    fn strict_builds_head_and_body() {
        let html = "<!DOCTYPE html><html><head><title>T</title></head><body><p>x</p></body></html>";
        let result = parse_strict(html).expect("valid HTML5");
        let head = find_tag(&result.dom, result.document, "head").expect("head");
        let body = find_tag(&result.dom, result.document, "body").expect("body");
        assert!(
            find_tag(&result.dom, head, "title").is_some(),
            "title under head"
        );
        assert!(find_tag(&result.dom, body, "p").is_some(), "p under body");
    }

    #[test]
    fn strict_text_content_coalesces() {
        let html = "<!DOCTYPE html><html><head></head><body><p>Hello world</p></body></html>";
        let result = parse_strict(html).expect("valid HTML5");
        let p = find_tag(&result.dom, result.document, "p").expect("p");
        assert_eq!(child_text(&result.dom, p), "Hello world");
    }

    #[test]
    fn strict_implied_head_body() {
        // No explicit <head>/<body>: the tree builder inserts them per
        // §13.2.6.4 (before head → in head → after head → in body).
        let html = "<!DOCTYPE html><html><p>x</p></html>";
        let result = parse_strict(html).expect("valid HTML5");
        assert!(find_tag(&result.dom, result.document, "head").is_some());
        let body = find_tag(&result.dom, result.document, "body").expect("body");
        assert!(find_tag(&result.dom, body, "p").is_some());
    }

    #[test]
    fn strict_nested_elements() {
        let html =
            "<!DOCTYPE html><html><head></head><body><div><span>x</span></div></body></html>";
        let result = parse_strict(html).expect("valid HTML5");
        let div = find_tag(&result.dom, result.document, "div").expect("div");
        let span = find_tag(&result.dom, div, "span").expect("span nested in div");
        assert_eq!(child_text(&result.dom, span), "x");
    }

    #[test]
    fn strict_comment_node() {
        let html = "<!DOCTYPE html><html><head></head><body><!-- note --><p>x</p></body></html>";
        let result = parse_strict(html);
        assert!(result.is_ok(), "comments are valid HTML5");
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
    fn strict_void_element() {
        let html = "<!DOCTYPE html><html><head></head><body><br></body></html>";
        let result = parse_strict(html);
        assert!(result.is_ok(), "void elements are valid HTML5");
    }
}
