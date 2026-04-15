//! PR3 C0: `EventTarget.prototype` intrinsic tests.
//!
//! Verifies that the prototype is allocated during `register_globals()` and
//! that it exposes the three interface methods as own data properties.
//! Functional testing of the methods themselves arrives in PR3 C5+ (first
//! green path) once `create_element_wrapper` (C2) and listener dispatch
//! (C5) are in place.

use super::super::value::{JsValue, ObjectKind, PropertyKey, PropertyValue};
use super::super::Vm;

#[test]
fn event_target_prototype_is_allocated() {
    let vm = Vm::new();
    assert!(
        vm.inner.event_target_prototype.is_some(),
        "register_globals() must allocate EventTarget.prototype"
    );
}

#[test]
fn event_target_prototype_exposes_three_methods() {
    let mut vm = Vm::new();
    let proto_id = vm
        .inner
        .event_target_prototype
        .expect("EventTarget.prototype must exist");

    // Each of the three interface methods must be an own data property
    // pointing at a NativeFunction.
    for name in ["addEventListener", "removeEventListener", "dispatchEvent"] {
        // intern is idempotent — returns the existing StringId if the
        // method name was already interned by register_event_target_prototype.
        let sid = vm.inner.strings.intern(name);
        let key = PropertyKey::String(sid);
        let storage = &vm.inner.get_object(proto_id).storage;
        let (slot, _attrs) = storage
            .get(key, &vm.inner.shapes)
            .unwrap_or_else(|| panic!("{name}: missing from EventTarget.prototype"));
        let fn_id = match slot {
            PropertyValue::Data(JsValue::Object(id)) => *id,
            other => panic!("{name}: expected Data(Object), got {other:?}"),
        };
        assert!(
            matches!(
                vm.inner.get_object(fn_id).kind,
                ObjectKind::NativeFunction(_)
            ),
            "{name}: expected NativeFunction, got other kind"
        );
    }
}

#[test]
fn event_target_prototype_chains_to_object_prototype() {
    // Per WHATWG DOM / ES spec, EventTarget.prototype's own [[Prototype]]
    // is %Object.prototype% — `create_object_with_methods` sets this by
    // default, but encode the invariant so a future refactor that changes
    // the helper can't silently break prototype-chain lookups.
    let vm = Vm::new();
    let proto_id = vm.inner.event_target_prototype.unwrap();
    let parent = vm
        .inner
        .get_object(proto_id)
        .prototype
        .expect("EventTarget.prototype must have %Object.prototype% as its prototype");
    assert_eq!(
        Some(parent),
        vm.inner.object_prototype,
        "EventTarget.prototype → Object.prototype chain broken"
    );
}
