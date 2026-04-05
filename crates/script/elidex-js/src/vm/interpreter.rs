//! Bytecode interpreter: public API for eval/call.
//!
//! The main dispatch loop lives in `dispatch.rs`; this module provides the
//! entry points (`eval`, `run_script`, `call`) and shared helpers.

use crate::bytecode::compiled::CompiledScript;

use super::value::{
    CallFrame, FuncId, JsValue, NativeContext, ObjectId, ObjectKind, VmError, VmErrorKind,
};
use super::Vm;

// ---------------------------------------------------------------------------
// Vm public API
// ---------------------------------------------------------------------------

impl Vm {
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
                    super::value::ThisMode::Global => {
                        // §9.2.1.2: non-strict functions coerce undefined/null
                        // this to the global object.
                        if matches!(this, JsValue::Undefined | JsValue::Null) {
                            JsValue::Object(self.inner.global_object)
                        } else {
                            this
                        }
                    }
                    super::value::ThisMode::Strict => this,
                };
                self.call_internal(func_id, effective_this, args, upvalue_ids)
            }
            ObjectKind::NativeFunction(nf) => {
                let func = nf.func;
                let mut ctx = NativeContext {
                    vm: &mut self.inner,
                };
                func(&mut ctx, this, args)
            }
            _ => Err(VmError::type_error("not a function")),
        }
    }

    /// Internal: push a frame and run a compiled function.
    fn call_internal(
        &mut self,
        func_id: FuncId,
        this: JsValue,
        args: &[JsValue],
        upvalue_ids: Vec<super::value::UpvalueId>,
    ) -> Result<JsValue, VmError> {
        let compiled = self.get_compiled(func_id);
        let local_count = compiled.local_count as usize;
        let param_count = compiled.param_count as usize;

        let base = self.inner.stack.len();

        // Allocate locals (initialized to Undefined).
        self.inner
            .stack
            .resize(base + local_count, JsValue::Undefined);

        // Copy args into param slots.
        let copy_count = args.len().min(param_count);
        self.inner.stack[base..base + copy_count].copy_from_slice(&args[..copy_count]);

        // Save and reset completion_value so that ReturnUndefined in nested
        // function calls does not leak the parent scope's completion value.
        let saved_completion = self.inner.completion_value;
        self.inner.completion_value = JsValue::Undefined;

        self.inner.frames.push(CallFrame {
            func_id,
            ip: 0,
            base,
            upvalue_ids,
            local_upvalue_ids: Vec::new(),
            this_value: this,
            exception_handlers: Vec::new(),
            tdz_slots: vec![true; local_count],
        });

        let result = self.run();

        // Restore the parent scope's completion value.
        self.inner.completion_value = saved_completion;

        result
    }

    /// Run a function as the initial (or only) frame.
    fn run_function(
        &mut self,
        func_id: FuncId,
        this: JsValue,
        args: &[JsValue],
    ) -> Result<JsValue, VmError> {
        self.call_internal(func_id, this, args, Vec::new())
    }
}

// ---------------------------------------------------------------------------
// Shared stack helpers (used by dispatch.rs and ops.rs)
// ---------------------------------------------------------------------------

impl Vm {
    pub(crate) fn pop(&mut self) -> Result<JsValue, VmError> {
        self.inner
            .stack
            .pop()
            .ok_or_else(|| VmError::internal("stack underflow"))
    }

    pub(crate) fn peek(&self) -> Result<JsValue, VmError> {
        self.inner
            .stack
            .last()
            .copied()
            .ok_or_else(|| VmError::internal("stack underflow on peek"))
    }
}
