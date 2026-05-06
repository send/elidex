//! Mark-phase tracer for the elidex-js VM GC.
//!
//! Split from [`super`] to keep both files below the
//! 1000-line convention (cleanup tranche 2).  The mark phase
//! breaks naturally into two halves — the root-set walker
//! ([`super::roots::mark_roots`]) which seeds the work list, and
//! the work-list trace
//! ([`trace_work_list`]) which drains it by visiting the
//! transitive closure of each enqueued object.  The latter is the
//! larger of the two and the only one whose body grows linearly
//! with `ObjectKind` variants, so it lives here.
//!
//! Sweep + IC invalidation + the `collect_garbage` orchestrator
//! stay in [`super::sweep`] / [`super::collect`] alongside the
//! bit-vector helpers and the single-object [`super::mark_value`] /
//! [`super::mark_object`] / [`super::mark_upvalue`] primitives that
//! this tracer calls back into.

#[cfg(feature = "engine")]
use super::super::value::ObjectId;
use super::super::value::{Object, ObjectKind, PropertyStorage, PropertyValue, Upvalue};
use super::{mark_object, mark_upvalue, mark_value};

/// Trace the work list: pop enqueued ObjectIds, mark their transitive references.
///
/// Uses exhaustive matching on `ObjectKind` — adding a new variant without
/// updating this function will produce a compile error (no wildcard fallback).
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub(super) fn trace_work_list(
    objects: &[Option<Object>],
    upvalues: &[Upvalue],
    #[cfg(feature = "engine")] abort_signal_states: &std::collections::HashMap<
        ObjectId,
        super::super::host::abort::AbortSignalState,
    >,
    #[cfg(feature = "engine")] request_states: &std::collections::HashMap<
        ObjectId,
        super::super::host::request_response::RequestState,
    >,
    #[cfg(feature = "engine")] response_states: &std::collections::HashMap<
        ObjectId,
        super::super::host::request_response::ResponseState,
    >,
    #[cfg(feature = "engine")] form_data_states: &std::collections::HashMap<
        ObjectId,
        Vec<super::super::host::form_data::FormDataEntry>,
    >,
    #[cfg(feature = "engine")] readable_stream_states: &std::collections::HashMap<
        ObjectId,
        super::super::host::readable_stream::ReadableStreamState,
    >,
    #[cfg(feature = "engine")] readable_stream_reader_states: &std::collections::HashMap<
        ObjectId,
        super::super::host::readable_stream::ReaderState,
    >,
    #[cfg(feature = "engine")] body_streams: &std::collections::HashMap<ObjectId, ObjectId>,
    #[cfg(feature = "engine")] url_states: &std::collections::HashMap<
        ObjectId,
        super::super::host::url::UrlState,
    >,
    #[cfg(feature = "engine")] usp_parent_url: &std::collections::HashMap<ObjectId, ObjectId>,
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
                use super::super::value::PromiseCombinatorStep as Step;
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
                            super::super::value::FrameCompletion::Return(v)
                            | super::super::value::FrameCompletion::Throw(v),
                        ) => {
                            mark_value(*v, obj_marks, work);
                        }
                        Some(super::super::value::FrameCompletion::Normal(_)) | None => {}
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
            // `AbortController` carries the paired signal `ObjectId`
            // as an internal slot — mark it so the signal survives as
            // long as the controller is reachable.  Same arm runs on
            // both feature flavours because the variant exists
            // unconditionally.
            ObjectKind::AbortController { signal_id } => {
                mark_object(*signal_id, obj_marks, work);
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
            // `Headers` payload (header list + guard) lives in
            // `VmInner::headers_states`; entries hold interned
            // `StringId`s only (pool-permanent), so there is
            // nothing to `mark_value` / `mark_object` here.  The
            // sweep tail prunes entries whose key is dead — same
            // pattern as `dom_exception_states`.  Engine-only: the
            // variant itself is gated behind `feature = "engine"`.
            #[cfg(feature = "engine")]
            ObjectKind::Headers => {}
            // `Request` / `Response` carry their paired Headers
            // as an `ObjectId` in `request_states` /
            // `response_states`.  Without marking it here the
            // companion Headers would be collected whenever the
            // user code only retained the Request / Response
            // itself.  Body bytes (`body_data[id]`) are plain
            // `Vec<u8>` — no ObjectId fan-out, so no marking
            // required.  URL / method / statusText are pool-
            // permanent StringIds.
            //
            // Entries missing from `request_states` /
            // `response_states` are treated as empty (matches a
            // freshly allocated instance whose state was never
            // installed — defensive, should not happen).
            //
            // `trace_work_list`'s immutable borrow on `objects`
            // forbids us from also borrowing `VmInner` here, so
            // the trace function takes the two state maps as
            // explicit args (see the match signature below).
            #[cfg(feature = "engine")]
            ObjectKind::Request => {
                if let Some(headers_id) =
                    request_states.get(&ObjectId(obj_idx)).map(|s| s.headers_id)
                {
                    mark_object(headers_id, obj_marks, work);
                }
                if let Some(&stream_id) = body_streams.get(&ObjectId(obj_idx)) {
                    mark_object(stream_id, obj_marks, work);
                }
            }
            #[cfg(feature = "engine")]
            ObjectKind::Response => {
                if let Some(headers_id) = response_states
                    .get(&ObjectId(obj_idx))
                    .map(|s| s.headers_id)
                {
                    mark_object(headers_id, obj_marks, work);
                }
                if let Some(&stream_id) = body_streams.get(&ObjectId(obj_idx)) {
                    mark_object(stream_id, obj_marks, work);
                }
            }
            // `ArrayBuffer` / `Blob` payloads are bytes-only —
            // ArrayBuffer's `Vec<u8>` and Blob's `Arc<[u8]>` both
            // hold no ObjectId references, so there is nothing to
            // fan out here.  The sweep tail prunes `body_data`
            // (ArrayBuffer storage, shared with Request / Response)
            // and `blob_data` (Blob storage) entries whose key was
            // collected, mirroring `headers_states` /
            // `abort_signal_states`.
            #[cfg(feature = "engine")]
            ObjectKind::ArrayBuffer | ObjectKind::Blob => {}
            // `HtmlCollection` / `NodeList` payloads (stored in
            // `live_collection_states` as
            // `elidex_dom_api::LiveCollection`) contain only
            // `Entity`, owned `String` / `Vec<String>` (filter
            // needles for `ByTagName` / `ByName` / `ByClassNames`),
            // `Vec<Entity>` (cached snapshot + Snapshot-variant
            // frozen list), and `u64` (subtree version) — no
            // `ObjectId` references, so the trace step has nothing
            // to fan out.  The sweep tail in [`super::sweep`] prunes
            // entries whose key `ObjectId` was collected.
            #[cfg(feature = "engine")]
            ObjectKind::HtmlCollection | ObjectKind::NodeList => {}
            // `NamedNodeMap` / `Attr` payloads (stored in
            // `named_node_map_states` / `attr_states`) carry only
            // `Entity` and `StringId` — no `ObjectId` references —
            // so the trace step has nothing to fan out.  Sweep tail
            // prunes entries whose key `ObjectId` was collected.
            #[cfg(feature = "engine")]
            ObjectKind::NamedNodeMap | ObjectKind::Attr => {}
            // `DOMTokenList` / `DOMStringMap` carry only an inline
            // `entity_bits: u64` (not an `ObjectId`) and have no
            // side table — every accessor reads through to the
            // owner `Entity`'s `class` / `data-*` attributes via
            // `elidex_dom_api` handlers.  Trace fan-out is a no-op;
            // identity caches (`class_list_wrapper_cache` /
            // `dataset_wrapper_cache`) are scanned in mark-roots
            // step `(e3)` and pruned in the sweep tail.
            #[cfg(feature = "engine")]
            ObjectKind::DOMTokenList { .. } | ObjectKind::DOMStringMap { .. } => {}
            // `TypedArray` / `DataView` each carry the backing
            // `ArrayBuffer`'s `ObjectId` inline — the trace step
            // keeps the buffer alive while any view is reachable.
            // No side table: all other fields (`byte_offset` /
            // `byte_length` / `element_kind`) are plain `Copy`
            // values, and the buffer's own bytes live in
            // `body_data` (pruned alongside ArrayBuffer itself).
            ObjectKind::TypedArray { buffer_id, .. } | ObjectKind::DataView { buffer_id, .. } => {
                mark_object(*buffer_id, obj_marks, work);
            }
            // `TextEncoder` is stateless (payload-free variant, no
            // side table).  `TextDecoder`'s state
            // (`text_decoder_states`) holds no `ObjectId`
            // references — `encoding: &'static Encoding` and the
            // opaque `encoding_rs::Decoder` are entirely non-GC —
            // so the trace step has nothing to fan out.  Sweep
            // tail prunes `text_decoder_states` entries whose key
            // `ObjectId` was collected.
            #[cfg(feature = "engine")]
            ObjectKind::TextEncoder | ObjectKind::TextDecoder => {}
            // `URLSearchParams`'s entry list lives in
            // `url_search_params_states` and carries only interned
            // `StringId`s (pool-permanent).  The wrapper does have
            // a back-edge into `usp_parent_url` (Phase 4 of slot
            // #9.5) — when that side-table maps the searchParams
            // `ObjectId` to a parent URL, mark the URL so a script
            // holding only the searchParams keeps the URL alive
            // (matches `URLSearchParams` mutation routing through
            // `usp_parent_url` in `host::url_search_params`).
            // Sweep tail prunes dead-key entries — same pattern
            // as `headers_states`.
            #[cfg(feature = "engine")]
            ObjectKind::URLSearchParams => {
                if let Some(&parent_url) = usp_parent_url.get(&ObjectId(obj_idx)) {
                    mark_object(parent_url, obj_marks, work);
                }
            }
            // `URL` per-instance state (`url_states`) holds a
            // [`url::Url`] (pool-permanent string contents, no
            // `ObjectId`) plus the linked `URLSearchParams`
            // `ObjectId` allocated by the constructor for the
            // `searchParams` IDL attribute.  Mark that link so
            // `let p = new URL("…").searchParams; …` keeps the URL
            // alive when only the searchParams reference is held
            // (the `usp_parent_url` arm above is the symmetric
            // half).  `search_params: None` (Phase 1 standalone)
            // is a no-op.  Sweep tail prunes dead-key entries.
            #[cfg(feature = "engine")]
            ObjectKind::URL => {
                if let Some(state) = url_states.get(&ObjectId(obj_idx)) {
                    if let Some(sp_id) = state.search_params {
                        mark_object(sp_id, obj_marks, work);
                    }
                }
            }
            // `FormData`'s entry list (`form_data_states`) carries
            // Blob `ObjectId`s for `FormDataValue::Blob` entries;
            // mark each one so the Blobs survive as long as the
            // FormData itself is reachable.  String entries hold
            // only pool-permanent `StringId`s.  Filenames are
            // `StringId`s too (no marking required).
            #[cfg(feature = "engine")]
            ObjectKind::FormData => {
                if let Some(entries) = form_data_states.get(&ObjectId(obj_idx)) {
                    for entry in entries {
                        if let super::super::host::form_data::FormDataValue::Blob(blob_id) =
                            entry.value
                        {
                            mark_object(blob_id, obj_marks, work);
                        }
                    }
                }
            }
            // `ReadableStream` per-instance state holds the
            // controller / reader back-refs, queue chunks (arbitrary
            // `JsValue`s for default streams), the source callbacks
            // (`start` / `pull` / `cancel`), the queuing-strategy
            // size algorithm, and the stored error reason.  All
            // need marking so a stream that is reachable through a
            // user variable keeps its enqueued chunks + source
            // callbacks alive between read() ticks.
            #[cfg(feature = "engine")]
            ObjectKind::ReadableStream => {
                if let Some(state) = readable_stream_states.get(&ObjectId(obj_idx)) {
                    mark_object(state.controller_id, obj_marks, work);
                    if let Some(reader_id) = state.reader_id {
                        mark_object(reader_id, obj_marks, work);
                    }
                    for &(chunk, _size) in &state.queue {
                        mark_value(chunk, obj_marks, work);
                    }
                    if let Some(alg) = state.size_algorithm {
                        mark_value(alg, obj_marks, work);
                    }
                    if let Some(cb) = state.source_start {
                        mark_value(cb, obj_marks, work);
                    }
                    if let Some(cb) = state.source_pull {
                        mark_value(cb, obj_marks, work);
                    }
                    if let Some(cb) = state.source_cancel {
                        mark_value(cb, obj_marks, work);
                    }
                    if let Some(uso) = state.underlying_source {
                        mark_value(uso, obj_marks, work);
                    }
                    mark_value(state.stored_error, obj_marks, work);
                }
            }
            // `ReadableStreamDefaultReader` owns the FIFO of
            // pending `read()` Promises (spec §4.3.2
            // `[[readRequests]]`) plus the cached `closed` Promise;
            // all live in `readable_stream_reader_states`.  Mark
            // each so a reader reachable through user code keeps
            // its in-flight reads' Promises alive until they
            // settle.  The stream back-ref is also marked here so
            // a reader that has outlived the user's reference to
            // the stream still keeps the stream alive (matches
            // spec §4.3 ownership of the parent stream).
            #[cfg(feature = "engine")]
            ObjectKind::ReadableStreamDefaultReader => {
                if let Some(state) = readable_stream_reader_states.get(&ObjectId(obj_idx)) {
                    if let Some(stream_id) = state.stream_id {
                        mark_object(stream_id, obj_marks, work);
                    }
                    for &p in &state.pending_read_promises {
                        mark_object(p, obj_marks, work);
                    }
                    mark_object(state.closed_promise, obj_marks, work);
                }
            }
            // `ReadableStreamDefaultController` only carries the
            // parent stream's `ObjectId` inline; mark it so the
            // controller doesn't outlive its stream — the
            // controller's mutable state lives on the stream side
            // table, so reachability of the controller alone (e.g.
            // captured in a closure variable) must keep the stream
            // reachable too.
            #[cfg(feature = "engine")]
            ObjectKind::ReadableStreamDefaultController { stream_id } => {
                mark_object(*stream_id, obj_marks, work);
            }
            // Internal stream step callables — each holds the
            // stream / promise `ObjectId` they fire against in an
            // internal slot.  Mark it so the target survives until
            // the step actually runs (the source-callback Promise
            // may sit in the microtask queue across a GC tick).
            #[cfg(feature = "engine")]
            ObjectKind::ReadableStreamStartStep { stream_id, .. }
            | ObjectKind::ReadableStreamPullStep { stream_id, .. } => {
                mark_object(*stream_id, obj_marks, work);
            }
            #[cfg(feature = "engine")]
            ObjectKind::ReadableStreamCancelStep { promise, .. } => {
                mark_object(*promise, obj_marks, work);
            }
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
