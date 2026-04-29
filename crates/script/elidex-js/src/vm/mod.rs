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
mod gc_trace;
mod globals;
mod globals_async;
mod globals_errors;
mod globals_primitives;
mod host;
pub mod host_data;
pub(crate) mod ic;
mod init;
mod inner;
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
mod object_kind;
mod ops;
mod ops_element;
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
    CallFrame, JsValue, NativeContext, NativeFunction, Object, ObjectId, StringId, SymbolId,
    SymbolRecord, VmError,
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
    /// `HTMLElement.prototype` — shared prototype for every HTML
    /// namespace element wrapper (WHATWG HTML §3.2.8).  Chains to
    /// `Element.prototype`, carrying focus / blur / click methods and
    /// HTML-specific IDL attrs (accessKey, tabIndex, draggable,
    /// hidden, lang, dir, title, translate, spellcheck,
    /// autocapitalize, inputMode, enterKeyHint, nonce,
    /// contentEditable, isContentEditable, autofocus).
    ///
    /// Tag-specific prototypes (e.g. `HTMLIFrameElement.prototype`)
    /// chain here, so the runtime proto chain is
    /// `HTMLIFrameElement.prototype → HTMLElement.prototype →
    /// Element.prototype → Node.prototype → EventTarget.prototype`.
    ///
    /// `None` until `register_html_element_prototype()` runs during
    /// `register_globals()` (after `register_element_prototype`,
    /// before `register_html_iframe_prototype`).
    #[cfg(feature = "engine")]
    pub(crate) html_element_prototype: Option<ObjectId>,
    /// `HTMLCollection.prototype` — shared prototype for every
    /// `ObjectKind::HtmlCollection` wrapper (WHATWG DOM §4.2.10).
    /// Chains to `Object.prototype`; carries `length` (getter),
    /// `item`, `namedItem`, and `[Symbol.iterator]`.
    ///
    /// `None` until `register_html_collection_prototype()` runs
    /// during `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) html_collection_prototype: Option<ObjectId>,
    /// `NodeList.prototype` — shared prototype for every
    /// `ObjectKind::NodeList` wrapper (WHATWG DOM §4.2.10.1).
    /// Chains to `Object.prototype`; carries `length`, `item`,
    /// `forEach`, and `[Symbol.iterator]`.
    ///
    /// `None` until `register_node_list_prototype()` runs during
    /// `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) node_list_prototype: Option<ObjectId>,
    /// `NamedNodeMap.prototype` — shared prototype for every
    /// `ObjectKind::NamedNodeMap` wrapper (WHATWG DOM §4.9.1).
    /// Carries `length`, `item`, `getNamedItem` / `setNamedItem` /
    /// `removeNamedItem` + the namespace-aware NS variants, and
    /// `[Symbol.iterator]`.
    #[cfg(feature = "engine")]
    pub(crate) named_node_map_prototype: Option<ObjectId>,
    /// Backing state for `ObjectKind::NamedNodeMap` wrappers — the
    /// Element entity whose attributes the map reflects.  Shared
    /// across repeated `element.attributes` reads because live
    /// semantics mean every accessor re-reads the same backing
    /// component regardless of which wrapper the caller holds.
    ///
    /// GC contract: `Entity` holds no `ObjectId` references, so no
    /// trace fan-out; sweep tail prunes entries whose key
    /// `ObjectId` was collected (pattern shared with
    /// `live_collection_states`).
    #[cfg(feature = "engine")]
    pub(crate) named_node_map_states: HashMap<ObjectId, elidex_ecs::Entity>,
    /// `Attr.prototype` — shared prototype for every
    /// `ObjectKind::Attr` wrapper (WHATWG DOM §4.9.2).  Carries the
    /// `name` / `value` / `ownerElement` / `namespaceURI` / `prefix`
    /// / `localName` / `specified` accessor suite.
    #[cfg(feature = "engine")]
    pub(crate) attr_prototype: Option<ObjectId>,
    /// Backing state for `ObjectKind::Attr` wrappers — the
    /// (owner Element, qualified-name `StringId`) tuple that ties
    /// each Attr back to its position in the owner's `Attributes`
    /// component.  An Attr with owner detached (attribute removed)
    /// surfaces `ownerElement === null` and `value === ""`.
    #[cfg(feature = "engine")]
    pub(crate) attr_states: HashMap<ObjectId, host::attr_proto::AttrState>,
    /// Identity cache for live `Attr` wrappers (WHATWG DOM §4.9.2).
    ///
    /// Keyed by `(owner Element entity, qualified-name StringId)`; a
    /// hit returns the same `ObjectId` so
    /// `el.getAttributeNode("id") === el.getAttributeNode("id")`
    /// (matches Chrome / Firefox / Safari).  `Attr.prototype.value` /
    /// `ownerElement` accessors read through to the owner's
    /// `Attributes` component on each call, so a single cached
    /// wrapper observes value mutations transparently.
    ///
    /// The cache is **invalidated** when the named attribute leaves
    /// the owner's attribute list — `removeAttribute`,
    /// `removeAttributeNode`, `toggleAttribute(off)`,
    /// `removeNamedItem`.  `setAttributeNode` / `setNamedItem`
    /// invalidate only when the passed-in Attr cannot remain
    /// canonical (cross-element source, or detached) — a live Attr
    /// already attached to the receiving element keeps the cache
    /// entry intact so `el.setAttributeNode(el.getAttributeNode("id"))`
    /// preserves identity.  Cross-element / detached arguments
    /// cannot be retargeted because the engine path does not
    /// mutate the passed-in Attr's `AttrState.owner` (Phase 2
    /// limitation paired with the existing AttrState ownership
    /// simplification).
    ///
    /// GC interaction: tracing fans out a cached `attr_id` only when
    /// the owner element wrapper is reachable (looked up via
    /// `HostData::wrapper_cache`); the sweep tail prunes entries
    /// whose `attr_id` was collected (same retain-on-key-mark pattern
    /// as `attr_states`).  This makes the cache effectively weak —
    /// it never extends an Attr's lifetime past its owner.
    #[cfg(feature = "engine")]
    pub(crate) attr_wrapper_cache: HashMap<(elidex_ecs::Entity, StringId), ObjectId>,
    /// `HTMLIFrameElement.prototype` — tag-specific intermediate
    /// prototype for `<iframe>` wrappers.  Chains to
    /// [`Self::html_element_prototype`] (after PR5b splice) so
    /// `iframe instanceof HTMLElement === true`.
    ///
    /// `None` until `register_html_iframe_prototype()` runs during
    /// `register_globals()` (after `register_html_element_prototype`).
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
    /// `AbortSignal.any(inputs)` fan-out — input `ObjectId` →
    /// composite signal `ObjectId`s that observe it (WHATWG DOM
    /// §3.1.3.3).  On input abort, [`host::abort::abort_signal`]
    /// removes the entry and fires each composite with the
    /// input's reason; composites' own `aborted` latch makes
    /// duplicate inputs (`any([a, a])`) safe.
    ///
    /// GC contract: **weak bookkeeping only** — composite
    /// ObjectIds stored in the values are NOT GC roots.  The mark
    /// phase deliberately skips this map so a `AbortSignal.any([a,
    /// b])` result that the user discards (without installing a
    /// listener or binding it to a long-lived slot) stays
    /// collectable even while `a` / `b` remain alive; otherwise
    /// tight loops that build composites and drop them would
    /// accumulate unreachable signals.  Composites survive when
    /// held via JS stack / global / upvalue paths in the normal
    /// way; the fan-out loop tolerates dead ObjectIds (the
    /// nested `abort_signal` call silently early-returns on a
    /// missing state entry).  Sweep prunes entries whose input
    /// key was collected, and filters each value list by composite
    /// liveness — see [`Self::abort_signal_states`] for the shared
    /// prune pattern.
    #[cfg(feature = "engine")]
    pub(crate) any_composite_map: HashMap<ObjectId, Vec<ObjectId>>,
    /// In-flight dispatch flag side table — WHATWG DOM §2.9 step 3
    /// rejects re-entrant `dispatchEvent()` on an event that is
    /// already propagating.  Kept out-of-band so `ObjectKind::Event`
    /// stays payload-free.  Membership is cleared at dispatch
    /// completion (happy path or throw), so a later sequential
    /// re-dispatch of the same instance succeeds.
    ///
    /// GC contract: sweep prunes entries whose key was collected,
    /// matching [`Self::abort_signal_states`].
    #[cfg(feature = "engine")]
    pub(crate) dispatched_events: HashSet<ObjectId>,
    /// `Event.prototype` (WebIDL §2.2).  Holds the four event methods
    /// (`preventDefault`, `stopPropagation`, `stopImmediatePropagation`,
    /// `composedPath`) and the `defaultPrevented` accessor, plus the
    /// `constructor` back-pointer to the `Event` global.  Methods are
    /// stateless `fn` pointers that match on `this`'s `ObjectKind::Event`
    /// for state, so a single prototype is shared across all dispatched
    /// events — avoids 5 native-fn allocations + 5 shape transitions
    /// per listener invocation.
    ///
    /// JS-visible via `globalThis.Event.prototype`.  Every
    /// `ObjectKind::Event` (UA-initiated or script-constructed)
    /// chains through this prototype.
    pub(crate) event_prototype: Option<ObjectId>,
    /// `CustomEvent.prototype` (WebIDL §2.3).  Chains to
    /// [`event_prototype`] and adds the `detail` accessor.  Set by
    /// `register_custom_event_global` during `register_globals`.
    #[cfg(feature = "engine")]
    pub(crate) custom_event_prototype: Option<ObjectId>,
    /// `UIEvent.prototype` (UI Events §3.1).  Chains to
    /// [`event_prototype`].  `view` / `detail` are own-data slots on
    /// every UIEvent-family instance (constructed via `new UIEvent` or
    /// any descendant ctor), kept in shape slot 9 / 10 so reads hit
    /// the own-property fast path.  `UIEvent.prototype` itself carries
    /// no instance state — it's the chain anchor for MouseEvent /
    /// KeyboardEvent / FocusEvent / InputEvent.  `None` until
    /// `register_ui_event_global()` runs during `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) ui_event_prototype: Option<ObjectId>,
    /// `MouseEvent.prototype` (UI Events §5.1).  Chains to
    /// [`ui_event_prototype`].  MouseEvent instances have `view` /
    /// `detail` + 13 mouse-specific slots (clientX/Y, button, buttons,
    /// altKey/ctrlKey/metaKey/shiftKey, screenX/Y, movementX/Y,
    /// relatedTarget) as own-data, matching WebIDL `[Unforgeable]`
    /// reflection.  `None` until `register_mouse_event_global()` runs.
    #[cfg(feature = "engine")]
    pub(crate) mouse_event_prototype: Option<ObjectId>,
    /// `KeyboardEvent.prototype` (UI Events §7.1).  Chains to
    /// [`ui_event_prototype`].  Adds 9 own-data slots (key, code,
    /// altKey/ctrlKey/metaKey/shiftKey, repeat, location, isComposing)
    /// beyond the UIEvent base.  `None` until
    /// `register_keyboard_event_global()` runs.
    #[cfg(feature = "engine")]
    pub(crate) keyboard_event_prototype: Option<ObjectId>,
    /// `FocusEvent.prototype` (UI Events §6.1).  Chains to
    /// [`ui_event_prototype`].  Adds `relatedTarget` own-data slot.
    #[cfg(feature = "engine")]
    pub(crate) focus_event_prototype: Option<ObjectId>,
    /// `InputEvent.prototype` (UI Events §8.1).  Chains to
    /// [`ui_event_prototype`].  Adds `inputType` / `data` /
    /// `isComposing` own-data slots.
    #[cfg(feature = "engine")]
    pub(crate) input_event_prototype: Option<ObjectId>,
    /// `PromiseRejectionEvent.prototype` (HTML §8.1.7.3.4).  Chains to
    /// [`event_prototype`] (sibling of UIEvent, not descendant).  Adds
    /// `promise` / `reason` own-data slots.
    #[cfg(feature = "engine")]
    pub(crate) promise_rejection_event_prototype: Option<ObjectId>,
    /// `ErrorEvent.prototype` (HTML §8.1.7.2).  Chains to
    /// [`event_prototype`].  Adds `message` / `filename` / `lineno` /
    /// `colno` / `error` own-data slots.
    #[cfg(feature = "engine")]
    pub(crate) error_event_prototype: Option<ObjectId>,
    /// `HashChangeEvent.prototype` (HTML §8.1.3).  Chains to
    /// [`event_prototype`].  Adds `oldURL` / `newURL` own-data slots
    /// (reuses the UA-dispatch `hash_change` shape).
    #[cfg(feature = "engine")]
    pub(crate) hash_change_event_prototype: Option<ObjectId>,
    /// `PopStateEvent.prototype` (HTML §8.8.1).  Chains to
    /// [`event_prototype`].  Adds `state` own-data slot.
    #[cfg(feature = "engine")]
    pub(crate) pop_state_event_prototype: Option<ObjectId>,
    /// `Headers.prototype` (WHATWG Fetch §5.2).  Chains to
    /// `Object.prototype` — Headers is a WebIDL interface with no
    /// EventTarget ancestry.  Holds `append` / `set` / `delete` /
    /// `get` / `has` / `getSetCookie` / `forEach` / `keys` /
    /// `values` / `entries` methods plus `[Symbol.iterator]`.
    /// Per-instance list and guard live in
    /// [`Self::headers_states`], keyed by the instance's `ObjectId`.
    ///
    /// `None` until `register_headers_global()` runs during
    /// `register_globals()` (after `register_prototypes` so
    /// `object_prototype` is populated).  Engine-gated because every
    /// consumer (Fetch API surface) is itself engine-only.
    #[cfg(feature = "engine")]
    pub(crate) headers_prototype: Option<ObjectId>,
    /// Per-`Headers` out-of-band state, keyed by the instance's own
    /// `ObjectId`.  Same pattern as [`Self::abort_signal_states`]:
    /// payload lives here so [`ObjectKind::Headers`] stays
    /// payload-free (preserves per-variant size discipline).
    ///
    /// Entries hold interned `StringId`s only (name / value are
    /// pool-permanent), so the trace step has nothing to mark.
    ///
    /// GC contract: sweep tail prunes entries whose key `ObjectId`
    /// was collected so a recycled slot can't observe a stale list
    /// or guard — matching `abort_signal_states` /
    /// `dom_exception_states`.
    #[cfg(feature = "engine")]
    pub(crate) headers_states: HashMap<ObjectId, host::headers::HeadersState>,
    /// `Request.prototype` (WHATWG Fetch §5.3).  Chains to
    /// `Object.prototype` (no EventTarget / Node ancestry).  Holds
    /// the IDL accessor suite (`method` / `url` / `headers` /
    /// `body` / `bodyUsed` / `redirect` / `mode` / `credentials`
    /// / `cache`) plus `clone`.
    ///
    /// `None` until `register_request_global()` runs during
    /// `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) request_prototype: Option<ObjectId>,
    /// Per-`Request` out-of-band state, keyed by the instance's
    /// own `ObjectId`.  Payload lives here so
    /// [`ObjectKind::Request`] stays payload-free.
    ///
    /// GC contract: trace marks `headers_id` (the paired Headers
    /// instance) so it survives alongside the Request.  URL /
    /// method are pool-permanent `StringId`s (no marking needed).
    /// Sweep tail prunes entries whose key `ObjectId` was
    /// collected so a recycled slot can't inherit stale state —
    /// matching `headers_states` / `abort_signal_states`.
    #[cfg(feature = "engine")]
    pub(crate) request_states: HashMap<ObjectId, host::request_response::RequestState>,
    /// `Response.prototype` (WHATWG Fetch §5.5).  Chains to
    /// `Object.prototype`.  Holds the IDL accessor suite
    /// (`status` / `ok` / `statusText` / `url` / `type` /
    /// `headers` / `body` / `bodyUsed` / `redirected`) plus
    /// `clone`.
    ///
    /// The `Response` constructor function itself carries three
    /// static factories — `Response.error` / `Response.redirect` /
    /// `Response.json` — installed on the ctor in
    /// `register_response_global`.
    ///
    /// `None` until `register_response_global()` runs during
    /// `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) response_prototype: Option<ObjectId>,
    /// Per-`Response` out-of-band state, keyed by the instance's
    /// own `ObjectId`.  Payload lives here so
    /// [`ObjectKind::Response`] stays payload-free and IDL
    /// readonly attrs can read from the authoritative internal
    /// slot rather than observable own-data (PR5a2 R7.1 lesson:
    /// `delete resp.url` must not affect `resp.url` reads).
    ///
    /// GC contract: identical to `request_states` — mark
    /// `headers_id`, prune dead keys in the sweep tail.
    #[cfg(feature = "engine")]
    pub(crate) response_states: HashMap<ObjectId, host::request_response::ResponseState>,
    /// Shared body byte storage for `Request` / `Response` /
    /// `ArrayBuffer` and the Body mixin read methods (`text` /
    /// `json` / `arrayBuffer` / `blob`).  `Blob` payloads live in
    /// the separate [`Self::blob_data`] table (R20.2); don't
    /// conflate them — future zero-copy / GC-sweep decisions
    /// pivot on which side table owns the bytes.  Keyed by the
    /// owning object's `ObjectId`; the value is an owned
    /// `Vec<u8>`, so TypedArray / DataView writes mutate it in
    /// place via [`super::host::byte_io`] (single-threaded VM,
    /// no shared mutability needed inside `body_data`).  Cross-
    /// subsystem callers that need to ferry bytes across an
    /// ownership boundary (`fetch` HTTP handoff,
    /// `body_mixin::take_body_bytes`, `structured_clone`,
    /// `array_buffer::array_buffer_view_bytes`) take an owned
    /// snapshot at the boundary — by `clone`, `remove`, or
    /// sub-range `to_vec` depending on whether the consumer is
    /// non-destructive or one-shot.  Some boundaries keep that
    /// snapshot as `Vec<u8>`; others convert it to `Arc<[u8]>`
    /// only when the downstream API requires shared-immutable
    /// bytes (`fetch` → `Bytes::from_owner` needs `Send + Sync`,
    /// `BlobData` stores `Arc<[u8]>` per-spec immutability).
    /// The snapshot semantics that the previous immutable-`Arc`
    /// storage delivered implicitly are now visible in those
    /// boundary APIs' types.
    ///
    /// Requests / Responses without body bytes simply omit their
    /// entry.  In Phase 2 the `.body` IDL getter is always `null`
    /// because `ReadableStream` is deferred to the PR5-streams
    /// tranche; the Body mixin read methods (`text` / `json` /
    /// `arrayBuffer` / `blob`) read directly from this map, so
    /// key presence is the "does this carry bytes?" signal rather
    /// than the JS-visible `.body` getter.
    ///
    /// GC contract: the values hold no `ObjectId` references, so
    /// the trace step skips them.  Sweep tail drops entries whose
    /// key was collected (matching `headers_states`) so that a
    /// recycled slot does not inherit stale bytes.
    #[cfg(feature = "engine")]
    pub(crate) body_data: HashMap<ObjectId, Vec<u8>>,
    /// One-shot "body already consumed" flag (WHATWG §5 `bodyUsed`).
    /// Inserted by the Body mixin read methods (`text()` /
    /// `.json()` / `.arrayBuffer()` / `.blob()`) the first time
    /// any one of them runs on a given object; a second call on
    /// the same object then rejects its returned Promise with
    /// `TypeError`.  The `.bodyUsed` IDL getter reads membership
    /// directly.
    ///
    /// GC contract: sweep tail prunes entries whose key was
    /// collected, same as the other side tables.
    #[cfg(feature = "engine")]
    pub(crate) body_used: HashSet<ObjectId>,
    /// `ArrayBuffer.prototype` (ES2020 §24.1, minimal Phase 2 form
    /// — `byteLength` getter + `slice` method only; TypedArray
    /// views are deferred to the next tranche).  Chains to
    /// `Object.prototype`.
    ///
    /// `None` until `register_array_buffer_global()` runs during
    /// `register_globals()`.  Per-instance byte storage shares the
    /// [`Self::body_data`] map so ArrayBuffer / Request / Response
    /// all prune through the same sweep path.
    #[cfg(feature = "engine")]
    pub(crate) array_buffer_prototype: Option<ObjectId>,
    /// `Blob.prototype` (File API §3, minimal Phase 2 form).
    /// Chains to `Object.prototype`.  Holds `size` / `type`
    /// getters + `slice` / `text` / `arrayBuffer` methods.
    ///
    /// `None` until `register_blob_global()` runs during
    /// `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) blob_prototype: Option<ObjectId>,
    /// Abstract `%TypedArray%.prototype` (ES2024 §23.2.3).  Shared
    /// parent of all 11 concrete subclass prototypes
    /// (`Uint8Array.prototype` et al., each of which chains here via
    /// `register_typed_array_subclass`).  Chains to `Object.prototype`.
    /// Carries the generic `buffer` / `byteOffset` / `byteLength` /
    /// `length` accessors + `@@toStringTag` getter — instance-method
    /// suite lands with PR5-typed-array §C4.
    ///
    /// `None` until `register_typed_array_prototype_global()` runs
    /// during `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) typed_array_prototype: Option<ObjectId>,
    /// `DataView.prototype` (ES2024 §25.3).  Chains directly to
    /// `Object.prototype` (DataView does NOT inherit from
    /// `%TypedArray%.prototype` — it's a sibling view type).  Method
    /// suite lands with PR5-typed-array §C5.
    #[cfg(feature = "engine")]
    pub(crate) data_view_prototype: Option<ObjectId>,
    /// Per-subclass TypedArray prototypes (ES §23.2.7), addressed
    /// by [`value::ElementKind::index`].  Each entry chains to
    /// [`Self::typed_array_prototype`].  Slots stay `None` until
    /// `register_typed_array_subclass()` runs for the corresponding
    /// [`value::ElementKind`] during `register_globals()`.  Stored
    /// as a fixed-size array so the GC trace can fold all eleven
    /// subclasses behind a single iterator (see `gc.rs`
    /// `proto_roots` / `subclass_array_proto_roots` split).
    #[cfg(feature = "engine")]
    pub(crate) subclass_array_prototypes: [Option<ObjectId>; value::ElementKind::COUNT],
    /// Per-subclass TypedArray constructors (ES §23.2.6), parallel
    /// to [`Self::subclass_array_prototypes`] and addressed by the
    /// same [`value::ElementKind::index`].  Reverse mapping
    /// (`ctor ObjectId → ElementKind`) supports the static
    /// `%TypedArray%.of` / `%TypedArray%.from` natives, which
    /// inspect `this` (the calling subclass ctor) to decide which
    /// concrete subclass to materialise.  Linear scan over the
    /// 11-entry array is cheap; no `HashMap` overhead.  Slots stay
    /// `None` until `register_typed_array_subclass()` runs.
    ///
    /// These entries are strong internal references and **must be
    /// traced by GC** in parallel with
    /// [`Self::subclass_array_prototypes`] — chained into the GC
    /// root set via `subclass_array_ctor_roots` in `gc.rs`.  Without
    /// tracing, severing the global ctor reference (`delete
    /// globalThis.Uint8Array`) could let the ctor be collected
    /// while this reverse-lookup table still holds a stale id.
    #[cfg(feature = "engine")]
    pub(crate) subclass_array_ctors: [Option<ObjectId>; value::ElementKind::COUNT],
    /// `TextEncoder.prototype` (WHATWG Encoding §8.2).  Chains
    /// directly to `Object.prototype`.  `None` until
    /// `register_text_encoder_global()` runs during
    /// `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) text_encoder_prototype: Option<ObjectId>,
    /// `TextDecoder.prototype` (WHATWG Encoding §8.1).  Chains
    /// directly to `Object.prototype`.  `None` until
    /// `register_text_decoder_global()` runs.
    #[cfg(feature = "engine")]
    pub(crate) text_decoder_prototype: Option<ObjectId>,
    /// Per-`Blob` out-of-band state, keyed by the instance's own
    /// `ObjectId`.  Separate from [`Self::body_data`] because a
    /// Blob carries a `type_sid` alongside its bytes; folding both
    /// into one map would force every Request / Response entry to
    /// carry a phantom type slot.
    ///
    /// GC contract: bytes are plain `Arc<[u8]>` with no ObjectId
    /// references, so the trace step does nothing.  Sweep tail
    /// prunes entries whose key `ObjectId` was collected — same
    /// pattern as `body_data` / `headers_states`.
    #[cfg(feature = "engine")]
    pub(crate) blob_data: HashMap<ObjectId, host::blob::BlobData>,
    /// Per-`TextDecoder` out-of-band state (WHATWG Encoding §8.1).
    /// Keyed by the instance's own `ObjectId`.  Holds the resolved
    /// encoding, the user-chosen `fatal` / `ignoreBOM` flags, and
    /// the live `encoding_rs::Decoder` whose `BOM` handling + partial
    /// sequence state persist across streaming `decode(..., {stream:
    /// true})` calls.
    ///
    /// GC contract: the payload holds no `ObjectId` references
    /// (`encoding` is `&'static`, `Decoder` is opaque to us), so
    /// the trace step does nothing.  Sweep tail prunes entries
    /// whose key `ObjectId` was collected — same pattern as
    /// `blob_data` / `headers_states`.
    #[cfg(feature = "engine")]
    pub(crate) text_decoder_states: HashMap<ObjectId, host::text_encoding::TextDecoderState>,
    /// Backing state for `ObjectKind::HtmlCollection` /
    /// `ObjectKind::NodeList` wrappers (WHATWG DOM §4.2.10 / §4.2.10.1).
    ///
    /// Shared between both collection interfaces because the filter
    /// discriminator already distinguishes HTMLCollection kinds
    /// (tag / class / children / forms / …) from NodeList kinds
    /// (childNodes / querySelectorAll snapshot / getElementsByName).
    /// One `HashMap` keeps the GC sweep tail tidy and lets the
    /// indexed / named property lookup in `ops_property::get_element`
    /// hit a single side-table regardless of the wrapper kind.
    ///
    /// GC contract: the stored `(LiveCollectionKind,
    /// LiveCollectionCache)` tuple holds only `Entity`, `StringId`,
    /// `Vec<StringId>` (class names), `Vec<Entity>` (querySelectorAll
    /// snapshot + per-wrapper SP2 entity-list cache), and
    /// `Cell<Option<u64>>` (cache version, `None` until the first
    /// miss-path populates it) — **no `ObjectId` references**, so
    /// the trace step does nothing.  The sweep tail prunes entries
    /// whose key `ObjectId` was collected, same pattern as
    /// `headers_states` / `blob_data`.
    #[cfg(feature = "engine")]
    pub(crate) live_collection_states: HashMap<
        ObjectId,
        (
            host::dom_collection::LiveCollectionKind,
            host::dom_collection::LiveCollectionCache,
        ),
    >,
    /// Content-thread `NetworkHandle` used by the `fetch()` host
    /// global.  `None` in test / standalone mode (`fetch()` then
    /// rejects with `TypeError`); the embedding harness —
    /// typically `elidex-shell` — installs a handle via
    /// [`Vm::install_network_handle`] after VM construction.
    ///
    /// Wrapped in `Rc` because every [`NetworkHandle`](elidex_net::broker::NetworkHandle)
    /// carries a [`RefCell<Vec<_>>`](std::cell::RefCell) of buffered
    /// events and so is `!Send + !Sync`.  The content thread is
    /// single-threaded (matches [`Vm`]'s own `!Send + !Sync`
    /// invariant from `host_data.rs`), so `Rc` instead of `Arc`
    /// is the tighter fit.  Each content thread owns its own
    /// handle; worker threads (future) allocate sibling handles
    /// via [`NetworkHandle::create_sibling_handle`].
    ///
    /// GC contract: this is Rust-owned, not a JS object — the GC
    /// does not mark / sweep it, and dropping the `Rc` at `Vm`
    /// teardown releases the handle.
    #[cfg(feature = "engine")]
    pub(crate) network_handle: Option<std::rc::Rc<elidex_net::broker::NetworkHandle>>,
    /// Fan-out map for `AbortSignal` → in-flight `FetchId`s.  When a
    /// signal aborts, [`host::abort::abort_signal`] drains the entry
    /// for that signal's `ObjectId` and sends
    /// [`elidex_net::broker::RendererToNetwork::CancelFetch`] for each
    /// recorded fetch so the broker can discard the response.
    ///
    /// ## Phase 2 limitation (documented)
    ///
    /// `NetworkHandle::fetch_blocking` blocks the content thread, so
    /// JS never runs between `fetch()` entry and the broker reply.
    /// No user code can therefore fire an abort mid-flight — the
    /// map stays empty for the lifetime of a blocking fetch and the
    /// drain loop in `abort_signal` is dead code until the PR5-async-
    /// fetch refactor lands.  The wire is in place so the async
    /// refactor only has to populate on entry and prune on broker
    /// reply.
    ///
    /// GC contract: sweep prunes entries whose key (signal) was
    /// collected, matching [`Self::abort_signal_states`] /
    /// [`Self::any_composite_map`].  Entries with live signal keys
    /// are retained as-is; the `FetchId`s inside are plain `u64`s
    /// that carry no GC obligations.
    #[cfg(feature = "engine")]
    pub(crate) fetch_abort_observers: HashMap<ObjectId, Vec<elidex_net::broker::FetchId>>,
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
    /// Backing storage for `window.name` (WHATWG HTML §7.3) — held
    /// as a `StringId` so the getter is a single field read and the
    /// setter stores `coerce::to_string`'s result directly without
    /// the round-trip through `String`.  Initialised to the
    /// well-known empty-string id per spec.  The §7.10.4 step 7
    /// cross-document navigation reset is **not** currently applied
    /// to this field (only init and the setter write to it); a
    /// future navigation-pipeline change must clear this slot when
    /// that step lands.
    #[cfg(feature = "engine")]
    pub(crate) window_name: StringId,
    /// HTML §8.1.5 same-window task queue.  Currently populated only
    /// by `window.postMessage`; drained at the end of every
    /// `VmInner::eval` after the microtask flush.  See
    /// [`host::pending_tasks`] for the full task shape and GC
    /// contract.
    #[cfg(feature = "engine")]
    pub(crate) pending_tasks: VecDeque<host::pending_tasks::PendingTask>,
    /// Reentrancy guard for [`Self::drain_tasks`] — nested drain
    /// calls (triggered by a listener body that enqueued and
    /// drained inline) are no-ops, matching the microtask queue's
    /// drain-depth invariant.
    #[cfg(feature = "engine")]
    pub(crate) task_drain_depth: u32,
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
