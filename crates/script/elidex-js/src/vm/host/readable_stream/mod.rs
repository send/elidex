//! `ReadableStream` + default reader / controller (WHATWG Streams §4).
//!
//! Phase-2 scope: read-output-only.  The interfaces this module
//! installs are:
//!
//! - [`ObjectKind::ReadableStream`] (§4.2) — constructor + brand
//!   check + state machine (`readable` / `closed` / `errored`),
//!   queue, source callbacks (`start` / `pull` / `cancel`).
//! - [`ObjectKind::ReadableStreamDefaultReader`] (§4.3) — `read()` /
//!   `releaseLock()` / `cancel()` / `closed` getter.
//! - [`ObjectKind::ReadableStreamDefaultController`] (§4.5) —
//!   `enqueue()` / `close()` / `error()` / `desiredSize` getter.
//!
//! Stream **input** (`new Request(url, {body: aReadableStream})`)
//! is **not** yet supported — the request constructor throws a
//! TypeError instead of silently consuming the stream.  Lands with
//! M4-13.2 PR-streams-body-input.
//!
//! Output integration (`Response.body` / `Request.body` /
//! `Blob.prototype.stream()`) wires this module's `create_*` helpers
//! into the IDL accessors so consumers can `for await (const chunk
//! of response.body)`.
//!
//! ## Module layout
//!
//! - [`mod@self`] — state types, brand check helpers, stream
//!   constructor + start-step dispatch + `cancel` + `locked`
//!   stream-side natives, and registration of the
//!   `ReadableStream` global / prototype.
//! - [`controller`] — pull pump, controller mutators
//!   (`enqueue` / `close` / `error`), reader-delivery loops
//!   (`deliver_pending_reads`, `reject_pending_reads`,
//!   `deliver_close_to_reader`), and registration of the
//!   `ReadableStreamDefaultController` prototype.
//! - [`reader`] — `getReader`, `ReadableStreamDefaultReader`
//!   constructor + natives (`read` / `releaseLock` / `cancel` /
//!   `closed`), and registration of the reader prototype + global.
//! - [`body_adapter`] — [`body_adapter::create_body_backed_stream`],
//!   used by `Response.body` / `Request.body` / `Blob.stream()` to
//!   wrap a one-shot byte payload as a `ReadableStream` without an
//!   embedded JS source callback.
//! - [`strategies`] — `CountQueuingStrategy` /
//!   `ByteLengthQueuingStrategy` constructors + `size` methods.
//!
//! ## Out-of-scope (defer to M4-13)
//!
//! `tee` / `pipeTo` / `pipeThrough` / BYOB reader + byte-stream
//! controller / `WritableStream` / `TransformStream` are all
//! deferred — see plan §"Out-of-scope" for the per-item slot bind.

#![cfg(feature = "engine")]

use std::collections::VecDeque;

use super::super::natives_promise::{create_promise, settle_promise, subscribe_then};
use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};

mod body_adapter;
mod controller;
mod reader;
mod strategies;

pub(crate) use body_adapter::create_body_backed_stream;

use controller::{error_stream, finalize_close, pull_if_needed};

// ---------------------------------------------------------------------------
// State types
// ---------------------------------------------------------------------------

/// Underlying state machine for a [`ObjectKind::ReadableStream`].
///
/// WHATWG Streams §4.2 defines `[[state]]` as one of `"readable"` /
/// `"closed"` / `"errored"`.  Errored streams keep their error in
/// the [`ReadableStreamState::stored_error`] internal slot so a late
/// `getReader().closed` / `read()` can reject with the recorded
/// reason without re-deriving it.
#[allow(dead_code)] // Variants land in Stage 1a (state transitions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReadableStreamStateKind {
    /// Active — can accept enqueues and serve reads.
    Readable,
    /// `controller.close()` was called and the queue has been
    /// fully drained by readers.
    Closed,
    /// `controller.error()` was called or a source callback
    /// rejected — `stored_error` carries the reason.
    Errored,
}

/// Per-`ReadableStream` out-of-band state.  Keyed in
/// [`super::super::VmInner::readable_stream_states`] by the stream
/// instance's own `ObjectId`.
///
/// The stream owns the queue and the source callbacks; the
/// controller is a brand-checked façade that mutates this state
/// through `controller.stream_id`.  Queue chunks are arbitrary
/// `JsValue`s for the default-controller flavour (byte streams /
/// BYOB land with M4-13.3 and switch to `Vec<u8>` chunks).
#[allow(dead_code)] // Fields land progressively across Stages 1a / 1b / 2.
#[derive(Debug)]
pub(crate) struct ReadableStreamState {
    pub(crate) state: ReadableStreamStateKind,
    /// Paired controller's `ObjectId`.  Set on construction; never
    /// changes for the stream's lifetime.
    pub(crate) controller_id: ObjectId,
    /// Currently-attached default reader.  `None` when no reader
    /// is attached; `Some` makes the stream "locked" per spec
    /// §4.2.3 — re-deriving avoids a separate `locked: bool` slot.
    pub(crate) reader_id: Option<ObjectId>,
    /// Pending chunks awaiting a reader, paired with the
    /// `size(chunk)` value computed at enqueue time.  Storing the
    /// per-chunk size lets `deliver_pending_reads` decrement
    /// `queue_total_size` by the *exact* contribution this chunk
    /// added — without it, a custom size algorithm (e.g.
    /// `ByteLengthQueuingStrategy`) would invert the
    /// `desiredSize` math at dequeue time.  Spec §4.5.4 step 4
    /// invariant.
    pub(crate) queue: VecDeque<(JsValue, f64)>,
    /// Sum of `size(chunk)` across queued chunks (spec
    /// `[[queueTotalSize]]`).  `f64` because the size algorithm
    /// returns an arbitrary numeric value that may exceed `u64`.
    pub(crate) queue_total_size: f64,
    /// User-supplied `highWaterMark` (default 1 for default
    /// controller, 0 for byte controller in §4.5.2 step 6).
    pub(crate) high_water_mark: f64,
    /// User-supplied size algorithm.  `None` defaults to "always
    /// 1" (count strategy).  When set, called with the chunk and
    /// expected to return a finite numeric size.
    pub(crate) size_algorithm: Option<JsValue>,
    /// Spec `[[started]]`: flips after the start callback (sync or
    /// async) settles.  Gates `pull` invocations — §4.5.10 step 4.
    pub(crate) start_called: bool,
    /// Re-entrancy guard for `pull`.  Spec §4.5.10 step 4 & 7.
    pub(crate) pull_in_flight: bool,
    /// Spec `[[pullAgain]]`: when a `pull` finishes and another
    /// pull is still wanted, set instead of recursing.
    pub(crate) pull_again: bool,
    /// `true` after `controller.close()` schedules close-on-drain.
    /// Once the queue empties the stream transitions to `Closed`.
    pub(crate) close_requested: bool,
    /// Source callbacks.  `None` for "no callback supplied" — spec
    /// §4.2.5 lets each of the three be omitted independently.
    pub(crate) source_start: Option<JsValue>,
    pub(crate) source_pull: Option<JsValue>,
    pub(crate) source_cancel: Option<JsValue>,
    /// The underlying-source object (the first ctor arg) — used
    /// as the `this` receiver for `start` / `pull` / `cancel`
    /// invocations per spec InvokeOrNoop semantics, so a JS
    /// `pull() { this.enqueue(...) }` shape works (Copilot R5
    /// finding).  `None` for VM-controlled streams (body /
    /// Blob.stream) where the callbacks are `None` anyway.
    pub(crate) underlying_source: Option<JsValue>,
    /// Stored error reason once the stream has transitioned to
    /// `Errored`.  Read by late `read()` / `closed` rejections.
    /// `Undefined` when not in errored state.
    pub(crate) stored_error: JsValue,
}

/// Per-`ReadableStreamDefaultReader` out-of-band state.  Keyed in
/// [`super::super::VmInner::readable_stream_reader_states`].
///
/// Spec §4.3.2 `[[readRequests]]` lives here as `pending_read_promises`
/// — owning the promises through the reader (rather than a VM-level
/// strong root list) makes their lifetime spec-direct: when the
/// reader is collected, the read promises become unreachable too.
#[allow(dead_code)] // Fields wired up in Stage 1b.
#[derive(Debug)]
pub(crate) struct ReaderState {
    /// `Some(stream)` while the reader is locked to a stream.
    /// Flipped to `None` by `releaseLock()` — subsequent reads
    /// reject and the cached `closed` promise rejects.
    pub(crate) stream_id: Option<ObjectId>,
    /// FIFO of pending `read()` promises.  Resolved by
    /// `controller.enqueue` (with `{value, done: false}`),
    /// `controller.close` (with `{value: undefined, done: true}`),
    /// or rejected by `controller.error`.
    pub(crate) pending_read_promises: VecDeque<ObjectId>,
    /// Cached `closed` promise — spec §4.3.5 step 6 requires
    /// `releaseLock()` to swap this out for a fresh rejected
    /// instance, so it is mutable.
    pub(crate) closed_promise: ObjectId,
}

// ---------------------------------------------------------------------------
// Brand checks
// ---------------------------------------------------------------------------

pub(super) fn require_stream_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "ReadableStream.prototype.{method} called on non-ReadableStream"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::ReadableStream) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "ReadableStream.prototype.{method} called on non-ReadableStream"
        )))
    }
}

// ---------------------------------------------------------------------------
// State accessors
// ---------------------------------------------------------------------------

pub(super) fn stream_state(vm: &VmInner, stream_id: ObjectId) -> &ReadableStreamState {
    vm.readable_stream_states
        .get(&stream_id)
        .expect("ReadableStream without readable_stream_states entry")
}

pub(super) fn stream_state_mut(vm: &mut VmInner, stream_id: ObjectId) -> &mut ReadableStreamState {
    vm.readable_stream_states
        .get_mut(&stream_id)
        .expect("ReadableStream without readable_stream_states entry")
}

// ---------------------------------------------------------------------------
// Constructor + helpers
// ---------------------------------------------------------------------------

/// Coerce a `highWaterMark` *property value* to a non-negative
/// finite f64 per spec §4.7 / §4.8 (and §6.1
/// ValidateAndNormalizeHighWaterMark).  Phase 2: rejects `NaN`
/// and negative numbers with `RangeError`, and rejects
/// non-Number / non-Undefined inputs (including `null`) with
/// `TypeError`.  Full ToNumber coercion lands with M4-13
/// spec-polish — at that point `null` would coerce to 0 and
/// other non-numerics to NaN (RangeError path).  `+Infinity` is
/// allowed (spec permits but pull algorithm checks `finite-ish`
/// behaviour separately).
///
/// Note this is the *property-value* coercion path — distinct
/// from the *positional argument* path in
/// `native_readable_stream_constructor`, which treats both
/// `undefined` and `null` as "no init object supplied" per
/// WebIDL dict-from-null semantics (matches Chromium's
/// `new ReadableStream(null)` acceptance).
fn normalize_high_water_mark(hwm: JsValue) -> Result<f64, VmError> {
    let n = match hwm {
        JsValue::Number(n) => n,
        JsValue::Undefined => return Ok(1.0),
        _ => return Err(VmError::type_error(
            "Failed to construct 'ReadableStream': highWaterMark coercion not supported in Phase 2",
        )),
    };
    if n.is_nan() || n < 0.0 {
        return Err(VmError::range_error(
            "Failed to construct 'ReadableStream': highWaterMark must be a non-negative number",
        ));
    }
    Ok(n)
}

/// Look up an optional callable property on an underlying-source /
/// queuing-strategy object.  Returns `None` for `undefined`,
/// `Some(value)` for a callable, and `Err` for a non-callable
/// non-undefined property — matches WHATWG Streams §4.2.4 step
/// "if startMethod is not undefined, callable check".
pub(super) fn extract_optional_callable(
    ctx: &mut NativeContext<'_>,
    obj_id: ObjectId,
    name_sid: StringId,
    debug_label: &str,
) -> Result<Option<JsValue>, VmError> {
    let key = PropertyKey::String(name_sid);
    let value = match super::super::coerce::get_property(ctx.vm, obj_id, key) {
        Some(prop) => ctx.vm.resolve_property(prop, JsValue::Object(obj_id))?,
        None => return Ok(None),
    };
    if matches!(value, JsValue::Undefined) {
        return Ok(None);
    }
    let JsValue::Object(fn_id) = value else {
        return Err(VmError::type_error(format!(
            "Failed to construct 'ReadableStream': {debug_label} is not a function"
        )));
    };
    if !ctx.vm.get_object(fn_id).kind.is_callable() {
        return Err(VmError::type_error(format!(
            "Failed to construct 'ReadableStream': {debug_label} is not a function"
        )));
    }
    Ok(Some(value))
}

/// `new ReadableStream(underlyingSource?, queuingStrategy?)`
/// (WHATWG Streams §4.2.3).
pub(super) fn native_readable_stream_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'ReadableStream': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(stream_id) = this else {
        unreachable!("ctor `this` always Object after `do_new`");
    };

    let source_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let strategy_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);

    // Underlying-source callbacks.  All three are optional.
    // `null` is treated as `undefined` per WebIDL dict-from-null
    // semantics (matches Chromium / Firefox `new
    // ReadableStream(null)`).
    let (source_start, source_pull, source_cancel) = match source_arg {
        JsValue::Undefined | JsValue::Null => (None, None, None),
        JsValue::Object(src_id) => {
            let start_sid = ctx.vm.well_known.start;
            let pull_sid = ctx.vm.well_known.pull;
            let cancel_sid = ctx.vm.well_known.cancel;
            (
                extract_optional_callable(ctx, src_id, start_sid, "underlyingSource.start")?,
                extract_optional_callable(ctx, src_id, pull_sid, "underlyingSource.pull")?,
                extract_optional_callable(ctx, src_id, cancel_sid, "underlyingSource.cancel")?,
            )
        }
        _ => {
            return Err(VmError::type_error(
                "Failed to construct 'ReadableStream': underlyingSource must be an object",
            ));
        }
    };

    // Queuing strategy: { highWaterMark?, size? }.  `size` defaults
    // to "always 1" for the default-controller flavour (count
    // strategy).  Phase 2 only supports a Number `highWaterMark` —
    // generic ToNumber coercion lands with the spec-polish PR.
    let (high_water_mark, size_algorithm) = match strategy_arg {
        JsValue::Undefined | JsValue::Null => (1.0, None),
        JsValue::Object(strat_id) => {
            let hwm_key = PropertyKey::String(ctx.vm.well_known.high_water_mark);
            let hwm_value = match super::super::coerce::get_property(ctx.vm, strat_id, hwm_key) {
                Some(prop) => ctx.vm.resolve_property(prop, strategy_arg)?,
                None => JsValue::Undefined,
            };
            let hwm = normalize_high_water_mark(hwm_value)?;
            let size_sid = ctx.vm.well_known.size;
            let size_alg = extract_optional_callable(ctx, strat_id, size_sid, "size")?;
            (hwm, size_alg)
        }
        _ => {
            return Err(VmError::type_error(
                "Failed to construct 'ReadableStream': queuingStrategy must be an object",
            ));
        }
    };

    // Allocate the controller — payload-free until we know the
    // stream's id below; we patch `stream_id` after the stream is
    // allocated.  Order: stream first, then controller pointing at
    // it; back-reference is stored in `state.controller_id`.
    let controller_proto = ctx.vm.readable_stream_default_controller_prototype;
    let controller_id = ctx.vm.alloc_object(Object {
        kind: ObjectKind::ReadableStreamDefaultController { stream_id },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: controller_proto,
        extensible: true,
    });

    // Promote the pre-allocated Ordinary `this` to ReadableStream
    // (matches Blob / Headers / Request / Response patterns —
    // preserving `do_new`'s prototype chain).
    ctx.vm.get_object_mut(stream_id).kind = ObjectKind::ReadableStream;

    ctx.vm.readable_stream_states.insert(
        stream_id,
        ReadableStreamState {
            state: ReadableStreamStateKind::Readable,
            controller_id,
            reader_id: None,
            queue: VecDeque::new(),
            queue_total_size: 0.0,
            high_water_mark,
            size_algorithm,
            start_called: false,
            pull_in_flight: false,
            pull_again: false,
            close_requested: false,
            source_start,
            source_pull,
            source_cancel,
            underlying_source: match source_arg {
                JsValue::Object(_) => Some(source_arg),
                _ => None,
            },
            stored_error: JsValue::Undefined,
        },
    );

    // Run start callback synchronously per §4.2.4 step 8-11.
    if let Some(start_cb) = source_start {
        let JsValue::Object(start_fn_id) = start_cb else {
            unreachable!("source_start was callable-checked");
        };
        // Spec InvokeOrNoop: `this` is the underlyingSource
        // object so `start() { this.enqueue(...) }` works.
        let this_arg = source_arg;
        let result = ctx.call_function(start_fn_id, this_arg, &[JsValue::Object(controller_id)]);
        match result {
            Ok(value) => finalize_start(ctx.vm, stream_id, value),
            Err(err) => {
                // Sync throw from start — error the stream and
                // surface the throw to the constructor caller per
                // spec ReadableStreamDefaultControllerError +
                // throw propagation.
                let reason = ctx.vm.vm_error_to_thrown(&err);
                error_stream(ctx.vm, stream_id, reason);
                return Err(err);
            }
        }
    } else {
        // No start callback supplied — equivalent to start
        // returning `undefined` synchronously.
        finalize_start(ctx.vm, stream_id, JsValue::Undefined);
    }

    Ok(JsValue::Object(stream_id))
}

/// Resolve `start_result` into a Promise and subscribe step
/// callables that flip `start_called` / pull-on-success or error
/// the stream on rejection (spec §4.2.4 step 9-11).
///
/// `p` is rooted across the step-callable allocations so a GC
/// triggered by `alloc_object` can't sweep the still-unreachable
/// Promise (Copilot R7 GC-safety finding).  The two step callables
/// each get rooted in turn so the next `alloc_object` can't
/// collect the previous one either.
fn finalize_start(vm: &mut VmInner, stream_id: ObjectId, start_result: JsValue) {
    let p = create_promise(vm);
    let _ = settle_promise(vm, p, false, start_result);
    let mut g_p = vm.push_temp_root(JsValue::Object(p));
    let on_fulfilled_proto = g_p.function_prototype;
    let on_fulfilled = g_p.alloc_object(Object {
        kind: ObjectKind::ReadableStreamStartStep {
            stream_id,
            is_reject: false,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: on_fulfilled_proto,
        extensible: true,
    });
    let mut g_f = g_p.push_temp_root(JsValue::Object(on_fulfilled));
    let on_rejected_proto = g_f.function_prototype;
    let on_rejected = g_f.alloc_object(Object {
        kind: ObjectKind::ReadableStreamStartStep {
            stream_id,
            is_reject: true,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: on_rejected_proto,
        extensible: true,
    });
    subscribe_then(&mut g_f, p, on_fulfilled, on_rejected);
    drop(g_f);
    drop(g_p);
}

/// Dispatcher for `ObjectKind::ReadableStreamStartStep` — called
/// from `interpreter.rs` when the start promise settles.
pub(crate) fn run_start_step(
    vm: &mut VmInner,
    stream_id: ObjectId,
    is_reject: bool,
    value: JsValue,
) -> Result<JsValue, VmError> {
    if !vm.readable_stream_states.contains_key(&stream_id) {
        // Stream was collected — defensive no-op (the trace step
        // marks the stream while a step is queued, so this is
        // unreachable in normal operation).
        return Ok(JsValue::Undefined);
    }
    if is_reject {
        error_stream(vm, stream_id, value);
        return Ok(JsValue::Undefined);
    }
    {
        let state = stream_state_mut(vm, stream_id);
        state.start_called = true;
    }
    pull_if_needed(vm, stream_id);
    Ok(JsValue::Undefined)
}

/// Dispatcher for `ObjectKind::ReadableStreamPullStep`.
pub(crate) fn run_pull_step(
    vm: &mut VmInner,
    stream_id: ObjectId,
    is_reject: bool,
    value: JsValue,
) -> Result<JsValue, VmError> {
    if !vm.readable_stream_states.contains_key(&stream_id) {
        return Ok(JsValue::Undefined);
    }
    if is_reject {
        error_stream(vm, stream_id, value);
        return Ok(JsValue::Undefined);
    }
    let pull_again = {
        let state = stream_state_mut(vm, stream_id);
        state.pull_in_flight = false;
        let again = state.pull_again;
        state.pull_again = false;
        again
    };
    // Spec §4.5.10 step "Upon fulfillment of pullPromise":
    // re-call pull-if-needed *only* when `pullAgain` was set
    // during the in-flight pull.  Calling unconditionally here
    // would create an infinite microtask loop when `pull()`
    // returns without enqueueing — `desiredSize` stays positive
    // forever, so a fresh pull would fire each microtask tick
    // (Copilot R7 finding).  New reads that arrive after this
    // step go through `native_reader_read` → `pull_if_needed`,
    // so the pump still wakes up on demand.
    if pull_again {
        pull_if_needed(vm, stream_id);
    }
    Ok(JsValue::Undefined)
}

/// Dispatcher for `ObjectKind::ReadableStreamCancelStep`.
/// Settles the caller's `cancel()` promise after `source.cancel()`
/// completes.  Spec §4.2.6 step 5: a rejection from source.cancel
/// **does** propagate to the caller's promise; resolution always
/// passes `undefined`.
pub(crate) fn run_cancel_step(
    vm: &mut VmInner,
    promise: ObjectId,
    is_reject: bool,
    value: JsValue,
) -> Result<JsValue, VmError> {
    if is_reject {
        let _ = settle_promise(vm, promise, true, value);
    } else {
        let _ = settle_promise(vm, promise, false, JsValue::Undefined);
    }
    Ok(JsValue::Undefined)
}

// ---------------------------------------------------------------------------
// Stream methods (locked / cancel)
// ---------------------------------------------------------------------------

pub(super) fn native_readable_stream_get_locked(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_stream_this(ctx, this, "locked")?;
    let locked = ctx
        .vm
        .readable_stream_states
        .get(&id)
        .is_some_and(|s| s.reader_id.is_some());
    Ok(JsValue::Boolean(locked))
}

/// `stream.cancel(reason?)` (WHATWG Streams §4.2.6).
///
/// Spec step 1: if `IsReadableStreamLocked(this) is true`, return
/// a Promise rejected with TypeError.  The reader-side cancel
/// (`reader.cancel`) goes through `do_stream_cancel` directly —
/// it bypasses this lock check because the reader being attached
/// is what `locked` describes, and reader-side cancel is
/// exactly the spec-blessed way to cancel a locked stream.
pub(super) fn native_readable_stream_cancel(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_stream_this(ctx, this, "cancel")?;
    if stream_state(ctx.vm, id).reader_id.is_some() {
        let p = create_promise(ctx.vm);
        let err = VmError::type_error(
            "Failed to execute 'cancel' on 'ReadableStream': cannot cancel a locked stream",
        );
        let reason = ctx.vm.vm_error_to_thrown(&err);
        let _ = settle_promise(ctx.vm, p, true, reason);
        return Ok(JsValue::Object(p));
    }
    let reason = args.first().copied().unwrap_or(JsValue::Undefined);
    Ok(JsValue::Object(do_stream_cancel(ctx.vm, id, reason)))
}

/// Shared implementation for `stream.cancel` + `reader.cancel`
/// (Stage 1b).  Returns the settle-promise's ObjectId.
///
/// The returned Promise `p` is rooted immediately after creation
/// so the subsequent `alloc_object` step-callable allocations
/// (and any user-JS path through `source.cancel`) cannot collect
/// it (Copilot R7 GC-safety finding).  `inner` (the wrapper
/// around source.cancel's resolution) is rooted symmetrically.
pub(super) fn do_stream_cancel(vm: &mut VmInner, stream_id: ObjectId, reason: JsValue) -> ObjectId {
    let p = create_promise(vm);
    let mut g_p = vm.push_temp_root(JsValue::Object(p));

    let state_kind = stream_state(&g_p, stream_id).state;
    match state_kind {
        ReadableStreamStateKind::Closed => {
            let _ = settle_promise(&mut g_p, p, false, JsValue::Undefined);
            drop(g_p);
            return p;
        }
        ReadableStreamStateKind::Errored => {
            let stored = stream_state(&g_p, stream_id).stored_error;
            let _ = settle_promise(&mut g_p, p, true, stored);
            drop(g_p);
            return p;
        }
        ReadableStreamStateKind::Readable => {}
    }

    {
        let state = stream_state_mut(&mut g_p, stream_id);
        state.queue.clear();
        state.queue_total_size = 0.0;
    }
    finalize_close(&mut g_p, stream_id);

    let cancel_cb = stream_state(&g_p, stream_id).source_cancel;
    let this_arg = stream_state(&g_p, stream_id)
        .underlying_source
        .unwrap_or(JsValue::Undefined);
    if let Some(JsValue::Object(fn_id)) = cancel_cb {
        let result = {
            let mut ctx = NativeContext { vm: &mut g_p };
            ctx.call_function(fn_id, this_arg, &[reason])
        };
        match result {
            Ok(value) => {
                let inner = create_promise(&mut g_p);
                let _ = settle_promise(&mut g_p, inner, false, value);
                let mut g_inner = g_p.push_temp_root(JsValue::Object(inner));
                let on_fulfilled_proto = g_inner.function_prototype;
                let on_fulfilled = g_inner.alloc_object(Object {
                    kind: ObjectKind::ReadableStreamCancelStep {
                        promise: p,
                        is_reject: false,
                    },
                    storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                    prototype: on_fulfilled_proto,
                    extensible: true,
                });
                let mut g_f = g_inner.push_temp_root(JsValue::Object(on_fulfilled));
                let on_rejected_proto = g_f.function_prototype;
                let on_rejected = g_f.alloc_object(Object {
                    kind: ObjectKind::ReadableStreamCancelStep {
                        promise: p,
                        is_reject: true,
                    },
                    storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                    prototype: on_rejected_proto,
                    extensible: true,
                });
                subscribe_then(&mut g_f, inner, on_fulfilled, on_rejected);
                drop(g_f);
                drop(g_inner);
            }
            Err(err) => {
                let r = g_p.vm_error_to_thrown(&err);
                let _ = settle_promise(&mut g_p, p, true, r);
            }
        }
    } else {
        let _ = settle_promise(&mut g_p, p, false, JsValue::Undefined);
    }
    drop(g_p);
    p
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Register the `ReadableStream` + `ReadableStreamDefaultController`
    /// globals + prototypes.  The matching
    /// [`Self::register_readable_stream_reader_global`] runs
    /// afterwards (see `globals.rs`) and back-patches `getReader`
    /// onto the stream prototype once the reader prototype exists.
    pub(in crate::vm) fn register_readable_stream_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_readable_stream_global called before register_prototypes");

        // Stream prototype.
        let stream_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_readable_stream_members(stream_proto);
        self.readable_stream_prototype = Some(stream_proto);

        let stream_ctor = self
            .create_constructable_function("ReadableStream", native_readable_stream_constructor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            stream_ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(stream_proto)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            stream_proto,
            ctor_key,
            PropertyValue::Data(JsValue::Object(stream_ctor)),
            PropertyAttrs::METHOD,
        );
        self.globals.insert(
            self.well_known.readable_stream_global,
            JsValue::Object(stream_ctor),
        );

        // Controller prototype.
        let controller_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_readable_stream_default_controller_members(controller_proto);
        self.readable_stream_default_controller_prototype = Some(controller_proto);

        let controller_ctor = self.create_constructable_function(
            "ReadableStreamDefaultController",
            controller::native_default_controller_illegal_constructor,
        );
        self.define_shaped_property(
            controller_ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(controller_proto)),
            PropertyAttrs::BUILTIN,
        );
        self.define_shaped_property(
            controller_proto,
            ctor_key,
            PropertyValue::Data(JsValue::Object(controller_ctor)),
            PropertyAttrs::METHOD,
        );
        self.globals.insert(
            self.well_known.readable_stream_default_controller_global,
            JsValue::Object(controller_ctor),
        );
    }

    fn install_readable_stream_members(&mut self, proto_id: ObjectId) {
        // `locked` getter.
        let locked_sid = self.well_known.locked_attr;
        self.install_accessor_pair(
            proto_id,
            locked_sid,
            native_readable_stream_get_locked as NativeFn,
            None,
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );
        // `cancel(reason?)` method.
        let cancel_sid = self.well_known.cancel;
        self.install_native_method(
            proto_id,
            cancel_sid,
            native_readable_stream_cancel as NativeFn,
            PropertyAttrs::METHOD,
        );
    }
}
