//! JS value types for the elidex-js VM.
//!
//! All values are handle-based: strings and objects are indices into
//! VM-owned tables, making `JsValue` `Copy` and trivially `Send + Sync`.

use std::fmt;
use std::sync::Arc;

// Coroutine runtime types live in `coroutine_types.rs` (split out to keep
// this file under the 1000-line convention).  Re-exported here so the
// ObjectKind variants + external callers resolve them via `value::` paths.
pub use super::coroutine_types::*;

// `VmError` / `VmErrorKind` live in `vm/error.rs` for the same reason.
// Re-exported here so the long-established `value::VmError` import path
// keeps working for downstream code without churn.
pub use super::error::{VmError, VmErrorKind};

// `ObjectKind` lives in `vm/object_kind.rs` (split out for the same
// 1000-line reason).  Re-exported so `value::ObjectKind` keeps
// resolving for the dozens of host modules and tests that import it.
pub use super::object_kind::ObjectKind;

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

/// Index into `VmInner::bigints`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BigIntId(pub(crate) u32);

impl fmt::Debug for BigIntId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BigIntId({})", self.0)
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
    /// Sparse array hole sentinel — never observable from JS code.
    /// Reading a hole returns `Undefined`; the distinction matters for
    /// `in`, `for-in`, `delete`, and `JSON.stringify`.
    Empty,
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(StringId),
    Symbol(SymbolId),
    BigInt(BigIntId),
    Object(ObjectId),
}

impl JsValue {
    /// Returns `true` for the array-hole sentinel.
    #[inline]
    pub fn is_empty(self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Convert a sparse hole to `Undefined`. Used at all read boundaries
    /// so that `Empty` never leaks to JS code.
    #[inline]
    #[must_use]
    pub fn or_undefined(self) -> Self {
        if self.is_empty() {
            Self::Undefined
        } else {
            self
        }
    }

    /// Extract the `f64` payload if this is a `Number`.
    #[inline]
    pub fn as_number(self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(n),
            _ => None,
        }
    }

    /// Returns `true` if the value is `undefined` or `null`.
    /// Empty is treated as nullish as a safety net.
    #[inline]
    pub fn is_nullish(self) -> bool {
        matches!(self, Self::Empty | Self::Undefined | Self::Null)
    }

    /// Returns `true` if the value is the boolean `false`, numeric `0`/`NaN`,
    /// `null`, `undefined`, or the empty string. The empty-string check
    /// requires access to the string pool and is handled in `Vm::to_boolean`.
    #[inline]
    pub fn is_primitive_falsy(self) -> bool {
        match self {
            Self::Empty | Self::Undefined | Self::Null => true,
            Self::Boolean(b) => !b,
            Self::Number(n) => n == 0.0 || n.is_nan(),
            // BigIntPool guarantees canonical 0n at BigIntId(0).
            Self::BigInt(id) => id.0 == 0,
            Self::String(_) | Self::Symbol(_) | Self::Object(_) => false,
        }
    }
}

impl PartialEq for JsValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty, Self::Empty)
            | (Self::Undefined, Self::Undefined)
            | (Self::Null, Self::Null) => true,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => {
                // JS strict equality: NaN !== NaN, +0 === -0
                a == b
            }
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Symbol(a), Self::Symbol(b)) => a == b,
            (Self::BigInt(a), Self::BigInt(b)) => a == b,
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
    /// Whether new properties can be added (§9.1.1). Starts `true`; set to
    /// `false` by `Object.preventExtensions`, `Object.seal`, `Object.freeze`.
    pub extensible: bool,
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

/// Element type discriminator for `ObjectKind::TypedArray` (ES2024 §23.2).
///
/// Each variant identifies both the in-memory byte layout (`bytes_per_element`)
/// and the JS-visible element-value domain (integer / float / BigInt, signed /
/// unsigned / clamped).  The enum is unconditional (no `#[cfg]` gate) so
/// `ObjectKind::TypedArray` stays in the always-available value model as
/// well.  TypedArray-specific feature-flagged logic (host methods, Fetch
/// body init, etc.) gates at its own call sites, not on this type.
///
/// Byte ordering for TypedArray indexed reads / writes is **little-endian
/// unconditionally** — an elidex implementation choice for cross-platform
/// determinism.  `IsLittleEndian()` (ES §25.1.3.1) is implementation-defined
/// and spec-compliant for any constant choice.  `DataView` exposes both
/// endiannesses explicitly via the `littleEndian` argument (ES §25.3.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElementKind {
    Int8,
    Uint8,
    Uint8Clamped,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Float32,
    Float64,
    BigInt64,
    BigUint64,
}

impl ElementKind {
    /// Total number of TypedArray subclasses (ES §23.2 table).
    /// Sized for the per-subclass prototype array stored on
    /// `VmInner` and the `proto_roots` GC slice that mirrors it.
    pub const COUNT: usize = 11;

    /// Stable 0-based index in [`Self::COUNT`] range, used to
    /// address per-subclass tables (per-subclass prototypes,
    /// future per-subclass install flags) without relying on the
    /// implicit enum discriminant.  The mapping is fixed: any
    /// future variant must be appended at the end and bump
    /// [`Self::COUNT`].
    #[inline]
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Int8 => 0,
            Self::Uint8 => 1,
            Self::Uint8Clamped => 2,
            Self::Int16 => 3,
            Self::Uint16 => 4,
            Self::Int32 => 5,
            Self::Uint32 => 6,
            Self::Float32 => 7,
            Self::Float64 => 8,
            Self::BigInt64 => 9,
            Self::BigUint64 => 10,
        }
    }

    /// Byte width of one element — `[[ElementSize]]` per ES §23.2.1 table.
    #[inline]
    #[must_use]
    pub const fn bytes_per_element(self) -> u8 {
        match self {
            Self::Int8 | Self::Uint8 | Self::Uint8Clamped => 1,
            Self::Int16 | Self::Uint16 => 2,
            Self::Int32 | Self::Uint32 | Self::Float32 => 4,
            Self::Float64 | Self::BigInt64 | Self::BigUint64 => 8,
        }
    }

    /// `true` when elements are BigInt (i.e. `BigInt64Array` /
    /// `BigUint64Array`).  Used at the indexed-write call site to route
    /// coercion through `ToBigInt64` / `ToBigUint64` instead of `ToIntXx`
    /// (ES §7.1.15 / .16 vs §7.1.6-.11).
    #[inline]
    #[must_use]
    pub const fn is_bigint(self) -> bool {
        matches!(self, Self::BigInt64 | Self::BigUint64)
    }

    /// Spec-shaped subclass name (e.g. `"Uint8Array"`) used as the
    /// `[[TypedArrayName]]` slot value returned by
    /// `%TypedArray%.prototype[@@toStringTag]` (ES §23.2.3.32).
    #[inline]
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Int8 => "Int8Array",
            Self::Uint8 => "Uint8Array",
            Self::Uint8Clamped => "Uint8ClampedArray",
            Self::Int16 => "Int16Array",
            Self::Uint16 => "Uint16Array",
            Self::Int32 => "Int32Array",
            Self::Uint32 => "Uint32Array",
            Self::Float32 => "Float32Array",
            Self::Float64 => "Float64Array",
            Self::BigInt64 => "BigInt64Array",
            Self::BigUint64 => "BigUint64Array",
        }
    }
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
    /// Only meaningful for `PropertyValue::Data`; ignored for `Accessor`
    /// (§6.2.5.1: accessor descriptors have no `[[Writable]]`).
    pub writable: bool,
    /// Meaningful for both Data and Accessor descriptors (§6.2.5).
    pub enumerable: bool,
    /// Meaningful for both Data and Accessor descriptors (§6.2.5).
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
    /// Saved `completion_value` from the parent scope, restored on return.
    pub saved_completion: JsValue,
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

/// Iterator kind for `ArrayIterState`.
/// 0 = Values (default), 1 = Keys, 2 = Entries.
pub type ArrayIterKind = u8;

/// `ArrayIterKind::Values` discriminant. See [`ArrayIterKind`] for
/// variant encoding.
pub const ARRAY_ITER_KIND_VALUES: ArrayIterKind = 0;

/// State for an array/iterable iterator.
pub struct ArrayIterState {
    /// The array being iterated.
    pub array_id: ObjectId,
    /// Current index.
    pub index: usize,
    /// 0 = Values, 1 = Keys, 2 = Entries.
    pub kind: ArrayIterKind,
}

/// State for a string iterator (yields individual code points).
pub struct StringIterState {
    /// The interned string being iterated (avoids O(n) clone).
    pub string_id: StringId,
    /// Current UTF-16 index.
    pub index: usize,
}

// `VmError` / `VmErrorKind` moved to `vm/error.rs` (re-exported above
// so existing `value::VmError` import paths still resolve).
