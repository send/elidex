//! Bytecode interpreter: public API for eval/call.
//!
//! The main dispatch loop lives in `dispatch.rs`; this module provides the
//! entry points (`eval`, `run_script`, `call`) and shared helpers.

use crate::bytecode::compiled::CompiledScript;

use std::sync::Arc;

use super::value::{
    CallFrame, FrameKind, FuncId, JsCalleeInfo, JsValue, ObjectId, ObjectKind, UpvalueId, VmError,
    VmErrorKind,
};
use super::VmInner;

// ---------------------------------------------------------------------------
// Vm public API
// ---------------------------------------------------------------------------

impl VmInner {
    /// Parse, compile, and execute JavaScript source code.
    ///
    /// HTML §8.1.4.2 step 7: after the classic script finishes, drain the
    /// microtask queue.  Drain runs regardless of whether the script
    /// succeeded or threw, so that reactions attached inside a thrown-from
    /// try/catch still fire (spec parity with browser microtask semantics).
    ///
    /// After microtasks, drain the same-window task queue (HTML §8.1.5)
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
        // inside `define` itself per HTML §4.13.4 step 16). Reaction
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
        // an inner Eval body's entry-gated `Op::Pop` write), so a
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

    #[allow(clippy::too_many_lines)] // dispatch table over every callable ObjectKind variant
    pub(super) fn call_dispatch(
        &mut self,
        func_obj_id: ObjectId,
        this: JsValue,
        args: &[JsValue],
        mode: super::value::CallMode,
    ) -> Result<JsValue, VmError> {
        // Unwrap BoundFunction chain iteratively to avoid stack overflow
        // on deeply nested .bind() chains.  MAX_BIND_CHAIN_DEPTH caps O(N²)
        // copy cost for attacker-controlled chain depth.
        let mut current_id = func_obj_id;
        let mut effective_this = this;
        let mut owned_args: Option<Vec<JsValue>> = None;
        let mut depth = 0usize;

        loop {
            let obj = self.get_object(current_id);
            match &obj.kind {
                ObjectKind::Function(fo) => {
                    let func_id = fo.func_id;
                    let upvalue_ids = fo.upvalue_ids.clone();
                    let resolved_this =
                        Self::compute_this_for_call(fo.this_mode, effective_this, fo.captured_this);
                    let call_args = owned_args.as_deref().unwrap_or(args);
                    return self.call_internal(
                        func_id,
                        resolved_this,
                        call_args,
                        upvalue_ids,
                        FrameKind::Function,
                    );
                }
                ObjectKind::NativeFunction(nf) => {
                    // WebIDL §3.7.1 step 1.2 + ECMA-262 §27.2.3.1 step 1
                    // `[[Construct]]`-only mandate.  When a native function
                    // is marked [`super::value::CallShape::ConstructorOnly`]
                    // and the entry call-mode is [`CallMode::Call`] (i.e.
                    // bare invocation without `new`), throw the canonical
                    // TypeError at dispatch — the single chokepoint for
                    // every Interface-object ctor + Promise ctor, replacing
                    // the historic per-body `if !ctx.is_construct() { ... }`
                    // guard.  Sibling of the ECMA-262 §10.2.1 step 4
                    // `is_class_ctor && !mode.is_construct()` gate for
                    // user-defined class ctors at `push_js_call_frame`.
                    if matches!(nf.shape, super::value::CallShape::ConstructorOnly)
                        && !mode.is_construct()
                    {
                        let name = self.strings.get_utf8(nf.name).to_string();
                        return Err(VmError::type_error(format!(
                            "Failed to construct '{name}': Please use the 'new' operator"
                        )));
                    }
                    let func = nf.func;
                    let saved_gc = self.gc_enabled;
                    // Stage the accessor's bound key (re-entrancy: a native may
                    // call another native, so save/restore rather than clear).
                    let saved_bound_key = self.active_bound_key;
                    self.active_bound_key = nf.bound_key;
                    self.gc_enabled = false;
                    let call_args = owned_args.as_deref().unwrap_or(args);
                    // Primary native dispatch site: bake `mode` from
                    // the outer [`Self::with_call_mode`] boundary into
                    // [`super::value::NativeContext::mode`] so the
                    // body's `ctx.is_construct()` / `ctx.new_target()`
                    // reads see the entry-frame's construct discipline
                    // (D-17b-r1 §7.2 — replaces the pre-r1
                    // `native_construct_stack` top-of-stack read).
                    let mut ctx = match mode {
                        super::value::CallMode::Construct { new_target } => {
                            super::value::NativeContext::new_construct(self, new_target)
                        }
                        super::value::CallMode::Call => super::value::NativeContext::new_call(self),
                    };
                    let result = func(&mut ctx, effective_this, call_args);
                    ctx.vm.gc_enabled = saved_gc;
                    ctx.vm.active_bound_key = saved_bound_key;
                    return result;
                }
                ObjectKind::BoundFunction {
                    target,
                    bound_this,
                    bound_args,
                } => {
                    depth += 1;
                    if depth > crate::vm::MAX_BIND_CHAIN_DEPTH {
                        return Err(VmError::range_error("Maximum bind chain depth exceeded"));
                    }
                    let next_id = *target;
                    effective_this = *bound_this;
                    if !bound_args.is_empty() {
                        let prev = owned_args.take();
                        let extra = prev.as_deref().unwrap_or(args);
                        let mut combined = bound_args.clone();
                        combined.extend_from_slice(extra);
                        owned_args = Some(combined);
                    }
                    current_id = next_id;
                }
                ObjectKind::PromiseResolver { promise, is_reject } => {
                    // ECMA-262 §27.2.1.3.1 / §27.2.1.3.2: invoking a Promise
                    // resolve/reject function drops its capability slot.  GC
                    // must not run inside the native body since we're about
                    // to mutate the target promise and enqueue reactions.
                    let promise = *promise;
                    let is_reject = *is_reject;
                    let saved_gc = self.gc_enabled;
                    self.gc_enabled = false;
                    let call_args = owned_args.as_deref().unwrap_or(args);
                    let value = call_args.first().copied().unwrap_or(JsValue::Undefined);
                    let result =
                        super::natives_promise::settle_promise(self, promise, is_reject, value);
                    self.gc_enabled = saved_gc;
                    return result;
                }
                ObjectKind::PromiseCombinatorStep(step) => {
                    let step = *step;
                    let saved_gc = self.gc_enabled;
                    self.gc_enabled = false;
                    let call_args = owned_args.as_deref().unwrap_or(args);
                    let value = call_args.first().copied().unwrap_or(JsValue::Undefined);
                    let result =
                        super::natives_promise_combinator::step_combinator(self, step, value);
                    self.gc_enabled = saved_gc;
                    return result;
                }
                ObjectKind::PromiseFinallyStep {
                    on_finally,
                    is_reject,
                } => {
                    // `on_finally` itself runs user JS, so leave GC alone —
                    // the callee paths already save/restore `gc_enabled`.
                    let on_finally = *on_finally;
                    let is_reject = *is_reject;
                    let call_args = owned_args.as_deref().unwrap_or(args);
                    let value = call_args.first().copied().unwrap_or(JsValue::Undefined);
                    return super::natives_promise_combinator::run_finally_step(
                        self, on_finally, is_reject, value,
                    );
                }
                ObjectKind::AsyncDriverStep { gen, is_throw } => {
                    let gen = *gen;
                    let is_throw = *is_throw;
                    let call_args = owned_args.as_deref().unwrap_or(args);
                    let value = call_args.first().copied().unwrap_or(JsValue::Undefined);
                    super::natives_generator::drive_async_coroutine(self, gen, value, is_throw)?;
                    return Ok(JsValue::Undefined);
                }
                #[cfg(feature = "engine")]
                ObjectKind::ReadableStreamStartStep {
                    stream_id,
                    is_reject,
                } => {
                    let stream_id = *stream_id;
                    let is_reject = *is_reject;
                    let call_args = owned_args.as_deref().unwrap_or(args);
                    let value = call_args.first().copied().unwrap_or(JsValue::Undefined);
                    return super::host::readable_stream::run_start_step(
                        self, stream_id, is_reject, value,
                    );
                }
                #[cfg(feature = "engine")]
                ObjectKind::ReadableStreamPullStep {
                    stream_id,
                    is_reject,
                } => {
                    let stream_id = *stream_id;
                    let is_reject = *is_reject;
                    let call_args = owned_args.as_deref().unwrap_or(args);
                    let value = call_args.first().copied().unwrap_or(JsValue::Undefined);
                    return super::host::readable_stream::run_pull_step(
                        self, stream_id, is_reject, value,
                    );
                }
                #[cfg(feature = "engine")]
                ObjectKind::ReadableStreamCancelStep { promise, is_reject } => {
                    let promise = *promise;
                    let is_reject = *is_reject;
                    let call_args = owned_args.as_deref().unwrap_or(args);
                    let value = call_args.first().copied().unwrap_or(JsValue::Undefined);
                    return super::host::readable_stream::run_cancel_step(
                        self, promise, is_reject, value,
                    );
                }
                // D-16 `#11-wasm-vm` — exported wasm function exotic
                // dispatch (WASM JS API §5.6).  The call adapter
                // resolves the engine-bridge `WasmFunc` from
                // `wasm_exported_func_storage[func_obj_id]` and
                // dispatches via F1 `WasmFunc::call(args,
                // ScriptHostBinding)`.  The arm threads `this =
                // JsValue::Object(func_obj_id)` so the adapter can
                // recover the payload by its own ObjectId (the
                // standard call path passes the callee as `this`
                // when no bound receiver is set).
                #[cfg(feature = "engine")]
                ObjectKind::WasmExportedFunction => {
                    let saved_gc = self.gc_enabled;
                    self.gc_enabled = false;
                    let call_args = owned_args.as_deref().unwrap_or(args);
                    let mut ctx = match mode {
                        super::value::CallMode::Construct { new_target } => {
                            super::value::NativeContext::new_construct(self, new_target)
                        }
                        super::value::CallMode::Call => super::value::NativeContext::new_call(self),
                    };
                    let result = super::host::wasm::exported_func::call_wasm_exported_function(
                        &mut ctx, current_id, call_args,
                    );
                    ctx.vm.gc_enabled = saved_gc;
                    return result;
                }
                _ => return Err(VmError::type_error("not a function")),
            }
        }
    }

    /// Internal: push a frame and run a compiled function.
    ///
    /// Used by the public `call()` API and `NativeContext` re-entrant calls.
    /// The inline dispatch path uses `push_js_call_frame` instead.
    ///
    /// `kind` discriminates script/`eval` body entries (only reached
    /// via `run_function` from `Vm::eval` / `Vm::run_script`) from
    /// every other entry; async + generator bodies are always
    /// `Function`-kind regardless of caller (§27.5/§27.7 completion
    /// machinery is not script-completion-value-shaped).
    #[allow(clippy::too_many_lines)] // generator + async + regular frame paths
    pub(crate) fn call_internal(
        &mut self,
        func_id: FuncId,
        this: JsValue,
        args: &[JsValue],
        upvalue_ids: Arc<[UpvalueId]>,
        kind: FrameKind,
    ) -> Result<JsValue, VmError> {
        // ECMA-262 §10.2.1 step 4: class constructors throw a
        // TypeError when invoked in `[[Call]]` mode (i.e. without
        // `new`). `call_internal` builds its frame inline rather
        // than via `push_js_call_frame` (where the equivalent guard
        // lives for the Op::Call / Op::CallMethod / `Vm::call`
        // entry paths), so we replicate the check here for the
        // `Vm::call`-Function-arm + `run_function` entry paths.
        // Construct mode (`do_new` / `construct_synchronous`) goes
        // through `push_js_call_frame` directly and never reaches
        // `call_internal`, so this guard does not gate legitimate
        // construct paths.
        let compiled = self.get_compiled(func_id);
        if compiled.is_class_ctor {
            let name = compiled.name.as_deref().unwrap_or("");
            let msg = if name.is_empty() {
                "Class constructor cannot be invoked without 'new'".to_string()
            } else {
                format!("Class constructor {name} cannot be invoked without 'new'")
            };
            return Err(VmError::type_error(msg));
        }
        let local_count = compiled.local_count as usize;
        let param_count = compiled.param_count as usize;
        let needs_arguments = compiled.needs_arguments;
        let is_generator = compiled.is_generator;
        let is_async = compiled.is_async;
        // Eval-kind callers (top-level script via `run_function`) must
        // not target an async or generator FuncId — the suspended-
        // frame arms below hardcode `FrameKind::Function` and a
        // mismatched `kind` would be silently dropped. Top-level
        // script bodies are never async/generator in elidex-js core
        // (per `docs/design/ja/14-script-engines-webapi.md` §14.1
        // strict-only baseline + top-level await deferred), so this
        // is a sanity guard against future refactors that route an
        // async/generator FuncId through `run_function`.
        debug_assert!(
            matches!(kind, FrameKind::Function) || !(is_async || is_generator),
            "call_internal: FrameKind::Eval requested for async/generator FuncId — \
             top-level script must be a non-async non-generator function",
        );
        // `home_class` is always `None` on the `call_internal` entry
        // path: class-ctor invocations (`new ClassCtor(...)`) go
        // through `push_js_call_frame` (which threads
        // `Some(closure_obj_id)` itself); any class-ctor frame that
        // reached here would have been rejected by the
        // `is_class_ctor` early-return above. Kept as an explicit
        // local so the `push_js_call_frame`-call below stays uniform
        // with its construct-mode siblings — D-17b R9 G9-1 dead-
        // branch + dead-parameter removal.
        let home_class: Option<ObjectId> = None;
        // Rest-param packing (Stage 0 prereq) — must materialize the
        // rest array BEFORE the args copy below clobbers slot
        // (param_count - 1) with the first excess arg only.
        let has_rest_param = compiled.has_rest_param;

        let entry_frames = self.frames.len();
        let base = self.stack.len();

        // Allocate locals (initialized to Undefined).
        self.stack.resize(base + local_count, JsValue::Undefined);

        // Copy args into param slots.
        let copy_count = args.len().min(param_count);
        self.stack[base..base + copy_count].copy_from_slice(&args[..copy_count]);

        // Rest-param packing for the entry-call path (Stage 0
        // sibling of `push_js_call_frame`'s logic): pack
        // `args[param_count - 1 ..]` into a fresh Array stored in
        // slot `param_count - 1`. Mirrors the inline-dispatch
        // pre-slot-adjust snapshot — call_internal pads with
        // Undefined first, so we snapshot directly from the `args`
        // slice (which still holds the actual values, including
        // any beyond `param_count - 1` that copy_count truncated).
        if has_rest_param && param_count > 0 {
            let rest_slot_idx = param_count - 1;
            let rest_elements: Vec<JsValue> = if args.len() > rest_slot_idx {
                args[rest_slot_idx..].to_vec()
            } else {
                Vec::new()
            };
            let arr_id = self.create_array_object(rest_elements);
            self.stack[base + rest_slot_idx] = JsValue::Object(arr_id);
        }

        let (tdz_bits, tdz_overflow) = CallFrame::tdz_init(local_count);

        // Async function re-entry via `call()`: same treatment as the
        // inline dispatch path — build an initial SuspendedFrame and
        // drive one step, returning the wrapper Promise.
        if is_async {
            let stack_slice: Vec<JsValue> = self.stack.drain(base..).collect();
            let initial_frame = CallFrame {
                func_id,
                ip: 0,
                base,
                cleanup_base: base,
                upvalue_ids: upvalue_ids.clone(),
                local_upvalue_ids: Vec::new(),
                this_value: this,
                exception_handlers: Vec::new(),
                tdz_bits,
                tdz_overflow: tdz_overflow.clone(),
                actual_args: if needs_arguments {
                    Some(args.to_vec())
                } else {
                    None
                },
                new_instance: None,
                mode: super::value::CallMode::Call,
                home_class,
                kind: FrameKind::Function,
                generator: None,
                pending_completion: None,
            };
            let suspended = super::value::SuspendedFrame {
                frame: initial_frame,
                stack_slice,
                upvalue_slots: Vec::new(),
            };
            return super::natives_generator::make_async_coroutine_and_drive(self, suspended);
        }

        // Generator: build initial suspended frame and return Generator
        // object directly — body runs only when `.next()` resumes.
        if is_generator {
            let stack_slice: Vec<JsValue> = self.stack.drain(base..).collect();
            let initial_frame = CallFrame {
                func_id,
                ip: 0,
                base,
                cleanup_base: base,
                upvalue_ids,
                local_upvalue_ids: Vec::new(),
                this_value: this,
                exception_handlers: Vec::new(),
                tdz_bits,
                tdz_overflow,
                actual_args: if needs_arguments {
                    Some(args.to_vec())
                } else {
                    None
                },
                new_instance: None,
                mode: super::value::CallMode::Call,
                home_class,
                kind: FrameKind::Function,
                generator: None,
                pending_completion: None,
            };
            let suspended = super::value::SuspendedFrame {
                frame: initial_frame,
                stack_slice,
                upvalue_slots: Vec::new(),
            };
            let proto = self.generator_prototype;
            let gen_id = self.alloc_object(super::value::Object {
                kind: super::value::ObjectKind::Generator(Box::new(super::value::GeneratorState {
                    status: super::value::GeneratorStatus::SuspendedStart,
                    suspended: Some(suspended),
                    wrapper: None,
                })),
                storage: super::value::PropertyStorage::shaped(super::shape::ROOT_SHAPE),
                prototype: proto,
                extensible: true,
            });
            if let super::value::ObjectKind::Generator(state) =
                &mut self.get_object_mut(gen_id).kind
            {
                if let Some(susp) = &mut state.suspended {
                    susp.frame.generator = Some(gen_id);
                }
            }
            return Ok(JsValue::Object(gen_id));
        }

        self.frames.push(CallFrame {
            func_id,
            ip: 0,
            base,
            upvalue_ids,
            local_upvalue_ids: Vec::new(),
            this_value: this,
            exception_handlers: Vec::new(),
            tdz_bits,
            tdz_overflow,
            actual_args: if needs_arguments {
                Some(args.to_vec())
            } else {
                None
            },
            cleanup_base: base,
            new_instance: None,
            mode: super::value::CallMode::Call,
            home_class,
            kind,
            generator: None,
            pending_completion: None,
        });

        let result = self.run();

        // On error, clean up the frame if it's still on the stack.
        // The inner run() may have left it if the throw was uncaught.
        if result.is_err()
            && self.frames.len() > entry_frames
            && self.frames.last().map(|f| f.base) == Some(base)
        {
            self.pop_frame();
        }

        result
    }

    /// Push a JS function call frame for the single dispatcher.
    ///
    /// Args are already on the stack. `cleanup_offset` is the number of
    /// extra slots below the args (1 for callee, 2 for receiver + callee).
    /// Does **not** call `run()` — the caller must `continue` the dispatch loop.
    ///
    /// Returns `Err` (without mutating the stack or frame state)
    /// when invoked in [`CallMode::Call`] mode on a class
    /// constructor — ECMA-262 §10.2.1 step 4 requires throwing a
    /// `TypeError` at the call boundary. Single chokepoint for this
    /// check so every Op::Call / Op::CallMethod / Vm::call entry
    /// path inherits the gate without per-site duplication
    /// (One-issue-one-way).
    #[allow(clippy::too_many_lines)] // generator + async + regular frame paths
    pub(crate) fn push_js_call_frame(
        &mut self,
        callee: JsCalleeInfo,
        this: JsValue,
        argc: usize,
        cleanup_offset: usize,
        new_instance: Option<ObjectId>,
        mode: super::value::CallMode,
    ) -> Result<(), VmError> {
        let compiled = self.get_compiled(callee.func_id);
        // ECMA-262 §10.2.1 step 4 — class constructor in call mode is
        // a TypeError. Construct-mode entries (`do_new` /
        // `construct_synchronous`) pass [`CallMode::Construct`] and so
        // bypass this guard.
        if compiled.is_class_ctor && !mode.is_construct() {
            let name = compiled.name.as_deref().unwrap_or("");
            let msg = if name.is_empty() {
                "Class constructor cannot be invoked without 'new'".to_string()
            } else {
                format!("Class constructor {name} cannot be invoked without 'new'")
            };
            return Err(VmError::type_error(msg));
        }
        let local_count = compiled.local_count as usize;
        let param_count = compiled.param_count as usize;
        let needs_arguments = compiled.needs_arguments;
        let is_generator = compiled.is_generator;
        let is_async = compiled.is_async;
        let has_rest_param = compiled.has_rest_param;
        // Class-ctor frames carry `home_class = Some(closure_id)` so
        // `Op::SuperCall` ([C13] SuperCall) resolves the super class
        // via `home_class.[[Prototype]]`. Regular methods + non-class
        // functions get `None` (CE-minimal scope per D-17b §3.1(c)
        // — non-ctor super-property reads stay Step-9-deferred, defer
        // slot `#11-step9-class-extras`).
        let home_class: Option<ObjectId> = if compiled.is_class_ctor {
            Some(callee.callee_obj_id)
        } else {
            None
        };

        let base = self.stack.len() - argc;
        let cleanup_base = base - cleanup_offset;

        // Capture actual args before mutating the stack (only when needed).
        let actual_args = if needs_arguments {
            Some(self.stack[base..base + argc].to_vec())
        } else {
            None
        };

        // Snapshot rest-param contents BEFORE slot adjust: the resize below
        // may drop excess args (argc > local_count) or pad with Undefined
        // (argc < local_count), either of which would lose the actual rest
        // values. The packed Array is installed after the resize via the
        // rest_args_snapshot path below.
        let rest_args_snapshot: Option<Vec<JsValue>> = if has_rest_param && param_count > 0 {
            let rest_slot_idx = param_count - 1;
            Some(if argc > rest_slot_idx {
                self.stack[base + rest_slot_idx..base + argc].to_vec()
            } else {
                Vec::new()
            })
        } else {
            None
        };

        // Overwrite slots at positions `param_count..argc` with Undefined:
        // these positions are non-param locals in this function, so the
        // excess argument values captured above (in `actual_args`) get
        // discarded from the stack frame itself.
        let clear_end = argc.min(local_count);
        for i in param_count..clear_end {
            self.stack[base + i] = JsValue::Undefined;
        }

        // Adjust stack to exactly local_count slots.
        if argc > local_count {
            self.stack.truncate(base + local_count);
        } else {
            self.stack.resize(base + local_count, JsValue::Undefined);
        }

        // Pack rest args into a fresh Array and install at slot
        // `param_count - 1`. Performed after the resize so the slot is
        // guaranteed to exist (local_count >= param_count by construction
        // in compile_nested_function / compile_arrow_function).
        if let Some(rest_vec) = rest_args_snapshot {
            let arr_id = self.create_array_object(rest_vec);
            let rest_slot_idx = param_count - 1;
            self.stack[base + rest_slot_idx] = JsValue::Object(arr_id);
        }

        let (tdz_bits, tdz_overflow) = CallFrame::tdz_init(local_count);

        // Async function short-circuit: treated as a Promise-wrapping
        // generator.  Build the initial SuspendedFrame, then let the
        // generator-based async driver settle a wrapper Promise as the
        // body yields / returns / throws.
        if is_async {
            let stack_slice: Vec<JsValue> = self.stack.drain(base..).collect();
            self.stack.truncate(cleanup_base);
            let initial_frame = CallFrame {
                func_id: callee.func_id,
                ip: 0,
                base,
                cleanup_base: base,
                upvalue_ids: callee.upvalue_ids,
                local_upvalue_ids: Vec::new(),
                this_value: this,
                exception_handlers: Vec::new(),
                tdz_bits,
                tdz_overflow,
                actual_args,
                new_instance,
                mode,
                home_class,
                kind: FrameKind::Function,
                generator: None,
                pending_completion: None,
            };
            let suspended = super::value::SuspendedFrame {
                frame: initial_frame,
                stack_slice,
                upvalue_slots: Vec::new(),
            };
            match super::natives_generator::make_async_coroutine_and_drive(self, suspended) {
                Ok(promise) => {
                    self.stack.push(promise);
                }
                Err(_e) => {
                    // make_async_coroutine_and_drive settles the wrapper
                    // Promise on throw and returns Ok; any Err here is an
                    // internal bug.  Fall back to pushing undefined so
                    // the caller's stack shape stays valid.
                    self.stack.push(JsValue::Undefined);
                }
            }
            return Ok(());
        }

        // Generator short-circuit: the call returns a Generator object
        // *without* executing the body.  Build an initial SuspendedFrame
        // that `.next()` will later resume, drop the call args from the
        // stack, and push the Generator in place of the call result.
        if is_generator {
            // Take the just-prepared locals as the initial stack slice.
            let stack_slice: Vec<JsValue> = self.stack.drain(base..).collect();
            // Drop the callee (+ receiver for method calls) that are
            // still sitting below `base` on the stack.
            self.stack.truncate(cleanup_base);

            let initial_frame = CallFrame {
                func_id: callee.func_id,
                ip: 0,
                // These two will be rebased on resume — store the original
                // base here for clarity; resume_generator rewrites them.
                base,
                cleanup_base: base,
                upvalue_ids: callee.upvalue_ids,
                local_upvalue_ids: Vec::new(),
                this_value: this,
                exception_handlers: Vec::new(),
                tdz_bits,
                tdz_overflow,
                actual_args,
                new_instance,
                mode,
                home_class,
                kind: FrameKind::Function,
                generator: None, // filled in after Generator alloc below
                pending_completion: None,
            };
            let suspended = super::value::SuspendedFrame {
                frame: initial_frame,
                stack_slice,
                upvalue_slots: Vec::new(),
            };
            let proto = self.generator_prototype;
            let gen_id = self.alloc_object(super::value::Object {
                kind: super::value::ObjectKind::Generator(Box::new(super::value::GeneratorState {
                    status: super::value::GeneratorStatus::SuspendedStart,
                    suspended: Some(suspended),
                    wrapper: None,
                })),
                storage: super::value::PropertyStorage::shaped(super::shape::ROOT_SHAPE),
                prototype: proto,
                extensible: true,
            });
            // Back-link the saved frame to the generator it belongs to.
            if let super::value::ObjectKind::Generator(state) =
                &mut self.get_object_mut(gen_id).kind
            {
                if let Some(susp) = &mut state.suspended {
                    susp.frame.generator = Some(gen_id);
                }
            }
            self.stack.push(JsValue::Object(gen_id));
            return Ok(());
        }

        self.frames.push(CallFrame {
            func_id: callee.func_id,
            ip: 0,
            base,
            upvalue_ids: callee.upvalue_ids,
            local_upvalue_ids: Vec::new(),
            this_value: this,
            exception_handlers: Vec::new(),
            tdz_bits,
            tdz_overflow,
            actual_args,
            cleanup_base,
            new_instance,
            mode,
            home_class,
            kind: FrameKind::Function,
            generator: None,
            pending_completion: None,
        });
        Ok(())
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
            // `Op::Pop` write fires (empty source, or last statement
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

#[cfg(test)]
mod tests {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    use super::super::value::CallMode;
    use super::super::Vm;
    use super::JsValue;

    /// D-17b-r1 Phase 0b/Phase 7 positive assertion: a panic inside
    /// `with_call_mode`'s closure must leave `VmInner.frames` /
    /// `VmInner.stack` at exactly the entry depth before the panic
    /// is re-raised. This replaces the pre-r1
    /// `dispatch_with_construct_entry_pops_on_panic` test (the
    /// `native_construct_stack` it pushed-and-popped no longer
    /// exists) — and crucially, it also covers a pre-r1
    /// frame-stack-stuck regression: `call_internal`'s
    /// `result.is_err()` cleanup gate did not run on a Rust panic
    /// mid-dispatch, leaving the pushed frame stuck (R2 CRIT-1
    /// discovery).
    #[test]
    fn with_call_mode_truncates_frame_stack_on_panic() {
        let mut vm = Vm::new();
        let entry_frames = vm.inner.frames.len();

        // Suppress the intentional panic's stderr trace.
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let result = catch_unwind(AssertUnwindSafe(|| {
            let _: Result<JsValue, _> = vm.inner.with_call_mode(CallMode::Call, |vm, _mode| {
                // Forge a pushed frame, then panic — the helper
                // must truncate the frames Vec back to
                // `entry_frames` before resume_unwind.
                use super::super::value::{CallFrame, FrameKind, FuncId};
                use std::sync::Arc;
                vm.frames.push(CallFrame {
                    func_id: FuncId(0),
                    ip: 0,
                    base: vm.stack.len(),
                    upvalue_ids: Arc::from([] as [super::super::value::UpvalueId; 0]),
                    local_upvalue_ids: Vec::new(),
                    this_value: JsValue::Undefined,
                    exception_handlers: Vec::new(),
                    tdz_bits: 0,
                    tdz_overflow: Box::default(),
                    actual_args: None,
                    cleanup_base: vm.stack.len(),
                    new_instance: None,
                    mode: CallMode::Call,
                    home_class: None,
                    kind: FrameKind::Function,
                    generator: None,
                    pending_completion: None,
                });
                panic!("intentional panic for with_call_mode frame-truncate regression")
            });
        }));

        std::panic::set_hook(prev_hook);

        assert!(result.is_err(), "panic should propagate via resume_unwind");
        assert_eq!(
            vm.inner.frames.len(),
            entry_frames,
            "frames must be truncated back to entry depth despite panic; got len = {}",
            vm.inner.frames.len()
        );
    }

    /// Sibling assertion for the value stack: a panic mid-dispatch
    /// must leave `VmInner.stack` at exactly the entry depth so any
    /// outer dispatch the caller resumes (after the catch_unwind)
    /// sees a clean stack.
    #[test]
    fn with_call_mode_truncates_value_stack_on_panic() {
        let mut vm = Vm::new();
        let entry_stack_depth = vm.inner.stack.len();

        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let result = catch_unwind(AssertUnwindSafe(|| {
            let _: Result<JsValue, _> = vm.inner.with_call_mode(CallMode::Call, |vm, _mode| {
                vm.stack.push(JsValue::Boolean(true));
                vm.stack.push(JsValue::Number(42.0));
                panic!("intentional panic for with_call_mode value-stack-truncate regression")
            });
        }));

        std::panic::set_hook(prev_hook);

        assert!(result.is_err(), "panic should propagate via resume_unwind");
        assert_eq!(
            vm.inner.stack.len(),
            entry_stack_depth,
            "value stack must be truncated back to entry depth despite panic; got len = {}",
            vm.inner.stack.len()
        );
    }

    /// D-17b-r1 panic-path upvalue-safety regression: a Rust panic
    /// mid-dispatch that triggers `with_call_mode`'s frame truncate
    /// must `close_upvalues` on every dropped frame's
    /// `local_upvalue_ids` BEFORE the stack is truncated, so an
    /// Upvalue still in `UpvalueState::Open { frame_base, slot }`
    /// gets its slot value snapshotted as `Closed(value)`. Without
    /// the close, the upvalue would point at a truncated-away stack
    /// region and subsequent reads (via a closure that escaped the
    /// panicked frame and survived the upstream `catch_unwind`)
    /// would return stale slot values or OOB-panic.
    ///
    /// Closed-via-upvalue invariant covers the F1 finding from the
    /// D-17b-r1 code review.
    #[test]
    fn with_call_mode_closes_open_upvalues_on_panic() {
        use super::super::value::{CallFrame, FrameKind, FuncId, Upvalue, UpvalueId, UpvalueState};
        use std::sync::Arc;

        let mut vm = Vm::new();
        let entry_frames = vm.inner.frames.len();
        let entry_stack_depth = vm.inner.stack.len();

        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        // Stash an UpvalueId before the panic so we can read its
        // post-panic state after with_call_mode runs.
        let captured_uv_id: UpvalueId = {
            let id = UpvalueId(vm.inner.upvalues.len() as u32);
            vm.inner.upvalues.push(Upvalue {
                state: UpvalueState::Open {
                    frame_base: entry_stack_depth,
                    slot: 0,
                },
            });
            id
        };

        let result = catch_unwind(AssertUnwindSafe(|| {
            let _: Result<JsValue, _> = vm.inner.with_call_mode(CallMode::Call, |vm, _mode| {
                // Push the slot the upvalue references onto the value
                // stack BEFORE pushing the frame that owns it.
                let captured_value = JsValue::Number(123.0);
                vm.stack.push(captured_value);
                // Push a forged frame whose `local_upvalue_ids` lists
                // the upvalue captured above; with_call_mode must
                // close it before stack.truncate drops slot at
                // `entry_stack_depth`.
                vm.frames.push(CallFrame {
                    func_id: FuncId(0),
                    ip: 0,
                    base: entry_stack_depth + 1,
                    upvalue_ids: Arc::from([] as [UpvalueId; 0]),
                    local_upvalue_ids: vec![captured_uv_id],
                    this_value: JsValue::Undefined,
                    exception_handlers: Vec::new(),
                    tdz_bits: 0,
                    tdz_overflow: Box::default(),
                    actual_args: None,
                    cleanup_base: entry_stack_depth + 1,
                    new_instance: None,
                    mode: CallMode::Call,
                    home_class: None,
                    kind: FrameKind::Function,
                    generator: None,
                    pending_completion: None,
                });
                panic!("intentional panic for close_upvalues regression")
            });
        }));

        std::panic::set_hook(prev_hook);

        assert!(result.is_err(), "panic should propagate via resume_unwind");
        assert_eq!(
            vm.inner.frames.len(),
            entry_frames,
            "frame stack must be truncated"
        );
        assert_eq!(
            vm.inner.stack.len(),
            entry_stack_depth,
            "value stack must be truncated"
        );

        // F1 invariant: the upvalue captured by the truncated frame
        // must be Closed(captured_value), NOT still Open into a
        // now-dropped stack region.
        match vm.inner.upvalues[captured_uv_id.0 as usize].state {
            UpvalueState::Closed(v) => assert_eq!(
                v,
                JsValue::Number(123.0),
                "upvalue must snapshot the pre-truncate slot value"
            ),
            UpvalueState::Open { frame_base, slot } => panic!(
                "upvalue still Open after panic-truncate: frame_base={frame_base}, slot={slot}"
            ),
        }
    }

    /// D-17b-r1 Copilot R2 + R3: `with_call_mode` saves
    /// `gc_enabled`, `active_bound_key`, and `completion_value` at
    /// entry and restores all three on every exit path (including
    /// panic), so a panicked dispatch that skips the inner per-arm
    /// restores (NativeFunction arm's `gc_enabled` / `bound_key`,
    /// `call_internal`'s `completion_value`) does NOT leak
    /// `gc_enabled = false` (which would silently disable GC for
    /// the VM's remaining lifetime), a stale `active_bound_key`
    /// (which would mis-attribute bound accessors), or a stale
    /// `completion_value` (which would corrupt the caller's
    /// evaluation state across a nested dispatch like
    /// `construct_synchronous`'s JS branch).
    #[test]
    fn with_call_mode_restores_vm_flags_on_panic() {
        use super::super::value::StringId;

        let mut vm = Vm::new();
        let saved_gc = true;
        let saved_key: Option<StringId> = None;
        let saved_completion = JsValue::Number(42.0);
        vm.inner.gc_enabled = saved_gc;
        vm.inner.active_bound_key = saved_key;
        vm.inner.completion_value = saved_completion;

        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let result = catch_unwind(AssertUnwindSafe(|| {
            let _: Result<JsValue, _> = vm.inner.with_call_mode(CallMode::Call, |vm, _mode| {
                // Simulate the mid-body state inner save/restore
                // patterns set up: gc_enabled flipped to false,
                // active_bound_key set to a forged sentinel, and
                // completion_value advanced to a different value —
                // then panic before any inner restore fires.
                vm.gc_enabled = false;
                vm.active_bound_key = Some(super::super::value::StringId(u32::MAX));
                vm.completion_value = JsValue::Number(-1.0);
                panic!("intentional panic for with_call_mode flag-restore regression")
            });
        }));

        std::panic::set_hook(prev_hook);

        assert!(result.is_err(), "panic should propagate via resume_unwind");
        assert_eq!(
            vm.inner.gc_enabled, saved_gc,
            "gc_enabled must be restored to entry value despite panic"
        );
        assert_eq!(
            vm.inner.active_bound_key, saved_key,
            "active_bound_key must be restored to entry value despite panic"
        );
        assert_eq!(
            vm.inner.completion_value, saved_completion,
            "completion_value must be restored to entry value despite panic"
        );
    }

    /// Round-trip: a successful (non-panicking) dispatch under the
    /// helper leaves the frame + value stacks at exactly the entry
    /// depth — the `Ok`-arm `debug_assert_eq` enforces push/pop
    /// balance, and `truncate` is a no-op.
    #[test]
    fn with_call_mode_round_trip_ok() {
        let mut vm = Vm::new();
        let frames_before = vm.inner.frames.len();
        let stack_before = vm.inner.stack.len();

        let result = vm.inner.with_call_mode(CallMode::Call, |_vm, _mode| {
            // Closure does no work; helper bookkeeping should balance.
            Ok(JsValue::Undefined)
        });

        assert!(result.is_ok());
        assert_eq!(
            vm.inner.frames.len(),
            frames_before,
            "frames length must round-trip"
        );
        assert_eq!(
            vm.inner.stack.len(),
            stack_before,
            "value stack length must round-trip"
        );
    }

    /// D-17b R19 spirit preserved: when the closure panics, the
    /// helper must surface the closure's original panic payload
    /// rather than double-panic from a debug-assert. Pre-D-17b-r1
    /// the assert was gated on `!std::thread::panicking()`, which
    /// `catch_unwind` made always-false on the panic path; a
    /// mismatched pop then aborted the thread with an "expected vs
    /// got" message, clobbering the upstream-observable failure
    /// shape. The r1 helper splits its frames-length assertion by
    /// result arm (strict `==` on `Ok`, weaker `>=` on `Err`) and
    /// truncates either way before `resume_unwind` — so even when a
    /// closure leaves the frames Vec in an unbalanced state, the
    /// original panic still surfaces.
    #[test]
    fn with_call_mode_panic_preserves_original_payload() {
        let mut vm = Vm::new();
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let result = catch_unwind(AssertUnwindSafe(|| {
            let _: Result<JsValue, _> = vm.inner.with_call_mode(CallMode::Call, |_vm, _mode| {
                panic!("R19_original_marker");
            });
        }));

        std::panic::set_hook(prev_hook);

        let payload = result.expect_err("closure panicked; helper must re-raise");
        let msg = payload
            .downcast_ref::<&'static str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("<non-string payload>");
        assert_eq!(
            msg, "R19_original_marker",
            "original panic must survive; got {msg:?}"
        );
    }
}
