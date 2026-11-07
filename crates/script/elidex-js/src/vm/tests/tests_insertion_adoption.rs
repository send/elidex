//! C0b (`docs/plans/2026-11-eh-attrs-c0b-node-document-fact.md` §6.1): DOM
//! *insert* step 7.1 / *replace* step 6 adopt the node into the parent's node
//! document (DOM §4.2.3; DOM §4.5 *adopt*), and every creation site stamps
//! the node document it is created given (DOM §4.4 *node document*), driven
//! through real JS.
//!
//! Red-first (§6.5): the cells name no NEW API, so they ran on the base as
//! written. A red cell asserts the spec outcome (the memo's *C0b = spec*
//! column; red on the base, green at C0b's head); its `_control` sibling reads
//! the moved / created node's `ownerDocument` before the operation and is
//! green on the base. T19 is withdrawn (memo §6.1).
//!
//! Compiled only under `engine`.

#![cfg(feature = "engine")]

use elidex_script_session::SessionCore;

use super::super::test_helpers::{bind_vm, eval_str, with_page};
use super::super::Vm;

/// `dp` / `dp2` (memo §6 header) and the result array `r`.
const PRELUDE: &str = "\
    globalThis.dp = new DOMParser().parseFromString('<div id=d><span id=s></span></div>', 'text/html'); \
    globalThis.dp2 = new DOMParser().parseFromString('<div id=d><span id=s></span></div>', 'text/html'); \
    globalThis.r = [];";

/// Run [`PRELUDE`] then `script` (which pushes into `r`) on a bound page;
/// return `r.join()`.
fn cell(script: &str) -> String {
    let mut out = String::new();
    with_page(|vm| out = run_in(vm, script));
    out
}

fn run_in(vm: &mut Vm, script: &str) -> String {
    vm.eval(PRELUDE).expect("prelude");
    vm.eval(script).expect("cell script");
    eval_str(vm, "r.join()")
}

/// Bind a VM to a page whose initial HTML is parsed by one parser tier
/// (`strict` = Tier-1, else the tolerant html5ever converter), run
/// [`PRELUDE`] + `script`, return `r.join()`.
fn parsed_page_cell(strict: bool, html: &str, script: &str) -> String {
    struct Bound(Vm);
    impl Drop for Bound {
        fn drop(&mut self) {
            self.0.unbind();
        }
    }
    let mut parsed = if strict {
        elidex_html_parser::parse_strict(html).expect("conforming input parses strict")
    } else {
        elidex_html_parser::parse_html(html)
    };
    let mut session = SessionCore::new();
    let mut bound = Bound(Vm::new());
    #[allow(unsafe_code)]
    // SAFETY: `parsed.dom` and `session` are declared before `bound`, so the
    // guard unbinds (on return or unwind) before either is dropped; neither
    // is touched while bound.
    unsafe {
        bind_vm(&mut bound.0, &mut session, &mut parsed.dom, parsed.document);
    }
    run_in(&mut bound.0, script)
}

// ---- T1: a DOMParser node (and its child) appended to the page ----
const T1: &str = "var d = dp.getElementById('d'); var s = d.firstChild; \
     document.body.append(d); \
     r.push(d.ownerDocument === document, s.ownerDocument === document);";

#[test]
fn t1_domparser_subtree_appended_to_page_is_adopted() {
    assert_eq!(cell(T1), "true,true");
}

#[test]
fn t1_control() {
    let got = cell(
        "var d = dp.getElementById('d'); var s = d.firstChild; \
         r.push(d.ownerDocument === dp, s.ownerDocument === dp);",
    );
    assert_eq!(got, "true,true");
}

// ---- T2: a page node appended into a DOMParser document ----
#[test]
fn t2_page_node_appended_into_domparser_doc_is_adopted() {
    let got = cell(
        "var x = document.createElement('div'); dp.body.append(x); \
         r.push(x.ownerDocument === dp);",
    );
    assert_eq!(got, "true");
}

#[test]
fn t2_control() {
    let got = cell("var x = document.createElement('div'); r.push(x.ownerDocument === document);");
    assert_eq!(got, "true");
}

// ---- T3 (×2): a parser-created page node moved into dp, then removed ----
const T3_HTML: &str = "<!DOCTYPE html><html><head></head><body><p id=p></p></body></html>";
const T3: &str = "var p = document.getElementById('p'); dp.body.append(p); \
     var a = p.ownerDocument === dp; p.remove(); \
     r.push(a, p.ownerDocument === dp);";
const T3_CONTROL: &str =
    "var p = document.getElementById('p'); r.push(p.ownerDocument === document);";

#[test]
fn t3_strict_removed_parse_node_keeps_its_adopted_document() {
    assert_eq!(parsed_page_cell(true, T3_HTML, T3), "true,true");
}

#[test]
fn t3_tolerant_removed_parse_node_keeps_its_adopted_document() {
    assert_eq!(parsed_page_cell(false, T3_HTML, T3), "true,true");
}

#[test]
fn t3_control_both_tiers() {
    assert_eq!(parsed_page_cell(true, T3_HTML, T3_CONTROL), "true");
    assert_eq!(parsed_page_cell(false, T3_HTML, T3_CONTROL), "true");
}

// ---- T4: a shadow host moved between DOMParser documents (P-6) ----
#[test]
fn t4_shadow_tree_adopted_with_its_host() {
    let got = cell(
        "var h = dp.createElement('div'); var sr = h.attachShadow({mode: 'open'}); \
         sr.innerHTML = '<b></b>'; dp2.body.append(h); \
         r.push(sr.ownerDocument === dp2, sr.firstChild.ownerDocument === dp2);",
    );
    assert_eq!(got, "true,true");
}

#[test]
fn t4_control() {
    let got = cell("var h = dp.createElement('div'); r.push(h.ownerDocument === dp);");
    assert_eq!(got, "true");
}

// ---- T5 (C2): a DOMParser template moved to the page — its contents adopt
// with it (template adopting steps; C1 later moves them to the inert document) ----
const T5_SETUP: &str =
    "var dp3 = new DOMParser().parseFromString('<template id=t><b></b></template>', 'text/html'); \
     var t = dp3.getElementById('t');";

#[test]
fn t5_template_contents_adopted_with_the_template() {
    let got = cell(&format!(
        "{T5_SETUP} document.body.append(t); \
         r.push(t.content.ownerDocument === document, t.content.firstChild.ownerDocument === document);"
    ));
    assert_eq!(got, "true,true");
}

#[test]
fn t5_control() {
    let got = cell(&format!("{T5_SETUP} r.push(t.ownerDocument === dp3);"));
    assert_eq!(got, "true");
}

// ---- T8–T10: N1's compile gate follows the adopted document ----
const T8_SETUP: &str = "var dp4 = new DOMParser().parseFromString('<div id=q onclick=\"window.hit=1\"></div>', 'text/html'); \
     var d2 = dp4.getElementById('q');";

#[test]
fn t8_moved_under_detached_page_node_compiles() {
    let got = cell(&format!(
        "{T8_SETUP} var x = document.createElement('div'); x.append(d2); r.push(typeof d2.onclick);"
    ));
    assert_eq!(got, "function");
}

#[test]
fn t9_removed_from_page_compiles() {
    let got = cell(&format!(
        "{T8_SETUP} document.body.append(d2); d2.remove(); r.push(typeof d2.onclick);"
    ));
    assert_eq!(got, "function");
}

#[test]
fn t8_t9_control() {
    let got = cell(&format!("{T8_SETUP} r.push(d2.ownerDocument === dp4);"));
    assert_eq!(got, "true");
}

const T10_SETUP: &str =
    "var y = document.createElement('div'); y.setAttribute('onclick', 'window.hit=1');";

#[test]
fn t10_page_node_moved_under_domparser_node_is_gated() {
    let got = cell(&format!(
        "{T10_SETUP} var h = dp.createElement('div'); h.append(y); r.push(typeof y.onclick);"
    ));
    assert_eq!(got, "object");
}

#[test]
fn t10_control() {
    let got = cell(&format!(
        "{T10_SETUP} r.push(y.ownerDocument === document);"
    ));
    assert_eq!(got, "true");
}

// ---- T11: replace-with-fragment adopts the fragment itself (*replace* 6);
// appending a fragment does not ----
#[test]
fn t11_replace_with_fragment_adopts_the_fragment() {
    let got = cell(
        "var f = dp.createDocumentFragment(); f.append(dp.createElement('i')); \
         var c = document.createElement('u'); document.body.append(c); \
         document.body.replaceChild(f, c); r.push(f.ownerDocument === document); \
         var f2 = dp.createDocumentFragment(); f2.append(dp.createElement('i')); \
         document.body.append(f2); r.push(f2.ownerDocument === dp);",
    );
    assert_eq!(got, "true,true");
}

#[test]
fn t11_control() {
    let got = cell("var f = dp.createDocumentFragment(); r.push(f.ownerDocument === dp);");
    assert_eq!(got, "true");
}

// ---- T12: a moved <base> updates its new document's base URL (P-7) ----
const T12_SETUP: &str =
    "var x = document.createElement('base'); x.setAttribute('href', 'https://other.test/');";

#[test]
fn t12_moved_base_sets_new_documents_base_url() {
    let got = cell(&format!(
        "{T12_SETUP} dp.head.append(x); var a = dp.createElement('a'); a.setAttribute('href', 'y'); \
         dp.body.append(a); r.push(a.href);"
    ));
    assert_eq!(got, "https://other.test/y");
}

#[test]
fn t12_control() {
    let got = cell(&format!(
        "{T12_SETUP} r.push(x.ownerDocument === document);"
    ));
    assert_eq!(got, "true");
}

// ---- T13 (R-B, PM 153): a body moved across documents, and one removed ----
const T13_SETUP: &str =
    "var dd = new DOMParser().parseFromString('<body></body>', 'text/html'); var b = dd.body;";

#[test]
fn t13_cross_document_body_moves() {
    let got = cell(&format!(
        "{T13_SETUP} var p = document.createElement('div'); p.append(b); r.push(b.ownerDocument === document); \
         var b2 = document.createElement('body'); dd.documentElement.append(b2); b2.remove(); \
         r.push(b2.ownerDocument === dd);"
    ));
    assert_eq!(got, "true,true");
}

#[test]
fn t13_control() {
    let got = cell(&format!("{T13_SETUP} r.push(b.ownerDocument === dd);"));
    assert_eq!(got, "true");
}

// ---- T20: a shallow clone is created in the operation's document ----
#[test]
fn t20_shallow_clone_keeps_the_source_document() {
    assert_eq!(
        cell("r.push(dp.body.cloneNode(false).ownerDocument === dp);"),
        "true"
    );
}

#[test]
fn t20_control() {
    assert_eq!(cell("r.push(dp.body.ownerDocument === dp);"), "true");
}

// ---- T21: the table helpers create in the table's node document ----
#[test]
fn t21_table_helpers_create_in_the_tables_document() {
    let got = cell(
        "var t = dp.createElement('table'); \
         r.push(...[t.createTHead(), t.insertRow(), t.createCaption(), t.tBodies[0].insertRow(), \
         t.rows[0].insertCell()].map(n => n.ownerDocument === dp));",
    );
    assert_eq!(got, "true,true,true,true,true");
}

#[test]
fn t21_control() {
    assert_eq!(
        cell("var t = dp.createElement('table'); r.push(t.ownerDocument === dp);"),
        "true"
    );
}

// ---- T22: append('s') texts and insertAdjacentText ----
#[test]
fn t22_convert_nodes_and_insert_adjacent_text_documents() {
    let got = cell(
        "var x = dp.createElement('div'); x.append('t', dp.createElement('i')); \
         x.insertAdjacentText('beforeend', 'u'); \
         r.push(x.firstChild.ownerDocument === dp, x.lastChild.ownerDocument === dp);",
    );
    assert_eq!(got, "true,true");
}

#[test]
fn t22_control() {
    assert_eq!(
        cell("var x = dp.createElement('div'); r.push(x.ownerDocument === dp);"),
        "true"
    );
}

// ---- T23 (Range extract; the cloneContents leg is RC-2b's, absent from the
// base: `range/contents.rs` does not exist) ----
const T23_SETUP: &str =
    "var q = new DOMParser().parseFromString('<p id=p>ab<i>c</i></p>', 'text/html'); \
     var p = q.getElementById('p');";

#[test]
fn t23_range_extract_creates_in_the_start_nodes_document() {
    let got = cell(&format!(
        "{T23_SETUP} var rg = document.createRange(); rg.setStart(p.firstChild, 1); rg.setEnd(p, 2); \
         var f = rg.extractContents(); \
         r.push(f.ownerDocument === q, f.firstChild.ownerDocument === q, f.lastChild.ownerDocument === q);"
    ));
    assert_eq!(got, "true,true,true");
}

#[test]
fn t23_control() {
    assert_eq!(
        cell(&format!("{T23_SETUP} r.push(p.ownerDocument === q);")),
        "true"
    );
}
