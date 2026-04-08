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
    pub fn eval(&mut self, source: &str) -> Result<JsValue, VmError> {
        let script = crate::compiler::compile_script(source).map_err(|e| VmError {
            kind: VmErrorKind::CompileError,
            message: e.message,
        })?;
        self.run_script(script)
    }

    /// Load and execute a compiled script.
    pub fn run_script(&mut self, script: CompiledScript) -> Result<JsValue, VmError> {
        let func_id = self.register_function(script.top_level);
        self.run_function(func_id, JsValue::Undefined, &[])
    }

    /// Call a JS function object with the given `this` and arguments.
    pub fn call(
        &mut self,
        func_obj_id: ObjectId,
        this: JsValue,
        args: &[JsValue],
    ) -> Result<JsValue, VmError> {
        let obj = self.get_object(func_obj_id);
        match &obj.kind {
            ObjectKind::Function(fo) => {
                let func_id = fo.func_id;
                let upvalue_ids = fo.upvalue_ids.clone();
                let effective_this = match fo.this_mode {
                    super::value::ThisMode::Lexical => {
                        fo.captured_this.unwrap_or(JsValue::Undefined)
                    }
                    super::value::ThisMode::Global => self.bind_this_global(this),
                    super::value::ThisMode::Strict => this,
                };
                self.call_internal(func_id, effective_this, args, upvalue_ids)
            }
            ObjectKind::NativeFunction(nf) => {
                let func = nf.func;
                // Disable GC during native calls: Rust stack locals holding
                // ObjectId are invisible to the GC root scanner.
                let saved_gc = self.gc_enabled;
                self.gc_enabled = false;
                let mut ctx = NativeContext { vm: self };
                let result = func(&mut ctx, this, args);
                ctx.vm.gc_enabled = saved_gc;
                result
            }
            _ => Err(VmError::type_error("not a function")),
        }
    }

    /// Internal: push a frame and run a compiled function.
    ///
    /// Used by the public `call()` API and `NativeContext` re-entrant calls.
    /// The inline dispatch path uses `push_js_call_frame` instead.
    pub(crate) fn call_internal(
        &mut self,
        func_id: FuncId,
        this: JsValue,
        args: &[JsValue],
        upvalue_ids: Arc<[UpvalueId]>,
    ) -> Result<JsValue, VmError> {
        let compiled = self.get_compiled(func_id);
        let local_count = compiled.local_count as usize;
        let param_count = compiled.param_count as usize;
        let needs_arguments = compiled.needs_arguments;

        let entry_frames = self.frames.len();
        let base = self.stack.len();

        // Allocate locals (initialized to Undefined).
        self.stack.resize(base + local_count, JsValue::Undefined);

        // Copy args into param slots.
        let copy_count = args.len().min(param_count);
        self.stack[base..base + copy_count].copy_from_slice(&args[..copy_count]);

        // Save and reset completion_value so that ReturnUndefined in nested
        // function calls does not leak the parent scope's completion value.
        let saved_completion = self.completion_value;
        self.completion_value = JsValue::Undefined;
        let (tdz_bits, tdz_overflow) = CallFrame::tdz_init(local_count);

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
            saved_completion,
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
    pub(crate) fn push_js_call_frame(
        &mut self,
        callee: JsCalleeInfo,
        this: JsValue,
        argc: usize,
        cleanup_offset: usize,
        new_instance: Option<ObjectId>,
    ) {
        let base = self.stack.len() - argc;
        let cleanup_base = base - cleanup_offset;
        let compiled = self.get_compiled(callee.func_id);
        let local_count = compiled.local_count as usize;
        let param_count = compiled.param_count as usize;
        let needs_arguments = compiled.needs_arguments;

        // Capture actual args before mutating the stack (only when needed).
        let actual_args = if needs_arguments {
            Some(self.stack[base..base + argc].to_vec())
        } else {
            None
        };

        // Clear extra arg slots beyond param_count (they should be
        // non-param locals, initialized to Undefined).
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

        let saved_completion = self.completion_value;
        self.completion_value = JsValue::Undefined;
        let (tdz_bits, tdz_overflow) = CallFrame::tdz_init(local_count);

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
            saved_completion,
        });
    }

    /// §9.2.1.2 OrdinaryCallBindThis for Global (non-strict) this mode:
    /// undefined/null → globalThis, primitives → ToObject wrapper.
    pub(crate) fn bind_this_global(&mut self, this: JsValue) -> JsValue {
        match this {
            JsValue::Undefined | JsValue::Null => JsValue::Object(self.global_object),
            JsValue::Number(n) => {
                let wrapper = self.alloc_object(super::value::Object {
                    kind: ObjectKind::NumberWrapper(n),
                    storage: super::value::PropertyStorage::shaped(super::shape::ROOT_SHAPE),
                    prototype: self.number_prototype,
                });
                JsValue::Object(wrapper)
            }
            JsValue::String(s) => {
                let wrapper = self.alloc_object(super::value::Object {
                    kind: ObjectKind::StringWrapper(s),
                    storage: super::value::PropertyStorage::shaped(super::shape::ROOT_SHAPE),
                    prototype: self.string_prototype,
                });
                JsValue::Object(wrapper)
            }
            JsValue::Boolean(b) => {
                let wrapper = self.alloc_object(super::value::Object {
                    kind: ObjectKind::BooleanWrapper(b),
                    storage: super::value::PropertyStorage::shaped(super::shape::ROOT_SHAPE),
                    prototype: self.boolean_prototype,
                });
                JsValue::Object(wrapper)
            }
            _ => this,
        }
    }

    /// Run a function as the initial (or only) frame.
    fn run_function(
        &mut self,
        func_id: FuncId,
        this: JsValue,
        args: &[JsValue],
    ) -> Result<JsValue, VmError> {
        self.call_internal(func_id, this, args, Arc::from([]))
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
