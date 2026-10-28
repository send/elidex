//! EH-attrs-at-creation slice a: VM cells C8, C17, C18, C19, C19b, C19c,
//! C20 (i), C20 (ii) and plan cell P3 (umbrella §6.1; plan §4.2, §4.5) —
//! Window-delegating pairs stay on HEAD's path (X-W).
//!
//! Window pins assert HEAD's outcome where it is not the spec's; each names
//! what inverts it and is never the correct answer. Every cell gets its own
//! VM, session and DOM (`with_page`); C20 (i) and C20 (ii) are separate
//! tests so (ii) does not inherit (i)'s `window.onhashchange` (plan §0.5).

#![cfg(feature = "engine")]

use super::super::test_helpers::{run, with_page};

// ---- C8 (window pin, X-W): a moved <body> re-raws its WEH attribute onto the Window ----
#[test]
fn c8_moved_body_reraws_window_handler() {
    with_page(|vm| {
        let got = run(
            vm,
            "var b = document.createElement('body'); \
             el.append(b); \
             b.setAttribute('onhashchange', 'window.hit = 1'); \
             window.onhashchange = fn; \
             var p = document.createElement('div'); \
             document.body.append(p); \
             p.append(b); \
             r.push(typeof window.onhashchange === 'function', window.onhashchange !== fn);",
        );
        // HEAD's. Spec: `fn`; slice b (S-5) inverts this pin.
        assert_eq!(
            got, "true,true",
            "C8: the move re-raws X onto the Window (HEAD's)"
        );
    });
}

// ---- C17 (guard / window pin, X-W at T-a3): DOMParser <body onhashchange>, IDL write, append ----
#[test]
fn c17_domparser_body_weh_idl_write_then_append() {
    with_page(|vm| {
        let got = run(
            vm,
            "var dd = new DOMParser().parseFromString('<body onhashchange=\"window.hit = 1\"></body>', 'text/html'); \
             var b = dd.body; \
             r.push(window.onhashchange === null); \
             b.onhashchange = fn; \
             r.push(window.onhashchange === fn); \
             el.append(b); \
             r.push(typeof window.onhashchange === 'function', window.onhashchange !== fn); \
             window.dispatchEvent(new Event('hashchange')); \
             r.push(window.hit === 1, fnRan === 0);",
        );
        // HEAD's at every stage. Spec: null target at the first push, null at the
        // second, nothing runs at the third; reached after T3 and slice b.
        assert_eq!(
            got, "true,true,true,true,true,true",
            "C17: HEAD's outcome at each step"
        );
    });
}

// ---- C18 (guard / window pin, X-W at T-a3): <body> in template contents, then into the document ----
#[test]
fn c18_template_contents_body_weh_then_append() {
    with_page(|vm| {
        let got = run(
            vm,
            "var b = document.createElement('body'); \
             var tpl = document.createElement('template'); \
             tpl.content.append(b); \
             b.setAttribute('onhashchange', 'window.hit = 1'); \
             b.onhashchange = fn; \
             r.push(window.onhashchange === fn); \
             document.body.append(b); \
             r.push(typeof window.onhashchange === 'function', window.onhashchange !== fn); \
             window.dispatchEvent(new Event('hashchange')); \
             r.push(window.hit === 1, fnRan === 0);",
        );
        // HEAD's. Spec: `window.onhashchange === null` (inert node document,
        // HTML §8.1.8.1 *determine the target of an event handler* step 3); reached after T3 and slice b.
        assert_eq!(got, "true,true,true,true,true", "C18: HEAD's outcome");
    });
}

// ---- C19 (guard, X-W at T-a2): a marked <body>'s WEH pair is re-raw'd at insertion ----
#[test]
fn c19_marked_body_weh_pair_reraw_at_insertion() {
    with_page(|vm| {
        let got = run(
            vm,
            "var b = document.createElement('body'); \
             b.setAttribute('onclick', 'globalThis.a = 1'); \
             b.setAttribute('onhashchange', 'window.hit = 1'); \
             dp.body.append(b); \
             b.onhashchange = fn; \
             el.append(b); \
             r.push(typeof window.onhashchange === 'function', window.onhashchange !== fn); \
             window.dispatchEvent(new Event('hashchange')); \
             r.push(window.hit === 1);",
        );
        assert_eq!(got, "true,true,true", "C19: HEAD's and the spec's");
    });
}

/// C19b / C19c shape on `tag`: a marked element with a reflecting-set pair.
fn reflecting_pair_shape(tag: &str) -> String {
    format!(
        "var b = document.createElement('{tag}'); \
         b.setAttribute('onclick', 'globalThis.a = 1'); \
         b.setAttribute('onfocus', 'window.hit = 1'); \
         dp.body.append(b); \
         b.onfocus = fn; \
         el.append(b); \
         r.push(typeof b.onfocus === 'function', b.onfocus !== fn); \
         b.dispatchEvent(new Event('focus')); \
         r.push(window.hit === 1, fnRan === 0);"
    )
}

// ---- C19b (window pin, X-W at T-a2, reflecting set) ----
#[test]
fn c19b_body_reflecting_pair_reraw_at_insertion() {
    with_page(|vm| {
        let got = run(vm, &reflecting_pair_shape("body"));
        // HEAD's. Spec: target is the Window (facet 4); reached after T3 and slice b.
        assert_eq!(got, "true,true,true,true", "C19b: HEAD's outcome");
    });
}

// ---- C19c (window pin, X-W at T-a2, <frameset>) ----
#[test]
fn c19c_frameset_reflecting_pair_reraw_at_insertion() {
    with_page(|vm| {
        let got = run(vm, &reflecting_pair_shape("frameset"));
        // HEAD's. Spec: target is the Window (facets 3, 4); reached after T3 and slice b.
        assert_eq!(got, "true,true,true,true", "C19c: HEAD's outcome");
    });
}

// ---- C20 (i) (guard, X-W at T-a3, mixed element) ----
#[test]
fn c20_i_mixed_body_write_leaves_window_handler() {
    with_page(|vm| {
        let got = run(
            vm,
            "var b = document.createElement('body'); \
             b.setAttribute('onhashchange', 'window.hit = 1'); \
             window.onhashchange = fn; \
             b.setAttribute('onclick', 'globalThis.a = 1'); \
             r.push(window.onhashchange === fn);",
        );
        assert_eq!(got, "true", "C20 (i): window.onhashchange === fn");
    });
}

// ---- C20 (ii) (guard, X-W at T-a3, mixed DOMParser element; fresh VM) ----
#[test]
fn c20_ii_mixed_domparser_body_write_leaves_window_null() {
    with_page(|vm| {
        let got = run(
            vm,
            "var dd = new DOMParser().parseFromString('<body onclick=\"globalThis.a = 1\" onhashchange=\"window.hit = 1\"></body>', 'text/html'); \
             dd.body.setAttribute('onclick', 'globalThis.b = 1'); \
             r.push(window.onhashchange === null);",
        );
        assert_eq!(got, "true", "C20 (ii): window.onhashchange === null");
    });
}

// ---- P3 (red, plan §4.5): a clone-sourced <frameset onclick onhashchange> ----
#[test]
fn p3_frameset_clone_classified_by_tag_and_name() {
    with_page(|vm| {
        let got = run(
            vm,
            "var src = document.createElement('frameset'); \
             src.setAttribute('onclick', 'globalThis.a = 1'); \
             src.setAttribute('onhashchange', 'window.h = 1'); \
             var f = src.cloneNode(); \
             f.setAttribute('onclick', 'globalThis.b = 1'); \
             f.onclick = fn; \
             el.append(f); \
             r.push(f.onclick === fn); \
             f.dispatchEvent(new Event('hashchange')); \
             r.push(window.h === 1);",
        );
        // First: spec (red). Second: HEAD's target, a window pin (T3 facet 3).
        assert_eq!(
            got, "true,true",
            "P3: f.onclick === fn; hashchange at f runs h=1"
        );
    });
}
