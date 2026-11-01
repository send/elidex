//! EH-attrs-at-creation slice a: VM cells C1–C7, C9, C11–C16 (umbrella
//! §6.1; plan §4.2) — the once rule (K-1), the write-side entry (K-2 / K-3,
//! D-8) and the HEAD-equal windows (W-K5), with `el` connected to the live,
//! bound document (`test_helpers::setup_page`).
//!
//! Every cell gets its own VM, session and DOM (`with_page`). Events are
//! driven by `dispatchEvent(new Event(...))`. A cell's script records each
//! oracle step into an array and returns `r.join()`, so a failure shows
//! every step.

#![cfg(feature = "engine")]

use elidex_script_session::HandlerAttrsProcessed;

use super::super::test_helpers::{entity_of, eval_str, run, with_page, PRELUDE};
use super::click_source;

// ---- C1 (red, K-1): a delivered element moved after an IDL write keeps the IDL value ----
#[test]
fn c1_move_keeps_idl_written_handler() {
    with_page(|vm| {
        let got = run(
            vm,
            "el.innerHTML = '<div id=t onclick=\"globalThis.a = 1\"></div>'; \
             var t = document.getElementById('t'); \
             t.onclick = fn; \
             el.append(t); \
             r.push(t.onclick === fn); \
             t.dispatchEvent(new Event('click')); \
             r.push(fnRan === 1, globalThis.a === undefined);",
        );
        assert_eq!(
            got, "true,true,true",
            "C1: t.onclick === fn; dispatch runs fn"
        );
    });
}

// ---- C2 (red, K-1): a compiled AttributeChange handler survives a move into a DOMParser document ----
#[test]
fn c2_compiled_handler_survives_move_into_domparser_document() {
    with_page(|vm| {
        let got = run(
            vm,
            "globalThis.t = document.createElement('div'); \
             el.append(t); \
             t.setAttribute('onclick', 'globalThis.s = 1'); \
             t.dispatchEvent(new Event('click')); \
             r.push(globalThis.s === 1); \
             globalThis.f = t.onclick; \
             dp.body.appendChild(t);",
        );
        assert_eq!(got, "true", "control: S compiled and ran");
        let t = {
            let v = vm.eval("t").expect("target element `t`");
            entity_of(vm, v)
        };
        assert_eq!(click_source(vm, t), None, "C2: no re-raw at the move");
        assert_eq!(
            eval_str(vm, "String(t.onclick === f)"),
            "true",
            "C2: t.onclick === f"
        );
    });
}

// ---- C3 (red, K-1): a baked handler's callable keeps identity across two live moves ----
#[test]
fn c3_callable_identity_across_live_moves() {
    with_page(|vm| {
        let got = run(
            vm,
            "el.innerHTML = '<div id=t onclick=\"globalThis.a = 1\"></div>'; \
             var t = document.getElementById('t'); \
             var f = t.onclick; \
             r.push(typeof f === 'function'); \
             var p = document.createElement('div'); \
             document.body.append(p); \
             p.append(t); \
             r.push(t.onclick === f); \
             el.append(t); \
             r.push(t.onclick === f);",
        );
        assert_eq!(got, "true,true,true", "C3: t.onclick === f after each move");
    });
}

// ---- C4 (red, K-2): a never-inserted element's compiled handler survives its first insertion ----
#[test]
fn c4_first_insertion_keeps_compiled_handler() {
    with_page(|vm| {
        let got = run(
            vm,
            "globalThis.c = document.createElement('div'); \
             c.setAttribute('onclick', 'globalThis.a = 1'); \
             c.dispatchEvent(new Event('click')); \
             r.push(globalThis.a === 1); \
             globalThis.f = c.onclick; \
             dp.body.append(c);",
        );
        assert_eq!(got, "true", "control: A compiled and ran");
        let c = {
            let v = vm.eval("c").expect("clone `c`");
            entity_of(vm, v)
        };
        assert_eq!(
            click_source(vm, c),
            None,
            "C4: no re-raw at the first insertion"
        );
        assert_eq!(
            eval_str(vm, "String(c.onclick === f)"),
            "true",
            "C4: c.onclick === f"
        );
    });
}

/// C5's DOMParser element: `d` with `onclick=A`, then `d.onclick = fn`.
const C5_SETUP: &str = "\
    var dd = new DOMParser().parseFromString('<div id=d onclick=\"globalThis.a = 1\"></div>', 'text/html'); \
    var d = dd.getElementById('d'); \
    d.onclick = fn;";

// ---- C5 (i) (red, K-3): into another DOMParser document ----
#[test]
fn c5_i_idl_write_survives_insert_into_domparser_document() {
    with_page(|vm| {
        let got = run(
            vm,
            &format!("{C5_SETUP} dp2.body.append(d); r.push(d.onclick === fn);"),
        );
        assert_eq!(got, "true", "C5 (i): d.onclick === fn");
    });
}

// ---- C5 (ii) (red, K-3): into the live document ----
#[test]
fn c5_ii_idl_write_survives_insert_into_live_document() {
    with_page(|vm| {
        let got = run(
            vm,
            &format!(
                "{C5_SETUP} el.append(d); r.push(d.onclick === fn); \
                 d.dispatchEvent(new Event('click')); \
                 r.push(fnRan === 1, globalThis.a === undefined);"
            ),
        );
        assert_eq!(
            got, "true,true,true",
            "C5 (ii): d.onclick === fn; dispatch runs fn"
        );
    });
}

// ---- C5 (iii) (red, K-3): a clone, IDL-written, inserted into the live document ----
#[test]
fn c5_iii_clone_idl_write_survives_insert() {
    with_page(|vm| {
        let got = run(
            vm,
            &format!(
                "{C5_SETUP} var c = d.cloneNode(); c.onclick = fn; el.append(c); \
                 r.push(c.onclick === fn); \
                 c.dispatchEvent(new Event('click')); \
                 r.push(fnRan === 1, globalThis.a === undefined);"
            ),
        );
        assert_eq!(
            got, "true,true,true",
            "C5 (iii): c.onclick === fn; dispatch runs fn"
        );
    });
}

// ---- C6 (guard): a DOMParser element's attribute is delivered at its insertion ----
#[test]
fn c6_domparser_element_delivered_at_insert() {
    with_page(|vm| {
        let got = run(
            vm,
            "var dd = new DOMParser().parseFromString('<div id=d onclick=\"globalThis.a = 1\"></div>', 'text/html'); \
             var d = dd.getElementById('d'); \
             el.append(d); \
             r.push(typeof d.onclick === 'function'); \
             d.dispatchEvent(new Event('click')); \
             r.push(globalThis.a === 1);",
        );
        assert_eq!(got, "true,true", "C6: delivered; dispatch runs A");
    });
}

// ---- C7 (guard): setAttribute on a delivered element replaces the handler ----
#[test]
fn c7_set_attribute_replaces_delivered_handler() {
    with_page(|vm| {
        let got = run(
            vm,
            "el.innerHTML = '<div id=t onclick=\"globalThis.a = 1\"></div>'; \
             var t = document.getElementById('t'); \
             var f = t.onclick; \
             t.setAttribute('onclick', 'globalThis.b = 1'); \
             r.push(t.onclick !== f); \
             t.dispatchEvent(new Event('click')); \
             r.push(globalThis.b === 1, globalThis.a === undefined);",
        );
        assert_eq!(
            got, "true,true,true",
            "C7: t.onclick !== f; dispatch runs B"
        );
    });
}

// ---- C9 (guard, I1×I4): a clone is a new, unmarked origin ----
#[test]
fn c9_clone_is_unprocessed_origin() {
    with_page(|vm| {
        vm.eval(PRELUDE).expect("prelude");
        vm.eval(
            "el.innerHTML = '<div id=t onclick=\"globalThis.a = 1\"></div>'; \
             var t = document.getElementById('t'); \
             t.onclick = fn; \
             globalThis.c = t.cloneNode();",
        )
        .expect("setup");
        let c = {
            let v = vm.eval("c").expect("clone `c`");
            entity_of(vm, v)
        };
        let marked = vm
            .host_data()
            .expect("bound")
            .dom()
            .world()
            .get::<&HandlerAttrsProcessed>(c)
            .is_ok();
        assert!(!marked, "C9: c unmarked before its insertion");
        vm.eval(
            "el.append(c); \
             r.push(typeof c.onclick === 'function', c.onclick !== fn); \
             c.dispatchEvent(new Event('click')); \
             r.push(globalThis.a === 1, fnRan === 0);",
        )
        .expect("insert");
        assert_eq!(
            eval_str(vm, "r.join()"),
            "true,true,true,true",
            "C9: after the insertion c.onclick is compiled A, not fn"
        );
    });
}

// ---- C11 (red, K-2): setAttribute then IDL write on a never-inserted element ----
#[test]
fn c11_idl_write_after_set_attribute_survives_first_insertion() {
    with_page(|vm| {
        let got = run(
            vm,
            "var c = document.createElement('div'); \
             c.setAttribute('onclick', 'globalThis.a = 1'); \
             c.onclick = fn; \
             el.append(c); \
             r.push(c.onclick === fn);",
        );
        assert_eq!(got, "true", "C11: c.onclick === fn");
    });
}

// ---- C12 (guard): a removed attribute on a clone delivers nothing ----
#[test]
fn c12_removed_attribute_on_clone_delivers_nothing() {
    with_page(|vm| {
        let got = run(
            vm,
            "var s = document.createElement('div'); \
             s.setAttribute('onclick', 'globalThis.a = 1'); \
             var c = s.cloneNode(); \
             r.push(c.getAttribute('onclick') !== null); \
             c.removeAttribute('onclick'); \
             el.append(c); \
             r.push(c.onclick === null);",
        );
        assert_eq!(got, "true,true", "C12: c.onclick === null");
    });
}

// ---- C13 (guard): both parser tiers deliver at insertion ----
#[test]
fn c13_both_parser_tiers_deliver() {
    with_page(|vm| {
        let got = run(
            vm,
            "el.innerHTML = '<button id=b1 onclick=\"globalThis.n1 = 1\">x</button>'; \
             var b1 = document.getElementById('b1'); \
             r.push(b1 !== null); \
             b1.dispatchEvent(new Event('click')); \
             r.push(globalThis.n1 === 1); \
             el.innerHTML = '<table><button id=b2 onclick=\"globalThis.n2 = 1\">y</button></table>'; \
             var b2 = document.getElementById('b2'); \
             r.push(b2 !== null); \
             b2.dispatchEvent(new Event('click')); \
             r.push(globalThis.n2 === 1);",
        );
        assert_eq!(
            got, "true,true,true,true",
            "C13: strict and tolerant tiers deliver"
        );
    });
}

// ---- C14 (window pin, W-K5 template): template contents get HEAD's raw value and run ----
// `x` is read as `tpl.content.firstChild`: the VM has no
// `DocumentFragment.prototype.getElementById`.
#[test]
fn c14_template_contents_handler_runs() {
    with_page(|vm| {
        let got = run(
            vm,
            "var tpl = document.createElement('template'); \
             el.append(tpl); \
             tpl.innerHTML = '<b id=x onclick=\"window.hit = 1\"></b>'; \
             var x = tpl.content.firstChild; \
             r.push(x !== null && x.id === 'x'); \
             x.dispatchEvent(new Event('click')); \
             r.push(window.hit === 1);",
        );
        // HEAD's outcome. Spec: does not run; slice b (S-2) inverts this pin.
        assert_eq!(got, "true,true", "C14: window.hit === 1 (HEAD's)");
    });
}

// ---- C15 (spec pin, W-K5 DOMParser): the raw value at the post-build insertion reads null ----
#[test]
fn c15_domparser_element_gets_raw_value_at_insertion() {
    with_page(|vm| {
        let got = run(
            vm,
            "globalThis.a = 0; \
             var dx = new DOMParser().parseFromString('<div id=x onclick=\"globalThis.a = 1\"></div>', 'text/html'); \
             var x = dx.getElementById('x'); \
             r.push(x.onclick === null); \
             dp2.body.append(x); \
             r.push(x.onclick === null); \
             x.dispatchEvent(new Event('click')); \
             r.push(a === 0);",
        );
        // The spec's outcome (and N1's, compile gate step 3.2): the raw value
        // in a DOMParser document reads null and runs nothing.
        assert_eq!(
            got, "true,true,true",
            "C15: null before and after the insertion; dispatch runs nothing"
        );
    });
}

// ---- C16 (guard, T-a3 restriction returns): two deliverable ordinary attributes ----
#[test]
fn c16_restriction_returns_on_two_ordinary_attributes() {
    with_page(|vm| {
        let got = run(
            vm,
            "var dd = new DOMParser().parseFromString('<div id=d onclick=\"globalThis.a = 1\" onmouseover=\"globalThis.c = 1\"></div>', 'text/html'); \
             var d = dd.getElementById('d'); \
             d.setAttribute('onclick', 'globalThis.b = 1'); \
             r.push(d.onmouseover === null);",
        );
        assert_eq!(got, "true", "C16: d.onmouseover === null");
    });
}
