//! Event-handler IDL attribute machinery (WHATWG HTML §8.1.8).
//!
//! Implements `el.onclick = fn` / `el.onclick` getter / inline
//! `<button onclick="...">` / `<body>.onbeforeunload`→Window delegation
//! on top of the engine-independent [`EventListeners`] component.
//!
//! ## Design (ECS-native)
//!
//! An event handler is a *special kind of event listener* (§8.1.8.1):
//! it lives as one entry in the target's [`EventListeners`] component
//! (`ListenerKind::EventHandler`), so dispatch walks it in registration
//! order alongside `addEventListener` listeners. The single source of
//! truth is that engine-independent component — there is **no** VM-side
//! reverse map. The IDL setter (here), the inline-attribute consumer
//! (`elidex-script-session`), the getter (here), and the dispatch walk
//! (`event_target_dispatch.rs`) all read/write the same component.
//!
//! ## Shared backend, bound key (prereq #211)
//!
//! All ~108 handler attributes install via two backend fn pairs over
//! [`VmInner::install_bound_accessor_pair`], parametrized by the
//! *event type* SID as the bound key (recovered at call time through
//! [`NativeContext::bound_key`]) — not one monomorphized fn per
//! attribute:
//!
//! - **normal pair** ([`native_event_handler_get`] / [`native_event_handler_set`]):
//!   `entity_from_this` is the target. Used for GlobalEventHandlers /
//!   WindowEventHandlers (on Window) / DocumentAndElementEventHandlers /
//!   Document-specific attributes.
//! - **body-delegation pair** ([`native_body_weh_get`] / [`native_body_weh_set`]):
//!   redirects the target to the Window entity (WHATWG HTML §8.1.8.2 —
//!   `<body>.onbeforeunload` reads/writes the Window's handler). Used
//!   only for the WindowEventHandlers overrides installed on
//!   `HTMLBodyElement.prototype`.
//!
//! ## Lazy compile
//!
//! Inline content-attribute handlers are stored as uncompiled source
//! (`ListenerKind::EventHandler { uncompiled: Some(..) }`) by the
//! engine-independent consumer, which never compiles (layering). The
//! source is compiled lazily — at first read (getter) or first dispatch
//! ([`lazy_compile_handler`]) — and an `uncompiled = Some` source always
//! takes precedence over any stale compiled callable (last-write-wins,
//! §8.1.8.1 "getting the current value of the event handler").

#![cfg(feature = "engine")]

use elidex_ecs::NodeKind;
use elidex_script_session::{
    event_handler_attr_event_type, EventListeners, HandlerScope, ListenerId, EVENT_HANDLER_ATTRS,
    WORKER_EVENT_HANDLER_ATTRS, WORKER_OBJECT_EVENT_HANDLER_ATTRS,
};

use super::super::shape::PropertyAttrs;
use super::super::value::{JsValue, NativeContext, ObjectId, VmError};
use super::super::{NativeFn, VmInner};
use super::dispatch_target::{target_from_this, DispatchTarget};
use super::event_target::require_receiver;

// ---------------------------------------------------------------------------
// Install
// ---------------------------------------------------------------------------

impl VmInner {
    /// Install the event-handler IDL attribute accessors whose
    /// [`HandlerScope`] is in `scopes` onto `target` (a prototype or a
    /// per-entity wrapper). Each attribute installs one real accessor
    /// over the shared [`native_event_handler_get`] /
    /// [`native_event_handler_set`] backend pair, keyed by its event
    /// type SID (the bound key). WHATWG HTML §8.1.8.2 / §8.1.8.2.1
    /// (GlobalEventHandlers / DocumentAndElementEventHandlers /
    /// WindowEventHandlers).
    pub(in crate::vm) fn install_event_handler_attrs(
        &mut self,
        target: ObjectId,
        scopes: &[HandlerScope],
    ) {
        self.install_handler_attr_family(
            target,
            scopes,
            native_event_handler_get as NativeFn,
            native_event_handler_set as NativeFn,
        );
    }

    /// Install the `WorkerGlobalScope` event-handler IDL attributes
    /// ([`WORKER_EVENT_HANDLER_ATTRS`]) onto the worker scope prototype, over
    /// the same shared backend pair as the Window/Element handlers. The
    /// receiver brand check ([`require_handler_receiver`]) accepts
    /// [`NodeKind::Worker`], so `self.onmessage = fn` records the handler in
    /// the `EventListeners` component on the worker-scope entity (WHATWG HTML
    /// §10.2.1.1 / §8.1.8.1).
    pub(in crate::vm) fn install_worker_handler_attrs(&mut self, target: ObjectId) {
        self.install_handler_attrs_from_list(target, WORKER_EVENT_HANDLER_ATTRS);
    }

    /// Install the main-side `Worker` object's event-handler IDL attributes
    /// ([`WORKER_OBJECT_EVENT_HANDLER_ATTRS`] = `onmessage` / `onmessageerror`
    /// from the dedicated `Worker` interface + `onerror` from the AbstractWorker
    /// mixin) on `Worker.prototype`, over the same shared backend pair. The
    /// receiver brand check ([`require_handler_receiver`]) accepts
    /// [`NodeKind::Worker`], so `worker.onmessage = fn` records the handler in
    /// the `EventListeners` component on the `Worker` entity (WHATWG HTML
    /// §10.2.6.1 / §10.2.6.3 / §8.1.8.1).
    pub(in crate::vm) fn install_worker_object_handler_attrs(&mut self, target: ObjectId) {
        self.install_handler_attrs_from_list(target, WORKER_OBJECT_EVENT_HANDLER_ATTRS);
    }

    /// Install the `ServiceWorkerGlobalScope` event-handler IDL attributes
    /// (`oninstall` / `onactivate` / `onfetch` / `onmessage`, WHATWG SW
    /// §4.1.5) onto the SW scope prototype, over the same Node-backed
    /// [`native_event_handler_get`] / [`native_event_handler_set`] pair as
    /// the worker handlers (the SW scope entity is a [`NodeKind::Worker`], so
    /// `self.onfetch = fn` records into its `EventListeners` component and
    /// the shared `dispatch_script_event` fires it).
    ///
    /// The SW event names are not in the `GlobalEventHandlers` source-of-
    /// truth, so — like the VmObject installer — the event type is derived
    /// by stripping the `on` prefix rather than consulting
    /// `event_handler_attr_event_type`.
    pub(in crate::vm) fn install_sw_handler_attrs(&mut self, target: ObjectId) {
        const SW_EVENT_HANDLER_ATTRS: &[&str] =
            &["oninstall", "onactivate", "onfetch", "onmessage"];
        for on_name in SW_EVENT_HANDLER_ATTRS {
            let event_type = on_name.strip_prefix("on").unwrap_or(on_name);
            let attr_name_sid = self.strings.intern(on_name);
            let event_type_sid = self.strings.intern(event_type);
            self.install_bound_accessor_pair(
                target,
                attr_name_sid,
                native_event_handler_get as NativeFn,
                Some(native_event_handler_set as NativeFn),
                event_type_sid,
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }

    /// Install a fixed list of event-handler IDL attributes (by attribute
    /// name) on `target` over the shared
    /// [`native_event_handler_get`] / [`native_event_handler_set`] backend,
    /// keyed by each attribute's event-type SID (bound key). Shared by the
    /// `WorkerGlobalScope` and `Worker` surfaces, whose attribute sets are
    /// hand-picked subsets rather than [`HandlerScope`]-tagged rows.
    fn install_handler_attrs_from_list(&mut self, target: ObjectId, attrs: &[&str]) {
        for attr_name in attrs {
            let event_type = event_handler_attr_event_type(attr_name)
                .expect("handler-attr list row must be a known event-handler attribute");
            let attr_name_sid = self.strings.intern(attr_name);
            let event_type_sid = self.strings.intern(event_type);
            self.install_bound_accessor_pair(
                target,
                attr_name_sid,
                native_event_handler_get as NativeFn,
                Some(native_event_handler_set as NativeFn),
                event_type_sid,
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }

    /// Install a list of `on<type>` event-handler IDL attributes on a
    /// **VmObject** EventTarget prototype (IndexedDB / `WebSocket` /
    /// `EventSource`), over the shared
    /// [`native_vm_event_handler_get`] / [`native_vm_event_handler_set`]
    /// backend keyed by each attribute's event type (the name minus
    /// `on`).  The VmObject sibling of [`Self::install_handler_attrs_from_list`]:
    /// that helper is Node/entity-backed and restricted to the
    /// `GlobalEventHandlers` SoT, but VmObject `on*` names
    /// (`onsuccess` / `onopen` / `onclose` / …) are not in that SoT, so
    /// the event type is derived by stripping the `on` prefix. The single
    /// shared VmObject handler-attr installer per plan-memo F3/DR-4a:
    /// IndexedDB's request / database / transaction prototypes call it too
    /// (`onsuccess` / `onversionchange` / `oncomplete` / …).
    pub(in crate::vm) fn install_vm_object_handler_attrs(
        &mut self,
        target: ObjectId,
        attrs: &[&str],
    ) {
        for on_name in attrs {
            let event_type = on_name.strip_prefix("on").unwrap_or(on_name);
            let attr_name_sid = self.strings.intern(on_name);
            let event_type_sid = self.strings.intern(event_type);
            self.install_bound_accessor_pair(
                target,
                attr_name_sid,
                native_vm_event_handler_get as NativeFn,
                Some(native_vm_event_handler_set as NativeFn),
                event_type_sid,
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }

    /// Install the WindowEventHandlers (18 attrs) onto
    /// `HTMLBodyElement.prototype` as **delegation** accessors: their
    /// getter/setter redirect to the Window object rather than the body
    /// element (WHATWG HTML §8.1.8.2 — body/frameset delegate
    /// WindowEventHandlers to the Window). GlobalEventHandlers are
    /// inherited from `HTMLElement.prototype` and not re-installed here.
    pub(in crate::vm) fn install_body_weh_delegation(&mut self, target: ObjectId) {
        self.install_handler_attr_family(
            target,
            &[HandlerScope::Window],
            native_body_weh_get as NativeFn,
            native_body_weh_set as NativeFn,
        );
    }

    /// Shared install loop: for every [`EVENT_HANDLER_ATTRS`] row whose
    /// scope is in `scopes`, intern the attribute-name SID (property key)
    /// and the event-type SID (bound key — derived through the single
    /// SoT helper [`event_handler_attr_event_type`], never an inline
    /// slice), then install the `get`/`set` accessor pair.
    fn install_handler_attr_family(
        &mut self,
        target: ObjectId,
        scopes: &[HandlerScope],
        get: NativeFn,
        set: NativeFn,
    ) {
        for (attr_name, scope) in EVENT_HANDLER_ATTRS {
            if !scopes.contains(scope) {
                continue;
            }
            let event_type = event_handler_attr_event_type(attr_name)
                .expect("EVENT_HANDLER_ATTRS row must be a known event-handler attribute");
            let attr_name_sid = self.strings.intern(attr_name);
            let event_type_sid = self.strings.intern(event_type);
            self.install_bound_accessor_pair(
                target,
                attr_name_sid,
                get,
                Some(set),
                event_type_sid,
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Shared backend (normal): target = `entity_from_this`
// ---------------------------------------------------------------------------

/// Brand check for the normal (non-delegating) handler accessors. The
/// GlobalEventHandlers / DocumentAndElementEventHandlers / Document /
/// WindowEventHandlers IDL surfaces live on `Element`, `Document`, and
/// `Window` (WHATWG HTML §8.1.8.2.1) — restrict the receiver to those
/// node kinds so the accessor cannot be borrowed onto a `Text` / `Attr`
/// node via `.call()`. Mirrors the sibling host accessors' use of
/// [`require_receiver`]: `Ok(None)` for an unbound / non-wrapper receiver
/// (detach-tolerant silent no-op), `Err` ("Illegal invocation") for a
/// bound wrapper of the wrong kind.
fn require_handler_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<elidex_ecs::Entity>, VmError> {
    require_receiver(ctx, this, "EventTarget", method, |kind| {
        matches!(
            kind,
            NodeKind::Element | NodeKind::Document | NodeKind::Window | NodeKind::Worker
        )
    })
}

/// Shared getter for every event-handler IDL attribute (WHATWG HTML
/// §8.1.8.1 "the event handler IDL attributes" — getter / "getting the
/// current value of the event handler"; WebIDL §3.7.6). Recovers its
/// event type from `ctx.bound_key()`. Returns the current callable, or
/// `null` (never `undefined` — an unset handler attribute reads as
/// `null` per the WebIDL `EventHandler?` nullable type).
fn native_event_handler_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let event_type = bound_event_type(ctx);
    let Some(entity) = require_handler_receiver(ctx, this, &format!("on{event_type}"))? else {
        return Ok(JsValue::Null);
    };
    Ok(read_event_handler(
        ctx,
        DispatchTarget::Node(entity),
        &event_type,
    ))
}

/// Shared setter for every event-handler IDL attribute (WHATWG HTML
/// §8.1.8.1 — setter / "activate an event handler"; WebIDL §3.7.6). A
/// callable value activates the handler; any non-callable value
/// (including `null`/`undefined`) clears it to `null` silently (the
/// WebIDL `EventHandler?` conversion does not throw).
fn native_event_handler_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let event_type = bound_event_type(ctx);
    let Some(entity) = require_handler_receiver(ctx, this, &format!("on{event_type}"))? else {
        return Ok(JsValue::Undefined);
    };
    let callable = callable_arg(ctx, args);
    activate_event_handler(ctx, DispatchTarget::Node(entity), &event_type, callable);
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// VmObject backend: target = a non-entity EventTarget (AbortSignal / IDB),
// listeners in `vm_event_listeners` (WHATWG HTML §8.1.8.1)
// ---------------------------------------------------------------------------

/// Resolve `this` to a `VmObject` [`DispatchTarget`] for the VmObject
/// event-handler accessors — `None` (silent no-op: `null` getter / no-op
/// setter) for any non-`VmObject` receiver, matching the node backend's
/// detach-tolerant `require_handler_receiver` policy.
fn vm_handler_target(ctx: &NativeContext<'_>, this: JsValue) -> Option<DispatchTarget> {
    match target_from_this(ctx, this) {
        Some(target @ DispatchTarget::VmObject(_)) => Some(target),
        _ => None,
    }
}

/// Shared getter for a non-entity EventTarget's event-handler IDL
/// attribute (`signal.onabort`, `req.onsuccess`, …).  Recovers its event
/// type from `ctx.bound_key()`; returns the current callable or `null`.
pub(super) fn native_vm_event_handler_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let event_type = bound_event_type(ctx);
    let Some(target) = vm_handler_target(ctx, this) else {
        return Ok(JsValue::Null);
    };
    Ok(read_event_handler(ctx, target, &event_type))
}

/// Shared setter for a non-entity EventTarget's event-handler IDL
/// attribute.  A callable activates the handler; any non-callable
/// (incl. `null`/`undefined`) clears it (WebIDL `EventHandler?`).
pub(super) fn native_vm_event_handler_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let event_type = bound_event_type(ctx);
    let Some(target) = vm_handler_target(ctx, this) else {
        return Ok(JsValue::Undefined);
    };
    let callable = callable_arg(ctx, args);
    activate_event_handler(ctx, target, &event_type, callable);
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Body-delegation backend: target = Window entity (HTML §8.1.8.2)
// ---------------------------------------------------------------------------

/// Brand check for the body-delegation accessors: the receiver must be an
/// `HTMLBodyElement` (WHATWG HTML §8.1.8.2 — only `<body>`/`<frameset>`
/// delegate WindowEventHandlers to the Window). Without this, the
/// accessor would redirect to the Window from *any* receiver via
/// `.call()`. Returns `Ok(None)` for an unbound / non-wrapper receiver
/// (silent), `Err` ("Illegal invocation") for a bound non-`<body>` wrapper.
fn require_body_receiver(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<Option<elidex_ecs::Entity>, VmError> {
    let entity = require_receiver(ctx, this, "HTMLBodyElement", method, |kind| {
        matches!(kind, NodeKind::Element)
    })?;
    if let Some(entity) = entity {
        if !ctx.host().tag_matches_ascii_case(entity, "body") {
            return Err(VmError::type_error(format!(
                "Failed to execute '{method}' on 'HTMLBodyElement': Illegal invocation"
            )));
        }
    }
    Ok(entity)
}

/// `HTMLBodyElement.prototype` WindowEventHandlers getter — brand-checks
/// the `<body>` receiver, then delegates to the Window object (WHATWG HTML
/// §8.1.8.2). No-op (`null`) if no Window is bound.
fn native_body_weh_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let event_type = bound_event_type(ctx);
    if require_body_receiver(ctx, this, &format!("on{event_type}"))?.is_none() {
        return Ok(JsValue::Null);
    }
    let Some(window_entity) = ctx.host().window_entity() else {
        return Ok(JsValue::Null);
    };
    Ok(read_event_handler(
        ctx,
        DispatchTarget::Node(window_entity),
        &event_type,
    ))
}

/// `HTMLBodyElement.prototype` WindowEventHandlers setter — brand-checks
/// the `<body>` receiver, then delegates to the Window object (WHATWG HTML
/// §8.1.8.2). No-op if no Window is bound.
fn native_body_weh_set(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let event_type = bound_event_type(ctx);
    if require_body_receiver(ctx, this, &format!("on{event_type}"))?.is_none() {
        return Ok(JsValue::Undefined);
    }
    let Some(window_entity) = ctx.host().window_entity() else {
        return Ok(JsValue::Undefined);
    };
    let callable = callable_arg(ctx, args);
    activate_event_handler(
        ctx,
        DispatchTarget::Node(window_entity),
        &event_type,
        callable,
    );
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Shared core
// ---------------------------------------------------------------------------

/// The event type this accessor serves, materialized from the bound key
/// (`ctx.bound_key()` = event-type SID, installed by
/// `install_bound_accessor_pair`) to an owned `String` for the
/// engine-independent `&str`-keyed [`EventListeners`] lookups.
fn bound_event_type(ctx: &NativeContext<'_>) -> String {
    let sid = ctx
        .bound_key()
        .expect("event-handler accessor missing bound_key");
    ctx.get_utf8(sid)
}

impl VmInner {
    /// Store `callable` as the compiled callable for handler listener `id`,
    /// replacing any previous one. Event-handler reassignment intentionally
    /// overwrites the slot (unlike `addEventListener`, whose `store_listener`
    /// asserts uniqueness): remove the old entry first so the insert does
    /// not trip that guard. The `listener_store` map is itself the GC root
    /// set, so the dropped callable is correctly unrooted and the new one
    /// rooted.
    pub(in crate::vm) fn set_handler_callable(&mut self, id: ListenerId, callable: ObjectId) {
        if let Some(host) = self.host_data.as_deref_mut() {
            let _ = host.remove_listener(id);
            host.store_listener(id, callable);
        }
    }

    /// Bring listener `id` (an event-handler IDL attribute backing) up to
    /// date before its callable is resolved, on **any** dispatch path
    /// (WHATWG HTML §8.1.8.1 "getting the current value of the event
    /// handler"). If it has a pending inline source, compile it now and
    /// overwrite the stored callable (a parse failure or the cleared flag
    /// drops the stale callable so the handler reads as null). Centralizing
    /// here means the script-dispatch walk, the session-crate UA dispatch
    /// (`ScriptEngine::call_listener`), and the promise-rejection dispatch
    /// all observe inline-source / cleared transitions identically. A
    /// no-op for `Normal` (addEventListener) listeners.
    pub(crate) fn ensure_event_handler_current(
        &mut self,
        entity: elidex_ecs::Entity,
        id: ListenerId,
    ) {
        // WHATWG HTML "getting the current value of the event handler" step 3.2:
        // when the document's active sandboxing flag set has the sandboxed
        // scripts flag set (scripting is disabled, §8.1.3.4), the algorithm
        // returns null for a raw uncompiled inline handler *without* compiling
        // it — so a `<button onclick=...>` in a sandboxed iframe lacking
        // `allow-scripts` never runs on UA dispatch. This is the
        // listener-dispatch / lazy-compile half of the same `scripts_allowed`
        // gate the engine applies to classic-script `eval`. Left raw (not
        // removed): the spec's "return null" does not deactivate the handler,
        // and a fixed-for-lifetime sandbox never re-enables it. addEventListener
        // (`Normal`) listeners hold no uncompiled source, so they are untouched
        // (and none can exist here — the script that would register one was
        // itself blocked by the `eval` gate).
        if self
            .host_data
            .as_deref()
            .is_some_and(|hd| !hd.scripts_allowed())
        {
            return;
        }
        let (uncompiled, cleared) = {
            let Some(host) = self.host_data.as_deref_mut() else {
                return;
            };
            host.dom()
                .world_mut()
                .get::<&mut EventListeners>(entity)
                .ok()
                .map_or((None, false), |mut listeners| {
                    (
                        listeners.take_uncompiled(id).map(|u| u.source),
                        listeners.is_handler_cleared(id),
                    )
                })
        };
        if let Some(source) = uncompiled {
            if let Some(callable) = self.lazy_compile_handler(&source) {
                self.set_handler_callable(id, callable);
            } else {
                self.remove_listener_and_prune_back_ref(id);
            }
        } else if cleared {
            self.remove_listener_and_prune_back_ref(id);
        }
    }
}

/// `args[0]` if it is a callable object, else `None` (the WebIDL
/// `EventHandler?` setter treats any non-callable as `null`).
fn callable_arg(ctx: &NativeContext<'_>, args: &[JsValue]) -> Option<ObjectId> {
    match args.first().copied().unwrap_or(JsValue::Undefined) {
        JsValue::Object(obj) if ctx.vm.get_object(obj).kind.is_callable() => Some(obj),
        _ => None,
    }
}

/// Read the current value of the `(target, event_type)` handler (WHATWG
/// HTML §8.1.8.1 "getting the current value of the event handler") from
/// the target's listener home (via the [`DispatchTarget`] adapter). Brings
/// the handler up to date via the adapter's [`DispatchTarget::reconcile_handler`]
/// (Node-only lazy compile / drop on clear; a no-op for VmObject), then
/// returns the stored callable or `null`.
fn read_event_handler(
    ctx: &mut NativeContext<'_>,
    target: DispatchTarget,
    event_type: &str,
) -> JsValue {
    let Some(id) = target
        .with_listeners(ctx, |listeners| listeners.find_event_handler(event_type))
        .flatten()
    else {
        return JsValue::Null;
    };
    target.reconcile_handler(ctx, id);
    ctx.vm
        .host_data
        .as_deref()
        .and_then(|h| h.get_listener(id))
        .map_or(JsValue::Null, JsValue::Object)
}

/// Activate (`Some`) or clear (`None`) the `(target, event_type)` event
/// handler (WHATWG HTML §8.1.8.1 "activate an event handler" / setter) in
/// the target's listener home (via the [`DispatchTarget`] adapter). The
/// listener entry is added at most once per `(target, event type)` and
/// reused on subsequent writes — the stored callable is what changes.
/// Clearing keeps the entry (registration-order stability) but drops the
/// callable so dispatch skips it.
fn activate_event_handler(
    ctx: &mut NativeContext<'_>,
    target: DispatchTarget,
    event_type: &str,
    callable: Option<ObjectId>,
) {
    if let Some(obj) = callable {
        let id = target.with_listeners_mut_or_insert(ctx, |listeners| {
            let id = listeners
                .find_event_handler(event_type)
                .unwrap_or_else(|| listeners.add_event_handler(event_type.to_string()));
            // IDL write is the last write: any stale inline source is
            // superseded by this fresh compiled callable.
            listeners.clear_uncompiled(id);
            id
        });
        if let Some(id) = id {
            ctx.vm.set_handler_callable(id, obj);
        }
    } else {
        let id = target
            .with_listeners_mut(ctx, |listeners| {
                let id = listeners.find_event_handler(event_type)?;
                listeners.clear_uncompiled(id);
                Some(id)
            })
            .flatten();
        if let Some(id) = id {
            ctx.vm.remove_listener_and_prune_back_ref(id);
        }
    }
}

impl VmInner {
    /// Compile an inline handler body as `function (event) { <body> }` and
    /// return the callable's `ObjectId` (WHATWG HTML §8.1.8.1 "getting the
    /// current value of the event handler" — compile step). Returns `None`
    /// if the body is not parsable (the caller then clears the handler to
    /// `null`). Shared by [`Self::ensure_event_handler_current`] across all
    /// dispatch paths and the getter.
    ///
    /// The body is wrapped in a function expression so a top-level `return`
    /// inside the inline handler (`onsubmit="return false"`) is valid.
    /// Compilation uses `run_script` (not `Vm::eval`) deliberately: `eval`
    /// drains the microtask + same-window task queues, which could re-enter
    /// event dispatch while this runs mid-dispatch; evaluating the function
    /// expression only allocates the closure and never runs user code, so
    /// no queues need draining.
    ///
    /// The special inline-handler scope chain (`with(document)
    /// with(form-owner) with(element)`) is deferred
    /// (`#11-inline-handler-scope-chain`); `event` + `this` = currentTarget
    /// cover the common case. The 5-argument `onerror` signature is deferred
    /// (`#11-onerror-error-event-args`).
    pub(in crate::vm) fn lazy_compile_handler(&mut self, source: &str) -> Option<ObjectId> {
        let wrapped = format!("(function (event) {{\n{source}\n}})");
        let script = crate::compiler::compile_script(&wrapped).ok()?;
        match self.run_script(script) {
            Ok(JsValue::Object(id)) if self.get_object(id).kind.is_callable() => Some(id),
            _ => None,
        }
    }
}
