//! Host integration for the VM — DOM wrappers, event objects, event
//! listeners, and session/ECS bridging.
//!
//! This module is the VM-side counterpart to boa's `bridge` / `globals`
//! machinery.  It assumes `HostData` is bound (raw `SessionCore` + `EcsDom`
//! pointers live through `NativeContext::host()`) and is only reachable
//! from code paths that have already verified boundness.
//!
//! ## Layering mandate
//!
//! Files under `vm/host/` are restricted to **engine-bound responsibilities
//! only**: prototype install, brand check, and `JsValue` ↔ `Entity`
//! marshalling.  DOM mutation algorithms, selector matching, form
//! validation, live-collection walkers, label association, and constraint
//! validation must be invoked through engine-independent crates
//! (`elidex-dom-api` / `elidex-form` / `elidex-css` /
//! `elidex-script-session::DomApiHandler`).  See CLAUDE.md "Layering
//! mandate" and `memory/m4-12-architectural-drift-incident.md` for
//! the rationale.
//!
//! ## Handler dispatch flow
//!
//! ```text
//! native fn (vm/host/*.rs)
//!   | brand check (entity_from_this / require_node_arg)
//!   | call-site ToString (coerce_first_arg_to_string)
//!   v
//! dom_bridge::invoke_dom_api(ctx, "<method>", entity, &args)
//!   | Phase 1: prepare_arg → PreVal (Symbol → TypeError per WebIDL
//!   |          §3.10.14; raw BigInt → TypeError as a bridge-level
//!   |          defensive rule — call sites that ToString-coerce
//!   |          first land here as JsValue::String)
//!   | Phase 2: with_session_and_dom — materialize args (session-side
//!   |          identity_map → JsObjectRef), invoke handler, resolve
//!   |          ObjectRef return through identity_map → Entity
//!   | Phase 3: dom_api_error_to_vm_error / wrap entity → ObjectId
//!   v
//! return JsValue
//! ```
//!
//! `VmInner.dom_registry: Rc<DomHandlerRegistry>` is initialised once
//! at `Vm::new` and never mutated.  Handler resolution is by
//! `&'static str` method name; missing handlers raise
//! `VmError::type_error("Unknown DOM method: ...")` — there is no
//! `EcsDom::*` direct-call fallback so that the layering rule cannot
//! silently regress.
//!
//! Submodule responsibilities:
//!
//! - [`event_target`] — `EventTarget.prototype` intrinsic + native
//!   `addEventListener` / `removeEventListener` / `dispatchEvent`
//!   inherited by every DOM wrapper.
//! - [`node_proto`] — `Node.prototype` intrinsic, carrying the
//!   Node-common accessors (`parentNode`, `textContent`, …) and
//!   tree-mutation methods (`appendChild`, …).  Chains to
//!   `EventTarget.prototype`.  `cloneNode` /
//!   `compareDocumentPosition` / `isEqualNode` /
//!   `ownerDocument` / `isSameNode` / `getRootNode` bodies live in
//!   [`node_methods_extras`] to keep `node_proto.rs` under the
//!   1000-line convention; the install-time references stay in
//!   `node_proto`.
//! - [`element_proto`] — `Element.prototype` intrinsic, carrying
//!   Element-only members (`getAttribute`, `children`, `matches`, …).
//!   Chains to `Node.prototype`.
//! - [`elements`] — `create_element_wrapper` (entity → wrapper
//!   ObjectId, with per-entity prototype branching: Element vs
//!   non-Element Nodes).
//! - [`dom_bridge`] — shared selector-parse / wrapper-lift helpers
//!   used by both `document.rs` and Element / Node prototype natives,
//!   **plus** the `DomApiHandler` dispatch bridge (`invoke_dom_api`).

pub(crate) mod abort;
pub(super) mod abort_statics;
#[cfg(feature = "engine")]
pub(crate) mod array_buffer;
#[cfg(feature = "engine")]
pub(crate) mod attr_proto;
pub(crate) mod blob;
#[cfg(feature = "engine")]
pub(super) mod body_mixin;
#[cfg(feature = "engine")]
mod byte_io;
#[cfg(feature = "engine")]
pub(super) mod canvas;
pub(super) mod character_data_proto;
pub(super) mod childnode;
#[cfg(feature = "engine")]
pub(super) mod class_list;
#[cfg(feature = "engine")]
pub(crate) mod cors;
#[cfg(feature = "engine")]
pub(super) mod crypto;
#[cfg(feature = "engine")]
pub(super) mod css_style_declaration;
#[cfg(feature = "engine")]
pub(super) mod cssom_sheet;
#[cfg(feature = "engine")]
pub(super) mod custom_elements;
#[cfg(feature = "engine")]
pub(crate) mod data_view;
#[cfg(feature = "engine")]
pub(super) mod dataset;
pub(super) mod document;
#[cfg(feature = "engine")]
pub(super) mod document_fragment_proto;
pub(super) mod document_type_proto;
pub(super) mod dom_bridge;
pub(super) mod dom_collection;
#[cfg(feature = "engine")]
pub(crate) mod dispatch_target;
pub(crate) mod dom_exception;
#[cfg(feature = "engine")]
pub(super) mod dom_inner_html;
#[cfg(feature = "engine")]
pub(crate) mod dom_rect;
#[cfg(feature = "engine")]
pub(super) mod element_attrs;
pub(super) mod element_insert_adjacent;
pub(super) mod element_proto;
#[cfg(feature = "engine")]
pub(super) mod element_shadow;
pub(super) mod elements;
#[cfg(feature = "engine")]
pub(super) mod event_handler_attrs;
pub(super) mod event_shapes;
#[cfg(feature = "engine")]
pub(super) mod event_source;
#[cfg(feature = "engine")]
pub(super) mod event_source_dispatch;
pub(super) mod event_target;
#[cfg(feature = "engine")]
pub(super) mod event_target_dispatch;
#[cfg(feature = "engine")]
pub(super) mod event_target_dispatch_vm;
pub(super) mod events;
#[cfg(feature = "engine")]
pub(super) mod events_extras;
#[cfg(feature = "engine")]
pub(super) mod events_misc;
#[cfg(feature = "engine")]
pub(super) mod events_modern;
#[cfg(feature = "engine")]
pub(super) mod events_ui;
#[cfg(feature = "engine")]
pub(super) mod fetch;
#[cfg(feature = "engine")]
pub(super) mod fetch_tick;
#[cfg(feature = "engine")]
pub(crate) mod file;
#[cfg(feature = "engine")]
pub(crate) mod file_list;
#[cfg(feature = "engine")]
pub(crate) mod file_reader;
#[cfg(feature = "engine")]
pub(crate) mod form_data;
pub(super) mod globals;
#[cfg(feature = "engine")]
pub(crate) mod headers;
pub(super) mod history;
#[cfg(feature = "engine")]
pub(super) mod html_anchor_proto;
#[cfg(feature = "engine")]
pub(super) mod html_area_proto;
#[cfg(feature = "engine")]
pub(super) mod html_base_proto;
#[cfg(feature = "engine")]
pub(super) mod html_button_proto;
#[cfg(feature = "engine")]
pub(super) mod html_data_proto;
#[cfg(feature = "engine")]
pub(super) mod html_datalist_proto;
#[cfg(feature = "engine")]
pub(super) mod html_details_proto;
#[cfg(feature = "engine")]
pub(super) mod html_dialog_proto;
pub(super) mod html_element_proto;
#[cfg(feature = "engine")]
pub(super) mod html_fieldset_proto;
#[cfg(feature = "engine")]
pub(super) mod html_form_proto;
#[cfg(feature = "engine")]
pub(super) mod html_hyperlink_mixin;
pub(super) mod html_iframe_proto;
#[cfg(feature = "engine")]
pub(super) mod html_image_proto;
#[cfg(feature = "engine")]
pub(super) mod html_label_proto;
#[cfg(feature = "engine")]
pub(super) mod html_legend_proto;
#[cfg(feature = "engine")]
pub(super) mod html_li_proto;
#[cfg(feature = "engine")]
pub(super) mod html_link_proto;
#[cfg(feature = "engine")]
pub(super) mod html_map_proto;
#[cfg(feature = "engine")]
pub(super) mod html_meta_proto;
#[cfg(feature = "engine")]
pub(super) mod html_meter_proto;
#[cfg(feature = "engine")]
pub(super) mod html_olist_proto;
#[cfg(feature = "engine")]
pub(super) mod html_optgroup_proto;
#[cfg(feature = "engine")]
pub(super) mod html_option_proto;
#[cfg(feature = "engine")]
pub(super) mod html_options_collection;
#[cfg(feature = "engine")]
pub(super) mod html_output_proto;
#[cfg(feature = "engine")]
pub(super) mod html_passive_protos;
#[cfg(feature = "engine")]
pub(super) mod html_progress_proto;
#[cfg(feature = "engine")]
pub(super) mod html_slot_proto;
#[cfg(feature = "engine")]
pub(super) mod html_template_proto;
#[cfg(feature = "engine")]
pub(crate) mod indexeddb;
#[cfg(feature = "engine")]
pub(super) mod offscreen_canvas;
#[cfg(feature = "engine")]
pub(super) mod shadow_root_proto;

#[cfg(feature = "engine")]
pub(super) mod document_traversal;
#[cfg(feature = "engine")]
pub(super) mod dom_selection_proto;
#[cfg(feature = "engine")]
pub(super) mod form_state_sync;
#[cfg(feature = "engine")]
pub(super) mod html_input_proto;
#[cfg(feature = "engine")]
pub(super) mod html_input_selection;
#[cfg(feature = "engine")]
pub(super) mod html_input_value;
#[cfg(feature = "engine")]
pub(super) mod html_quote_proto;
#[cfg(feature = "engine")]
pub(super) mod html_script_proto;
#[cfg(feature = "engine")]
pub(super) mod html_select_proto;
#[cfg(feature = "engine")]
pub(super) mod html_style_proto;
#[cfg(feature = "engine")]
pub(super) mod html_table_caption_proto;
#[cfg(feature = "engine")]
pub(super) mod html_table_cell_proto;
#[cfg(feature = "engine")]
pub(super) mod html_table_col_proto;
#[cfg(feature = "engine")]
pub(super) mod html_table_proto;
#[cfg(feature = "engine")]
pub(super) mod html_table_row_proto;
#[cfg(feature = "engine")]
pub(super) mod html_table_section_proto;
#[cfg(feature = "engine")]
pub(super) mod html_textarea_proto;
#[cfg(feature = "engine")]
pub(super) mod html_time_proto;
#[cfg(feature = "engine")]
pub(super) mod html_title_proto;
#[cfg(feature = "engine")]
pub(super) mod idl_coerce;
#[cfg(feature = "engine")]
pub(super) mod intersection_observer;
pub(super) mod location;
#[cfg(feature = "engine")]
pub(super) mod multipart;
#[cfg(feature = "engine")]
pub(super) mod mutation_observer;
pub(super) mod named_node_map;
#[cfg(feature = "engine")]
pub(super) mod named_property_exotic;
pub(super) mod navigation;
pub(super) mod navigator;
#[cfg(feature = "engine")]
pub(super) mod node_filter_dispatch;
#[cfg(feature = "engine")]
pub(super) mod node_filter_namespace;
#[cfg(feature = "engine")]
pub(super) mod node_iterator_proto;
pub(super) mod node_methods_extras;
pub(super) mod node_proto;
#[cfg(feature = "engine")]
pub(super) mod observer_common;
pub(super) mod parentnode;
#[cfg(feature = "engine")]
pub(crate) mod pending_tasks;
pub(super) mod performance;
#[cfg(feature = "engine")]
pub(super) mod range_proto;
#[cfg(feature = "engine")]
pub(super) mod range_proto_mutation;
#[cfg(feature = "engine")]
pub(crate) mod readable_stream;
#[cfg(feature = "engine")]
pub(crate) mod request_response;
#[cfg(feature = "engine")]
pub(super) mod resize_observer;
#[cfg(feature = "engine")]
pub(super) mod selection_api;
#[cfg(feature = "engine")]
pub(super) mod static_range_proto;
#[cfg(feature = "engine")]
pub(super) mod storage;
#[cfg(feature = "engine")]
pub(super) mod storage_event;
#[cfg(feature = "engine")]
pub(super) mod structured_clone;
#[cfg(feature = "engine")]
pub(super) mod subtle_crypto;
#[cfg(feature = "engine")]
pub(crate) mod text_encoding;
pub(super) mod text_proto;
#[cfg(feature = "engine")]
pub(super) mod tree_walker_proto;
#[cfg(feature = "engine")]
pub(crate) mod typed_array;
#[cfg(feature = "engine")]
pub(super) mod typed_array_ctor;
#[cfg(feature = "engine")]
pub(super) mod typed_array_hof;
#[cfg(feature = "engine")]
pub(super) mod typed_array_install;
#[cfg(feature = "engine")]
pub(super) mod typed_array_methods;
#[cfg(feature = "engine")]
pub(super) mod typed_array_parts;
#[cfg(feature = "engine")]
pub(super) mod typed_array_static;
#[cfg(feature = "engine")]
pub(crate) mod url;
#[cfg(feature = "engine")]
pub(crate) mod url_search_params;
#[cfg(feature = "engine")]
pub(super) mod validity_state;
#[cfg(feature = "engine")]
pub(in crate::vm) mod wasm;
#[cfg(feature = "engine")]
pub(super) mod websocket;
#[cfg(feature = "engine")]
pub(super) mod websocket_dispatch;
pub(super) mod window;
#[cfg(feature = "engine")]
pub(super) mod worker;
#[cfg(feature = "engine")]
pub(super) mod worker_scope;
