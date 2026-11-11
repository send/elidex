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

// ---- T5 (C1): a DOMParser template moved to the page — its contents adopt
// with it (template adopting steps) into the page's inert template document,
// the owner document `tpl.content` of a page template has (HTML §4.12.3
// *appropriate template contents owner document*; C1 memo T13's value) ----
const T5_SETUP: &str =
    "var dp3 = new DOMParser().parseFromString('<template id=t><b></b></template>', 'text/html'); \
     var t = dp3.getElementById('t');";

#[test]
fn t5_template_contents_adopted_with_the_template() {
    let got = cell(&format!(
        "{T5_SETUP} document.body.append(t); \
         var tpl = document.createElement('template'); \
         r.push(t.content.ownerDocument === tpl.content.ownerDocument, \
                t.content.firstChild.ownerDocument === tpl.content.ownerDocument);"
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

// ===========================================================================
// C0c (`docs/plans/2026-11-eh-attrs-c0c-adopt-notification-adoptnode.md`
// §6.1): `adoptedCallback` (DOM §4.5 *adopt* step 3.3.3; HTML §4.13.6) and
// `document.adoptNode` (DOM §4.5 `adoptNode` steps 1–4).
//
// Reactions drain at the end of each eval, so a cell reads `log` in an eval
// after the one that enqueues (memo §6.1's `‖`). Red-first (§6.5): T6 / T7
// ran on the base as written (no `'a'` entry); T15 / T16 / T18 are red by
// absence there (`typeof document.adoptNode` → `'undefined'`). T14 is a
// guard and T15r a known-defect cell; neither runs on the base.
// ===========================================================================

/// T6's `x-a` (one `define` per test): `['a', o === document, n === dp]`
/// (adopted), `['c', this.ownerDocument === dp]` (connected), `['d',
/// this.id]` (disconnected; read at the drain, so by id).
const DEFINE_XA: &str = "globalThis.log = []; \
    customElements.define('x-a', class extends HTMLElement { \
      adoptedCallback(o, n) { log.push(['a', o === document, n === dp]); } \
      connectedCallback() { log.push(['c', this.ownerDocument === dp]); } \
      disconnectedCallback() { log.push(['d', this.id]); } \
    });";

/// Run [`PRELUDE`], [`DEFINE_XA`], then each eval of `evals` in turn (one
/// `vm.eval` per memo `‖` segment); return `JSON.stringify` of the last
/// eval's completion value, read in an eval of its own.
fn ce_cell(evals: &[&str]) -> String {
    let mut out = String::new();
    with_page(|vm| {
        vm.eval(PRELUDE).expect("prelude");
        vm.eval(DEFINE_XA).expect("define x-a");
        let (last, init) = evals.split_last().expect("at least one eval");
        for script in init {
            vm.eval(script).expect("cell eval");
        }
        vm.eval(&format!("globalThis.__out = JSON.stringify({last});"))
            .expect("cell read");
        out = eval_str(vm, "__out");
    });
    out
}

const T6: &str = "globalThis.e = document.createElement('x-a'); dp.body.append(e);";

// ---- T6: a custom element appended into another document ----
#[test]
fn t6_insertion_adoption_fires_adopted_before_connected() {
    assert_eq!(ce_cell(&[T6, "log"]), r#"[["a",true,true],["c",true]]"#);
}

#[test]
fn t6_control() {
    assert_eq!(ce_cell(&[T6, "e.ownerDocument === dp"]), "true");
}

// ---- T7: moved back; and `replaceChild` with a cross-document newChild ----
#[test]
fn t7_move_back_fires_disconnected_adopted_connected() {
    let got = ce_cell(&[T6, "log = []; document.body.append(e);", "log"]);
    assert_eq!(got, r#"[["d",""],["a",false,false],["c",false]]"#);
}

const T7_VARIANT_SETUP: &str = "globalThis.old = document.createElement('x-a'); old.id = 'o'; \
    document.body.append(old); globalThis.e2 = document.createElement('x-a'); e2.id = 'n'; \
    dp.body.append(e2);";

/// `Adopted(e2)` (replace step 6) precedes `Disconnected(old)` (step 7).
///
/// C0c column, not the spec's: known deviation. elidex invokes reactions from
/// one flat queue, so they run in enqueue order (e2, e2, old, e2). HTML
/// §4.13.6 *invoke custom element reactions* dequeues an element and drains
/// that element's whole reaction queue, so the spec value is
/// `[["d","n"],["a",false,false],["c",false],["d","o"]]`;
/// `#11-ce-element-queue-grouping` changes this cell to the spec order. The
/// order this cell pins, `Adopted(e2)` before `Disconnected(old)`, holds
/// under both orders.
#[test]
fn t7_variant_replace_child_adopts_before_removing_old_child() {
    let got = ce_cell(&[
        T7_VARIANT_SETUP,
        "log = []; document.body.replaceChild(e2, old);",
        "log",
    ]);
    assert_eq!(
        got,
        r#"[["d","n"],["a",false,false],["d","o"],["c",false]]"#
    );
}

#[test]
fn t7_control() {
    assert_eq!(
        ce_cell(&[T7_VARIANT_SETUP, "e2.ownerDocument === dp"]),
        "true"
    );
}

// ---- T14 (guard, K-0): a same-document move enqueues no `Adopted` ----
#[test]
fn t14_same_document_move_fires_no_adopted() {
    let got = ce_cell(&[
        "globalThis.q = document.createElement('x-a'); document.body.append(q);",
        "log = []; document.body.prepend(q);",
        "log",
    ]);
    assert_eq!(got, r#"[["d",""],["c",false]]"#);
}

// ---- T15: `document.adoptNode` ----
#[test]
fn t15_i_adopt_node_removes_rehomes_and_returns_node() {
    let got = ce_cell(&[
        "globalThis.n = dp.getElementById('d'); globalThis.r1 = document.adoptNode(n);",
        "[r1 === n, n.ownerDocument === document, n.parentNode]",
    ]);
    assert_eq!(got, "[true,true,null]");
}

#[test]
fn t15_ii_adopt_node_rejects_a_document_and_a_shadow_root() {
    let got = ce_cell(&[
        "globalThis.errs = []; \
         try { document.adoptNode(dp); errs.push('none'); } catch (x) { errs.push(x.name); } \
         globalThis.h = document.createElement('div'); h.attachShadow({mode: 'open'}); \
         try { document.adoptNode(h.shadowRoot); errs.push('none'); } catch (x) { errs.push(x.name); }",
        "errs",
    ]);
    assert_eq!(got, r#"["NotSupportedError","HierarchyRequestError"]"#);
}

#[test]
fn t15_iii_adopt_node_fires_disconnected_then_adopted() {
    let got = ce_cell(&[
        "globalThis.c = document.createElement('x-a'); c.id = 'c'; dp.body.append(c);",
        "log = []; document.adoptNode(c);",
        "log",
    ]);
    assert_eq!(got, r#"[["d","c"],["a",false,false]]"#);
}

/// A non-bound Document's own `adoptNode` (installed by `dom_parser.rs`).
#[test]
fn t15_iv_non_bound_document_adopt_node() {
    let got = ce_cell(&[
        "globalThis.x = document.createElement('div'); document.body.append(x); globalThis.r1 = dp.adoptNode(x);",
        "[r1 === x, x.ownerDocument === dp, x.parentNode]",
    ]);
    assert_eq!(got, "[true,true,null]");
}

#[test]
fn t15_t16_control() {
    assert_eq!(ce_cell(&["typeof document.createElement"]), r#""function""#);
}

// ---- T15r (known defect, memo §1 / §8): a custom element wrongly upgraded
// inside the null-registry DOMParser document ----
/// C0c column, not the spec's. elidex upgrades `k` inside `dp` (the
/// `createElement` native upgrades whatever the document), so adopting it
/// fires `adoptedCallback(dp, document)`. Spec value (derived from the spec
/// text): `k` is not custom in `dp`, so `[["c",false]]` after its upgrade at
/// insertion; `#11-document-null-registry-adopt-reset` changes this cell to
/// that.
#[test]
fn t15r_null_registry_element_known_defect_extra_adopted() {
    let got = ce_cell(&[
        "globalThis.k = dp.createElement('x-a'); k.id = 'k'; dp.body.append(k);",
        "log = []; document.body.append(k);",
        "log",
    ]);
    assert_eq!(got, r#"[["d","k"],["a",false,false],["c",false]]"#);
}

// ---- T15 (v) (known defect, memo D-13 / §6.1): `adoptNode(attr)` ----
/// C0c column, not the spec's. DOM `adoptNode` does not exclude an Attr:
/// *adopt* step 2 removes nothing (an Attr has no parent) and step 3 re-homes
/// only the Attr, so the spec returns `a` with `a.ownerDocument === dp` and
/// `a.ownerElement === x`. The VM's Attr is not a Node (an entity-less
/// `ObjectKind::Attr` wrapper whose chain lacks `Node.prototype`), so the
/// native throws `TypeError`. The ECS side already handles an Attr entity:
/// U22 (`u22_adopt_attr_stamps_only_the_attr`) shows `EcsDom::adopt(attr, D)`
/// stamps only the Attr. `#11-dom-api-attr-entity-model-unification` changes
/// this cell to the spec value.
#[test]
fn t15v_adopt_node_attr_known_defect_type_error() {
    let got = ce_cell(&[
        "globalThis.x = document.createElement('div'); x.id = 'x'; document.body.append(x); \
         globalThis.a = x.getAttributeNode('id');",
        "(() => { try { dp.adoptNode(a); return 'no throw'; } catch (e) { return e instanceof TypeError; } })()",
    ]);
    assert_eq!(got, "true");
}

// ---- T16: `adoptNode`'s only record is the removal's (DOM §4.2.3 *remove*
// step 16) ----
// The siblings are read by `nodeName`, standing in for `localName`, which
// returns `null` today (`#11-element-localname-null`); switch to `localName`
// when that slot lands.
#[test]
fn t16_adopt_node_queues_the_removal_record() {
    let got = ce_cell(&[
        "globalThis.d = dp.getElementById('d'); globalThis.p = d.parentNode; \
         p.prepend(dp.createElement('i')); p.append(dp.createElement('b')); \
         globalThis.mo = new MutationObserver(() => {}); mo.observe(p, {childList: true}); \
         document.adoptNode(d); globalThis.recs = mo.takeRecords();",
        "recs.map(x => [x.type, x.target === p, x.removedNodes.length, x.removedNodes[0] === d, \
         x.addedNodes.length, x.previousSibling && x.previousSibling.nodeName, \
         x.nextSibling && x.nextSibling.nodeName])",
    ]);
    assert_eq!(got, r#"[["childList",true,1,true,0,"I","B"]]"#);
}

// ---- T18 (per-node pairs; P-4): contents adopted apart from their template ----
const T18: &str = "globalThis.t = dp.createElement('template'); dp.body.append(t); \
    globalThis.e = document.createElement('x-a'); t.content.append(e); document.adoptNode(t.content);";

/// `e`'s pair is its segment's (`old = document`, the contents' node
/// document; `new = f(dp2)` since C1), not the root's (`dp`, `dp2`).
#[test]
fn t18_contents_element_gets_its_segments_pair() {
    let got = ce_cell(&[T18, "log = []; dp2.body.append(t);", "log"]);
    assert_eq!(got, r#"[["a",true,false]]"#);
}

#[test]
fn t18_control() {
    let got = ce_cell(&[
        "globalThis.t = dp.createElement('template'); dp.body.append(t); \
         globalThis.e = document.createElement('x-a'); t.content.append(e);",
        // appended into the contents, `e` reads the contents' node document: the
        // inert template document of `dp` (C1 INV-C), not `dp`
        "e.ownerDocument === t.content.ownerDocument && e.ownerDocument !== dp",
    ]);
    assert_eq!(got, "true");
}
