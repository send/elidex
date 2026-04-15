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
mod coroutine_types;
mod dispatch;
mod dispatch_helpers;
mod dispatch_ic;
mod dispatch_iter;
mod dispatch_objects;
pub(crate) mod gc;
mod globals;
mod globals_async;
mod globals_errors;
mod host;
pub mod host_data;
pub(crate) mod ic;
pub mod interpreter;
mod native_context;
mod natives;
mod natives_array;
mod natives_array_hof;
mod natives_bigint;
mod natives_boolean;
#[cfg(feature = "engine")]
mod natives_event;
mod natives_function;
mod natives_generator;
mod natives_json;
mod natives_math;
mod natives_number;
mod natives_object;
mod natives_promise;
mod natives_promise_combinator;
mod natives_regexp;
mod natives_string;
mod natives_string_ext;
mod natives_symbol;
mod natives_timer;
mod ops;
mod ops_property;
pub mod pools;
pub(crate) mod shape;
pub mod value;
mod vm_api;

#[cfg(test)]
mod tests;

use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

use pools::{BigIntPool, StringPool};
#[cfg(feature = "engine")]
use value::same_value;
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
    /// Promise.prototype object (§25.6.5).
    pub(crate) promise_prototype: Option<ObjectId>,
    /// Microtask queue (HTML §8.1.4.3).  Drained at HTML microtask
    /// checkpoints (end of `eval`, end of each event listener).
    pub(crate) microtask_queue: VecDeque<natives_promise::Microtask>,
    /// Reentrancy guard — nonzero while a drain is in progress, so nested
    /// eval/listener calls don't reorder the rest of the queue.
    pub(crate) microtask_drain_depth: u32,
    /// Rejected promises with no reject handler attached at settle time.
    /// End-of-drain scan warns on entries still `Rejected && !handled`.
    /// PromiseRejectionEvent dispatch ships with PR3.
    pub(crate) pending_rejections: Vec<ObjectId>,
    /// Error.prototype (§19.5.3) — shared by Error and the built-in
    /// error subclasses (TypeError, RangeError, …, AggregateError).
    pub(crate) error_prototype: Option<ObjectId>,
    /// AggregateError.prototype (§20.5.7) — chains to Error.prototype
    /// (NOT Object.prototype) so `instanceof Error` is true for
    /// AggregateError instances.
    pub(crate) aggregate_error_prototype: Option<ObjectId>,
    /// Generator.prototype — shared prototype for generator iterators.
    pub(crate) generator_prototype: Option<ObjectId>,
    /// `EventTarget.prototype` — shared prototype for every DOM wrapper.
    /// Holds `addEventListener` / `removeEventListener` / `dispatchEvent`.
    /// `None` until `register_event_target_prototype()` runs during
    /// `register_globals()`.  Read when `create_element_wrapper` (PR3 C2)
    /// allocates a `HostObject` wrapper.
    pub(crate) event_target_prototype: Option<ObjectId>,
    /// Internal prototype for `ObjectKind::Event` instances.  Holds the
    /// four event methods (`preventDefault`, `stopPropagation`,
    /// `stopImmediatePropagation`, `composedPath`) and the
    /// `defaultPrevented` accessor.  Methods are stateless `fn`
    /// pointers that match on `this`'s `ObjectKind::Event` for state,
    /// so a single prototype is shared across all dispatched events —
    /// avoids 5 native-fn allocations + 5 shape transitions per
    /// listener invocation.
    ///
    /// NOT exposed as `Event.prototype` to JS (the spec global +
    /// constructor land in PR5a alongside `new Event(...)`); this is
    /// a pure VM intrinsic.  When PR5a lands, `Event.prototype` can
    /// become this object's parent or replace it outright.
    pub(crate) event_methods_prototype: Option<ObjectId>,
    /// Terminal `ShapeId` per `EventPayload` variant, built once
    /// during `register_globals` (PR3.6).  `None` on non-engine builds
    /// (events don't dispatch there), `Some` on engine builds after
    /// VM creation.
    ///
    /// Allows `create_event_object` to allocate at the final shape
    /// instead of walking `shape_add_transition` 9-17 times per event
    /// — the hot path for high-frequency dispatchers like mousemove.
    /// See `host/event_shapes.rs` module doc for the per-variant
    /// property list.
    #[cfg(feature = "engine")]
    pub(crate) precomputed_event_shapes: Option<host::event_shapes::PrecomputedEventShapes>,
    /// Set by `Op::Yield` to signal the enclosing `resume_generator` of
    /// the yielded value.  `None` outside a yield dispatch.
    pub(crate) generator_yielded: Option<JsValue>,
    /// Currently-executing microtask, held between `pop_front` and the end
    /// of its callback so the task's `handler` / `capability` / `resolution`
    /// (or bare `Callback { func }`) stay GC-rooted while the user JS
    /// attached to them runs.  Without this, a Promise handler that
    /// triggers a GC could see its own capability Promise / callback
    /// collected (they are no longer in the queue, and only a Rust local
    /// held them otherwise).
    pub(crate) current_microtask: Option<natives_promise::Microtask>,
    /// Pending timers ordered by nearest deadline; fired by
    /// `drain_timers(now)` (driven by the shell on each event-loop tick).
    pub(crate) timer_queue: BinaryHeap<natives_timer::TimerEntry>,
    /// Currently-firing timer entry, owned by the VM during callback
    /// execution so `entry.callback` and `entry.args` survive any GC
    /// triggered by the callback.  The entry is popped out of
    /// `timer_queue` before running and moved into this slot; on return
    /// the drain loop takes it back for interval re-arm / active-set
    /// cleanup.
    pub(crate) current_timer: Option<natives_timer::TimerEntry>,
    /// Monotonically-increasing IDs returned by `setTimeout` / `setInterval`.
    pub(crate) next_timer_id: u32,
    /// IDs of currently-live timers: inserted on schedule, removed on
    /// fire (for one-shot) or cancel.  Intervals stay in the set across
    /// re-arm because their id is reused.  This lets `clearTimeout` /
    /// `clearInterval` reject ids that aren't ours in O(1) — the naive
    /// "iterate the heap" alternative misses intervals whose callback
    /// cancels itself (the heap entry is popped before the callback
    /// runs, so an any-in-queue test would return `false` and the
    /// subsequent re-arm would evade cancellation).
    pub(crate) active_timer_ids: HashSet<u32>,
    /// IDs cleared before firing — skipped at drain time.
    pub(crate) cancelled_timers: HashSet<u32>,
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
    pub(crate) source: StringId,
    pub(crate) flags: StringId,
    pub(crate) status: StringId,
    pub(crate) fulfilled: StringId,
    pub(crate) rejected: StringId,
    pub(crate) reason: StringId,
    pub(crate) errors: StringId,
    pub(crate) aggregate_error: StringId,

    // -- Event-dispatch identifiers (PR3) --
    // Property names installed on every event object — pre-interned
    // here to avoid a HashMap lookup per name per dispatch.  Listener
    // option keys (`capture`/`once`/`passive`) live here too since
    // every `addEventListener` with options-object form reads them.
    pub(crate) event_type: StringId,
    pub(crate) bubbles: StringId,
    pub(crate) cancelable: StringId,
    pub(crate) event_phase: StringId,
    pub(crate) target: StringId,
    pub(crate) current_target: StringId,
    pub(crate) time_stamp: StringId,
    pub(crate) composed: StringId,
    pub(crate) is_trusted: StringId,
    pub(crate) default_prevented: StringId,
    pub(crate) prevent_default: StringId,
    pub(crate) stop_propagation: StringId,
    pub(crate) stop_immediate_propagation: StringId,
    pub(crate) composed_path: StringId,
    pub(crate) capture: StringId,
    pub(crate) once: StringId,
    pub(crate) passive: StringId,
    pub(crate) document: StringId,
    pub(crate) unhandledrejection: StringId,
    pub(crate) promise: StringId,

    // -- Event payload property keys (PR3.6) --
    // Pre-interned so `create_event_object`'s payload installation
    // can feed them directly into the precomputed-shape slot array
    // without per-dispatch `strings.intern(name)` calls.  Also used
    // by `PrecomputedEventShapes::build` to walk the shape-transition
    // chain once at `register_globals` time.
    //
    // Shared keys (used by multiple payload variants) are defined
    // once: `alt_key`, `ctrl_key`, `meta_key`, `shift_key`, `data`,
    // `code`, `elapsed_time`, `pseudo_element`, `key`.
    pub(crate) client_x: StringId,
    pub(crate) client_y: StringId,
    pub(crate) button: StringId,
    pub(crate) buttons: StringId,
    pub(crate) alt_key: StringId,
    pub(crate) ctrl_key: StringId,
    pub(crate) meta_key: StringId,
    pub(crate) shift_key: StringId,
    pub(crate) key: StringId,
    pub(crate) code: StringId,
    pub(crate) repeat: StringId,
    pub(crate) property_name: StringId,
    pub(crate) elapsed_time: StringId,
    pub(crate) pseudo_element: StringId,
    pub(crate) animation_name: StringId,
    pub(crate) input_type: StringId,
    pub(crate) data: StringId,
    pub(crate) is_composing: StringId,
    pub(crate) data_type: StringId,
    pub(crate) related_target: StringId,
    pub(crate) delta_x: StringId,
    pub(crate) delta_y: StringId,
    pub(crate) delta_mode: StringId,
    pub(crate) origin: StringId,
    pub(crate) last_event_id: StringId,
    // `close_event_code` — CloseEvent's numeric code (WebSocket close
    // frame code).  Distinct from the `code` key (KeyboardEvent).
    // JS-visible name is `"code"`; the Rust field disambiguates.
    pub(crate) close_event_code: StringId,
    pub(crate) was_clean: StringId,
    pub(crate) old_url: StringId,
    pub(crate) new_url: StringId,
    pub(crate) persisted: StringId,
    pub(crate) old_value: StringId,
    pub(crate) new_value: StringId,
    pub(crate) url: StringId,
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

    /// Resolve a constructor's receiver for both `new`-mode and
    /// call-mode invocations.
    ///
    /// - `new F(...)`: native dispatch sets `self.in_construct = true`
    ///   and `do_new` supplies a pre-allocated object receiver — we
    ///   must reuse `this` as-is so the constructor initializes the
    ///   same instance the caller will receive.
    /// - `F(...)` (call-mode): `in_construct = false`; allocate a
    ///   fresh Ordinary with `prototype`.  An explicit receiver
    ///   passed via `F.call(obj, ...)` / `F.apply(obj, ...)` is *not*
    ///   reused — spec §19.5.1.1 step 2 (OrdinaryCreateFromConstructor)
    ///   always yields a new object.
    ///
    /// Implements the "callable constructor" shape of §19.5.1.1
    /// step 1-2.
    pub(crate) fn ensure_instance_or_alloc(
        &mut self,
        this: JsValue,
        prototype: Option<ObjectId>,
    ) -> JsValue {
        if self.in_construct {
            if let JsValue::Object(_) = this {
                return this;
            }
        }
        let obj = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: value::PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype,
            extensible: true,
        });
        JsValue::Object(obj)
    }

    /// Allocate an `ObjectKind::Array` with the standard prototype.
    pub(crate) fn create_array_object(&mut self, elements: Vec<JsValue>) -> ObjectId {
        // `alloc_object` can trigger GC *before* the new object is
        // inserted into `self.objects`.  At that point `elements` lives
        // only in the Rust-local `Object` struct — not a GC root — so
        // any `JsValue::Object` entries could be collected mid-call.
        // Push a temporary rooted copy onto the VM stack for the
        // allocation window; GC scans `self.stack`, so every element
        // stays alive.  After the new object is installed in
        // `self.objects`, its elements are reachable via the object
        // and the stack copy can go.
        let stack_root = self.stack.len();
        self.stack.extend_from_slice(&elements);
        let obj = self.alloc_object(Object {
            kind: ObjectKind::Array { elements },
            storage: value::PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: self.array_prototype,
            extensible: true,
        });
        self.stack.truncate(stack_root);
        obj
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

    /// Push `value` onto the VM stack as a temporary GC root and
    /// return an RAII guard that restores the stack on drop.
    ///
    /// See [`VmTempRoot`] for the rooting contract — the guard
    /// derefs to `&mut VmInner` so the rooted region is written as
    /// method calls on the guard:
    ///
    /// ```rust,ignore
    /// let mut g = vm.push_temp_root(JsValue::Object(id));
    /// let _ = g.call(func_id, this, &[arg]);
    /// match g.get_object(id).kind { ... }
    /// // guard drops here; stack restored to pre-push length
    /// ```
    ///
    /// Engine-feature gated: rooting matters only when host code can
    /// produce un-rooted intermediate `JsValue`s (event objects,
    /// PromiseRejection synthetic events, etc.).  Without the engine
    /// feature there is no host bridge and no caller.
    #[cfg(feature = "engine")]
    pub(crate) fn push_temp_root(&mut self, value: JsValue) -> VmTempRoot<'_> {
        let saved_len = self.stack.len();
        self.stack.push(value);
        VmTempRoot {
            vm: self,
            saved_len,
            expected: value,
        }
    }
}

/// RAII guard for a temporary GC root pushed onto the VM stack.
///
/// Created via [`VmInner::push_temp_root`].  Restores the stack to
/// its pre-push length on drop, **including during panic unwinding**
/// — this is the key property over a bare `push` / `pop` pair, which
/// would leak the root through a `catch_unwind` boundary upstream
/// and corrupt subsequent GC cycles.
///
/// On normal (non-panic) drop, two release-enforced asserts catch
/// closure-side stack-corruption bugs:
///
/// - **Length check**: stack must end at `saved_len + 1` (no leaked
///   pushes, no over-pops).
/// - **Slot identity**: `stack[saved_len]` must still hold the
///   pushed value (defends against pop-then-push-different which
///   leaves length unchanged but unroots the original).  Comparison
///   uses `same_value` (NaN-safe) since `JsValue::Number(NaN) !=
///   JsValue::Number(NaN)` under JS strict equality.
///
/// During panic unwinding (`std::thread::panicking()`) both asserts
/// are skipped to avoid double-panic process-abort; the stack is
/// truncated unconditionally so any propagation through
/// `catch_unwind` upstream sees a clean stack.
#[cfg(feature = "engine")]
pub(crate) struct VmTempRoot<'a> {
    vm: &'a mut VmInner,
    saved_len: usize,
    expected: JsValue,
}

#[cfg(feature = "engine")]
impl std::ops::Deref for VmTempRoot<'_> {
    type Target = VmInner;
    #[inline]
    fn deref(&self) -> &VmInner {
        self.vm
    }
}

#[cfg(feature = "engine")]
impl std::ops::DerefMut for VmTempRoot<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut VmInner {
        self.vm
    }
}

#[cfg(feature = "engine")]
impl Drop for VmTempRoot<'_> {
    fn drop(&mut self) {
        let stack = &mut self.vm.stack;
        if std::thread::panicking() {
            // Avoid double-panic; just restore.  An assertion failure
            // here while unwinding would abort the process and lose the
            // original panic's diagnostic.
            stack.truncate(self.saved_len);
            return;
        }
        assert_eq!(
            stack.len(),
            self.saved_len + 1,
            "VmTempRoot: rooted region left the VM stack at {} entries, \
             expected {} — GC root corruption hazard",
            stack.len(),
            self.saved_len + 1,
        );
        assert!(
            same_value(stack[self.saved_len], self.expected),
            "VmTempRoot: stack[saved_len] no longer holds the rooted value \
             — rooted region pop'd and re-push'd the slot"
        );
        stack.truncate(self.saved_len);
    }
}

impl VmInner {
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

    /// Install a pre-built shape and its matching slot values on an
    /// object in a single operation — skipping the per-property
    /// transition walk.
    ///
    /// Used by hot paths where the final property layout is fixed at
    /// VM creation time (e.g. event objects via `PrecomputedEventShapes`):
    /// allocate the object at `ROOT_SHAPE` with an empty slot vec,
    /// then call this API once with the precomputed terminal shape and
    /// the assembled slot values.  Replaces ~N `define_shaped_property`
    /// calls with a single `PropertyStorage` replacement.
    ///
    /// All slots are installed as [`value::PropertyValue::Data`].
    /// Callers that need accessor properties on the fast path must
    /// fall back to `define_shaped_property` (a design trade-off —
    /// this API is optimised for the event-object case, where every
    /// own property is a data property and accessors live on the
    /// shared `event_methods_prototype`).
    ///
    /// # Panics
    ///
    /// Debug-only asserts the slot count matches the shape's property
    /// count; mismatch means the caller assembled the slot Vec in a
    /// different order than the shape was built with — a structural
    /// bug that would otherwise silently write values into the wrong
    /// JS-visible property names.
    ///
    /// Also panics if the object is in `Dictionary` storage mode —
    /// caller should only route objects that have never left
    /// `Shaped` (freshly-allocated event objects never transition to
    /// Dictionary).
    //
    // Engine-feature gated — the sole consumer is
    // `host::events::create_event_object`, which is itself engine-only
    // (no DOM events to dispatch in non-engine builds).  A future
    // non-engine caller can relax this, but for now it keeps the
    // non-engine build free of dead-code warnings.
    #[cfg(feature = "engine")]
    pub(crate) fn define_with_precomputed_shape(
        &mut self,
        obj_id: ObjectId,
        shape_id: shape::ShapeId,
        slots: &[JsValue],
    ) {
        debug_assert_eq!(
            self.shapes[shape_id as usize].property_count() as usize,
            slots.len(),
            "define_with_precomputed_shape: slot count ({}) does not match shape property count ({}) — caller built the slot Vec in a different order than the shape",
            slots.len(),
            self.shapes[shape_id as usize].property_count(),
        );
        let property_slots: Vec<value::PropertyValue> = slots
            .iter()
            .copied()
            .map(value::PropertyValue::Data)
            .collect();
        let obj = self.objects[obj_id.0 as usize].as_mut().unwrap();
        match &mut obj.storage {
            value::PropertyStorage::Shaped { shape, slots: s } => {
                *shape = shape_id;
                *s = property_slots;
            }
            value::PropertyStorage::Dictionary(_) => {
                panic!("define_with_precomputed_shape requires Shaped storage; got Dictionary");
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
            source: strings.intern("source"),
            flags: strings.intern("flags"),
            status: strings.intern("status"),
            fulfilled: strings.intern("fulfilled"),
            rejected: strings.intern("rejected"),
            reason: strings.intern("reason"),
            errors: strings.intern("errors"),
            aggregate_error: strings.intern("AggregateError"),
            event_type: strings.intern("type"),
            bubbles: strings.intern("bubbles"),
            cancelable: strings.intern("cancelable"),
            event_phase: strings.intern("eventPhase"),
            target: strings.intern("target"),
            current_target: strings.intern("currentTarget"),
            time_stamp: strings.intern("timeStamp"),
            composed: strings.intern("composed"),
            is_trusted: strings.intern("isTrusted"),
            default_prevented: strings.intern("defaultPrevented"),
            prevent_default: strings.intern("preventDefault"),
            stop_propagation: strings.intern("stopPropagation"),
            stop_immediate_propagation: strings.intern("stopImmediatePropagation"),
            composed_path: strings.intern("composedPath"),
            capture: strings.intern("capture"),
            once: strings.intern("once"),
            passive: strings.intern("passive"),
            document: strings.intern("document"),
            unhandledrejection: strings.intern("unhandledrejection"),
            promise: strings.intern("promise"),

            // Event-payload property keys (PR3.6).  Interned once here
            // so `create_event_object` can feed slots into
            // `define_with_precomputed_shape` without re-interning.
            client_x: strings.intern("clientX"),
            client_y: strings.intern("clientY"),
            button: strings.intern("button"),
            buttons: strings.intern("buttons"),
            alt_key: strings.intern("altKey"),
            ctrl_key: strings.intern("ctrlKey"),
            meta_key: strings.intern("metaKey"),
            shift_key: strings.intern("shiftKey"),
            key: strings.intern("key"),
            code: strings.intern("code"),
            repeat: strings.intern("repeat"),
            property_name: strings.intern("propertyName"),
            elapsed_time: strings.intern("elapsedTime"),
            pseudo_element: strings.intern("pseudoElement"),
            animation_name: strings.intern("animationName"),
            input_type: strings.intern("inputType"),
            data: strings.intern("data"),
            is_composing: strings.intern("isComposing"),
            data_type: strings.intern("dataType"),
            related_target: strings.intern("relatedTarget"),
            delta_x: strings.intern("deltaX"),
            delta_y: strings.intern("deltaY"),
            delta_mode: strings.intern("deltaMode"),
            origin: strings.intern("origin"),
            last_event_id: strings.intern("lastEventId"),
            // CloseEvent's numeric code reuses the JS-visible name
            // `"code"`, but we pre-intern it under a distinct Rust
            // field name to avoid shadowing the `code` (Keyboard) key.
            // The underlying StringId is equal because `intern` is
            // canonical.
            close_event_code: strings.intern("code"),
            was_clean: strings.intern("wasClean"),
            old_url: strings.intern("oldURL"),
            new_url: strings.intern("newURL"),
            persisted: strings.intern("persisted"),
            old_value: strings.intern("oldValue"),
            new_value: strings.intern("newValue"),
            url: strings.intern("url"),
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
                promise_prototype: None,
                microtask_queue: VecDeque::new(),
                microtask_drain_depth: 0,
                pending_rejections: Vec::new(),
                error_prototype: None,
                aggregate_error_prototype: None,
                generator_prototype: None,
                event_target_prototype: None,
                event_methods_prototype: None,
                #[cfg(feature = "engine")]
                precomputed_event_shapes: None,
                generator_yielded: None,
                current_microtask: None,
                timer_queue: BinaryHeap::new(),
                current_timer: None,
                next_timer_id: 1,
                active_timer_ids: HashSet::new(),
                cancelled_timers: HashSet::new(),
            },
        };

        vm.inner.register_globals();
        vm.inner.gc_enabled = true;
        vm
    }

    // -- Public API --
    //
    // The thin wrapper methods that delegate into `VmInner` live in
    // `vm_api.rs` — split out to keep this file under the 1000-line
    // convention.
}
