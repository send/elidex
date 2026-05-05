//! PR3 C9: `document` host global tests.
//!
//! Verifies that `Vm::bind` automatically installs `document` as a
//! `HostObject` wrapper of the bound document entity, that
//! `addEventListener`/`removeEventListener` resolve via the
//! `EventTarget.prototype` chain on it, and that wrapper identity
//! holds across rebinds.

#![cfg(feature = "engine")]

use elidex_ecs::EcsDom;
use elidex_script_session::SessionCore;

use super::super::test_helpers::{bind_vm, listeners_on};
use super::super::value::{JsValue, ObjectKind};
use super::super::Vm;

#[test]
fn bind_installs_document_as_host_object() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let JsValue::Object(doc_id) = vm
        .get_global("document")
        .expect("bind() must install document")
    else {
        panic!("document must be an Object");
    };
    let ObjectKind::HostObject { entity_bits } = vm.inner.get_object(doc_id).kind else {
        panic!("document must be HostObject");
    };
    assert_eq!(
        entity_bits,
        doc.to_bits().get(),
        "document HostObject must wrap the document entity"
    );

    vm.unbind();
}

#[test]
fn document_inherits_event_target_methods() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // `addEventListener` is inherited via the prototype chain.
    vm.eval(
        "globalThis.h = function () {};
         document.addEventListener('DOMContentLoaded', globalThis.h);",
    )
    .unwrap();

    let entry_count = listeners_on(&mut vm, doc)
        .matching_all("DOMContentLoaded")
        .len();
    assert_eq!(
        entry_count, 1,
        "document.addEventListener must register on the document entity"
    );

    vm.unbind();
}

#[test]
fn document_identity_is_stable_across_rebinds() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let first = vm.get_global("document").unwrap();
    vm.unbind();

    // Rebind without reinstalling HostData — the wrapper_cache entry
    // from the prior bind cycle must survive and reproduce the same
    // ObjectId (document identity stable across bind/unbind).
    // `bind_vm` is idempotent w.r.t. HostData: on this second call the
    // install step is skipped (HostData already present) and the
    // pre-existing wrapper_cache carries through.
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }
    let second = vm.get_global("document").unwrap();

    assert_eq!(
        first, second,
        "document wrapper identity must persist across bind/unbind cycles"
    );

    vm.unbind();
}
