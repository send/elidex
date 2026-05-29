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
    #[cfg(feature = "engine")] data_transfer_states: &std::collections::HashMap<
        ObjectId,
        super::super::host::events_modern::DataTransferState,
    >,
    #[cfg(feature = "engine")] touch_states: &std::collections::HashMap<
        ObjectId,
        super::super::host::events_modern::TouchState,
    >,
    #[cfg(feature = "engine")] touch_list_states: &std::collections::HashMap<
        ObjectId,
        super::super::host::events_modern::TouchListState,
    >,
    // Copilot R8: TreeWalker / NodeIterator filter ObjectIds traced
    // per-wrapper from the state tables so unreachable wrappers
    // drop their filter root (avoids the leak cycle when a filter
    // closure captures the wrapper).
    #[cfg(feature = "engine")] tree_walker_states: &std::collections::HashMap<
        u64,
        super::super::host_data::TreeWalkerState,
    >,
    #[cfg(feature = "engine")] tree_walker_instances: &std::collections::HashMap<u64, ObjectId>,
    #[cfg(feature = "engine")] node_iterator_states_shared: &std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<u64, elidex_dom_api::NodeIteratorState>>,
    >,
    #[cfg(feature = "engine")] node_iterator_instances: &std::collections::HashMap<u64, ObjectId>,
    // D-8 PR-B `#11-traversal-and-range-pr-b-selection`: Selection
    // trace fan-out marks the cached `Range` wrapper at
    // `range_instances[active_range_id.bits()]` so the registry entry
    // survives across sweeps even when the user has dropped their JS
    // Range reference (Selection itself is the JS-reachable root).
    #[cfg(feature = "engine")] selection_instance: Option<ObjectId>,
    #[cfg(feature = "engine")] selection_active_range_id_bits: Option<u64>,
    #[cfg(feature = "engine")] range_instances: &std::collections::HashMap<u64, ObjectId>,
    // D-14 `#11-file-api` — FileList / FileReader payload fan-out.
    // FileList marks each `File` wrapper in its `file_ids` Vec; FileReader
    // marks `target_blob`, `error`, the 6 on* handler ObjectIds, and an
    // `ArrayBuffer` wrapper referenced from `result`.  File has no
    // ObjectId payload (FileSideData carries StringId / f64 only) so the
    // arm is a no-op for that variant.
    #[cfg(feature = "engine")] file_list_data: &std::collections::HashMap<
        ObjectId,
        super::super::host::file_list::FileListSideData,
    >,
    #[cfg(feature = "engine")] file_reader_data: &std::collections::HashMap<
        ObjectId,
        super::super::host::file_reader::FileReaderSideData,
    >,
    // `#11-wrapper-identity-seam` — unified wrapper-identity store.
    // The `<input type=file>.files` FileList `[SameObject]` cache lives
    // here keyed by `WrapperKey::object(input_id, FileList)`: when the
    // HTMLInputElement `HostObject` is marked, its cached FileList must
    // be marked too (`MarkAgent::ViaOwnerTrace`) — otherwise the entry
    // gets sweep-pruned and the next `input.files` read allocates a
    // fresh wrapper, breaking `input.files === input.files` across GC.
    #[cfg(feature = "engine")] wrapper_store: &std::collections::HashMap<
        super::super::wrapper_intern::WrapperKey,
        ObjectId,
    >,
    // D-12 `#11-net-ws-sse` — WebSocket / EventSource handler ObjectId
    // fan-out.  Each WebSocket arm walks the 4 `on*` handler slots
    // held in the side-table; each EventSource arm walks the 3 `on*`
    // slots PLUS every listener `ObjectId` in the per-instance
    // `event_listeners` registry (CRIT-3 minimal addEventListener
    // shim).  Without this fan-out a user-assigned handler closure
    // can be collected even while the WS / SSE instance is JS-
    // reachable, causing the next event delivery to invoke a freed
    // ObjectId.
    #[cfg(feature = "engine")] websocket_states: &std::collections::HashMap<
        ObjectId,
        super::super::host_data::WebSocketState,
    >,
    #[cfg(feature = "engine")] event_source_states: &std::collections::HashMap<
        ObjectId,
        super::super::host_data::EventSourceState,
    >,
    // D-16 `#11-wasm-vm` — WebAssembly side-store fan-out.  Instance arms
    // mark `module_id` (always set) + `exports_id` if `Some` so the
    // parent Module + cached exports namespace survive while the
    // Instance is reachable.  Memory arms mark `buffer_id` if `Some`
    // so the cached `ArrayBuffer` aliasing wasm linear memory survives
    // (SameObject-style identity, plan-memo DR-11).  ExportedFunction
    // arms mark `instance_id` so the parent `WasmInstance` (and through
    // it the wasm module + linker state keeping the function callable)
    // survives.  Module / Table / Global arms are no-ops (no internal
    // `ObjectId` references).
    #[cfg(feature = "engine")] wasm_instance_storage: &std::collections::HashMap<
        ObjectId,
        super::super::wasm_payload::WasmInstancePayload,
    >,
    #[cfg(feature = "engine")] wasm_memory_storage: &std::collections::HashMap<
        ObjectId,
        super::super::wasm_payload::WasmMemoryPayload,
    >,
    #[cfg(feature = "engine")] wasm_exported_func_storage: &std::collections::HashMap<
        ObjectId,
        super::super::wasm_payload::WasmExportedFuncPayload,
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
                    // this_value, upvalue_ids, actual_args, new_instance,
                    // and the stack slice that was taken off the VM stack
                    // at yield time.
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
                    if let super::super::value::CallMode::Construct { new_target } = susp.frame.mode
                    {
                        mark_object(new_target, obj_marks, work);
                    }
                    if let Some(id) = susp.frame.home_class {
                        mark_object(id, obj_marks, work);
                    }
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
            | ObjectKind::SymbolWrapper(_) => {}
            // `HostObject` itself carries no inline ObjectId, but the
            // `<input type=file>.files` `[SameObject]` cache uses the
            // input wrapper's ObjectId as key — fan out so the cached
            // FileList survives across GC even when JS has dropped
            // its direct reference.  HashMap::get is O(1) and the map
            // is empty whenever no `<input type=file>.files` has been
            // observed, so this stays close to a no-op in common case.
            ObjectKind::HostObject { .. } => {
                #[cfg(feature = "engine")]
                if let Some(&file_list_id) =
                    wrapper_store.get(&super::super::wrapper_intern::WrapperKey::object(
                        ObjectId(obj_idx),
                        super::super::wrapper_intern::WrapperKind::FileList,
                    ))
                {
                    mark_object(file_list_id, obj_marks, work);
                }
            }
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
            // `CSSStyleDeclaration` carries only `(source, key_bits)`
            // inline — no `ObjectId` references and no side table.
            // Inline-source identity is tracked by
            // `style_wrapper_cache` (scanned in mark-roots `(e3)`,
            // pruned in the sweep tail); Computed-source wrappers are
            // not cached.  Trace fan-out is a no-op.
            #[cfg(feature = "engine")]
            ObjectKind::CSSStyleDeclaration { .. } => {}
            // CSSOM stylesheet wrappers (`#11-style-declaration` PR-B)
            // — every variant carries only inline `Entity` / `rule_id`
            // bits, no `ObjectId` references.  Identity for cached
            // wrappers (CSSStyleSheet / CSSStyleRule /
            // CSSRuleStyleDeclaration) is tracked by their respective
            // `*_wrapper_cache` maps, scanned in mark-roots `(e3)` and
            // pruned in the sweep tail.  CSSRuleList / StyleSheetList
            // are fresh-alloc per access (not cached).
            #[cfg(feature = "engine")]
            ObjectKind::CSSStyleSheet { .. }
            | ObjectKind::CSSRuleList { .. }
            | ObjectKind::CSSStyleRule { .. }
            | ObjectKind::CSSRuleStyleDeclaration { .. }
            | ObjectKind::StyleSheetList { .. } => {}
            // Observer-family (MutationObserver / ResizeObserver /
            // IntersectionObserver) — unified `ObjectKind::Observer`
            // variant.  Carries only the inline `(kind, observer_id)`,
            // no `ObjectId`, so trace fan-out from here is a no-op for
            // all three kinds.  Per-observer state lives crate-side
            // (queued records / target lists / init config) or as
            // `*ObservedBy` ECS components on observed entities (no
            // per-VM `ObjectId` to fan out to); the JS `(callback,
            // instance)` pair lives on `HostData::*_observer_bindings`
            // and is rooted via `HostData::gc_root_object_ids` for the
            // observer's lifetime.  The registries' queued records hold
            // only `Entity` / `String` values (no `ObjectId`), so they
            // also need no trace pass.
            #[cfg(feature = "engine")]
            ObjectKind::Observer { .. } => {}
            // `Storage` instances carry only the `is_local: bool`
            // discriminator inline (not an `ObjectId`).  The cached
            // `localStorage` / `sessionStorage` wrappers are rooted
            // via `VmInner::storage_local_instance` /
            // `VmInner::storage_session_instance` (mark-roots step).
            // No trace fan-out here.
            #[cfg(feature = "engine")]
            ObjectKind::Storage { .. } => {}
            // `Crypto` / `SubtleCrypto` are payload-free singletons.
            // The cached `crypto` / `crypto.subtle` wrappers are rooted
            // via `VmInner::crypto_instance` /
            // `VmInner::subtle_crypto_instance` (mark-roots step).
            // No side-table state, no inline `ObjectId` — trace
            // fan-out is a no-op.
            #[cfg(feature = "engine")]
            ObjectKind::Crypto | ObjectKind::SubtleCrypto => {}
            // `CustomElementRegistry` is a payload-free singleton.
            // Registered constructor `ObjectId`s + pending whenDefined
            // resolvers live on `HostData` (per-VM, traced through the
            // host-data GC root path), not inline on the wrapper.
            #[cfg(feature = "engine")]
            ObjectKind::CustomElementRegistry => {}
            // `StorageEvent` has no inline `ObjectId` payload — the
            // 5 IDL attributes (`key` / `oldValue` / `newValue` /
            // `url` / `storageArea`) live as own-data props on the
            // shape, traced through the ordinary shaped-storage walk.
            #[cfg(feature = "engine")]
            ObjectKind::StorageEvent => {}
            // `ValidityState` carries only an `entity_bits: u64` —
            // no inline `ObjectId` payload to trace.  The
            // `validity_state_wrappers` cache is pruned in the
            // sweep tail so a recycled entity never inherits a
            // stale wrapper.
            #[cfg(feature = "engine")]
            ObjectKind::ValidityState { .. } => {}
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
            // D-9 events-modern-input (slot `#11-events-modern-input`).
            // `DataTransfer` state holds `[SameObject]` wrapper
            // caches + per-entry blob references — mark all
            // reachable ObjectIds.
            #[cfg(feature = "engine")]
            ObjectKind::DataTransfer => {
                if let Some(state) = data_transfer_states.get(&ObjectId(obj_idx)) {
                    if let Some(items_wrapper) = state.items_wrapper {
                        mark_object(items_wrapper, obj_marks, work);
                    }
                    if let Some(files_wrapper) = state.files_wrapper {
                        mark_object(files_wrapper, obj_marks, work);
                    }
                    for entry in &state.items {
                        if let super::super::host::events_modern::DataTransferEntry::File {
                            file_id,
                            ..
                        } = entry
                        {
                            mark_object(*file_id, obj_marks, work);
                        }
                    }
                    // D-14 `#11-file-api` — `file_entries` carries the
                    // `[SameObject]` File wrapper IDs surfaced via
                    // `DataTransfer.files`.  Mark them so the FileList
                    // wrapper + each File survives across GC cycles
                    // while the DataTransfer is reachable.
                    for &file_id in &state.file_entries {
                        mark_object(file_id, obj_marks, work);
                    }
                    // `drag_image_entity` is raw `entity_bits` (not an
                    // ObjectId); the corresponding HostObject wrapper
                    // is rooted via `HostData::wrapper_cache` separately,
                    // so no extra mark needed here.
                }
            }
            // `DataTransferItem` / `DataTransferItemList` carry the
            // parent DataTransfer `ObjectId` inline; mark it so the
            // parent stays reachable through any held wrapper.
            #[cfg(feature = "engine")]
            ObjectKind::DataTransferItem { parent_dt_id, .. } => {
                mark_object(*parent_dt_id, obj_marks, work);
            }
            #[cfg(feature = "engine")]
            ObjectKind::DataTransferItemList { parent_dt_id } => {
                mark_object(*parent_dt_id, obj_marks, work);
            }
            // `Touch` state holds the EventTarget `target` ObjectId
            // (HostObject / AbortSignal / etc.).  Mark it so the
            // target survives as long as the Touch is reachable.
            #[cfg(feature = "engine")]
            ObjectKind::Touch => {
                if let Some(state) = touch_states.get(&ObjectId(obj_idx)) {
                    if let Some(target) = state.target {
                        mark_object(target, obj_marks, work);
                    }
                }
            }
            // `TouchList` state holds the ordered Vec of member
            // `Touch` ObjectIds; mark each.
            #[cfg(feature = "engine")]
            ObjectKind::TouchList => {
                if let Some(state) = touch_list_states.get(&ObjectId(obj_idx)) {
                    for &touch_id in &state.items {
                        mark_object(touch_id, obj_marks, work);
                    }
                }
            }
            // D-8 PR-A2 — `Range` carries only the registry ID
            // inline.  Range struct fields (start/end containers,
            // owner_document) are Entity bits (ECS-managed, NOT
            // GC-rooted — dangling-collapse handles post-destroy
            // semantics).  No filter callback → no trace fan-out.
            #[cfg(feature = "engine")]
            ObjectKind::Range { .. } => {}
            // `StaticRange` boundaries are eagerly captured entity
            // bits; no filter / no ObjectId payload.  Stale entity
            // bits return `isValid() == false`.
            #[cfg(feature = "engine")]
            ObjectKind::StaticRange { .. } => {}
            // Copilot R8: trace fan-out marks the filter ObjectId
            // ONLY when the wrapper itself is reachable (via the
            // wrapper → state-table → filter chain).  Previously,
            // `HostData::gc_root_object_ids` rooted filters
            // unconditionally, which leaked the wrapper when the
            // filter closure captured it (cycle: filter root → wrapper
            // mark → state-table preserved → filter still rooted).
            // Wrapper unreachability now correctly drops the filter.
            #[cfg(feature = "engine")]
            ObjectKind::TreeWalker { walker_id } => {
                // Reverse-look-up the walker_id from the instance
                // map; if its wrapper ObjectId matches the current
                // object we're tracing, mark the filter.  (The
                // tree_walker_instances map's value IS this
                // ObjectId, so the lookup-by-key gives the canonical
                // filter ObjectId.)
                if tree_walker_instances.get(walker_id) == Some(&ObjectId(obj_idx)) {
                    if let Some(state) = tree_walker_states.get(walker_id) {
                        if let Some(bits) = state.filter_object_id {
                            #[allow(clippy::cast_possible_truncation)]
                            mark_object(ObjectId(bits as u32), obj_marks, work);
                        }
                    }
                }
            }
            #[cfg(feature = "engine")]
            ObjectKind::NodeIterator { iterator_id } => {
                if node_iterator_instances.get(iterator_id) == Some(&ObjectId(obj_idx)) {
                    if let Ok(guard) = node_iterator_states_shared.lock() {
                        if let Some(state) = guard.get(iterator_id) {
                            if let Some(bits) = state.filter_object_id {
                                #[allow(clippy::cast_possible_truncation)]
                                mark_object(ObjectId(bits as u32), obj_marks, work);
                            }
                        }
                    }
                }
            }
            // D-8 PR-B Selection singleton — fans out to the
            // canonical Range wrapper at
            // `range_instances[active_range_id.bits()]`.  Per arch
            // IMP-3 of the plan-v4 self-review: without this edge, a
            // Range whose only JS-reachable reference is `Selection`
            // (e.g. set internally by `collapse(node, 0)` and never
            // exposed via `getRangeAt(0)`) would have no wrapper to
            // mark, and the GC sweep tail would unregister the
            // RangeId — leaving subsequent `getRangeAt(0)` unable to
            // resolve.  Marking the wrapper (when one has been
            // materialised) keeps the wrapper alive across sweeps,
            // which in turn keeps the RangeId registered.  If no
            // wrapper exists yet, `getRangeAt(0)` builds one on
            // demand from the still-registered RangeId.
            #[cfg(feature = "engine")]
            ObjectKind::Selection => {
                if selection_instance == Some(ObjectId(obj_idx)) {
                    if let Some(bits) = selection_active_range_id_bits {
                        if let Some(&range_wrapper) = range_instances.get(&bits) {
                            mark_object(range_wrapper, obj_marks, work);
                        }
                    }
                }
            }
            // D-14 `#11-file-api` — File API trace fan-outs.  File's
            // bytes live in `vm.blob_data` keyed by the File's own
            // ObjectId (already marked at this point) so File itself
            // has no extra payload to walk.  FileList marks each File
            // in `file_ids`; FileReader marks `target_blob`, `error`,
            // `result.ArrayBuffer(buf_id)`, and the 6 on* handler
            // ObjectIds — without this, a live FileReader whose
            // `r.result` is the only reference to the ArrayBuffer can
            // see it swept while still JS-observable.
            #[cfg(feature = "engine")]
            ObjectKind::File => {}
            #[cfg(feature = "engine")]
            ObjectKind::FileList => {
                if let Some(state) = file_list_data.get(&ObjectId(obj_idx)) {
                    for &file_id in &state.file_ids {
                        mark_object(file_id, obj_marks, work);
                    }
                }
            }
            #[cfg(feature = "engine")]
            ObjectKind::FileReader => {
                if let Some(state) = file_reader_data.get(&ObjectId(obj_idx)) {
                    if let Some(blob) = state.target_blob {
                        mark_object(blob, obj_marks, work);
                    }
                    if let Some(err) = state.error {
                        mark_object(err, obj_marks, work);
                    }
                    if let super::super::host::file_reader::ReaderResult::ArrayBuffer(buf_id) =
                        state.result
                    {
                        mark_object(buf_id, obj_marks, work);
                    }
                    for &handler in state.handlers.values() {
                        mark_object(handler, obj_marks, work);
                    }
                }
            }
            #[cfg(feature = "engine")]
            ObjectKind::WebSocket => {
                if let Some(state) = websocket_states.get(&ObjectId(obj_idx)) {
                    for handler in [state.onopen, state.onmessage, state.onerror, state.onclose]
                        .into_iter()
                        .flatten()
                    {
                        mark_object(handler, obj_marks, work);
                    }
                }
            }
            #[cfg(feature = "engine")]
            ObjectKind::EventSource => {
                if let Some(state) = event_source_states.get(&ObjectId(obj_idx)) {
                    for handler in [state.onopen, state.onmessage, state.onerror]
                        .into_iter()
                        .flatten()
                    {
                        mark_object(handler, obj_marks, work);
                    }
                    for listeners in state.event_listeners.values() {
                        for &listener in listeners {
                            mark_object(listener, obj_marks, work);
                        }
                    }
                }
            }
            // D-16 `#11-wasm-vm` (WASM JS API §5.1) — `WebAssembly.Module`
            // engine-indep `WasmModule` handle holds source bytes
            // (`Arc<[u8]>`) internally; no `ObjectId` references.  Sweep
            // tail prunes `wasm_module_storage` entries whose key was
            // collected.
            #[cfg(feature = "engine")]
            ObjectKind::WasmModule => {}
            // D-16 `#11-wasm-vm` (WASM JS API §5.2) — `WebAssembly.Instance`.
            // Mark `module_id` (always set at ctor time — keeps the parent
            // Module alive) + `exports_id` if `Some` (the cached
            // wrapper-identity-stable exports namespace per
            // `initialize an instance object` step 3; without marking it
            // the namespace + per-export wrappers would be collected
            // even while the Instance is reachable, breaking the
            // `i.exports === i.exports` identity contract).
            #[cfg(feature = "engine")]
            ObjectKind::WasmInstance => {
                if let Some(payload) = wasm_instance_storage.get(&ObjectId(obj_idx)) {
                    mark_object(payload.module_id, obj_marks, work);
                    if let Some(exports_id) = payload.exports_id {
                        mark_object(exports_id, obj_marks, work);
                    }
                }
            }
            // D-16 `#11-wasm-vm` (WASM JS API §5.3) — `WebAssembly.Memory`.
            // Mark `buffer_id` if `Some` (the cached JS `ArrayBuffer`
            // aliasing wasm linear memory must survive while the Memory
            // is reachable so `mem.buffer === mem.buffer` ergonomics
            // hold; IDL has no `[SameObject]`, this is an elidex impl
            // choice motivated by `Object.isFrozen` + identity-across-
            // access patterns).  The stashed `view: WasmMemoryView` is
            // not a JS ObjectId reference (engine-bridge state only), so
            // no mark needed — drop-on-payload-drop is sufficient.
            #[cfg(feature = "engine")]
            ObjectKind::WasmMemory => {
                if let Some(payload) = wasm_memory_storage.get(&ObjectId(obj_idx)) {
                    if let Some(buffer_id) = payload.buffer_id {
                        mark_object(buffer_id, obj_marks, work);
                    }
                }
            }
            // D-16 `#11-wasm-vm` (WASM JS API §5.4 / §5.5) —
            // `WebAssembly.Table` / `WebAssembly.Global`.  No internal
            // `ObjectId` references; element / value reads flow through
            // the engine-bridge handle's internal store.  Sweep tail
            // prunes the matching storage map.
            #[cfg(feature = "engine")]
            ObjectKind::WasmTable | ObjectKind::WasmGlobal => {}
            // D-16 `#11-wasm-vm` (WASM JS API §5.6) — exported function.
            // Mark `instance_id` so the parent `WasmInstance` (and
            // through it the wasm module + linker state) survives for
            // the lifetime of the exported function; the engine-indep
            // `WasmFunc` clone carries its own `WasmStoreHandle` (F1
            // D-ii) so structural shared-store invariants are preserved
            // independently of GC.
            #[cfg(feature = "engine")]
            ObjectKind::WasmExportedFunction => {
                if let Some(payload) = wasm_exported_func_storage.get(&ObjectId(obj_idx)) {
                    mark_object(payload.instance_id, obj_marks, work);
                }
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
