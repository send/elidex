//! Stack-based bytecode VM for elidex-js (Stage 2).
//!
//! All JS values are handle-based: strings and objects are indices into
//! VM-owned tables. `JsValue` is `Copy`, and the `Vm` is naturally `Send`.

pub mod coerce;
mod dispatch;
mod dispatch_helpers;
mod dispatch_iter;
mod dispatch_objects;
mod globals;
pub mod interpreter;
mod natives;
mod natives_boolean;
mod natives_number;
mod natives_regexp;
mod natives_string;
mod natives_symbol;
mod ops;
pub mod value;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use value::{
    CallFrame, FuncId, JsValue, NativeContext, NativeFunction, Object, ObjectId, ObjectKind,
    StringId, SymbolId, SymbolRecord, UpvalueId, VmError,
};

use crate::wtf16::Wtf16Interner;

use crate::bytecode::compiled::CompiledFunction;

/// Function pointer type for native (Rust-implemented) JS functions.
type NativeFn = fn(&mut NativeContext<'_>, JsValue, &[JsValue]) -> Result<JsValue, VmError>;

// ---------------------------------------------------------------------------
// StringPool
// ---------------------------------------------------------------------------

/// Interned string pool backed by a WTF-16 contiguous buffer. All runtime
/// strings are stored here and referenced by `StringId`. Deduplication
/// ensures that property-name comparisons are O(1) integer equality.
pub struct StringPool(Wtf16Interner);

impl StringPool {
    fn new() -> Self {
        Self(Wtf16Interner::new())
    }

    /// Intern a string from UTF-8, returning its `StringId`.
    pub fn intern(&mut self, s: &str) -> StringId {
        StringId(self.0.intern(s))
    }

    /// Intern a string from raw WTF-16 code units.
    pub fn intern_utf16(&mut self, units: &[u16]) -> StringId {
        StringId(self.0.intern_wtf16(units))
    }

    /// Look up a string by its ID, returning WTF-16 code units.
    #[inline]
    pub fn get(&self, id: StringId) -> &[u16] {
        self.0.get(id.0)
    }

    /// Check if a string is already interned (without inserting), returning
    /// its `StringId` if found. O(1) hash lookup.
    pub fn lookup(&self, s: &str) -> Option<StringId> {
        let units: Vec<u16> = s.encode_utf16().collect();
        self.0.lookup_wtf16(&units).map(StringId)
    }

    /// Look up a string by its ID, returning a UTF-8 `String` (lossy for
    /// lone surrogates).
    pub fn get_utf8(&self, id: StringId) -> String {
        self.0.get_utf8(id.0)
    }

    /// Returns the number of interned strings.
    #[allow(dead_code, clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

// ---------------------------------------------------------------------------
// Vm (public wrapper) + VmInner (internal state)
// ---------------------------------------------------------------------------

/// The internal state of the VM, exposed to native functions via `NativeContext`.
pub(crate) struct VmInner {
    pub(crate) stack: Vec<JsValue>,
    pub(crate) frames: Vec<CallFrame>,
    pub(crate) strings: StringPool,
    pub(crate) objects: Vec<Option<Object>>,
    pub(crate) free_objects: Vec<u32>,
    pub(crate) compiled_functions: Vec<CompiledFunction>,
    pub(crate) upvalues: Vec<value::Upvalue>,
    pub(crate) free_upvalues: Vec<u32>,
    pub(crate) globals: HashMap<StringId, JsValue>,
    /// Symbol table: indexed by `SymbolId`.
    pub(crate) symbols: Vec<SymbolRecord>,
    /// Global Symbol registry for `Symbol.for()` / `Symbol.keyFor()`.
    pub(crate) symbol_registry: HashMap<StringId, SymbolId>,
    /// Reverse map for `Symbol.keyFor()`: O(1) lookup from SymbolId → key.
    pub(crate) symbol_reverse_registry: HashMap<SymbolId, StringId>,
    /// Well-known interned strings (cached for fast lookup).
    pub(crate) well_known: WellKnownStrings,
    /// Well-known symbols (cached for fast property lookup).
    pub(crate) well_known_symbols: WellKnownSymbols,
    /// String.prototype object: methods like charAt, indexOf, etc.
    pub(crate) string_prototype: Option<ObjectId>,
    /// Symbol.prototype object: toString, etc.
    pub(crate) symbol_prototype: Option<ObjectId>,
    /// Object.prototype (root of the prototype chain for ordinary objects).
    pub(crate) object_prototype: Option<ObjectId>,
    /// Array.prototype (prototype for array instances).
    pub(crate) array_prototype: Option<ObjectId>,
    /// Number.prototype (prototype for number wrapper objects / primitive access).
    pub(crate) number_prototype: Option<ObjectId>,
    /// Boolean.prototype (prototype for boolean wrapper objects / primitive access).
    pub(crate) boolean_prototype: Option<ObjectId>,
    /// RegExp.prototype (prototype for RegExp instances).
    pub(crate) regexp_prototype: Option<ObjectId>,
    /// Shared prototype for array iterator objects (next + @@iterator).
    pub(crate) array_iterator_prototype: Option<ObjectId>,
    /// Shared prototype for string iterator objects (next + @@iterator).
    pub(crate) string_iterator_prototype: Option<ObjectId>,
    /// The global object (`globalThis`). Used for `this` coercion in
    /// non-strict functions (§9.2.1.2).
    pub(crate) global_object: ObjectId,
    /// Completion value for eval: the last value popped by a Pop opcode
    /// at the script (entry) frame level.
    pub(crate) completion_value: JsValue,
    /// The most recently thrown/caught exception value (for PushException).
    pub(crate) current_exception: JsValue,
    /// xorshift64 PRNG state for `Math.random()`.
    pub(crate) rng_state: u64,
}

/// Frequently used interned string IDs, cached at VM creation.
#[allow(dead_code)] // Fields used by interpreter and future built-ins.
pub(crate) struct WellKnownStrings {
    pub(crate) undefined: StringId,
    pub(crate) null: StringId,
    pub(crate) r#true: StringId,
    pub(crate) r#false: StringId,
    pub(crate) nan: StringId,
    pub(crate) infinity: StringId,
    pub(crate) neg_infinity: StringId,
    pub(crate) zero: StringId,
    pub(crate) empty: StringId,
    pub(crate) prototype: StringId,
    pub(crate) constructor: StringId,
    pub(crate) length: StringId,
    pub(crate) name: StringId,
    pub(crate) message: StringId,
    pub(crate) log: StringId,
    pub(crate) error: StringId,
    pub(crate) warn: StringId,
    pub(crate) object_type: StringId,
    pub(crate) boolean_type: StringId,
    pub(crate) number_type: StringId,
    pub(crate) string_type: StringId,
    pub(crate) function_type: StringId,
    pub(crate) symbol_type: StringId,
    pub(crate) object_to_string: StringId,
    pub(crate) next: StringId,
    pub(crate) value: StringId,
    pub(crate) done: StringId,
    pub(crate) return_str: StringId,
}

/// Well-known symbol IDs, allocated at VM creation.
#[allow(dead_code)]
pub(crate) struct WellKnownSymbols {
    pub(crate) iterator: SymbolId,
    pub(crate) async_iterator: SymbolId,
    pub(crate) has_instance: SymbolId,
    pub(crate) to_primitive: SymbolId,
    pub(crate) to_string_tag: SymbolId,
    pub(crate) species: SymbolId,
    pub(crate) is_concat_spreadable: SymbolId,
}

impl VmInner {
    /// Allocate a new symbol, returning its `SymbolId`.
    pub(crate) fn alloc_symbol(&mut self, description: Option<StringId>) -> SymbolId {
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(SymbolRecord { description });
        id
    }

    /// Allocate an object, returning its `ObjectId`.
    pub(crate) fn alloc_object(&mut self, obj: Object) -> ObjectId {
        if let Some(idx) = self.free_objects.pop() {
            self.objects[idx as usize] = Some(obj);
            ObjectId(idx)
        } else {
            let id = ObjectId(self.objects.len() as u32);
            self.objects.push(Some(obj));
            id
        }
    }

    /// Get a reference to an object.
    ///
    /// # Panics
    /// Panics if the object has been freed.
    #[inline]
    pub(crate) fn get_object(&self, id: ObjectId) -> &Object {
        self.objects[id.0 as usize]
            .as_ref()
            .expect("object already freed")
    }

    /// Get a mutable reference to an object.
    ///
    /// # Panics
    /// Panics if the object has been freed.
    #[inline]
    pub(crate) fn get_object_mut(&mut self, id: ObjectId) -> &mut Object {
        self.objects[id.0 as usize]
            .as_mut()
            .expect("object already freed")
    }

    // -- Compiled functions --------------------------------------------------

    /// Register a compiled function in the VM, returning its `FuncId`.
    pub(crate) fn register_function(&mut self, func: CompiledFunction) -> FuncId {
        let id = FuncId(self.compiled_functions.len() as u32);
        self.compiled_functions.push(func);
        id
    }

    /// Get a reference to a compiled function.
    #[inline]
    pub(crate) fn get_compiled(&self, id: FuncId) -> &CompiledFunction {
        &self.compiled_functions[id.0 as usize]
    }

    // -- Upvalues ------------------------------------------------------------

    /// Allocate an upvalue, returning its `UpvalueId`.
    pub(crate) fn alloc_upvalue(&mut self, uv: value::Upvalue) -> UpvalueId {
        if let Some(idx) = self.free_upvalues.pop() {
            self.upvalues[idx as usize] = uv;
            UpvalueId(idx)
        } else {
            let id = UpvalueId(self.upvalues.len() as u32);
            self.upvalues.push(uv);
            id
        }
    }

    // -- Native function helpers ---------------------------------------------

    /// Helper: create a native function object (non-constructable by default,
    /// matching the ES2020 spec for most built-in functions).
    pub(crate) fn create_native_function(
        &mut self,
        name: &str,
        func: fn(&mut NativeContext<'_>, JsValue, &[JsValue]) -> Result<JsValue, VmError>,
    ) -> ObjectId {
        let name_id = self.strings.intern(name);
        self.alloc_object(Object {
            kind: ObjectKind::NativeFunction(NativeFunction {
                name: name_id,
                func,
                constructable: false,
            }),
            properties: Vec::new(),
            prototype: None,
        })
    }

    /// Helper: create a constructable native function object (for Error, etc.).
    pub(crate) fn create_constructable_function(
        &mut self,
        name: &str,
        func: fn(&mut NativeContext<'_>, JsValue, &[JsValue]) -> Result<JsValue, VmError>,
    ) -> ObjectId {
        let name_id = self.strings.intern(name);
        self.alloc_object(Object {
            kind: ObjectKind::NativeFunction(NativeFunction {
                name: name_id,
                func,
                constructable: true,
            }),
            properties: Vec::new(),
            prototype: None,
        })
    }

    /// Resolve a `PropertyValue` slot to a `JsValue`, invoking the getter
    /// if the slot is an accessor.
    pub(crate) fn resolve_slot(
        &mut self,
        slot: value::PropertyValue,
        this: JsValue,
    ) -> Result<JsValue, VmError> {
        match slot {
            value::PropertyValue::Data(v) => Ok(v),
            value::PropertyValue::Accessor {
                getter: Some(g), ..
            } => self.call(g, this, &[]),
            value::PropertyValue::Accessor { getter: None, .. } => Ok(JsValue::Undefined),
        }
    }

    /// Perform a fresh `Get` (§7.3.1) on an object by `PropertyKey`.
    pub(crate) fn get_property_value(
        &mut self,
        obj_id: value::ObjectId,
        key: value::PropertyKey,
    ) -> Result<JsValue, VmError> {
        let result = coerce::get_property(self, obj_id, key);
        match result {
            Some(coerce::PropertyResult::Data(v)) => Ok(v),
            Some(coerce::PropertyResult::Getter(g)) => self.call(g, JsValue::Object(obj_id), &[]),
            None => Ok(JsValue::Undefined),
        }
    }
}

/// The elidex-js bytecode VM.
///
/// Persistent across `eval` calls: globals, object heap, and interned strings
/// survive between evaluations.
pub struct Vm {
    pub(crate) inner: VmInner,
}

impl Vm {
    /// Create a new VM with built-in globals registered.
    pub fn new() -> Self {
        let mut strings = StringPool::new();

        let well_known = WellKnownStrings {
            undefined: strings.intern("undefined"),
            null: strings.intern("null"),
            r#true: strings.intern("true"),
            r#false: strings.intern("false"),
            nan: strings.intern("NaN"),
            infinity: strings.intern("Infinity"),
            neg_infinity: strings.intern("-Infinity"),
            zero: strings.intern("0"),
            empty: strings.intern(""),
            prototype: strings.intern("prototype"),
            constructor: strings.intern("constructor"),
            length: strings.intern("length"),
            name: strings.intern("name"),
            message: strings.intern("message"),
            log: strings.intern("log"),
            error: strings.intern("error"),
            warn: strings.intern("warn"),
            object_type: strings.intern("object"),
            boolean_type: strings.intern("boolean"),
            number_type: strings.intern("number"),
            string_type: strings.intern("string"),
            function_type: strings.intern("function"),
            symbol_type: strings.intern("symbol"),
            object_to_string: strings.intern("[object Object]"),
            next: strings.intern("next"),
            value: strings.intern("value"),
            done: strings.intern("done"),
            return_str: strings.intern("return"),
        };

        // Allocate well-known symbols (fixed IDs 0-6).
        let mut symbols = Vec::new();
        let mut alloc_wk = |desc: &str| -> SymbolId {
            let id = SymbolId(symbols.len() as u32);
            symbols.push(SymbolRecord {
                description: Some(strings.intern(desc)),
            });
            id
        };
        let well_known_symbols = WellKnownSymbols {
            iterator: alloc_wk("Symbol.iterator"),
            async_iterator: alloc_wk("Symbol.asyncIterator"),
            has_instance: alloc_wk("Symbol.hasInstance"),
            to_primitive: alloc_wk("Symbol.toPrimitive"),
            to_string_tag: alloc_wk("Symbol.toStringTag"),
            species: alloc_wk("Symbol.species"),
            is_concat_spreadable: alloc_wk("Symbol.isConcatSpreadable"),
        };

        let mut vm = Vm {
            inner: VmInner {
                stack: Vec::with_capacity(256),
                frames: Vec::with_capacity(16),
                strings,
                objects: Vec::new(),
                free_objects: Vec::new(),
                compiled_functions: Vec::new(),
                upvalues: Vec::new(),
                free_upvalues: Vec::new(),
                globals: HashMap::new(),
                symbols,
                symbol_registry: HashMap::new(),
                symbol_reverse_registry: HashMap::new(),
                well_known,
                well_known_symbols,
                string_prototype: None,
                symbol_prototype: None,
                object_prototype: None,
                array_prototype: None,
                number_prototype: None,
                boolean_prototype: None,
                regexp_prototype: None,
                array_iterator_prototype: None,
                string_iterator_prototype: None,
                // Placeholder — immediately replaced by register_globals().
                global_object: ObjectId(0),
                completion_value: JsValue::Undefined,
                current_exception: JsValue::Undefined,
                rng_state: {
                    // Seed from OS-RNG via RandomState so each Vm gets a
                    // unique sequence without requiring `rand`.
                    use std::collections::hash_map::RandomState;
                    use std::hash::{BuildHasher, Hasher};
                    let mut hasher = RandomState::new().build_hasher();
                    hasher.write_u64(0);
                    let seed = hasher.finish();
                    // Ensure non-zero (xorshift64 fixpoint).
                    if seed == 0 {
                        1
                    } else {
                        seed
                    }
                },
            },
        };

        vm.inner.register_globals();
        vm
    }

    // -- Public API: all delegate to VmInner --------------------------------

    /// Parse, compile, and execute JavaScript source code.
    pub fn eval(&mut self, source: &str) -> Result<JsValue, VmError> {
        self.inner.eval(source)
    }

    /// Load and execute a compiled script.
    pub fn run_script(
        &mut self,
        script: crate::bytecode::compiled::CompiledScript,
    ) -> Result<JsValue, VmError> {
        self.inner.run_script(script)
    }

    /// Call a JS function object with the given `this` and arguments.
    pub fn call(
        &mut self,
        func_obj_id: ObjectId,
        this: JsValue,
        args: &[JsValue],
    ) -> Result<JsValue, VmError> {
        self.inner.call(func_obj_id, this, args)
    }

    /// Intern a string, returning its `StringId`.
    #[inline]
    pub fn intern(&mut self, s: &str) -> StringId {
        self.inner.strings.intern(s)
    }

    /// Look up an interned string by its ID, returning WTF-16 code units.
    #[inline]
    pub fn get_string_u16(&self, id: StringId) -> &[u16] {
        self.inner.strings.get(id)
    }

    /// Look up an interned string by its ID, returning a UTF-8 `String`.
    #[inline]
    pub fn get_string(&self, id: StringId) -> String {
        self.inner.strings.get_utf8(id)
    }

    /// Allocate an object, returning its `ObjectId`.
    pub fn alloc_object(&mut self, obj: Object) -> ObjectId {
        self.inner.alloc_object(obj)
    }

    /// Get a reference to an object.
    #[inline]
    pub fn get_object(&self, id: ObjectId) -> &Object {
        self.inner.get_object(id)
    }

    /// Get a mutable reference to an object.
    #[inline]
    pub fn get_object_mut(&mut self, id: ObjectId) -> &mut Object {
        self.inner.get_object_mut(id)
    }

    /// Register a compiled function in the VM, returning its `FuncId`.
    pub fn register_function(&mut self, func: CompiledFunction) -> FuncId {
        self.inner.register_function(func)
    }

    /// Get a reference to a compiled function.
    #[inline]
    pub fn get_compiled(&self, id: FuncId) -> &CompiledFunction {
        self.inner.get_compiled(id)
    }

    /// Allocate an upvalue, returning its `UpvalueId`.
    pub fn alloc_upvalue(&mut self, uv: value::Upvalue) -> UpvalueId {
        self.inner.alloc_upvalue(uv)
    }

    /// Set a global variable.
    pub fn set_global(&mut self, name: &str, value: JsValue) {
        let id = self.inner.strings.intern(name);
        self.inner.globals.insert(id, value);
    }

    /// Get a global variable.
    pub fn get_global(&self, name: &str) -> Option<JsValue> {
        let sid = self.inner.strings.lookup(name)?;
        self.inner.globals.get(&sid).copied()
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Native function helpers
// ---------------------------------------------------------------------------

impl NativeContext<'_> {
    /// Intern a string from UTF-8.
    #[inline]
    pub fn intern(&mut self, s: &str) -> StringId {
        self.vm.strings.intern(s)
    }

    /// Intern a string from raw WTF-16 code units.
    #[inline]
    pub fn intern_utf16(&mut self, units: &[u16]) -> StringId {
        self.vm.strings.intern_utf16(units)
    }

    /// Look up an interned string as WTF-16.
    #[inline]
    pub fn get_u16(&self, id: StringId) -> &[u16] {
        self.vm.strings.get(id)
    }

    /// Look up an interned string as UTF-8 (lossy for lone surrogates).
    #[inline]
    pub fn get_utf8(&self, id: StringId) -> String {
        self.vm.strings.get_utf8(id)
    }

    /// Allocate an object.
    pub fn alloc_object(&mut self, obj: Object) -> ObjectId {
        self.vm.alloc_object(obj)
    }

    /// Get a reference to an object.
    #[inline]
    pub fn get_object(&self, id: ObjectId) -> &Object {
        self.vm.get_object(id)
    }

    /// Get a mutable reference to an object.
    #[inline]
    pub fn get_object_mut(&mut self, id: ObjectId) -> &mut Object {
        self.vm.get_object_mut(id)
    }

    /// Convert a value to f64 using ES2020 ToNumber.
    /// Returns `Err(VmError)` for Symbol values (ES2020 §7.1.4).
    #[inline]
    pub fn to_number(&self, val: JsValue) -> Result<f64, VmError> {
        coerce::to_number(self.vm, val)
    }

    /// Convert a value to an interned string using ES2020 ToString.
    /// Returns `Err(VmError)` for Symbol values (ES2020 §7.1.12).
    #[inline]
    pub fn to_string_val(&mut self, val: JsValue) -> Result<StringId, VmError> {
        coerce::to_string(self.vm, val)
    }

    /// Convert a value to bool using ES2020 ToBoolean.
    #[inline]
    pub fn to_boolean(&self, val: JsValue) -> bool {
        coerce::to_boolean(self.vm, val)
    }

    /// Call a JS function object by `ObjectId` (e.g. getter/setter invoke).
    ///
    /// Enables native functions to call back into the VM for re-entrant
    /// execution (e.g. invoking accessor getters from `Object.values`).
    pub fn call_function(
        &mut self,
        callee: ObjectId,
        this: JsValue,
        args: &[JsValue],
    ) -> Result<JsValue, VmError> {
        self.vm.call(callee, this, args)
    }

    /// Call a value as a function (type-checked: must be an object).
    pub fn call_value(
        &mut self,
        callee: JsValue,
        this: JsValue,
        args: &[JsValue],
    ) -> Result<JsValue, VmError> {
        self.vm.call_value(callee, this, args)
    }

    /// Resolve a `PropertyValue` slot to a `JsValue`, invoking the getter
    /// if the slot is an accessor.
    pub fn resolve_slot(
        &mut self,
        slot: value::PropertyValue,
        this: JsValue,
    ) -> Result<JsValue, VmError> {
        self.vm.resolve_slot(slot, this)
    }

    /// Perform a fresh `Get` (§7.3.1) on an object by `PropertyKey`.
    /// Looks up the property (own + prototype chain), resolves accessors.
    /// Returns `JsValue::Undefined` when the property does not exist.
    pub fn get_property_value(
        &mut self,
        obj_id: value::ObjectId,
        key: value::PropertyKey,
    ) -> Result<JsValue, VmError> {
        self.vm.get_property_value(obj_id, key)
    }
}
