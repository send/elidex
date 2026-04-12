//! Timer globals (WHATWG HTML §8.7).
//!
//! `setTimeout(callback, delay, ...args)` / `setInterval(...)` schedule a
//! callback on the VM's timer min-heap.  `clearTimeout` / `clearInterval`
//! cancel a pending entry by id.  The host drives execution by calling
//! [`VmInner::drain_timers`] on each event-loop tick — typical wiring
//! lives in the shell (PR6).

// `drain_timers` / `fire_timer` are dead until PR6 wires them into the
// shell; same for `TimerEntry.repeat` via pattern destructuring.  Allow
// until then so the rest of the file stays warning-clean.
#![allow(dead_code)]

use std::time::{Duration, Instant};

use super::value::{JsValue, NativeContext, VmError};
use super::{TimerEntry, VmInner};

// ---------------------------------------------------------------------------
// Scheduling primitives
// ---------------------------------------------------------------------------

/// Core scheduler: allocates an id, pushes a [`TimerEntry`] onto the heap.
/// `repeat=None` for `setTimeout`, `Some(delay)` for `setInterval`.
fn schedule_timer(
    ctx: &mut NativeContext<'_>,
    args: &[JsValue],
    repeat: bool,
) -> Result<JsValue, VmError> {
    let callback = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(func) = callback else {
        // WHATWG §8.7 step 1: non-function handler is stringified + eval'd.
        // We don't support string handlers (well-known XSS footgun and a
        // compiler-level re-entry point we haven't plumbed).  Throw so the
        // caller notices.
        return Err(VmError::type_error(
            "setTimeout/setInterval handler must be a function (string handlers not supported)",
        ));
    };
    if !ctx.get_object(func).kind.is_callable() {
        return Err(VmError::type_error(
            "setTimeout/setInterval handler is not callable",
        ));
    }

    // Delay: arg[1] clamped to >= 0 ms.  Non-finite / negative → 0.
    let delay_ms = match args.get(1).copied() {
        Some(JsValue::Number(n)) if n.is_finite() && n > 0.0 => n,
        _ => 0.0,
    };
    // Clamp to a safe f64→u64 range.  2^53 microseconds is ~285 years,
    // well beyond any realistic timer horizon; anything larger is not
    // representable in f64 anyway.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let delay = {
        let micros_f64 = (delay_ms * 1000.0).min((1u64 << 53) as f64);
        Duration::from_micros(micros_f64 as u64)
    };

    let positional = if args.len() > 2 {
        args[2..].to_vec()
    } else {
        Vec::new()
    };

    let id = ctx.vm.next_timer_id;
    ctx.vm.next_timer_id = ctx.vm.next_timer_id.wrapping_add(1);
    ctx.vm.timer_queue.push(TimerEntry {
        id,
        deadline: Instant::now() + delay,
        callback: func,
        repeat: if repeat { Some(delay) } else { None },
        args: positional,
    });
    Ok(JsValue::Number(f64::from(id)))
}

/// `setTimeout(callback, delay, ...args)` — WHATWG §8.7.
pub(super) fn native_set_timeout(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    schedule_timer(ctx, args, false)
}

/// `setInterval(callback, delay, ...args)` — WHATWG §8.7.
pub(super) fn native_set_interval(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    schedule_timer(ctx, args, true)
}

fn cancel_timer(ctx: &mut NativeContext<'_>, args: &[JsValue]) -> Result<JsValue, VmError> {
    let Some(JsValue::Number(n)) = args.first().copied() else {
        // Non-number ids are silently ignored (WHATWG §8.7 step 1 Note).
        return Ok(JsValue::Undefined);
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let id = n as u32;
    ctx.vm.cancelled_timers.insert(id);
    Ok(JsValue::Undefined)
}

/// `clearTimeout(id)` — WHATWG §8.7.
pub(super) fn native_clear_timeout(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    cancel_timer(ctx, args)
}

/// `clearInterval(id)` — WHATWG §8.7.
pub(super) fn native_clear_interval(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    cancel_timer(ctx, args)
}

// ---------------------------------------------------------------------------
// Drain — invoked by the event loop on each tick
// ---------------------------------------------------------------------------

impl VmInner {
    /// Fire every timer whose deadline is `<= now`.  Cancelled entries
    /// are skipped; interval entries are re-queued with `deadline + repeat`.
    /// Callback exceptions are reported via `eprintln!` so a single bad
    /// timer doesn't abort the whole drain — a PR6 follow-up will route
    /// these to `host.session().report_error(...)`.
    ///
    /// After firing expired timers, drains the microtask queue (HTML
    /// §8.1.4.2 step 8).
    pub(crate) fn drain_timers(&mut self, now: Instant) -> usize {
        let mut fired = 0usize;
        loop {
            // Peek, not pop, so cancelled entries in the future don't
            // disappear — we only evict the head when it's ready to fire.
            let head_ready = self
                .timer_queue
                .peek()
                .is_some_and(|entry| entry.deadline <= now);
            if !head_ready {
                break;
            }
            let entry = self
                .timer_queue
                .pop()
                .expect("head_ready implies non-empty");
            if self.cancelled_timers.remove(&entry.id) {
                continue;
            }
            if let Err(e) = self.fire_timer(&entry) {
                eprintln!("timer callback {} threw: {e}", entry.id);
            }
            fired += 1;
            // Interval: re-arm.  Even if this firing ran long, we keep the
            // "scheduled" deadline monotonic rather than drifting with
            // callback duration — matches browser scheduler semantics
            // closely enough for now.
            if let Some(repeat) = entry.repeat {
                self.timer_queue.push(TimerEntry {
                    id: entry.id,
                    deadline: entry.deadline + repeat,
                    callback: entry.callback,
                    repeat: Some(repeat),
                    args: entry.args,
                });
            }
        }
        self.drain_microtasks();
        fired
    }

    /// Invoke a single timer's callback.  Bridges the stored
    /// `ObjectId` + args into `vm.call()`; the callee's `this` is
    /// `globalThis` per WHATWG §8.7 step 13.1.
    fn fire_timer(&mut self, entry: &TimerEntry) -> Result<(), VmError> {
        // Sanity: if the callback object was GC'd (shouldn't happen —
        // the timer_queue marks it live), bail quietly.
        let still_callable = {
            let slot = self
                .objects
                .get(entry.callback.0 as usize)
                .and_then(Option::as_ref);
            slot.is_some_and(|o| o.kind.is_callable())
        };
        if !still_callable {
            return Ok(());
        }
        let this = JsValue::Object(self.global_object);
        let _ = self.call(entry.callback, this, &entry.args)?;
        Ok(())
    }
}
