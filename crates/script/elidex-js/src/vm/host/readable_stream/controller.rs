//! `ReadableStreamDefaultController` — pull pump, queue mutators,
//! reader-delivery loops, and the JS-facing controller methods
//! (`enqueue` / `close` / `error` / `desiredSize`).
//!
//! Lives alongside [`super::reader`] because the controller and
//! the reader cooperate through the shared
//! [`super::ReadableStreamState`] queue: `controller_enqueue` wakes
//! pending reads via [`deliver_pending_reads`], and
//! `controller_close` drains then finalises (with the reader's
//! `closed` Promise resolved here too).

use super::super::super::natives_promise::settle_promise;
use super::super::super::shape::PropertyAttrs;
use super::super::super::value::{
    JsValue, NativeContext, ObjectId, ObjectKind, PropertyKey, VmError,
};
use super::super::super::{NativeFn, VmInner};
use super::{stream_state, stream_state_mut, ReadableStreamState, ReadableStreamStateKind};

// ---------------------------------------------------------------------------
// Brand check + state lookup helpers (controller-side only)
// ---------------------------------------------------------------------------

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
// Pull pump
// ---------------------------------------------------------------------------

/// Compute `[[strategyHWM]] - [[queueTotalSize]]` (the spec
/// `[[desiredSize]]` slot).  Returns `f64` to preserve the
/// `+Infinity - finite` edges.
fn desired_size(state: &ReadableStreamState) -> f64 {
    state.high_water_mark - state.queue_total_size
}

/// Should the pull pump fire?  Spec §4.5.13
/// (ReadableStreamDefaultControllerShouldCallPull):
/// - Stream must be `Readable`.
/// - `start_called` is set (pull only fires after start settles).
/// - **Either** there is a pending `read()` request on a locked
///   reader **or** `[[desiredSize]] > 0`.  The pending-read
///   branch is what makes `new ReadableStream({pull}, {highWaterMark: 0})`
///   work — without a chunk in the queue and no headroom for one,
///   the pull pump would otherwise never fire and `reader.read()`
///   would hang forever (Copilot R3).
fn pull_should_fire(vm: &VmInner, state: &ReadableStreamState) -> bool {
    if state.state != ReadableStreamStateKind::Readable
        || state.close_requested
        || !state.start_called
    {
        return false;
    }
    if desired_size(state) > 0.0 {
        return true;
    }
    // No headroom — fall through to spec §4.5.13 step 4: pull
    // when a locked reader has at least one pending read request.
    state
        .reader_id
        .and_then(|reader_id| vm.readable_stream_reader_states.get(&reader_id))
        .is_some_and(|r| !r.pending_read_promises.is_empty())
}

/// Drive the pull pump (§4.5.10 ReadableStreamDefaultControllerCallPullIfNeeded).
pub(super) fn pull_if_needed(vm: &mut VmInner, stream_id: ObjectId) {
    let (should_pull, pull_cb, controller_id, underlying_source) = {
        let state = stream_state(vm, stream_id);
        (
            pull_should_fire(vm, state),
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
            // callables (mirrors finalize_start, including its
            // GC-safety nested-temp-root chain — Copilot R7).
            let p = super::super::super::natives_promise::create_promise(vm);
            let _ = settle_promise(vm, p, false, value);
            let mut g_p = vm.push_temp_root(JsValue::Object(p));
            let on_fulfilled_proto = g_p.function_prototype;
            let on_fulfilled = g_p.alloc_object(super::super::super::value::Object {
                kind: ObjectKind::ReadableStreamPullStep {
                    stream_id,
                    is_reject: false,
                },
                storage: super::super::super::value::PropertyStorage::shaped(
                    super::super::super::shape::ROOT_SHAPE,
                ),
                prototype: on_fulfilled_proto,
                extensible: true,
            });
            let mut g_f = g_p.push_temp_root(JsValue::Object(on_fulfilled));
            let on_rejected_proto = g_f.function_prototype;
            let on_rejected = g_f.alloc_object(super::super::super::value::Object {
                kind: ObjectKind::ReadableStreamPullStep {
                    stream_id,
                    is_reject: true,
                },
                storage: super::super::super::value::PropertyStorage::shaped(
                    super::super::super::shape::ROOT_SHAPE,
                ),
                prototype: on_rejected_proto,
                extensible: true,
            });
            super::super::super::natives_promise::subscribe_then(
                &mut g_f,
                p,
                on_fulfilled,
                on_rejected,
            );
            drop(g_f);
            drop(g_p);
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

    // Wake any waiting reader by popping freshly-queued chunks
    // into pending `read()` promises (spec §4.5.4 step 5).
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
pub(super) fn finalize_close(vm: &mut VmInner, stream_id: ObjectId) {
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
pub(super) fn deliver_pending_reads(vm: &mut VmInner, stream_id: ObjectId) {
    while let Some(reader_id) = stream_state(vm, stream_id).reader_id {
        let read_promise_id = {
            let Some(reader_state) = vm.readable_stream_reader_states.get_mut(&reader_id) else {
                break;
            };
            let Some(p) = reader_state.pending_read_promises.pop_front() else {
                break;
            };
            p
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
        if let Some(chunk) = chunk_pair {
            let result = vm.create_iter_result(chunk, false);
            let _ = settle_promise(vm, read_promise_id, false, JsValue::Object(result));
        } else {
            if let Some(reader_state) = vm.readable_stream_reader_states.get_mut(&reader_id) {
                reader_state
                    .pending_read_promises
                    .push_front(read_promise_id);
            }
            break;
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
    let Some(reader_id) = stream_state(vm, stream_id).reader_id else {
        return;
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
    let Some(reader_id) = stream_state(vm, stream_id).reader_id else {
        return;
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
pub(super) fn native_default_controller_illegal_constructor(
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
    pub(super) fn install_readable_stream_default_controller_members(
        &mut self,
        proto_id: ObjectId,
    ) {
        // `desiredSize` getter.
        let ds_sid = self.well_known.desired_size;
        self.install_accessor_pair(
            proto_id,
            ds_sid,
            native_readable_stream_default_controller_get_desired_size as NativeFn,
            None,
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        let methods: [(super::super::super::value::StringId, NativeFn); 3] = [
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

        // Suppress unused PropertyKey re-import warning when the
        // module is split — `PropertyKey` is used transitively
        // through helper traits but no direct reference here.
        let _ = PropertyKey::String(self.well_known.error);
    }
}
