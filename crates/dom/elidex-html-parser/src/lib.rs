//! HTML parser for elidex.
//!
//! Parses HTML5 documents into an ECS-backed DOM tree. Conforming documents
//! take the strict parser (design doc §11.3 Tier-1); html5ever is the tolerant
//! fallback (Tier-2) and the backend for the bare string/fragment entry points.
//!
//! Entry points:
//! - [`parse_html`] — UTF-8 string input, tolerant html5ever (§11.3 Tier-2).
//! - [`parse_progressive`] — raw byte input: charset auto-detection + §11.3
//!   strict-first dispatch (Tier-1) with a tolerant fallback (Tier-2).
//! - [`parse_strict`] — strict mode that rejects documents with parse errors.

pub mod charset;
mod convert;
mod element_init;

pub use charset::{detect_and_decode, DecodeResult, EncodingConfidence};
// `ParseResult`, `ParseTier`, `ParseFragmentOptions`, `StrictParseError`,
// and `parse_fragment_strict` are owned by the engine-independent
// `elidex-html-parser-strict` crate (the strict-mode SoT). They are
// re-exported here so existing `elidex_html_parser::…` import paths keep
// working; the tolerant html5ever entry points below produce the same
// `ParseResult` type (tagged `ParseTier::Recovered`).
//
// `parse_strict` is intentionally NOT re-exported: this crate defines its
// own [`parse_strict`] wrapper (below) that runs the derived-component pass,
// so every public document entry point (`parse_html` / `parse_progressive` /
// `parse_strict`) yields a complete `ParseResult`. `parse_fragment_strict`
// stays the raw re-export — the fragment derived-component pass lives in the
// [`parse_fragment_progressive`] dispatcher (§11.3 slice 2b), which every
// fragment caller routes through, so the raw strict entry never reaches a
// caller un-derived.
pub use elidex_html_parser_strict::{
    parse_fragment_strict, ParseFragmentOptions, ParseResult, ParseTier, StrictParseError,
};

use convert::convert_document;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use html5ever::ParseOpts;
use markup5ever_rcdom::RcDom;

/// Parse an HTML5 document from a UTF-8 string.
///
/// Uses html5ever for spec-compliant parsing with full error recovery.
/// Parse warnings are collected in [`ParseResult::errors`].
#[must_use]
pub fn parse_html(html: &str) -> ParseResult {
    let rc_dom = parse_document(RcDom::default(), ParseOpts::default()).one(html);
    convert_document(rc_dom)
}

/// Parse a conforming HTML5 document with the strict (Tier-1) backend.
///
/// Wrapper over [`elidex_html_parser_strict::parse_strict`] that runs the
/// canonical derived-component pass (`element_init`) on success, so the
/// public strict entry point produces the same complete [`ParseResult`] —
/// `CustomElementState` (HTML §4.13.3) / `IframeData`
/// (§4.8.5) attached — as [`parse_html`] and [`parse_progressive`]. The
/// strict tree-builder crate itself stays DOM-semantics-free (no
/// `elidex-custom-elements` dep); the derivation lives one layer up here.
///
/// # Errors
/// Propagates the [`StrictParseError`] from the strict backend when the
/// input is not conforming HTML5 (callers fall back to [`parse_html`]).
pub fn parse_strict(html: &str) -> Result<ParseResult, StrictParseError> {
    let mut result = elidex_html_parser_strict::parse_strict(html)?;
    element_init::derive_element_components(&mut result.dom, result.document);
    Ok(result)
}

/// Parse an HTML fragment string into **detached** child nodes — the tolerant
/// (§11.3 Tier-2) backend for the WHATWG HTML §13.4 fragment-parsing algorithm.
///
/// `context` is the fragment-context element; its tag and namespace select
/// html5ever's parsing rules (§13.4: the new HTML parser's fragment context
/// element, and the tokenization stage state switched on it). Returns the fragment's
/// top-level nodes **detached** (parentless) in tree order (§13.4: return fragment),
/// with their node document set to `context`'s owner document. `context` is
/// **not** mutated; the caller places the returned nodes.
///
/// Structurally mirrors the strict backend's `parse_fragment_strict`: build
/// under a synthetic throwaway document + `<html>` root, then return root's
/// children via the shared [`elidex_ecs::EcsDom::finish_detached_fragment`]
/// teardown — so a strict-first dispatcher ([`parse_fragment_progressive`])
/// routes either backend through one detached-return contract
/// (One-issue-one-way).
pub fn parse_html_fragment(
    html: &str,
    context: elidex_ecs::Entity,
    dom: &mut elidex_ecs::EcsDom,
    opts: ParseFragmentOptions,
) -> Vec<elidex_ecs::Entity> {
    use html5ever::{ns, LocalName, QualName};

    // §13.4 (fragment context element): derive the context QualName (local name + namespace)
    // from the context entity. The unified detached-return signature carries
    // the entity, so the namespace is *derived*, not hardcoded to HTML — this
    // eliminates the old HTML-ns fork (One-issue-one-way). A context with no
    // `TagType` falls back to "div" (HTML §13.4 "Any other element" generic
    // context, the safe default).
    let local = dom.with_tag_name(context, |t| t.unwrap_or("div").to_owned());
    let context_ns = match dom.namespace_of(context) {
        elidex_ecs::Namespace::Svg => ns!(svg),
        elidex_ecs::Namespace::MathMl => ns!(mathml),
        elidex_ecs::Namespace::Html => ns!(html),
    };
    let context_name = QualName::new(None, context_ns, LocalName::from(local.as_str()));
    let rc_dom = html5ever::parse_fragment(
        RcDom::default(),
        ParseOpts::default(),
        context_name,
        Vec::new(),
        // §13.2.4.5 scripting flag: enabled by default (scriptingMode ≠
        // Disabled), DISABLED for an inert parse (`DOMParser`) so `<noscript>`
        // content parses as ordinary elements.
        !opts.scripting_disabled,
    )
    .one(html);

    // html5ever wraps the fragment children under a single synthetic `<html>`
    // element; unwrap to the handle whose children are the real fragment roots
    // (else use the document handle directly).
    let children_owner = {
        let roots = rc_dom.document.children.borrow();
        match roots.as_slice() {
            [only] => match &only.data {
                markup5ever_rcdom::NodeData::Element { name, .. }
                    if name.local.as_ref() == "html" =>
                {
                    only.clone()
                }
                _ => rc_dom.document.clone(),
            },
            _ => rc_dom.document.clone(),
        }
    };

    // Build the converted subtree under a synthetic throwaway document +
    // `<html>` root (the shared prologue both backends use; mutation dispatch is
    // suppressed for the throwaway build, restored after teardown), then return
    // root's children detached via the shared teardown.
    let (document, root, saved) = dom.begin_detached_fragment();
    // The template start tag (§13.2.6.4.4) takes declarativeShadowHostElement to
    // be the current node, not the adjusted current node, and hostIsRoot (the
    // fragment root also has a redirection-map entry) makes a top-level
    // `<template shadowrootmode>` a regular template: `context` is never mutated.
    // `convert_fragment_top_level` therefore never attaches; nested templates
    // attach to their real parent. Stamp every parsed `<template>` content
    // fragment with the **context's** node document, not the throwaway
    // fragment `document`: a content fragment is parentless, so
    // `finish_detached_fragment` does not adopt it — stamping the throwaway doc would leave that content's owner dangling once
    // the throwaway document is destroyed (Codex PR380 R2). `None` (documentless
    // context) leaves the fragments owner-less, matching the orphan-detached
    // light nodes `finish_detached_fragment` returns then.
    let owner_doc = dom.owner_document(context);
    convert::convert_fragment_top_level(&children_owner, root, dom, owner_doc, opts);
    let detached = dom.finish_detached_fragment(root, document, context);
    if let Some(dispatcher) = saved {
        dom.set_mutation_dispatcher(dispatcher);
    }
    detached
}

/// Parse an already-decoded HTML5 string with `§11.3` progressive
/// degradation — the `&str` twin of [`parse_progressive`].
///
/// Tries the strict (Tier-1) parser first: conforming HTML5 takes the fast
/// strict path ([`ParseTier::Clean`]). On the first strict parse error (a
/// WHATWG HTML `§13.2.2` parse error) it falls back to the tolerant
/// html5ever backend (Tier-2 rule-based recovery, [`ParseTier::Recovered`])
/// over the *same* text. This is the single try-strict-or-tolerant decision
/// site; [`parse_progressive`] (byte input) wraps it with charset detection.
///
/// Use this for whole-document input that is **already a `&str`** (no charset
/// step to run): the in-process shell pipeline (blank-tab markup, iframe
/// `srcdoc`, `about:blank`, tests). `encoding` stays `None` — the bare-`&str`
/// contract, matching [`parse_strict`] / [`parse_html`] (see
/// [`ParseResult::encoding`]).
///
/// The fallback is correctness-safe: the strict parser rejects
/// conservatively (no error recovery), so the worst case is the same tree the
/// tolerant backend would have produced on its own. Both arms attach the
/// parser-owned derived components (the local [`parse_strict`] wrapper runs
/// `derive_element_components` on success; the tolerant fallback derives inline
/// via `convert_node`), so the returned tree carries the CustomElementState /
/// IframeData components either way.
#[must_use]
pub fn parse_progressive_str(html: &str) -> ParseResult {
    parse_strict(html).unwrap_or_else(|_| parse_html(html))
}

/// Parse an HTML5 document from raw bytes with `§11.3` progressive
/// degradation (browser mode).
///
/// Detects the character encoding once, up front, then runs the str-input
/// strict-first dispatch [`parse_progressive_str`] over the decoded text (no
/// re-decode) and stamps the detected encoding. `charset_hint` is the HTTP
/// `Content-Type` header's `charset` parameter, if available. The resulting
/// [`ParseResult::tier`] records which tier ran, making the strict-vs-fallback
/// gradient observable (`§11.3`).
#[must_use]
pub fn parse_progressive(bytes: &[u8], charset_hint: Option<&str>) -> ParseResult {
    let decoded = detect_and_decode(bytes, charset_hint);
    let mut result = parse_progressive_str(&decoded.text);
    result.encoding = Some(decoded.encoding);
    result
}

/// Parse an HTML fragment with `§11.3` progressive degradation — the fragment
/// twin of [`parse_progressive`], and the single entry point every fragment
/// caller (innerHTML / outerHTML / insertAdjacentHTML) routes through.
///
/// Tries the strict (Tier-1) fragment parser first; conforming fragments take
/// the fast strict path. On the first WHATWG HTML `§13.2.2` parse error it
/// falls back to the tolerant html5ever backend ([`parse_html_fragment`]) over
/// an **uncontaminated** `dom` (the strict parser rolls back its partial
/// subtree on reject, leaving `dom` pristine — `parse_fragment_strict`'s
/// contract). Both backends honour the identical detached-return contract: the
/// fragment's top-level nodes come back parentless, adopted into `context`'s
/// owner document, with `context` itself unmodified — the caller places them.
///
/// `opts.allow_declarative_shadow` selects `innerHTML` (false) vs
/// `setHTMLUnsafe` (true) semantics.
#[must_use]
pub fn parse_fragment_progressive(
    html: &str,
    context: elidex_ecs::Entity,
    dom: &mut elidex_ecs::EcsDom,
    opts: ParseFragmentOptions,
) -> Vec<elidex_ecs::Entity> {
    match parse_fragment_strict(html, context, dom, opts) {
        Ok(roots) => {
            // The strict tree-builder crate is DOM-semantics-free, so the
            // parser-owned derived components (CustomElementState §4.13.3 /
            // IframeData §4.8.5) are attached here — the same
            // `derive_element_components` pass `parse_strict` runs for whole
            // documents (the tolerant backend derives inline via
            // `convert_node`). Each returned root is its own detached subtree,
            // so derive per root (the walker is root-inclusive).
            for &root in &roots {
                element_init::derive_element_components(dom, root);
            }
            roots
        }
        // Tier-2 fallback over the pristine dom; the tolerant path derives
        // inline during conversion.
        Err(_) => parse_html_fragment(html, context, dom, opts),
    }
}

#[cfg(test)]
pub(crate) mod test_helpers;

#[cfg(test)]
mod tests;
