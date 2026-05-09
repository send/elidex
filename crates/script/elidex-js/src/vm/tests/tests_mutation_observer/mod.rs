//! M4-12 #11-mutation-observer — `MutationObserver` thin VM binding tests.
//!
//! Phase C2 surface: constructor + brand check + 3 method stubs.
//! Phase C3 surface: `observe` / `disconnect` / `takeRecords` semantics.
//! Phase C4 surface: `mutation_record_to_js` + `Vm::deliver_mutation_records`.
//! Phase C5 surface: post-unbind tolerance + `Vm::unbind` cleanup.
//!
//! The original 1319-line single file was split phase-aligned to keep
//! each child under the 1000-line convention:
//!
//! - [`setup`] — C2 (constructor + brand) + C3 (observe / init parsing) +
//!   argument-validation edge cases.
//! - [`delivery`] — C4 (record delivery) + §G / §H + multi-observer +
//!   callback-exception isolation + takeRecords semantics.
//! - [`lifecycle`] — C5 (post-unbind tolerance + cleanup) + rebind.

#![cfg(feature = "engine")]

mod delivery;
mod lifecycle;
mod setup;

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::JsValue;
use super::super::Vm;

pub(super) fn build_doc(dom: &mut EcsDom) -> elidex_ecs::Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

pub(super) fn run(script: &str) -> String {
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

pub(super) fn run_throws(script: &str) -> String {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = build_doc(&mut dom);

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let err = vm.eval(script).expect_err("expected an error");
    vm.unbind();
    format!("{err:?}")
}

/// Build a typical document tree with a `<div id="root">` returned for
/// targeted mutations, and bind the VM.  Exposes the root-element
/// JS wrapper as `globalThis.root`.
pub(super) fn setup_with_root(
    vm: &mut Vm,
    session: &mut SessionCore,
    dom: &mut EcsDom,
) -> (elidex_ecs::Entity, elidex_ecs::Entity) {
    let doc = build_doc(dom);
    let body = dom
        .first_child_with_tag(dom.first_child_with_tag(doc, "html").unwrap(), "body")
        .unwrap();
    let root = dom.create_element("div", elidex_ecs::Attributes::default());
    assert!(dom.append_child(body, root));

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(vm, session, dom, doc);
    }
    let wrapper = vm.inner.create_element_wrapper(root);
    vm.set_global("root", JsValue::Object(wrapper));
    (doc, root)
}
