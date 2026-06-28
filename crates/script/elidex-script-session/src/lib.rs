//! Script session abstraction bridging JS engines and the ECS DOM.
//!
//! This crate provides the unified boundary between script engines
//! (`SpiderMonkey`, V8, etc.) and the elidex ECS-based DOM. It includes:
//!
//! - **`SessionCore`** — Mutation buffer and identity mapping coordinator
//! - **`IdentityMap`** — Bidirectional mapping between ECS entities and JS object references
//! - **`Mutation`** — Buffered DOM mutation operations applied on flush
//! - **`DomApiHandler`** / **`CssomApiHandler`** — Traits for DOM/CSSOM method dispatch

#[macro_use]
mod macros;
mod cssom_api;
mod dom_api;
mod engine;
pub mod event_dispatch;
pub mod event_handler_consumer;
pub mod event_listener;
pub mod event_queue;
mod identity_map;
mod mutation;
mod navigation;
mod session;
mod types;

pub use cssom_api::CssomApiHandler;
pub use dom_api::DomApiHandler;
pub use engine::{EvalResult, HostDriver, ScriptContext, ScriptEngine};
pub use event_dispatch::{
    apply_retarget, build_dispatch_plan, build_propagation_path, composed_path_for_js,
    dispatch_event, retarget, script_dispatch_event, script_dispatch_event_core, DispatchEvent,
    DispatchFlags, DispatchPlan, ListenerPlanEntry,
};
pub use event_handler_consumer::{
    document_cookie_spec_level, event_handler_attr_event_type, event_handler_attr_spec_level,
    live_collection_spec_level, web_storage_spec_level, EventHandlerAttributeConsumer,
    HandlerScope, EVENT_HANDLER_ATTRS, WORKER_EVENT_HANDLER_ATTRS,
    WORKER_OBJECT_EVENT_HANDLER_ATTRS,
};
pub use event_listener::{
    EventListeners, ListenerEntry, ListenerId, ListenerKind, UncompiledHandler,
};
pub use event_queue::{EventQueue, QueuedEvent};
pub use identity_map::IdentityMap;
pub use mutation::{
    apply_append_child, apply_insert_before, apply_mutation, apply_remove_attribute,
    apply_remove_child, apply_replace_all, apply_replace_child, apply_replace_data,
    apply_set_attribute, apply_set_inner_html, apply_set_outer_html, attribute_record,
    character_data_record, convert_arg_source_records, Mutation, MutationKind, MutationRecord,
    OuterHtmlError, SetInnerHtmlOptions,
};
pub use navigation::{HistoryAction, NavigationRequest};
pub use session::{CssomSheetState, SessionCore};
pub use types::{ComponentKind, DomApiError, DomApiErrorKind, JsObjectRef, ReadyState};
