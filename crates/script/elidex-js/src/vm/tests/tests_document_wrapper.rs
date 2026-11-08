//! C1-pre — every Document entity's wrapper is a Document (plan
//! `docs/plans/2026-11-eh-attrs-c1-pre-document-wrapper.md` §6.1).
//!
//! The cells W1 / W15 / W22 need a second Document entity in the bound
//! `EcsDom` (made by `create_document_node`, the spawn C1's inert template
//! document will use), not bound, reached through `p.ownerDocument` or the
//! adopted-reaction flush: a generic-path wrap of a Document the bridge has
//! never seen.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

/// JS preamble: `F` formats one value, `T` formats a thunk's value or the
/// error name it throws, `L` joins formatted legs.
const PRE: &str = "\
globalThis.F = function (v) { \
  return v === undefined ? 'undefined' : v === null ? 'null' \
    : typeof v === 'object' ? 'object' \
    : typeof v === 'string' ? JSON.stringify(v) : String(v); }; \
globalThis.T = function (f) { \
  try { return F(f()); } catch (e) { return 'throws:' + e.name; } }; \
globalThis.L = function (...fs) { \
  return fs.map(function (f) { return T(f); }).join('|'); };";

fn page(dom: &mut EcsDom) -> Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element_ownerless_fixture("html", Attributes::default());
    let body = dom.create_element_ownerless_fixture("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

fn eval_str(vm: &mut Vm, src: &str) -> String {
    match vm.eval(src).expect("eval failed") {
        JsValue::String(sid) => vm.inner.strings.get_utf8(sid),
        other => panic!("expected string from `{src}`, got {other:?}"),
    }
}

/// Run `body` on a VM bound to a page, with `L` & co. defined.
fn with_page(body: impl FnOnce(&mut Vm)) {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = page(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(PRE).expect("preamble");
    body(&mut vm);
    vm.unbind();
}

/// The W1 / W15 / W22 fixture: a second, unbound Document `D` in the page
/// `EcsDom` with an element `p` (node document `D`) appended, `p` exposed as
/// a global; nothing wraps `D` before the script. `focus` sets
/// `ElementState::FOCUS` on `p` from Rust.
fn with_second_document(focus: bool, body: impl FnOnce(&mut Vm)) {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = page(&mut dom);
    let d = dom.create_document_node();
    let owner = dom.node_document_for_new_nodes(d);
    let p = dom.create_element_with_owner("div", Attributes::default(), owner);
    assert!(dom.append_child(d, p));
    if focus {
        elidex_dom_api::focus::set_focus_bit(&mut dom, Some(p));
        assert_eq!(
            elidex_dom_api::focus::current_focus(&dom, d),
            Some(p),
            "fixture precondition: D's focused area is p"
        );
    }
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let w = vm.inner.create_element_wrapper(p);
    vm.set_global("p", JsValue::Object(w));
    vm.eval(PRE).expect("preamble");
    body(&mut vm);
    vm.unbind();
}

const DP: &str = "globalThis.dp = new DOMParser().parseFromString('<p></p>', 'text/html');";

#[test]
fn w1_generic_path_document_wrapper_is_a_document() {
    with_second_document(false, |vm| {
        vm.eval("globalThis.d = p.ownerDocument;").unwrap();
        let got = eval_str(
            vm,
            "L(function () { return typeof d.createElement; }, \
               function () { return d.defaultView === null; }, \
               function () { return 'createElement' in Object.getPrototypeOf(d); }, \
               function () { return d.createElement('i').ownerDocument === d; }, \
               function () { return d.body; });",
        );
        assert_eq!(got, "\"function\"|true|true|true|null");
    });
}

#[test]
fn w2_document_members_are_not_own_properties() {
    with_page(|vm| {
        let got = eval_str(
            vm,
            "L(function () { return Object.getOwnPropertyNames(document).length; }, \
               function () { return 'createElement' in Object.getPrototypeOf(document); });",
        );
        assert_eq!(got, "0|true");
    });
}

/// `Object.getOwnPropertyNames(document)` on the base (`0a5a77ba`), in
/// enumeration order: the A1 contract "install order = enumeration order"
/// now stated on `Document.prototype`.
const PINNED_NAMES: &str = "\
    getElementById,querySelector,querySelectorAll,getElementsByTagName,getElementsByClassName,\
    getElementsByName,createElement,createTextNode,createComment,createDocumentFragment,\
    hasFocus,getSelection,adoptNode,createRange,createTreeWalker,createNodeIterator,\
    documentElement,head,body,firstElementChild,lastElementChild,children,childElementCount,\
    URL,documentURI,baseURI,readyState,compatMode,defaultView,doctype,referrer,forms,images,\
    links,activeElement,hidden,visibilityState,styleSheets,title,cookie,prepend,append,\
    replaceChildren,onabort,onauxclick,onbeforeinput,onbeforematch,onbeforetoggle,onblur,\
    oncancel,oncanplay,oncanplaythrough,onchange,onclick,onclose,oncontextlost,oncontextmenu,\
    oncontextrestored,oncuechange,ondblclick,ondrag,ondragend,ondragenter,ondragleave,\
    ondragover,ondragstart,ondrop,ondurationchange,onemptied,onended,onerror,onfocus,\
    onformdata,ongotpointercapture,oninput,oninvalid,onkeydown,onkeypress,onkeyup,onload,\
    onloadeddata,onloadedmetadata,onloadstart,onlostpointercapture,onmousedown,onmouseenter,\
    onmouseleave,onmousemove,onmouseout,onmouseover,onmouseup,onpause,onplay,onplaying,\
    onpointercancel,onpointerdown,onpointerenter,onpointerleave,onpointermove,onpointerout,\
    onpointerover,onpointerup,onprogress,onratechange,onreset,onresize,onscroll,onscrollend,\
    onsecuritypolicyviolation,onseeked,onseeking,onselect,onslotchange,onstalled,onsubmit,\
    onsuspend,ontimeupdate,ontoggle,ontransitioncancel,ontransitionend,ontransitionrun,\
    ontransitionstart,onvolumechange,onwaiting,onwheel,oncopy,oncut,onpaste,onreadystatechange,\
    onvisibilitychange,__a1LegacyHandlerProbe";

#[test]
fn w3_prototype_member_order_is_the_pinned_list() {
    with_page(|vm| {
        let got = eval_str(
            vm,
            "Object.getOwnPropertyNames(Object.getPrototypeOf(document)).join(',');",
        );
        assert_eq!(got, PINNED_NAMES);
    });
}

#[test]
fn w4_identity_guards() {
    with_second_document(false, |vm| {
        vm.eval(DP).unwrap();
        let got = eval_str(
            vm,
            "L(function () { return p.ownerDocument === p.ownerDocument; }, \
               function () { return dp.documentElement.parentNode === dp; }, \
               function () { return document.documentElement.parentNode === document; });",
        );
        assert_eq!(got, "true|true|true");
    });
}

#[test]
fn w5_domparser_and_clone_share_the_prototype() {
    with_page(|vm| {
        vm.eval(DP).unwrap();
        let got = eval_str(
            vm,
            "var c = document.cloneNode(true); \
             L(function () { return [Object.getPrototypeOf(dp), Object.getPrototypeOf(c)] \
                  .every(function (x) { return x === Object.getPrototypeOf(document); }); }, \
               function () { return typeof dp.createElement; }, \
               function () { return typeof c.createElement; });",
        );
        assert_eq!(got, "true|\"function\"|\"function\"");
    });
}

#[cfg(feature = "compat-webapi")]
#[test]
fn w7_domparser_document_no_browsing_context_values_guard() {
    with_page(|vm| {
        vm.eval(DP).unwrap();
        let got = eval_str(
            vm,
            "L(function () { return dp.defaultView; }, \
               function () { return dp.cookie; }, \
               function () { return (dp.cookie = 'a=b', dp.cookie); }, \
               function () { return dp.referrer; }, \
               function () { return dp.hasFocus(); }, \
               function () { return dp.readyState; }, \
               function () { return dp.compatMode; });",
        );
        assert_eq!(got, "null|\"\"|\"\"|\"\"|false|\"complete\"|\"CSS1Compat\"");
    });
}

#[test]
fn w8_bound_document_unchanged_guard() {
    with_page(|vm| {
        let got = eval_str(
            vm,
            "L(function () { return document.defaultView === globalThis; }, \
               function () { return document.hidden; }, \
               function () { return document.visibilityState; }, \
               function () { return document.getSelection() === getSelection(); }, \
               function () { return document.compatMode; }, \
               function () { return document.readyState; });",
        );
        assert_eq!(
            got,
            "true|false|\"visible\"|true|\"CSS1Compat\"|\"complete\""
        );
    });
}

/// Name and message of the error `src` throws, or `returned:<value>`.
const ERR: &str = "globalThis.E = function (f) { \
  try { return 'returned:' + F(f()); } catch (e) { return e.name + ': ' + e.message; } };";

#[test]
fn w9_accessor_brand_check_rejects_a_non_document() {
    with_page(|vm| {
        vm.eval(ERR).unwrap();
        let got = eval_str(
            vm,
            "var P = Object.getOwnPropertyDescriptor(document, 'body') ? document \
                      : Object.getPrototypeOf(document); \
             E(function () { return Object.getOwnPropertyDescriptor(P, 'body').get.call(document.body); });",
        );
        assert!(
            got.starts_with("TypeError: ") && got.contains("Illegal invocation"),
            "got {got:?}"
        );
    });
}

#[test]
fn w11_rebind_keeps_the_prototype_and_installs_no_own_members() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc1 = page(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc1);
    }
    vm.eval("globalThis.saved = Object.getPrototypeOf(document);")
        .unwrap();
    vm.eval(PRE).unwrap();
    vm.unbind();

    let doc2 = page(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc2);
    }
    let got = eval_str(
        &mut vm,
        "L(function () { return typeof document.getElementById; }, \
           function () { return Object.getPrototypeOf(document) === saved; }, \
           function () { return document.hasOwnProperty('getElementById'); });",
    );
    assert_eq!(got, "\"function\"|true|false");
    vm.unbind();
}

#[test]
fn w12_gates_guard() {
    with_page(|vm| {
        let got = eval_str(
            vm,
            "L(function () { return 'getElementsByTagName' in document; }, \
               function () { return 'cookie' in document; });",
        );
        #[cfg(feature = "compat-webapi")]
        assert_eq!(got, "true|true");
        #[cfg(not(feature = "compat-webapi"))]
        assert_eq!(got, "true|false");
    });

    // An App-mode VM: `document.cookie` is not installed.
    let mut vm = Vm::new_with_mode(elidex_plugin::EngineMode::App);
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = page(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(PRE).unwrap();
    let got = eval_str(&mut vm, "L(function () { return 'cookie' in document; });");
    assert_eq!(got, "false");
    vm.unbind();
}
