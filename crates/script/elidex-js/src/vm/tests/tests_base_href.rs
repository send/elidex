//! `<base href>` document base URL tests: WHATWG HTML §2.4.3
//! "Document base URLs" + §4.2.3 "The base element" + WHATWG DOM §4.4
//! Interface Node `baseURI` getter.
//!
//! Exercises the BaseUrlMaintainer consumer composed in
//! `elidex_dom_api::ConsumerDispatcher` — Insert / Remove /
//! AttributeChange events on `<base>` elements maintain
//! [`DocumentBaseUrl`] (Layer 2) and per-element [`BaseFrozenUrl`]
//! (Layer 1).

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_fixture(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let head = dom.create_element("head", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, head));
    assert!(dom.append_child(html, body));
    doc
}

fn eval_string(vm: &mut Vm, src: &str) -> String {
    match vm.eval(src).unwrap() {
        JsValue::String(sid) => String::from_utf16_lossy(vm.inner.strings.get(sid)),
        other => panic!("expected string, got {other:?}"),
    }
}

// ===========================================================================
// document.baseURI
// ===========================================================================

#[test]
fn document_base_uri_returns_about_blank_when_no_base_element() {
    // HTML §2.4.3 fallback URL = document's URL (elidex stub:
    // about:blank pending #11-document-url-real-navigation).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let result = eval_string(&mut vm, "document.baseURI;");
    assert_eq!(result, "about:blank");
    vm.unbind();
}

#[test]
fn document_base_uri_reflects_inserted_base_element() {
    // Insert <base href="https://example.com/page"> into <head>;
    // document.baseURI must update to reflect.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        r"
        var b = document.createElement('base');
        b.setAttribute('href', 'https://example.com/page');
        document.head.appendChild(b);
        ",
    )
    .unwrap();

    let result = eval_string(&mut vm, "document.baseURI;");
    assert_eq!(result, "https://example.com/page");
    vm.unbind();
}

#[test]
fn document_base_uri_changes_when_base_href_mutates() {
    // setAttribute('href', ...) on an attached <base> must update
    // document.baseURI immediately (synchronous dispatch).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        r"
        var b = document.createElement('base');
        b.setAttribute('href', 'https://first.example/');
        document.head.appendChild(b);
        b.setAttribute('href', 'https://second.example/');
        ",
    )
    .unwrap();

    let result = eval_string(&mut vm, "document.baseURI;");
    assert_eq!(result, "https://second.example/");
    vm.unbind();
}

#[test]
fn document_base_uri_reverts_to_fallback_on_href_removal() {
    // removeAttribute('href') on the doc's <base> must revert
    // document.baseURI to the fallback (about:blank).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        r"
        var b = document.createElement('base');
        b.setAttribute('href', 'https://example.com/');
        document.head.appendChild(b);
        b.removeAttribute('href');
        ",
    )
    .unwrap();

    let result = eval_string(&mut vm, "document.baseURI;");
    assert_eq!(result, "about:blank");
    vm.unbind();
}

#[test]
fn document_base_uri_first_base_wins_with_multiple_bases() {
    // HTML §2.4.3 step 1: "the first base element ... that has an
    // href attribute, in tree order".
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        r"
        var b1 = document.createElement('base');
        b1.setAttribute('href', 'https://first.example/');
        var b2 = document.createElement('base');
        b2.setAttribute('href', 'https://second.example/');
        document.head.appendChild(b1);
        document.head.appendChild(b2);
        ",
    )
    .unwrap();

    let result = eval_string(&mut vm, "document.baseURI;");
    assert_eq!(result, "https://first.example/");
    vm.unbind();
}

#[test]
fn document_base_uri_skips_base_without_href() {
    // HTML §2.4.3 step 1: only <base> elements WITH href qualify.
    // The first <base> WITHOUT href is skipped.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        r"
        var b1 = document.createElement('base');
        // no href on b1
        var b2 = document.createElement('base');
        b2.setAttribute('href', 'https://second.example/');
        document.head.appendChild(b1);
        document.head.appendChild(b2);
        ",
    )
    .unwrap();

    let result = eval_string(&mut vm, "document.baseURI;");
    assert_eq!(result, "https://second.example/");
    vm.unbind();
}

// ===========================================================================
// Node.prototype.baseURI
// ===========================================================================

#[test]
fn node_base_uri_returns_document_base_uri() {
    // WHATWG DOM §4.4 Node.baseURI getter returns the node
    // document's base URL.  Any node should reflect the same value
    // as document.baseURI.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        r"
        var b = document.createElement('base');
        b.setAttribute('href', 'https://example.com/');
        document.head.appendChild(b);
        var div = document.createElement('div');
        document.body.appendChild(div);
        ",
    )
    .unwrap();

    assert_eq!(
        eval_string(&mut vm, "document.body.baseURI;"),
        "https://example.com/"
    );
    assert_eq!(
        eval_string(&mut vm, "document.body.firstChild.baseURI;"),
        "https://example.com/"
    );
    vm.unbind();
}

// ===========================================================================
// <a>.href resolves against <base>
// ===========================================================================

#[test]
fn anchor_href_resolves_against_base_url() {
    // The core D-31 user-visible behavior: <a href="relative">.href
    // returns the URL resolved against the doc base.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        r"
        var b = document.createElement('base');
        b.setAttribute('href', 'https://example.com/dir/');
        document.head.appendChild(b);
        var a = document.createElement('a');
        a.setAttribute('href', 'page.html');
        document.body.appendChild(a);
        ",
    )
    .unwrap();

    let result = eval_string(&mut vm, "document.body.firstChild.href;");
    assert_eq!(result, "https://example.com/dir/page.html");
    vm.unbind();
}

#[test]
fn anchor_href_updates_when_base_changes() {
    // Changing <base>.href reflects in <a>.href on next read.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        r"
        var b = document.createElement('base');
        b.setAttribute('href', 'https://first.example/');
        document.head.appendChild(b);
        var a = document.createElement('a');
        a.setAttribute('href', 'page');
        document.body.appendChild(a);
        b.setAttribute('href', 'https://second.example/');
        ",
    )
    .unwrap();

    let result = eval_string(&mut vm, "document.body.firstChild.href;");
    assert_eq!(result, "https://second.example/page");
    vm.unbind();
}

// ===========================================================================
// <base>.href IDL getter (WHATWG HTML §4.2.3)
//
// The getter does NOT return the element's BaseFrozenUrl directly;
// it parses the raw `href` content attribute against the document's
// fallback base URL (about:blank in elidex's stub) and returns:
// - the serialized parsed URL on parse success
// - the raw href value on parse failure (matches the spec's
//   "if url is failure, return the value of the content attribute")
//
// Two tests pin the two branches independently.
// ===========================================================================

#[test]
fn base_href_getter_returns_parsed_url_when_href_is_absolute() {
    // Parse-success branch: absolute href parses against the
    // about:blank fallback successfully → getter returns the
    // serialized parsed URL (which coincidentally equals the raw
    // href + the element's BaseFrozenUrl for absolute hrefs, since
    // all three are the same URL).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        r"
        var b = document.createElement('base');
        b.setAttribute('href', 'https://example.com/page');
        document.head.appendChild(b);
        ",
    )
    .unwrap();

    let result = eval_string(&mut vm, "document.head.lastChild.href;");
    assert_eq!(result, "https://example.com/page");
    vm.unbind();
}

#[test]
fn base_href_getter_returns_raw_value_when_href_unparseable() {
    // Parse-failure branch: relative href cannot resolve against
    // the about:blank fallback (url crate refuses to join relative
    // paths against opaque schemes) → getter returns the raw href
    // value per WHATWG HTML §4.2.3 IDL getter step 3 ("if url is
    // failure, return the value of the content attribute").  This
    // is the branch that distinguishes the getter from a naive
    // "return frozen URL" implementation: frozen URL would be the
    // fallback ("about:blank") for the same input — collapsing the
    // raw-fallback signal.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        r"
        var b = document.createElement('base');
        b.setAttribute('href', 'sub/path');
        document.head.appendChild(b);
        ",
    )
    .unwrap();

    let result = eval_string(&mut vm, "document.head.lastChild.href;");
    assert_eq!(
        result, "sub/path",
        "parse-failure branch must return raw href, not the fallback URL"
    );
    vm.unbind();
}

// ===========================================================================
// ECS-state hygiene + reparent + template-skip
// ===========================================================================

#[test]
fn remove_child_on_base_detaches_base_frozen_url_component() {
    // ECS hygiene: removing a <base> from the document tree must
    // detach its BaseFrozenUrl component (cleanup runs in the
    // BaseUrlMaintainer Remove arm via the descendants snapshot).
    // Without this the component lingers on the orphaned entity and
    // pollutes the world-wide BaseFrozenUrl query that gates the
    // short-circuit in subsequent Insert/Remove handling.
    use elidex_ecs::BaseFrozenUrl;

    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // All mutations through JS so the dispatcher stays live for the
    // Remove arm to execute.  appendChild attaches BaseFrozenUrl;
    // removeChild must detach it as part of the cleanup.
    vm.eval(
        r"
        var b = document.createElement('base');
        b.setAttribute('href', 'https://example.com/');
        document.head.appendChild(b);
        document.head.removeChild(b);
        ",
    )
    .unwrap();
    vm.unbind();

    // Post-cleanup: no entity in the world should retain BaseFrozenUrl.
    let any_frozen = dom
        .world()
        .query::<&BaseFrozenUrl>()
        .iter()
        .next()
        .is_some();
    assert!(
        !any_frozen,
        "BaseFrozenUrl must be detached from orphaned <base> entity \
         (ECS hygiene — Remove arm cleanup via descendants snapshot)"
    );
}

#[test]
fn base_element_reparented_mid_document_updates_first_base_winner() {
    // Two <base href> elements in <head>: a then b → tree order picks
    // a.  Move a after b (appendChild on already-attached node = move
    // to end) → tree order picks b.  Exercises the recompute when the
    // first-base shifts mid-document.
    //
    // All JS in a single eval call (vm.eval is a fresh program each
    // call; `var` declarations don't persist across calls).
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Phase 1: a then b → a first → baseURI = a.
    vm.eval(
        r"
        var a = document.createElement('base');
        a.setAttribute('href', 'https://a.example/');
        var b = document.createElement('base');
        b.setAttribute('href', 'https://b.example/');
        document.head.appendChild(a);
        document.head.appendChild(b);
        globalThis.__phase1_a = a;
        globalThis.__phase1_b = b;
        ",
    )
    .unwrap();
    assert_eq!(
        eval_string(&mut vm, "document.baseURI;"),
        "https://a.example/"
    );

    // Phase 2: move a to end via stashed reference on globalThis →
    // b is now first → baseURI = b.
    vm.eval("document.head.appendChild(globalThis.__phase1_a);")
        .unwrap();
    assert_eq!(
        eval_string(&mut vm, "document.baseURI;"),
        "https://b.example/"
    );
    vm.unbind();
}

#[test]
fn pre_bind_base_element_is_picked_up_by_init_pass() {
    // Pre-bind tree state: parser-style fixture where `<base href>`
    // exists in the DOM BEFORE the `MutationDispatcher` is installed
    // via `Vm::bind`.  Without the dispatcher's init pass
    // (`ConsumerDispatcher::initialize_consumers` → `BaseUrlMaintainer::
    // initialize_from_tree`) these nodes would never have produced
    // `MutationEvent::Insert`, so `BaseFrozenUrl` would not be
    // attached and `document.baseURI` would stay stuck at the
    // fallback `about:blank` until a subsequent mutation fires.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    // Insert `<base href>` BEFORE bind — directly via `EcsDom`, no
    // dispatcher exists yet.
    let head = dom.first_child_with_tag(doc, "html").unwrap();
    let head = dom.first_child_with_tag(head, "head").unwrap();
    let base = dom.create_element("base", Attributes::default());
    assert!(dom.set_attribute(base, "href", "https://pre-bind.example/"));
    assert!(dom.append_child(head, base));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // First read after bind MUST see the pre-bind `<base href>`
    // (init pass attached `BaseFrozenUrl` + recomputed
    // `DocumentBaseUrl`).
    assert_eq!(
        eval_string(&mut vm, "document.baseURI;"),
        "https://pre-bind.example/"
    );

    // Removing the pre-bind `<base>` must also work — the init pass
    // is what wired `BaseFrozenUrl`, without which the Remove arm's
    // `removed_a_qualifying_base` short-circuit would never trip and
    // recompute would never re-derive `about:blank`.
    vm.eval(
        r"
        var b = document.head.querySelector('base');
        document.head.removeChild(b);
        ",
    )
    .unwrap();
    assert_eq!(eval_string(&mut vm, "document.baseURI;"), "about:blank");
    vm.unbind();
}

#[test]
fn base_inside_shadow_tree_does_not_affect_document_base_uri() {
    // R5 F2 regression: WHATWG HTML §2.4.3 — shadow trees form
    // separate documents; a `<base href>` inside a shadow tree must
    // not affect the host document's base URL.  EcsDom's fire-site
    // filter only suppresses events where the node OR parent IS a
    // ShadowRoot; deeper shadow-tree mutations (`<base>` nested 1+
    // levels into shadow) reach BaseUrlMaintainer.  The
    // `in_main_light_tree` early-return in each arm enforces the
    // carve-out.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        r"
        var host = document.createElement('div');
        document.body.appendChild(host);
        var sr = host.attachShadow({mode: 'open'});
        var inner = document.createElement('section');
        sr.appendChild(inner);
        var b = document.createElement('base');
        b.setAttribute('href', 'https://inside-shadow.example/');
        inner.appendChild(b);
        ",
    )
    .unwrap();

    // <base> inside a shadow tree must NOT affect document.baseURI.
    assert_eq!(eval_string(&mut vm, "document.baseURI;"), "about:blank");
    vm.unbind();
}

#[test]
fn base_inside_template_does_not_affect_document_base_uri() {
    // WHATWG HTML §2.4.3 carve-out: template contents form a
    // separate document.  A <base href> inside <template> must NOT
    // be selected as the host document's first-base.  Walker skip
    // via dom.is_template_element.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    vm.eval(
        r"
        var t = document.createElement('template');
        var b = document.createElement('base');
        b.setAttribute('href', 'https://inside-template.example/');
        t.appendChild(b);
        document.body.appendChild(t);
        ",
    )
    .unwrap();

    // <base> inside <template> must be invisible to the document
    // base-URL algorithm.
    let result = eval_string(&mut vm, "document.baseURI;");
    assert_eq!(result, "about:blank");
    vm.unbind();
}
