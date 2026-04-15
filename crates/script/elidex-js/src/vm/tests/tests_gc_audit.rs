//! PR3 C11: GC audit tests for new host-side roots.
//!
//! Exercises every PR3-introduced ObjectId-bearing structure to confirm
//! they correctly participate in the mark phase and either survive a
//! collection (when reachable) or get reclaimed (when not):
//!
//! - `HostData::listener_store` — function ObjectIds rooted via
//!   `gc_root_object_ids`.
//! - `HostData::wrapper_cache` — element wrapper ObjectIds rooted
//!   via the same iterator.
//! - `ObjectKind::Event { composed_path: Option<ObjectId>, .. }` —
//!   the cached path Array is reachable from a rooted Event.
//! - `HostData::remove_wrapper` API — removing a wrapper from the
//!   cache makes it reclaimable on the next cycle.

#![cfg(feature = "engine")]

use elidex_ecs::{Attributes, EcsDom};
use elidex_script_session::SessionCore;

use super::super::shape;
use super::super::test_helpers::bind_vm;
use super::super::value::{JsValue, Object, ObjectId, ObjectKind, PropertyStorage};
use super::super::Vm;

/// Allocate an Event with a given `composed_path` ObjectId and return
/// the Event's id.  No prototype installed — these tests touch the
/// internal-slot path only.
fn alloc_event_with_path(vm: &mut Vm, path: Option<ObjectId>) -> ObjectId {
    vm.inner.alloc_object(Object {
        kind: ObjectKind::Event {
            default_prevented: false,
            propagation_stopped: false,
            immediate_propagation_stopped: false,
            cancelable: true,
            passive: false,
            composed_path: path,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: None,
        extensible: true,
    })
}

#[test]
fn listener_store_roots_function_object_across_gc() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("div", Attributes::default());
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Compile a function and store it in listener_store.  Hold no
    // other JS-side root (no global, not on stack) — only the
    // listener_store entry.
    vm.eval("globalThis.tmp = function () {};").unwrap();
    let JsValue::Object(func_id) = vm.get_global("tmp").unwrap() else {
        panic!("tmp must be an Object");
    };
    let listener_id = elidex_script_session::ListenerId::from_raw(99);
    vm.host_data().unwrap().store_listener(listener_id, func_id);

    // Drop the JS-side root by overwriting `tmp`.  Now listener_store
    // is the sole strong reference.
    vm.eval("globalThis.tmp = undefined;").unwrap();
    vm.inner.collect_garbage();

    assert!(
        vm.inner.objects[func_id.0 as usize].is_some(),
        "function held by listener_store was collected"
    );
    let _ = el;

    vm.unbind();
}

#[test]
fn wrapper_cache_roots_element_wrapper_across_gc() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let el = dom.create_element("span", Attributes::default());
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let wrapper = vm.inner.create_element_wrapper(el);
    // No global / stack reference — only wrapper_cache holds it.
    vm.inner.collect_garbage();

    assert!(
        vm.inner.objects[wrapper.0 as usize].is_some(),
        "wrapper held by wrapper_cache was collected"
    );

    vm.unbind();
}

#[test]
fn remove_wrapper_makes_entry_collectible_on_next_gc() {
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
    assert!(vm.inner.objects[wrapper.0 as usize].is_some());

    // Drop the cache entry.  The wrapper has no other root.
    let removed = vm.host_data().unwrap().remove_wrapper(el);
    assert_eq!(removed, Some(wrapper));

    vm.inner.collect_garbage();
    assert!(
        vm.inner.objects[wrapper.0 as usize].is_none(),
        "wrapper must be reclaimed once removed from cache"
    );

    vm.unbind();
}

#[test]
fn event_composed_path_is_traced_when_event_is_rooted() {
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    // Allocate the path Array first (no root).
    let path = vm.inner.create_array_object(vec![]);
    // Allocate an Event whose composed_path slot points at it.  Push
    // ONLY the Event onto the stack — the Array has no other root.
    let event = alloc_event_with_path(&mut vm, Some(path));
    vm.inner.stack.push(JsValue::Object(event));

    vm.inner.collect_garbage();

    assert!(
        vm.inner.objects[event.0 as usize].is_some(),
        "rooted Event was collected"
    );
    assert!(
        vm.inner.objects[path.0 as usize].is_some(),
        "composed_path Array must be traced via Event variant"
    );

    vm.inner.stack.pop();
    vm.unbind();
}

#[test]
fn event_composed_path_none_does_not_panic_in_trace() {
    // Defensive: Event { composed_path: None } must not crash GC.
    let mut vm = Vm::new();
    let event = alloc_event_with_path(&mut vm, None);
    vm.inner.stack.push(JsValue::Object(event));
    vm.inner.collect_garbage();
    assert!(vm.inner.objects[event.0 as usize].is_some());
    vm.inner.stack.pop();
}

#[test]
fn unrooted_event_is_collected() {
    let mut vm = Vm::new();
    let event = alloc_event_with_path(&mut vm, None);
    vm.inner.collect_garbage();
    assert!(
        vm.inner.objects[event.0 as usize].is_none(),
        "unrooted Event must be reclaimed"
    );
}

#[test]
fn host_object_in_global_survives_gc() {
    // End-to-end: a HostObject installed as a global (`document`)
    // must survive GC because globals are roots.
    let mut vm = Vm::new();
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    #[allow(unsafe_code)]
    unsafe {
        bind_vm(&mut vm, &mut session, &mut dom, doc);
    }

    let JsValue::Object(doc_id) = vm.get_global("document").unwrap() else {
        panic!("document must be installed");
    };
    vm.inner.collect_garbage();
    assert!(
        vm.inner.objects[doc_id.0 as usize].is_some(),
        "document HostObject reachable via globals + wrapper_cache must survive"
    );

    vm.unbind();
}
