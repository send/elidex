//! D-5 `#11-tags-T2b-passive` — per-element prototype identity +
//! head-bundle accessor + grouping/empty accessor coverage.
//!
//! Coverage matches the D-5 plan memo §C5 surface:
//! - per-element brand-check + prototype identity
//! - HTMLHeading shared across h1-h6 (single prototype)
//! - HTMLQuote shared across blockquote+q
//! - `<title>.text` textContent alias (round-trip + replaces children)
//! - `<base>.href` URL-resolve-fallback-to-raw + plain `<base>.target`
//! - `<meta>` 6 string-reflect (incl. camelCase IDL → hyphen attr)
//! - `<style>.{media,type}` reflect + `<style>.sheet` `[SameObject]`
//! - `<style>.disabled` deferred — accessor not present
//! - `<ol>.reversed` boolean reflect (presence-only)
//! - `<ol>.start` long IDL (default 1, parse fail → default)
//! - `<ol>.type` "limited to only known values" (case-sensitive)
//! - `<li>.value` long IDL (default 0)
//! - `<map>.areas` live HTMLCollection (mutate descendants → length)
//! - `<map>.areas` `[SameObject]` identity
//! - `<data>.value` / `<time>.dateTime` string reflect
//! - foreign-receiver TypeError brand check on accessor-bearing protos
//! - `<body>` brand-only — no `onload`/event-handler IDL attrs
//!   (all 16 deferred to slot `#11-tags-T2b-body-events`)

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
// Per-element prototype identity (24 brand checks via shared parent)
// =====================================================================

#[test]
fn html_brand_distinct_prototype() {
    let out = run("var a = document.createElement('html'); \
         var b = document.createElement('html'); \
         (Object.getPrototypeOf(a) === Object.getPrototypeOf(b)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn head_brand_distinct_from_body() {
    let out = run("var h = document.createElement('head'); \
         var b = document.createElement('body'); \
         (Object.getPrototypeOf(h) !== Object.getPrototypeOf(b)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn body_has_no_onload_event_handler_idl_attr() {
    // T2b ships HTMLBody as brand-only; the 16 event-handler IDL
    // attributes (HTML §4.3.1.1) are deferred to slot
    // `#11-tags-T2b-body-events` paired with D-10 EventHandlerAttribute
    // base machinery.  This test pins the absence so a future
    // accidental install on HTMLElement.prototype gets caught.
    let out = run("var b = document.createElement('body'); \
         var hasOnload = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(b), 'onload') !== undefined; \
         hasOnload ? 'has-onload' : 'no-onload';");
    assert_eq!(out, "no-onload");
}

#[test]
fn heading_h1_h6_share_one_prototype() {
    // h1-h6 dispatch to the single shared HTMLHeadingElement
    // prototype.  Per HTML §4.3.6 the interface IS shared across all
    // six heading levels; only the rendering differs.
    let out = run("var h1 = document.createElement('h1'); \
         var h6 = document.createElement('h6'); \
         (Object.getPrototypeOf(h1) === Object.getPrototypeOf(h6)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn quote_blockquote_q_share_one_prototype() {
    // HTMLQuoteElement is the shared interface for `<blockquote>`
    // and `<q>` per WebIDL — both expose the same `.cite` IDL
    // attribute through one prototype.
    let out = run("var bq = document.createElement('blockquote'); \
         var q = document.createElement('q'); \
         (Object.getPrototypeOf(bq) === Object.getPrototypeOf(q)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn span_div_distinct_per_tag_prototypes() {
    let out = run("var s = document.createElement('span'); \
         var d = document.createElement('div'); \
         (Object.getPrototypeOf(s) !== Object.getPrototypeOf(d)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn br_hr_pre_p_distinct_prototypes() {
    let out = run("var br = document.createElement('br'); \
         var hr = document.createElement('hr'); \
         var pre = document.createElement('pre'); \
         var p = document.createElement('p'); \
         var allDistinct = (Object.getPrototypeOf(br) !== Object.getPrototypeOf(hr)) \
            && (Object.getPrototypeOf(hr) !== Object.getPrototypeOf(pre)) \
            && (Object.getPrototypeOf(pre) !== Object.getPrototypeOf(p)); \
         allDistinct ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn p_does_not_match_picture_substring() {
    // Sanity pin: the `tag_matches_ascii_case("p")` arm must not
    // accidentally match `<picture>` via substring.  `eq_ignore_ascii_case`
    // is exact match per host_data.rs:609.
    let out = run("var p = document.createElement('p'); \
         var pic = document.createElement('picture'); \
         (Object.getPrototypeOf(p) !== Object.getPrototypeOf(pic)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// HTMLTitleElement.text — textContent alias
// =====================================================================

#[test]
fn title_text_get_returns_text_content() {
    let out = run("var t = document.createElement('title'); \
         t.textContent = 'My Page'; \
         t.text;");
    assert_eq!(out, "My Page");
}

#[test]
fn title_text_set_replaces_children_with_text_node() {
    let out = run("var t = document.createElement('title'); \
         var span = document.createElement('span'); \
         t.appendChild(span); \
         t.text = 'Replaced'; \
         t.textContent + '|' + t.childNodes.length;");
    assert_eq!(out, "Replaced|1");
}

#[test]
fn title_text_get_on_div_throws() {
    let out = run("var d = document.createElement('div'); \
         var titleProto = Object.getPrototypeOf(document.createElement('title')); \
         var getter = Object.getOwnPropertyDescriptor(titleProto, 'text').get; \
         try { getter.call(d); 'no-throw'; } catch (e) { 'TypeError'; }");
    assert_eq!(out, "TypeError");
}

// =====================================================================
// HTMLBaseElement.href — URL-resolve-fallback-to-raw
// =====================================================================

#[test]
fn base_href_absolute_url_round_trip() {
    let out = run("var b = document.createElement('base'); \
         b.href = 'https://example.com/path'; \
         b.href;");
    assert_eq!(out, "https://example.com/path");
}

#[test]
fn base_href_unparseable_returns_raw_attribute() {
    // about:blank is opaque-origin so relative-href resolution
    // fails; per HTML §4.2.3 step 4 the getter must return the
    // raw `href` content attribute (NOT empty string).  Same
    // semantic as T2a `<a>.href` / `<area>.href`.
    let out = run("var b = document.createElement('base'); \
         b.href = '/relative/path'; \
         b.href;");
    assert_eq!(out, "/relative/path");
}

#[test]
fn base_href_missing_returns_empty() {
    let out = run("var b = document.createElement('base'); \
         (b.href === '') ? 'empty' : 'not-empty:' + b.href;");
    assert_eq!(out, "empty");
}

#[test]
fn base_target_string_reflect() {
    let out = run("var b = document.createElement('base'); \
         b.target = '_blank'; \
         b.target + '|' + b.getAttribute('target');");
    assert_eq!(out, "_blank|_blank");
}

// =====================================================================
// HTMLMetaElement — 6 string reflects
// =====================================================================

#[test]
fn meta_name_round_trip() {
    let out = run("var m = document.createElement('meta'); \
         m.name = 'viewport'; \
         m.name + '|' + m.getAttribute('name');");
    assert_eq!(out, "viewport|viewport");
}

#[test]
fn meta_http_equiv_camelcase_idl_to_hyphen_attr() {
    // HTML §4.2.5: the IDL attribute is `httpEquiv` but the content
    // attribute is `http-equiv` (hyphenated).  The getter/setter pair
    // must route through the hyphenated content-attribute name.
    let out = run("var m = document.createElement('meta'); \
         m.httpEquiv = 'refresh'; \
         m.httpEquiv + '|' + m.getAttribute('http-equiv');");
    assert_eq!(out, "refresh|refresh");
}

#[test]
fn meta_content_round_trip() {
    let out = run("var m = document.createElement('meta'); \
         m.content = 'width=device-width'; \
         m.content;");
    assert_eq!(out, "width=device-width");
}

#[test]
fn meta_charset_round_trip() {
    let out = run("var m = document.createElement('meta'); \
         m.charset = 'utf-8'; \
         m.charset;");
    assert_eq!(out, "utf-8");
}

#[test]
fn meta_media_round_trip() {
    let out = run("var m = document.createElement('meta'); \
         m.media = 'print'; \
         m.media;");
    assert_eq!(out, "print");
}

#[test]
fn meta_scheme_round_trip_deprecated_but_reflected() {
    let out = run("var m = document.createElement('meta'); \
         m.scheme = 'iso-3166'; \
         m.scheme;");
    assert_eq!(out, "iso-3166");
}

// =====================================================================
// HTMLStyleElement — media / type / sheet (disabled deferred)
// =====================================================================

#[test]
fn style_media_round_trip() {
    let out = run("var s = document.createElement('style'); \
         s.media = 'screen'; \
         s.media;");
    assert_eq!(out, "screen");
}

#[test]
fn style_type_round_trip() {
    let out = run("var s = document.createElement('style'); \
         s.type = 'text/css'; \
         s.type;");
    assert_eq!(out, "text/css");
}

#[test]
fn style_sheet_same_object_identity() {
    // CSSOM §6.2 `[SameObject]` for HTMLStyleElement.sheet.
    // Detached `<style>` is sufficient: the wrapper cache is keyed
    // by the `<style>` Entity, not by document attachment state.
    let out = run("var s = document.createElement('style'); \
         (s.sheet === s.sheet) ? 'same' : 'different';");
    assert_eq!(out, "same");
}

#[test]
fn style_sheet_accessor_only_on_style_prototype() {
    // T2b moved `sheet` from the shared HTMLElement.prototype (PR-B
    // convenience location) to HTMLStyleElement.prototype.  Confirm
    // it's an own descriptor on HTMLStyleElement.prototype.
    let out = run("var s = document.createElement('style'); \
         var styleProto = Object.getPrototypeOf(s); \
         var hasSheet = Object.getOwnPropertyDescriptor(styleProto, 'sheet') !== undefined; \
         hasSheet ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn style_disabled_accessor_not_installed_pending_defer() {
    // Slot `#11-stylesheet-disabled` defers cross-crate cascade
    // integration.  HTMLStyle.disabled is folded into that slot —
    // exposing a no-op stub here would silently mislead callers.
    let out = run("var s = document.createElement('style'); \
         var styleProto = Object.getPrototypeOf(s); \
         var hasDisabled = Object.getOwnPropertyDescriptor(styleProto, 'disabled') !== undefined; \
         hasDisabled ? 'has-disabled' : 'no-disabled';");
    assert_eq!(out, "no-disabled");
}

// =====================================================================
// HTMLQuoteElement.cite — shared blockquote+q
// =====================================================================

#[test]
fn blockquote_cite_round_trip() {
    let out = run("var bq = document.createElement('blockquote'); \
         bq.cite = 'https://example.com/source'; \
         bq.cite;");
    assert_eq!(out, "https://example.com/source");
}

#[test]
fn q_cite_round_trip_via_shared_prototype() {
    let out = run("var q = document.createElement('q'); \
         q.cite = 'https://example.com/q'; \
         q.cite + '|' + q.getAttribute('cite');");
    assert_eq!(out, "https://example.com/q|https://example.com/q");
}

#[test]
fn quote_cite_getter_on_div_throws() {
    let out = run("var d = document.createElement('div'); \
         var quoteProto = Object.getPrototypeOf(document.createElement('blockquote')); \
         var getter = Object.getOwnPropertyDescriptor(quoteProto, 'cite').get; \
         try { getter.call(d); 'no-throw'; } catch (e) { 'TypeError'; }");
    assert_eq!(out, "TypeError");
}

// =====================================================================
// HTMLOListElement — reversed / start / type
// =====================================================================

#[test]
fn ol_reversed_default_false() {
    let out = run("var ol = document.createElement('ol'); \
         (ol.reversed === false) ? 'false' : 'truthy:' + ol.reversed;");
    assert_eq!(out, "false");
}

#[test]
fn ol_reversed_round_trip() {
    let out = run("var ol = document.createElement('ol'); \
         ol.reversed = true; \
         var afterTrue = ol.reversed; \
         var attrPresent = ol.hasAttribute('reversed'); \
         ol.reversed = false; \
         var afterFalse = ol.reversed; \
         var attrAbsent = !ol.hasAttribute('reversed'); \
         (afterTrue === true && attrPresent && afterFalse === false && attrAbsent) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn ol_start_default_one() {
    // HTML §4.4.5: missing default = 1.
    let out = run("var ol = document.createElement('ol'); \
         String(ol.start);");
    assert_eq!(out, "1");
}

#[test]
fn ol_start_parses_attribute() {
    let out = run("var ol = document.createElement('ol'); \
         ol.setAttribute('start', '5'); \
         String(ol.start);");
    assert_eq!(out, "5");
}

#[test]
fn ol_start_negative_round_trip() {
    let out = run("var ol = document.createElement('ol'); \
         ol.setAttribute('start', '-10'); \
         String(ol.start);");
    assert_eq!(out, "-10");
}

#[test]
fn ol_start_unparseable_returns_default() {
    let out = run("var ol = document.createElement('ol'); \
         ol.setAttribute('start', 'garbage'); \
         String(ol.start);");
    assert_eq!(out, "1");
}

#[test]
fn ol_start_setter_serialises_number() {
    let out = run("var ol = document.createElement('ol'); \
         ol.start = 42; \
         ol.getAttribute('start') + '|' + String(ol.start);");
    assert_eq!(out, "42|42");
}

#[test]
fn ol_start_setter_overflow_saturates() {
    let out = run("var ol = document.createElement('ol'); \
         ol.start = 1e20; \
         String(ol.start);");
    // js_number_to_i32_saturating returns i32::MAX for `>= i32::MAX`.
    assert_eq!(out, "2147483647");
}

#[test]
fn ol_start_setter_routes_object_through_value_of() {
    // `serialise_long_idl_arg` goes through ECMAScript ToNumber per
    // WebIDL §3.10.7, which fires user-defined `valueOf` on objects
    // (and truncates 5.7 → 5 via the saturating cast).  Pre-refactor
    // the bespoke match dispatched objects to `coerce_first_arg_to_string_id`,
    // which routed through `toString` and stored "5.7" as the
    // attribute value — observably wrong for IDL `long`.
    let out = run("var ol = document.createElement('ol'); \
         ol.start = { valueOf: function() { return 5.7; } }; \
         ol.getAttribute('start') + '|' + String(ol.start);");
    assert_eq!(out, "5|5");
}

#[test]
fn ol_type_missing_returns_empty() {
    // "limited to only known values" — missing default = "".
    let out = run("var ol = document.createElement('ol'); \
         (ol.type === '') ? 'empty' : 'not-empty:' + ol.type;");
    assert_eq!(out, "empty");
}

#[test]
fn ol_type_canonical_keywords_all_pass_through() {
    // Per HTML §4.4.5, "1" / "a" / "A" / "i" / "I" are case-sensitive
    // distinct keywords.  All five must round-trip exactly.
    let out = run("var ol = document.createElement('ol'); \
         var results = []; \
         var keywords = ['1', 'a', 'A', 'i', 'I']; \
         for (var i = 0; i < keywords.length; i++) { \
            ol.setAttribute('type', keywords[i]); \
            results.push(ol.type); \
         } \
         results.join('|');");
    assert_eq!(out, "1|a|A|i|I");
}

#[test]
fn ol_type_invalid_returns_empty() {
    let out = run("var ol = document.createElement('ol'); \
         ol.setAttribute('type', 'X'); \
         (ol.type === '') ? 'empty' : 'not-empty:' + ol.type;");
    assert_eq!(out, "empty");
}

#[test]
fn ol_type_case_sensitive_lowercase_uppercase_distinct() {
    // The shared `canonicalize_enumerated_attr` (used by T2a
    // referrerPolicy etc.) is ASCII-case-insensitive — that would
    // collapse "A" → "a".  T2b's `canonicalize_limited_to_known_values`
    // is case-sensitive so the spec'd distinction is preserved.
    let out = run("var ol = document.createElement('ol'); \
         ol.setAttribute('type', 'A'); \
         var upperPath = ol.type; \
         ol.setAttribute('type', 'a'); \
         var lowerPath = ol.type; \
         (upperPath === 'A' && lowerPath === 'a') ? 'ok' : ('upper=' + upperPath + ' lower=' + lowerPath);");
    assert_eq!(out, "ok");
}

// =====================================================================
// HTMLLIElement.value
// =====================================================================

#[test]
fn li_value_default_zero() {
    let out = run("var li = document.createElement('li'); \
         String(li.value);");
    assert_eq!(out, "0");
}

#[test]
fn li_value_round_trip() {
    let out = run("var li = document.createElement('li'); \
         li.value = 7; \
         String(li.value) + '|' + li.getAttribute('value');");
    assert_eq!(out, "7|7");
}

#[test]
fn li_value_negative_round_trip() {
    let out = run("var li = document.createElement('li'); \
         li.setAttribute('value', '-3'); \
         String(li.value);");
    assert_eq!(out, "-3");
}

// =====================================================================
// HTMLMapElement — name / areas live HTMLCollection
// =====================================================================

#[test]
fn map_name_round_trip() {
    let out = run("var m = document.createElement('map'); \
         m.name = 'mainmap'; \
         m.name + '|' + m.getAttribute('name');");
    assert_eq!(out, "mainmap|mainmap");
}

#[test]
fn map_areas_returns_html_collection() {
    let out = run("var m = document.createElement('map'); \
         var area = document.createElement('area'); \
         m.appendChild(area); \
         String(m.areas.length);");
    assert_eq!(out, "1");
}

#[test]
fn map_areas_is_live_after_descendant_mutation() {
    let out = run("var m = document.createElement('map'); \
         var areas = m.areas; \
         var beforeLen = areas.length; \
         m.appendChild(document.createElement('area')); \
         m.appendChild(document.createElement('area')); \
         var afterLen = areas.length; \
         beforeLen + '|' + afterLen;");
    assert_eq!(out, "0|2");
}

#[test]
fn map_areas_walks_nested_descendants() {
    // `<area>` can be nested under wrappers (per HTML §4.8.13 the
    // map's `areas` reads ALL `<area>` descendants).  Confirm the
    // ByTagName filter walks the full subtree.
    let out = run("var m = document.createElement('map'); \
         var div = document.createElement('div'); \
         div.appendChild(document.createElement('area')); \
         div.appendChild(document.createElement('area')); \
         m.appendChild(div); \
         String(m.areas.length);");
    assert_eq!(out, "2");
}

#[test]
fn map_areas_is_same_object_per_map() {
    // `[SameObject]` per HTML §4.8.13 — backed by
    // `map_areas_wrappers` cache.
    let out = run("var m = document.createElement('map'); \
         (m.areas === m.areas) ? 'same' : 'different';");
    assert_eq!(out, "same");
}

#[test]
fn map_areas_distinct_per_map() {
    let out = run("var m1 = document.createElement('map'); \
         var m2 = document.createElement('map'); \
         (m1.areas !== m2.areas) ? 'distinct' : 'same';");
    assert_eq!(out, "distinct");
}

#[test]
fn map_areas_only_picks_up_area_elements() {
    let out = run("var m = document.createElement('map'); \
         m.appendChild(document.createElement('area')); \
         m.appendChild(document.createElement('div')); \
         m.appendChild(document.createElement('area')); \
         String(m.areas.length);");
    assert_eq!(out, "2");
}

// =====================================================================
// HTMLDataElement.value / HTMLTimeElement.dateTime
// =====================================================================

#[test]
fn data_value_round_trip() {
    let out = run("var d = document.createElement('data'); \
         d.value = '42'; \
         d.value + '|' + d.getAttribute('value');");
    assert_eq!(out, "42|42");
}

#[test]
fn time_date_time_round_trip() {
    // Camel-case IDL `dateTime` ↔ all-lowercase content attribute
    // `datetime`.
    let out = run("var t = document.createElement('time'); \
         t.dateTime = '2026-05-10T13:00:00Z'; \
         t.dateTime + '|' + t.getAttribute('datetime');");
    assert_eq!(out, "2026-05-10T13:00:00Z|2026-05-10T13:00:00Z");
}

#[test]
fn time_date_time_lowercase_storage_only() {
    // The IDL spelling `dateTime` is camel-case, but the storage
    // (= content-attribute key) is all lowercase `datetime` per
    // HTML §4.5.14.  `hasAttribute('datetime')` confirms storage;
    // `hasAttribute('dateTime')` is currently case-sensitive in
    // elidex (a separate spec gap tracked outside T2b — WHATWG DOM
    // §4.9.6 specifies ASCII-CI lookup).  This test pins T2b's
    // commitment: the IDL setter writes to lowercase `datetime`,
    // not to a separate camelCase attribute.
    let out = run("var t = document.createElement('time'); \
         t.dateTime = 'now'; \
         (t.hasAttribute('datetime') ? 'has-datetime' : 'no-datetime');");
    assert_eq!(out, "has-datetime");
}

// =====================================================================
// Brand check on accessor-bearing protos (foreign receivers throw)
// =====================================================================

#[test]
fn ol_start_getter_on_div_throws() {
    let out = run("var d = document.createElement('div'); \
         var olProto = Object.getPrototypeOf(document.createElement('ol')); \
         var getter = Object.getOwnPropertyDescriptor(olProto, 'start').get; \
         try { getter.call(d); 'no-throw'; } catch (e) { 'TypeError'; }");
    assert_eq!(out, "TypeError");
}

#[test]
fn map_areas_getter_on_div_throws() {
    let out = run("var d = document.createElement('div'); \
         var mapProto = Object.getPrototypeOf(document.createElement('map')); \
         var getter = Object.getOwnPropertyDescriptor(mapProto, 'areas').get; \
         try { getter.call(d); 'no-throw'; } catch (e) { 'TypeError'; }");
    assert_eq!(out, "TypeError");
}

#[test]
fn meta_http_equiv_setter_on_div_throws() {
    let out = run("var d = document.createElement('div'); \
         var metaProto = Object.getPrototypeOf(document.createElement('meta')); \
         var setter = Object.getOwnPropertyDescriptor(metaProto, 'httpEquiv').set; \
         try { setter.call(d, 'refresh'); 'no-throw'; } catch (e) { 'TypeError'; }");
    assert_eq!(out, "TypeError");
}

// =====================================================================
// Brand-only protos chain integrity
// =====================================================================

#[test]
fn brand_only_prototypes_all_chain_to_html_element() {
    // 13 brand-only T2b protos all chain to the same HTMLElement
    // prototype.  Compare via `<div>`'s grandparent (`<div>` itself
    // routes through HTMLDivElement → HTMLElement → ...).
    let out = run("var divHtmlElementProto = Object.getPrototypeOf(Object.getPrototypeOf(document.createElement('div'))); \
         var brandOnlyTags = ['html', 'head', 'body', 'span', 'br', 'hr', 'pre', 'p', 'ul', 'dl', 'menu', 'picture']; \
         var allChain = true; \
         for (var i = 0; i < brandOnlyTags.length; i++) { \
            var el = document.createElement(brandOnlyTags[i]); \
            var parent = Object.getPrototypeOf(Object.getPrototypeOf(el)); \
            if (parent !== divHtmlElementProto) { allChain = false; break; } \
         } \
         allChain ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}
