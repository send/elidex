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
mod error;
pub(crate) mod gc;
#[cfg(test)]
mod gc_tests;
mod globals;
mod globals_async;
mod globals_errors;
mod globals_primitives;
mod host;
pub mod host_data;
pub(crate) mod ic;
mod init;
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
mod shape_ops;
mod temp_root;
pub mod value;
mod vm_api;
mod well_known;

#[cfg(feature = "engine")]
pub(crate) use temp_root::VmTempRoot;

#[cfg(feature = "engine")]
#[doc(hidden)]
pub mod test_helpers;

#[cfg(test)]
mod tests;

use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

use pools::{BigIntPool, StringPool};
use value::{
    CallFrame, JsValue, NativeContext, NativeFunction, Object, ObjectId, ObjectKind, StringId,
    SymbolId, SymbolRecord, VmError,
};
use well_known::{WellKnownStrings, WellKnownSymbols};

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
    /// `EventTarget.prototype` — root of the DOM wrapper chain
    /// (WHATWG DOM §2.7).  Holds only `addEventListener` /
    /// `removeEventListener` / `dispatchEvent`.  Node-level accessors
    /// live on `Node.prototype` one level up, so they do not leak to
    /// non-Node EventTargets (`window`, future `XMLHttpRequest`).
    /// `None` until `register_event_target_prototype()` runs during
    /// `register_globals()`.
    pub(crate) event_target_prototype: Option<ObjectId>,
    /// `Node.prototype` — shared prototype for every DOM **Node**
    /// wrapper (WHATWG DOM §4.4).  Chains to `EventTarget.prototype`
    /// and carries the Node-common accessors (`parentNode`,
    /// `nodeType`, `textContent`, …) plus the mutation methods
    /// (`appendChild`, `removeChild`, `insertBefore`, `replaceChild`).
    /// Sits between `EventTarget.prototype` and `Element.prototype`
    /// so Element / Text / Comment wrappers all see Node members but
    /// `Window` (EventTarget-but-not-Node) does not.  `None` until
    /// `register_node_prototype()` runs during `register_globals()`.
    pub(crate) node_prototype: Option<ObjectId>,
    /// `Element.prototype` — shared prototype for every Element wrapper
    /// (WHATWG DOM §4.9).  Chains to `Node.prototype` so the
    /// Element-only members layered here (attribute ops, ParentNode
    /// accessors, `matches` / `closest`) sit above the Node-common
    /// surface.  Text and Comment wrappers skip this level and chain
    /// straight to `Node.prototype`.  `None` until
    /// `register_element_prototype()` runs during `register_globals()`.
    pub(crate) element_prototype: Option<ObjectId>,
    /// `CharacterData.prototype` — shared prototype for Text and
    /// Comment wrappers (WHATWG DOM §4.10).  Chains to `Node.prototype`
    /// and carries the `data` / `length` accessors plus the
    /// `appendData` / `insertData` / `deleteData` / `replaceData` /
    /// `substringData` methods.  Text has a further intermediate
    /// `Text.prototype` (see [`Self::text_prototype`]) that chains
    /// here, so `splitText` stays off Comment wrappers.
    ///
    /// `None` until `register_character_data_prototype()` runs during
    /// `register_globals()` (between `register_node_prototype` and
    /// `register_element_prototype`).
    #[cfg(feature = "engine")]
    pub(crate) character_data_prototype: Option<ObjectId>,
    /// `Text.prototype` — intermediate prototype layer for Text
    /// wrappers, carrying Text-only members (e.g. `splitText`).
    /// Chains to `CharacterData.prototype`.
    ///
    /// `None` until `register_text_prototype()` runs during
    /// `register_globals()` (right after the CharacterData prototype).
    #[cfg(feature = "engine")]
    pub(crate) text_prototype: Option<ObjectId>,
    /// `DocumentType.prototype` — intermediate prototype layer for
    /// DocumentType wrappers, carrying `name` / `publicId` /
    /// `systemId`.  Chains to `Node.prototype`.
    ///
    /// `None` until `register_document_type_prototype()` runs during
    /// `register_globals()` (after `register_node_prototype`).
    #[cfg(feature = "engine")]
    pub(crate) document_type_prototype: Option<ObjectId>,
    /// `HTMLIFrameElement.prototype` — tag-specific intermediate
    /// prototype for `<iframe>` wrappers.  Chains to
    /// `Element.prototype` today; PR5b will splice in
    /// `HTMLElement.prototype` between the two as part of the wider
    /// HTMLElement work (see plan §D2 for the migration invariant).
    ///
    /// `None` until `register_html_iframe_prototype()` runs during
    /// `register_globals()` (after `register_element_prototype`).
    #[cfg(feature = "engine")]
    pub(crate) html_iframe_prototype: Option<ObjectId>,
    /// `DOMException.prototype` (WebIDL §3.14.1).  Chains to
    /// `Error.prototype` so `instanceof Error` holds for DOMException
    /// instances.  Holds the `name` / `message` / `code` accessor
    /// triplet that reads from
    /// [`Self::dom_exception_states`] via receiver brand-check.
    ///
    /// `None` until `register_dom_exception_global()` runs during
    /// `register_globals()` (after `register_error_constructors` so
    /// `error_prototype` is populated).  Engine-gated: every
    /// consumer (insertAdjacent*, ChildNode / ParentNode mixins,
    /// removeChild, AbortSignal, location) lives behind the
    /// `engine` feature, so the prototype itself is gated too.
    #[cfg(feature = "engine")]
    pub(crate) dom_exception_prototype: Option<ObjectId>,
    /// Per-`DOMException` out-of-band state, keyed by the instance's
    /// own `ObjectId` (same pattern as
    /// [`Self::abort_signal_states`]).  `name` / `message` accessor
    /// reads go through this side table instead of own-data
    /// properties, matching the WebIDL §3.6.8 spec (attribute
    /// accessors read internal slots).
    ///
    /// GC contract:
    /// - Trace step: entries whose key `ObjectId` is reachable via
    ///   the `DOMException.prototype` chain stay — `name` and
    ///   `message` are interned `StringId`s (pool-permanent), so the
    ///   `DomExceptionState` payload needs no `mark_value` pass.
    /// - Sweep tail (`collect_garbage`): entries whose key was
    ///   collected are pruned so a recycled `ObjectId` does not
    ///   inherit stale `name` / `message`.
    #[cfg(feature = "engine")]
    pub(crate) dom_exception_states: HashMap<ObjectId, host::dom_exception::DomExceptionState>,
    /// `Window.prototype` — prototype for the `globalThis` / `window`
    /// `HostObject` (WHATWG HTML §7.2).  Inherits from
    /// `EventTarget.prototype` so `window.addEventListener` resolves
    /// without a per-entity method install; own-property slots for
    /// window-specific APIs (`innerWidth`, `scrollTo`, `navigator`,
    /// `location`, …) land on this prototype in later PR4b commits.
    ///
    /// `None` until `register_window_prototype()` runs during
    /// `register_globals()` (right after `register_event_target_prototype`
    /// so the chain is built bottom-up).
    pub(crate) window_prototype: Option<ObjectId>,
    /// `AbortSignal.prototype` — chained directly to
    /// `EventTarget.prototype` (Node.prototype is **skipped**: WHATWG
    /// DOM §3.1 / §7.2 — AbortSignal is an EventTarget but not a
    /// Node, mirroring the Window arrangement).  Holds the signal's
    /// own-property suite (`aborted`, `reason`, `onabort` accessors;
    /// `throwIfAborted` method) plus listener overrides that route
    /// through `abort_signal_states` instead of an ECS entity.
    /// `None` until `register_abort_signal_global()` runs during
    /// `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) abort_signal_prototype: Option<ObjectId>,
    /// Per-signal mutable state, keyed by the `AbortSignal`'s own
    /// `ObjectId`.  Out-of-band so [`ObjectKind::AbortSignal`] stays
    /// payload-free and per-variant size discipline is preserved.
    ///
    /// GC contract:
    /// - Trace step (`trace_work_list`) marks every `abort_listeners`
    ///   callback ObjectId and the `reason` JsValue when the
    ///   AbortSignal object is reachable.
    /// - Sweep tail (`collect_garbage`) prunes entries whose key
    ///   ObjectId was collected, so a recycled slot never inherits
    ///   stale state.
    #[cfg(feature = "engine")]
    pub(crate) abort_signal_states: HashMap<ObjectId, host::abort::AbortSignalState>,
    /// Reverse index from a `ListenerId` (registered via
    /// `addEventListener(type, cb, {signal})`) back to the
    /// `AbortSignal` `ObjectId` that owns it.  Lets
    /// `removeEventListener` prune the corresponding back-ref entry
    /// in `abort_signal_states[signal_id].bound_listener_removals`
    /// in O(1) — without this lookup, the back-ref list would grow
    /// unbounded across add/remove cycles for a long-lived signal.
    ///
    /// GC contract: cleaned alongside `abort_signal_states` in the
    /// post-sweep pass — entries whose value `ObjectId` was
    /// collected are dropped, since the owning signal no longer
    /// exists.
    #[cfg(feature = "engine")]
    pub(crate) abort_listener_back_refs: HashMap<elidex_script_session::ListenerId, ObjectId>,
    /// Pending `AbortSignal.timeout(ms)` registrations — keyed by
    /// timer id (the `u32` returned by `schedule_timer`), value is
    /// the signal `ObjectId` to abort when the timer fires.  PR5a
    /// C8 plumbing: the timer's callback slot carries a sentinel
    /// no-op function, and the drain path consults this map BEFORE
    /// invoking the callback — if an entry exists, the VM performs
    /// an internal `abort(DOMException("TimeoutError"))` on the
    /// signal and skips the JS callback dispatch.
    ///
    /// GC contract:
    /// - Each value `ObjectId` is treated as a root (traced in
    ///   `mark_roots`) so a `timeout(100)` signal stranded in this
    ///   map survives until the timer fires.
    /// - On timer fire / explicit cancel, the entry is removed (the
    ///   signal drops back to "reachable only if some listener /
    ///   captured variable holds it").
    /// - GC sweep prunes entries whose signal was collected via a
    ///   different path — a defensive cleanup that's cheap (empty
    ///   map in the common case).
    #[cfg(feature = "engine")]
    pub(crate) pending_timeout_signals: HashMap<u32, ObjectId>,
    /// `Event.prototype` (WebIDL §2.2).  Holds the four event methods
    /// (`preventDefault`, `stopPropagation`, `stopImmediatePropagation`,
    /// `composedPath`) and the `defaultPrevented` accessor, plus the
    /// `constructor` back-pointer to the `Event` global.  Methods are
    /// stateless `fn` pointers that match on `this`'s `ObjectKind::Event`
    /// for state, so a single prototype is shared across all dispatched
    /// events — avoids 5 native-fn allocations + 5 shape transitions
    /// per listener invocation.
    ///
    /// JS-visible via `globalThis.Event.prototype`; PR5a2 landed the
    /// spec-visible `Event` constructor.  Every `ObjectKind::Event`
    /// (UA-initiated or script-constructed) chains through this
    /// prototype.
    pub(crate) event_prototype: Option<ObjectId>,
    /// `CustomEvent.prototype` (WebIDL §2.3).  Chains to
    /// [`event_prototype`] and adds the `detail` accessor.  Set by
    /// `register_custom_event_global` during `register_globals`.
    #[cfg(feature = "engine")]
    pub(crate) custom_event_prototype: Option<ObjectId>,
    /// Terminal `ShapeId` per `EventPayload` variant, built once
    /// during `register_globals`.  `None` on non-engine builds
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
    /// Monotonic reference point for `performance.now()` and
    /// `Event.timeStamp` (WHATWG DOM §2.2 / HR-Time §5).  Set once at
    /// `Vm::new`; both APIs return `self.start_instant.elapsed()` in
    /// milliseconds with sub-ms precision.  Sharing a single
    /// `Instant` guarantees `event.timeStamp` and `performance.now()`
    /// observed inside the same listener are directly comparable
    /// (spec requirement — the time origin is the same).
    ///
    /// Consumed by `performance.now()` and `Event.timeStamp` —
    /// HR-Time §5 requires the two to share a time origin.
    ///
    /// Engine-only: both consumers (`performance.now`, `Event.timeStamp`)
    /// live behind `#[cfg(feature = "engine")]`, so gating the field
    /// keeps the non-engine VM minimal.
    #[cfg(feature = "engine")]
    pub(crate) start_instant: std::time::Instant,
    /// Browsing-context navigation state — backs `location.*`,
    /// `history.*`, and `document.URL` / `document.documentURI`.  See
    /// `host::navigation::NavigationState` for the field list and
    /// Phase 2 scope (in-memory only, no shell bridge yet).
    #[cfg(feature = "engine")]
    pub(crate) navigation: host::navigation::NavigationState,
    /// Viewport size + scroll offset backing the window getters
    /// (`innerWidth`, `innerHeight`, `scrollX`, `scrollY`,
    /// `devicePixelRatio`) and setters (`scrollTo` / `scrollBy`).
    /// Phase 2 defaults; shell pushes real values in PR6.
    #[cfg(feature = "engine")]
    pub(crate) viewport: host::window::ViewportState,
}

impl VmInner {
    /// Drop a `ListenerId` from `HostData::listener_store` AND prune
    /// any `AbortSignal` back-ref to it.
    ///
    /// This is the canonical retirement path — both
    /// `removeEventListener` and the `{once}` auto-removal that
    /// `event_dispatch` triggers via `Engine::remove_listener` route
    /// through this helper so the back-ref index stays bounded
    /// regardless of how the listener was retired.  Skipping the
    /// back-ref scrub would let `abort_listener_back_refs` and
    /// `abort_signal_states[…].bound_listener_removals` grow
    /// unbounded across `addEventListener({signal}, {once: true})`
    /// dispatch cycles.
    ///
    /// Engine-only: `abort_signal_states` /
    /// `abort_listener_back_refs` only exist behind the `engine`
    /// feature; without it, the helper just defers to
    /// `host_data.remove_listener`.
    #[cfg(feature = "engine")]
    pub(crate) fn remove_listener_and_prune_back_ref(
        &mut self,
        listener_id: elidex_script_session::ListenerId,
    ) {
        if let Some(host) = self.host_data.as_deref_mut() {
            host.remove_listener(listener_id);
        }
        if let Some(signal_id) = self.abort_listener_back_refs.remove(&listener_id) {
            if let Some(state) = self.abort_signal_states.get_mut(&signal_id) {
                state.bound_listener_removals.remove(&listener_id);
            }
        }
    }

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
}

/// The elidex-js bytecode VM.
///
/// Persistent across `eval` calls: globals, object heap, and interned strings
/// survive between evaluations.
pub struct Vm {
    pub(crate) inner: VmInner,
}

// `Vm::new` lives in `vm/init.rs` — split out so this file stays
// focused on type definitions; the thin wrapper methods that
// delegate into `VmInner` live in `vm/vm_api.rs`.
