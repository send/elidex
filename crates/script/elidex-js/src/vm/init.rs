//! [`Vm::new`] — VM construction and built-in registration.
//!
//! Extracted from `vm/mod.rs` to keep that file under the project's
//! 1000-line convention.  Construction logic is self-contained
//! (touches every `VmInner` field once and then hands off to
//! `register_globals`), so isolating it here keeps `mod.rs` focused
//! on the type definitions.

use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

use elidex_plugin::EngineMode;

use super::pools::{BigIntPool, StringPool};
use super::shape;
use super::value::{JsValue, ObjectId};
use super::well_known::{WellKnownStrings, WellKnownSymbols};
use super::{Vm, VmInner};

#[cfg(feature = "engine")]
use super::host;

impl Vm {
    /// Create a new main-thread (Window) VM with built-in globals registered.
    ///
    /// Uses [`EngineMode::BrowserCompat`] — the full compat surface, byte-identical
    /// to the pre-gate engine. This is the **sole public production constructor**:
    /// the non-compat modes are not selectable for a real session until the async
    /// core storage (`#11-async-core-storage-cookiestore`) lands — see
    /// `Vm::new_with_mode` (test-only, `#[cfg(test)]`).
    pub fn new() -> Self {
        Self::new_with_scope(super::GlobalScopeKind::Window, EngineMode::BrowserCompat)
    }

    /// Create a new main-thread (Window) VM under an explicit [`EngineMode`].
    ///
    /// The mode is the single engine-wide authority for the core/compat/deprecated
    /// split; it is fixed here at construction (before `register_globals` installs
    /// any surface) and cannot be changed afterwards.
    ///
    /// **`#[cfg(test)]` — not a production surface (F10).** [`EngineMode::BrowserCore`]
    /// / [`EngineMode::App`] must not be selected for a real session until the async
    /// core storage (`#11-async-core-storage-cookiestore`) lands (a core session is
    /// contracted to expose `elidex.storage`, design §14.4.3 — selecting these modes
    /// before then yields a session with *no* storage API). Production embedders use
    /// [`Vm::new`] (BrowserCompat); the `#[cfg(test)]` gate enforces the precondition
    /// by construction rather than by doc warning. The async-core PR removes the gate.
    #[cfg(test)]
    #[must_use]
    pub fn new_with_mode(engine_mode: EngineMode) -> Self {
        Self::new_with_scope(super::GlobalScopeKind::Window, engine_mode)
    }

    /// Create a new dedicated-worker VM (WHATWG HTML §10.2.1.1).
    ///
    /// The worker realm installs the `WorkerGlobalScope` surface instead of
    /// `window` / `document`. `name` is the `new Worker(url, { name })` value
    /// (empty when unnamed); `script_url` backs `WorkerLocation` and labels
    /// uncaught-error reports; `is_secure_context` is inherited from the
    /// creator's environment (WHATWG HTML §8.1.3.5 — not derived from
    /// `script_url`). The caller binds an empty `EcsDom` + worker scope entity
    /// and installs a worker-side `NetworkHandle` before eval.
    ///
    /// `engine_mode` is the engine-wide mode the spawning realm runs under — a
    /// worker realm inherits its creator's mode (the worker installs the same
    /// policy-gated surface, e.g. the DOM-handler registry and the currently
    /// over-exposed storage globals A2 demotes), so it must NOT be reset to the
    /// default. The caller (`vm/host/worker.rs`) propagates the parent VM's mode.
    ///
    /// **`pub(crate)` — not a production embedder surface (F10).** This constructor
    /// takes an explicit `engine_mode`, so leaving it `pub` would let a production
    /// embedder select [`EngineMode::BrowserCore`] / [`EngineMode::App`] for a
    /// worker realm and create the same no-storage realm the `#[cfg(test)]` gate on
    /// [`Vm::new_with_mode`] prevents on the main thread (a core session is
    /// contracted to expose `elidex.storage`, design §14.4.3). Restricting it to the
    /// crate keeps mode selection by-construction-safe: the only in-crate caller
    /// (`vm/worker_thread.rs`, driven by `vm/host/worker.rs`) threads the
    /// already-authorized parent mode, which is `BrowserCompat` until
    /// `#11-async-core-storage-cookiestore` lands — that PR re-publishes this with
    /// the explicit-mode capability.
    #[cfg(feature = "engine")]
    #[must_use]
    pub(crate) fn new_worker(
        name: String,
        script_url: url::Url,
        is_secure_context: bool,
        credentials: elidex_net::CredentialsMode,
        engine_mode: EngineMode,
    ) -> Self {
        Self::new_with_scope(
            super::GlobalScopeKind::DedicatedWorker {
                name,
                script_url,
                is_secure_context,
                credentials,
            },
            engine_mode,
        )
    }

    /// Create a new Service Worker VM (WHATWG Service Workers §4.1
    /// `ServiceWorkerGlobalScope`).
    ///
    /// The SW realm installs the `ServiceWorkerGlobalScope` surface
    /// (`self` / `clients` / `skipWaiting` + `oninstall`/`onactivate`/
    /// `onfetch`/`onmessage`) instead of `window` / `document` or the
    /// dedicated-worker `postMessage`/`close`.  `scope_url` is the
    /// registration scope (backs `WorkerLocation`); `script_url` labels
    /// error reports and is the `importScripts` base.  The caller
    /// (`vm/sw_thread.rs`) binds an empty `EcsDom` + SW scope entity,
    /// installs the shared cache backend + a `Send` `NetworkHandle`, and
    /// seeds the client snapshot before eval.
    ///
    /// `engine_mode` is the engine-wide mode supplied by the embedder for this
    /// SW realm (the SW installs the same policy-gated surface as any realm). The
    /// SW has no in-process parent VM, so the mode is threaded from the spawn
    /// entry (`sw_thread::sw_thread_main`); the embedder supplies it when it wires
    /// the elidex-js SW (today the shell still spawns SWs via the boa engine, so
    /// this path is exercised by elidex-js tests, which pass `BrowserCompat`).
    ///
    /// **`pub(crate)` — not a production embedder surface (F10).** Like
    /// [`Vm::new_worker`], the explicit `engine_mode` is restricted to the crate so
    /// a production embedder cannot select [`EngineMode::BrowserCore`] /
    /// [`EngineMode::App`] for a SW realm and create the no-storage realm the
    /// `#[cfg(test)]` gate on [`Vm::new_with_mode`] prevents on the main thread. The
    /// public SW spawn entry (`sw_thread::sw_thread_main`) hard-derives
    /// `BrowserCompat` (a SW has no parent VM to inherit from); the explicit-mode
    /// parameter here is exercised only by the crate-internal `run_service_worker`
    /// (elidex-js tests). `#11-async-core-storage-cookiestore` threads the
    /// authorized embedder mode through both when non-compat modes become
    /// production-selectable.
    #[cfg(feature = "engine")]
    #[must_use]
    pub(crate) fn new_service_worker(
        scope_url: url::Url,
        script_url: url::Url,
        is_secure_context: bool,
        credentials: elidex_net::CredentialsMode,
        engine_mode: EngineMode,
    ) -> Self {
        Self::new_with_scope(
            super::GlobalScopeKind::ServiceWorker {
                scope_url,
                script_url,
                is_secure_context,
                credentials,
            },
            engine_mode,
        )
    }

    /// Construct a VM for the given global scope kind + engine mode, then
    /// register globals.
    ///
    /// `register_globals` runs at the tail (before any caller can observe the
    /// VM), so the scope kind **and** the engine mode must be threaded in here
    /// rather than flipped afterwards — the install seams consult the derived
    /// [`SpecLevelPolicy`](elidex_plugin::SpecLevelPolicy) at install time, so a
    /// mode that arrived later could not *prevent* an install. In non-`engine`
    /// builds the scope kind / mode are unused (no DOM / worker / Web-API
    /// surface exists) but accepted to keep one shared construction body.
    #[allow(clippy::too_many_lines)]
    #[cfg_attr(not(feature = "engine"), allow(unused_variables))]
    fn new_with_scope(global_scope_kind: super::GlobalScopeKind, engine_mode: EngineMode) -> Self {
        let mut strings = StringPool::new();

        let well_known = WellKnownStrings::intern_all(&mut strings);
        // Captured before the struct literal moves `well_known`; used
        // to seed `window_name` to the empty-string id.
        #[cfg(feature = "engine")]
        let initial_window_name = well_known.empty;
        // The install policy is derived once here and consulted by every install
        // seam. It is stored in the `spec_level_policy` field below (set before
        // `register_globals` runs at the tail) and used inline to build the
        // policy-aware DOM-handler registry (seam-4). Both readers see the same
        // value — the gate cannot silently no-op (see field doc in `vm/mod.rs`).
        #[cfg(feature = "engine")]
        let spec_level_policy = engine_mode.spec_level_policy();
        // `compat-webapi` is the hard compile-time ceiling above the runtime
        // mode (the two faces of one mechanism): when the compat shims are not
        // compiled in (the `App`-profile build selects `engine` without
        // `compat-webapi`), no `Legacy` Web/DOM API may install regardless of
        // the `EngineMode` chosen at runtime. A1 marks nothing `Legacy`, so this
        // is latent today; A2/A3/B rely on it for the app-absence guarantee.
        #[cfg(all(feature = "engine", not(feature = "compat-webapi")))]
        let spec_level_policy = spec_level_policy.with_legacy_excluded();
        let (well_known_symbols, symbols) = WellKnownSymbols::alloc_all(&mut strings);

        let mut vm = Vm {
            inner: VmInner {
                stack: Vec::with_capacity(256),
                frames: Vec::with_capacity(16),
                strings,
                bigints: BigIntPool::new(),
                objects: Vec::new(),
                free_objects: Vec::new(),
                compiled_functions: Vec::new(),
                upvalues: Vec::new(),
                free_upvalues: Vec::new(),
                globals: HashMap::new(),
                symbols,
                symbol_registry: HashMap::new(),
                symbol_reverse_registry: HashMap::new(),
                well_known,
                well_known_symbols,
                string_prototype: None,
                symbol_prototype: None,
                object_prototype: None,
                array_prototype: None,
                number_prototype: None,
                boolean_prototype: None,
                bigint_prototype: None,
                function_prototype: None,
                regexp_prototype: None,
                array_iterator_prototype: None,
                string_iterator_prototype: None,
                // Placeholder — immediately replaced by register_globals().
                global_object: ObjectId(0),
                completion_value: JsValue::Undefined,
                saved_completion_stack: Vec::new(),
                current_exception: JsValue::Undefined,
                rng_state: {
                    // Seed from OS-RNG via RandomState so each Vm gets a
                    // unique sequence without requiring `rand`.
                    use std::collections::hash_map::RandomState;
                    use std::hash::{BuildHasher, Hasher};
                    let mut hasher = RandomState::new().build_hasher();
                    hasher.write_u64(0);
                    let seed = hasher.finish();
                    // Ensure non-zero (xorshift64 fixpoint).
                    if seed == 0 {
                        1
                    } else {
                        seed
                    }
                },
                shapes: vec![shape::Shape::root()],
                gc_object_marks: Vec::new(),
                gc_upvalue_marks: Vec::new(),
                gc_work_list: Vec::new(),
                gc_bytes_since_last: 0,
                gc_threshold: 65536,
                gc_enabled: false,
                html_element_constructor: None,
                active_bound_key: None,
                host_data: None,
                #[cfg(feature = "engine")]
                dom_registry: std::rc::Rc::new(
                    elidex_dom_api::registry::create_dom_registry_with_policy(spec_level_policy),
                ),
                promise_prototype: None,
                microtask_queue: VecDeque::new(),
                microtask_drain_depth: 0,
                #[cfg(feature = "engine")]
                pending_tasks: VecDeque::new(),
                #[cfg(feature = "engine")]
                task_drain_depth: 0,
                pending_rejections: Vec::new(),
                error_prototype: None,
                aggregate_error_prototype: None,
                generator_prototype: None,
                event_target_prototype: None,
                node_prototype: None,
                element_prototype: None,
                #[cfg(feature = "engine")]
                worker_scope_prototype: None,
                #[cfg(feature = "engine")]
                worker_prototype: None,
                #[cfg(feature = "engine")]
                character_data_prototype: None,
                #[cfg(feature = "engine")]
                text_prototype: None,
                #[cfg(feature = "engine")]
                document_type_prototype: None,
                #[cfg(feature = "engine")]
                html_element_prototype: None,
                #[cfg(feature = "engine")]
                html_collection_prototype: None,
                #[cfg(feature = "engine")]
                node_list_prototype: None,
                #[cfg(feature = "engine")]
                live_collection_states: HashMap::new(),
                #[cfg(feature = "engine")]
                named_node_map_prototype: None,
                #[cfg(feature = "engine")]
                named_node_map_states: HashMap::new(),
                #[cfg(feature = "engine")]
                dom_token_list_prototype: None,
                #[cfg(feature = "engine")]
                dom_string_map_prototype: None,
                #[cfg(feature = "engine")]
                attr_prototype: None,
                #[cfg(feature = "engine")]
                attr_states: HashMap::new(),
                #[cfg(feature = "engine")]
                shadow_root_prototype: None,
                #[cfg(feature = "engine")]
                document_fragment_prototype: None,
                #[cfg(feature = "engine")]
                html_slot_prototype: None,
                #[cfg(feature = "engine")]
                pending_slot_change_signals: std::collections::VecDeque::new(),
                #[cfg(feature = "engine")]
                mutation_observer_microtask_queued: false,
                #[cfg(feature = "engine")]
                css_style_declaration_prototype: None,
                #[cfg(feature = "engine")]
                css_stylesheet_prototype: None,
                #[cfg(feature = "engine")]
                css_rule_list_prototype: None,
                #[cfg(feature = "engine")]
                css_style_rule_prototype: None,
                #[cfg(feature = "engine")]
                style_sheet_list_prototype: None,
                #[cfg(feature = "engine")]
                mutation_observer_prototype: None,
                #[cfg(feature = "engine")]
                resize_observer_prototype: None,
                #[cfg(feature = "engine")]
                intersection_observer_prototype: None,
                #[cfg(feature = "engine")]
                range_prototype: None,
                #[cfg(feature = "engine")]
                static_range_prototype: None,
                #[cfg(feature = "engine")]
                tree_walker_prototype: None,
                #[cfg(feature = "engine")]
                node_iterator_prototype: None,
                #[cfg(feature = "engine")]
                selection_prototype: None,
                #[cfg(feature = "engine")]
                storage_prototype: None,
                #[cfg(feature = "engine")]
                storage_event_prototype: None,
                #[cfg(feature = "engine")]
                storage_local_instance: None,
                #[cfg(feature = "engine")]
                storage_session_instance: None,
                #[cfg(feature = "engine")]
                crypto_prototype: None,
                #[cfg(feature = "engine")]
                subtle_crypto_prototype: None,
                #[cfg(feature = "engine")]
                crypto_key_prototype: None,
                #[cfg(feature = "engine")]
                websocket_prototype: None,
                #[cfg(feature = "engine")]
                event_source_prototype: None,
                #[cfg(feature = "engine")]
                crypto_instance: None,
                #[cfg(feature = "engine")]
                subtle_crypto_instance: None,
                #[cfg(feature = "engine")]
                custom_element_registry_prototype: None,
                #[cfg(feature = "engine")]
                custom_element_registry_instance: None,
                #[cfg(feature = "engine")]
                html_iframe_prototype: None,
                #[cfg(feature = "engine")]
                html_label_prototype: None,
                #[cfg(feature = "engine")]
                html_optgroup_prototype: None,
                #[cfg(feature = "engine")]
                html_legend_prototype: None,
                #[cfg(feature = "engine")]
                html_option_prototype: None,
                #[cfg(feature = "engine")]
                html_fieldset_prototype: None,
                #[cfg(feature = "engine")]
                html_form_prototype: None,
                #[cfg(feature = "engine")]
                html_button_prototype: None,
                #[cfg(feature = "engine")]
                html_textarea_prototype: None,
                #[cfg(feature = "engine")]
                html_select_prototype: None,
                #[cfg(feature = "engine")]
                html_input_prototype: None,
                #[cfg(feature = "engine")]
                html_anchor_prototype: None,
                #[cfg(feature = "engine")]
                html_area_prototype: None,
                #[cfg(feature = "engine")]
                html_image_prototype: None,
                #[cfg(feature = "engine")]
                html_script_prototype: None,
                #[cfg(feature = "engine")]
                html_link_prototype: None,
                #[cfg(feature = "engine")]
                html_canvas_prototype: None,
                #[cfg(feature = "engine")]
                canvas_rendering_context_2d_prototype: None,
                #[cfg(feature = "engine")]
                image_data_prototype: None,
                #[cfg(feature = "engine")]
                offscreen_canvas_prototype: None,
                #[cfg(feature = "engine")]
                offscreen_canvas_rendering_context_2d_prototype: None,
                // T2b passive head + grouping prototypes
                // (slot `#11-tags-T2b-passive`).
                #[cfg(feature = "engine")]
                html_html_prototype: None,
                #[cfg(feature = "engine")]
                html_head_prototype: None,
                #[cfg(feature = "engine")]
                html_body_prototype: None,
                #[cfg(feature = "engine")]
                html_title_prototype: None,
                #[cfg(feature = "engine")]
                html_base_prototype: None,
                #[cfg(feature = "engine")]
                html_meta_prototype: None,
                #[cfg(feature = "engine")]
                html_style_prototype: None,
                #[cfg(feature = "engine")]
                html_div_prototype: None,
                #[cfg(feature = "engine")]
                html_span_prototype: None,
                #[cfg(feature = "engine")]
                html_br_prototype: None,
                #[cfg(feature = "engine")]
                html_hr_prototype: None,
                #[cfg(feature = "engine")]
                html_pre_prototype: None,
                #[cfg(feature = "engine")]
                html_p_prototype: None,
                #[cfg(feature = "engine")]
                html_heading_prototype: None,
                #[cfg(feature = "engine")]
                html_quote_prototype: None,
                #[cfg(feature = "engine")]
                html_olist_prototype: None,
                #[cfg(feature = "engine")]
                html_ulist_prototype: None,
                #[cfg(feature = "engine")]
                html_li_prototype: None,
                #[cfg(feature = "engine")]
                html_dlist_prototype: None,
                #[cfg(feature = "engine")]
                html_menu_prototype: None,
                #[cfg(feature = "engine")]
                html_map_prototype: None,
                #[cfg(feature = "engine")]
                html_picture_prototype: None,
                #[cfg(feature = "engine")]
                html_data_prototype: None,
                #[cfg(feature = "engine")]
                html_time_prototype: None,
                #[cfg(feature = "engine")]
                html_form_controls_collection_prototype: None,
                #[cfg(feature = "engine")]
                html_options_collection_prototype: None,
                #[cfg(feature = "engine")]
                validity_state_prototype: None,
                #[cfg(feature = "engine")]
                html_table_prototype: None,
                #[cfg(feature = "engine")]
                html_table_section_prototype: None,
                #[cfg(feature = "engine")]
                html_table_row_prototype: None,
                #[cfg(feature = "engine")]
                html_table_cell_prototype: None,
                #[cfg(feature = "engine")]
                html_table_caption_prototype: None,
                #[cfg(feature = "engine")]
                html_table_col_prototype: None,
                #[cfg(feature = "engine")]
                html_dialog_prototype: None,
                #[cfg(feature = "engine")]
                html_details_prototype: None,
                #[cfg(feature = "engine")]
                html_template_prototype: None,
                #[cfg(feature = "engine")]
                html_datalist_prototype: None,
                #[cfg(feature = "engine")]
                html_output_prototype: None,
                #[cfg(feature = "engine")]
                html_progress_prototype: None,
                #[cfg(feature = "engine")]
                html_meter_prototype: None,
                #[cfg(feature = "engine")]
                dom_exception_prototype: None,
                #[cfg(feature = "engine")]
                dom_exception_states: HashMap::new(),
                #[cfg(feature = "engine")]
                crypto_key_states: HashMap::new(),
                #[cfg(feature = "engine")]
                crypto_key_js_cache: HashMap::new(),
                #[cfg(feature = "engine")]
                dom_rect_readonly_prototype: None,
                #[cfg(feature = "engine")]
                dom_rect_prototype: None,
                #[cfg(feature = "engine")]
                dom_rect_states: HashMap::new(),
                window_prototype: None,
                event_prototype: None,
                #[cfg(feature = "engine")]
                custom_event_prototype: None,
                #[cfg(feature = "engine")]
                ui_event_prototype: None,
                #[cfg(feature = "engine")]
                mouse_event_prototype: None,
                #[cfg(feature = "engine")]
                keyboard_event_prototype: None,
                #[cfg(feature = "engine")]
                focus_event_prototype: None,
                #[cfg(feature = "engine")]
                input_event_prototype: None,
                #[cfg(feature = "engine")]
                promise_rejection_event_prototype: None,
                #[cfg(feature = "engine")]
                error_event_prototype: None,
                #[cfg(feature = "engine")]
                hash_change_event_prototype: None,
                #[cfg(feature = "engine")]
                pop_state_event_prototype: None,
                #[cfg(feature = "engine")]
                animation_event_prototype: None,
                #[cfg(feature = "engine")]
                transition_event_prototype: None,
                #[cfg(feature = "engine")]
                close_event_prototype: None,
                // D-10 events-misc: 10 NEW Event constructor prototypes
                // (slot `#11-events-misc`).
                #[cfg(feature = "engine")]
                submit_event_prototype: None,
                #[cfg(feature = "engine")]
                formdata_event_prototype: None,
                #[cfg(feature = "engine")]
                toggle_event_prototype: None,
                #[cfg(feature = "engine")]
                composition_event_prototype: None,
                #[cfg(feature = "engine")]
                clipboard_event_prototype: None,
                #[cfg(feature = "engine")]
                progress_event_prototype: None,
                #[cfg(feature = "engine")]
                before_unload_event_prototype: None,
                #[cfg(feature = "engine")]
                before_unload_return_values: HashMap::new(),
                #[cfg(feature = "engine")]
                message_event_prototype: None,
                #[cfg(feature = "engine")]
                wheel_event_prototype: None,
                #[cfg(feature = "engine")]
                page_transition_event_prototype: None,
                // D-9 events-modern-input (slot
                // `#11-events-modern-input`).
                #[cfg(feature = "engine")]
                pointer_event_prototype: None,
                #[cfg(feature = "engine")]
                drag_event_prototype: None,
                #[cfg(feature = "engine")]
                touch_event_prototype: None,
                #[cfg(feature = "engine")]
                touch_prototype: None,
                #[cfg(feature = "engine")]
                touch_list_prototype: None,
                #[cfg(feature = "engine")]
                data_transfer_prototype: None,
                #[cfg(feature = "engine")]
                data_transfer_item_prototype: None,
                #[cfg(feature = "engine")]
                data_transfer_item_list_prototype: None,
                #[cfg(feature = "engine")]
                data_transfer_states: HashMap::new(),
                #[cfg(feature = "engine")]
                touch_states: HashMap::new(),
                #[cfg(feature = "engine")]
                touch_list_states: HashMap::new(),
                #[cfg(feature = "engine")]
                headers_prototype: None,
                #[cfg(feature = "engine")]
                headers_states: HashMap::new(),
                #[cfg(feature = "engine")]
                request_prototype: None,
                #[cfg(feature = "engine")]
                request_states: HashMap::new(),
                #[cfg(feature = "engine")]
                response_prototype: None,
                #[cfg(feature = "engine")]
                response_states: HashMap::new(),
                #[cfg(feature = "engine")]
                body_data: HashMap::new(),
                #[cfg(feature = "engine")]
                disturbed: HashSet::new(),
                #[cfg(feature = "engine")]
                detached_buffers: HashSet::new(),
                #[cfg(feature = "engine")]
                wasm_module_storage: HashMap::new(),
                #[cfg(feature = "engine")]
                wasm_instance_storage: HashMap::new(),
                #[cfg(feature = "engine")]
                wasm_memory_storage: HashMap::new(),
                #[cfg(feature = "engine")]
                wasm_table_storage: HashMap::new(),
                #[cfg(feature = "engine")]
                wasm_global_storage: HashMap::new(),
                #[cfg(feature = "engine")]
                wasm_exported_func_storage: HashMap::new(),
                #[cfg(feature = "engine")]
                wasm_backed_buffers: HashMap::new(),
                #[cfg(feature = "engine")]
                wasm_runtime: std::cell::OnceCell::new(),
                #[cfg(feature = "engine")]
                wasm_module_prototype: None,
                #[cfg(feature = "engine")]
                wasm_instance_prototype: None,
                #[cfg(feature = "engine")]
                wasm_memory_prototype: None,
                #[cfg(feature = "engine")]
                wasm_table_prototype: None,
                #[cfg(feature = "engine")]
                wasm_global_prototype: None,
                #[cfg(feature = "engine")]
                wasm_compile_error_prototype: None,
                #[cfg(feature = "engine")]
                wasm_link_error_prototype: None,
                #[cfg(feature = "engine")]
                wasm_runtime_error_prototype: None,
                #[cfg(feature = "engine")]
                array_buffer_prototype: None,
                #[cfg(feature = "engine")]
                blob_prototype: None,
                #[cfg(feature = "engine")]
                blob_data: HashMap::new(),
                #[cfg(feature = "engine")]
                file_prototype: None,
                #[cfg(feature = "engine")]
                file_list_prototype: None,
                #[cfg(feature = "engine")]
                file_reader_prototype: None,
                #[cfg(feature = "engine")]
                file_data: HashMap::new(),
                #[cfg(feature = "engine")]
                file_list_data: HashMap::new(),
                #[cfg(feature = "engine")]
                file_reader_data: HashMap::new(),
                #[cfg(feature = "engine")]
                typed_array_prototype: None,
                #[cfg(feature = "engine")]
                data_view_prototype: None,
                #[cfg(feature = "engine")]
                subclass_array_prototypes: [None; super::value::ElementKind::COUNT],
                #[cfg(feature = "engine")]
                subclass_array_ctors: [None; super::value::ElementKind::COUNT],
                #[cfg(feature = "engine")]
                text_encoder_prototype: None,
                #[cfg(feature = "engine")]
                text_decoder_prototype: None,
                #[cfg(feature = "engine")]
                text_decoder_states: HashMap::new(),
                #[cfg(feature = "engine")]
                url_search_params_prototype: None,
                #[cfg(feature = "engine")]
                url_search_params_states: HashMap::new(),
                #[cfg(feature = "engine")]
                url_prototype: None,
                #[cfg(feature = "engine")]
                url_states: HashMap::new(),
                #[cfg(feature = "engine")]
                usp_parent_url: HashMap::new(),
                #[cfg(feature = "engine")]
                form_data_prototype: None,
                #[cfg(feature = "engine")]
                form_data_states: HashMap::new(),
                #[cfg(feature = "engine")]
                readable_stream_prototype: None,
                #[cfg(feature = "engine")]
                readable_stream_default_reader_prototype: None,
                #[cfg(feature = "engine")]
                readable_stream_default_controller_prototype: None,
                #[cfg(feature = "engine")]
                readable_stream_states: HashMap::new(),
                #[cfg(feature = "engine")]
                readable_stream_reader_states: HashMap::new(),
                #[cfg(feature = "engine")]
                body_streams: HashMap::new(),
                #[cfg(feature = "engine")]
                count_queuing_strategy_prototype: None,
                #[cfg(feature = "engine")]
                byte_length_queuing_strategy_prototype: None,
                #[cfg(feature = "engine")]
                network_handle: None,
                #[cfg(feature = "engine")]
                idb_backend: None,
                #[cfg(feature = "engine")]
                idb_request_states: HashMap::new(),
                #[cfg(feature = "engine")]
                idb_database_states: HashMap::new(),
                #[cfg(feature = "engine")]
                idb_object_store_states: HashMap::new(),
                #[cfg(feature = "engine")]
                idb_transaction_states: HashMap::new(),
                #[cfg(feature = "engine")]
                idb_key_range_states: HashMap::new(),
                #[cfg(feature = "engine")]
                idb_index_states: HashMap::new(),
                #[cfg(feature = "engine")]
                idb_cursor_states: HashMap::new(),
                #[cfg(feature = "engine")]
                idb_factory_prototype: None,
                #[cfg(feature = "engine")]
                idb_request_prototype: None,
                #[cfg(feature = "engine")]
                idb_open_db_request_prototype: None,
                #[cfg(feature = "engine")]
                idb_database_prototype: None,
                #[cfg(feature = "engine")]
                idb_object_store_prototype: None,
                #[cfg(feature = "engine")]
                idb_transaction_prototype: None,
                #[cfg(feature = "engine")]
                idb_key_range_prototype: None,
                #[cfg(feature = "engine")]
                idb_index_prototype: None,
                #[cfg(feature = "engine")]
                idb_cursor_prototype: None,
                #[cfg(feature = "engine")]
                idb_cursor_with_value_prototype: None,
                #[cfg(feature = "engine")]
                idb_version_change_event_prototype: None,
                #[cfg(feature = "engine")]
                cache_handle_states: HashMap::new(),
                #[cfg(feature = "engine")]
                cache_storage_prototype: None,
                #[cfg(feature = "engine")]
                cache_prototype: None,
                #[cfg(feature = "engine")]
                fetch_event_states: HashMap::new(),
                #[cfg(feature = "engine")]
                extendable_event_states: HashMap::new(),
                #[cfg(feature = "engine")]
                client_states: HashMap::new(),
                #[cfg(feature = "engine")]
                sw_clients: Vec::new(),
                #[cfg(feature = "engine")]
                sw_outgoing: Vec::new(),
                #[cfg(feature = "engine")]
                service_worker_scope_prototype: None,
                #[cfg(feature = "engine")]
                extendable_event_prototype: None,
                #[cfg(feature = "engine")]
                fetch_event_prototype: None,
                #[cfg(feature = "engine")]
                clients_prototype: None,
                #[cfg(feature = "engine")]
                client_prototype: None,
                // navigator.serviceWorker client (D-19 PR-3).
                #[cfg(feature = "engine")]
                sw_registrations: HashMap::new(),
                #[cfg(feature = "engine")]
                sw_registration_states: HashMap::new(),
                #[cfg(feature = "engine")]
                service_worker_states: HashMap::new(),
                #[cfg(feature = "engine")]
                pending_registration_promises: HashMap::new(),
                #[cfg(feature = "engine")]
                pending_unregister_promises: HashMap::new(),
                #[cfg(feature = "engine")]
                sw_ready_promise: None,
                #[cfg(feature = "engine")]
                sw_container: None,
                #[cfg(feature = "engine")]
                sw_controller_scope: None,
                #[cfg(feature = "engine")]
                sw_messages_enabled: false,
                #[cfg(feature = "engine")]
                sw_message_buffer: Vec::new(),
                #[cfg(feature = "engine")]
                sw_client_outgoing: Vec::new(),
                #[cfg(feature = "engine")]
                sw_registration_prototype: None,
                #[cfg(feature = "engine")]
                sw_worker_prototype: None,
                #[cfg(feature = "engine")]
                fetch_abort_observers: HashMap::new(),
                #[cfg(feature = "engine")]
                pending_fetches: HashMap::new(),
                #[cfg(feature = "engine")]
                pending_fetch_cors: HashMap::new(),
                #[cfg(feature = "engine")]
                fetch_signal_back_refs: HashMap::new(),
                #[cfg(feature = "engine")]
                abort_signal_prototype: None,
                #[cfg(feature = "engine")]
                media_query_list_prototype: None,
                #[cfg(feature = "engine")]
                media_query_list_event_prototype: None,
                #[cfg(feature = "engine")]
                media_query_list_registry: HashMap::new(),
                #[cfg(feature = "engine")]
                media_query_list_next_seq: 0,
                #[cfg(feature = "engine")]
                abort_signal_states: HashMap::new(),
                #[cfg(feature = "engine")]
                abort_listener_back_refs: HashMap::new(),
                #[cfg(feature = "engine")]
                pending_timeout_signals: HashMap::new(),
                #[cfg(feature = "engine")]
                any_composite_map: HashMap::new(),
                #[cfg(feature = "engine")]
                dispatched_events: HashSet::new(),
                #[cfg(feature = "engine")]
                vm_event_listeners: HashMap::new(),
                #[cfg(feature = "engine")]
                precomputed_event_shapes: None,
                generator_yielded: None,
                current_microtask: None,
                timer_queue: BinaryHeap::new(),
                current_timer: None,
                next_timer_id: 1,
                active_timer_ids: HashSet::new(),
                cancelled_timers: HashSet::new(),
                #[cfg(feature = "engine")]
                start_instant: std::time::Instant::now(),
                #[cfg(feature = "engine")]
                navigation: host::navigation::NavigationState::new(),
                #[cfg(feature = "engine")]
                viewport: host::window::ViewportState::new(),
                #[cfg(feature = "engine")]
                window_name: initial_window_name,
                #[cfg(feature = "engine")]
                global_scope_kind,
                #[cfg(feature = "engine")]
                spec_level_policy,
                #[cfg(feature = "engine")]
                engine_mode,
                #[cfg(feature = "engine")]
                worker_outgoing: Vec::new(),
                #[cfg(feature = "engine")]
                worker_close_requested: false,
                #[cfg(feature = "engine")]
                worker_registry: elidex_api_workers::WorkerRegistry::new(),
                #[cfg(feature = "engine")]
                worker_entities: HashMap::new(),
            },
        };

        vm.inner.register_globals();
        vm.inner.gc_enabled = true;
        vm
    }
}
