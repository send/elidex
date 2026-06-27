//! `DOMParser` (HTML §8.5.1) + `XMLSerializer` (HTML §8.5.8)
//! tests.
//!
//! Scenario baseline = the 7 boa tests
//! (`elidex-js-boa/.../web_apis/dom_parser.rs`), but the assertions are
//! written against the **real-Document** VM design (S5-1, D1): where the
//! real Document diverges from boa's closure-object stub — e.g.
//! `documentElement` is now a real `<html>` element — the spec-correct
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
    // suppression in `dom_parser.rs`): building the throwaway document must
    // NOT fire the live page mutation dispatcher. A DOMParser document is
    // inert per HTML §13.4 (no browsing context, scripting disabled), so a
    // custom element in the parsed markup must NOT be upgraded and must NOT
    // fire `connectedCallback`.
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
        "DOMParser document is inert (§13.4): a custom element in the parsed \
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
