//! Element `localName` / `namespaceURI` / `prefix` accessors and the
//! namespace-aware `tagName` / `nodeName` (WHATWG DOM §4.9 *Element*,
//! §4.4 *Node* `nodeName`) — slot `#11-element-localname-null`.

use super::super::test_helpers::{eval_str, with_page};

/// Evaluate `setup`, then `JSON.stringify` the completion of `expr`.
fn cell(setup: &str, expr: &str) -> String {
    let mut out = String::new();
    with_page(|vm| {
        vm.eval(setup).expect("setup");
        vm.eval(&format!("globalThis.__out = JSON.stringify({expr});"))
            .expect("read");
        out = eval_str(vm, "__out");
    });
    out
}

const SVG: &str = "globalThis.h = document.createElement('div'); document.body.append(h); \
    h.innerHTML = '<svg><foreignObject id=f></foreignObject></svg>'; \
    globalThis.f = h.firstChild.firstChild;";

const MATHML: &str = "globalThis.h = document.createElement('div'); document.body.append(h); \
    h.innerHTML = '<math><mi id=m></mi></math>'; \
    globalThis.m = h.firstChild.firstChild;";

#[test]
fn html_element_names() {
    let got = cell(
        "globalThis.i = document.createElement('i');",
        "[i.localName, i.namespaceURI, i.prefix, i.tagName, i.nodeName]",
    );
    assert_eq!(got, r#"["i","http://www.w3.org/1999/xhtml",null,"I","I"]"#);
}

#[test]
fn svg_element_preserves_case() {
    let got = cell(
        SVG,
        "[f.localName, f.tagName, f.nodeName, f.namespaceURI, f.prefix]",
    );
    assert_eq!(
        got,
        r#"["foreignObject","foreignObject","foreignObject","http://www.w3.org/2000/svg",null]"#
    );
}

#[test]
fn mathml_element_namespace() {
    let got = cell(MATHML, "[m.localName, m.namespaceURI, m.tagName]");
    assert_eq!(got, r#"["mi","http://www.w3.org/1998/Math/MathML","mi"]"#);
}

#[test]
fn accessors_are_own_properties_of_element_prototype() {
    // The prototype that owns `tagName` is `Element.prototype`.
    let got = cell(
        "globalThis.ep = Object.getPrototypeOf(document.createElement('i')); \
         while (ep && !Object.prototype.hasOwnProperty.call(ep, 'tagName')) { \
           ep = Object.getPrototypeOf(ep); }",
        "['localName', 'namespaceURI', 'prefix'].map(n => { \
           const d = Object.getOwnPropertyDescriptor(ep, n); \
           return !!d && typeof d.get === 'function' && d.set === undefined; })",
    );
    assert_eq!(got, "[true,true,true]");
}

#[test]
fn attr_local_name_regression() {
    let got = cell(
        "globalThis.e = document.createElement('div'); e.setAttribute('data-x', '1'); \
         globalThis.a = e.getAttributeNode('data-x');",
        "[a.localName, a.namespaceURI, a.prefix]",
    );
    assert_eq!(got, r#"["data-x",null,null]"#);
}
