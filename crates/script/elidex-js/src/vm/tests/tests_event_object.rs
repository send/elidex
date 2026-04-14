//! PR3 C3: `ObjectKind::Event` variant + four native methods.
//!
//! Tests drive the natives directly via `NativeContext` rather than via
//! JS `eval` — the properties that expose the methods / flag state on
//! the JS side land in PR3 C4 (`create_event_object`) and PR3 C5
//! (listener dispatch sync-back).

use super::super::natives_event::{
    native_event_composed_path, native_event_prevent_default,
    native_event_stop_immediate_propagation, native_event_stop_propagation,
};
use super::super::value::{JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyStorage};
use super::super::{shape, Vm};

/// Allocate a bare `ObjectKind::Event` with the given cancelable/passive
/// settings.  No properties installed — these tests poke the internal
/// slots directly via get_object / NativeContext to decouple from C4's
/// property-installation work.
fn alloc_event(vm: &mut Vm, cancelable: bool, passive: bool) -> ObjectId {
    vm.inner.alloc_object(Object {
        kind: ObjectKind::Event {
            default_prevented: false,
            propagation_stopped: false,
            immediate_propagation_stopped: false,
            cancelable,
            passive,
            composed_path: None,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: None,
        extensible: true,
    })
}

fn event_flags(vm: &Vm, id: ObjectId) -> (bool, bool, bool) {
    match vm.inner.get_object(id).kind {
        ObjectKind::Event {
            default_prevented,
            propagation_stopped,
            immediate_propagation_stopped,
            ..
        } => (
            default_prevented,
            propagation_stopped,
            immediate_propagation_stopped,
        ),
        _ => panic!("expected Event variant"),
    }
}

#[test]
fn prevent_default_sets_flag_on_cancelable_non_passive() {
    let mut vm = Vm::new();
    let id = alloc_event(
        &mut vm, /* cancelable */ true, /* passive */ false,
    );
    let mut ctx = NativeContext { vm: &mut vm.inner };
    native_event_prevent_default(&mut ctx, JsValue::Object(id), &[]).unwrap();
    assert_eq!(event_flags(&vm, id), (true, false, false));
}

#[test]
fn prevent_default_is_noop_when_not_cancelable() {
    let mut vm = Vm::new();
    let id = alloc_event(
        &mut vm, /* cancelable */ false, /* passive */ false,
    );
    let mut ctx = NativeContext { vm: &mut vm.inner };
    native_event_prevent_default(&mut ctx, JsValue::Object(id), &[]).unwrap();
    assert_eq!(event_flags(&vm, id), (false, false, false));
}

#[test]
fn prevent_default_is_noop_on_passive_listener() {
    let mut vm = Vm::new();
    let id = alloc_event(&mut vm, /* cancelable */ true, /* passive */ true);
    let mut ctx = NativeContext { vm: &mut vm.inner };
    native_event_prevent_default(&mut ctx, JsValue::Object(id), &[]).unwrap();
    assert_eq!(event_flags(&vm, id), (false, false, false));
}

#[test]
fn prevent_default_silent_noop_on_detached_this() {
    let mut vm = Vm::new();
    // Detached-method invocation (`const pd = e.preventDefault; pd()`)
    // passes `this === undefined`.  Browser engines silently no-op —
    // matching that keeps user-land code observing the same behaviour.
    let mut ctx = NativeContext { vm: &mut vm.inner };
    let res = native_event_prevent_default(&mut ctx, JsValue::Undefined, &[]);
    assert!(res.is_ok());
}

#[test]
fn stop_propagation_sets_only_outer_flag() {
    let mut vm = Vm::new();
    let id = alloc_event(&mut vm, true, false);
    let mut ctx = NativeContext { vm: &mut vm.inner };
    native_event_stop_propagation(&mut ctx, JsValue::Object(id), &[]).unwrap();
    assert_eq!(event_flags(&vm, id), (false, true, false));
}

#[test]
fn stop_immediate_propagation_sets_both_flags() {
    let mut vm = Vm::new();
    let id = alloc_event(&mut vm, true, false);
    let mut ctx = NativeContext { vm: &mut vm.inner };
    native_event_stop_immediate_propagation(&mut ctx, JsValue::Object(id), &[]).unwrap();
    // WHATWG DOM §2.9: stopImmediatePropagation() sets BOTH flags
    // (propagation_stopped and immediate_propagation_stopped).  Setting
    // only the "immediate" flag is non-conforming — spec text explicitly
    // sets the outer flag first.
    assert_eq!(event_flags(&vm, id), (false, true, true));
}

#[test]
fn composed_path_returns_empty_array_when_slot_none() {
    let mut vm = Vm::new();
    let id = alloc_event(&mut vm, true, false);
    let mut ctx = NativeContext { vm: &mut vm.inner };
    let result = native_event_composed_path(&mut ctx, JsValue::Object(id), &[]).unwrap();
    let JsValue::Object(arr_id) = result else {
        panic!("composedPath should return an Object (Array), got {result:?}");
    };
    match &vm.inner.get_object(arr_id).kind {
        ObjectKind::Array { elements } => {
            assert!(elements.is_empty(), "empty path → empty array");
        }
        _ => panic!("composedPath should return an Array"),
    }
}

#[test]
fn composed_path_returns_cached_array_when_present() {
    let mut vm = Vm::new();
    let id = alloc_event(&mut vm, true, false);
    // Pre-seed the internal slot with a fresh Array; composedPath() must
    // return exactly that ObjectId (identity, per spec).
    let cached = vm.inner.create_array_object(vec![]);
    if let ObjectKind::Event {
        ref mut composed_path,
        ..
    } = &mut vm.inner.get_object_mut(id).kind
    {
        *composed_path = Some(cached);
    } else {
        panic!("expected Event variant");
    }

    let mut ctx = NativeContext { vm: &mut vm.inner };
    let result = native_event_composed_path(&mut ctx, JsValue::Object(id), &[]).unwrap();
    assert_eq!(
        result,
        JsValue::Object(cached),
        "composedPath must return the cached Array, not a fresh copy"
    );
}
