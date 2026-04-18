//! `AbortController` / `AbortSignal` primitives (WHATWG DOM §3.1).
//!
//! `AbortSignal` is an `EventTarget` that is *not* a `Node`.  Its
//! prototype chain therefore mirrors `Window`'s:
//!
//! ```text
//! AbortSignal instance (HostObject-style wrapper, ObjectKind::AbortSignal)
//!   → AbortSignal.prototype       (this module)
//!     → EventTarget.prototype     (no Node members)
//!       → Object.prototype
//! ```
//!
//! Skipping `Node.prototype` keeps `signal.parentNode` etc. resolving
//! to `undefined`, matching the Web platform.
//!
//! ## State storage
//!
//! Per-signal mutable state ([`AbortSignalState`]) lives **out of band**
//! in [`super::super::VmInner::abort_signal_states`], keyed by the
//! signal's own `ObjectId`.  The variant
//! [`super::super::value::ObjectKind::AbortSignal`] is payload-free so
//! the per-variant size discipline of [`super::super::value::ObjectKind`]
//! is preserved.  GC traces the state via the HashMap (see `gc.rs`)
//! and prunes dead entries after sweep.
//!
//! ## Listener model
//!
//! Unlike DOM EventTargets backed by an ECS entity, `AbortSignal`
//! manages its `'abort'` listeners in `AbortSignalState::abort_listeners`.
//! There is no entity, so [`super::event_target::native_event_target_add_event_listener`]
//! cannot store anything via `HostData::store_listener`.  Instead this
//! module shadows the inherited `addEventListener` /
//! `removeEventListener` / `dispatchEvent` with versions that touch
//! the in-VM listener Vec.  The `'abort'` event fires exactly once on
//! the first `controller.abort()` call.
//!
//! ## Scope (PR4d)
//!
//! Implemented:
//! - `new AbortController()` → object with `.signal` and `.abort()`.
//! - `signal.aborted` / `signal.reason` / `signal.onabort`.
//! - `signal.throwIfAborted()`.
//! - `signal.addEventListener('abort', cb)` /
//!   `signal.removeEventListener(...)`.
//! - `controller.abort(reason?)` — synchronously sets state and
//!   dispatches `'abort'` to every registered listener and the
//!   `onabort` slot.  Idempotent; second call is a no-op.
//! - PR4d C3 hook: [`AbortSignalState::bound_listener_removals`]
//!   stores `(entity, listener_id)` pairs that the
//!   `addEventListener({signal})` flow pushes here, so the back-ref
//!   list can be drained on `abort()` to detach the listener from
//!   its host's ECS `EventListeners` component.
//!
//! Deferred to PR5a (alongside `Event` constructors and `fetch`):
//! - `AbortSignal.abort(reason)` static factory.
//! - `AbortSignal.timeout(ms)` static factory.
//! - `AbortSignal.any(signals)` (recent WHATWG addition).

#![cfg(feature = "engine")]

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};

use elidex_ecs::Entity;
use elidex_script_session::ListenerId;

/// Per-signal mutable state, owned by [`super::super::VmInner::abort_signal_states`]
/// and looked up via the signal's `ObjectId`.
#[derive(Debug)]
pub(crate) struct AbortSignalState {
    /// `true` once `controller.abort()` has fired.  Latches — flipping
    /// back to `false` is not a spec-defined operation.
    pub(crate) aborted: bool,
    /// Reason supplied to `abort(reason)`, or the default
    /// `AbortError`-tagged Error created when the call omitted one.
    /// Reads `undefined` while `aborted == false`.
    pub(crate) reason: JsValue,
    /// Single `onabort` IDL handler slot, written via `signal.onabort = fn`.
    /// Spec §3.1: when `'abort'` fires, the `onabort` handler runs
    /// alongside any addEventListener-registered `'abort'` callbacks.
    /// PR4d invokes `onabort` first, then the listener Vec.
    pub(crate) onabort: Option<ObjectId>,
    /// Callbacks registered via `signal.addEventListener('abort', cb)`.
    /// Fires exactly once on first abort, then cleared.
    pub(crate) abort_listeners: Vec<ObjectId>,
    /// Back-references for `addEventListener(type, cb, {signal})` on
    /// other EventTargets — when this signal aborts, the runtime
    /// removes each `(entity, listener_id)` from the host's ECS
    /// `EventListeners` component so the listener stops firing.
    /// Populated by PR4d C3.
    pub(crate) bound_listener_removals: Vec<(Entity, ListenerId)>,
}

impl AbortSignalState {
    /// Fresh, never-aborted state with `reason === undefined` and
    /// no listeners.  Hand-rolled instead of `derive(Default)`
    /// because [`JsValue`] does not implement `Default` (no canonical
    /// "empty" value — `Undefined` is the right one in this context
    /// but the trait would force a project-wide policy decision).
    fn new() -> Self {
        Self {
            aborted: false,
            reason: JsValue::Undefined,
            onabort: None,
            abort_listeners: Vec::new(),
            bound_listener_removals: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Registration (called from register_globals)
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `AbortSignal.prototype`, install its native methods /
    /// accessors, and expose the `AbortController` constructor +
    /// `AbortSignal` (non-constructable) globals.
    ///
    /// Called from `register_globals()` **after**
    /// [`Self::register_event_target_prototype`] (the prototype
    /// chains directly to `event_target_prototype`).
    ///
    /// # Panics
    ///
    /// Panics if `event_target_prototype` is `None` — would mean
    /// `register_event_target_prototype` was skipped or run in the
    /// wrong order.
    pub(in crate::vm) fn register_abort_signal_global(&mut self) {
        let event_target_proto = self
            .event_target_prototype
            .expect("register_abort_signal_global called before register_event_target_prototype");

        // ---- AbortSignal.prototype ----
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(event_target_proto),
            extensible: true,
        });
        self.install_abort_signal_accessors(proto_id);
        self.install_abort_signal_methods(proto_id);
        self.abort_signal_prototype = Some(proto_id);

        // ---- AbortSignal global ----
        // WHATWG §3.1: `AbortSignal` *is* listed as a constructor but
        // its body always throws — only the static factories
        // (`AbortSignal.abort` / `AbortSignal.timeout`, PR5a) and
        // `new AbortController().signal` produce instances.  Marking
        // it `create_constructable_function` is what lets `new
        // AbortSignal()` reach our throw site (otherwise `do_new`
        // would short-circuit with "X is not a constructor").
        let abort_signal_ctor =
            self.create_constructable_function("AbortSignal", native_abort_signal_constructor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            abort_signal_ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(abort_signal_ctor)),
            PropertyAttrs::METHOD,
        );
        let abort_signal_name = self.well_known.abort_signal;
        self.globals
            .insert(abort_signal_name, JsValue::Object(abort_signal_ctor));

        // ---- AbortController.prototype + global ----
        let ctrl_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: self.object_prototype,
            extensible: true,
        });
        // `abort` is a method; `signal` is a per-instance own data
        // property set by the constructor (not a prototype accessor),
        // because each controller owns a unique signal.
        let abort_fn = self.create_native_function("abort", native_abort_controller_abort);
        let abort_key = PropertyKey::String(self.well_known.abort);
        self.define_shaped_property(
            ctrl_proto,
            abort_key,
            PropertyValue::Data(JsValue::Object(abort_fn)),
            PropertyAttrs::METHOD,
        );

        let ctrl_ctor = self
            .create_constructable_function("AbortController", native_abort_controller_constructor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctrl_ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(ctrl_proto)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            ctrl_proto,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctrl_ctor)),
            PropertyAttrs::METHOD,
        );
        let ac_name = self.well_known.abort_controller;
        self.globals.insert(ac_name, JsValue::Object(ctrl_ctor));
    }

    fn install_abort_signal_accessors(&mut self, proto_id: ObjectId) {
        // `aborted` and `reason` are RO accessors; `onabort` is RW.
        for (name_sid, getter, setter) in [
            (
                self.well_known.aborted,
                native_abort_signal_get_aborted as NativeFn,
                None::<NativeFn>,
            ),
            (
                self.well_known.reason,
                native_abort_signal_get_reason as NativeFn,
                None::<NativeFn>,
            ),
            (
                self.well_known.onabort,
                native_abort_signal_get_onabort as NativeFn,
                Some(native_abort_signal_set_onabort as NativeFn),
            ),
        ] {
            let name = self.strings.get_utf8(name_sid);
            let gid = self.create_native_function(&format!("get {name}"), getter);
            let sid = setter.map(|s| self.create_native_function(&format!("set {name}"), s));
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Accessor {
                    getter: Some(gid),
                    setter: sid,
                },
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }

    fn install_abort_signal_methods(&mut self, proto_id: ObjectId) {
        // `addEventListener` / `removeEventListener` / `dispatchEvent`
        // shadow the inherited EventTarget methods because the listener
        // store is in-VM, not ECS-attached.  `throwIfAborted` is a
        // signal-specific method.
        //
        // String IDs for the three EventTarget method names are not
        // pre-cached on `WellKnownStrings` (the EventTarget prototype
        // installer interns them lazily).  Re-interning here is a
        // HashMap *hit* because `register_event_target_prototype`
        // ran earlier in `register_globals` — the StringId returned
        // matches the one already on `EventTarget.prototype`, so
        // overriding via shape lookup works correctly.
        let add_sid = self.strings.intern("addEventListener");
        let remove_sid = self.strings.intern("removeEventListener");
        let dispatch_sid = self.strings.intern("dispatchEvent");
        for (name_sid, func) in [
            (add_sid, native_abort_signal_add_event_listener as NativeFn),
            (remove_sid, native_abort_signal_remove_event_listener),
            (dispatch_sid, native_abort_signal_dispatch_event),
            (
                self.well_known.throw_if_aborted,
                native_abort_signal_throw_if_aborted,
            ),
        ] {
            let name = self.strings.get_utf8(name_sid);
            let fn_id = self.create_native_function(&name, func);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
        }
    }

    /// Allocate a fresh `AbortSignal` instance with its state row
    /// installed in [`Self::abort_signal_states`].  Used by the
    /// `AbortController` constructor — never directly callable from
    /// JS (the `new AbortSignal()` path throws TypeError).
    pub(in crate::vm) fn create_abort_signal(&mut self) -> ObjectId {
        let proto = self.abort_signal_prototype;
        let id = self.alloc_object(Object {
            kind: ObjectKind::AbortSignal,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: proto,
            extensible: true,
        });
        self.abort_signal_states.insert(id, AbortSignalState::new());
        id
    }
}

// ---------------------------------------------------------------------------
// AbortController constructor
// ---------------------------------------------------------------------------

/// `new AbortController()` — allocates the controller object and a
/// paired `AbortSignal`, exposing the signal as `controller.signal`.
fn native_abort_controller_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "AbortController constructor cannot be invoked without 'new'",
        ));
    }
    // `do_new` already pre-allocated an Ordinary instance whose
    // prototype is `AbortController.prototype` — repurpose it so the
    // chain is correct without a second alloc.
    let ctrl_id = match this {
        JsValue::Object(id) => id,
        _ => unreachable!("constructor `this` is always an Object after `do_new`"),
    };
    let signal_id = ctx.vm.create_abort_signal();
    let signal_key = PropertyKey::String(ctx.vm.well_known.signal);
    // WHATWG: `signal` is an own property on the controller, RO and
    // configurable (matches WebIDL `[[Reflect]]` reflection).
    ctx.vm.define_shaped_property(
        ctrl_id,
        signal_key,
        PropertyValue::Data(JsValue::Object(signal_id)),
        PropertyAttrs::WEBIDL_RO,
    );
    Ok(JsValue::Object(ctrl_id))
}

/// `controller.abort(reason?)` — sets the paired signal's state and
/// fires `'abort'` exactly once.
fn native_abort_controller_abort(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(ctrl_id) = this else {
        return Err(VmError::type_error(
            "AbortController.prototype.abort called on non-object",
        ));
    };
    // Locate the paired signal via own property `signal`.
    let signal_key = PropertyKey::String(ctx.vm.well_known.signal);
    let signal_val = match ctx
        .vm
        .get_object(ctrl_id)
        .storage
        .get(signal_key, &ctx.vm.shapes)
    {
        Some((PropertyValue::Data(v), _)) => *v,
        _ => {
            return Err(VmError::type_error(
                "AbortController.prototype.abort called on object without a signal",
            ))
        }
    };
    let JsValue::Object(signal_id) = signal_val else {
        return Err(VmError::type_error(
            "AbortController.prototype.abort: signal is not an object",
        ));
    };
    let reason = args.first().copied().unwrap_or(JsValue::Undefined);
    abort_signal(ctx, signal_id, reason)?;
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// AbortSignal constructor (always throws — non-constructable per spec)
// ---------------------------------------------------------------------------

fn native_abort_signal_constructor(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // WHATWG §3.1: `new AbortSignal()` throws.  PR5a will add the
    // `AbortSignal.abort(reason)` / `AbortSignal.timeout(ms)` static
    // factories — until then, the only way to obtain one is via
    // `new AbortController().signal`.
    Err(VmError::type_error("AbortSignal is not constructable"))
}

// ---------------------------------------------------------------------------
// AbortSignal accessors
// ---------------------------------------------------------------------------

/// Resolve `this` to an `AbortSignal` ObjectId.  Returns a
/// `TypeError` for any other receiver — accessor / method invocations
/// off non-signal `this` (e.g. `Object.getOwnPropertyDescriptor(AbortSignal.prototype, 'aborted').get.call({})`)
/// must not silently produce `undefined`.
fn require_abort_signal_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "AbortSignal.prototype.{method} called on non-AbortSignal"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::AbortSignal) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "AbortSignal.prototype.{method} called on non-AbortSignal"
        )))
    }
}

fn native_abort_signal_get_aborted(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_abort_signal_this(ctx, this, "aborted")?;
    let aborted = ctx
        .vm
        .abort_signal_states
        .get(&id)
        .map_or(false, |s| s.aborted);
    Ok(JsValue::Boolean(aborted))
}

fn native_abort_signal_get_reason(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_abort_signal_this(ctx, this, "reason")?;
    let reason = ctx
        .vm
        .abort_signal_states
        .get(&id)
        .map_or(JsValue::Undefined, |s| s.reason);
    Ok(reason)
}

fn native_abort_signal_get_onabort(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_abort_signal_this(ctx, this, "onabort")?;
    let handler = ctx.vm.abort_signal_states.get(&id).and_then(|s| s.onabort);
    Ok(handler.map_or(JsValue::Null, JsValue::Object))
}

fn native_abort_signal_set_onabort(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_abort_signal_this(ctx, this, "onabort")?;
    let value = args.first().copied().unwrap_or(JsValue::Undefined);
    // WHATWG event-handler IDL: `null` clears, callable installs,
    // anything else (object without [[Call]], primitive) is
    // silently ignored — matches Web platform behaviour where
    // `el.onclick = 'foo'` no-ops rather than throwing.
    let new_handler = match value {
        JsValue::Null | JsValue::Undefined => None,
        JsValue::Object(obj_id) if ctx.vm.get_object(obj_id).kind.is_callable() => Some(obj_id),
        _ => return Ok(JsValue::Undefined),
    };
    if let Some(state) = ctx.vm.abort_signal_states.get_mut(&id) {
        state.onabort = new_handler;
    }
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// AbortSignal methods
// ---------------------------------------------------------------------------

/// `signal.throwIfAborted()` — WHATWG §3.1: if aborted, throw the
/// stored reason; otherwise no-op.
fn native_abort_signal_throw_if_aborted(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_abort_signal_this(ctx, this, "throwIfAborted")?;
    let state = ctx.vm.abort_signal_states.get(&id);
    if let Some(s) = state {
        if s.aborted {
            return Err(VmError::throw(s.reason));
        }
    }
    Ok(JsValue::Undefined)
}

/// `signal.addEventListener(type, callback)`.  Only `'abort'` is
/// meaningful — other types are accepted but their callbacks will
/// never fire (matches browsers).
fn native_abort_signal_add_event_listener(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_abort_signal_this(ctx, this, "addEventListener")?;
    let type_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let type_sid = super::super::coerce::to_string(ctx.vm, type_arg)?;
    // Filter: only register `'abort'` listeners — anything else is
    // accepted (no throw) but discarded, since the only event this
    // signal ever dispatches is `'abort'`.
    let abort_sid = ctx.vm.well_known.abort;

    let callback_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let cb_id = match callback_arg {
        JsValue::Null | JsValue::Undefined => return Ok(JsValue::Undefined),
        JsValue::Object(cb) if ctx.vm.get_object(cb).kind.is_callable() => cb,
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'addEventListener' on 'EventTarget': \
                 parameter 2 is not of type 'EventListener'.",
            ));
        }
    };
    if type_sid != abort_sid {
        return Ok(JsValue::Undefined);
    }
    if let Some(state) = ctx.vm.abort_signal_states.get_mut(&id) {
        // Already-aborted signal: spec says the callback is queued
        // immediately as a microtask, but PR4d's MVP simply skips
        // registration (aborts are one-shot — re-registering after
        // the fact is a no-op even in browsers).  Full microtask
        // queueing lands in PR5a alongside `AbortSignal.abort(reason)`.
        if state.aborted {
            return Ok(JsValue::Undefined);
        }
        // Spec §2.6 step 4 forbids duplicate (type, callback, capture)
        // tuples.  AbortSignal listeners share `type='abort'` and
        // `capture=false`, so dedupe on callback identity alone.
        if !state.abort_listeners.contains(&cb_id) {
            state.abort_listeners.push(cb_id);
        }
    }
    Ok(JsValue::Undefined)
}

fn native_abort_signal_remove_event_listener(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_abort_signal_this(ctx, this, "removeEventListener")?;
    let type_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let type_sid = super::super::coerce::to_string(ctx.vm, type_arg)?;
    let abort_sid = ctx.vm.well_known.abort;
    let callback_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(cb_id) = callback_arg else {
        return Ok(JsValue::Undefined);
    };
    if type_sid != abort_sid {
        return Ok(JsValue::Undefined);
    }
    if let Some(state) = ctx.vm.abort_signal_states.get_mut(&id) {
        state.abort_listeners.retain(|&c| c != cb_id);
    }
    Ok(JsValue::Undefined)
}

fn native_abort_signal_dispatch_event(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // PR4d MVP: dispatching arbitrary events into a signal is a
    // no-op — the runtime only ever dispatches `'abort'` from
    // `controller.abort()`, and that path doesn't go through
    // `dispatchEvent`.  Full implementation lands in PR5a alongside
    // `new Event(...)` (the only meaningful way to construct the
    // argument from script).  Returns `false` matching WHATWG's
    // "event not dispatched" default.
    Ok(JsValue::Boolean(false))
}

// ---------------------------------------------------------------------------
// Internal abort dispatch
// ---------------------------------------------------------------------------

/// Fire `'abort'` on `signal_id`: set state, then call every
/// registered listener exactly once (idempotent if already aborted).
///
/// Listeners are called with `(undefined)` as the sole argument in
/// PR4d — the proper Event object construction (`new AbortEvent`-like
/// payload) lands in PR5a once the Event constructor surface is in
/// place.  Browsers do build a real Event here, but every listener
/// observed during PR4d testing only inspects `signal.aborted` /
/// `signal.reason`, both of which are stable on the signal itself.
///
/// The abort listeners + onabort handler are cleared after firing to
/// implement WHATWG's one-shot semantics — re-aborting is a no-op
/// because the listener list is empty.
fn abort_signal(
    ctx: &mut NativeContext<'_>,
    signal_id: ObjectId,
    reason: JsValue,
) -> Result<(), VmError> {
    // Already-aborted signals are a spec no-op (idempotent).
    let already = ctx
        .vm
        .abort_signal_states
        .get(&signal_id)
        .is_some_and(|s| s.aborted);
    if already {
        return Ok(());
    }

    // Materialise the abort reason: `undefined` becomes a fresh
    // Error with `name === "AbortError"` and a default message.  Full
    // `DOMException` (the spec-correct reason type) lands in PR5a;
    // until then a plain Error with the right `name` satisfies every
    // PR4d test path and matches common library detection
    // (`err.name === 'AbortError'`).
    let materialised_reason = if matches!(reason, JsValue::Undefined) {
        create_default_abort_error(ctx)
    } else {
        reason
    };

    // Snapshot listeners + handler, then clear state — calling user
    // code while holding the borrow would re-enter via
    // `signal.addEventListener` etc., and we want the new
    // registration to be ignored (the signal already aborted).
    let (onabort, listeners, removals) = {
        let Some(state) = ctx.vm.abort_signal_states.get_mut(&signal_id) else {
            return Ok(());
        };
        state.aborted = true;
        state.reason = materialised_reason;
        let onabort = state.onabort.take();
        let listeners = std::mem::take(&mut state.abort_listeners);
        let removals = std::mem::take(&mut state.bound_listener_removals);
        (onabort, listeners, removals)
    };

    // Detach back-referenced listeners (set up via PR4d C3
    // `addEventListener({signal})`) from their host's ECS
    // `EventListeners` component + `HostData::listener_store`, so
    // subsequent dispatches skip them.  Errors from a despawned
    // entity are silently absorbed (the listener is already gone).
    detach_bound_listeners(ctx, &removals);

    // Fire `onabort` first (matches WHATWG §8.1.5 — event handler
    // attribute is "the first in addition to others registered").
    let signal_val = JsValue::Object(signal_id);
    if let Some(handler) = onabort {
        // Per PR4d MVP, the listener gets `undefined` as the event
        // argument; PR5a will swap in a properly-constructed
        // AbortEvent when the Event constructor lands.
        let _ = ctx.call_function(handler, signal_val, &[JsValue::Undefined]);
    }
    for cb in listeners {
        let _ = ctx.call_function(cb, signal_val, &[JsValue::Undefined]);
    }
    Ok(())
}

/// Construct the default abort reason — an `Error` instance with
/// `name === "AbortError"` and a generic message.  Mirrors the
/// own-property layout `error_ctor_impl` produces (so `JSON.stringify`,
/// `Object.keys`, `e.toString()` all behave the same way).  PR5a will
/// promote this to a real `DOMException` once that interface lands.
fn create_default_abort_error(ctx: &mut NativeContext<'_>) -> JsValue {
    let proto = ctx.vm.error_prototype;
    let id = ctx.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    let name_key = PropertyKey::String(ctx.vm.well_known.name);
    let name_val = JsValue::String(ctx.vm.well_known.abort_error);
    ctx.vm.define_shaped_property(
        id,
        name_key,
        PropertyValue::Data(name_val),
        PropertyAttrs::METHOD,
    );
    let msg_sid = ctx.intern("signal is aborted without reason");
    let msg_key = PropertyKey::String(ctx.vm.well_known.message);
    ctx.vm.define_shaped_property(
        id,
        msg_key,
        PropertyValue::Data(JsValue::String(msg_sid)),
        PropertyAttrs::METHOD,
    );
    JsValue::Object(id)
}

/// Detach `(entity, listener_id)` pairs from their host's ECS
/// `EventListeners` component and the `HostData::listener_store`.
/// Used when an `AbortSignal` aborts to drop listeners registered via
/// `addEventListener({signal})` (PR4d C3).
fn detach_bound_listeners(ctx: &mut NativeContext<'_>, removals: &[(Entity, ListenerId)]) {
    if removals.is_empty() || ctx.host_if_bound().is_none() {
        return;
    }
    for &(entity, listener_id) in removals {
        // Drop from ECS first (scoped block so the world borrow
        // releases before we re-grab `host` for listener_store).
        {
            let dom = ctx.host().dom();
            if let Ok(mut listeners) = dom
                .world_mut()
                .get::<&mut elidex_script_session::EventListeners>(entity)
            {
                listeners.remove(listener_id);
            }
        }
        ctx.host().remove_listener(listener_id);
    }
}
