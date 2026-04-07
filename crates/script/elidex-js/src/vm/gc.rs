//! Tracing mark-and-sweep garbage collector for the elidex-js VM.
//!
//! Collects unreachable [`Object`]s and [`Upvalue`]s.  Strings, symbols,
//! shapes, and compiled functions are permanent and not collected.
//!
//! ## Design
//!
//! - **Stop-the-world**: GC pauses all JS execution.
//! - **Bit-vector marks**: `Vec<u64>` cached on `VmInner` to avoid re-allocating.
//! - **Explicit work list**: `Vec<u32>` avoids deep recursion on the object graph.
//! - **Free functions for mark phase**: split borrow — mark bits are `&mut` while
//!   objects/upvalues/stack/frames are `&` (immutable).
//! - **IC invalidation**: after sweep, stale IC entries referencing collected
//!   objects are cleared.
//!
//! ## Future evolution
//!
//! 1. Generational GC (nursery + semi-space scavenger)
//! 2. Incremental marking (write barriers via `PropertyStorage` access points)
//! 3. Lazy sweeping + selective compaction
//! 4. Concurrent marking (separate thread)

use std::collections::HashMap;

use super::ic;
use super::value::{
    CallFrame, JsValue, Object, ObjectId, ObjectKind, PropertyStorage, PropertyValue, Upvalue,
    UpvalueId, UpvalueState,
};
use super::VmInner;
use crate::bytecode::compiled::CompiledFunction;
use crate::vm::value::StringId;

// ---------------------------------------------------------------------------
// Bit-vector helpers
// ---------------------------------------------------------------------------

#[inline]
fn bit_set(words: &mut [u64], idx: u32) {
    let (word, bit) = (idx as usize / 64, u64::from(idx) % 64);
    if word < words.len() {
        words[word] |= 1u64 << bit;
    }
}

#[inline]
fn bit_get(words: &[u64], idx: u32) -> bool {
    let (word, bit) = (idx as usize / 64, u64::from(idx) % 64);
    word < words.len() && (words[word] & (1u64 << bit)) != 0
}

fn resize_marks(marks: &mut Vec<u64>, capacity: usize) {
    let needed = capacity.div_ceil(64);
    if marks.len() < needed {
        marks.resize(needed, 0);
    }
}

fn clear_marks(marks: &mut [u64]) {
    marks.fill(0);
}

// ---------------------------------------------------------------------------
// Mark phase (free functions for split borrow)
// ---------------------------------------------------------------------------

/// Mark a JsValue: if it's an Object, enqueue for tracing.
#[inline]
fn mark_value(val: JsValue, obj_marks: &mut [u64], work: &mut Vec<u32>) {
    if let JsValue::Object(id) = val {
        mark_object(id, obj_marks, work);
    }
}

/// Mark an ObjectId as live and enqueue it for tracing (if not already marked).
#[inline]
fn mark_object(id: ObjectId, obj_marks: &mut [u64], work: &mut Vec<u32>) {
    let idx = id.0;
    if !bit_get(obj_marks, idx) {
        bit_set(obj_marks, idx);
        work.push(idx);
    }
}

/// Mark an Upvalue as live and trace its closed-over value.
#[inline]
fn mark_upvalue(
    uv_id: UpvalueId,
    upvalues: &[Upvalue],
    uv_marks: &mut [u64],
    obj_marks: &mut [u64],
    work: &mut Vec<u32>,
) {
    let idx = uv_id.0;
    if !bit_get(uv_marks, idx) {
        bit_set(uv_marks, idx);
        // Open upvalues reference the stack (already a root).
        // Closed upvalues hold a JsValue that needs marking.
        if let UpvalueState::Closed(val) = upvalues[idx as usize].state {
            mark_value(val, obj_marks, work);
        }
    }
}

/// Snapshot of all GC root sets, borrowed immutably from `VmInner`.
struct GcRoots<'a> {
    stack: &'a [JsValue],
    frames: &'a [CallFrame],
    globals: &'a HashMap<StringId, JsValue>,
    completion_value: JsValue,
    current_exception: JsValue,
    proto_roots: [Option<ObjectId>; 9],
    global_object: ObjectId,
    upvalues: &'a [Upvalue],
    objects: &'a [Option<Object>],
}

/// Scan all GC roots and enqueue reachable objects.
fn mark_roots(
    roots: &GcRoots<'_>,
    obj_marks: &mut [u64],
    uv_marks: &mut [u64],
    work: &mut Vec<u32>,
) {
    // (a) Stack values
    for &val in roots.stack {
        mark_value(val, obj_marks, work);
    }

    // (b) Call frame roots
    for frame in roots.frames {
        mark_value(frame.this_value, obj_marks, work);
        for &uv_id in &frame.upvalue_ids {
            mark_upvalue(uv_id, roots.upvalues, uv_marks, obj_marks, work);
        }
        for &uv_id in &frame.local_upvalue_ids {
            mark_upvalue(uv_id, roots.upvalues, uv_marks, obj_marks, work);
        }
        if let Some(ref args) = frame.actual_args {
            for &val in args {
                mark_value(val, obj_marks, work);
            }
        }
    }

    // (c) Global variables
    for &val in roots.globals.values() {
        mark_value(val, obj_marks, work);
    }

    // (d) Completion and exception
    mark_value(roots.completion_value, obj_marks, work);
    mark_value(roots.current_exception, obj_marks, work);

    // (e) Prototype ObjectIds + global object
    for &id in roots.proto_roots.iter().flatten() {
        mark_object(id, obj_marks, work);
    }
    mark_object(roots.global_object, obj_marks, work);
}

/// Trace the work list: pop enqueued ObjectIds, mark their transitive references.
///
/// Uses exhaustive matching on `ObjectKind` — adding a new variant without
/// updating this function will produce a compile error (no wildcard fallback).
fn trace_work_list(
    objects: &[Option<Object>],
    upvalues: &[Upvalue],
    obj_marks: &mut [u64],
    uv_marks: &mut [u64],
    work: &mut Vec<u32>,
) {
    while let Some(obj_idx) = work.pop() {
        let Some(obj) = &objects[obj_idx as usize] else {
            continue;
        };

        // Prototype
        if let Some(proto) = obj.prototype {
            mark_object(proto, obj_marks, work);
        }

        // Property storage
        trace_storage(&obj.storage, obj_marks, work);

        // ObjectKind — exhaustive, no wildcard
        match &obj.kind {
            ObjectKind::Array { elements } => {
                for &v in elements {
                    mark_value(v, obj_marks, work);
                }
            }
            ObjectKind::Function(fo) => {
                for &uv_id in &fo.upvalue_ids {
                    mark_upvalue(uv_id, upvalues, uv_marks, obj_marks, work);
                }
                if let Some(ct) = fo.captured_this {
                    mark_value(ct, obj_marks, work);
                }
            }
            ObjectKind::BoundFunction {
                target,
                bound_this,
                bound_args,
            } => {
                mark_object(*target, obj_marks, work);
                mark_value(*bound_this, obj_marks, work);
                for &v in bound_args {
                    mark_value(v, obj_marks, work);
                }
            }
            ObjectKind::ArrayIterator(state) => {
                mark_object(state.array_id, obj_marks, work);
            }
            ObjectKind::Arguments { values } => {
                for &v in values {
                    mark_value(v, obj_marks, work);
                }
            }
            // No ObjectId references — only StringId / scalar fields.
            ObjectKind::Ordinary
            | ObjectKind::NativeFunction(_)
            | ObjectKind::RegExp { .. }
            | ObjectKind::Error { .. }
            | ObjectKind::ForInIterator(_)
            | ObjectKind::StringIterator(_)
            | ObjectKind::NumberWrapper(_)
            | ObjectKind::StringWrapper(_)
            | ObjectKind::BooleanWrapper(_) => {}
        }
    }
}

fn trace_storage(storage: &PropertyStorage, obj_marks: &mut [u64], work: &mut Vec<u32>) {
    match storage {
        PropertyStorage::Shaped { slots, .. } => {
            for slot in slots {
                trace_property_value(slot, obj_marks, work);
            }
        }
        PropertyStorage::Dictionary(vec) => {
            for (_, prop) in vec {
                trace_property_value(&prop.slot, obj_marks, work);
            }
        }
    }
}

#[inline]
fn trace_property_value(pv: &PropertyValue, obj_marks: &mut [u64], work: &mut Vec<u32>) {
    match pv {
        PropertyValue::Data(v) => mark_value(*v, obj_marks, work),
        PropertyValue::Accessor { getter, setter } => {
            if let Some(g) = getter {
                mark_object(*g, obj_marks, work);
            }
            if let Some(s) = setter {
                mark_object(*s, obj_marks, work);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Sweep phase
// ---------------------------------------------------------------------------

/// Sweep unreachable objects and rebuild the free list.
/// Returns the number of live objects (`objects.len() - free_list.len()`).
fn sweep_objects(objects: &mut [Option<Object>], free_list: &mut Vec<u32>, marks: &[u64]) -> usize {
    free_list.clear();
    for (i, slot) in objects.iter_mut().enumerate() {
        let idx = i as u32;
        if slot.is_some() && !bit_get(marks, idx) {
            *slot = None;
            free_list.push(idx);
        } else if slot.is_none() {
            free_list.push(idx);
        }
    }
    objects.len() - free_list.len()
}

fn sweep_upvalues(upvalues: &mut [Upvalue], free_list: &mut Vec<u32>, marks: &[u64]) {
    free_list.clear();
    for (i, uv) in upvalues.iter_mut().enumerate() {
        let idx = i as u32;
        if !bit_get(marks, idx) {
            uv.state = UpvalueState::Closed(JsValue::Undefined);
            free_list.push(idx);
        }
    }
}

// ---------------------------------------------------------------------------
// IC invalidation
// ---------------------------------------------------------------------------

fn invalidate_ics(compiled_functions: &mut [CompiledFunction], obj_marks: &[u64]) {
    for cf in compiled_functions {
        for slot in &mut cf.ic_slots {
            if let Some(ic) = slot {
                if let ic::ICHolder::Proto { proto_id, .. } = ic.holder {
                    if !bit_get(obj_marks, proto_id.0) {
                        *slot = None;
                    }
                }
            }
        }
        for slot in &mut cf.call_ic_slots {
            if let Some(ic) = slot {
                if !bit_get(obj_marks, ic.callee.0) {
                    *slot = None;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

impl VmInner {
    /// Run a full GC cycle: mark, sweep, invalidate ICs.
    pub(crate) fn collect_garbage(&mut self) {
        // 1. Resize and clear mark bit-vectors.
        resize_marks(&mut self.gc_object_marks, self.objects.len());
        resize_marks(&mut self.gc_upvalue_marks, self.upvalues.len());
        clear_marks(&mut self.gc_object_marks);
        clear_marks(&mut self.gc_upvalue_marks);

        // 2. Mark phase — split borrow: mark bits are &mut, everything else is &.
        let roots = GcRoots {
            stack: &self.stack,
            frames: &self.frames,
            globals: &self.globals,
            completion_value: self.completion_value,
            current_exception: self.current_exception,
            proto_roots: [
                self.string_prototype,
                self.symbol_prototype,
                self.object_prototype,
                self.array_prototype,
                self.number_prototype,
                self.boolean_prototype,
                self.regexp_prototype,
                self.array_iterator_prototype,
                self.string_iterator_prototype,
            ],
            global_object: self.global_object,
            upvalues: &self.upvalues,
            objects: &self.objects,
        };

        self.gc_work_list.clear();

        mark_roots(
            &roots,
            &mut self.gc_object_marks,
            &mut self.gc_upvalue_marks,
            &mut self.gc_work_list,
        );

        trace_work_list(
            roots.objects,
            roots.upvalues,
            &mut self.gc_object_marks,
            &mut self.gc_upvalue_marks,
            &mut self.gc_work_list,
        );

        // 3. Sweep phase.
        let live_count = sweep_objects(
            &mut self.objects,
            &mut self.free_objects,
            &self.gc_object_marks,
        );
        sweep_upvalues(
            &mut self.upvalues,
            &mut self.free_upvalues,
            &self.gc_upvalue_marks,
        );

        // 4. IC invalidation.
        invalidate_ics(&mut self.compiled_functions, &self.gc_object_marks);

        // 5. Reset allocation counter and adjust threshold.
        self.gc_bytes_since_last = 0;
        self.gc_threshold = (live_count * 128).max(32768);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::shape;
    use super::super::value::{
        FunctionObject, JsValue, Object, ObjectKind, Property, PropertyStorage, PropertyValue,
        Upvalue, UpvalueState,
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
                upvalue_ids: vec![uv_id],
                this_mode: super::super::value::ThisMode::Strict,
                name: None,
                captured_this: None,
            }),
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: None,
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
        // Base live count includes built-in prototypes/constructors (~250).
        // Without GC, 1000 loop iterations would create 1000+ additional objects.
        assert!(
            live < 500,
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
}
