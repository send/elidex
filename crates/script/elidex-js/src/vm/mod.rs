//! Stack-based bytecode VM for elidex-js (Stage 2).
//!
//! All JS values are handle-based: strings and objects are indices into
//! VM-owned tables. `JsValue` is `Copy`.  Without the `engine` feature the
//! VM is `Send` (pure interpreter); with `engine` enabled, `VmInner`
//! carries `Option<Box<HostData>>` whose raw pointers render `Vm` `!Send`
//! by default — see [`host_data`].

pub mod coerce;
pub(crate) mod coerce_format;
pub(crate) mod coerce_ops;
mod dispatch;
mod dispatch_helpers;
mod dispatch_ic;
mod dispatch_iter;
mod dispatch_objects;
pub(crate) mod gc;
mod globals;
pub mod host_data;
pub(crate) mod ic;
pub mod interpreter;
mod natives;
mod natives_array;
mod natives_array_hof;
mod natives_bigint;
mod natives_boolean;
mod natives_function;
mod natives_json;
mod natives_math;
mod natives_number;
mod natives_object;
mod natives_regexp;
mod natives_string;
mod natives_string_ext;
mod natives_symbol;
mod ops;
mod ops_property;
pub mod pools;
pub(crate) mod shape;
pub mod value;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use pools::{BigIntPool, StringPool};
use value::{
    CallFrame, FuncId, JsValue, NativeContext, NativeFunction, Object, ObjectId, ObjectKind,
    StringId, SymbolId, SymbolRecord, UpvalueId, VmError,
};

use crate::bytecode::compiled::CompiledFunction;

/// Function pointer type for native (Rust-implemented) JS functions.
type NativeFn = fn(&mut NativeContext<'_>, JsValue, &[JsValue]) -> Result<JsValue, VmError>;

/// Maximum `bind()` chain depth before a `RangeError` is thrown.  Prevents
/// O(N²) copy costs and unbounded heap allocation from user-constructed chains.
pub(crate) const MAX_BIND_CHAIN_DEPTH: usize = 10_000;

// ---------------------------------------------------------------------------
// Vm (public wrapper) + VmInner (internal state)
// ---------------------------------------------------------------------------

/// The internal state of the VM, exposed to native functions via `NativeContext`.
pub(crate) struct VmInner {
    pub(crate) stack: Vec<JsValue>,
    pub(crate) frames: Vec<CallFrame>,
    pub(crate) strings: StringPool,
    pub(crate) bigints: BigIntPool,
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
    /// BigInt.prototype (prototype for BigInt primitive access).
    pub(crate) bigint_prototype: Option<ObjectId>,
    /// Function.prototype (prototype for all function objects).
    pub(crate) function_prototype: Option<ObjectId>,
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
    /// Hidden class (Shape) table.  `shapes[0]` is always the root (empty) shape.
    pub(crate) shapes: Vec<shape::Shape>,
    // -- GC state --
    /// Mark bits for objects (one bit per `objects` slot).
    pub(crate) gc_object_marks: Vec<u64>,
    /// Mark bits for upvalues (one bit per `upvalues` slot).
    pub(crate) gc_upvalue_marks: Vec<u64>,
    /// Reusable work list for GC mark phase (avoids per-cycle allocation).
    pub(crate) gc_work_list: Vec<u32>,
    /// Estimated bytes allocated since the last GC cycle.
    pub(crate) gc_bytes_since_last: usize,
    /// Byte threshold for triggering the next collection.
    pub(crate) gc_threshold: usize,
    /// GC enabled flag.  `false` during init and native function calls.
    pub(crate) gc_enabled: bool,
    /// Set while a native function is invoked via `[[Construct]]` (i.e. `new`).
    /// Read by constructors to distinguish `new F(...)` from `F(...)`.
    pub(crate) in_construct: bool,
    /// Host-provided data for browser shell integration (event listeners,
    /// DOM wrappers, timers, etc.).  `None` when the VM runs standalone
    /// (e.g., in unit tests without the `engine` feature).
    pub(crate) host_data: Option<Box<host_data::HostData>>,
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
    pub(crate) bigint_type: StringId,
    pub(crate) object_to_string: StringId,
    pub(crate) next: StringId,
    pub(crate) value: StringId,
    pub(crate) done: StringId,
    pub(crate) return_str: StringId,
    pub(crate) last_index: StringId,
    pub(crate) index: StringId,
    pub(crate) input: StringId,
    pub(crate) join: StringId,
    pub(crate) to_json: StringId,
    pub(crate) get: StringId,
    pub(crate) set: StringId,
    pub(crate) enumerable: StringId,
    pub(crate) configurable: StringId,
    pub(crate) writable: StringId,
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
    ///
    /// May trigger a GC cycle if the allocation pressure threshold is exceeded.
    /// GC runs **before** the new object is placed in the heap, so the new
    /// object cannot be prematurely collected.
    /// Estimated byte cost per object allocation (struct size + inline overhead).
    const OBJECT_ALLOC_ESTIMATE: usize = std::mem::size_of::<Object>() + 64;

    pub(crate) fn alloc_object(&mut self, obj: Object) -> ObjectId {
        // GC trigger BEFORE insertion.  Callers must ensure that any
        // ObjectIds reachable only through `obj`'s fields (prototype,
        // array elements, property slots) are already rooted on the VM
        // stack or otherwise reachable from GC roots.  Prototype ObjectIds
        // from VmInner fields (e.g., `self.object_prototype`) are always
        // rooted.  For complex cases (e.g., `create_closure`, `do_new`),
        // callers temporarily push values onto the stack or disable GC.
        if self.gc_enabled
            && self
                .gc_bytes_since_last
                .saturating_add(Self::OBJECT_ALLOC_ESTIMATE)
                >= self.gc_threshold
        {
            self.collect_garbage();
        }
        // Increment AFTER potential GC so the current allocation is still
        // counted towards the next cycle's threshold.
        self.gc_bytes_since_last += Self::OBJECT_ALLOC_ESTIMATE;

        if let Some(idx) = self.free_objects.pop() {
            self.objects[idx as usize] = Some(obj);
            ObjectId(idx)
        } else {
            let id = ObjectId(self.objects.len() as u32);
            self.objects.push(Some(obj));
            id
        }
    }

    /// Allocate an `ObjectKind::Array` with the standard prototype.
    pub(crate) fn create_array_object(&mut self, elements: Vec<JsValue>) -> ObjectId {
        self.alloc_object(Object {
            kind: ObjectKind::Array { elements },
            storage: value::PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: self.array_prototype,
            extensible: true,
        })
    }

    /// Allocate a `StringWrapper` with `length` stored as a non-writable data
    /// property (immutable inner string → no accessor needed).
    pub(crate) fn create_string_wrapper(&mut self, sid: StringId) -> ObjectId {
        #[allow(clippy::cast_precision_loss)]
        let len = self.strings.get(sid).len() as f64;
        let obj = self.alloc_object(Object {
            kind: ObjectKind::StringWrapper(sid),
            storage: value::PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: self.string_prototype,
            extensible: true,
        });
        self.install_string_wrapper_length(obj, len);
        obj
    }

    /// Promote an existing Ordinary instance (typically pre-allocated by
    /// `do_new` for a native constructor) into a StringWrapper in place,
    /// reusing the object slot to avoid a second allocation.
    pub(crate) fn promote_to_string_wrapper(&mut self, obj_id: ObjectId, sid: StringId) {
        #[allow(clippy::cast_precision_loss)]
        let len = self.strings.get(sid).len() as f64;
        {
            let obj = self.get_object_mut(obj_id);
            obj.kind = ObjectKind::StringWrapper(sid);
        }
        self.install_string_wrapper_length(obj_id, len);
    }

    /// Promote an existing Ordinary instance into an Array in place.  Same
    /// motivation as `promote_to_string_wrapper`: reuse the object slot
    /// pre-allocated by `do_new` instead of allocating a fresh array.
    pub(crate) fn promote_to_array(&mut self, obj_id: ObjectId, elements: Vec<JsValue>) {
        let obj = self.get_object_mut(obj_id);
        obj.kind = ObjectKind::Array { elements };
    }

    fn install_string_wrapper_length(&mut self, obj_id: ObjectId, len: f64) {
        let length_key = value::PropertyKey::String(self.well_known.length);
        self.define_shaped_property(
            obj_id,
            length_key,
            value::PropertyValue::Data(JsValue::Number(len)),
            shape::PropertyAttrs {
                writable: false,
                enumerable: false,
                configurable: false,
                is_accessor: false,
            },
        );
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

    // -- Shape helpers --------------------------------------------------------

    /// Add-transition: add a new property to a Shape, returning the child ShapeId.
    /// Reuses an existing transition if the same (key, attrs) was added before.
    pub(crate) fn shape_add_transition(
        &mut self,
        parent: shape::ShapeId,
        key: value::PropertyKey,
        attrs: shape::PropertyAttrs,
    ) -> shape::ShapeId {
        let tk = shape::TransitionKey::Add(key, attrs);
        if let Some(&child) = self.shapes[parent as usize].transitions.get(&tk) {
            return child;
        }
        let parent_shape = &self.shapes[parent as usize];
        debug_assert!(
            !parent_shape.property_map.contains_key(&key),
            "shape_add_transition called for existing key; use shape_reconfigure_transition instead"
        );
        let mut property_map = parent_shape.property_map.clone();
        let slot_index = parent_shape.ordered_entries.len() as u16;
        property_map.insert(key, slot_index);
        let mut ordered_entries = parent_shape.ordered_entries.clone();
        ordered_entries.push((key, attrs));
        let child_id = self.shapes.len() as shape::ShapeId;
        self.shapes.push(shape::Shape {
            transitions: HashMap::new(),
            property_map,
            ordered_entries,
        });
        self.shapes[parent as usize]
            .transitions
            .insert(tk, child_id);
        child_id
    }

    /// Reconfigure-transition: change the attributes of an existing property.
    /// Slot index is unchanged; only attrs in ordered_entries are updated.
    pub(crate) fn shape_reconfigure_transition(
        &mut self,
        parent: shape::ShapeId,
        key: value::PropertyKey,
        attrs: shape::PropertyAttrs,
    ) -> shape::ShapeId {
        let tk = shape::TransitionKey::Reconfigure(key, attrs);
        if let Some(&child) = self.shapes[parent as usize].transitions.get(&tk) {
            return child;
        }
        let parent_shape = &self.shapes[parent as usize];
        debug_assert!(
            parent_shape.property_map.contains_key(&key),
            "shape_reconfigure_transition called for non-existent key"
        );
        let slot_index = parent_shape.property_map[&key];
        let property_map = parent_shape.property_map.clone();
        let mut ordered_entries = parent_shape.ordered_entries.clone();
        ordered_entries[slot_index as usize].1 = attrs;
        let child_id = self.shapes.len() as shape::ShapeId;
        self.shapes.push(shape::Shape {
            transitions: HashMap::new(),
            property_map,
            ordered_entries,
        });
        self.shapes[parent as usize]
            .transitions
            .insert(tk, child_id);
        child_id
    }

    /// Reconfigure an existing property's attributes on a Shaped object.
    /// Updates the shape via reconfigure transition and optionally writes a new slot value.
    pub(crate) fn reconfigure_property(
        &mut self,
        obj_id: ObjectId,
        key: value::PropertyKey,
        new_attrs: shape::PropertyAttrs,
        new_value: Option<value::PropertyValue>,
    ) {
        let current_shape = match &self.objects[obj_id.0 as usize].as_ref().unwrap().storage {
            value::PropertyStorage::Shaped { shape, .. } => *shape,
            value::PropertyStorage::Dictionary(_) => return, // no-op for dictionary
        };
        let new_shape = self.shape_reconfigure_transition(current_shape, key, new_attrs);
        let obj = self.objects[obj_id.0 as usize].as_mut().unwrap();
        if let value::PropertyStorage::Shaped { shape, slots } = &mut obj.storage {
            *shape = new_shape;
            if let Some(val) = new_value {
                let slot_idx = self.shapes[new_shape as usize].property_map[&key];
                slots[slot_idx as usize] = val;
            }
        }
    }

    /// Define a new property on a Shaped object: transition + slot push.
    /// If the object is in Dictionary mode, pushes directly.
    pub(crate) fn define_shaped_property(
        &mut self,
        obj_id: ObjectId,
        key: value::PropertyKey,
        value: value::PropertyValue,
        attrs: shape::PropertyAttrs,
    ) {
        // Read current shape.
        let current_shape = match &self.objects[obj_id.0 as usize].as_ref().unwrap().storage {
            value::PropertyStorage::Shaped { shape, .. } => *shape,
            value::PropertyStorage::Dictionary(_) => {
                let prop = value::Property::from_attrs(value, attrs);
                self.get_object_mut(obj_id).storage.push_dict(key, prop);
                return;
            }
        };
        // Transition shape.
        let new_shape = self.shape_add_transition(current_shape, key, attrs);
        // Update object.
        let obj = self.objects[obj_id.0 as usize].as_mut().unwrap();
        if let value::PropertyStorage::Shaped { shape, slots } = &mut obj.storage {
            *shape = new_shape;
            slots.push(value);
        }
    }

    /// Convert a Shaped object to Dictionary mode (for delete).
    pub(crate) fn convert_to_dictionary(&mut self, obj_id: ObjectId) {
        let obj = self.objects[obj_id.0 as usize].as_mut().unwrap();
        let new_storage = match &obj.storage {
            value::PropertyStorage::Dictionary(_) => return, // already dictionary
            value::PropertyStorage::Shaped { shape, slots } => {
                let s = &self.shapes[*shape as usize];
                let vec: Vec<(value::PropertyKey, value::Property)> = s
                    .ordered_entries
                    .iter()
                    .enumerate()
                    .map(|(i, (key, attrs))| {
                        (
                            *key,
                            value::Property {
                                slot: slots[i],
                                writable: attrs.writable,
                                enumerable: attrs.enumerable,
                                configurable: attrs.configurable,
                            },
                        )
                    })
                    .collect();
                value::PropertyStorage::Dictionary(vec)
            }
        };
        obj.storage = new_storage;
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
        self.create_native_function_impl(name, func, false)
    }

    /// Helper: create a constructable native function object (for Error, etc.).
    pub(crate) fn create_constructable_function(
        &mut self,
        name: &str,
        func: fn(&mut NativeContext<'_>, JsValue, &[JsValue]) -> Result<JsValue, VmError>,
    ) -> ObjectId {
        self.create_native_function_impl(name, func, true)
    }

    fn create_native_function_impl(
        &mut self,
        name: &str,
        func: fn(&mut NativeContext<'_>, JsValue, &[JsValue]) -> Result<JsValue, VmError>,
        constructable: bool,
    ) -> ObjectId {
        let name_id = self.strings.intern(name);
        let obj = self.alloc_object(Object {
            kind: ObjectKind::NativeFunction(NativeFunction {
                name: name_id,
                func,
                constructable,
            }),
            storage: value::PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: self.function_prototype,
            extensible: true,
        });
        // §19.2.4.2: `name` is a non-enumerable, non-writable, configurable
        // data property on every built-in function.
        let name_key = value::PropertyKey::String(self.well_known.name);
        self.define_shaped_property(
            obj,
            name_key,
            value::PropertyValue::Data(JsValue::String(name_id)),
            shape::PropertyAttrs {
                writable: false,
                enumerable: false,
                configurable: true,
                is_accessor: false,
            },
        );
        obj
    }

    /// Update an existing data property or define a new one.
    pub(crate) fn upsert_data_property(
        &mut self,
        obj_id: ObjectId,
        key: value::PropertyKey,
        val: JsValue,
        attrs: shape::PropertyAttrs,
    ) {
        let existing_attrs = {
            let shapes = &self.shapes;
            let obj = self.objects[obj_id.0 as usize].as_ref().unwrap();
            obj.storage.get(key, shapes).map(|(_, a)| a)
        };
        match existing_attrs {
            Some(current_attrs) if current_attrs == attrs => {
                // Same attrs — just update the slot value.
                let shapes = &self.shapes;
                let obj = self.objects[obj_id.0 as usize].as_mut().unwrap();
                if let Some((slot, _)) = obj.storage.get_mut(key, shapes) {
                    *slot = value::PropertyValue::Data(val);
                }
            }
            Some(_) => {
                // Attrs differ — update both value and attrs.
                let new_val = value::PropertyValue::Data(val);
                let is_shaped = matches!(
                    self.objects[obj_id.0 as usize].as_ref().unwrap().storage,
                    value::PropertyStorage::Shaped { .. }
                );
                if is_shaped {
                    // Shaped: write value then reconfigure shape.
                    {
                        let shapes = &self.shapes;
                        let obj = self.objects[obj_id.0 as usize].as_mut().unwrap();
                        if let Some((slot, _)) = obj.storage.get_mut(key, shapes) {
                            *slot = new_val;
                        }
                    }
                    self.reconfigure_property(obj_id, key, attrs, None);
                } else {
                    // Dictionary: replace the entire Property.
                    let obj = self.objects[obj_id.0 as usize].as_mut().unwrap();
                    if let value::PropertyStorage::Dictionary(vec) = &mut obj.storage {
                        if let Some((_, prop)) = vec.iter_mut().find(|(k, _)| *k == key) {
                            *prop = value::Property::from_attrs(new_val, attrs);
                        }
                    }
                }
            }
            None => {
                // Non-extensible objects cannot gain new properties.
                if !self.get_object(obj_id).extensible {
                    return;
                }
                self.define_shaped_property(obj_id, key, value::PropertyValue::Data(val), attrs);
            }
        }
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
    #[allow(clippy::too_many_lines)]
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
            bigint_type: strings.intern("bigint"),
            object_to_string: strings.intern("[object Object]"),
            next: strings.intern("next"),
            value: strings.intern("value"),
            done: strings.intern("done"),
            return_str: strings.intern("return"),
            last_index: strings.intern("lastIndex"),
            index: strings.intern("index"),
            input: strings.intern("input"),
            join: strings.intern("join"),
            to_json: strings.intern("toJSON"),
            get: strings.intern("get"),
            set: strings.intern("set"),
            enumerable: strings.intern("enumerable"),
            configurable: strings.intern("configurable"),
            writable: strings.intern("writable"),
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
                bigints: BigIntPool::new(),
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
                bigint_prototype: None,
                function_prototype: None,
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
                shapes: vec![shape::Shape::root()],
                gc_object_marks: Vec::new(),
                gc_upvalue_marks: Vec::new(),
                gc_work_list: Vec::new(),
                gc_bytes_since_last: 0,
                gc_threshold: 65536,
                gc_enabled: false,
                in_construct: false,
                host_data: None,
            },
        };

        vm.inner.register_globals();
        vm.inner.gc_enabled = true;
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

    /// Install a `HostData` instance for browser shell integration.
    /// Call once, typically at `ElidexJsEngine` construction.
    ///
    /// # Panics
    ///
    /// Panics if a `HostData` is already installed, to prevent accidentally
    /// dropping caches (listener_store, wrapper_cache) from a prior bind.
    pub fn install_host_data(&mut self, hd: host_data::HostData) {
        assert!(
            self.inner.host_data.is_none(),
            "HostData already installed; use host_data() to access or a fresh Vm to reinstall"
        );
        self.inner.host_data = Some(Box::new(hd));
    }

    /// Access the host data (if installed).
    pub fn host_data(&mut self) -> Option<&mut host_data::HostData> {
        self.inner.host_data.as_deref_mut()
    }

    /// Bind host pointers for a JS execution call.  No-op if `HostData` is absent.
    ///
    /// # Safety
    ///
    /// See [`host_data::HostData::bind`]: pointers must remain valid (and not
    /// be aliased via any Rust reference) until `unbind()` is called.
    #[cfg(feature = "engine")]
    #[allow(unsafe_code)]
    pub unsafe fn bind(
        &mut self,
        session: *mut elidex_script_session::SessionCore,
        dom: *mut elidex_ecs::EcsDom,
        document: elidex_ecs::Entity,
    ) {
        if let Some(hd) = self.inner.host_data.as_deref_mut() {
            unsafe { hd.bind(session, dom, document) };
        }
    }

    /// Clear host pointers after JS execution.  No-op if unbound.
    pub fn unbind(&mut self) {
        if let Some(hd) = self.inner.host_data.as_deref_mut() {
            hd.unbind();
        }
    }

    /// Install a new global variable.
    ///
    /// Reusing a name is normally a bug — shell host globals and JS-visible
    /// built-ins must not collide — so this convenience method ignores any
    /// previous value.  Use [`Vm::set_global_checked`] if the caller needs
    /// to detect replacement explicitly.
    pub fn set_global(&mut self, name: &str, value: JsValue) {
        let _ = self.set_global_checked(name, value);
    }

    /// Install a new global variable and return the previous value, if any.
    pub fn set_global_checked(&mut self, name: &str, value: JsValue) -> Option<JsValue> {
        let id = self.inner.strings.intern(name);
        self.inner.globals.insert(id, value)
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

    /// Returns `true` if the current native function is being invoked via
    /// `[[Construct]]` (i.e. `new F(...)`).  Used by constructors to choose
    /// between wrapper-object and primitive return paths.
    #[inline]
    pub fn is_construct(&self) -> bool {
        self.vm.in_construct
    }

    /// Access the host data.
    ///
    /// # Panics
    ///
    /// Panics if no `HostData` has been installed on the VM.
    pub fn host(&mut self) -> &mut host_data::HostData {
        self.vm
            .host_data
            .as_deref_mut()
            .expect("NativeContext::host() called without HostData installed")
    }

    /// Access the host data, returning `None` if not installed.
    pub fn host_opt(&mut self) -> Option<&mut host_data::HostData> {
        self.vm.host_data.as_deref_mut()
    }

    /// `HasProperty` + `Get` (§7.3.1): returns `None` if the property does
    /// not exist anywhere on the prototype chain, `Some(value)` otherwise.
    pub fn try_get_property_value(
        &mut self,
        obj_id: value::ObjectId,
        key: value::PropertyKey,
    ) -> Result<Option<JsValue>, VmError> {
        let exists = coerce::get_property(self.vm, obj_id, key).is_some();
        if !exists {
            return Ok(None);
        }
        Ok(Some(self.vm.get_property_value(obj_id, key)?))
    }
}
