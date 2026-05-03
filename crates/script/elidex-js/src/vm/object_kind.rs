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
    /// Wrapper object for BigInt primitives.
    BigIntWrapper(BigIntId),
    /// Wrapper object for Symbol primitives.
    SymbolWrapper(SymbolId),
    /// A Promise (ES2020 §25.6).  Holds the state machine (status + result)
    /// and reaction lists; reactions are drained via the microtask queue.
    Promise(PromiseState),
    /// Resolve/reject function bound to a specific Promise capability
    /// (ES2020 §25.6.1.3).  Created synchronously by `new Promise(executor)`
    /// and handed to the executor; settling is idempotent because subsequent
    /// invocations see `status != Pending` and become no-ops.
    PromiseResolver {
        /// The Promise this function settles.
        promise: ObjectId,
        /// Which side — `true` for reject, `false` for resolve.
        is_reject: bool,
    },
    /// Aggregator state for `Promise.all` / `Promise.allSettled` / `Promise.any`
    /// (ES2020 §25.6.4.1 / §25.6.4.2 / §25.6.4.3).  Shared across every
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
    /// An ES2020 §25.4 Generator object.  Created by a generator function
    /// call (the function body never runs on the initial call — instead,
    /// the Generator holds the initial suspended frame).  `.next()` /
    /// `.return()` / `.throw()` drive execution.
    Generator(GeneratorState),
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
    /// `ArrayBuffer` instance (ES2020 §24.1, minimal Phase 2 form).
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
    /// links).  Payload-free; the discriminator + filter parameters
    /// (root Entity, tag StringId, …) live out-of-band in
    /// `VmInner::live_collection_states` keyed by this object's
    /// `ObjectId`.
    ///
    /// Live semantics: every read (`length`, `item(i)`, indexed
    /// access, iterator) re-traverses the ECS from the stored root.
    /// Callers that need a snapshot can spread into an Array.
    ///
    /// GC contract: the side-table holds only `Entity` and
    /// `StringId` values (no `ObjectId`), so **no GC tracing is
    /// required**; the sweep tail prunes `live_collection_states`
    /// entries whose `ObjectId` key was collected.
    #[cfg(feature = "engine")]
    HtmlCollection,
    /// `NodeList` instance (WHATWG DOM §4.2.10.1).  An ordered
    /// collection of Node values — may be *live* (from
    /// `Node.prototype.childNodes` or `document.getElementsByName`)
    /// or *static* (from `querySelectorAll`, per §4.2.6).  Shares
    /// the `live_collection_states` side-table with `HtmlCollection`;
    /// the side-table's discriminator disambiguates between the two
    /// interfaces, and also records whether a `NodeList` is live or
    /// snapshot-backed.
    ///
    /// GC contract: identical to [`Self::HtmlCollection`] — the
    /// side-table carries no `ObjectId` references; pruning alongside
    /// HTMLCollection entries in the sweep tail is sufficient.  For
    /// static NodeLists, the `VmInner::live_collection_states` entry's
    /// snapshot state stores a `Vec<Entity>` whose entries are plain
    /// ECS keys (no ObjectId), so the `Vec` likewise needs no tracing.
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
    /// `TypedArray` instance view over an `ArrayBuffer` (ES2024 §23.2) —
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
    /// `DataView` instance view over an `ArrayBuffer` (ES2024 §25.3) —
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
    /// [`super::VmInner::url_search_params_states`] keyed by this
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
    /// stability) live out-of-band in
    /// [`super::VmInner::url_states`] keyed by this `ObjectId`.
    /// Same model as `URLSearchParams` — keeping the variant
    /// payload-free preserves per-variant size discipline.
    ///
    /// GC contract: the trace step marks the linked `URLSearchParams`
    /// `ObjectId` if any, so `let p = new URL("…").searchParams; …`
    /// keeps the URL alive while only the `searchParams` reference
    /// is held (the `URLSearchParams` mutator natives consult
    /// [`super::VmInner::usp_parent_url`] to write changes back to
    /// the URL's query).  Sweep tail prunes entries whose key
    /// `ObjectId` was collected.
    #[cfg(feature = "engine")]
    URL,
    /// `FormData` instance (WHATWG XHR §4.3).  Payload-free;
    /// the entry list lives out-of-band in
    /// [`super::VmInner::form_data_states`] keyed by this
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
    /// [`super::VmInner::readable_stream_states`] keyed by this
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
    /// [`super::VmInner::readable_stream_reader_states`].
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

/// `IsConstructor(value)` (ES §7.2.4): true when the object has
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
            ObjectKind::NativeFunction(nf) => return nf.constructable,
            ObjectKind::BoundFunction { target, .. } => current = *target,
            _ => return false,
        }
    }
    // Chain length exceeded — reject defensively (matches
    // `do_new`'s `Maximum bind chain depth exceeded` RangeError
    // intent at the `IsConstructor`-precheck level).
    false
}
