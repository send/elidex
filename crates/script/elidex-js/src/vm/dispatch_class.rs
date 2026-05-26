//! Class-related opcode handlers: `Op::SuperCall`, `Op::SuperCallSpread`,
//! `Op::NewTarget`, plus the synchronous [[Construct]] helper
//! (`construct_synchronous`) used to invoke a constructor via
//! `super(...)`.
//!
//! Spec references via D-17b §0.5 citation table:
//! - \[C11\] ECMA-262 §10.2.2 `[[Construct]]` semantics — new_target
//!   parameter discipline drives the per-invocation NewTarget rather
//!   than a global flag.
//! - \[C13\] ECMA-262 §13.3.7.1 SuperCall evaluation — invokes the
//!   super constructor with the **outer execution context's
//!   NewTarget** unchanged (propagation invariant).
//! - \[C19\] ECMA-262 §13.3.8.1 ArgumentListEvaluation (spread variant)
//!   — `Op::SuperCallSpread` consumes a pre-built args array.

use super::value::{CallMode, JsValue, ObjectId, ObjectKind, VmError};
use super::VmInner;

impl VmInner {
    /// Read `new.target` from the topmost JS call frame (\[C11\]
    /// step 4 — propagated from the constructor that started the
    /// `new` chain). `Op::NewTarget` handler.
    ///
    /// Returns `JsValue::Object(new_target)` when inside a `new`
    /// invocation, else `JsValue::Undefined` (per ECMA-262 §9.4.5
    /// `GetNewTarget`, which reads the active Function Environment
    /// Record's `[[NewTarget]]` slot — §9.1.1.3).
    #[inline]
    pub(super) fn op_new_target(&mut self) {
        let frame_idx = self.frames.len() - 1;
        let val = match self.frames[frame_idx].mode {
            CallMode::Construct { new_target } => JsValue::Object(new_target),
            CallMode::Call => JsValue::Undefined,
        };
        self.stack.push(val);
    }

    /// `Op::SuperCall <argc>` — `super(...)` invocation in a derived
    /// class constructor body. Pops `argc` args from the stack,
    /// resolves the super class via `home_class.[[Prototype]]`
    /// (\[C13\] GetSuperConstructor), dispatches `[[Construct]]` with
    /// the outer frame's `new_target` propagated unchanged, and
    /// substitutes the current frame's `this_value` + `new_instance`
    /// to the returned object so subsequent reads see the
    /// super-initialized receiver.
    pub(super) fn op_super_call(
        &mut self,
        argc: u8,
        entry_frame_depth: usize,
    ) -> Result<(), VmError> {
        let argc = argc as usize;
        // Snapshot args from the stack before any error path so the
        // stack stays in a consistent shape on TypeError throw.
        if self.stack.len() < argc {
            return Err(VmError::internal("super call: stack underflow"));
        }
        let drain_start = self.stack.len() - argc;
        let call_args: Vec<JsValue> = self.stack.drain(drain_start..).collect();
        self.dispatch_super(&call_args, entry_frame_depth)
    }

    /// `Op::SuperCallSpread` — `super(...args)` invocation. Pops the
    /// args array from the stack (\[C19\] ArgumentListEvaluation
    /// spread variant), then dispatches identically to
    /// [`Self::op_super_call`]. The Array shape is guaranteed by the
    /// compiler (super-spread emits `CreateArray; ArraySpread x;
    /// SuperCallSpread` — `ArraySpread` already runs the iterator
    /// protocol per ECMA-262 §13.3.8.1, so by the time this op
    /// fires the TOS is a real `ObjectKind::Array`); the
    /// non-Array branch below is defensive only and would surface
    /// a compiler-emit bug rather than user error.
    pub(super) fn op_super_call_spread(&mut self, entry_frame_depth: usize) -> Result<(), VmError> {
        let args_value = self.pop()?;
        let call_args = match args_value {
            JsValue::Object(arr_id) => match &self.get_object(arr_id).kind {
                ObjectKind::Array { elements } => elements.clone(),
                _ => {
                    return self.throw_error(
                        VmError::internal(
                            "Op::SuperCallSpread received non-Array (compiler invariant violated)",
                        ),
                        entry_frame_depth,
                    );
                }
            },
            _ => {
                return self.throw_error(
                    VmError::internal(
                        "Op::SuperCallSpread received non-Object (compiler invariant violated)",
                    ),
                    entry_frame_depth,
                );
            }
        };
        self.dispatch_super(&call_args, entry_frame_depth)
    }

    /// Shared dispatch core for `Op::SuperCall` and `Op::SuperCallSpread`
    /// — resolves the super class, validates the frame context,
    /// invokes `[[Construct]]`, then substitutes the receiver in the
    /// outer frame. Routes its own `Err` through `throw_error` so the
    /// two opcode handlers stay one-line wrappers (One-issue-one-way:
    /// only this fn owns the exception-throw fan-out).
    fn dispatch_super(
        &mut self,
        args: &[JsValue],
        entry_frame_depth: usize,
    ) -> Result<(), VmError> {
        match self.dispatch_super_inner(args) {
            Ok(()) => Ok(()),
            Err(e) => self.throw_error(e, entry_frame_depth),
        }
    }

    fn dispatch_super_inner(&mut self, args: &[JsValue]) -> Result<(), VmError> {
        let frame_idx = self.frames.len() - 1;
        let home = self.frames[frame_idx]
            .home_class
            .ok_or_else(|| VmError::syntax_error("'super' keyword unexpected here"))?;
        // Resolve super class = home.[[Prototype]] (§13.3.7.2 GetSuperConstructor).
        let super_class = self
            .get_object(home)
            .prototype
            .ok_or_else(|| VmError::type_error("Super constructor null is not a constructor"))?;
        // Propagate the outer frame's new_target unchanged — \[C13\]
        // SuperCall "GetNewTarget" returns the outermost-invoked
        // class regardless of nesting. Derived-ctor frames carry
        // `CallMode::Construct { new_target }` by construction (the
        // `Call` fallback is defensive only — a `super()` reached
        // via a non-ctor frame would already be a SyntaxError at the
        // `home_class` check above).
        let new_target = match self.frames[frame_idx].mode {
            CallMode::Construct { new_target } => new_target,
            CallMode::Call => home,
        };
        let receiver = self.frames[frame_idx].this_value;
        let pre_alloc = self.frames[frame_idx].new_instance;

        let result = self.construct_synchronous(
            super_class,
            receiver,
            args,
            CallMode::Construct { new_target },
            pre_alloc,
        )?;

        // Substitute the outer frame's `this`/`new_instance` to the
        // super-returned object so subsequent reads (e.g.
        // `this.tagName` after `super()`) see the spec receiver.
        let frame_idx = self.frames.len() - 1;
        if let JsValue::Object(id) = result {
            self.frames[frame_idx].this_value = result;
            self.frames[frame_idx].new_instance = Some(id);
        }

        self.stack.push(result);
        Ok(())
    }

    /// Unwrap a BoundFunction chain on `ctor_id` (ECMA-262 §10.4.1.2
    /// Bound Function Exotic Objects `[[Construct]]`). Returns the innermost
    /// non-bound callable's `ObjectId` plus the bound-args segments
    /// concatenated in innermost→outermost order, ready to splice in
    /// front of the user's call args.
    ///
    /// Returns `Err(RangeError)` when the chain exceeds
    /// [`super::MAX_BIND_CHAIN_DEPTH`]. The chain itself is acyclic
    /// by construction (each `BoundFunction::target` is captured at
    /// bind-time and cannot be mutated), but an attacker can build
    /// arbitrarily long chains.
    ///
    /// Shared SoT for [[Construct]] BoundFunction unwrapping: called
    /// by both [`super::VmInner::do_new`] (the user-visible `new`
    /// path) and [`Self::construct_synchronous`] (the `super(...)` /
    /// invoke_upgrade path). Keeping the logic in one place ensures
    /// `new BoundCtor()` and `class B extends BoundCtor { ... }` /
    /// `super()` behave consistently — D-17b R4 G4-4 fix for the
    /// divergence where `construct_synchronous` rejected BoundFunction
    /// outright.
    pub(crate) fn unwrap_bound_function_chain(
        &self,
        ctor_id: ObjectId,
    ) -> Result<(ObjectId, Vec<JsValue>), VmError> {
        let mut id = ctor_id;
        let mut segments: Vec<Vec<JsValue>> = Vec::new();
        for _ in 0..super::MAX_BIND_CHAIN_DEPTH {
            let ObjectKind::BoundFunction {
                target, bound_args, ..
            } = &self.get_object(id).kind
            else {
                break;
            };
            let next = *target;
            if !bound_args.is_empty() {
                segments.push(bound_args.clone());
            }
            id = next;
        }
        if matches!(self.get_object(id).kind, ObjectKind::BoundFunction { .. }) {
            return Err(VmError::range_error("Maximum bind chain depth exceeded"));
        }
        let total: usize = segments.iter().map(Vec::len).sum();
        let mut prepended: Vec<JsValue> = Vec::with_capacity(total);
        for seg in segments.iter().rev() {
            prepended.extend_from_slice(seg);
        }
        Ok((id, prepended))
    }

    /// Synchronous `[[Construct]]` dispatch (\[C11\]) used by
    /// `Op::SuperCall` and the upcoming `vm.construct` API (Stage 5).
    /// Threads `mode` through both the native-call path (via the
    /// outer `with_call_mode` boundary that bakes `NativeContext::mode`
    /// for the entry frame) and the JS-call path (via the
    /// `push_js_call_frame` `mode` arg → `CallFrame::mode`), then
    /// applies the explicit-Object-return-wins-else-pre-alloc
    /// substitution that matches `[[Construct]]`'s
    /// OrdinaryCreateFromConstructor completion semantics.
    ///
    /// Always invoked with [`CallMode::Construct`] in current callers
    /// (`do_new`'s JS branch, `dispatch_super_inner`); the `mode`
    /// parameter keeps the signature symmetric with
    /// [`super::VmInner::push_js_call_frame`] and primes the path for
    /// a future native-`Function.construct`-via-this-helper caller.
    ///
    /// BoundFunction superclasses (`class B extends A.bind(null) {
    /// ... super() ... }`) are unwrapped via
    /// [`Self::unwrap_bound_function_chain`] so the inner callable is
    /// dispatched with the chain's bound args prepended — matches
    /// `do_new`'s behavior so the two construct paths stay
    /// consistent.
    pub(crate) fn construct_synchronous(
        &mut self,
        ctor_id: ObjectId,
        receiver: JsValue,
        args: &[JsValue],
        mode: CallMode,
        pre_alloc_instance: Option<ObjectId>,
    ) -> Result<JsValue, VmError> {
        // Unwrap BoundFunction chain before checking constructability —
        // a BoundFunction targeting a constructable callable is itself
        // constructable (ECMA-262 §10.4.1.2).
        let (ctor_id, prepended) = self.unwrap_bound_function_chain(ctor_id)?;
        let combined_args: Vec<JsValue> = if prepended.is_empty() {
            args.to_vec()
        } else {
            let mut v = Vec::with_capacity(prepended.len() + args.len());
            v.extend_from_slice(&prepended);
            v.extend_from_slice(args);
            v
        };
        let args = combined_args.as_slice();

        let is_js = matches!(&self.get_object(ctor_id).kind, ObjectKind::Function(_));
        let is_native_ctor = match &self.get_object(ctor_id).kind {
            ObjectKind::NativeFunction(nf) => nf.constructable,
            _ => false,
        };
        if !is_js && !is_native_ctor {
            // Tailor the error for non-constructable natives so the
            // user sees the function's name (matches do_new's
            // "{name} is not a constructor").
            if let ObjectKind::NativeFunction(nf) = &self.get_object(ctor_id).kind {
                let name = self.strings.get_utf8(nf.name);
                return Err(VmError::type_error(format!("{name} is not a constructor")));
            }
            return Err(VmError::type_error("not a constructor"));
        }

        // Thread `mode` through both the JS and native dispatch
        // branches via the outer `with_call_mode` boundary
        // (D-17b-r1 §7.3): the JS branch threads `mode` into
        // `push_js_call_frame` so `CallFrame::mode` matches, and the
        // native branch threads it into `NativeContext::new_construct`
        // so the body's `ctx.is_construct()` / `ctx.new_target()` see
        // the right context. Single SoT for per-invocation construct
        // mode (replaces D-17b §7 `native_construct_stack`).
        let raw_result = self.with_call_mode(mode, |vm, mode| {
            if is_js {
                // JS construct: push frame + re-entrant `run()` until the
                // pushed frame returns. push_js_call_frame consumes
                // [callee, arg0..argN] from the stack (cleanup_offset=1),
                // matching `Op::Call`'s shape — so we lay it out then
                // hand off to the dispatcher. The `with_call_mode`
                // closure binds the catch_unwind boundary so a panic
                // during `run()` is caught and the frame/value stacks
                // are truncated back to the entry depth before
                // re-raise (D-17b-r1 R2 CRIT-1 fix).
                match vm.extract_js_callee(ctor_id) {
                    Some(callee) => {
                        vm.stack.push(JsValue::Object(ctor_id));
                        for &a in args {
                            vm.stack.push(a);
                        }
                        // Construct mode threads `mode` directly; the
                        // class-ctor-call-mode guard never fires when
                        // `mode = CallMode::Construct { .. }`.
                        vm.push_js_call_frame(
                            callee,
                            receiver,
                            args.len(),
                            1,
                            pre_alloc_instance,
                            mode,
                        )?;
                        vm.run()
                    }
                    None => Err(VmError::internal(
                        "construct_synchronous: ObjectKind::Function lost between probe and dispatch",
                    )),
                }
            } else {
                debug_assert!(is_native_ctor);
                // Native construct: dispatch directly inside the
                // boundary so the `NativeContext` built at
                // `call_dispatch` time bakes the outer `mode`.
                vm.call_dispatch(ctor_id, receiver, args, mode)
            }
        });

        let raw = raw_result?;
        // [[Construct]] return: explicit Object wins, else the
        // pre-allocated instance (`new_instance`) — mirrors
        // `complete_inline_frame`'s substitution.
        let final_val = if matches!(raw, JsValue::Object(_)) {
            raw
        } else if let Some(id) = pre_alloc_instance {
            JsValue::Object(id)
        } else {
            raw
        };
        Ok(final_val)
    }
}
