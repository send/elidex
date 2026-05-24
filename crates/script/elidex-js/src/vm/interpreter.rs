//! Bytecode interpreter: public API for eval/call.
//!
//! The main dispatch loop lives in `dispatch.rs`; this module provides the
//! entry points (`eval`, `run_script`, `call`) and shared helpers.

use crate::bytecode::compiled::CompiledScript;

use std::sync::Arc;

use super::value::{
    CallFrame, FuncId, JsCalleeInfo, JsValue, NativeContext, ObjectId, ObjectKind, UpvalueId,
    VmError, VmErrorKind,
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
    /// Marks the dispatch boundary with a `None` entry on
    /// [`VmInner::native_construct_stack`] for the duration of the
    /// call (D-17b §7 SoT — `None` = call mode, `Some(new_target)` =
    /// construct mode pushed by `do_new`'s native-ctor branch or
    /// `construct_synchronous`). This is what makes
    /// `NativeContext::is_construct()` return `false` for a
    /// `[[Call]]`-mode invocation nested inside an outer construct
    /// chain (e.g. `Error.call(this)` inside a CE class ctor body —
    /// the outer CE-upgrade's `Some(constructor)` is shadowed by
    /// this `None` for the nested native ctor's lifetime, preventing
    /// wrapper-receiver pollution that the global `in_construct`
    /// flag could not avoid). Construct-mode callers MUST use
    /// [`Self::call_construct_native`] instead so the native body's
    /// `is_construct()` / `new_target()` reads see the right
    /// invocation context.
    pub fn call(
        &mut self,
        func_obj_id: ObjectId,
        this: JsValue,
        args: &[JsValue],
    ) -> Result<JsValue, VmError> {
        self.native_construct_stack.push(None);
        let result = self.call_dispatch(func_obj_id, this, args);
        let popped = self.native_construct_stack.pop();
        debug_assert!(
            matches!(popped, Some(None)),
            "Vm::call native_construct_stack push/pop mismatch (saw {popped:?})"
        );
        result
    }

    /// `[[Construct]]`-mode counterpart to [`Self::call`] for native
    /// constructor dispatch via `do_new`'s native-ctor branch. Pushes
    /// `Some(new_target)` onto `native_construct_stack` for the
    /// duration of the dispatch so the native body's
    /// `ctx.is_construct()` / `ctx.new_target()` reads see the right
    /// construct context. Otherwise identical to `Vm::call`.
    pub(crate) fn call_construct_native(
        &mut self,
        func_obj_id: ObjectId,
        this: JsValue,
        args: &[JsValue],
        new_target: ObjectId,
    ) -> Result<JsValue, VmError> {
        self.native_construct_stack.push(Some(new_target));
        let result = self.call_dispatch(func_obj_id, this, args);
        let popped = self.native_construct_stack.pop();
        debug_assert!(
            matches!(popped, Some(Some(_))),
            "Vm::call_construct_native native_construct_stack push/pop mismatch (saw {popped:?})"
        );
        result
    }

    #[allow(clippy::too_many_lines)] // dispatch table over every callable ObjectKind variant
    fn call_dispatch(
        &mut self,
        func_obj_id: ObjectId,
        this: JsValue,
        args: &[JsValue],
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
                        Some(current_id),
                    );
                }
                ObjectKind::NativeFunction(nf) => {
                    let func = nf.func;
                    let saved_gc = self.gc_enabled;
                    // Stage the accessor's bound key (re-entrancy: a native may
                    // call another native, so save/restore rather than clear).
                    let saved_bound_key = self.active_bound_key;
                    self.active_bound_key = nf.bound_key;
                    self.gc_enabled = false;
                    let call_args = owned_args.as_deref().unwrap_or(args);
                    let mut ctx = NativeContext { vm: self };
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
                    // ES2020 §25.6.1.3.1 / §25.6.1.3.2: invoking a Promise
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
                _ => return Err(VmError::type_error("not a function")),
            }
        }
    }

    /// Internal: push a frame and run a compiled function.
    ///
    /// Used by the public `call()` API and `NativeContext` re-entrant calls.
    /// The inline dispatch path uses `push_js_call_frame` instead.
    #[allow(clippy::too_many_lines)] // generator + async + regular frame paths
    pub(crate) fn call_internal(
        &mut self,
        func_id: FuncId,
        this: JsValue,
        args: &[JsValue],
        upvalue_ids: Arc<[UpvalueId]>,
        closure_obj_id: Option<ObjectId>,
    ) -> Result<JsValue, VmError> {
        // ECMA-262 §10.2.1 step 2: class constructors throw a
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
        // D-17b §3.1(c) home_class threading: class-ctor frames get
        // `home_class = Some(closure_obj_id)` so `Op::SuperCall`
        // resolves super via `home_class.[[Prototype]]`. Non-class
        // calls + run_function (no closure) leave home_class None.
        let home_class: Option<ObjectId> = if compiled.is_class_ctor {
            closure_obj_id
        } else {
            None
        };
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

        // Save and reset completion_value so that ReturnUndefined in nested
        // function calls does not leak the parent scope's completion value.
        let saved_completion = self.completion_value;
        self.completion_value = JsValue::Undefined;
        let (tdz_bits, tdz_overflow) = CallFrame::tdz_init(local_count);

        // Async function re-entry via `call()`: same treatment as the
        // inline dispatch path — build an initial SuspendedFrame and
        // drive one step, returning the wrapper Promise.
        if is_async {
            let stack_slice: Vec<JsValue> = self.stack.drain(base..).collect();
            self.completion_value = saved_completion;
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
                new_target: None,
                home_class,
                saved_completion: JsValue::Undefined,
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
            self.completion_value = saved_completion;
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
                new_target: None,
                home_class,
                saved_completion: JsValue::Undefined,
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
            new_target: None,
            home_class,
            saved_completion,
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

        // Restore the parent scope's completion value.
        self.completion_value = saved_completion;

        result
    }

    /// Push a JS function call frame for the single dispatcher.
    ///
    /// Args are already on the stack. `cleanup_offset` is the number of
    /// extra slots below the args (1 for callee, 2 for receiver + callee).
    /// Does **not** call `run()` — the caller must `continue` the dispatch loop.
    ///
    /// Returns `Err` (without mutating the stack or frame state)
    /// when invoked in call mode (`new_target = None`) on a class
    /// constructor — ECMA-262 §10.2.1 step 2 requires throwing a
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
        new_target: Option<ObjectId>,
    ) -> Result<(), VmError> {
        let compiled = self.get_compiled(callee.func_id);
        // ECMA-262 §10.2.1 step 2 — class constructor in call mode is
        // a TypeError. Construct-mode entries (`do_new` /
        // `construct_synchronous`) pass `Some(new_target)` and so
        // bypass this guard.
        if compiled.is_class_ctor && new_target.is_none() {
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
        // — non-ctor super-property reads stay Step-9-deferred).
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

        let saved_completion = self.completion_value;
        let (tdz_bits, tdz_overflow) = CallFrame::tdz_init(local_count);

        // Async function short-circuit: treated as a Promise-wrapping
        // generator.  Build the initial SuspendedFrame, then let the
        // generator-based async driver settle a wrapper Promise as the
        // body yields / returns / throws.
        if is_async {
            let stack_slice: Vec<JsValue> = self.stack.drain(base..).collect();
            self.completion_value = saved_completion;
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
                new_target,
                home_class,
                saved_completion: JsValue::Undefined,
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
            // Restore caller's completion_value — we never started the body.
            self.completion_value = saved_completion;
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
                new_target,
                home_class,
                saved_completion: JsValue::Undefined,
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

        self.completion_value = JsValue::Undefined;
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
            new_target,
            home_class,
            saved_completion,
            generator: None,
            pending_completion: None,
        });
        Ok(())
    }

    /// Run a function as the initial (or only) frame.
    fn run_function(
        &mut self,
        func_id: FuncId,
        this: JsValue,
        args: &[JsValue],
    ) -> Result<JsValue, VmError> {
        self.call_internal(func_id, this, args, Arc::from([]), None)
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
