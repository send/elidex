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

fn require_stream_this(
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

fn require_controller_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "ReadableStreamDefaultController.prototype.{method} called on non-controller"
        )));
    };
    if matches!(
        ctx.vm.get_object(id).kind,
        ObjectKind::ReadableStreamDefaultController { .. }
    ) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "ReadableStreamDefaultController.prototype.{method} called on non-controller"
        )))
    }
}

/// Walk a controller's internal `stream_id` slot.
fn controller_stream_id(vm: &VmInner, controller_id: ObjectId) -> ObjectId {
    let ObjectKind::ReadableStreamDefaultController { stream_id } =
        vm.get_object(controller_id).kind
    else {
        unreachable!("controller_stream_id: caller did not brand-check");
    };
    stream_id
}

// ---------------------------------------------------------------------------
// State accessors
// ---------------------------------------------------------------------------

fn stream_state(vm: &VmInner, stream_id: ObjectId) -> &ReadableStreamState {
    vm.readable_stream_states
        .get(&stream_id)
        .expect("ReadableStream without readable_stream_states entry")
}

fn stream_state_mut(vm: &mut VmInner, stream_id: ObjectId) -> &mut ReadableStreamState {
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
fn extract_optional_callable(
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
fn finalize_start(vm: &mut VmInner, stream_id: ObjectId, start_result: JsValue) {
    // `Promise.resolve(start_result)` — a Promise-typed resolution
    // is forwarded; non-Promise resolves immediately.  Either way
    // we can subscribe via `subscribe_then`.
    let p = create_promise(vm);
    let _ = settle_promise(vm, p, false, start_result);
    let on_fulfilled = vm.alloc_object(Object {
        kind: ObjectKind::ReadableStreamStartStep {
            stream_id,
            is_reject: false,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.function_prototype,
        extensible: true,
    });
    let on_rejected = vm.alloc_object(Object {
        kind: ObjectKind::ReadableStreamStartStep {
            stream_id,
            is_reject: true,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.function_prototype,
        extensible: true,
    });
    subscribe_then(vm, p, on_fulfilled, on_rejected);
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
    if pull_again {
        pull_if_needed(vm, stream_id);
    } else {
        // After pull settled, more chunks may be desired —
        // recheck.  `pull_if_needed` self-gates on `desired_size`.
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
// Pull pump
// ---------------------------------------------------------------------------

/// Compute `[[strategyHWM]] - [[queueTotalSize]]` (the spec
/// `[[desiredSize]]` slot).  Returns `f64` to preserve the
/// `+Infinity - finite` edges.
fn desired_size(state: &ReadableStreamState) -> f64 {
    state.high_water_mark - state.queue_total_size
}

/// Should the pull pump fire?  Spec §4.5.10 step 1-3:
/// - Stream must be `Readable`.
/// - `[[desiredSize]] > 0`.
/// - Either `start_called` (pull only fires after start settles).
fn pull_should_fire(state: &ReadableStreamState) -> bool {
    state.state == ReadableStreamStateKind::Readable
        && !state.close_requested
        && state.start_called
        && desired_size(state) > 0.0
}

/// Drive the pull pump (§4.5.10 ReadableStreamDefaultControllerCallPullIfNeeded).
fn pull_if_needed(vm: &mut VmInner, stream_id: ObjectId) {
    let (should_pull, pull_cb, controller_id, underlying_source) = {
        let state = stream_state(vm, stream_id);
        (
            pull_should_fire(state),
            state.source_pull,
            state.controller_id,
            state.underlying_source,
        )
    };
    if !should_pull {
        return;
    }
    if stream_state(vm, stream_id).pull_in_flight {
        // Re-entrancy: defer until current pull settles — set
        // `pull_again` so `run_pull_step` re-fires.
        stream_state_mut(vm, stream_id).pull_again = true;
        return;
    }
    stream_state_mut(vm, stream_id).pull_in_flight = true;

    let Some(pull_cb) = pull_cb else {
        // No pull callback — treat as instantly resolved.
        // Spec: a missing pull is equivalent to one that returns
        // `undefined` synchronously.
        let state = stream_state_mut(vm, stream_id);
        state.pull_in_flight = false;
        return;
    };
    let JsValue::Object(pull_fn_id) = pull_cb else {
        return;
    };

    // Invoke pull(controller).  Sync throw → error stream.
    // Spec InvokeOrNoop: `this` is the underlyingSource so
    // `pull() { this.enqueue(...) }` works.
    let this_arg = underlying_source.unwrap_or(JsValue::Undefined);
    let result = {
        let mut ctx = NativeContext { vm };
        ctx.call_function(pull_fn_id, this_arg, &[JsValue::Object(controller_id)])
    };
    match result {
        Ok(value) => {
            // Wrap result in a Promise and subscribe pull step
            // callables (mirrors finalize_start).
            let p = create_promise(vm);
            let _ = settle_promise(vm, p, false, value);
            let on_fulfilled = vm.alloc_object(Object {
                kind: ObjectKind::ReadableStreamPullStep {
                    stream_id,
                    is_reject: false,
                },
                storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                prototype: vm.function_prototype,
                extensible: true,
            });
            let on_rejected = vm.alloc_object(Object {
                kind: ObjectKind::ReadableStreamPullStep {
                    stream_id,
                    is_reject: true,
                },
                storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                prototype: vm.function_prototype,
                extensible: true,
            });
            subscribe_then(vm, p, on_fulfilled, on_rejected);
        }
        Err(err) => {
            stream_state_mut(vm, stream_id).pull_in_flight = false;
            let reason = vm.vm_error_to_thrown(&err);
            error_stream(vm, stream_id, reason);
        }
    }
}

// ---------------------------------------------------------------------------
// Controller mutators (internal — share state across stream + reader)
// ---------------------------------------------------------------------------

/// `controller.enqueue(chunk)` — spec §4.5.7.
fn controller_enqueue(
    vm: &mut VmInner,
    stream_id: ObjectId,
    chunk: JsValue,
) -> Result<(), VmError> {
    let state = stream_state(vm, stream_id);
    if state.state != ReadableStreamStateKind::Readable {
        return Err(VmError::type_error(
            "Failed to execute 'enqueue' on 'ReadableStreamDefaultController': stream is not readable",
        ));
    }
    if state.close_requested {
        return Err(VmError::type_error(
            "Failed to execute 'enqueue' on 'ReadableStreamDefaultController': stream is closing",
        ));
    }

    // Compute chunk size via size algorithm, defaulting to 1.
    let size_alg = state.size_algorithm;
    let chunk_size = if let Some(alg) = size_alg {
        let JsValue::Object(fn_id) = alg else {
            return Err(VmError::type_error("size algorithm is not callable"));
        };
        let res = {
            let mut ctx = NativeContext { vm };
            ctx.call_function(fn_id, JsValue::Undefined, &[chunk])
        };
        match res {
            // Spec §4.5.4 step 4: size must be a non-NaN, finite,
            // non-negative number.  Negatives would invert
            // `desiredSize` arithmetic and let `queue_total_size`
            // grow above `highWaterMark` — pull would never fire.
            Ok(JsValue::Number(n)) if n.is_finite() && n >= 0.0 => n,
            Ok(_) => {
                let err =
                    VmError::range_error("size algorithm returned a non-finite or negative number");
                let reason = vm.vm_error_to_thrown(&err);
                error_stream(vm, stream_id, reason);
                return Err(err);
            }
            Err(err) => {
                let reason = vm.vm_error_to_thrown(&err);
                error_stream(vm, stream_id, reason);
                return Err(err);
            }
        }
    } else {
        1.0
    };

    {
        let state = stream_state_mut(vm, stream_id);
        state.queue.push_back((chunk, chunk_size));
        state.queue_total_size += chunk_size;
    }

    // Wake any waiting reader — Stage 1b will plumb this into
    // `pending_read_promises`.  For Stage 1a the queue just grows.
    deliver_pending_reads(vm, stream_id);

    pull_if_needed(vm, stream_id);
    Ok(())
}

/// `controller.close()` — spec §4.5.6.  Drains any pending reads
/// and transitions the stream to Closed once the queue empties.
fn controller_close(vm: &mut VmInner, stream_id: ObjectId) -> Result<(), VmError> {
    let state = stream_state(vm, stream_id);
    if state.close_requested {
        return Err(VmError::type_error(
            "Failed to execute 'close' on 'ReadableStreamDefaultController': close already requested",
        ));
    }
    if state.state != ReadableStreamStateKind::Readable {
        return Err(VmError::type_error(
            "Failed to execute 'close' on 'ReadableStreamDefaultController': stream is not readable",
        ));
    }
    stream_state_mut(vm, stream_id).close_requested = true;

    // Drain remaining reads (Stage 1b) — if queue empty, immediate
    // close; else wait until reads drain the queue.
    if stream_state(vm, stream_id).queue.is_empty() {
        finalize_close(vm, stream_id);
    } else {
        // Pending reads will pop chunks; once queue empties the
        // close is finalised by deliver_pending_reads.
    }
    Ok(())
}

/// `controller.error(e)` — spec §4.5.8.
fn controller_error(vm: &mut VmInner, stream_id: ObjectId, reason: JsValue) {
    let state = stream_state(vm, stream_id);
    if state.state != ReadableStreamStateKind::Readable {
        return;
    }
    error_stream(vm, stream_id, reason);
}

/// Internal: transition stream to Errored, drop any queued chunks,
/// reject pending reads (Stage 1b) + reader.closed.
pub(super) fn error_stream(vm: &mut VmInner, stream_id: ObjectId, reason: JsValue) {
    let state = stream_state_mut(vm, stream_id);
    if state.state != ReadableStreamStateKind::Readable {
        return;
    }
    state.state = ReadableStreamStateKind::Errored;
    state.stored_error = reason;
    state.queue.clear();
    state.queue_total_size = 0.0;
    // Stage 1b: reject pending reads, reject reader.closed.
    reject_pending_reads(vm, stream_id, reason);
}

/// Internal: transition stream to Closed (queue is empty).
fn finalize_close(vm: &mut VmInner, stream_id: ObjectId) {
    let state = stream_state_mut(vm, stream_id);
    if state.state != ReadableStreamStateKind::Readable {
        return;
    }
    state.state = ReadableStreamStateKind::Closed;
    // Stage 1b: deliver done=true to any pending read; resolve
    // reader.closed.
    deliver_close_to_reader(vm, stream_id);
}

// ---------------------------------------------------------------------------
// Reader-side delivery (Stage 1b)
// ---------------------------------------------------------------------------

/// Pop chunks from the queue and resolve pending reader reads.
/// Spec §4.5.4 ReadableStreamDefaultControllerEnqueue's reader
/// dispatch path + §4.5.6 close finalisation.
///
/// Each dequeued chunk decrements `queue_total_size` by the size
/// recorded at enqueue time (default 1.0; `ByteLengthQueuingStrategy`
/// records the chunk's `byteLength`), keeping `desiredSize`
/// arithmetic exact under arbitrary user-supplied size
/// algorithms.  When the queue empties while
/// `close_requested == true`, the stream finalises Closed here so
/// `reader.closed` resolves and any subsequent `read()` returns
/// `{value: undefined, done: true}` (spec §4.5.6 step 4 / R1
/// finding: a `close()` before the queue drained would otherwise
/// leave the reader's `closed` Promise pending forever).
fn deliver_pending_reads(vm: &mut VmInner, stream_id: ObjectId) {
    loop {
        let reader_id = match stream_state(vm, stream_id).reader_id {
            Some(id) => id,
            None => break,
        };
        let read_promise_id = {
            let Some(reader_state) = vm.readable_stream_reader_states.get_mut(&reader_id) else {
                break;
            };
            match reader_state.pending_read_promises.pop_front() {
                Some(p) => p,
                None => break,
            }
        };
        let chunk_pair = {
            let state = stream_state_mut(vm, stream_id);
            match state.queue.pop_front() {
                Some((chunk, size)) => {
                    state.queue_total_size = (state.queue_total_size - size).max(0.0);
                    Some(chunk)
                }
                None => None,
            }
        };
        match chunk_pair {
            Some(chunk) => {
                let result = vm.create_iter_result(chunk, false);
                let _ = settle_promise(vm, read_promise_id, false, JsValue::Object(result));
            }
            None => {
                if let Some(reader_state) = vm.readable_stream_reader_states.get_mut(&reader_id) {
                    reader_state
                        .pending_read_promises
                        .push_front(read_promise_id);
                }
                break;
            }
        }
    }

    let needs_finalize = {
        let s = stream_state(vm, stream_id);
        s.close_requested && s.state == ReadableStreamStateKind::Readable && s.queue.is_empty()
    };
    if needs_finalize {
        finalize_close(vm, stream_id);
    }
}

/// Reject every pending read on the stream's locked reader (and
/// the reader's `closed` Promise) with the same reason.  Spec
/// §4.5.5 ReadableStreamDefaultControllerError.
fn reject_pending_reads(vm: &mut VmInner, stream_id: ObjectId, reason: JsValue) {
    let reader_id = match stream_state(vm, stream_id).reader_id {
        Some(id) => id,
        None => return,
    };
    let pending: Vec<ObjectId> =
        if let Some(reader_state) = vm.readable_stream_reader_states.get_mut(&reader_id) {
            std::mem::take(&mut reader_state.pending_read_promises)
                .into_iter()
                .collect()
        } else {
            return;
        };
    for p in pending {
        let _ = settle_promise(vm, p, true, reason);
    }
    let closed_promise = vm
        .readable_stream_reader_states
        .get(&reader_id)
        .map(|s| s.closed_promise);
    if let Some(p) = closed_promise {
        let _ = settle_promise(vm, p, true, reason);
    }
}

/// Resolve every pending read with `{value: undefined, done: true}`
/// and resolve the reader's `closed` Promise.  Called when the
/// stream finalises Closed.
fn deliver_close_to_reader(vm: &mut VmInner, stream_id: ObjectId) {
    let reader_id = match stream_state(vm, stream_id).reader_id {
        Some(id) => id,
        None => return,
    };
    let pending: Vec<ObjectId> =
        if let Some(reader_state) = vm.readable_stream_reader_states.get_mut(&reader_id) {
            std::mem::take(&mut reader_state.pending_read_promises)
                .into_iter()
                .collect()
        } else {
            return;
        };
    for p in pending {
        let result = vm.create_iter_result(JsValue::Undefined, true);
        let _ = settle_promise(vm, p, false, JsValue::Object(result));
    }
    let closed_promise = vm
        .readable_stream_reader_states
        .get(&reader_id)
        .map(|s| s.closed_promise);
    if let Some(p) = closed_promise {
        let _ = settle_promise(vm, p, false, JsValue::Undefined);
    }
}

// ---------------------------------------------------------------------------
// ReadableStreamDefaultReader
// ---------------------------------------------------------------------------

fn require_reader_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "ReadableStreamDefaultReader.prototype.{method} called on non-reader"
        )));
    };
    if matches!(
        ctx.vm.get_object(id).kind,
        ObjectKind::ReadableStreamDefaultReader
    ) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "ReadableStreamDefaultReader.prototype.{method} called on non-reader"
        )))
    }
}

/// Allocate + initialise a default reader against `stream_id`.
/// Used by the `getReader()` path on the stream prototype.
/// Errors if the stream is already locked.
fn acquire_default_reader(vm: &mut VmInner, stream_id: ObjectId) -> Result<ObjectId, VmError> {
    if stream_state(vm, stream_id).reader_id.is_some() {
        return Err(VmError::type_error(
            "Failed to execute 'getReader' on 'ReadableStream': stream is already locked to a reader",
        ));
    }
    let proto = vm.readable_stream_default_reader_prototype;
    let reader_id = vm.alloc_object(Object {
        kind: ObjectKind::ReadableStreamDefaultReader,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    initialise_default_reader(vm, reader_id, stream_id);
    Ok(reader_id)
}

/// Wire `reader_id` ↔ `stream_id`: allocate the reader's `closed`
/// Promise, settle it immediately if the stream is already Closed
/// or Errored (spec §4.3.3), insert the reader state and lock
/// the stream.  Shared between the `getReader()` path
/// (`acquire_default_reader`) and the
/// `new ReadableStreamDefaultReader(stream)` constructor — the
/// latter promotes a pre-allocated `this` instead of allocating,
/// so the wiring step alone needs to be reusable.
fn initialise_default_reader(vm: &mut VmInner, reader_id: ObjectId, stream_id: ObjectId) {
    let closed_promise = create_promise(vm);
    let state_kind = stream_state(vm, stream_id).state;
    match state_kind {
        ReadableStreamStateKind::Closed => {
            let _ = settle_promise(vm, closed_promise, false, JsValue::Undefined);
        }
        ReadableStreamStateKind::Errored => {
            let stored = stream_state(vm, stream_id).stored_error;
            let _ = settle_promise(vm, closed_promise, true, stored);
        }
        ReadableStreamStateKind::Readable => {}
    }
    vm.readable_stream_reader_states.insert(
        reader_id,
        ReaderState {
            stream_id: Some(stream_id),
            pending_read_promises: VecDeque::new(),
            closed_promise,
        },
    );
    stream_state_mut(vm, stream_id).reader_id = Some(reader_id);
}

pub(super) fn native_readable_stream_get_reader(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let stream_id = require_stream_this(ctx, this, "getReader")?;
    // WebIDL: the `options` argument is a dictionary, so non-`null` /
    // non-`undefined` non-objects must throw TypeError (Copilot R5
    // finding).
    let opts_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    match opts_arg {
        JsValue::Undefined | JsValue::Null => {}
        JsValue::Object(_) => {}
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'getReader' on 'ReadableStream': options must be an object",
            ));
        }
    }
    // Spec §4.2.4 step 2: `options.mode` is undefined ⇒ default
    // reader; `"byob"` ⇒ BYOB reader (Phase 2 unsupported, throws);
    // any other value ⇒ TypeError per WebIDL `ReadableStreamReaderMode`
    // enumeration.  Comparing against the literal `"byob"` avoids
    // R1's bug of accepting `mode: ""` and rejecting `mode: "default"`.
    if let JsValue::Object(opts_id) = opts_arg {
        let mode_sid = ctx.vm.strings.intern("mode");
        let mode_key = PropertyKey::String(mode_sid);
        if let Some(prop) = super::super::coerce::get_property(ctx.vm, opts_id, mode_key) {
            let v = ctx.vm.resolve_property(prop, JsValue::Object(opts_id))?;
            match v {
                JsValue::Undefined => {}
                JsValue::String(sid) => {
                    let s = ctx.vm.strings.get_utf8(sid);
                    if s == "byob" {
                        return Err(VmError::type_error(
                            "Failed to execute 'getReader' on 'ReadableStream': BYOB readers are not yet supported",
                        ));
                    }
                    // Any other string is not a valid enumeration
                    // member.  WebIDL throws TypeError.
                    return Err(VmError::type_error(
                        "Failed to execute 'getReader' on 'ReadableStream': options.mode must be 'byob' or undefined",
                    ));
                }
                _ => {
                    return Err(VmError::type_error(
                        "Failed to execute 'getReader' on 'ReadableStream': options.mode must be 'byob' or undefined",
                    ));
                }
            }
        }
    }
    let reader_id = acquire_default_reader(ctx.vm, stream_id)?;
    Ok(JsValue::Object(reader_id))
}

/// `new ReadableStreamDefaultReader(stream)` — spec §4.3.3.
///
/// Promotes the pre-allocated `this` (built by `do_new` with the
/// caller's `new.target.prototype`) to
/// [`ObjectKind::ReadableStreamDefaultReader`] so subclassing /
/// `new.target` semantics survive — matches the
/// `Blob` / `Headers` / `Request` / `Response` ctor pattern.  An
/// earlier draft routed through `acquire_default_reader`, which
/// always allocated a fresh `Object` and discarded the
/// pre-allocated receiver, breaking subclassing and leaking the
/// unused `this` (Copilot R1 finding).
pub(super) fn native_readable_stream_default_reader_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'ReadableStreamDefaultReader': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("ctor `this` always Object after `do_new`");
    };
    let stream_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(stream_id) = stream_arg else {
        return Err(VmError::type_error(
            "Failed to construct 'ReadableStreamDefaultReader': argument is not a ReadableStream",
        ));
    };
    if !matches!(
        ctx.vm.get_object(stream_id).kind,
        ObjectKind::ReadableStream
    ) {
        return Err(VmError::type_error(
            "Failed to construct 'ReadableStreamDefaultReader': argument is not a ReadableStream",
        ));
    }
    if stream_state(ctx.vm, stream_id).reader_id.is_some() {
        return Err(VmError::type_error(
            "Failed to construct 'ReadableStreamDefaultReader': stream is already locked to a reader",
        ));
    }
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::ReadableStreamDefaultReader;
    initialise_default_reader(ctx.vm, inst_id, stream_id);
    Ok(JsValue::Object(inst_id))
}

pub(super) fn native_reader_read(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_reader_this(ctx, this, "read")?;
    let p = create_promise(ctx.vm);

    let stream_id_opt = ctx
        .vm
        .readable_stream_reader_states
        .get(&id)
        .and_then(|s| s.stream_id);
    let Some(stream_id) = stream_id_opt else {
        // Reader is released — spec §4.3.4: reject with TypeError.
        let err = VmError::type_error(
            "Failed to execute 'read' on 'ReadableStreamDefaultReader': reader is released",
        );
        let reason = ctx.vm.vm_error_to_thrown(&err);
        let _ = settle_promise(ctx.vm, p, true, reason);
        return Ok(JsValue::Object(p));
    };

    match stream_state(ctx.vm, stream_id).state {
        ReadableStreamStateKind::Errored => {
            let stored = stream_state(ctx.vm, stream_id).stored_error;
            let _ = settle_promise(ctx.vm, p, true, stored);
            return Ok(JsValue::Object(p));
        }
        ReadableStreamStateKind::Closed if stream_state(ctx.vm, stream_id).queue.is_empty() => {
            let result = ctx.vm.create_iter_result(JsValue::Undefined, true);
            let _ = settle_promise(ctx.vm, p, false, JsValue::Object(result));
            return Ok(JsValue::Object(p));
        }
        _ => {}
    }

    // If a chunk is already queued, deliver synchronously (after
    // queueing the promise on the reader so deliver_pending_reads
    // pops it).  Otherwise the promise stays pending until a
    // future `controller.enqueue` / `controller.close` /
    // `controller.error`.
    if let Some(reader_state) = ctx.vm.readable_stream_reader_states.get_mut(&id) {
        reader_state.pending_read_promises.push_back(p);
    }
    deliver_pending_reads(ctx.vm, stream_id);
    // Even if nothing was delivered, a queued read may now satisfy
    // backpressure for `pull` — recompute pump.
    pull_if_needed(ctx.vm, stream_id);
    Ok(JsValue::Object(p))
}

pub(super) fn native_reader_release_lock(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_reader_this(ctx, this, "releaseLock")?;
    let stream_id_opt = ctx
        .vm
        .readable_stream_reader_states
        .get(&id)
        .and_then(|s| s.stream_id);
    let Some(stream_id) = stream_id_opt else {
        return Ok(JsValue::Undefined);
    };
    // Spec §4.3.5: reject any pending reads with TypeError, then
    // unlock.
    let pending: Vec<ObjectId> =
        if let Some(reader_state) = ctx.vm.readable_stream_reader_states.get_mut(&id) {
            std::mem::take(&mut reader_state.pending_read_promises)
                .into_iter()
                .collect()
        } else {
            Vec::new()
        };
    for read_p in pending {
        let err = VmError::type_error(
            "Pending reader read was released before the stream produced a value",
        );
        let reason = ctx.vm.vm_error_to_thrown(&err);
        let _ = settle_promise(ctx.vm, read_p, true, reason);
    }
    // Spec §4.3.5 step 6: replace `closed` with a freshly-rejected
    // Promise so subsequent `reader.closed.then(...)` sees a
    // TypeError immediately.
    let new_closed = create_promise(ctx.vm);
    let err = VmError::type_error("ReadableStream reader was released");
    let reason = ctx.vm.vm_error_to_thrown(&err);
    let _ = settle_promise(ctx.vm, new_closed, true, reason);
    if let Some(reader_state) = ctx.vm.readable_stream_reader_states.get_mut(&id) {
        reader_state.stream_id = None;
        reader_state.closed_promise = new_closed;
    }
    if let Some(stream_state) = ctx.vm.readable_stream_states.get_mut(&stream_id) {
        if stream_state.reader_id == Some(id) {
            stream_state.reader_id = None;
        }
    }
    Ok(JsValue::Undefined)
}

pub(super) fn native_reader_get_closed(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_reader_this(ctx, this, "closed")?;
    let p = ctx
        .vm
        .readable_stream_reader_states
        .get(&id)
        .map(|s| s.closed_promise);
    match p {
        Some(p) => Ok(JsValue::Object(p)),
        None => Err(VmError::type_error("Invalid reader state")),
    }
}

pub(super) fn native_reader_cancel(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_reader_this(ctx, this, "cancel")?;
    let reason = args.first().copied().unwrap_or(JsValue::Undefined);
    let stream_id_opt = ctx
        .vm
        .readable_stream_reader_states
        .get(&id)
        .and_then(|s| s.stream_id);
    let Some(stream_id) = stream_id_opt else {
        let p = create_promise(ctx.vm);
        let err = VmError::type_error(
            "Failed to execute 'cancel' on 'ReadableStreamDefaultReader': reader is released",
        );
        let r = ctx.vm.vm_error_to_thrown(&err);
        let _ = settle_promise(ctx.vm, p, true, r);
        return Ok(JsValue::Object(p));
    };
    Ok(JsValue::Object(do_stream_cancel(ctx.vm, stream_id, reason)))
}

// ---------------------------------------------------------------------------
// Body integration helpers (§4.2 + WHATWG Fetch §5 .body)
// ---------------------------------------------------------------------------

/// Allocate a fresh `ReadableStream` whose queue carries one
/// `Uint8Array` chunk built from `bytes` and whose state is
/// already `close_requested`.  Used by `Request.body` /
/// `Response.body` / `Blob.prototype.stream()` to expose body
/// bytes as a stream without an embedded JS source callback.
///
/// Phase-2 simplification: emits one chunk regardless of size.
/// Chunked streaming (e.g. broker push of partial response
/// payloads) lands with Phase 5 PR-streams-network.
pub(crate) fn create_body_backed_stream(vm: &mut VmInner, bytes: Vec<u8>) -> ObjectId {
    let stream_proto = vm.readable_stream_prototype;
    let stream_id = vm.alloc_object(Object {
        kind: ObjectKind::ReadableStream,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: stream_proto,
        extensible: true,
    });

    let controller_proto = vm.readable_stream_default_controller_prototype;
    let controller_id = vm.alloc_object(Object {
        kind: ObjectKind::ReadableStreamDefaultController { stream_id },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: controller_proto,
        extensible: true,
    });

    // Materialise a single `Uint8Array` chunk from `bytes`.  The
    // ArrayBuffer goes through `body_data` like any other buffer
    // — the stream's own ObjectKind doesn't carry the bytes.
    //
    // TypedArray's `byte_length` is stored as `u32`; bodies > 4GiB
    // would silently truncate, exposing a Uint8Array view that
    // doesn't cover the full payload.  Phase 2 doesn't yet split
    // oversized bodies into multiple chunks (Phase 5
    // PR-streams-network covers chunked emit), so for the rare
    // >4GiB case we ship an immediately-errored stream rather
    // than a half-truncated chunk.
    let buf_id = super::array_buffer::create_array_buffer_from_bytes(vm, bytes);
    let bytes_len = vm.body_data.get(&buf_id).map_or(0, std::vec::Vec::len);
    let oversize = bytes_len > u32::MAX as usize;
    #[allow(clippy::cast_possible_truncation)]
    let byte_length = if oversize { 0 } else { bytes_len as u32 };
    let byte_offset: u32 = 0;
    let element_kind = super::super::value::ElementKind::Uint8;
    let typed_proto = vm.subclass_array_prototypes[element_kind.index()];
    let typed_id = vm.alloc_object(Object {
        kind: ObjectKind::TypedArray {
            buffer_id: buf_id,
            byte_offset,
            byte_length,
            element_kind,
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: typed_proto,
        extensible: true,
    });

    let mut queue = VecDeque::new();
    if byte_length > 0 {
        queue.push_back((JsValue::Object(typed_id), 1.0));
    }
    let queue_total_size = if byte_length > 0 { 1.0 } else { 0.0 };

    vm.readable_stream_states.insert(
        stream_id,
        ReadableStreamState {
            state: ReadableStreamStateKind::Readable,
            controller_id,
            reader_id: None,
            queue,
            queue_total_size,
            high_water_mark: 1.0,
            size_algorithm: None,
            start_called: true,
            pull_in_flight: false,
            pull_again: false,
            close_requested: true,
            source_start: None,
            source_pull: None,
            source_cancel: None,
            underlying_source: None,
            stored_error: JsValue::Undefined,
        },
    );

    // Stream is `Readable` with a queued chunk + close_requested
    // — the spec's "wait for the queue to drain before closing"
    // path matches exactly.  Oversize must check FIRST: a
    // `finalize_close` flips state to Closed, after which
    // `error_stream` early-returns and the oversize stream
    // would silently report `done: true` instead of rejecting
    // (Copilot R5 finding).  So error before any close.
    if oversize {
        let err = VmError::range_error(
            "Failed to materialise body stream: payload exceeds 4 GiB Uint8Array view limit",
        );
        let reason = vm.vm_error_to_thrown(&err);
        error_stream(vm, stream_id, reason);
    } else if byte_length == 0 {
        finalize_close(vm, stream_id);
    }
    stream_id
}

// ---------------------------------------------------------------------------
// Queuing strategies (§6.1 / §6.2)
// ---------------------------------------------------------------------------

/// Shared body for `new CountQueuingStrategy({highWaterMark})` and
/// `new ByteLengthQueuingStrategy({highWaterMark})`.  Spec §6.1.2 /
/// §6.2.2 — both ctors read the `highWaterMark` from the
/// init-object verbatim (no coercion / validation; the stream
/// constructor's normaliser handles that).
fn extract_strategy_high_water_mark(
    ctx: &mut NativeContext<'_>,
    init_arg: JsValue,
    iface: &str,
) -> Result<JsValue, VmError> {
    let JsValue::Object(obj_id) = init_arg else {
        return Err(VmError::type_error(format!(
            "Failed to construct '{iface}': init must be an object"
        )));
    };
    let key = PropertyKey::String(ctx.vm.well_known.high_water_mark);
    match super::super::coerce::get_property(ctx.vm, obj_id, key) {
        Some(prop) => ctx.vm.resolve_property(prop, JsValue::Object(obj_id)),
        None => Err(VmError::type_error(format!(
            "Failed to construct '{iface}': init.highWaterMark is required"
        ))),
    }
}

fn install_high_water_mark_own(vm: &mut VmInner, inst_id: ObjectId, hwm: JsValue) {
    let key = PropertyKey::String(vm.well_known.high_water_mark);
    vm.define_shaped_property(inst_id, key, PropertyValue::Data(hwm), PropertyAttrs::DATA);
}

fn native_count_queuing_strategy_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'CountQueuingStrategy': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("ctor `this` always Object after `do_new`");
    };
    let init = args.first().copied().unwrap_or(JsValue::Undefined);
    let hwm = extract_strategy_high_water_mark(ctx, init, "CountQueuingStrategy")?;
    install_high_water_mark_own(ctx.vm, inst_id, hwm);
    Ok(JsValue::Object(inst_id))
}

fn native_byte_length_queuing_strategy_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'ByteLengthQueuingStrategy': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("ctor `this` always Object after `do_new`");
    };
    let init = args.first().copied().unwrap_or(JsValue::Undefined);
    let hwm = extract_strategy_high_water_mark(ctx, init, "ByteLengthQueuingStrategy")?;
    install_high_water_mark_own(ctx.vm, inst_id, hwm);
    Ok(JsValue::Object(inst_id))
}

/// `CountQueuingStrategy.prototype.size(_chunk)` — always returns 1.
fn native_count_queuing_strategy_size(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(JsValue::Number(1.0))
}

/// `ByteLengthQueuingStrategy.prototype.size(chunk)` — returns
/// `chunk.byteLength`.  Spec §6.2.4: returns the chunk's
/// `byteLength` IDL property if it has one; otherwise the
/// algorithm propagates whatever value (or undefined) the lookup
/// yields.
fn native_byte_length_queuing_strategy_size(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let chunk = args.first().copied().unwrap_or(JsValue::Undefined);
    let JsValue::Object(chunk_id) = chunk else {
        return Ok(JsValue::Undefined);
    };
    let key = PropertyKey::String(ctx.vm.well_known.byte_length);
    match super::super::coerce::get_property(ctx.vm, chunk_id, key) {
        Some(prop) => ctx.vm.resolve_property(prop, chunk),
        None => Ok(JsValue::Undefined),
    }
}

// ---------------------------------------------------------------------------
// Reader registration
// ---------------------------------------------------------------------------

impl VmInner {
    pub(in crate::vm) fn register_readable_stream_reader_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_readable_stream_reader_global called before register_prototypes");
        let reader_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_readable_stream_reader_members(reader_proto);
        self.readable_stream_default_reader_prototype = Some(reader_proto);

        let reader_ctor = self.create_constructable_function(
            "ReadableStreamDefaultReader",
            native_readable_stream_default_reader_constructor,
        );
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            reader_ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(reader_proto)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            reader_proto,
            ctor_key,
            PropertyValue::Data(JsValue::Object(reader_ctor)),
            PropertyAttrs::METHOD,
        );
        self.globals.insert(
            self.well_known.readable_stream_default_reader_global,
            JsValue::Object(reader_ctor),
        );

        // Install `getReader` on the stream prototype now that the
        // reader prototype exists (avoids forward-reference at the
        // earlier stream-registration site).
        let stream_proto = self
            .readable_stream_prototype
            .expect("readable_stream_prototype must be set");
        self.install_native_method(
            stream_proto,
            self.well_known.get_reader,
            native_readable_stream_get_reader as NativeFn,
            PropertyAttrs::METHOD,
        );
    }

    /// Register `CountQueuingStrategy` + `ByteLengthQueuingStrategy`
    /// (WHATWG Streams §6.1 / §6.2).  Each is a regular constructor
    /// that produces an Ordinary instance with a `highWaterMark`
    /// own property and a `size` method on the prototype.  The
    /// stream constructor then reads them via the same path it
    /// uses for ad-hoc `{highWaterMark, size}` objects.
    pub(in crate::vm) fn register_queuing_strategy_globals(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_queuing_strategy_globals called before register_prototypes");

        // CountQueuingStrategy
        let count_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_native_method(
            count_proto,
            self.well_known.size,
            native_count_queuing_strategy_size as NativeFn,
            PropertyAttrs::METHOD,
        );
        self.count_queuing_strategy_prototype = Some(count_proto);
        let count_ctor = self.create_constructable_function(
            "CountQueuingStrategy",
            native_count_queuing_strategy_constructor,
        );
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            count_ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(count_proto)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            count_proto,
            ctor_key,
            PropertyValue::Data(JsValue::Object(count_ctor)),
            PropertyAttrs::METHOD,
        );
        self.globals.insert(
            self.well_known.count_queuing_strategy_global,
            JsValue::Object(count_ctor),
        );

        // ByteLengthQueuingStrategy
        let byte_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_native_method(
            byte_proto,
            self.well_known.size,
            native_byte_length_queuing_strategy_size as NativeFn,
            PropertyAttrs::METHOD,
        );
        self.byte_length_queuing_strategy_prototype = Some(byte_proto);
        let byte_ctor = self.create_constructable_function(
            "ByteLengthQueuingStrategy",
            native_byte_length_queuing_strategy_constructor,
        );
        self.define_shaped_property(
            byte_ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(byte_proto)),
            PropertyAttrs::BUILTIN,
        );
        self.define_shaped_property(
            byte_proto,
            ctor_key,
            PropertyValue::Data(JsValue::Object(byte_ctor)),
            PropertyAttrs::METHOD,
        );
        self.globals.insert(
            self.well_known.byte_length_queuing_strategy_global,
            JsValue::Object(byte_ctor),
        );
    }

    fn install_readable_stream_reader_members(&mut self, proto_id: ObjectId) {
        // `closed` getter.
        let closed_sid = self.well_known.closed_attr;
        self.install_accessor_pair(
            proto_id,
            closed_sid,
            native_reader_get_closed as NativeFn,
            None,
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        let methods: [(StringId, NativeFn); 3] = [
            (self.well_known.read, native_reader_read as NativeFn),
            (
                self.well_known.release_lock,
                native_reader_release_lock as NativeFn,
            ),
            (self.well_known.cancel, native_reader_cancel as NativeFn),
        ];
        for (name_sid, func) in methods {
            self.install_native_method(proto_id, name_sid, func, PropertyAttrs::METHOD);
        }
    }
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
pub(super) fn do_stream_cancel(vm: &mut VmInner, stream_id: ObjectId, reason: JsValue) -> ObjectId {
    let p = create_promise(vm);

    let state_kind = stream_state(vm, stream_id).state;
    match state_kind {
        ReadableStreamStateKind::Closed => {
            let _ = settle_promise(vm, p, false, JsValue::Undefined);
            return p;
        }
        ReadableStreamStateKind::Errored => {
            let stored = stream_state(vm, stream_id).stored_error;
            let _ = settle_promise(vm, p, true, stored);
            return p;
        }
        ReadableStreamStateKind::Readable => {}
    }

    // Drop queued chunks; prepare to close after source.cancel
    // settles.
    {
        let state = stream_state_mut(vm, stream_id);
        state.queue.clear();
        state.queue_total_size = 0.0;
    }
    // Spec §4.2.6 step 5: close stream synchronously *after* drop
    // (note: spec's exact ordering is "close after queue clear,
    // before source.cancel returns").  We match: close finalises
    // here; source.cancel settles the returned promise.
    finalize_close(vm, stream_id);

    let cancel_cb = stream_state(vm, stream_id).source_cancel;
    let this_arg = stream_state(vm, stream_id)
        .underlying_source
        .unwrap_or(JsValue::Undefined);
    if let Some(JsValue::Object(fn_id)) = cancel_cb {
        // Spec InvokeOrNoop: `this` is the underlyingSource so
        // `cancel() { this... }` works.
        let result = {
            let mut ctx = NativeContext { vm };
            ctx.call_function(fn_id, this_arg, &[reason])
        };
        match result {
            Ok(value) => {
                let inner = create_promise(vm);
                let _ = settle_promise(vm, inner, false, value);
                let on_fulfilled = vm.alloc_object(Object {
                    kind: ObjectKind::ReadableStreamCancelStep {
                        promise: p,
                        is_reject: false,
                    },
                    storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                    prototype: vm.function_prototype,
                    extensible: true,
                });
                let on_rejected = vm.alloc_object(Object {
                    kind: ObjectKind::ReadableStreamCancelStep {
                        promise: p,
                        is_reject: true,
                    },
                    storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
                    prototype: vm.function_prototype,
                    extensible: true,
                });
                subscribe_then(vm, inner, on_fulfilled, on_rejected);
            }
            Err(err) => {
                let reason = vm.vm_error_to_thrown(&err);
                let _ = settle_promise(vm, p, true, reason);
            }
        }
    } else {
        let _ = settle_promise(vm, p, false, JsValue::Undefined);
    }
    p
}

// ---------------------------------------------------------------------------
// Controller methods (enqueue / close / error / desiredSize)
// ---------------------------------------------------------------------------

pub(super) fn native_readable_stream_default_controller_get_desired_size(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_controller_this(ctx, this, "desiredSize")?;
    let stream_id = controller_stream_id(ctx.vm, id);
    let state = stream_state(ctx.vm, stream_id);
    let v = match state.state {
        ReadableStreamStateKind::Errored => return Ok(JsValue::Null),
        ReadableStreamStateKind::Closed => 0.0,
        ReadableStreamStateKind::Readable => desired_size(state),
    };
    Ok(JsValue::Number(v))
}

pub(super) fn native_readable_stream_default_controller_enqueue(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_controller_this(ctx, this, "enqueue")?;
    let stream_id = controller_stream_id(ctx.vm, id);
    let chunk = args.first().copied().unwrap_or(JsValue::Undefined);
    controller_enqueue(ctx.vm, stream_id, chunk)?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_readable_stream_default_controller_close(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_controller_this(ctx, this, "close")?;
    let stream_id = controller_stream_id(ctx.vm, id);
    controller_close(ctx.vm, stream_id)?;
    Ok(JsValue::Undefined)
}

pub(super) fn native_readable_stream_default_controller_error(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_controller_this(ctx, this, "error")?;
    let stream_id = controller_stream_id(ctx.vm, id);
    let reason = args.first().copied().unwrap_or(JsValue::Undefined);
    controller_error(ctx.vm, stream_id, reason);
    Ok(JsValue::Undefined)
}

// `new ReadableStreamDefaultController()` is illegal per spec
// §4.5.3 — only the stream constructor allocates one.
fn native_default_controller_illegal_constructor(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Err(VmError::type_error(
        "Failed to construct 'ReadableStreamDefaultController': Illegal constructor",
    ))
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Register the `ReadableStream` + `ReadableStreamDefaultController`
    /// globals + prototypes.  Stage 1b will append the
    /// `ReadableStreamDefaultReader` registration in a follow-up
    /// `register_readable_stream_reader_global` so the two halves
    /// can land in distinct commits.
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
            native_default_controller_illegal_constructor,
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

    fn install_readable_stream_default_controller_members(&mut self, proto_id: ObjectId) {
        // `desiredSize` getter.
        let ds_sid = self.well_known.desired_size;
        self.install_accessor_pair(
            proto_id,
            ds_sid,
            native_readable_stream_default_controller_get_desired_size as NativeFn,
            None,
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        let methods: [(StringId, NativeFn); 3] = [
            (
                self.well_known.enqueue,
                native_readable_stream_default_controller_enqueue as NativeFn,
            ),
            (
                self.well_known.close,
                native_readable_stream_default_controller_close as NativeFn,
            ),
            (
                self.well_known.error,
                native_readable_stream_default_controller_error as NativeFn,
            ),
        ];
        for (name_sid, func) in methods {
            self.install_native_method(proto_id, name_sid, func, PropertyAttrs::METHOD);
        }
    }
}
