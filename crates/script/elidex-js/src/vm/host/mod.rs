//! Host integration for the VM — DOM wrappers, event objects, event
//! listeners, and session/ECS bridging.
//!
//! This module is the VM-side counterpart to boa's `bridge` / `globals`
//! machinery.  It assumes `HostData` is bound (raw `SessionCore` + `EcsDom`
//! pointers live through `NativeContext::host()`) and is only reachable
//! from code paths that have already verified boundness.
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
//! - [`dom_bridge`] — shared selector-parse and wrapper-lift helpers
//!   used by both `document.rs` and Element / Node prototype natives.

pub(crate) mod abort;
pub(super) mod abort_statics;
pub(super) mod character_data_proto;
pub(super) mod childnode;
pub(super) mod document;
pub(super) mod document_type_proto;
pub(super) mod dom_bridge;
pub(crate) mod dom_exception;
pub(super) mod element_insert_adjacent;
pub(super) mod element_proto;
pub(super) mod elements;
pub(super) mod event_shapes;
pub(super) mod event_target;
pub(super) mod events;
#[cfg(feature = "engine")]
pub(super) mod events_extras;
#[cfg(feature = "engine")]
pub(super) mod events_ui;
pub(super) mod globals;
#[cfg(feature = "engine")]
pub(crate) mod headers;
pub(super) mod history;
pub(super) mod html_iframe_proto;
pub(super) mod location;
pub(super) mod navigation;
pub(super) mod navigator;
pub(super) mod node_methods_extras;
pub(super) mod node_proto;
pub(super) mod parentnode;
pub(super) mod performance;
pub(super) mod text_proto;
pub(super) mod window;
