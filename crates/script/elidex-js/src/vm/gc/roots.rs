//! GC root-set walker: snapshot of every reachable origin in
//! [`super::super::VmInner`] plus the `mark_roots` step that seeds
//! the trace work list.
//!
//! Split from [`super::collect`] to keep each phase's file under the
//! 1000-line convention.  The struct + walker form a single
//! conceptual unit (the snapshot's only purpose is to feed
//! `mark_roots`), so they live together rather than across two
//! files.

use std::collections::HashMap;

use super::super::natives_promise::Microtask;
use super::super::value::StringId;
use super::super::value::{CallFrame, JsValue, Object, ObjectId, Upvalue};
use super::{mark_object, mark_upvalue, mark_value};

/// Snapshot of all GC root sets, borrowed immutably from `VmInner`.
pub(super) struct GcRoots<'a> {
    pub(super) stack: &'a [JsValue],
    pub(super) frames: &'a [CallFrame],
    pub(super) globals: &'a HashMap<StringId, JsValue>,
    pub(super) completion_value: JsValue,
    pub(super) current_exception: JsValue,
    pub(super) proto_roots: [Option<ObjectId>; 64],
    /// Per-subclass TypedArray prototype slots, addressed by
    /// [`super::super::value::ElementKind::index`].  Held as a borrowed
    /// slice rather than inlined into `proto_roots` so all eleven
    /// subclass entries fold into a single iter step in the mark
    /// phase (SP14).  Empty in non-engine builds.
    pub(super) subclass_array_proto_roots: &'a [Option<ObjectId>],
    /// Per-subclass TypedArray constructor slots, parallel to
    /// [`Self::subclass_array_proto_roots`] and addressed by the
    /// same [`super::super::value::ElementKind::index`].  Strong roots
    /// because [`super::super::VmInner::subclass_array_ctors`] holds
    /// these `ObjectId`s for the static `%TypedArray%.of` /
    /// `.from` reverse-lookup; without GC marking, severing the
    /// global ctor reference (e.g. `delete globalThis.Uint8Array`)
    /// would let the ctor be collected while the reverse table
    /// retains a stale id (SP8a R1 finding).  Empty in non-engine
    /// builds.
    pub(super) subclass_array_ctor_roots: &'a [Option<ObjectId>],
    pub(super) global_object: ObjectId,
    pub(super) upvalues: &'a [Upvalue],
    pub(super) objects: &'a [Option<Object>],
    /// Host-data (listeners, wrappers) if installed.
    pub(super) host_data: Option<&'a super::super::host_data::HostData>,
    /// Pending microtasks — hold references to handler functions, capability
    /// promises, and resolution values that would otherwise be unreachable.
    pub(super) microtask_queue: &'a std::collections::VecDeque<Microtask>,
    /// Currently-executing microtask (popped out of `microtask_queue`).
    /// Rooted here so the task's referenced objects survive a GC triggered
    /// by the user callback that we're running.
    pub(super) current_microtask: Option<&'a Microtask>,
    /// Rejected promises awaiting end-of-drain unhandled-rejection reporting.
    pub(super) pending_rejections: &'a [ObjectId],
    /// Pending timers — pin callbacks + args so they aren't collected
    /// between scheduling and firing.
    pub(super) timer_queue:
        &'a std::collections::BinaryHeap<super::super::natives_timer::TimerEntry>,
    /// Currently-firing timer entry (popped out of `timer_queue`).  Same
    /// invariant as `current_microtask`: the callback/args must survive
    /// any GC triggered by the running callback.
    pub(super) current_timer: Option<&'a super::super::natives_timer::TimerEntry>,
    /// Navigation state — `HistoryEntry.state: JsValue` holds arbitrary
    /// values passed to `history.pushState` / `replaceState`.  Without
    /// tracing them here, objects stored in `history.state` could be
    /// collected while still reachable via `history.state` read.
    /// Engine-only — `VmInner::navigation` is gated behind
    /// `feature = "engine"`.
    #[cfg(feature = "engine")]
    pub(super) navigation: &'a super::super::host::navigation::NavigationState,
    /// `AbortSignal` per-instance state, traced when the owning
    /// signal object survives.  Out-of-band `HashMap` so
    /// `ObjectKind::AbortSignal` stays payload-free; tracing visits
    /// every entry whose key was marked, marking the `reason` JsValue
    /// and every `abort_listeners` callback ObjectId.  Sweep tail
    /// prunes entries whose key was collected.
    #[cfg(feature = "engine")]
    pub(super) abort_signal_states:
        &'a std::collections::HashMap<ObjectId, super::super::host::abort::AbortSignalState>,
    /// `Request` / `Response` companion-Headers pointers live in
    /// these two side tables.  Passed through so `trace_work_list`
    /// can mark the paired Headers when the owning Request /
    /// Response is reachable — otherwise the Headers would be
    /// collected despite being reachable via the state entry.
    #[cfg(feature = "engine")]
    pub(super) request_states:
        &'a std::collections::HashMap<ObjectId, super::super::host::request_response::RequestState>,
    #[cfg(feature = "engine")]
    pub(super) response_states: &'a std::collections::HashMap<
        ObjectId,
        super::super::host::request_response::ResponseState,
    >,
    /// `FormData` entry list — each entry's `Blob` ObjectId must
    /// be marked so `formData.append("file", blob)` keeps the
    /// Blob alive as long as the FormData is reachable.  Same
    /// shape as `request_states` / `response_states`: `trace_work_list`
    /// looks up the entry list when an `ObjectKind::FormData`
    /// instance pops off the work list.
    #[cfg(feature = "engine")]
    pub(super) form_data_states:
        &'a std::collections::HashMap<ObjectId, Vec<super::super::host::form_data::FormDataEntry>>,
    /// `ReadableStream` per-instance state — trace step marks
    /// queue chunks, source callbacks, controller / reader
    /// back-refs, the size algorithm, and the stored error
    /// reason.  Without this fan-out the chunk values held in
    /// the queue could be collected while the stream still has
    /// a pending reader.
    #[cfg(feature = "engine")]
    pub(super) readable_stream_states: &'a std::collections::HashMap<
        ObjectId,
        super::super::host::readable_stream::ReadableStreamState,
    >,
    /// `ReadableStreamDefaultReader` per-instance state — trace
    /// step marks the stream back-ref + every pending
    /// `read()` Promise + the cached `closed` Promise.  Pending
    /// read promises are owned through the reader (rather than a
    /// VM-level strong-root list) — collecting the reader makes
    /// its read promises unreachable too, matching the spec slot
    /// `[[readRequests]]`.
    #[cfg(feature = "engine")]
    pub(super) readable_stream_reader_states:
        &'a std::collections::HashMap<ObjectId, super::super::host::readable_stream::ReaderState>,
    /// Cached `Request` / `Response` `.body` lazy stream — value
    /// `ObjectId` must be marked when the receiver is reachable
    /// so `r.body === r.body` keeps the same instance alive
    /// across GC ticks.
    #[cfg(feature = "engine")]
    pub(super) body_streams: &'a std::collections::HashMap<ObjectId, ObjectId>,
    /// `URL` per-instance state — trace step marks the linked
    /// `URLSearchParams` `ObjectId` if any so the searchParams
    /// reference held only via `let p = new URL("…").searchParams`
    /// keeps the URL's wrapper instance alive.
    #[cfg(feature = "engine")]
    pub(super) url_states:
        &'a std::collections::HashMap<ObjectId, super::super::host::url::UrlState>,
    /// `URLSearchParams ObjectId → owning URL ObjectId` reverse
    /// linkage.  Trace step marks the URL value when the keyed
    /// `URLSearchParams` is reachable so the symmetric "drop URL
    /// wrapper" case keeps the parent alive when only the
    /// searchParams identity is observable.
    #[cfg(feature = "engine")]
    pub(super) usp_parent_url: &'a std::collections::HashMap<ObjectId, ObjectId>,
    /// Pending `AbortSignal.timeout(ms)` registrations — the
    /// `ObjectId` values are signals that must survive until the
    /// timer fires (see `VmInner::pending_timeout_signals` for the
    /// full contract).  Keys are `u32` timer ids (not `ObjectId`s)
    /// so they don't need tracing.
    #[cfg(feature = "engine")]
    pub(super) pending_timeout_signals: &'a HashMap<u32, ObjectId>,
    /// Queued same-window tasks (HTML §8.1.5).  Each task holds a
    /// `JsValue` payload plus target / source `ObjectId`s that the
    /// dispatch step will read — tracing them here keeps the payload
    /// alive if GC triggers between `postMessage` and `drain_tasks`.
    #[cfg(feature = "engine")]
    pub(super) pending_tasks:
        &'a std::collections::VecDeque<super::super::host::pending_tasks::PendingTask>,
    /// `Attr` wrapper identity cache (WHATWG DOM §4.9.2).  Keyed by
    /// `(owner Element entity, qualified-name StringId)`.  Values
    /// are pinned only when the owner element wrapper is reachable
    /// — looked up via `HostData::get_cached_wrapper(entity)`; this
    /// keeps the cache effectively *weak* through the owner so a
    /// dropped element does not extend its Attrs' lifetimes.  Sweep
    /// tail prunes entries whose value `ObjectId` was collected.
    #[cfg(feature = "engine")]
    pub(super) attr_wrapper_cache:
        &'a HashMap<(elidex_ecs::Entity, super::super::value::StringId), ObjectId>,
    /// `DOMTokenList` (`Element.classList`) wrapper identity cache.
    /// Same weak-through-owner semantics as
    /// [`Self::attr_wrapper_cache`] — entries are pinned only when
    /// the owner element wrapper is still reachable.  Sweep tail
    /// prunes entries whose value `ObjectId` was collected.
    #[cfg(feature = "engine")]
    pub(super) class_list_wrapper_cache: &'a HashMap<elidex_ecs::Entity, ObjectId>,
    /// `DOMStringMap` (`HTMLElement.dataset`) wrapper identity cache.
    /// Same weak-through-owner semantics as
    /// [`Self::class_list_wrapper_cache`].
    #[cfg(feature = "engine")]
    pub(super) dataset_wrapper_cache: &'a HashMap<elidex_ecs::Entity, ObjectId>,
    /// In-flight async `fetch()` Promise pins.  Values are Promise
    /// ObjectIds that must survive until the broker reply (or abort
    /// fan-out) settles them — see [`super::super::VmInner::pending_fetches`]
    /// for the full lifecycle.  Without rooting, a `let p =
    /// fetch(url)` whose `p` is never stored elsewhere would let
    /// the Promise be collected before its settlement target lands.
    #[cfg(feature = "engine")]
    pub(super) pending_fetches: &'a HashMap<elidex_net::broker::FetchId, ObjectId>,
    // `any_composite_map` is weak bookkeeping only — no GC roots
    // live there.  The sweep pass prunes dead ObjectIds post-GC
    // and `abort_signal`'s fan-out tolerates missing state — both
    // routes avoid keeping composite signals alive through this
    // map (see `mark_roots` step (k) for the rationale).
}

/// Scan all GC roots and enqueue reachable objects.
#[allow(clippy::too_many_lines)]
pub(super) fn mark_roots(
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
                super::super::value::FrameCompletion::Return(v)
                | super::super::value::FrameCompletion::Throw(v),
            ) => {
                mark_value(*v, obj_marks, work);
            }
            Some(super::super::value::FrameCompletion::Normal(_)) | None => {}
        }
    }

    // (c) Global variables
    for &val in roots.globals.values() {
        mark_value(val, obj_marks, work);
    }

    // (d) Completion and exception
    mark_value(roots.completion_value, obj_marks, work);
    mark_value(roots.current_exception, obj_marks, work);

    // (e) Prototype ObjectIds + global object.  Subclass TypedArray
    // prototypes share the same mark step via the chained slice so
    // adding a 12th subclass is a single VmInner array bump rather
    // than a per-entry edit here.
    for &id in roots
        .proto_roots
        .iter()
        .chain(roots.subclass_array_proto_roots.iter())
        .chain(roots.subclass_array_ctor_roots.iter())
        .flatten()
    {
        mark_object(id, obj_marks, work);
    }
    mark_object(roots.global_object, obj_marks, work);

    if let Some(hd) = roots.host_data {
        for id in hd.gc_root_object_ids() {
            mark_object(id, obj_marks, work);
        }
        // (e2) `Attr` identity cache — fan out a cached `attr_id`
        // only when the owner element wrapper is still reachable
        // through `HostData::wrapper_cache`.  This makes the cache
        // weak through the owner: an element wrapper dropped from
        // `wrapper_cache` (entity destroyed via `remove_wrapper`)
        // releases its cached Attrs in the same GC, since the
        // `attr_id` is no longer reached from the owner-wrapper
        // root.  Attrs themselves carry no further fan-out
        // (`AttrState` holds only `Entity` / `StringId`), so a
        // single mark is enough — no work-list re-add needed.
        #[cfg(feature = "engine")]
        for ((entity, _), &attr_id) in roots.attr_wrapper_cache {
            if hd.get_cached_wrapper(*entity).is_some() {
                mark_object(attr_id, obj_marks, work);
            }
        }
        // (e3) `DOMTokenList` / `DOMStringMap` identity caches —
        // weak-through-owner like `attr_wrapper_cache` above: a
        // cached wrapper survives only while the owner element
        // wrapper is still rooted via `HostData::wrapper_cache`.
        // The variants are payload-free, so a single mark suffices.
        #[cfg(feature = "engine")]
        for (entity, &id) in roots.class_list_wrapper_cache {
            if hd.get_cached_wrapper(*entity).is_some() {
                mark_object(id, obj_marks, work);
            }
        }
        #[cfg(feature = "engine")]
        for (entity, &id) in roots.dataset_wrapper_cache {
            if hd.get_cached_wrapper(*entity).is_some() {
                mark_object(id, obj_marks, work);
            }
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
    let mark_timer_entry = |entry: &super::super::natives_timer::TimerEntry,
                            obj_marks: &mut [u64],
                            work: &mut Vec<u32>| {
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
            super::super::host::pending_tasks::PendingTask::PostMessage {
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

    // (j.3) Pending async-fetch Promises.  Each entry's Promise
    // ObjectId must survive between `fetch_async` enqueue and the
    // `tick_network` settlement step — without this root, a fetch
    // whose Promise the user never stored (e.g. `fetch(url).then(...)`
    // where `.then` returns a derived Promise reachable only via
    // its own reaction queue) could be collected mid-flight.  The
    // Signal back-refs map (`fetch_signal_back_refs`) is *not*
    // rooted here: signals are kept alive by their own
    // `abort_signal_states` entry, which is reached via the user's
    // `controller.signal` reference — collecting a signal whose
    // user references are gone is the correct outcome (its abort
    // handler can never fire again).
    #[cfg(feature = "engine")]
    for &promise_id in roots.pending_fetches.values() {
        mark_object(promise_id, obj_marks, work);
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
