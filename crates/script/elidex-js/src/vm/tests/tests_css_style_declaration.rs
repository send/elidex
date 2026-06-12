//! M4-12 #11-style-declaration PR-A — `Element.style` (CSSStyleDeclaration)
//! + `getComputedStyle` + CSS namespace tests.
//!
//! Covers:
//! - `el.style` returns an identity-preserving Inline CSSStyleDeclaration.
//! - `getComputedStyle(el)` returns a fresh wrapper each call (no identity).
//! - `length`, `item(i)`, indexed exotic, named exotic [[Get]] / [[Set]] /
//!   [[Delete]].
//! - `cssText` get/set including all-or-nothing replace on invalid input.
//! - CRIT-1 round-trip: `el.style.color = "red"` reflects in
//!   `el.getAttribute("style")`.
//! - IMP-3: ASCII-lowercase normalisation; `--` custom property case
//!   sensitivity.
//! - CSS.escape / CSS.supports (2-arg known-property + 1-arg returns false).
//! - Brand check: `CSSStyleDeclaration.prototype.setProperty.call({})` throws.

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

// --- identity ----------------------------------------------------

#[test]
fn style_identity_preserved() {
    let out = run("var d = document.createElement('div'); \
         (d.style === d.style) ? 'same' : 'different';");
    assert_eq!(out, "same");
}

#[test]
fn computed_style_fresh_per_call() {
    // CSSOM §7.2: getComputedStyle returns a NEW declaration block per
    // call; identity is NOT preserved (matches WPT).
    let out = run("var d = document.createElement('div'); \
         document.body.appendChild(d); \
         var a = window.getComputedStyle(d); \
         var b = window.getComputedStyle(d); \
         (a === b) ? 'same' : 'different';");
    assert_eq!(out, "different");
}

// --- named-exotic [[Set]] + [[Get]] -------------------------------

#[test]
fn named_set_and_get() {
    let out = run("var d = document.createElement('div'); \
         d.style.color = 'red'; \
         d.style.color;");
    assert_eq!(out, "#ff0000");
}

/// CRIT-1 regression in JS: setting through `el.style.color` must
/// reflect in `el.getAttribute('style')` so the cascade observes it.
#[test]
fn set_property_syncs_to_attrs_style() {
    let out = run("var d = document.createElement('div'); \
         d.style.color = 'red'; \
         d.getAttribute('style');");
    assert_eq!(out, "color: #ff0000");
}

#[test]
fn set_method_and_get_method() {
    let out = run("var d = document.createElement('div'); \
         d.style.setProperty('display', 'block'); \
         d.style.getPropertyValue('display');");
    assert_eq!(out, "block");
}

/// CSSOM §6.6.1 `setProperty` third argument + `getPropertyPriority`:
/// the priority is stored, readable, excluded from `getPropertyValue`,
/// and re-emitted into the style attribute (cascade-visible).
#[test]
fn set_property_important_priority_round_trip() {
    let out = run("var d = document.createElement('div'); \
         d.style.setProperty('color', 'red', 'important'); \
         d.style.getPropertyPriority('color') + '/' + \
         d.style.getPropertyValue('color') + '/' + \
         d.getAttribute('style');");
    assert_eq!(out, "important/#ff0000/color: #ff0000 !important");
}

/// CSSOM §6.6.1 `setProperty` step 4: a priority that is neither empty
/// nor an ASCII-case-insensitive "important" returns without effect.
#[test]
fn set_property_invalid_priority_no_op() {
    let out = run("var d = document.createElement('div'); \
         d.style.setProperty('color', 'red', 'very-important'); \
         d.style.getPropertyValue('color');");
    assert_eq!(out, "");
}

/// Re-setting without a priority clears a prior `!important` flag.
#[test]
fn set_property_clears_priority_on_normal_set() {
    let out = run("var d = document.createElement('div'); \
         d.style.setProperty('color', 'red', 'IMPORTANT'); \
         d.style.setProperty('color', 'blue'); \
         d.style.getPropertyPriority('color');");
    assert_eq!(out, "");
}

/// The named-exotic set path (`style.color = ...`) reaches the same
/// `style.setProperty` handler with an empty priority, so it too must
/// clear a prior `!important` flag (CSSOM §6.6.1).
#[test]
fn named_set_clears_priority() {
    let out = run("var d = document.createElement('div'); \
         d.style.setProperty('color', 'red', 'important'); \
         d.style.color = 'blue'; \
         d.style.getPropertyPriority('color');");
    assert_eq!(out, "");
}

/// §6.6.1 steps 5–6: a value with trailing input — a smuggled
/// `!important` or an injected `; other: decl` — is rejected whole, so
/// the style attribute the cascade re-parses can't grow fabricated
/// declarations or priority out of a value string.
#[test]
fn set_property_rejects_value_injection() {
    let out = run("var d = document.createElement('div'); \
         d.style.setProperty('color', 'red; background: url(//evil)'); \
         d.style.setProperty('width', '10px !important'); \
         '|' + d.style.cssText + '|' + (d.getAttribute('style') || '') + '|';");
    assert_eq!(out, "|||");
}

/// §6.6.1 step 3: the empty string as value removes the declaration.
#[test]
fn set_property_empty_value_removes() {
    let out = run("var d = document.createElement('div'); \
         d.style.setProperty('display', 'block'); \
         d.style.setProperty('display', ''); \
         '|' + d.style.getPropertyValue('display') + '|' + d.style.length;");
    assert_eq!(out, "||0");
}

#[test]
fn delete_via_named_exotic() {
    let out = run("var d = document.createElement('div'); \
         d.style.color = 'red'; \
         d.style.display = 'block'; \
         delete d.style.color; \
         d.style.color + '/' + d.style.display;");
    // CSSOM §6.6.1 named-getter: an absent supported name returns the
    // empty string (NOT undefined), so the deleted `color` resolves to
    // `""` rather than the prototype-chain fall-through that
    // `dataset.try_get` uses.
    assert_eq!(out, "/block");
}

#[test]
fn remove_property_returns_old_value() {
    let out = run("var d = document.createElement('div'); \
         d.style.color = 'blue'; \
         d.style.removeProperty('color');");
    assert_eq!(out, "#0000ff");
}

// --- length / item / indexed exotic --------------------------------

#[test]
fn length_and_indexed_access() {
    let out = run("var d = document.createElement('div'); \
         d.style.color = 'red'; \
         d.style.display = 'block'; \
         d.style.length + ':' + d.style[0] + ':' + d.style.item(1);");
    assert_eq!(out, "2:color:display");
}

#[test]
fn indexed_oob_returns_empty_string() {
    let out = run("var d = document.createElement('div'); \
         d.style.color = 'red'; \
         var v = d.style[5]; \
         '' + v + '/' + d.style.item(5);");
    assert_eq!(out, "/");
}

// --- cssText -------------------------------------------------------

#[test]
fn css_text_set_and_get() {
    let out = run("var d = document.createElement('div'); \
         d.style.cssText = 'color: red; display: block'; \
         (d.style.color !== '') + '/' + d.style.display + '/' + d.style.length;");
    // `color: red` parses through `parse_declaration_block` which
    // converts `red` to `CssValue::Color(...)`; the round-trip
    // serializes via the colour `Display` impl (hex form) — not a
    // verbatim `red` keyword.  Lossless colour-keyword round-trip is
    // paired with the CSSOM serializer work in PR-B.  Pin the
    // observable `non-empty / block / 2` shape here.
    assert_eq!(out, "true/block/2");
}

#[test]
fn css_text_get_serializes() {
    let out = run("var d = document.createElement('div'); \
         d.style.color = 'red'; \
         d.style.cssText.indexOf('color: #ff0000') >= 0 ? 'yes' : 'no';");
    // Canonical §6.6.1 value parse: `red` stores in hex form.
    assert_eq!(out, "yes");
}

/// IMP-8: cssText="garbage" clears the block (all-or-nothing).
#[test]
fn css_text_invalid_clears_block() {
    let out = run("var d = document.createElement('div'); \
         d.style.color = 'red'; \
         d.style.cssText = 'garbage }}}'; \
         '' + d.style.length;");
    assert_eq!(out, "0");
}

// --- IMP-3: case normalisation ------------------------------------

#[test]
fn property_name_lowercased() {
    let out = run("var d = document.createElement('div'); \
         d.style.setProperty('Color', 'red'); \
         d.style.getPropertyValue('color');");
    assert_eq!(out, "#ff0000");
}

#[test]
fn custom_property_case_preserved() {
    let out = run("var d = document.createElement('div'); \
         d.style.setProperty('--MyVar', '42'); \
         d.style.getPropertyValue('--MyVar') + '/' + d.style.getPropertyValue('--myvar');");
    assert_eq!(out, "42/");
}

// --- getComputedStyle ---------------------------------------------

#[test]
fn computed_style_get_property_value_reads_computed() {
    // ComputedStyle is populated by the cascade; without a live cascade
    // the wrapper exists but `getPropertyValue` falls back to the
    // handler error path → undefined-shaped read.  The smoke test here
    // confirms the API is callable end-to-end (no panic, brand-check
    // passes).
    let out = run("var d = document.createElement('div'); \
         document.body.appendChild(d); \
         var cs = window.getComputedStyle(d); \
         (typeof cs.getPropertyValue === 'function') ? 'fn' : 'no';");
    assert_eq!(out, "fn");
}

#[test]
fn computed_style_named_get_falls_through_when_no_computed() {
    // No ComputedStyle component on the bare element → the handler
    // throws NotFoundError, so the named-getter path returns the empty
    // string (caught at the bridge → ECMA "" coerce).  Confirms the
    // prototype chain is intact (`length` accessor returns 0).
    let out = run("var d = document.createElement('div'); \
         document.body.appendChild(d); \
         var cs = window.getComputedStyle(d); \
         '' + cs.length;");
    assert_eq!(out, "0");
}

// --- CSS namespace -------------------------------------------------

#[test]
fn css_escape_basic() {
    let out = run("CSS.escape('foo bar');");
    assert_eq!(out, "foo\\ bar");
}

#[test]
fn css_escape_leading_digit() {
    let out = run("CSS.escape('123');");
    assert_eq!(out, "\\31 23");
}

#[test]
fn css_supports_known_property() {
    let out = run("CSS.supports('color', 'red') ? 'yes' : 'no';");
    assert_eq!(out, "yes");
}

#[test]
fn css_supports_one_arg_returns_false() {
    let out = run("CSS.supports('(display: flex)') ? 'yes' : 'no';");
    assert_eq!(out, "no");
}

// --- brand check ---------------------------------------------------

#[test]
fn cross_receiver_throws_type_error() {
    let out = run("var d = document.createElement('div'); \
         var fn = Object.getOwnPropertyDescriptor( \
             Object.getPrototypeOf(d.style), 'cssText').get; \
         try { fn.call({}); 'no' } catch (e) { 'yes' }");
    assert_eq!(out, "yes");
}

/// Copilot R1 #1: `parentRule` accessor must brand-check the receiver
/// (WebIDL §3.10) — a `.call({})` invocation must throw TypeError, not
/// silently return null.
#[test]
fn parent_rule_brand_check_throws_on_alien_receiver() {
    let out = run("var d = document.createElement('div'); \
         var fn = Object.getOwnPropertyDescriptor( \
             Object.getPrototypeOf(d.style), 'parentRule').get; \
         try { fn.call({}); 'no' } catch (e) { 'yes' }");
    assert_eq!(out, "yes");
}

/// `parentRule` returns `null` for both Inline and Computed source
/// (only stylesheet-rule-owned declarations have a non-null parent).
#[test]
fn parent_rule_returns_null_for_inline_source() {
    let out = run("var d = document.createElement('div'); \
         (d.style.parentRule === null) ? 'null' : 'other';");
    assert_eq!(out, "null");
}

/// Copilot R1 #3: `getComputedStyle` must reject non-Element node
/// arguments (Text / Comment / Document) — the WebIDL signature is
/// `getComputedStyle(Element elt, ...)`.
#[test]
fn get_computed_style_rejects_text_node() {
    let out = run("var t = document.createTextNode('hi'); \
         try { window.getComputedStyle(t); 'no' } catch (e) { 'yes' }");
    assert_eq!(out, "yes");
}

#[test]
fn get_computed_style_rejects_document() {
    let out = run("try { window.getComputedStyle(document); 'no' } \
         catch (e) { 'yes' }");
    assert_eq!(out, "yes");
}

/// Copilot R1 #7: `cssText` setter parses through
/// `parse_declaration_block` which lowercases ident tokens; the parser
/// must preserve case for `--*` custom properties (CSS Variables L1 §2).
#[test]
fn css_text_preserves_custom_property_case() {
    let out = run("var d = document.createElement('div'); \
         d.style.cssText = '--MyVar: 42'; \
         d.style.getPropertyValue('--MyVar') + '/' + \
         d.style.getPropertyValue('--myvar');");
    assert_eq!(out, "42/");
}

/// Copilot R2 #1: `style[0] = "x"` must NOT redirect to
/// `setProperty("0", "x")` (CSSOM §6.6.1 indexed properties are
/// read-only).  Falls through to ordinary [[Set]] which the
/// non-extensible wrapper rejects.
#[test]
fn indexed_set_does_not_create_numeric_property() {
    let out = run("var d = document.createElement('div'); \
         d.style.color = 'red'; \
         try { d.style[0] = 'oops' } catch (e) {} \
         d.style.length + ':' + d.style[0];");
    assert_eq!(out, "1:color");
}

/// Copilot R2 #2: `delete style[0]` must NOT route to
/// `removeProperty("0")` — indexed properties are not deletable.  The
/// declared `color` property at slot 0 stays.
#[test]
fn indexed_delete_does_not_remove_property() {
    let out = run("var d = document.createElement('div'); \
         d.style.color = 'red'; \
         try { delete d.style[0] } catch (e) {} \
         d.style.length + ':' + d.style.color;");
    assert_eq!(out, "1:#ff0000");
}

// --- prototype chain -----------------------------------------------

#[test]
fn style_prototype_chains_to_object_prototype() {
    // CSSStyleDeclaration.prototype chains directly to Object.prototype
    // (CSSOM §6.6 — the interface is not an EventTarget; mirrors the
    // DOMTokenList / DOMStringMap prototype installs).  Verify by
    // checking that an `Object.prototype.toString` call resolves
    // through the chain (would throw / return undefined if the chain
    // were broken).
    let out = run("var d = document.createElement('div'); \
         (typeof d.style.toString === 'function') ? 'yes' : 'no';");
    assert_eq!(out, "yes");
}

// --- liveness / round-trip ----------------------------------------

/// Read-side `attrs("style")` → `InlineStyle` hydration is deferred:
/// `setAttribute("style", ...)` does not auto-populate `InlineStyle`,
/// so a *pure read* like `el.style.getPropertyValue("color")` returns
/// the empty string until the first mutation triggers
/// `ensure_inline_style`'s seed-from-attrs hydration (Copilot R5 IMP-1
/// fix).  Pinning the read-side divergence here keeps the eager-read-
/// hydration follow-up traceable.
#[test]
fn set_attribute_does_not_populate_inline_style_on_read() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('style', 'color: red'); \
         '|' + d.style.getPropertyValue('color') + '|';");
    assert_eq!(out, "||");
}

/// Copilot R5 IMP-1 regression: `setAttribute("style", ...)` followed
/// by `el.style.setProperty("foo", "bar")` must NOT drop the
/// pre-existing inline declarations (the post-mutation
/// `sync_to_attribute` round-trip used to overwrite `attrs("style")`
/// with an empty `InlineStyle`'s `css_text()` because the ECS component
/// was seeded empty).  After the fix, the first mutation hydrates
/// `InlineStyle` from the attribute string.
#[test]
fn set_attribute_then_set_property_preserves_existing_decls() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('style', 'color: red; display: block'); \
         d.style.setProperty('font-weight', 'bold'); \
         var s = d.getAttribute('style'); \
         (s.indexOf('color') >= 0 && s.indexOf('display') >= 0 && \
          s.indexOf('font-weight') >= 0) ? 'all-three' : s;");
    assert_eq!(out, "all-three");
}

/// Copilot R5 IMP-2 regression: `setAttribute("style", ...)` followed
/// by `el.style.removeProperty(...)` must remove the declaration from
/// the cascade-visible attribute (previously a silent no-op because
/// `removeProperty` early-returned on absent `InlineStyle`).
#[test]
fn set_attribute_then_remove_property_takes_effect() {
    let out = run("var d = document.createElement('div'); \
         d.setAttribute('style', 'color: red; display: block'); \
         d.style.removeProperty('color'); \
         var s = d.getAttribute('style'); \
         (s.indexOf('color') < 0 && s.indexOf('display') >= 0) \
             ? 'removed' : s;");
    assert_eq!(out, "removed");
}
