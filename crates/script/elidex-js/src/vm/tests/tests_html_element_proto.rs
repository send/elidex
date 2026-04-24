//! PR5b §C1 — `HTMLElement.prototype` chain + `focus()` / `blur()`
//! + `document.activeElement` / `document.hasFocus()`.
//!
//! Verifies that:
//!
//! 1. HTML-namespace element wrappers chain through
//!    `HTMLElement.prototype` (spliced in between `HTMLIFrameElement`
//!    and `Element.prototype` — confirms the PR5b chain rewrite).
//! 2. `focus()` / `blur()` mutate `HostData::focused_entity` and are
//!    observable via `document.activeElement`.
//! 3. `document.activeElement` falls back to `<body>` when no element
//!    is focused (WHATWG §6.6.3 step 2).
//! 4. `document.hasFocus()` tracks `HostData::focused_entity`.

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

// --- Prototype chain --------------------------------------------

#[test]
fn html_element_proto_chain_includes_html_element() {
    // `div.__proto__` must be `HTMLElement.prototype`, not
    // `Element.prototype`.  `Element.prototype` still sits one step
    // further up.
    let out = run("var div = document.createElement('div'); \
         var p1 = Object.getPrototypeOf(div); \
         var p2 = Object.getPrototypeOf(p1); \
         var divA = document.createElement('div'); \
         var divB = document.createElement('span'); \
         var sameProto = Object.getPrototypeOf(divA) === Object.getPrototypeOf(divB); \
         (p1 !== p2 && sameProto) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn iframe_proto_chain_splices_html_element() {
    // `iframe.__proto__ = HTMLIFrameElement.prototype`; the next
    // step must be `HTMLElement.prototype` (spliced in by PR5b),
    // not `Element.prototype`.  Identity compared via <div>'s own
    // `__proto__` which IS `HTMLElement.prototype`.
    let out = run("var iframe = document.createElement('iframe'); \
         var div = document.createElement('div'); \
         var iframeGrandparent = Object.getPrototypeOf(Object.getPrototypeOf(iframe)); \
         var htmlElementProto = Object.getPrototypeOf(div); \
         (iframeGrandparent === htmlElementProto) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// --- focus() / blur() --------------------------------------------

#[test]
fn focus_marks_element_as_active_element() {
    let out = run("var d = document.createElement('div'); \
         document.body.appendChild(d); \
         d.focus(); \
         (document.activeElement === d) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn blur_clears_focused_only_when_receiver_matches() {
    // `blur()` on a non-focused element is a no-op.
    let out = run("var a = document.createElement('div'); \
         var b = document.createElement('div'); \
         document.body.appendChild(a); \
         document.body.appendChild(b); \
         a.focus(); \
         b.blur(); \
         if (document.activeElement !== a) 'fail-wrong-blur'; \
         else { a.blur(); \
                (document.activeElement === document.body) ? 'ok' : 'fail-no-fallback'; }");
    assert_eq!(out, "ok");
}

// --- document.activeElement fallback -----------------------------

#[test]
fn active_element_falls_back_to_body_when_unfocused() {
    let out = run("var ae = document.activeElement; \
         (ae === document.body) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn active_element_falls_back_to_body_when_focused_entity_detached() {
    // Focus an element, then remove it from the tree.  The
    // `focused_entity` cache still points at the detached entity,
    // but `activeElement` must report `<body>` because the cached
    // entity is no longer connected.
    //
    // `native_document_get_active_element` walks `get_parent` back
    // up to the document; if the chain does not terminate at the
    // bound document (i.e. the entity was detached), the cached
    // focus is ignored and the fallback path kicks in.  No ECS
    // detach hook is required — the getter enforces the invariant
    // on read.
    let out = run("var d = document.createElement('div'); \
         document.body.appendChild(d); \
         d.focus(); \
         d.remove(); \
         var ae = document.activeElement; \
         (ae === document.body) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// --- document.hasFocus() -----------------------------------------

#[test]
fn has_focus_reflects_focused_entity_presence() {
    let out = run("var before = document.hasFocus(); \
         var d = document.createElement('div'); \
         document.body.appendChild(d); \
         d.focus(); \
         var after = d.blur() || document.hasFocus(); \
         d.focus(); \
         var afterFocus = document.hasFocus(); \
         d.blur(); \
         var afterBlur = document.hasFocus(); \
         (before === false && afterFocus === true && afterBlur === false) \
           ? 'ok' : ('fail:' + before + ',' + afterFocus + ',' + afterBlur);");
    assert_eq!(out, "ok");
}

// Regression guard for Copilot R1 #1: `hasFocus()` must treat a
// detached focused entity as "not focused", mirroring
// `activeElement`'s connectedness filter.  Without the filter, the
// stale `HostData::focused_entity` remained `Some(...)` after
// `removeChild` and `hasFocus()` returned `true` while
// `activeElement` correctly fell back to `<body>`.
#[test]
fn has_focus_returns_false_after_focused_element_detached() {
    let out = run("var d = document.createElement('div'); \
         document.body.appendChild(d); \
         d.focus(); \
         var before = document.hasFocus(); \
         document.body.removeChild(d); \
         var after = document.hasFocus(); \
         var activeIsBody = document.activeElement === document.body; \
         (before === true && after === false && activeIsBody) \
           ? 'ok' : ('fail:' + before + ',' + after + ',' + activeIsBody);");
    assert_eq!(out, "ok");
}

// --- Brand checks ------------------------------------------------

#[test]
fn focus_brand_check_rejects_plain_object() {
    let out = run(
        "var proto = Object.getPrototypeOf(document.createElement('div')); \
         var focusFn = proto.focus; \
         try { focusFn.call({}); 'no-throw'; } \
         catch (e) { (e && e.message && e.message.indexOf('Illegal') >= 0) \
                        ? 'ok' : 'bad:' + (e && e.message); }",
    );
    assert_eq!(out, "ok");
}

// =========================================================================
// IDL attrs — PR5b §C2
// =========================================================================

// ---- plain DOMString reflect ----

#[test]
fn access_key_reflects_accesskey_attribute() {
    let out = run("var d = document.createElement('div'); \
         var before = d.accessKey; \
         d.accessKey = 'K'; \
         var raw = d.getAttribute('accesskey'); \
         d.setAttribute('accesskey', 'X'); \
         var idl = d.accessKey; \
         (before === '' && raw === 'K' && idl === 'X') \
           ? 'ok' : 'fail:' + before + '/' + raw + '/' + idl;");
    assert_eq!(out, "ok");
}

#[test]
fn lang_title_nonce_are_plain_reflects() {
    let out = run("var d = document.createElement('div'); \
         d.lang = 'ja'; d.title = 'hi'; d.nonce = 'n1'; \
         (d.getAttribute('lang') === 'ja' && \
          d.getAttribute('title') === 'hi' && \
          d.getAttribute('nonce') === 'n1' && \
          d.lang === 'ja' && d.title === 'hi' && d.nonce === 'n1') \
           ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// ---- enumerated / limited-to-known-values ----

#[test]
fn dir_getter_canonicalises_and_rejects_invalid() {
    let out = run("var d = document.createElement('div'); \
         var empty = d.dir; \
         d.setAttribute('dir', 'LTR'); \
         var ltr = d.dir; \
         d.setAttribute('dir', 'bogus'); \
         var invalid = d.dir; \
         d.dir = 'rtl'; \
         var setter = d.getAttribute('dir'); \
         (empty === '' && ltr === 'ltr' && invalid === '' && setter === 'rtl') \
           ? 'ok' : 'fail:' + empty + '/' + ltr + '/' + invalid + '/' + setter;");
    assert_eq!(out, "ok");
}

#[test]
fn autocapitalize_getter_limited_to_known_values() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('autocapitalize', 'WORDS'); \
         var known = d.autocapitalize; \
         d.setAttribute('autocapitalize', 'bogus'); \
         var invalid = d.autocapitalize; \
         (known === 'words' && invalid === '') ? 'ok' : 'fail:' + known + '/' + invalid;");
    assert_eq!(out, "ok");
}

#[test]
fn input_mode_and_enter_key_hint_are_enumerated() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('inputmode', 'NUMERIC'); \
         d.setAttribute('enterkeyhint', 'GO'); \
         (d.inputMode === 'numeric' && d.enterKeyHint === 'go') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn content_editable_defaults_to_inherit_and_is_enumerated() {
    let out = run("var d = document.createElement('div'); \
         var none = d.contentEditable; \
         d.setAttribute('contenteditable', 'TRUE'); \
         var t = d.contentEditable; \
         d.setAttribute('contenteditable', 'bogus'); \
         var invalid = d.contentEditable; \
         (none === 'inherit' && t === 'true' && invalid === 'inherit') \
           ? 'ok' : 'fail:' + none + '/' + t + '/' + invalid;");
    assert_eq!(out, "ok");
}

#[test]
fn is_content_editable_walks_ancestors() {
    let out = run("var p = document.createElement('div'); \
         var c = document.createElement('span'); \
         p.appendChild(c); \
         document.body.appendChild(p); \
         var before = c.isContentEditable; \
         p.setAttribute('contenteditable', 'true'); \
         var after = c.isContentEditable; \
         c.setAttribute('contenteditable', 'false'); \
         var own = c.isContentEditable; \
         (before === false && after === true && own === false) \
           ? 'ok' : 'fail:' + before + '/' + after + '/' + own;");
    assert_eq!(out, "ok");
}

// ---- hidden tri-state ----

#[test]
fn hidden_absent_returns_false() {
    let out = run("var d = document.createElement('div'); \
         (d.hidden === false) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn hidden_present_returns_true() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('hidden', ''); \
         (d.hidden === true) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn hidden_until_found_surfaces_as_string() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('hidden', 'until-found'); \
         (d.hidden === 'until-found') ? 'ok' : 'fail:' + d.hidden;");
    assert_eq!(out, "ok");
}

#[test]
fn hidden_setter_accepts_true_false_and_until_found_string() {
    let out = run("var d = document.createElement('div'); \
         d.hidden = true; var t = d.getAttribute('hidden'); \
         d.hidden = false; var f = d.getAttribute('hidden'); \
         d.hidden = 'until-found'; var uf = d.getAttribute('hidden'); \
         (t === '' && f === null && uf === 'until-found') \
           ? 'ok' : 'fail:' + t + '/' + f + '/' + uf;");
    assert_eq!(out, "ok");
}

// ---- boolean presence attrs ----

#[test]
fn autofocus_is_boolean_reflect() {
    let out = run("var d = document.createElement('div'); \
         var before = d.autofocus; \
         d.autofocus = true; var after = d.autofocus; \
         var raw = d.getAttribute('autofocus'); \
         d.autofocus = false; var cleared = d.autofocus; \
         (before === false && after === true && raw === '' && cleared === false) \
           ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// ---- draggable with per-element default ----

#[test]
fn draggable_default_for_img_is_true() {
    let out = run("var img = document.createElement('img'); \
         var a = document.createElement('a'); \
         var aWithHref = document.createElement('a'); \
         aWithHref.setAttribute('href', 'https://example.com/'); \
         var div = document.createElement('div'); \
         (img.draggable === true && a.draggable === false && \
          aWithHref.draggable === true && div.draggable === false) \
           ? 'ok' : 'fail:' + img.draggable + '/' + a.draggable + '/' + \
                   aWithHref.draggable + '/' + div.draggable;");
    assert_eq!(out, "ok");
}

#[test]
fn draggable_setter_writes_true_or_false_literal() {
    let out = run("var d = document.createElement('div'); \
         d.draggable = true; var t = d.getAttribute('draggable'); \
         d.draggable = false; var f = d.getAttribute('draggable'); \
         (t === 'true' && f === 'false') ? 'ok' : 'fail:' + t + '/' + f;");
    assert_eq!(out, "ok");
}

// ---- translate / spellcheck ----

#[test]
fn translate_defaults_to_true_and_maps_yes_no() {
    let out = run("var d = document.createElement('div'); \
         var def = d.translate; \
         d.setAttribute('translate', 'no'); var no = d.translate; \
         d.setAttribute('translate', 'yes'); var yes = d.translate; \
         d.translate = false; var setFalse = d.getAttribute('translate'); \
         (def === true && no === false && yes === true && setFalse === 'no') \
           ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn spellcheck_defaults_to_true_and_maps_true_false() {
    let out = run("var d = document.createElement('div'); \
         var def = d.spellcheck; \
         d.setAttribute('spellcheck', 'false'); var off = d.spellcheck; \
         d.setAttribute('spellcheck', 'true'); var on = d.spellcheck; \
         d.spellcheck = false; var setFalse = d.getAttribute('spellcheck'); \
         (def === true && off === false && on === true && setFalse === 'false') \
           ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// ---- tabIndex with per-element default ----

#[test]
fn tab_index_default_minus_one_for_plain_div() {
    let out = run("var d = document.createElement('div'); \
         (d.tabIndex === -1) ? 'ok' : 'fail:' + d.tabIndex;");
    assert_eq!(out, "ok");
}

#[test]
fn tab_index_default_zero_for_link_with_href() {
    let out = run("var a = document.createElement('a'); \
         var before = a.tabIndex; \
         a.setAttribute('href', 'https://example.com/'); \
         var after = a.tabIndex; \
         (before === -1 && after === 0) ? 'ok' : 'fail:' + before + '/' + after;");
    assert_eq!(out, "ok");
}

#[test]
fn tab_index_default_zero_for_form_controls_and_embeds() {
    let out = run("var b = document.createElement('button'); \
         var s = document.createElement('select'); \
         var ta = document.createElement('textarea'); \
         var iframe = document.createElement('iframe'); \
         var input = document.createElement('input'); \
         var hidden = document.createElement('input'); \
         hidden.setAttribute('type', 'hidden'); \
         (b.tabIndex === 0 && s.tabIndex === 0 && ta.tabIndex === 0 && \
          iframe.tabIndex === 0 && input.tabIndex === 0 && hidden.tabIndex === -1) \
           ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn tab_index_parses_attribute_and_writes_truncated_integer() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('tabindex', '5'); \
         var explicit = d.tabIndex; \
         d.setAttribute('tabindex', 'bogus'); \
         var bad = d.tabIndex; \
         d.tabIndex = 7.9; \
         var written = d.getAttribute('tabindex'); \
         (explicit === 5 && bad === -1 && written === '7') \
           ? 'ok' : 'fail:' + explicit + '/' + bad + '/' + written;");
    assert_eq!(out, "ok");
}

// ---- contenteditable default tabIndex ----

#[test]
fn contenteditable_element_gets_tab_index_zero() {
    let out = run("var d = document.createElement('div'); \
         var before = d.tabIndex; \
         d.setAttribute('contenteditable', 'true'); \
         var after = d.tabIndex; \
         (before === -1 && after === 0) ? 'ok' : 'fail:' + before + '/' + after;");
    assert_eq!(out, "ok");
}

// ---- IDL attrs brand-check ----

#[test]
fn idl_attr_getter_brand_check_rejects_plain_object() {
    let out = run(
        "var proto = Object.getPrototypeOf(document.createElement('div')); \
         var getter = Object.getOwnPropertyDescriptor(proto, 'hidden').get; \
         try { getter.call({}); 'no-throw'; } \
         catch (e) { (e && e.message && e.message.indexOf('Illegal') >= 0) \
                        ? 'ok' : 'bad:' + (e && e.message); }",
    );
    assert_eq!(out, "ok");
}
