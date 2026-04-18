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
use super::natives_promise::Microtask;
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
    // BigInts, strings, symbols are permanent (pooled) — no tracing needed.
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
    proto_roots: [Option<ObjectId>; 21],
    global_object: ObjectId,
    upvalues: &'a [Upvalue],
    objects: &'a [Option<Object>],
    /// Host-data (listeners, wrappers) if installed.
    host_data: Option<&'a super::host_data::HostData>,
    /// Pending microtasks — hold references to handler functions, capability
    /// promises, and resolution values that would otherwise be unreachable.
    microtask_queue: &'a std::collections::VecDeque<Microtask>,
    /// Currently-executing microtask (popped out of `microtask_queue`).
    /// Rooted here so the task's referenced objects survive a GC triggered
    /// by the user callback that we're running.
    current_microtask: Option<&'a Microtask>,
    /// Rejected promises awaiting end-of-drain unhandled-rejection reporting.
    pending_rejections: &'a [ObjectId],
    /// Pending timers — pin callbacks + args so they aren't collected
    /// between scheduling and firing.
    timer_queue: &'a std::collections::BinaryHeap<super::natives_timer::TimerEntry>,
    /// Currently-firing timer entry (popped out of `timer_queue`).  Same
    /// invariant as `current_microtask`: the callback/args must survive
    /// any GC triggered by the running callback.
    current_timer: Option<&'a super::natives_timer::TimerEntry>,
    /// Navigation state — `HistoryEntry.state: JsValue` holds arbitrary
    /// values passed to `history.pushState` / `replaceState`.  Without
    /// tracing them here, objects stored in `history.state` could be
    /// collected while still reachable via `history.state` read.
    /// Engine-only — `VmInner::navigation` is gated behind
    /// `feature = "engine"`.
    #[cfg(feature = "engine")]
    navigation: &'a super::host::navigation::NavigationState,
    /// `AbortSignal` per-instance state, traced when the owning
    /// signal object survives.  Out-of-band `HashMap` so
    /// `ObjectKind::AbortSignal` stays payload-free; tracing visits
    /// every entry whose key was marked, marking the `reason` JsValue
    /// and every `abort_listeners` callback ObjectId.  Sweep tail
    /// prunes entries whose key was collected.
    #[cfg(feature = "engine")]
    abort_signal_states:
        &'a std::collections::HashMap<ObjectId, super::host::abort::AbortSignalState>,
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
        for &uv_id in frame.upvalue_ids.iter() {
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
        if let Some(id) = frame.new_instance {
            mark_object(id, obj_marks, work);
        }
        mark_value(frame.saved_completion, obj_marks, work);
        if let Some(gen_id) = frame.generator {
            mark_object(gen_id, obj_marks, work);
        }
        // Pending abrupt completion value (Return/Throw) — held across a
        // finally body execution, only alive for that window but an
        // independent root during it.
        match frame.pending_completion.as_deref() {
            Some(
                super::value::FrameCompletion::Return(v) | super::value::FrameCompletion::Throw(v),
            ) => {
                mark_value(*v, obj_marks, work);
            }
            Some(super::value::FrameCompletion::Normal(_)) | None => {}
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

    if let Some(hd) = roots.host_data {
        for id in hd.gc_root_object_ids() {
            mark_object(id, obj_marks, work);
        }
    }

    // (f) Pending microtasks.  Reactions reference their handler function
    // object, the derived (capability) promise to settle, and the resolution
    // value — all of which may be otherwise unreachable while the task waits.
    let mark_microtask = |task: &Microtask, obj_marks: &mut [u64], work: &mut Vec<u32>| match task {
        Microtask::PromiseReaction {
            handler,
            capability,
            resolution,
            ..
        } => {
            if let Some(h) = handler {
                mark_object(*h, obj_marks, work);
            }
            if let Some(cap) = capability {
                mark_object(*cap, obj_marks, work);
            }
            mark_value(*resolution, obj_marks, work);
        }
        Microtask::Callback { func } => {
            mark_object(*func, obj_marks, work);
        }
    };
    for task in roots.microtask_queue {
        mark_microtask(task, obj_marks, work);
    }
    if let Some(task) = roots.current_microtask {
        mark_microtask(task, obj_marks, work);
    }

    // (g) Unhandled-rejection watchlist.  These promises must survive until
    // the end-of-drain scan so their status/reason can be inspected for
    // diagnostic output.
    for &id in roots.pending_rejections {
        mark_object(id, obj_marks, work);
    }

    // (h) Pending timers.  Each entry pins its callback function and the
    // positional args captured at scheduling time.
    let mark_timer_entry =
        |entry: &super::natives_timer::TimerEntry, obj_marks: &mut [u64], work: &mut Vec<u32>| {
            mark_object(entry.callback, obj_marks, work);
            for &v in &entry.args {
                mark_value(v, obj_marks, work);
            }
        };
    for entry in roots.timer_queue {
        mark_timer_entry(entry, obj_marks, work);
    }
    if let Some(entry) = roots.current_timer {
        mark_timer_entry(entry, obj_marks, work);
    }

    // (i) Navigation state — `history.pushState(state, ...)` and
    // `replaceState` values.  Each entry's `state` is a `JsValue` that
    // is not reachable from any other root (not the stack, not a
    // function upvalue, and the entry itself lives on `VmInner`
    // directly).  Without marking, objects handed to `pushState` can
    // be collected between the call and a later `history.state` read.
    #[cfg(feature = "engine")]
    for entry in &roots.navigation.history_entries {
        mark_value(entry.state, obj_marks, work);
    }
}

/// Trace the work list: pop enqueued ObjectIds, mark their transitive references.
///
/// Uses exhaustive matching on `ObjectKind` — adding a new variant without
/// updating this function will produce a compile error (no wildcard fallback).
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
fn trace_work_list(
    objects: &[Option<Object>],
    upvalues: &[Upvalue],
    #[cfg(feature = "engine")] abort_signal_states: &std::collections::HashMap<
        ObjectId,
        super::host::abort::AbortSignalState,
    >,
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
                for &uv_id in fo.upvalue_ids.iter() {
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
            ObjectKind::Promise(state) => {
                mark_value(state.result, obj_marks, work);
                for reaction in &state.fulfill_reactions {
                    if let Some(h) = reaction.handler {
                        mark_object(h, obj_marks, work);
                    }
                    if let Some(cap) = reaction.capability {
                        mark_object(cap, obj_marks, work);
                    }
                }
                for reaction in &state.reject_reactions {
                    if let Some(h) = reaction.handler {
                        mark_object(h, obj_marks, work);
                    }
                    if let Some(cap) = reaction.capability {
                        mark_object(cap, obj_marks, work);
                    }
                }
            }
            ObjectKind::PromiseResolver { promise, .. } => {
                mark_object(*promise, obj_marks, work);
            }
            ObjectKind::PromiseCombinatorState(state) => {
                mark_object(state.result, obj_marks, work);
                for &v in &state.values {
                    mark_value(v, obj_marks, work);
                }
            }
            ObjectKind::PromiseCombinatorStep(step) => {
                use super::value::PromiseCombinatorStep as Step;
                let state_id = match step {
                    Step::AllFulfill { state, .. }
                    | Step::AllReject { state }
                    | Step::AllSettledFulfill { state, .. }
                    | Step::AllSettledReject { state, .. }
                    | Step::AnyFulfill { state }
                    | Step::AnyReject { state, .. } => *state,
                };
                mark_object(state_id, obj_marks, work);
            }
            ObjectKind::PromiseFinallyStep { on_finally, .. } => {
                mark_object(*on_finally, obj_marks, work);
            }
            ObjectKind::AsyncDriverStep { gen, .. } => {
                mark_object(*gen, obj_marks, work);
            }
            ObjectKind::Event { composed_path, .. } => {
                if let Some(id) = *composed_path {
                    mark_object(id, obj_marks, work);
                }
            }
            ObjectKind::Generator(state) => {
                if let Some(wrapper) = state.wrapper {
                    mark_object(wrapper, obj_marks, work);
                }
                if let Some(susp) = &state.suspended {
                    // The suspended frame carries its own set of roots —
                    // this_value, upvalue_ids, actual_args, saved_completion,
                    // new_instance, and the stack slice that was taken off
                    // the VM stack at yield time.
                    mark_value(susp.frame.this_value, obj_marks, work);
                    for &uv_id in susp.frame.upvalue_ids.iter() {
                        mark_upvalue(uv_id, upvalues, uv_marks, obj_marks, work);
                    }
                    for &uv_id in &susp.frame.local_upvalue_ids {
                        mark_upvalue(uv_id, upvalues, uv_marks, obj_marks, work);
                    }
                    if let Some(ref args) = susp.frame.actual_args {
                        for &v in args {
                            mark_value(v, obj_marks, work);
                        }
                    }
                    if let Some(id) = susp.frame.new_instance {
                        mark_object(id, obj_marks, work);
                    }
                    mark_value(susp.frame.saved_completion, obj_marks, work);
                    match susp.frame.pending_completion.as_deref() {
                        Some(
                            super::value::FrameCompletion::Return(v)
                            | super::value::FrameCompletion::Throw(v),
                        ) => {
                            mark_value(*v, obj_marks, work);
                        }
                        Some(super::value::FrameCompletion::Normal(_)) | None => {}
                    }
                    for &v in &susp.stack_slice {
                        mark_value(v, obj_marks, work);
                    }
                }
            }
            // `AbortSignal`'s mutable state lives out-of-band in
            // `VmInner::abort_signal_states`, keyed by this object's
            // own `ObjectId`.  Trace it now so reachable callbacks +
            // the `reason` value survive the sweep.  An entry that's
            // missing from the map is treated as empty (matches a
            // freshly allocated signal whose state was never
            // touched — should not happen because
            // `create_abort_signal` always inserts, but defensive
            // here costs nothing and makes the trace robust to
            // partial construction).
            #[cfg(feature = "engine")]
            ObjectKind::AbortSignal => {
                if let Some(state) = abort_signal_states.get(&ObjectId(obj_idx)) {
                    mark_value(state.reason, obj_marks, work);
                    if let Some(handler) = state.onabort {
                        mark_object(handler, obj_marks, work);
                    }
                    for &cb in &state.abort_listeners {
                        mark_object(cb, obj_marks, work);
                    }
                    // `bound_listener_removals` carries (Entity,
                    // ListenerId) pairs — Entity bits are not
                    // `ObjectId`s, and `ListenerId` lookups go through
                    // `HostData::listener_store`, which is itself
                    // rooted via `gc_root_object_ids`.  No tracing
                    // needed here.
                }
            }
            // No ObjectId references — only StringId / scalar fields.
            // `HostObject` is listed explicitly (not folded under a
            // wildcard) so adding a future field that holds an ObjectId
            // (e.g. cached child wrapper list) becomes a compile error
            // until this arm is updated.
            ObjectKind::Ordinary
            | ObjectKind::NativeFunction(_)
            | ObjectKind::RegExp { .. }
            | ObjectKind::Error { .. }
            | ObjectKind::ForInIterator(_)
            | ObjectKind::StringIterator(_)
            | ObjectKind::NumberWrapper(_)
            | ObjectKind::StringWrapper(_)
            | ObjectKind::BooleanWrapper(_)
            | ObjectKind::BigIntWrapper(_)
            | ObjectKind::SymbolWrapper(_)
            | ObjectKind::HostObject { .. } => {}
            // Non-engine builds never construct AbortSignal — the
            // variant exists in `ObjectKind` regardless of feature
            // (gating an enum variant requires `cfg_attr` on the
            // enum, which fragments downstream matching).  Tracing
            // it as a no-op leak is correct because the
            // `abort_signal_states` HashMap doesn't exist either —
            // there's nothing to trace through.
            #[cfg(not(feature = "engine"))]
            ObjectKind::AbortSignal => {}
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
                self.function_prototype,
                self.bigint_prototype,
                self.regexp_prototype,
                self.array_iterator_prototype,
                self.string_iterator_prototype,
                self.promise_prototype,
                self.generator_prototype,
                self.error_prototype,
                self.aggregate_error_prototype,
                self.event_target_prototype,
                self.node_prototype,
                self.element_prototype,
                self.window_prototype,
                self.event_methods_prototype,
                #[cfg(feature = "engine")]
                self.abort_signal_prototype,
                #[cfg(not(feature = "engine"))]
                None,
            ],
            global_object: self.global_object,
            upvalues: &self.upvalues,
            objects: &self.objects,
            host_data: self.host_data.as_deref(),
            microtask_queue: &self.microtask_queue,
            current_microtask: self.current_microtask.as_ref(),
            pending_rejections: &self.pending_rejections,
            timer_queue: &self.timer_queue,
            current_timer: self.current_timer.as_ref(),
            #[cfg(feature = "engine")]
            navigation: &self.navigation,
            #[cfg(feature = "engine")]
            abort_signal_states: &self.abort_signal_states,
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
            #[cfg(feature = "engine")]
            roots.abort_signal_states,
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

        // 4. AbortSignal out-of-band state cleanup.  Drop entries
        // whose key `ObjectId` was collected — otherwise a recycled
        // slot allocated for a different `ObjectKind` would inherit
        // stale `aborted` / `reason` / listener data.  The reverse
        // index (`abort_listener_back_refs`) is keyed by
        // `ListenerId` and valued by signal `ObjectId`; prune entries
        // whose value points at a now-dead signal so the index stays
        // bounded.
        #[cfg(feature = "engine")]
        {
            let marks = &self.gc_object_marks;
            self.abort_signal_states
                .retain(|id, _| bit_get(marks, id.0));
            self.abort_listener_back_refs
                .retain(|_, signal_id| bit_get(marks, signal_id.0));
        }

        // 5. IC invalidation.
        invalidate_ics(&mut self.compiled_functions, &self.gc_object_marks);

        // 6. Reset allocation counter and adjust threshold.
        self.gc_bytes_since_last = 0;
        self.gc_threshold = (live_count * 128).max(32768);
    }
}

// Tests live in `vm/gc_tests.rs` (sibling module declared in
// `vm/mod.rs`).  Splitting them out keeps this file under the
// project's 1000-line convention; the move is mechanical (test
// bodies unchanged, `super::super::*` paths shortened to `super::*`
// because the new file sits one level higher in the module tree).
