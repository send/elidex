//! Slot `#11-tags-T1-v2` Phase 2 — `HTMLOptionElement.prototype`
//! coverage.

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

// ---------------------------------------------------------------------------
// disabled (boolean reflect)
// ---------------------------------------------------------------------------

#[test]
fn option_disabled_default_false() {
    let out = run("var o = document.createElement('option'); '' + o.disabled;");
    assert_eq!(out, "false");
}

#[test]
fn option_disabled_setter_round_trip() {
    let out = run("var o = document.createElement('option'); \
         o.disabled = true; \
         '' + o.hasAttribute('disabled');");
    assert_eq!(out, "true");
}

// ---------------------------------------------------------------------------
// label / value DOMString reflect with text fallback
// ---------------------------------------------------------------------------

#[test]
fn option_label_default_empty() {
    let out = run("var o = document.createElement('option'); o.label;");
    assert_eq!(out, "");
}

#[test]
fn option_label_attr_round_trip() {
    let out = run("var o = document.createElement('option'); \
         o.label = 'lbl'; \
         o.label + '/' + o.getAttribute('label');");
    assert_eq!(out, "lbl/lbl");
}

#[test]
fn option_label_falls_back_to_text() {
    let out = run("var o = document.createElement('option'); \
         o.text = 'Hello'; \
         o.label;");
    assert_eq!(out, "Hello");
}

#[test]
fn option_value_default_empty() {
    let out = run("var o = document.createElement('option'); o.value;");
    assert_eq!(out, "");
}

#[test]
fn option_value_attr_round_trip() {
    let out = run("var o = document.createElement('option'); \
         o.value = 'v1'; \
         o.value + '/' + o.getAttribute('value');");
    assert_eq!(out, "v1/v1");
}

#[test]
fn option_value_falls_back_to_text() {
    let out = run("var o = document.createElement('option'); \
         o.text = 'World'; \
         o.value;");
    assert_eq!(out, "World");
}

// ---------------------------------------------------------------------------
// text — reads/writes textContent
// ---------------------------------------------------------------------------

#[test]
fn option_text_default_empty() {
    let out = run("var o = document.createElement('option'); o.text;");
    assert_eq!(out, "");
}

#[test]
fn option_text_setter_replaces_children() {
    let out = run("var o = document.createElement('option'); \
         o.text = 'Hello'; \
         o.textContent;");
    assert_eq!(out, "Hello");
}

#[test]
fn option_text_setter_coerces_to_string() {
    let out = run("var o = document.createElement('option'); \
         o.text = 42; \
         o.text;");
    assert_eq!(out, "42");
}

// ---------------------------------------------------------------------------
// defaultSelected / selected
// ---------------------------------------------------------------------------

#[test]
fn option_default_selected_false_default() {
    let out = run("var o = document.createElement('option'); '' + o.defaultSelected;");
    assert_eq!(out, "false");
}

#[test]
fn option_default_selected_round_trip() {
    let out = run("var o = document.createElement('option'); \
         o.defaultSelected = true; \
         '' + o.hasAttribute('selected');");
    assert_eq!(out, "true");
}

#[test]
fn option_selected_aliased_to_default() {
    let out = run("var o = document.createElement('option'); \
         o.selected = true; \
         '' + o.hasAttribute('selected') + '/' + o.defaultSelected;");
    assert_eq!(out, "true/true");
}

// ---------------------------------------------------------------------------
// index
// ---------------------------------------------------------------------------

#[test]
fn option_index_minus_one_when_no_select_parent() {
    let out = run("var o = document.createElement('option'); '' + o.index;");
    assert_eq!(out, "-1");
}

#[test]
fn option_index_position_in_select() {
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         var o3 = document.createElement('option'); \
         s.appendChild(o1); s.appendChild(o2); s.appendChild(o3); \
         o1.index + '/' + o2.index + '/' + o3.index;");
    assert_eq!(out, "0/1/2");
}

#[test]
fn option_index_traverses_optgroup() {
    let out = run("var s = document.createElement('select'); \
         var g = document.createElement('optgroup'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         g.appendChild(o2); \
         s.appendChild(o1); s.appendChild(g); \
         o1.index + '/' + o2.index;");
    assert_eq!(out, "0/1");
}

#[test]
fn option_index_recognises_datalist_parent() {
    // R8 F2 regression — HTML §4.10.10.2: option.index counts
    // position in the select.options / datalist.options list, so
    // a datalist parent must also yield a valid index.
    let out = run("var dl = document.createElement('datalist'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         dl.appendChild(o1); dl.appendChild(o2); \
         o1.index + '/' + o2.index;");
    assert_eq!(out, "0/1");
}

#[test]
fn option_index_walks_through_nested_optgroups() {
    // R18 F1 regression — HTML disallows nested `<optgroup>` but
    // JS `appendChild` can construct one.  `option.index` should
    // walk arbitrarily-deep optgroup ancestors until finding the
    // enclosing `<select>` / `<datalist>` container, not return
    // -1 just because the immediate / grandparent container check
    // fails.  `walk_options` already recurses through nested
    // optgroups, so once the container is found the index is
    // computed correctly.
    let out = run("var s = document.createElement('select'); \
         var g1 = document.createElement('optgroup'); \
         var g2 = document.createElement('optgroup'); \
         var o = document.createElement('option'); \
         g2.appendChild(o); \
         g1.appendChild(g2); \
         s.appendChild(g1); \
         '' + o.index;");
    assert_eq!(out, "0");
}

#[test]
fn option_index_recognises_optgroup_under_datalist() {
    // R8 F2 regression — optgroup nesting under datalist is also
    // valid per HTML §4.10.9 / §4.10.10.
    let out = run("var dl = document.createElement('datalist'); \
         var g = document.createElement('optgroup'); \
         var o = document.createElement('option'); \
         g.appendChild(o); \
         dl.appendChild(g); \
         '' + o.index;");
    assert_eq!(out, "0");
}

// ---------------------------------------------------------------------------
// form
// ---------------------------------------------------------------------------

#[test]
fn option_form_null_when_no_select_ancestor() {
    let out = run("var o = document.createElement('option'); \
         (o.form === null) ? 'null' : 'non-null';");
    assert_eq!(out, "null");
}

#[test]
fn option_form_resolves_via_select_ancestor() {
    let out = run("var f = document.createElement('form'); \
         var s = document.createElement('select'); \
         var o = document.createElement('option'); \
         s.appendChild(o); f.appendChild(s); \
         (o.form === f) ? 'same' : 'diff';");
    assert_eq!(out, "same");
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

#[test]
fn option_brand_check_throws_on_non_option_receiver() {
    let out = run("var d = document.createElement('div'); \
         var o = document.createElement('option'); \
         var getter = Object.getOwnPropertyDescriptor(\
             Object.getPrototypeOf(o), 'value').get; \
         try { getter.call(d); 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'type' : 'other'; }");
    assert_eq!(out, "type");
}

#[test]
fn option_prototype_chains_through_html_element() {
    let out = run("var o = document.createElement('option'); \
         var p = Object.getPrototypeOf(o); \
         var pp = Object.getPrototypeOf(p); \
         (pp === Object.getPrototypeOf(document.createElement('div'))) ? 'good' : 'bad';");
    assert_eq!(out, "good");
}
