//! `create_element_wrapper` + wrapper_cache identity tests.
//!
//! These exercise the full host-bound path: a real `SessionCore` +
//! `EcsDom` is constructed, the VM is bound against them, and
//! `create_element_wrapper` is called via the host pointer.  The
//! tests verify:
//!
//! 1. Identity (`el === el`) — same Entity yields the same ObjectId.
//! 2. Distinct entities yield distinct ObjectIds.
//! 3. Element wrappers receive `Element.prototype`; Text wrappers
//!    skip it and receive `Node.prototype`.
//! 4. The wrapper's `ObjectKind` is `HostObject` with matching
//!    `entity_bits`.
//! 5. A wrapper held only by `wrapper_cache` survives a GC cycle
//!    (rooted via `HostData::gc_root_object_ids`).
//!
//! Compiled only with `feature = "engine"` — `HostData` is a stub
//! otherwise.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::test_helpers::bind_vm;
use super::super::value::{JsValue, ObjectKind};
use super::super::Vm;

#[test]
fn wrapper_is_identity_cached() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("div", Attributes::default());

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let first = vm.inner.create_element_wrapper(el);
    let second = vm.inner.create_element_wrapper(el);
    assert_eq!(first, second, "create_element_wrapper must cache by Entity");

    vm.unbind();
}

#[test]
fn wrapper_is_distinct_per_entity() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let a = dom.create_element("div", Attributes::default());
    let b = dom.create_element("span", Attributes::default());

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let wa = vm.inner.create_element_wrapper(a);
    let wb = vm.inner.create_element_wrapper(b);
    assert_ne!(wa, wb, "distinct entities must get distinct wrappers");

    vm.unbind();
}

#[test]
fn element_wrapper_prototype_chain_element_node_event_target() {
    // Full chain assertion:
    //   wrapper → Element.prototype → Node.prototype → EventTarget.prototype
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("div", Attributes::default());

    let html_element_proto = vm.inner.html_element_prototype;
    let element_proto = vm.inner.element_prototype;
    let node_proto = vm.inner.node_prototype;
    let event_target_proto = vm.inner.event_target_prototype;

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // PR5b §C1: HTML-namespace elements chain through
    // `HTMLElement.prototype` so `div instanceof HTMLElement === true`
    // (WHATWG §3.2.8).  The chain now climbs
    // `wrapper → HTMLElement → Element → Node → EventTarget`.
    let wrapper = vm.inner.create_element_wrapper(el);
    assert_eq!(
        vm.inner.get_object(wrapper).prototype,
        html_element_proto,
        "Element wrapper → HTMLElement.prototype"
    );
    assert_eq!(
        vm.inner.get_object(html_element_proto.unwrap()).prototype,
        element_proto,
        "HTMLElement.prototype → Element.prototype"
    );
    assert_eq!(
        vm.inner.get_object(element_proto.unwrap()).prototype,
        node_proto,
        "Element.prototype → Node.prototype"
    );
    assert_eq!(
        vm.inner.get_object(node_proto.unwrap()).prototype,
        event_target_proto,
        "Node.prototype → EventTarget.prototype"
    );

    vm.unbind();
}

#[test]
fn text_wrapper_prototype_is_text_prototype() {
    // PR4e C5.5: Text wrappers' immediate prototype is
    // `Text.prototype` which in turn chains to
    // `CharacterData.prototype → Node.prototype → EventTarget.prototype`.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let text = dom.create_text("hello");

    let text_proto = vm.inner.text_prototype;
    let char_data_proto = vm.inner.character_data_prototype;

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let wrapper = vm.inner.create_element_wrapper(text);
    assert_eq!(
        vm.inner.get_object(wrapper).prototype,
        text_proto,
        "Text wrapper → Text.prototype"
    );
    // Chain check: Text.prototype → CharacterData.prototype.
    assert_eq!(
        vm.inner.get_object(text_proto.unwrap()).prototype,
        char_data_proto,
        "Text.prototype → CharacterData.prototype"
    );

    vm.unbind();
}

#[test]
fn wrapper_kind_carries_entity_bits() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("div", Attributes::default());
    let expected_bits = el.to_bits().get();

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let wrapper = vm.inner.create_element_wrapper(el);
    match vm.inner.get_object(wrapper).kind {
        ObjectKind::HostObject { entity_bits } => {
            assert_eq!(entity_bits, expected_bits);
        }
        _ => panic!("expected HostObject, got different ObjectKind"),
    }

    vm.unbind();
}

#[test]
fn wrapper_survives_gc_via_cache_root() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("div", Attributes::default());

    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let wrapper = vm.inner.create_element_wrapper(el);
    // No stack root, no global — the only strong reference is
    // HostData::wrapper_cache.  A GC cycle must not reclaim the slot.
    vm.inner.collect_garbage();

    assert!(
        vm.inner.objects[wrapper.0 as usize].is_some(),
        "wrapper held by wrapper_cache was collected"
    );
    // Calling again for the same Entity must still return the same id.
    let second = vm.inner.create_element_wrapper(el);
    assert_eq!(second, wrapper, "wrapper_cache lookup must survive GC");

    // Sanity: the slot is still reachable as a JS value.
    let as_value = JsValue::Object(wrapper);
    assert!(matches!(as_value, JsValue::Object(_)));

    vm.unbind();
}
