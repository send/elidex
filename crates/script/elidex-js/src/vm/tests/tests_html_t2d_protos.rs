//! D-7 `#11-tags-T2d-interactive` — HTML interactive bundle prototype +
//! accessor + method coverage.
//!
//! Coverage matches the D-7 plan memo §C2-C6 surface:
//! - per-element brand check + prototype identity (7 element interfaces, 7
//!   dispatch arms — no shared prototypes)
//! - `<dialog>` open / returnValue / show / showModal / close round-trip
//!   + `close` event firing + InvalidStateError on double-modal
//! - `<details>` open / name reflect (NO ToggleEvent fire)
//! - `<template>.content` `[SameObject]` DocumentFragment + lazy alloc
//! - `<datalist>.options` `[SameObject]` HTMLCollection + descendant filter
//! - `<output>.htmlFor` `[SameObject]` DOMTokenList + add/contains +
//!   PutForwards-via-string-set
//! - `<output>.value` / `defaultValue` state machine (default → value
//!   mode switch + form-reset round-trip)
//! - `<output>.form` form-owner accessor
//! - `<output>` ConstraintValidation mixin smoke test
//! - `<progress>.value` / `max` / `position` boundary values
//! - `<meter>.value` / `min` / `max` / `low` / `high` / `optimum` clamping
//! - foreign-receiver TypeError brand check
//! - prototype-absence tests (defer-proof for ToggleEvent + dialog event
//!   handlers, walking the full prototype chain to Object.prototype per
//!   T2b lesson #204)

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
// Per-element prototype identity (7 distinct prototypes)
// =====================================================================

#[test]
fn dialog_brand_distinct_from_div() {
    let out = run("var d = document.createElement('dialog'); \
         var v = document.createElement('div'); \
         (Object.getPrototypeOf(d) !== Object.getPrototypeOf(v)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn details_brand_distinct() {
    let out = run("var d = document.createElement('details'); \
         var v = document.createElement('div'); \
         (Object.getPrototypeOf(d) !== Object.getPrototypeOf(v)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn template_brand_distinct() {
    let out = run("var t = document.createElement('template'); \
         var v = document.createElement('div'); \
         (Object.getPrototypeOf(t) !== Object.getPrototypeOf(v)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn datalist_brand_distinct() {
    let out = run("var d = document.createElement('datalist'); \
         var v = document.createElement('div'); \
         (Object.getPrototypeOf(d) !== Object.getPrototypeOf(v)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn output_brand_distinct() {
    let out = run("var o = document.createElement('output'); \
         var v = document.createElement('div'); \
         (Object.getPrototypeOf(o) !== Object.getPrototypeOf(v)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn progress_brand_distinct() {
    let out = run("var p = document.createElement('progress'); \
         var v = document.createElement('div'); \
         (Object.getPrototypeOf(p) !== Object.getPrototypeOf(v)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn meter_brand_distinct() {
    let out = run("var m = document.createElement('meter'); \
         var v = document.createElement('div'); \
         (Object.getPrototypeOf(m) !== Object.getPrototypeOf(v)) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn t2d_prototypes_all_distinct() {
    // Each of the 7 T2d prototypes must be a unique identity
    // (no shared prototypes — every tag has its own).  Plain
    // pairwise-distinct check rather than `new Set(...)` (the
    // custom VM does not ship `Set`).
    let out = run(
        "var tags = ['dialog','details','template','datalist','output','progress','meter']; \
         var protos = tags.map(function(t) { return Object.getPrototypeOf(document.createElement(t)); }); \
         var ok = true; \
         for (var i = 0; i < protos.length; i++) { \
             for (var j = i + 1; j < protos.length; j++) { \
                 if (protos[i] === protos[j]) { ok = false; } \
             } \
         } \
         ok ? 'ok' : 'fail';",
    );
    assert_eq!(out, "ok");
}

// =====================================================================
// <dialog> — open / returnValue / show / showModal / close
// =====================================================================

#[test]
fn dialog_open_default_false() {
    let out = run("var d = document.createElement('dialog'); \
         (d.open === false) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn dialog_open_setter_toggles_attribute() {
    let out = run("var d = document.createElement('dialog'); \
         d.open = true; \
         var a = d.hasAttribute('open'); \
         d.open = false; \
         var b = d.hasAttribute('open'); \
         (a === true && b === false) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn dialog_show_sets_open_attribute() {
    let out = run("var d = document.createElement('dialog'); \
         d.show(); \
         (d.open === true && d.hasAttribute('open')) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn dialog_show_modal_sets_open() {
    let out = run("var d = document.createElement('dialog'); \
         d.showModal(); \
         (d.open === true) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn dialog_show_modal_then_show_throws_invalid_state() {
    let out = run("var d = document.createElement('dialog'); \
         d.showModal(); \
         var caught = false; \
         try { d.show(); } catch (e) { caught = (e.name === 'InvalidStateError'); } \
         caught ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn dialog_show_modal_when_already_open_throws() {
    let out = run("var d = document.createElement('dialog'); \
         d.show(); \
         var caught = false; \
         try { d.showModal(); } catch (e) { caught = (e.name === 'InvalidStateError'); } \
         caught ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn dialog_close_clears_open() {
    let out = run("var d = document.createElement('dialog'); \
         d.show(); \
         d.close(); \
         (d.open === false && !d.hasAttribute('open')) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn dialog_close_with_arg_sets_return_value() {
    let out = run("var d = document.createElement('dialog'); \
         d.show(); \
         d.close('confirmed'); \
         (d.returnValue === 'confirmed') ? 'ok' : 'fail:' + d.returnValue;");
    assert_eq!(out, "ok");
}

#[test]
fn dialog_close_without_arg_keeps_return_value() {
    let out = run("var d = document.createElement('dialog'); \
         d.returnValue = 'preset'; \
         d.show(); \
         d.close(); \
         (d.returnValue === 'preset') ? 'ok' : 'fail:' + d.returnValue;");
    assert_eq!(out, "ok");
}

#[test]
fn dialog_return_value_default_empty() {
    let out = run("var d = document.createElement('dialog'); \
         (d.returnValue === '') ? 'ok' : 'fail:' + d.returnValue;");
    assert_eq!(out, "ok");
}

#[test]
fn dialog_close_fires_close_event() {
    let out = run("var d = document.createElement('dialog'); \
         document.body.appendChild(d); \
         var fired = false; \
         d.addEventListener('close', function() { fired = true; }); \
         d.show(); \
         d.close(); \
         fired ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn dialog_close_event_does_not_bubble() {
    let out = run("var d = document.createElement('dialog'); \
         document.body.appendChild(d); \
         var bubbled = false; \
         document.body.addEventListener('close', function() { bubbled = true; }); \
         d.show(); \
         d.close(); \
         bubbled ? 'fail' : 'ok';");
    assert_eq!(out, "ok");
}

#[test]
fn dialog_close_when_closed_no_event() {
    let out = run("var d = document.createElement('dialog'); \
         var fired = false; \
         d.addEventListener('close', function() { fired = true; }); \
         d.close(); \
         fired ? 'fail' : 'ok';");
    assert_eq!(out, "ok");
}

#[test]
fn dialog_no_event_handler_idl_attrs() {
    // Defer-proof: oncancel / onclose are NOT installed on
    // HTMLDialogElement.prototype (deferred to D-10).  Walk full chain
    // to Object.prototype and assert absence (T2b lesson #204).
    let out = run("var d = document.createElement('dialog'); \
         function inChain(obj, name) { \
             while (obj) { \
                 if (Object.getOwnPropertyDescriptor(obj, name)) return true; \
                 obj = Object.getPrototypeOf(obj); \
             } \
             return false; \
         } \
         var cancel = inChain(d, 'oncancel'); \
         var close = inChain(d, 'onclose'); \
         (!cancel && !close) ? 'ok' : 'fail:cancel=' + cancel + ',close=' + close;");
    assert_eq!(out, "ok");
}

// =====================================================================
// <details> — open / name reflect; defer-proof for ToggleEvent
// =====================================================================

#[test]
fn details_open_default_false() {
    let out = run("var d = document.createElement('details'); \
         (d.open === false) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn details_open_setter_toggles_attribute() {
    let out = run("var d = document.createElement('details'); \
         d.open = true; var a = d.hasAttribute('open'); \
         d.open = false; var b = d.hasAttribute('open'); \
         (a === true && b === false) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn details_name_default_empty() {
    let out = run("var d = document.createElement('details'); \
         (d.name === '') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn details_name_reflect() {
    let out = run("var d = document.createElement('details'); \
         d.name = 'group1'; \
         (d.name === 'group1' && d.getAttribute('name') === 'group1') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn details_no_event_handler_idl_attrs() {
    // Defer-proof (slot `#11-tags-T2d-details-toggle-event`):
    // oncancel / onclose / ontoggle / onbeforetoggle MUST NOT be
    // installed yet.  Walk full chain to Object.prototype per
    // T2b lesson #204.
    let out = run("var d = document.createElement('details'); \
         function inChain(obj, name) { \
             while (obj) { \
                 if (Object.getOwnPropertyDescriptor(obj, name)) return true; \
                 obj = Object.getPrototypeOf(obj); \
             } \
             return false; \
         } \
         var t = inChain(d, 'ontoggle'); \
         var bt = inChain(d, 'onbeforetoggle'); \
         (!t && !bt) ? 'ok' : 'fail:toggle=' + t + ',beforetoggle=' + bt;");
    assert_eq!(out, "ok");
}

#[test]
fn details_open_setter_does_not_fire_toggle_event() {
    // Defer-proof: setting `open` on `<details>` must NOT fire a
    // ToggleEvent — that fire is paired with D-10.
    let out = run("var d = document.createElement('details'); \
         var fired = false; \
         d.addEventListener('toggle', function() { fired = true; }); \
         d.open = true; \
         fired ? 'fail' : 'ok';");
    assert_eq!(out, "ok");
}

// =====================================================================
// <template>.content — SameObject + lazy alloc + DocumentFragment
// =====================================================================

#[test]
fn template_content_same_object() {
    let out = run("var t = document.createElement('template'); \
         (t.content === t.content) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn template_content_is_document_fragment() {
    let out = run("var t = document.createElement('template'); \
         (t.content.nodeType === 11) ? 'ok' : 'fail:' + t.content.nodeType;");
    assert_eq!(out, "ok");
}

#[test]
fn template_content_starts_empty() {
    let out = run("var t = document.createElement('template'); \
         (t.content.firstChild === null && t.content.childNodes.length === 0) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn template_content_holds_appended_children() {
    let out = run("var t = document.createElement('template'); \
         var p = document.createElement('p'); \
         t.content.appendChild(p); \
         (t.content.childNodes.length === 1 && t.content.firstChild === p) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn template_content_isolated_from_template_element() {
    // Children appended to `<template>.content` do NOT show up as
    // children of the `<template>` element itself.
    let out = run("var t = document.createElement('template'); \
         var p = document.createElement('p'); \
         t.content.appendChild(p); \
         (t.childNodes.length === 0) ? 'ok' : 'fail:' + t.childNodes.length;");
    assert_eq!(out, "ok");
}

// =====================================================================
// <datalist>.options — SameObject + descendant <option> filtering
// =====================================================================

#[test]
fn datalist_options_same_object() {
    let out = run("var d = document.createElement('datalist'); \
         (d.options === d.options) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn datalist_options_initially_empty() {
    let out = run("var d = document.createElement('datalist'); \
         (d.options.length === 0) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn datalist_options_filters_descendants() {
    let out = run("var d = document.createElement('datalist'); \
         var o1 = document.createElement('option'); \
         var o2 = document.createElement('option'); \
         var div = document.createElement('div'); \
         d.appendChild(o1); d.appendChild(div); d.appendChild(o2); \
         (d.options.length === 2 && d.options.item(0) === o1 && d.options.item(1) === o2) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn datalist_options_live_after_mutation() {
    let out = run("var d = document.createElement('datalist'); \
         document.body.appendChild(d); \
         var opts = d.options; \
         var n0 = opts.length; \
         d.appendChild(document.createElement('option')); \
         var n1 = opts.length; \
         (n0 === 0 && n1 === 1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// <output>.htmlFor — SameObject + DOMTokenList add/contains +
// PutForwards-via-string-set
// =====================================================================

#[test]
fn output_html_for_same_object() {
    let out = run("var o = document.createElement('output'); \
         (o.htmlFor === o.htmlFor) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn output_html_for_initial_empty() {
    let out = run("var o = document.createElement('output'); \
         (o.htmlFor.length === 0 && o.htmlFor.value === '') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn output_html_for_add_remove_contains() {
    let out = run("var o = document.createElement('output'); \
         o.htmlFor.add('input1'); \
         o.htmlFor.add('input2'); \
         var c1 = o.htmlFor.contains('input1'); \
         o.htmlFor.remove('input1'); \
         var c2 = o.htmlFor.contains('input1'); \
         var c3 = o.htmlFor.contains('input2'); \
         (c1 && !c2 && c3 && o.getAttribute('for') === 'input2') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn output_html_for_put_forwards_value_set() {
    // PutForwards=value: assigning to o.htmlFor delegates to
    // o.htmlFor.value.set, which writes the `for` attribute.
    let out = run("var o = document.createElement('output'); \
         o.htmlFor = 'a b c'; \
         (o.getAttribute('for') === 'a b c' && o.htmlFor.length === 3) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// <output> — type / name / form
// =====================================================================

#[test]
fn output_type_constant() {
    let out = run("var o = document.createElement('output'); \
         (o.type === 'output') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn output_name_reflect() {
    let out = run("var o = document.createElement('output'); \
         o.name = 'result'; \
         (o.name === 'result' && o.getAttribute('name') === 'result') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn output_form_owner() {
    let out = run("var f = document.createElement('form'); \
         var o = document.createElement('output'); \
         f.appendChild(o); \
         (o.form === f) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn output_form_null_when_unowned() {
    let out = run("var o = document.createElement('output'); \
         (o.form === null) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// <output>.value / defaultValue state machine
// =====================================================================

#[test]
fn output_value_default_empty() {
    let out = run("var o = document.createElement('output'); \
         (o.value === '' && o.defaultValue === '') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn output_default_value_setter_updates_text_in_default_mode() {
    let out = run("var o = document.createElement('output'); \
         o.defaultValue = 'hello'; \
         (o.defaultValue === 'hello' && o.value === 'hello' && o.textContent === 'hello') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn output_value_setter_switches_to_value_mode() {
    let out = run("var o = document.createElement('output'); \
         o.defaultValue = 'def'; \
         o.value = 'override'; \
         (o.value === 'override' && o.defaultValue === 'def' && o.textContent === 'override') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn output_default_value_setter_in_value_mode_does_not_touch_display() {
    let out = run("var o = document.createElement('output'); \
         o.value = 'shown'; \
         o.defaultValue = 'newdef'; \
         (o.value === 'shown' && o.defaultValue === 'newdef' && o.textContent === 'shown') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn output_value_setter_snapshots_initial_text_into_default() {
    // Self-review IMP-1: switching from default → value mode must
    // freeze the implicit default (descendant text content) into
    // `OutputDefaultValue`, otherwise the spec-mandated form-reset
    // round-trip loses the original default.
    let out = run("var o = document.createElement('output'); \
         o.appendChild(document.createTextNode('initial')); \
         o.value = 'override'; \
         (o.defaultValue === 'initial' && o.value === 'override') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn output_form_reset_preserves_pristine_default_text() {
    // Copilot R1 IMP regression test: an `<output>` that has never
    // entered value mode (no `OutputValueOverride`) keeps its
    // descendant text content on form reset — the children represent
    // the implicit default per HTML §4.10.13, not a stale value-mode
    // display that needs wiping.
    let out = run("var f = document.createElement('form'); \
         var o = document.createElement('output'); \
         o.appendChild(document.createTextNode('initial')); \
         f.appendChild(o); document.body.appendChild(f); \
         f.reset(); \
         (o.textContent === 'initial' && o.value === 'initial' && o.defaultValue === 'initial') ? 'ok' : 'fail:text=' + o.textContent + ',val=' + o.value + ',def=' + o.defaultValue;");
    assert_eq!(out, "ok");
}

#[test]
fn output_form_reset_reverts_to_default_mode() {
    let out = run("var f = document.createElement('form'); \
         var o = document.createElement('output'); \
         f.appendChild(o); document.body.appendChild(f); \
         o.defaultValue = 'def'; \
         o.value = 'override'; \
         f.reset(); \
         (o.value === 'def' && o.defaultValue === 'def' && o.textContent === 'def') ? 'ok' : 'fail:value=' + o.value + ',text=' + o.textContent;");
    assert_eq!(out, "ok");
}

// =====================================================================
// <output> labels stub + ConstraintValidation mixin smoke
// =====================================================================

#[test]
fn output_labels_empty_nodelist() {
    let out = run("var o = document.createElement('output'); \
         (o.labels.length === 0) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn output_will_validate_smoke() {
    // ConstraintValidation mixin installed: willValidate accessor
    // resolves (boolean output, exact value depends on candidate
    // policy — just confirm the accessor exists and returns a
    // boolean).
    let out = run("var o = document.createElement('output'); \
         (typeof o.willValidate === 'boolean') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn output_check_validity_smoke() {
    let out = run("var o = document.createElement('output'); \
         (typeof o.checkValidity === 'function') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn output_set_custom_validity_smoke() {
    let out = run("var o = document.createElement('output'); \
         o.setCustomValidity('error'); \
         (typeof o.validationMessage === 'string') ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// <progress>.value / max / position
// =====================================================================

#[test]
fn progress_default_value_zero() {
    let out = run("var p = document.createElement('progress'); \
         (p.value === 0) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn progress_default_max_one() {
    let out = run("var p = document.createElement('progress'); \
         (p.max === 1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn progress_position_indeterminate_when_no_value() {
    let out = run("var p = document.createElement('progress'); \
         (p.position === -1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn progress_position_when_value_set() {
    let out = run("var p = document.createElement('progress'); \
         p.value = 0.25; \
         p.max = 1; \
         (p.position === 0.25) ? 'ok' : 'fail:' + p.position;");
    assert_eq!(out, "ok");
}

#[test]
fn progress_value_clamps_to_max() {
    let out = run("var p = document.createElement('progress'); \
         p.max = 10; \
         p.value = 50; \
         (p.value === 10) ? 'ok' : 'fail:' + p.value;");
    assert_eq!(out, "ok");
}

#[test]
fn progress_value_clamps_negative_to_zero() {
    let out = run("var p = document.createElement('progress'); \
         p.value = -5; \
         (p.value === 0) ? 'ok' : 'fail:' + p.value;");
    assert_eq!(out, "ok");
}

#[test]
fn progress_max_zero_collapses_to_one() {
    let out = run("var p = document.createElement('progress'); \
         p.max = 0; \
         (p.max === 1) ? 'ok' : 'fail:' + p.max;");
    assert_eq!(out, "ok");
}

#[test]
fn progress_max_negative_collapses_to_one() {
    let out = run("var p = document.createElement('progress'); \
         p.max = -3; \
         (p.max === 1) ? 'ok' : 'fail:' + p.max;");
    assert_eq!(out, "ok");
}

#[test]
fn progress_value_setter_serialises_negative_zero_as_zero() {
    // Copilot R2 IMP regression test: ES Number::ToString (ES2020
    // §7.1.12) emits "0" for -0; Rust's Display emits "-0".  Setter
    // must route through the VM's `coerce::to_string` so the reflected
    // content attribute matches browser semantics.
    let out = run("var p = document.createElement('progress'); \
         p.value = -0; \
         (p.getAttribute('value') === '0') ? 'ok' : 'fail:' + p.getAttribute('value');");
    assert_eq!(out, "ok");
}

#[test]
fn meter_value_setter_serialises_negative_zero_as_zero() {
    let out = run("var m = document.createElement('meter'); \
         m.min = -1; m.max = 1; m.value = -0; \
         (m.getAttribute('value') === '0') ? 'ok' : 'fail:' + m.getAttribute('value');");
    assert_eq!(out, "ok");
}

#[test]
fn progress_value_setter_rejects_non_finite() {
    // WebIDL §3.10.5: restricted `double` setter throws TypeError
    // on NaN / ±Infinity.  HTML §4.10.14 declares `<progress>.value`
    // as plain `double` (not `unrestricted double`).
    let out = run("var p = document.createElement('progress'); \
         try { p.value = NaN; 'no-throw'; } catch (e) { 'TypeError'; }");
    assert_eq!(out, "TypeError");
    let out = run("var p = document.createElement('progress'); \
         try { p.value = Infinity; 'no-throw'; } catch (e) { 'TypeError'; }");
    assert_eq!(out, "TypeError");
}

#[test]
fn progress_value_of_invoked() {
    // WebIDL ToNumber on object args.
    let out = run("var p = document.createElement('progress'); \
         p.value = {valueOf: function() { return 0.5; }}; \
         (p.value === 0.5) ? 'ok' : 'fail:' + p.value;");
    assert_eq!(out, "ok");
}

#[test]
fn progress_labels_empty() {
    let out = run("var p = document.createElement('progress'); \
         (p.labels.length === 0) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// <meter>.value / min / max / low / high / optimum
// =====================================================================

#[test]
fn meter_defaults() {
    let out = run("var m = document.createElement('meter'); \
         (m.value === 0 && m.min === 0 && m.max === 1) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn meter_value_clamps_to_max() {
    let out = run("var m = document.createElement('meter'); \
         m.max = 10; \
         m.value = 100; \
         (m.value === 10) ? 'ok' : 'fail:' + m.value;");
    assert_eq!(out, "ok");
}

#[test]
fn meter_value_clamps_to_min() {
    let out = run("var m = document.createElement('meter'); \
         m.min = 5; \
         m.value = 0; \
         (m.value === 5) ? 'ok' : 'fail:' + m.value;");
    assert_eq!(out, "ok");
}

#[test]
fn meter_value_default_is_zero_clamped() {
    // Self-review IMP-3: HTML §4.10.15 says missing `value` defaults
    // to 0 then clamped to `[min, max]` — distinct from defaulting
    // straight to `min` when `min < 0`.
    let out = run("var m = document.createElement('meter'); \
         m.min = -10; m.max = 10; \
         (m.value === 0) ? 'ok' : 'fail:' + m.value;");
    assert_eq!(out, "ok");
}

#[test]
fn meter_max_below_min_promotes_to_min() {
    let out = run("var m = document.createElement('meter'); \
         m.min = 10; \
         m.max = 5; \
         (m.max === 10) ? 'ok' : 'fail:' + m.max;");
    assert_eq!(out, "ok");
}

#[test]
fn meter_low_high_optimum_round_trip() {
    let out = run("var m = document.createElement('meter'); \
         m.min = 0; m.max = 100; \
         m.low = 25; m.high = 75; m.optimum = 50; \
         (m.low === 25 && m.high === 75 && m.optimum === 50) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

#[test]
fn meter_high_clamps_to_low() {
    // HTML §4.10.15 actual-high algorithm: if `high < low`, the
    // actual-high promotes to `low`.  Self-review fix: clamp `high`
    // to `[low, max]` (not `[min, max]`) so the spec promotion path
    // is observable through the IDL getter.
    let out = run("var m = document.createElement('meter'); \
         m.min = 0; m.max = 100; \
         m.low = 30; m.high = 10; \
         (m.high === 30) ? 'ok' : 'fail:' + m.high;");
    assert_eq!(out, "ok");
}

#[test]
fn meter_optimum_default_midpoint() {
    let out = run("var m = document.createElement('meter'); \
         m.min = 0; m.max = 10; \
         (m.optimum === 5) ? 'ok' : 'fail:' + m.optimum;");
    assert_eq!(out, "ok");
}

#[test]
fn meter_no_position_idl() {
    // <meter> has NO `position` IDL accessor (only <progress> does).
    let out = run("var m = document.createElement('meter'); \
         function inChain(obj, name) { \
             while (obj) { \
                 if (Object.getOwnPropertyDescriptor(obj, name)) return true; \
                 obj = Object.getPrototypeOf(obj); \
             } \
             return false; \
         } \
         inChain(m, 'position') ? 'fail' : 'ok';");
    assert_eq!(out, "ok");
}

#[test]
fn meter_labels_empty() {
    let out = run("var m = document.createElement('meter'); \
         (m.labels.length === 0) ? 'ok' : 'fail';");
    assert_eq!(out, "ok");
}

// =====================================================================
// Foreign-receiver brand check (TypeError)
// =====================================================================

#[test]
fn dialog_show_on_div_throws() {
    // `dlg.show.call(div)` — direct method-from-prototype invocation
    // with a foreign receiver throws TypeError per the brand check.
    // Returns 'TypeError' on throw / 'no-throw' otherwise (matches the
    // T2a foreign-receiver test pattern, which avoids inspecting the
    // thrown object's `constructor.name`).
    let out = run("var dlg = document.createElement('dialog'); \
         var d = document.createElement('div'); \
         try { dlg.show.call(d); 'no-throw'; } catch (e) { 'TypeError'; }");
    assert_eq!(out, "TypeError");
}

#[test]
fn output_set_custom_validity_on_div_throws() {
    // ConstraintValidation mixin brand check: `setCustomValidity` on
    // a non-form-control receiver throws.  Mirrors the input/textarea
    // mixin tests in tests_validity_state.rs.
    let out = run("var o = document.createElement('output'); \
         var d = document.createElement('div'); \
         try { o.setCustomValidity.call(d, 'x'); 'no-throw'; } catch (e) { 'TypeError'; }");
    assert_eq!(out, "TypeError");
}
