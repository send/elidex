//! EH-attrs-at-creation T3-a: VM cells T-21, T-A1 to T-A4 (plan
//! `docs/plans/2026-11-eh-attrs-t3a-applicability.md` §6.1) — a name is
//! processed only if it is an event handler content attribute of its element
//! (WHATWG HTML §8.1.8.1 Event handlers, attribute change steps step 1,
//! local-name half; §8.1.8.2; SVG2 §15.8; MathML Core §2.1.4).
//!
//! Every cell gets its own VM, session and DOM. Foreign `body` elements come
//! from the harness (`with_page_and_foreign_bodies`): no script can create one.

#![cfg(feature = "engine")]

use super::super::test_helpers::{run, with_page, with_page_and_foreign_bodies};

// ---- T-21 (red; R-D2, FN-4): a WindowEventHandlers content attribute on a MathML `body` installs nothing ----
#[test]
fn t21_mathml_body_window_handler_attribute_installs_nothing() {
    with_page_and_foreign_bodies(|vm| {
        let got = run(
            vm,
            "mathBody.setAttribute('onhashchange', 'window.hit = 1'); \
             mathBody.dispatchEvent(new Event('hashchange')); \
             r.push(window.hit === undefined, window.onhashchange === null);",
        );
        assert_eq!(
            got, "true,true",
            "T-21: MathML admits GlobalEventHandlers names only; nothing reaches the Window"
        );
    });
}

// ---- T-A1 (red; AP-3, AP-4): W and D names on a `div` install nothing; a G name does ----
#[test]
fn t_a1_html_div_keeps_only_global_names() {
    with_page(|vm| {
        let got = run(
            vm,
            "var d = document.createElement('div'); \
             d.setAttribute('onhashchange', 'window.h = 1'); \
             d.setAttribute('onreadystatechange', 'window.s = 1'); \
             d.setAttribute('onclick', 'window.c = 1'); \
             for (const t of ['hashchange', 'readystatechange', 'click']) d.dispatchEvent(new Event(t)); \
             r.push(window.h === undefined, window.s === undefined, window.c === 1);",
        );
        assert_eq!(got, "true,true,true", "T-A1");
    });
}

// ---- T-A2 (red; AP-6, AP-10): a parser-built MathML element keeps G names only ----
#[test]
fn t_a2_parser_built_mathml_element_keeps_only_global_names() {
    with_page(|vm| {
        let got = run(
            vm,
            "el.innerHTML = '<math><mi onhashchange=\"window.h = 1\" onclick=\"window.c = 1\"></mi></math>'; \
             var mi = el.firstChild.firstChild; \
             mi.dispatchEvent(new Event('hashchange')); \
             mi.dispatchEvent(new Event('click')); \
             r.push(window.h === undefined, window.c === 1);",
        );
        assert_eq!(got, "true,true", "T-A2");
    });
}

// ---- T-A3 (guard; AP-5): a parser-built SVG element keeps every name ----
#[test]
fn t_a3_parser_built_svg_element_keeps_every_name() {
    with_page(|vm| {
        let got = run(
            vm,
            "el.innerHTML = '<svg><rect onhashchange=\"window.h = 1\" onreadystatechange=\"window.s = 1\"></rect></svg>'; \
             var rc = el.firstChild.firstChild; \
             rc.dispatchEvent(new Event('hashchange')); \
             rc.dispatchEvent(new Event('readystatechange')); \
             r.push(window.h === 1, window.s === 1);",
        );
        assert_eq!(
            got, "true,true",
            "T-A3: SVG2 §15.8, every name on every SVG element"
        );
    });
}

// ---- T-A4 (red; AP-8, D-T8 (b)): a non-applicable handler-named attribute does not count toward D-8 ----
#[test]
fn t_a4_non_applicable_name_does_not_defeat_the_write_entry() {
    with_page(|vm| {
        let got = run(
            vm,
            "el.innerHTML = '<div onclick=\"window.c = 1\" onhashchange=\"window.h = 1\"></div>'; \
             var c = el.firstChild.cloneNode(); \
             c.onclick = fn; \
             el.append(c); \
             c.dispatchEvent(new Event('click')); \
             r.push(fnRan === 1, window.c === undefined);",
        );
        assert_eq!(
            got, "true,true",
            "T-A4: one deliverable name, processed at the write, then `fn` replaces it"
        );
    });
}
