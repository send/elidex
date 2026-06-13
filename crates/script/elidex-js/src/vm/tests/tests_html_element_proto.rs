//! PR5b §C1 — `HTMLElement.prototype` chain + `focus()` / `blur()`
//! + `document.activeElement` / `document.hasFocus()`.
//!
//! Verifies that:
//!
//! 1. HTML-namespace element wrappers chain through
//!    `HTMLElement.prototype` (spliced in between `HTMLIFrameElement`
//!    and `Element.prototype` — confirms the PR5b chain rewrite).
//! 2. `focus()` / `blur()` reconcile the canonical `ElementState::FOCUS`
//!    component (only a focusable area is focused) and are observable
//!    via `document.activeElement` (single-focus: focusing a second
//!    element clears the first).
//! 3. `document.activeElement` falls back to `<body>` when no element
//!    is focused (WHATWG §6.6.6).
//! 4. `document.hasFocus()` reads the `FOCUS` bit via the connectedness
//!    filter (a detached focused element counts as not focused).

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

// --- Constructor gate (CallShape::ConstructorOnly) ---------------

/// WebIDL §3.7.1 step 1.2 + HTML `[HTMLConstructor]` — bare
/// `HTMLElement()` (no `new`) throws the canonical TypeError at the
/// dispatch-side `CallShape::ConstructorOnly` gate.  Site #67 in
/// plan-memo §5 (added per `/code-review` F1 2026-05-30 after the
/// 3-pattern audit missed the `let Some(...) = ctx.new_target() else`
/// guard idiom HTMLElement used pre-migration).
#[test]
fn html_element_ctor_requires_new() {
    use super::helpers::assert_ctor_requires_new;
    assert_ctor_requires_new("HTMLElement()", "HTMLElement");
}

// --- Prototype chain --------------------------------------------

#[test]
fn html_element_proto_chain_includes_html_element() {
    // T2b carve-out: `<div>` and `<span>` each have their own
    // per-tag prototype (HTMLDivElement.prototype /
    // HTMLSpanElement.prototype), each chaining to
    // HTMLElement.prototype.  The chain `wrapper → HTMLDivElement →
    // HTMLElement → Element → Node → EventTarget` must climb through
    // the per-tag layer first, then through HTMLElement.prototype.
    // The two per-tag prototypes are distinct (separate identity)
    // but their parent (HTMLElement.prototype) is shared.
    let out = run("var div = document.createElement('div'); \
         var span = document.createElement('span'); \
         var divProto = Object.getPrototypeOf(div); \
         var spanProto = Object.getPrototypeOf(span); \
         var distinctPerTag = divProto !== spanProto; \
         var sharedHtmlElement = Object.getPrototypeOf(divProto) === Object.getPrototypeOf(spanProto); \
         var htmlElementNotElement = Object.getPrototypeOf(divProto) !== Object.getPrototypeOf(Object.getPrototypeOf(divProto)); \
         (distinctPerTag && sharedHtmlElement && htmlElementNotElement) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn iframe_proto_chain_splices_html_element() {
    // `iframe.__proto__ = HTMLIFrameElement.prototype`; the next
    // step must be `HTMLElement.prototype` (spliced in by PR5b),
    // not `Element.prototype`.  Identity compared via <div>'s
    // grandparent — T2b moved <div> behind a per-tag prototype too,
    // so HTMLElement.prototype is now <div>'s **grand**parent, not
    // its direct parent.
    let out = run("var iframe = document.createElement('iframe'); \
         var div = document.createElement('div'); \
         var iframeGrandparent = Object.getPrototypeOf(Object.getPrototypeOf(iframe)); \
         var htmlElementProto = Object.getPrototypeOf(Object.getPrototypeOf(div)); \
         (iframeGrandparent === htmlElementProto) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// --- focus() / blur() --------------------------------------------

#[test]
fn focus_marks_element_as_active_element() {
    // `tabindex` makes the `<div>` a focusable area (WHATWG §6.6.2);
    // `.focus()` on a non-focusable element is a no-op.
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('tabindex', '0'); \
         document.body.appendChild(d); \
         d.focus(); \
         (document.activeElement === d) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn focus_is_noop_on_non_focusable_element() {
    // A plain `<div>` (no tabindex / contenteditable) is not a
    // focusable area, so `.focus()` does not change `activeElement`
    // (WHATWG §6.6.4 focusing steps gate on §6.6.2 focusable area).
    let out = run("var d = document.createElement('div'); \
         document.body.appendChild(d); \
         d.focus(); \
         (document.activeElement === document.body) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn blur_clears_focused_only_when_receiver_matches() {
    // `blur()` on a non-focused element is a no-op.
    let out = run("var a = document.createElement('div'); \
         var b = document.createElement('div'); \
         a.setAttribute('tabindex', '0'); \
         document.body.appendChild(a); \
         document.body.appendChild(b); \
         a.focus(); \
         b.blur(); \
         if (document.activeElement !== a) 'fail-wrong-blur'; \
         else { a.blur(); \
                (document.activeElement === document.body) ? 'ok' : 'fail-no-fallback'; }");
    assert_eq!(out, "ok");
}

#[test]
fn focus_is_single_focus_clearing_prior_holder() {
    // Single-focus invariant (WHATWG §6.6 — `set_focus_bit`'s
    // clear-all-then-set sweep): focusing `b` must clear `a`'s FOCUS
    // bit, so after `b.blur()` `activeElement` is `<body>` — NOT `a`
    // (which would prove `a` kept a stale bit, i.e. no sweep).
    let out = run("var a = document.createElement('div'); \
         var b = document.createElement('div'); \
         a.setAttribute('tabindex', '0'); \
         b.setAttribute('tabindex', '0'); \
         document.body.appendChild(a); \
         document.body.appendChild(b); \
         a.focus(); \
         b.focus(); \
         var bIsActive = document.activeElement === b; \
         b.blur(); \
         var bodyAfterBlur = document.activeElement === document.body; \
         (bIsActive && bodyAfterBlur) ? 'ok' \
           : ('fail:' + bIsActive + ',' + bodyAfterBlur);");
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
    // Focus an element, then remove it from the tree.  Its
    // `ElementState::FOCUS` bit may persist (detach does not clear
    // it), but `activeElement` must report `<body>` because
    // `current_focus`'s connectedness filter excludes the
    // now-disconnected entity.
    //
    // `current_focus` walks `get_parent` back up to the document; if
    // the chain does not terminate at the bound document (i.e. the
    // entity was detached), the focused entity is ignored and the
    // fallback path kicks in.  No ECS detach hook is required — the
    // read helper enforces the invariant on read.
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('tabindex', '0'); \
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
         d.setAttribute('tabindex', '0'); \
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
// stale `ElementState::FOCUS` bit persists after `removeChild` (detach
// does not clear it) and `hasFocus()` would return `true` while
// `activeElement` correctly fell back to `<body>` — `current_focus`'s
// connectedness walk closes the gap for both.
#[test]
fn has_focus_returns_false_after_focused_element_detached() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('tabindex', '0'); \
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

#[test]
fn removed_focused_element_does_not_resurrect_on_reattach() {
    // focus → remove → blur → reattach must NOT resurrect the element as
    // activeElement. The `FOCUS` bit is cleared at `d.remove()` itself
    // (`EcsDom::fire_after_remove`, WHATWG HTML §2.1.4 removing steps — silent),
    // so `d.blur()` is already a no-op and the reattach finds no stale bit.
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('tabindex', '0'); \
         document.body.appendChild(d); \
         d.focus(); \
         d.remove(); \
         d.blur(); \
         document.body.appendChild(d); \
         (document.activeElement === document.body) ? 'ok' \
           : (document.activeElement === d ? 'fail-resurrected' : 'fail-other');");
    assert_eq!(out, "ok");
}

#[test]
fn focus_on_disconnected_element_is_noop() {
    // §6.6.2: a focusable area must be "being rendered" (⊇ connected), so
    // `createElement().focus()` on an unattached element is a no-op — the
    // `is_focusable` connectedness gate prevents the orphan from ever holding
    // the FOCUS bit (the single read model needs no by-identity fallback).
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('tabindex', '0'); \
         d.focus(); \
         var orphanActive = (document.activeElement === d); \
         document.body.appendChild(d); \
         (orphanActive === false && document.activeElement === document.body) \
           ? 'ok' : ('fail:' + orphanActive);");
    assert_eq!(out, "ok");
}

#[test]
fn hidden_visibility_getters_brand_check_receiver() {
    // Codex R1 F2: the page-visibility getters must not leak the bound tab's
    // state to a non-Document receiver (`get.call({})`).
    let out = run(
        "var hg = Object.getOwnPropertyDescriptor(document, 'hidden').get; \
         var vg = Object.getOwnPropertyDescriptor(document, 'visibilityState').get; \
         (hg.call({}) === false && vg.call({}) === 'visible' \
            && hg.call(document) === false && vg.call(document) === 'visible') \
           ? 'ok' : ('fail:' + hg.call({}) + ',' + vg.call({}));",
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
    // T2b: `<div>`'s direct prototype is now HTMLDivElement.prototype
    // (which has no own `hidden` accessor — `hidden` lives on the
    // shared HTMLElement.prototype one step further up).  Climb one
    // more `getPrototypeOf` to reach the `hidden` accessor.
    let out = run(
        "var proto = Object.getPrototypeOf(Object.getPrototypeOf(document.createElement('div'))); \
         var getter = Object.getOwnPropertyDescriptor(proto, 'hidden').get; \
         try { getter.call({}); 'no-throw'; } \
         catch (e) { (e && e.message && e.message.indexOf('Illegal') >= 0) \
                        ? 'ok' : 'bad:' + (e && e.message); }",
    );
    assert_eq!(out, "ok");
}
