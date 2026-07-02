//! `MutationObserver` interface (WHATWG DOM §4.3.1) — VM thin
//! binding to the engine-independent
//! [`elidex_api_observers::mutation::MutationObserverRegistry`].
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file contains only the
//! engine-bound responsibilities: prototype install, brand check,
//! `MutationObserverInit` dictionary parsing (JS → Rust marshalling),
//! and one-line dispatch into the registry helpers.  The actual
//! observation algorithm — target list management, subtree
//! ancestry checks, attribute-filter matching, record building —
//! lives in [`elidex_api_observers::mutation`].
//!
//! ## State storage
//!
//! Observer state is split between two side tables:
//!
//! - [`super::super::host_data::HostData::mutation_observers`] — the
//!   [`elidex_api_observers::mutation::MutationObserverRegistry`]
//!   that owns target lists, init options, and pending records.
//! - [`super::super::host_data::HostData::mutation_observer_bindings`]
//!   — `HashMap<u64, ObserverBinding>` from observer ID to the
//!   `(callback, instance)` JS-identity pair.  Both `ObjectId`s in
//!   each binding are rooted by the keepalive seam's predicate
//!   ([`super::super::gc::keepalive::keepalive_survivors`], S5-3c — ≥1
//!   active observation OR ≥1 pending undelivered record, the full
//!   statement lives at the seam) so they survive GC while the observer
//!   observes ≥1 target or still has a record to deliver, and the
//!   binding row is sweep-pruned once collectible.
//!
//! [`super::super::value::ObjectKind::Observer`] with
//! [`super::super::value::ObserverKind::Mutation`] carries the
//! observer ID inline (`observer_id: u64`); the JS object itself
//! has no other own state.
//!
//! ## `native_*` docstring convention
//!
//! Per-method native `fn`s here (and in `resize_observer.rs` /
//! `intersection_observer.rs`) deliberately rely on the constructor's
//! docstring + the module-level spec citation as their primary
//! documentation.  Brand-checked function names
//! (`native_mutation_observer_observe` etc.) are unique enough to
//! disambiguate without a per-fn docstring; the spec section is
//! cited inline at the call site where it actually matters.
//!
//! ## Lifecycle preconditions
//!
//! - **Constructor** (`new MutationObserver(cb)`) requires
//!   [`super::super::Vm::install_host_data`] to have been called.
//!   It does *not* require a bound `EcsDom`/`SessionCore`, because
//!   callback / instance bookkeeping lives entirely on the
//!   `HostData`-owned side tables (no DOM pointer access during
//!   construction).  Pre-`install_host_data` calls return a
//!   `TypeError` rather than panicking via
//!   [`super::super::native_context::NativeContext::host`].
//! - **Method natives** (`observe` / `disconnect` / `takeRecords`)
//!   check `ctx.host_if_bound()` first and return a safe no-op
//!   (empty array / `undefined`) so a retained `mo` reference
//!   survives a [`super::super::Vm::unbind`] boundary.

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectKind, PropertyKey, PropertyStorage, PropertyValue,
    VmError,
};
use super::super::{NativeFn, VmInner};

use elidex_api_observers::mutation::MutationObserverId;

impl VmInner {
    /// Allocate `MutationObserver.prototype` chained to
    /// `Object.prototype`, install its three method natives, and
    /// expose the `MutationObserver` constructor on `globalThis`.
    ///
    /// Called from `register_globals()` after `register_prototypes`
    /// (which populates `object_prototype`).
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — would mean
    /// `register_prototypes` was skipped or the wrong order.
    pub(in crate::vm) fn register_mutation_observer_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_mutation_observer_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });

        let wk = &self.well_known;
        let entries = [
            (wk.observe, native_mutation_observer_observe as NativeFn),
            (
                wk.disconnect,
                native_mutation_observer_disconnect as NativeFn,
            ),
            (
                wk.take_records,
                native_mutation_observer_take_records as NativeFn,
            ),
        ];
        for (name_sid, func) in entries {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }
        self.mutation_observer_prototype = Some(proto_id);

        let ctor = self.create_constructor_only_function(
            "MutationObserver",
            native_mutation_observer_constructor,
        );
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            shape::PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor)),
            shape::PropertyAttrs::METHOD,
        );
        let name_sid = self.well_known.mutation_observer_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

/// Recover the `MutationObserverId` from `this`, returning a TypeError
/// if `this` is not a `MutationObserver` instance.
fn require_mutation_observer_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<MutationObserverId, VmError> {
    let raw = super::observer_common::require_observer_receiver(
        ctx,
        this,
        super::super::value::ObserverKind::Mutation,
        method,
    )?;
    Ok(MutationObserverId::from_raw(raw))
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

/// `new MutationObserver(callback)` (WHATWG DOM §4.3.1).
fn native_mutation_observer_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    // Validate the callback up front — must be callable per WebIDL
    // §3.10.2 (MutationCallback).
    let callback_id = match args.first().copied() {
        Some(JsValue::Object(id)) if ctx.vm.get_object(id).kind.is_callable() => id,
        _ => {
            return Err(VmError::type_error(
                "Failed to construct 'MutationObserver': parameter 1 is not of type 'Function'.",
            ));
        }
    };
    let JsValue::Object(this_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };

    // Pre-`install_host_data` guard: callback / instance bookkeeping
    // lives on `HostData` side tables, so without it there's nowhere
    // to store the JS callback.  A bare `ctx.host()` would panic
    // (`HostData accessed while unbound` is the bound-state assert,
    // but `host()` itself panics earlier when `host_data` is `None`).
    if ctx.host_opt().is_none() {
        return Err(VmError::type_error(
            "Failed to construct 'MutationObserver': host environment is not initialised",
        ));
    }
    // Allocate an observer ID in the registry, then promote the
    // pre-allocated Ordinary instance to `MutationObserver` so the
    // `new.target.prototype` chain installed by `do_new` is preserved
    // (URL/URLSearchParams precedent — PR5a2 R7.2/R7.3 lesson).
    let observer_id = ctx.host().mutation_observers.register().raw();
    ctx.vm.get_object_mut(this_id).kind = ObjectKind::Observer {
        kind: super::super::value::ObserverKind::Mutation,
        observer_id,
    };
    ctx.host().mutation_observer_bindings.insert(
        observer_id,
        super::observer_common::ObserverBinding {
            callback: callback_id,
            instance: this_id,
        },
    );

    Ok(JsValue::Object(this_id))
}

// ---------------------------------------------------------------------------
// Natives
// ---------------------------------------------------------------------------

fn native_mutation_observer_observe(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_mutation_observer_receiver(ctx, this, "observe")?;
    // Post-unbind no-op MUST come before any DOM access:
    // `require_target_node` → `node_proto::require_node_arg` reaches
    // `ctx.host().dom()`, which asserts on unbound state.  Honouring
    // the documented post-unbind tolerance contract here lets a
    // retained `mo.observe(...)` call survive an `unbind()` boundary.
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    // WebIDL signature: `observe(Node target, MutationObserverInit options)` —
    // both arguments required.  Match Chrome/Firefox arg-count error
    // message before falling through to the per-argument coercion errors.
    if args.len() < 2 {
        return Err(VmError::type_error(format!(
            "Failed to execute 'observe' on 'MutationObserver': 2 arguments required, \
             but only {} present.",
            args.len()
        )));
    }
    let target = require_target_node(ctx, args.first().copied(), "observe")?;
    let init = parse_mutation_observer_init(ctx, args.get(1).copied())?;
    if !init.child_list && !init.attributes && !init.character_data {
        return Err(VmError::type_error(
            "Failed to execute 'observe' on 'MutationObserver': The options object must \
             set at least one of 'attributes', 'characterData', or 'childList' to true.",
        ));
    }
    let (dom, observers) = ctx.host().split_dom_mut_and_observers();
    observers.observe(dom, id, target, init);
    Ok(JsValue::Undefined)
}

fn native_mutation_observer_disconnect(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_mutation_observer_receiver(ctx, this, "disconnect")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Undefined);
    }
    let (dom, observers) = ctx.host().split_dom_mut_and_observers();
    observers.disconnect(dom, id);
    Ok(JsValue::Undefined)
}

fn native_mutation_observer_take_records(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_mutation_observer_receiver(ctx, this, "takeRecords")?;
    if ctx.host_if_bound().is_none() {
        return Ok(JsValue::Object(ctx.vm.create_array_object(Vec::new())));
    }
    let records = ctx.host().mutation_observers.take_records(id);
    Ok(build_mutation_records_array(ctx.vm, &records))
}

/// Marshal a slice of [`elidex_api_observers::mutation::MutationRecord`]s
/// into a JS Array of MutationRecord objects (WHATWG DOM §4.3.3).
///
/// Delegates to the shared
/// [`super::observer_common::build_marshalled_array`] for the
/// outer-array + temp-root + element-push discipline (same shape
/// used by Resize/Intersection entry array builders); per-record
/// marshalling stays in [`mutation_record_to_js`].
pub(super) fn build_mutation_records_array(
    vm: &mut VmInner,
    records: &[elidex_api_observers::mutation::MutationRecord],
) -> JsValue {
    super::observer_common::build_marshalled_array(vm, records, mutation_record_to_js)
}

/// Marshal a single [`elidex_api_observers::mutation::MutationRecord`]
/// to a JS Object with WHATWG DOM §4.3.3 shape.
///
/// `addedNodes` / `removedNodes` are rooted (via `push_temp_root`)
/// across the wrapper allocation so a GC triggered by `alloc_object`
/// cannot collect the just-allocated arrays before the wrapper
/// reaches them through `define_shaped_property` (Lesson R8).
fn mutation_record_to_js(
    vm: &mut VmInner,
    record: &elidex_api_observers::mutation::MutationRecord,
) -> JsValue {
    use super::super::shape::PropertyAttrs;

    let added = build_node_array(vm, &record.added_nodes);
    let mut added_guard = vm.push_temp_root(added);
    let removed = build_node_array(&mut added_guard, &record.removed_nodes);
    let mut removed_guard = added_guard.push_temp_root(removed);

    let target_val = JsValue::Object(removed_guard.create_element_wrapper(record.target));
    let prev_sibling = record.previous_sibling.map_or(JsValue::Null, |e| {
        JsValue::Object(removed_guard.create_element_wrapper(e))
    });
    let next_sibling = record.next_sibling.map_or(JsValue::Null, |e| {
        JsValue::Object(removed_guard.create_element_wrapper(e))
    });
    let attribute_name = record
        .attribute_name
        .as_deref()
        .map_or(JsValue::Null, |name| {
            JsValue::String(removed_guard.strings.intern(name))
        });
    let old_value = record.old_value.as_deref().map_or(JsValue::Null, |v| {
        JsValue::String(removed_guard.strings.intern(v))
    });
    // The 3 spec values for `MutationRecord.type` are already
    // pre-interned via `well_known.{child_list,attributes,character_data}`
    // (StringPool dedupes by literal) — match the registry's
    // `mutation_type` String against them so each delivery skips
    // the `intern(...)` round-trip on the dominant childList /
    // attribute hot path.
    let wk = &removed_guard.well_known;
    let type_sid = match record.mutation_type.as_str() {
        "childList" => wk.child_list,
        "attributes" => wk.attributes,
        "characterData" => wk.character_data,
        // Forward-compat: an unknown variant from
        // `MutationObserverRegistry::notify` is a contract violation,
        // but fall back to a fresh intern rather than panic so the
        // record still surfaces with its actual type string.
        other => removed_guard.strings.intern(other),
    };
    let type_val = JsValue::String(type_sid);

    let object_proto = removed_guard.object_prototype;
    let record_obj = removed_guard.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(super::super::shape::ROOT_SHAPE),
        prototype: object_proto,
        extensible: true,
    });

    let wk_type = removed_guard.well_known.event_type;
    let wk_target = removed_guard.well_known.target;
    let wk_added = removed_guard.well_known.added_nodes;
    let wk_removed = removed_guard.well_known.removed_nodes;
    let wk_prev = removed_guard.well_known.previous_sibling;
    let wk_next = removed_guard.well_known.next_sibling;
    let wk_attr_name = removed_guard.well_known.attribute_name;
    let wk_old_value = removed_guard.well_known.old_value;
    // WHATWG DOM §4.3.3: every `MutationRecord` member is a
    // `readonly attribute`, so install with `WEBIDL_RO` (¬W, E, C).
    // This VM's property-set path throws `TypeError("Cannot assign
    // to read only property")` on any assignment regardless of
    // strict-mode (browsers silently ignore in non-strict; the
    // strict-mode parity gap is a VM-wide concern, not specific to
    // MutationRecord).  Matches the event-object property
    // installation pattern.
    for (key_sid, value) in [
        (wk_type, type_val),
        (wk_target, target_val),
        (wk_added, added),
        (wk_removed, removed),
        (wk_prev, prev_sibling),
        (wk_next, next_sibling),
        (wk_attr_name, attribute_name),
        (wk_old_value, old_value),
    ] {
        removed_guard.define_shaped_property(
            record_obj,
            PropertyKey::String(key_sid),
            PropertyValue::Data(value),
            PropertyAttrs::WEBIDL_RO,
        );
    }

    drop(removed_guard);
    drop(added_guard);
    JsValue::Object(record_obj)
}

/// Build a JS Array of node wrappers for the given entity slice.
/// `addedNodes` / `removedNodes` may carry non-Element nodes (Text /
/// Comment / ProcessingInstruction / CDATA) per WHATWG DOM §4.3.3,
/// and the underlying [`super::elements::create_element_wrapper`]
/// dispatches to the right prototype per
/// [`super::super::host_data::HostData::prototype_kind_for`]
/// (Element / Text / CharacterData / Node) — the helper name is
/// engine-historic, the behaviour covers every node kind.  Identity
/// caching applies, so `addedNodes[0] === document.body` for matched
/// targets.
fn build_node_array(vm: &mut VmInner, entities: &[elidex_ecs::Entity]) -> JsValue {
    let mut elements = Vec::with_capacity(entities.len());
    for &e in entities {
        let id = vm.create_element_wrapper(e);
        elements.push(JsValue::Object(id));
    }
    JsValue::Object(vm.create_array_object(elements))
}

// ---------------------------------------------------------------------------
// Embedder API — `Vm::deliver_mutation_records` core logic
// ---------------------------------------------------------------------------

impl VmInner {
    /// See [`super::super::Vm::deliver_mutation_records`] for the
    /// documented semantics.  Implementation lives here next to
    /// `mutation_record_to_js`.
    pub(crate) fn deliver_mutation_records(
        &mut self,
        records: &[elidex_script_session::MutationRecord],
    ) {
        // Embedder / `HostDriver`-trait entry: the host pushes externally-built
        // records (e.g. layout-derived) AND drives the "notify mutation
        // observers" checkpoint in the same call — the embedder's call site *is*
        // the checkpoint (this mirrors how the shell drives the boa runtime's
        // delivery post-layout). Internal JS mutations instead use
        // `queue_mutation_record` (deferred to the engine's own §4.3 microtask).
        // Both share `notify_one` + `deliver_pending_mutation_records`; they
        // differ only in *when* the callbacks run (embedder-chosen vs microtask).
        if !self
            .host_data
            .as_deref()
            .is_some_and(super::super::host_data::HostData::is_bound)
        {
            return;
        }
        for record in records {
            self.notify_one(record);
        }
        self.deliver_pending_mutation_records();
    }

    /// "Queue a mutation record" + "queue a mutation observer microtask"
    /// (WHATWG DOM §4.3.2 steps 4–5) for a single already-applied mutation.
    /// Synchronously enqueues the record into each interested observer's queue
    /// (the ancestor walk), and — if any observer was interested — schedules the
    /// `NotifyMutationObservers` microtask that invokes the callbacks. The flag
    /// coalesces one microtask per checkpoint (shared with slotchange).
    pub(crate) fn queue_mutation_record(&mut self, record: &elidex_script_session::MutationRecord) {
        // Silent no-op post-unbind so a stray late record does not panic via
        // `host_data` access.
        if !self
            .host_data
            .as_deref()
            .is_some_and(super::super::host_data::HostData::is_bound)
        {
            return;
        }
        if self.notify_one(record) && !self.mutation_observer_microtask_queued {
            self.mutation_observer_microtask_queued = true;
            self.microtask_queue
                .push_back(super::super::natives_promise::Microtask::NotifyMutationObservers);
        }
    }

    /// Deliver all pending observer records to their JS callbacks (WHATWG DOM
    /// §4.3 "notify mutation observers" steps 2–6). Called from the
    /// `NotifyMutationObservers` microtask, alongside the slotchange step 7.
    pub(crate) fn deliver_pending_mutation_records(&mut self) {
        // Silent no-op post-unbind (mirrors the producer guard).
        if !self
            .host_data
            .as_deref()
            .is_some_and(super::super::host_data::HostData::is_bound)
        {
            return;
        }
        // Take the pending mutation observers as the §4.3 notifySet (step 2 clone
        // + step 3 empty). Draining up front means re-entrant `mo.observe` /
        // `mo.disconnect` / queued records from callbacks land in the next
        // microtask's set, not this one. Keyed on the pending set (not record-queue
        // non-emptiness) so an observer the page drained via `takeRecords()` before
        // this microtask is still cleared of transients in step 6.3 below.
        let host = self
            .host_data
            .as_deref_mut()
            .expect("deliver_pending_mutation_records: HostData required when bound");
        let observer_ids: Vec<u64> = host
            .mutation_observers
            .take_pending_observers()
            .into_iter()
            .map(elidex_api_observers::mutation::MutationObserverId::raw)
            .collect();

        super::observer_common::deliver_to_observer_callbacks(
            self,
            &observer_ids,
            // `lookup`: resolve the binding for the Phase-1 batch root.
            // Every pending observer in `notifySet` has a binding (the
            // constructor inserts it and only a GC-collection prunes it,
            // which the batch root now prevents mid-batch), so none is
            // dropped from the batch — preserving delivery order + count.
            |vm, observer_id| {
                vm.host_data
                    .as_deref()
                    .and_then(|hd| hd.mutation_observer_bindings.get(&observer_id).copied())
            },
            // `prepare`: registry work only, NO JS allocation.
            |vm, observer_id| {
                let mo_id =
                    elidex_api_observers::mutation::MutationObserverId::from_raw(observer_id);
                let host = vm
                    .host_data
                    .as_deref_mut()
                    .expect("deliver_pending_mutation_records: HostData required when bound");
                let records = host.mutation_observers.take_records(mo_id);
                // §4.3 "notify mutation observers" step 6.3: remove this
                // observer's transient registered observers. Spec runs 6.3
                // *interleaved* — per mo, after emptying its record queue (6.2)
                // and before invoking its callback (6.4) — so a transient created
                // by an earlier observer's callback for a later observer still in
                // `notifySet` is cleared at that later observer's turn. Doing it
                // here (per mo, before the `records.is_empty()` short-circuit)
                // rather than as one upfront set pass preserves that ordering and
                // lets a transient created by *this* observer's own callback
                // survive to the next microtask.
                //
                // ORDER is spec-load-bearing: take_records (6.2) →
                // clear_transient (6.3) → is_empty short-circuit. `prepare`
                // does NO JS allocation, and the observer's `(instance,
                // callback)` binding is already batch-rooted in Phase 1
                // (see `deliver_to_observer_callbacks`), so draining the
                // pending-record queue here — even though it releases the
                // sole keepalive anchor once the target has despawned —
                // cannot leave the binding collectible during the
                // subsequent GC-capable record-array build.
                elidex_api_observers::mutation::clear_transient_observers(host.dom(), mo_id);
                if records.is_empty() {
                    return None;
                }
                Some(records)
            },
            |vm, records| build_mutation_records_array(vm, &records),
        );
    }

    /// Wrapper around `MutationObserverRegistry::notify`, which reads
    /// each node's `MutationObservedBy` component while walking the
    /// record target's inclusive ancestors (WHATWG DOM §4.3.2). The
    /// shared `&EcsDom` lets the registry perform the ancestor walk via
    /// `EcsDom::get_parent` directly. Returns whether any observer was
    /// interested (so the caller can decide to schedule the microtask).
    ///
    /// This is the single eager per-record notify chokepoint — both the
    /// embedder `deliver_mutation_records` batch loop and the internal
    /// `queue_mutation_record` route through it. It runs synchronously at the
    /// mutation (in JS-execution order), so the WHATWG DOM §4.2.3 "remove"
    /// step-15 transient-observer creation hooked here happens "during remove",
    /// before the removed subtree can be re-mutated — correct by construction.
    fn notify_one(&mut self, record: &elidex_script_session::MutationRecord) -> bool {
        use elidex_script_session::MutationKind;

        let host = self
            .host_data
            .as_deref_mut()
            .expect("notify_one: HostData required when bound");
        // §4.2.3 "remove" step 15: append transient registered observers to the
        // removed nodes (so an ancestor's `subtree:true` observer keeps seeing
        // mutations in the now-detached subtree until the next microtask). Keyed
        // on the removal record — `record.target` is the parent the nodes left
        // (for a move-adopt, the source-removal record's target is the *old*
        // parent, §4.5), `record.removed_nodes` the detached set (covers
        // coalesced replace / replace-all removals too). Not gated by
        // `suppressObservers` (step 15 ≠ step 16).
        if record.kind == MutationKind::ChildList && !record.removed_nodes.is_empty() {
            let (dom, observers) = host.split_dom_mut_and_observers();
            observers.add_transient_observers(dom, record.target, &record.removed_nodes);
        }
        let (dom, observers) = host.split_dom_and_observers();
        observers.notify(dom, record)
    }
}

// ---------------------------------------------------------------------------
// MutationObserverInit dictionary parsing + target extraction
// ---------------------------------------------------------------------------

fn require_target_node(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
    method: &'static str,
) -> Result<elidex_ecs::Entity, VmError> {
    super::node_proto::require_node_arg_required(ctx, arg, "MutationObserver", method)
}

/// Parse the `MutationObserverInit` dictionary (WHATWG DOM §4.3 +
/// §4.3.2 spec corrections) — JS object → Rust struct marshalling.
/// Returns the default-constructed init when `arg` is `undefined` /
/// missing (which subsequently fails the
/// "must set at least one" check at the call site).
fn parse_mutation_observer_init(
    ctx: &mut NativeContext<'_>,
    arg: Option<JsValue>,
) -> Result<elidex_api_observers::mutation::MutationObserverInit, VmError> {
    use elidex_api_observers::mutation::MutationObserverInit;

    let mut init = MutationObserverInit::default();
    // WebIDL §3.10.7 dictionary conversion: `undefined` and `null`
    // both yield the default-init dictionary (empty object branch).
    // The subsequent "at least one of childList / attributes /
    // characterData" check at the call site then surfaces a
    // TypeError, matching Chrome/Firefox.
    let value = match arg {
        None | Some(JsValue::Undefined | JsValue::Null) => return Ok(init),
        Some(v) => v,
    };
    let JsValue::Object(opts_id) = value else {
        // Primitives (number / boolean / string / Symbol / BigInt)
        // would, per spec, be ToObject-coerced to a wrapper before
        // reading fields — every field then absent so default-init
        // applies.  Phase 2 simplification: reject with TypeError;
        // tracked at `#11-mutation-observer-extras` for full
        // primitive→ToObject parity (low real-world hit rate).
        return Err(VmError::type_error(
            "Failed to execute 'observe' on 'MutationObserver': options is not an object",
        ));
    };

    let wk_child_list = ctx.vm.well_known.child_list;
    let wk_attributes = ctx.vm.well_known.attributes;
    let wk_character_data = ctx.vm.well_known.character_data;
    let wk_subtree = ctx.vm.well_known.subtree;
    let wk_attribute_old_value = ctx.vm.well_known.attribute_old_value;
    let wk_character_data_old_value = ctx.vm.well_known.character_data_old_value;
    let wk_attribute_filter = ctx.vm.well_known.attribute_filter;
    let wk_length = ctx.vm.well_known.length;

    let mut attributes_explicit = false;
    let mut character_data_explicit = false;

    if let Some(v) = super::observer_common::read_dict_field(ctx, opts_id, wk_child_list)? {
        init.child_list = ctx.to_boolean(v);
    }
    if let Some(v) = super::observer_common::read_dict_field(ctx, opts_id, wk_attributes)? {
        init.attributes = ctx.to_boolean(v);
        attributes_explicit = true;
    }
    if let Some(v) = super::observer_common::read_dict_field(ctx, opts_id, wk_character_data)? {
        init.character_data = ctx.to_boolean(v);
        character_data_explicit = true;
    }
    if let Some(v) = super::observer_common::read_dict_field(ctx, opts_id, wk_subtree)? {
        init.subtree = ctx.to_boolean(v);
    }
    if let Some(v) = super::observer_common::read_dict_field(ctx, opts_id, wk_attribute_old_value)?
    {
        init.attribute_old_value = ctx.to_boolean(v);
    }
    if let Some(v) =
        super::observer_common::read_dict_field(ctx, opts_id, wk_character_data_old_value)?
    {
        init.character_data_old_value = ctx.to_boolean(v);
    }
    if let Some(value) = super::observer_common::read_dict_field(ctx, opts_id, wk_attribute_filter)?
    {
        init.attribute_filter = Some(parse_attribute_filter(ctx, value, wk_length)?);
    }

    // WHATWG DOM §4.3.2 step 3: if `attributeOldValue` or
    // `attributeFilter` is given but `attributes` is not, set
    // `attributes` to true.
    if !attributes_explicit && (init.attribute_old_value || init.attribute_filter.is_some()) {
        init.attributes = true;
    }
    // §4.3.2 step 4: if `characterDataOldValue` is given but
    // `characterData` is not, set `characterData` to true.
    if !character_data_explicit && init.character_data_old_value {
        init.character_data = true;
    }
    // §4.3.2 step 6: `attributeOldValue: true` requires `attributes`
    // to be true (or absent — covered by step 3 above).  Fires only
    // when `attributes` was explicitly set to false.
    if init.attribute_old_value && !init.attributes {
        return Err(VmError::type_error(
            "Failed to execute 'observe' on 'MutationObserver': The options object \
             may only set 'attributeOldValue' to true when 'attributes' is true \
             or not present.",
        ));
    }
    // §4.3.2 step 7: `attributeFilter` requires `attributes` to be
    // true (or absent).
    if init.attribute_filter.is_some() && !init.attributes {
        return Err(VmError::type_error(
            "Failed to execute 'observe' on 'MutationObserver': The options object \
             may only set 'attributeFilter' when 'attributes' is true or not present.",
        ));
    }
    // §4.3.2 step 8: `characterDataOldValue: true` requires
    // `characterData` to be true (or absent).
    if init.character_data_old_value && !init.character_data {
        return Err(VmError::type_error(
            "Failed to execute 'observe' on 'MutationObserver': The options object \
             may only set 'characterDataOldValue' to true when 'characterData' is \
             true or not present.",
        ));
    }

    Ok(init)
}

/// Cap on the number of `attributeFilter` items accepted; matches
/// the IntersectionObserver `threshold` cap.  A hostile `length`
/// (e.g. `4_000_000_000`) would otherwise loop billions of times,
/// interning each numeric index into the permanent `StringPool` —
/// unbounded CPU + memory growth on a single observe() call.
const MAX_ATTRIBUTE_FILTER_LEN: f64 = 65_536.0;

/// Coerce a JS `attributeFilter` value into a `Vec<String>` per
/// WebIDL §3.10.20 sequence conversion.  Phase 2 simplification —
/// accepts any Object with a numeric `length` (covers Array,
/// NodeList, arguments-shaped objects); non-Object values raise
/// TypeError so misconfigured observers fail loudly.  Full
/// `Symbol.iterator`-protocol support is tracked at
/// `#11-mutation-observer-extras`.
///
/// `length` conversion uses ToLength (ECMA-262 §7.1.21), not ToUint32:
/// negative / NaN clamps to 0; oversize values RangeError (avoids
/// the prior `length: -1` → ToUint32 wrap → 4 GiB `Vec::with_capacity`
/// abort).  No pre-allocation: the index-loop pushes incrementally
/// so even a giant `length` only allocates as items are read.
fn parse_attribute_filter(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    wk_length: super::super::value::StringId,
) -> Result<Vec<String>, VmError> {
    let JsValue::Object(arr_id) = value else {
        return Err(VmError::type_error(
            "Failed to execute 'observe' on 'MutationObserver': \
             'attributeFilter' is not iterable",
        ));
    };
    let len_val = ctx.get_property_value(arr_id, PropertyKey::String(wk_length))?;
    let len_f = ctx.to_number(len_val)?;
    let len_clamped = if len_f.is_nan() || len_f <= 0.0 {
        0.0
    } else {
        len_f.trunc()
    };
    if len_clamped > MAX_ATTRIBUTE_FILTER_LEN {
        return Err(VmError::range_error(
            "Failed to execute 'observe' on 'MutationObserver': \
             'attributeFilter' length exceeds the supported maximum",
        ));
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let len = len_clamped as u32;
    let mut filter = Vec::new();
    for i in 0..len {
        // Numeric key path through `get_element` — the Array /
        // arguments / TypedArray fast paths in `ops_element` short-
        // circuit on a `Number` key without ever stringifying it.
        // The previous `intern(&i.to_string())` flow leaked an
        // entry into the permanent `StringPool` per index (up to
        // 65k under the worst-case cap); the same convention used
        // by `typed_array_ctor` / `natives_json` avoids that.
        let item = ctx
            .vm
            .get_element(JsValue::Object(arr_id), JsValue::Number(f64::from(i)))?;
        let sid = ctx.to_string_val(item)?;
        filter.push(ctx.vm.strings.get_utf8(sid));
    }
    Ok(filter)
}
