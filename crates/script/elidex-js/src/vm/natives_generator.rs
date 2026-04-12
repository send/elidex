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
    GeneratorStatus, JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey,
    PropertyStorage, PropertyValue, SuspendedFrame, UpvalueState, VmError, VmErrorKind,
};
#[allow(unused_imports)] // VmErrorKind is used by resume_generator_throw
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
    let mut upvalue_slots = Vec::new();
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
    /// Resume `gen_id` with input `arg`.  Returns the yielded (or returned)
    /// value plus a `done` flag; builds the actual `{value, done}` object
    /// on top of that.
    ///
    /// Failure modes:
    /// - Generator already Running → TypeError
    /// - Generator already Completed → `{value: undefined, done: true}`
    /// - Body throws → propagate the thrown value and mark Completed
    pub(crate) fn resume_generator(
        &mut self,
        gen_id: ObjectId,
        arg: JsValue,
    ) -> Result<(JsValue, bool), VmError> {
        // ── Lift the suspended frame out of the Generator ───────────────
        let (suspended, initial) = {
            let ObjectKind::Generator(state) = &mut self.get_object_mut(gen_id).kind else {
                return Err(VmError::type_error(
                    "Generator.prototype.next called on non-Generator",
                ));
            };
            match state.status {
                GeneratorStatus::Completed => {
                    return Ok((JsValue::Undefined, true));
                }
                GeneratorStatus::Running => {
                    return Err(VmError::type_error(
                        "Cannot resume a generator that is already running",
                    ));
                }
                GeneratorStatus::SuspendedStart => {
                    state.status = GeneratorStatus::Running;
                    (
                        state.suspended.take().expect("SuspendedStart has frame"),
                        true,
                    )
                }
                GeneratorStatus::SuspendedYield => {
                    state.status = GeneratorStatus::Running;
                    (
                        state.suspended.take().expect("SuspendedYield has frame"),
                        false,
                    )
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
            frame.local_upvalue_ids.push(*uv_id);
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

        // On resume from yield, the arg becomes the value of the `yield`
        // expression.  On the initial `.next()` call the generator hasn't
        // hit a yield yet, so we don't push anything (arg is discarded).
        if !initial {
            self.stack.push(arg);
        }

        self.frames.push(frame);

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

        let mark_completed = |vm: &mut Self| {
            if let ObjectKind::Generator(state) = &mut vm.get_object_mut(gen_id).kind {
                state.status = GeneratorStatus::Completed;
                state.suspended = None;
            }
        };

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
                let _ = entry_frames;
                Ok((return_value, true))
            }
            Err(e) => {
                mark_completed(self);
                Err(e)
            }
        }
    }

    /// Resume `gen_id` by throwing `reason` at the point where it last
    /// yielded.  Used by async-function reject continuations: if the
    /// awaited Promise rejects, the awaiting body should observe a throw
    /// rather than a value.
    ///
    /// The mechanics mirror [`resume_generator`] but instead of pushing the
    /// resume arg, we restore the frame and invoke `handle_exception` with
    /// the generator frame as the entry point.  If the body has a handler
    /// in scope, execution continues at catch/finally; otherwise the error
    /// propagates out of the coroutine.
    pub(crate) fn resume_generator_throw(
        &mut self,
        gen_id: ObjectId,
        reason: JsValue,
    ) -> Result<(JsValue, bool), VmError> {
        // ── Lift the suspended frame out (same gate as resume_generator) ─
        let suspended = {
            let ObjectKind::Generator(state) = &mut self.get_object_mut(gen_id).kind else {
                return Err(VmError::type_error("throw on non-Generator"));
            };
            match state.status {
                GeneratorStatus::Completed => {
                    // Spec §25.4.1.4: throw on a completed iterator propagates
                    // the reason as the thrown completion.
                    return Err(VmError {
                        kind: VmErrorKind::ThrowValue(reason),
                        message: String::new(),
                    });
                }
                GeneratorStatus::Running => {
                    return Err(VmError::type_error(
                        "Cannot resume a generator that is already running",
                    ));
                }
                GeneratorStatus::SuspendedStart | GeneratorStatus::SuspendedYield => {
                    state.status = GeneratorStatus::Running;
                    state.suspended.take().expect("suspended state must exist")
                }
            }
        };
        let super::value::SuspendedFrame {
            mut frame,
            stack_slice,
            upvalue_slots,
        } = suspended;

        let new_base = self.stack.len();
        self.stack.extend(stack_slice);
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
            frame.local_upvalue_ids.push(*uv_id);
        }
        frame.base = new_base;
        frame.cleanup_base = new_base;
        for h in &mut frame.exception_handlers {
            h.stack_depth += new_base;
        }
        let saved_completion = self.completion_value;
        self.completion_value = JsValue::Undefined;
        frame.saved_completion = saved_completion;
        self.frames.push(frame);

        // ── Inject a throw at the suspension point ──────────────────────
        let entry_frames = self.frames.len() - 1;
        let mark_completed = |vm: &mut Self| {
            if let ObjectKind::Generator(state) = &mut vm.get_object_mut(gen_id).kind {
                state.status = GeneratorStatus::Completed;
                state.suspended = None;
            }
        };

        if !self.handle_exception(reason, entry_frames) {
            // No handler — frame was left in place (handle_exception stops
            // at `frame_idx <= entry_frame_depth`).  Pop it and surface
            // the throw as a VmError to our caller.
            self.frames.pop();
            self.completion_value = saved_completion;
            mark_completed(self);
            return Err(VmError {
                kind: VmErrorKind::ThrowValue(reason),
                message: String::new(),
            });
        }

        // Handler active — proceed with normal run() resumption.
        let entry_frames_post_inject = self.frames.len() - 1;
        let run_result = self.run();
        if run_result.is_err()
            && self.frames.len() > entry_frames_post_inject
            && self.frames.last().map(|f| f.base) == Some(new_base)
        {
            self.pop_frame();
        }
        let yielded = self.generator_yielded.take();
        self.completion_value = saved_completion;

        match run_result {
            Ok(_) if yielded.is_some() => Ok((yielded.expect("just checked"), false)),
            Ok(return_value) => {
                mark_completed(self);
                Ok((return_value, true))
            }
            Err(e) => {
                mark_completed(self);
                Err(e)
            }
        }
    }

    /// Build a `{value, done}` iterator-result object, used by
    /// `Generator.prototype.next` and similar iterator steps.
    pub(super) fn iter_result_object(&mut self, value: JsValue, done: bool) -> ObjectId {
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
    let (value, done) = ctx.vm.resume_generator(gen_id, arg)?;
    let obj = ctx.vm.iter_result_object(value, done);
    Ok(JsValue::Object(obj))
}

/// `Generator.prototype.return(value)` — §25.4.1.3
///
/// PR2 commit 4: simplified form — marks the generator Completed without
/// running the body's `finally` blocks, and returns `{value, done: true}`.
/// Spec-complete semantics (abrupt-completion forwarding + finally) ship
/// with PR2.5.
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
    if let ObjectKind::Generator(state) = &mut ctx.vm.get_object_mut(gen_id).kind {
        state.status = GeneratorStatus::Completed;
        state.suspended = None;
    }
    let obj = ctx.vm.iter_result_object(value, true);
    Ok(JsValue::Object(obj))
}

/// `Generator.prototype.throw(err)` — §25.4.1.4
///
/// PR2 commit 4: simplified form — marks the generator Completed and
/// propagates the thrown value synchronously as the native result.  Full
/// semantics (catch-block forwarding + finally) ship with PR2.5.
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
    if let ObjectKind::Generator(state) = &mut ctx.vm.get_object_mut(gen_id).kind {
        state.status = GeneratorStatus::Completed;
        state.suspended = None;
    }
    let reason = args.first().copied().unwrap_or(JsValue::Undefined);
    Err(VmError {
        kind: super::value::VmErrorKind::ThrowValue(reason),
        message: String::new(),
    })
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
    let result = if is_throw {
        vm.resume_generator_throw(gen_id, value)
    } else {
        vm.resume_generator(gen_id, value)
    };

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
            let fulfill_step = alloc_async_step(vm, gen_id, false);
            let reject_step = alloc_async_step(vm, gen_id, true);
            super::natives_promise::subscribe_then(vm, awaited, fulfill_step, reject_step);
        }
        Ok((return_value, true)) => {
            let _ = super::natives_promise::settle_promise(vm, wrapper, false, return_value);
        }
        Err(e) => {
            let reason = if let VmErrorKind::ThrowValue(v) = e.kind {
                v
            } else {
                let msg = vm.strings.intern(&e.to_string());
                JsValue::String(msg)
            };
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
