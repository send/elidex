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

pub(super) mod elements;
pub(super) mod event_shapes;
pub(super) mod event_target;
pub(super) mod events;
pub(super) mod globals;
pub(super) mod location;
pub(super) mod navigation;
pub(super) mod navigator;
pub(super) mod performance;
pub(super) mod window;
