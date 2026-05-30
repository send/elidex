//! [`ObjectKind`] enum + impl, split out of [`super::value`] to
//! keep that file below the 1000-line convention (cleanup tranche
//! 2 — final file).
//!
//! ## Why a dedicated module
//!
//! `ObjectKind` is the dominant content of `vm/value.rs` (~440
//! lines on its own — every variant carries multi-line spec /
//! GC / WebIDL doc comments).  Splitting it out shrinks the
//! parent file substantially while keeping the type itself
//! reachable at the canonical `value::ObjectKind` path via a
//! `pub use` re-export, so no caller has to update an import.
//!
//! ## Variant shape
//!
//! Every JS-facing object type lives as one variant here.  The
//! VM's heap stores [`super::value::Object`] entries whose
//! `kind: ObjectKind` field discriminates between:
//!
//! - **Built-in primitives**: `Ordinary`, `Array`, `Function`,
//!   `BoundFunction`, `NativeFunction`, `Arguments`,
//!   `BooleanWrapper` / `NumberWrapper` / `StringWrapper` /
//!   `BigIntWrapper` / `SymbolWrapper`.
//! - **Iterator state**: `ArrayIterator`, `StringIterator`,
//!   `ForInIterator`.
//! - **Async machinery**: `Promise`, `PromiseResolver`,
//!   `PromiseCombinatorState`, `PromiseCombinatorStep`,
//!   `PromiseFinallyStep`, `Generator`, `AsyncDriverStep`.
//! - **DOM / Web-related types**: `Event`, `HostObject`,
//!   `AbortController` / `AbortSignal`, the Fetch family
//!   (`Headers` / `Request` / `Response` / `Blob`), the typed-array
//!   family (`ArrayBuffer` / `TypedArray` / `DataView`), live DOM
//!   collections (`HtmlCollection` / `NodeList` / `NamedNodeMap` /
//!   `Attr`), the structured-clone helpers
//!   (`TextEncoder` / `TextDecoder`), and `Error`.
//!   Some of these variants are additionally gated on
//!   `feature = "engine"`; see the individual enum variants for the
//!   exact `cfg` boundaries.

use super::coroutine_types::{
    GeneratorState, PromiseCombinatorState, PromiseCombinatorStep, PromiseState,
};
use super::value::{
    ArrayIterState, BigIntId, ElementKind, ForInState, FunctionObject, JsValue, NativeFunction,
    ObjectId, StringId, StringIterState, SymbolId,
};

/// Discriminator for [`ObjectKind::DOMTokenList`] indicating which
/// content attribute backs the wrapper.  Slot
/// `#11-tags-T2a-url-bearing` (CRIT-2 Option A — separate per-attr
/// caches with shared `DOMTokenList.prototype`).  Encoded as `u8`
/// for compact storage in `ObjectKind`.
#[cfg(feature = "engine")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DomTokenListSource {
    /// `Element.classList` → `class` attribute.
    Class = 0,
    /// `<a>.relList` / `<area>.relList` → `rel` attribute.
    RelHyperlink = 1,
    /// `<link>.relList` → `rel` attribute.
    RelLink = 2,
    /// `<link>.sizes` → `sizes` attribute.
    LinkSizes = 3,
    /// `<output>.htmlFor` → `for` attribute (slot `#11-tags-T2d-interactive`).
    OutputHtmlFor = 4,
}

/// Discriminator for [`ObjectKind::Observer`] identifying which of the
/// three observer surfaces (WHATWG DOM §4.3.1 `MutationObserver`, W3C
/// Resize Observer §2.1 `ResizeObserver`, W3C Intersection Observer §2.2
/// `IntersectionObserver`) the instance brands as.
///
/// All three observer JS-objects share the identical payload shape — a
/// single per-registry monotonic `observer_id: u64` (each kind's
/// registry owns its own counter, so the three kinds share the `u64`
/// keyspace independently — the inline kind discriminator
/// disambiguates) keyed into per-kind `HostData::*_observer_bindings`
/// / registry state — so they collapse
/// into a single `ObjectKind::Observer { kind, observer_id }` variant
/// per CLAUDE.md "One issue, one way" + lesson #276 (ObjectKind
/// Resolution Path Uniformity).  Brand check ramifies on this enum;
/// the rest of the GC / structured-clone / trace machinery treats all
/// three identically (payload-free, side-table-rooted).
#[cfg(feature = "engine")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ObserverKind {
    /// `MutationObserver` (WHATWG DOM §4.3.1).
    Mutation = 0,
    /// `ResizeObserver` (W3C Resize Observer §2.1).
    Resize = 1,
    /// `IntersectionObserver` (W3C Intersection Observer §2.2).
    Intersection = 2,
}

#[cfg(feature = "engine")]
impl ObserverKind {
    /// The WebIDL interface name for this observer kind — used by
    /// brand-check error messages (`"Failed to execute '{method}' on
    /// '{interface}': Illegal invocation"`).
    #[must_use]
    pub fn interface_name(self) -> &'static str {
        match self {
            Self::Mutation => "MutationObserver",
            Self::Resize => "ResizeObserver",
            Self::Intersection => "IntersectionObserver",
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
    /// Wrapper object for Number primitives (§10.2.1.2 this-boxing).
    NumberWrapper(f64),
    /// Wrapper object for String primitives (§10.2.1.2 this-boxing).
    StringWrapper(StringId),
    /// Wrapper object for Boolean primitives (§10.2.1.2 this-boxing).
    BooleanWrapper(bool),
    /// Wrapper object for BigInt primitives.
    BigIntWrapper(BigIntId),
    /// Wrapper object for Symbol primitives.
    SymbolWrapper(SymbolId),
    /// A Promise (ECMA-262 §27.2).  Holds the state machine (status + result)
    /// and reaction lists; reactions are drained via the microtask queue.
    Promise(PromiseState),
    /// Resolve/reject function bound to a specific Promise capability
    /// (ECMA-262 §27.2.1.3).  Created synchronously by `new Promise(executor)`
    /// and handed to the executor; settling is idempotent because subsequent
    /// invocations see `status != Pending` and become no-ops.
    PromiseResolver {
        /// The Promise this function settles.
        promise: ObjectId,
        /// Which side — `true` for reject, `false` for resolve.
        is_reject: bool,
    },
    /// Aggregator state for `Promise.all` / `Promise.allSettled` / `Promise.any`
    /// (ECMA-262 §27.2.4.1 / §27.2.4.2 / §27.2.4.3).  Shared across every
    /// per-item step; holds the output promise, the values vec (fulfilled
    /// results for all, `{status,value/reason}` objects for allSettled,
    /// rejection reasons for any), and the remaining counter.
    PromiseCombinatorState(PromiseCombinatorState),
    /// Per-item fulfill/reject step for a combinator.  Stores the shared
    /// state handle + the index the callback should write, so the native
    /// function pointer itself stays stateless.
    PromiseCombinatorStep(PromiseCombinatorStep),
    /// `Promise.prototype.finally` wrapper step — calls `on_finally()` and
    /// then passes through the original value (or re-throws the original
    /// reason).  Thenable assimilation on the `on_finally` return value is
    /// deferred (see PR2 plan "Test262 alignment").
    PromiseFinallyStep {
        on_finally: ObjectId,
        is_reject: bool,
    },
    /// An ECMA-262 §27.5 Generator object.  Created by a generator function
    /// call (the function body never runs on the initial call — instead,
    /// the Generator holds the initial suspended frame).  `.next()` /
    /// `.return()` / `.throw()` drive execution.
    Generator(Box<GeneratorState>),
    /// Continuation callback attached to the awaited Promise of an async
    /// function.  When the Promise settles, this step resumes the
    /// associated coroutine with the fulfilment value (or rethrows the
    /// rejection reason inside the coroutine, depending on `is_throw`).
    AsyncDriverStep {
        /// The ObjectId of the `ObjectKind::Generator` carrying
        /// `GeneratorState { wrapper: Some(_), .. }`.
        gen: ObjectId,
        /// `false` for the fulfil handler (resume normally), `true` for
        /// the reject handler (throw the received value inside the body).
        is_throw: bool,
    },
    /// An Event object — the JS-side view of a DispatchEvent during
    /// listener invocation.
    ///
    /// All three `*_prevented` / `*_stopped` flags live in internal slots
    /// (not observable as own properties) and are written by the native
    /// methods `preventDefault` / `stopPropagation` /
    /// `stopImmediatePropagation`.  The `cancelable` / `passive` fields
    /// are immutable after construction — `preventDefault` silently
    /// no-ops when either is `false` (matches browser behaviour for
    /// passive listeners and non-cancelable events, WHATWG DOM §2.9).
    ///
    /// `composed_path` is the lazily-built JS array returned by
    /// `composedPath()` — cached on first call so repeated invocations
    /// observe identical array identity, per WHATWG DOM §2.9
    /// (`composedPath()` returns the internal list, so the same Array
    /// exotic object is returned — exposing a different array each
    /// call would be spec-non-conforming).
    Event {
        default_prevented: bool,
        propagation_stopped: bool,
        immediate_propagation_stopped: bool,
        /// Immutable — set from `DispatchEvent::cancelable` at construction.
        cancelable: bool,
        /// Immutable — `true` when this event object is threaded to a
        /// listener registered with `{passive: true}`.  Gates
        /// `preventDefault` into a silent no-op.
        passive: bool,
        /// Immutable internal slot — set from the ctor's first argument.
        /// `dispatchEvent` reads this for listener type matching so a
        /// user-side `delete evt.type` / overridden prototype accessor
        /// cannot hijack dispatch (matches browsers' IDL-attribute /
        /// internal-slot semantics; the data property on the instance
        /// is a mirror of this slot).
        type_sid: StringId,
        /// Immutable internal slot — `event.bubbles` as established by
        /// the ctor.  Read by `dispatchEvent` in place of the JS
        /// property so `delete evt.bubbles` cannot turn a bubbling
        /// event into a non-bubbling one mid-dispatch.
        bubbles: bool,
        /// Immutable internal slot — `event.composed` as established by
        /// the ctor.  Read by `dispatchEvent` in place of the JS
        /// property so `delete evt.composed` cannot change shadow
        /// boundary crossing behaviour mid-dispatch.
        composed: bool,
        /// Lazily-allocated `[target, ...ancestors]` Array returned by
        /// `composedPath()`.  `None` until the first call.
        composed_path: Option<ObjectId>,
    },
    /// Host (DOM) object — the VM-side wrapper for an ECS `Entity`.
    ///
    /// Every DOM element / document / window surfaces in JS as a
    /// `HostObject` with its entity packed into `entity_bits`
    /// (`Entity::to_bits().get()`).  Native DOM methods recover the
    /// Entity by pattern-matching on this variant and consulting
    /// `HostData::dom()`.
    ///
    /// Identity is preserved across lookups (`el === el`) via
    /// `HostData::wrapper_cache`, which maps `entity_bits` to the
    /// existing `ObjectId` so repeated `create_element_wrapper` calls
    /// return the same object.  The prototype is `EventTarget.prototype`
    /// so `addEventListener` / `removeEventListener` / `dispatchEvent`
    /// are inherited without per-wrapper allocation.
    ///
    /// The variant carries no `ObjectId` references, so GC has nothing
    /// to trace.  The wrapper itself is kept alive by `wrapper_cache`
    /// (rooted via `HostData::gc_root_object_ids`).
    HostObject { entity_bits: u64 },
    /// `AbortSignal` instance (WHATWG DOM §3.1).  An EventTarget that
    /// is *not* a Node — its prototype chain skips `Node.prototype`
    /// and goes directly `AbortSignal.prototype → EventTarget.prototype
    /// → Object.prototype` (same shape as `Window`).
    ///
    /// The mutable signal state (`aborted` flag, `reason` value,
    /// registered `'abort'` callbacks, and back-references for
    /// `addEventListener({signal})` auto-removal) lives **out-of-band**
    /// in `VmInner::abort_signal_states`, keyed by this object's
    /// `ObjectId`.  Keeping the variant payload-free preserves
    /// per-variant size discipline (every other DOM-side wrapper is
    /// also payload-free or holds at most a small `Copy` field) and
    /// lets GC trace state without widening `ObjectKind`.
    ///
    /// GC contract: the trace step looks up the entry in
    /// `abort_signal_states` and marks `reason` + every
    /// `abort_listeners` callback ObjectId.  After sweep, dead
    /// AbortSignal entries are removed from the HashMap so the next
    /// allocation that recycles the `ObjectId` slot does not inherit
    /// stale state.
    AbortSignal,
    /// `AbortController` instance (WHATWG DOM §3.1).  Carries the
    /// paired `AbortSignal`'s `ObjectId` as an internal slot — the
    /// spec models this as `[[signal]]` on the controller, accessible
    /// only via `controller.signal` and `controller.abort()`.  Storing
    /// the reference here (rather than reading it back from the
    /// JS-visible `signal` own property) means user code cannot
    /// retarget `abort()` by mutating object storage
    /// (`Object.defineProperty(c, 'signal', {value: alien})`) and
    /// `AbortController.prototype.abort.call({signal: realSignal})`
    /// throws TypeError instead of aborting `realSignal` (Copilot
    /// R4 finding).
    ///
    /// The visible `signal` data property is still set by the
    /// constructor for normal `controller.signal` reads — both the
    /// own-property write and the internal-slot write happen in
    /// `native_abort_controller_constructor`, and they always agree
    /// because user JS cannot reach the constructor's internal-slot
    /// write path.
    ///
    /// GC contract: trace marks `signal_id` so the paired signal
    /// survives as long as the controller is reachable.
    AbortController { signal_id: ObjectId },
    /// `Headers` instance (WHATWG Fetch §5.2) — the header list
    /// backing `Request.headers` / `Response.headers` and also
    /// constructible standalone via `new Headers(init)`.
    ///
    /// The actual header list (lowercased name → value strings) plus
    /// the WebIDL `guard` (`none` / `immutable` / `request` /
    /// `response` / `request-no-cors`) lives **out-of-band** in
    /// `VmInner::headers_states`, keyed by this object's `ObjectId`.
    /// Payload-free here so per-variant size discipline matches
    /// [`Self::AbortSignal`].
    ///
    /// GC contract: the trace step looks up the entry in
    /// `headers_states` — entries carry interned `StringId`s only
    /// (pool-permanent), so no `mark_value` / `mark_object` pass is
    /// needed for the payload.  Sweep tail prunes entries whose key
    /// was collected so a recycled slot does not inherit stale
    /// header lists.
    #[cfg(feature = "engine")]
    Headers,
    /// `Request` instance (WHATWG Fetch §5.3).  Payload-free; the
    /// `method` / `url` / `headers_id` state lives in
    /// `VmInner::request_states`.  Body bytes (when present) live
    /// in the shared `VmInner::body_data` map keyed by this
    /// object's `ObjectId` — `clone()` deep-copies the entry's
    /// `Vec<u8>` so the cloned Request owns its bytes
    /// independently.
    ///
    /// GC contract: the trace step marks the `headers_id` Companion
    /// from the state entry.  Body bytes are plain `Vec<u8>` (no
    /// `ObjectId` references), so they need no marking.  Sweep
    /// tail prunes `request_states` / `body_data` / `disturbed`
    /// entries whose key was collected.
    #[cfg(feature = "engine")]
    Request,
    /// `Response` instance (WHATWG Fetch §5.5).  Payload-free;
    /// `status` / `statusText` / `url` / `headers_id` /
    /// `response_type` live in `VmInner::response_states`.  Body
    /// bytes share `VmInner::body_data` with `Request` — the map
    /// key is the Response's `ObjectId`.  IDL readonly attrs read
    /// from the side-table state, not from own-data properties on
    /// the instance, so `delete resp.status` is immune (PR5a2 R7.1
    /// lesson: IDL attr internal slot is authoritative).
    ///
    /// GC contract: same shape as `Request` — mark the companion
    /// `headers_id`, prune state / body entries in the sweep tail.
    #[cfg(feature = "engine")]
    Response,
    /// `ArrayBuffer` instance (ECMA-262 §25.1, minimal Phase 2 form).
    /// Payload-free; the backing bytes live in the shared
    /// `VmInner::body_data` map (owned `Vec<u8>`) keyed by this
    /// object's `ObjectId`.  `.slice()` allocates a fresh
    /// ArrayBuffer with its own `Vec<u8>` range copy.  TypedArray
    /// and DataView views read and **mutate in place** through
    /// `byte_io` over the shared `body_data` entry, so writes
    /// through any view are visible through any other view over
    /// the same `buffer_id`.
    ///
    /// IDL readonly `byteLength` reads the `Vec<u8>::len()` of
    /// the `body_data` entry — authoritative internal slot (PR5a2
    /// R7.1 lesson: `delete buf.byteLength` must not break reads).
    ///
    /// GC contract: payload-free — the trace step has nothing to
    /// fan out (body bytes are plain `Vec<u8>`, no ObjectId
    /// references).  Sweep tail pruning of `body_data` already
    /// drops dead-key entries alongside Request / Response.
    #[cfg(feature = "engine")]
    ArrayBuffer,
    /// `Blob` instance (File API §3, minimal Phase 2 form).
    /// Payload-free; bytes plus MIME type live out-of-band in
    /// `VmInner::blob_data` keyed by this object's `ObjectId`.
    /// Body bytes are **not** shared with `body_data` — Blob has
    /// its own side table because its state carries a `type_sid`
    /// alongside the bytes.
    ///
    /// IDL readonly `size` / `type` read from the authoritative
    /// side-table slot.
    ///
    /// GC contract: payload-free — blob bytes hold no ObjectId
    /// references.  The sweep tail prunes `blob_data` entries whose
    /// key was collected.
    #[cfg(feature = "engine")]
    Blob,
    /// `HTMLCollection` instance (WHATWG DOM §4.2.10).  A *live*
    /// ordered collection of Element nodes matching one of several
    /// filter kinds (by tag, by class, children, forms / images /
    /// links).  Payload-free; the backing
    /// [`elidex_dom_api::LiveCollection`] (root Entity, owned filter
    /// strings, cached snapshot, subtree version) lives out-of-band
    /// in `VmInner::live_collection_states` keyed by this object's
    /// `ObjectId`.
    ///
    /// Live semantics: every read (`length`, `item(i)`, indexed
    /// access, iterator) consults the cached snapshot, refreshing
    /// it from the ECS on a subtree-version bump. Callers that need
    /// a snapshot can spread into an Array.
    ///
    /// GC contract: the side-table holds only `Entity`, owned
    /// `String` / `Vec<String>` (filter needles for
    /// `ByTagName` / `ByName` / `ByClassNames`), `Vec<Entity>`
    /// (cached snapshot + `Snapshot` filter's frozen list), and
    /// `u64` (cached subtree version) — no `ObjectId` references,
    /// so **no GC tracing is required**; the sweep tail prunes
    /// `live_collection_states` entries whose `ObjectId` key was
    /// collected.
    #[cfg(feature = "engine")]
    HtmlCollection,
    /// `NodeList` instance (WHATWG DOM §4.2.10.1).  An ordered
    /// collection of Node values — may be *live* (from
    /// `Node.prototype.childNodes` or `document.getElementsByName`)
    /// or *static* (from `querySelectorAll`, per §4.2.6 — backed by
    /// [`elidex_dom_api::CollectionFilter::Snapshot`]).  Shares
    /// the `live_collection_states` side-table with `HtmlCollection`;
    /// the [`elidex_dom_api::LiveCollection::kind`] field
    /// disambiguates between the two interfaces, and the filter
    /// variant records whether a `NodeList` is live or snapshot-
    /// backed. Do not extend `ObjectKind` for new collection
    /// variants — extend `CollectionFilter` / `CollectionKind`
    /// instead so the engine-bound prototype split stays minimal.
    ///
    /// GC contract: identical to [`Self::HtmlCollection`] — the
    /// side-table carries no `ObjectId` references; pruning alongside
    /// HTMLCollection entries in the sweep tail is sufficient.  For
    /// static NodeLists, the `LiveCollection`'s `Snapshot` filter
    /// stores a `Vec<Entity>` whose entries are plain ECS keys (no
    /// ObjectId), so the `Vec` likewise needs no tracing.
    #[cfg(feature = "engine")]
    NodeList,
    /// `NamedNodeMap` instance (WHATWG DOM §4.9.1) — the live
    /// collection of an Element's attributes exposed via
    /// `element.attributes`.  Payload-free; the backing Element
    /// `Entity` lives in `VmInner::named_node_map_states` keyed by
    /// this `ObjectId`.
    ///
    /// Per spec, NamedNodeMap reflects the element's current
    /// attribute list on every read — add / remove / update through
    /// `setAttribute` et al. are visible to a previously-obtained
    /// NamedNodeMap instance.  Implemented by re-reading the ECS
    /// `Attributes` component on each access, matching the
    /// HTMLCollection / NodeList design (no cache, no invalidation
    /// surface).
    ///
    /// GC contract: the side-table stores only an `Entity` — no
    /// `ObjectId` references — so no trace fan-out.  Sweep tail
    /// prunes entries whose key `ObjectId` was collected.
    #[cfg(feature = "engine")]
    NamedNodeMap,
    /// `DOMTokenList` instance (WHATWG DOM §3.5 / §7.1) — the
    /// live wrapper backing `Element.classList`.  Carries the owner
    /// `Entity` inline (`entity_bits`) so the indexed-property
    /// exotic + the 10 `classList.*` natives can recover the owner
    /// without a side-table lookup; every accessor re-reads the
    /// owner's `class` attribute through the
    /// `elidex_dom_api::class_list` handlers, so the wrapper is
    /// stateless beyond the entity reference.
    ///
    /// Identity is preserved via `VmInner::class_list_wrapper_cache`
    /// (when `source` is `DomTokenListSource::Class`) and the
    /// per-attribute caches added in slot `#11-tags-T2a-url-bearing`
    /// (`rel_list_wrapper_cache` / `link_rel_list_wrapper_cache` /
    /// `link_sizes_wrapper_cache`).  GC contract: payload-free in
    /// trace terms (the `entity_bits` is not an `ObjectId`); the
    /// sweep tail prunes the matching cache entry whose value
    /// `ObjectId` was collected.
    ///
    /// `source` discriminates which content attribute backs the
    /// wrapper — see `DomTokenListSource`.  The native methods on
    /// `DOMTokenList.prototype` route their `invoke_dom_api` method
    /// name (`"classList.add"` / `"relList.add"` /
    /// `"linkSizes.add"`) by reading this discriminator at call time.
    #[cfg(feature = "engine")]
    DOMTokenList {
        entity_bits: u64,
        source: DomTokenListSource,
    },
    /// `DOMStringMap` instance (WHATWG HTML §3.2.6 / WebIDL §3.10)
    /// — the named-property exotic backing `HTMLElement.dataset`.
    /// Carries the owner `Entity` inline.  `[[Get]]` / `[[Set]]` /
    /// `[[Delete]]` / `[[OwnPropertyKeys]]` route through
    /// `dataset.*` handlers in `elidex_dom_api::element::attrs`.
    ///
    /// Identity is preserved via `VmInner::dataset_wrapper_cache`
    /// keyed by owner `Entity`.  GC contract: same as
    /// [`Self::DOMTokenList`] — the `entity_bits` is not an
    /// `ObjectId`, so trace fan-out is a no-op; the sweep tail prunes
    /// `dataset_wrapper_cache` entries whose value `ObjectId` was
    /// collected.
    #[cfg(feature = "engine")]
    DOMStringMap { entity_bits: u64 },
    /// `CSSStyleDeclaration` instance (CSSOM §6.6.1) — backs both
    /// `Element.style` (mutable inline-style) and the read-only
    /// declaration returned by `window.getComputedStyle(el)`.
    ///
    /// `source` discriminates the backing store:
    /// - `0` (Inline): `key_bits` = owner `Entity::to_bits().get()`.
    ///   Mutable; identity-cached per Entity via
    ///   `VmInner::style_wrapper_cache` so `el.style === el.style`.
    /// - `1` (Computed): `key_bits` = owner `Entity::to_bits().get()`.
    ///   Read-only; allocated fresh on each `getComputedStyle` call
    ///   (matches WPT — identity is NOT preserved across reads).
    ///
    /// PR-B will extend with `source = 2` (Rule) keyed by
    /// `(sheet_entity_bits << 32) | rule_id_low_32_bits`.  Keeping
    /// the variant unified with a tagged source saves ~600 LoC of
    /// dispatch boilerplate vs. three separate ObjectKinds.
    ///
    /// GC contract: payload-free in trace terms (`source` / `key_bits`
    /// carry no `ObjectId` references).  Sweep tail prunes the Inline
    /// `style_wrapper_cache` entries whose value `ObjectId` was
    /// collected; Computed wrappers are not cached so no prune needed.
    #[cfg(feature = "engine")]
    CSSStyleDeclaration { source: u8, key_bits: u64 },
    /// `CSSStyleSheet` instance (CSSOM §6.2) — wraps a `<style>` element
    /// entity; `cssRules` / `insertRule` / `deleteRule` route to the
    /// per-`<style>` parsed `Stylesheet` snapshot in
    /// `SessionCore::cssom_sheets`.  Identity preserved via
    /// `VmInner::stylesheet_wrapper_cache` keyed by owner `Entity` so
    /// `el.sheet === el.sheet`.
    ///
    /// GC contract: payload-free in trace terms (`entity_bits` carries
    /// no `ObjectId` reference).  Sweep tail prunes
    /// `stylesheet_wrapper_cache` entries whose value `ObjectId` was
    /// collected.
    #[cfg(feature = "engine")]
    CSSStyleSheet { entity_bits: u64 },
    /// `CSSRuleList` instance (CSSOM §6.3) — `[[Indexed]]` legacy
    /// platform object backing `CSSStyleSheet.cssRules`.  Fresh-alloc
    /// on every `cssRules` read (matches WPT identity rules — repeated
    /// reads do NOT preserve the same reference; framework code rarely
    /// holds the rule list anyway).
    ///
    /// GC contract: same as [`Self::CSSStyleSheet`] — payload-free.
    #[cfg(feature = "engine")]
    CSSRuleList { sheet_entity_bits: u64 },
    /// `CSSStyleRule` instance (CSSOM §6.4 / §6.6) — opaque rule wrapper
    /// keyed by `(sheet entity, rule_id)` where `rule_id` is the stable
    /// id issued by [`elidex_css::Stylesheet::next_rule_id`] at parse /
    /// `insertRule` time.  Identity preserved via
    /// `VmInner::css_style_rule_wrapper_cache` keyed by `(Entity, u64)`.
    ///
    /// GC contract: payload-free.  Sweep tail prunes
    /// `css_style_rule_wrapper_cache` entries whose value was collected.
    #[cfg(feature = "engine")]
    CSSStyleRule {
        sheet_entity_bits: u64,
        rule_id: u64,
    },
    /// Rule-source `CSSStyleDeclaration` (CSSOM §6.6.1).  Distinct
    /// variant from [`Self::CSSStyleDeclaration`] because the (sheet,
    /// rule_id) key needs both `u64`s inline — packing them into the
    /// PR-A unified variant's `key_bits: u64` would require truncating
    /// `Entity::to_bits()` which is not safe across hecs generations.
    /// Shares `css_style_declaration_prototype` with the Inline /
    /// Computed sources so brand checks accept either kind; the
    /// dispatch trampoline routes by variant.
    ///
    /// PR-B ships read-only declaration access (`getPropertyValue` /
    /// `length` / `item` / `cssText` get).  Mutators
    /// (`setProperty` / `removeProperty` / `cssText.set`) are silent
    /// no-ops — write-back through rule-source mutation is deferred
    /// to slot `#11-css-rule-style-mutation` (requires Selector +
    /// Declaration serialisers to round-trip the rule's
    /// `source_text`).
    ///
    /// GC contract: payload-free.  Sweep tail prunes
    /// `rule_style_wrapper_cache` entries whose value was collected.
    #[cfg(feature = "engine")]
    CSSRuleStyleDeclaration {
        sheet_entity_bits: u64,
        rule_id: u64,
    },
    /// `StyleSheetList` instance (CSSOM §6.8) — `[[Indexed]]` legacy
    /// platform object backing `document.styleSheets`.  Fresh-alloc
    /// on every read (mirrors `CSSRuleList`).  The walker enumerates
    /// `<style>` descendants of the document; `<link rel="stylesheet">`
    /// is deferred to slot `#11-link-stylesheet-loading`.
    ///
    /// GC contract: payload-free.
    #[cfg(feature = "engine")]
    StyleSheetList { document_entity_bits: u64 },
    /// `Attr` instance (WHATWG DOM §4.9.2) — the wrapper returned by
    /// `getAttributeNode` / `setAttributeNode` / NamedNodeMap
    /// indexed + named access.  Payload-free; the backing
    /// (owner `Entity`, qualified-name `StringId`) tuple lives in
    /// `VmInner::attr_states` keyed by this `ObjectId`.
    ///
    /// Phase 2 simplification: `namespaceURI` / `prefix` return
    /// `null` for every Attr, `localName` equals the qualified
    /// name — XML namespace support lands in Phase 3 alongside
    /// full XML document handling (plan §Deferred #21).
    ///
    /// Identity is **not** preserved across calls: repeated
    /// `getAttributeNode('id')` allocates a fresh wrapper.  This
    /// mirrors HTMLCollection / NodeList's per-access allocation
    /// and avoids the GC root machinery that a cache would demand.
    ///
    /// GC contract: `AttrState` holds an `Entity` and a `StringId`
    /// — no `ObjectId` references — so no trace fan-out.  Sweep
    /// tail prunes `attr_states` entries whose key `ObjectId` was
    /// collected.
    #[cfg(feature = "engine")]
    Attr,
    /// `TypedArray` instance view over an `ArrayBuffer` (ECMA-262 §23.2) —
    /// one of the 11 concrete subclasses identified by `element_kind`.
    /// The `[[ViewedArrayBuffer]]` / `[[ByteOffset]]` / `[[ByteLength]]` /
    /// `[[ArrayLength]]` / `[[TypedArrayName]]` / `[[ContentType]]` spec
    /// slots are all derived from the four fields carried here:
    /// - `[[ViewedArrayBuffer]]` ← `buffer_id`
    /// - `[[ByteOffset]]` ← `byte_offset`
    /// - `[[ByteLength]]` ← `byte_length`
    /// - `[[ArrayLength]]` ← `byte_length / element_kind.bytes_per_element()`
    /// - `[[TypedArrayName]]` ← `element_kind.name()`
    /// - `[[ContentType]]` ← `element_kind.is_bigint() ? BigInt : Number`
    ///
    /// All four fields are **immutable after construction** in this PR —
    /// `ArrayBuffer.prototype.transfer` / `resize` / `detached` tracking
    /// (ES2024) are deferred to the M4-12 cutover-residual PR along with
    /// transferable integration.  Matches the `Event` variant precedent
    /// of immutable spec slots inline in the enum.
    ///
    /// GC contract: the trace step marks `buffer_id` so the backing
    /// ArrayBuffer survives as long as any view is reachable.  No
    /// side-table — there is nothing to prune post-sweep.
    TypedArray {
        buffer_id: ObjectId,
        byte_offset: u32,
        byte_length: u32,
        element_kind: ElementKind,
    },
    /// `DataView` instance view over an `ArrayBuffer` (ECMA-262 §25.3) —
    /// endian-aware read / write at byte-level granularity.  Unlike
    /// `TypedArray`, has no element-kind: callers pick the type per
    /// call via `getInt8` / `getFloat64` etc., with an optional
    /// `littleEndian` boolean (default `false` per §25.3.4).
    ///
    /// `[[ViewedArrayBuffer]]` / `[[ByteOffset]]` / `[[ByteLength]]`
    /// live inline as for `TypedArray`.
    ///
    /// GC contract: identical to `TypedArray` — mark `buffer_id`, no
    /// side table to prune.
    DataView {
        buffer_id: ObjectId,
        byte_offset: u32,
        byte_length: u32,
    },
    /// `TextEncoder` instance (WHATWG Encoding §8.2).  Stateless
    /// (encoding is always `"utf-8"`), payload-free — brand check
    /// is the sole reason a dedicated variant exists rather than
    /// reusing `Ordinary`.  Lets `encode` / `encodeInto` reject
    /// `{encode: TextEncoder.prototype.encode}.encode()` with
    /// TypeError instead of silently misbehaving.
    ///
    /// GC contract: payload-free — nothing to trace or prune.
    #[cfg(feature = "engine")]
    TextEncoder,
    /// `TextDecoder` instance (WHATWG Encoding §8.1).  Payload-free;
    /// the encoder handle + `fatal` / `ignoreBOM` flags live in
    /// `VmInner::text_decoder_states`.  Same model as `Headers` /
    /// `Request` / `Response` / `Blob` — keeping the variant
    /// payload-free preserves per-variant size discipline.
    ///
    /// GC contract: the state entry holds no `ObjectId` references
    /// (encoding is `&'static`, decoder state is opaque
    /// `encoding_rs::Decoder`), so the trace step does nothing.
    /// Sweep tail prunes entries whose key `ObjectId` was collected.
    #[cfg(feature = "engine")]
    TextDecoder,
    /// `URLSearchParams` instance (WHATWG URL §6).  Payload-free;
    /// the entry list (`Vec<(StringId, StringId)>` of name/value
    /// pairs in insertion order) lives out-of-band in
    /// `VmInner::url_search_params_states` keyed by this
    /// `ObjectId`.  Same model as `Headers` — keeping the variant
    /// payload-free preserves per-variant size discipline.
    ///
    /// GC contract: the entry list holds only interned `StringId`s
    /// (pool-permanent, no `ObjectId` references), so the trace
    /// step does nothing.  Sweep tail prunes entries whose key
    /// `ObjectId` was collected.
    #[cfg(feature = "engine")]
    URLSearchParams,
    /// `URL` instance (WHATWG URL §6.1).  Payload-free; the parsed
    /// [`url::Url`] + the linked `URLSearchParams` `ObjectId` (eagerly
    /// allocated by the constructor for `searchParams` identity
    /// stability) live out-of-band in `VmInner::url_states` keyed
    /// by this `ObjectId`.  Same model as `URLSearchParams` —
    /// keeping the variant payload-free preserves per-variant
    /// size discipline.
    ///
    /// GC contract: the trace step marks the linked `URLSearchParams`
    /// `ObjectId` if any, so `let p = new URL("…").searchParams; …`
    /// keeps the URL alive while only the `searchParams` reference
    /// is held (the `URLSearchParams` mutator natives consult
    /// `VmInner::usp_parent_url` to write changes back to the
    /// URL's query).  Sweep tail prunes entries whose key
    /// `ObjectId` was collected.
    #[cfg(feature = "engine")]
    URL,
    /// `FormData` instance (WHATWG XHR §4.3).  Payload-free;
    /// the entry list lives out-of-band in
    /// `VmInner::form_data_states` keyed by this
    /// `ObjectId`.  Each entry is `(name, value, filename?)`,
    /// where `value` is either a `StringId` (string entry) or a
    /// `Blob` `ObjectId` (file entry).
    ///
    /// GC contract: the trace step marks every Blob `ObjectId`
    /// referenced by the state's entry list so a Blob that was
    /// appended to a FormData survives as long as the FormData
    /// itself is reachable.  Sweep tail prunes entries whose key
    /// `ObjectId` was collected.
    #[cfg(feature = "engine")]
    FormData,
    /// `ReadableStream` instance (WHATWG Streams §4.2).  Payload-free;
    /// the state machine (state enum, queue, controller back-ref,
    /// reader back-ref, source callbacks, queuing-strategy
    /// algorithm) lives out-of-band in
    /// `VmInner::readable_stream_states` keyed by this
    /// `ObjectId`.
    ///
    /// GC contract: the trace step marks reachable values inside
    /// the state — queue chunks, source-callback ObjectIds,
    /// controller / reader back-refs, the size algorithm, the
    /// stored error.  Sweep tail prunes entries whose key
    /// `ObjectId` was collected.
    #[cfg(feature = "engine")]
    ReadableStream,
    /// `ReadableStreamDefaultReader` instance (WHATWG Streams §4.3).
    /// Payload-free; reader-owned state (back-ref to the stream,
    /// FIFO of pending `read()` promises, cached `closed` promise)
    /// lives out-of-band in
    /// `VmInner::readable_stream_reader_states`.
    ///
    /// GC contract: trace step marks the stream back-ref + every
    /// pending read Promise + the cached `closed` Promise.  This
    /// is the spec-correct ownership shape: §4.3 `[[readRequests]]`
    /// is an internal slot of the reader, so the promises share the
    /// reader's lifetime — no VM-level strong-root list needed.
    #[cfg(feature = "engine")]
    ReadableStreamDefaultReader,
    /// `ReadableStreamDefaultController` instance (WHATWG Streams
    /// §4.5).  Payload-free; carries only the parent stream's
    /// `ObjectId` as an internal slot — the controller's mutable
    /// state (the queue, `close_requested`, `pull_in_flight`) all
    /// lives on the parent `ReadableStreamState`, so the controller
    /// is just a brand-checked façade.
    ///
    /// GC contract: marks `stream_id` so the parent stream stays
    /// reachable.  The stream's own trace fan-out then handles the
    /// queue and source callbacks.
    #[cfg(feature = "engine")]
    ReadableStreamDefaultController { stream_id: ObjectId },
    /// Internal callable that handles the `start`-completion of a
    /// `ReadableStream` source (WHATWG Streams §4.2.4 step 10/11).
    /// Allocated by the constructor and registered as the fulfil
    /// (or reject, with `is_reject = true`) handler on the start
    /// promise.  When called it dispatches to
    /// `super::host::readable_stream::run_start_step`.
    ///
    /// Carries `stream_id` inline so the callable can locate its
    /// stream without a closure or BoundFunction wrapper — same
    /// shape as [`Self::PromiseFinallyStep`].
    #[cfg(feature = "engine")]
    ReadableStreamStartStep {
        stream_id: ObjectId,
        is_reject: bool,
    },
    /// Internal callable for the `pull`-completion of a
    /// `ReadableStream` source (WHATWG Streams §4.5.10).  Same
    /// shape as [`Self::ReadableStreamStartStep`]: invoked when the
    /// pull-promise settles, dispatches to
    /// `super::host::readable_stream::run_pull_step`.
    #[cfg(feature = "engine")]
    ReadableStreamPullStep {
        stream_id: ObjectId,
        is_reject: bool,
    },
    /// Internal callable for the source.cancel-completion of a
    /// `ReadableStream` (WHATWG Streams §4.4.2).  Settles the
    /// caller's `cancel()` Promise with the source-cancel result.
    #[cfg(feature = "engine")]
    ReadableStreamCancelStep { promise: ObjectId, is_reject: bool },
    /// Observer-family instance — `MutationObserver` (WHATWG DOM §4.3.1),
    /// `ResizeObserver` (W3C Resize Observer §2.1), or
    /// `IntersectionObserver` (W3C Intersection Observer §2.2),
    /// discriminated by inline [`ObserverKind`].
    ///
    /// Payload-free at the JS-object level: the per-observer registry
    /// state (queued records / target list / init config) lives on
    /// `HostData::{mutation,resize,intersection}_observers` and the JS
    /// `(callback, instance)` pair lives on
    /// `HostData::{mutation,resize,intersection}_observer_bindings`,
    /// all keyed by the same `observer_id: u64`.  The observation
    /// targets + options live as `{Mutation,Resize,Intersection}ObservedBy`
    /// components on the observed entities (ECS-native).  The observer
    /// ID is per-registry monotonic (each kind's registry owns its
    /// own `next_id` counter, so kinds share the `u64` keyspace
    /// independently) and sized at 64 bits to match
    /// `elidex_api_observers::*::*ObserverId::raw`.
    ///
    /// Three observer surfaces share this single variant per
    /// CLAUDE.md "One issue, one way" + lesson #276 (ObjectKind
    /// Resolution Path Uniformity): the state shape is identical
    /// across kinds (one `u64`), so the brand check parameterises on
    /// `ObserverKind` rather than splitting into three same-shaped
    /// variants.  See `host::observer_common::require_observer_receiver`
    /// for the generic brand-check helper.
    ///
    /// GC contract: the variant has no inline `ObjectId`, so the
    /// trace step has nothing to fan out.  The `ObjectId`s carried in
    /// each `*_observer_bindings` entry are rooted via
    /// [`super::host_data::HostData::gc_root_object_ids`] — they
    /// stay alive as long as the observer is registered.  `Vm::unbind`
    /// drains each registry's per-observer target lists (so a rebind
    /// to a different `EcsDom` cannot match a stale `Entity`) but
    /// intentionally retains the binding maps (keyed by per-registry
    /// monotonic `observer_id`, no cross-DOM aliasing risk) so a
    /// retained `mo` / `ro` / `io` reference can re-observe after a
    /// rebind with its callback intact.
    #[cfg(feature = "engine")]
    Observer {
        /// Which observer surface this instance brands as.
        kind: ObserverKind,
        /// Per-registry monotonic observer id keying the per-kind
        /// registry + `*_observer_bindings` map on `HostData`.
        observer_id: u64,
    },
    /// `Storage` instance backing `window.localStorage` /
    /// `window.sessionStorage` (WHATWG HTML §11.2).
    ///
    /// Carries the area discriminator inline (`is_local: true` for
    /// localStorage, `false` for sessionStorage). Identity is preserved
    /// via `VmInner::storage_local_instance` /
    /// `VmInner::storage_session_instance` — the Window getters return
    /// the cached `ObjectId` so `localStorage === localStorage` (WebIDL
    /// `[SameObject]`). The wrapper itself is stateless; every method
    /// reads through to `HostData::web_storage` (origin-scoped JSON
    /// persistence) for `Local` and `HostData::session_storage`
    /// (per-VM in-memory `IndexMap`) for `Session`.
    ///
    /// GC contract: payload-free in trace terms (`is_local: bool` is
    /// not an `ObjectId`); the caches in `VmInner` are cleared on
    /// `Vm::unbind` so a retained reference cannot leak the previous
    /// origin's data after a rebind.
    #[cfg(feature = "engine")]
    Storage { is_local: bool },
    /// `StorageEvent` instance (WHATWG HTML §11.4.2). Subclass of
    /// `Event`; the `key` / `oldValue` / `newValue` / `url` /
    /// `storageArea` attributes live as own-data props at terminal
    /// shape `precomputed_event_shapes.storage`.  No side-table state
    /// — payload is fully shape-resident.
    ///
    /// GC contract: this variant carries no inline `ObjectId`; the
    /// shape-resident `storageArea` slot (potentially a Storage
    /// reference) is traced via the ordinary shaped-storage walk.
    #[cfg(feature = "engine")]
    StorageEvent,
    /// `ValidityState` wrapper (HTML §4.10.20.3).  Carries the
    /// owning form-control entity so brand-checked accessor natives
    /// can re-read [`elidex_form::FormControlState`] each time —
    /// no flag mirroring on the JS side.
    ///
    /// Identity is preserved via
    /// `VmInner::validity_state_wrappers`
    /// (`HashMap<Entity, ObjectId>`) so `input.validity ===
    /// input.validity` holds.  Sweep-pruned weak-through-owner;
    /// the cache entry survives only while the owner element
    /// wrapper is reachable via `HostData::wrapper_cache`.
    #[cfg(feature = "engine")]
    ValidityState { entity_bits: u64 },
    /// `DataTransfer` instance (HTML DnD §6.2) — the transferable
    /// data container for clipboard / drag-and-drop events.
    /// Payload-free; the mutable state (drop-effect / effect-allowed
    /// enum, ordered entry list, `[SameObject]` wrapper caches for
    /// `items` / `files`, drag-image entity + offsets) lives
    /// out-of-band in `super::VmInner::data_transfer_states`
    /// keyed by this object's `ObjectId`.
    ///
    /// `new DataTransfer()` is exposed per HTML §6.2 since 2018 and
    /// creates an empty container.  UA-fired drag / clipboard events
    /// receive a populated instance (UA fire path deferred to slot
    /// `#11-event-dispatch-extra`).
    ///
    /// GC contract: the trace step fans out via the state entry's
    /// `items_wrapper`, `files_wrapper`, per-entry blob `ObjectId`,
    /// and the drag-image element wrapper (if any).  Sweep tail
    /// prunes `data_transfer_states` entries whose key was
    /// collected.  `Vm::unbind` additionally clears the entire
    /// state map because `drag_image_entity` is cross-DOM.
    #[cfg(feature = "engine")]
    DataTransfer,
    /// `DataTransferItem` wrapper (HTML DnD §6.3).  Carries the
    /// owning [`Self::DataTransfer`] `ObjectId` plus the entry index
    /// inline; the actual entry data lives on the parent's state
    /// (`items: Vec<DataTransferEntry>`).  Identity is preserved via
    /// `super::VmInner::data_transfer_item_wrapper_cache` keyed by
    /// `(parent_dt_id, index)` so `dt.items[0] === dt.items[0]`.
    ///
    /// GC contract: trace marks `parent_dt_id` so the parent stays
    /// reachable.  The cache entry survives only while the parent
    /// DataTransfer + the resolved item index are both live (sweep
    /// prunes entries whose value `ObjectId` was collected).
    #[cfg(feature = "engine")]
    DataTransferItem { parent_dt_id: ObjectId, index: u32 },
    /// `DataTransferItemList` wrapper (HTML DnD §6.3).  Indexed
    /// platform object reflecting the parent DataTransfer's item
    /// list.  Payload-free apart from the parent reference; every
    /// read consults the parent's state.  Identity preserved via
    /// the parent's `items_wrapper` slot (a single wrapper per
    /// parent — matches Chrome `[SameObject]` semantics).
    ///
    /// GC contract: trace marks `parent_dt_id`.
    #[cfg(feature = "engine")]
    DataTransferItemList { parent_dt_id: ObjectId },
    /// `Touch` instance (Touch Events §5).  Payload-free; the
    /// 12 IDL members (identifier / target / coordinates / radii /
    /// rotation / force) live in
    /// `super::VmInner::touch_states` keyed by this object's
    /// `ObjectId`.
    ///
    /// `new Touch(init)` is exposed per Touch Events §5.5 since
    /// 2014.  UA-fired touch events are deferred to slot
    /// `#11-event-dispatch-extra` (UA fire path still routes
    /// `EventPayload::Mouse`).
    ///
    /// GC contract: trace marks the state entry's `target`
    /// `ObjectId` (any EventTarget — Element / Document / Window /
    /// AbortSignal / etc.).  Sweep tail prunes `touch_states`
    /// entries whose key was collected.
    #[cfg(feature = "engine")]
    Touch,
    /// `TouchList` instance (Touch Events §5.6).  Indexed platform
    /// object backing TouchEvent.touches / targetTouches /
    /// changedTouches.  Payload-free; the ordered list of
    /// [`Self::Touch`] `ObjectId`s lives in
    /// `super::VmInner::touch_list_states`.  No constructor (per
    /// IDL); allocated by the TouchEvent ctor and by UA dispatch.
    ///
    /// GC contract: trace marks every `Touch` ObjectId in the
    /// state entry's `items` Vec.  Sweep tail prunes entries
    /// whose key was collected.
    #[cfg(feature = "engine")]
    TouchList,
    /// `Range` instance (WHATWG DOM §4.4).  Live range whose
    /// boundaries (`startContainer`/`startOffset` /
    /// `endContainer`/`endOffset` / `owner_document`) live in the
    /// engine-indep `LiveRangeRegistry` (registered at
    /// `document.createRange()` / `new Range()`; unregistered at
    /// GC sweep).  Carries only the monotonic registry ID inline.
    ///
    /// GC contract: payload-free in trace terms.  The Range struct
    /// holds Entity refs (start/end containers + owner_document)
    /// which are ECS-managed; the dangling-collapse fallback in
    /// `LiveRangeRegistry::finalize_pending` handles post-destroy
    /// consistency.  Sweep tail unregisters the RangeId from
    /// `LiveRangeRegistry` so the registry doesn't leak.
    #[cfg(feature = "engine")]
    Range { range_id: u64 },
    /// `StaticRange` instance (WHATWG DOM §4.5).  Eager / immutable
    /// boundary holder — NOT registered in `LiveRangeRegistry`,
    /// boundaries are captured at construction time and may become
    /// invalid as the tree mutates (`isValid()` validates lazily).
    ///
    /// GC contract: payload-free in trace terms.  Entity bits for
    /// `start_container` / `end_container` are ECS-managed; no
    /// rooting.  Stale entity bits return `isValid() == false`.
    #[cfg(feature = "engine")]
    StaticRange {
        start_container_bits: u64,
        start_offset: u32,
        end_container_bits: u64,
        end_offset: u32,
        /// Copilot R9: `HostData::bind_epoch` snapshot at ctor.
        /// `isValid()` rejects when the current epoch differs —
        /// detects retained instances across `Vm::unbind`/rebind
        /// even when the new `EcsDom` happens to reuse the same
        /// `Entity` slots.
        bind_epoch: u32,
    },
    /// `TreeWalker` instance (WHATWG DOM §6.4).  Stateful walker
    /// over a DOM subtree filtered by `whatToShow` + optional
    /// NodeFilter callback.  State (root / current node / filter
    /// callback ObjectId / active flag) lives in
    /// `HostData::tree_walker_states`.  Carries only the monotonic
    /// state-table ID inline.
    ///
    /// GC contract (Copilot R8): filter callback `ObjectId`s are
    /// reached via per-wrapper trace fan-out in `vm/gc/trace.rs`
    /// (looking up `filter_object_id` from
    /// `HostData::tree_walker_states[walker_id]` when the wrapper
    /// itself is being marked).  `HostData::gc_root_object_ids`
    /// explicitly does NOT root these filters — earlier rounds
    /// (R4-R7) used unconditional rooting and Copilot flagged the
    /// leak cycle where a filter closure capturing the wrapper
    /// would survive forever.  Wrapper unreachability now drops
    /// the filter naturally; sweep tail in `vm/gc/collect.rs`
    /// prunes the state-table entry alongside.
    #[cfg(feature = "engine")]
    TreeWalker { walker_id: u64 },
    /// `NodeIterator` instance (WHATWG DOM §6.1).  Stateful pre-
    /// order iterator with WHATWG §6.1 pre-removing-steps
    /// adjustment on DOM mutation.  State lives in
    /// `HostData::node_iterator_states_shared` (`Arc<Mutex<...>>`
    /// shared with `MutationBridge` so hook-fire path can adjust
    /// reference on `after_remove_with_descendants`).  Carries
    /// only the monotonic state-table ID inline.
    ///
    /// GC contract (Copilot R8): same per-wrapper trace fan-out as
    /// `TreeWalker` — filter `ObjectId` reached only when the
    /// wrapper is marked (avoids the filter-captures-wrapper leak
    /// cycle).  Sweep tail prunes `node_iterator_instances` + the
    /// shared `node_iterator_states_shared` map under the mutex.
    #[cfg(feature = "engine")]
    NodeIterator { iterator_id: u64 },
    /// `Selection` per-document singleton (Selection API §3, formerly
    /// WHATWG HTML §7.5.5).  Payload-free brand: the single per-document
    /// `Selection` state lives in `HostData::selection_state` (an
    /// `Option<SelectionState>`) and the canonical wrapper `ObjectId`
    /// lives in `HostData::selection_instance` (an `Option<ObjectId>`).
    /// Both are `Option<...>` rather than `HashMap<...>` because the
    /// M4-12 VM models exactly one Window+Document; promote to a map
    /// keyed by document `Entity` when multi-document arrives (D-15
    /// ShadowRoot / iframe).
    ///
    /// `window.getSelection()` and `document.getSelection()` both resolve
    /// to this singleton — they return the SAME `ObjectId` per spec
    /// `[SameObject]` semantics, which the host-data singleton slot
    /// gives for free (no per-call wrapper allocation).
    ///
    /// GC contract: trace fan-out (in `vm/gc/trace.rs`) marks the
    /// currently-selected `Range` wrapper at
    /// `HostData::range_instances[selection_state.current_range_id().bits()]`,
    /// keeping the registry entry alive across sweeps even when the user
    /// has dropped their JS Range reference.  If the wrapper has not yet
    /// been materialised (Selection set internally via
    /// `collapse`/`extend`/`setBaseAndExtent` without anyone calling
    /// `getRangeAt(0)`), trace fan-out is a no-op — `getRangeAt` builds
    /// a wrapper on demand, with the `RangeId` as the source of truth.
    /// Sweep tail clears `HostData::selection_instance` when this
    /// wrapper is collected.
    #[cfg(feature = "engine")]
    Selection,
    /// `File` instance (File API §4).  Subclass of [`Self::Blob`] via
    /// prototype chain (`File.prototype.[[Prototype]] = Blob.prototype`),
    /// NOT via ObjectKind inheritance — File has its own discriminator
    /// so brand checks distinguish `instanceof File` from a plain Blob.
    /// Payload-free; the per-instance state
    /// (`blob_id` reference to the backing Blob wrapper, `name` USVString,
    /// `last_modified` epoch ms) lives in `VmInner::file_data` keyed by
    /// this `ObjectId`.
    ///
    /// Storing a `blob_id` reference rather than copying bytes lets
    /// `File.size` / `.type` / `.slice()` / `.text()` / `.arrayBuffer()`
    /// reuse `BlobData` directly through the prototype chain — the
    /// inherited Blob accessors brand-check `ObjectKind::Blob`, so the
    /// File-side install adds an explicit Blob brand acceptance for
    /// File instances via the `require_blob_or_file_this` helper.
    ///
    /// GC contract: the trace step marks `blob_id` so the backing
    /// Blob survives as long as the File wrapper is reachable.  Sweep
    /// tail prunes `file_data` entries whose key was collected.
    #[cfg(feature = "engine")]
    File,
    /// `FileList` instance (File API §5).  Payload-free; the ordered
    /// list of [`Self::File`] `ObjectId`s lives in
    /// `VmInner::file_list_data` keyed by this `ObjectId`.
    ///
    /// **Indexed-property exotic NOT IMPLEMENTED**: `list[0]` returns
    /// `undefined`; callers must use `.item(i)`.  Deferred to slot
    /// `#11-filelist-indexed-exotic` (paired with the general
    /// `#11-events-modern-indexed-exotic` infrastructure also pending
    /// for `TouchList` / `DataTransferItemList` / `CSSRuleList`).
    ///
    /// GC contract: the trace step marks every `File` ObjectId in the
    /// state entry's `file_ids` Vec.  Sweep tail prunes
    /// `file_list_data` entries whose key was collected.
    #[cfg(feature = "engine")]
    FileList,
    /// `FileReader` instance (File API §6).  Payload-free; the
    /// readyState machine (state enum, result, error, target blob,
    /// abort sequence counter) lives in `VmInner::file_reader_data`
    /// keyed by this `ObjectId`.
    ///
    /// The reader is async: `readAs*()` methods set `state = LOADING`,
    /// fire `loadstart` synchronously, then enqueue a
    /// `PendingTask::FileRead` task that
    /// drains at the next eval boundary.  The task carries a snapshot
    /// of `abort_seq` at enqueue time; on drain, if the snapshot no
    /// longer matches the current state's `abort_seq`, the result is
    /// discarded (an `abort()` happened, OR a new `readAs*()` was
    /// invoked which incremented the counter).
    ///
    /// GC contract: the trace step marks `target_blob` (if any) and
    /// `result` (if an ArrayBuffer ObjectId).  Sweep tail prunes
    /// `file_reader_data` entries whose key was collected.
    #[cfg(feature = "engine")]
    FileReader,
    /// `Crypto` instance (WebCrypto §10) — accessed via `window.crypto`
    /// singleton.  Payload-free brand; the wrapper is stateless (every
    /// `getRandomValues` / `randomUUID` call routes to OS CSPRNG /
    /// `uuid::Uuid::new_v4()`).  Identity preserved via
    /// `VmInner::crypto_instance` so `window.crypto === window.crypto`.
    ///
    /// `new Crypto()` throws TypeError (`Illegal constructor`) per
    /// WebIDL §10 — only the constructor identifier is exposed.
    ///
    /// GC contract: payload-free.  The singleton is rooted via
    /// `VmInner::crypto_instance` (mark-roots step in
    /// `vm/gc/collect.rs`); the wrapper itself carries no inline
    /// `ObjectId`.  `Vm::unbind` clears `crypto_instance` so a
    /// retained reference cannot leak across rebinds.
    #[cfg(feature = "engine")]
    Crypto,
    /// `SubtleCrypto` instance (WebCrypto §14) — accessed via the
    /// `Crypto.prototype.subtle` accessor (per spec `[SameObject]`).
    /// Payload-free brand; current scope ships only
    /// `digest(algorithm, data)`.
    ///
    /// `new SubtleCrypto()` throws TypeError per WebIDL §14.
    ///
    /// GC contract: payload-free.  The singleton is rooted via
    /// `VmInner::subtle_crypto_instance`.  `Vm::unbind` clears the
    /// slot alongside `crypto_instance`.
    #[cfg(feature = "engine")]
    SubtleCrypto,
    /// `WebSocket` instance (WHATWG WebSockets §9.3).  Payload-free
    /// brand; the per-instance state (4-state `readyState` + URL +
    /// negotiated protocol/extensions + `bufferedAmount` +
    /// `binaryType` + broker `conn_id` + `on{open,message,error,close}`
    /// handler `ObjectId`s) lives in
    /// `HostData::websocket_states` keyed by this `ObjectId`.
    /// Reverse lookup from broker `conn_id` → `ObjectId` lives in
    /// `HostData::ws_conn_to_object` so the network drain (extended
    /// `VmInner::tick_network`) can route `WsEvent`s back to the
    /// instance.
    ///
    /// **Constructor is user-callable** (`new WebSocket(url,
    /// protocols?)`); the URL parse / scheme promotion / mixed-content
    /// gate / broker open all happen at construction time.  No
    /// "illegal constructor" stub (unlike `Crypto`).
    ///
    /// GC contract: trace fan-out marks the 4 `on*` handler
    /// `ObjectId`s held in the side-table entry; sweep tail prunes
    /// `websocket_states` + `ws_conn_to_object` for collected
    /// instances and emits a `RendererToNetwork::WebSocketClose` to
    /// the broker so the I/O thread terminates.  Side-tables are
    /// cleared on `Vm::unbind` (browsing-context scope per the
    /// Selection/Range precedent — broker handles die on unbind so
    /// state must too).
    #[cfg(feature = "engine")]
    WebSocket,
    /// `EventSource` instance (WHATWG HTML §9.2).  Payload-free
    /// brand; the per-instance state (3-state `readyState` + URL +
    /// `withCredentials` + sticky `lastEventId` + broker `conn_id` +
    /// `on{open,message,error}` handler `ObjectId`s + minimal
    /// `addEventListener` registry per `event_listeners:
    /// HashMap<String, Vec<ObjectId>>`) lives in
    /// `HostData::event_source_states` keyed by this `ObjectId`.
    /// Reverse lookup from broker `conn_id` → `ObjectId` lives in
    /// `HostData::sse_conn_to_object`.
    ///
    /// **Constructor is user-callable** (`new EventSource(url,
    /// init?)`).  Unlike `WebSocket`, EventSource ships a minimal
    /// `addEventListener(type, listener)` shim so spec-required
    /// named-event delivery does not silently drop user data; full
    /// `addEventListener` options (capture / once / signal) are
    /// deferred to `#11-realtime-event-listeners`.
    ///
    /// GC contract: trace fan-out marks the 3 `on*` handler
    /// `ObjectId`s plus every listener `ObjectId` across the
    /// `event_listeners` registry; sweep tail prunes
    /// `event_source_states` + `sse_conn_to_object` for collected
    /// instances and emits a `RendererToNetwork::EventSourceClose`
    /// to the broker.  Side-tables are cleared on `Vm::unbind`
    /// (browsing-context scope).
    #[cfg(feature = "engine")]
    EventSource,
    /// `CustomElementRegistry` instance (WHATWG HTML §4.13.4) — the
    /// singleton exposed as `window.customElements`. Payload-free brand;
    /// the actual registry (definitions / pending-upgrade queue) plus
    /// the reaction queue plus `whenDefined` pending resolvers live on
    /// `HostData` (`ce_registry` / `ce_reaction_queue` /
    /// `ce_when_defined`). Identity preserved via
    /// `VmInner::custom_element_registry_instance`.
    ///
    /// `new CustomElementRegistry()` throws TypeError ("Illegal
    /// constructor") per WebIDL §3.7 (interface object \[\[Construct\]\]) —
    /// HTML §4.13.4 declares `CustomElementRegistry` as having no
    /// constructor exposed.
    ///
    /// GC contract: payload-free. The singleton is rooted via
    /// `VmInner::custom_element_registry_instance` (mark-roots step).
    /// `Vm::unbind` clears the slot so the wrapper can be collected and
    /// re-allocated lazily after the next bind. Custom element
    /// constructor + `whenDefined` resolver `ObjectId`s live on
    /// `HostData` and are rooted separately via that path.
    #[cfg(feature = "engine")]
    CustomElementRegistry,
    /// `WebAssembly.Module` instance (WASM JS API §5.1).  Payload-free
    /// brand; the compiled `WasmModule` engine-indep handle lives in
    /// `VmInner::wasm_module_storage` keyed by this `ObjectId`.  Identity
    /// preserved by the storage entry; the wrapper is plain JS object,
    /// brand-checked by this variant on each static-method consumer.
    ///
    /// GC contract: payload-free in the trace step (the engine-indep
    /// `WasmModule` handle holds no `ObjectId` references — its source
    /// bytes are an `Arc<[u8]>` owned internally).  The sweep tail
    /// prunes `wasm_module_storage` entries whose key was collected.
    #[cfg(feature = "engine")]
    WasmModule,
    /// `WebAssembly.Instance` instance (WASM JS API §5.2).  Payload-free
    /// brand; the engine-indep `WasmInstance` handle plus the
    /// `module_id` / `exports_id` cache live in
    /// `VmInner::wasm_instance_storage` keyed by this `ObjectId`.
    ///
    /// GC contract: the trace step marks `module_id` (always set —
    /// keeps the parent Module alive while instances exist) and
    /// `exports_id` if `Some` (the lazily-allocated exports namespace
    /// per WASM JS API §5 `initialize an instance object` step 3; see
    /// the `WasmInstancePayload` struct in `crate::vm::wasm_payload`).
    /// The sweep tail prunes `wasm_instance_storage` entries whose
    /// key was collected.
    #[cfg(feature = "engine")]
    WasmInstance,
    /// `WebAssembly.Memory` instance (WASM JS API §5.3).  Payload-free
    /// brand; the engine-indep `WasmMemory` handle plus the cached
    /// `.buffer` ArrayBuffer ObjectId + the live `WasmMemoryView`
    /// backing that buffer live in `VmInner::wasm_memory_storage`.
    ///
    /// GC contract: the trace step marks `buffer_id` if `Some` (the
    /// JS-visible `ArrayBuffer` aliasing wasm linear memory — must
    /// survive while the Memory is reachable so the SameObject-style
    /// `mem.buffer === mem.buffer` ergonomics hold; IDL has no
    /// `[SameObject]` attribute on `Memory.buffer`, this is an
    /// elidex impl choice).  The stashed `view: Option<WasmMemoryView>`
    /// is not a JS ObjectId reference (holds wasmtime store + view
    /// flags Rc only), so no GC mark needed.  Sweep tail prunes
    /// `wasm_memory_storage` + the `wasm_backed_buffers` reverse-lookup
    /// entries whose ArrayBuffer key was collected.
    #[cfg(feature = "engine")]
    WasmMemory,
    /// `WebAssembly.Table` instance (WASM JS API §5.4).  Payload-free
    /// brand; the engine-indep `WasmTable` handle plus the cached
    /// `element_kind` (read once at ctor / exports-wrap time via
    /// F2 `WasmTable::element_kind()`) live in
    /// `VmInner::wasm_table_storage`.
    ///
    /// GC contract: no internal `ObjectId` references — element values
    /// (funcref / externref) flow through the engine-bridge handle's
    /// internal store; the JS side reaches them through `.get(idx)`
    /// which freshly wraps each access.  Sweep tail prunes
    /// `wasm_table_storage` entries whose key was collected.
    #[cfg(feature = "engine")]
    WasmTable,
    /// `WebAssembly.Global` instance (WASM JS API §5.5).  Payload-free
    /// brand; the engine-indep `WasmGlobal` handle lives in
    /// `VmInner::wasm_global_storage`.  `value_type` / `mutable` are
    /// read on demand via `WasmGlobal::value_type()` / `mutable()`
    /// per the plan-memo §2.2 sentinel discipline (no duplicate
    /// metadata fields).
    ///
    /// GC contract: no internal `ObjectId` references.  Sweep tail
    /// prunes `wasm_global_storage` entries whose key was collected.
    #[cfg(feature = "engine")]
    WasmGlobal,
    /// Exported wasm function exotic (WASM JS API §5.6).  The instance-
    /// owned `WasmFunc` engine-indep handle (carrying its `WasmStoreHandle`
    /// clone — `[[FunctionAddress]]` interpreted relative to the
    /// surrounding agent's associated store per §4.1) plus the cached
    /// per-param `Vec<WasmValueType>` for arg coerce + the parent
    /// `instance_id` for GC trace live in
    /// `VmInner::wasm_exported_func_storage` keyed by this `ObjectId`.
    ///
    /// Distinct from `Function` / `NativeFunction` because the call
    /// dispatch + lifetime semantics differ structurally: standard JS
    /// Function objects route through `vm_inner.functions` and the
    /// standard bytecode / Native call paths, whereas WasmExported
    /// routes through `WasmFunc::call(args, ScriptHostBinding)` with
    /// JS↔wasm value coerce per F1 marshalling boundary.
    ///
    /// GC contract: the trace step marks `instance_id` so the parent
    /// `WasmInstance` (and through it, the wasm module + linker state
    /// that keeps the exported function callable) survives as long as
    /// any exported function exists.  Sweep tail prunes
    /// `wasm_exported_func_storage` entries whose key was collected.
    #[cfg(feature = "engine")]
    WasmExportedFunction,
}

impl ObjectKind {
    /// Returns `true` if this object kind is callable (Function, NativeFunction,
    /// BoundFunction, PromiseResolver, one of the Promise combinator/finally
    /// step wrappers, or an async-driver continuation step).
    #[inline]
    pub fn is_callable(&self) -> bool {
        let extra_engine = {
            #[cfg(feature = "engine")]
            {
                matches!(
                    self,
                    Self::ReadableStreamStartStep { .. }
                        | Self::ReadableStreamPullStep { .. }
                        | Self::ReadableStreamCancelStep { .. }
                        | Self::WasmExportedFunction
                )
            }
            #[cfg(not(feature = "engine"))]
            {
                false
            }
        };
        extra_engine
            || matches!(
                self,
                Self::Function(_)
                    | Self::NativeFunction(_)
                    | Self::BoundFunction { .. }
                    | Self::PromiseResolver { .. }
                    | Self::PromiseCombinatorStep(_)
                    | Self::PromiseFinallyStep { .. }
                    | Self::AsyncDriverStep { .. }
            )
    }
}

/// `IsConstructor(value)` (ECMA-262 §7.2.4): true when the object has
/// a `[[Construct]]` internal slot.  Walks `BoundFunction` chains
/// up to [`crate::vm::MAX_BIND_CHAIN_DEPTH`] and inspects the
/// underlying target — a bound chain ending in an arrow function,
/// async function, generator function, or non-constructable native
/// must NOT report constructor (`do_new` in `ops.rs` does the
/// same unwrap before validating, so without this recursive check
/// the `IsConstructor` gate at `%TypedArray%.of` / `.from` could
/// be bypassed by
/// `Object.setPrototypeOf((()=>{}).bind(null), Uint8Array)` /
/// `Object.setPrototypeOf(async function(){}, Uint8Array)`).
///
/// For JS `Function` objects, this VM treats a function as
/// constructable iff it is not an arrow function
/// (`ThisMode::Lexical`) AND its compiled metadata is neither
/// `is_async` nor `is_generator`.  In other words, any non-arrow,
/// non-async, non-generator JS function is considered to have
/// `[[Construct]]` here — that includes classic `function`
/// declarations and `class` ctors alike (the latter is a
/// strict-mode `function` in our compiled representation).
///
/// Free function rather than a method on [`ObjectKind`] because
/// the chain walk needs `VmInner` access to look up each
/// `BoundFunction.target` by `ObjectId` and to fetch the compiled
/// function metadata for the async/generator check.
#[cfg(feature = "engine")]
pub(crate) fn is_constructor(vm: &super::VmInner, id: super::value::ObjectId) -> bool {
    let mut current = id;
    // `0..=MAX_BIND_CHAIN_DEPTH` (one more iteration than the
    // half-open range) so a chain of exactly `MAX_BIND_CHAIN_DEPTH`
    // `BoundFunction` wrappers can fully unwrap to its target —
    // matches `do_new`'s policy in `ops.rs` (allows MAX wrappers,
    // errors only on MAX+1).  An off-by-one half-open range
    // would exit before inspecting the final target and incorrectly
    // report non-constructor on otherwise-valid bound chains.
    for _ in 0..=crate::vm::MAX_BIND_CHAIN_DEPTH {
        match &vm.get_object(current).kind {
            ObjectKind::Function(fo) => {
                if fo.this_mode == crate::vm::value::ThisMode::Lexical {
                    return false;
                }
                let compiled = vm.get_compiled(fo.func_id);
                return !compiled.is_async && !compiled.is_generator;
            }
            ObjectKind::NativeFunction(nf) => return nf.shape.can_construct(),
            ObjectKind::BoundFunction { target, .. } => current = *target,
            _ => return false,
        }
    }
    // Chain length exceeded — reject defensively (matches
    // `do_new`'s `Maximum bind chain depth exceeded` RangeError
    // intent at the `IsConstructor`-precheck level).
    false
}
