//! Form-control attribute reconciliation (`FormControlReconciler`)
//! tests — validates that DOM attribute mutations (via `setAttribute`
//! IDL method and parser-path element insertion) flow through the
//! D-31 `ConsumerDispatcher` to update `FormControlState` derived
//! fields, observable through JS-level `input.value` (FCS-mediated
//! getter).
//!
//! Engine-indep arm coverage lives in `elidex-form/src/reconciler.rs`
//! Rust tests (E-1..E-8b).  This file covers the VM-bound paths
//! (E-9: createElement + setAttribute; E-10: innerHTML parser).
//!
//! Setup is inlined per-test (DOM/session live in the stack frame
//! for the duration of the `bind_vm`/`unbind` window; factoring the
//! binding into a helper would move them, invalidating the unsafe
//! pointer the bind establishes).

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

fn build_fixture(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let head = dom.create_element("head", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, head));
    assert!(dom.append_child(html, body));
    doc
}

fn eval_string(vm: &mut Vm, src: &str) -> String {
    match vm.eval(src).unwrap() {
        JsValue::String(sid) => String::from_utf16_lossy(vm.inner.strings.get(sid)),
        other => panic!("expected string, got {other:?}"),
    }
}

/// E-9 — `document.createElement('input')` + `setAttribute('value', …)`
/// → reconciler updates FCS.value → JS-observable via `input.value`
/// getter (FCS-mediated `read_state_value`).
///
/// Validates the data-flow path:
/// 1. `createElement('input')` attaches FCS (via Document post-handler).
/// 2. `setAttribute("value", "v")` writes Attributes + fires
///    `MutationEvent::AttributeChange` per D-31 chokepoint.
/// 3. `ConsumerDispatcher` dispatches to `FormControlReconciler`.
/// 4. Reconciler's `value` arm (HTML §4.10.5.4) updates FCS.value
///    (dirty_value=false at this point — no user input).
/// 5. `i.value` JS getter returns FCS.value via `read_state_value`.
#[test]
fn e9_create_element_set_attribute_value_reflects() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let out = eval_string(
        &mut vm,
        "var i = document.createElement('input'); \
         i.setAttribute('value', 'reconciled'); \
         i.value;",
    );
    assert_eq!(out, "reconciled");
    vm.unbind();
}

/// E-9b — `setAttribute("value", …)` AFTER user-input dirty flag is
/// set must NOT change `input.value` (HTML §4.10.5.4 dirty value
/// flag suppression).  Validates the reconciler's dirty-flag check
/// is end-to-end observable.
#[test]
fn e9b_set_attribute_value_suppressed_when_dirty() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let out = eval_string(
        &mut vm,
        "var i = document.createElement('input'); \
         i.value = 'user-typed'; \
         i.setAttribute('value', 'from-attr'); \
         i.value;",
    );
    // user-typed sets dirty_value=true (via IDL setter → set_value);
    // subsequent content-attribute write must NOT clobber FCS.value.
    assert_eq!(out, "user-typed");
    vm.unbind();
}

/// E-10 — `innerHTML` parser path: form-control element appended to
/// the document via parser triggers `MutationEvent::Insert` →
/// reconciler attaches FCS reading parser-set Attributes → JS reads
/// FCS-mediated `input.value`.
///
/// Validates the data-flow path:
/// 1. innerHTML parser creates `<input value="parsed">` entity with
///    Attributes pre-populated.
/// 2. Parser calls `dom.append_child(body, input)` → `Insert` event.
/// 3. `ConsumerDispatcher` dispatches to `FormControlReconciler`.
/// 4. Reconciler's Insert arm checks tag (form-control) + FCS-
///    absence guard + calls `create_form_control_state` which reads
///    parser-set Attributes → FCS.value = "parsed".
/// 5. `body.children[0].value` JS getter returns FCS.value.
#[test]
fn e10_innerhtml_parser_attaches_fcs_with_attribute_values() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_fixture(&mut dom);
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let out = eval_string(
        &mut vm,
        "document.body.innerHTML = '<input value=\"parsed\">'; \
         document.body.children[0].value;",
    );
    assert_eq!(out, "parsed");
    vm.unbind();
}
