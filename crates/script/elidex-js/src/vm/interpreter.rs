//! Bytecode interpreter: public API for eval/call.
//!
//! The main dispatch loop lives in `dispatch.rs`; this module provides the
//! entry points (`eval`, `run_script`, `call`) and shared helpers.

use crate::bytecode::compiled::CompiledScript;

use super::value::{
    CallFrame, FuncId, JsValue, NativeContext, ObjectId, ObjectKind, VmError, VmErrorKind,
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
                    super::value::ThisMode::Global => {
                        // §9.2.1.2 OrdinaryCallBindThis:
                        // Step 5: undefined/null → globalThis
                        // Step 6.b.ii: primitive → ToObject wrapper
                        match this {
                            JsValue::Undefined | JsValue::Null => {
                                JsValue::Object(self.global_object)
                            }
                            JsValue::Number(n) => {
                                let wrapper = self.alloc_object(super::value::Object {
                                    kind: ObjectKind::NumberWrapper(n),
                                    properties: Vec::new(),
                                    prototype: self.number_prototype,
                                });
                                JsValue::Object(wrapper)
                            }
                            JsValue::String(s) => {
                                let wrapper = self.alloc_object(super::value::Object {
                                    kind: ObjectKind::StringWrapper(s),
                                    properties: Vec::new(),
                                    prototype: self.string_prototype,
                                });
                                JsValue::Object(wrapper)
                            }
                            JsValue::Boolean(b) => {
                                let wrapper = self.alloc_object(super::value::Object {
                                    kind: ObjectKind::BooleanWrapper(b),
                                    properties: Vec::new(),
                                    prototype: self.boolean_prototype,
                                });
                                JsValue::Object(wrapper)
                            }
                            _ => this,
                        }
                    }
                    super::value::ThisMode::Strict => this,
                };
                self.call_internal(func_id, effective_this, args, upvalue_ids)
            }
            ObjectKind::NativeFunction(nf) => {
                let func = nf.func;
                let mut ctx = NativeContext { vm: self };
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

        self.frames.push(CallFrame {
            func_id,
            ip: 0,
            base,
            upvalue_ids,
            local_upvalue_ids: Vec::new(),
            this_value: this,
            exception_handlers: Vec::new(),
            tdz_slots: vec![true; local_count],
            actual_args: if needs_arguments {
                Some(args.to_vec())
            } else {
                None
            },
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
