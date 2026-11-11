//! Interpreter frame state: upvalues, call frames, frame kinds and
//! exception-handler entries.

use std::sync::Arc;

use super::coroutine_types::FrameCompletion;
use super::value::{CallMode, FuncId, JsValue, ObjectId, UpvalueId};

// ---------------------------------------------------------------------------
// Upvalue
// ---------------------------------------------------------------------------

/// An upvalue: a captured variable that may still be on the stack (Open)
/// or has been moved to the heap (Closed) when its frame was popped.
#[derive(Clone, Debug)]
pub struct Upvalue {
    pub state: UpvalueState,
}

/// Whether an upvalue refers to a live stack slot or a captured value.
#[derive(Clone, Debug)]
pub enum UpvalueState {
    /// The variable is still on the stack at `stack[frame_base + slot]`.
    Open { frame_base: usize, slot: u16 },
    /// The variable was captured when the frame was popped.
    Closed(JsValue),
}

// ---------------------------------------------------------------------------
// CallFrame
// ---------------------------------------------------------------------------

/// Frame-level discriminator separating script/`eval` completion-value
/// semantics (ECMA-262 §16.1.6 ScriptEvaluation step 13.a + 13.b + 17 /
/// §19.2.1.1 PerformEval step 29.a + 30.a + 33) from function/ctor body
/// return semantics (§10.2.1 `[[Call]]` step 8 / §10.2.2 `[[Construct]]`
/// step 11-13/15-17, with the body completion modeled by §10.2.1.4
/// OrdinaryCallEvaluateBody + §15.2.3 EvaluateFunctionBody step 4
/// `ReturnCompletion(undefined)`).
///
/// Frames with `kind = Eval` write the last ExpressionStatement value to
/// `VmInner::completion_value` via the entry-gated `Op::PopCompletion` arm and
/// return that value on implicit fall-through; frames with
/// `kind = Function` never touch `completion_value` (Op::PopCompletion discards,
/// Op::ReturnUndefined returns literal Undefined). The type-level split
/// dissolves the pre-D-17b-r2 `is_class_ctor` carve-out and the
/// `CallFrame::saved_completion` save/restore plumbing.
///
/// Generators and async function bodies remain `Function` — their
/// machinery consumes the body's completion elsewhere (§27.8.3.1
/// GeneratorStart step 4.i-4.l / §27.10.5.2 AsyncBlockStart step
/// 1.f-1.h) and does not observe `completion_value`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameKind {
    /// Top-level script body or `eval` body — the entry-gated `Op::PopCompletion`
    /// captures the last ExpressionStatement value into
    /// `completion_value`, returned on implicit fall-through.
    Eval,
    /// Function / method / class-ctor / generator / async body —
    /// `completion_value` is invariant across the frame's run.
    Function,
}

/// A single call frame on the VM's call stack.
pub struct CallFrame {
    /// The compiled function being executed.
    pub func_id: FuncId,
    /// Instruction pointer (byte offset into bytecode).
    pub ip: usize,
    /// Stack base: `stack[base..base+local_count]` are this frame's locals.
    pub base: usize,
    /// Upvalue handles for this invocation (shared via `Arc`).
    pub upvalue_ids: Arc<[UpvalueId]>,
    /// Upvalues that capture *this* frame's local slots (closed on frame pop).
    pub local_upvalue_ids: Vec<UpvalueId>,
    /// The `this` value for this call.
    pub this_value: JsValue,
    /// Active exception handlers (try/catch/finally).
    pub exception_handlers: Vec<HandlerEntry>,
    /// Bit-packed TDZ tracking. Bit N set = slot N is uninitialized (in TDZ).
    /// Inline word covers slots 0–63 with no heap allocation.
    pub tdz_bits: u64,
    /// Extended TDZ bits for functions with > 64 locals.  An empty
    /// `Box<[u64]>` (no backing allocation) when local_count ≤ 64.
    pub tdz_overflow: Box<[u64]>,
    /// Actual arguments passed to this call (for `arguments` object creation).
    /// Only populated when the compiled function has a `CreateArguments` opcode.
    pub actual_args: Option<Vec<JsValue>>,
    /// Stack position to truncate to on return (accounts for callee/receiver
    /// slots below `base` that the caller left on the stack).
    pub cleanup_base: usize,
    /// For `new` calls: the constructed instance to return if the constructor
    /// does not return an object. Not ECMAScript `new.target` (which refers
    /// to the constructor function).
    pub new_instance: Option<ObjectId>,
    /// `[[Call]]` vs `[[Construct]]` discipline for this frame
    /// (ECMA-262 §10.2.1 / §10.2.2). Replaces the pre-D-17b-r1
    /// `new_target: Option<ObjectId>` field: encoding
    /// "construct iff new_target is set" inside [`CallMode::Construct`]'s
    /// data makes the invariant type-level. Propagated unchanged
    /// through nested `super()` invocations so `Op::NewTarget` always
    /// reads the outermost-invoked class (\[C13\] §13.3.7.1 step 1).
    /// Distinct from `new_instance`: `new_instance` is the
    /// pre-allocated receiver hint; `mode`'s `Construct.new_target`
    /// is the spec-level constructor function.
    pub mode: CallMode,
    /// Enclosing class for `super()` resolution (\[C13\] §13.3.7.2
    /// GetSuperConstructor — `super.[[Prototype]]` is the super
    /// class). Set on class-ctor frames only; `None` for ordinary
    /// function frames and non-ctor methods (CE-minimal scope per
    /// D-17b §3.4 — `Op::GetSuperProp` support is Step-9-deferred,
    /// defer slot `#11-step9-class-extras`). The CE-minimal contract is
    /// fail-closed-by-construction: a non-ctor method frame has
    /// `home_class = None`, so any future super-property reader
    /// trips a SyntaxError fallback rather than consuming wrong data.
    pub home_class: Option<ObjectId>,
    /// Frame-kind discriminator (Eval / Function); see [`FrameKind`].
    /// Drives the entry-frame `Op::PopCompletion` gate and `Op::ReturnUndefined`
    /// branch so script/`eval` completion-value semantics stay confined
    /// to `Eval` frames; the only `Eval` push site is the private
    /// `VmInner::run_function`, reached via the public `Vm::eval` /
    /// `Vm::run_script` host entry points. elidex-js core does not
    /// expose a JS-level `eval()` global (per design doc §14.1
    /// strict-only baseline at `docs/design/ja/14-script-engines-webapi.md`),
    /// so all `FrameKind::Eval` frames originate at the Rust-level
    /// host boundary; sloppy direct-eval is LegacySemantics compat-
    /// plugin territory and would not need its own `FrameKind`
    /// variant (same dispatch shape, plugin-level scope injection).
    pub kind: FrameKind,
    /// If set, this frame belongs to a generator; `Op::Yield` suspends
    /// into this generator object instead of completing normally.  `None`
    /// for ordinary (non-generator) frames.
    pub generator: Option<ObjectId>,
    /// Pending abrupt completion for `Op::EndFinally` at the tail of a
    /// finally body.  Set when jumping into finally via an externally
    /// injected abrupt completion (e.g. `Generator.prototype.return`);
    /// consulted by `Op::EndFinally` to resume that completion once the
    /// finally body finishes.  `None` for normal control flow.
    ///
    /// Boxed so `CallFrame` stays pointer-sized for the field — the
    /// common case is `None` on every call, and the inline 24-byte
    /// `Option<FrameCompletion>` would transitively push
    /// `ObjectKind::Generator` past the `large_enum_variant` limit.
    /// The heap allocation only fires on `.return()` / `.throw()`
    /// injection or a finally cascade (cold paths).
    pub pending_completion: Option<Box<FrameCompletion>>,
}

impl CallFrame {
    /// Initialize TDZ bits for `local_count` locals (all slots start in TDZ).
    pub fn tdz_init(local_count: usize) -> (u64, Box<[u64]>) {
        let bits = if local_count == 0 {
            0
        } else if local_count >= 64 {
            u64::MAX
        } else {
            (1u64 << local_count) - 1
        };
        let overflow = if local_count <= 64 {
            Box::default()
        } else {
            let overflow_bits = local_count - 64;
            let mut v = vec![u64::MAX; overflow_bits.div_ceil(64)];
            let remainder = overflow_bits % 64;
            if remainder != 0 {
                let last = v.len() - 1;
                v[last] = (1u64 << remainder) - 1;
            }
            v.into_boxed_slice()
        };
        (bits, overflow)
    }

    /// Check whether `slot` is in the temporal dead zone.
    #[inline]
    pub fn is_in_tdz(&self, slot: usize) -> bool {
        if slot < 64 {
            self.tdz_bits & (1u64 << slot) != 0
        } else {
            let adj = slot - 64;
            self.tdz_overflow
                .get(adj / 64)
                .is_some_and(|w| w & (1u64 << (adj % 64)) != 0)
        }
    }

    /// Clear the TDZ flag for `slot` (mark as initialized).
    #[inline]
    pub fn clear_tdz(&mut self, slot: usize) {
        if slot < 64 {
            self.tdz_bits &= !(1u64 << slot);
        } else {
            let adj = slot - 64;
            if let Some(w) = self.tdz_overflow.get_mut(adj / 64) {
                *w &= !(1u64 << (adj % 64));
            }
        }
    }
}

/// A registered exception handler within a call frame.
///
/// The compiler encodes a missing slot as `0xFFFF` in the bytecode
/// operand; `PushExceptionHandler` decodes that to `None` here so
/// runtime callers work with a type-safe sentinel instead of a raw
/// magic number (avoids confusing `u32::MAX` vs `0xFFFF` mismatches
/// — see PR #72 round 7).
#[derive(Clone, Debug)]
pub struct HandlerEntry {
    /// Bytecode offset of the catch block, or `None` if no catch.
    pub catch_ip: Option<u32>,
    /// Bytecode offset of the finally block, or `None` if no finally.
    pub finally_ip: Option<u32>,
    /// Stack depth when the handler was registered (for unwinding).
    pub stack_depth: usize,
}
