//! `AbortController` / `AbortSignal` primitives (WHATWG DOM §3.1 Interface
//! AbortController / §3.2 Interface AbortSignal).
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
//! The row also carries the DOM §3.2 dependent-signal record
//! ([`DependentLinks`]: the **dependent** flag, **source signals**,
//! **dependent signals**).  The graph algorithms — "create a dependent
//! abort signal" and "signal abort" steps 1-4 — live in the
//! engine-independent [`elidex_dom_api::abort`] module, which reads and
//! writes the row through the [`AbortSignalGraph`] impl below; this
//! module keeps what only the engine can do (the `DOMException`
//! materialisation, the abort algorithms, the `'abort'` fire).
//!
//! ## Listener model (`#11-eventtarget-dispatch-core`)
//!
//! `AbortSignal` is a full member of the **unified EventTarget dispatch
//! core**.  It inherits `addEventListener` / `removeEventListener` /
//! `dispatchEvent` from `EventTarget.prototype` (no shadows): the shared
//! natives route an `AbortSignal` receiver to its
//! [`super::super::VmInner::vm_event_listeners`] home via
//! [`super::dispatch_target::DispatchTarget::VmObject`], with callbacks in
//! `HostData::listener_store`.  Its `onabort` is an event-handler IDL
//! attribute (one `ListenerKind::EventHandler` entry in the same list, via
//! the shared VmObject event-handler backend).  `controller.abort()` fires
//! a real `Event('abort')` through the shared UA-fire
//! ([`dispatch_vm_simple_event`]); `AbortSignal` is a flat target
//! (get-the-parent → null, so at-target only) and `'abort'` fires exactly
//! once ("signal abort" step 1 makes a second `abort()` a no-op).
//!
//! [`AbortSignalState`] keeps only the non-listener state (`reason` /
//! the dependent-signal links + the `{signal}` back-ref bookkeeping).
//!
//! ## Implemented
//!
//! - `new AbortController()` → object with `.signal` and `.abort()`.
//! - `signal.aborted` / `signal.reason` / `signal.onabort`.
//! - `signal.throwIfAborted()`; `signal.addEventListener` /
//!   `removeEventListener` / `dispatchEvent` (inherited, unified).
//! - `controller.abort(reason?)` — "signal abort": synchronously sets the
//!   reason of the signal and of every dependent, then runs the abort steps
//!   for the signal and for each dependent (`'abort'` fires at each:
//!   onabort + every `addEventListener('abort', …)` callback in
//!   registration order).  Idempotent; second call is a no-op.
//! - `AbortSignal.abort` / `.timeout` / `.any` static factories (in
//!   `abort_statics`).
//! - `addEventListener({signal})` integration — the shared add native
//!   inserts `listener_id → target` (a [`DispatchTarget`]) into
//!   [`AbortSignalState::bound_listener_removals`] and a reverse
//!   `listener_id → signal_id` index on
//!   [`super::super::VmInner::abort_listener_back_refs`].
//!   `removeEventListener` prunes the entry in O(1); the abort steps drain
//!   the map and detach each listener from its home (ECS component for a
//!   `Node`, `vm_event_listeners` for a `VmObject`) via the listener-home
//!   adapter.

#![cfg(feature = "engine")]

use std::collections::HashMap;

use elidex_dom_api::abort::{signal_abort, AbortSignalGraph, DependentLinks};

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};

use super::dispatch_target::DispatchTarget;
use super::event_target_dispatch_vm::dispatch_vm_simple_event;
use elidex_script_session::ListenerId;

/// Per-signal mutable state, owned by [`super::super::VmInner::abort_signal_states`]
/// and looked up via the signal's `ObjectId`.
#[derive(Debug)]
pub(crate) struct AbortSignalState {
    /// The signal's abort reason; `None` ⇔ not aborted (DOM §3.2: "An
    /// AbortSignal object is aborted when its abort reason is not
    /// undefined").  The reason is the value supplied to `abort(reason)`
    /// or the default "AbortError" `DOMException` materialised when the
    /// call omitted one, so an aborted signal's reason is never
    /// `undefined` — one fact, one field.  Set once, by "signal abort"
    /// step 2 / 4.1.1 or by the static `abort(reason)` / `any(signals)`
    /// reason-only paths; never cleared.
    pub(crate) reason: Option<JsValue>,
    /// DOM §3.2 dependent-signal record: the **dependent** flag and the
    /// **source signals** / **dependent signals** weak sets.  Private to
    /// [`elidex_dom_api::abort`]; this module reads it through the
    /// accessors and the sweep prunes it with
    /// [`DependentLinks::retain_live`].  Not traced (the sets are weak).
    pub(crate) links: DependentLinks<ObjectId>,
    /// Back-references for `addEventListener(type, cb, {signal})` on
    /// other EventTargets — when this signal aborts, the runtime removes
    /// each `(listener_id → target)` from that target's listener home (the
    /// ECS `EventListeners` component for a `Node`, `vm_event_listeners`
    /// for a `VmObject`) so the listener stops firing.
    ///
    /// Stored as a `HashMap` (not `Vec`) so `removeEventListener`'s scrub
    /// path is O(1).  Pruning on plain removal is essential — without it a
    /// long-lived signal that sees N add/remove cycles accumulates N stale
    /// entries and `abort()` does N redundant no-op detach attempts
    /// (Copilot R2 finding).  The reverse `ListenerId → ObjectId(signal)`
    /// index for the lookup itself lives on
    /// [`super::super::VmInner::abort_listener_back_refs`].
    ///
    /// The signal's OWN `'abort'` listeners + `onabort` handler do NOT
    /// live here — they live in the unified [`super::super::VmInner::vm_event_listeners`]
    /// home keyed by the signal's `ObjectId`, dispatched by the shared
    /// EventTarget core (`controller.abort()` fires `'abort'` via the
    /// shared UA-fire on `VmObject(signal_id)`).
    pub(crate) bound_listener_removals: HashMap<ListenerId, DispatchTarget>,
}

impl AbortSignalState {
    /// Fresh, never-aborted, non-dependent state with no listeners.
    fn new() -> Self {
        Self {
            reason: None,
            links: DependentLinks::default(),
            bound_listener_removals: HashMap::new(),
        }
    }
}

/// The engine-independent algorithms of [`elidex_dom_api::abort`] read and
/// write the signal rows through this impl — one row per signal, where the
/// row already is.
impl AbortSignalGraph for VmInner {
    type Handle = ObjectId;
    type Reason = JsValue;

    fn abort_reason(&self, h: ObjectId) -> Option<JsValue> {
        self.abort_signal_states.get(&h).and_then(|s| s.reason)
    }

    fn set_abort_reason(&mut self, h: ObjectId, reason: JsValue) {
        if let Some(state) = self.abort_signal_states.get_mut(&h) {
            state.reason = Some(reason);
        }
    }

    fn links(&self, h: ObjectId) -> Option<&DependentLinks<ObjectId>> {
        self.abort_signal_states.get(&h).map(|s| &s.links)
    }

    fn links_mut(&mut self, h: ObjectId) -> Option<&mut DependentLinks<ObjectId>> {
        self.abort_signal_states.get_mut(&h).map(|s| &mut s.links)
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
        // WebIDL: `AbortSignal` declares NO constructor operation
        // (instances come from `new AbortController().signal` or the
        // static `AbortSignal.abort` / `.timeout` / `.any` factories), so
        // both `new AbortSignal()` and bare `AbortSignal()` throw a
        // TypeError per the WebIDL §3.7.1 (Interface object) creation
        // algorithm step 1.1.
        // Registered as `CallShape::IllegalConstructor`: the gate in
        // `admit_construct` / `call_dispatch` raises the canonical "Failed to
        // construct 'AbortSignal': Illegal constructor" (shared SoT
        // `VmError::illegal_constructor`) before any body runs. Still a
        // real global so `signal instanceof AbortSignal` and
        // `AbortSignal.prototype` parity work.
        let abort_signal_ctor = self.create_illegal_constructor_function(
            "AbortSignal",
            super::super::value::native_illegal_constructor_unreachable,
        );
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
        // factories (DOM §3.2 — the three static method steps).  They live as own methods on
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

        let ctrl_ctor = self.create_constructor_only_function(
            "AbortController",
            native_abort_controller_constructor,
        );
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
        // `aborted` and `reason` are RO accessors.  `onabort` is the
        // event-handler IDL attribute (WHATWG HTML §8.1.8.1) — installed
        // separately via the shared event-handler-attr backend (keyed by
        // the `'abort'` event-type SID) so it lives as a
        // `ListenerKind::EventHandler` entry in the unified
        // `vm_event_listeners` home, dispatched in registration order
        // alongside `addEventListener('abort', …)` callbacks.
        for (name_sid, getter) in [
            (
                self.well_known.aborted,
                native_abort_signal_get_aborted as NativeFn,
            ),
            (
                self.well_known.reason,
                native_abort_signal_get_reason as NativeFn,
            ),
        ] {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                None::<NativeFn>,
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }
        // `onabort` event-handler IDL attribute over the VmObject
        // event-handler backend, bound key = the `'abort'` event-type SID.
        let onabort_sid = self.well_known.onabort;
        let abort_event_sid = self.well_known.abort;
        self.install_bound_accessor_pair(
            proto_id,
            onabort_sid,
            super::event_handler_attrs::native_vm_event_handler_get as NativeFn,
            Some(super::event_handler_attrs::native_vm_event_handler_set as NativeFn),
            abort_event_sid,
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
    }

    fn install_abort_signal_methods(&mut self, proto_id: ObjectId) {
        // `addEventListener` / `removeEventListener` / `dispatchEvent` are
        // INHERITED from `EventTarget.prototype` now — the unified
        // dispatch core routes an `AbortSignal` receiver to its
        // `vm_event_listeners` home (`DispatchTarget::VmObject`), so the
        // old in-VM shadows are deleted.  Only `throwIfAborted` (no
        // EventTarget analogue) is installed here.
        self.install_native_method(
            proto_id,
            self.well_known.throw_if_aborted,
            native_abort_signal_throw_if_aborted,
            PropertyAttrs::METHOD,
        );
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
    Ok(JsValue::Boolean(ctx.vm.abort_reason(id).is_some()))
}

fn native_abort_signal_get_reason(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_abort_signal_this(ctx, this, "reason")?;
    Ok(ctx.vm.abort_reason(id).unwrap_or(JsValue::Undefined))
}

// ---------------------------------------------------------------------------
// AbortSignal methods
// ---------------------------------------------------------------------------

/// `signal.throwIfAborted()` — DOM §3.2 "The throwIfAborted() method steps
/// are to throw this's abort reason, if this is aborted."; otherwise no-op.
fn native_abort_signal_throw_if_aborted(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_abort_signal_this(ctx, this, "throwIfAborted")?;
    if let Some(reason) = ctx.vm.abort_reason(id) {
        return Err(VmError::throw(reason));
    }
    Ok(JsValue::Undefined)
}

// `addEventListener` / `removeEventListener` / `dispatchEvent` are now
// inherited from `EventTarget.prototype` and route through the unified
// dispatch core (`DispatchTarget::VmObject`) — the old in-VM shadows are
// deleted.  A non-`abort` listener still never fires (only `'abort'` is
// dispatched), and `signal.dispatchEvent(new Event('abort'))` now actually
// walks the `vm_event_listeners` home.

// ---------------------------------------------------------------------------
// Internal abort dispatch
// ---------------------------------------------------------------------------

/// DOM §3.2 "signal abort" for `signal_id`: set the reasons (steps 1-4,
/// in [`elidex_dom_api::abort::signal_abort`]), then run the abort steps
/// for the signal and for each planned dependent (steps 5-6,
/// [`run_abort_steps`]).  Idempotent: an already-aborted signal is a no-op
/// (step 1).
///
/// Step 2's "otherwise" branch — `reason === undefined` becomes a new
/// "AbortError" `DOMException` — is engine-bound: the module calls the
/// `materialise_abort_reason` closure once, only after step 1 has passed
/// (`create_default_abort_error` routes through
/// `VmInner::build_dom_exception` for the real instance).
///
/// **Depth 1, no recursion.**  A dependent's source signals are never
/// dependent (create a dependent abort signal step 4.2), so the plan holds
/// every transitive dependent of `signal_id` and the abort steps run in one
/// loop.  Every reason is set before any JS runs, so a dependent reads as
/// aborted inside the source's `'abort'` listeners.
///
/// # GC safety
///
/// The plan's handles are rooted on the VM stack for the rest of the call
/// (one [`VmInner::push_stack_scope`] frame).  `controller.abort()` is the
/// native entry: it runs through the native-call gate (`gc_enabled =
/// false`), where the root is inert (the static `AbortSignal.abort()` and
/// `AbortSignal.any()` never reach this function — their reason-only steps
/// are set without running abort steps, D-5);
/// `AbortSignal.timeout(ms)` fires from the timer drain with
/// `gc_enabled = true`, where a listener's allocation can run a collection
/// between two plan entries — an unrooted, unreferenced planned dependent
/// would be swept and its `ObjectId` recycled, and its abort steps would
/// run against the new occupant.  The listener callbacks themselves live in
/// `HostData::listener_store` (a GC root via `gc_root_object_ids`) and the
/// event object is rooted via `dispatched_events` for the dispatch window.
///
/// # Errors
///
/// Every plan entry runs its abort steps regardless of earlier results; the
/// first `Err` is kept and returned after the loop (no `?` inside it).  The
/// spec has no failure path in steps 5-6, and step 4 has already committed
/// every dependent's reason, so skipping a dependent's abort steps would
/// leave it aborted for ever with its algorithms never run and its event
/// never fired.
///
/// Entry point used by `AbortController.abort()` and, through
/// [`internal_abort_signal`], by the `AbortSignal.timeout` timer drain.
pub(super) fn abort_signal(
    ctx: &mut NativeContext<'_>,
    signal_id: ObjectId,
    reason: JsValue,
) -> Result<(), VmError> {
    // Steps 1-4: reasons set, plan fixed, before any JS runs.  Step 1's
    // no-op (no such signal, or already aborted) is the module's `None`,
    // taken before the closure materialises the default reason.
    let Some(plan) = signal_abort(&mut *ctx.vm, signal_id, |vm| {
        materialise_abort_reason(vm, reason)
    }) else {
        return Ok(());
    };

    // Root the plan for the rest of the call (see "GC safety").
    let mut frame = ctx.vm.push_stack_scope();
    frame.stack.push(JsValue::Object(plan.signal));
    for &dependent in &plan.dependents {
        frame.stack.push(JsValue::Object(dependent));
    }
    let mut rooted = NativeContext::new_call(&mut frame);

    // Steps 5-6.
    let mut first_err = run_abort_steps(&mut rooted, plan.signal).err();
    for &dependent in &plan.dependents {
        if let Err(e) = run_abort_steps(&mut rooted, dependent) {
            first_err.get_or_insert(e);
        }
    }
    first_err.map_or(Ok(()), Err)
}

/// DOM §3.2 "run the abort steps" for `signal_id`: (1) run the signal's
/// abort algorithms, (2) empty them, (3) fire an event named `abort` at it.
///
/// The abort algorithms are two stores today
/// (`#11-abort-algorithms-single-set`): the per-row
/// `bound_listener_removals` (`addEventListener({signal})`, "add an event
/// listener" step 6) and the per-VM `fetch_abort_observers` (`fetch()`
/// method steps step 11).  Both are drained here, before the fire.
///
/// The `onabort` handler + every `addEventListener('abort', …)` callback
/// run in registration order (HTML §8.1.8.1 Event handlers), each
/// receiving a proper `Event('abort')` whose `target` is the signal.  The
/// listeners persist in the home — a second fire cannot happen because
/// "signal abort" step 1 returns for an aborted signal.
fn run_abort_steps(ctx: &mut NativeContext<'_>, signal_id: ObjectId) -> Result<(), VmError> {
    // A row missing for the handle is a no-op (unreachable once the plan
    // is rooted).
    let Some(reason) = ctx.vm.abort_reason(signal_id) else {
        return Ok(());
    };

    // Step 1 (listener removals): `bound_listener_removals` is drained
    // because its content is `(ListenerId → DispatchTarget)` pairs (no
    // `ObjectId`s the GC must keep), so moving them out is free.  Step 2
    // (empty the algorithms) is the drain itself.
    let removals = ctx
        .vm
        .abort_signal_states
        .get_mut(&signal_id)
        .map(|state| std::mem::take(&mut state.bound_listener_removals))
        .unwrap_or_default();

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
    // signal-option path) from each target's listener home (ECS
    // `EventListeners` for a `Node`, `vm_event_listeners` for a `VmObject`
    // — incl. the IndexedDB EventTargets, now unified) + the
    // `HostData::listener_store`, so subsequent dispatches skip them.
    // Despawned-entity errors are silently absorbed — the listener is
    // already gone.
    detach_bound_listeners(ctx, &removals);

    // Step 1 (fetch): every in-flight `fetch()` that registered this
    // signal in `VmInner::fetch_abort_observers` — Fetch §5.6 Fetch methods,
    // `fetch(input, init)` method steps step 11 "Add the following abort
    // steps to requestObject's signal: … 4. Abort the fetch() call with p,
    // request, responseObject, and requestObject's signal's abort reason."
    //
    // For each fetch_id we (a) reject its pending Promise synchronously
    // with the signal's abort reason — this is the user-visible "Promise
    // rejected at the abort moment", not queued behind the broker
    // round-trip — and (b) send `CancelFetch` so the broker can stop
    // waiting on the network and post an early aborted-reply.  The
    // eventual broker reply for this fetch is silently dropped because
    // `pending_fetches.remove` returned `Some` here, so the `tick_network`
    // settle-step's lookup will return `None`.
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
                super::blob::reject_promise_sync(&mut g, promise, reason);
                drop(g);
            }
            if let Some(ref h) = handle {
                let _ = h.cancel_fetch(fetch_id);
            }
        }
    }

    // Step 3: fire `'abort'` (bubbles=false, cancelable=false) through the
    // shared EventTarget core on this signal's `vm_event_listeners` home.
    #[cfg(all(test, feature = "engine"))]
    if std::mem::take(&mut ctx.vm.fail_next_abort_dispatch) {
        return Err(VmError::range_error("injected abort-dispatch failure"));
    }
    let abort_sid = ctx.vm.well_known.abort;
    dispatch_vm_simple_event(ctx, signal_id, abort_sid, false, false).map(|_| ())
}

/// Construct the default abort reason — a `DOMException` instance
/// with `name === "AbortError"` (DOM §3.2 "signal abort" step 2 and the
/// static `abort(reason)` method steps step 2: "Set signal's abort reason
/// to reason if it is given; otherwise to a new "AbortError"
/// DOMException.").  Routed through
/// [`VmInner::build_dom_exception`] so the instance has
/// `DOMException.prototype` in its chain and the side-table entry
/// populated — `reason.code === 20`,
/// `reason instanceof DOMException`, `reason instanceof Error` all
/// hold as a result.
fn create_default_abort_error(vm: &mut VmInner) -> JsValue {
    let name = vm.well_known.abort_error;
    vm.build_dom_exception(name, "signal is aborted without reason")
}

/// An abort reason as the spec sets it: `reason` if given, otherwise a new
/// `"AbortError"` `DOMException` ("signal abort" step 2 / the static
/// `abort(reason)` method steps step 2).
pub(super) fn materialise_abort_reason(vm: &mut VmInner, reason: JsValue) -> JsValue {
    if matches!(reason, JsValue::Undefined) {
        create_default_abort_error(vm)
    } else {
        reason
    }
}

/// Entry point for the timer drain path — synthesises a
/// `NativeContext` so the internal abort dispatch can reuse the
/// same state-update + listener plumbing as a user-visible
/// `controller.abort()`.  Engine-only because the caller
/// (`natives_timer::drain_timers`) only routes through here when
/// the `pending_timeout_signals` map has an entry, which is itself
/// engine-feature-gated.
///
/// Caller obligation (plan §4.1, "The `reason` closure's contract"): a caller
/// that passes `undefined` — so the default `DOMException` is allocated inside
/// the abort — while GC is enabled must keep `signal_id` reachable (root it)
/// across the call, since the allocation may sweep.  The native paths satisfy
/// this because they run with `gc_enabled = false`; the timer path passes a
/// pre-built `TimeoutError`, so nothing is allocated.
pub(in crate::vm) fn internal_abort_signal(
    vm: &mut super::super::VmInner,
    signal_id: ObjectId,
    reason: JsValue,
) -> Result<(), VmError> {
    let mut ctx = super::super::value::NativeContext::new_call(vm);
    abort_signal(&mut ctx, signal_id, reason)
}

// Static factories live in `abort_statics.rs` to keep this file
// under the 1000-line convention.

/// Detach `(listener_id → target)` pairs from each target's listener
/// home (the ECS `EventListeners` component for a `Node`,
/// `vm_event_listeners` for a `VmObject`) and the `HostData::listener_store`.
/// Used when an `AbortSignal` aborts to drop listeners registered via
/// `addEventListener({signal})` — touchpoint (f) of the listener-home
/// adapter (the per-home branch lives in `DispatchTarget::remove_listener_entry`).
///
/// The two cleanup steps have **independent prerequisites**:
///
/// - `listener_store` removal requires only that `HostData` be
///   *installed* — the entries can be cleaned up regardless of the bind
///   state because the store is in-VM.
/// - the home mutation: a `Node` home requires the world to be *bound*
///   (a live `EcsDom` pointer — the adapter returns `None` when unbound);
///   a `VmObject` home (`vm_event_listeners`) is always reachable.
///
/// Splitting the prerequisites keeps both stores in sync regardless of
/// bind state — without it, `controller.abort()` across an unbind boundary
/// (JS retained the controller in a global, shell unbound the VM between
/// registration and abort) would leak `listener_store` entries and keep
/// their JS function `ObjectId`s rooted for the rest of the VM's life.
fn detach_bound_listeners(
    ctx: &mut NativeContext<'_>,
    removals: &HashMap<ListenerId, DispatchTarget>,
) {
    if removals.is_empty() {
        return;
    }
    for (&listener_id, &target) in removals {
        // Drop from the target's listener home (no-op when a `Node` home
        // is unbound / despawned — the adapter absorbs it).
        target.remove_listener_entry(ctx, listener_id);
        // `listener_store` cleanup runs whether or not the VM is currently
        // bound — we just need `HostData` itself to be installed.  Skipping
        // this when unbound would leave the JS function `ObjectId` rooted
        // via `gc_root_object_ids` for the rest of the VM's life.
        if let Some(host) = ctx.host_opt() {
            host.remove_listener(listener_id);
        }
    }
}
