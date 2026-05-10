//! D-4 `#11-tags-T2a-url-bearing` — per-element prototype + URL accessor
//! mixin + DOMTokenList sources + enumerated/numeric reflect tests.
//!
//! Coverage matches the D-4 plan memo §C5 surface:
//! - per-element brand check + reflect round-trip
//! - URL accessor 11-property round-trip + `toString()` (IMP-1)
//! - getter-on-foreign-receiver TypeError
//! - base URL behaviour pin (`"about:blank"` ⇒ getters return `""` for
//!   relative href)
//! - relList shared identity across anchor / area / link
//! - enumerated reflect canonical pin
//! - URL setter href round-trip
//! - `<link>.sheet === null` paranoid pin (defer scope lock)
//! - iframe sandbox 既存挙動 regression
//! - `<link>.sizes` `[SameObject, PutForwards=value]`
//! - `<a>.text` accessor (textContent alias, IMP-2)
//! - `<img>.width` numeric reflect (engine-indep parse_unsigned_long)

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_empty_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
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
    let doc = build_empty_doc(&mut dom);
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

// =====================================================================
// Per-element prototype identity
// =====================================================================

#[test]
fn anchor_has_distinct_prototype() {
    let out = run("var a = document.createElement('a'); \
         var b = document.createElement('a'); \
         var same = Object.getPrototypeOf(a) === Object.getPrototypeOf(b); \
         var hasHref = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(a), 'href') !== undefined; \
         (same && hasHref) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn area_has_distinct_prototype() {
    let out = run("var a = document.createElement('area'); \
         var hasShape = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(a), 'shape') !== undefined; \
         hasShape ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn image_has_distinct_prototype() {
    let out = run("var img = document.createElement('img'); \
         var hasNaturalWidth = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(img), 'naturalWidth') !== undefined; \
         hasNaturalWidth ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn script_has_distinct_prototype() {
    let out = run("var s = document.createElement('script'); \
         var hasAsync = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(s), 'async') !== undefined; \
         hasAsync ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn link_has_distinct_prototype() {
    let out = run("var l = document.createElement('link'); \
         var hasRelList = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(l), 'relList') !== undefined; \
         var hasSheet = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(l), 'sheet') !== undefined; \
         (hasRelList && hasSheet) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// URL accessor 11-property round-trip + toString()
// =====================================================================

#[test]
fn anchor_url_accessor_full_decomposition() {
    let out = run("var a = document.createElement('a'); \
         a.href = 'https://user:pass@example.com:8443/path?q=1#frag'; \
         a.protocol + '|' + a.host + '|' + a.hostname + '|' + a.port + '|' + \
         a.pathname + '|' + a.search + '|' + a.hash + '|' + a.username + '|' + a.password;");
    assert_eq!(
        out,
        "https:|example.com:8443|example.com|8443|/path|?q=1|#frag|user|pass"
    );
}

#[test]
fn anchor_origin_is_readonly() {
    // No setter installed for `origin` — attempted assignment throws in
    // strict mode (silent no-op in sloppy).  Either way the value is
    // unchanged.  Use try/catch to tolerate the strict-mode throw and
    // assert the post-state.
    let out = run("var a = document.createElement('a'); \
         a.href = 'https://example.com/'; \
         var before = a.origin; \
         try { a.origin = 'https://other.com'; } catch (e) {} \
         var after = a.origin; \
         (before === after && before === 'https://example.com') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn anchor_to_string_returns_href() {
    let out = run("var a = document.createElement('a'); \
         a.href = 'https://example.com/path'; \
         (a.toString() === a.href) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn anchor_protocol_setter_updates_href_attr() {
    // Lesson #181: setter writes back through getAttribute('href').
    let out = run("var a = document.createElement('a'); \
         a.href = 'http://example.com/'; \
         a.protocol = 'https'; \
         var attr = a.getAttribute('href'); \
         attr;");
    assert_eq!(out, "https://example.com/");
}

#[test]
fn anchor_host_setter_round_trip() {
    // MIN-2: explicit URL setter → href attr round-trip.
    let out = run("var a = document.createElement('a'); \
         a.href = 'https://example.com/'; \
         a.host = 'newhost:8080'; \
         a.getAttribute('href');");
    assert_eq!(out, "https://newhost:8080/");
}

// =====================================================================
// Base URL behaviour pin (about:blank — relative href returns empty)
// =====================================================================

#[test]
fn anchor_relative_href_unparseable_getters_return_empty() {
    // about:blank is opaque-origin so relative resolution fails;
    // WHATWG URL §6.2 specifies getters return "" on parse failure.
    let out = run("var a = document.createElement('a'); \
         a.href = '/relative/path'; \
         a.protocol + '|' + a.host + '|' + a.pathname;");
    assert_eq!(out, "||");
}

#[test]
fn anchor_empty_href_getters_return_empty() {
    let out = run("var a = document.createElement('a'); \
         a.protocol + '|' + a.host + '|' + a.href;");
    assert_eq!(out, "||");
}

// =====================================================================
// Foreign-receiver TypeError brand check
// =====================================================================

#[test]
fn anchor_href_getter_on_div_throws() {
    let out = run("var d = document.createElement('div'); \
         var anchorHrefDesc = Object.getOwnPropertyDescriptor(\
             Object.getPrototypeOf(document.createElement('a')), 'href'); \
         try { anchorHrefDesc.get.call(d); 'no-throw'; } catch (e) { 'TypeError'; }");
    assert_eq!(out, "TypeError");
}

#[test]
fn area_shape_getter_on_div_throws() {
    let out = run("var d = document.createElement('div'); \
         var shapeDesc = Object.getOwnPropertyDescriptor(\
             Object.getPrototypeOf(document.createElement('area')), 'shape'); \
         try { shapeDesc.get.call(d); 'no-throw'; } catch (e) { 'TypeError'; }");
    assert_eq!(out, "TypeError");
}

// =====================================================================
// relList — shared prototype, separate identity per Entity
// =====================================================================

#[test]
fn anchor_rel_list_same_object_per_element() {
    let out = run("var a = document.createElement('a'); \
         (a.relList === a.relList) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn anchor_rel_list_distinct_per_element() {
    let out = run("var a = document.createElement('a'); \
         var b = document.createElement('a'); \
         (a.relList !== b.relList) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn rel_list_shared_prototype_across_anchor_area_link() {
    // CRIT-2 Option A: separate Entity-keyed caches but shared
    // DOMTokenList.prototype across anchor / area / link.
    let out = run("var a = document.createElement('a'); \
         var ar = document.createElement('area'); \
         var l = document.createElement('link'); \
         var p1 = Object.getPrototypeOf(a.relList); \
         var p2 = Object.getPrototypeOf(ar.relList); \
         var p3 = Object.getPrototypeOf(l.relList); \
         (p1 === p2 && p2 === p3) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn rel_list_add_remove_round_trip_anchor() {
    let out = run("var a = document.createElement('a'); \
         a.relList.add('noopener'); \
         a.relList.add('noreferrer'); \
         var attr = a.getAttribute('rel'); \
         a.relList.remove('noopener'); \
         attr + '|' + a.getAttribute('rel');");
    assert_eq!(out, "noopener noreferrer|noreferrer");
}

#[test]
fn rel_list_round_trip_link() {
    let out = run("var l = document.createElement('link'); \
         l.relList.add('stylesheet'); \
         l.getAttribute('rel');");
    assert_eq!(out, "stylesheet");
}

// =====================================================================
// Enumerated reflect canonical pin
// =====================================================================

#[test]
fn anchor_referrer_policy_invalid_returns_empty() {
    let out = run("var a = document.createElement('a'); \
         a.setAttribute('referrerpolicy', 'bogus'); \
         a.referrerPolicy;");
    assert_eq!(out, "");
}

#[test]
fn anchor_referrer_policy_canonical_pass_through() {
    let out = run("var a = document.createElement('a'); \
         a.setAttribute('referrerpolicy', 'origin'); \
         a.referrerPolicy;");
    assert_eq!(out, "origin");
}

#[test]
fn anchor_referrer_policy_ascii_case_insensitive() {
    let out = run("var a = document.createElement('a'); \
         a.setAttribute('referrerpolicy', 'ORIGIN'); \
         a.referrerPolicy;");
    assert_eq!(out, "origin");
}

#[test]
fn area_shape_invalid_returns_rect() {
    // IMP-4: invalid default is `rect`.
    let out = run("var a = document.createElement('area'); \
         a.setAttribute('shape', 'triangle'); \
         a.shape;");
    assert_eq!(out, "rect");
}

#[test]
fn area_shape_missing_returns_rect() {
    let out = run("var a = document.createElement('area'); \
         a.shape;");
    assert_eq!(out, "rect");
}

#[test]
fn image_loading_invalid_returns_eager() {
    let out = run("var img = document.createElement('img'); \
         img.setAttribute('loading', 'bogus'); \
         img.loading;");
    assert_eq!(out, "eager");
}

#[test]
fn image_decoding_missing_returns_auto() {
    let out = run("var img = document.createElement('img'); \
         img.decoding;");
    assert_eq!(out, "auto");
}

#[test]
fn image_cross_origin_invalid_returns_anonymous() {
    let out = run("var img = document.createElement('img'); \
         img.setAttribute('crossorigin', 'bogus'); \
         img.crossOrigin;");
    assert_eq!(out, "anonymous");
}

#[test]
fn image_fetch_priority_canonicalises() {
    let out = run("var img = document.createElement('img'); \
         img.setAttribute('fetchpriority', 'HIGH'); \
         img.fetchpriority;");
    assert_eq!(out, "high");
}

// =====================================================================
// `<a>.text` / `<script>.text` accessors (IMP-2)
// =====================================================================

#[test]
fn anchor_text_getter_returns_text_content() {
    let out = run("var a = document.createElement('a'); \
         a.appendChild(document.createTextNode('hello')); \
         a.text;");
    assert_eq!(out, "hello");
}

#[test]
fn anchor_text_setter_replaces_children() {
    let out = run("var a = document.createElement('a'); \
         a.appendChild(document.createTextNode('old')); \
         a.text = 'new'; \
         a.text;");
    assert_eq!(out, "new");
}

#[test]
fn script_text_round_trip() {
    let out = run("var s = document.createElement('script'); \
         s.text = 'console.log(1)'; \
         s.text;");
    assert_eq!(out, "console.log(1)");
}

// =====================================================================
// `<img>.width` / `<img>.height` numeric reflect (IMP-5)
// =====================================================================

#[test]
fn image_width_parses_simple_integer() {
    let out = run("var img = document.createElement('img'); \
         img.setAttribute('width', '100'); \
         String(img.width);");
    assert_eq!(out, "100");
}

#[test]
fn image_width_parses_with_leading_whitespace() {
    let out = run("var img = document.createElement('img'); \
         img.setAttribute('width', '  100  '); \
         String(img.width);");
    assert_eq!(out, "100");
}

#[test]
fn image_width_garbage_returns_zero() {
    let out = run("var img = document.createElement('img'); \
         img.setAttribute('width', 'garbage'); \
         String(img.width);");
    assert_eq!(out, "0");
}

#[test]
fn image_width_setter_writes_integer_string() {
    let out = run("var img = document.createElement('img'); \
         img.width = 200; \
         img.getAttribute('width');");
    assert_eq!(out, "200");
}

#[test]
fn image_natural_width_returns_zero_stub() {
    // Defer slot `#11-tags-T2a-img-natural-size` — paint pipeline
    // not yet wired, returns 0.
    let out = run("var img = document.createElement('img'); \
         String(img.naturalWidth) + '|' + String(img.complete);");
    assert_eq!(out, "0|true");
}

#[test]
fn image_decode_returns_resolved_promise() {
    // Defer slot `#11-tags-T2a-img-decode` — Promise.resolve(undefined) stub.
    let out = run("var img = document.createElement('img'); \
         var p = img.decode(); \
         (p instanceof Promise) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// `<link>.sheet === null` (defer scope lock, MIN-2)
// =====================================================================

#[test]
fn link_sheet_returns_null() {
    // Paired with deferred slot `#11-tags-T2a-link-stylesheet`.
    // Stays null until that slot lands.
    let out = run("var l = document.createElement('link'); \
         l.setAttribute('rel', 'stylesheet'); \
         l.setAttribute('href', 'foo.css'); \
         (l.sheet === null) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// `<link>.sizes` — DOMTokenList + PutForwards=value (IMP-3)
// =====================================================================

#[test]
fn link_sizes_is_token_list() {
    let out = run("var l = document.createElement('link'); \
         l.setAttribute('sizes', '16x16 32x32'); \
         String(l.sizes.length) + '|' + l.sizes.contains('16x16');");
    assert_eq!(out, "2|true");
}

#[test]
fn link_sizes_same_object_identity() {
    let out = run("var l = document.createElement('link'); \
         (l.sizes === l.sizes) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn link_sizes_distinct_from_rel_list() {
    let out = run("var l = document.createElement('link'); \
         (l.sizes !== l.relList) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn link_sizes_put_forwards_writes_value() {
    // PutForwards=value: link.sizes = "..." writes the sizes attr.
    let out = run("var l = document.createElement('link'); \
         l.sizes = '64x64'; \
         l.getAttribute('sizes');");
    assert_eq!(out, "64x64");
}

// =====================================================================
// `<iframe>.sandbox` regression (CRIT-3 deferred behavior — string-reflect
// preserved, NOT upgraded to DOMTokenList in this PR)
// =====================================================================

#[test]
fn iframe_sandbox_remains_string_reflect() {
    // Scope lock for `#11-iframe-sandbox-tokenlist` defer slot:
    // iframe.sandbox stays as a plain string reflect.  When the
    // defer slot lands and upgrades to DOMTokenList, this test
    // pin will break and must be updated.
    let out = run("var iframe = document.createElement('iframe'); \
         iframe.setAttribute('sandbox', 'allow-scripts'); \
         var t = typeof iframe.sandbox; \
         t + '|' + iframe.sandbox;");
    assert_eq!(out, "string|allow-scripts");
}

// =====================================================================
// String reflect round-trip — anchor/area/img/script/link
// =====================================================================

#[test]
fn anchor_string_reflect_attrs_round_trip() {
    let out = run("var a = document.createElement('a'); \
         a.target = '_blank'; a.download = 'file.bin'; \
         a.ping = 'https://ping.example/'; a.hreflang = 'en-US'; a.type = 'text/html'; \
         a.target + '|' + a.download + '|' + a.ping + '|' + a.hreflang + '|' + a.type;");
    assert_eq!(out, "_blank|file.bin|https://ping.example/|en-US|text/html");
}

#[test]
fn area_string_reflect_attrs_round_trip() {
    let out = run("var a = document.createElement('area'); \
         a.alt = 'desc'; a.coords = '1,2,3,4'; a.target = '_top'; \
         a.alt + '|' + a.coords + '|' + a.target;");
    assert_eq!(out, "desc|1,2,3,4|_top");
}

#[test]
fn image_string_reflect_attrs_round_trip() {
    let out = run("var img = document.createElement('img'); \
         img.alt = 'pic'; img.srcset = 'a.png 1x'; \
         img.sizes = '100vw'; img.useMap = '#m'; \
         img.alt + '|' + img.srcset + '|' + img.sizes + '|' + img.useMap;");
    assert_eq!(out, "pic|a.png 1x|100vw|#m");
}

#[test]
fn script_bool_reflect_round_trip() {
    let out = run("var s = document.createElement('script'); \
         s.async = true; s.defer = true; \
         var hasAsync = s.hasAttribute('async'); \
         var hasDefer = s.hasAttribute('defer'); \
         s.async = false; \
         var stillAsync = s.hasAttribute('async'); \
         (hasAsync && hasDefer && !stillAsync) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn link_disabled_round_trip() {
    let out = run("var l = document.createElement('link'); \
         l.disabled = true; \
         var on = l.hasAttribute('disabled'); \
         l.disabled = false; \
         var off = l.hasAttribute('disabled'); \
         (on && !off) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn link_string_reflect_attrs_round_trip() {
    let out = run("var l = document.createElement('link'); \
         l.href = 'foo.css'; l.media = 'screen'; l.type = 'text/css'; l.as = 'style'; \
         l.href + '|' + l.media + '|' + l.type + '|' + l.as;");
    assert_eq!(out, "foo.css|screen|text/css|style");
}

#[test]
fn image_is_map_bool_reflect() {
    let out = run("var img = document.createElement('img'); \
         var before = img.isMap; \
         img.isMap = true; \
         var hasAttr = img.hasAttribute('ismap'); \
         var after = img.isMap; \
         (before === false && hasAttr && after === true) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}
