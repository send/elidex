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
//! ## Implemented
//!
//! - `new AbortController()` → object with `.signal` and `.abort()`.
//! - `signal.aborted` / `signal.reason` / `signal.onabort`.
//! - `signal.throwIfAborted()`.
//! - `signal.addEventListener('abort', cb)` /
//!   `signal.removeEventListener(...)`.
//! - `controller.abort(reason?)` — synchronously sets state and
//!   dispatches `'abort'` to every registered listener and the
//!   `onabort` slot.  Idempotent; second call is a no-op.
//! - `addEventListener({signal})` integration — the EventTarget
//!   path inserts `listener_id → entity` into
//!   [`AbortSignalState::bound_listener_removals`] and writes a
//!   reverse `listener_id → signal_id` index entry on
//!   [`super::super::VmInner::abort_listener_back_refs`].
//!   `removeEventListener` consults the reverse index to prune the
//!   entry in O(1); `abort()` drains the map to detach each
//!   listener from its host's ECS `EventListeners` component.
//!
//! ## Deferred (require Event constructor or `fetch` integration)
//!
//! - `AbortSignal.abort(reason)` static factory.
//! - `AbortSignal.timeout(ms)` static factory.
//! - `AbortSignal.any(signals)` (recent WHATWG addition).

#![cfg(feature = "engine")]

use std::collections::HashMap;

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
    /// alongside any addEventListener-registered `'abort'` callbacks,
    /// in registration order (WHATWG §8.1.5: event-handler IDL
    /// attribute is "first in addition to others registered").
    pub(crate) onabort: Option<ObjectId>,
    /// Callbacks registered via `signal.addEventListener('abort', cb)`.
    /// Fires exactly once on first abort, then cleared.
    pub(crate) abort_listeners: Vec<ObjectId>,
    /// Back-references for `addEventListener(type, cb, {signal})` on
    /// other EventTargets — when this signal aborts, the runtime
    /// removes each `(listener_id → entity)` from the host's ECS
    /// `EventListeners` component so the listener stops firing.
    ///
    /// Stored as a `HashMap` (not `Vec`) so `removeEventListener`'s
    /// scrub path is O(1).  Pruning on plain removal is essential —
    /// without it a long-lived signal that sees N add/remove cycles
    /// accumulates N stale entries and `abort()` does N redundant
    /// no-op detach attempts (Copilot R2 finding).  The reverse
    /// `ListenerId → ObjectId(signal)` index for the lookup itself
    /// lives on [`super::super::VmInner::abort_listener_back_refs`].
    pub(crate) bound_listener_removals: HashMap<ListenerId, Entity>,
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
            bound_listener_removals: HashMap::new(),
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
        use super::abort_statics::{
            native_abort_signal_static_abort, native_abort_signal_static_any,
            native_abort_signal_static_timeout,
        };

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
        // its body always throws — instances are only obtainable via
        // `new AbortController().signal` (or, once added, the static
        // `AbortSignal.abort` / `AbortSignal.timeout` factories).
        // Marking it `create_constructable_function` is what lets
        // `new AbortSignal()` reach our throw site; otherwise `do_new`
        // would short-circuit with "X is not a constructor".
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
        // Install `AbortSignal.abort` / `.timeout` / `.any` static
        // factories (WHATWG §3.1.3).  They live as own methods on
        // the constructor function object itself, not on the
        // prototype — `AbortSignal.abort()` reads a
        // constructor-static method just like `Array.from`.
        // Bodies live in `abort_statics.rs`.
        for (name_sid, func) in [
            (
                self.strings.intern("abort"),
                native_abort_signal_static_abort as NativeFn,
            ),
            (
                self.strings.intern("timeout"),
                native_abort_signal_static_timeout,
            ),
            (self.strings.intern("any"), native_abort_signal_static_any),
        ] {
            self.install_native_method(abort_signal_ctor, name_sid, func, PropertyAttrs::METHOD);
        }

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
        self.install_native_method(
            ctrl_proto,
            self.well_known.abort,
            native_abort_controller_abort,
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
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                setter,
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
    }

    fn install_abort_signal_methods(&mut self, proto_id: ObjectId) {
        // `addEventListener` / `removeEventListener` / `dispatchEvent`
        // shadow the inherited EventTarget methods because the
        // listener store is in-VM, not ECS-attached.  Reusing the
        // pre-interned `WellKnownStrings` IDs guarantees these match
        // the names `register_event_target_prototype` published on
        // `EventTarget.prototype`, so the shape-based lookup hits
        // the override rather than the inherited slot.
        for (name_sid, func) in [
            (
                self.well_known.add_event_listener,
                native_abort_signal_add_event_listener as NativeFn,
            ),
            (
                self.well_known.remove_event_listener,
                native_abort_signal_remove_event_listener,
            ),
            (
                self.well_known.dispatch_event,
                native_abort_signal_dispatch_event,
            ),
            (
                self.well_known.throw_if_aborted,
                native_abort_signal_throw_if_aborted,
            ),
        ] {
            self.install_native_method(proto_id, name_sid, func, PropertyAttrs::METHOD);
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
    let JsValue::Object(ctrl_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };
    let signal_id = ctx.vm.create_abort_signal();
    // Promote the pre-allocated Ordinary instance to an
    // `AbortController` carrying `signal_id` as an internal slot.
    // The internal slot is what `abort()` consults — the JS-visible
    // `signal` own property (set below) is for `controller.signal`
    // reads only and cannot be used to retarget `abort()` even if
    // the user mutates it via `Object.defineProperty`.
    ctx.vm.get_object_mut(ctrl_id).kind = ObjectKind::AbortController { signal_id };
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
    // Read the paired signal from the controller's internal slot
    // (`ObjectKind::AbortController`'s `signal_id`), not from the
    // JS-visible `signal` own property.  Reading the property would
    // let user code retarget abort via
    // `Object.defineProperty(c, 'signal', {value: alien})` and would
    // make `AbortController.prototype.abort.call({signal: real})`
    // succeed against arbitrary objects — both spec-non-conforming.
    let signal_id = match this {
        JsValue::Object(ctrl_id) => match ctx.vm.get_object(ctrl_id).kind {
            ObjectKind::AbortController { signal_id } => signal_id,
            _ => {
                return Err(VmError::type_error(
                    "AbortController.prototype.abort called on incompatible receiver",
                ))
            }
        },
        _ => {
            return Err(VmError::type_error(
                "AbortController.prototype.abort called on non-object",
            ))
        }
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
    // WHATWG §3.1: `new AbortSignal()` throws.  Instances are
    // obtained via `new AbortController().signal` or the spec's
    // `AbortSignal.abort(reason)` / `.timeout(ms)` / `.any(signals)`
    // static factories (see [`super::abort_statics`]).
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
        .is_some_and(|s| s.aborted);
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
        // Already-aborted signals drop the registration — strictly
        // the spec queues a microtask that fires the callback
        // once, but wiring the microtask synthesis through the
        // shadowed dispatch path is out of scope for PR5a2.
        // Dropping is what the current test fixtures observe and
        // matches browsers when the caller inspects
        // `signal.aborted` after the add.
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
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    // `this` is validated even though the body is a stub — calling
    // the method with a non-AbortSignal receiver
    // (e.g. `AbortSignal.prototype.dispatchEvent.call({})`) is a
    // WebIDL conversion failure that should throw, matching the
    // other AbortSignal accessors / methods.  Without this guard
    // the stub silently returns `false`, masking the misuse.
    let _ = require_abort_signal_this(ctx, this, "dispatchEvent")?;
    // Stub returning `false` (WHATWG's "event not dispatched"
    // default).  `Event` constructors exist as of PR5a2, but
    // AbortSignal keeps its `'abort'` listener list in
    // [`AbortSignalState::abort_listeners`] rather than on an
    // ECS entity — the shared `EventTarget.prototype.dispatchEvent`
    // walk therefore has nothing to iterate here.  Routing
    // script-side `signal.dispatchEvent(new Event('abort'))` into
    // that custom store is tracked separately; `controller.abort()`
    // synthesises its dispatch internally without going through
    // this method, so the stub does not block the primary
    // AbortSignal use-case.
    Ok(JsValue::Boolean(false))
}

// ---------------------------------------------------------------------------
// Internal abort dispatch
// ---------------------------------------------------------------------------

/// Fire `'abort'` on `signal_id`: set state, then call every
/// registered listener exactly once (idempotent if already aborted).
///
/// Listeners are called with `(undefined)` as the sole argument
/// rather than a proper Event payload — typical handlers inspect
/// `signal.aborted` / `signal.reason`, both stable on the signal,
/// so the missing payload does not affect observable behaviour.
/// Threading a synthesised `Event('abort')` object through here
/// (now that Event constructors exist) is a separate refactor
/// because AbortSignal listeners do not live on an ECS entity
/// and so cannot reuse the shared dispatch walk directly.
///
/// # GC safety
///
/// The callback `ObjectId`s **must remain rooted** in
/// `abort_signal_states` for the duration of dispatch.  If we
/// `mem::take` them into a Rust local before iterating, a GC
/// triggered by an earlier callback can reclaim the function
/// objects we have not yet called (those `ObjectId`s would no
/// longer be reachable from any GC root).  Instead we set the
/// latch (`aborted = true`) up front — re-entrant
/// `addEventListener` then short-circuits via the already-aborted
/// guard — clone the `ObjectId` list into a local for stable
/// iteration, and leave the originals in `state` so the trace
/// step keeps marking them.  The `abort_listeners` Vec is drained
/// at the very end to honour WHATWG's one-shot semantics.
///
/// `onabort` is intentionally **not** cleared by dispatch.  The
/// IDL handler attribute remains observable from script after
/// the event fires (browsers expose the same handler reference
/// to subsequent `signal.onabort` reads).
/// Entry point used by both the public `AbortController.abort()`
/// dispatch and the `AbortSignal.timeout` internal fire — see
/// module doc for the contract.  Exposed to `natives_timer`
/// via [`internal_abort_signal`] so the drain path can route
/// a fired timeout through the same state-update + listener
/// dispatch as a user-visible abort.  Also `pub(super)` so the
/// static factories in [`super::abort_statics`] can compose it
/// with fresh signal allocation.
pub(super) fn abort_signal(
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
    // `DOMException("AbortError")` (WHATWG DOM §3.1.2 step 1) —
    // `create_default_abort_error` routes through
    // `VmInner::build_dom_exception` for the real instance.
    let materialised_reason = if matches!(reason, JsValue::Undefined) {
        create_default_abort_error(ctx)
    } else {
        reason
    };

    // Latch state and snapshot for iteration.  `onabort` and
    // `abort_listeners` are *cloned* (not taken) so they remain
    // reachable from `abort_signal_states` while user callbacks
    // run — see the # GC safety section above.
    //
    // `bound_listener_removals` is drained because its content is
    // `(ListenerId → Entity)` pairs (no `ObjectId`s), so the GC has
    // nothing to lose by moving them out.
    let (onabort, listeners, removals) = {
        let Some(state) = ctx.vm.abort_signal_states.get_mut(&signal_id) else {
            return Ok(());
        };
        state.aborted = true;
        state.reason = materialised_reason;
        let onabort = state.onabort;
        let listeners = state.abort_listeners.clone();
        let removals = std::mem::take(&mut state.bound_listener_removals);
        (onabort, listeners, removals)
    };

    // The reverse-index entries for these listener IDs are no longer
    // load-bearing (the back-refs themselves are about to be drained),
    // so prune them up front to keep `abort_listener_back_refs`
    // bounded.  A subsequent `removeEventListener` on one of these
    // listeners (e.g. inside an abort callback) will then short-circuit
    // its own scrub path on the missing entry, which is harmless.
    for listener_id in removals.keys() {
        ctx.vm.abort_listener_back_refs.remove(listener_id);
    }

    // Detach back-referenced listeners (set up by `addEventListener`'s
    // signal-option path) from their host's ECS `EventListeners`
    // component + `HostData::listener_store`, so subsequent dispatches
    // skip them.  Despawned-entity errors are silently absorbed —
    // the listener is already gone.
    detach_bound_listeners(ctx, &removals);

    // Fire `onabort` first (matches WHATWG §8.1.5 — event handler
    // attribute is "the first in addition to others registered").
    let signal_val = JsValue::Object(signal_id);
    if let Some(handler) = onabort {
        let _ = ctx.call_function(handler, signal_val, &[JsValue::Undefined]);
    }
    for cb in listeners {
        let _ = ctx.call_function(cb, signal_val, &[JsValue::Undefined]);
    }

    // One-shot: drain the listener list so a hypothetical second
    // `controller.abort()` (already a no-op via the latch above)
    // and any post-dispatch introspection see an empty list.
    // `onabort` is intentionally retained — the IDL handler stays
    // observable post-abort, matching browser behaviour.
    if let Some(state) = ctx.vm.abort_signal_states.get_mut(&signal_id) {
        state.abort_listeners.clear();
    }

    // `AbortSignal.any(inputs)` fan-out (WHATWG §3.1.3.3) — if
    // this signal appears as an input in any composite built
    // above, propagate the abort to each composite using *this*
    // signal's materialised reason.  Entries are removed as we
    // visit them: once the input has aborted, future aborts on
    // it are no-ops (the latch above returns early), so the
    // fan-out need not run twice.  The composite's own `aborted`
    // latch inside a recursive `abort_signal` call guards
    // against duplicate fires if multiple inputs share a
    // composite in the same call stack.
    // Reason was just latched into `state.reason` above; hoist the
    // read outside the loop so composites receive the exact
    // propagated value and we skip N-1 HashMap lookups.
    if let Some(composites) = ctx.vm.any_composite_map.remove(&signal_id) {
        let reason = materialised_reason;
        for composite_id in composites {
            abort_signal(ctx, composite_id, reason)?;
        }
    }

    // Fan out to every in-flight `fetch()` that registered this
    // signal in `VmInner::fetch_abort_observers` (WHATWG Fetch §5.1
    // step 13: if abort signal is aborted, set request's done flag).
    //
    // For each fetch_id we (a) reject its pending Promise
    // synchronously with the signal's materialised reason — this is
    // the user-visible "Promise rejected at the abort moment", not
    // queued behind the broker round-trip — and (b) send
    // `CancelFetch` so the broker can stop waiting on the network
    // and post an early aborted-reply.  The eventual broker reply
    // for this fetch is silently dropped because
    // `pending_fetches.remove` returned `Some` here, so the
    // `tick_network` settle-step's lookup will return `None`.
    if let Some(fetch_ids) = ctx.vm.fetch_abort_observers.remove(&signal_id) {
        let handle = ctx.vm.network_handle.as_ref().map(std::rc::Rc::clone);
        for fetch_id in fetch_ids {
            // Drop the reverse index up front — the fetch is no
            // longer signal-bound from this map's perspective.
            ctx.vm.fetch_signal_back_refs.remove(&fetch_id);
            // Reject the pending Promise.  Late broker replies for
            // this fetch_id will see `pending_fetches.remove` return
            // `None` and skip settlement — the user only ever
            // observes one rejection.
            //
            // GC root the Promise across `reject_promise_sync` (R2.1):
            // `pending_fetches` was its only root for the
            // user-discarded case, and a future runtime relaxing the
            // native-call `gc_enabled = false` gate could see the
            // settlement path allocate (microtask record, capability
            // routing) and reclaim `promise` mid-settle.  Defensive
            // root matches the surrounding codebase's invariant.
            if let Some(promise) = ctx.vm.pending_fetches.remove(&fetch_id) {
                ctx.vm.pending_fetch_cors.remove(&fetch_id);
                let mut g = ctx.vm.push_temp_root(JsValue::Object(promise));
                super::blob::reject_promise_sync(&mut g, promise, materialised_reason);
                drop(g);
            }
            if let Some(ref h) = handle {
                let _ = h.cancel_fetch(fetch_id);
            }
        }
    }

    Ok(())
}

/// Construct the default abort reason — a `DOMException` instance
/// with `name === "AbortError"` (WHATWG DOM §3.1.2 step 1: "set
/// reason to a new 'AbortError' DOMException").  Routed through
/// [`VmInner::build_dom_exception`] so the instance has
/// `DOMException.prototype` in its chain and the side-table entry
/// populated — `reason.code === 20`,
/// `reason instanceof DOMException`, `reason instanceof Error` all
/// hold as a result.
fn create_default_abort_error(ctx: &mut NativeContext<'_>) -> JsValue {
    let name = ctx.vm.well_known.abort_error;
    ctx.vm
        .build_dom_exception(name, "signal is aborted without reason")
}

/// Entry point for the timer drain path — synthesises a
/// `NativeContext` so the internal abort dispatch can reuse the
/// same state-update + listener plumbing as a user-visible
/// `controller.abort()`.  Engine-only because the caller
/// (`natives_timer::drain_timers`) only routes through here when
/// the `pending_timeout_signals` map has an entry, which is itself
/// engine-feature-gated.
pub(in crate::vm) fn internal_abort_signal(
    vm: &mut super::super::VmInner,
    signal_id: ObjectId,
    reason: JsValue,
) -> Result<(), VmError> {
    let mut ctx = super::super::value::NativeContext { vm };
    abort_signal(&mut ctx, signal_id, reason)
}

// Static factories live in `abort_statics.rs` to keep this file
// under the 1000-line convention.

/// Detach `(entity, listener_id)` pairs from their host's ECS
/// `EventListeners` component and the `HostData::listener_store`.
/// Used when an `AbortSignal` aborts to drop listeners registered via
/// `addEventListener({signal})`.
///
/// The two cleanup steps have **independent prerequisites**:
///
/// - `listener_store` removal requires only that `HostData` be
///   *installed* — the entries can be cleaned up regardless of the
///   bind state because the store is in-VM.
/// - ECS `EventListeners` mutation requires the world to be *bound*
///   (we need a live `EcsDom` pointer).
///
/// Combining the two under a single `host_if_bound()` early-return
/// would leak `listener_store` entries (and keep their JS function
/// `ObjectId`s rooted) whenever `controller.abort()` runs across an
/// unbind boundary — e.g. JS retained the controller in a global
/// and the shell unbound the VM between the registration and the
/// abort.  Splitting the prerequisites keeps both stores in sync
/// regardless of bind state.
fn detach_bound_listeners(ctx: &mut NativeContext<'_>, removals: &HashMap<ListenerId, Entity>) {
    if removals.is_empty() {
        return;
    }
    let bound = ctx.host_if_bound().is_some();
    for (&listener_id, &entity) in removals {
        if bound {
            // Drop from ECS first (scoped block so the world borrow
            // releases before we re-grab `host` for listener_store).
            let dom = ctx.host().dom();
            if let Ok(mut listeners) = dom
                .world_mut()
                .get::<&mut elidex_script_session::EventListeners>(entity)
            {
                listeners.remove(listener_id);
            }
        }
        // listener_store cleanup runs whether or not the VM is
        // currently bound — we just need `HostData` itself to be
        // installed.  Skipping this when unbound would leave the JS
        // function `ObjectId` rooted via `gc_root_object_ids` for
        // the rest of the VM's life.
        if let Some(host) = ctx.host_opt() {
            host.remove_listener(listener_id);
        }
    }
}
