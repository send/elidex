//! Unit tests for the tracing mark-and-sweep GC.
//!
//! Extracted from `vm/gc.rs` to keep that file under the project's
//! 1000-line convention.  Tests live as a child module of
//! [`super`] (declared via `#[cfg(test)] mod tests;` in
//! `vm/gc/mod.rs`), so `super::super::*` resolves to `crate::vm`
//! and the helpers below can reach `gc::collect_garbage` and the
//! surrounding object/upvalue machinery without re-export plumbing.

#![cfg(test)]

use super::super::coroutine_types::{FrameCompletion, GeneratorStatus, SuspendedFrame};
use super::super::frame::{CallFrame, Upvalue, UpvalueState};
use super::super::shape;
use super::super::tests::forged_frame;
use super::super::value::{
    CallMode, FunctionObject, GeneratorState, JsValue, Object, ObjectId, ObjectKind, Property,
    PropertyStorage, PropertyValue, UpvalueId,
};
use super::super::Vm;

/// Helper: create a VM and return mutable access to inner.
fn test_vm() -> Vm {
    Vm::new()
}

/// Helper: create an ordinary Object with a given prototype.
fn ordinary(proto: Option<super::super::value::ObjectId>) -> Object {
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

/// The `DOMRectReadOnly` / `DOMRect` prototypes must survive GC even
/// after their global constructors are deleted, because `VmInner`
/// retains their `ObjectId`s for host-side allocation.  Without a
/// proto-root entry the only strong path is the (deletable) global →
/// ctor → `.prototype`, so deleting the globals would leave a dangling
/// id; this guards that the prototypes are in the proto-root set.
#[cfg(feature = "engine")]
#[test]
fn gc_roots_dom_rect_prototypes_after_global_deletion() {
    let mut vm = test_vm();
    let ro = vm.inner.dom_rect_readonly_prototype.expect("ro proto set");
    let rw = vm.inner.dom_rect_prototype.expect("rw proto set");
    // Sever the only JS-reachable path to the proto↔ctor cycle.
    let dom_rect = vm.inner.strings.intern("DOMRect");
    let dom_rect_ro = vm.inner.strings.intern("DOMRectReadOnly");
    vm.inner.globals.remove(&dom_rect);
    vm.inner.globals.remove(&dom_rect_ro);
    vm.inner.gc_enabled = true;
    vm.inner.collect_garbage();
    assert!(
        vm.inner.objects[ro.0 as usize].is_some(),
        "DOMRectReadOnly.prototype must survive GC via proto-roots"
    );
    assert!(
        vm.inner.objects[rw.0 as usize].is_some(),
        "DOMRect.prototype must survive GC via proto-roots"
    );
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
            func_id: super::super::value::FuncId(0),
            upvalue_ids: vec![uv_id].into(),
            this_mode: super::super::value::ThisMode::Strict,
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
            super::super::value::PropertyKey::String(key_x),
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
    cf.call_ic_slots.push(Some(super::super::ic::CallIC {
        callee,
        func_id: super::super::value::FuncId(0),
        this_mode: super::super::value::ThisMode::Strict,
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
    cf.ic_slots.push(Some(super::super::ic::PropertyIC {
        receiver_shape: shape::ROOT_SHAPE,
        slot: 0,
        holder: super::super::ic::ICHolder::Proto {
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
    // 1000 loop iterations would push this well over 3000, so the
    // `< 2200` assertion remains a meaningful "GC actually ran"
    // signal while leaving headroom for future built-ins.  Bumped
    // from 1500 with D-9 `#11-events-modern-input`
    // (PointerEvent / DragEvent / TouchEvent / Touch / TouchList /
    // DataTransfer / DataTransferItem / DataTransferItemList — 8 new
    // ctors + 8 new prototypes + ~40 accessor pairs / methods total).
    // Bumped from 2200 with D-28 `#11-event-handler-attribute-vm`: the
    // ~108 GlobalEventHandlers / WindowEventHandlers / DAEH IDL attribute
    // accessors install one native getter + setter each across
    // HTMLElement.prototype / Window.prototype / the Document wrapper, a
    // fixed baseline of several hundred extra native-function objects.
    // Bumped from 2800 with D-17b `#11-html-element-constructor-base-vm`:
    // `globalThis.HTMLElement` adds the ctor function object + its
    // prototype-slot link + the prototype's method/accessor entries
    // and the brand-check plumbing's auxiliary objects. The
    // CE-constructor-id branding lives on
    // `HostData::ce_constructor_to_id` (host-side Rust map; D-17b R2
    // G1), so no per-ctor JS property contributes to this baseline.
    // Empirical baseline ~3216 live; 3500 keeps loop headroom.
    // Bumped from 3500 with `#11-vm-date-builtin`: `Date.prototype` + its ~45
    // method/accessor function objects (get*/getUTC*/set*/setUTC*/to* /
    // `[Symbol.toPrimitive]`) + the `Date` ctor + `now`/`parse`/`UTC` statics.
    // Empirical baseline ~3535 live; 3800 keeps the 1000-iteration loop as a
    // meaningful "GC actually ran" signal.
    assert!(
        live < 3800,
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

// ── Frame-root neutrality cells (R-0b) ────────────────
//
// Each cell builds a `CallFrame` with the shared `forged_frame` helper
// (`vm/tests/helpers.rs`) whose only reference to a fresh
// object is one handle field, runs a collection, and asks whether the object
// is still allocated.  Two residencies: a live frame on `VmInner::frames`,
// and a frame stored in a rooted `Generator`'s `SuspendedFrame`.  The
// negative controls clear the handle: the object must then be collected, so
// the cells can see a collection.

/// The frame handles under test (one per GC-relevant `CallFrame` field, with
/// `pending_completion` split by variant).
#[derive(Clone, Copy, Debug)]
enum Handle {
    This,
    UpvalueIds,
    LocalUpvalueIds,
    ActualArgs,
    NewInstance,
    ModeConstruct,
    HomeClass,
    Generator,
    PendingReturn,
    PendingThrow,
    PendingNormal,
}

/// A `SuspendedFrame` holding `frame` and nothing else.
fn suspended(frame: CallFrame) -> SuspendedFrame {
    SuspendedFrame {
        frame,
        stack_slice: Vec::new(),
        upvalue_slots: Vec::new(),
    }
}

/// A `Generator` object holding `suspended`.
fn alloc_generator(vm: &mut Vm, suspended: Option<SuspendedFrame>) -> ObjectId {
    vm.inner.alloc_object(Object {
        kind: ObjectKind::Generator(Box::new(GeneratorState {
            status: GeneratorStatus::SuspendedYield,
            suspended,
            wrapper: None,
        })),
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: None,
        extensible: true,
    })
}

/// A closed upvalue holding `obj`.
fn closed_upvalue(vm: &mut Vm, obj: ObjectId) -> UpvalueId {
    vm.inner.alloc_upvalue(Upvalue {
        state: UpvalueState::Closed(JsValue::Object(obj)),
    })
}

/// A fresh object plus a frame whose only reference to it is `handle`
/// (set iff `place`).  For `Handle::Generator` the object is a generator.
fn frame_holding(vm: &mut Vm, handle: Handle, place: bool) -> (ObjectId, CallFrame) {
    let obj = if matches!(handle, Handle::Generator) {
        alloc_generator(vm, None)
    } else {
        vm.inner.alloc_object(ordinary(None))
    };
    let mut f = forged_frame(0, Vec::new());
    if place {
        match handle {
            Handle::This => f.this_value = JsValue::Object(obj),
            Handle::UpvalueIds => {
                let uv = closed_upvalue(vm, obj);
                f.upvalue_ids = std::sync::Arc::from([uv]);
            }
            Handle::LocalUpvalueIds => {
                let uv = closed_upvalue(vm, obj);
                f.local_upvalue_ids = vec![uv];
            }
            Handle::ActualArgs => f.actual_args = Some(vec![JsValue::Object(obj)]),
            Handle::NewInstance => f.new_instance = Some(obj),
            Handle::ModeConstruct => f.mode = CallMode::Construct { new_target: obj },
            Handle::HomeClass => f.home_class = Some(obj),
            Handle::Generator => f.generator = Some(obj),
            Handle::PendingReturn => {
                f.pending_completion =
                    Some(Box::new(FrameCompletion::Return(JsValue::Object(obj))));
            }
            Handle::PendingNormal => {
                f.pending_completion =
                    Some(Box::new(FrameCompletion::Normal(JsValue::Object(obj))));
            }
            Handle::PendingThrow => {
                f.pending_completion = Some(Box::new(FrameCompletion::Throw(JsValue::Object(obj))));
            }
        }
    }
    (obj, f)
}

/// Root `gen_id` through a global.
fn root_via_global(vm: &mut Vm, gen_id: ObjectId) {
    let key = vm.inner.strings.intern("frame_root_cell_generator");
    vm.inner.globals.insert(key, JsValue::Object(gen_id));
}

/// Collect and report whether `obj` is still allocated.
fn collect_and_alive(vm: &mut Vm, obj: ObjectId) -> bool {
    vm.inner.gc_enabled = true;
    vm.inner.collect_garbage();
    vm.inner.objects[obj.0 as usize].is_some()
}

/// Live residency: the frame is pushed on `VmInner::frames`.
fn live_frame_keeps(handle: Handle, place: bool) -> bool {
    let mut vm = test_vm();
    let (obj, f) = frame_holding(&mut vm, handle, place);
    vm.inner.frames.push(f);
    collect_and_alive(&mut vm, obj)
}

/// Suspended residency: the frame sits in a `Generator`'s `SuspendedFrame`,
/// the generator rooted through a global.
fn suspended_frame_keeps(handle: Handle, place: bool) -> bool {
    let mut vm = test_vm();
    let (obj, f) = frame_holding(&mut vm, handle, place);
    let gen_id = alloc_generator(&mut vm, Some(suspended(f)));
    root_via_global(&mut vm, gen_id);
    collect_and_alive(&mut vm, obj)
}

const LIVE_HANDLES: [Handle; 11] = [
    Handle::This,
    Handle::UpvalueIds,
    Handle::LocalUpvalueIds,
    Handle::ActualArgs,
    Handle::NewInstance,
    Handle::ModeConstruct,
    Handle::HomeClass,
    Handle::Generator,
    Handle::PendingReturn,
    Handle::PendingThrow,
    // D-3: `Normal` is never stored by a writer; marked as the fail-safe.
    Handle::PendingNormal,
];

/// The suspended column: `Handle::Generator` is not in this list because that
/// row would build a *separate* generator; it is covered by its own test,
/// `suspended_frame_foreign_generator_is_rooted`.
const SUSPENDED_HANDLES: [Handle; 10] = [
    Handle::This,
    Handle::UpvalueIds,
    Handle::LocalUpvalueIds,
    Handle::ActualArgs,
    Handle::NewInstance,
    Handle::ModeConstruct,
    Handle::HomeClass,
    Handle::PendingReturn,
    Handle::PendingThrow,
    // D-3: `Normal` is never stored by a writer; marked as the fail-safe.
    Handle::PendingNormal,
];

#[test]
fn live_frame_handles_survive_collection() {
    for h in LIVE_HANDLES {
        assert!(live_frame_keeps(h, true), "live frame {h:?} must be rooted");
    }
}

#[test]
fn live_frame_negative_control_collects() {
    for h in LIVE_HANDLES {
        assert!(
            !live_frame_keeps(h, false),
            "live frame {h:?} cleared: the object must be collected"
        );
    }
}

#[test]
fn suspended_frame_handles_survive_collection() {
    for h in SUSPENDED_HANDLES {
        assert!(
            suspended_frame_keeps(h, true),
            "suspended frame {h:?} must be rooted"
        );
    }
}

#[test]
fn suspended_frame_negative_control_collects() {
    for h in SUSPENDED_HANDLES {
        assert!(
            !suspended_frame_keeps(h, false),
            "suspended frame {h:?} cleared: the object must be collected"
        );
    }
}

/// Smoke check, not a neutrality cell: the suspended frame's `generator` is
/// its owner, which survives through the global whether or not the field is
/// marked.  Proves only that marking the owner from inside its own arm
/// neither panics nor loops.
#[test]
fn suspended_frame_owner_generator_smoke() {
    let mut vm = test_vm();
    let gen_id = alloc_generator(&mut vm, None);
    let mut f = forged_frame(0, Vec::new());
    f.generator = Some(gen_id);
    let ObjectKind::Generator(state) = &mut vm.inner.get_object_mut(gen_id).kind else {
        unreachable!("alloc_generator allocates a Generator");
    };
    state.suspended = Some(suspended(f));
    root_via_global(&mut vm, gen_id);
    assert!(collect_and_alive(&mut vm, gen_id));
}

/// D-2's fail-safe: a suspended frame whose `generator` is a *foreign*
/// generator (no writer produces this) still roots it.  Unlike the owner
/// smoke check, this cell can see `generator` marked in a suspended frame.
#[test]
fn suspended_frame_foreign_generator_is_rooted() {
    assert!(suspended_frame_keeps(Handle::Generator, true));
    assert!(
        !suspended_frame_keeps(Handle::Generator, false),
        "negative control: cleared field, the foreign generator is collected"
    );
}

/// D-2's rule for `upvalue_slots`: an id outside `local_upvalue_ids` (no
/// writer produces this) is still marked; observable through the object its
/// upvalue's `Closed` value holds.
fn upvalue_slots_keep(place: bool) -> bool {
    let mut vm = test_vm();
    let obj = vm.inner.alloc_object(ordinary(None));
    let uv = closed_upvalue(&mut vm, obj);
    let gen_id = alloc_generator(
        &mut vm,
        Some(SuspendedFrame {
            upvalue_slots: if place { vec![(uv, 0)] } else { Vec::new() },
            ..suspended(forged_frame(0, Vec::new()))
        }),
    );
    root_via_global(&mut vm, gen_id);
    collect_and_alive(&mut vm, obj)
}

#[test]
fn suspended_upvalue_slots_outside_local_ids_are_rooted() {
    assert!(upvalue_slots_keep(true));
    assert!(
        !upvalue_slots_keep(false),
        "negative control: id removed from upvalue_slots, the object is collected"
    );
}

/// Pins `SuspendedFrame`'s own `stack_slice` loop in the `Generator` arm of
/// `trace`; that loop stays outside `mark_frame` (plan §1 "Out").  Observable
/// through an ordinary object referenced only from the slice.
fn stack_slice_keeps(place: bool) -> bool {
    let mut vm = test_vm();
    let obj = vm.inner.alloc_object(ordinary(None));
    let gen_id = alloc_generator(
        &mut vm,
        Some(SuspendedFrame {
            stack_slice: if place {
                vec![JsValue::Object(obj)]
            } else {
                Vec::new()
            },
            ..suspended(forged_frame(0, Vec::new()))
        }),
    );
    root_via_global(&mut vm, gen_id);
    collect_and_alive(&mut vm, obj)
}

#[test]
fn suspended_stack_slice_is_rooted() {
    assert!(stack_slice_keeps(true));
    assert!(
        !stack_slice_keeps(false),
        "negative control: value removed from stack_slice, the object is collected"
    );
}
