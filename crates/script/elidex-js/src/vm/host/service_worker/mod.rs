//! Service Worker realm host bindings (WHATWG Service Workers §4; slot
//! `#11-service-workers-vm` / D-19 PR-2).
//!
//! ```text
//! ServiceWorkerGlobalScope (globalThis)  → ServiceWorkerGlobalScope.prototype → EventTarget.prototype
//!   self / clients / skipWaiting() + oninstall/onactivate/onfetch/onmessage
//! FetchEvent       (ObjectKind::Event)   → FetchEvent.prototype       → ExtendableEvent.prototype → Event.prototype
//!   request / clientId / resultingClientId + respondWith() (+ inherited waitUntil())
//! ExtendableEvent  (ObjectKind::Event)   → ExtendableEvent.prototype  → Event.prototype
//!   waitUntil()
//! Clients          (façade)              → Clients.prototype          → Object.prototype
//!   get() / matchAll() / claim()
//! Client           (ObjectKind::Client)  → Client.prototype           → Object.prototype
//!   id / url / type / frameType + postMessage()
//! ```
//!
//! ## Layering (CLAUDE.md Layering mandate)
//!
//! This module is marshalling + ECS event dispatch ONLY.  The IPC
//! protocol (`ContentToSw` / `SwToContent` / `ClientSnapshot`) and every
//! pure SW algorithm (security / scope / update) live in the engine-
//! independent `elidex-api-sw` crate.  `host/service_worker/` converts
//! `SwRequest` → a `Request` object and a `Response` object → `SwResponse`
//! (`marshal.rs`), fires `install`/`activate`/`fetch`/`message` through the
//! shared `dispatch_script_event` core, and stages outbound IPC on
//! `VmInner::sw_outgoing` for the SW thread loop (`vm/sw_thread.rs`) to
//! forward.  The DR-C `respondWith` real-promise drain lives in the loop
//! (it pumps the VM), not here.
//!
//! ## ObjectKind decision (DR-C / §D)
//!
//! `FetchEvent` / `ExtendableEvent` are ordinary `ObjectKind::Event`s, NOT
//! own brands: `dispatch_script_event` reads the `ObjectKind::Event`
//! internal slots and `unreachable!`s on any other kind, so a dispatched
//! event MUST be `ObjectKind::Event` (the `MessageEvent` precedent).  Their
//! "FetchEvent-ness" / "ExtendableEvent-ness" lives in the
//! [`FetchEventState`] / [`ExtendableEventState`] side-stores, which double
//! as the brand for `respondWith` / `waitUntil`.  (StorageEvent gets an own
//! brand only because it is never dispatched.)  `Client` *is* an own brand
//! (`ObjectKind::Client`) — it is a plain object, never dispatched.

#![cfg(feature = "engine")]

use super::super::value::{
    native_illegal_constructor_unreachable, CallShape, JsValue, NativeContext, ObjectId, VmError,
};
use super::super::{GlobalScopeKind, NativeFn, VmInner};
use super::events::install_ctor;

pub(crate) mod clients;
pub(crate) mod event;
pub(crate) mod marshal;

/// Per-`FetchEvent`-`ObjectId` `respondWith` state (SW §4.6.7).  Both the
/// FetchEvent brand and the mutable post-dispatch state the SW loop reads
/// back (DR-C): `responded` guards double / wrong-phase `respondWith`, and
/// `response_promise` is the `Promise.resolve(r)`-wrapped value the loop
/// drains to a `SwResponse` (or network passthrough on reject/timeout).
#[derive(Debug)]
pub(crate) struct FetchEventState {
    /// Whether `respondWith` has been called (double-call → InvalidStateError).
    pub(crate) responded: bool,
    /// The `Promise.resolve(r)` the SW loop polls; `None` until `respondWith`.
    pub(crate) response_promise: Option<ObjectId>,
}

/// Per-`ExtendableEvent`-`ObjectId` `waitUntil` lifetime-promise list
/// (SW §4.4.1).  The ExtendableEvent brand for `waitUntil`; present on
/// install/activate events and on fetch events (FetchEvent : ExtendableEvent).
#[derive(Debug)]
pub(crate) struct ExtendableEventState {
    /// `Promise.resolve(f)` for each `waitUntil(f)`; the lifecycle loop
    /// drains these to decide `LifecycleComplete{success}`.
    pub(crate) lifetime_promises: Vec<ObjectId>,
}

impl VmInner {
    /// Whether this VM is a Service Worker realm.
    pub(crate) fn is_service_worker(&self) -> bool {
        matches!(
            self.global_scope_kind,
            GlobalScopeKind::ServiceWorker { .. }
        )
    }

    /// Install `ServiceWorkerGlobalScope.prototype` (SW §4.1) chaining to
    /// `EventTarget.prototype`, then splice it as `globalThis`'s prototype.
    /// The SW analog of [`register_worker_global_scope_prototype`]
    /// (`host/worker_scope.rs`); installs `skipWaiting` + `importScripts`
    /// (inherited `WorkerGlobalScope` op, SW §4.1) and the SW event-handler
    /// IDL attributes (`oninstall`/`onactivate`/`onfetch`/`onmessage`).
    ///
    /// [`register_worker_global_scope_prototype`]: VmInner::register_worker_global_scope_prototype
    pub(in crate::vm) fn register_service_worker_global_scope_prototype(&mut self) {
        use super::super::shape;
        use super::super::value::{Object, ObjectKind, PropertyStorage};

        let event_target_proto = self.event_target_prototype.expect(
            "register_service_worker_global_scope_prototype called before register_event_target_prototype",
        );
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(event_target_proto),
            extensible: true,
        });
        let methods: &[(&str, NativeFn)] = &[
            ("skipWaiting", native_skip_waiting),
            ("importScripts", super::worker_scope::native_import_scripts),
        ];
        self.install_methods(proto_id, methods);
        // `oninstall`/`onactivate`/`onfetch`/`onmessage` — backed by the
        // `EventListeners` component on the SW scope entity (SW §4.1.5).
        self.install_sw_handler_attrs(proto_id);
        self.service_worker_scope_prototype = Some(proto_id);
    }

    /// Install `self.registration`-independent SW realm globals that need
    /// the `Event.prototype` chain (registered after the scope-kind match):
    /// the `ExtendableEvent` / `FetchEvent` / `Clients` / `Client`
    /// interface prototypes + the `clients` singleton (SW §4.1.1).
    ///
    /// `self.registration` / `self.serviceWorker` (the full
    /// ServiceWorkerRegistration / ServiceWorker objects) are PR-3 — they
    /// need the coordinator→window back-channel (DR-B) PR-2 does not wire.
    pub(in crate::vm) fn register_service_worker_globals(&mut self) {
        event::register_extendable_event_interface(self);
        event::register_fetch_event_interface(self);
        clients::register_clients_interface(self);
        clients::register_client_interface(self);
        clients::install_clients_singleton(self);
    }

    /// Stage an outbound `SwToContent` message for the SW thread loop to
    /// forward over the channel (the SW analog of `worker_outgoing`).
    pub(crate) fn queue_sw_message(&mut self, msg: elidex_api_sw::SwToContent) {
        self.sw_outgoing.push(msg);
    }

    /// Replace the SW realm's client snapshot (spawn-payload seed +
    /// `ContentToSw::ClientList`, SW §4.1(3)).
    pub(crate) fn set_sw_clients(&mut self, clients: Vec<elidex_api_sw::ClientSnapshot>) {
        self.sw_clients = clients;
    }
}

/// Install an `IllegalConstructor` interface object wired to `proto` and
/// exposed on `globalThis` under `name` (WebIDL §3.7.1 — the SW event /
/// `Clients` / `Client` interfaces are not script-constructable in this
/// PR; the UA vends every instance).
pub(super) fn install_interface(vm: &mut VmInner, proto: ObjectId, name: &str) {
    let global_sid = vm.strings.intern(name);
    install_ctor(
        vm,
        proto,
        name,
        native_illegal_constructor_unreachable,
        global_sid,
        CallShape::IllegalConstructor,
    );
}

/// `self.skipWaiting()` → `Promise<undefined>` (SW §4.1.4): stage a
/// `SwToContent::SkipWaiting` for the coordinator and resolve immediately.
///
/// Spec resolves once the waiting worker becomes active; without the
/// coordinator→window back-channel (PR-3 / D-26) PR-2 resolves eagerly —
/// the wire signal (`SkipWaiting`) is correct, the resolution *timing* is
/// the deferred part (observably harmless: pages `await skipWaiting()` then
/// proceed).
fn native_skip_waiting(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    ctx.vm
        .queue_sw_message(elidex_api_sw::SwToContent::SkipWaiting);
    let promise = super::super::natives_promise::create_promise(ctx.vm);
    super::blob::resolve_promise_sync(ctx.vm, promise, JsValue::Undefined);
    Ok(JsValue::Object(promise))
}
