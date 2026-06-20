//! `VmInner::collect_garbage` orchestrator — composes the mark /
//! trace / sweep phases into a single GC cycle.
//!
//! Split from [`super`] to keep each phase's file under the
//! 1000-line convention.  The 224-line `proto_roots: [...]` literal
//! stays inline rather than pulled into a helper because every
//! entry reads a `VmInner` field directly and the cfg-gated
//! `None` placeholders are easier to scan top-to-bottom in one
//! place than across a `fn collect_proto_roots(&self)` indirection.

#[cfg(feature = "engine")]
use super::super::value::ObjectId;
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

        // Snapshot live CSSOM rule_ids (per `<style>` entity) from the
        // bound session so the mark-roots fan-out for the rule-keyed
        // wrapper caches can gate on rule_id liveness — stale rule_ids
        // (deleted via `deleteRule` or reissued after textContent
        // rewrite) get unmarked → swept → pruned by the sweep-tail
        // `retain`.  Without this gate, insertRule/deleteRule cycles
        // would accumulate permanently-pinned cache entries (Copilot
        // R9 finding).  Empty when the VM is unbound.
        #[cfg(feature = "engine")]
        let active_cssom_rule_ids = match self.host_data.as_deref_mut() {
            Some(hd) if hd.is_bound() => hd.session().active_cssom_rule_ids(),
            _ => std::collections::HashMap::new(),
        };
        #[cfg(not(feature = "engine"))]
        let active_cssom_rule_ids: std::collections::HashMap<
            elidex_ecs::Entity,
            std::collections::HashSet<u64>,
        > = std::collections::HashMap::new();

        // 2. Mark phase — split borrow: mark bits are &mut, everything else is &.
        let roots = GcRoots {
            stack: &self.stack,
            frames: &self.frames,
            globals: &self.globals,
            completion_value: self.completion_value,
            saved_completion_stack: &self.saved_completion_stack,
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
                // `MediaQueryList.prototype` (CSSOM-View §4.2) — rooted like
                // every cached interface prototype so a severed
                // `globalThis.MediaQueryList` + GC can't sweep it out from
                // under the `media_query_list_prototype` cache (Codex R1).
                #[cfg(feature = "engine")]
                self.media_query_list_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.media_query_list_event_prototype,
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
                // D-15 #11-shadow-dom-surface — DocumentFragment.prototype +
                // ShadowRoot.prototype + HTMLSlotElement.prototype.  Same
                // invariant as the prototype roots above: without marking,
                // GC would reclaim the prototype slots between register-time
                // and the next wrapper-creation, leaving subsequent
                // `create_element_wrapper` calls binding fresh wrappers to
                // recycled slots of unrelated types (observed during D-15
                // bring-up — sr.prototype showed up as `createElement`
                // function until this fix landed).
                #[cfg(feature = "engine")]
                self.document_fragment_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.shadow_root_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_slot_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 46 + 2 (DOMTokenList + DOMStringMap) = 48.  Both
                // chain directly to Object.prototype.  Without
                // marking, the prototype gets collected while
                // `dom_token_list_prototype` / `dom_string_map_prototype`
                // retain a stale id; the next `el.classList` /
                // `el.dataset` then binds a fresh wrapper to a
                // recycled slot of an unrelated type.  Same invariant
                // as every other intrinsic prototype in this list.
                #[cfg(feature = "engine")]
                self.dom_token_list_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.dom_string_map_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 48 + 2 (PR5-typed-array §C1/C2: %TypedArray% abstract
                // + DataView) = 50.  The 11 concrete subclass
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
                // 50 + 2 (PR5a-fetch2: TextEncoder + TextDecoder) = 52.
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
                // 52 + 2 (M4-12 PR-form-url: URLSearchParams + FormData) = 54.
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
                // 54 + 5 (M4-12 PR5-streams: ReadableStream +
                // DefaultReader + DefaultController + 2 queuing
                // strategies) = 59.  All chain to Object.prototype.
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
                // 59 + 1 (M4-12 slot #9.5: URL) = 60.  Chains to
                // `Object.prototype`.  Same invariant as
                // `url_search_params_prototype` above — `delete
                // globalThis.URL` must not let the prototype be
                // collected while `VmInner::url_prototype` retains
                // a stale id.
                #[cfg(feature = "engine")]
                self.url_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 60 + 3 (M4-12 slot #11a: AnimationEvent /
                // TransitionEvent / CloseEvent) = 63.  All three
                // chain to `Event.prototype` (sibling subclasses of
                // Event, not UIEvent).  Same `delete globalThis.<X>`
                // invariant as the non-UIEvent specialised ctors at
                // [33..37] — `VmInner::<x>_event_prototype` retains a
                // stale id if the prototype is collected behind a
                // severed global binding.
                #[cfg(feature = "engine")]
                self.animation_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.transition_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.close_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 63 + 1 (M4-12 slot #11-mutation-observer:
                // MutationObserver) = 64.  Chains to
                // `Object.prototype`.  Same `delete
                // globalThis.MutationObserver` invariant as the URL
                // prototype above — the
                // `VmInner::mutation_observer_prototype` field
                // retains a stale id otherwise.
                #[cfg(feature = "engine")]
                self.mutation_observer_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 64 + 1 (M4-12 slot #11-storage-web:
                // Storage.prototype, WHATWG HTML §11.2). Chains to
                // `Object.prototype`. Cached `localStorage` /
                // `sessionStorage` instances are rooted via separate
                // slots below.
                #[cfg(feature = "engine")]
                self.storage_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 65 + 1 = 66 (M4-12 slot #11-storage-web:
                // StorageEvent.prototype, WHATWG HTML §11.4.2).
                // Chains to `Event.prototype`.
                #[cfg(feature = "engine")]
                self.storage_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 66 + 1 = 67 — cached `localStorage` Storage
                // instance ([SameObject] semantics).  Stored as an
                // intrinsic root rather than a wrapper-cache entry
                // because it has no owner Entity to weak-anchor on;
                // cleared on `Vm::unbind` to avoid cross-origin
                // leakage.
                #[cfg(feature = "engine")]
                self.storage_local_instance,
                #[cfg(not(feature = "engine"))]
                None,
                // 67 + 1 = 68 — cached `sessionStorage` Storage
                // instance.  Same lifecycle as
                // `storage_local_instance`.
                #[cfg(feature = "engine")]
                self.storage_session_instance,
                #[cfg(not(feature = "engine"))]
                None,
                // 68 + 4 (M4-12 slot #11-crypto-subtle-min:
                // Crypto.prototype + SubtleCrypto.prototype + cached
                // `crypto` singleton + cached `crypto.subtle` singleton).
                // Prototypes follow the same `delete globalThis.<X>`
                // invariant as every other intrinsic prototype above
                // (`VmInner::crypto_prototype` / `subtle_crypto_prototype`
                // retain stale ids otherwise).  Both instance singletons
                // are rooted here rather than via a wrapper-cache entry
                // because, like Storage, they have no owner Entity to
                // weak-anchor on.  Cleared on `Vm::unbind` to avoid
                // cross-bind leakage.
                #[cfg(feature = "engine")]
                self.crypto_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.subtle_crypto_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // `CryptoKey.prototype` (`#11-crypto-subtle-full`).
                // Rooted so retained `CryptoKey` instances keep a live
                // prototype; instances are not singletons, so only the
                // prototype is a root.
                #[cfg(feature = "engine")]
                self.crypto_key_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.crypto_instance,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.subtle_crypto_instance,
                #[cfg(not(feature = "engine"))]
                None,
                // 72 + 2 = 74 (D-17 `#11-custom-elements-vm`):
                // `customElements` singleton prototype + instance. Same
                // rationale as the crypto pair above: retained because
                // `delete globalThis.customElements` must not collect
                // the prototype that retained registered constructors
                // chain to via their own prototype slot.
                #[cfg(feature = "engine")]
                self.custom_element_registry_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.custom_element_registry_instance,
                #[cfg(not(feature = "engine"))]
                None,
                // 74 + 13 = 87 — slot `#11-tags-T1-v2` HTML form-control
                // prototypes (HTML §4.10).  10 per-tag prototypes + 2
                // live-collection prototypes (HTMLFormControlsCollection
                // / HTMLOptionsCollection) + ValidityState.prototype.
                // All chain through HTMLElement.prototype except the
                // collection prototypes (HTMLCollection.prototype) and
                // ValidityState (Object.prototype).  Same `delete
                // globalThis.<X>` invariant as every other intrinsic
                // prototype above — the matching `VmInner::<x>_prototype`
                // slot retains a stale id otherwise.
                #[cfg(feature = "engine")]
                self.html_label_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_optgroup_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_legend_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_option_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_fieldset_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_form_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_button_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_textarea_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_select_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_input_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_form_controls_collection_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_options_collection_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.validity_state_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 87 + 1 = 88 (M4-12 slot #11-style-declaration:
                // CSSStyleDeclaration.prototype, CSSOM §6.6).  Chains
                // to `Object.prototype`.  Same `delete globalThis.<X>`
                // invariant as every other intrinsic prototype above
                // — `VmInner::css_style_declaration_prototype` retains
                // a stale id if the prototype is collected behind a
                // severed global binding.  PR-A only ships Inline /
                // Computed sources; PR-B's Rule source shares the
                // same prototype, so the entry covers both.
                #[cfg(feature = "engine")]
                self.css_style_declaration_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 88 + 4 = 92 (M4-12 slot #11-style-declaration PR-B:
                // CSSStyleSheet / CSSRuleList / CSSStyleRule /
                // StyleSheetList prototypes).  Each chains to
                // `Object.prototype`.  Without these entries the
                // freshly-allocated prototype objects can be collected
                // before the first JS access reaches them through the
                // wrapper chain (the wrapper's `prototype: Some(id)`
                // is not a strong root by itself when the wrapper
                // itself isn't yet referenced).
                #[cfg(feature = "engine")]
                self.css_stylesheet_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.css_rule_list_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.css_style_rule_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.style_sheet_list_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 92 + 5 = 97 (M4-12 slot #11-tags-T2a-url-bearing:
                // HTMLAnchorElement / HTMLAreaElement /
                // HTMLImageElement / HTMLScriptElement /
                // HTMLLinkElement prototypes).  Each chains to
                // `HTMLElement.prototype`.
                #[cfg(feature = "engine")]
                self.html_anchor_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_area_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_image_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_script_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_link_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // +3 (M4-12 slot #11-canvas-2d-vm: HTMLCanvasElement /
                // CanvasRenderingContext2D / ImageData prototypes).
                // HTMLCanvasElement.prototype is looked up per canvas-
                // wrapper creation, the context prototype on every
                // getContext, and the ImageData prototype on every
                // getImageData / createImageData / `new ImageData` — so
                // each is read at arbitrary times and must stay rooted
                // here (same `delete globalThis.<X>` invariant as every
                // other intrinsic prototype: the matching
                // `VmInner::<x>_prototype` field retains a stale id
                // otherwise).
                #[cfg(feature = "engine")]
                self.html_canvas_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.canvas_rendering_context_2d_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.image_data_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.offscreen_canvas_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.offscreen_canvas_rendering_context_2d_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 97 + 24 = 121 (M4-12 slot #11-tags-T2b-passive:
                // 7 head + 17 grouping prototypes — h1-h6 share one
                // HTMLHeadingElement prototype and blockquote+q
                // share one HTMLQuoteElement prototype, so the field
                // count is 24 rather than 25).  All chain to
                // `HTMLElement.prototype`.
                #[cfg(feature = "engine")]
                self.html_html_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_head_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_body_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_title_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_base_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_meta_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_style_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_div_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_span_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_br_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_hr_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_pre_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_p_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_heading_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_quote_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_olist_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_ulist_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_li_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_dlist_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_menu_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_map_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_picture_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_data_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_time_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 121 + 6 = 127 (M4-12 slot #11-tags-T2c-table:
                // HTMLTableElement / HTMLTableSectionElement (shared
                // thead/tbody/tfoot) / HTMLTableRowElement /
                // HTMLTableCellElement (shared td/th) /
                // HTMLTableCaptionElement / HTMLTableColElement
                // (shared col/colgroup) prototypes).  Each chains to
                // `HTMLElement.prototype`.
                #[cfg(feature = "engine")]
                self.html_table_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_table_section_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_table_row_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_table_cell_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_table_caption_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_table_col_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 127 + 7 = 134 (slot `#11-tags-T2d-interactive`:
                // HTMLDialogElement / HTMLDetailsElement /
                // HTMLTemplateElement / HTMLDataListElement /
                // HTMLOutputElement / HTMLProgressElement /
                // HTMLMeterElement) = 128.  Each chains to
                // `HTMLElement.prototype`.
                #[cfg(feature = "engine")]
                self.html_dialog_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_details_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_template_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_datalist_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_output_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_progress_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.html_meter_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 134 + 10 = 144 (M4-12 slot `#11-events-misc`:
                // SubmitEvent / FormDataEvent / ToggleEvent /
                // CompositionEvent / ClipboardEvent / ProgressEvent /
                // BeforeUnloadEvent / MessageEvent / WheelEvent /
                // PageTransitionEvent prototypes).  Same `delete
                // globalThis.<X>` invariant as every other intrinsic
                // prototype in this list — the matching
                // `VmInner::<x>_event_prototype` field retains a stale
                // id otherwise.
                #[cfg(feature = "engine")]
                self.submit_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.formdata_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.toggle_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.composition_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.clipboard_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.progress_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.before_unload_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.message_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.wheel_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.page_transition_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // D-9 events-modern-input (slot
                // `#11-events-modern-input`).  144 + 8 = 152.  Eight
                // new prototypes — PointerEvent / DragEvent / Touch /
                // TouchList / TouchEvent / DataTransfer /
                // DataTransferItem / DataTransferItemList.  Without
                // marking, a freshly-allocated wrapper after GC could
                // bind a recycled slot of an unrelated type.
                #[cfg(feature = "engine")]
                self.pointer_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.drag_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.touch_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.touch_list_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.touch_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.data_transfer_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.data_transfer_item_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.data_transfer_item_list_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // M4-12 slot #11-traversal-and-range-pr-a2-bindings:
                // Range / StaticRange / TreeWalker / NodeIterator
                // prototypes.  Each chains to `Object.prototype`.
                // Without these entries, `delete globalThis.Range`
                // (etc.) can let the prototype be swept while
                // `VmInner::range_prototype` (etc.) retains a stale
                // id — the next `cloneRange()` /
                // `document.createTreeWalker()` would bind its
                // wrapper to a recycled slot of an unrelated type.
                // Copilot R14: NodeFilter is a constants-namespace
                // object, not a constructable interface, so no
                // matching `node_filter_prototype` field exists.
                #[cfg(feature = "engine")]
                self.range_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.static_range_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.tree_walker_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.node_iterator_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 156 + 1 (D-8 PR-B `#11-traversal-and-range-pr-b-selection`:
                // `Selection.prototype`) = 157.  Per-document singleton
                // Selection is reached through this prototype slot;
                // the wrapper itself is held in
                // `HostData::selection_instance` and traced via
                // `vm/gc/trace.rs::trace_selection_instance` (which
                // fans out to the current Range wrapper at
                // `range_instances[range_id.bits()]`).
                #[cfg(feature = "engine")]
                self.selection_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // 157 + 2 = 159 (slot `#11-net-ws-sse`:
                // `WebSocket.prototype` + `EventSource.prototype`).
                // Each chains to `EventTarget.prototype` (since
                // `#11-realtime-event-listeners`, so `addEventListener`
                // / `removeEventListener` / `dispatchEvent` are
                // inherited).  Per-instance listeners (on* handlers +
                // every `addEventListener` registration) live in the
                // unified `VmInner::vm_event_listeners` home, rooted via
                // `HostData::listener_store`; the prototypes themselves
                // are rooted here because the matching
                // `VmInner::<x>_prototype` slot retains a stale id
                // otherwise (same `delete globalThis.WebSocket`
                // invariant as every other intrinsic prototype above).
                #[cfg(feature = "engine")]
                self.websocket_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.event_source_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // M4-12 slot #11-dom-rect-readonly: DOMRectReadOnly /
                // DOMRect prototypes (W3C Geometry §3).  Both chain to
                // `Object.prototype`.  Rooted here because the
                // `VmInner::dom_rect_*_prototype` slots retain a stale
                // id if the prototype is collected behind a severed
                // global binding (same `delete globalThis.DOMRect`
                // invariant as every other intrinsic prototype above) —
                // load-bearing once a host-side allocator (D-22) mints
                // DOMRects without a live global ctor.
                #[cfg(feature = "engine")]
                self.dom_rect_readonly_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.dom_rect_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // M4-12 slot #11-resize-observer-vm: ResizeObserver.prototype
                // (W3C Resize Observer §2.1).  Chains to `Object.prototype`.
                // Same `delete globalThis.ResizeObserver` invariant as
                // every other intrinsic prototype above — the
                // `VmInner::resize_observer_prototype` slot retains a
                // stale id if the prototype is collected behind a
                // severed global binding.
                #[cfg(feature = "engine")]
                self.resize_observer_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // M4-12 slot #11-intersection-observer-vm:
                // IntersectionObserver.prototype (W3C Intersection
                // Observer §2.2).  Same rationale as
                // `resize_observer_prototype` above.
                #[cfg(feature = "engine")]
                self.intersection_observer_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // M4-12 slot #11-indexed-db-vm: the 11 IndexedDB interface
                // prototypes are cached in `VmInner` and reused for every
                // host-created IDB object, so they must be rooted here —
                // otherwise deleting/severing a global IDB constructor could
                // let its cached prototype be swept while `VmInner` still hands
                // out its (now-recycled) `ObjectId` to later
                // `indexedDB.open()` results.
                #[cfg(feature = "engine")]
                self.idb_factory_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.idb_request_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.idb_open_db_request_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.idb_database_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.idb_object_store_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.idb_transaction_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.idb_key_range_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.idb_index_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.idb_cursor_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.idb_cursor_with_value_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.idb_version_change_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // Cache API (D-19 PR-1): root the cached `CacheStorage` /
                // `Cache` interface prototypes so `delete globalThis.caches`
                // / `delete globalThis.Cache` cannot let them be swept while
                // `VmInner::cache_*_prototype` still hands the (recycled)
                // `ObjectId` to the next `caches.open()` — same defensive
                // invariant as the IDB / intrinsic prototype slots above.
                #[cfg(feature = "engine")]
                self.cache_storage_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.cache_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                // Service Worker realm (D-19 PR-2): root the SW scope + event
                // (`ExtendableEvent`/`FetchEvent`) + `Clients`/`Client`
                // interface prototypes so `delete globalThis.FetchEvent` etc.
                // cannot sweep a prototype while `VmInner` still hands its
                // (recycled) `ObjectId` to the next UA-built SW event/client.
                #[cfg(feature = "engine")]
                self.service_worker_scope_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.extendable_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.fetch_event_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.clients_prototype,
                #[cfg(not(feature = "engine"))]
                None,
                #[cfg(feature = "engine")]
                self.client_prototype,
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
            html_element_constructor: self.html_element_constructor,
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
            data_transfer_states: &self.data_transfer_states,
            #[cfg(feature = "engine")]
            touch_states: &self.touch_states,
            #[cfg(feature = "engine")]
            touch_list_states: &self.touch_list_states,
            #[cfg(feature = "engine")]
            pending_timeout_signals: &self.pending_timeout_signals,
            #[cfg(feature = "engine")]
            pending_tasks: &self.pending_tasks,
            #[cfg(feature = "engine")]
            active_cssom_rule_ids: &active_cssom_rule_ids,
            #[cfg(feature = "engine")]
            pending_fetches: &self.pending_fetches,
            #[cfg(feature = "engine")]
            idb_transaction_states: &self.idb_transaction_states,
            #[cfg(feature = "engine")]
            dispatched_events: &self.dispatched_events,
        };

        self.gc_work_list.clear();

        mark_roots(
            &roots,
            &mut self.gc_object_marks,
            &mut self.gc_upvalue_marks,
            &mut self.gc_work_list,
        );

        // Service Worker realm (D-19 PR-2): root the in-flight `respondWith` /
        // `waitUntil` promises.  They live ONLY in these side-stores during a
        // single event's dispatch+pump window (`sw_thread::run_fetch` /
        // `run_lifecycle` remove the entry afterward), and the
        // `respondWith`/`waitUntil` native returns control to the listener
        // body (which runs with GC enabled) before the SW loop reads them
        // back — so a GC mid-listener would otherwise sweep a promise still
        // owed to the loop.  A no-op (empty maps) outside that window.  The
        // event objects themselves are `ObjectKind::Event`, rooted via the
        // operand stack + `dispatched_events`; only their side-store promises
        // need this explicit root.
        #[cfg(feature = "engine")]
        {
            let marks = &mut self.gc_object_marks;
            let work = &mut self.gc_work_list;
            for state in self.fetch_event_states.values() {
                if let Some(promise) = state.response_promise {
                    super::mark_object(promise, marks, work);
                }
            }
            for state in self.extendable_event_states.values() {
                for &promise in &state.lifetime_promises {
                    super::mark_object(promise, marks, work);
                }
            }
            // `navigator.serviceWorker` client (D-19 PR-3) — a pending
            // `register()` / `unregister()` / `ready` promise is reachable
            // ONLY through these side-stores until it settles, so force-mark
            // them (the `pending_fetches` discipline; never value-swept,
            // drained on settle).
            for promises in self.pending_registration_promises.values() {
                for &promise in promises {
                    super::mark_object(promise, marks, work);
                }
            }
            for promises in self.pending_unregister_promises.values() {
                for &promise in promises {
                    super::mark_object(promise, marks, work);
                }
            }
            if let Some(promise) = self.sw_ready_promise {
                super::mark_object(promise, marks, work);
            }
            // Live SW registrations keep their interned `ServiceWorkerRegistration`
            // / `ServiceWorker` wrappers alive (so identity survives GC): the
            // seam mark loop deliberately SKIPS the `Scope`-owned wrappers
            // (`NoProactiveMark`), so walk the registry and mark each live
            // (scope, kind) wrapper directly via the wrapper store (R2-3; the
            // `fetch_event_states` precedent above).  `get_wrapper` is a `&self`
            // method that would conflict with the `marks`/`work` borrows, so the
            // store is read field-wise.
            if let Some(hd) = self.host_data.as_deref() {
                for entry in self.sw_registrations.values() {
                    for kind in [
                        super::super::wrapper_intern::WrapperKind::ServiceWorkerRegistration,
                        super::super::wrapper_intern::WrapperKind::ServiceWorker,
                    ] {
                        let key =
                            super::super::wrapper_intern::WrapperKey::scope(entry.scope_sid, kind);
                        if let Some(&id) = hd.wrapper_store.get(&key) {
                            super::mark_object(id, marks, work);
                        }
                    }
                }
            }
        }

        // Copilot R8: empty fallbacks when HostData is absent — the
        // TreeWalker / NodeIterator trace fan-out arms can never
        // fire in that case (instances are stored only in HostData),
        // so the empty maps are observably dead.
        #[cfg(feature = "engine")]
        let empty_tree_walker_states = std::collections::HashMap::new();
        #[cfg(feature = "engine")]
        let empty_walker_instances: std::collections::HashMap<u64, ObjectId> =
            std::collections::HashMap::new();
        #[cfg(feature = "engine")]
        let empty_iter_shared: std::sync::Arc<
            std::sync::Mutex<std::collections::HashMap<u64, elidex_dom_api::NodeIteratorState>>,
        > = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
        #[cfg(feature = "engine")]
        let empty_iter_instances: std::collections::HashMap<u64, ObjectId> =
            std::collections::HashMap::new();

        #[cfg(feature = "engine")]
        let (tw_states, tw_instances, iter_shared, iter_instances) = match self.host_data.as_deref()
        {
            Some(hd) => (
                hd.tree_walker_states_ref(),
                hd.tree_walker_instances_ref(),
                hd.node_iterator_states_shared_ref(),
                hd.node_iterator_instances_ref(),
            ),
            None => (
                &empty_tree_walker_states,
                &empty_walker_instances,
                &empty_iter_shared,
                &empty_iter_instances,
            ),
        };

        // D-8 PR-B Selection fan-out: pass the Selection wrapper id +
        // active RangeId.bits + range_instances cache so the trace
        // can mark the cached Range wrapper when the Selection itself
        // is reachable.  Empty maps + None fallbacks when HostData is
        // unbound — the Selection trace arm short-circuits on
        // wrapper-id mismatch.
        #[cfg(feature = "engine")]
        let empty_range_instances: std::collections::HashMap<u64, ObjectId> =
            std::collections::HashMap::new();
        #[cfg(feature = "engine")]
        let (selection_instance, selection_active_range_id_bits, range_instances_ref) =
            match self.host_data.as_deref() {
                Some(hd) => (
                    hd.selection_instance_id(),
                    hd.selection_active_range_id_bits(),
                    hd.range_instances_ref(),
                ),
                None => (None, None, &empty_range_instances),
            };

        // D-12 `#11-net-ws-sse` — WebSocket / EventSource handler

        // `#11-wrapper-identity-seam` — the `<input>.files` FileList
        // `[SameObject]` cache is now interned in `hd.wrapper_store`
        // (keyed by `WrapperKey::object(input_id, FileList)`).  The
        // `<input>` `HostObject` trace arm marks the cached FileList
        // through this store (`MarkAgent::ViaOwnerTrace`).  Empty
        // fallback when HostData is unbound — no input wrapper can
        // exist then.
        #[cfg(feature = "engine")]
        let empty_wrapper_store: std::collections::HashMap<
            super::super::wrapper_intern::WrapperKey,
            ObjectId,
        > = std::collections::HashMap::new();
        #[cfg(feature = "engine")]
        let wrapper_store_ref = match self.host_data.as_deref() {
            Some(hd) => &hd.wrapper_store,
            None => &empty_wrapper_store,
        };

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
            #[cfg(feature = "engine")]
            roots.data_transfer_states,
            #[cfg(feature = "engine")]
            roots.touch_states,
            #[cfg(feature = "engine")]
            roots.touch_list_states,
            #[cfg(feature = "engine")]
            tw_states,
            #[cfg(feature = "engine")]
            tw_instances,
            #[cfg(feature = "engine")]
            iter_shared,
            #[cfg(feature = "engine")]
            iter_instances,
            #[cfg(feature = "engine")]
            selection_instance,
            #[cfg(feature = "engine")]
            selection_active_range_id_bits,
            #[cfg(feature = "engine")]
            range_instances_ref,
            #[cfg(feature = "engine")]
            &self.file_list_data,
            #[cfg(feature = "engine")]
            &self.file_reader_data,
            #[cfg(feature = "engine")]
            wrapper_store_ref,
            #[cfg(feature = "engine")]
            &self.wasm_instance_storage,
            #[cfg(feature = "engine")]
            &self.wasm_memory_storage,
            #[cfg(feature = "engine")]
            &self.wasm_exported_func_storage,
            #[cfg(feature = "engine")]
            &self.wasm_backed_buffers,
            #[cfg(feature = "engine")]
            &self.idb_request_states,
            #[cfg(feature = "engine")]
            &self.idb_transaction_states,
            #[cfg(feature = "engine")]
            &self.idb_object_store_states,
            #[cfg(feature = "engine")]
            &self.idb_index_states,
            #[cfg(feature = "engine")]
            &self.idb_cursor_states,
            #[cfg(feature = "engine")]
            &self.crypto_key_js_cache,
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
            // Live `MediaQueryList` registry (CSSOM-View §4.2): prune
            // entries whose MQL `ObjectId` was collected so a recycled
            // slot never inherits a stale entry (the `seq` field also
            // guards this in `deliver_media_query_changes`, but a dropped
            // MQL still has its entry pruned here).  The value is
            // `ObjectId`/`JsValue`-free, so there is no trace
            // pass — this sweep-prune is the ONLY GC delete-path and it
            // balances the `matchMedia` insert write-path.
            self.media_query_list_registry
                .retain(|id, _| bit_get(marks, id.0));
            // DOMException out-of-band state: prune entries whose
            // instance was collected so a recycled slot can't
            // inherit stale `name` / `message`.  Payload is
            // `StringId` pairs (pool-permanent) — no trace pass
            // needed during mark, only this post-sweep GC.
            self.dom_exception_states
                .retain(|id, _| bit_get(marks, id.0));
            // `CryptoKey` side table (`#11-crypto-subtle-full`).  Prune
            // is a CORRECTNESS invariant, not just hygiene: `ObjectId`
            // slots are reused (`alloc_object` free-list), so a stale
            // entry left after collection would bind another wrapper's
            // key material.  The cached `algorithm` / `usages` wrappers
            // (`crypto_key_js_cache`) are traced via the
            // `ObjectKind::CryptoKey` arm (so they survive while the key
            // is reachable) and pruned here with the same key.
            self.crypto_key_states.retain(|id, _| bit_get(marks, id.0));
            self.crypto_key_js_cache
                .retain(|id, _| bit_get(marks, id.0));
            // DOMRect value-type side table (GC contract on the field doc).
            self.dom_rect_states.retain(|id, _| bit_get(marks, id.0));
            // BeforeUnloadEvent.returnValue side table — pool-permanent
            // StringId payload (no trace step), but the key ObjectId
            // entry must be pruned when the event instance is collected
            // so a recycled slot can't observe a stale string written
            // by a previous BeforeUnloadEvent.
            self.before_unload_return_values
                .retain(|id, _| bit_get(marks, id.0));
            // D-9 events-modern-input — sweep tail prunes the three
            // new side tables + the DataTransferItem identity cache.
            self.data_transfer_states
                .retain(|id, _| bit_get(marks, id.0));
            self.touch_states.retain(|id, _| bit_get(marks, id.0));
            self.touch_list_states.retain(|id, _| bit_get(marks, id.0));
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
            // `detached_buffers` — same shape as `disturbed`: each
            // entry is an `ObjectId` of an `ArrayBuffer` whose
            // `[[ArrayBufferData]]` was nulled per ECMA-262 §25.1.3.5
            // `DetachArrayBuffer`.  Prune entries whose key
            // `ObjectId` was collected so a recycled slot can't
            // inherit a stale detach flag (which would surface
            // spec-divergent TypeError on a freshly-allocated buffer).
            self.detached_buffers.retain(|id| bit_get(marks, id.0));
            // D-16 `#11-wasm-vm` — sweep tail for the 6 WebAssembly
            // side-store maps + the `wasm_backed_buffers` reverse-lookup.
            // Standard prune-by-key-mark contract: ObjectId-keyed side
            // tables drop entries whose key wrapper was collected so a
            // recycled `ObjectId` slot can't inherit stale state
            // (`buffer_id` / `exports_id` / `module_id` / `instance_id`
            // / `params` / `element_kind` / engine-bridge handles).
            // Trace-step fan-out marks `module_id` + `exports_id` +
            // `buffer_id` + `instance_id` while the parent wrapper is
            // reachable, so surviving entries have all references kept
            // alive.  `wasm_backed_buffers` is keyed by the ArrayBuffer
            // wrapper ObjectId; drop entries whose ArrayBuffer was
            // collected (the matching `WasmMemoryPayload.buffer_id` /
            // `view` are cleared at detach time elsewhere — sweep here
            // only handles GC-induced collection of the ArrayBuffer
            // wrapper).
            self.wasm_module_storage
                .retain(|id, _| bit_get(marks, id.0));
            self.wasm_instance_storage
                .retain(|id, _| bit_get(marks, id.0));
            self.wasm_memory_storage
                .retain(|id, _| bit_get(marks, id.0));
            self.wasm_table_storage.retain(|id, _| bit_get(marks, id.0));
            self.wasm_global_storage
                .retain(|id, _| bit_get(marks, id.0));
            self.wasm_exported_func_storage
                .retain(|id, _| bit_get(marks, id.0));
            self.wasm_backed_buffers
                .retain(|ab_id, _| bit_get(marks, ab_id.0));
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
            // `#11-wrapper-identity-seam` — ONE sweep-retain over the
            // unified wrapper-identity store, dispatched by
            // [`WrapperKind::retain`], replacing the ~24 per-cache
            // `.retain` sites this consolidated.  The prune predicates
            // are faithful to the originals:
            //
            // - `NeverSweep` (the primary `Node` wrapper): kept here —
            //   node wrappers are pruned only via `remove_wrapper`
            //   (despawn), never value-swept.
            // - `ValueMark` (classList / dataset / Attr / CSSOM /
            //   collection secondaries): drop iff the wrapper
            //   `ObjectId` value was collected this sweep.  Owner
            //   destruction flows through this prune because the
            //   weak-through-owner mark gate left the value unmarked.
            // - `ValueAndOwnerMark` (`<input>.files` FileList /
            //   `DataTransferItem`): drop iff the value OR its owning
            //   wrapper `ObjectId` (recoverable from `key.owner`) was
            //   collected — the two-predicate prune that keeps a
            //   recycled owner slot from inheriting a stale entry.
            //
            // Borrow note: `marks` borrows `self.gc_object_marks`,
            // `host_data` is a disjoint field, so the `&mut` store
            // borrow does not alias.
            #[cfg(feature = "engine")]
            if let Some(hd) = self.host_data.as_deref_mut() {
                hd.wrapper_store.retain(|key, value| {
                    match key.kind.retain() {
                        super::super::wrapper_intern::RetainPredicate::NeverSweep => true,
                        super::super::wrapper_intern::RetainPredicate::ValueMark => {
                            bit_get(marks, value.0)
                        }
                        super::super::wrapper_intern::RetainPredicate::ValueAndOwnerMark => {
                            let owner_live = match key.owner {
                                super::super::wrapper_intern::WrapperOwner::Object(owner_id) => {
                                    bit_get(marks, owner_id.0)
                                }
                                // No `ValueAndOwnerMark` kind is entity- or
                                // scope-owned today (only FileList /
                                // DataTransferItem, both Object-owned); if one
                                // is added, fall back to value-only liveness.
                                super::super::wrapper_intern::WrapperOwner::Entity(_)
                                | super::super::wrapper_intern::WrapperOwner::Scope(_) => true,
                            };
                            bit_get(marks, value.0) && owner_live
                        }
                    }
                });
            }
            // `file_data` / `file_list_data` / `file_reader_data` —
            // payload side-tables keyed by the instance's own
            // ObjectId.  Standard prune-by-key-mark contract.  Phase
            // 4 (FileReader) extends `file_reader_data` GC trace to
            // mark `target_blob` + ArrayBuffer / error referenced from
            // `result` / `error`; until then the read pipeline is
            // inert so no fan-out is missed.
            self.file_data.retain(|id, _| bit_get(marks, id.0));
            self.file_list_data.retain(|id, _| bit_get(marks, id.0));
            self.file_reader_data.retain(|id, _| bit_get(marks, id.0));
            // IndexedDB side-stores (D-20) — prune entries whose key
            // wrapper `ObjectId` was collected.  Standard prune-by-key-mark
            // contract; the trace step (gc/trace.rs IDB arms) marks the
            // handler / listener / result / source / transaction fan-out
            // so live entries survive.  A pruned transaction's `backend_txn`
            // has NO `Drop` rollback (the backend exposes only an explicit
            // `abort`), so roll back any still-open handle before dropping it
            // — leaving the shared SQLite connection mid-transaction would
            // block later IDB ops.  Reachable Active/Committing transactions
            // are kept alive by the auto-commit sweep / pending tasks, so this
            // normally finds nothing; it enforces the invariant by
            // construction rather than relying on collection order (mirrors
            // the `Vm::unbind` rollback).
            if let Some(backend) = self.idb_backend.clone() {
                for (id, st) in &mut self.idb_transaction_states {
                    if !bit_get(marks, id.0) {
                        if let Some(mut txn) = st.backend_txn.take() {
                            let _ = txn.abort(backend.conn());
                        }
                    }
                }
            }
            self.idb_request_states.retain(|id, _| bit_get(marks, id.0));
            self.idb_transaction_states
                .retain(|id, _| bit_get(marks, id.0));
            self.idb_database_states
                .retain(|id, _| bit_get(marks, id.0));
            self.idb_object_store_states
                .retain(|id, _| bit_get(marks, id.0));
            self.idb_key_range_states
                .retain(|id, _| bit_get(marks, id.0));
            self.idb_index_states.retain(|id, _| bit_get(marks, id.0));
            self.idb_cursor_states.retain(|id, _| bit_get(marks, id.0));
            // Cache API (D-19 PR-1): prune `Cache` handle-name tuples whose
            // wrapper `ObjectId` was collected (mirrors the IDB side-stores
            // / `headers_states`).
            self.cache_handle_states
                .retain(|id, _| bit_get(marks, id.0));
            // Service Worker realm (D-19 PR-2): prune the FetchEvent /
            // ExtendableEvent / Client side-stores whose `ObjectId` was
            // collected.  The events live only across one dispatch+pump (the
            // loop removes them explicitly), but a GC mid-pump must not orphan
            // a stale key onto a recycled id — same invariant as above.
            self.fetch_event_states.retain(|id, _| bit_get(marks, id.0));
            self.extendable_event_states
                .retain(|id, _| bit_get(marks, id.0));
            self.client_states.retain(|id, _| bit_get(marks, id.0));
            // `navigator.serviceWorker` client (D-19 PR-3) — prune the
            // `ServiceWorkerRegistration` / `ServiceWorker` brand side-stores
            // whose wrapper `ObjectId` was collected (the registry-walk mark
            // above keeps live ones marked; a JS-held redundant worker survives
            // by its own reachability).
            self.sw_registration_states
                .retain(|id, _| bit_get(marks, id.0));
            self.service_worker_states
                .retain(|id, _| bit_get(marks, id.0));
            // `vm_event_listeners` — the unified listener home for the
            // non-entity EventTargets (AbortSignal / IDB / WebSocket /
            // EventSource).  When a target `ObjectId` was collected, prune
            // its entry AND retire
            // each of its `ListenerId`s from `HostData::listener_store` (the
            // GC root) + `abort_listener_back_refs` in lockstep — else the
            // dead target's callbacks stay rooted forever (leak).  Collect
            // the dead listener ids first (immutable borrow), then prune the
            // map, then retire (the retirement helper re-borrows `self`).
            let dead_vm_listener_ids: Vec<elidex_script_session::ListenerId> = self
                .vm_event_listeners
                .iter()
                .filter(|(id, _)| !bit_get(marks, id.0))
                .flat_map(|(_, listeners)| listeners.ids())
                .collect();
            self.vm_event_listeners.retain(|id, _| bit_get(marks, id.0));
            // Retire each dead-target listener from `listener_store` +
            // `abort_listener_back_refs` (inlined from
            // `remove_listener_and_prune_back_ref`, which takes `&mut self`
            // and would conflict with the live `marks` borrow of
            // `self.gc_object_marks`; the fields touched here are disjoint).
            for listener_id in dead_vm_listener_ids {
                if let Some(host) = self.host_data.as_deref_mut() {
                    host.remove_listener(listener_id);
                }
                if let Some(signal_id) = self.abort_listener_back_refs.remove(&listener_id) {
                    if let Some(state) = self.abort_signal_states.get_mut(&signal_id) {
                        state.bound_listener_removals.remove(&listener_id);
                    }
                }
            }
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

            // D-8 PR-A2 — Range / TreeWalker / NodeIterator instance
            // side-tables.  `range_instances` keys are `RangeId` bits
            // (NOT ObjectIds) so the prune predicate inspects the
            // VALUE wrapper's mark bit; for each dead wrapper, also
            // unregister the corresponding RangeId from
            // `LiveRangeRegistry` to release the engine-side Range.
            // TreeWalker / NodeIterator follow the same pattern but
            // additionally drop the state-table entry (no separate
            // engine-side registry).
            //
            // `mutation_observer_callbacks` / `_instances` use the
            // same per-observer-id pattern but the callback ObjectId
            // is unconditionally rooted via `gc_root_object_ids`,
            // making this prune the only place where the side-table
            // can shed entries.  TreeWalker / NodeIterator filter
            // ObjectIds (Copilot R8) are NOT rooted via
            // `gc_root_object_ids` — they reach GC only via the
            // per-wrapper trace fan-out in `vm/gc/trace.rs`.  This
            // sweep prunes the state-table entry once the wrapper
            // ObjectId itself is unmarked, mirroring the
            // mutation_observer pattern except for the rooting side.
            if let Some(hd) = self.host_data.as_deref_mut() {
                // Range — collect dead range_ids first to avoid
                // double-borrowing `host_data` (we need both
                // `range_instances` for filtering AND
                // `live_range_registry` for unregister).
                let dead_range_ids: Vec<u64> = hd
                    .range_instances
                    .iter()
                    .filter_map(|(range_id, obj_id)| {
                        if bit_get(marks, obj_id.0) {
                            None
                        } else {
                            Some(*range_id)
                        }
                    })
                    .collect();
                for rid in &dead_range_ids {
                    hd.range_instances.remove(rid);
                    hd.live_range_registry
                        .unregister(elidex_dom_api::RangeId(*rid));
                }
                // TreeWalker — prune instances + states by dead wrapper.
                let dead_walker_ids: Vec<u64> = hd
                    .tree_walker_instances
                    .iter()
                    .filter_map(|(wid, oid)| {
                        if bit_get(marks, oid.0) {
                            None
                        } else {
                            Some(*wid)
                        }
                    })
                    .collect();
                for wid in &dead_walker_ids {
                    hd.tree_walker_instances.remove(wid);
                    hd.tree_walker_states.remove(wid);
                }
                // NodeIterator — same pattern; `node_iterator_states_shared`
                // is the shared `Arc<Mutex<HashMap>>` (held jointly with
                // the bridge), so take the lock briefly.
                let dead_iter_ids: Vec<u64> = hd
                    .node_iterator_instances
                    .iter()
                    .filter_map(|(iid, oid)| {
                        if bit_get(marks, oid.0) {
                            None
                        } else {
                            Some(*iid)
                        }
                    })
                    .collect();
                for iid in &dead_iter_ids {
                    hd.node_iterator_instances.remove(iid);
                    if let Ok(mut guard) = hd.node_iterator_states_shared.lock() {
                        guard.remove(iid);
                    }
                }
                // Selection — per-document singleton wrapper.  When
                // the Selection wrapper's mark bit is clear, the
                // `ObjectId` slot will be reused on the next
                // allocation; if we don't clear `selection_instance`
                // here, the next `window.getSelection()` /
                // `document.getSelection()` returns the stale id
                // (now pointing at an unrelated object), breaking
                // the `Selection` brand check.  Copilot R2 IMP.
                if let Some(sel_id) = hd.selection_instance {
                    if !bit_get(marks, sel_id.0) {
                        hd.selection_instance = None;
                    }
                }
                // D-12 `#11-net-ws-sse` — WebSocket / EventSource
                // side-table prune.  Standard mark-the-key contract
                // PLUS emit a `WebSocketClose(conn_id)` /
                // `EventSourceClose(conn_id)` per swept instance so
                // the broker terminates the underlying I/O thread
                // before the renderer-side state vanishes.  Without
                // this, a GC-discarded WebSocket would leak its
                // network thread (the broker keeps the worker alive
                // until JS code explicitly closes — see CLAUDE.md
                // "後方互換性は維持しない": no implicit cleanup is
                // an explicit design choice, GC sweep IS the
                // explicit close for orphaned wrappers).
                //
                // Reverse maps are pruned by VALUE (`ObjectId`)
                // mark bit, mirroring the conn → instance routing
                // direction.
                let dead_ws_conns: Vec<u64> = hd
                    .websocket_states
                    .iter()
                    .filter_map(|(obj_id, state)| {
                        if bit_get(marks, obj_id.0) {
                            None
                        } else {
                            Some(state.conn_id)
                        }
                    })
                    .collect();
                hd.websocket_states.retain(|id, _| bit_get(marks, id.0));
                hd.ws_conn_to_object
                    .retain(|_, obj_id| bit_get(marks, obj_id.0));
                let dead_sse_conns: Vec<u64> = hd
                    .event_source_states
                    .iter()
                    .filter_map(|(obj_id, state)| {
                        if bit_get(marks, obj_id.0) {
                            None
                        } else {
                            Some(state.conn_id)
                        }
                    })
                    .collect();
                hd.event_source_states.retain(|id, _| bit_get(marks, id.0));
                hd.sse_conn_to_object
                    .retain(|_, obj_id| bit_get(marks, obj_id.0));
                // Emit Close messages to the broker for swept
                // connections.  Best-effort: a disconnected handle
                // silently no-ops via `send`'s bool return.
                if !dead_ws_conns.is_empty() || !dead_sse_conns.is_empty() {
                    if let Some(handle) = self.network_handle.as_ref() {
                        for conn_id in dead_ws_conns {
                            let _ = handle.send(
                                elidex_net::broker::RendererToNetwork::WebSocketClose(conn_id),
                            );
                        }
                        for conn_id in dead_sse_conns {
                            let _ = handle.send(
                                elidex_net::broker::RendererToNetwork::EventSourceClose(conn_id),
                            );
                        }
                    }
                }
            }
        }

        // 5. IC invalidation.
        invalidate_ics(&mut self.compiled_functions, &self.gc_object_marks);

        // 6. Reset allocation counter and adjust threshold.
        self.gc_bytes_since_last = 0;
        self.gc_threshold = (live_count * 128).max(32768);
    }
}
