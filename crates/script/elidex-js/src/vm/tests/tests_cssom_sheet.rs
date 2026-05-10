//! M4-12 #11-style-declaration PR-B — CSSStyleSheet / CSSRuleList /
//! CSSStyleRule / CSSRuleStyleDeclaration / StyleSheetList tests.
//!
//! Covers:
//! - `<style>.sheet` returns CSSStyleSheet (`null` for non-style elements,
//!   identity preserved across reads).
//! - `sheet.cssRules` indexed exotic + `length` + `item(i)`.
//! - `sheet.insertRule(text, index)` + stable rule_id across `deleteRule`.
//! - `rule.cssText` / `rule.selectorText` / `rule.parentStyleSheet`.
//! - `rule.style.getPropertyValue(name)` reads declarations.
//! - `rule.style.setProperty` is a silent no-op (Rule source mutation
//!   deferred to slot `#11-css-rule-style-mutation`).
//! - `document.styleSheets` walker enumerates `<style>` descendants.
//! - Brand checks reject foreign receivers.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_doc_with_style(dom: &mut EcsDom, css: &str) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let head = dom.create_element("head", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    let style = dom.create_element("style", Attributes::default());
    let text = dom.create_text(css.to_string());
    assert!(dom.append_child(style, text));
    assert!(dom.append_child(head, style));
    assert!(dom.append_child(html, head));
    assert!(dom.append_child(html, body));
    assert!(dom.append_child(doc, html));
    doc
}

fn run_with_css(css: &str, script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc_with_style(&mut dom, css);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let result = vm.eval(script).unwrap();
    let out = match result {
        JsValue::String(sid) => vm.inner.strings.get_utf8(sid),
        JsValue::Number(n) => n.to_string(),
        JsValue::Boolean(b) => b.to_string(),
        JsValue::Null => "null".to_string(),
        JsValue::Undefined => "undefined".to_string(),
        _ => format!("{result:?}"),
    };
    vm.unbind();
    out
}

// --- <style>.sheet ------------------------------------------------------

#[test]
fn style_element_sheet_is_css_style_sheet() {
    let out = run_with_css(
        "div { color: red; }",
        "var s = document.getElementsByTagName('style')[0]; \
         (s.sheet !== null && typeof s.sheet === 'object') ? 'sheet' : 'no-sheet';",
    );
    assert_eq!(out, "sheet");
}

#[test]
fn non_style_element_sheet_is_null() {
    let out = run_with_css(
        "div {}",
        "var d = document.createElement('div'); (d.sheet === null) ? 'null' : 'not-null';",
    );
    assert_eq!(out, "null");
}

#[test]
fn style_sheet_identity_preserved() {
    let out = run_with_css(
        "div {}",
        "var s = document.getElementsByTagName('style')[0]; \
         (s.sheet === s.sheet) ? 'same' : 'different';",
    );
    assert_eq!(out, "same");
}

// --- cssRules -----------------------------------------------------------

#[test]
fn css_rules_length_matches_input() {
    let out = run_with_css(
        "div { color: red; } p { color: blue; } span { display: none; }",
        "var s = document.getElementsByTagName('style')[0]; \
         String(s.sheet.cssRules.length);",
    );
    assert_eq!(out, "3");
}

#[test]
fn css_rules_indexed_returns_css_style_rule() {
    let out = run_with_css(
        "div { color: red; }",
        "var s = document.getElementsByTagName('style')[0]; \
         var r = s.sheet.cssRules[0]; \
         r.selectorText;",
    );
    assert_eq!(out, "div");
}

#[test]
fn css_rules_indexed_out_of_range_is_null() {
    let out = run_with_css(
        "div {}",
        "var s = document.getElementsByTagName('style')[0]; \
         (s.sheet.cssRules[5] === null) ? 'null' : 'not-null';",
    );
    assert_eq!(out, "null");
}

#[test]
fn css_rules_item_method_matches_indexed() {
    let out = run_with_css(
        "div {}",
        "var s = document.getElementsByTagName('style')[0]; \
         (s.sheet.cssRules.item(0) === s.sheet.cssRules[0]) ? 'same' : 'different';",
    );
    // CSSRuleList indexed access goes through the same alloc-or-cached
    // path as `item`, so the wrappers compare equal.
    assert_eq!(out, "same");
}

// --- rule cssText / selectorText ----------------------------------------

#[test]
fn rule_css_text_returns_source() {
    let out = run_with_css(
        "div { color: red; }",
        "var s = document.getElementsByTagName('style')[0]; \
         s.sheet.cssRules[0].cssText;",
    );
    // Source-text capture preserves the rule's verbatim form.
    assert!(out.contains("div"));
    assert!(out.contains("color"));
    assert!(out.contains("red"));
}

#[test]
fn rule_selector_text_returns_selector_only() {
    let out = run_with_css(
        ".foo > .bar { color: red; }",
        "var s = document.getElementsByTagName('style')[0]; \
         s.sheet.cssRules[0].selectorText;",
    );
    assert!(out.contains(".foo"));
    assert!(out.contains(".bar"));
    assert!(!out.contains("color"));
    assert!(!out.contains('{'));
}

#[test]
fn rule_parent_style_sheet_returns_owner() {
    let out = run_with_css(
        "div {}",
        "var s = document.getElementsByTagName('style')[0]; \
         (s.sheet.cssRules[0].parentStyleSheet === s.sheet) ? 'same' : 'different';",
    );
    assert_eq!(out, "same");
}

// --- rule.style ---------------------------------------------------------

#[test]
fn rule_style_get_property_value() {
    // Returned value passes through `elidex_dom_api::css_value_to_string`
    // which canonicalises colours to hex form (matches the
    // `getComputedStyle` round-trip in PR-A).
    let out = run_with_css(
        "div { color: red; background: blue; }",
        "var s = document.getElementsByTagName('style')[0]; \
         s.sheet.cssRules[0].style.getPropertyValue('color');",
    );
    assert_eq!(out, "#ff0000");
}

#[test]
fn rule_style_named_get() {
    let out = run_with_css(
        "div { color: red; }",
        "var s = document.getElementsByTagName('style')[0]; \
         s.sheet.cssRules[0].style.color;",
    );
    assert_eq!(out, "#ff0000");
}

#[test]
fn rule_style_length() {
    let out = run_with_css(
        "div { color: red; display: block; }",
        "var s = document.getElementsByTagName('style')[0]; \
         String(s.sheet.cssRules[0].style.length);",
    );
    assert_eq!(out, "2");
}

#[test]
fn rule_style_length_dedupes_property_names() {
    // R3 IMP regression: CSSOM §6.6.1 supported-property-name list
    // counts distinct names; `div { color: red; color: blue; }`
    // reports length 1 (matches Chrome).
    let out = run_with_css(
        "div { color: red; color: blue; }",
        "var s = document.getElementsByTagName('style')[0]; \
         String(s.sheet.cssRules[0].style.length);",
    );
    assert_eq!(out, "1");
}

#[test]
fn rule_style_item_dedupes_and_uses_first_occurrence_order() {
    // R3 IMP regression: indexed access enumerates the supported-name
    // list (deduped, first-occurrence order).
    let out = run_with_css(
        "div { color: red; background: blue; color: green; }",
        "var s = document.getElementsByTagName('style')[0]; \
         var st = s.sheet.cssRules[0].style; \
         st[0] + ',' + st[1];",
    );
    // background expands to many longhands; color appears first so it
    // takes index 0 regardless of which background-* longhand lands at
    // index 1.  Test only that `color` is the first slot (deterministic
    // shorthand expansion ordering is deferred to slot
    // `#11-style-shorthand-expand`).
    assert!(out.starts_with("color,"), "actual: {out}");
}

#[test]
fn rule_style_get_property_value_preserves_custom_property_case() {
    // R3 IMP regression: custom properties (`--*`) are case-sensitive
    // per CSS Variables L1 §2.  The stylesheet parser was unconditionally
    // lowercasing, so `getPropertyValue('--MyVar')` against a rule
    // declaring `--MyVar: blue` was missing.
    let out = run_with_css(
        "div { --MyVar: blue; }",
        "var s = document.getElementsByTagName('style')[0]; \
         s.sheet.cssRules[0].style.getPropertyValue('--MyVar');",
    );
    assert_eq!(out, "blue");
}

#[test]
fn rule_style_set_property_is_silent_noop() {
    // Rule-source mutation is deferred to slot `#11-css-rule-style-mutation`.
    // PR-B accepts the call but does not change the underlying rule.
    let out = run_with_css(
        "div { color: red; }",
        "var s = document.getElementsByTagName('style')[0]; \
         var r = s.sheet.cssRules[0]; \
         r.style.color = 'blue'; \
         r.style.getPropertyValue('color');",
    );
    assert_eq!(out, "#ff0000");
}

#[test]
fn rule_style_set_property_strict_mode_silent_not_throw() {
    // Strict-mode regression for IMP-1: the named-property exotic [[Set]]
    // path in `ops_element` must intercept `CSSRuleStyleDeclaration` so the
    // ordinary [[Set]] (which would TypeError on the non-extensible
    // wrapper in strict mode) never runs.  Without the intercept this
    // expression throws `TypeError: Cannot add property color, object is
    // not extensible`.
    let out = run_with_css(
        "div { color: red; }",
        "'use strict'; \
         var s = document.getElementsByTagName('style')[0]; \
         var r = s.sheet.cssRules[0]; \
         try { r.style.color = 'blue'; 'ok'; } catch (e) { 'threw: ' + e.message; }",
    );
    assert_eq!(out, "ok");
}

#[test]
fn rule_style_identity_preserved() {
    let out = run_with_css(
        "div {}",
        "var s = document.getElementsByTagName('style')[0]; \
         var r = s.sheet.cssRules[0]; \
         (r.style === r.style) ? 'same' : 'different';",
    );
    assert_eq!(out, "same");
}

// --- insertRule / deleteRule --------------------------------------------

#[test]
fn insert_rule_extends_rules() {
    let out = run_with_css(
        "div {}",
        "var s = document.getElementsByTagName('style')[0]; \
         s.sheet.insertRule('p { color: green; }', 1); \
         String(s.sheet.cssRules.length) + ',' + \
         s.sheet.cssRules[1].selectorText;",
    );
    assert_eq!(out, "2,p");
}

#[test]
fn insert_rule_round_trips_through_text_content() {
    // After insertRule, the cascade re-reads <style>.textContent on the
    // next walk; the new rule must appear in the serialised text.
    let out = run_with_css(
        "div {}",
        "var s = document.getElementsByTagName('style')[0]; \
         s.sheet.insertRule('p { color: green; }', 1); \
         (s.textContent.indexOf('p') !== -1) ? 'present' : 'missing';",
    );
    assert_eq!(out, "present");
}

#[test]
fn insert_rule_coerces_string_index() {
    // R1 IMP regression: WebIDL `unsigned long` ToUint32 coercion must
    // run on `index`, so `insertRule(rule, '1')` lands at index 1
    // rather than defaulting to 0 via a non-Number short-circuit.
    let out = run_with_css(
        "div {} span {}",
        "var s = document.getElementsByTagName('style')[0]; \
         s.sheet.insertRule('p { color: green; }', '1'); \
         s.sheet.cssRules[1].selectorText;",
    );
    assert_eq!(out, "p");
}

#[test]
fn delete_rule_missing_arg_is_type_error() {
    // R1 IMP regression: required WebIDL arg → TypeError, not
    // IndexSizeError.
    let out = run_with_css(
        "div {} p {}",
        "var s = document.getElementsByTagName('style')[0]; \
         try { s.sheet.deleteRule(); 'ok'; } catch (e) { e.name; }",
    );
    assert_eq!(out, "TypeError");
}

#[test]
fn delete_rule_coerces_string_index() {
    // R1 IMP regression: ToUint32 coercion lets `deleteRule('1')`
    // succeed (delete the rule at index 1) instead of throwing on the
    // raw non-Number JsValue.
    let out = run_with_css(
        "div {} p {} span {}",
        "var s = document.getElementsByTagName('style')[0]; \
         s.sheet.deleteRule('1'); \
         String(s.sheet.cssRules.length) + ',' + s.sheet.cssRules[1].selectorText;",
    );
    assert_eq!(out, "2,span");
}

#[test]
fn delete_rule_shrinks_rules() {
    let out = run_with_css(
        "div {} p {} span {}",
        "var s = document.getElementsByTagName('style')[0]; \
         s.sheet.deleteRule(1); \
         String(s.sheet.cssRules.length);",
    );
    assert_eq!(out, "2");
}

#[test]
fn rule_id_stable_across_delete() {
    // After deleting rule 0, the wrapper for the originally-second rule
    // (held in JS) still points to that same logical rule via its
    // stable rule_id — selectorText survives unchanged.
    let out = run_with_css(
        "div {} p {} span {}",
        "var s = document.getElementsByTagName('style')[0]; \
         var r = s.sheet.cssRules[1]; \
         s.sheet.deleteRule(0); \
         r.selectorText;",
    );
    assert_eq!(out, "p");
}

// --- document.styleSheets -----------------------------------------------

#[test]
fn document_style_sheets_length() {
    let out = run_with_css("div {}", "String(document.styleSheets.length);");
    assert_eq!(out, "1");
}

#[test]
fn document_style_sheets_indexed_returns_css_style_sheet() {
    let out = run_with_css(
        "div {}",
        "var s = document.getElementsByTagName('style')[0]; \
         (document.styleSheets[0] === s.sheet) ? 'same' : 'different';",
    );
    assert_eq!(out, "same");
}

#[test]
fn style_element_sheet_matches_uppercase_tag_via_ascii_ci() {
    // R5 IMP regression: WHATWG DOM §4.2.6.2 mandates ASCII-CI tag
    // matching for HTML documents.  `<STYLE>` (raw create_element with
    // uppercase) must surface a `CSSStyleSheet` via `el.sheet` just like
    // lowercase `<style>` does.  Mirrors `tests_dom_collection.rs::
    // get_elements_by_tag_name_matches_uppercase_element_via_ascii_ci`.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc_with_style(&mut dom, "div {}");
    // Inject a sibling `<STYLE>` (uppercase) element under <head>.
    let head = dom
        .first_child_with_tag(dom.first_child_with_tag(doc, "html").unwrap(), "head")
        .unwrap();
    let upper_style = dom.create_element("STYLE", Attributes::default());
    let upper_text = dom.create_text("p { color: red; }".to_string());
    assert!(dom.append_child(upper_style, upper_text));
    assert!(dom.append_child(head, upper_style));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    // getElementsByTagName('style') matches both lower + upper via ASCII-CI;
    // the second element is the uppercase one.  Verify its `.sheet` returns
    // a CSSStyleSheet (not null).
    let result = vm
        .eval(
            "var els = document.getElementsByTagName('style'); \
             (els[1].sheet !== null && typeof els[1].sheet === 'object') ? 'ok' : 'null';",
        )
        .unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}")
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    assert_eq!(out, "ok");
}

#[test]
fn document_style_sheets_includes_uppercase_style_via_ascii_ci() {
    // R5 IMP regression: `document.styleSheets` walker must also match
    // `<STYLE>` (mixed-case) per WHATWG DOM §4.2.6.2.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc_with_style(&mut dom, "div {}");
    let head = dom
        .first_child_with_tag(dom.first_child_with_tag(doc, "html").unwrap(), "head")
        .unwrap();
    let upper_style = dom.create_element("STYLE", Attributes::default());
    let upper_text = dom.create_text("p { color: red; }".to_string());
    assert!(dom.append_child(upper_style, upper_text));
    assert!(dom.append_child(head, upper_style));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let result = vm.eval("String(document.styleSheets.length);").unwrap();
    let JsValue::String(sid) = result else {
        panic!("expected string, got {result:?}")
    };
    let out = vm.inner.strings.get_utf8(sid);
    vm.unbind();
    assert_eq!(out, "2");
}

#[test]
fn html_element_sheet_getter_foreign_receiver_throws() {
    // R7 IMP regression: WebIDL brand-check semantics — `<style>.sheet`
    // getter called with a non-HostObject receiver (`{}`) must throw
    // `TypeError`, mirroring PR-A's `HTMLElement.style` accessor.
    // Sibling Document accessors (`document.styleSheets`, `head`,
    // `body`) keep the safe-default-null convention; HTMLElement
    // accessors brand-check.
    let out = run_with_css(
        "div {}",
        "var s = document.getElementsByTagName('style')[0]; \
         var getter = Object.getOwnPropertyDescriptor( \
             Object.getPrototypeOf(s), 'sheet').get; \
         try { getter.call({}); 'no-throw'; } catch (e) { e.name; }",
    );
    assert_eq!(out, "TypeError");
}

#[test]
fn insert_rule_rejects_multi_rule_input() {
    // R8 IMP regression: CSSOM §6.4 specifies that `insertRule(text)`
    // must throw `SyntaxError` for input that contains more than one
    // rule.  The previous `parse_single_rule` used `parse_stylesheet`
    // which silently dropped invalid / at-rule content via CSS error
    // recovery, so `insertRule("@media screen {} div {}")` succeeded
    // when it should reject.  Strict variant rejects any input where
    // the StyleSheetParser yields anything other than exactly one
    // qualified rule.
    let out = run_with_css(
        "div {}",
        "var s = document.getElementsByTagName('style')[0]; \
         try { s.sheet.insertRule('@media screen { p {} } span {}', 0); 'no-throw'; } \
         catch (e) { e.name; }",
    );
    assert_eq!(out, "SyntaxError");
}

#[test]
fn insert_rule_rejects_at_rule_input() {
    // R8 IMP regression: pure at-rule input (e.g. `@media`) should
    // fail because the strict parser treats unrecognised at-rules as
    // skipped content per `parse_stylesheet`'s drop policy.
    let out = run_with_css(
        "div {}",
        "var s = document.getElementsByTagName('style')[0]; \
         try { s.sheet.insertRule('@media screen { p {} }', 0); 'no-throw'; } \
         catch (e) { e.name; }",
    );
    assert_eq!(out, "SyntaxError");
}

#[test]
fn rule_selector_text_handles_brace_in_attribute_value() {
    // R7 MIN regression: `selectorText` previously used `split_once('{')`
    // which mis-sliced selectors containing `{` inside an attribute
    // value.  Parser now captures `selector_text` separately; the
    // attribute-string brace is preserved.
    let out = run_with_css(
        "[data-x=\"{\"] { color: red; }",
        "var s = document.getElementsByTagName('style')[0]; \
         s.sheet.cssRules[0].selectorText;",
    );
    assert!(out.contains("[data-x"), "actual: {out}");
    assert!(
        out.contains("\"{\"") || out.contains("'{'"),
        "actual: {out}"
    );
}

#[test]
fn document_style_sheets_non_host_receiver_returns_null() {
    // R2 IMP regression: when `require_receiver` returns `Ok(None)`
    // (the receiver isn't a HostObject — e.g. a plain `{}` after the
    // accessor is rebound, or any post-unbind retained wrapper), the
    // styleSheets getter must surface a safe-default `null` instead
    // of `TypeError`, mirroring sibling Document accessors
    // (`head` / `body` / etc.) that already follow this convention.
    let out = run_with_css(
        "div {}",
        "var getter = Object.getOwnPropertyDescriptor(document, 'styleSheets').get; \
         try { var r = getter.call({}); (r === null) ? 'null' : 'not-null'; } \
         catch (e) { 'threw: ' + e.name; }",
    );
    assert_eq!(out, "null");
}

#[test]
fn document_style_sheets_out_of_range_is_null() {
    let out = run_with_css(
        "div {}",
        "(document.styleSheets[5] === null) ? 'null' : 'not-null';",
    );
    assert_eq!(out, "null");
}

// --- accessor / IDL ------------------------------------------------------

#[test]
fn sheet_type_is_text_css() {
    let out = run_with_css(
        "div {}",
        "var s = document.getElementsByTagName('style')[0]; s.sheet.type;",
    );
    assert_eq!(out, "text/css");
}

#[test]
fn sheet_href_is_null_for_style_element() {
    let out = run_with_css(
        "div {}",
        "var s = document.getElementsByTagName('style')[0]; \
         (s.sheet.href === null) ? 'null' : 'not-null';",
    );
    assert_eq!(out, "null");
}

#[test]
fn sheet_owner_node_is_style_element() {
    let out = run_with_css(
        "div {}",
        "var s = document.getElementsByTagName('style')[0]; \
         (s.sheet.ownerNode === s) ? 'same' : 'different';",
    );
    assert_eq!(out, "same");
}
