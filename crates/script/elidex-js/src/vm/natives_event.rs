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
//! Detached method handles — `const pd = e.preventDefault; pd()` —
//! see `this === undefined` and silently no-op, matching browser
//! behaviour.  Calling the method with a `this` that is not an
//! `ObjectKind::Event` is likewise a silent no-op; we do not throw,
//! since the spec prose (§2.9) treats these methods as unconditionally
//! callable on any Event instance — the flag writes just happen to be
//! unobservable from non-Event receivers.

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

/// `Event.prototype.composedPath()` — returns the lazily-cached Array
/// stored in the Event's internal slot.
///
/// WHATWG DOM §2.9 requires the same Array be returned on every call
/// (the internal propagation-path list is "cloned" into an Array
/// once, and subsequent `composedPath()` invocations return that same
/// Array).  The dispatch machinery (PR3 C5+) writes the actual
/// target/ancestor wrapper list into `composed_path` before the first
/// listener fires; if a listener calls `composedPath()` before that
/// happens (or on a UA event with no propagation path), we lazily
/// allocate an empty Array and write it back into the slot so the
/// next call returns the same id.  Without this writeback, identity
/// (`e.composedPath() === e.composedPath()`) would be lost.
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
        // Lazy alloc + writeback for Event receivers.  Disable GC
        // across the alloc + slot-write so the receiver `id` cannot
        // be collected mid-sequence; this matters because direct
        // native invocation from tests doesn't get the interpreter's
        // per-native gc gate (interpreter.rs:71), so a GC threshold
        // hit during `create_array_object` could otherwise reclaim
        // the only Rust-local reference to the event.
        if matches!(ctx.vm.get_object(id).kind, ObjectKind::Event { .. }) {
            let saved_gc = ctx.vm.gc_enabled;
            ctx.vm.gc_enabled = false;
            let arr = ctx.vm.create_array_object(Vec::new());
            if let ObjectKind::Event { composed_path, .. } = &mut ctx.vm.get_object_mut(id).kind {
                *composed_path = Some(arr);
            }
            ctx.vm.gc_enabled = saved_gc;
            return Ok(JsValue::Object(arr));
        }
    }
    // Fallback — `this` not an Event.  No slot to cache in; return
    // a fresh empty Array (callers in this branch are detached
    // method invocations, identity is not meaningful).
    let empty = ctx.vm.create_array_object(Vec::new());
    Ok(JsValue::Object(empty))
}
