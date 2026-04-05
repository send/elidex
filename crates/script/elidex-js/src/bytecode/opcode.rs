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
    /// `[v -- ]`
    Pop,
    /// `[a b -- b a]`
    Swap,

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
    /// Operand: u16 (constant index for name). `[object closure -- object]`
    DefineGetter,
    /// Operand: u16 (constant index for name). `[object closure -- object]`
    DefineSetter,
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
    /// `[ -- exception]`
    PushException,

    // ── Class operations ────────────────────────────────────────────
    /// Operand: u16 (constant index → class descriptor). `[super_or_undefined -- class]`
    CreateClass,
    /// Operand: u16 (name constant), u8 (flags: static|kind). `[class closure -- class]`
    DefineMethod,
    /// Operand: u16 (name constant), u8 (flags). `[class value -- class]`
    DefineField,
    /// Operand: u8 (argc). `[new.target arg0..argN -- this]`
    SuperCall,
    /// `[new.target args_array -- this]`
    SuperCallSpread,

    // ── Generator / Async ───────────────────────────────────────────
    /// `[value -- resumed_value]`
    Yield,
    /// `[iterator -- result]`
    YieldDelegate,
    /// `[promise -- resolved_value]`
    Await,
    /// `[ -- generator]`
    CreateGenerator,
    /// `[ -- async_generator]`
    CreateAsyncGenerator,

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
            | Self::Swap
            | Self::GetElem
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
            | Self::Yield
            | Self::YieldDelegate
            | Self::Await
            | Self::CreateGenerator
            | Self::CreateAsyncGenerator
            | Self::NewTarget
            | Self::ImportMeta
            | Self::DynamicImport
            | Self::Debugger
            | Self::CallSpread
            | Self::NewSpread
            | Self::SuperCallSpread
            | Self::ForInIterator
            | Self::ForInNext
            | Self::Wide => 0,

            // 1-byte operand (u8 or i8)
            Self::PushI8
            | Self::Call
            | Self::CallMethod
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
            | Self::GetProp
            | Self::SetProp
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
            | Self::DefineGetter
            | Self::DefineSetter
            | Self::TemplateConcat
            | Self::DestructureProp
            | Self::ObjectRest
            | Self::DefaultIfUndefined
            | Self::CreateClass
            | Self::GetModuleVar
            | Self::SwitchJump => 2,

            // 3-byte operand (u16 + u8)
            Self::IncLocal
            | Self::DecLocal
            | Self::IncProp
            | Self::DecProp
            | Self::DefineMethod
            | Self::DefineField => 3,

            // 4-byte operand (u16 + u16)
            Self::PushExceptionHandler => 4,
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
