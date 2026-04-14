//! `EventTarget.prototype` intrinsic — holds the three native methods
//! (`addEventListener`, `removeEventListener`, `dispatchEvent`) that every
//! DOM wrapper inherits.
//!
//! ## Why a shared prototype?
//!
//! The alternative — registering the three methods directly on each
//! element wrapper at creation time — would allocate N × 3 native-function
//! objects for N elements.  A single shared prototype matches the spec
//! (WHATWG DOM §2.7 `EventTarget` interface) and aligns with how
//! `Promise.prototype` / `Array.prototype` are structured elsewhere in
//! the VM.
//!
//! ## Stub status (PR3 C0)
//!
//! At C0 the three method bodies are **stubs** — they return `undefined`.
//! Real implementations land later:
//!
//! - `addEventListener` / `removeEventListener`: PR3 C7 / C8 — register
//!   the listener in the `EventListeners` ECS component + `HostData::
//!   listener_store`, honouring `capture` / `once` / `passive`.
//! - `dispatchEvent`: **deferred to PR5a** alongside `Event` constructors,
//!   which are the only meaningful way to pass a JS-constructed event
//!   into a synchronous dispatch from script.  Until then the stub is a
//!   no-op; `dispatchEvent` is still resolvable via the prototype chain,
//!   which is enough for scripts that only feature-test its existence.

use super::super::value::{JsValue, NativeContext, VmError};
use super::super::{NativeFn, VmInner};

impl VmInner {
    /// Populate `self.event_target_prototype` with the `EventTarget`
    /// interface methods (WHATWG DOM §2.7).
    ///
    /// Called from `register_globals()` after `Object.prototype` is in
    /// place (every DOM wrapper's prototype chain terminates in
    /// `Object.prototype`, so this intrinsic sits one level above it).
    ///
    /// The three method bodies are **stubs** at C0 — see module doc for
    /// the per-method replacement schedule.
    pub(in crate::vm) fn register_event_target_prototype(&mut self) {
        let proto_id = self.create_object_with_methods(&[
            (
                "addEventListener",
                native_event_target_add_event_listener as NativeFn,
            ),
            (
                "removeEventListener",
                native_event_target_remove_event_listener,
            ),
            ("dispatchEvent", native_event_target_dispatch_event),
        ]);
        self.event_target_prototype = Some(proto_id);
    }
}

/// `EventTarget.prototype.addEventListener(type, listener, options)` — stub.
///
/// Real implementation arrives in PR3 C7.
pub(super) fn native_event_target_add_event_listener(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Undefined)
}

/// `EventTarget.prototype.removeEventListener(type, listener, options)` — stub.
///
/// Real implementation arrives in PR3 C8.
pub(super) fn native_event_target_remove_event_listener(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Undefined)
}

/// `EventTarget.prototype.dispatchEvent(event)` — stub.
///
/// Real implementation is deferred to **PR5a** (which lands `new Event(...)`
/// and the Event constructor family).  Until then, invoking this returns
/// `false` (the spec default for "event not dispatched").
pub(super) fn native_event_target_dispatch_event(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Boolean(false))
}
