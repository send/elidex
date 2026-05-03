//! `VmInner::collect_garbage` orchestrator — composes the mark /
//! trace / sweep phases into a single GC cycle.
//!
//! Split from [`super`] to keep each phase's file under the
//! 1000-line convention.  The 224-line `proto_roots: [...]` literal
//! stays inline rather than pulled into a helper because every
//! entry reads a `VmInner` field directly and the cfg-gated
//! `None` placeholders are easier to scan top-to-bottom in one
//! place than across a `fn collect_proto_roots(&self)` indirection.

use super::super::VmInner;

#[cfg(feature = "engine")]
use super::bit_get;
use super::roots::{mark_roots, GcRoots};
use super::sweep::{invalidate_ics, sweep_objects, sweep_upvalues};
use super::trace::trace_work_list;
use super::{clear_marks, resize_marks};

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
                // 46 + 2 (PR5-typed-array §C1/C2: %TypedArray% abstract
                // + DataView) = 48.  The 11 concrete subclass
                // prototypes used to live here as cfg-gated slots; SP14
                // moved them into the chained
                // `subclass_array_proto_roots` slice below so adding a
                // 12th subclass is a single `VmInner::subclass_array_prototypes`
                // array bump rather than 22 lines of cfg-gating.
                #[cfg(feature = "engine")]
                self.typed_array_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.data_view_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 48 + 2 (PR5a-fetch2: TextEncoder + TextDecoder) = 50.
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
                // 50 + 2 (M4-12 PR-form-url: URLSearchParams + FormData) = 52.
                // Both chain directly to Object.prototype.  Without
                // marking these intrinsic prototypes here, user code
                // that severs the global binding (e.g. `delete
                // globalThis.URLSearchParams`) could let the
                // prototype be collected while `VmInner::
                // url_search_params_prototype` retains a stale id;
                // the next `new URLSearchParams()` would then bind
                // its instance to a recycled slot of an unrelated
                // type.  Same invariant as every other intrinsic
                // prototype in this list.
                #[cfg(feature = "engine")]
                self.url_search_params_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.form_data_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 52 + 5 (M4-12 PR5-streams: ReadableStream +
                // DefaultReader + DefaultController + 2 queuing
                // strategies) = 57.  All chain to Object.prototype.
                #[cfg(feature = "engine")]
                self.readable_stream_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.readable_stream_default_reader_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.readable_stream_default_controller_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.count_queuing_strategy_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.byte_length_queuing_strategy_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 57 + 1 (M4-12 slot #9.5: URL) = 58.  Chains to
                // `Object.prototype`.  Same invariant as
                // `url_search_params_prototype` above — `delete
                // globalThis.URL` must not let the prototype be
                // collected while `VmInner::url_prototype` retains
                // a stale id.
                #[cfg(feature = "engine")]
                self.url_prototype,
                #[cfg(not(feature = "engine"))]
                None,
            ],
            #[cfg(feature = "engine")]
            subclass_array_proto_roots: &self.subclass_array_prototypes,
            #[cfg(not(feature = "engine"))]
            subclass_array_proto_roots: &[],
            #[cfg(feature = "engine")]
            subclass_array_ctor_roots: &self.subclass_array_ctors,
            #[cfg(not(feature = "engine"))]
            subclass_array_ctor_roots: &[],
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
            form_data_states: &self.form_data_states,
            #[cfg(feature = "engine")]
            readable_stream_states: &self.readable_stream_states,
            #[cfg(feature = "engine")]
            readable_stream_reader_states: &self.readable_stream_reader_states,
            #[cfg(feature = "engine")]
            body_streams: &self.body_streams,
            #[cfg(feature = "engine")]
            url_states: &self.url_states,
            #[cfg(feature = "engine")]
            usp_parent_url: &self.usp_parent_url,
            #[cfg(feature = "engine")]
            pending_timeout_signals: &self.pending_timeout_signals,
            #[cfg(feature = "engine")]
            pending_tasks: &self.pending_tasks,
            #[cfg(feature = "engine")]
            attr_wrapper_cache: &self.attr_wrapper_cache,
            #[cfg(feature = "engine")]
            pending_fetches: &self.pending_fetches,
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
            #[cfg(feature = "engine")]
            roots.form_data_states,
            #[cfg(feature = "engine")]
            roots.readable_stream_states,
            #[cfg(feature = "engine")]
            roots.readable_stream_reader_states,
            #[cfg(feature = "engine")]
            roots.body_streams,
            #[cfg(feature = "engine")]
            roots.url_states,
            #[cfg(feature = "engine")]
            roots.usp_parent_url,
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
            // `disturbed` — companion-Headers pointers were rooted
            // during mark for reachable keys, so surviving entries
            // are intact.  Prune entries whose key was collected to
            // avoid a recycled slot inheriting stale method /
            // status / body bytes (same pattern as
            // `abort_signal_states`).  `body_data` / `disturbed`
            // reach across both Request and Response keys — pruning
            // by the key's mark bit handles both cases in one pass.
            self.request_states.retain(|id, _| bit_get(marks, id.0));
            self.response_states.retain(|id, _| bit_get(marks, id.0));
            self.body_data.retain(|id, _| bit_get(marks, id.0));
            self.disturbed.retain(|id| bit_get(marks, id.0));
            // `readable_stream_states` / `readable_stream_reader_states`
            // — payload references (queue chunks, source callbacks,
            // controller / reader back-refs, pending read promises,
            // closed promise) were marked during the trace phase,
            // so a surviving entry has all its references kept
            // alive.  Drop entries whose key `ObjectId` was
            // collected so a recycled slot can't inherit stale
            // queue / state.
            self.readable_stream_states
                .retain(|id, _| bit_get(marks, id.0));
            self.readable_stream_reader_states
                .retain(|id, _| bit_get(marks, id.0));
            // `body_streams` — entry is removed when the receiver
            // (Request / Response) was collected.  The stream
            // value-side was kept alive during mark via the
            // Request / Response trace fan-out.
            self.body_streams.retain(|id, _| bit_get(marks, id.0));
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
            // `url_search_params_states` — payload is `StringId` only
            // (pool-permanent), so no trace fan-out.  Sweep prunes
            // entries whose key `URLSearchParams` instance was
            // collected.  Same pattern as `headers_states`.
            self.url_search_params_states
                .retain(|id, _| bit_get(marks, id.0));
            // `url_states` — payload is `url::Url` (pool-permanent
            // bytes) + an optional linked `URLSearchParams`
            // `ObjectId` that the trace step has already marked
            // (slot #9.5).  Sweep prunes entries whose key URL
            // instance was collected.
            self.url_states.retain(|id, _| bit_get(marks, id.0));
            // `usp_parent_url` — keys are `URLSearchParams`
            // instances, values are owning `URL` instances.  Drop
            // entries whose key OR value `ObjectId` was collected so
            // the side-table can't pin a pair of recycled slots
            // (the symmetric arms in `trace_work_list` keep the pair
            // marked together while either side is reachable).
            self.usp_parent_url
                .retain(|sp_id, url_id| bit_get(marks, sp_id.0) && bit_get(marks, url_id.0));
            // `form_data_states` — payload includes Blob ObjectIds
            // for `FormDataValue::Blob` entries; those are marked
            // through the `trace_work_list` arm so by sweep time the
            // Blobs are alive whenever the FormData is alive.  Drop
            // entries whose key `FormData` instance was collected
            // (the entry's Blob references are no longer reachable
            // through the FormData wrapper anyway).
            self.form_data_states.retain(|id, _| bit_get(marks, id.0));
            // `attr_wrapper_cache` — drop entries whose wrapper was
            // collected in this sweep.  Owner-wrapper destruction
            // via `remove_wrapper` flows through this prune because
            // the `(e2)` mark-roots fan-out gates Attr marking on
            // owner-wrapper presence.
            self.attr_wrapper_cache
                .retain(|_, attr_id| bit_get(marks, attr_id.0));
            // `fetch_abort_observers` — prune entries whose key
            // `AbortSignal` was collected so a recycled slot can't
            // pick up stale fan-out `FetchId`s.  The values are
            // plain `FetchId(u64)` and carry no GC obligation, so
            // no per-entry filtering is needed.  Same pattern as
            // `abort_signal_states`.
            self.fetch_abort_observers
                .retain(|id, _| bit_get(marks, id.0));
            // `fetch_signal_back_refs` — prune entries whose Signal
            // value was collected.  The reverse-index is consulted
            // by `tick_network` to find the signal that registered
            // the fetch; a dead signal means the abort fan-out can
            // never fire for this fetch, so the entry's only
            // remaining purpose is to occupy a slot.  Keys
            // (`FetchId`) carry no GC obligation; surviving entries
            // are removed explicitly when the broker reply lands.
            // `pending_fetches` is *not* swept here because its
            // values are roots (still live by definition); entries
            // are removed explicitly at settlement / abort fan-out.
            self.fetch_signal_back_refs
                .retain(|_, signal_id| bit_get(marks, signal_id.0));
        }

        // 5. IC invalidation.
        invalidate_ics(&mut self.compiled_functions, &self.gc_object_marks);

        // 6. Reset allocation counter and adjust threshold.
        self.gc_bytes_since_last = 0;
        self.gc_threshold = (live_count * 128).max(32768);
    }
}
