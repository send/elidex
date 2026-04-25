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

use super::gc_trace::trace_work_list;
use super::ic;
use super::natives_promise::Microtask;
use super::value::{CallFrame, JsValue, Object, ObjectId, Upvalue, UpvalueId, UpvalueState};
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
pub(super) fn mark_value(val: JsValue, obj_marks: &mut [u64], work: &mut Vec<u32>) {
    if let JsValue::Object(id) = val {
        mark_object(id, obj_marks, work);
    }
    // BigInts, strings, symbols are permanent (pooled) — no tracing needed.
}

/// Mark an ObjectId as live and enqueue it for tracing (if not already marked).
#[inline]
pub(super) fn mark_object(id: ObjectId, obj_marks: &mut [u64], work: &mut Vec<u32>) {
    let idx = id.0;
    if !bit_get(obj_marks, idx) {
        bit_set(obj_marks, idx);
        work.push(idx);
    }
}

/// Mark an Upvalue as live and trace its closed-over value.
#[inline]
pub(super) fn mark_upvalue(
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
    proto_roots: [Option<ObjectId>; 61],
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
    /// `Request` / `Response` companion-Headers pointers live in
    /// these two side tables.  Passed through so `trace_work_list`
    /// can mark the paired Headers when the owning Request /
    /// Response is reachable — otherwise the Headers would be
    /// collected despite being reachable via the state entry.
    #[cfg(feature = "engine")]
    request_states:
        &'a std::collections::HashMap<ObjectId, super::host::request_response::RequestState>,
    #[cfg(feature = "engine")]
    response_states:
        &'a std::collections::HashMap<ObjectId, super::host::request_response::ResponseState>,
    /// Pending `AbortSignal.timeout(ms)` registrations — the
    /// `ObjectId` values are signals that must survive until the
    /// timer fires (see `VmInner::pending_timeout_signals` for the
    /// full contract).  Keys are `u32` timer ids (not `ObjectId`s)
    /// so they don't need tracing.
    #[cfg(feature = "engine")]
    pending_timeout_signals: &'a HashMap<u32, ObjectId>,
    /// Queued same-window tasks (HTML §8.1.5).  Each task holds a
    /// `JsValue` payload plus target / source `ObjectId`s that the
    /// dispatch step will read — tracing them here keeps the payload
    /// alive if GC triggers between `postMessage` and `drain_tasks`.
    #[cfg(feature = "engine")]
    pending_tasks: &'a std::collections::VecDeque<super::host::pending_tasks::PendingTask>,
    // `any_composite_map` is weak bookkeeping only — no GC roots
    // live there.  The sweep pass prunes dead ObjectIds post-GC
    // and `abort_signal`'s fan-out tolerates missing state — both
    // routes avoid keeping composite signals alive through this
    // map (see `mark_roots` step (k) for the rationale).
}

/// Scan all GC roots and enqueue reachable objects.
#[allow(clippy::too_many_lines)]
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

    // (j) Pending AbortSignal.timeout registrations.  The signal
    // ObjectId is only reachable via this map until the timer
    // fires — without this root, a `let s = AbortSignal.timeout(100);`
    // where `s` is not captured anywhere else would collect the
    // signal and the subsequent internal-abort-on-fire would
    // reference a dead slot.
    #[cfg(feature = "engine")]
    for &signal_id in roots.pending_timeout_signals.values() {
        mark_object(signal_id, obj_marks, work);
    }

    // (j.2) Queued same-window tasks — JsValue payload, target, and
    // source ObjectIds must survive between `postMessage` enqueue
    // and the `drain_tasks` dispatch at the end of eval.  An
    // intermediate GC cycle (triggered by a user-script allocation
    // burst, say) would otherwise collect a message payload whose
    // only reference lives inside a `PendingTask::PostMessage`.
    #[cfg(feature = "engine")]
    for task in roots.pending_tasks {
        match task {
            super::host::pending_tasks::PendingTask::PostMessage {
                target_window_id,
                data,
                source_window_id,
                ..
            } => {
                mark_object(*target_window_id, obj_marks, work);
                mark_value(*data, obj_marks, work);
                if let Some(src) = source_window_id {
                    mark_object(*src, obj_marks, work);
                }
            }
        }
    }

    // (k) `AbortSignal.any` composite fan-out entries are weak
    // bookkeeping only — NOT GC roots.  Marking composite values
    // here would retain every `any([...])` result until every
    // input signal also dies, letting a caller that discards the
    // composite (e.g. `AbortSignal.any([a, b])` in a loop without
    // storing results) accumulate unreachable composites
    // indefinitely.
    //
    // Rooting lives on the indirect path instead: a composite
    // with an installed `'abort'` listener or `onabort` handler is
    // kept alive through `abort_signal_states` (its listener
    // callbacks are traced when the signal's own ObjectId is
    // marked — and the signal is marked through whatever JS
    // reference held it: stack frame, global, upvalue, etc.).
    // A composite with no such anchor is correctly collected; the
    // sweep tail prunes its any_composite_map entry and the
    // fan-out path in `abort_signal` tolerates dead ObjectIds
    // (`abort_signal` itself silently early-returns for
    // already-aborted / missing state).
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
    #[allow(clippy::too_many_lines)]
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
                self.event_prototype,
                #[cfg(feature = "engine")]
                self.abort_signal_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.character_data_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.text_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 23 (PR4e post-CharacterData/Text) + 1 (DocumentType) = 24.
                #[cfg(feature = "engine")]
                self.document_type_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 24 + 1 (HTMLIFrameElement, PR4f C8) = 25.
                #[cfg(feature = "engine")]
                self.html_iframe_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 25 + 1 (HTMLElement, PR5b §C1) = 26.  Spliced in
                // between HTMLIFrameElement.prototype and
                // Element.prototype so `iframe instanceof HTMLElement`
                // holds true (WHATWG §3.2.8).  Follow-up tag-specific
                // prototypes (HTMLDivElement, HTMLAnchorElement, …)
                // will chain here via the same pattern.
                #[cfg(feature = "engine")]
                self.html_element_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 26 + 1 (DOMException) = 27.
                #[cfg(feature = "engine")]
                self.dom_exception_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 27 + 1 (CustomEvent) = 28.
                #[cfg(feature = "engine")]
                self.custom_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 28 + 5 (UIEvent family) = 33.
                #[cfg(feature = "engine")]
                self.ui_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.mouse_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.keyboard_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.focus_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.input_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 33 + 4 (non-UIEvent specialized ctors) = 37.
                #[cfg(feature = "engine")]
                self.promise_rejection_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.error_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.hash_change_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.pop_state_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 37 + 5 (Fetch surface: Headers / Request / Response
                // / ArrayBuffer / Blob) = 42.  Slots past
                // `headers_prototype` are `None` placeholders until
                // the later Fetch prototypes install; the
                // `.iter().flatten()` pattern in `mark_roots` skips
                // them safely, so the array can grow in one step
                // here without committing dead arms piecemeal.
                // Every new trace entry added to a placeholder slot
                // **must** keep the flatten pattern — direct
                // indexing at a `None` slot would mark
                // `ObjectId(0)` erroneously.
                #[cfg(feature = "engine")]
                self.headers_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // [39] request_prototype / [40] response_prototype
                // land together with the Request / Response ctors.
                #[cfg(feature = "engine")]
                self.request_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.response_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // [41] array_buffer_prototype / [42] blob_prototype
                // land together with the ArrayBuffer + Blob ctors
                // (follow-up commit in the same tranche).
                #[cfg(feature = "engine")]
                self.array_buffer_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.blob_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 42 + 2 (HTMLCollection + NodeList, PR5b §C3) = 44.
                #[cfg(feature = "engine")]
                self.html_collection_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.node_list_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 44 + 2 (NamedNodeMap + Attr, PR5b §C4 / §C4.5) = 46.
                #[cfg(feature = "engine")]
                self.named_node_map_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.attr_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 46 + 13 (PR5-typed-array §C1/C2: %TypedArray% abstract
                // + DataView + 11 concrete subclass prototypes) = 59.
                // Ordered: abstract parent first (`%TypedArray%` +
                // DataView are independent; `%TypedArray%` precedes so
                // subclass rooting folds inside the abstract's reach),
                // then subclasses in `ElementKind` declaration order.
                #[cfg(feature = "engine")]
                self.typed_array_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.data_view_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.int8_array_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.uint8_array_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.uint8_clamped_array_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.int16_array_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.uint16_array_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.int32_array_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.uint32_array_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.float32_array_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.float64_array_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.bigint64_array_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.biguint64_array_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 59 + 2 (PR5a-fetch2: TextEncoder + TextDecoder) = 61.
                // WHATWG Encoding §8 surface; both chain directly to
                // Object.prototype (no shared abstract parent).
                #[cfg(feature = "engine")]
                self.text_encoder_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.text_decoder_prototype,
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
            #[cfg(feature = "engine")]
            request_states: &self.request_states,
            #[cfg(feature = "engine")]
            response_states: &self.response_states,
            #[cfg(feature = "engine")]
            pending_timeout_signals: &self.pending_timeout_signals,
            #[cfg(feature = "engine")]
            pending_tasks: &self.pending_tasks,
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
            #[cfg(feature = "engine")]
            roots.request_states,
            #[cfg(feature = "engine")]
            roots.response_states,
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
            // DOMException out-of-band state: prune entries whose
            // instance was collected so a recycled slot can't
            // inherit stale `name` / `message`.  Payload is
            // `StringId` pairs (pool-permanent) — no trace pass
            // needed during mark, only this post-sweep GC.
            self.dom_exception_states
                .retain(|id, _| bit_get(marks, id.0));
            // `pending_timeout_signals` — values are rooted during
            // mark so a collected signal is an invariant violation
            // (the `mark_roots` pass kept them alive).  Defensively
            // prune any entry whose signal *did* get collected
            // (e.g. from a hypothetical non-strong-ref path a
            // future PR introduces).
            self.pending_timeout_signals
                .retain(|_, signal_id| bit_get(marks, signal_id.0));
            // `dispatched_events` — event ObjectIds whose dispatch is
            // currently in flight.  The event is rooted during its
            // listener walk (via the caller's JS stack), so a
            // collected entry indicates the walk completed without
            // calling `dispatched_events.remove` (e.g. a Rust panic
            // in a native helper between the insert and the cleanup
            // sentinel).  Treat it as defensive: drop the stale id
            // so a recycled slot can't observe "already dispatching"
            // membership.
            self.dispatched_events.retain(|id| bit_get(marks, id.0));
            // `any_composite_map` — input → composites fan-out.
            // Prune entries whose key (input signal) was collected;
            // for surviving entries, filter the composite list by
            // live-ness.  Composites were roots during mark so a
            // filtered-out composite indicates it was reachable
            // only via this map (same pattern as
            // `pending_timeout_signals`).  An empty list after
            // filter is dropped so the map shrinks as inputs
            // outlive their composites.
            self.any_composite_map.retain(|input_id, composites| {
                if !bit_get(marks, input_id.0) {
                    return false;
                }
                composites.retain(|composite_id| bit_get(marks, composite_id.0));
                !composites.is_empty()
            });
            // `headers_states` — prune entries whose key `Headers`
            // instance was collected so a recycled slot does not
            // inherit a stale list / guard.  Matches the
            // `dom_exception_states` / `abort_signal_states`
            // post-sweep pattern.
            self.headers_states.retain(|id, _| bit_get(marks, id.0));
            // `request_states` / `response_states` / `body_data` /
            // `body_used` — companion-Headers pointers were rooted
            // during mark for reachable keys, so surviving entries
            // are intact.  Prune entries whose key was collected to
            // avoid a recycled slot inheriting stale method /
            // status / body bytes (same pattern as
            // `abort_signal_states`).  `body_data` / `body_used`
            // reach across both Request and Response keys — pruning
            // by the key's mark bit handles both cases in one pass.
            self.request_states.retain(|id, _| bit_get(marks, id.0));
            self.response_states.retain(|id, _| bit_get(marks, id.0));
            self.body_data.retain(|id, _| bit_get(marks, id.0));
            self.body_used.retain(|id| bit_get(marks, id.0));
            // `blob_data` — prune entries whose key `Blob`
            // instance was collected so a recycled slot can't
            // inherit stale bytes / type.  Matches `body_data` /
            // `headers_states` pattern.
            self.blob_data.retain(|id, _| bit_get(marks, id.0));
            // `text_decoder_states` — prune entries whose key
            // `TextDecoder` instance was collected.  The payload
            // holds no `ObjectId` references, so no per-entry
            // fan-out tracing is needed.  Same pattern as
            // `blob_data` / `headers_states`.
            self.text_decoder_states
                .retain(|id, _| bit_get(marks, id.0));
            // `live_collection_states` — shared side-table backing
            // every `ObjectKind::HtmlCollection` / `NodeList`
            // wrapper.  Same prune-by-key-mark pattern: collected
            // wrappers lose their filter entry so a recycled
            // `ObjectId` slot doesn't inherit stale filter state.
            self.live_collection_states
                .retain(|id, _| bit_get(marks, id.0));
            // `named_node_map_states` / `attr_states` — side-tables
            // for `ObjectKind::NamedNodeMap` / `ObjectKind::Attr`
            // wrappers.  Same prune pattern as above.
            self.named_node_map_states
                .retain(|id, _| bit_get(marks, id.0));
            self.attr_states.retain(|id, _| bit_get(marks, id.0));
            // `fetch_abort_observers` — prune entries whose key
            // `AbortSignal` was collected so a recycled slot can't
            // pick up stale fan-out `FetchId`s.  The values are
            // plain `FetchId(u64)` and carry no GC obligation, so
            // no per-entry filtering is needed.  Same pattern as
            // `abort_signal_states`.
            self.fetch_abort_observers
                .retain(|id, _| bit_get(marks, id.0));
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
