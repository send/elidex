//! JS value types for the elidex-js VM.
//!
//! All values are handle-based: strings and objects are indices into
//! VM-owned tables, making `JsValue` `Copy` and trivially `Send + Sync`.

use std::fmt;
use std::sync::Arc;

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

/// SameValue (ES2020 §7.2.10): differs from strict equality in that
/// `NaN` is considered equal to `NaN` and `+0` is not the same as `-0`.
pub fn same_value(a: JsValue, b: JsValue) -> bool {
    match (a, b) {
        (JsValue::Number(x), JsValue::Number(y)) => {
            if x.is_nan() && y.is_nan() {
                return true;
            }
            if x == 0.0 && y == 0.0 {
                return x.to_bits() == y.to_bits();
            }
            x == y
        }
        _ => a == b,
    }
}

// ---------------------------------------------------------------------------
// Object model
// ---------------------------------------------------------------------------

/// A JS object stored in `Vm::objects`.
pub struct Object {
    pub kind: ObjectKind,
    /// Property storage — either Shape-based (fast) or Dictionary (fallback).
    pub storage: PropertyStorage,
    /// Prototype chain link (`__proto__`).
    pub prototype: Option<ObjectId>,
}

/// Iterator over property entries that avoids heap allocation.
///
/// Each variant wraps the concrete iterator type from one of the
/// [`PropertyStorage`] representations (Shaped vs Dictionary).
pub enum PropertyIter<S, D> {
    Shaped(S),
    Dictionary(D),
}

impl<S, D, T> Iterator for PropertyIter<S, D>
where
    S: Iterator<Item = T>,
    D: Iterator<Item = T>,
{
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        match self {
            Self::Shaped(s) => s.next(),
            Self::Dictionary(d) => d.next(),
        }
    }
}

/// Property storage for an object.
///
/// New objects start in [`Shaped`](PropertyStorage::Shaped) mode with the root
/// shape.  The only operation that triggers a fallback to
/// [`Dictionary`](PropertyStorage::Dictionary) mode is property deletion.
/// Attribute changes (including data↔accessor conversion) use Shape
/// reconfigure transitions and stay in Shaped mode.
pub enum PropertyStorage {
    /// Shape-based storage: the Shape defines property names, order and
    /// attributes; `slots[i]` holds the value for `shape.ordered_entries[i]`.
    Shaped {
        shape: super::shape::ShapeId,
        slots: Vec<PropertyValue>,
    },
    /// Dictionary fallback after `delete`: identical to the old `Vec` storage.
    Dictionary(Vec<(PropertyKey, Property)>),
}

impl PropertyStorage {
    /// Create a new Shaped storage with the given shape and no slots.
    #[inline]
    pub fn shaped(shape: super::shape::ShapeId) -> Self {
        Self::Shaped {
            shape,
            slots: Vec::new(),
        }
    }

    /// Create a new empty Dictionary storage.
    #[inline]
    pub fn dictionary() -> Self {
        Self::Dictionary(Vec::new())
    }

    /// Return the shape ID if in Shaped mode.
    #[inline]
    pub fn shape_id(&self) -> Option<super::shape::ShapeId> {
        match self {
            Self::Shaped { shape, .. } => Some(*shape),
            Self::Dictionary(_) => None,
        }
    }

    /// Iterate all properties in insertion order as `(key, &value, attrs)`.
    pub fn iter_properties<'a>(
        &'a self,
        shapes: &'a [super::shape::Shape],
    ) -> PropertyIter<
        impl Iterator<Item = (PropertyKey, &'a PropertyValue, super::shape::PropertyAttrs)> + 'a,
        impl Iterator<Item = (PropertyKey, &'a PropertyValue, super::shape::PropertyAttrs)> + 'a,
    > {
        match self {
            Self::Shaped { shape, slots } => {
                let s = &shapes[*shape as usize];
                PropertyIter::Shaped(
                    s.ordered_entries
                        .iter()
                        .enumerate()
                        .map(move |(i, (key, attrs))| (*key, &slots[i], *attrs)),
                )
            }
            Self::Dictionary(vec) => {
                PropertyIter::Dictionary(vec.iter().map(|(k, p)| (*k, &p.slot, p.attrs())))
            }
        }
    }

    /// Iterate all keys in insertion order as `(key, attrs)`.
    pub fn iter_keys<'a>(
        &'a self,
        shapes: &'a [super::shape::Shape],
    ) -> PropertyIter<
        impl Iterator<Item = (PropertyKey, super::shape::PropertyAttrs)> + 'a,
        impl Iterator<Item = (PropertyKey, super::shape::PropertyAttrs)> + 'a,
    > {
        match self {
            Self::Shaped { shape, .. } => {
                let s = &shapes[*shape as usize];
                PropertyIter::Shaped(s.ordered_entries.iter().copied())
            }
            Self::Dictionary(vec) => {
                PropertyIter::Dictionary(vec.iter().map(|(k, p)| (*k, p.attrs())))
            }
        }
    }

    /// O(1) property lookup by key.
    pub fn get(
        &self,
        key: PropertyKey,
        shapes: &[super::shape::Shape],
    ) -> Option<(&PropertyValue, super::shape::PropertyAttrs)> {
        match self {
            Self::Shaped { shape, slots } => {
                let s = &shapes[*shape as usize];
                s.lookup(key)
                    .map(|(idx, attrs)| (&slots[idx as usize], attrs))
            }
            Self::Dictionary(vec) => vec
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, p)| (&p.slot, p.attrs())),
        }
    }

    /// O(1) mutable slot access.  Returns the mutable value reference and attrs.
    pub fn get_mut(
        &mut self,
        key: PropertyKey,
        shapes: &[super::shape::Shape],
    ) -> Option<(&mut PropertyValue, super::shape::PropertyAttrs)> {
        match self {
            Self::Shaped { shape, slots } => {
                let s = &shapes[*shape as usize];
                s.lookup(key)
                    .map(move |(idx, attrs)| (&mut slots[idx as usize], attrs))
            }
            Self::Dictionary(vec) => vec.iter_mut().find(|(k, _)| *k == key).map(|(_, p)| {
                let attrs = p.attrs();
                (&mut p.slot, attrs)
            }),
        }
    }

    /// Check whether a property exists.
    pub fn has(&self, key: PropertyKey, shapes: &[super::shape::Shape]) -> bool {
        match self {
            Self::Shaped { shape, .. } => shapes[*shape as usize].has(key),
            Self::Dictionary(vec) => vec.iter().any(|(k, _)| *k == key),
        }
    }

    /// Push a property in Dictionary mode.
    ///
    /// # Panics
    /// Panics if not in Dictionary mode.
    pub fn push_dict(&mut self, key: PropertyKey, prop: Property) {
        match self {
            Self::Dictionary(vec) => vec.push((key, prop)),
            Self::Shaped { .. } => panic!("push_dict called on Shaped storage"),
        }
    }

    /// Remove a property by position in Dictionary mode.
    ///
    /// # Panics
    /// Panics if not in Dictionary mode.
    pub fn remove_dict(&mut self, pos: usize) -> (PropertyKey, Property) {
        match self {
            Self::Dictionary(vec) => vec.remove(pos),
            Self::Shaped { .. } => panic!("remove_dict called on Shaped storage"),
        }
    }

    /// Write a value to a specific slot (by index).
    pub fn set_slot_value(&mut self, index: u16, val: PropertyValue) {
        match self {
            Self::Shaped { slots, .. } => slots[index as usize] = val,
            Self::Dictionary(_) => panic!("set_slot_value called on Dictionary storage"),
        }
    }

    /// Find a property's position in Dictionary mode (for deletion).
    pub fn dict_position(&self, key: PropertyKey) -> Option<usize> {
        match self {
            Self::Dictionary(vec) => vec.iter().position(|(k, _)| *k == key),
            Self::Shaped { .. } => None,
        }
    }

    /// Get a property by position in Dictionary mode.
    pub fn dict_get(&self, pos: usize) -> &Property {
        match self {
            Self::Dictionary(vec) => &vec[pos].1,
            Self::Shaped { .. } => panic!("dict_get called on Shaped storage"),
        }
    }
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
    /// A RegExp value with compiled regex for execution.
    RegExp {
        pattern: StringId,
        flags: StringId,
        compiled: Box<regress::Regex>,
    },
    /// An Error instance.
    Error { name: StringId },
    /// For-in iterator state.
    ForInIterator(ForInState),
    /// Array/iterable iterator state.
    ArrayIterator(ArrayIterState),
    /// String iterator state (for `String.prototype[Symbol.iterator]()`).
    StringIterator(StringIterState),
    /// The `arguments` array-like object for non-arrow functions.
    Arguments { values: Vec<JsValue> },
    /// Wrapper object for Number primitives (§9.2.1.2 this-boxing).
    NumberWrapper(f64),
    /// Wrapper object for String primitives (§9.2.1.2 this-boxing).
    StringWrapper(StringId),
    /// Wrapper object for Boolean primitives (§9.2.1.2 this-boxing).
    BooleanWrapper(bool),
}

/// A compiled JS function with captured upvalues.
pub struct FunctionObject {
    /// Index into `Vm::compiled_functions`.
    pub func_id: FuncId,
    /// Captured upvalue handles (shared via `Arc` to avoid clone overhead).
    pub upvalue_ids: Arc<[UpvalueId]>,
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

/// Extracted callee info for JS function calls via the single dispatcher.
/// Produced by `extract_js_callee`, consumed by `push_js_call_frame`.
pub struct JsCalleeInfo {
    pub func_id: FuncId,
    pub upvalue_ids: Arc<[UpvalueId]>,
    pub this_mode: ThisMode,
    pub captured_this: Option<JsValue>,
}

/// A native function callable from JS.
///
/// The signature takes `&mut Vm` (via a wrapper in interpreter.rs) so that
/// native functions can allocate objects, intern strings, etc.
/// The `this` argument and positional args are passed as `JsValue`.
pub struct NativeFunction {
    pub name: StringId,
    pub func: fn(&mut NativeContext<'_>, JsValue, &[JsValue]) -> Result<JsValue, VmError>,
    /// Whether `new` can be used on this function. `false` for Symbol etc.
    pub constructable: bool,
}

/// Context passed to native functions, providing mutable access to the VM.
/// Defined here, implemented in `mod.rs`.
pub struct NativeContext<'a> {
    pub(crate) vm: &'a mut super::VmInner,
}

/// The value slot of a property: either a data value or an accessor pair.
#[derive(Clone, Copy, Debug)]
pub enum PropertyValue {
    /// A plain data value.
    Data(JsValue),
    /// An accessor property with optional getter/setter functions.
    Accessor {
        getter: Option<ObjectId>,
        setter: Option<ObjectId>,
    },
}

/// A property descriptor on an object.
#[derive(Clone, Copy, Debug)]
pub struct Property {
    pub slot: PropertyValue,
    /// Only meaningful for `PropertyValue::Data`.
    pub writable: bool,
    pub enumerable: bool,
    pub configurable: bool,
}

impl Property {
    /// Create a writable, enumerable, configurable data property.
    pub fn data(value: JsValue) -> Self {
        Self {
            slot: PropertyValue::Data(value),
            writable: true,
            enumerable: true,
            configurable: true,
        }
    }

    /// Create a non-enumerable, non-configurable, non-writable property (for built-ins).
    pub fn builtin(value: JsValue) -> Self {
        Self {
            slot: PropertyValue::Data(value),
            writable: false,
            enumerable: false,
            configurable: false,
        }
    }

    /// Create a writable, non-enumerable, configurable property (for built-in methods).
    pub fn method(value: JsValue) -> Self {
        Self {
            slot: PropertyValue::Data(value),
            writable: true,
            enumerable: false,
            configurable: true,
        }
    }

    /// Create a configurable accessor property with the given enumerability.
    pub fn accessor(getter: Option<ObjectId>, setter: Option<ObjectId>, enumerable: bool) -> Self {
        Self {
            slot: PropertyValue::Accessor { getter, setter },
            writable: false,
            enumerable,
            configurable: true,
        }
    }

    /// Return the data value, or `Undefined` for accessor properties.
    pub fn data_value(&self) -> JsValue {
        match self.slot {
            PropertyValue::Data(v) => v,
            PropertyValue::Accessor { .. } => JsValue::Undefined,
        }
    }

    /// Extract `PropertyAttrs` from this property descriptor.
    pub fn attrs(&self) -> super::shape::PropertyAttrs {
        super::shape::PropertyAttrs {
            writable: self.writable,
            enumerable: self.enumerable,
            configurable: self.configurable,
            is_accessor: matches!(self.slot, PropertyValue::Accessor { .. }),
        }
    }

    /// Construct a `Property` from a `PropertyValue` and `PropertyAttrs`.
    pub fn from_attrs(value: PropertyValue, attrs: super::shape::PropertyAttrs) -> Self {
        Self {
            slot: value,
            writable: attrs.writable,
            enumerable: attrs.enumerable,
            configurable: attrs.configurable,
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
    /// Extended TDZ bits for functions with > 64 locals (rare).
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
    /// Saved `completion_value` from the parent scope, restored on return.
    pub saved_completion: JsValue,
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

/// State for a string iterator (yields individual code points).
pub struct StringIterState {
    /// The interned string being iterated (avoids O(n) clone).
    pub string_id: StringId,
    /// Current UTF-16 index.
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
