//! `AbortSignal` static factories — `abort(reason?)` /
//! `timeout(ms)` / `any(signals)` (WHATWG DOM §3.1.3).
//!
//! Split out of [`super::abort`] to keep that module under the
//! project's 1000-line convention.  The functions live as own
//! methods on the `AbortSignal` constructor function object (not on
//! `AbortSignal.prototype`), installed by
//! [`VmInner::register_abort_signal_global`](super::abort::VmInner).
//!
//! The core signal state machine ([`super::abort::abort_signal`])
//! remains in `abort.rs`; these factories only compose it with
//! fresh signal allocation / timer scheduling.

#![cfg(feature = "engine")]

use super::super::value::{JsValue, NativeContext, ObjectId, ObjectKind, VmError};

use super::abort::abort_signal;

/// `AbortSignal.abort(reason?)` — returns an already-aborted signal
/// (WHATWG §3.1.3.1).  Equivalent to:
///
/// ```js
/// const c = new AbortController();
/// c.abort(reason);
/// c.signal;
/// ```
///
/// …but without allocating the controller.
pub(super) fn native_abort_signal_static_abort(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let signal_id = ctx.vm.create_abort_signal();
    let reason = args.first().copied().unwrap_or(JsValue::Undefined);
    // `abort_signal` materialises the default AbortError reason
    // when `reason === undefined`, matching WHATWG §3.1.3.1 step 2.
    abort_signal(ctx, signal_id, reason)?;
    Ok(JsValue::Object(signal_id))
}

/// `AbortSignal.timeout(ms)` — returns a signal that aborts with a
/// `DOMException("TimeoutError")` reason after `ms` milliseconds
/// (WHATWG §3.1.3.2).  The timer is managed via
/// [`VmInner::pending_timeout_signals`](super::super::VmInner);
/// on fire, the VM synthesises an internal abort *without*
/// invoking any JS callback.
pub(super) fn native_abort_signal_static_timeout(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let ms_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let ms = ctx.to_number(ms_arg)?;
    // Share the setTimeout clamp so `AbortSignal.timeout(Number.MAX_VALUE)`
    // cannot overflow `Instant::now() + delay` and panic the VM.
    let delay = super::super::natives_timer::clamp_delay_to_duration(ms);

    let signal_id = ctx.vm.create_abort_signal();
    // Reserve a timer id + push a heap entry.  The callback slot is
    // a placeholder (we skip the JS dispatch when
    // `pending_timeout_signals` has the entry); passing the signal
    // itself as a stand-in avoids a fresh native-function alloc,
    // and the drain guard never reads it.
    let timer_id = ctx.vm.next_timer_id;
    ctx.vm.next_timer_id = ctx.vm.next_timer_id.wrapping_add(1);
    ctx.vm.active_timer_ids.insert(timer_id);
    ctx.vm.pending_timeout_signals.insert(timer_id, signal_id);
    ctx.vm
        .timer_queue
        .push(super::super::natives_timer::TimerEntry {
            id: timer_id,
            deadline: std::time::Instant::now() + delay,
            callback: signal_id,
            repeat: None,
            args: Vec::new(),
        });
    Ok(JsValue::Object(signal_id))
}

/// `AbortSignal.any(signals)` — returns a signal that aborts as
/// soon as any input signal aborts (WHATWG §3.1.3.3).  If any
/// input is already aborted at call time, the returned signal is
/// aborted synchronously with the first-aborted signal's reason;
/// otherwise the composite is non-aborted and propagation happens
/// lazily via [`super::super::VmInner::any_composite_map`] —
/// every abort on an input consults the map and fires on each
/// observing composite before returning to the user.  Chained
/// composites (`any([any([a, b]), c])`) propagate through the
/// map's recursive `abort_signal` call.
///
/// PR5a2 C6 superseded the earlier "TODO multi-input propagation"
/// marker — no `addEventListener('abort', …)` indirection is
/// needed because the fan-out runs inside
/// [`super::abort::abort_signal`] directly, sparing a pair of
/// engine-side function allocations per input.
pub(super) fn native_abort_signal_static_any(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let iterable = args.first().copied().unwrap_or(JsValue::Undefined);
    // Array-shaped input only for PR5a — the full iterable
    // protocol (Symbol.iterator / for-of hardening) lands in
    // PR5a2.  Read elements directly from `ObjectKind::Array` so
    // we don't route through the `coerce::get_property` path
    // (which doesn't handle numeric-index array reads — that
    // logic lives in the bytecode dispatch opcode).
    let JsValue::Object(arr_id) = iterable else {
        return Err(VmError::type_error(
            "AbortSignal.any: expected an iterable of AbortSignals",
        ));
    };
    let elements: Vec<JsValue> = match &ctx.vm.get_object(arr_id).kind {
        ObjectKind::Array { elements } => elements.clone(),
        _ => {
            return Err(VmError::type_error(
                "AbortSignal.any: expected an iterable of AbortSignals",
            ));
        }
    };

    // Pre-validation pass — side-effect-free.  We validate every
    // element (brand check + AbortSignal kind) *before* allocating
    // the composite signal so a throw on a bogus element does not
    // strand an unreferenced entry in `abort_signal_states`.  Also
    // capture the first already-aborted signal's reason so we can
    // propagate it when (and only when) the allocation succeeds
    // (WHATWG §3.1.3.3 step 2).
    let mut inputs: Vec<ObjectId> = Vec::with_capacity(elements.len());
    let mut first_aborted_reason: Option<JsValue> = None;
    for v in elements {
        let JsValue::Object(sig_id) = v else {
            return Err(VmError::type_error(
                "AbortSignal.any: iterable element is not an AbortSignal",
            ));
        };
        if !matches!(ctx.vm.get_object(sig_id).kind, ObjectKind::AbortSignal) {
            return Err(VmError::type_error(
                "AbortSignal.any: iterable element is not an AbortSignal",
            ));
        }
        if first_aborted_reason.is_none() {
            if let Some(state) = ctx.vm.abort_signal_states.get(&sig_id) {
                if state.aborted {
                    first_aborted_reason = Some(state.reason);
                }
            }
        }
        inputs.push(sig_id);
    }

    // Validation succeeded — allocate the composite exactly once.
    let composite = ctx.vm.create_abort_signal();
    if let Some(reason) = first_aborted_reason {
        // Already-aborted fast path: composite is sync-aborted with
        // the first input's reason; fan-out registration is
        // redundant (any subsequent input abort would target an
        // already-aborted composite).
        abort_signal(ctx, composite, reason)?;
    } else {
        // Multi-input propagation (WHATWG §3.1.3.3) — register the
        // composite against every still-active input so each
        // input's `abort_signal` fire path fans out to it.  Skip
        // the composite itself (self-referential entries cannot
        // arise from the input list — `create_abort_signal`
        // allocates a fresh id — but this is the cheapest guard
        // against a future refactor that swaps the order of
        // allocate-after-validate).
        for input_sid in inputs {
            if input_sid == composite {
                continue;
            }
            ctx.vm
                .any_composite_map
                .entry(input_sid)
                .or_default()
                .push(composite);
        }
    }
    Ok(JsValue::Object(composite))
}
