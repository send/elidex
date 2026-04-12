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
    PropertyStorage, PropertyValue, UpvalueState, VmError,
};
use super::VmInner;

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
