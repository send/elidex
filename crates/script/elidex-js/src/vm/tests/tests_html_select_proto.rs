//! Slot `#11-tags-T1-v2` Phase 7 — `HTMLSelectElement.prototype` +
//! options live collection (Options variant) +
//! form.elements / fieldset.elements (FormControls variant) coverage.

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
// HTMLSelectElement basic surface
// ---------------------------------------------------------------------------

#[test]
fn select_type_default_select_one() {
    let out = run("var s = document.createElement('select'); s.type;");
    assert_eq!(out, "select-one");
}

#[test]
fn select_type_select_multiple_when_multiple_set() {
    let out = run("var s = document.createElement('select'); \
         s.multiple = true; \
         s.type;");
    assert_eq!(out, "select-multiple");
}

#[test]
fn select_size_default_zero() {
    let out = run("var s = document.createElement('select'); '' + s.size;");
    assert_eq!(out, "0");
}

#[test]
fn select_size_round_trip() {
    let out = run("var s = document.createElement('select'); \
         s.size = 5; \
         '' + s.size;");
    assert_eq!(out, "5");
}

#[test]
fn select_disabled_round_trip() {
    let out = run("var s = document.createElement('select'); \
         s.disabled = true; \
         '' + s.hasAttribute('disabled');");
    assert_eq!(out, "true");
}

#[test]
fn select_required_round_trip() {
    let out = run("var s = document.createElement('select'); \
         s.required = true; \
         '' + s.hasAttribute('required');");
    assert_eq!(out, "true");
}

#[test]
fn select_name_round_trip() {
    let out = run("var s = document.createElement('select'); \
         s.name = 'choice'; \
         s.name;");
    assert_eq!(out, "choice");
}

#[test]
fn select_form_resolves_via_form_ancestor() {
    let out = run("var f = document.createElement('form'); \
         var s = document.createElement('select'); \
         f.appendChild(s); \
         (s.form === f) ? 'same' : 'diff';");
    assert_eq!(out, "same");
}

// ---------------------------------------------------------------------------
// options live collection
// ---------------------------------------------------------------------------

#[test]
fn select_options_returns_collection() {
    let out = run("var s = document.createElement('select'); \
         (s.options != null && typeof s.options.length === 'number') ? 'ok' : 'bad';");
    assert_eq!(out, "ok");
}

#[test]
fn select_options_same_object() {
    let out = run("var s = document.createElement('select'); \
         (s.options === s.options) ? 'same' : 'diff';");
    assert_eq!(out, "same");
}

#[test]
fn select_options_length_reflects_children() {
    let out = run("var s = document.createElement('select'); \
         s.appendChild(document.createElement('option')); \
         s.appendChild(document.createElement('option')); \
         s.appendChild(document.createElement('option')); \
         '' + s.options.length;");
    assert_eq!(out, "3");
}

#[test]
fn select_length_mirrors_options_length() {
    let out = run("var s = document.createElement('select'); \
         s.appendChild(document.createElement('option')); \
         s.appendChild(document.createElement('option')); \
         '' + s.length;");
    assert_eq!(out, "2");
}

#[test]
fn select_options_traverses_optgroup() {
    let out = run("var s = document.createElement('select'); \
         var g = document.createElement('optgroup'); \
         g.appendChild(document.createElement('option')); \
         g.appendChild(document.createElement('option')); \
         s.appendChild(g); \
         s.appendChild(document.createElement('option')); \
         '' + s.options.length;");
    assert_eq!(out, "3");
}

#[test]
fn select_item_returns_option_by_index() {
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         s.appendChild(o1); s.appendChild(o2); \
         (s.item(0) === o1 && s.item(1) === o2) ? 'ok' : 'bad';");
    assert_eq!(out, "ok");
}

#[test]
fn select_named_item_finds_by_name_attribute() {
    let out = run("var s = document.createElement('select'); \
         var o = document.createElement('option'); \
         o.setAttribute('name', 'foo'); \
         s.appendChild(o); \
         (s.namedItem('foo') === o) ? 'same' : 'diff';");
    assert_eq!(out, "same");
}

#[test]
fn select_named_item_finds_by_id() {
    let out = run("var s = document.createElement('select'); \
         var o = document.createElement('option'); \
         o.setAttribute('id', 'bar'); \
         s.appendChild(o); \
         (s.namedItem('bar') === o) ? 'same' : 'diff';");
    assert_eq!(out, "same");
}

#[test]
fn select_add_appends_option() {
    let out = run("var s = document.createElement('select'); \
         var o = document.createElement('option'); \
         s.add(o); \
         '' + s.options.length + '/' + (s.options.item(0) === o);");
    assert_eq!(out, "1/true");
}

#[test]
fn select_add_with_before_inserts_at_position() {
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         var o3 = document.createElement('option'); \
         s.add(o1); s.add(o2); s.add(o3, o2); \
         (s.item(0) === o1 && s.item(1) === o3 && s.item(2) === o2) ? 'ok' : 'bad';");
    assert_eq!(out, "ok");
}

#[test]
fn select_add_rejects_non_option_element() {
    let out = run("var s = document.createElement('select'); \
         var d = document.createElement('div'); \
         try { s.add(d); 'no-throw'; } \
         catch (e) { (e.name === 'HierarchyRequestError') ? 'ok' : 'other:' + e.name; }");
    assert_eq!(out, "ok");
}

#[test]
fn select_add_with_fractional_numeric_before_truncates_via_to_int32() {
    // F6 regression — numeric `before` is coerced through WebIDL
    // ToInt32 (HTML §4.10.7.5).  Fractional → trunc-toward-zero,
    // so 1.7 → 1, meaning insert before options[1].
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         var o3 = document.createElement('option'); \
         s.add(o1); s.add(o2); \
         s.add(o3, 1.7); \
         (s.item(0) === o1 && s.item(1) === o3 && s.item(2) === o2) ? 'ok' : 'bad';");
    assert_eq!(out, "ok");
}

#[test]
fn select_add_with_out_of_range_numeric_before_appends() {
    // F6 regression — index >= options.length resolves to no
    // `before` reference and appends.  Pre-fix this used a raw
    // `as i64` cast which behaves consistently for in-range values
    // but loses the ToInt32 wrap semantics for very large numbers.
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         s.add(o1); \
         s.add(o2, 999); \
         (s.item(0) === o1 && s.item(1) === o2) ? 'ok' : 'bad';");
    assert_eq!(out, "ok");
}

#[test]
fn select_add_with_string_before_coerces_via_to_int32() {
    // R4 F1 regression — WebIDL overload resolution: any non-null /
    // non-Element argument flows to ToInt32; string "1" → 1.
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         var o3 = document.createElement('option'); \
         s.add(o1); s.add(o2); \
         s.add(o3, '1'); \
         (s.item(0) === o1 && s.item(1) === o3 && s.item(2) === o2) ? 'ok' : 'bad';");
    assert_eq!(out, "ok");
}

#[test]
fn select_add_with_boolean_before_coerces_via_to_int32() {
    // R4 F1 regression — boolean true → 1, false → 0.
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         var o3 = document.createElement('option'); \
         s.add(o1); s.add(o2); \
         s.add(o3, true); \
         (s.item(0) === o1 && s.item(1) === o3 && s.item(2) === o2) ? 'ok' : 'bad';");
    assert_eq!(out, "ok");
}

#[test]
fn select_add_with_negative_numeric_before_appends() {
    // F6 regression — negative `before` index appends per spec.
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         s.add(o1); \
         s.add(o2, -5); \
         (s.item(0) === o1 && s.item(1) === o2) ? 'ok' : 'bad';");
    assert_eq!(out, "ok");
}

#[test]
fn select_add_before_option_in_optgroup_inserts_into_optgroup() {
    // R2 F1 regression — HTML §4.10.7.5 only requires `before` to
    // be a descendant of the select; `before`'s immediate parent
    // (the optgroup) is the actual insertion target.  Pre-fix this
    // threw NotFoundError because the check insisted on a direct
    // child of the select.
    let out = run("var s = document.createElement('select'); \
         var g = document.createElement('optgroup'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         g.appendChild(o1); \
         s.appendChild(g); \
         s.add(o2, o1); \
         var optgroup_first = g.firstChild; \
         (optgroup_first === o2 && o2.parentNode === g) ? 'ok' : 'bad';");
    assert_eq!(out, "ok");
}

#[test]
fn select_add_before_not_descendant_throws_not_found() {
    // R2 F1 regression — `before` outside the select's subtree
    // must still throw NotFoundError per spec.
    let out = run("var s = document.createElement('select'); \
         var stray = document.createElement('option'); \
         try { s.add(document.createElement('option'), stray); 'no-throw'; } \
         catch (e) { (e.name === 'NotFoundError') ? 'ok' : 'other:' + e.name; }");
    assert_eq!(out, "ok");
}

#[test]
fn select_remove_with_index_drops_option() {
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         s.add(o1); s.add(o2); \
         s.remove(0); \
         '' + s.options.length + '/' + (s.options.item(0) === o2);");
    assert_eq!(out, "1/true");
}

// ---------------------------------------------------------------------------
// selectedIndex / value
// ---------------------------------------------------------------------------

#[test]
fn select_selected_index_default_first_for_size_one() {
    let out = run("var s = document.createElement('select'); \
         s.appendChild(document.createElement('option')); \
         '' + s.selectedIndex;");
    assert_eq!(out, "0");
}

#[test]
fn select_selected_index_minus_one_when_multiple() {
    let out = run("var s = document.createElement('select'); \
         s.multiple = true; \
         s.appendChild(document.createElement('option')); \
         '' + s.selectedIndex;");
    assert_eq!(out, "-1");
}

#[test]
fn select_selected_index_setter_marks_option_selected() {
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         s.add(o1); s.add(o2); \
         s.selectedIndex = 1; \
         '' + o2.hasAttribute('selected') + '/' + s.selectedIndex;");
    assert_eq!(out, "true/1");
}

#[test]
fn select_selected_index_default_skips_options_in_disabled_optgroup() {
    // R7 F3 regression — implicit default selection (size=1) must
    // skip options whose `<optgroup>` ancestor is disabled (HTML
    // §4.10.10.2 — option is disabled if it has `disabled` attr OR
    // sits inside a disabled optgroup).
    let out = run("var s = document.createElement('select'); \
         var g = document.createElement('optgroup'); g.disabled = true; \
         var inGroup = document.createElement('option'); \
         var outside = document.createElement('option'); \
         g.appendChild(inGroup); \
         s.appendChild(g); \
         s.appendChild(outside); \
         '' + s.selectedIndex + '/' + (s.options.item(s.selectedIndex) === outside);");
    assert_eq!(out, "1/true");
}

#[test]
fn select_value_default_skips_options_in_disabled_optgroup() {
    // R7 F4 regression — the implicit-default value getter must
    // also skip optgroup-disabled options.
    let out = run("var s = document.createElement('select'); \
         var g = document.createElement('optgroup'); g.disabled = true; \
         var inGroup = document.createElement('option'); \
         inGroup.value = 'group-val'; \
         var outside = document.createElement('option'); \
         outside.value = 'free-val'; \
         g.appendChild(inGroup); \
         s.appendChild(g); \
         s.appendChild(outside); \
         s.value;");
    assert_eq!(out, "free-val");
}

#[test]
fn select_selected_index_setter_negative_one_deselects() {
    // Per HTML §4.10.7: setting any out-of-range index clears all
    // selectedness; -1 is the canonical "deselect everything" call.
    let out = run("var s = document.createElement('select'); \
         s.setAttribute('multiple', ''); \
         var o1 = document.createElement('option'); \
         o1.setAttribute('selected', ''); \
         s.add(o1); \
         s.selectedIndex = -1; \
         '' + o1.hasAttribute('selected') + '/' + s.selectedIndex;");
    assert_eq!(out, "false/-1");
}

#[test]
fn select_selected_index_setter_out_of_range_clears() {
    // n >= options.length must clear without panicking and leave
    // selectedIndex at -1 (with `multiple` so the size=1 default-first
    // fallback doesn't kick back in).
    let out = run("var s = document.createElement('select'); \
         s.setAttribute('multiple', ''); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         o2.setAttribute('selected', ''); \
         s.add(o1); s.add(o2); \
         s.selectedIndex = 999; \
         '' + o2.hasAttribute('selected') + '/' + s.selectedIndex;");
    assert_eq!(out, "false/-1");
}

#[test]
fn select_selected_index_setter_large_negative_clears() {
    let out = run("var s = document.createElement('select'); \
         s.setAttribute('multiple', ''); \
         var o = document.createElement('option'); \
         o.setAttribute('selected', ''); \
         s.add(o); \
         s.selectedIndex = -2147483648; \
         '' + o.hasAttribute('selected') + '/' + s.selectedIndex;");
    assert_eq!(out, "false/-1");
}

#[test]
fn select_value_returns_selected_option_value() {
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); o1.value = 'a'; \
         var o2 = document.createElement('option'); o2.value = 'b'; \
         o2.setAttribute('selected', ''); \
         s.add(o1); s.add(o2); \
         s.value;");
    assert_eq!(out, "b");
}

#[test]
fn select_value_setter_selects_matching_option() {
    let out = run("var s = document.createElement('select'); \
         var o1 = document.createElement('option'); o1.value = 'a'; \
         var o2 = document.createElement('option'); o2.value = 'b'; \
         s.add(o1); s.add(o2); \
         s.value = 'b'; \
         s.value + '/' + o2.hasAttribute('selected');");
    assert_eq!(out, "b/true");
}

#[test]
fn select_brand_check_throws_on_non_select_receiver() {
    let out = run("var d = document.createElement('div'); \
         var s = document.createElement('select'); \
         var getter = Object.getOwnPropertyDescriptor(\
             Object.getPrototypeOf(s), 'options').get; \
         try { getter.call(d); 'no-throw'; } \
         catch (e) { e instanceof TypeError ? 'type' : 'other'; }");
    assert_eq!(out, "type");
}

// ---------------------------------------------------------------------------
// form.elements / fieldset.elements (FormControls variant)
// ---------------------------------------------------------------------------

#[test]
fn form_elements_includes_listed_descendants() {
    let out = run("var f = document.createElement('form'); \
         var i = document.createElement('input'); \
         var b = document.createElement('button'); \
         var t = document.createElement('textarea'); \
         f.appendChild(i); f.appendChild(b); f.appendChild(t); \
         '' + f.elements.length;");
    assert_eq!(out, "3");
}

#[test]
fn form_elements_excludes_non_listed_descendants() {
    let out = run("var f = document.createElement('form'); \
         var d = document.createElement('div'); \
         var i = document.createElement('input'); \
         d.appendChild(i); \
         f.appendChild(d); \
         '' + f.elements.length;");
    assert_eq!(out, "1");
}

#[test]
fn form_length_mirrors_elements_length() {
    let out = run("var f = document.createElement('form'); \
         f.appendChild(document.createElement('input')); \
         f.appendChild(document.createElement('select')); \
         '' + f.length;");
    assert_eq!(out, "2");
}

#[test]
fn fieldset_elements_includes_listed_descendants() {
    let out = run("var fs = document.createElement('fieldset'); \
         var i = document.createElement('input'); \
         fs.appendChild(i); \
         '' + fs.elements.length;");
    assert_eq!(out, "1");
}
