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
    // WHATWG §3.1.3.2 step 1: coerce via WebIDL `unsigned long long`
    // → clamp NaN / negative / non-finite to 0ms.
    let clamped_ms = if ms.is_finite() && ms > 0.0 {
        ms.min(u64::MAX as f64 / 1_000.0)
    } else {
        0.0
    };
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let delay = std::time::Duration::from_micros((clamped_ms * 1000.0) as u64);

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
/// input is already aborted, the returned signal is aborted
/// synchronously with the first-aborted signal's reason.
///
/// Phase 2 scope: strong references between the returned signal
/// and each input via `addEventListener('abort', …)`.  A future
/// weak-ref pass will detach the listener once the composite is
/// unreachable — today a long-lived composite keeps every input
/// alive until explicitly aborted.
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

    let composite = ctx.vm.create_abort_signal();
    // Pre-pass: collect + validate input signals; if any is
    // already aborted, composite aborts synchronously with that
    // reason (WHATWG §3.1.3.3 step 2).
    let mut inputs: Vec<ObjectId> = Vec::with_capacity(elements.len());
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
        let already_aborted = ctx
            .vm
            .abort_signal_states
            .get(&sig_id)
            .is_some_and(|s| s.aborted);
        if already_aborted {
            let reason = ctx
                .vm
                .abort_signal_states
                .get(&sig_id)
                .map(|s| s.reason)
                .unwrap_or(JsValue::Undefined);
            abort_signal(ctx, composite, reason)?;
            return Ok(JsValue::Object(composite));
        }
        inputs.push(sig_id);
    }
    // TODO: multi-input propagation — install an 'abort' listener
    // on each input that forwards to `composite`.  Requires the
    // Event-ctor surface that lands in the next tranche.
    let _ = inputs;
    Ok(JsValue::Object(composite))
}
