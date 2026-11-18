//! `AbortSignal` static factories — `abort(reason?)` /
//! `timeout(ms)` / `any(signals)` (DOM §3.2 Interface AbortSignal: the
//! static `abort(reason)` / `timeout(milliseconds)` / `any(signals)`
//! method steps).
//!
//! Split out of [`super::abort`] to keep that module under the
//! project's 1000-line convention.  The functions live as own
//! methods on the `AbortSignal` constructor function object (not on
//! `AbortSignal.prototype`), installed by
//! [`VmInner::register_abort_signal_global`](crate::vm::VmInner::register_abort_signal_global).
//!
//! The core signal state machine ([`super::abort::abort_signal`])
//! remains in `abort.rs`; these factories compose fresh signal
//! allocation / timer scheduling with the engine-independent
//! "create a dependent abort signal" algorithm in
//! [`elidex_dom_api::abort`].

#![cfg(feature = "engine")]

use elidex_dom_api::abort::{create_dependent_abort_signal, AbortSignalGraph};

use super::super::value::{JsValue, NativeContext, ObjectId, ObjectKind, VmError};

use super::abort::materialise_abort_reason;

/// `AbortSignal.abort(reason?)` — returns an already-aborted signal
/// (DOM §3.2 "The static abort(reason) method steps").  Observably
/// equivalent to:
///
/// ```js
/// const c = new AbortController();
/// c.abort(reason);
/// c.signal;
/// ```
///
/// …but without allocating the controller.  The steps only *set* the
/// new signal's abort reason (step 2) and return it — no abort steps run
/// (a fresh signal has no listeners or algorithms).
pub(super) fn native_abort_signal_static_abort(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let signal_id = ctx.vm.create_abort_signal();
    // Static abort(reason) method steps step 2.
    let reason =
        materialise_abort_reason(ctx.vm, args.first().copied().unwrap_or(JsValue::Undefined));
    ctx.vm.set_abort_reason(signal_id, reason);
    Ok(JsValue::Object(signal_id))
}

/// `AbortSignal.timeout(ms)` — returns a signal that aborts with a
/// `DOMException("TimeoutError")` reason after `ms` milliseconds
/// (DOM §3.2 "The static timeout(milliseconds) method steps").  The timer is managed via
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

/// `AbortSignal.any(signals)` — DOM §3.2 "The static any(signals) method
/// steps are to return the result of creating a dependent abort signal from
/// signals using AbortSignal and the current realm."
///
/// The algorithm itself is [`create_dependent_abort_signal`] in
/// [`elidex_dom_api::abort`]; this native keeps what is engine-bound: the
/// argument read, the brand checks and the allocation of the result.  The
/// result records the **non-dependent** sources of its inputs (a dependent
/// input contributes its own source signals), so no dependent signal has
/// dependent signals of its own and a source's abort reaches every
/// transitive dependent in one step.
///
/// The argument is read as an `Array` only (no `Symbol.iterator`): the
/// WebIDL `sequence<AbortSignal>` conversion is the concern of
/// `#11-webidl-sequence-dense-array-fast-path` and
/// `#11-vm-webidl-sequence-early-exit-drain`.
pub(super) fn native_abort_signal_static_any(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let iterable = args.first().copied().unwrap_or(JsValue::Undefined);
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
    // the result signal so a throw on a bogus element does not
    // strand an unreferenced entry in `abort_signal_states`.
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
        inputs.push(sig_id);
    }

    // Validation succeeded — allocate the result exactly once (create a
    // dependent abort signal step 1), then run steps 2-5.
    let result = ctx.vm.create_abort_signal();
    // Steps 2-5 are the module's: an aborted input makes the result take
    // its reason (not dependent; no abort steps run — a fresh signal has no
    // listeners or algorithms), otherwise the result is linked.  The
    // outcome is on the result's row.
    create_dependent_abort_signal(&mut *ctx.vm, result, &inputs);
    Ok(JsValue::Object(result))
}
