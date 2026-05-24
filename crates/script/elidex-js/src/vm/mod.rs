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
#[cfg(feature = "engine")]
pub mod consumer_dispatcher;
mod coroutine_types;
mod dispatch;
mod dispatch_helpers;
mod dispatch_ic;
mod dispatch_iter;
mod dispatch_objects;
mod error;
pub(crate) mod gc;
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
pub(crate) mod webidl_sequence;
mod well_known;
#[cfg(feature = "engine")]
pub(crate) mod worker_thread;
#[cfg(feature = "engine")]
pub(crate) mod wrapper_intern;

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

/// The kind of global scope a [`Vm`] realizes.
///
/// Read by `register_globals` to fork the Window-only prototype block: a
/// Window VM (WHATWG HTML §7.2) installs `window` / `document` / `location` /
/// `history`, whereas a dedicated worker VM (WHATWG HTML §10.2.1.1) installs
/// the `WorkerGlobalScope` surface (`self` / `postMessage` / `close` /
/// `importScripts` / `WorkerLocation` / `WorkerNavigator`) and never binds a
/// document. Set once at construction, before `register_globals` runs.
#[derive(Clone, Debug)]
pub enum GlobalScopeKind {
    /// Main-thread Window scope (WHATWG HTML §7.2).
    Window,
    /// Dedicated worker scope (WHATWG HTML §10.2.1.1), carrying the worker
    /// name + script URL needed to build `name` / `WorkerLocation` /
    /// `WorkerNavigator` and to label uncaught-error reports.
    ///
    /// `engine`-only: the whole worker surface is feature-gated, and the
    /// `credentials` field references `elidex_net` (an `engine`-only dep), so
    /// the variant must not exist in non-`engine` builds.
    #[cfg(feature = "engine")]
    DedicatedWorker {
        /// Worker name (`new Worker(url, { name })`; empty when unnamed).
        name: String,
        /// Worker script URL — source for `WorkerLocation` and error filename.
        script_url: url::Url,
        /// Whether the worker runs in a secure context (WHATWG HTML §8.1.3.5 /
        /// W3C Secure Contexts): inherited from the **creator's** environment,
        /// not derived from `script_url` (a `data:` / `blob:` worker spawned by
        /// a secure parent is itself secure).
        is_secure_context: bool,
        /// Credentials mode for the worker's own subresource fetches
        /// (`importScripts`, WHATWG HTML §10.2.6.3 `WorkerOptions.credentials`).
        /// Applied — with the worker's origin — to the `importScripts` request
        /// so cookie attachment is gated correctly.
        credentials: elidex_net::CredentialsMode,
    },
}

/// The internal state of the VM, exposed to native functions via `NativeContext`.
// Not an API surface: this is the VM's monolithic interpreter-state struct, not
// a config object — the `struct_excessive_bools` ergonomics lint (aimed at
// builder/argument structs) does not apply.
#[allow(clippy::struct_excessive_bools)]
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
    /// Key bound to the native accessor currently executing — staged from
    /// [`value::NativeFunction::bound_key`] for the duration of the native
    /// call (save/restore around dispatch). A shared backend fn reads it via
    /// [`value::NativeContext::bound_key`] to recover which property it serves.
    /// `None` outside a bound-accessor call.
    pub(crate) active_bound_key: Option<value::StringId>,
    /// Host-provided data for browser shell integration (event listeners,
    /// DOM wrappers, timers, etc.).  `None` when the VM runs standalone
    /// (e.g., in unit tests without the `engine` feature).
    pub(crate) host_data: Option<Box<host_data::HostData>>,
    /// DOM API handler dispatch table.  Initialized once at `Vm::new`
    /// (engine feature only) and shared across every native DOM
    /// method invocation via `vm/host/dom_bridge.rs::invoke_dom_api`.
    /// Keeping the `DomApiHandler` dispatch path on the engine-
    /// independent side enforces the CLAUDE.md "Layering mandate".
    #[cfg(feature = "engine")]
    pub(crate) dom_registry: std::rc::Rc<elidex_dom_api::registry::DomHandlerRegistry>,
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
    /// `DOMTokenList.prototype` — shared prototype for every
    /// `ObjectKind::DOMTokenList` wrapper backing
    /// `Element.classList` (WHATWG DOM §3.5 / §7.1).  Chains to
    /// `Object.prototype`; carries `length` / `value` accessors,
    /// `item` / `contains` / `add` / `remove` / `toggle` / `replace`
    /// / `supports` methods, and `[Symbol.iterator]`.
    #[cfg(feature = "engine")]
    pub(crate) dom_token_list_prototype: Option<ObjectId>,
    /// `DOMStringMap.prototype` — shared prototype for every
    /// `ObjectKind::DOMStringMap` wrapper backing
    /// `HTMLElement.dataset` (WHATWG HTML §3.2.6 / WebIDL §3.10).
    /// Chains to `Object.prototype`; carries no own members — the
    /// named-property exotic semantics are dispatched directly from
    /// `ops_element` / `ops_property` / `dispatch_iter` /
    /// `coerce_format` based on `ObjectKind`.
    #[cfg(feature = "engine")]
    pub(crate) dom_string_map_prototype: Option<ObjectId>,
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
    /// `ShadowRoot.prototype` — shared prototype for every shadow root
    /// wrapper (WHATWG DOM §4.8).  ShadowRoot wrappers are themselves
    /// `ObjectKind::HostObject { entity_bits }` whose backing entity
    /// carries the `elidex_ecs::ShadowRoot` component
    /// ([feedback_objectkind-resolution-uniformity]); identity across
    /// reads is preserved via `HostData::wrapper_cache` like Element
    /// wrappers.  Carries the `host` / `mode` / `delegatesFocus` /
    /// `slotAssignment` / `clonable` / `serializable` accessor suite.
    /// Chains to `DocumentFragment.prototype`.
    #[cfg(feature = "engine")]
    pub(crate) shadow_root_prototype: Option<ObjectId>,
    /// `DocumentFragment.prototype` — shared prototype for every
    /// `DocumentFragment` node wrapper (`document.createDocumentFragment()`,
    /// `<template>.content`, ShadowRoot inherits from this).  Chains
    /// to `Node.prototype` and carries the full ParentNode mixin
    /// (WHATWG §4.2.6) — mutation methods via
    /// [`Self::install_parent_node_mixin`] and the read surface
    /// (`children` / `firstElementChild` / `lastElementChild` /
    /// `childElementCount` / `querySelector` / `querySelectorAll`) via
    /// [`Self::install_parent_node_readers`].  Installed by
    /// `register_document_fragment_prototype` in `init.rs` after the
    /// Node prototype + ParentNode mixin natives are available.
    #[cfg(feature = "engine")]
    pub(crate) document_fragment_prototype: Option<ObjectId>,
    /// `HTMLSlotElement.prototype` — shared prototype for every
    /// `<slot>` element wrapper (WHATWG HTML §4.12.4).  Carries the
    /// `name` reflected attribute + `assign` / `assignedNodes` /
    /// `assignedElements` methods.  Chains to `HTMLElement.prototype`.
    #[cfg(feature = "engine")]
    pub(crate) html_slot_prototype: Option<ObjectId>,
    /// Signal-slots set for the `slotchange` event (WHATWG DOM
    /// §4.2.2.5 "signal a slot change" + §4.3.4 "notify mutation
    /// observers").  Each `<slot>` entity appended here gets a
    /// `slotchange` Event fired at it (bubbles=true, composed=false)
    /// at the next microtask checkpoint.  Drained from
    /// [`super::host::html_slot_proto::dispatch_pending_slotchange_signals`]
    /// at the end of `drain_microtasks`.
    ///
    /// `VecDeque` for O(1) front-pop in FIFO drain order; dedup on
    /// append uses a linear scan because the set is typically tiny
    /// (a handful per microtask burst — even a list-view re-render
    /// signals once per slot, not per item).  Order is preserved
    /// per the spec's "signal slots" ordered-set semantics.
    #[cfg(feature = "engine")]
    pub(crate) pending_slot_change_signals: std::collections::VecDeque<elidex_ecs::Entity>,
    /// Coalescing flag for the "notify mutation observers" microtask
    /// (WHATWG DOM §4.3.4 step 1).  Set to `true` when
    /// [`super::host::html_slot_proto::VmInner::signal_slot_change`]
    /// enqueues the first signal of a tick and resets to `false`
    /// when the microtask dispatches.  Ensures exactly one
    /// `slotchange` checkpoint per microtask burst, ordered at
    /// signal time relative to subsequent `Promise.then` callbacks
    /// (NOT at drain-tail).
    #[cfg(feature = "engine")]
    pub(crate) mutation_observer_microtask_queued: bool,
    /// `CSSStyleDeclaration.prototype` — shared prototype for every
    /// `ObjectKind::CSSStyleDeclaration` wrapper backing both
    /// `Element.style` (Inline source) and `getComputedStyle`
    /// (Computed source).  Chains to `Object.prototype`; carries
    /// `length` / `cssText` accessors and the
    /// `getPropertyValue` / `getPropertyPriority` / `setProperty` /
    /// `removeProperty` / `item` methods.
    #[cfg(feature = "engine")]
    pub(crate) css_style_declaration_prototype: Option<ObjectId>,
    /// `CSSStyleSheet.prototype` (CSSOM §6.2).  Chains to
    /// `Object.prototype`; carries `cssRules` / `ownerNode` /
    /// `type` / `disabled` / `href` / `media` accessors and
    /// `insertRule` / `deleteRule` methods.  `None` until
    /// `register_cssom_sheet_prototypes()` runs in `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) css_stylesheet_prototype: Option<ObjectId>,
    /// `CSSRuleList.prototype` — `length` accessor + `item` method.
    #[cfg(feature = "engine")]
    pub(crate) css_rule_list_prototype: Option<ObjectId>,
    /// `CSSStyleRule.prototype` (CSSOM §6.6) — `cssText` /
    /// `selectorText` accessors and `style` accessor.
    #[cfg(feature = "engine")]
    pub(crate) css_style_rule_prototype: Option<ObjectId>,
    /// `StyleSheetList.prototype` (CSSOM §6.8) — `length` accessor +
    /// `item` method.
    #[cfg(feature = "engine")]
    pub(crate) style_sheet_list_prototype: Option<ObjectId>,
    /// `MutationObserver.prototype` (WHATWG DOM §4.3).  Chains to
    /// `Object.prototype` and carries the `observe` / `disconnect` /
    /// `takeRecords` methods.  `None` until
    /// `register_mutation_observer_global()` runs during
    /// `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) mutation_observer_prototype: Option<ObjectId>,
    /// `ResizeObserver.prototype` (W3C Resize Observer §3.1).  Chains
    /// to `Object.prototype` and carries the `observe` / `unobserve` /
    /// `disconnect` methods.  `None` until
    /// `register_resize_observer_global()` runs during
    /// `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) resize_observer_prototype: Option<ObjectId>,
    /// `IntersectionObserver.prototype` (W3C Intersection Observer §3.1).
    /// Chains to `Object.prototype` and carries the `observe` /
    /// `unobserve` / `disconnect` / `takeRecords` methods.  `None` until
    /// `register_intersection_observer_global()` runs during
    /// `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) intersection_observer_prototype: Option<ObjectId>,
    /// `Range.prototype` (WHATWG DOM §4.4).  Chains to
    /// `Object.prototype`.  Carries the 23 Range surface members +
    /// 4 boundary-compare constants.  `None` until
    /// `register_range_global()` runs.
    #[cfg(feature = "engine")]
    pub(crate) range_prototype: Option<ObjectId>,
    /// `StaticRange.prototype` (WHATWG DOM §4.5).  Chains to
    /// `Object.prototype`.  Carries the 5 AbstractRange-derived
    /// readonly props + `collapsed` + `isValid()`.
    #[cfg(feature = "engine")]
    pub(crate) static_range_prototype: Option<ObjectId>,
    /// `TreeWalker.prototype` (WHATWG DOM §6.4).  Chains to
    /// `Object.prototype`.  Carries `root` / `whatToShow` / `filter` /
    /// `currentNode` accessors + 7 traversal methods.
    #[cfg(feature = "engine")]
    pub(crate) tree_walker_prototype: Option<ObjectId>,
    /// `NodeIterator.prototype` (WHATWG DOM §6.1).  Chains to
    /// `Object.prototype`.  Carries 5 readonly props + `nextNode` /
    /// `previousNode` / `detach`.
    #[cfg(feature = "engine")]
    pub(crate) node_iterator_prototype: Option<ObjectId>,
    /// `Selection.prototype` (Selection API §3).  Chains to
    /// `Object.prototype`.  Carries 8 readonly props (anchorNode /
    /// anchorOffset / focusNode / focusOffset / isCollapsed /
    /// rangeCount / type / direction) + 15 methods (getRangeAt /
    /// addRange / removeRange / removeAllRanges / empty / collapse /
    /// setPosition (alias) / collapseToStart / collapseToEnd / extend /
    /// setBaseAndExtent / selectAllChildren / deleteFromDocument /
    /// containsNode / toString).  `None` until
    /// `register_selection_global()` runs.
    #[cfg(feature = "engine")]
    pub(crate) selection_prototype: Option<ObjectId>,
    /// `Storage.prototype` (WHATWG HTML §11.2).  Chains to
    /// `Object.prototype` and carries `getItem` / `setItem` /
    /// `removeItem` / `clear` / `key` / `length`.  `None` until
    /// `register_storage_global()` runs during `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) storage_prototype: Option<ObjectId>,
    /// `StorageEvent.prototype` (WHATWG HTML §11.4.2).  Chains to
    /// `Event.prototype`.  `None` until
    /// `register_storage_event_global()` runs.
    #[cfg(feature = "engine")]
    pub(crate) storage_event_prototype: Option<ObjectId>,
    /// Cached `Storage` wrapper for `window.localStorage`
    /// (`[SameObject]` per WebIDL — same `ObjectId` returned across
    /// reads for the lifetime of one bind cycle).  Cleared on
    /// `Vm::unbind` to avoid cross-origin data leaking through a
    /// retained reference after a rebind to a different document.
    #[cfg(feature = "engine")]
    pub(crate) storage_local_instance: Option<ObjectId>,
    /// Cached `Storage` wrapper for `window.sessionStorage` —
    /// mirror of [`Self::storage_local_instance`].  Cleared on
    /// `Vm::unbind`.
    #[cfg(feature = "engine")]
    pub(crate) storage_session_instance: Option<ObjectId>,
    /// `Crypto.prototype` (WebCrypto §10).  Chains to
    /// `Object.prototype`.  Carries `getRandomValues` / `randomUUID`
    /// methods + `subtle` accessor.  `None` until
    /// `register_crypto_global()` runs during `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) crypto_prototype: Option<ObjectId>,
    /// `SubtleCrypto.prototype` (WebCrypto §14).  Chains to
    /// `Object.prototype`.  Carries `digest` (current scope; full
    /// surface in `#11-crypto-subtle-full`).  `None` until
    /// `register_subtle_crypto_global()` runs.
    #[cfg(feature = "engine")]
    pub(crate) subtle_crypto_prototype: Option<ObjectId>,
    /// `WebSocket.prototype` (WHATWG WebSockets §9.3).  Chains
    /// directly to `Object.prototype` (NOT `EventTarget.prototype`
    /// in this PR — addEventListener delivery for non-Entity
    /// EventTargets is deferred to `#11-realtime-event-listeners`).
    /// Carries readyState / url / protocol / extensions /
    /// bufferedAmount / binaryType accessors, send / close
    /// methods, and 4 `on*` handler getter/setter pairs.
    /// `CONNECTING` / `OPEN` / `CLOSING` / `CLOSED` IDL constants
    /// are installed on BOTH this prototype and the
    /// `WebSocket` constructor object.  `None` until
    /// `register_websocket_global()` runs during
    /// `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) websocket_prototype: Option<ObjectId>,
    /// `EventSource.prototype` (WHATWG HTML §9.2).  Chains
    /// directly to `Object.prototype` (same scope decision as
    /// `websocket_prototype`; the per-instance addEventListener
    /// shim is a minimal CRIT-3 fold, not the full EventTarget
    /// surface).  Carries readyState / url / withCredentials
    /// accessors, close / addEventListener / removeEventListener
    /// methods, and 3 `on*` handler getter/setter pairs.
    /// `CONNECTING` / `OPEN` / `CLOSED` IDL constants installed
    /// on both the prototype and the `EventSource` constructor.
    /// `None` until `register_event_source_global()` runs.
    #[cfg(feature = "engine")]
    pub(crate) event_source_prototype: Option<ObjectId>,
    /// Cached `Crypto` wrapper for `window.crypto` (`[SameObject]`
    /// per WebIDL — same `ObjectId` returned across reads for the
    /// lifetime of one bind cycle).  Eager-initialised at
    /// `register_crypto_global()` since `window.crypto` is always
    /// reachable from `globalThis`.  Cleared on `Vm::unbind`.
    #[cfg(feature = "engine")]
    pub(crate) crypto_instance: Option<ObjectId>,
    /// Cached `SubtleCrypto` wrapper for `crypto.subtle`
    /// (`[SameObject]` per WebIDL).  Lazily allocated on the first
    /// `Crypto.prototype.subtle` accessor read via
    /// `alloc_or_cached_subtle_crypto`.  Cleared on `Vm::unbind`.
    #[cfg(feature = "engine")]
    pub(crate) subtle_crypto_instance: Option<ObjectId>,
    /// `CustomElementRegistry.prototype` (HTML §4.13.4). Chains to
    /// `Object.prototype`. `None` until
    /// `register_custom_element_registry_global()` runs during
    /// `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) custom_element_registry_prototype: Option<ObjectId>,
    /// Cached `CustomElementRegistry` singleton wrapper exposed as
    /// `window.customElements` (per-VM identity per HTML §4.13.4).
    /// Eager-initialised at `register_custom_element_registry_global()`.
    /// Cleared on `Vm::unbind`.
    #[cfg(feature = "engine")]
    pub(crate) custom_element_registry_instance: Option<ObjectId>,
    /// `HTMLIFrameElement.prototype` — tag-specific intermediate
    /// prototype for `<iframe>` wrappers.  Chains to
    /// [`Self::html_element_prototype`] (after PR5b splice) so
    /// `iframe instanceof HTMLElement === true`.
    ///
    /// `None` until `register_html_iframe_prototype()` runs during
    /// `register_globals()` (after `register_html_element_prototype`).
    #[cfg(feature = "engine")]
    pub(crate) html_iframe_prototype: Option<ObjectId>,
    /// `HTMLLabelElement.prototype` (HTML §4.10.4).  Chained to
    /// [`Self::html_element_prototype`].  `None` until
    /// `register_html_label_prototype()` runs.
    #[cfg(feature = "engine")]
    pub(crate) html_label_prototype: Option<ObjectId>,
    /// `HTMLOptGroupElement.prototype` (HTML §4.10.9).
    #[cfg(feature = "engine")]
    pub(crate) html_optgroup_prototype: Option<ObjectId>,
    /// `HTMLLegendElement.prototype` (HTML §4.10.16).
    #[cfg(feature = "engine")]
    pub(crate) html_legend_prototype: Option<ObjectId>,
    /// `HTMLOptionElement.prototype` (HTML §4.10.10).
    #[cfg(feature = "engine")]
    pub(crate) html_option_prototype: Option<ObjectId>,
    /// `HTMLFieldSetElement.prototype` (HTML §4.10.15).
    #[cfg(feature = "engine")]
    pub(crate) html_fieldset_prototype: Option<ObjectId>,
    /// `HTMLFormElement.prototype` (HTML §4.10.3).
    #[cfg(feature = "engine")]
    pub(crate) html_form_prototype: Option<ObjectId>,
    /// `HTMLButtonElement.prototype` (HTML §4.10.6).
    #[cfg(feature = "engine")]
    pub(crate) html_button_prototype: Option<ObjectId>,
    /// `HTMLTextAreaElement.prototype` (HTML §4.10.11).
    #[cfg(feature = "engine")]
    pub(crate) html_textarea_prototype: Option<ObjectId>,
    /// `HTMLSelectElement.prototype` (HTML §4.10.7).
    #[cfg(feature = "engine")]
    pub(crate) html_select_prototype: Option<ObjectId>,
    /// `HTMLInputElement.prototype` (HTML §4.10.5).
    #[cfg(feature = "engine")]
    pub(crate) html_input_prototype: Option<ObjectId>,
    /// `HTMLAnchorElement.prototype` (HTML §4.6.1).  Carries the
    /// HTMLHyperlinkElementUtils mixin (URL accessor 11 IDL attrs
    /// `toString()`), DOMString reflect (`target`, `download`,
    /// `ping`, `hreflang`, `type`), enumerated reflect
    /// (`referrerPolicy`), the `text` accessor, and `relList`.
    /// `None` until `register_html_anchor_prototype()` runs.
    #[cfg(feature = "engine")]
    pub(crate) html_anchor_prototype: Option<ObjectId>,
    /// `HTMLAreaElement.prototype` (HTML §4.6.2).  Same
    /// HTMLHyperlinkElementUtils mixin as anchor plus `alt`,
    /// `coords`, `shape` (enumerated, missing+invalid default
    /// `rect`), `target`, `download`, `ping`, `referrerPolicy`,
    /// and `relList`.
    #[cfg(feature = "engine")]
    pub(crate) html_area_prototype: Option<ObjectId>,
    /// `HTMLImageElement.prototype` (HTML §4.8.4).  Carries
    /// DOMString reflect (`alt`, `src`, `srcset`, `sizes`, `useMap`),
    /// enumerated reflect (`crossOrigin`, `referrerPolicy`,
    /// `decoding`, `loading`, `fetchpriority`), boolean reflect
    /// (`isMap`), numeric reflect (`width`, `height`), and stub
    /// accessors `naturalWidth` / `naturalHeight` / `complete` /
    /// `decode()` (paint pipeline deferred).
    #[cfg(feature = "engine")]
    pub(crate) html_image_prototype: Option<ObjectId>,
    /// `HTMLScriptElement.prototype` (HTML §4.12.1).  Carries
    /// DOMString reflect (`src`, `type`, `integrity`), enumerated
    /// reflect (`crossOrigin`, `referrerPolicy`, `fetchpriority`),
    /// boolean reflect (`async`, `defer`, `noModule`), and the
    /// `text` accessor (textContent alias).
    #[cfg(feature = "engine")]
    pub(crate) html_script_prototype: Option<ObjectId>,
    /// `HTMLLinkElement.prototype` (HTML §4.6.7).  Carries
    /// DOMString reflect (`href`, `media`, `hreflang`, `type`,
    /// `integrity`, `imageSrcset`, `imageSizes`, `as`), enumerated
    /// reflect (`crossOrigin`, `referrerPolicy`, `fetchpriority`),
    /// boolean reflect (`disabled`), the `relList` and `sizes`
    /// DOMTokenLists, and a `sheet` stub (`null`, pending defer slot
    /// `#11-link-stylesheet-loading`).
    #[cfg(feature = "engine")]
    pub(crate) html_link_prototype: Option<ObjectId>,
    /// `HTMLCanvasElement.prototype` (HTML §4.12.5).  Carries
    /// `getContext('2d')` + `width` / `height` numeric reflect.
    /// Looked up per canvas-wrapper creation, so rooted in
    /// `gc::collect` like the other element prototypes.
    #[cfg(feature = "engine")]
    pub(crate) html_canvas_prototype: Option<ObjectId>,
    /// `CanvasRenderingContext2D.prototype` (HTML §4.12.5.1).
    /// Read on every `getContext` to seed the context wrapper's
    /// prototype, so rooted in `gc::collect`.
    #[cfg(feature = "engine")]
    pub(crate) canvas_rendering_context_2d_prototype: Option<ObjectId>,
    /// `ImageData.prototype` (HTML §4.12.5.1.16).  Read on every
    /// `getImageData` / `createImageData` / `new ImageData`, so
    /// rooted in `gc::collect`.
    #[cfg(feature = "engine")]
    pub(crate) image_data_prototype: Option<ObjectId>,
    /// `OffscreenCanvas.prototype` (HTML §4.12.5.3).  Chains
    /// `EventTarget.prototype` (OC is an EventTarget but not a Node).
    /// Looked up per `new OffscreenCanvas(w, h)` and per
    /// `transferControlToOffscreen` (host wraps the spawned OC entity),
    /// so rooted in `gc::collect`.
    #[cfg(feature = "engine")]
    pub(crate) offscreen_canvas_prototype: Option<ObjectId>,
    /// `OffscreenCanvasRenderingContext2D.prototype` (HTML §4.12.5.3.1,
    /// same surface as §4.12.5.1).  Read on every `oc.getContext('2d')`
    /// to seed the context wrapper's prototype, so rooted in
    /// `gc::collect`.
    #[cfg(feature = "engine")]
    pub(crate) offscreen_canvas_rendering_context_2d_prototype: Option<ObjectId>,
    // -----------------------------------------------------------------
    // T2b passive head + grouping prototypes (slot
    // `#11-tags-T2b-passive`).  All chain to `HTMLElement.prototype`.
    // 7 head + 17 grouping = 24 prototypes; HTMLHeading is shared
    // across h1-h6 and HTMLQuote is shared across blockquote+q so the
    // dispatch chain has more arms than this field set.
    // -----------------------------------------------------------------
    /// `HTMLHtmlElement.prototype` (HTML §4.1.1).  Brand-only —
    /// deprecated `version` attribute is not surfaced.
    #[cfg(feature = "engine")]
    pub(crate) html_html_prototype: Option<ObjectId>,
    /// `HTMLHeadElement.prototype` (HTML §4.2.1).  Brand-only.
    #[cfg(feature = "engine")]
    pub(crate) html_head_prototype: Option<ObjectId>,
    /// `HTMLBodyElement.prototype` (HTML §4.3.1).  Brand-only — the
    /// 16 event-handler IDL attributes (`onload` / `onbeforeunload`
    /// / etc.) are deferred to slot `#11-tags-T2b-body-events`
    /// pending the generic EventHandlerAttribute infrastructure
    /// (paired with D-10 `#11-events-misc`).
    #[cfg(feature = "engine")]
    pub(crate) html_body_prototype: Option<ObjectId>,
    /// `HTMLTitleElement.prototype` (HTML §4.2.2).  Carries the
    /// `text` accessor (textContent alias).
    #[cfg(feature = "engine")]
    pub(crate) html_title_prototype: Option<ObjectId>,
    /// `HTMLBaseElement.prototype` (HTML §4.2.3).  Carries `href`
    /// (URL-resolved-fallback-to-raw via
    /// [`elidex_dom_api::element::href_accessor::href_value_or_raw`])
    /// and `target` (string reflect).  `<base href>` propagation
    /// into anchor / area / img / link / script base resolution is
    /// deferred to slot `#11-base-href-resolution` (re-noted from
    /// T2a).
    #[cfg(feature = "engine")]
    pub(crate) html_base_prototype: Option<ObjectId>,
    /// `HTMLMetaElement.prototype` (HTML §4.2.5).  Six string
    /// reflects: `name` / `httpEquiv` / `content` / `charset` /
    /// `media` / `scheme` (deprecated but reflected for legacy
    /// scripts that read `<meta scheme>`).
    #[cfg(feature = "engine")]
    pub(crate) html_meta_prototype: Option<ObjectId>,
    /// `HTMLStyleElement.prototype` (HTML §4.2.6).  Carries `media`
    /// and `type` (string reflect) plus `sheet` (`[SameObject]`
    /// CSSStyleSheet wrapper via PR-B's seam kind
    /// `WrapperKind::StyleSheet`).  `disabled` is folded
    /// into the existing slot `#11-stylesheet-disabled` (cross-crate
    /// cascade integration shared with `CSSStyleSheet.disabled`).
    #[cfg(feature = "engine")]
    pub(crate) html_style_prototype: Option<ObjectId>,
    /// `HTMLDivElement.prototype` (HTML §4.4.16).  Brand-only;
    /// deprecated `align` deferred to slot
    /// `#11-tags-deprecated-attr-sweep`.
    #[cfg(feature = "engine")]
    pub(crate) html_div_prototype: Option<ObjectId>,
    /// `HTMLSpanElement.prototype` (HTML §4.5.26).  Brand-only.
    #[cfg(feature = "engine")]
    pub(crate) html_span_prototype: Option<ObjectId>,
    /// `HTMLBRElement.prototype` (HTML §4.5.27).  Brand-only;
    /// deprecated `clear` deferred.
    #[cfg(feature = "engine")]
    pub(crate) html_br_prototype: Option<ObjectId>,
    /// `HTMLHRElement.prototype` (HTML §4.4.2).  Brand-only;
    /// 5 deprecated attrs deferred.
    #[cfg(feature = "engine")]
    pub(crate) html_hr_prototype: Option<ObjectId>,
    /// `HTMLPreElement.prototype` (HTML §4.4.3).  Brand-only;
    /// deprecated `width` deferred.
    #[cfg(feature = "engine")]
    pub(crate) html_pre_prototype: Option<ObjectId>,
    /// `HTMLParagraphElement.prototype` (HTML §4.4.1).  Brand-only;
    /// deprecated `align` deferred.
    #[cfg(feature = "engine")]
    pub(crate) html_p_prototype: Option<ObjectId>,
    /// `HTMLHeadingElement.prototype` (HTML §4.3.6).  Shared across
    /// h1-h6.  Brand-only.
    #[cfg(feature = "engine")]
    pub(crate) html_heading_prototype: Option<ObjectId>,
    /// `HTMLQuoteElement.prototype` (HTML §4.5.4 / §4.5.5).  Shared
    /// across `<blockquote>` and `<q>`.  Carries `cite` (string
    /// reflect, plain DOMString IDL).
    #[cfg(feature = "engine")]
    pub(crate) html_quote_prototype: Option<ObjectId>,
    /// `HTMLOListElement.prototype` (HTML §4.4.5).  Carries
    /// `reversed` (boolean), `start` (long, default 1), and `type`
    /// (DOMString limited-to-only-known-values: `1` / `a` / `A` /
    /// `i` / `I`, case-sensitive match per spec).  Deprecated
    /// `compact` deferred.
    #[cfg(feature = "engine")]
    pub(crate) html_olist_prototype: Option<ObjectId>,
    /// `HTMLUListElement.prototype` (HTML §4.4.6).  Brand-only;
    /// deprecated `compact` / `type` deferred.
    #[cfg(feature = "engine")]
    pub(crate) html_ulist_prototype: Option<ObjectId>,
    /// `HTMLLIElement.prototype` (HTML §4.4.8).  Carries `value`
    /// (long, default 0).  Deprecated `type` deferred.
    #[cfg(feature = "engine")]
    pub(crate) html_li_prototype: Option<ObjectId>,
    /// `HTMLDListElement.prototype` (HTML §4.4.9).  Brand-only;
    /// deprecated `compact` deferred.
    #[cfg(feature = "engine")]
    pub(crate) html_dlist_prototype: Option<ObjectId>,
    /// `HTMLMenuElement.prototype` (HTML §4.4.7).  Brand-only.
    #[cfg(feature = "engine")]
    pub(crate) html_menu_prototype: Option<ObjectId>,
    /// `HTMLMapElement.prototype` (HTML §4.8.13).  Carries `name`
    /// (string reflect) and `areas` (live `HTMLCollection` of
    /// descendant `<area>` elements via
    /// [`elidex_dom_api::live_collection::LiveCollection`]).
    #[cfg(feature = "engine")]
    pub(crate) html_map_prototype: Option<ObjectId>,
    /// `HTMLPictureElement.prototype` (HTML §4.8.1).  Brand-only.
    #[cfg(feature = "engine")]
    pub(crate) html_picture_prototype: Option<ObjectId>,
    /// `HTMLDataElement.prototype` (HTML §4.5.13).  Carries `value`
    /// (string reflect, attr `value`).
    #[cfg(feature = "engine")]
    pub(crate) html_data_prototype: Option<ObjectId>,
    /// `HTMLTimeElement.prototype` (HTML §4.5.14).  Carries
    /// `dateTime` (string reflect, attr `datetime`).
    #[cfg(feature = "engine")]
    pub(crate) html_time_prototype: Option<ObjectId>,
    /// `HTMLTableElement.prototype` (HTML §4.9.1).  Carries
    /// `caption` / `tHead` / `tFoot` getter+setter pairs +
    /// `tBodies` / `rows` `[SameObject]` HTMLCollections +
    /// `createTHead` / `createTFoot` / `createCaption` /
    /// `createTBody` / `deleteTHead` / `deleteTFoot` /
    /// `deleteCaption` / `insertRow` / `deleteRow`.
    /// Slot `#11-tags-T2c-table`.
    #[cfg(feature = "engine")]
    pub(crate) html_table_prototype: Option<ObjectId>,
    /// `HTMLTableSectionElement.prototype` (HTML §4.9.5-7) — shared
    /// across `<thead>` / `<tbody>` / `<tfoot>`.  Carries `rows`
    /// `[SameObject]` HTMLCollection of direct `<tr>` children +
    /// `insertRow` / `deleteRow`.
    #[cfg(feature = "engine")]
    pub(crate) html_table_section_prototype: Option<ObjectId>,
    /// `HTMLTableRowElement.prototype` (HTML §4.9.8).  Carries
    /// `rowIndex` / `sectionRowIndex` + `cells` `[SameObject]`
    /// HTMLCollection of direct `<td>`/`<th>` children +
    /// `insertCell` / `deleteCell`.
    #[cfg(feature = "engine")]
    pub(crate) html_table_row_prototype: Option<ObjectId>,
    /// `HTMLTableCellElement.prototype` (HTML §4.9.9-10) — shared
    /// across `<td>` / `<th>`.  Carries `cellIndex` + `colSpan` /
    /// `rowSpan` (clamped long IDL) + `headers` / `abbr` (string
    /// reflect) + `scope` (enumerated reflect).
    #[cfg(feature = "engine")]
    pub(crate) html_table_cell_prototype: Option<ObjectId>,
    /// `HTMLTableCaptionElement.prototype` (HTML §4.9.2).
    /// Brand-only; deprecated `align` deferred.
    #[cfg(feature = "engine")]
    pub(crate) html_table_caption_prototype: Option<ObjectId>,
    /// `HTMLTableColElement.prototype` (HTML §4.9.4) — shared across
    /// `<col>` / `<colgroup>`.  Carries `span` (long, default 1,
    /// clamped 1..=1000).  Deprecated `align`/`vAlign`/`width`/`ch`/`chOff`
    /// deferred.
    #[cfg(feature = "engine")]
    pub(crate) html_table_col_prototype: Option<ObjectId>,
    /// `HTMLDialogElement.prototype` (HTML §4.11.4).  Carries
    /// `open` (boolean reflect) + `returnValue` (state via
    /// [`elidex_ecs::DialogReturnValue`]) + `show()` / `showModal()`
    /// (sets [`elidex_ecs::IsModalDialog`] marker) /
    /// `close(returnValue?)` (clears state, fires `close` event).
    /// Slot `#11-tags-T2d-interactive`.
    #[cfg(feature = "engine")]
    pub(crate) html_dialog_prototype: Option<ObjectId>,
    /// `HTMLDetailsElement.prototype` (HTML §4.11.1).  Carries
    /// `open` (boolean reflect) + `name` (string reflect).  ToggleEvent
    /// fire on open change is deferred to slot
    /// `#11-tags-T2d-details-toggle-event`.
    #[cfg(feature = "engine")]
    pub(crate) html_details_prototype: Option<ObjectId>,
    /// `HTMLTemplateElement.prototype` (HTML §4.12.3).  Carries the
    /// `[SameObject]` `content` DocumentFragment accessor.  Lazy
    /// allocation interned under `WrapperKind::TemplateContent`.
    #[cfg(feature = "engine")]
    pub(crate) html_template_prototype: Option<ObjectId>,
    /// `HTMLDataListElement.prototype` (HTML §4.10.10).  Carries the
    /// `[SameObject]` `options` HTMLCollection of descendant
    /// `<option>` elements.
    #[cfg(feature = "engine")]
    pub(crate) html_datalist_prototype: Option<ObjectId>,
    /// `HTMLOutputElement.prototype` (HTML §4.10.13).  Carries
    /// `htmlFor` (`[SameObject, PutForwards=value]` DOMTokenList) +
    /// `form` / `name` / `type` / `defaultValue` / `value` (state
    /// machine via [`elidex_ecs::OutputDefaultValue`] +
    /// [`elidex_ecs::OutputValueOverride`]) / `labels` stub +
    /// ConstraintValidation mixin.
    #[cfg(feature = "engine")]
    pub(crate) html_output_prototype: Option<ObjectId>,
    /// `HTMLProgressElement.prototype` (HTML §4.10.14).  Carries
    /// `value` / `max` (double IDL with clamping) + `position`
    /// (computed: -1 if indeterminate else clamp(value,0,max)/max) +
    /// `labels` stub.
    #[cfg(feature = "engine")]
    pub(crate) html_progress_prototype: Option<ObjectId>,
    /// `HTMLMeterElement.prototype` (HTML §4.10.15).  Carries
    /// `value` / `min` / `max` / `low` / `high` / `optimum` (all
    /// double IDL) + `labels` stub.
    #[cfg(feature = "engine")]
    pub(crate) html_meter_prototype: Option<ObjectId>,
    /// `HTMLFormControlsCollection.prototype` (HTML §4.10.18.4) —
    /// reserved-not-yet-registered slot.  When the
    /// `#11-tags-radionodelist` defer slot lands, this will hold a
    /// prototype chained to `HTMLCollection.prototype` with a
    /// `namedItem` override returning `RadioNodeList`.  Currently
    /// `alloc_collection` always falls through to the plain
    /// `HTMLCollection.prototype` — this `Option<ObjectId>` is a
    /// pre-allocated slot to avoid renumbering `proto_roots[]`
    /// when the proper register fn lands.
    #[cfg(feature = "engine")]
    pub(crate) html_form_controls_collection_prototype: Option<ObjectId>,
    /// `HTMLOptionsCollection.prototype` (HTML §4.10.10.2) —
    /// reserved-not-yet-registered slot.  Will host the mutable
    /// surface (`length` setter / `add` / `remove` / `selectedIndex`)
    /// when the `#11-options-collection-mutable-surface` defer slot
    /// lands.  Same pre-allocation rationale as
    /// [`Self::html_form_controls_collection_prototype`].
    #[cfg(feature = "engine")]
    pub(crate) html_options_collection_prototype: Option<ObjectId>,
    /// `ValidityState.prototype` (HTML §4.10.20.3).  Plain Object
    /// prototype with 11 boolean accessor methods that read from
    /// `elidex_form::validation::validate_control` directly.
    #[cfg(feature = "engine")]
    pub(crate) validity_state_prototype: Option<ObjectId>,
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
    /// `DOMRectReadOnly.prototype` (W3C Geometry Interfaces Module
    /// Level 1 §3).  Chains to `Object.prototype`.  Holds the
    /// getter-only `x` / `y` / `width` / `height` accessors plus the
    /// computed `top` / `right` / `bottom` / `left` getters and the
    /// `toJSON` method, all reading [`Self::dom_rect_states`] via
    /// receiver brand-check.  `None` until `register_dom_rect_globals()`
    /// runs during `register_globals()` (after `object_prototype`).
    #[cfg(feature = "engine")]
    pub(crate) dom_rect_readonly_prototype: Option<ObjectId>,
    /// `DOMRect.prototype` (W3C Geometry Interfaces Module Level 1 §3).
    /// Chains to [`Self::dom_rect_readonly_prototype`] (DOMRect is a
    /// DOMRectReadOnly subclass), re-declaring `x` / `y` / `width` /
    /// `height` as read-write accessor pairs; `top`/`right`/`bottom`/
    /// `left`/`toJSON` are inherited from the base prototype.
    #[cfg(feature = "engine")]
    pub(crate) dom_rect_prototype: Option<ObjectId>,
    /// Per-`DOMRectReadOnly` / `DOMRect` out-of-band state, keyed by the
    /// instance's own `ObjectId` (same value-type pattern as
    /// [`Self::dom_exception_states`]).  `mutable` distinguishes the
    /// DOMRect brand (read-write) from DOMRectReadOnly (read-only): the
    /// `x`/`y`/`width`/`height` setters require `mutable == true`, so a
    /// cross-called setter on a DOMRectReadOnly receiver throws.
    ///
    /// GC contract: the payload is `Copy` (4×`f64` + `bool`) with no
    /// `ObjectId` fan-out, so no trace pass is needed; the sweep tail
    /// (`collect_garbage`) prunes entries whose key was collected,
    /// matching `dom_exception_states` / `abort_signal_states`.
    #[cfg(feature = "engine")]
    pub(crate) dom_rect_states: HashMap<ObjectId, host::dom_rect::DomRectState>,
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
    /// `WorkerGlobalScope.prototype` — the worker analog of
    /// [`window_prototype`](Self::window_prototype), chained to
    /// `EventTarget.prototype`. `None` in a Window VM; set by
    /// `register_worker_global_scope_prototype()` during the worker fork of
    /// `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) worker_scope_prototype: Option<ObjectId>,
    /// `Worker.prototype` — the main-side `Worker` object's prototype
    /// (WHATWG HTML §10.2.6), chained to `EventTarget.prototype`. `None` in a
    /// worker VM; set by `register_worker_global()` during the Window fork of
    /// `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) worker_prototype: Option<ObjectId>,
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
    /// `AnimationEvent.prototype` (CSS Animations Level 1 §4.2).
    /// Chains to [`event_prototype`].  Adds `animationName` /
    /// `elapsedTime` / `pseudoElement` own-data slots (reuses the
    /// UA-dispatch `animation` shape).
    #[cfg(feature = "engine")]
    pub(crate) animation_event_prototype: Option<ObjectId>,
    /// `TransitionEvent.prototype` (CSS Transitions Level 1 §6).
    /// Chains to [`event_prototype`].  Adds `propertyName` /
    /// `elapsedTime` / `pseudoElement` own-data slots (reuses the
    /// UA-dispatch `transition` shape).
    #[cfg(feature = "engine")]
    pub(crate) transition_event_prototype: Option<ObjectId>,
    /// `CloseEvent.prototype` (WHATWG HTML §10.4 — paired with
    /// WebSocket / EventSource close).  Chains to
    /// [`event_prototype`].  Adds `code` / `reason` / `wasClean`
    /// own-data slots (reuses the UA-dispatch `close_event` shape).
    #[cfg(feature = "engine")]
    pub(crate) close_event_prototype: Option<ObjectId>,
    /// `SubmitEvent.prototype` (HTML §4.10.21.5.5).  Chains to
    /// [`event_prototype`].  Adds `submitter` own-data slot.
    #[cfg(feature = "engine")]
    pub(crate) submit_event_prototype: Option<ObjectId>,
    /// `FormDataEvent.prototype` (HTML §4.10.21.5.4).  Chains to
    /// [`event_prototype`].  Adds `formData` own-data slot.
    #[cfg(feature = "engine")]
    pub(crate) formdata_event_prototype: Option<ObjectId>,
    /// `ToggleEvent.prototype` (HTML §4.11.1.5).  Chains to
    /// [`event_prototype`].  Adds `newState` / `oldState` own-data slots.
    #[cfg(feature = "engine")]
    pub(crate) toggle_event_prototype: Option<ObjectId>,
    /// `CompositionEvent.prototype` (UI Events §5.6).  Chains to
    /// [`ui_event_prototype`].  Adds `data` own-data slot beyond the
    /// UIEvent base.
    #[cfg(feature = "engine")]
    pub(crate) composition_event_prototype: Option<ObjectId>,
    /// `ClipboardEvent.prototype` (Clipboard API §3).  Chains to
    /// [`event_prototype`].  Adds `clipboardData` own-data slot.
    #[cfg(feature = "engine")]
    pub(crate) clipboard_event_prototype: Option<ObjectId>,
    /// `ProgressEvent.prototype` (XHR §10).  Chains to
    /// [`event_prototype`].  Adds `lengthComputable` / `loaded` / `total`
    /// own-data slots.
    #[cfg(feature = "engine")]
    pub(crate) progress_event_prototype: Option<ObjectId>,
    /// `BeforeUnloadEvent.prototype` (HTML §9.10.2).  Chains to
    /// [`event_prototype`].  No public constructor — `new
    /// BeforeUnloadEvent(...)` throws TypeError "Illegal constructor".
    /// `returnValue` is a mutable accessor pair installed on the
    /// prototype (legacy spec — script reads/writes it inside an
    /// `onbeforeunload` handler).  Still registered so UA-dispatched
    /// instances pass `instanceof BeforeUnloadEvent`.
    #[cfg(feature = "engine")]
    pub(crate) before_unload_event_prototype: Option<ObjectId>,
    /// Per-`BeforeUnloadEvent` instance `returnValue` slot, keyed by
    /// the instance's `ObjectId`.  Lazy: only present after a setter
    /// invocation; the getter returns the empty string for missing
    /// entries.  GC contract: sweep tail prunes entries whose key
    /// `ObjectId` was collected so a recycled slot can't observe a
    /// stale string.
    #[cfg(feature = "engine")]
    pub(crate) before_unload_return_values: HashMap<ObjectId, StringId>,
    /// `MessageEvent.prototype` (HTML §9.4.4).  Chains to
    /// [`event_prototype`].  Adds `data` / `origin` / `lastEventId` /
    /// `source` / `ports` own-data slots (reuses the UA-dispatch
    /// `message` shape).
    #[cfg(feature = "engine")]
    pub(crate) message_event_prototype: Option<ObjectId>,
    /// `WheelEvent.prototype` (UI Events §5.5).  Chains to
    /// [`mouse_event_prototype`].  Adds `deltaX` / `deltaY` / `deltaZ` /
    /// `deltaMode` own-data slots beyond the MouseEvent base, plus
    /// DOM_DELTA_* constants installed as static fields on the prototype.
    #[cfg(feature = "engine")]
    pub(crate) wheel_event_prototype: Option<ObjectId>,
    /// `PageTransitionEvent.prototype` (HTML §7.10.1.7.4).  Chains to
    /// [`event_prototype`].  Adds `persisted` own-data slot (reuses the
    /// UA-dispatch `page_transition` shape).
    #[cfg(feature = "engine")]
    pub(crate) page_transition_event_prototype: Option<ObjectId>,
    // -- D-9 events-modern-input (slot #11-events-modern-input) --
    /// `PointerEvent.prototype` (UI Events Pointer §6).  Chains to
    /// [`mouse_event_prototype`].  Adds 12 own-data slots (pointerId
    /// / width / height / pressure / tangentialPressure / tiltX /
    /// tiltY / twist / altitudeAngle / azimuthAngle / pointerType /
    /// isPrimary), plus `getCoalescedEvents()` / `getPredictedEvents()`
    /// stub methods that return fresh empty Arrays per call.
    #[cfg(feature = "engine")]
    pub(crate) pointer_event_prototype: Option<ObjectId>,
    /// `DragEvent.prototype` (HTML DnD §6.4).  Chains to
    /// [`mouse_event_prototype`].  Adds `dataTransfer` own-data slot.
    #[cfg(feature = "engine")]
    pub(crate) drag_event_prototype: Option<ObjectId>,
    /// `TouchEvent.prototype` (Touch Events §5.5).  Chains to
    /// [`ui_event_prototype`].  Adds 7 own-data slots (3 TouchLists +
    /// 4 modifier flags).
    #[cfg(feature = "engine")]
    pub(crate) touch_event_prototype: Option<ObjectId>,
    /// `Touch.prototype` (Touch Events §5.6).  Chains to
    /// `Object.prototype`.  All accessors read from
    /// [`Self::touch_states`].
    #[cfg(feature = "engine")]
    pub(crate) touch_prototype: Option<ObjectId>,
    /// `TouchList.prototype` (Touch Events §5.6).  Chains to
    /// `Object.prototype`.  Length getter + indexed exotic +
    /// `item(idx)` method backed by [`Self::touch_list_states`].
    #[cfg(feature = "engine")]
    pub(crate) touch_list_prototype: Option<ObjectId>,
    /// `DataTransfer.prototype` (HTML DnD §6.2).  Chains to
    /// `Object.prototype`.  Holds the 4 enum-string accessors
    /// (dropEffect / effectAllowed) + 3 `[SameObject]` accessors
    /// (items / files / types) + 4 mutator methods (getData / setData
    /// / clearData / setDragImage).  Mutable instance state in
    /// [`Self::data_transfer_states`].
    #[cfg(feature = "engine")]
    pub(crate) data_transfer_prototype: Option<ObjectId>,
    /// `DataTransferItem.prototype` (HTML DnD §6.3).  Chains to
    /// `Object.prototype`.  Holds the `kind` / `type` accessors +
    /// `getAsString(cb)` / `getAsFile()` methods.
    #[cfg(feature = "engine")]
    pub(crate) data_transfer_item_prototype: Option<ObjectId>,
    /// `DataTransferItemList.prototype` (HTML DnD §6.3).  Chains to
    /// `Object.prototype`.  Holds the `length` accessor +
    /// `add(...)` / `remove(idx)` / `clear()` methods, plus an
    /// indexed exotic `[[GetOwnProperty]]` for `list[i]`.
    #[cfg(feature = "engine")]
    pub(crate) data_transfer_item_list_prototype: Option<ObjectId>,
    /// Per-`DataTransfer` mutable state (HTML DnD §6.2).  Keyed by
    /// the instance's `ObjectId`.  Holds the dropEffect / effectAllowed
    /// enum values (as `u8` indices), the ordered entry list, the
    /// `[SameObject]` wrapper caches for `items` / `files`, and the
    /// optional drag-image (entity_bits + x/y offsets).
    ///
    /// GC contract: the trace step fans out via the wrappers and
    /// any blob `ObjectId`s on file entries.  `Vm::unbind` clears
    /// the map because `drag_image_entity` is cross-DOM.  Sweep
    /// tail prunes entries whose key was collected.
    #[cfg(feature = "engine")]
    pub(crate) data_transfer_states: HashMap<ObjectId, host::events_modern::DataTransferState>,
    /// Per-`Touch` instance state (Touch Events §5.6).  Keyed by the
    /// instance's `ObjectId`.  All 12 IDL members live here as
    /// `f64` + `Option<ObjectId>` (the EventTarget `target`).
    ///
    /// GC contract: trace marks the state entry's `target`
    /// `ObjectId`.  Sweep tail prunes entries whose key was
    /// collected.
    #[cfg(feature = "engine")]
    pub(crate) touch_states: HashMap<ObjectId, host::events_modern::TouchState>,
    /// Per-`TouchList` instance state (Touch Events §5.6).  Keyed
    /// by the instance's `ObjectId`.  Holds the ordered Vec of
    /// member `Touch` ObjectIds.
    ///
    /// GC contract: trace marks every Touch ObjectId in the state
    /// entry's `items` Vec.  Sweep tail prunes entries whose key
    /// was collected.
    #[cfg(feature = "engine")]
    pub(crate) touch_list_states: HashMap<ObjectId, host::events_modern::TouchListState>,
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
    /// "Body stream is disturbed" flag (WHATWG Fetch §5 + Streams §4.2
    /// — spec slot is named `[[disturbed]]`; the JS-visible `bodyUsed`
    /// IDL getter reads membership of this set).  Inserted by the
    /// Body mixin read methods (`text()` / `.json()` /
    /// `.arrayBuffer()` / `.blob()`) the first time any one of them
    /// runs on a given Request / Response, AND by the
    /// `Response.body` / `Request.body` getter the first time it
    /// materialises the lazy stream (Phase-2 simplification —
    /// spec defers this until an actual chunk is read; M4-13
    /// spec-polish moves the flag to the reader-read path).  A
    /// second consumer then rejects with `TypeError`.
    ///
    /// `locked` is **not** a separate slot — it is derived from
    /// "the body's stream has a reader attached", which the
    /// `Response.body` / `Request.body` accessor exposes via
    /// `readable_stream_states[stream_id].reader_id.is_some()`.
    /// Keeping `disturbed` and `locked` distinct here matches
    /// WHATWG Fetch §5 (clone() throws on `disturbed || locked`).
    ///
    /// GC contract: sweep tail prunes entries whose key was
    /// collected, same as the other side tables.
    #[cfg(feature = "engine")]
    pub(crate) disturbed: HashSet<ObjectId>,
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
    /// `File.prototype` (File API §4).  Chains through
    /// `Blob.prototype → Object.prototype`.  `None` until
    /// `register_file_global()` runs during `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) file_prototype: Option<ObjectId>,
    /// `FileList.prototype` (File API §5).  Chains directly to
    /// `Object.prototype`.
    #[cfg(feature = "engine")]
    pub(crate) file_list_prototype: Option<ObjectId>,
    /// `FileReader.prototype` (File API §6).  Chains directly to
    /// `EventTarget.prototype` (FileReader is an EventTarget per
    /// FileAPI §6.2 IDL).
    #[cfg(feature = "engine")]
    pub(crate) file_reader_prototype: Option<ObjectId>,
    /// Per-`File` out-of-band state, keyed by the File instance's
    /// `ObjectId`.  Holds the link to the backing Blob wrapper
    /// (`blob_id`), the file `name`, and `lastModified` epoch ms.
    /// See [`host::file::FileSideData`].
    ///
    /// GC contract: the trace step marks `blob_id` so the backing
    /// Blob survives as long as the File is reachable.  Sweep tail
    /// prunes entries whose key `ObjectId` was collected.
    #[cfg(feature = "engine")]
    pub(crate) file_data: HashMap<ObjectId, host::file::FileSideData>,
    /// Per-`FileList` out-of-band state, keyed by the FileList
    /// instance's `ObjectId`.  Holds the ordered list of File
    /// wrapper `ObjectId`s.  See [`host::file_list::FileListSideData`].
    ///
    /// GC contract: the trace step marks every File ObjectId in
    /// `file_ids`.  Sweep tail prunes entries whose key was
    /// collected.
    #[cfg(feature = "engine")]
    pub(crate) file_list_data: HashMap<ObjectId, host::file_list::FileListSideData>,
    /// Per-`FileReader` out-of-band state, keyed by the FileReader
    /// instance's `ObjectId`.  Holds the readyState machine,
    /// result, error, target blob reference, and the abort sequence
    /// counter used to invalidate stale read tasks.  See
    /// [`host::file_reader::FileReaderSideData`].
    ///
    /// GC contract: the trace step marks `target_blob` (if any) and
    /// any ObjectId stored in `result` (ArrayBuffer for
    /// readAsArrayBuffer / error wrapper).  Sweep tail prunes
    /// entries whose key was collected.
    #[cfg(feature = "engine")]
    pub(crate) file_reader_data: HashMap<ObjectId, host::file_reader::FileReaderSideData>,
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
    /// `URLSearchParams.prototype` (WHATWG URL §6).  Chains to
    /// `Object.prototype`.  `None` until
    /// `register_url_search_params_global()` runs during
    /// `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) url_search_params_prototype: Option<ObjectId>,
    /// Per-`URLSearchParams` entry list keyed by the instance's own
    /// `ObjectId` (WHATWG URL §6 "list of name-value pairs").  Names
    /// and values are stored as interned `StringId`s in insertion
    /// order — `Vec<(StringId, StringId)>`.
    ///
    /// GC contract: the entry list holds only `StringId`s
    /// (pool-permanent), so the trace step does nothing.  Sweep
    /// tail prunes entries whose key `ObjectId` was collected —
    /// same pattern as `headers_states`.
    #[cfg(feature = "engine")]
    pub(crate) url_search_params_states: HashMap<ObjectId, Vec<(StringId, StringId)>>,
    /// `URL.prototype` (WHATWG URL §6.1).  Chains to
    /// `Object.prototype`.  `None` until `register_url_global()`
    /// runs during `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) url_prototype: Option<ObjectId>,
    /// Per-`URL` instance state keyed by the instance's own
    /// `ObjectId` (WHATWG URL §6.1).  Holds the parsed [`url::Url`]
    /// + the linked `URLSearchParams` `ObjectId` allocated by the
    ///   constructor (eager-create for `searchParams` identity
    ///   stability — `url.searchParams === url.searchParams` is a
    ///   spec invariant).
    ///
    /// GC contract: the trace step marks the linked `search_params`
    /// `ObjectId` if any.  Sweep tail prunes entries whose key
    /// `ObjectId` was collected — same pattern as
    /// `url_search_params_states`.
    #[cfg(feature = "engine")]
    pub(crate) url_states: HashMap<ObjectId, host::url::UrlState>,
    /// Reverse linkage `URLSearchParams ObjectId → owning URL
    /// ObjectId` (WHATWG URL §6.1 "URL → searchParams" back-edge).
    /// Populated when the URL constructor allocates a fresh
    /// `URLSearchParams` for the `searchParams` IDL attribute;
    /// empty for standalone `URLSearchParams` instances.  The USP
    /// mutator natives (`append` / `delete` / `set` / `sort`) consult
    /// this map at their tail to write the serialised entry list
    /// back into the URL's query.
    ///
    /// GC contract: the trace step marks the URL value when the
    /// keyed `URLSearchParams` is reachable so a script holding only
    /// the `searchParams` reference still keeps its parent URL alive
    /// (the symmetric arm).  Sweep tail prunes entries whose key
    /// `ObjectId` was collected.
    #[cfg(feature = "engine")]
    pub(crate) usp_parent_url: HashMap<ObjectId, ObjectId>,
    /// `FormData.prototype` (WHATWG XHR §4.3).  Chains to
    /// `Object.prototype`.  `None` until
    /// `register_form_data_global()` runs during
    /// `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) form_data_prototype: Option<ObjectId>,
    /// Per-`FormData` entry list keyed by the instance's own
    /// `ObjectId` (WHATWG XHR §4.3 "entry list").  Each entry
    /// carries a name + value (`String` or `Blob`-backed) +
    /// optional filename.
    ///
    /// GC contract: the trace step marks every Blob `ObjectId`
    /// referenced by [`host::form_data::FormDataValue::Blob`] so
    /// Blobs appended to a FormData survive as long as the
    /// FormData is reachable.  String entries hold only interned
    /// `StringId`s.  Sweep tail prunes entries whose key
    /// `ObjectId` was collected.
    #[cfg(feature = "engine")]
    pub(crate) form_data_states: HashMap<ObjectId, Vec<host::form_data::FormDataEntry>>,
    /// `ReadableStream.prototype` (WHATWG Streams §4.2).  Chains
    /// to `Object.prototype`.  `None` until
    /// `register_readable_stream_global()` runs during
    /// `register_globals()`.
    #[cfg(feature = "engine")]
    pub(crate) readable_stream_prototype: Option<ObjectId>,
    /// `ReadableStreamDefaultReader.prototype` (WHATWG Streams §4.3).
    /// Chains to `Object.prototype`.
    #[cfg(feature = "engine")]
    pub(crate) readable_stream_default_reader_prototype: Option<ObjectId>,
    /// `ReadableStreamDefaultController.prototype` (WHATWG Streams
    /// §4.5).  Chains to `Object.prototype`.
    #[cfg(feature = "engine")]
    pub(crate) readable_stream_default_controller_prototype: Option<ObjectId>,
    /// Per-`ReadableStream` out-of-band state (WHATWG Streams §4.2).
    /// Keyed by the instance's own `ObjectId`.  Holds the stream
    /// state machine, queue, controller / reader back-refs, source
    /// callbacks, and queuing-strategy size algorithm.
    ///
    /// GC contract: trace step marks queue chunks (`JsValue`s),
    /// source-callback ObjectIds, controller / reader back-refs,
    /// the size algorithm, and the stored error reason.  Sweep
    /// tail prunes entries whose key `ObjectId` was collected so a
    /// recycled slot can't inherit stale state.
    #[cfg(feature = "engine")]
    pub(crate) readable_stream_states:
        HashMap<ObjectId, host::readable_stream::ReadableStreamState>,
    /// Per-`ReadableStreamDefaultReader` out-of-band state
    /// (WHATWG Streams §4.3).  Keyed by the reader instance's own
    /// `ObjectId`.  Owns the FIFO of pending `read()` promises so
    /// the spec `[[readRequests]]` slot is directly modelled — no
    /// VM-level strong-root list.
    ///
    /// GC contract: trace step marks the stream back-ref + every
    /// pending read Promise + the cached `closed` Promise.
    #[cfg(feature = "engine")]
    pub(crate) readable_stream_reader_states: HashMap<ObjectId, host::readable_stream::ReaderState>,
    /// Cached lazy body stream per Request / Response, keyed by
    /// the receiver's `ObjectId`.  Populated on first `.body`
    /// access so subsequent `.body === .body` reads share the
    /// same stream instance — required by WHATWG Fetch §5
    /// (`.body` is an internal slot, not a fresh allocation per
    /// access).
    ///
    /// GC contract: the stored `ObjectId` (a `ReadableStream`) is
    /// rooted whenever the receiver is rooted — the trace path
    /// for `Request` / `Response` marks it.  Sweep tail prunes
    /// entries whose receiver was collected.
    #[cfg(feature = "engine")]
    pub(crate) body_streams: HashMap<ObjectId, ObjectId>,
    /// `CountQueuingStrategy.prototype` (WHATWG Streams §6.1).
    #[cfg(feature = "engine")]
    pub(crate) count_queuing_strategy_prototype: Option<ObjectId>,
    /// `ByteLengthQueuingStrategy.prototype` (WHATWG Streams §6.2).
    #[cfg(feature = "engine")]
    pub(crate) byte_length_queuing_strategy_prototype: Option<ObjectId>,
    /// Backing state for `ObjectKind::HtmlCollection` /
    /// `ObjectKind::NodeList` wrappers (WHATWG DOM §4.2.10 / §4.2.10.1).
    ///
    /// Shared between both collection interfaces because the
    /// underlying [`elidex_dom_api::LiveCollection`] tracks both the
    /// filter (`CollectionFilter`) and the kind (`CollectionKind`)
    /// in one struct. One `HashMap` keeps the GC sweep tail tidy and
    /// lets the indexed / named property lookup in
    /// `ops_property::get_element` hit a single side-table
    /// regardless of the wrapper kind.
    ///
    /// GC contract: the stored `LiveCollection` holds only `Entity`,
    /// owned `String` / `Vec<String>` (filter needles for
    /// `ByTagName` / `ByName` / `ByClassNames`), `Vec<Entity>`
    /// (cached snapshot + querySelectorAll-bound static list), and
    /// `u64` (subtree version) — **no `ObjectId` references** — so
    /// the trace step does nothing. The sweep tail prunes entries
    /// whose key `ObjectId` was collected, same pattern as
    /// `headers_states` / `blob_data`.
    #[cfg(feature = "engine")]
    pub(crate) live_collection_states: HashMap<ObjectId, elidex_dom_api::LiveCollection>,
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
    /// for that signal's `ObjectId`, sends
    /// [`elidex_net::broker::RendererToNetwork::CancelFetch`] for each
    /// recorded fetch so the broker can post an early `Err("aborted")`
    /// reply, and rejects the matching pending Promise via
    /// [`Self::pending_fetches`].
    ///
    /// GC contract: sweep prunes entries whose key (signal) was
    /// collected, matching [`Self::abort_signal_states`] /
    /// [`Self::any_composite_map`].  Entries with live signal keys
    /// are retained as-is; the `FetchId`s inside are plain `u64`s
    /// that carry no GC obligations.
    #[cfg(feature = "engine")]
    pub(crate) fetch_abort_observers: HashMap<ObjectId, Vec<elidex_net::broker::FetchId>>,
    /// In-flight async `fetch()` requests: broker `FetchId` → pending
    /// Promise [`ObjectId`].  Populated when [`host::fetch::native_fetch`]
    /// enqueues a request via [`elidex_net::broker::NetworkHandle::fetch_async`]
    /// and drained when [`Self::tick_network`] sees the matching
    /// `FetchResponse` (broker success / error / synthesised abort).
    /// A late reply for an entry already removed by an earlier abort
    /// fan-out lands here as a `None` and is silently dropped — the
    /// dedupe path that lets [`host::abort::abort_signal`] reject the
    /// Promise synchronously without coordinating with the broker.
    ///
    /// GC contract: values (Promise `ObjectId`s) are **strong roots**
    /// — without them, a Promise whose only reference is the user's
    /// `let p = fetch(url)` (and which they never store anywhere
    /// else) would be collected before the broker reply lands and
    /// the `tick_network` settlement step would target a recycled
    /// slot.  Sweep does not prune by value-mark because the value
    /// is *kept alive* by being a root; entries are removed
    /// explicitly on settlement / abort fan-out.
    #[cfg(feature = "engine")]
    pub(crate) pending_fetches: HashMap<elidex_net::broker::FetchId, ObjectId>,
    /// Per-`FetchId` CORS metadata captured at dispatch time so
    /// the `tick_network` settlement step can run the response-
    /// type classifier ([`host::cors::classify_response_type`]).
    /// Holds the request URL / origin / mode + redirect mode so
    /// the classifier doesn't depend on threading those values
    /// through the broker (which is intentionally CORS-blind).
    /// Drained on settlement / abort / handle-swap reject —
    /// same lifecycle as `pending_fetches`.
    #[cfg(feature = "engine")]
    pub(crate) pending_fetch_cors: HashMap<elidex_net::broker::FetchId, host::cors::FetchCorsMeta>,
    /// Reverse index for `FetchId → AbortSignal ObjectId` so the
    /// `tick_network` reply handler can prune
    /// [`Self::fetch_abort_observers`]`[signal_id]` in O(1) without
    /// scanning every signal's observer list.  Populated alongside
    /// [`Self::pending_fetches`] when the originating `fetch()` call
    /// carried an `init.signal`; absent when no signal was supplied.
    /// Drained on settlement (matching `pending_fetches`) and on
    /// abort fan-out.
    ///
    /// GC contract: values are signals which already carry their
    /// own root path through [`Self::abort_signal_states`] (and the
    /// user's `controller.signal` reference).  Sweep prunes entries
    /// whose signal value was collected so a recycled slot can't
    /// claim a stale fan-out — same defensive pattern as
    /// `fetch_abort_observers`.
    #[cfg(feature = "engine")]
    pub(crate) fetch_signal_back_refs: HashMap<elidex_net::broker::FetchId, ObjectId>,
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
    /// Which global scope this VM realizes (WHATWG HTML §10.2.1.1).
    /// `register_globals` reads it to fork the Window-only prototype block;
    /// the worker globals (`postMessage` / `close` / `WorkerLocation`) read
    /// the embedded name + script URL. Set once at construction.
    #[cfg(feature = "engine")]
    pub(crate) global_scope_kind: GlobalScopeKind,
    /// Worker-side outgoing `postMessage` data (JSON strings), enqueued by the
    /// worker scope's `postMessage()` (WHATWG HTML §10.2.1.2) and drained by
    /// the worker thread loop into `WorkerToParent::PostMessage`. Empty in a
    /// Window VM — workers never route through the window `pending_tasks`
    /// queue (that is Window-target only).
    #[cfg(feature = "engine")]
    pub(crate) worker_outgoing: Vec<String>,
    /// Set when the worker scope calls `close()` (WHATWG HTML §10.2.1.2 /
    /// the §10.2.2 closing flag). The worker thread loop observes it and
    /// exits after the current tick. Always `false` in a Window VM.
    #[cfg(feature = "engine")]
    pub(crate) worker_close_requested: bool,
    /// Main-side registry of spawned dedicated workers
    /// ([`WorkerId`](elidex_api_workers::WorkerId) → transport handle), keyed
    /// the same as each `Worker` object's `WorkerRef` ECS component (WHATWG
    /// HTML §10.2.6). Empty in a worker VM (workers do not currently spawn
    /// nested workers). Holds only cross-thread channel handles + `JoinHandle`s
    /// — listener state lives in the `EventListeners` ECS component on the
    /// `Worker` entity, not here.
    #[cfg(feature = "engine")]
    pub(crate) worker_registry: elidex_api_workers::WorkerRegistry,
    /// Live-worker `WorkerId` → backing `NodeKind::Worker` entity map (main
    /// mode). The drain iterates **this** (live workers only) rather than
    /// scanning every `WorkerRef` entity in the world — terminated workers'
    /// entities are retained for the brand check (so `postMessage` after close
    /// stays a silent no-op) but removed from here, so the per-frame drain
    /// stays O(live workers). Lifecycle-synced with [`Self::worker_registry`]:
    /// inserted by the `Worker` ctor, removed on `terminate()` / close-drain /
    /// unbind.
    #[cfg(feature = "engine")]
    pub(crate) worker_entities: HashMap<elidex_api_workers::WorkerId, elidex_ecs::Entity>,
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
