//! JS value types for the elidex-js VM.
//!
//! All values are handle-based: strings and objects are indices into
//! VM-owned tables, making `JsValue` `Copy` and trivially `Send + Sync`.

use std::fmt;

// ---------------------------------------------------------------------------
// Handle types (u32 indices into Vm tables)
// ---------------------------------------------------------------------------

/// Index into `Vm::strings` (StringPool).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct StringId(pub(crate) u32);

impl fmt::Debug for StringId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StringId({})", self.0)
    }
}

/// Index into `Vm::objects`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId(pub(crate) u32);

impl fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ObjectId({})", self.0)
    }
}

/// Index into `Vm::compiled_functions`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FuncId(pub(crate) u32);

impl fmt::Debug for FuncId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FuncId({})", self.0)
    }
}

/// Index into `Vm::upvalues`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct UpvalueId(pub(crate) u32);

impl fmt::Debug for UpvalueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UpvalueId({})", self.0)
    }
}

/// Index into `Vm::symbols`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub(crate) u32);

impl fmt::Debug for SymbolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SymbolId({})", self.0)
    }
}

/// A Symbol record stored in the VM's symbol table.
pub struct SymbolRecord {
    /// Optional description (e.g., `Symbol("foo")` → `"foo"`).
    pub description: Option<StringId>,
}

// ---------------------------------------------------------------------------
// PropertyKey — String or Symbol key for object properties
// ---------------------------------------------------------------------------

/// A property key: either a string name or a symbol.
/// Used as the key type in `Object.properties`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PropertyKey {
    String(StringId),
    Symbol(SymbolId),
}

// ---------------------------------------------------------------------------
// JsValue — the core runtime value (16 bytes, Copy)
// ---------------------------------------------------------------------------

/// A JavaScript value. All heap-allocated data (strings, objects) is
/// represented as a handle into VM-owned storage, making this type `Copy`.
#[derive(Clone, Copy, Debug)]
pub enum JsValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(StringId),
    Symbol(SymbolId),
    Object(ObjectId),
}

impl JsValue {
    /// Returns `true` if the value is `undefined` or `null`.
    #[inline]
    pub fn is_nullish(self) -> bool {
        matches!(self, Self::Undefined | Self::Null)
    }

    /// Returns `true` if the value is the boolean `false`, numeric `0`/`NaN`,
    /// `null`, `undefined`, or the empty string. The empty-string check
    /// requires access to the string pool and is handled in `Vm::to_boolean`.
    #[inline]
    pub fn is_primitive_falsy(self) -> bool {
        match self {
            Self::Undefined | Self::Null => true,
            Self::Boolean(b) => !b,
            Self::Number(n) => n == 0.0 || n.is_nan(),
            Self::String(_) | Self::Symbol(_) | Self::Object(_) => false,
        }
    }
}

impl PartialEq for JsValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Undefined, Self::Undefined) | (Self::Null, Self::Null) => true,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => {
                // JS strict equality: NaN !== NaN, +0 === -0
                a == b
            }
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Symbol(a), Self::Symbol(b)) => a == b,
            (Self::Object(a), Self::Object(b)) => a == b,
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Object model
// ---------------------------------------------------------------------------

/// A JS object stored in `Vm::objects`.
pub struct Object {
    pub kind: ObjectKind,
    /// Property list. Linear scan for now; M4-11 adds hidden classes.
    pub properties: Vec<(PropertyKey, Property)>,
    /// Prototype chain link (`__proto__`).
    pub prototype: Option<ObjectId>,
}

/// The internal kind of an object.
pub enum ObjectKind {
    /// Plain `{}` object.
    Ordinary,
    /// Array with dense element storage.
    Array { elements: Vec<JsValue> },
    /// A compiled JS function (closure).
    Function(FunctionObject),
    /// A bound function (`Function.prototype.bind`).
    BoundFunction {
        target: ObjectId,
        bound_this: JsValue,
        bound_args: Vec<JsValue>,
    },
    /// A native (Rust) function callable from JS.
    NativeFunction(NativeFunction),
    /// A RegExp value (stores pattern+flags; execution deferred to M4-10.2).
    RegExp { pattern: StringId, flags: StringId },
    /// An Error instance.
    Error { name: StringId },
    /// For-in iterator state.
    ForInIterator(ForInState),
    /// Array/iterable iterator state.
    ArrayIterator(ArrayIterState),
}

/// A compiled JS function with captured upvalues.
pub struct FunctionObject {
    /// Index into `Vm::compiled_functions`.
    pub func_id: FuncId,
    /// Captured upvalue handles.
    pub upvalue_ids: Vec<UpvalueId>,
    /// How `this` is resolved.
    pub this_mode: ThisMode,
    /// Function name (for stack traces / `.name` property).
    pub name: Option<StringId>,
    /// For arrow functions: the lexical `this` captured at closure-creation time.
    pub captured_this: Option<JsValue>,
}

/// How `this` is bound for a function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThisMode {
    /// Arrow function: inherits `this` from enclosing scope.
    Lexical,
    /// Non-strict function: `this` defaults to global object.
    Global,
    /// Strict function: `this` is exactly what was passed.
    Strict,
}

/// A native function callable from JS.
///
/// The signature takes `&mut Vm` (via a wrapper in interpreter.rs) so that
/// native functions can allocate objects, intern strings, etc.
/// The `this` argument and positional args are passed as `JsValue`.
pub struct NativeFunction {
    pub name: StringId,
    pub func: fn(&mut NativeContext<'_>, JsValue, &[JsValue]) -> Result<JsValue, VmError>,
}

/// Context passed to native functions, providing mutable access to the VM.
/// Defined here, implemented in `mod.rs`.
pub struct NativeContext<'a> {
    pub(crate) vm: &'a mut super::VmInner,
}

/// A data property on an object.
#[derive(Clone, Copy, Debug)]
pub struct Property {
    pub value: JsValue,
    pub writable: bool,
    pub enumerable: bool,
    pub configurable: bool,
}

impl Property {
    /// Create a writable, enumerable, configurable data property.
    pub fn data(value: JsValue) -> Self {
        Self {
            value,
            writable: true,
            enumerable: true,
            configurable: true,
        }
    }

    /// Create a non-enumerable, non-configurable, non-writable property (for built-ins).
    pub fn builtin(value: JsValue) -> Self {
        Self {
            value,
            writable: false,
            enumerable: false,
            configurable: false,
        }
    }

    /// Create a writable, non-enumerable, configurable property (for built-in methods).
    pub fn method(value: JsValue) -> Self {
        Self {
            value,
            writable: true,
            enumerable: false,
            configurable: true,
        }
    }
}

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

/// A single call frame on the VM's call stack.
pub struct CallFrame {
    /// The compiled function being executed.
    pub func_id: FuncId,
    /// Instruction pointer (byte offset into bytecode).
    pub ip: usize,
    /// Stack base: `stack[base..base+local_count]` are this frame's locals.
    pub base: usize,
    /// Upvalue handles for this invocation (captures from parent).
    pub upvalue_ids: Vec<UpvalueId>,
    /// Upvalues that capture *this* frame's local slots (closed on frame pop).
    pub local_upvalue_ids: Vec<UpvalueId>,
    /// The `this` value for this call.
    pub this_value: JsValue,
    /// Active exception handlers (try/catch/finally).
    pub exception_handlers: Vec<HandlerEntry>,
    /// TDZ tracking: `true` = slot is uninitialized (in temporal dead zone).
    /// Only `let`/`const` bindings are checked; `var` slots are cleared at frame creation.
    pub tdz_slots: Vec<bool>,
}

/// A registered exception handler within a call frame.
#[derive(Clone, Debug)]
pub struct HandlerEntry {
    /// Bytecode offset of the catch block (`u32::MAX` if no catch).
    pub catch_ip: u32,
    /// Bytecode offset of the finally block (`u32::MAX` if no finally).
    pub finally_ip: u32,
    /// Stack depth when the handler was registered (for unwinding).
    pub stack_depth: usize,
}

// ---------------------------------------------------------------------------
// ForInState / ArrayIterState
// ---------------------------------------------------------------------------

/// State for a `for-in` iterator.
pub struct ForInState {
    /// The collected enumerable keys.
    pub keys: Vec<StringId>,
    /// Current index into `keys`.
    pub index: usize,
}

/// State for an array/iterable iterator.
pub struct ArrayIterState {
    /// The array being iterated.
    pub array_id: ObjectId,
    /// Current index.
    pub index: usize,
}

// ---------------------------------------------------------------------------
// VmError
// ---------------------------------------------------------------------------

/// An error raised during VM execution.
#[derive(Debug)]
pub struct VmError {
    pub kind: VmErrorKind,
    pub message: String,
}

/// The kind of VM error.
#[derive(Debug)]
pub enum VmErrorKind {
    TypeError,
    ReferenceError,
    RangeError,
    SyntaxError,
    /// A user `throw` value — the thrown JS value is preserved.
    ThrowValue(JsValue),
    /// Internal VM error (should not occur in correct programs).
    InternalError,
    /// Compilation error.
    CompileError,
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match &self.kind {
            VmErrorKind::TypeError => "TypeError",
            VmErrorKind::ReferenceError => "ReferenceError",
            VmErrorKind::RangeError => "RangeError",
            VmErrorKind::SyntaxError => "SyntaxError",
            VmErrorKind::ThrowValue(_) => "Uncaught",
            VmErrorKind::InternalError => "InternalError",
            VmErrorKind::CompileError => "CompileError",
        };
        write!(f, "{prefix}: {}", self.message)
    }
}

impl std::error::Error for VmError {}

impl VmError {
    pub fn type_error(message: impl Into<String>) -> Self {
        Self {
            kind: VmErrorKind::TypeError,
            message: message.into(),
        }
    }

    pub fn reference_error(message: impl Into<String>) -> Self {
        Self {
            kind: VmErrorKind::ReferenceError,
            message: message.into(),
        }
    }

    pub fn range_error(message: impl Into<String>) -> Self {
        Self {
            kind: VmErrorKind::RangeError,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: VmErrorKind::InternalError,
            message: message.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Static assertions
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_value_is_copy() {
        let v = JsValue::Number(42.0);
        let v2 = v; // Copy
        assert_eq!(v, v2);
    }

    #[test]
    fn js_value_size() {
        // JsValue should be at most 16 bytes (tag + f64).
        assert!(std::mem::size_of::<JsValue>() <= 16);
    }

    #[test]
    fn js_value_nan_inequality() {
        let nan = JsValue::Number(f64::NAN);
        assert_ne!(nan, nan); // NaN !== NaN
    }

    #[test]
    fn js_value_zero_equality() {
        let pos = JsValue::Number(0.0);
        let neg = JsValue::Number(-0.0);
        assert_eq!(pos, neg); // +0 === -0
    }

    #[test]
    fn js_value_nullish() {
        assert!(JsValue::Undefined.is_nullish());
        assert!(JsValue::Null.is_nullish());
        assert!(!JsValue::Boolean(false).is_nullish());
        assert!(!JsValue::Number(0.0).is_nullish());
    }

    #[test]
    fn js_value_primitive_falsy() {
        assert!(JsValue::Undefined.is_primitive_falsy());
        assert!(JsValue::Null.is_primitive_falsy());
        assert!(JsValue::Boolean(false).is_primitive_falsy());
        assert!(JsValue::Number(0.0).is_primitive_falsy());
        assert!(JsValue::Number(f64::NAN).is_primitive_falsy());
        assert!(!JsValue::Boolean(true).is_primitive_falsy());
        assert!(!JsValue::Number(1.0).is_primitive_falsy());
        // String/Object falsiness requires Vm access (empty string check).
        assert!(!JsValue::String(StringId(0)).is_primitive_falsy());
        assert!(!JsValue::Object(ObjectId(0)).is_primitive_falsy());
    }

    #[test]
    fn string_id_equality() {
        assert_eq!(StringId(0), StringId(0));
        assert_ne!(StringId(0), StringId(1));
    }

    #[test]
    fn property_constructors() {
        let p = Property::data(JsValue::Number(42.0));
        assert!(p.writable && p.enumerable && p.configurable);

        let p = Property::builtin(JsValue::Undefined);
        assert!(!p.writable && !p.enumerable && !p.configurable);

        let p = Property::method(JsValue::Undefined);
        assert!(p.writable && !p.enumerable && p.configurable);
    }
}
