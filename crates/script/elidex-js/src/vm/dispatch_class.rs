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

use super::value::{JsValue, ObjectId, ObjectKind, VmError};
use super::VmInner;

impl VmInner {
    /// Read `new.target` from the topmost JS call frame (\[C11\]
    /// step 4 — propagated from the constructor that started the
    /// `new` chain). `Op::NewTarget` handler.
    ///
    /// Returns `JsValue::Object(new_target)` when inside a `new`
    /// invocation, else `JsValue::Undefined` (per ECMA-262 §9.1.1.2
    /// `[[GetThisBinding]]` analog for `new.target` in ordinary
    /// function code).
    #[inline]
    pub(super) fn op_new_target(&mut self) {
        let frame_idx = self.frames.len() - 1;
        let val = match self.frames[frame_idx].new_target {
            Some(id) => JsValue::Object(id),
            None => JsValue::Undefined,
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
        // Resolve super class = home.[[Prototype]] (§13.3.7.1 GetSuperConstructor).
        let super_class = self
            .get_object(home)
            .prototype
            .ok_or_else(|| VmError::type_error("Super constructor null is not a constructor"))?;
        // Propagate the outer frame's new_target unchanged — \[C13\]
        // SuperCall "GetNewTarget" returns the outermost-invoked
        // class regardless of nesting.
        let new_target = self.frames[frame_idx].new_target.unwrap_or(home);
        let receiver = self.frames[frame_idx].this_value;
        let pre_alloc = self.frames[frame_idx].new_instance;

        let result =
            self.construct_synchronous(super_class, receiver, args, new_target, pre_alloc)?;

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

    /// Synchronous `[[Construct]]` dispatch (\[C11\]) used by
    /// `Op::SuperCall` and the upcoming `vm.construct` API (Stage 5).
    /// Threads `new_target` through both the native-call path
    /// (via `native_construct_stack`) and the JS-call path (via
    /// `CallFrame::new_target`), then applies the
    /// explicit-Object-return-wins-else-pre-alloc substitution that
    /// matches `[[Construct]]`'s OrdinaryCreateFromConstructor
    /// completion semantics.
    pub(crate) fn construct_synchronous(
        &mut self,
        ctor_id: ObjectId,
        receiver: JsValue,
        args: &[JsValue],
        new_target: ObjectId,
        pre_alloc_instance: Option<ObjectId>,
    ) -> Result<JsValue, VmError> {
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

        // Push the per-invocation `new_target` onto the native-call
        // stack so any native ctor reached by this construct (either
        // directly when ctor is native, or via nested `do_new` /
        // `super` inside the JS body) sees the right NewTarget.
        // Single SoT for native-side construct mode (D-17b §7) — the
        // stack-top read in `NativeContext::is_construct()` +
        // `ensure_instance_or_alloc` derives the same boolean the
        // legacy `in_construct` flag did, plus the new_target ObjectId
        // that flag couldn't express. The native-ctor path delegates
        // its push to `Vm::call_construct_native` so the entry stays
        // on top during the dispatch body (using plain `Vm::call`
        // here would have shadowed our `Some(new_target)` with a
        // `None` and broken `ctx.is_construct()` / `ctx.new_target()`
        // reads inside the native body).
        let raw_result = if is_js {
            self.native_construct_stack.push(Some(new_target));
            // JS construct: push frame + re-entrant `run()` until the
            // pushed frame returns. push_js_call_frame consumes
            // [callee, arg0..argN] from the stack (cleanup_offset=1),
            // matching `Op::Call`'s shape — so we lay it out then
            // hand off to the dispatcher.
            let r = match self.extract_js_callee(ctor_id) {
                Some(callee) => {
                    self.stack.push(JsValue::Object(ctor_id));
                    for &a in args {
                        self.stack.push(a);
                    }
                    // Construct mode (new_target = Some), so the
                    // class-ctor-call-mode guard never fires; map
                    // the `?` shape here for compile parity.
                    if let Err(e) = self.push_js_call_frame(
                        callee,
                        receiver,
                        args.len(),
                        1,
                        pre_alloc_instance,
                        Some(new_target),
                    ) {
                        Err(e)
                    } else {
                        self.run()
                    }
                }
                None => Err(VmError::internal(
                    "construct_synchronous: ObjectKind::Function lost between probe and dispatch",
                )),
            };
            let popped = self.native_construct_stack.pop();
            debug_assert!(
                matches!(popped, Some(Some(_))),
                "construct_synchronous JS path: native_construct_stack push/pop mismatch"
            );
            r
        } else {
            debug_assert!(is_native_ctor);
            // `call_construct_native` owns the push/pop discipline
            // for the native-dispatch boundary.
            self.call_construct_native(ctor_id, receiver, args, new_target)
        };

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
