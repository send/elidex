//! Native methods installed on Event objects (`ObjectKind::Event`).
//!
//! All four methods read `this` as the Event object and match on
//! `ObjectKind::Event` to locate the internal-slot flag fields.  This
//! mirrors how `Promise.prototype.then` matches on `ObjectKind::Promise`
//! and `Generator.prototype.next` matches on `ObjectKind::Generator` —
//! no closure capture, no per-instance state in the native `fn`
//! pointer, and the flag fields live in the canonical internal-slot
//! location rather than as hidden JS properties.
//!
//! ## Receiver type-check policy (PR3)
//!
//! Detached method handles — `const pd = e.preventDefault; pd()` —
//! see `this === undefined`.  Calls with a non-`ObjectKind::Event`
//! receiver get the same treatment.  PR3 chooses **silent no-op**
//! for both cases.
//!
//! This deviates from WebIDL: the spec generates non-generic
//! bindings that throw `TypeError: Illegal invocation` when `this`
//! fails the [[Brand]] check.  Spec-correct enforcement is
//! deferred to a later M4-12 tranche that lands the `Event`
//! constructor and the rest of the Event prototype's strict
//! bindings (also covers the `defaultPrevented` getter).
//!
//! Until then, the no-op is a pragmatic choice: real-world code
//! that escapes a method handle off an Event is exceedingly rare
//! (linters block it), and the silent path keeps the dispatch
//! machinery in PR3 simple.  WPT alignment tests for §2.9 receiver
//! brand-checks will surface the gap when they run (Phase 4 late).

use super::value::{JsValue, NativeContext, ObjectKind, VmError};

/// Accessor getter for `Event.prototype.defaultPrevented`.
///
/// WHATWG DOM §2.9 defines `defaultPrevented` as a getter attribute
/// that returns the current state of the canceled flag — i.e. it must
/// reflect a `preventDefault()` call inside the same listener.
/// Exposing as a live getter (rather than a stale data property) is
/// the only spec-conforming representation.
pub(super) fn native_event_get_default_prevented(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    if let JsValue::Object(id) = this {
        if let ObjectKind::Event {
            default_prevented, ..
        } = &ctx.vm.get_object(id).kind
        {
            return Ok(JsValue::Boolean(*default_prevented));
        }
    }
    // Unreachable in well-formed callers — the getter is installed on
    // the event object only, so `this` is always an `ObjectKind::Event`.
    // Return `false` rather than throwing to match detached-method
    // behaviour on the other event methods.
    Ok(JsValue::Boolean(false))
}

/// `Event.prototype.preventDefault()` — WHATWG DOM §2.9 step 1 only
/// sets `canceled flag` when the event is `cancelable` AND the listener
/// is NOT passive; otherwise silent no-op.  The spec warning-in-console
/// case for passive listeners is omitted (we follow the silent-no-op
/// path major engines take).
pub(super) fn native_event_prevent_default(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    if let JsValue::Object(id) = this {
        if let ObjectKind::Event {
            default_prevented,
            cancelable,
            passive,
            ..
        } = &mut ctx.vm.get_object_mut(id).kind
        {
            if *cancelable && !*passive {
                *default_prevented = true;
            }
        }
    }
    Ok(JsValue::Undefined)
}

/// `Event.prototype.stopPropagation()` — WHATWG DOM §2.9 sets the
/// event's `stop propagation flag`.  No preconditions.
pub(super) fn native_event_stop_propagation(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    if let JsValue::Object(id) = this {
        if let ObjectKind::Event {
            propagation_stopped,
            ..
        } = &mut ctx.vm.get_object_mut(id).kind
        {
            *propagation_stopped = true;
        }
    }
    Ok(JsValue::Undefined)
}

/// `Event.prototype.stopImmediatePropagation()` — WHATWG DOM §2.9
/// sets BOTH the `stop propagation flag` and `stop immediate propagation
/// flag`.  Setting only the latter is non-conforming: spec text
/// "Set this's stop propagation flag" precedes the immediate-flag set.
pub(super) fn native_event_stop_immediate_propagation(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    if let JsValue::Object(id) = this {
        if let ObjectKind::Event {
            propagation_stopped,
            immediate_propagation_stopped,
            ..
        } = &mut ctx.vm.get_object_mut(id).kind
        {
            *propagation_stopped = true;
            *immediate_propagation_stopped = true;
        }
    }
    Ok(JsValue::Undefined)
}

/// `Event.prototype.composedPath()` — returns the Array stored in
/// the Event's internal `composed_path` slot.
///
/// WHATWG DOM §2.9 requires the same Array be returned on every call
/// (the internal propagation-path list is "cloned" into an Array once
/// and subsequent invocations return that same Array).
///
/// `create_event_object` populates the slot from
/// `DispatchEvent.composed_path` when non-empty (resolving each
/// Entity to its `HostObject` wrapper).  For events whose dispatch
/// path didn't seed `composed_path` (UA events without a propagation
/// path), the slot starts as `None`; this function then lazily
/// allocates an empty Array, writes it back into the slot, and
/// returns it — so the next call returns the same ObjectId and
/// identity (`e.composedPath() === e.composedPath()`) holds.
pub(super) fn native_event_composed_path(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    if let JsValue::Object(id) = this {
        // Fast path: cached.
        if let ObjectKind::Event {
            composed_path: Some(arr),
            ..
        } = &ctx.vm.get_object(id).kind
        {
            return Ok(JsValue::Object(*arr));
        }
        // Lazy alloc + writeback for Event receivers.  Root the
        // receiver `this` on the VM stack across `create_array_object`
        // — the existing Event `id` is the only thing that ties the
        // about-to-be-created Array back to a GC root, so it must
        // outlive the alloc.  RAII guard is panic-safe (a panic
        // between alloc and slot-write would otherwise leak the
        // freshly-allocated Array's potential reachability).
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::Event { .. }) {
            let mut g = ctx.vm.push_temp_root(this);
            let arr = g.create_array_object(Vec::new());
            if let ObjectKind::Event { composed_path, .. } = &mut g.get_object_mut(id).kind {
                *composed_path = Some(arr);
            }
            drop(g);
            return Ok(JsValue::Object(arr));
        }
    }
    // Fallback — `this` not an Event.  No slot to cache in; return
    // a fresh empty Array (callers in this branch are detached
    // method invocations, identity is not meaningful).
    let empty = ctx.vm.create_array_object(Vec::new());
    Ok(JsValue::Object(empty))
}
