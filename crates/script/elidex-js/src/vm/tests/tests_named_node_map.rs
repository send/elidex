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

// --- Attr identity (WHATWG DOM §4.9.2 — SP5) ---------------------

#[test]
fn get_attribute_node_preserves_identity_across_calls() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         (d.getAttributeNode('id') === d.getAttributeNode('id')) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn get_attribute_node_identity_matches_named_node_map_paths() {
    // `getAttributeNode`, `attributes.getNamedItem`, `attributes.item`,
    // and the indexed/named property accesses on the NamedNodeMap all
    // resolve to the same Attr wrapper for a given (element, name).
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         var via_get = d.getAttributeNode('id'); \
         var via_named = d.attributes.getNamedItem('id'); \
         var via_item = d.attributes.item(0); \
         var via_indexed = d.attributes[0]; \
         var via_keyed = d.attributes['id']; \
         (via_get === via_named && via_named === via_item \
          && via_item === via_indexed && via_indexed === via_keyed) \
             ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn get_attribute_node_identity_survives_set_attribute_value_mutation() {
    // `setAttribute` only mutates the attribute's value; the cached
    // wrapper observes the new value transparently and identity is
    // preserved.
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         var a = d.getAttributeNode('id'); \
         d.setAttribute('id', 'y'); \
         (a === d.getAttributeNode('id') && a.value === 'y') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn remove_attribute_invalidates_identity() {
    // After `removeAttribute` + `setAttribute` (re-adding the same
    // name), `getAttributeNode` returns a fresh wrapper distinct
    // from any caller-held handle to the prior incarnation.  The
    // prior wrapper's `.value` re-reads through to the current
    // owner attribute (Phase 2 detachment-on-remove is out of
    // scope for SP5), so identity is the only invariant under test
    // here.
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         var a = d.getAttributeNode('id'); \
         d.removeAttribute('id'); \
         d.setAttribute('id', 'y'); \
         var b = d.getAttributeNode('id'); \
         (a !== b && b.value === 'y') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn remove_attribute_node_invalidates_identity() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         var a = d.getAttributeNode('id'); \
         d.removeAttributeNode(a); \
         d.setAttribute('id', 'y'); \
         (a !== d.getAttributeNode('id')) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn remove_named_item_invalidates_identity() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         var a = d.getAttributeNode('id'); \
         d.attributes.removeNamedItem('id'); \
         d.setAttribute('id', 'y'); \
         (a !== d.getAttributeNode('id')) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn toggle_attribute_off_invalidates_identity() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('hidden', ''); \
         var a = d.getAttributeNode('hidden'); \
         d.toggleAttribute('hidden', false); \
         d.toggleAttribute('hidden', true); \
         (a !== d.getAttributeNode('hidden')) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn set_attribute_node_self_preserves_identity() {
    // `el.setAttributeNode(el.getAttributeNode(name))` — passing
    // the live wrapper for the same `(element, name)` back in
    // must not invalidate the cache, since its backing state is
    // already canonical.
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         var a = d.getAttributeNode('id'); \
         d.setAttributeNode(a); \
         (d.getAttributeNode('id') === a) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn set_named_item_self_preserves_identity() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         var a = d.getAttributeNode('id'); \
         d.attributes.setNamedItem(a); \
         (d.getAttributeNode('id') === a) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn set_attribute_node_reattach_after_remove_preserves_identity() {
    // Reattachment sequence:
    //   1. `getAttributeNode` populates the cache with `a`.
    //   2. `removeAttribute` empties the cache.
    //   3. `setAttributeNode(a)` — `a` is still a live wrapper for
    //      this `(element, name)`, so the cache must be repopulated
    //      to point at `a` (rather than left empty, which would
    //      cause the next `getAttributeNode` to allocate fresh).
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         var a = d.getAttributeNode('id'); \
         d.removeAttribute('id'); \
         d.setAttributeNode(a); \
         (d.getAttributeNode('id') === a) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn set_named_item_reattach_after_remove_preserves_identity() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         var a = d.getAttributeNode('id'); \
         d.attributes.removeNamedItem('id'); \
         d.attributes.setNamedItem(a); \
         (d.getAttributeNode('id') === a) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn set_attribute_node_from_other_element_invalidates_cache() {
    // Passing an Attr from a *different* element cannot retarget
    // its `AttrState.owner`, so the cache must drop and the next
    // `getAttributeNode` allocate a fresh canonical wrapper.
    let out = run("var src = document.createElement('div'); \
         var dst = document.createElement('div'); \
         src.setAttribute('id', 'x'); \
         dst.setAttribute('id', 'y'); \
         var dst_before = dst.getAttributeNode('id'); \
         dst.setAttributeNode(src.getAttributeNode('id')); \
         (dst.getAttributeNode('id') !== dst_before) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn reflected_boolean_setter_invalidates_identity_cache() {
    // `el.hidden = false` removes the `hidden` attribute and must
    // invalidate the identity cache so a subsequent `el.hidden =
    // true; el.getAttributeNode("hidden")` returns a fresh
    // canonical wrapper rather than the stale one cached before
    // the boolean-setter detach.
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('hidden', ''); \
         var a = d.getAttributeNode('hidden'); \
         d.hidden = false; \
         d.hidden = true; \
         (a !== d.getAttributeNode('hidden')) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn distinct_elements_and_names_have_distinct_identities() {
    let out = run("var a = document.createElement('div'); \
         var b = document.createElement('div'); \
         a.setAttribute('id', 'x'); \
         b.setAttribute('id', 'x'); \
         a.setAttribute('class', 'y'); \
         var ai = a.getAttributeNode('id'); \
         var bi = b.getAttributeNode('id'); \
         var ac = a.getAttributeNode('class'); \
         (ai !== bi && ai !== ac && bi !== ac) ? 'ok' : 'fail';");
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

// ---------------------------------------------------------------------------
// Copilot R2 #5 regression — `attrs['0']` (numeric-string key) must
// route through the indexed path, same as `attrs[0]`.  Previously
// string keys were treated as attribute-name lookups only.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Copilot R16 #2+#3 regression — NamedNodeMap methods and Attr
// accessors must not panic when invoked on wrappers retained
// across `Vm::unbind()`.
// ---------------------------------------------------------------------------

#[test]
fn named_node_map_methods_after_unbind_return_safe_defaults() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         globalThis.nnm = d.attributes; \
         globalThis.attr = d.getAttributeNode('id');",
    )
    .unwrap();
    vm.unbind();

    // NamedNodeMap `.length` → 0
    let r = vm.eval("globalThis.nnm.length;").unwrap();
    assert!(matches!(r, JsValue::Number(n) if n == 0.0), "{r:?}");
    // `.item(0)` → null
    let r = vm.eval("globalThis.nnm.item(0);").unwrap();
    assert!(matches!(r, JsValue::Null), "{r:?}");
    // `.getNamedItem('id')` → null
    let r = vm.eval("globalThis.nnm.getNamedItem('id');").unwrap();
    assert!(matches!(r, JsValue::Null), "{r:?}");
    // Attr `.ownerElement` → null
    let r = vm.eval("globalThis.attr.ownerElement;").unwrap();
    assert!(matches!(r, JsValue::Null), "{r:?}");
    // Attr `.value` → empty string (live Attr, former owner now
    // unbound)
    let r = vm.eval("globalThis.attr.value;").unwrap();
    if let JsValue::String(sid) = r {
        assert_eq!(vm.inner.strings.get_utf8(sid), "");
    } else {
        panic!("expected String, got {r:?}");
    }
    // Attr setter is a no-op (does not panic, does not crash)
    vm.eval("globalThis.attr.value = 'newval';").unwrap();
}

#[test]
fn named_node_map_numeric_string_index_returns_attr() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         d.setAttribute('data-y', 'z'); \
         var attrs = d.attributes; \
         var byNumber = attrs[0]; \
         var byString = attrs['0']; \
         (byNumber && byString && byNumber.name === byString.name) \
           ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// ---------------------------------------------------------------------------
// Copilot R3 #1 regression — non-canonical numeric strings ("01" /
// "+1" / "1.0") MUST NOT alias attribute indices.  ES §7.1.21
// canonical-numeric-index-string parsing rejects leading zeros
// (except "0"), so `attrs['01']` must fall through to
// attribute-name lookup.
// ---------------------------------------------------------------------------

#[test]
fn named_node_map_non_canonical_numeric_string_does_not_alias_index() {
    // Two attrs: `id` at index 0, `data-y` at index 1.  `attrs[1]`
    // returns the `data-y` Attr; `attrs['01']` must NOT — it's a
    // non-canonical index, and no attribute is literally named
    // `"01"`, so lookup returns undefined.
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         d.setAttribute('data-y', 'z'); \
         var attrs = d.attributes; \
         (attrs['01'] === undefined) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// ---------------------------------------------------------------------------
// Copilot R10 #3 regression — `setNamedItem(attr)` returned "prev"
// must be a detached snapshot of the REPLACED value, not a live
// view that observes the just-written value.
// ---------------------------------------------------------------------------

#[test]
fn set_named_item_returns_detached_previous_value() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'old'); \
         var src = document.createElement('span'); \
         src.setAttribute('id', 'new'); \
         var srcAttr = src.getAttributeNode('id'); \
         var prev = d.attributes.setNamedItem(srcAttr); \
         var prevValue = prev.value; \
         var currentValue = d.getAttribute('id'); \
         var prevOwner = prev.ownerElement; \
         (prevValue === 'old' && currentValue === 'new' && prevOwner === null) \
           ? 'ok' : 'fail:' + prevValue + '/' + currentValue + '/' + (prevOwner === null);");
    assert_eq!(out, "ok");
}

// ---------------------------------------------------------------------------
// Copilot R10 #4 regression — a detached Attr (returned by
// removeNamedItem / removeAttributeNode / setNamedItem-prev) MUST
// stay detached: subsequent same-name `setAttribute` on the former
// owner must NOT make the Attr appear to re-attach.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Copilot R10 #1 regression — `element.setAttributeNode(new)`
// returned "prev" Attr must be a detached snapshot of the
// REPLACED value, parallel to R10 #3 (setNamedItem).
// ---------------------------------------------------------------------------

#[test]
fn element_set_attribute_node_returns_detached_previous_value() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'old'); \
         var src = document.createElement('span'); \
         src.setAttribute('id', 'new'); \
         var srcAttr = src.getAttributeNode('id'); \
         var prev = d.setAttributeNode(srcAttr); \
         (prev.value === 'old' && d.getAttribute('id') === 'new' \
          && prev.ownerElement === null) \
           ? 'ok' : 'fail:' + prev.value + '/' + d.getAttribute('id');");
    assert_eq!(out, "ok");
}

// ---------------------------------------------------------------------------
// Copilot R10 #2 regression — `element.removeAttributeNode(attr)`
// must throw `NotFoundError` when `attr` is not attached to the
// receiver, even if the receiver has a same-named attribute of
// its own.  Pre-R10 the owner check was missing and the wrong
// attribute was removed.
// ---------------------------------------------------------------------------

#[test]
fn element_remove_attribute_node_rejects_cross_element_attr() {
    let out = run("var a = document.createElement('div'); \
         a.setAttribute('id', 'a-id'); \
         var b = document.createElement('div'); \
         b.setAttribute('id', 'b-id'); \
         var attrA = a.getAttributeNode('id'); \
         var caught = null; \
         try { b.removeAttributeNode(attrA); } \
         catch (e) { caught = e.name; } \
         var aStillHas = a.getAttribute('id'); \
         var bStillHas = b.getAttribute('id'); \
         (caught === 'NotFoundError' \
          && aStillHas === 'a-id' && bStillHas === 'b-id') \
           ? 'ok' : 'fail:' + caught + '/' + aStillHas + '/' + bStillHas;");
    assert_eq!(out, "ok");
}

#[test]
fn element_remove_attribute_node_detaches_input_attr() {
    // The passed Attr itself becomes detached after
    // `removeAttributeNode` — subsequent `.value` returns the
    // snapshot captured at removal time, `ownerElement` returns
    // null, and a same-name `setAttribute` on the former owner
    // does NOT re-attach.
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'initial'); \
         var attr = d.getAttributeNode('id'); \
         var removed = d.removeAttributeNode(attr); \
         var sameObject = removed === attr; \
         var afterRemove = attr.value; \
         d.setAttribute('id', 'later'); \
         var afterReset = attr.value; \
         (sameObject && afterRemove === 'initial' && afterReset === 'initial' \
          && attr.ownerElement === null) \
           ? 'ok' : 'fail:' + afterRemove + '/' + afterReset;");
    assert_eq!(out, "ok");
}

// ---------------------------------------------------------------------------
// Copilot R12 #1 regression — `setNamedItem(detachedAttr)` /
// `setAttributeNode(detachedAttr)` must write the detached
// snapshot as the attribute value, not re-read from the source
// Attr's former owner (which would yield empty / stale data).
// Mirrors `Attr.prototype.value` precedence.
// ---------------------------------------------------------------------------

#[test]
fn set_named_item_uses_detached_snapshot_as_value() {
    // Source Attr is detached with value "snapshot".  Target has
    // no prior 'id'.  Result must be target.id === 'snapshot'.
    let out = run("var src = document.createElement('span'); \
         src.setAttribute('id', 'snapshot'); \
         var detached = src.removeAttributeNode(src.getAttributeNode('id')); \
         var target = document.createElement('div'); \
         target.attributes.setNamedItem(detached); \
         (target.getAttribute('id') === 'snapshot') \
           ? 'ok' : 'fail:' + target.getAttribute('id');");
    assert_eq!(out, "ok");
}

#[test]
fn element_set_attribute_node_uses_detached_snapshot_as_value() {
    let out = run("var src = document.createElement('span'); \
         src.setAttribute('data-v', 'kept'); \
         var detached = src.removeAttributeNode(src.getAttributeNode('data-v')); \
         var target = document.createElement('div'); \
         target.setAttributeNode(detached); \
         (target.getAttribute('data-v') === 'kept') \
           ? 'ok' : 'fail:' + target.getAttribute('data-v');");
    assert_eq!(out, "ok");
}

#[test]
fn removed_attr_stays_detached_after_same_name_set() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('id', 'first'); \
         var removed = d.attributes.removeNamedItem('id'); \
         var snapshotBefore = removed.value; \
         var ownerBefore = removed.ownerElement; \
         d.setAttribute('id', 'second'); \
         var snapshotAfter = removed.value; \
         var ownerAfter = removed.ownerElement; \
         (snapshotBefore === 'first' && ownerBefore === null \
          && snapshotAfter === 'first' && ownerAfter === null) \
           ? 'ok' : 'fail:' + snapshotBefore + '/' + snapshotAfter;");
    assert_eq!(out, "ok");
}

// ---------------------------------------------------------------------------
// R2 #3 regression — `Vm::unbind` must clear the Entity-keyed
// `attr_wrapper_cache` so a rebind to a fresh `EcsDom::new()` cannot
// resolve through a previous DOM's cached Attr wrapper (the two
// worlds share entity-index space).
// ---------------------------------------------------------------------------

#[test]
fn attr_wrapper_cache_cleared_on_unbind() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    vm.eval(
        "var d = document.createElement('div'); \
         d.setAttribute('id', 'x'); \
         d.getAttributeNode('id');",
    )
    .unwrap();
    assert!(
        !vm.inner.attr_wrapper_cache.is_empty(),
        "attr_wrapper_cache should be populated after getAttributeNode"
    );
    vm.unbind();
    assert!(
        vm.inner.attr_wrapper_cache.is_empty(),
        "attr_wrapper_cache must be empty after unbind to avoid cross-DOM aliasing"
    );
    assert!(
        vm.inner.class_list_wrapper_cache.is_empty(),
        "class_list_wrapper_cache must be empty after unbind"
    );
    assert!(
        vm.inner.dataset_wrapper_cache.is_empty(),
        "dataset_wrapper_cache must be empty after unbind"
    );
}
