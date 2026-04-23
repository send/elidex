//! PR5b §C4 + §C4.5 — `NamedNodeMap` + `Attr.prototype` tests.
//!
//! Covers:
//! - `element.attributes` returns a `NamedNodeMap` wrapper.
//! - live semantics (length / item / namedItem reflect concurrent
//!   mutations).
//! - indexed + named property access (`attrs[0]`, `attrs['id']`).
//! - `getAttributeNode` / `setAttributeNode` / `removeAttributeNode`.
//! - `Attr.prototype`: `name` / `value` / `ownerElement` /
//!   `namespaceURI` / `prefix` / `localName` / `specified`.
//! - brand check (non-NamedNodeMap / non-Attr receivers throw).
//! - `removeNamedItem` on absent key throws `NotFoundError`
//!   `DOMException`.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
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

// --- NamedNodeMap liveness ---------------------------------------

#[test]
fn attributes_reflects_live_mutations() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         var a = d.attributes; \
         var before = a.length; \
         d.setAttribute('class', 'y'); \
         var after = a.length; \
         (before === 1 && after === 2) ? 'ok' : 'fail:' + before + '/' + after;");
    assert_eq!(out, "ok");
}

#[test]
fn attributes_allocates_fresh_wrapper_per_access() {
    let out = run("var d = document.createElement('div'); \
         (d.attributes === d.attributes) ? 'same' : 'fresh';");
    assert_eq!(out, "fresh");
}

// --- item / getNamedItem / indexed access ------------------------

#[test]
fn named_node_map_item_and_indexed() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); d.setAttribute('class', 'y'); \
         var a = d.attributes; \
         var first = a.item(0).name; \
         var second = a[1].name; \
         var oob = a.item(2); \
         (first === 'id' && second === 'class' && oob === null) \
           ? 'ok' : 'fail:' + first + '/' + second + '/' + oob;");
    assert_eq!(out, "ok");
}

#[test]
fn named_node_map_get_named_item_and_named_access() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('data-foo', 'bar'); \
         var a = d.attributes; \
         var byGet = a.getNamedItem('data-foo').value; \
         var byKey = a['data-foo'].value; \
         var missing = a.getNamedItem('nope'); \
         (byGet === 'bar' && byKey === 'bar' && missing === null) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// --- setNamedItem / removeNamedItem ------------------------------

#[test]
fn set_named_item_copies_value_onto_target() {
    let out = run("var source = document.createElement('div'); \
         source.setAttribute('title', 'src-val'); \
         var target = document.createElement('div'); \
         var attr = source.getAttributeNode('title'); \
         var prev = target.attributes.setNamedItem(attr); \
         (target.getAttribute('title') === 'src-val' && prev === null) \
           ? 'ok' : 'fail:' + target.getAttribute('title') + '/' + prev;");
    assert_eq!(out, "ok");
}

#[test]
fn set_named_item_returns_previous_attr_when_replacing() {
    let out = run("var source = document.createElement('div'); \
         source.setAttribute('id', 'new'); \
         var target = document.createElement('div'); \
         target.setAttribute('id', 'old'); \
         var newAttr = source.getAttributeNode('id'); \
         var prev = target.attributes.setNamedItem(newAttr); \
         (target.getAttribute('id') === 'new' && prev !== null && prev.name === 'id') \
           ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn remove_named_item_throws_not_found_when_absent() {
    let out = run("var d = document.createElement('div'); \
         try { d.attributes.removeNamedItem('missing'); 'no-throw'; } \
         catch (e) { (e && e.name === 'NotFoundError' && e instanceof DOMException) \
             ? 'ok' : 'bad:' + (e && e.name); }");
    assert_eq!(out, "ok");
}

#[test]
fn remove_named_item_detaches_attribute() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         var removed = d.attributes.removeNamedItem('id'); \
         (d.hasAttribute('id') === false && removed.name === 'id') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// --- iteration --------------------------------------------------

#[test]
fn named_node_map_is_iterable_via_spread() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('a', '1'); d.setAttribute('b', '2'); d.setAttribute('c', '3'); \
         var names = [...d.attributes].map(function(a) { return a.name; }).join(','); \
         (names === 'a,b,c') ? 'ok' : 'fail:' + names;");
    assert_eq!(out, "ok");
}

// --- Attr accessors ---------------------------------------------

#[test]
fn attr_name_and_value_round_trip() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('data-x', 'hi'); \
         var a = d.getAttributeNode('data-x'); \
         var before = a.value; \
         a.value = 'bye'; \
         (a.name === 'data-x' && before === 'hi' && d.getAttribute('data-x') === 'bye' \
           && a.value === 'bye') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn attr_owner_element_reflects_attachment() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         var a = d.getAttributeNode('id'); \
         var ownerBefore = a.ownerElement; \
         d.removeAttribute('id'); \
         var ownerAfter = a.ownerElement; \
         var valueAfter = a.value; \
         (ownerBefore === d && ownerAfter === null && valueAfter === '') \
           ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn attr_namespace_uri_prefix_and_local_name_phase2_defaults() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('data-foo', 'bar'); \
         var a = d.getAttributeNode('data-foo'); \
         (a.namespaceURI === null && a.prefix === null && a.localName === 'data-foo' \
           && a.specified === true) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn attr_value_setter_on_detached_attr_is_noop() {
    // Once the Attr is detached (attribute removed), setting
    // `.value` should not re-attach it.  Matches browsers where
    // the detached Attr is a free-standing node until reinserted
    // via `setAttributeNode`.
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         var a = d.getAttributeNode('id'); \
         d.removeAttribute('id'); \
         a.value = 'z'; \
         (d.hasAttribute('id') === false && a.value === '') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// --- getAttributeNode / setAttributeNode / removeAttributeNode ---

#[test]
fn element_get_attribute_node_returns_null_when_absent() {
    let out = run("var d = document.createElement('div'); \
         (d.getAttributeNode('id') === null) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn element_remove_attribute_node_detaches_and_returns_wrapper() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('title', 't'); \
         var a = d.getAttributeNode('title'); \
         var returned = d.removeAttributeNode(a); \
         (d.hasAttribute('title') === false && returned.name === 'title') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn element_remove_attribute_node_throws_when_not_attached() {
    let out = run("var a = document.createElement('div'); \
         var b = document.createElement('div'); \
         a.setAttribute('id', 'x'); \
         var attr = a.getAttributeNode('id'); \
         try { b.removeAttributeNode(attr); 'no-throw'; } \
         catch (e) { (e && e.name === 'NotFoundError') ? 'ok' : 'bad:' + (e && e.name); }");
    assert_eq!(out, "ok");
}

// --- Brand checks -----------------------------------------------

#[test]
fn named_node_map_method_brand_check_rejects_plain_object() {
    let out = run(
        "var proto = Object.getPrototypeOf(document.createElement('div').attributes); \
         try { proto.item.call({}, 0); 'no-throw'; } \
         catch (e) { (e && e.message && e.message.indexOf('Illegal') >= 0) \
             ? 'ok' : 'bad:' + (e && e.message); }",
    );
    assert_eq!(out, "ok");
}

#[test]
fn attr_accessor_brand_check_rejects_plain_object() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         var attr = d.getAttributeNode('id'); \
         var proto = Object.getPrototypeOf(attr); \
         var getter = Object.getOwnPropertyDescriptor(proto, 'value').get; \
         try { getter.call({}); 'no-throw'; } \
         catch (e) { (e && e.message && e.message.indexOf('Illegal') >= 0) \
             ? 'ok' : 'bad:' + (e && e.message); }");
    assert_eq!(out, "ok");
}
