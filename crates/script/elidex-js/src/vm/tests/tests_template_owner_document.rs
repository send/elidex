//! C1 (`docs/plans/2026-10-eh-attrs-c1-template-owner-document.md` §6.1):
//! template contents are owned by the inert template contents owner document
//! (WHATWG HTML §4.12.3 *appropriate template contents owner document*),
//! driven through real JS.
//!
//! Every cell asserts the memo's *C1 = spec* column. Red-first (§6): the
//! cells name no NEW API, so each red cell ran on the base (`8344af2b`) as
//! written; its `_control` sibling reads the legs whose value is the same on
//! the base and the head and is green on both. T17 (umbrella C14 inverted) is
//! `tests_event_handler_attrs_once.rs`'s; T18 (umbrella C18, unchanged) is
//! `tests_event_handler_attrs_window_delegating.rs`'s.
//!
//! Element access (memo §6, rev 3.10): a node in template contents is reached
//! through `content.firstChild` / `.lastChild`, never `getElementById` (the
//! VM's DocumentFragment has none, `#11-nonelementparentnode-on-fragments`).
//!
//! Compiled only under `engine`.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom, Entity};
use elidex_script_session::SessionCore;

use super::super::test_helpers::{bind_vm, eval_str, with_page};
use super::super::Vm;

/// The result array `r` and the reference template `tpl` (memo §6: `tpl` is
/// `document.createElement('template')` unless the cell defines it).
const PRELUDE: &str = "globalThis.r = []; \
    globalThis.tpl = document.createElement('template');";

/// Run [`PRELUDE`] then `script` (which pushes into `r`); return `r.join()`.
fn run_in(vm: &mut Vm, script: &str) -> String {
    vm.eval(PRELUDE).expect("prelude");
    vm.eval(script).expect("cell script");
    eval_str(vm, "r.join()")
}

/// A cell on a bound [`with_page`] page (`<html><body><div id=el>`).
fn cell(script: &str) -> String {
    let mut out = String::new();
    with_page(|vm| out = run_in(vm, script));
    out
}

/// Unbinds on drop (also on unwind), before the DOM and session it points at.
struct Bound(Vm);
impl Drop for Bound {
    fn drop(&mut self) {
        self.0.unbind();
    }
}

/// A cell on a page whose initial HTML is parsed by one parser tier
/// (`strict` = Tier-1, else the tolerant html5ever converter).
fn parsed_page_cell(strict: bool, html: &str, script: &str) -> String {
    let mut parsed = if strict {
        elidex_html_parser::parse_strict(html).expect("conforming input parses strict")
    } else {
        elidex_html_parser::parse_html(html)
    };
    let mut session = SessionCore::new();
    let mut bound = Bound(Vm::new());
    #[allow(unsafe_code)]
    // SAFETY: `parsed.dom` and `session` are declared before `bound`, so the
    // guard unbinds (on return or unwind) before either is dropped.
    unsafe {
        bind_vm(&mut bound.0, &mut session, &mut parsed.dom, parsed.document);
    }
    run_in(&mut bound.0, script)
}

/// `<html><head></head><body></body></html>` built in `doc` (the shape of
/// `tests_base_href.rs`'s `build_fixture`).
fn head_body_fixture(dom: &mut EcsDom) -> Entity {
    let doc = dom.create_document_root();
    let mut mk = |tag: &str| {
        let owner = dom.node_document_for_new_nodes(doc);
        dom.create_element_with_owner(tag, Attributes::default(), owner)
    };
    let html = mk("html");
    let head = mk("head");
    let body = mk("body");
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, head));
    assert!(dom.append_child(html, body));
    doc
}

/// A cell on a [`head_body_fixture`] page; `page_url` sets the VM's
/// navigation URL first (T29's fixture).
fn head_body_cell(page_url: Option<&str>, script: &str) -> String {
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = head_body_fixture(&mut dom);
    let mut bound = Bound(Vm::new());
    if let Some(url) = page_url {
        bound.0.inner.navigation.current_url = url::Url::parse(url).unwrap();
    }
    #[allow(unsafe_code)]
    // SAFETY: `dom` and `session` are declared before `bound`.
    unsafe {
        bind_vm(&mut bound.0, &mut session, &mut dom, doc);
    }
    run_in(&mut bound.0, script)
}

/// T20 / T9's `<base href>` fixture script.
const BASE_HREF: &str = "var b = document.createElement('base'); \
    b.setAttribute('href', 'http://example.com/dir/'); document.head.appendChild(b);";

/// `x-a` (one `define` per page): `adoptedCallback(o, n)` logs the pair.
const DEFINE_XA: &str = "globalThis.log = []; \
    customElements.define('x-a', class extends HTMLElement { \
      adoptedCallback(o, n) { log.push([o, n]); } \
    });";

/// Run [`PRELUDE`], [`DEFINE_XA`], then each eval of `evals` (one `vm.eval`
/// per memo `‖` segment: reactions drain at the end of each eval); the last
/// eval pushes into `r`. Returns `r.join()`.
fn ce_cell(evals: &[&str]) -> String {
    let mut out = String::new();
    with_page(|vm| {
        vm.eval(PRELUDE).expect("prelude");
        vm.eval(DEFINE_XA).expect("define x-a");
        for script in evals {
            vm.eval(script).expect("cell eval");
        }
        out = eval_str(vm, "r.join()");
    });
    out
}

// ---- T1: the inert document is a Document of its own ----
const T1: &str = "var t = document.createElement('template'); var I = t.content.ownerDocument;";

#[test]
fn t1_contents_owner_is_a_separate_document() {
    let got = cell(&format!(
        "{T1} r.push(I !== document, \
         Object.getPrototypeOf(I) === Object.getPrototypeOf(document), I.defaultView === null);"
    ));
    assert_eq!(got, "true,true,true");
}

#[test]
fn t1_control() {
    let got = cell(&format!(
        "{T1} r.push(Object.getPrototypeOf(I) === Object.getPrototypeOf(document));"
    ));
    assert_eq!(got, "true");
}

// ---- T2 (guard): one inert document per document ----
#[test]
fn t2_two_templates_share_one_inert_document() {
    let got = cell(
        "var t1 = document.createElement('template'); var t2 = document.createElement('template'); \
         r.push(t1.content.ownerDocument === t2.content.ownerDocument);",
    );
    assert_eq!(got, "true");
}

// ---- T3 (×2): parsed template contents ----
const T3_HTML: &str = "<!DOCTYPE html><html><head></head><body>\
    <template id=a><b></b></template></body></html>";
const T3: &str = "var a = document.getElementById('a');";

fn t3(strict: bool) -> String {
    parsed_page_cell(
        strict,
        T3_HTML,
        &format!(
            "{T3} r.push(a.content.ownerDocument !== document, \
             a.content.firstChild.ownerDocument === a.content.ownerDocument);"
        ),
    )
}

#[test]
fn t3_parsed_contents_strict() {
    assert_eq!(t3(true), "true,true");
}

#[test]
fn t3_parsed_contents_tolerant() {
    assert_eq!(t3(false), "true,true");
}

#[test]
fn t3_control() {
    for strict in [true, false] {
        let got = parsed_page_cell(
            strict,
            T3_HTML,
            &format!(
                "{T3} r.push(a.content.firstChild.ownerDocument === a.content.ownerDocument);"
            ),
        );
        assert_eq!(got, "true", "strict = {strict}");
    }
}

// ---- T4 (×2): nested templates resolve to the same inert document ----
const T4_HTML: &str = "<!DOCTYPE html><html><head></head><body>\
    <template id=o><template><p></p></template></template></body></html>";
const T4: &str = "var o = document.getElementById('o'); var i = o.content.firstChild;";

fn t4(strict: bool, legs: &str) -> String {
    parsed_page_cell(strict, T4_HTML, &format!("{T4} r.push({legs});"))
}

const T4_LEGS: &str = "i.ownerDocument === o.content.ownerDocument, \
    i.content.ownerDocument === o.content.ownerDocument, o.content.ownerDocument !== document";
const T4_CONTROL_LEGS: &str = "i.ownerDocument === o.content.ownerDocument, \
    i.content.ownerDocument === o.content.ownerDocument";

#[test]
fn t4_nested_contents_strict() {
    assert_eq!(t4(true, T4_LEGS), "true,true,true");
}

#[test]
fn t4_nested_contents_tolerant() {
    assert_eq!(t4(false, T4_LEGS), "true,true,true");
}

#[test]
fn t4_control() {
    assert_eq!(t4(true, T4_CONTROL_LEGS), "true,true");
    assert_eq!(t4(false, T4_CONTROL_LEGS), "true,true");
}

// ---- T5 (guard, with U2): a template created by the inert document ----
#[test]
fn t5_template_created_in_the_inert_document() {
    let got = cell(
        "var I = tpl.content.ownerDocument; var t = I.createElement('template'); \
         r.push(t.content.ownerDocument === I);",
    );
    assert_eq!(got, "true");
}

// ---- T6: a DOMParser document's templates get its own inert document ----
const T6: &str =
    "var dp = new DOMParser().parseFromString('<template id=t><i></i></template>', 'text/html'); \
    var c = dp.getElementById('t').content;";

#[test]
fn t6_domparser_contents_owner() {
    let got = cell(&format!(
        "{T6} r.push(c.ownerDocument !== dp, c.ownerDocument !== document, \
         c.ownerDocument !== tpl.content.ownerDocument, c.firstChild.ownerDocument === c.ownerDocument);"
    ));
    assert_eq!(got, "true,true,true,true");
}

#[test]
fn t6_control() {
    let got = cell(&format!(
        "{T6} r.push(c.ownerDocument !== document, c.ownerDocument !== tpl.content.ownerDocument, \
         c.firstChild.ownerDocument === c.ownerDocument);"
    ));
    assert_eq!(got, "true,true,true");
}

// ---- T7: innerHTML on a template ----
const T7: &str = "tpl.innerHTML = '<b></b>'; var x = tpl.content.firstChild;";

#[test]
fn t7_inner_html_into_contents() {
    let got = cell(&format!(
        "{T7} r.push(x.ownerDocument === tpl.content.ownerDocument, x.ownerDocument !== document);"
    ));
    assert_eq!(got, "true,true");
}

#[test]
fn t7_control() {
    let got = cell(&format!(
        "{T7} r.push(x.ownerDocument === tpl.content.ownerDocument);"
    ));
    assert_eq!(got, "true");
}

// ---- T8: a node appended into contents is adopted into their document ----
const T8: &str = "var d = document.createElement('div'); tpl.content.append(d);";

#[test]
fn t8_append_into_contents() {
    let got = cell(&format!(
        "{T8} r.push(d.ownerDocument === tpl.content.ownerDocument, d.ownerDocument !== document);"
    ));
    assert_eq!(got, "true,true");
}

#[test]
fn t8_control() {
    let got = cell(&format!(
        "{T8} r.push(d.ownerDocument === tpl.content.ownerDocument);"
    ));
    assert_eq!(got, "true");
}

// ---- T9 (guard, K-0; §1 rows 1–2): a contents clone appended to the document ----
const T9: &str = "tpl.innerHTML = '<a href=x.html></a>'; var c = tpl.content.cloneNode(true); \
    var a = c.firstChild; r.push(a.ownerDocument !== document); document.body.append(c);";

#[test]
fn t9_contents_clone_appended_to_document() {
    let got = head_body_cell(
        None,
        &format!(
            "{BASE_HREF} {T9} r.push(a.ownerDocument === document, \
             a.href === 'http://example.com/dir/x.html');"
        ),
    );
    assert_eq!(got, "true,true,true");
}

#[test]
fn t9_control() {
    let got = head_body_cell(
        None,
        &format!(
            "{BASE_HREF} {T9} r.shift(); r.push(a.ownerDocument === document, \
             a.href === 'http://example.com/dir/x.html');"
        ),
    );
    assert_eq!(got, "true,true");
}

// ---- T10: template cloning steps ----
const T10: &str = "tpl.innerHTML = '<template><p></p></template>'; var t2 = tpl.cloneNode(true); \
    var n2 = t2.content.firstChild; var s2 = tpl.cloneNode(false);";

#[test]
fn t10_clone_contents_owner() {
    let got = cell(&format!(
        "{T10} r.push(t2.content !== tpl.content, \
         t2.content.ownerDocument === tpl.content.ownerDocument, \
         n2.content.ownerDocument === tpl.content.ownerDocument, \
         t2.content.ownerDocument !== document, s2.content.firstChild === null, \
         s2.content.ownerDocument !== document);"
    ));
    assert_eq!(got, "true,true,true,true,true,true");
}

#[test]
fn t10_control() {
    let got = cell(&format!(
        "{T10} r.push(t2.content !== tpl.content, \
         t2.content.ownerDocument === tpl.content.ownerDocument, \
         n2.content.ownerDocument === tpl.content.ownerDocument, s2.content.firstChild === null);"
    ));
    assert_eq!(got, "true,true,true,true");
}

// ---- T11: a cloned document gets its own inert document ----
const T11_HTML: &str =
    "<!DOCTYPE html><html><head></head><body><template></template></body></html>";
const T11: &str =
    "var tpl = document.querySelector('template'); var dc = document.cloneNode(true); \
    var x = dc.querySelector('template').content.ownerDocument;";

#[test]
fn t11_document_clone_contents_owner() {
    let got = parsed_page_cell(
        true,
        T11_HTML,
        &format!("{T11} r.push(x !== dc, x !== tpl.content.ownerDocument);"),
    );
    assert_eq!(got, "true,true");
}

#[test]
fn t11_control() {
    let got = parsed_page_cell(
        true,
        T11_HTML,
        &format!("{T11} r.push(x !== tpl.content.ownerDocument);"),
    );
    assert_eq!(got, "true");
}

// ---- T12: shadow roots in contents (DSD legs strict only; variant ×2) ----
const T12_HTML: &str = "<!DOCTYPE html><html><head></head><body><template id=t><div>\
    <template shadowrootmode=open><span></span></template></div></template></body></html>";
const T12: &str = "var t = document.getElementById('t'); var h = t.content.firstChild; \
    var span = h.shadowRoot.firstChild;";

#[test]
fn t12_declarative_shadow_in_contents_strict() {
    let got = parsed_page_cell(
        true,
        T12_HTML,
        &format!(
            "{T12} r.push(h.shadowRoot.ownerDocument === t.content.ownerDocument, \
             span.ownerDocument === t.content.ownerDocument, \
             h.shadowRoot.ownerDocument !== document);"
        ),
    );
    assert_eq!(got, "true,true,true");
}

#[test]
fn t12_control() {
    let got = parsed_page_cell(
        true,
        T12_HTML,
        &format!(
            "{T12} r.push(h.shadowRoot.ownerDocument === t.content.ownerDocument, \
             span.ownerDocument === t.content.ownerDocument);"
        ),
    );
    assert_eq!(got, "true,true");
}

const T12V_HTML: &str =
    "<!DOCTYPE html><html><head></head><body><template id=v><div></div></template></body></html>";
const T12V: &str = "var d = document.getElementById('v').content.firstChild; \
    var sr = d.attachShadow({mode:'open'});";

fn t12_variant(strict: bool) -> String {
    parsed_page_cell(
        strict,
        T12V_HTML,
        &format!("{T12V} r.push(sr.ownerDocument !== document);"),
    )
}

#[test]
fn t12_attach_shadow_in_contents_strict() {
    assert_eq!(t12_variant(true), "true");
}

#[test]
fn t12_attach_shadow_in_contents_tolerant() {
    assert_eq!(t12_variant(false), "true");
}

#[test]
fn t12_variant_control() {
    for strict in [true, false] {
        let got = parsed_page_cell(
            strict,
            T12V_HTML,
            &format!("{T12V} r.push(sr.ownerDocument === d.ownerDocument);"),
        );
        assert_eq!(got, "true", "strict = {strict}");
    }
}

// ---- T13: a DOMParser template adopted into the document ----
const T13: &str = "var dp = new DOMParser().parseFromString(\
    '<template id=t><template><p></p></template><div></div></template>', 'text/html'); \
    var t = dp.getElementById('t'); var h = t.content.lastChild; \
    h.attachShadow({mode:'open'}).innerHTML = '<i></i>'; document.body.append(t);";

#[test]
fn t13_adopted_template_contents_go_to_the_inert_document() {
    let got = cell(&format!(
        "{T13} r.push(t.ownerDocument === document, \
         t.content.ownerDocument === tpl.content.ownerDocument, \
         t.content.firstChild.content.ownerDocument === tpl.content.ownerDocument, \
         h.shadowRoot.firstChild.ownerDocument === tpl.content.ownerDocument, \
         t.content.ownerDocument !== document);"
    ));
    assert_eq!(got, "true,true,true,true,true");
}

#[test]
fn t13_control() {
    let got = cell(&format!(
        "{T13} r.push(t.ownerDocument === document, \
         t.content.firstChild.content.ownerDocument === t.content.ownerDocument, \
         h.shadowRoot.firstChild.ownerDocument === t.content.ownerDocument);"
    ));
    assert_eq!(got, "true,true,true");
}

// ---- T14 (×2): a template parsed by innerHTML into the document ----
// The tolerant leg appends `</br>`, a parse error, so the strict-first
// dispatcher falls back to the tolerant tier (as umbrella C13's `<table>`).
fn t14(markup: &str, legs: &str) -> String {
    cell(&format!(
        "var el = document.body; el.innerHTML = '{markup}'; var f = el.firstChild; r.push({legs});"
    ))
}

const T14_STRICT: &str = "<template><p></p></template>";
const T14_TOLERANT: &str = "<template><p></p></template></br>";
const T14_LEGS: &str = "f.content.ownerDocument === tpl.content.ownerDocument, \
    f.content.firstChild.ownerDocument === tpl.content.ownerDocument, \
    f.content.ownerDocument !== document";
const T14_CONTROL_LEGS: &str =
    "f.content.firstChild.ownerDocument === f.content.ownerDocument, f.tagName === 'TEMPLATE'";

#[test]
fn t14_fragment_parsed_template_strict() {
    assert_eq!(t14(T14_STRICT, T14_LEGS), "true,true,true");
}

#[test]
fn t14_fragment_parsed_template_tolerant() {
    assert_eq!(t14(T14_TOLERANT, T14_LEGS), "true,true,true");
}

#[test]
fn t14_control() {
    assert_eq!(t14(T14_STRICT, T14_CONTROL_LEGS), "true,true");
    assert_eq!(
        t14(
            T14_TOLERANT,
            &format!("{T14_CONTROL_LEGS}, el.lastChild.tagName === 'BR'")
        ),
        "true,true,true"
    );
}

// ---- T19 (×2): foreign content in parsed contents, then moved out ----
const T19_HTML: &str = "<!DOCTYPE html><html><head></head><body>\
    <template id=s><svg></svg><b></b></template></body></html>";
const T19: &str = "var s = document.getElementById('s'); var v = s.content.firstChild; \
    var w = s.content.lastChild;";

fn t19(strict: bool, inside: &str) -> String {
    parsed_page_cell(
        strict,
        T19_HTML,
        &format!(
            "{T19} r.push({inside}); document.body.append(s.content); \
             r.push(v.ownerDocument === document, w.ownerDocument === document);"
        ),
    )
}

const T19_INSIDE: &str =
    "v.ownerDocument !== document, v.ownerDocument === s.content.ownerDocument";

#[test]
fn t19_foreign_contents_strict() {
    assert_eq!(t19(true, T19_INSIDE), "true,true,true,true");
}

#[test]
fn t19_foreign_contents_tolerant() {
    assert_eq!(t19(false, T19_INSIDE), "true,true,true,true");
}

#[test]
fn t19_control() {
    let inside = "v.ownerDocument === s.content.ownerDocument";
    assert_eq!(t19(true, inside), "true,true,true");
    assert_eq!(t19(false, inside), "true,true,true");
}

// ---- T20 (R-C2): URL resolution in contents ----
#[test]
fn t20_contents_href_resolves_against_the_inert_document() {
    let got = head_body_cell(
        None,
        &format!(
            "{BASE_HREF} tpl.innerHTML = '<a id=a href=x.html></a>'; \
             r.push(tpl.content.firstChild.href);"
        ),
    );
    assert_eq!(got, "x.html");
}

#[test]
fn t20_control() {
    let got = head_body_cell(
        None,
        &format!(
            "{BASE_HREF} var a = document.createElement('a'); a.setAttribute('href', 'x.html'); \
             document.body.append(a); r.push(a.href);"
        ),
    );
    assert_eq!(got, "http://example.com/dir/x.html");
}

// ---- T22: a raw handler in contents reads null ----
const T22: &str = "tpl.innerHTML = '<b onclick=\"1\">'; var x = tpl.content.firstChild;";

#[test]
fn t22_contents_raw_handler_reads_null() {
    assert_eq!(cell(&format!("{T22} r.push(x.onclick === null);")), "true");
}

#[test]
fn t22_control() {
    assert_eq!(
        cell(&format!(
            "{T22} r.push(x.getAttribute('onclick') === '1', x.ownerDocument === tpl.content.ownerDocument);"
        )),
        "true,true"
    );
}

// ---- T23 (R-A closure): every route into contents ----
const T23_I_HTML: &str = "<!DOCTYPE html><html><head></head><body>\
    <template id=p><b onfocus=\"window.hit=1\"></b></template></body></html>";
const T23_LEGS: &str =
    "b.ownerDocument === tpl.content.ownerDocument, b.ownerDocument !== document";
const T23_CONTROL_LEGS: &str = "b.ownerDocument === tpl.content.ownerDocument";

fn t23_parsed(strict: bool, legs: &str) -> String {
    parsed_page_cell(
        strict,
        T23_I_HTML,
        &format!("var tpl = document.getElementById('p'); var b = tpl.content.firstChild; r.push({legs});"),
    )
}

const T23_II: &str = "var b = document.createElement('b'); \
    b.setAttribute('onfocus', 'window.hit=1'); tpl.content.append(b);";
const T23_III: &str = "tpl.innerHTML = '<b onfocus=\"window.hit=1\"></b>'; \
    var c = tpl.content.cloneNode(true); var b = c.firstChild;";
const T23_IV: &str = "tpl.innerHTML = '<b onfocus=\"window.hit=1\"></b>'; \
    var c = tpl.cloneNode(true); var b = c.content.firstChild;";

#[test]
fn t23_route_i_parsed_strict() {
    assert_eq!(t23_parsed(true, T23_LEGS), "true,true");
}

#[test]
fn t23_route_i_parsed_tolerant() {
    assert_eq!(t23_parsed(false, T23_LEGS), "true,true");
}

#[test]
fn t23_route_ii_appended() {
    assert_eq!(
        cell(&format!("{T23_II} r.push({T23_LEGS}, b.onfocus === null);")),
        "true,true,true"
    );
}

#[test]
fn t23_route_iii_contents_cloned() {
    assert_eq!(cell(&format!("{T23_III} r.push({T23_LEGS});")), "true,true");
}

#[test]
fn t23_route_iv_template_cloned() {
    assert_eq!(cell(&format!("{T23_IV} r.push({T23_LEGS});")), "true,true");
}

/// Route (v), the condition's control (INV-C (c)): contents adopted whole
/// into the document are in the document, and the raw value compiles.
#[test]
fn t23_route_v_contents_adopted_out() {
    let got = cell(
        "document.adoptNode(tpl.content); var b = document.createElement('b'); \
         b.setAttribute('onfocus', 'window.hit=1'); tpl.content.append(b); \
         r.push(b.ownerDocument === document, typeof b.onfocus === 'function');",
    );
    assert_eq!(got, "true,true");
}

#[test]
fn t23_control() {
    assert_eq!(t23_parsed(true, T23_CONTROL_LEGS), "true");
    assert_eq!(t23_parsed(false, T23_CONTROL_LEGS), "true");
    for route in [T23_II, T23_III, T23_IV] {
        assert_eq!(
            cell(&format!("{route} r.push({T23_CONTROL_LEGS});")),
            "true",
            "{route}"
        );
    }
}

// ---- T24: a clonable shadow host in contents, cloned with its template ----
const T24: &str = "tpl.innerHTML = '<div id=h></div>'; var h = tpl.content.firstChild; \
    h.attachShadow({mode:'open', clonable:true}).innerHTML = '<i></i>'; \
    var c = tpl.cloneNode(true); var ch = c.content.firstChild;";

#[test]
fn t24_cloned_shadow_host_in_contents() {
    let got = cell(&format!(
        "{T24} r.push(ch.ownerDocument !== document, ch.shadowRoot.ownerDocument !== document, \
         ch.shadowRoot.firstChild.ownerDocument !== document);"
    ));
    assert_eq!(got, "true,true,true");
}

#[test]
fn t24_control() {
    let got = cell(&format!(
        "{T24} r.push(ch.shadowRoot.ownerDocument === ch.ownerDocument, \
         ch.shadowRoot.firstChild.ownerDocument === ch.ownerDocument);"
    ));
    assert_eq!(got, "true,true");
}

// ---- T25 (R-C8): adoptedCallback pair values for contents ----
const T25: &str = "globalThis.I = tpl.content.ownerDocument; \
    globalThis.dp = new DOMParser().parseFromString('<template id=t></template>', 'text/html'); \
    globalThis.I_dp = dp.getElementById('t').content.ownerDocument; \
    globalThis.e = document.createElement('x-a'); dp.getElementById('t').content.append(e); \
    document.body.append(dp.getElementById('t'));";

#[test]
fn t25_adopted_callback_pairs_in_contents() {
    let got = ce_cell(&[
        T25,
        "r.push(log.length, log[0][0] === document, log[0][1] === I_dp, log[0][1] !== dp, \
         log[1][0] === I_dp, log[1][1] === I, log[1][1] !== document);",
    ]);
    assert_eq!(got, "2,true,true,true,true,true,true");
}

#[test]
fn t25_control() {
    let got = ce_cell(&[T25, "r.push(log.length, log[0][0] === document);"]);
    assert_eq!(got, "2,true");
}

// ---- T26: the adopt computes f(target) on first use ----
const T26: &str = "var dp = new DOMParser().parseFromString('<p></p>', 'text/html'); \
    tpl.content.append(document.createElement('i')); dp.body.append(tpl); \
    var x = tpl.content.ownerDocument;";

#[test]
fn t26_adopt_creates_the_targets_inert_document() {
    let got = cell(&format!(
        "{T26} r.push(x !== dp, x !== document, tpl.content.firstChild.ownerDocument === x, \
         dp.createElement('template').content.ownerDocument === x);"
    ));
    assert_eq!(got, "true,true,true,true");
}

#[test]
fn t26_control() {
    let got = cell(&format!(
        "{T26} r.push(x !== document, tpl.content.firstChild.ownerDocument === x, \
         dp.createElement('template').content.ownerDocument === x);"
    ));
    assert_eq!(got, "true,true,true");
}

// ---- T27: C0c's skip compares with the mapped new (forward) ----
const T27: [&str; 2] = [
    "tpl.innerHTML = '<template id=n></template>'; globalThis.n = tpl.content.firstChild; \
     globalThis.e = document.createElement('x-a'); n.content.append(e);",
    "log = []; document.body.append(n);",
];

#[test]
fn t27_same_inert_document_segment_is_skipped() {
    let got = ce_cell(&[
        T27[0],
        T27[1],
        "r.push(log.length, e.ownerDocument !== document, n.ownerDocument === document);",
    ]);
    assert_eq!(got, "0,true,true");
}

#[test]
fn t27_control() {
    let got = ce_cell(&[
        T27[0],
        T27[1],
        "r.push(log.length, n.ownerDocument === document);",
    ]);
    assert_eq!(got, "0,true");
}

// ---- T28: C0c's skip compares with the mapped new (reverse) ----
const T28: [&str; 2] = [
    "globalThis.dp = new DOMParser().parseFromString('<template id=t></template>', 'text/html'); \
     globalThis.I_dp = dp.getElementById('t').content.ownerDocument; \
     globalThis.t = document.createElement('template'); globalThis.e = document.createElement('x-a'); \
     t.content.append(e); dp.adoptNode(t.content);",
    "log = []; dp.body.append(t);",
];

#[test]
fn t28_contents_in_target_go_to_its_inert_document() {
    let got = ce_cell(&[
        T28[0],
        T28[1],
        "r.push(log.length, log.length && log[0][0] === dp, log.length && log[0][1] === I_dp, \
         t.content.ownerDocument !== dp, e.ownerDocument === t.content.ownerDocument);",
    ]);
    assert_eq!(got, "1,true,true,true,true");
}

#[test]
fn t28_control() {
    let got = ce_cell(&[
        T28[0],
        T28[1],
        "r.push(e.ownerDocument === t.content.ownerDocument, t.ownerDocument === dp);",
    ]);
    assert_eq!(got, "true,true");
}

// ---- T29 (R-C2, DOMParser): URL resolution in a DOMParser document's contents ----
const T29: &str = "var dp = new DOMParser().parseFromString(\
    '<a id=b href=x.html></a><template id=t><a href=x.html></a></template>', 'text/html');";

#[test]
fn t29_domparser_contents_href() {
    let got = head_body_cell(
        Some("http://example.com/dir/page"),
        &format!(
            "{T29} r.push(dp.getElementById('b').href, dp.getElementById('t').content.firstChild.href);"
        ),
    );
    assert_eq!(got, "http://example.com/dir/x.html,x.html");
}

#[test]
fn t29_control() {
    let got = head_body_cell(
        Some("http://example.com/dir/page"),
        &format!("{T29} r.push(dp.getElementById('b').href);"),
    );
    assert_eq!(got, "http://example.com/dir/x.html");
}
