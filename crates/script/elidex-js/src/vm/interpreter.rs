//! Bytecode interpreter: public API for eval/call.
//!
//! The main dispatch loop lives in `dispatch.rs`; this module provides the
//! entry points (`eval`, `run_script`, `call`, `call_construct_native`,
//! `with_call_mode`, and `run_function` for the initial `Eval` frame) and the
//! shared stack helpers.  Call dispatch and frame entry (`call_dispatch`,
//! `call_internal`, `push_js_call_frame`) live in `interpreter_call.rs`.

use crate::bytecode::compiled::CompiledScript;

use std::sync::Arc;

use super::value::{FrameKind, FuncId, JsValue, ObjectId, VmError, VmErrorKind};
use super::VmInner;

// ---------------------------------------------------------------------------
// Vm public API
// ---------------------------------------------------------------------------

impl VmInner {
    /// Parse, compile, and execute JavaScript source code.
    ///
    /// HTML §8.1.7.3 (perform a microtask checkpoint): after the classic
    /// script finishes, drain the microtask queue.  Drain runs regardless
    /// of whether the script
    /// succeeded or threw, so that reactions attached inside a thrown-from
    /// try/catch still fire (spec parity with browser microtask semantics).
    ///
    /// After microtasks, drain the same-window task queue (HTML §8.1.7.3)
    /// so `window.postMessage` listeners observe the event within the
    /// same `eval` call instead of silently deferring to the next host
    /// tick.  `drain_tasks` itself runs a microtask checkpoint between
    /// tasks; the outer drain here clears any microtasks queued by
    /// the tasks' listener bodies.
    pub fn eval(&mut self, source: &str) -> Result<JsValue, VmError> {
        let script = crate::compiler::compile_script(source).map_err(|e| VmError {
            kind: VmErrorKind::CompileError,
            message: e.message,
        })?;
        let result = self.run_script(script);
        self.drain_microtasks();
        #[cfg(feature = "engine")]
        self.drain_tasks();
        // D-17 `#11-custom-elements-vm` — drain any Custom Elements
        // reactions enqueued during script execution + task delivery
        // (Insert / Remove / AttributeChange mutations land in the
        // queue via the `CustomElementReactionConsumer`; pending
        // upgrades from `customElements.define()` are already flushed
        // inside `define` itself per HTML §4.13.4 define() step 18 +
        // §4.13.6 CE-reactions gating). Reaction
        // callbacks may enqueue more reactions — `flush_ce_reactions`
        // iterates until empty (bounded by MAX_CE_DRAIN_ITERATIONS).
        #[cfg(feature = "engine")]
        self.flush_ce_reactions();
        result
    }

    /// Load and execute a compiled script.
    pub fn run_script(&mut self, script: CompiledScript) -> Result<JsValue, VmError> {
        let func_id = self.register_function(script.top_level);
        self.run_function(func_id, JsValue::Undefined, &[])
    }

    /// Call a JS function object with the given `this` and arguments.
    ///
    /// Wrapped in [`Self::with_call_mode`] with [`super::value::CallMode::Call`]
    /// so the `NativeContext` the dispatcher builds for the
    /// NativeFunction arm bakes call-mode (the body's
    /// `ctx.is_construct()` returns `false` and `ctx.new_target()`
    /// returns `None`), and so a panic mid-dispatch truncates the
    /// frame + value stacks back to the entry depth before re-raise
    /// (D-17b-r1 R2 CRIT-1 pre-existing-issue fix). Construct-mode
    /// callers MUST use [`Self::call_construct_native`] instead so
    /// the native body's `is_construct()` / `new_target()` reads see
    /// the right invocation context.
    pub fn call(
        &mut self,
        func_obj_id: ObjectId,
        this: JsValue,
        args: &[JsValue],
    ) -> Result<JsValue, VmError> {
        self.with_call_mode(super::value::CallMode::Call, |vm, mode| {
            vm.call_dispatch(func_obj_id, this, args, mode)
        })
    }

    /// `[[Construct]]`-mode counterpart to [`Self::call`] for native
    /// constructor dispatch via `do_new`'s native-ctor branch. Wraps
    /// the dispatch in [`Self::with_call_mode`] with
    /// `CallMode::Construct { new_target }` so the `NativeContext`
    /// for the entry frame bakes the right construct context — the
    /// body's `ctx.is_construct()` / `ctx.new_target()` then read
    /// from [`super::value::NativeContext::mode`] instead of an
    /// out-of-band side channel. Otherwise identical to `Vm::call`.
    pub(crate) fn call_construct_native(
        &mut self,
        func_obj_id: ObjectId,
        this: JsValue,
        args: &[JsValue],
        new_target: ObjectId,
    ) -> Result<JsValue, VmError> {
        self.with_call_mode(
            super::value::CallMode::Construct { new_target },
            |vm, mode| vm.call_dispatch(func_obj_id, this, args, mode),
        )
    }

    /// Single `catch_unwind` boundary for a VM dispatch invocation.
    /// Records the entry frame + value-stack depths, runs `f(self,
    /// mode)`, and on either return path truncates both stacks back
    /// to the entry depth so `VmInner` stays consistent for any
    /// outer dispatch (in the panic case the truncate runs **before**
    /// `resume_unwind`, so the upstream catch sees a clean VM).
    ///
    /// Replaces the pre-D-17b-r1 `dispatch_with_construct_entry`
    /// (D-17b R12 G12-2 origin) — the per-invocation construct mode
    /// it tracked via a `Vec<Option<ObjectId>>` side channel is now
    /// a type-level [`super::value::CallMode`] threaded through the
    /// `mode` parameter into both the JS path
    /// ([`Self::push_js_call_frame`] → `CallFrame::mode`) and the
    /// native path ([`super::value::NativeContext::new_call`] /
    /// `::new_construct`). Same machinery, new purpose
    /// (one-issue-one-way full unification).
    ///
    /// **Catch_unwind contract** (AssertUnwindSafe rationale per
    /// D-17b-r1 Phase 0b step 4):
    /// * **Manually restored on panic**: `self.frames` length +
    ///   `self.stack` length (both truncated back to entry depth
    ///   before re-raise so a subsequent dispatch sees a clean state).
    ///   Open upvalues on the truncated frames are `close_upvalues`'d
    ///   before the truncate so any escaped closure that captured a
    ///   parent-frame local sees the snapshotted `Closed(value)` —
    ///   without this step, an Upvalue in `Open { frame_base, slot }`
    ///   would point at a stack index the truncate just dropped,
    ///   causing OOB or silent slot reuse on subsequent reads.
    ///   `self.gc_enabled`, `self.active_bound_key`, and
    ///   `self.completion_value` are saved on entry and restored
    ///   unconditionally on every exit path (Ok/Err return + panic
    ///   re-raise). For `gc_enabled` / `active_bound_key` this is a
    ///   safety net that wraps the NativeFunction arm's per-call
    ///   `gc_enabled = saved_gc` / `active_bound_key =
    ///   saved_bound_key` — if a panic skips the inner restore, the
    ///   outer restore here puts the field back to its
    ///   pre-`with_call_mode` value, preventing `gc_enabled = false`
    ///   from persisting (which would silently disable GC for the
    ///   rest of the VM's lifetime) and `active_bound_key` from
    ///   leaking a stale bound-key into the next bound-accessor
    ///   invocation. For `completion_value` this **is** the save /
    ///   restore (no inner pair after D-17b-r2
    ///   `#11-frame-completion-disentanglement`): the
    ///   [`super::value::FrameKind`] split makes Function-kind frames
    ///   leave `completion_value` invariant, so the only writes are
    ///   Eval-kind frames inside a `with_call_mode` boundary, and
    ///   this outer restore preserves the caller's value across the
    ///   inner Eval body (nested `Vm::eval`, `construct_synchronous`
    ///   JS branch, or any native callback that re-enters).
    /// * **Leaked-but-acceptable on panic**: `exception_handlers`
    ///   on popped frames (handlers belong to the truncated frames
    ///   and die with them — no leak).
    pub(crate) fn with_call_mode<F, R>(
        &mut self,
        mode: super::value::CallMode,
        f: F,
    ) -> Result<R, VmError>
    where
        F: FnOnce(&mut Self, super::value::CallMode) -> Result<R, VmError>,
    {
        use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
        let entry_frames = self.frames.len();
        let entry_stack_depth = self.stack.len();
        // Snapshot VmInner fields restored unconditionally on every
        // exit path below. For `gc_enabled` / `active_bound_key`
        // this wraps the NativeFunction arm's per-call save/restore
        // (`saved_gc` / `saved_bound_key`), guarding against panics
        // that skip the inner restore. For `completion_value` this
        // **is** the save/restore — the [`super::value::FrameKind`]
        // split makes Function-kind frames invariant under
        // `completion_value`, so this boundary preserves the
        // caller's value across any Eval body re-entered inside the
        // closure (D-17b-r2 `#11-frame-completion-disentanglement`).
        //
        // `completion_value` is pushed onto `saved_completion_stack`
        // (walked by `gc/roots.rs::mark_roots`) rather than kept in
        // a Rust local — without that, an outer-scope heap Object
        // held only by the Rust local would be unreachable to GC
        // when the closure overwrites `self.completion_value` (e.g.
        // an inner Eval body's entry-gated `Op::PopCompletion` write), so a
        // collection mid-closure would sweep the slot and the
        // cleanup restore below would write a dangling ObjectId back
        // into VmInner. Pre-r2 the analogous root was
        // `CallFrame::saved_completion` (walked by
        // `super::gc::roots::mark_roots`'s frame loop before this PR).
        let saved_gc_enabled = self.gc_enabled;
        let saved_active_bound_key = self.active_bound_key;
        self.saved_completion_stack.push(self.completion_value);
        let result = catch_unwind(AssertUnwindSafe(|| f(self, mode)));
        // R19 spirit preserved via three-way split debug_assert:
        //   (a) no panic + inner Ok: strict `==` — normal return must
        //       restore the exact entry frame depth (push/pop balance).
        //   (b) no panic + inner Err: weak `>=` — an uncaught JS throw
        //       may leave the pushed call frame on the stack (matches
        //       `call_internal`'s post-`run()` error-cleanup), and
        //       the truncate below cleans it.
        //   (c) panic (catch_unwind Err): weak `>=` — panic-recovery
        //       may have pushed-but-stuck entries the truncate cleans.
        // Shared `>=` bound on the value stack catches underflow in
        // all three cases.
        match &result {
            Ok(Ok(_)) => {
                debug_assert_eq!(
                    self.frames.len(),
                    entry_frames,
                    "Ok return leaked {} frame(s) above entry {}",
                    self.frames.len().saturating_sub(entry_frames),
                    entry_frames,
                );
                debug_assert_eq!(
                    self.stack.len(),
                    entry_stack_depth,
                    "Ok return leaked {} value-stack slot(s) above entry {}",
                    self.stack.len().saturating_sub(entry_stack_depth),
                    entry_stack_depth,
                );
            }
            Ok(Err(_)) | Err(_) => debug_assert!(
                self.frames.len() >= entry_frames,
                "frame stack underflow: length {} below entry {}",
                self.frames.len(),
                entry_frames,
            ),
        }
        debug_assert!(
            self.stack.len() >= entry_stack_depth,
            "value-stack length {} below entry {} — underflow",
            self.stack.len(),
            entry_stack_depth,
        );
        // Close upvalues on any frames above the entry depth BEFORE
        // truncating — otherwise an Upvalue still in
        // `UpvalueState::Open { frame_base, slot }` would point at a
        // stack index the `stack.truncate` drops, causing OOB or
        // silent slot reuse on subsequent reads. Iterate from the top
        // (`frames.last()`) downward so a closure escaped from frame
        // N capturing frame N-1's local still observes the snapshot
        // taken at frame N-1's close. The `Ok(Ok(_))` arm's truncates
        // below are no-ops (asserted above), so this loop only does
        // real work on the Err and panic paths.
        while self.frames.len() > entry_frames {
            // Pop first so the owned `frame.local_upvalue_ids` Vec
            // can be passed to `close_upvalues` without cloning.
            // `close_upvalues` reads `self.stack[frame_base + slot]`
            // by absolute index, so doing the pop before the stack
            // truncate (which happens after the loop) is still safe.
            let frame = self
                .frames
                .pop()
                .expect("frames.len() > entry_frames per loop guard");
            self.close_upvalues(&frame.local_upvalue_ids);
        }
        self.stack.truncate(entry_stack_depth);
        // Outer restore (Ok/Err/panic equally). `gc_enabled` /
        // `active_bound_key` wrap the NativeFunction arm's
        // per-invocation inner save/restore; `completion_value` is
        // the sole save/restore boundary (see docstring), kept
        // alive via `saved_completion_stack` so GC sees it across
        // any inner Eval write.
        self.gc_enabled = saved_gc_enabled;
        self.active_bound_key = saved_active_bound_key;
        self.completion_value = self
            .saved_completion_stack
            .pop()
            .expect("with_call_mode push/pop balance");
        match result {
            Ok(r) => r,
            Err(payload) => resume_unwind(payload),
        }
    }

    /// Run a function as the initial (or only) frame.
    ///
    /// Sole [`FrameKind::Eval`] push site: top-level script or `eval`
    /// body invoked via [`Self::eval`] / [`Self::run_script`].
    /// Wrapping in [`Self::with_call_mode`] (with
    /// [`super::value::CallMode::Call`]) makes the entry an outer
    /// save/restore boundary for `completion_value`, so nested
    /// `Vm::eval` / native re-entry preserves the outer Eval frame's
    /// script-completion value across the inner body's
    /// [`super::value::FrameKind::Eval`] writes (ECMA-262 §16.1.6
    /// step 13.a + 17 / §19.2.1.1 step 29.a + 33). Mirrors `Vm::call`
    /// / `Vm::call_construct_native`'s panic-safety contract for the
    /// Eval entry path.
    fn run_function(
        &mut self,
        func_id: FuncId,
        this: JsValue,
        args: &[JsValue],
    ) -> Result<JsValue, VmError> {
        self.with_call_mode(super::value::CallMode::Call, |vm, _mode| {
            // ECMA-262 §16.1.6 ScriptEvaluation step 13.b — the body's
            // initial completion is `empty`, surfaced as
            // `NormalCompletion(undefined)` when no entry-frame
            // `Op::PopCompletion` write fires (empty source, or last statement
            // is not an ExpressionStatement). The outer caller's
            // value is already preserved on `saved_completion_stack`
            // by `with_call_mode`'s entry push, so resetting here
            // does not corrupt nested re-entry.
            vm.completion_value = JsValue::Undefined;
            vm.call_internal(func_id, this, args, Arc::from([]), FrameKind::Eval)
        })
    }
}

// ---------------------------------------------------------------------------
// Shared stack helpers (used by dispatch.rs and ops.rs)
// ---------------------------------------------------------------------------

impl VmInner {
    pub(crate) fn pop(&mut self) -> Result<JsValue, VmError> {
        self.stack
            .pop()
            .ok_or_else(|| VmError::internal("stack underflow"))
    }

    pub(crate) fn peek(&self) -> Result<JsValue, VmError> {
        self.stack
            .last()
            .copied()
            .ok_or_else(|| VmError::internal("stack underflow on peek"))
    }
}
