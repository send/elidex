//! D-31 `#11-base-href-resolution`: WHATWG HTML §2.4.3 Document base
//! URLs + §4.2.3 The base element + WHATWG DOM §4.4 Interface Node
//! `baseURI` getter.
//!
//! Exercises the BaseUrlMaintainer consumer composed in
//! `elidex_dom_api::ConsumerDispatcher` — Insert / Remove /
//! AttributeChange events on `<base>` elements maintain
//! [`DocumentBaseUrl`] + [`DocumentFirstBase`] +
//! [`DocumentBaseUrlVersion`] (Layer 2 + 3) and per-element
//! [`BaseFrozenUrl`] (Layer 1).

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
// <base>.href self-resolution
// ===========================================================================

#[test]
fn base_href_getter_returns_self_frozen_url() {
    // <base>.href IDL getter returns the frozen URL of the <base>
    // itself (HTML §4.2.3) — NOT the document base URL.
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
