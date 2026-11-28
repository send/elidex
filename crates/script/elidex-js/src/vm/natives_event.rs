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
//! Event members return fallback values on incompatible receivers
//! (#11-event-members-receiver-brand).

use super::value::{JsValue, NativeContext, ObjectKind, VmError};

/// Accessor getter for `Event.prototype.defaultPrevented`.
///
/// DOM §2.2 returns true when the event's canceled flag is set.
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
    // Borrowing the getter with an incompatible receiver reaches this fallback
    // (#11-event-members-receiver-brand).
    Ok(JsValue::Boolean(false))
}

/// `Event.prototype.preventDefault()` — WHATWG DOM §2.2 set the canceled flag only
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

/// DOM §2.2 `stopPropagation()` sets the event's stop propagation flag.
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

/// DOM §2.2 `stopImmediatePropagation()` sets the event's stop propagation
/// and stop immediate propagation flags.
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
