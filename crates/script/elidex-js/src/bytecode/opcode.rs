//! Bytecode opcodes for the elidex-js stack-based VM.
//!
//! Each opcode documents its stack effect as `[inputs -- outputs]` and
//! its operand encoding. All multi-byte operands are little-endian.

/// Bytecode instruction opcodes.
///
/// Operand notation:
/// - No suffix: zero operands (1-byte instruction)
/// - `u8`: one byte operand
/// - `u16`: two-byte operand (little-endian)
/// - `i16`: signed two-byte operand (jump offsets)
/// - `Wide` prefix: doubles operand widths to u32/i32
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Op {
    // ── Stack manipulation ──────────────────────────────────────────
    /// `[ -- undefined]`
    PushUndefined = 0,
    /// `[ -- null]`
    PushNull,
    /// `[ -- true]`
    PushTrue,
    /// `[ -- false]`
    PushFalse,
    /// Operand: i8. `[ -- Number]`
    PushI8,
    /// Operand: u16 (constant index). `[ -- value]`
    PushConst,
    /// `[v -- v v]`
    Dup,
    /// `[v -- ]` Discard the top of the stack.
    ///
    /// A pure stack operation: it does **not** touch `completion_value`. Use
    /// [`Op::PopCompletion`] for the one discard whose value is observable.
    Pop,
    /// `[v -- ]` Discard the top of the stack and record it as the script's
    /// completion value.
    ///
    /// Emitted for an `ExpressionStatement` only — ECMA-262 §14.5.1 makes its
    /// expression's value the statement's completion, and §16.1.6 steps 13.a +
    /// 13.b.i + 17 / §19.2.1.1 steps 29.a + 30.a + 33 surface the script's or
    /// `eval`'s last such value (13.b.i / 30.a normalise an empty completion to
    /// `undefined`).
    /// Any other body reaching this opcode must discard instead, which is what
    /// the dispatch arm's second condition enforces: it writes
    /// `completion_value` only from the **entry** frame and only when that frame
    /// is `FrameKind::Eval`. The gate is the invariant — a body kind is
    /// unobservable here because it runs in a non-entry frame, not because of
    /// anything about its production.
    ///
    /// Splitting this out of [`Op::Pop`] is what keeps internal stack
    /// housekeeping — reference cleanup, hoisting stores, destructuring
    /// scratch — from writing a completion value it does not own: `var x = 5;`
    /// evaluated to `5` instead of `undefined` because the declaration's store
    /// discarded through the recording opcode.
    ///
    /// ⚠ This is the only site that records an **expression's** value, which is
    /// not the same as being the only legitimate writer. §14.2.2 threads
    /// completions through `UpdateEmpty` (AO §6.2.4.4), and several statements
    /// yield a *non-empty* `undefined` that must overwrite the accumulated value
    /// — §14.6.2 step 3 / step 5 for `if`, and likewise iteration, `switch`,
    /// labelled and `try` statements. The VM has no `UpdateEmpty` equivalent, so
    /// the register is sticky across them and `42; if (false) {}` yields `42`
    /// where the spec says `undefined`. Carved as
    /// `#11-vm-statement-completion-updateempty`, pinned by
    /// `statement_completion_is_sticky_known_divergence`.
    PopCompletion,
    /// `[a b -- b a]`
    Swap,
    /// Operand: u8 (count `n`). `[a1 … an v -- v]` Discard `n` slots from
    /// **beneath** the top, keeping the top.
    ///
    /// Emitted to drop a member target's reference slots when a logical
    /// assignment short-circuits: `obj[key] ??= v` keeps the old value and must
    /// leave the `[object key]` pair behind.
    ///
    /// One instruction for the whole cleanup: `Swap; Pop` repeated `n` times is
    /// an emulation of it, and interleaving a swap dance with the pops is how the
    /// short-circuit tail's stack effect went wrong twice before. `n` is the
    /// target's reference-slot count (0 identifier / 1 named member / 2 computed
    /// member), so the retained value ends up alone on the stack whatever shape
    /// the target had.
    PopUnder,

    // ── Local variable access ───────────────────────────────────────
    /// Operand: u16 (local index). `[ -- value]`
    GetLocal,
    /// Operand: u16 (local index). `[value -- value]`
    SetLocal,

    // ── Temporal Dead Zone ──────────────────────────────────────────
    /// Operand: u16 (local index). `[ -- ]` Throws ReferenceError if TDZ.
    CheckTdz,
    /// Operand: u16 (local index). `[ -- ]` Marks local as initialized.
    InitLocal,

    // ── Upvalue (closure) access ────────────────────────────────────
    /// Operand: u16 (upvalue index). `[ -- value]`
    GetUpvalue,
    /// Operand: u16 (upvalue index). `[value -- value]`
    SetUpvalue,

    // ── Global access ───────────────────────────────────────────────
    /// Operand: u16 (constant index for name). `[ -- value]`
    GetGlobal,
    /// Operand: u16 (constant index for name). `[value -- value]`
    SetGlobal,

    // ── Property access ─────────────────────────────────────────────
    /// Operand: u16 (constant index for name). `[object -- value]`
    GetProp,
    /// Operand: u16 (constant index for name). `[object value -- value]`
    SetProp,
    /// `[object key -- value]`
    GetElem,
    /// `[object key -- object key' value]` Load through a computed member
    /// reference, keeping the reference for a following store.
    ///
    /// Emitted for compound and logical assignment to a computed member
    /// (`obj[key] += v`, `obj[key] ??= v`). Both read-modify-write ECMA-262
    /// §13.15.2 productions evaluate the LeftHandSideExpression **once** and
    /// reuse that reference for both `GetValue` and `PutValue`, so the operand
    /// expressions are not re-emitted — a side-effecting *key expression* runs
    /// once. The step indices are per-production: `LHS AssignmentOperator
    /// AssignmentExpression` is steps 1 / 3 / 9, while the logical forms
    /// (`&&=` `||=` `??=`) are steps 1 / 2 / 6.  (Simple `=` is not one of
    /// them: its production evaluates `leftRef` at step 1.a and `PutValue`s at
    /// step 1.e with no `GetValue`, so it needs no reference-preserving opcode.)
    ///
    /// `key'` is the **converted** key: §6.2.5.5 GetValue step 3.c.i writes
    /// `ToPropertyKey`'s result back into the Reference Record, so a later
    /// `PutValue` through the same reference does not re-run user
    /// `toString`/`@@toPrimitive`. Emitting the conversion here rather than at a
    /// separate site also preserves step 3.a's ordering — base coercion throws
    /// before the key is converted (`null[k] += 1` must be a `TypeError` even
    /// when `k.toString()` also throws).
    GetElemRef,
    /// `[object key value -- value]`
    SetElem,
    /// Operand: u16 (constant index). `[object -- bool]`
    DeleteProp,
    /// `[object key -- bool]`
    DeleteElem,
    /// Operand: u16 (constant index for #name). `[object -- value]`
    GetPrivate,
    /// Operand: u16 (constant index for #name). `[object value -- value]`
    SetPrivate,
    /// Operand: u16 (constant index). `[object -- bool]`
    PrivateIn,
    /// Operand: u16 (constant index for name). `[ -- value]`
    GetSuperProp,
    /// Operand: u16 (constant index for name). `[value -- value]`
    SetSuperProp,
    /// `[key -- value]`
    GetSuperElem,

    // ── Arithmetic ──────────────────────────────────────────────────
    /// `[a b -- a+b]`
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Exp,

    // ── Bitwise ─────────────────────────────────────────────────────
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    UShr,

    // ── Comparison ──────────────────────────────────────────────────
    Eq,
    NotEq,
    StrictEq,
    StrictNotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Instanceof,
    In,

    // ── Unary ───────────────────────────────────────────────────────
    /// `[a -- -a]`
    Neg,
    /// `[a -- +a]` (ToNumber)
    Pos,
    /// `[a -- !a]`
    Not,
    /// `[a -- ~a]`
    BitNot,
    /// `[a -- string]`
    TypeOf,
    /// Operand: u16 (constant index for name). `[ -- string]`
    /// Does NOT throw on unresolved globals.
    TypeOfGlobal,
    /// `[a -- undefined]`
    Void,

    // ── Update ──────────────────────────────────────────────────────
    /// Operand: u16 (local index), u8 (0=postfix, 1=prefix). `[ -- value]`
    IncLocal,
    DecLocal,
    /// Operand: u16 (name constant), u8 (prefix flag). `[object -- value]`
    IncProp,
    DecProp,
    /// Operand: u8 (prefix flag). `[object key -- value]`
    IncElem,
    DecElem,

    // ── Control flow ────────────────────────────────────────────────
    /// Operand: i16 (relative offset). Unconditional jump.
    Jump,
    /// Operand: i16. `[cond -- ]` Jump if falsy (pops).
    JumpIfFalse,
    /// Operand: i16. `[cond -- ]` Jump if truthy (pops).
    JumpIfTrue,
    /// Operand: i16. `[val -- val]` Jump if nullish (does NOT pop).
    JumpIfNullish,
    /// Operand: i16. `[val -- val]` Jump if NOT nullish (does NOT pop).
    JumpIfNotNullish,

    // ── Function operations ─────────────────────────────────────────
    /// Operand: u8 (argc). `[callee arg0..argN -- result]`
    Call,
    /// Operand: u8 (argc). `[receiver callee arg0..argN -- result]`
    CallMethod,
    /// Operand: u8 (argc). `[constructor arg0..argN -- instance]`
    New,
    /// `[callee args_array -- result]`
    CallSpread,
    /// `[constructor args_array -- instance]`
    NewSpread,
    /// `[value -- ]` Return TOS from function.
    Return,
    /// `[ -- ]` Return undefined from function.
    ReturnUndefined,
    /// `[ -- this]`
    PushThis,
    /// Operand: u16 (constant index → CompiledFunction). `[ -- closure]`
    /// Followed by N upvalue descriptors.
    Closure,

    // ── Object/Array creation ───────────────────────────────────────
    /// `[ -- obj]`
    CreateObject,
    /// Operand: u16 (constant index for name). `[object value -- object]`
    DefineProperty,
    /// `[object key value -- object]`
    DefineComputedProperty,
    /// Like DefineComputedProperty but non-enumerable (for class methods). `[object key value -- object]`
    DefineComputedMethod,
    /// Operand: u16 (constant index for name), u8 (flags: bit 0 = enumerable). `[object closure -- object]`
    DefineGetter,
    /// Operand: u16 (constant index for name), u8 (flags: bit 0 = enumerable). `[object closure -- object]`
    DefineSetter,
    /// Computed-key getter: `[object key closure -- object]`. Non-enumerable (class accessor).
    DefineComputedGetter,
    /// Computed-key setter: `[object key closure -- object]`. Non-enumerable (class accessor).
    DefineComputedSetter,
    /// `[object source -- object]`
    SpreadObject,
    /// `[ -- array]`
    CreateArray,
    /// `[array value -- array]`
    ArrayPush,
    /// `[array iterable -- array]`
    ArraySpread,
    /// `[array -- array]`
    ArrayHole,

    // ── Template literals ───────────────────────────────────────────
    /// Operand: u16 (count). `[s0..sN -- result]`
    TemplateConcat,
    /// Operand: u8 (expr count). `[tag template_obj e0..eN -- result]`
    TaggedTemplate,

    // ── Destructuring ───────────────────────────────────────────────
    /// `[iterable -- iterator]`
    GetIterator,
    /// `[iterator -- iterator value done]`
    IteratorNext,
    /// `[iterator -- ]`
    IteratorClose,
    /// `[iterator -- array]`
    IteratorRest,
    /// Operand: u16 (constant index for name). `[object -- object value]`
    DestructureProp,
    /// `[object key -- object value]`
    DestructureElem,
    /// Operand: u16 (count of already-destructured keys). `[object key0..keyN -- rest_object]`
    ObjectRest,
    /// Operand: i16 (jump offset). `[value -- value_or_replaced]`
    DefaultIfUndefined,

    // ── Exception handling ──────────────────────────────────────────
    /// Operand: u16 (catch offset), u16 (finally offset; 0xFFFF = none).
    PushExceptionHandler,
    /// `[ -- ]`
    PopExceptionHandler,
    /// `[value -- ]`
    Throw,
    /// Operand: u16 (constant index of the message). `[ -- ]` Always throws a
    /// `TypeError` carrying that message; control never falls through.
    ///
    /// The emit path for a construct the compiler cannot yet lower. The umbrella's
    /// **I-1** requires such a path to raise a *loud, **scoped*** error, and a
    /// `CompileError` is loud but not scoped — it fails the whole script, so one
    /// unsupported construct anywhere costs every other statement in the file.
    /// This keeps the failure local to the statement that executes it, and
    /// catchable, exactly as `Op::CheckTdz` does for the TDZ `ReferenceError`.
    /// Reserve it for constructs that are genuinely unimplemented; a construct the
    /// *language* rejects belongs in the parser or a `CompileError`.
    ThrowUnsupported,
    /// `[ -- exception]`
    PushException,
    /// End-of-finally-body marker.  Consults `frame.pending_completion`
    /// (set when the finally was entered via an externally injected abrupt
    /// completion — e.g. `Generator.prototype.return`) and resumes that
    /// completion if present.  `None`/`Normal` → no-op, fall through to
    /// normal code after the try statement.  `Return(v)` → perform return
    /// (walking further outer finallies).  `Throw(e)` → re-raise.
    /// `[ -- ]`
    EndFinally,

    // ── Class operations ────────────────────────────────────────────
    /// Operand: u16 (constant index → class descriptor). `[super_or_undefined -- class]`
    CreateClass,
    /// Operand: u16 (name constant), u8 (flags: static|kind). `[class closure -- class]`
    DefineMethod,
    /// Operand: u16 (name constant), u8 (flags). `[class value -- class]`
    DefineField,
    /// Operand: u8 (argc). `[arg0..argN -- this]`
    ///
    /// `new.target` is NOT a stack operand — the dispatcher reads it
    /// from the active [`super::super::vm::value::CallFrame::mode`]
    /// (a [`super::super::vm::value::CallMode::Construct`] variant
    /// carries the `new_target`; threaded through
    /// `construct_synchronous`); the super class is resolved via
    /// [`super::super::vm::value::CallFrame::home_class`]'s
    /// `[[Prototype]]` per ECMA-262 §13.3.7.2 GetSuperConstructor.
    SuperCall,
    /// `[args_array -- this]`
    ///
    /// As [`Self::SuperCall`]: `new.target` / super constructor come
    /// from the call frame, not the operand stack.
    SuperCallSpread,
    /// `[child parent -- child]` Set `child.[[Prototype]] = parent`.
    /// `parent` must be Object or Null; otherwise throws TypeError.
    /// Used by class-chain setup (\[C16\] ClassDefinitionEvaluation
    /// constructorParent / protoParent wiring).
    SetPrototype,
    /// `[value -- ]` Pop `value`; throw TypeError if it is not a
    /// constructor. Used by class-heritage validation (ECMA-262
    /// §15.7.14 ClassDefinitionEvaluation step 6.f) to reject
    /// `class B extends Symbol {}` / `class B extends BigInt {}` —
    /// callable natives that lack `[[Construct]]` — at definition
    /// time, before any prototype splice fires.
    AssertConstructor,

    /// Create the `arguments` object from the current frame's actual args. `[ -- arguments_obj]`
    CreateArguments,

    // ── Generator / Async ───────────────────────────────────────────
    /// `[value -- resumed_value]`
    Yield,
    /// `[promise -- resolved_value]`
    Await,

    // ── Misc ────────────────────────────────────────────────────────
    /// `[ -- new.target_or_undefined]`
    NewTarget,
    /// `[ -- import.meta_object]`
    ImportMeta,
    /// `[source -- promise]`
    DynamicImport,
    /// No-op in production.
    Debugger,
    /// Operand: u16 (module binding index). `[ -- value]`
    GetModuleVar,
    /// Operand: u16 (jump table constant index). `[discriminant -- ]`
    SwitchJump,
    /// `[object -- for_in_iterator]`
    ForInIterator,
    /// `[for_in_iterator -- for_in_iterator key done]`
    ForInNext,
    /// Next opcode uses 32-bit operands instead of 16-bit.
    Wide,
}

impl Op {
    /// Decode a byte into an opcode, if valid.
    #[must_use]
    #[allow(unsafe_code)]
    pub fn from_byte(byte: u8) -> Option<Self> {
        if byte <= Self::Wide as u8 {
            // SAFETY: All values 0..=Wide are valid repr(u8) discriminants
            // with no gaps (enum variants are sequential from 0).
            Some(unsafe { std::mem::transmute::<u8, Self>(byte) })
        } else {
            None
        }
    }

    /// Encode as a single byte.
    #[must_use]
    pub fn to_byte(self) -> u8 {
        self as u8
    }

    /// Number of operand bytes following this opcode (excluding Wide prefix).
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn operand_size(self) -> usize {
        match self {
            // Zero operands
            Self::PushUndefined
            | Self::PushNull
            | Self::PushTrue
            | Self::PushFalse
            | Self::Dup
            | Self::Pop
            | Self::PopCompletion
            | Self::Swap
            | Self::GetElem
            | Self::GetElemRef
            | Self::SetElem
            | Self::DeleteElem
            | Self::GetSuperElem
            | Self::Add
            | Self::Sub
            | Self::Mul
            | Self::Div
            | Self::Mod
            | Self::Exp
            | Self::BitAnd
            | Self::BitOr
            | Self::BitXor
            | Self::Shl
            | Self::Shr
            | Self::UShr
            | Self::Eq
            | Self::NotEq
            | Self::StrictEq
            | Self::StrictNotEq
            | Self::Lt
            | Self::LtEq
            | Self::Gt
            | Self::GtEq
            | Self::Instanceof
            | Self::In
            | Self::Neg
            | Self::Pos
            | Self::Not
            | Self::BitNot
            | Self::TypeOf
            | Self::Void
            | Self::Return
            | Self::ReturnUndefined
            | Self::PushThis
            | Self::CreateObject
            | Self::DefineComputedProperty
            | Self::DefineComputedMethod
            | Self::DefineComputedGetter
            | Self::DefineComputedSetter
            | Self::SpreadObject
            | Self::CreateArray
            | Self::ArrayPush
            | Self::ArraySpread
            | Self::ArrayHole
            | Self::GetIterator
            | Self::IteratorNext
            | Self::IteratorClose
            | Self::IteratorRest
            | Self::DestructureElem
            | Self::PopExceptionHandler
            | Self::Throw
            | Self::PushException
            | Self::EndFinally
            | Self::Yield
            | Self::Await
            | Self::NewTarget
            | Self::ImportMeta
            | Self::DynamicImport
            | Self::Debugger
            | Self::CallSpread
            | Self::NewSpread
            | Self::SuperCallSpread
            | Self::ForInIterator
            | Self::ForInNext
            | Self::CreateArguments
            | Self::SetPrototype
            | Self::AssertConstructor
            | Self::Wide => 0,

            // 1-byte operand (u8 or i8)
            Self::PushI8
            | Self::PopUnder
            | Self::New
            | Self::SuperCall
            | Self::TaggedTemplate
            | Self::IncElem
            | Self::DecElem => 1,

            // 2-byte operand (u16 or i16)
            Self::PushConst
            | Self::GetLocal
            | Self::SetLocal
            | Self::CheckTdz
            | Self::InitLocal
            | Self::GetUpvalue
            | Self::SetUpvalue
            | Self::GetGlobal
            | Self::SetGlobal
            | Self::DeleteProp
            | Self::GetPrivate
            | Self::SetPrivate
            | Self::PrivateIn
            | Self::GetSuperProp
            | Self::SetSuperProp
            | Self::TypeOfGlobal
            | Self::Jump
            | Self::JumpIfFalse
            | Self::JumpIfTrue
            | Self::JumpIfNullish
            | Self::JumpIfNotNullish
            | Self::Closure
            | Self::DefineProperty
            | Self::TemplateConcat
            | Self::DestructureProp
            | Self::ObjectRest
            | Self::DefaultIfUndefined
            | Self::CreateClass
            | Self::GetModuleVar
            | Self::ThrowUnsupported
            | Self::SwitchJump => 2,

            // 3-byte operand (u8 + u16 call IC index, or u16 + u8)
            Self::Call
            | Self::CallMethod
            | Self::IncLocal
            | Self::DecLocal
            | Self::IncProp
            | Self::DecProp
            | Self::DefineMethod
            | Self::DefineField
            | Self::DefineGetter
            | Self::DefineSetter => 3,

            // 4-byte operand (u16 + u16)
            Self::GetProp | Self::SetProp | Self::PushExceptionHandler => 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for byte in 0..=Op::Wide as u8 {
            let op = Op::from_byte(byte).unwrap();
            assert_eq!(op.to_byte(), byte);
        }
    }

    #[test]
    fn invalid_byte() {
        assert!(Op::from_byte(255).is_none());
    }

    #[test]
    fn push_undefined_is_zero() {
        assert_eq!(Op::PushUndefined as u8, 0);
    }
}
