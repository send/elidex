//! Stack-based bytecode VM for elidex-js (Stage 2).
//!
//! All JS values are handle-based: strings and objects are indices into
//! VM-owned tables. `JsValue` is `Copy`, and the `Vm` is naturally `Send`.

pub mod coerce;
mod dispatch;
mod globals;
pub mod interpreter;
mod natives;
mod ops;
pub mod value;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use value::{
    CallFrame, FuncId, JsValue, NativeContext, NativeFunction, Object, ObjectId, ObjectKind,
    StringId, UpvalueId, VmError,
};

use crate::bytecode::compiled::CompiledFunction;

/// Function pointer type for native (Rust-implemented) JS functions.
type NativeFn = fn(&mut NativeContext<'_>, JsValue, &[JsValue]) -> Result<JsValue, VmError>;

// ---------------------------------------------------------------------------
// StringPool
// ---------------------------------------------------------------------------

/// Interned string pool. All runtime strings are stored here and referenced
/// by `StringId`. Deduplication ensures that property-name comparisons are
/// O(1) integer equality.
pub struct StringPool {
    /// Indexed by `StringId`.
    strings: Vec<Box<str>>,
    /// Reverse map for interning (dedup).
    intern_map: HashMap<Box<str>, StringId>,
}

impl StringPool {
    fn new() -> Self {
        Self {
            strings: Vec::new(),
            intern_map: HashMap::new(),
        }
    }

    /// Intern a string, returning its `StringId`. If the string was already
    /// interned, the existing ID is returned.
    pub fn intern(&mut self, s: &str) -> StringId {
        if let Some(&id) = self.intern_map.get(s) {
            return id;
        }
        let id = StringId(self.strings.len() as u32);
        let boxed: Box<str> = s.into();
        self.intern_map.insert(boxed.clone(), id);
        self.strings.push(boxed);
        id
    }

    /// Look up a string by its ID.
    #[inline]
    pub fn get(&self, id: StringId) -> &str {
        &self.strings[id.0 as usize]
    }

    /// Returns the number of interned strings.
    #[allow(dead_code, clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.strings.len()
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
    /// Well-known interned strings (cached for fast lookup).
    pub(crate) well_known: WellKnownStrings,
    /// String.prototype object: methods like charAt, indexOf, etc.
    pub(crate) string_prototype: Option<ObjectId>,
    /// Completion value for eval: the last value popped by a Pop opcode
    /// at the script (entry) frame level.
    pub(crate) completion_value: JsValue,
    /// The most recently thrown/caught exception value (for PushException).
    pub(crate) current_exception: JsValue,
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
    pub(crate) object_to_string: StringId,
}

impl VmInner {
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
            object_to_string: strings.intern("[object Object]"),
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
                well_known,
                string_prototype: None,
                completion_value: JsValue::Undefined,
                current_exception: JsValue::Undefined,
            },
        };

        vm.register_globals();
        vm
    }

    // -- String pool ---------------------------------------------------------

    /// Intern a string, returning its `StringId`.
    #[inline]
    pub fn intern(&mut self, s: &str) -> StringId {
        self.inner.strings.intern(s)
    }

    /// Look up an interned string by its ID.
    #[inline]
    pub fn get_string(&self, id: StringId) -> &str {
        self.inner.strings.get(id)
    }

    // -- Object heap ---------------------------------------------------------

    /// Allocate an object, returning its `ObjectId`.
    pub fn alloc_object(&mut self, obj: Object) -> ObjectId {
        self.inner.alloc_object(obj)
    }

    /// Get a reference to an object.
    ///
    /// # Panics
    /// Panics if the object has been freed.
    #[inline]
    pub fn get_object(&self, id: ObjectId) -> &Object {
        self.inner.get_object(id)
    }

    /// Get a mutable reference to an object.
    ///
    /// # Panics
    /// Panics if the object has been freed.
    #[inline]
    pub fn get_object_mut(&mut self, id: ObjectId) -> &mut Object {
        self.inner.get_object_mut(id)
    }

    // -- Compiled functions --------------------------------------------------

    /// Register a compiled function in the VM, returning its `FuncId`.
    pub fn register_function(&mut self, func: CompiledFunction) -> FuncId {
        let id = FuncId(self.inner.compiled_functions.len() as u32);
        self.inner.compiled_functions.push(func);
        id
    }

    /// Get a reference to a compiled function.
    #[inline]
    pub fn get_compiled(&self, id: FuncId) -> &CompiledFunction {
        &self.inner.compiled_functions[id.0 as usize]
    }

    // -- Upvalues ------------------------------------------------------------

    /// Allocate an upvalue, returning its `UpvalueId`.
    pub fn alloc_upvalue(&mut self, uv: value::Upvalue) -> UpvalueId {
        if let Some(idx) = self.inner.free_upvalues.pop() {
            self.inner.upvalues[idx as usize] = uv;
            UpvalueId(idx)
        } else {
            let id = UpvalueId(self.inner.upvalues.len() as u32);
            self.inner.upvalues.push(uv);
            id
        }
    }

    // -- Globals -------------------------------------------------------------

    /// Set a global variable.
    pub fn set_global(&mut self, name: &str, value: JsValue) {
        let id = self.inner.strings.intern(name);
        self.inner.globals.insert(id, value);
    }

    /// Get a global variable.
    pub fn get_global(&self, name: &str) -> Option<JsValue> {
        // Linear lookup through intern_map — only used by external callers,
        // not the hot interpreter path (which uses StringId directly).
        self.inner
            .strings
            .intern_map
            .get(name)
            .and_then(|id| self.inner.globals.get(id).copied())
    }

    /// Helper: create a native function object.
    pub(crate) fn create_native_function(
        &mut self,
        name: &str,
        func: fn(&mut NativeContext<'_>, JsValue, &[JsValue]) -> Result<JsValue, VmError>,
    ) -> ObjectId {
        let name_id = self.inner.strings.intern(name);
        self.alloc_object(Object {
            kind: ObjectKind::NativeFunction(NativeFunction {
                name: name_id,
                func,
            }),
            properties: Vec::new(),
            prototype: None,
        })
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
    /// Intern a string.
    #[inline]
    pub fn intern(&mut self, s: &str) -> StringId {
        self.vm.strings.intern(s)
    }

    /// Look up an interned string.
    #[inline]
    pub fn get_string(&self, id: StringId) -> &str {
        self.vm.strings.get(id)
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
    #[inline]
    pub fn to_number(&self, val: JsValue) -> f64 {
        coerce::to_number(self.vm, val)
    }

    /// Convert a value to an interned string using ES2020 ToString.
    #[inline]
    pub fn to_string_val(&mut self, val: JsValue) -> StringId {
        coerce::to_string(self.vm, val)
    }

    /// Convert a value to bool using ES2020 ToBoolean.
    #[inline]
    pub fn to_boolean(&self, val: JsValue) -> bool {
        coerce::to_boolean(self.vm, val)
    }
}
