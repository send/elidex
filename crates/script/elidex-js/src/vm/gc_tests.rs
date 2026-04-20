//! Unit tests for the tracing mark-and-sweep GC.
//!
//! Extracted from `vm/gc.rs` to keep that file under the project's
//! 1000-line convention.  Tests stay in the same `crate::vm` module
//! tree (declared via `#[cfg(test)] mod gc_tests;` in `vm/mod.rs`),
//! so `super::*` resolves to `crate::vm` and the helpers below can
//! reach `gc::collect_garbage` and the surrounding object/upvalue
//! machinery without re-export plumbing.

#![cfg(test)]

use super::shape;
use super::value::{
    FunctionObject, JsValue, Object, ObjectKind, Property, PropertyStorage, PropertyValue, Upvalue,
    UpvalueState,
};
use super::Vm;

/// Helper: create a VM and return mutable access to inner.
fn test_vm() -> Vm {
    Vm::new()
}

/// Helper: create an ordinary Object with a given prototype.
fn ordinary(proto: Option<super::value::ObjectId>) -> Object {
    Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    }
}

// ── Basic correctness ─────────────────────────────────

#[test]
fn gc_collects_unreachable_object() {
    let mut vm = test_vm();
    let id = vm.inner.alloc_object(ordinary(None));
    // Object is not referenced by any root.
    vm.inner.gc_enabled = true;
    vm.inner.collect_garbage();
    assert!(vm.inner.objects[id.0 as usize].is_none());
}

#[test]
fn gc_retains_reachable_from_stack() {
    let mut vm = test_vm();
    let id = vm.inner.alloc_object(ordinary(None));
    vm.inner.stack.push(JsValue::Object(id));
    vm.inner.collect_garbage();
    assert!(vm.inner.objects[id.0 as usize].is_some());
    vm.inner.stack.pop();
}

#[test]
fn gc_retains_reachable_from_global() {
    let mut vm = test_vm();
    let id = vm.inner.alloc_object(ordinary(None));
    let key = vm.inner.strings.intern("test_global");
    vm.inner.globals.insert(key, JsValue::Object(id));
    vm.inner.collect_garbage();
    assert!(vm.inner.objects[id.0 as usize].is_some());
}

#[test]
fn gc_traces_prototype_chain() {
    let mut vm = test_vm();
    let parent = vm.inner.alloc_object(ordinary(None));
    let child = vm.inner.alloc_object(ordinary(Some(parent)));
    // Root the child only.
    vm.inner.stack.push(JsValue::Object(child));
    vm.inner.collect_garbage();
    assert!(
        vm.inner.objects[parent.0 as usize].is_some(),
        "parent should survive via prototype chain"
    );
    assert!(vm.inner.objects[child.0 as usize].is_some());
    vm.inner.stack.pop();
}

#[test]
fn gc_collects_unreachable_upvalue() {
    let mut vm = test_vm();
    let obj = vm.inner.alloc_object(ordinary(None));
    let uv_id = vm.inner.alloc_upvalue(Upvalue {
        state: UpvalueState::Closed(JsValue::Object(obj)),
    });
    // Neither the upvalue nor the object is rooted.
    let _ = uv_id;
    vm.inner.collect_garbage();
    assert!(vm.inner.objects[obj.0 as usize].is_none());
}

// ── Object graph traversal ────────────────────────────

#[test]
fn gc_traces_array_elements() {
    let mut vm = test_vm();
    let elem = vm.inner.alloc_object(ordinary(None));
    let arr = vm.inner.alloc_object(Object {
        kind: ObjectKind::Array {
            elements: vec![JsValue::Object(elem)],
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: None,
        extensible: true,
    });
    vm.inner.stack.push(JsValue::Object(arr));
    vm.inner.collect_garbage();
    assert!(
        vm.inner.objects[elem.0 as usize].is_some(),
        "element should survive via array"
    );
    vm.inner.stack.pop();
}

#[test]
fn gc_traces_function_upvalues() {
    let mut vm = test_vm();
    let captured = vm.inner.alloc_object(ordinary(None));
    let uv_id = vm.inner.alloc_upvalue(Upvalue {
        state: UpvalueState::Closed(JsValue::Object(captured)),
    });
    let func = vm.inner.alloc_object(Object {
        kind: ObjectKind::Function(FunctionObject {
            func_id: super::value::FuncId(0),
            upvalue_ids: vec![uv_id].into(),
            this_mode: super::value::ThisMode::Strict,
            name: None,
            captured_this: None,
        }),
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: None,
        extensible: true,
    });
    vm.inner.stack.push(JsValue::Object(func));
    vm.inner.collect_garbage();
    assert!(
        vm.inner.objects[captured.0 as usize].is_some(),
        "captured object should survive via upvalue"
    );
    vm.inner.stack.pop();
}

#[test]
fn gc_traces_bound_function() {
    let mut vm = test_vm();
    let target = vm.inner.alloc_object(ordinary(None));
    let arg_obj = vm.inner.alloc_object(ordinary(None));
    let bound = vm.inner.alloc_object(Object {
        kind: ObjectKind::BoundFunction {
            target,
            bound_this: JsValue::Undefined,
            bound_args: vec![JsValue::Object(arg_obj)],
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: None,
        extensible: true,
    });
    vm.inner.stack.push(JsValue::Object(bound));
    vm.inner.collect_garbage();
    assert!(vm.inner.objects[target.0 as usize].is_some());
    assert!(vm.inner.objects[arg_obj.0 as usize].is_some());
    vm.inner.stack.pop();
}

#[test]
fn gc_traces_accessor_property() {
    let mut vm = test_vm();
    let getter = vm.inner.alloc_object(ordinary(None));
    let key_x = vm.inner.strings.intern("x");
    let obj = vm.inner.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::Dictionary(vec![(
            super::value::PropertyKey::String(key_x),
            Property {
                slot: PropertyValue::Accessor {
                    getter: Some(getter),
                    setter: None,
                },
                writable: false,
                enumerable: true,
                configurable: true,
            },
        )]),
        prototype: None,
        extensible: true,
    });
    vm.inner.stack.push(JsValue::Object(obj));
    vm.inner.collect_garbage();
    assert!(
        vm.inner.objects[getter.0 as usize].is_some(),
        "getter should survive via accessor"
    );
    vm.inner.stack.pop();
}

// ── Sweep + free list ─────────────────────────────────

#[test]
fn gc_free_list_reuse() {
    let mut vm = test_vm();
    let id1 = vm.inner.alloc_object(ordinary(None));
    let idx = id1.0;
    // Not rooted → will be collected.
    vm.inner.collect_garbage();
    assert!(vm.inner.objects[idx as usize].is_none());
    // Allocate again — should reuse the freed slot.
    let id2 = vm.inner.alloc_object(ordinary(None));
    assert_eq!(id2.0, idx, "freed slot should be reused");
}

#[test]
fn gc_preserves_already_free_slots() {
    let mut vm = test_vm();
    let _initial_free = vm.inner.free_objects.len();
    // Manually free a slot.
    let id = vm.inner.alloc_object(ordinary(None));
    vm.inner.objects[id.0 as usize] = None;
    vm.inner.collect_garbage();
    // The manually freed slot + any other free slots should be on the free list.
    assert!(vm.inner.free_objects.contains(&id.0));
}

// ── IC invalidation ───────────────────────────────────

#[test]
fn gc_invalidates_call_ic() {
    use crate::bytecode::compiled::CompiledFunction;

    let mut vm = test_vm();
    let callee = vm.inner.alloc_object(ordinary(None));
    // Ensure at least one compiled function exists.
    if vm.inner.compiled_functions.is_empty() {
        vm.inner.compiled_functions.push(CompiledFunction::new());
    }
    // Set up a call IC referencing the callee (not rooted).
    let cf = vm
        .inner
        .compiled_functions
        .first_mut()
        .expect("compiled_functions must not be empty");
    cf.call_ic_slots.push(Some(super::ic::CallIC {
        callee,
        func_id: super::value::FuncId(0),
        this_mode: super::value::ThisMode::Strict,
        upvalue_ids: std::sync::Arc::from([]),
        captured_this: None,
    }));
    vm.inner.collect_garbage();
    // The callee was collected → IC should be invalidated.
    let cf = vm
        .inner
        .compiled_functions
        .first()
        .expect("compiled_functions must not be empty");
    let slot = cf
        .call_ic_slots
        .last()
        .expect("call_ic_slots must not be empty");
    assert!(
        slot.is_none(),
        "call IC should be invalidated after callee collected"
    );
}

#[test]
fn gc_invalidates_proto_ic() {
    use crate::bytecode::compiled::CompiledFunction;

    let mut vm = test_vm();
    let proto = vm.inner.alloc_object(ordinary(None));
    // Ensure at least one compiled function exists.
    if vm.inner.compiled_functions.is_empty() {
        vm.inner.compiled_functions.push(CompiledFunction::new());
    }
    let cf = vm
        .inner
        .compiled_functions
        .first_mut()
        .expect("compiled_functions must not be empty");
    cf.ic_slots.push(Some(super::ic::PropertyIC {
        receiver_shape: shape::ROOT_SHAPE,
        slot: 0,
        holder: super::ic::ICHolder::Proto {
            proto_shape: shape::ROOT_SHAPE,
            proto_slot: 0,
            proto_id: proto,
        },
    }));
    vm.inner.collect_garbage();
    let cf = vm
        .inner
        .compiled_functions
        .first()
        .expect("compiled_functions must not be empty");
    let slot = cf.ic_slots.last().expect("ic_slots must not be empty");
    assert!(
        slot.is_none(),
        "proto IC should be invalidated after proto collected"
    );
}

// ── E2E ───────────────────────────────────────────────

#[test]
fn gc_heap_bounded_in_loop() {
    let mut vm = test_vm();
    // Set very low threshold to force frequent GC.
    vm.inner.gc_threshold = 128;
    vm.inner.gc_enabled = true;
    let result = vm.eval(
        "var sum = 0; for (var i = 0; i < 1000; i++) { var obj = {x: i}; sum += obj.x; } sum;",
    );
    assert_eq!(result.unwrap(), JsValue::Number(499_500.0));
    // Heap should not have grown to 1000+ objects.
    let live = vm.inner.objects.iter().filter(|o| o.is_some()).count();
    // Base live count includes built-in prototypes, constructors,
    // and their installed methods.  The count grows each time we
    // ship a new built-in interface (every `register_*_global`
    // adds one ctor + one prototype + its methods).  Without GC,
    // 1000 loop iterations would push this well over 1500, so the
    // `< 1000` assertion remains a meaningful "GC actually ran"
    // signal while leaving headroom for future built-ins.
    assert!(
        live < 1000,
        "heap should be bounded by GC, got {live} live objects"
    );
}

#[test]
fn gc_correctness_under_stress() {
    let mut vm = test_vm();
    vm.inner.gc_threshold = 128;
    vm.inner.gc_enabled = true;
    let result = vm.eval(
        "
        function make(n) {
            if (n <= 0) return {val: 0};
            var child = make(n - 1);
            return {val: n, child: child};
        }
        var tree = make(20);
        var sum = 0;
        var node = tree;
        while (node !== undefined) {
            sum += node.val;
            node = node.child;
        }
        sum;
    ",
    );
    // 0 + 1 + 2 + ... + 20 = 210
    assert_eq!(result.unwrap(), JsValue::Number(210.0));
}
