//! WHATWG HTML §13.4 strict fragment-parsing tests.
//!
//! These pin the fragment-specific invariants the corpus harness cannot
//! express directly: the detached-return contract (step 20), parse-error
//! rollback isolation, coalescing isolation, the context-driven insertion
//! mode (step 16) / tokenizer state (step 10) / form pointer (step 17), and
//! the non-element-context guard. Conformant tree shapes are also
//! cross-checked against the html5lib `#document` serialization (the
//! corpus-driven cases live in `tests_html5lib_tree`).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use elidex_ecs::{Attributes, EcsDom, Entity, MutationDispatcher, MutationEvent, Namespace};

use super::tests::serialize_fragment as serialize_roots;
use crate::{parse_fragment_strict, ParseFragmentOptions};

/// Parse `html` in a fresh context element named `context_tag`, returning the
/// dom and the detached fragment roots. Panics if strict rejects the input.
fn frag(context_tag: &str, html: &str) -> (EcsDom, Vec<Entity>) {
    let mut dom = EcsDom::new();
    let ctx = dom.create_element(context_tag, Attributes::default());
    let roots = parse_fragment_strict(html, ctx, &mut dom, ParseFragmentOptions::default())
        .expect("valid fragment should parse");
    (dom, roots)
}

// ----- context-driven insertion mode (§13.4 step 16) -----

#[test]
fn div_context_parses_in_body() {
    let (dom, roots) = frag("div", "<p>a</p><p>b</p>");
    assert_eq!(
        serialize_roots(&dom, &roots),
        "\
| <p>
|   \"a\"
| <p>
|   \"b\"
"
    );
}

#[test]
fn table_context_implies_tbody() {
    // §13.4 step 16: a `table` context resets to "in table", so `<tr>` implies
    // a `<tbody>` exactly as in a real table.
    let (dom, roots) = frag("table", "<tr><td>x</td></tr>");
    assert_eq!(
        serialize_roots(&dom, &roots),
        "\
| <tbody>
|   <tr>
|     <td>
|       \"x\"
"
    );
}

#[test]
fn tr_context_resets_to_in_row() {
    // A `tr` context resets to "in row": a `<td>` lands directly, with no
    // implied table / tbody / tr wrapper.
    let (dom, roots) = frag("tr", "<td>x</td>");
    assert_eq!(
        serialize_roots(&dom, &roots),
        "\
| <td>
|   \"x\"
"
    );
}

#[test]
fn td_context_resets_to_in_body() {
    // The fragment-case core (§13.2.4.1 step 4 "td/th && last is false"): a
    // `td` context is itself the last stack node, so the cell check is skipped
    // and the reset falls through to "in body" — flow content lands directly.
    let (dom, roots) = frag("td", "<p>x</p>");
    assert_eq!(
        serialize_roots(&dom, &roots),
        "\
| <p>
|   \"x\"
"
    );
}

#[test]
fn select_context_retains_input_per_customizable_select() {
    // Current WHATWG HTML removed the "in select" / "in select in table"
    // insertion modes (customizable-`<select>`): the §13.2.6.4 insertion-mode
    // list (§13.2.6.4.1–.21, verified via webref — §13.2.6.4.7 is "in body",
    // there is no "in select") no longer special-cases `select`, so a
    // `select`-context fragment parses through "in body" and an `<input>` is
    // *retained* with no parse error. The html5lib snapshot that drops the
    // `<input>` and flags a parse error predates the change (see the
    // `known_spec_divergence` skip in `tests_html5lib_tree`); this pins that
    // strict matches current spec, so that corpus skip masks no real failure.
    // `frag` panics on rejection, so reaching the asserts also proves strict
    // returns Ok (does not treat `<input>` as a fatal parse error).
    let (dom, roots) = frag("select", "<input><option>");
    assert!(
        roots.iter().any(|&r| dom.has_tag(r, "input")),
        "current-spec strict retains <input> in a select context"
    );
    assert!(
        roots.iter().any(|&r| dom.has_tag(r, "option")),
        "the <option> is retained too"
    );
}

// ----- tokenizer initial state from context (§13.4 step 10) -----

#[test]
fn title_context_is_rcdata() {
    // RCDATA: markup is literal text, no appropriate end tag in the fragment
    // case (so `</title>` would also be text — exercised by `runs_to_eof`).
    let (dom, roots) = frag("title", "<b>x</b>");
    assert_eq!(roots.len(), 1);
    assert_eq!(serialize_roots(&dom, &roots), "| \"<b>x</b>\"\n");
}

#[test]
fn rcdata_context_runs_to_eof_without_appropriate_end_tag() {
    // §13.4 note: no last start tag is recorded, so even a matching end tag is
    // not an appropriate end tag — the whole input stays RCDATA text.
    let (dom, roots) = frag("textarea", "a</textarea>b");
    assert_eq!(serialize_roots(&dom, &roots), "| \"a</textarea>b\"\n");
}

#[test]
fn script_context_is_script_data() {
    let (dom, roots) = frag("script", "x<y</script>z");
    assert_eq!(serialize_roots(&dom, &roots), "| \"x<y</script>z\"\n");
}

#[test]
fn style_context_is_rawtext() {
    let (dom, roots) = frag("style", "a<b>c");
    assert_eq!(serialize_roots(&dom, &roots), "| \"a<b>c\"\n");
}

// ----- detached-return contract (§13.4 step 20) -----

#[test]
fn fragment_roots_are_detached() {
    let (dom, roots) = frag("div", "<p></p><span></span>");
    assert_eq!(roots.len(), 2);
    for &root in &roots {
        assert_eq!(
            dom.get_parent(root),
            None,
            "step 20 returns root's children detached; the caller places them"
        );
    }
    // No synthetic `<html>` root survives in the dom.
    assert!(
        !dom.root_entities().iter().any(|&e| dom.has_tag(e, "html")),
        "synthetic root is despawned after detaching its children"
    );
}

#[test]
fn detached_return_does_not_coalesce_with_context_text() {
    // The fragment parses under a synthetic root, isolated from the context's
    // existing children — a fragment-leading text node is a *separate* node,
    // never coalesced into the context's trailing text (Approach A hazard).
    let mut dom = EcsDom::new();
    let ctx = dom.create_element("div", Attributes::default());
    let pre = dom.create_text("pre");
    assert!(dom.append_child(ctx, pre));
    let roots = parse_fragment_strict("x", ctx, &mut dom, ParseFragmentOptions::default()).unwrap();
    assert_eq!(roots.len(), 1);
    assert_ne!(roots[0], pre, "fragment text is a distinct node");
    assert_eq!(dom.get_parent(roots[0]), None, "fragment root is detached");
    assert_eq!(
        dom.children(ctx),
        vec![pre],
        "context's existing children are untouched"
    );
}

// ----- parse-error rollback isolation (the dispatch precondition) -----

#[test]
fn parse_error_rolls_back_with_no_leak_and_pristine_context() {
    // `</div>` closes while `<span>` is current = a §13.2.6.4.7 parse error
    // strict rejects. The partial subtree + synthetic root must be torn down
    // so a strict-then-tolerant dispatcher falls back over a pristine dom.
    let mut dom = EcsDom::new();
    let ctx = dom.create_element("div", Attributes::default());
    let live_before = dom.world().len();
    let ctx_children_before = dom.children(ctx).len();

    let result = parse_fragment_strict(
        "<div><span></div>",
        ctx,
        &mut dom,
        ParseFragmentOptions::default(),
    );

    assert!(result.is_err(), "strict rejects the misnested end tag");
    assert_eq!(
        dom.world().len(),
        live_before,
        "no leaked entities on the rollback path (synthetic root + partial subtree despawned)"
    );
    assert_eq!(
        dom.children(ctx).len(),
        ctx_children_before,
        "the context element is never mutated"
    );
}

// ----- form element pointer from context ancestors (§13.4 step 17) -----

#[test]
fn form_pointer_from_ancestor_rejects_nested_form() {
    // A `<div>` context nested inside a `<form>`: step 17 sets the form
    // element pointer to the ancestor form, so a `<form>` start tag in the
    // fragment is the "misnested-form" parse error.
    let mut dom = EcsDom::new();
    let form = dom.create_element("form", Attributes::default());
    let ctx = dom.create_element("div", Attributes::default());
    assert!(dom.append_child(form, ctx));
    let result = parse_fragment_strict(
        "<form></form>",
        ctx,
        &mut dom,
        ParseFragmentOptions::default(),
    );
    assert!(
        result.is_err(),
        "ancestor form ⇒ form pointer set ⇒ nested form rejected"
    );
}

#[test]
fn no_form_ancestor_allows_form() {
    // Same input, no ancestor form: the pointer stays null and the `<form>`
    // is accepted — proves the rejection above is the pointer, not the markup.
    let (dom, roots) = frag("div", "<form></form>");
    assert_eq!(roots.len(), 1);
    assert!(dom.has_tag(roots[0], "form"));
}

// ----- non-element context guard -----

#[test]
fn non_element_context_falls_back_to_in_body() {
    // A caller may hand a non-element entity (e.g. a text node) as context.
    // Reading its tag yields `None` → Data tokenizer state + "in body" reset,
    // and the parse must not panic on the absent `TagType`.
    let mut dom = EcsDom::new();
    let ctx = dom.create_text("not an element");
    let roots =
        parse_fragment_strict("<p>x</p>", ctx, &mut dom, ParseFragmentOptions::default()).unwrap();
    assert_eq!(roots.len(), 1);
    assert!(dom.has_tag(roots[0], "p"));
}

// ----- declarative shadow opt threading (§13.4 step 6) -----

#[test]
fn declarative_shadow_opt_off_keeps_template_element() {
    // With the flag off, a `<template shadowrootmode>` stays an ordinary
    // template element in the light tree (no shadow attach).
    let (dom, roots) = frag(
        "div",
        "<template shadowrootmode=\"open\"><p>s</p></template>",
    );
    assert_eq!(roots.len(), 1);
    assert!(
        dom.has_tag(roots[0], "template"),
        "flag off ⇒ template is a plain element"
    );
}

#[test]
fn declarative_shadow_opt_on_attaches_shadow_root() {
    // With the flag on, the `<template shadowrootmode=open>` is consumed: the
    // host (the context `<div>`)… but in fragment parsing the host is the
    // synthetic root's child. Here the template is a child of the fragment's
    // first element, so attach it to that element. Use a wrapping `<section>`
    // so the host is a returned fragment node we can inspect.
    let mut dom = EcsDom::new();
    let ctx = dom.create_element("div", Attributes::default());
    let opts = ParseFragmentOptions {
        allow_declarative_shadow: true,
    };
    let roots = parse_fragment_strict(
        "<section><template shadowrootmode=\"open\"><p>s</p></template></section>",
        ctx,
        &mut dom,
        opts,
    )
    .unwrap();
    assert_eq!(roots.len(), 1);
    let section = roots[0];
    assert!(dom.has_tag(section, "section"));
    assert!(
        dom.get_shadow_root(section).is_some(),
        "flag on ⇒ declarative shadow attached to the host element"
    );
    assert!(
        dom.children(section).is_empty(),
        "the <template> is consumed, not left in the light tree"
    );
}

// ----- foreign content gets a valid owner document (§13.2.6.1) -----

#[test]
fn foreign_element_in_html_context_has_document_owner() {
    // An SVG element inside an HTML-namespace context fragment is created via
    // `create_element_ns` with the throwaway Document as owner. If the fragment
    // root were a bare element rather than a Document, `create_element_ns`
    // would assert (debug) / mis-stamp the owner document (release). This must
    // parse without panicking and yield the foreign element.
    let mut dom = EcsDom::new();
    let ctx = dom.create_element("div", Attributes::default());
    let roots = parse_fragment_strict(
        "<svg></svg>",
        ctx,
        &mut dom,
        ParseFragmentOptions::default(),
    )
    .unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(
        dom.namespace_of(roots[0]),
        elidex_ecs::Namespace::Svg,
        "the <svg> is a foreign (SVG-namespace) element"
    );
}

// ----- the throwaway document does not clobber the caller's document_root -----

#[test]
fn fragment_parse_preserves_caller_document_root() {
    // The §13.4 throwaway document is created cache-free
    // (`create_document_node`), so a fragment parse over a live dom must leave
    // the caller's cached `document_root` intact — not overwrite it with the
    // throwaway and then leave it dangling after despawn.
    let mut dom = EcsDom::new();
    let real_doc = dom.create_document_root();
    let ctx = dom.create_element("div", Attributes::default());
    assert!(dom.append_child(real_doc, ctx));

    let _roots =
        parse_fragment_strict("<p>x</p>", ctx, &mut dom, ParseFragmentOptions::default()).unwrap();

    assert_eq!(
        dom.document_root(),
        Some(real_doc),
        "the caller's cached document_root is preserved across a fragment parse"
    );
    assert!(
        dom.contains(real_doc),
        "the caller's real document is not despawned"
    );
}

// ----- rollback despawns a shadow host's shadow root (no leak) -----

#[test]
fn rollback_despawns_shadow_root_with_no_leak() {
    // A fragment that attaches a declarative shadow root and THEN hits a parse
    // error must tear the whole partial subtree down — including the shadow
    // root entity (which `for_each_shadow_inclusive_descendant` does not visit
    // directly). `world().len()` returning to its pre-parse value proves the
    // shadow root did not leak.
    let mut dom = EcsDom::new();
    let ctx = dom.create_element("div", Attributes::default());
    let opts = ParseFragmentOptions {
        allow_declarative_shadow: true,
    };
    let live_before = dom.world().len();

    // `<section>` attaches a shadow; the trailing `</div>` over an open
    // `<span>` is the parse error that triggers rollback.
    let result = parse_fragment_strict(
        "<section><template shadowrootmode=\"open\"><p>s</p></template></section><div><span></div>",
        ctx,
        &mut dom,
        opts,
    );

    assert!(result.is_err(), "the misnested </div> is rejected");
    assert_eq!(
        dom.world().len(),
        live_before,
        "rollback despawns the whole subtree incl. the shadow root — no leak"
    );
}

// ----- a non-HTML-namespace context element is declined (not silently
// HTML-namespaced), so a strict-first dispatcher can fall back -----

#[test]
fn foreign_namespace_context_is_rejected_leaving_dom_pristine() {
    // An SVG / MathML host's innerHTML needs the foreign-content initial
    // conditions slice 2a does not implement
    // (`#11-strict-fragment-foreign-context`). Rather than route the fragment
    // through HTML insertion and return a silently HTML-namespaced tree the
    // caller cannot tell is wrong, strict aborts so the caller falls back to
    // the tolerant backend — over a dom untouched by any synthetic node.
    let mut dom = EcsDom::new();
    let svg_ctx = dom.create_element_ns("svg", Namespace::Svg, Attributes::default(), None);
    let live_before = dom.world().len();

    let result = parse_fragment_strict(
        "<circle></circle>",
        svg_ctx,
        &mut dom,
        ParseFragmentOptions::default(),
    );

    assert!(result.is_err(), "a foreign-namespace context is declined");
    assert_eq!(
        dom.world().len(),
        live_before,
        "decline creates no synthetic node — dom is pristine for fallback"
    );
}

// ----- a top-level declarative-shadow template (host = external context) is
// declined; a nested one (host = in-fragment element) still attaches -----

#[test]
fn top_level_declarative_shadow_on_context_is_rejected() {
    // §13.2.4.2: with only the synthetic root on the stack the adjusted
    // current node is the *external* context. Step 10.1 would attach a shadow
    // root to it, mutating the caller's context — forbidden by 2a's
    // read-only-context isolation (rollback could not undo it). Strict aborts
    // so the caller falls back; faithful DSD-on-context is
    // `#11-strict-fragment-declarative-shadow-on-context` (slice 2b
    // `setHTMLUnsafe`, which owns context mutation).
    let mut dom = EcsDom::new();
    let ctx = dom.create_element("div", Attributes::default());
    let opts = ParseFragmentOptions {
        allow_declarative_shadow: true,
    };
    let live_before = dom.world().len();

    let result = parse_fragment_strict(
        "<template shadowrootmode=\"open\"><p>s</p></template>",
        ctx,
        &mut dom,
        opts,
    );

    assert!(result.is_err(), "a context-hosted DSD template is declined");
    assert!(
        dom.get_shadow_root(ctx).is_none(),
        "the context element is never given a shadow root"
    );
    assert_eq!(
        dom.world().len(),
        live_before,
        "decline rolls the synthetic subtree back — dom is pristine for fallback"
    );
}

// ----- the synthetic build fires no mutation events (isolation) -----

#[test]
fn fragment_build_suppresses_mutation_dispatch() {
    // Building the synthetic throwaway document on a live dom with a
    // dispatcher installed must fire no insert/remove events: `is_connected`
    // treats any `Document` root as connected, so consumers (custom elements,
    // observers, Range) would otherwise observe internal fragment nodes the
    // caller has not yet placed, then observe their teardown.
    struct Recorder {
        count: Arc<AtomicUsize>,
    }
    impl MutationDispatcher for Recorder {
        fn dispatch(&mut self, _event: &MutationEvent<'_>, _dom: &mut EcsDom) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    let mut dom = EcsDom::new();
    let ctx = dom.create_element("div", Attributes::default());
    let count = Arc::new(AtomicUsize::new(0));
    dom.set_mutation_dispatcher(Box::new(Recorder {
        count: count.clone(),
    }));

    let roots = parse_fragment_strict(
        "<section><p>a</p></section>",
        ctx,
        &mut dom,
        ParseFragmentOptions::default(),
    )
    .expect("valid fragment parses");
    assert!(!roots.is_empty(), "the fragment produced nodes");

    assert_eq!(
        count.load(Ordering::SeqCst),
        0,
        "no mutation event fires during the isolated synthetic build"
    );
}
