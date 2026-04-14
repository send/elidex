//! PR3 C1: `ObjectKind::HostObject` variant tests.
//!
//! The variant itself carries no GC-traceable references (`entity_bits`
//! is a plain `u64`), so the GC contract is simply "wildcard-free: the
//! arm compiles if and only if the variant is explicitly listed in
//! `gc::trace_work_list`".  These tests exercise allocation, field
//! round-tripping, and survival under a GC cycle when rooted via the VM
//! stack — a simpler substitute for the full `wrapper_cache` rooting
//! path, which is wired up in PR3 C2.

use super::super::value::{JsValue, Object, ObjectKind, PropertyStorage};
use super::super::{shape, Vm};

const FAKE_ENTITY_BITS: u64 = 0xDEAD_BEEF_CAFE_BABE;

fn alloc_host_object(vm: &mut Vm, entity_bits: u64) -> super::super::value::ObjectId {
    vm.inner.alloc_object(Object {
        kind: ObjectKind::HostObject { entity_bits },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        // PR3 C2 will flip this to `event_target_prototype` when the
        // full wrapper-creation path lands; for C1 `None` is fine —
        // we only care about allocation + GC semantics here.
        prototype: None,
        extensible: true,
    })
}

#[test]
fn host_object_round_trips_entity_bits() {
    let mut vm = Vm::new();
    let id = alloc_host_object(&mut vm, FAKE_ENTITY_BITS);
    match vm.inner.get_object(id).kind {
        ObjectKind::HostObject { entity_bits } => {
            assert_eq!(entity_bits, FAKE_ENTITY_BITS);
        }
        _ => panic!("expected HostObject, got different ObjectKind"),
    }
}

#[test]
fn host_object_survives_gc_when_rooted_on_stack() {
    let mut vm = Vm::new();
    let id = alloc_host_object(&mut vm, FAKE_ENTITY_BITS);

    // Root the object via the VM stack (which is scanned by the mark
    // phase) and force a collection.  After the cycle, the object slot
    // must still be `Some` and still contain a HostObject with the
    // original entity_bits.
    vm.inner.stack.push(JsValue::Object(id));
    vm.inner.collect_garbage();

    assert!(
        vm.inner.objects[id.0 as usize].is_some(),
        "rooted HostObject was collected"
    );
    match vm.inner.get_object(id).kind {
        ObjectKind::HostObject { entity_bits } => {
            assert_eq!(entity_bits, FAKE_ENTITY_BITS, "entity_bits corrupted by GC");
        }
        _ => panic!("expected HostObject after GC, got different ObjectKind"),
    }

    vm.inner.stack.pop();
}

#[test]
fn host_object_is_collected_when_unrooted() {
    let mut vm = Vm::new();
    let id = alloc_host_object(&mut vm, FAKE_ENTITY_BITS);
    // No root held anywhere — the id is just a local copy; GC must
    // reclaim the slot.
    vm.inner.collect_garbage();
    assert!(
        vm.inner.objects[id.0 as usize].is_none(),
        "unrooted HostObject survived GC"
    );
}
