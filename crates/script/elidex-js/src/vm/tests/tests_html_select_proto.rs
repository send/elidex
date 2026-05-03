//! M4-12 slot #11-tags-T1 Phase 7 — `HTMLSelectElement.prototype` +
//! `HTMLOptionsCollection.prototype` tests.
//!
//! Covers reflected attributes (autocomplete, disabled, multiple,
//! name, required, size), the derived `type` getter, `length` /
//! `options` / `selectedOptions` / `selectedIndex` / `value`
//! accessors, the `add()` / `remove()` / `item()` / `namedItem()`
//! proxy methods, and the mutable `HTMLOptionsCollection` surface
//! (length setter, add(opt, before?), remove(idx)).

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_empty_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

fn run(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_empty_doc(&mut dom);
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

// --- Prototype identity --------------------------------------------

#[test]
fn select_wrapper_has_html_select_prototype() {
    let out = run("var s1 = document.createElement('select'); \
         var s2 = document.createElement('select'); \
         var proto = Object.getPrototypeOf(s1); \
         var same = Object.getPrototypeOf(s2) === proto; \
         var hasOptions = Object.getOwnPropertyDescriptor(proto, 'options') !== undefined; \
         var hasType = Object.getOwnPropertyDescriptor(proto, 'type') !== undefined; \
         (same && hasOptions && hasType) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// --- Reflected attrs -----------------------------------------------

#[test]
fn select_string_attrs_round_trip() {
    let out = run("var s = document.createElement('select'); \
         s.autocomplete = 'on'; \
         s.name = 'choice'; \
         s.autocomplete + '|' + s.name;");
    assert_eq!(out, "on|choice");
}

#[test]
fn select_disabled_round_trip() {
    let out = run("var s = document.createElement('select'); \
         s.disabled = true; \
         var on = s.disabled + '|' + s.hasAttribute('disabled'); \
         s.disabled = false; \
         on + '/' + s.disabled;");
    assert_eq!(out, "true|true/false");
}

#[test]
fn select_multiple_round_trip() {
    let out = run("var s = document.createElement('select'); \
         s.multiple = true; \
         s.multiple + '|' + s.hasAttribute('multiple');");
    assert_eq!(out, "true|true");
}

#[test]
fn select_required_round_trip() {
    let out = run("var s = document.createElement('select'); \
         s.required = true; \
         s.required.toString();");
    assert_eq!(out, "true");
}

// --- size ---------------------------------------------------------

#[test]
fn select_size_default_is_one_when_single() {
    let out = run("var s = document.createElement('select'); \
         s.size.toString();");
    assert_eq!(out, "1");
}

#[test]
fn select_size_default_is_four_when_multiple() {
    let out = run("var s = document.createElement('select'); \
         s.multiple = true; \
         s.size.toString();");
    assert_eq!(out, "4");
}

#[test]
fn select_size_round_trip() {
    let out = run("var s = document.createElement('select'); \
         s.size = 8; \
         s.size + '|' + s.getAttribute('size');");
    assert_eq!(out, "8|8");
}

// --- type (derived) -----------------------------------------------

#[test]
fn select_type_is_select_one_by_default() {
    let out = run("var s = document.createElement('select'); \
         s.type;");
    assert_eq!(out, "select-one");
}

#[test]
fn select_type_is_select_multiple_when_multiple_attr_present() {
    let out = run("var s = document.createElement('select'); \
         s.multiple = true; \
         s.type;");
    assert_eq!(out, "select-multiple");
}

// --- length / options / selectedOptions ---------------------------

#[test]
fn select_length_starts_zero() {
    let out = run("var s = document.createElement('select'); \
         s.length.toString();");
    assert_eq!(out, "0");
}

#[test]
fn select_length_counts_appended_options() {
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         s.appendChild(o1); s.appendChild(o2); \
         s.length.toString();");
    assert_eq!(out, "2");
}

#[test]
fn select_options_is_html_options_collection() {
    let out = run("var s = document.createElement('select'); \
         var o = document.createElement('option'); \
         s.appendChild(o); \
         var opts = s.options; \
         (opts.length === 1) + '|' + (opts.item(0) === o) + '|' + (typeof opts.add === 'function');");
    assert_eq!(out, "true|true|true");
}

#[test]
fn select_options_includes_optgroup_nested_options() {
    let out = run("var s = document.createElement('select'); \
         var g = document.createElement('optgroup'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         g.appendChild(o1); g.appendChild(o2); \
         s.appendChild(g); \
         s.options.length.toString();");
    assert_eq!(out, "2");
}

#[test]
fn select_selected_options_filters_to_selected_only() {
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         o2.selected = true; \
         s.appendChild(o1); s.appendChild(o2); \
         var sel = s.selectedOptions; \
         sel.length + '|' + (sel.item(0) === o2);");
    assert_eq!(out, "1|true");
}

// --- selectedIndex (RW) -------------------------------------------

#[test]
fn select_selected_index_is_negative_one_when_none_selected() {
    let out = run("var s = document.createElement('select'); \
         var o = document.createElement('option'); \
         s.appendChild(o); \
         s.selectedIndex.toString();");
    assert_eq!(out, "-1");
}

#[test]
fn select_selected_index_returns_first_selected() {
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         o2.selected = true; \
         s.appendChild(o1); s.appendChild(o2); \
         s.selectedIndex.toString();");
    assert_eq!(out, "1");
}

#[test]
fn select_selected_index_setter_clears_others_and_sets_target() {
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         var o3 = document.createElement('option'); \
         o1.selected = true; \
         s.appendChild(o1); s.appendChild(o2); s.appendChild(o3); \
         s.selectedIndex = 2; \
         (s.options.item(0).selected ? '1' : '0') + ',' + \
         (s.options.item(1).selected ? '1' : '0') + ',' + \
         (s.options.item(2).selected ? '1' : '0');");
    assert_eq!(out, "0,0,1");
}

#[test]
fn select_selected_index_setter_negative_clears_all() {
    let out = run("var s = document.createElement('select'); \
         var o = document.createElement('option'); \
         o.selected = true; \
         s.appendChild(o); \
         s.selectedIndex = -1; \
         o.selected.toString();");
    assert_eq!(out, "false");
}

// --- value (RW) ---------------------------------------------------

#[test]
fn select_value_returns_first_selected_options_value() {
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); o1.value = 'a'; \
         var o2 = document.createElement('option'); o2.value = 'b'; \
         o2.selected = true; \
         s.appendChild(o1); s.appendChild(o2); \
         s.value;");
    assert_eq!(out, "b");
}

#[test]
fn select_value_setter_selects_matching_option() {
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); o1.value = 'a'; \
         var o2 = document.createElement('option'); o2.value = 'b'; \
         var o3 = document.createElement('option'); o3.value = 'c'; \
         s.appendChild(o1); s.appendChild(o2); s.appendChild(o3); \
         s.value = 'b'; \
         (o1.selected ? '1' : '0') + ',' + \
         (o2.selected ? '1' : '0') + ',' + \
         (o3.selected ? '1' : '0');");
    assert_eq!(out, "0,1,0");
}

#[test]
fn select_value_setter_no_match_clears_all() {
    let out = run("var s = document.createElement('select'); \
         var o = document.createElement('option'); o.value = 'a'; \
         o.selected = true; \
         s.appendChild(o); \
         s.value = 'no-match'; \
         o.selected.toString();");
    assert_eq!(out, "false");
}

// --- length setter (mutable HTMLOptionsCollection -----------------

#[test]
fn select_length_setter_truncates() {
    let out = run("var s = document.createElement('select'); \
         for (var i = 0; i < 5; i++) s.appendChild(document.createElement('option')); \
         s.length = 2; \
         s.length.toString();");
    assert_eq!(out, "2");
}

#[test]
fn select_length_setter_extends_with_empty_options() {
    let out = run("var s = document.createElement('select'); \
         s.length = 3; \
         s.length + '|' + s.options.item(0).tagName;");
    assert_eq!(out, "3|OPTION");
}

#[test]
fn options_collection_length_setter_works_directly() {
    let out = run("var s = document.createElement('select'); \
         for (var i = 0; i < 4; i++) s.appendChild(document.createElement('option')); \
         s.options.length = 1; \
         s.options.length.toString();");
    assert_eq!(out, "1");
}

// --- add / remove -------------------------------------------------

#[test]
fn select_add_appends_option_when_before_is_null() {
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         s.add(o1); \
         s.add(o2); \
         s.length + '|' + (s.options.item(0) === o1) + '|' + (s.options.item(1) === o2);");
    assert_eq!(out, "2|true|true");
}

#[test]
fn select_add_inserts_before_index() {
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); o1.value = 'a'; \
         var o2 = document.createElement('option'); o2.value = 'b'; \
         var o3 = document.createElement('option'); o3.value = 'c'; \
         s.add(o1); s.add(o3); \
         s.add(o2, 1); \
         s.options.item(0).value + s.options.item(1).value + s.options.item(2).value;");
    assert_eq!(out, "abc");
}

#[test]
fn select_add_inserts_before_element() {
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); o1.value = 'a'; \
         var o3 = document.createElement('option'); o3.value = 'c'; \
         s.add(o1); s.add(o3); \
         var o2 = document.createElement('option'); o2.value = 'b'; \
         s.add(o2, o3); \
         s.options.item(0).value + s.options.item(1).value + s.options.item(2).value;");
    assert_eq!(out, "abc");
}

#[test]
fn select_remove_with_index_removes_option() {
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         var o3 = document.createElement('option'); \
         s.add(o1); s.add(o2); s.add(o3); \
         s.remove(1); \
         s.length + '|' + (s.options.item(1) === o3);");
    assert_eq!(out, "2|true");
}

#[test]
fn select_remove_with_no_args_removes_self_from_parent() {
    let out = run("var s = document.createElement('select'); \
         document.body.appendChild(s); \
         s.remove(); \
         (s.parentNode === null) ? 'detached' : 'attached';");
    assert_eq!(out, "detached");
}

#[test]
fn select_remove_negative_index_is_noop() {
    let out = run("var s = document.createElement('select'); \
         s.add(document.createElement('option')); \
         s.remove(-1); \
         s.length.toString();");
    assert_eq!(out, "1");
}

// --- item / namedItem ---------------------------------------------

#[test]
fn select_item_returns_option_at_index() {
    let out = run("var s = document.createElement('select'); \
         var o = document.createElement('option'); \
         s.appendChild(o); \
         (s.item(0) === o) + '|' + (s.item(5) === null);");
    assert_eq!(out, "true|true");
}

#[test]
fn select_named_item_id_match_wins() {
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); o1.id = 'x'; \
         var o2 = document.createElement('option'); o2.name = 'x'; \
         s.appendChild(o2); s.appendChild(o1); \
         (s.namedItem('x') === o1) ? 'id-wins' : 'name-wins';");
    assert_eq!(out, "id-wins");
}

#[test]
fn select_named_item_returns_null_for_unknown() {
    let out = run("var s = document.createElement('select'); \
         (s.namedItem('nothing') === null) ? 'null' : 'wrong';");
    assert_eq!(out, "null");
}

// --- HTMLOptionsCollection mutable surface -------------------------

#[test]
fn options_collection_add_throws_for_non_option_element() {
    let out = run("var s = document.createElement('select'); \
         var div = document.createElement('div'); \
         try { s.options.add(div); 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "TypeError");
}

#[test]
fn options_collection_remove_works_directly() {
    let out = run("var s = document.createElement('select'); \
         s.appendChild(document.createElement('option')); \
         s.appendChild(document.createElement('option')); \
         s.options.remove(0); \
         s.length.toString();");
    assert_eq!(out, "1");
}

#[test]
fn options_collection_length_setter_extends_zero_to_n() {
    let out = run("var s = document.createElement('select'); \
         s.options.length = 5; \
         s.options.length.toString();");
    assert_eq!(out, "5");
}

// --- form / labels -------------------------------------------------

#[test]
fn select_form_resolves_through_ancestor() {
    let out = run("var f = document.createElement('form'); \
         var s = document.createElement('select'); \
         f.appendChild(s); \
         document.body.appendChild(f); \
         (s.form === f) ? 'same' : 'other';");
    assert_eq!(out, "same");
}

#[test]
fn select_labels_collects_for_id_match() {
    let out = run("var s = document.createElement('select'); \
         s.id = 'pick'; \
         var lbl = document.createElement('label'); \
         lbl.htmlFor = 'pick'; \
         document.body.appendChild(s); \
         document.body.appendChild(lbl); \
         var nl = s.labels; \
         nl.length + '|' + (nl.item(0) === lbl);");
    assert_eq!(out, "1|true");
}

// --- Brand check ---------------------------------------------------

#[test]
fn select_options_throws_on_non_select_receiver() {
    let out = run("var s = document.createElement('select'); \
         var div = document.createElement('div'); \
         var getter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(s), 'options').get; \
         try { getter.call(div); 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "TypeError");
}

#[test]
fn options_collection_length_setter_throws_on_non_options_receiver() {
    let out = run("var s = document.createElement('select'); \
         s.appendChild(document.createElement('option')); \
         var coll = s.options; \
         var fcc = document.body.children; \
         var setter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(coll), 'length').set; \
         try { setter.call(fcc, 0); 'no throw'; } catch (e) { e.name; }");
    assert_eq!(out, "TypeError");
}
