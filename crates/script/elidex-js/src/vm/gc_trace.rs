//! Mark-phase tracer for the elidex-js VM GC.
//!
//! Split from [`super::gc`] to keep both files below the
//! 1000-line convention (cleanup tranche 2).  The mark phase
//! breaks naturally into two halves — the root-set walker
//! ([`super::gc::mark_roots`]) which seeds the work list, and
//! the work-list trace
//! ([`trace_work_list`]) which drains it by visiting the
//! transitive closure of each enqueued object.  The latter is the
//! larger of the two and the only one whose body grows linearly
//! with `ObjectKind` variants, so it lives here.
//!
//! Sweep + IC invalidation + the `collect_garbage` orchestrator
//! stay in [`super::gc`] alongside the bit-vector helpers and the
//! single-object [`super::gc::mark_value`] /
//! [`super::gc::mark_object`] / [`super::gc::mark_upvalue`]
//! primitives that this tracer calls back into.

use super::gc::{mark_object, mark_upvalue, mark_value};
#[cfg(feature = "engine")]
use super::value::ObjectId;
use super::value::{Object, ObjectKind, PropertyStorage, PropertyValue, Upvalue};

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
        super::host::abort::AbortSignalState,
    >,
    #[cfg(feature = "engine")] request_states: &std::collections::HashMap<
        ObjectId,
        super::host::request_response::RequestState,
    >,
    #[cfg(feature = "engine")] response_states: &std::collections::HashMap<
        ObjectId,
        super::host::request_response::ResponseState,
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
            // `Arc<[u8]>` — no ObjectId fan-out, so no marking
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
            }
            #[cfg(feature = "engine")]
            ObjectKind::Response => {
                if let Some(headers_id) = response_states
                    .get(&ObjectId(obj_idx))
                    .map(|s| s.headers_id)
                {
                    mark_object(headers_id, obj_marks, work);
                }
            }
            // `ArrayBuffer` / `Blob` payloads are bytes-only —
            // the backing `Arc<[u8]>` holds no ObjectId
            // references, so there is nothing to fan out here.
            // The sweep tail prunes `body_data` (ArrayBuffer
            // storage, shared with Request / Response) and
            // `blob_data` (Blob storage) entries whose key was
            // collected, mirroring `headers_states` /
            // `abort_signal_states`.
            #[cfg(feature = "engine")]
            ObjectKind::ArrayBuffer | ObjectKind::Blob => {}
            // `HtmlCollection` / `NodeList` payloads (stored in
            // `live_collection_states`) contain only `Entity`,
            // `StringId`, `Vec<StringId>`, and `Vec<Entity>` — no
            // `ObjectId` references, so the trace step has nothing
            // to fan out.  The sweep tail in `super::gc` prunes
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
