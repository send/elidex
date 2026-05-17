//! [`Vm::new`] — VM construction and built-in registration.
//!
//! Extracted from `vm/mod.rs` to keep that file under the project's
//! 1000-line convention.  Construction logic is self-contained
//! (touches every `VmInner` field once and then hands off to
//! `register_globals`), so isolating it here keeps `mod.rs` focused
//! on the type definitions.

use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

use super::pools::{BigIntPool, StringPool};
use super::shape;
use super::value::{JsValue, ObjectId};
use super::well_known::{WellKnownStrings, WellKnownSymbols};
use super::{Vm, VmInner};

#[cfg(feature = "engine")]
use super::host;

impl Vm {
    /// Create a new VM with built-in globals registered.
    #[allow(clippy::too_many_lines)]
    pub fn new() -> Self {
        let mut strings = StringPool::new();

        let well_known = WellKnownStrings::intern_all(&mut strings);
        // Captured before the struct literal moves `well_known`; used
        // to seed `window_name` to the empty-string id.
        #[cfg(feature = "engine")]
        let initial_window_name = well_known.empty;
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
                in_construct: false,
                host_data: None,
                #[cfg(feature = "engine")]
                dom_registry: std::rc::Rc::new(elidex_dom_api::registry::create_dom_registry()),
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
                attr_wrapper_cache: HashMap::new(),
                #[cfg(feature = "engine")]
                class_list_wrapper_cache: HashMap::new(),
                #[cfg(feature = "engine")]
                dataset_wrapper_cache: HashMap::new(),
                #[cfg(feature = "engine")]
                rel_list_wrapper_cache: HashMap::new(),
                #[cfg(feature = "engine")]
                link_rel_list_wrapper_cache: HashMap::new(),
                #[cfg(feature = "engine")]
                link_sizes_wrapper_cache: HashMap::new(),
                #[cfg(feature = "engine")]
                css_style_declaration_prototype: None,
                #[cfg(feature = "engine")]
                style_wrapper_cache: HashMap::new(),
                #[cfg(feature = "engine")]
                css_stylesheet_prototype: None,
                #[cfg(feature = "engine")]
                css_rule_list_prototype: None,
                #[cfg(feature = "engine")]
                css_style_rule_prototype: None,
                #[cfg(feature = "engine")]
                style_sheet_list_prototype: None,
                #[cfg(feature = "engine")]
                stylesheet_wrapper_cache: HashMap::new(),
                #[cfg(feature = "engine")]
                css_style_rule_wrapper_cache: HashMap::new(),
                #[cfg(feature = "engine")]
                rule_style_wrapper_cache: HashMap::new(),
                #[cfg(feature = "engine")]
                mutation_observer_prototype: None,
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
                websocket_prototype: None,
                #[cfg(feature = "engine")]
                event_source_prototype: None,
                #[cfg(feature = "engine")]
                crypto_instance: None,
                #[cfg(feature = "engine")]
                subtle_crypto_instance: None,
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
                validity_state_wrappers: HashMap::new(),
                #[cfg(feature = "engine")]
                options_collection_wrappers: HashMap::new(),
                #[cfg(feature = "engine")]
                form_controls_collection_wrappers: HashMap::new(),
                #[cfg(feature = "engine")]
                map_areas_wrappers: HashMap::new(),
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
                table_rows_wrappers: HashMap::new(),
                #[cfg(feature = "engine")]
                table_bodies_wrappers: HashMap::new(),
                #[cfg(feature = "engine")]
                table_section_rows_wrappers: HashMap::new(),
                #[cfg(feature = "engine")]
                table_row_cells_wrappers: HashMap::new(),
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
                template_content_wrappers: HashMap::new(),
                #[cfg(feature = "engine")]
                datalist_options_wrappers: HashMap::new(),
                #[cfg(feature = "engine")]
                output_html_for_wrappers: HashMap::new(),
                #[cfg(feature = "engine")]
                dom_exception_prototype: None,
                #[cfg(feature = "engine")]
                dom_exception_states: HashMap::new(),
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
                data_transfer_item_wrapper_cache: HashMap::new(),
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
                input_files_cache: HashMap::new(),
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
            },
        };

        vm.inner.register_globals();
        vm.inner.gc_enabled = true;
        vm
    }
}
