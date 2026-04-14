//! Generator iterator (ES2020 §25.4).
//!
//! The generator machinery itself lives inside the dispatcher — this module
//! just exposes it through the `Generator.prototype.next` native, builds
//! `{value, done}` iterator-result objects, and delegates the actual
//! resume/suspend work to [`VmInner::resume_generator`] (defined here).
//!
//! `.return()` / `.throw()` and full `yield*` support land with PR2.5
//! (generator spec completion).

use super::shape::{self, PropertyAttrs};
use super::value::{
    FrameCompletion, GeneratorStatus, JsValue, NativeContext, Object, ObjectId, ObjectKind,
    PropertyKey, PropertyStorage, PropertyValue, SuspendedFrame, UpvalueState, VmError,
};
use super::VmInner;

// ---------------------------------------------------------------------------
// Suspend — invoked from the dispatcher on Op::Yield / Op::Await
// ---------------------------------------------------------------------------

/// Capture the current frame into its enclosing generator as a
/// [`SuspendedFrame`] and record the yielded value on
/// [`VmInner::generator_yielded`].  The dispatcher calls this from the
/// `Op::Yield | Op::Await` arm; resumption is handled separately by
/// [`VmInner::resume_generator`].
///
/// Semantics captured here:
/// - Pop the current frame out of `vm.frames` and drain its stack slice.
/// - Restore the caller's `completion_value`.
/// - Close every open upvalue pointing at this frame's locals (save the
///   `(uv_id, slot)` pairs so `resume_generator` can reopen).
/// - Rebase frame-relative handler depths.
/// - Mark the generator `SuspendedYield` with the built [`SuspendedFrame`].
pub(crate) fn op_yield_suspend(
    vm: &mut VmInner,
    frame_idx: usize,
    value: JsValue,
) -> Result<(), VmError> {
    let gen_id = vm.frames[frame_idx]
        .generator
        .ok_or_else(|| VmError::internal("Yield/Await outside a coroutine frame"))?;

    let mut frame = vm.frames.pop().expect("frame for Yield");
    let stack_slice: Vec<JsValue> = vm.stack.drain(frame.base..).collect();
    vm.completion_value = frame.saved_completion;

    // Close open upvalues; remember their slots for the later reopen.
    let mut upvalue_slots = Vec::with_capacity(frame.local_upvalue_ids.len());
    for &uv_id in &frame.local_upvalue_ids {
        let uv = &mut vm.upvalues[uv_id.0 as usize];
        if let UpvalueState::Open { slot, .. } = uv.state {
            let captured = stack_slice
                .get(slot as usize)
                .copied()
                .unwrap_or(JsValue::Undefined);
            uv.state = UpvalueState::Closed(captured);
            upvalue_slots.push((uv_id, slot));
        }
    }

    // Make handler stack_depths frame-relative; `resume_generator` adds
    // the new base back on restore.
    for h in &mut frame.exception_handlers {
        h.stack_depth = h.stack_depth.saturating_sub(frame.base);
    }

    let suspended = SuspendedFrame {
        frame,
        stack_slice,
        upvalue_slots,
    };
    if let ObjectKind::Generator(state) = &mut vm.get_object_mut(gen_id).kind {
        state.suspended = Some(suspended);
        state.status = GeneratorStatus::SuspendedYield;
    }
    vm.generator_yielded = Some(value);
    Ok(())
}

// ---------------------------------------------------------------------------
// Resume — invoked by Generator.prototype.next
// ---------------------------------------------------------------------------

impl VmInner {
    /// Resume `gen_id` with the given completion injected at the yield
    /// point.  Returns the yielded (or returned) value plus a `done` flag;
    /// `Generator.prototype.next` then wraps the pair in a `{value, done}`
    /// iterator-result object.
    ///
    /// `completion`:
    /// - [`FrameCompletion::Normal(v)`] — resume `.next(v)`.  `v` is used
    ///   as the value of the yield expression (or discarded on the first
    ///   entry, which hasn't hit a yield yet).
    /// - [`FrameCompletion::Throw(e)`] — resume `.throw(e)`: inject an
    ///   exception at the yield point, letting any in-scope catch/finally
    ///   observe it.
    /// - [`FrameCompletion::Return(v)`] — resume `.return(v)` or implicit
    ///   iterator close (e.g. `for (… of gen) { break }`): walk handlers
    ///   to find a finally block, run it, then complete with `v`.  If no
    ///   finally is in scope, no body runs — the generator is marked
    ///   Completed directly.
    ///
    /// Failure modes (same across variants):
    /// - Generator already Running → TypeError.
    /// - Generator already Completed → `Normal` returns
    ///   `(undefined, true)`; `Return(v)` returns `(v, true)`; `Throw(e)`
    ///   surfaces the thrown value as a `VmError`.
    /// - Body throws uncaught → propagate and mark Completed.
    #[allow(clippy::too_many_lines)] // Normal / Return / Throw + setup share one frame-restore
    pub(crate) fn resume_generator(
        &mut self,
        gen_id: ObjectId,
        completion: FrameCompletion,
    ) -> Result<(JsValue, bool), VmError> {
        // ── Early exits + lift the suspended frame in one borrow ────────
        //
        // Hot path (Normal + SuspendedYield → take the frame, transition
        // to Running) takes a single `get_object_mut`; the cold error /
        // already-completed branches short-circuit out before the take.
        let (suspended, initial) = {
            let ObjectKind::Generator(state) = &mut self.get_object_mut(gen_id).kind else {
                return Err(VmError::type_error("resume on non-Generator"));
            };
            match (state.status, completion) {
                (GeneratorStatus::Completed, FrameCompletion::Normal(_)) => {
                    return Ok((JsValue::Undefined, true));
                }
                (GeneratorStatus::Completed, FrameCompletion::Return(v)) => {
                    return Ok((v, true));
                }
                (GeneratorStatus::Completed, FrameCompletion::Throw(e)) => {
                    // §25.4.1.4: throw on a completed iterator propagates
                    // the reason as the thrown completion.
                    return Err(VmError::throw(e));
                }
                (GeneratorStatus::Running, _) => {
                    return Err(VmError::type_error(
                        "Cannot resume a generator that is already running",
                    ));
                }
                (status, _) => {
                    let initial = matches!(status, GeneratorStatus::SuspendedStart);
                    state.status = GeneratorStatus::Running;
                    (state.suspended.take().expect("suspended frame"), initial)
                }
            }
        };

        // ── Rebase + restore the frame onto the VM stack ────────────────
        let super::value::SuspendedFrame {
            mut frame,
            stack_slice,
            upvalue_slots,
        } = suspended;
        let entry_frames = self.frames.len();
        let new_base = self.stack.len();

        // Write the restored slots back onto the stack.
        self.stack.extend(stack_slice);

        // Reopen closed upvalues pointing to this frame's locals — before
        // reopening, write the *current* closed value back into the stack
        // slot so writes done while the generator was suspended aren't lost.
        //
        // We do NOT re-push `uv_id` into `frame.local_upvalue_ids`: the list
        // was preserved verbatim in the suspended frame (`op_yield_suspend`
        // iterates it without mutating), so pushing here would accumulate
        // duplicates across every yield/resume cycle — each subsequent
        // suspend would then close the same upvalue multiple times.
        for (uv_id, slot) in &upvalue_slots {
            if let UpvalueState::Closed(val) = self.upvalues[uv_id.0 as usize].state {
                let idx = new_base + *slot as usize;
                if idx < self.stack.len() {
                    self.stack[idx] = val;
                }
            }
            self.upvalues[uv_id.0 as usize].state = UpvalueState::Open {
                frame_base: new_base,
                slot: *slot,
            };
        }

        // Rebase frame + handler absolute depths.
        frame.base = new_base;
        frame.cleanup_base = new_base;
        for h in &mut frame.exception_handlers {
            // stack_depth was made frame-relative in Op::Yield; restore.
            h.stack_depth += new_base;
        }
        // Preserve the caller's completion_value across the resumed body.
        let saved_completion = self.completion_value;
        self.completion_value = JsValue::Undefined;
        frame.saved_completion = saved_completion;

        let mark_completed = |vm: &mut Self| {
            if let ObjectKind::Generator(state) = &mut vm.get_object_mut(gen_id).kind {
                state.status = GeneratorStatus::Completed;
                state.suspended = None;
            }
        };

        match completion {
            FrameCompletion::Normal(arg) => {
                // On resume from yield, the arg becomes the value of the
                // `yield` expression.  On the initial `.next()` call the
                // generator hasn't hit a yield yet, so we don't push
                // anything (arg is discarded).
                if !initial {
                    self.stack.push(arg);
                }
                self.frames.push(frame);
            }
            FrameCompletion::Return(v) => {
                // `.return(v)` injection: if the suspended frame has no
                // finally in scope, abandon it entirely — no user code
                // runs, the generator is simply completed with `v`.  Spec
                // §25.4.1.3 step 3 plus §13.15: `.return(v)` bypasses
                // `catch` blocks, only finally observes it.
                let has_finally = frame
                    .exception_handlers
                    .iter()
                    .any(|h| h.finally_ip.is_some());
                if !has_finally {
                    // Pop closed upvalues we already reopened (they'd
                    // otherwise dangle), drop the frame and stack slice.
                    for (uv_id, _) in &upvalue_slots {
                        let val = match self.upvalues[uv_id.0 as usize].state {
                            UpvalueState::Open { frame_base, slot } => {
                                self.stack[frame_base + slot as usize]
                            }
                            UpvalueState::Closed(v) => v,
                        };
                        self.upvalues[uv_id.0 as usize].state = UpvalueState::Closed(val);
                    }
                    self.stack.truncate(new_base);
                    self.completion_value = saved_completion;
                    mark_completed(self);
                    return Ok((v, true));
                }
                self.frames.push(frame);
                // Route to innermost finally on the now-top frame.
                let target_ip = self
                    .route_to_next_finally(FrameCompletion::Return(v))
                    .expect("has_finally checked above");
                self.frames.last_mut().unwrap().ip = target_ip;
            }
            FrameCompletion::Throw(reason) => {
                self.frames.push(frame);
                let entry_frames_post = self.frames.len() - 1;
                if !self.handle_exception(reason, entry_frames_post) {
                    // No handler — frame was left in place
                    // (handle_exception stops at entry_frame_depth).
                    // Clean up the frame properly: close any upvalues
                    // we reopened on the restored frame, drop the
                    // restored stack slice, then pop.  Using raw
                    // `frames.pop()` without these would leak
                    // `UpvalueState::Open` pointers into a stack slot
                    // we're about to discard.
                    for (uv_id, _) in &upvalue_slots {
                        let val = match self.upvalues[uv_id.0 as usize].state {
                            UpvalueState::Open { frame_base, slot } => {
                                self.stack[frame_base + slot as usize]
                            }
                            UpvalueState::Closed(v) => v,
                        };
                        self.upvalues[uv_id.0 as usize].state = UpvalueState::Closed(val);
                    }
                    self.stack.truncate(new_base);
                    self.frames.pop();
                    self.completion_value = saved_completion;
                    mark_completed(self);
                    return Err(VmError::throw(reason));
                }
            }
        }

        // ── Run until Yield / Return / Throw ───────────────────────────
        let run_result = self.run();

        // An uncaught throw inside the body leaves the frame in place —
        // the dispatcher's Throw handler only unwinds above
        // `entry_frame_depth + 1`.  Pop the dangling entry frame so
        // subsequent opcodes in the outer script run in the correct
        // frame (same cleanup pattern as `call_internal`).
        if run_result.is_err()
            && self.frames.len() > entry_frames
            && self.frames.last().map(|f| f.base) == Some(new_base)
        {
            self.pop_frame();
        }

        // ── Branch on how the body exited ───────────────────────────────
        // Taking the yield marker tells us whether to treat run_result as
        // a yield value (discard) or a return value (propagate).
        let yielded = self.generator_yielded.take();
        self.completion_value = saved_completion;

        match run_result {
            Ok(_) if yielded.is_some() => {
                // Yield path — `generator_yielded` holds the value, and
                // Op::Yield has already re-populated `state.suspended`.
                // The Ok value itself is a placeholder (JsValue::Undefined)
                // written by Op::Yield's exit branch.
                let value = yielded.expect("just checked");
                Ok((value, false))
            }
            Ok(return_value) => {
                // Body ran off the end (ReturnUndefined path) or hit an
                // explicit `return expr` — either way `run()` returns the
                // value to use for the last iterator result.
                mark_completed(self);
                Ok((return_value, true))
            }
            Err(e) => {
                mark_completed(self);
                Err(e)
            }
        }
    }

    /// Build a `{ value, done }` iterator-result object (§7.4.8
    /// CreateIterResultObject).  Shared between generator `.next` /
    /// `.return` / `.throw`, array iterator next, string iterator next,
    /// and any other `IteratorResult`-shaped allocation.
    ///
    /// Prototype is `%Object.prototype%` per spec (CreateIterResultObject
    /// step 1: OrdinaryObjectCreate(%Object.prototype%)), making
    /// `gen.next().toString === Object.prototype.toString`.
    pub(crate) fn create_iter_result(&mut self, value: JsValue, done: bool) -> ObjectId {
        let obj = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: self.object_prototype,
            extensible: true,
        });
        let value_key = PropertyKey::String(self.well_known.value);
        self.define_shaped_property(
            obj,
            value_key,
            PropertyValue::Data(value),
            PropertyAttrs::DATA,
        );
        let done_key = PropertyKey::String(self.well_known.done);
        self.define_shaped_property(
            obj,
            done_key,
            PropertyValue::Data(JsValue::Boolean(done)),
            PropertyAttrs::DATA,
        );
        obj
    }
}

// ---------------------------------------------------------------------------
// Native: Generator.prototype.next (+ [Symbol.iterator] = self)
// ---------------------------------------------------------------------------

/// `Generator.prototype.next(value)` — §25.4.1.2
pub(super) fn native_generator_next(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(gen_id) = this else {
        return Err(VmError::type_error(
            "Generator.prototype.next called on non-Generator",
        ));
    };
    if !matches!(ctx.get_object(gen_id).kind, ObjectKind::Generator(_)) {
        return Err(VmError::type_error(
            "Generator.prototype.next called on non-Generator",
        ));
    }
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let (value, done) = ctx
        .vm
        .resume_generator(gen_id, FrameCompletion::Normal(arg))?;
    let obj = ctx.vm.create_iter_result(value, done);
    Ok(JsValue::Object(obj))
}

/// `Generator.prototype.return(value)` — §25.4.1.3.
///
/// Injects a `return v` completion at the suspension point.  The
/// generator's in-scope `finally` blocks run (via the shared
/// [`VmInner::resume_generator`] path); if a finally performs its own
/// abrupt completion that overrides per §13.15.  If no finally is in
/// scope, the generator is simply completed with `v`.
pub(super) fn native_generator_return(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(gen_id) = this else {
        return Err(VmError::type_error(
            "Generator.prototype.return called on non-Generator",
        ));
    };
    if !matches!(ctx.get_object(gen_id).kind, ObjectKind::Generator(_)) {
        return Err(VmError::type_error(
            "Generator.prototype.return called on non-Generator",
        ));
    }
    let value = args.first().copied().unwrap_or(JsValue::Undefined);
    let (v, done) = ctx
        .vm
        .resume_generator(gen_id, FrameCompletion::Return(value))?;
    let obj = ctx.vm.create_iter_result(v, done);
    Ok(JsValue::Object(obj))
}

/// `Generator.prototype.throw(err)` — §25.4.1.4.
///
/// Injects a throw at the suspension point so in-scope catch / finally
/// can observe it.  An uncaught throw propagates out as a [`VmError`]
/// and marks the generator Completed.
pub(super) fn native_generator_throw(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(gen_id) = this else {
        return Err(VmError::type_error(
            "Generator.prototype.throw called on non-Generator",
        ));
    };
    if !matches!(ctx.get_object(gen_id).kind, ObjectKind::Generator(_)) {
        return Err(VmError::type_error(
            "Generator.prototype.throw called on non-Generator",
        ));
    }
    let reason = args.first().copied().unwrap_or(JsValue::Undefined);
    let (v, done) = ctx
        .vm
        .resume_generator(gen_id, FrameCompletion::Throw(reason))?;
    let obj = ctx.vm.create_iter_result(v, done);
    Ok(JsValue::Object(obj))
}

/// `Generator.prototype[Symbol.iterator]` returns `this`.
pub(super) fn native_generator_iterator_self(
    _ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(this)
}

// ---------------------------------------------------------------------------
// Async coroutine — Promise-wrapped generator
// ---------------------------------------------------------------------------

/// Drive one step of an async coroutine — invoked initially by the async
/// function call site, and subsequently by Promise `.then` continuations
/// attached to each awaited Promise.
///
/// On yield: treat the yielded value as a Promise, subscribe continuation
/// steps (`AsyncDriverStep { is_throw: false/true }`) for fulfil/reject.
///
/// On return: resolve the wrapper Promise with the final return value.
///
/// On throw: reject the wrapper Promise with the thrown reason.
pub(crate) fn drive_async_coroutine(
    vm: &mut VmInner,
    gen_id: ObjectId,
    value: JsValue,
    is_throw: bool,
) -> Result<(), VmError> {
    // Resume: either with a value (from fulfill) or a throw (from reject).
    let completion = if is_throw {
        FrameCompletion::Throw(value)
    } else {
        FrameCompletion::Normal(value)
    };
    let result = vm.resume_generator(gen_id, completion);

    // Look up the wrapper promise (pre-established at async function call).
    let wrapper = match &vm.get_object(gen_id).kind {
        ObjectKind::Generator(state) => state.wrapper,
        _ => None,
    };
    let Some(wrapper) = wrapper else {
        // Someone drove a user-visible generator via this path — shouldn't
        // happen; the Generator.prototype.next native uses a different
        // entry point.  Just forward the error upward.
        return result.map(|_| ());
    };

    match result {
        Ok((yielded_value, false)) => {
            // Await: treat the yielded value as a Promise (auto-wrap).
            // `AsyncDriverStep` dispatch in `interpreter.rs` does not
            // save/restore `gc_enabled` (user JS inside the resumed body
            // needs GC to keep running), so the intervening allocations
            // below could observe a collection cycle between the first
            // `alloc_async_step` and the call to `subscribe_then`.  The
            // first step would then exist only as an `ObjectId` in a Rust
            // local — not a GC root — and be reclaimed.  Disable GC for
            // the allocation + linkage window and restore on exit; once
            // `subscribe_then` has attached the step to the awaited
            // promise's reaction list, normal reachability takes over.
            let awaited = match yielded_value {
                JsValue::Object(id) if matches!(vm.get_object(id).kind, ObjectKind::Promise(_)) => {
                    id
                }
                other => {
                    let p = super::natives_promise::create_promise(vm);
                    let _ = super::natives_promise::settle_promise(vm, p, false, other);
                    p
                }
            };
            // Attach fulfil + reject continuation steps.
            let saved_gc_enabled = vm.gc_enabled;
            vm.gc_enabled = false;
            let fulfill_step = alloc_async_step(vm, gen_id, false);
            let reject_step = alloc_async_step(vm, gen_id, true);
            super::natives_promise::subscribe_then(vm, awaited, fulfill_step, reject_step);
            vm.gc_enabled = saved_gc_enabled;
        }
        Ok((return_value, true)) => {
            let _ = super::natives_promise::settle_promise(vm, wrapper, false, return_value);
        }
        Err(e) => {
            let reason = vm.vm_error_to_thrown(&e);
            let _ = super::natives_promise::settle_promise(vm, wrapper, true, reason);
        }
    }
    Ok(())
}

/// Allocate an `AsyncDriverStep` callable for a continuation.
fn alloc_async_step(vm: &mut VmInner, gen: ObjectId, is_throw: bool) -> ObjectId {
    let proto = vm.function_prototype;
    vm.alloc_object(Object {
        kind: ObjectKind::AsyncDriverStep { gen, is_throw },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    })
}

/// Create an async coroutine wrapping `suspended` and drive its first
/// step.  Returns the wrapper Promise (the async function's return value).
///
/// The initial frame is prepared by the call-site (same shape as a
/// generator initial frame); `drive_async_coroutine` runs the body until
/// the first await, return, or throw.
pub(crate) fn make_async_coroutine_and_drive(
    vm: &mut VmInner,
    suspended: super::value::SuspendedFrame,
) -> Result<JsValue, VmError> {
    let wrapper = super::natives_promise::create_promise(vm);
    let proto = vm.generator_prototype;
    let gen_id = vm.alloc_object(Object {
        kind: ObjectKind::Generator(super::value::GeneratorState {
            status: GeneratorStatus::SuspendedStart,
            suspended: Some(suspended),
            wrapper: Some(wrapper),
        }),
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    // Back-link the saved frame to the coroutine it belongs to.
    if let ObjectKind::Generator(state) = &mut vm.get_object_mut(gen_id).kind {
        if let Some(susp) = &mut state.suspended {
            susp.frame.generator = Some(gen_id);
        }
    }
    // Initial drive: arg value is ignored for SuspendedStart (it's never
    // pushed onto the body's stack), so we pass Undefined.
    drive_async_coroutine(vm, gen_id, JsValue::Undefined, false)?;
    Ok(JsValue::Object(wrapper))
}
