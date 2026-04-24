//! PR5b §C3 — Live `HTMLCollection` + `NodeList` class tests.
//!
//! Covers:
//! - prototype identity + instance-of surfaces (`children` is
//!   `HTMLCollection`, `childNodes` is `NodeList`).
//! - live semantics for HTMLCollection (every read re-traverses).
//! - static semantics for `querySelectorAll` (snapshot at call
//!   time).
//! - `length` / `item` / `namedItem` (with HTML tag allowlist for
//!   the name fallback) / `forEach` / indexed property access /
//!   `[Symbol.iterator]`.
//! - `document.getElementsByName` (new — live NodeList, WHATWG
//!   §3.1.5).

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

fn run(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let result = vm.eval(script).unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}")
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    out
}

// --- HTMLCollection live semantics --------------------------------

#[test]
fn children_is_live() {
    let out = run("var p = document.createElement('div'); \
         document.body.appendChild(p); \
         var coll = p.children; \
         var beforeLen = coll.length; \
         p.appendChild(document.createElement('span')); \
         var afterLen = coll.length; \
         (beforeLen === 0 && afterLen === 1) ? 'ok' : 'fail:' + beforeLen + '/' + afterLen;");
    assert_eq!(out, "ok");
}

#[test]
fn get_elements_by_tag_name_is_live() {
    let out = run("var p = document.createElement('div'); \
         document.body.appendChild(p); \
         var coll = document.getElementsByTagName('span'); \
         var before = coll.length; \
         p.appendChild(document.createElement('span')); \
         p.appendChild(document.createElement('span')); \
         var after = coll.length; \
         (before === 0 && after === 2) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn get_elements_by_class_name_is_live() {
    let out = run("var p = document.createElement('div'); \
         document.body.appendChild(p); \
         var coll = p.getElementsByClassName('foo'); \
         var before = coll.length; \
         var s = document.createElement('span'); s.setAttribute('class', 'foo bar'); \
         p.appendChild(s); \
         var after = coll.length; \
         (before === 0 && after === 1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn forms_images_links_are_live_html_collections() {
    let out = run(
        "var formsBefore = document.forms.length; \
         var imagesBefore = document.images.length; \
         var linksBefore = document.links.length; \
         var f = document.createElement('form'); \
         var i = document.createElement('img'); \
         var a = document.createElement('a'); a.setAttribute('href', '#'); \
         document.body.appendChild(f); \
         document.body.appendChild(i); \
         document.body.appendChild(a); \
         var after = document.forms.length + document.images.length + document.links.length; \
         (formsBefore + imagesBefore + linksBefore === 0 && after === 3) ? 'ok' : 'fail:' + after;",
    );
    assert_eq!(out, "ok");
}

// --- Indexed + named property access ------------------------------

#[test]
fn html_collection_indexed_access() {
    // Indexed access semantics (§4.2.10): `coll[i]` returns the
    // item or `undefined` for out-of-range — contrasting with
    // `coll.item(i)` which returns `null`.
    let out = run("var p = document.createElement('div'); \
         var a = document.createElement('span'); \
         var b = document.createElement('span'); \
         p.appendChild(a); p.appendChild(b); \
         document.body.appendChild(p); \
         var c = p.children; \
         (c[0] === a && c[1] === b && c[2] === undefined) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn html_collection_named_item_prefers_id_then_name_allowlist() {
    let out = run("var p = document.createElement('div'); \
         var a = document.createElement('form'); a.setAttribute('name', 'foo'); \
         var b = document.createElement('div'); b.setAttribute('id', 'foo'); \
         var c = document.createElement('div'); c.setAttribute('name', 'bar'); \
         p.appendChild(a); p.appendChild(b); p.appendChild(c); \
         document.body.appendChild(p); \
         var byIdOverName = p.children.namedItem('foo'); \
         var divWithNameIgnored = p.children.namedItem('bar'); \
         (byIdOverName === b && divWithNameIgnored === null) ? 'ok' \
             : 'fail:' + (byIdOverName === b) + '/' + (divWithNameIgnored === null);");
    assert_eq!(out, "ok");
}

#[test]
fn html_collection_named_access_via_indexed_string() {
    let out = run("var p = document.createElement('div'); \
         var a = document.createElement('div'); a.setAttribute('id', 'foo'); \
         p.appendChild(a); \
         document.body.appendChild(p); \
         (p.children['foo'] === a) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// --- NodeList --------------------------------------------------

#[test]
fn child_nodes_is_live_node_list_not_array() {
    // Before PR5b, `childNodes` returned a plain Array (not
    // an instance of any collection class).  Now it must be a
    // NodeList wrapper.
    let out = run("var p = document.createElement('div'); \
         p.appendChild(document.createTextNode('a')); \
         document.body.appendChild(p); \
         var nl = p.childNodes; \
         var nlIsNotArray = !Array.isArray(nl); \
         var lenBefore = nl.length; \
         p.appendChild(document.createTextNode('b')); \
         var lenAfter = nl.length; \
         (nlIsNotArray && lenBefore === 1 && lenAfter === 2) \
           ? 'ok' : 'fail:' + nlIsNotArray + '/' + lenBefore + '/' + lenAfter;");
    assert_eq!(out, "ok");
}

#[test]
fn query_selector_all_is_static_node_list() {
    // `querySelectorAll` returns a static NodeList (WHATWG §4.2.6).
    // Mutations after the call do NOT alter the returned list.
    let out = run("var p = document.createElement('div'); \
         var a = document.createElement('span'); \
         p.appendChild(a); \
         document.body.appendChild(p); \
         var snapshot = p.querySelectorAll('span'); \
         var before = snapshot.length; \
         p.appendChild(document.createElement('span')); \
         var after = snapshot.length; \
         (before === 1 && after === 1) ? 'ok' : 'fail:' + before + '/' + after;");
    assert_eq!(out, "ok");
}

#[test]
fn node_list_for_each() {
    // `p.childNodes` allocates a fresh wrapper per access (identity
    // is not preserved across reads — per the per-access re-
    // traversal design), so cache the wrapper in a local before
    // comparing `list === nl` inside the callback.
    let out = run("var p = document.createElement('div'); \
         var a = document.createElement('span'); \
         var b = document.createElement('span'); \
         p.appendChild(a); p.appendChild(b); \
         document.body.appendChild(p); \
         var nl = p.childNodes; \
         var count = 0; var lastIndex = -1; var listOk = true; \
         nl.forEach(function(n, i, list) { \
             count++; lastIndex = i; \
             if (list !== nl) listOk = false; \
         }); \
         (count === 2 && lastIndex === 1 && listOk) \
           ? 'ok' : 'fail:' + count + '/' + lastIndex + '/' + listOk;");
    assert_eq!(out, "ok");
}

#[test]
fn node_list_for_each_throws_on_non_callable() {
    let out = run(
        "try { document.createElement('div').childNodes.forEach(null); 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'ok' : 'bad:' + e; }",
    );
    assert_eq!(out, "ok");
}

// --- document.getElementsByName ----------------------------------

#[test]
fn get_elements_by_name_is_live_node_list() {
    let out = run(
        "var a = document.createElement('input'); a.setAttribute('name', 'x'); \
         document.body.appendChild(a); \
         var nl = document.getElementsByName('x'); \
         var before = nl.length; \
         var b = document.createElement('input'); b.setAttribute('name', 'x'); \
         document.body.appendChild(b); \
         var after = nl.length; \
         (before === 1 && after === 2 && nl[0] === a && nl[1] === b) \
           ? 'ok' : 'fail:' + before + '/' + after;",
    );
    assert_eq!(out, "ok");
}

// --- Iterator protocol -------------------------------------------

#[test]
fn html_collection_iteration_via_spread() {
    let out = run("var p = document.createElement('div'); \
         var a = document.createElement('span'); \
         var b = document.createElement('span'); \
         p.appendChild(a); p.appendChild(b); \
         document.body.appendChild(p); \
         var arr = [...p.children]; \
         (arr.length === 2 && arr[0] === a && arr[1] === b) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn array_from_on_node_list() {
    let out = run("var p = document.createElement('div'); \
         p.appendChild(document.createElement('span')); \
         p.appendChild(document.createTextNode('t')); \
         document.body.appendChild(p); \
         var arr = Array.from(p.childNodes); \
         (arr.length === 2) ? 'ok' : 'fail:' + arr.length;");
    assert_eq!(out, "ok");
}

// --- Prototype identity ------------------------------------------

#[test]
fn html_collection_and_node_list_share_no_prototype() {
    // HTMLCollection exposes `namedItem` but not `forEach`; NodeList
    // is the reverse.  Confirm the two prototypes are distinct and
    // neither leaks the other's methods.
    let out = run("var p = document.createElement('div'); \
         p.appendChild(document.createElement('span')); \
         document.body.appendChild(p); \
         var hc = p.children; \
         var nl = p.childNodes; \
         var differentProtos = Object.getPrototypeOf(hc) !== Object.getPrototypeOf(nl); \
         var hcHasNamedItem = typeof hc.namedItem === 'function'; \
         var nlHasForEach = typeof nl.forEach === 'function'; \
         var nlHasNoNamedItem = typeof nl.namedItem !== 'function'; \
         (differentProtos && hcHasNamedItem && nlHasForEach && nlHasNoNamedItem) \
           ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// --- Brand check -------------------------------------------------

#[test]
fn collection_method_brand_check_rejects_plain_object() {
    let out = run("var p = document.createElement('div'); \
         document.body.appendChild(p); \
         var proto = Object.getPrototypeOf(p.children); \
         try { proto.item.call({}, 0); 'no-throw'; } \
         catch (e) { (e && e.message && e.message.indexOf('Illegal') >= 0) \
                        ? 'ok' : 'bad:' + (e && e.message); }");
    assert_eq!(out, "ok");
}

// --- item() out-of-range ----------------------------------------

#[test]
fn item_out_of_range_returns_null() {
    let out = run("var p = document.createElement('div'); \
         var c = document.createElement('span'); \
         p.appendChild(c); \
         document.body.appendChild(p); \
         var ok = p.children.item(0) === c \
               && p.children.item(1) === null \
               && p.children.item(-1) === null; \
         ok ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// ---------------------------------------------------------------------------
// Cross-interface brand checks (Copilot R1 #3 regression guard).
// HTMLCollection-only methods (`namedItem`) must reject NodeList
// receivers; NodeList-only methods (`forEach`) must reject
// HTMLCollection receivers.  Shared methods (`length` / `item`)
// remain accepted on both.
// ---------------------------------------------------------------------------

#[test]
fn named_item_rejects_node_list_receiver_with_illegal_invocation() {
    let out = run("var nl = document.body.childNodes; \
         var hcProto = Object.getPrototypeOf(document.body.children); \
         var named = hcProto.namedItem; \
         try { named.call(nl, 'x'); 'no-throw'; } \
         catch (e) { (e && e.message && e.message.indexOf('Illegal') >= 0) \
                        ? 'ok' : 'bad:' + (e && e.message); }");
    assert_eq!(out, "ok");
}

#[test]
fn for_each_rejects_html_collection_receiver_with_illegal_invocation() {
    let out = run("var hc = document.body.children; \
         var nlProto = Object.getPrototypeOf(document.body.childNodes); \
         var forEach = nlProto.forEach; \
         try { forEach.call(hc, function(){}); 'no-throw'; } \
         catch (e) { (e && e.message && e.message.indexOf('Illegal') >= 0) \
                        ? 'ok' : 'bad:' + (e && e.message); }");
    assert_eq!(out, "ok");
}

#[test]
fn shared_methods_accept_either_collection_receiver() {
    // `length` + `item` live on both prototypes and must work via
    // cross-receiver `.call`.
    let out = run("var p = document.createElement('div'); \
         p.appendChild(document.createElement('a')); \
         var hcProto = Object.getPrototypeOf(document.body.children); \
         var nlProto = Object.getPrototypeOf(document.body.childNodes); \
         var hcLenDesc = Object.getOwnPropertyDescriptor(hcProto, 'length'); \
         var nlLenDesc = Object.getOwnPropertyDescriptor(nlProto, 'length'); \
         var hcLenOnNl = hcLenDesc.get.call(p.childNodes); \
         var nlLenOnHc = nlLenDesc.get.call(p.children); \
         var hcItemOnNl = hcProto.item.call(p.childNodes, 0); \
         var nlItemOnHc = nlProto.item.call(p.children, 0); \
         (hcLenOnNl === 1 && nlLenOnHc === 1 \
          && hcItemOnNl !== null && nlItemOnHc !== null) \
           ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}
