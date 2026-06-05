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
    native_illegal_constructor_unreachable, CallShape, JsValue, NativeContext, Object, ObjectId,
    ObjectKind, PropertyStorage, StringId, VmError,
};
use super::super::wrapper_intern::{WrapperKey, WrapperKind};
use super::super::{GlobalScopeKind, NativeFn, VmInner};
use super::events::install_ctor;

pub(crate) mod clients;
pub(crate) mod container;
pub(crate) mod deliver;
pub(crate) mod event;
pub(crate) mod marshal;
pub(crate) mod registration;
pub(crate) mod worker;

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

/// Per-realm `ServiceWorkerRegistration` registry entry (SW §3.2) for the
/// window-realm `navigator.serviceWorker` client (D-19 PR-3).  Keyed in
/// [`VmInner::sw_registrations`] by the canonical scope string
/// (`Url::as_str()`); the authoritative live set the container accessors +
/// the GC registry-walk mark loop read.
#[derive(Debug, Clone)]
pub(crate) struct SwRegistrationEntry {
    /// The interned canonical scope — the `WrapperOwner::Scope` identity for
    /// this registration's wrappers AND the GC registry-walk key (stored so the
    /// mark loop need not re-intern while it holds the wrapper-store borrow).
    pub(crate) scope_sid: super::super::value::StringId,
    /// `ServiceWorkerRegistration.updateViaCache` (SW §3.2.7), default "imports".
    pub(crate) update_via_cache: elidex_api_sw::UpdateViaCache,
    /// The registration's single current worker (one worker per scope in the
    /// shell's single-`SwState` model); `None` until a `Registered` deliver.
    pub(crate) worker: Option<elidex_api_sw::SwWorkerSnapshot>,
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

    /// Install the **window-realm** `navigator.serviceWorker` client surface
    /// (WHATWG SW §3.1/§3.2/§3.4; D-19 PR-3): the `ServiceWorker` /
    /// `ServiceWorkerRegistration` interface prototypes + the
    /// `ServiceWorkerContainer` prototype and its eagerly-built singleton
    /// (NG-5 — listeners must exist before a pre-access deliver).  Called from
    /// `register_globals()` (Window scope only) just before
    /// `register_navigator_global`, which exposes the singleton as
    /// `navigator.serviceWorker`.
    pub(in crate::vm) fn register_service_worker_client(&mut self) {
        worker::register_service_worker_interface(self);
        registration::register_service_worker_registration_interface(self);
        container::register_service_worker_container(self);
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

    /// Stage an outbound `navigator.serviceWorker` client request (D-19 PR-3,
    /// SW §3.2/§3.4) for the content event loop to forward to the coordinator
    /// (D-26) — the window-realm twin of [`Self::queue_sw_message`].
    #[cfg(feature = "engine")]
    pub(crate) fn queue_sw_client_request(&mut self, req: elidex_api_sw::SwClientRequest) {
        self.sw_client_outgoing.push(req);
    }

    /// Take the staged outbound SW client requests (the harness asserts on
    /// these; D-26 forwards them as `ContentToBrowser` IPC).
    #[cfg(feature = "engine")]
    pub(crate) fn drain_sw_client_requests(&mut self) -> Vec<elidex_api_sw::SwClientRequest> {
        std::mem::take(&mut self.sw_client_outgoing)
    }

    /// Deliver an inbound `navigator.serviceWorker` back-channel update (DR-B';
    /// the body is [`deliver::deliver_sw_client_update`]).  The 7th member of
    /// the `vm_api.rs` `deliver_*` family: a silent no-op post-unbind (no JS
    /// runs while unbound), then its **own trailing microtask checkpoint** so a
    /// `.then` chained from a settled `register()` / a fired `statechange`
    /// runs before the call returns.
    #[cfg(feature = "engine")]
    pub(crate) fn deliver_sw_client_update(&mut self, update: elidex_api_sw::SwClientUpdate) {
        if !self
            .host_data
            .as_deref()
            .is_some_and(super::super::host_data::HostData::is_bound)
        {
            return;
        }
        {
            let mut ctx = NativeContext::new_call(self);
            deliver::deliver_sw_client_update(&mut ctx, update);
        }
        self.drain_microtasks();
    }

    /// Seed the initial `navigator.serviceWorker` controller + registrations a
    /// page is controlled by AT navigation (SW §3.4.1, F2 construction-init
    /// seed) — written to the VM-level container state at t0, *before* any
    /// runtime deliver, so a controlled page shows `controller` / `ready` /
    /// `getRegistration` immediately.  No events fire (it is the t0 state, not
    /// a transition).  Production write-path = D-26; the harness seeds directly.
    #[cfg(feature = "engine")]
    pub(crate) fn seed_sw_client(
        &mut self,
        controller: Option<String>,
        registrations: Vec<(String, elidex_api_sw::SwWorkerSnapshot)>,
    ) {
        for (scope, worker) in registrations {
            let scope_sid = self.strings.intern(&scope);
            self.sw_registrations.insert(
                scope.clone(),
                SwRegistrationEntry {
                    scope_sid,
                    update_via_cache: elidex_api_sw::UpdateViaCache::default(),
                    worker: Some(worker),
                },
            );
        }
        self.sw_controller_scope = controller;
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
    Ok(JsValue::Object(promise_resolve(ctx.vm, JsValue::Undefined)))
}

/// A fresh promise resolved with `value` — the SW realm's `Promise.resolve`
/// (`value` is followed if it is itself a promise, else fulfilled
/// immediately; reactions fire on the next `drain_microtasks` in the SW
/// loop).  `value` is rooted across `create_promise` (which can GC) so a
/// freshly built `Response` / `Client` held only in this local is not swept
/// before it reaches the promise.  Shared by `skipWaiting` / `clients.*` /
/// `respondWith` / `waitUntil`.
pub(super) fn promise_resolve(vm: &mut VmInner, value: JsValue) -> ObjectId {
    let mut g = vm.push_temp_root(value);
    let promise = super::super::natives_promise::create_promise(&mut g);
    let mut g2 = g.push_temp_root(JsValue::Object(promise));
    super::blob::resolve_promise_sync(&mut g2, promise, value);
    drop(g2);
    drop(g);
    promise
}

/// A fresh promise rejected with `reason` — the reject mirror of
/// [`promise_resolve`].  `reason` (a thrown exception value, not a thenable)
/// is rooted across `create_promise` (which can GC).  Used by the
/// `navigator.serviceWorker` natives to turn a synchronous registration
/// failure into a rejected Promise (WebIDL §3.7.7), and never follows
/// `reason` as a thenable.
#[cfg(feature = "engine")]
pub(super) fn promise_reject(vm: &mut VmInner, reason: JsValue) -> ObjectId {
    let mut g = vm.push_temp_root(reason);
    let promise = super::super::natives_promise::create_promise(&mut g);
    drop(g);
    settle_rooted(vm, promise, true, reason);
    promise
}

/// Settle an existing `promise` with `value`, rooting BOTH across
/// `settle_promise` (which runs reactions / may GC, so a value reachable only
/// through this local must survive).  The single rooted-settle body shared by
/// the `register()`/`unregister()` deliver settles + the `ready` resolve +
/// [`promise_reject`].
#[cfg(feature = "engine")]
pub(super) fn settle_rooted(vm: &mut VmInner, promise: ObjectId, is_reject: bool, value: JsValue) {
    let mut g = vm.push_temp_root(value);
    let mut g2 = g.push_temp_root(JsValue::Object(promise));
    let _ = super::super::natives_promise::settle_promise(&mut g2, promise, is_reject, value);
    drop(g2);
    drop(g);
}

// ---------------------------------------------------------------------------
// Shared `navigator.serviceWorker` client helpers (D-19 PR-3)
// ---------------------------------------------------------------------------

/// A rejected Promise carrying `err`'s thrown value (WebIDL §3.7.7 — a
/// promise-returning operation surfaces synchronous failures as a *rejected*
/// Promise, not a thrown exception).
pub(super) fn reject_promise(vm: &mut VmInner, err: &VmError) -> ObjectId {
    let reason = vm.vm_error_to_thrown(err);
    promise_reject(vm, reason)
}

/// Allocate a fresh interface-prototype object chaining to `Object.prototype`
/// (shared by the container / registration / worker prototype installs).
pub(super) fn alloc_client_prototype(vm: &mut VmInner) -> ObjectId {
    let parent = vm.object_prototype;
    vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: parent,
        extensible: true,
    })
}

/// Install a readonly WebIDL attribute getter on an SW client interface
/// prototype.
pub(super) fn install_ro_getter(vm: &mut VmInner, proto: ObjectId, name: &str, getter: NativeFn) {
    let sid = vm.strings.intern(name);
    vm.install_accessor_pair(
        proto,
        sid,
        getter,
        None,
        super::super::shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
    );
}

/// Map an engine-indep [`elidex_api_sw::SwRegisterError`] to its VM exception
/// (SW §3.4.3 register) — the pure 1:1 WebIDL mapping the native owes; every
/// scheme/origin/scope-path/secure decision stays in `elidex-api-sw`.
pub(super) fn map_sw_register_error(vm: &VmInner, err: &elidex_api_sw::SwRegisterError) -> VmError {
    match err {
        elidex_api_sw::SwRegisterError::TypeError(m) => VmError::type_error(m.clone()),
        elidex_api_sw::SwRegisterError::SecurityError(m) => {
            VmError::dom_exception(vm.well_known.dom_exc_security_error, m.clone())
        }
    }
}

/// Get-or-create the per-realm `ServiceWorkerRegistration` wrapper for a
/// canonical `scope` (interned by `scope_sid`) — one object per scope so
/// `reg === getRegistration()` (SW §3.2 registration object map).  Allocates +
/// registers the `sw_registration_states` brand on first access only.
pub(super) fn registration_object(vm: &mut VmInner, scope: &str, scope_sid: StringId) -> ObjectId {
    let key = WrapperKey::scope(scope_sid, WrapperKind::ServiceWorkerRegistration);
    let scope_owned = scope.to_owned();
    vm.intern_wrapper(key, |vm| {
        let proto = vm.sw_registration_prototype;
        let id = vm.alloc_object(Object {
            kind: ObjectKind::ServiceWorkerRegistration,
            storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
            prototype: proto,
            extensible: true,
        });
        vm.sw_registration_states.insert(id, scope_owned);
        id
    })
}

/// Get-or-create the per-realm `ServiceWorker` wrapper for a canonical `scope`
/// (interned by `scope_sid`) — one object per scope so `reg.active ===
/// controller` and identity survives state transitions (SW §3.1 service worker
/// object map).  Allocates + registers the `service_worker_states` brand on
/// first access only.
pub(super) fn worker_object(vm: &mut VmInner, scope: &str, scope_sid: StringId) -> ObjectId {
    let key = WrapperKey::scope(scope_sid, WrapperKind::ServiceWorker);
    let scope_owned = scope.to_owned();
    vm.intern_wrapper(key, |vm| {
        let proto = vm.sw_worker_prototype;
        let id = vm.alloc_object(Object {
            kind: ObjectKind::ServiceWorker,
            storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
            prototype: proto,
            extensible: true,
        });
        vm.service_worker_states.insert(id, scope_owned);
        id
    })
}

/// Resolve the `navigator.serviceWorker.ready` promise (SW §3.4.2) with the
/// registration for `scope`, if one has been requested and is still pending.
/// `settle_promise` is idempotent, so a later active worker is a no-op (ready
/// resolves once, with the first active registration).
pub(super) fn resolve_sw_ready(vm: &mut VmInner, scope: &str, scope_sid: StringId) {
    let Some(promise) = vm.sw_ready_promise else {
        return;
    };
    let reg = registration_object(vm, scope, scope_sid);
    settle_rooted(vm, promise, false, JsValue::Object(reg));
}
