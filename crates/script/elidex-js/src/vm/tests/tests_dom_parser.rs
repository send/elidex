//! `DOMParser` (HTML §8.5.1) + `XMLSerializer` (HTML §8.5.8)
//! tests.
//!
//! The assertions are written against the **real-Document** VM design
//! (S5-1, D1): the real Document exposes real nodes — e.g.
//! `documentElement` is a real `<html>` element — so the spec-correct
//! value is asserted, noted inline.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

/// Build a bound VM over a minimal page document (so the DOMParser /
/// XMLSerializer natives have an `EcsDom` to spawn the throwaway
/// document into / read nodes from). Returns the owned `(vm, session,
/// dom)` triple — callers keep them alive for the VM's lifetime and
/// call `vm.unbind()` when done.
fn bound_vm() -> (Vm, Box<SessionCore>, Box<EcsDom>) {
    let mut vm = Vm::new();
    let mut session = Box::new(SessionCore::new());
    let mut dom = Box::new(EcsDom::new());
    let doc = dom.create_document_root();

    // `session` / `dom` are boxed so their heap contents keep a stable
    // address when the `(vm, session, dom)` triple is moved out of this
    // helper — `vm.bind` stores raw pointers into them, so a stack-value
    // move would dangle, but a `Box` move only relocates the 8-byte
    // handle, not the pointee.
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    (vm, session, dom)
}

/// Eval `source` and assert it returns a boolean `true`.
fn assert_true(vm: &mut Vm, source: &str) {
    match vm.eval(source).unwrap() {
        JsValue::Boolean(true) => {}
        other => panic!("expected `true`, got {other:?} for source:\n{source}"),
    }
}

/// Eval `setup`, then read back a string expression in a SECOND eval so
/// the custom-element reaction drain at the first eval's tail has
/// completed (lifecycle callbacks that write globals are not visible
/// within the same eval's return — mirrors `tests_custom_elements::
/// run_then_read`). Returns the read string.
fn eval_then_read(vm: &mut Vm, setup: &str, read_expr: &str) -> String {
    vm.eval(setup).expect("setup eval failed");
    match vm.eval(read_expr).expect("read eval failed") {
        JsValue::String(sid) => vm.inner.strings.get_utf8(sid),
        other => panic!("expected string from read, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// DOMParser
// ---------------------------------------------------------------------------

#[test]
fn dom_parser_basic_query_selector() {
    let (mut vm, _s, _d) = bound_vm();
    assert_true(
        &mut vm,
        r#"
        var parser = new DOMParser();
        var doc = parser.parseFromString('<div id="test">hello</div>', 'text/html');
        var el = doc.querySelector('#test');
        el !== null && el.textContent === 'hello';
    "#,
    );
    vm.unbind();
}

#[test]
fn dom_parser_query_selector_all_count() {
    let (mut vm, _s, _d) = bound_vm();
    assert_true(
        &mut vm,
        r"
        var parser = new DOMParser();
        var doc = parser.parseFromString('<p>a</p><p>b</p>', 'text/html');
        var ps = doc.querySelectorAll('p');
        ps.length === 2;
    ",
    );
    vm.unbind();
}

#[test]
fn dom_parser_unsupported_mime_throws_type_error() {
    let (mut vm, _s, _d) = bound_vm();
    // `application/json` is outside the boa-parity accepted set → TypeError.
    assert_true(
        &mut vm,
        r"
        var parser = new DOMParser();
        var threw = false;
        try { parser.parseFromString('test', 'application/json'); }
        catch (e) { threw = e instanceof TypeError; }
        threw;
    ",
    );
    vm.unbind();
}

#[test]
fn dom_parser_xml_family_mime_accepted() {
    let (mut vm, _s, _d) = bound_vm();
    // `application/xml` is in the accepted set; all accepted types are
    // HTML-parsed (boa parity — no real XML parser yet).
    assert_true(
        &mut vm,
        r"
        var parser = new DOMParser();
        var doc = parser.parseFromString('<root><child></child></root>', 'application/xml');
        typeof doc.querySelector === 'function';
    ",
    );
    vm.unbind();
}

#[test]
fn dom_parser_document_element_is_real_html_element() {
    let (mut vm, _s, _d) = bound_vm();
    // DIVERGENCE FROM BOA: boa's stub returned a fake object whose
    // `documentElement` getter walked the container's first child
    // element. The VM returns a real Document whose `documentElement` is
    // the synthesized `<html>` element — so we assert the spec-correct
    // tagName (`HTML`) rather than merely `!== null`.
    assert_true(
        &mut vm,
        r"
        var parser = new DOMParser();
        var doc = parser.parseFromString('<html><body><p>text</p></body></html>', 'text/html');
        doc.documentElement !== null && doc.documentElement.tagName === 'HTML';
    ",
    );
    vm.unbind();
}

#[test]
fn dom_parser_body_synthesized_from_markup() {
    let (mut vm, _s, _d) = bound_vm();
    // Fragment-parsing in `<html>` context lets html5ever synthesize
    // `<head>`/`<body>` so `doc.body` resolves on real markup (D2).
    assert_true(
        &mut vm,
        r"
        var parser = new DOMParser();
        var doc = parser.parseFromString('<html><body><p id=x>hi</p></body></html>', 'text/html');
        doc.body !== null && doc.body.querySelector('#x') !== null;
    ",
    );
    vm.unbind();
}

#[test]
fn dom_parser_returns_distinct_document_not_page_document() {
    let (mut vm, _s, _d) = bound_vm();
    // The throwaway Document must NOT be the page's `document` (D1 —
    // `create_document_node` does not clobber `document_root`).
    assert_true(
        &mut vm,
        r"
        var parser = new DOMParser();
        var doc = parser.parseFromString('<div></div>', 'text/html');
        doc !== document;
    ",
    );
    vm.unbind();
}

#[test]
fn dom_parser_brand_check_throws_on_alien_receiver() {
    let (mut vm, _s, _d) = bound_vm();
    assert_true(
        &mut vm,
        r"
        var threw = false;
        try { DOMParser.prototype.parseFromString.call({}, '<div></div>', 'text/html'); }
        catch (e) { threw = e instanceof TypeError; }
        threw;
    ",
    );
    vm.unbind();
}

// ---------------------------------------------------------------------------
// XMLSerializer
// ---------------------------------------------------------------------------

#[test]
fn xml_serializer_element_round_trips_markup() {
    let (mut vm, _s, _d) = bound_vm();
    assert_true(
        &mut vm,
        r"
        var s = new XMLSerializer();
        var div = document.createElement('div');
        div.setAttribute('class', 'test');
        var result = s.serializeToString(div);
        result.indexOf('<div') === 0 && result.indexOf('class') > 0;
    ",
    );
    vm.unbind();
}

#[test]
fn xml_serializer_text_node_returns_text_content() {
    let (mut vm, _s, _d) = bound_vm();
    assert_true(
        &mut vm,
        r"
        var s = new XMLSerializer();
        var text = document.createTextNode('hello world');
        var result = s.serializeToString(text);
        result === 'hello world';
    ",
    );
    vm.unbind();
}

#[test]
fn xml_serializer_document_round_trips_to_markup() {
    let (mut vm, _s, _d) = bound_vm();
    // BUG A regression: serializing a Document (a node with no tag of its
    // own) must return the markup of its CHILDREN, NOT the concatenated
    // descendant text (`collect_text_content`, the prior wrong call). The
    // canonical DOMParser→XMLSerializer round-trip therefore yields real
    // markup containing `<div`/`<span`, not `"hi"`.
    assert_true(
        &mut vm,
        r"
        var parser = new DOMParser();
        var doc = parser.parseFromString('<div id=a><span>hi</span></div>', 'text/html');
        var s = new XMLSerializer();
        var out = s.serializeToString(doc);
        out.indexOf('<div') >= 0 && out.indexOf('<span') >= 0 && out.indexOf('>hi<') >= 0;
    ",
    );
    vm.unbind();
}

#[test]
fn xml_serializer_comment_node_serializes_with_delimiters() {
    let (mut vm, _s, _d) = bound_vm();
    // BUG A regression: a Comment node serializes as `<!--data-->`, NOT the
    // empty string (the prior `collect_text_content` returned "" for a
    // comment — it has no `TextContent` descendant).
    assert_true(
        &mut vm,
        r"
        var s = new XMLSerializer();
        var c = document.createComment('hello');
        s.serializeToString(c) === '<!--hello-->';
    ",
    );
    vm.unbind();
}

#[test]
fn xml_serializer_document_fragment_serializes_children_markup() {
    let (mut vm, _s, _d) = bound_vm();
    // A DocumentFragment (like Document) has no tag of its own, so it
    // serializes as the markup of its children.
    assert_true(
        &mut vm,
        r#"
        var s = new XMLSerializer();
        var frag = document.createDocumentFragment();
        var div = document.createElement('div');
        div.setAttribute('id', 'f');
        frag.appendChild(div);
        var out = s.serializeToString(frag);
        out.indexOf('<div') === 0 && out.indexOf('id="f"') > 0;
    "#,
    );
    vm.unbind();
}

#[test]
fn dom_parser_inert_document_no_custom_element_upgrade() {
    let (mut vm, _s, _d) = bound_vm();
    // BUG B regression (the LOAD-BEARING test for the dispatcher
    // suppression in `elidex_form::parse_into_inert_document`): building the
    // throwaway document must NOT fire the live page mutation dispatcher. A
    // DOMParser document is inert per HTML §8.5.1 (no browsing context,
    // §13.2.4.5 scripting flag disabled), so a custom element in the parsed
    // markup must NOT be upgraded and must NOT fire `connectedCallback`.
    //
    // Mechanism the suppression defeats: `apply_set_inner_html` appends the
    // parsed `<x-test>` under an `<html>` rooted at a `NodeKind::Document`,
    // so `dom.is_connected(node)` is TRUE. With the page dispatcher live,
    // the `CustomElementReactionConsumer`'s Insert handler would
    // `try-to-upgrade` the element (its name matches the registered
    // definition) → the constructor runs → `finalize_success` enqueues a
    // Connected reaction → `connectedCallback` fires at the eval-tail CE
    // drain. The dispatcher suppression scoped to the throwaway build is the
    // ONLY thing preventing that.
    //
    // This test PINS the fix: empirically verified that REVERTING the
    // `take_mutation_dispatcher` / `set_mutation_dispatcher` suppression
    // flips both counters to 1 (constructor + connectedCallback fire), so a
    // regression would fail this assertion — unlike the prior page-element
    // MutationObserver test, which passed identically with the fix reverted
    // because the parsed nodes live in a disjoint tree the observer never
    // saw (tree disjointness, not suppression, made that one pass).
    let out = eval_then_read(
        &mut vm,
        r"
        globalThis.__ctor = 0;
        globalThis.__connected = 0;
        customElements.define('x-test', class extends HTMLElement {
            constructor() { super(); globalThis.__ctor += 1; }
            connectedCallback() { globalThis.__connected += 1; }
        });
        var parser = new DOMParser();
        parser.parseFromString('<x-test></x-test>', 'text/html');
    ",
        "'' + globalThis.__ctor + ':' + globalThis.__connected;",
    );
    assert_eq!(
        out, "0:0",
        "DOMParser document is inert (§8.5.1): a custom element in the parsed \
         markup must NOT be upgraded (constructor) or connected \
         (connectedCallback). got ctor:connected = {out}"
    );
    vm.unbind();
}

#[test]
fn dom_parser_returned_document_is_live_after_build() {
    let (mut vm, _s, _d) = bound_vm();
    // BUG B companion: suppression is scoped to the throwaway build — the
    // RETURNED document is live, so a MutationObserver registered on a node
    // INSIDE it AFTER parseFromString still delivers records for subsequent
    // mutations (the dispatcher was restored, not permanently dropped).
    assert_true(
        &mut vm,
        r"
        var parser = new DOMParser();
        var doc = parser.parseFromString('<div id=host></div>', 'text/html');
        var host = doc.querySelector('#host');
        var fired = 0;
        var obs = new MutationObserver(function(records) { fired += records.length; });
        obs.observe(host, { childList: true });
        host.appendChild(doc.createElement('span'));
        var n = obs.takeRecords().length;
        n >= 1;
    ",
    );
    vm.unbind();
}

#[test]
fn dom_parser_form_control_has_value() {
    let (mut vm, _s, _d) = bound_vm();
    // Codex F1 regression: a DOMParser'd `<input value=x>` must expose
    // `.value === 'x'`. The throwaway-build dispatcher suppression (the §8.5.1
    // inert guarantee) also takes out `FormControlReconciler`, the live-page
    // path that attaches `FormControlState`. The engine-indep
    // `elidex_form::parse_into_inert_document` primitive re-runs the FCS attach
    // SUBTREE-SCOPED (`create_form_control_state` per parsed descendant) inside
    // the suppressed window; without it the control has no FCS and `.value`
    // reads "".
    assert_true(
        &mut vm,
        r"
        var parser = new DOMParser();
        var doc = parser.parseFromString('<input value=x>', 'text/html');
        doc.querySelector('input').value === 'x';
    ",
    );
    vm.unbind();
}

#[test]
fn dom_parser_does_not_clobber_page_form_control_state() {
    let (mut vm, _s, _d) = bound_vm();
    // SEVERE-regression guard: the throwaway DOMParser document shares the
    // bound PAGE document's `EcsDom`, so attaching `FormControlState` to the
    // parsed controls must be SUBTREE-SCOPED to the throwaway document — NOT a
    // whole-dom `init_form_controls(dom)`, which queries the ENTIRE world and
    // rebuilds FCS from attributes for every form control, INCLUDING the live
    // page's controls. That would reset every page `<input>`/`<select>`/
    // `<textarea>` to its attribute-derived state, destroying the dirty-value
    // flag / user-typed `.value`. I.e. `new DOMParser().parseFromString(
    // '<input>', 'text/html')` would wipe all user input on the page.
    //
    // ROOTING: `bound_vm()`'s page document is a bare `Document` node (no
    // `<body>`), so the live page control is rooted by appending a `<div>`
    // container under `document` and the `<input>` under it — a genuine
    // CONNECTED page control whose FCS the whole-dom query would reach. The
    // control gets FCS at `document.createElement('input')` time (the live
    // createElement post-hook), and `pageInput.value = …` sets its dirty value
    // flag + user-typed value.
    //
    // This test FAILS with the buggy whole-dom `init_form_controls(dom)` (the
    // page input's `.value` resets to "" / the parser attribute) and PASSES
    // with the subtree-scoped `create_form_control_state` attach.
    assert_true(
        &mut vm,
        r"
        // Live page form control with a dirty, user-typed value.
        var container = document.createElement('div');
        document.appendChild(container);
        var pageInput = document.createElement('input');
        container.appendChild(pageInput);
        pageInput.value = 'typed-by-user';

        // Parse a document containing a form control — its FCS attach must NOT
        // clobber the shared page document's controls.
        var parser = new DOMParser();
        var doc = parser.parseFromString('<input value=fromparser>', 'text/html');

        // (1) The PAGE input is UNCLOBBERED (still the user-typed value).
        var pagePreserved = pageInput.value === 'typed-by-user';
        // (2) The PARSED document's own input still works (F1 still fixed).
        var parsedWorks = doc.querySelector('input').value === 'fromparser';

        pagePreserved && parsedWorks;
    ",
    );
    vm.unbind();
}

#[test]
fn dom_parser_base_href_sets_document_base_url() {
    let (mut vm, _s, _d) = bound_vm();
    // Codex R2-F2 regression: a parsed `<base href>` must set the throwaway
    // document's base URL, exactly as a live parse would. The throwaway build
    // suppresses the WHOLE mutation dispatcher (the §8.5.1 inert guarantee),
    // which also takes out `BaseUrlMaintainer` (#3 in `consumer_dispatcher.rs`)
    // — the live-page Insert path that derives `DocumentBaseUrl` from the first
    // `<base href>` (HTML §2.4.3). `BaseUrlMaintainer` is a STRUCTURAL-FACT
    // reconciler (`baseURI` is a DOM fact independent of scripting), so it is
    // re-run document-scoped inside the suppressed window by the engine-indep
    // `elidex_form::parse_into_inert_document` primitive (via
    // `initialize_base_url_for_document(dom, doc, caller_url)`). WITHOUT that
    // call the base URL stays the caller-URL fallback (here `about:blank`, the
    // bound test page's default) and this assertion fails; WITH it the parsed
    // `<base>` is honored.
    assert_true(
        &mut vm,
        r#"
        var parser = new DOMParser();
        var doc = parser.parseFromString(
            '<head><base href="https://example.com/dir/"></head><body></body>',
            'text/html');
        doc.baseURI === 'https://example.com/dir/';
    "#,
    );
    vm.unbind();
}

#[test]
fn dom_parser_relative_href_resolves_against_parsed_base() {
    let (mut vm, _s, _d) = bound_vm();
    // Codex R2-F2 companion: the downstream effect of the parsed `<base href>`
    // — a relative `<a href>` in the same document resolves against the
    // document base URL (HTML §2.4.3 + the `<a>.href` URL-reflection getter),
    // not the `about:blank` fallback. Pins the user-observable consequence of
    // the base-url structural-fact finalization, not just the `baseURI` read.
    assert_true(
        &mut vm,
        r#"
        var parser = new DOMParser();
        var doc = parser.parseFromString(
            '<head><base href="https://example.com/dir/"></head>' +
            '<body><a id=a href="page.html">x</a></body>',
            'text/html');
        doc.querySelector('#a').href === 'https://example.com/dir/page.html';
    "#,
    );
    vm.unbind();
}

#[test]
fn dom_parser_no_base_href_falls_back_to_caller_document_url() {
    // R3-F1 regression: per HTML §8.5.1 step 2 the new Document's URL is the
    // CALLER document's URL, so a parsed document with NO `<base href>` must
    // resolve relative URLs against the calling page — NOT `about:blank`. This
    // pins the caller-URL base fallback threaded through
    // `elidex_form::parse_into_inert_document` →
    // `initialize_base_url_for_document(.., caller_url)`.
    //
    // Empirically: WITHOUT the caller-URL fallback (the pre-fix `about:blank`
    // hardcode), `.href` resolves to `about:blank` + the relative path (a
    // failed/`about:`-rooted URL), so the assertion below fails — confirming the
    // test actually exercises the fix.
    let (mut vm, _s, _d) = bound_vm();
    // Bind path leaves `current_url` at its `about:blank` default; set the page
    // URL the way `vm_with_url_and_mock` does (document.URL / fetch Referer read
    // the same `navigation.current_url`).
    vm.inner.navigation.current_url =
        url::Url::parse("https://example.com/dir/page.html").expect("valid page URL");
    assert_true(
        &mut vm,
        r#"
        var parser = new DOMParser();
        var doc = parser.parseFromString('<a href="p.html"></a>', 'text/html');
        doc.querySelector('a').href === 'https://example.com/dir/p.html';
    "#,
    );
    vm.unbind();
}

#[test]
fn dom_parser_noscript_parsed_as_elements_scripting_disabled() {
    let (mut vm, _s, _d) = bound_vm();
    // Codex F3 regression: a DOMParser document is inert (HTML §8.5.1 — no
    // browsing context, §13.2.4.5 scripting flag DISABLED), so `<noscript>`
    // content is parsed as ordinary ELEMENTS, not raw text. With scripting
    // enabled (the live-element `innerHTML` default), `<noscript><p id=x></p>`
    // is RAWTEXT and the `<p>` does not exist — DOMParser must create it.
    // The `<noscript>` is placed inside an explicit `<body>` so the "in body"
    // noscript arm (`modes/in_body.rs`) routes it to "any other start tag"
    // when scripting is disabled, parsing `<p>` as its real child. (A BARE
    // leading `<noscript>` routes via the implied `<head>` into "in head
    // noscript", where the `<p>` is a strict parse error AND html5ever's
    // fragment fallback ignores the scripting flag for noscript — so the bare
    // case stays rawtext today: documented at
    // `dom_parser_bare_noscript_stays_rawtext_fragment_mode_boundary` and the
    // parse site, slot `#11-domparser-full-document-parse-fidelity`.)
    assert_true(
        &mut vm,
        r"
        var parser = new DOMParser();
        var doc = parser.parseFromString(
            '<body><noscript><p id=x></p></noscript></body>', 'text/html');
        var p = doc.querySelector('#x');
        p !== null && p.parentNode.tagName === 'NOSCRIPT';
    ",
    );
    vm.unbind();
}

#[test]
fn dom_parser_bare_noscript_stays_rawtext_fragment_mode_boundary() {
    let (mut vm, _s, _d) = bound_vm();
    // Codex F3 EXACT repro — PINS THE CURRENT (documented) BEHAVIOR at a
    // genuine fragment-mode boundary, NOT the ideal. A BARE leading
    // `<noscript>` (no explicit `<body>`/`<head>` wrapper) does NOT yet parse
    // its content as elements: `querySelector('#x')` is `null` because the
    // `<p>` survives as RAW TEXT inside `<noscript>`. The ideal DOMParser
    // (inert, scripting disabled) would create the `<p>` — that requires a true
    // full-document parse and is deferred to
    // `#11-domparser-full-document-parse-fidelity`.
    //
    // Why the scripting-flag forwarding does NOT fix this case (empirically
    // verified, both backends, in the parser crate):
    //   * DOMParser fragment-parses in `<html>` context. §13.2.4.1 reset →
    //     "before head"; `<noscript>` → "anything else" → implied `<head>`,
    //     reprocess in "in head"; scripting disabled → insert `<noscript>` and
    //     switch to "in head noscript", where the `<p>` start tag is a strict
    //     parse error — strict deliberately omits the spec's "pop noscript and
    //     reprocess" recovery (`modes/in_head_noscript.rs`). Strict therefore
    //     Errs → §11.3 tolerant html5ever fallback.
    //   * html5ever's `parse_fragment` IGNORES its `scripting_enabled` argument
    //     for `<noscript>`: with scripting=false it STILL emits
    //     `<noscript>#text "<p id=x></p>"` — byte-identical to scripting=true.
    //     (html5ever honors the flag only in `parse_document`, where the bare
    //     case correctly hoists the `<p>` into `<body>`. The fix is a true
    //     document parse, not the fragment parse this PR uses.)
    // So the bare case stays rawtext. The earlier edge-note claiming "tolerant
    // does not honor the scripting flag" is CORRECT for fragment mode; the
    // claim that html5ever "SHOULD honor it" holds only for document mode.
    //
    // The WRAPPED forms DO work today and are covered by
    // `dom_parser_noscript_parsed_as_elements_scripting_disabled` (`<body>`
    // context → strict "in body" noscript→any-other-start-tag) and
    // `dom_parser_head_noscript_parsed_as_elements_scripting_disabled`
    // (`<head>` context → strict "in head noscript"). It is only the BARE
    // implied-head routing that hits the boundary.
    assert_true(
        &mut vm,
        r"
        var parser = new DOMParser();
        var doc = parser.parseFromString('<noscript><p id=x></p></noscript>', 'text/html');
        // Current behavior: <p> is raw text, so #x does not exist as an element.
        doc.querySelector('#x') === null;
    ",
    );
    vm.unbind();
}

#[test]
fn dom_parser_head_noscript_parsed_as_elements_scripting_disabled() {
    let (mut vm, _s, _d) = bound_vm();
    // Codex F3 (head context): a `<noscript>` directly inside `<head>` with
    // scripting disabled exercises the strict "in head noscript" insertion
    // mode (`modes/in_head_noscript.rs`, previously unreachable). Its
    // `<link>`/`<meta>`/`<style>` children are head-content elements; the
    // `<noscript>` element itself must exist in the parsed head as a real
    // element (not raw text), so a `<link>` inside it is reachable via the
    // document.
    assert_true(
        &mut vm,
        r#"
        var parser = new DOMParser();
        var doc = parser.parseFromString(
            '<head><noscript><link id=l rel="stylesheet" href="a.css"></noscript></head>',
            'text/html'
        );
        var noscript = doc.querySelector('noscript');
        var link = doc.querySelector('#l');
        noscript !== null && link !== null && link.parentNode.tagName === 'NOSCRIPT';
    "#,
    );
    vm.unbind();
}

#[test]
fn dom_parser_live_element_noscript_still_rawtext() {
    let (mut vm, _s, _d) = bound_vm();
    // Codex F3 default-preservation: `scripting_disabled` defaults to false, so
    // setting `innerHTML` on a NORMAL (live) element keeps scripting ENABLED —
    // `<noscript>` content stays raw text and `#x` is NOT found. Confirms the
    // new option does not regress the live `innerHTML` path.
    assert_true(
        &mut vm,
        r"
        var host = document.createElement('div');
        host.innerHTML = '<noscript><p id=x></p></noscript>';
        host.querySelector('#x') === null;
    ",
    );
    vm.unbind();
}

#[test]
fn xml_serializer_brand_check_throws_on_alien_receiver() {
    let (mut vm, _s, _d) = bound_vm();
    assert_true(
        &mut vm,
        r"
        var div = document.createElement('div');
        var threw = false;
        try { XMLSerializer.prototype.serializeToString.call({}, div); }
        catch (e) { threw = e instanceof TypeError; }
        threw;
    ",
    );
    vm.unbind();
}

#[test]
fn xml_serializer_missing_node_arg_throws_type_error() {
    let (mut vm, _s, _d) = bound_vm();
    // boa threw when the arg was absent / not a node; the VM matches —
    // a non-Node receiver fails the HostObject extraction → TypeError.
    assert_true(
        &mut vm,
        r"
        var s = new XMLSerializer();
        var threw = false;
        try { s.serializeToString({}); }
        catch (e) { threw = e instanceof TypeError; }
        threw;
    ",
    );
    vm.unbind();
}

#[test]
fn xml_serializer_window_throws_type_error() {
    let (mut vm, _s, _d) = bound_vm();
    // Codex F4 regression: `globalThis` / `window` is a HostObject over
    // `NodeKind::Window` — an EventTarget but NOT a Node (`nodeType == 0`).
    // The entity-bits extraction resolves it to a Window entity, so the
    // serializer must additionally gate on `NodeKind::is_node()` and throw a
    // TypeError rather than serializing "".
    assert_true(
        &mut vm,
        r"
        var s = new XMLSerializer();
        var threw = false;
        try { s.serializeToString(globalThis); }
        catch (e) { threw = e instanceof TypeError; }
        threw;
    ",
    );
    vm.unbind();
}

#[test]
fn xml_serializer_real_node_still_serializes_after_window_gate() {
    let (mut vm, _s, _d) = bound_vm();
    // Codex F4 companion: the Node-vs-Window gate must NOT reject a real
    // element/document. A plain `<div>` still serializes to its outer markup.
    assert_true(
        &mut vm,
        r"
        var s = new XMLSerializer();
        var div = document.createElement('div');
        var out = s.serializeToString(div);
        out.indexOf('<div') === 0;
    ",
    );
    vm.unbind();
}

#[test]
fn dom_parser_xml_serializer_not_exposed_in_worker_scope() {
    // Codex R4: DOMParser + XMLSerializer are `[Exposed=Window]`
    // (HTML §8.5.1 / §8.5.8, webref-verified) — a worker realm has no
    // document surface, so it must NOT get either constructor (otherwise
    // worker code could build DOM `Document` wrappers in a realm that has
    // no document at all). Mirrors `media_query_list_not_exposed_in_worker_scope`.
    let mut vm = Vm::new_worker(
        "w".to_string(),
        url::Url::parse("https://example.com/w.js").unwrap(),
        true,
        elidex_net::CredentialsMode::SameOrigin,
        elidex_plugin::EngineMode::BrowserCompat,
    );
    vm.install_host_data(super::super::host_data::HostData::new());
    assert_true(
        &mut vm,
        "typeof DOMParser === 'undefined' && typeof XMLSerializer === 'undefined';",
    );
}

#[test]
fn dom_parser_and_serializer_no_op_after_unbind_without_throwing() {
    // Codex R4: a DOMParser / XMLSerializer wrapper retained across
    // `Vm::unbind()` must follow the silent-detached policy the rest of the
    // DOM-touching native family uses (`class_list` / `css_style_declaration`):
    // the binding check runs BEFORE argument coercion / validation, so a
    // detached call no-op's instead of throwing.
    //   - `parseFromString` returns null without ToString-coercing the args
    //     (a `Symbol` arg would otherwise throw) or validating the MIME (an
    //     unsupported type would otherwise throw).
    //   - `serializeToString` returns "" without running the Node-arg gate
    //     (a non-Node arg would otherwise throw a TypeError).
    let (mut vm, _s, _d) = bound_vm();
    vm.eval("globalThis.p = new DOMParser(); globalThis.s = new XMLSerializer();")
        .unwrap();
    vm.unbind();
    assert_true(
        &mut vm,
        r"
        globalThis.p.parseFromString(Symbol('x'), 'text/html') === null
        && globalThis.p.parseFromString('<p>x</p>', 'application/json') === null
        && globalThis.s.serializeToString(42) === '';
    ",
    );
}

#[test]
fn dom_parser_attaches_form_control_state_inside_template_content() {
    let (mut vm, _s, _d) = bound_vm();
    // Codex R5: a control inside `<template>` content (a detached
    // `DocumentFragment`, HTML §4.12.3) must still receive `FormControlState`
    // during the inert build, so its `.value` reflects the parsed `value`
    // attribute. `for_each_shadow_inclusive_descendant` does not reach template
    // contents, so the FCS walk adds a template-content frontier; without it
    // `template.content.querySelector('input').value` falls back to `''`.
    assert_true(
        &mut vm,
        r#"
        var parser = new DOMParser();
        var doc = parser.parseFromString(
            '<body><template><input value="x"></template></body>', 'text/html');
        doc.querySelector('template').content.querySelector('input').value === 'x';
    "#,
    );
    vm.unbind();
}

#[test]
fn dom_parser_base_maintenance_active_for_parsed_document() {
    let (mut vm, _s, _d) = bound_vm();
    // Codex R5: a DOMParser document is a SECOND live `Document` (not the page
    // `document_root`), so its `<base>` mutations must still drive
    // `BaseUrlMaintainer`. The `in_document_light_tree` guard now keys on "tree
    // root is a `Document`" rather than "tree root is THE page root", so
    // mutating the parsed `<base>`'s href post-parse updates `doc.baseURI`
    // (exercises the AttributeChange arm; the shared predicate covers
    // Insert/Remove identically). Absolute hrefs keep this independent of the
    // deferred caller-URL fallback (`#11-document-url-real-navigation`).
    assert_true(
        &mut vm,
        r#"
        var parser = new DOMParser();
        var doc = parser.parseFromString(
            '<head><base id=b href="https://a.example/x/"></head><body></body>',
            'text/html');
        var before = doc.baseURI;
        doc.querySelector('#b').setAttribute('href', 'https://b.example/y/');
        before === 'https://a.example/x/' && doc.baseURI === 'https://b.example/y/';
    "#,
    );
    vm.unbind();
}

#[test]
fn xml_serializer_rejects_canvas_2d_context_as_non_node() {
    let (mut vm, _s, _d) = bound_vm();
    // Codex R5: a `CanvasRenderingContext2D` wrapper deliberately shares its
    // `<canvas>` entity (which IS a Node), so `serializeToString` must reject it
    // via the reverse canvas-context brand check rather than serializing the
    // backing `<canvas>`. Mirrors `node_proto::require_node_arg`.
    assert_true(
        &mut vm,
        r"
        var canvas = document.createElement('canvas');
        var ctx = canvas.getContext('2d');
        var threw = false;
        try { new XMLSerializer().serializeToString(ctx); }
        catch (e) { threw = e instanceof TypeError; }
        threw;
    ",
    );
    vm.unbind();
}
