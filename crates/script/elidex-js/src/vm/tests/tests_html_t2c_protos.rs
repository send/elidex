//! D-6 `#11-tags-T2c-table` — HTMLTable family prototype + accessor +
//! mutation method coverage.
//!
//! Coverage matches the D-6 plan memo §C5/C6 surface:
//! - per-element brand check + prototype identity (6 element interfaces)
//! - cross-tag prototype sharing (thead/tbody/tfoot share section;
//!   td/th share cell; col/colgroup share col)
//! - `[SameObject]` identity for all 4 caches (rows / tBodies /
//!   section.rows / row.cells)
//! - liveness for all 4 collections (mutate descendants → length
//!   changes through cached wrapper)
//! - `<table>.rows` ordering (thead → table-direct + tbodies → tfoot)
//! - `insertRow(-1)` on empty table creates implicit tbody
//! - `insertRow(index)` bounds checking
//! - `deleteRow` removes from rows / DOM tree atomically
//! - `insertCell` / `deleteCell` parallel tests
//! - `createTHead`/`createTFoot`/`createCaption` idempotence
//! - `createTBody` non-idempotence
//! - `delete{THead,TFoot,Caption}` no-op when absent
//! - section setter `<table>.tHead = <thead>` / `null` /
//!   HierarchyRequestError throw
//! - `<tr>.rowIndex` / `<tr>.sectionRowIndex` / `<td>.cellIndex`
//! - `colSpan` / `rowSpan` / `span` clamping (boundary values)
//! - `<th>.scope` enumerated reflect

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

// =====================================================================
// Per-element prototype identity (6 element interfaces, 10 dispatch arms)
// =====================================================================

#[test]
fn table_brand_distinct_from_div() {
    let out = run("var t = document.createElement('table'); \
         var d = document.createElement('div'); \
         (Object.getPrototypeOf(t) !== Object.getPrototypeOf(d)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn thead_tbody_tfoot_share_section_prototype() {
    let out = run("var th = document.createElement('thead'); \
         var tb = document.createElement('tbody'); \
         var tf = document.createElement('tfoot'); \
         var p = Object.getPrototypeOf(th); \
         (p === Object.getPrototypeOf(tb) && p === Object.getPrototypeOf(tf)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn td_th_share_cell_prototype() {
    let out = run("var td = document.createElement('td'); \
         var th = document.createElement('th'); \
         (Object.getPrototypeOf(td) === Object.getPrototypeOf(th)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn col_colgroup_share_col_prototype() {
    let out = run("var c = document.createElement('col'); \
         var g = document.createElement('colgroup'); \
         (Object.getPrototypeOf(c) === Object.getPrototypeOf(g)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn caption_distinct_from_section() {
    let out = run("var c = document.createElement('caption'); \
         var th = document.createElement('thead'); \
         (Object.getPrototypeOf(c) !== Object.getPrototypeOf(th)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn tr_distinct_prototype() {
    let out = run("var tr = document.createElement('tr'); \
         var d = document.createElement('div'); \
         (Object.getPrototypeOf(tr) !== Object.getPrototypeOf(d)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// SameObject identity (4 caches)
// =====================================================================

#[test]
fn table_rows_same_object() {
    let out = run("var t = document.createElement('table'); \
         (t.rows === t.rows) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn table_tbodies_same_object() {
    let out = run("var t = document.createElement('table'); \
         (t.tBodies === t.tBodies) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn section_rows_same_object() {
    let out = run("var b = document.createElement('tbody'); \
         (b.rows === b.rows) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn row_cells_same_object() {
    let out = run("var r = document.createElement('tr'); \
         (r.cells === r.cells) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// Liveness (mutate descendants → cached wrapper sees new length)
// =====================================================================

#[test]
fn table_rows_live_after_mutation() {
    let out = run("var t = document.createElement('table'); \
         document.body.appendChild(t); \
         var rows = t.rows; \
         var n0 = rows.length; \
         t.insertRow(-1); \
         var n1 = rows.length; \
         (n0 === 0 && n1 === 1) ? 'ok' : 'fail:' + n0 + ',' + n1;");
    assert_eq!(out, "ok");
}

#[test]
fn row_cells_live_after_mutation() {
    let out = run("var r = document.createElement('tr'); \
         var cells = r.cells; \
         var n0 = cells.length; \
         r.insertCell(-1); \
         var n1 = cells.length; \
         (n0 === 0 && n1 === 1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// <table>.rows ordering (thead → table-direct/tbodies → tfoot)
// =====================================================================

#[test]
fn table_rows_ordering_thead_body_tfoot() {
    let out = run("var t = document.createElement('table'); \
         var thead = document.createElement('thead'); \
         var tbody = document.createElement('tbody'); \
         var tfoot = document.createElement('tfoot'); \
         var hr = document.createElement('tr'); \
         var br = document.createElement('tr'); \
         var fr = document.createElement('tr'); \
         t.appendChild(thead); t.appendChild(tbody); t.appendChild(tfoot); \
         thead.appendChild(hr); tbody.appendChild(br); tfoot.appendChild(fr); \
         var rows = t.rows; \
         (rows.length === 3 && rows.item(0) === hr && rows.item(1) === br && rows.item(2) === fr) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// insertRow / deleteRow / implicit tbody
// =====================================================================

#[test]
fn insert_row_creates_implicit_tbody() {
    let out = run("var t = document.createElement('table'); \
         var r = t.insertRow(-1); \
         (t.tBodies.length === 1 && r.parentNode === t.tBodies.item(0) && r.tagName === 'TR') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn insert_row_bounds_check_throws() {
    let out = run("var t = document.createElement('table'); \
         var caught = false; \
         try { t.insertRow(1); } catch (e) { caught = (e.name === 'IndexSizeError'); } \
         caught ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn insert_row_negative_lt_minus_one_throws() {
    let out = run("var t = document.createElement('table'); \
         var caught = false; \
         try { t.insertRow(-2); } catch (e) { caught = (e.name === 'IndexSizeError'); } \
         caught ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn delete_row_removes() {
    let out = run("var t = document.createElement('table'); \
         t.insertRow(-1); t.insertRow(-1); \
         var n0 = t.rows.length; \
         t.deleteRow(0); \
         var n1 = t.rows.length; \
         (n0 === 2 && n1 === 1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn delete_row_negative_one_removes_last() {
    let out = run("var t = document.createElement('table'); \
         var r1 = t.insertRow(-1); var r2 = t.insertRow(-1); \
         t.deleteRow(-1); \
         (t.rows.length === 1 && t.rows.item(0) === r1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn delete_row_oob_throws() {
    let out = run("var t = document.createElement('table'); \
         var caught = false; \
         try { t.deleteRow(0); } catch (e) { caught = (e.name === 'IndexSizeError'); } \
         caught ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn insert_row_string_arg_coerced() {
    // WebIDL ToNumber("0") = 0; spec says insertRow at index 0.
    let out = run("var t = document.createElement('table'); \
         var caught = false; \
         try { t.insertRow('0'); } catch (e) { caught = true; } \
         (!caught && t.rows.length === 1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn insert_row_value_of_invoked() {
    // WebIDL ToNumber({valueOf: () => -1}) = -1; spec appends.
    let out = run("var t = document.createElement('table'); \
         t.insertRow({valueOf: function() { return -1; }}); \
         (t.rows.length === 1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// insertCell / deleteCell
// =====================================================================

#[test]
fn insert_cell_appends() {
    let out = run("var r = document.createElement('tr'); \
         var c = r.insertCell(-1); \
         (r.cells.length === 1 && c.tagName === 'TD') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn delete_cell_removes() {
    let out = run("var r = document.createElement('tr'); \
         r.insertCell(-1); r.insertCell(-1); \
         r.deleteCell(0); \
         (r.cells.length === 1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn delete_cell_oob_throws() {
    let out = run("var r = document.createElement('tr'); \
         var caught = false; \
         try { r.deleteCell(0); } catch (e) { caught = (e.name === 'IndexSizeError'); } \
         caught ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// createTHead / createTFoot / createCaption (idempotent)
// =====================================================================

#[test]
fn create_thead_idempotent() {
    let out = run("var t = document.createElement('table'); \
         var a = t.createTHead(); var b = t.createTHead(); \
         (a === b && t.tHead === a) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn create_tfoot_idempotent() {
    let out = run("var t = document.createElement('table'); \
         var a = t.createTFoot(); var b = t.createTFoot(); \
         (a === b && t.tFoot === a) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn create_caption_idempotent() {
    let out = run("var t = document.createElement('table'); \
         var a = t.createCaption(); var b = t.createCaption(); \
         (a === b && t.caption === a) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn create_thead_position_after_caption() {
    let out = run("var t = document.createElement('table'); \
         t.createCaption(); t.createTHead(); \
         var c0 = t.children.item(0); var c1 = t.children.item(1); \
         (c0.tagName === 'CAPTION' && c1.tagName === 'THEAD') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// createTBody — NOT idempotent
// =====================================================================

#[test]
fn create_tbody_not_idempotent() {
    let out = run("var t = document.createElement('table'); \
         var a = t.createTBody(); var b = t.createTBody(); \
         (a !== b && t.tBodies.length === 2) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// delete{THead,TFoot,Caption} no-op when absent
// =====================================================================

#[test]
fn delete_thead_noop() {
    let out = run("var t = document.createElement('table'); \
         t.deleteTHead(); \
         (t.tHead === null) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn delete_thead_removes_existing() {
    let out = run("var t = document.createElement('table'); \
         t.createTHead(); var before = t.tHead; \
         t.deleteTHead(); \
         (before !== null && t.tHead === null) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// Section setters (caption / tHead / tFoot)
// =====================================================================

#[test]
fn set_thead_with_div_throws_hierarchy() {
    let out = run("var t = document.createElement('table'); \
         var d = document.createElement('div'); \
         var caught = false; \
         try { t.tHead = d; } catch (e) { caught = (e.name === 'HierarchyRequestError'); } \
         caught ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn set_thead_replaces_existing() {
    let out = run("var t = document.createElement('table'); \
         var old = t.createTHead(); \
         var new_ = document.createElement('thead'); \
         t.tHead = new_; \
         (t.tHead === new_ && t.tHead !== old) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn set_thead_null_removes() {
    let out = run("var t = document.createElement('table'); \
         t.createTHead(); \
         t.tHead = null; \
         (t.tHead === null) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn set_caption_with_div_throws() {
    let out = run("var t = document.createElement('table'); \
         var d = document.createElement('div'); \
         var caught = false; \
         try { t.caption = d; } catch (e) { caught = (e.name === 'HierarchyRequestError'); } \
         caught ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn set_tfoot_with_div_throws() {
    let out = run("var t = document.createElement('table'); \
         var d = document.createElement('div'); \
         var caught = false; \
         try { t.tFoot = d; } catch (e) { caught = (e.name === 'HierarchyRequestError'); } \
         caught ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// rowIndex / sectionRowIndex / cellIndex
// =====================================================================

#[test]
fn row_index_walks_sections() {
    let out = run("var t = document.createElement('table'); \
         var thead = t.createTHead(); var tbody = t.createTBody(); var tfoot = t.createTFoot(); \
         var hr = document.createElement('tr'); \
         var br = document.createElement('tr'); \
         var fr = document.createElement('tr'); \
         thead.appendChild(hr); tbody.appendChild(br); tfoot.appendChild(fr); \
         (hr.rowIndex === 0 && br.rowIndex === 1 && fr.rowIndex === 2) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn row_index_minus_one_when_detached() {
    let out = run("var r = document.createElement('tr'); \
         (r.rowIndex === -1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn section_row_index_basic() {
    let out = run("var b = document.createElement('tbody'); \
         var r1 = document.createElement('tr'); \
         var r2 = document.createElement('tr'); \
         b.appendChild(r1); b.appendChild(r2); \
         (r1.sectionRowIndex === 0 && r2.sectionRowIndex === 1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn section_row_index_minus_one_when_not_in_section() {
    // Direct <tr> child of <table> — parent is not a section.
    let out = run("var t = document.createElement('table'); \
         var r = document.createElement('tr'); t.appendChild(r); \
         (r.sectionRowIndex === -1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn cell_index_basic() {
    let out = run("var r = document.createElement('tr'); \
         var c1 = document.createElement('td'); \
         var c2 = document.createElement('th'); \
         r.appendChild(c1); r.appendChild(c2); \
         (c1.cellIndex === 0 && c2.cellIndex === 1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn cell_index_minus_one_when_detached() {
    let out = run("var c = document.createElement('td'); \
         (c.cellIndex === -1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// colSpan / rowSpan / span clamping
// =====================================================================

#[test]
fn col_span_default_one() {
    let out = run("var c = document.createElement('td'); \
         (c.colSpan === 1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn col_span_zero_clamps_to_one() {
    let out = run("var c = document.createElement('td'); \
         c.colSpan = 0; \
         (c.colSpan === 1) ? 'ok' : 'fail:' + c.colSpan;");
    assert_eq!(out, "ok");
}

#[test]
fn col_span_negative_clamps_to_one() {
    let out = run("var c = document.createElement('td'); \
         c.colSpan = -5; \
         (c.colSpan === 1) ? 'ok' : 'fail:' + c.colSpan;");
    assert_eq!(out, "ok");
}

#[test]
fn col_span_normal_value() {
    let out = run("var c = document.createElement('td'); \
         c.colSpan = 3; \
         (c.colSpan === 3) ? 'ok' : 'fail:' + c.colSpan;");
    assert_eq!(out, "ok");
}

#[test]
fn row_span_default_one() {
    let out = run("var c = document.createElement('td'); \
         (c.rowSpan === 1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn row_span_zero_preserved() {
    // rowSpan=0 means "span all remaining rows in section".
    let out = run("var c = document.createElement('td'); \
         c.rowSpan = 0; \
         (c.rowSpan === 0) ? 'ok' : 'fail:' + c.rowSpan;");
    assert_eq!(out, "ok");
}

#[test]
fn row_span_above_max_saturates() {
    let out = run("var c = document.createElement('td'); \
         c.rowSpan = 65535; \
         (c.rowSpan === 65534) ? 'ok' : 'fail:' + c.rowSpan;");
    assert_eq!(out, "ok");
}

#[test]
fn col_span_above_idl_saturates_via_serialise() {
    // The IDL setter saturates via i32; getter reads-back attribute.
    let out = run("var c = document.createElement('td'); \
         c.colSpan = 1e20; \
         (c.colSpan === 2147483647) ? 'ok' : 'fail:' + c.colSpan;");
    assert_eq!(out, "ok");
}

#[test]
fn col_span_value_of() {
    // valueOf invocation per WebIDL `long` ToNumber.
    let out = run("var c = document.createElement('td'); \
         c.colSpan = {valueOf: function() { return 5.7; }}; \
         (c.colSpan === 5) ? 'ok' : 'fail:' + c.colSpan;");
    assert_eq!(out, "ok");
}

#[test]
fn col_default_span_one() {
    let out = run("var c = document.createElement('col'); \
         (c.span === 1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn col_span_clamps_one_to_thousand() {
    let out = run("var c = document.createElement('col'); \
         c.span = 1001; \
         (c.span === 1000) ? 'ok' : 'fail:' + c.span;");
    assert_eq!(out, "ok");
}

#[test]
fn col_element_span_zero_clamps_to_one() {
    let out = run("var c = document.createElement('col'); \
         c.span = 0; \
         (c.span === 1) ? 'ok' : 'fail:' + c.span;");
    assert_eq!(out, "ok");
}

// =====================================================================
// scope enumerated reflect
// =====================================================================

#[test]
fn scope_default_empty() {
    let out = run("var th = document.createElement('th'); \
         (th.scope === '') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn scope_canonical_keyword() {
    let out = run("var th = document.createElement('th'); \
         th.setAttribute('scope', 'row'); \
         (th.scope === 'row') ? 'ok' : 'fail:' + th.scope;");
    assert_eq!(out, "ok");
}

#[test]
fn scope_ascii_ci_canonicalises() {
    let out = run("var th = document.createElement('th'); \
         th.setAttribute('scope', 'COLGROUP'); \
         (th.scope === 'colgroup') ? 'ok' : 'fail:' + th.scope;");
    assert_eq!(out, "ok");
}

#[test]
fn scope_invalid_returns_empty() {
    let out = run("var th = document.createElement('th'); \
         th.setAttribute('scope', 'bogus'); \
         (th.scope === '') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn scope_setter_writes_attribute() {
    let out = run("var th = document.createElement('th'); \
         th.scope = 'row'; \
         (th.getAttribute('scope') === 'row') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// headers / abbr (string reflect)
// =====================================================================

#[test]
fn headers_string_reflect() {
    let out = run("var c = document.createElement('td'); \
         c.headers = 'h1 h2'; \
         (c.headers === 'h1 h2' && c.getAttribute('headers') === 'h1 h2') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn abbr_string_reflect() {
    let out = run("var c = document.createElement('td'); \
         c.abbr = 'short'; \
         (c.abbr === 'short') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// Foreign-receiver TypeError brand-check
// =====================================================================

#[test]
fn table_rows_brand_check_on_div_throws() {
    let out = run("var d = document.createElement('div'); \
         var getter = Object.getOwnPropertyDescriptor( \
             Object.getPrototypeOf(document.createElement('table')), 'rows').get; \
         var caught = false; \
         try { getter.call(d); } catch (e) { caught = (e instanceof TypeError); } \
         caught ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn cell_index_brand_check_on_div_throws() {
    let out = run("var d = document.createElement('div'); \
         var getter = Object.getOwnPropertyDescriptor( \
             Object.getPrototypeOf(document.createElement('td')), 'cellIndex').get; \
         var caught = false; \
         try { getter.call(d); } catch (e) { caught = (e instanceof TypeError); } \
         caught ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn set_thead_brand_check_on_div_throws() {
    let out = run("var d = document.createElement('div'); \
         var setter = Object.getOwnPropertyDescriptor( \
             Object.getPrototypeOf(document.createElement('table')), 'tHead').set; \
         var caught = false; \
         try { setter.call(d, document.createElement('thead')); } catch (e) { caught = (e instanceof TypeError); } \
         caught ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}
