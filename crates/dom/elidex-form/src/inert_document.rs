//! Inert-document construction — HTML §8.5.1 `DOMParser.parseFromString`
//! (the `text/html` path, which §8.5.1 routes through the §13.2 document
//! parser).
//!
//! [`parse_into_inert_document`] is the engine-independent home for the
//! whole "build an inert Document from a markup string" ALGORITHM, lifted
//! out of `elidex-js`'s `vm/host/dom_parser.rs` (CLAUDE.md Layering
//! mandate — `vm/host/` is marshalling-only; DOM-semantics algorithms,
//! including the *decision of which structural-fact reconcilers to re-run*,
//! live engine-indep). The VM's `parseFromString` native now reduces to:
//! brand-check, ToString the args, validate the MIME, resolve the caller
//! URL, call this primitive, wrap the returned entity.
//!
//! `elidex-form` is the correct crate for this: it is the lowest
//! engine-independent crate that can reach ALL three layers the build
//! touches — `elidex_script_session::apply_set_inner_html` (the §11.3
//! strict-first fragment parse), `elidex_dom_api::initialize_base_url_for_document`
//! (the §2.4.3 base-URL finalizer), and this crate's own
//! [`create_form_control_state`](crate::create_form_control_state) (the §4.10
//! form-control-state attach). The crate DAG is
//! `form → dom-api → script-session → ecs`, so none of these calls inverts
//! layering (script-session / dom-api do not depend on form — no cycle).

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_script_session::{apply_set_inner_html, SetInnerHtmlOptions};
use url::Url;

use crate::create_form_control_state;

/// HTML §8.5.1 — create an **inert** Document (no browsing context,
/// scripting disabled) from a `markup` string, finalized with the
/// structural-fact-derived state a real parse would attach, and return the
/// Document entity.
///
/// `caller_base_url` is the calling document's URL (HTML §8.5.1 step 2:
/// "Let document be a new Document … whose … URL is this's relevant global
/// object's associated Document's URL"), used as the base-URL fallback when
/// the parsed markup has no `<base href>`: a `<base>`-less DOMParser document
/// then resolves relative URLs against the CALLER's page, not `about:blank`.
///
/// The build runs against the `dom`'s existing world but does NOT clobber the
/// page's cached `document_root` (the throwaway document is a separate
/// `NodeKind::Document` entity from [`EcsDom::create_document_node`]). It is
/// performed with the live mutation dispatcher TAKEN OUT — see the inert
/// discipline below — so the construction fires no script-activation
/// reactions; the dispatcher is restored before returning, so the RETURNED
/// document is live (observers / mutations JS adds *after* `parseFromString`
/// still fire).
///
/// # Inert-document discipline (HTML §8.5.1, §13.2.4.5 scripting disabled)
///
/// A DOMParser document has no browsing context and scripting is disabled,
/// so its construction must fire NO mutation reactions. `apply_set_inner_html`'s
/// final append/remove run against a target rooted at a `NodeKind::Document`
/// (so `is_connected` is true) and would otherwise drive the live page
/// dispatcher that `Vm::bind` installs — firing Insert events, custom-element
/// reactions, and Range side effects for an inert document. This is
/// JS-OBSERVABLE: a custom element in the parsed markup whose name matches a
/// registered definition would be `try-to-upgrade`'d on the connected Insert
/// (constructor runs → connectedCallback fires) — exactly the inert-document
/// violation §8.5.1 forbids. So the dispatcher is taken out for the build
/// (mirroring `EcsDom::begin_detached_fragment`) and restored afterwards.
///
/// ## Structural-fact reconcilers re-run here vs. script-activation suppressed
///
/// The dispatcher suppression silences ALL seven mutation consumers
/// (`elidex_js::vm::consumer_dispatcher`) for the inert build. Those consumers
/// split into two classes, and only one class must be re-run document-scoped
/// here — this is the engine-indep home for that DOM-semantics decision (it
/// closes the "host owns the reconciler list" trap):
///
///   * **STRUCTURAL-FACT reconcilers** — DOM facts a real parse would set that
///     are INDEPENDENT of scripting (`.value`, `baseURI`). These MUST run even
///     for an inert §8.5.1 document. Exactly two:
///     `FormControlReconciler` (FCS attach) and `BaseUrlMaintainer`
///     (`<base href>` → `DocumentBaseUrl`). Both are re-run document-scoped
///     below.
///   * **SCRIPT-ACTIVATION reconcilers** — these run scripts / compile handlers
///     and MUST STAY suppressed for an inert document (no browsing context,
///     scripting disabled): `CustomElementReactionConsumer` (constructor +
///     connectedCallback upgrade) and `EventHandlerAttributeConsumer` (`on*`
///     content-attr handler processing — HTML §8.1.3.4 does NOT process
///     event-handler attrs when scripting is disabled, which a DOMParser
///     document is). Leaving both off is the CORRECT inert behavior, not a gap.
///
/// The remaining three consumers need NO init here: `LiveRangeBridge` /
/// `NodeIteratorAdjuster` track JS-allocated handles that can only exist
/// post-bind (nothing to seed for a fresh document), and `CanvasReconciler`
/// is AttributeChange-only with initial canvas state seeded at element
/// creation. So `{FCS, base-url}` is the COMPLETE structural-fact set for the
/// inert throwaway document — this CLOSES the reconciler class (no per-round
/// reconciler gap left).
///
/// # Deferred — persistent post-return inertness
///
/// The document is inert DURING the build, but the dispatcher is shared with
/// the page and restored on return, so LATER mutations on the returned
/// document run through the page's script-activation consumers. Keeping the
/// returned document permanently inert per §8.5.1 requires an engine-wide
/// inert-document scripting MARKER that every dispatcher consumer gates on —
/// a larger feature deferred to `#11-domparser-full-document-parse-fidelity`
/// (persistent inert-document state). This primitive keeps today's behavior
/// (inert during build only).
///
/// Likewise the fragment-parse-in-`<html>` approximation here is
/// boa-parity-bounded; a true §13.2 full-document parse (doctype + exact
/// html/head/body construction from arbitrary markup) is deferred to the same
/// slot.
#[must_use]
pub fn parse_into_inert_document(dom: &mut EcsDom, markup: &str, caller_base_url: &Url) -> Entity {
    // Take the live dispatcher out for the inert build, restore it after.
    let saved_dispatcher = dom.take_mutation_dispatcher();

    // Throwaway Document — does NOT clobber the page's cached `document_root`
    // (see `EcsDom::create_document_node`). A synthesized `<html>` becomes its
    // documentElement; the markup is fragment-parsed INTO `<html>` so html5ever
    // synthesizes `<head>`/`<body>` and `doc.body`/`doc.head` resolve on real
    // markup.
    let doc = dom.create_document_node();
    let html_el = dom.create_element_with_owner("html", Attributes::default(), Some(doc));
    let _ = dom.append_child(doc, html_el);

    // §11.3 strict-first fragment parse via the engine-indep
    // `apply_set_inner_html` seam (no `elidex_html_parser` call here directly).
    // `scripting_disabled: true` parses the inert (§13.2.4.5 scripting-disabled)
    // document so `<noscript>` content becomes real elements (not raw text).
    // The returned MutationRecord is intentionally dropped: a throwaway document
    // has no registered MutationObserver.
    //
    // BOUNDARY (`#11-domparser-full-document-parse-fidelity`):
    // `scripting_disabled` only takes effect where the FRAGMENT parse routes
    // `<noscript>` to a real-element arm. A BARE leading `<noscript>` routes via
    // the implied `<head>` into "in head noscript", where the first flow content
    // is a strict parse error → §11.3 tolerant html5ever fallback, whose
    // `parse_fragment` IGNORES `scripting_enabled` for `<noscript>`. So a bare
    // `<noscript>` stays RAWTEXT. The fix is a true §13.2 full-document parse,
    // deferred to that slot.
    let _ = apply_set_inner_html(
        dom,
        html_el,
        markup,
        SetInnerHtmlOptions {
            scripting_disabled: true,
            ..SetInnerHtmlOptions::default()
        },
    );

    // === Inert-document STRUCTURAL-FACT finalization (see fn docs) ===

    // --- FormControlReconciler (FCS) ---
    // Attach `FormControlState` to the parsed form controls, SUBTREE-SCOPED to
    // ONLY the throwaway document's descendants (NOT whole-dom
    // `init_form_controls`, which would clobber the shared page document's
    // form-control state — wiping every live page `<input>`/`<select>`/
    // `<textarea>`'s dirty-value flag / user-typed `.value` / checkedness).
    // `create_form_control_state` is a PURE component attach (NO custom-element
    // upgrade / NO script reactions), so doing it inside the suppressed window
    // preserves the inert guarantee. Two-phase (collect-then-mutate): the walker
    // borrows `&self` but `create_form_control_state` needs `&mut dom`, so the
    // entity ids are buffered first to avoid the borrow conflict.
    // Frontier = the document's descendants PLUS every `<template>`'s content
    // fragment (shared `collect_template_inclusive_descendants`): template
    // contents are a detached `DocumentFragment` (HTML §4.12.3) that
    // `for_each_shadow_inclusive_descendant` alone misses, so a control inside a
    // `<template>` would otherwise never receive `FormControlState` (Codex R5).
    // Subtree-scoped (NOT whole-world `init_form_controls`) to avoid clobbering
    // the shared page document.
    for entity in crate::init::collect_template_inclusive_descendants(dom, doc) {
        let _ = create_form_control_state(dom, entity);
    }

    // --- BaseUrlMaintainer (base URL) ---
    // Derive the throwaway document's `DocumentBaseUrl` from any parsed
    // `<base href>` (HTML §2.4.3), document-scoped to `doc`, via the engine-indep
    // base-url finalizer shared with the bind path (one-issue-one-way). The
    // `caller_base_url` fallback (§8.5.1 step 2) is used both to resolve a
    // relative `<base href>` and as the document base URL when there is no
    // `<base href>` at all — so a `<base>`-less parsed document resolves relative
    // URLs against the CALLER's page, not `about:blank`. Like FCS this is a PURE
    // component attach (NO script reactions), safe inside the suppressed window.
    //
    // BOUNDARY (`#11-document-url-real-navigation`): `caller_base_url` is a
    // one-shot SEED for `DocumentBaseUrl` only — it is NOT stored as this
    // document's URL fact. The OTHER base-URL consumers that fall back to a
    // document URL therefore still see `about:blank` for this throwaway
    // document: the `<base>.href` IDL getter (`href_accessor::effective_base_url`)
    // and any POST-return `<base>` insert/remove/attribute reconcile (the live
    // `BaseUrlMaintainer` arms all fall back to `about_blank_url()`). So
    // §8.5.1 step 2 ("the new Document's URL is the caller's") is applied to
    // `baseURI` but only PARTIALLY to those derived paths. The full fix is the
    // engine-wide document-URL-as-stored-fact graduation — shared with the page
    // document (which carries the same `about:blank` stub) and owned by that
    // slot, NOT this DOMParser slice (a DOMParser-only `DocumentUrl` component
    // would be a strangler half-migration). VM ≥ boa is preserved (boa never
    // stored a document URL either). Codex R4-F1.
    elidex_dom_api::initialize_base_url_for_document(dom, doc, caller_base_url);

    if let Some(dispatcher) = saved_dispatcher {
        dom.set_mutation_dispatcher(dispatcher);
    }
    doc
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_dom_api::element::document_base::document_base_url;

    /// A `<base href>`-less inert document adopts the CALLER URL as its base
    /// (HTML §8.5.1 step 2), NOT `about:blank` — the R3-F1 fix.
    #[test]
    fn parse_into_inert_document_uses_caller_url_as_base_fallback() {
        let mut dom = EcsDom::new();
        let _page = dom.create_document_root();
        let caller = Url::parse("https://example.com/dir/page.html").unwrap();

        let doc = parse_into_inert_document(&mut dom, "<a href=\"p.html\">x</a>", &caller);

        // No <base> in the markup → the document's base URL is the caller URL.
        assert_eq!(document_base_url(&dom, doc), caller);
    }

    /// A parsed `<base href>` overrides the caller-URL fallback (HTML §2.4.3
    /// first-base rule), and a relative `<base href>` resolves against the
    /// caller URL.
    #[test]
    fn parse_into_inert_document_honors_parsed_base_href() {
        let mut dom = EcsDom::new();
        let _page = dom.create_document_root();
        let caller = Url::parse("https://example.com/dir/page.html").unwrap();

        let doc = parse_into_inert_document(
            &mut dom,
            "<head><base href=\"https://other.example/sub/\"></head><body></body>",
            &caller,
        );
        assert_eq!(
            document_base_url(&dom, doc).as_str(),
            "https://other.example/sub/"
        );
    }

    /// The returned entity is a distinct Document, not the page's
    /// `document_root` (does not clobber it).
    #[test]
    fn parse_into_inert_document_does_not_clobber_page_root() {
        let mut dom = EcsDom::new();
        let page = dom.create_document_root();
        let caller = Url::parse("https://example.com/").unwrap();

        let doc = parse_into_inert_document(&mut dom, "<div></div>", &caller);

        assert_ne!(doc, page);
        assert_eq!(dom.document_root(), Some(page));
    }

    /// A parsed form control gets `FormControlState` attached (its `.value`
    /// reflects the `value` attribute) — the structural-fact FCS reconciler
    /// ran document-scoped.
    #[test]
    fn parse_into_inert_document_attaches_form_control_state() {
        use crate::FormControlState;
        let mut dom = EcsDom::new();
        let _page = dom.create_document_root();
        let caller = Url::parse("https://example.com/").unwrap();

        let doc = parse_into_inert_document(&mut dom, "<input value=x>", &caller);

        // Locate the parsed <input> and confirm it carries FormControlState.
        let mut found = None;
        dom.for_each_shadow_inclusive_descendant(doc, &mut |e| {
            if dom
                .world()
                .get::<&elidex_ecs::TagType>(e)
                .is_ok_and(|t| t.0 == "input")
            {
                found = Some(e);
            }
        });
        let input = found.expect("parsed <input> present");
        let fcs = dom
            .world()
            .get::<&FormControlState>(input)
            .expect("FormControlState attached to parsed input");
        assert_eq!(fcs.value(), "x");
    }
}
