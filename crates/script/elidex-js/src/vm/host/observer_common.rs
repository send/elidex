//! Observer-family shared infrastructure.
//!
//! WHATWG/W3C ships three "callback-driven, per-target observation"
//! interfaces over the DOM — `MutationObserver`, `ResizeObserver`,
//! `IntersectionObserver` — and their JS-side thin bindings have a
//! large overlapping shape:
//!
//! - per-observer state lives on `HostData` keyed by a monotonic
//!   `observer_id: u64` (CLAUDE.md side-store exception (a) — per-VM
//!   identity handle: callback `ObjectId` + instance `ObjectId`),
//! - delivery iterates the observer ids with work, looks up the
//!   `(callback, instance)` pair, temp-roots both across the JS
//!   callback invocation, and ends with a microtask checkpoint,
//! - records / entries are marshalled into a JS Array of plain
//!   shaped Objects with `WEBIDL_RO` member properties.
//!
//! This module is the single canonical home for that shape:
//!
//! - [`ObserverBinding`] — the `(callback, instance)` pair, owned by
//!   `HostData` as a single map entry per observer (replaces the
//!   prior per-kind `*_callbacks` + `*_instances` map pair, halving
//!   the GC-root chain count and the "did I update both?" foot-gun).
//! - [`build_marshalled_array`] — generic `&[T] -> JS Array`
//!   marshaller with the standard temp-root discipline (outer array
//!   rooted across per-element marshal calls so a GC triggered by an
//!   element allocation does not collapse the partially-filled
//!   array).
//! - [`deliver_to_observer_callbacks`] — the shared per-observer
//!   delivery loop: resolve + **batch-root** every observer's
//!   `(instance, callback)` this delivery will touch, then per observer
//!   run the registry `prepare`, marshal the records/entries Array with
//!   `build`, invoke the JS callback with `(records, observer)`, report
//!   exceptions via `eprintln!`, and drain microtasks at end.  Each
//!   per-kind `deliver_*` method on `VmInner` is now a thin shell that
//!   supplies (a) a `lookup` closure resolving the binding for the batch
//!   root, (b) a `prepare` closure doing registry work ONLY (record
//!   drain / observation lookup — no JS allocation) returning the opaque
//!   owned record data, and (c) a `build` closure that marshals that
//!   owned data into the JS Array once the whole batch is rooted.
//! - [`read_dict_field`] — `undefined → None`, anything else →
//!   `Some(value)` WebIDL §3.10.7 dictionary-member reader, hoisted
//!   from the original `mutation_observer.rs` site.  Used by all
//!   three observer init-dict parsers.
//!
//! ## Why a shared module and not one-off duplicates
//!
//! CLAUDE.md "One issue, one way": three near-identical copies of the
//! same outer-loop + temp-root + call + drain pattern is the textbook
//! strangler middle-state.  Converging to a single canonical form
//! eliminates the "did this PR remember to update all three?"
//! decision tax.

#![cfg(feature = "engine")]

use super::super::value::{
    JsValue, NativeContext, ObjectId, ObjectKind, ObserverKind, PropertyKey, StringId, VmError,
};
use super::super::VmInner;

// ---------------------------------------------------------------------------
// Generic observer brand check
// ---------------------------------------------------------------------------

/// Brand check for the unified `ObjectKind::Observer { kind, observer_id }`
/// variant — parameterised on the expected [`ObserverKind`] so the three
/// observer surfaces (`MutationObserver` / `ResizeObserver` /
/// `IntersectionObserver`) share one implementation instead of three
/// near-identical copies.
///
/// Returns the inline `observer_id` on success.  The caller wraps it in
/// the kind-specific newtype (`MutationObserverId::from_raw(_)` etc.) at
/// the call site — the raw `u64` is the canonical cross-kind handle.
///
/// Error wording follows WebIDL "illegal invocation" shape:
/// `"Failed to execute '<method>' on '<interface>': Illegal invocation"`,
/// with `<interface>` taken from the *expected* kind (so calling
/// `mo.observe.call(intersectionObserverInstance, …)` reports the receiver
/// failed the `MutationObserver` brand check — matches Chrome).
pub(crate) fn require_observer_receiver(
    ctx: &NativeContext<'_>,
    this: JsValue,
    expect_kind: ObserverKind,
    method: &'static str,
) -> Result<u64, VmError> {
    let interface = expect_kind.interface_name();
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{interface}': Illegal invocation"
        )));
    };
    match ctx.vm.get_object(id).kind {
        ObjectKind::Observer { kind, observer_id } if kind == expect_kind => Ok(observer_id),
        _ => Err(VmError::type_error(format!(
            "Failed to execute '{method}' on '{interface}': Illegal invocation"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Per-observer binding
// ---------------------------------------------------------------------------

/// The per-observer JS-identity pair carried in
/// `HostData::*_observer_bindings` for each of the three observer
/// kinds.  Keyed by the per-registry monotonic `observer_id: u64` (the
/// inline payload of `ObjectKind::Observer { kind, observer_id }`).
/// Each of `mutation_observers` / `resize_observers` /
/// `intersection_observers` owns its own `next_id` counter, so the
/// three kinds share the `u64` keyspace independently — the
/// `ObserverKind` discriminator on the brand-checked variant
/// disambiguates `(Mutation, 0)` from `(Resize, 0)` etc.
///
/// Both `ObjectId`s are rooted by the keepalive seam's active-observation
/// predicate ([`super::super::gc::keepalive::keepalive_survivors`], S5-3c) — kept
/// iff the observer has ≥1 active observation — so the JS callback + observer
/// wrapper survive any GC cycle **while the observer observes**, and become
/// collectible once its last observation ends (unless independently JS-rooted).
/// The binding-map row is sweep-pruned by the `instance` mark bit
/// (`gc/collect.rs`).  Retained across `Vm::unbind` because the
/// `u64` key is per-registry monotonic (no `Entity` / recycled
/// `ObjectId` aliasing risk) so a retained `mo` / `ro` / `io`
/// reference can `observe()` again after a rebind and have its
/// callback fire.
///
/// Always written together (constructor) and read together
/// (delivery); pairing them into one struct removes the prior
/// "two maps, must stay in sync" foot-gun.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ObserverBinding {
    /// The JS callback `Function` object passed to the observer
    /// constructor (`new MutationObserver(cb)` etc.).
    pub callback: ObjectId,
    /// The observer instance itself — passed back as both the
    /// callback's `this` value and its second argument per spec
    /// (`MutationCallback(records, observer)`,
    /// `ResizeObserverCallback(entries, observer)`,
    /// `IntersectionObserverCallback(entries, observer)`).
    pub instance: ObjectId,
}

// ---------------------------------------------------------------------------
// Per-element JS-Array marshalling (records / entries)
// ---------------------------------------------------------------------------

/// Build a JS `Array` from a Rust slice, marshalling each element via
/// the caller-supplied closure.  Standard temp-root discipline: the
/// outer array is rooted across every per-element marshal call so a
/// GC triggered inside `marshal` (transitive allocations: wrappers,
/// DOMRectReadOnly side-table inserts, Array boxing) cannot collapse
/// the partially-filled array.
///
/// `marshal` receives the same `&mut VmInner` the helper is using —
/// the implicit rooting is via the temp-root stack, not an explicit
/// `&mut VmTempRoot` parameter, so closures stay simple.  The outer
/// array's `ObjectKind::Array { elements }` is pushed-to directly via
/// `get_object_mut` rather than going through `Array.prototype.push`,
/// matching the existing `build_mutation_records_array` discipline
/// (no user-visible side effects, no length recompute).
pub(crate) fn build_marshalled_array<T, F>(vm: &mut VmInner, items: &[T], mut marshal: F) -> JsValue
where
    F: FnMut(&mut VmInner, &T) -> JsValue,
{
    let outer = vm.create_array_object(Vec::with_capacity(items.len()));
    let mut guard = vm.push_temp_root(JsValue::Object(outer));
    for item in items {
        let value = marshal(&mut guard, item);
        let ObjectKind::Array { ref mut elements } = guard.get_object_mut(outer).kind else {
            // `create_array_object` returns an `ObjectKind::Array` by
            // construction; a divergence here would silently drop
            // entries and corrupt observer payloads.  Fail loudly so
            // a future refactor cannot regress the invariant.
            unreachable!("create_array_object must yield ObjectKind::Array");
        };
        elements.push(value);
    }
    drop(guard);
    JsValue::Object(outer)
}

// ---------------------------------------------------------------------------
// Per-observer delivery loop
// ---------------------------------------------------------------------------

/// Iterate `observer_ids` and invoke each observer's JS callback with
/// the records array `build` produces for it.  Handles the temp-root
/// discipline, callback-exception reporting (matches the boa-side
/// `eprintln!("[JS … Observer Error] {err}")` form), and the trailing
/// microtask checkpoint (HTML §8.1.7.3 — perform a microtask checkpoint; chained `.then` reactions
/// fire before this call returns).
///
/// **GC-keepalive invariant (S5-3c data-loss fix): a delivery batch is a
/// single GC snapshot — *every* binding this delivery will touch must
/// stay rooted from batch-start until batch-end.** An earlier observer's
/// callback can, mid-batch, collapse a *later, not-yet-delivered* peer's
/// last keepalive anchor and drop its only JS reference; a GC anywhere in
/// the remainder of the batch (an allocation in the callback itself, or
/// in a subsequent observer's `build`) would then sweep-prune the peer's
/// binding row and free/reuse its `instance` / `callback` `ObjectId`
/// slots — so when the loop reaches the peer its lookup misses, its
/// already-gathered entry is silently dropped (lost notification), or a
/// copied binding dangles (use-after-collect). The anchor lapses in two
/// distinct shapes:
///
/// - **Resize / Intersection**: the delivery *pre-gathers* entries for
///   ALL observer ids into a local map BEFORE the delivery loop.  An
///   earlier observer's callback can reentrantly `disconnect()` /
///   `unobserve()` a later peer, dropping its active-observation
///   membership (the RO §3.5 / IO §3.3 keepalive anchor) and its JS ref
///   while the peer's entry sits gathered-but-undelivered.
/// - **Mutation**: each pending observer *self-anchors* via its own
///   record queue (`observers_with_pending_records`) until its
///   `take_records` at its own turn, so a peer cannot be collected
///   mid-loop by another observer's callback.  But `take_records` *at
///   its own turn* releases that anchor before the (GC-capable)
///   `build_mutation_records_array` runs.
///
/// The batch root is the **uniform** guarantee across both shapes (one
/// issue, one way): rather than root only the current turn's binding
/// (which covers the mutation build window but not the RO/IO pre-gather
/// window), resolve + root every binding up front and hold the roots
/// until the whole batch is delivered.  The observer instances have **no
/// other root** once the keepalive predicate stops covering them: the
/// `ObjectKind::Observer` trace edge only marks a binding's callback when
/// its instance is itself reachable-from-a-root, which it is NOT once
/// disconnected + unreferenced mid-batch — hence both slots are pushed
/// explicitly here.
///
/// The three-closure split makes that possible:
///
/// - `lookup(vm, observer_id) -> Option<ObserverBinding>` — resolve the
///   binding for the batch root.  MUST NOT allocate JS objects (runs in
///   the Phase-1 root-building loop).  Returning `None` drops that id
///   from the batch (its binding lookup failed — a legitimate "race with
///   a prior collection / unbind" outcome).
/// - `prepare(vm, observer_id) -> Option<R>` — registry work ONLY
///   (record drain / observation lookup).  MUST NOT allocate JS objects.
///   Returning `None` skips that observer's callback (the per-kind
///   callsites use this to drop ids whose registry has no pending
///   records — a legitimate "drained via `takeRecords()`" outcome).  `R`
///   is opaque owned per-kind record data (`Vec<MutationRecord>` /
///   `Vec<ResizeObserverEntry>` / `Vec<IntersectionObserverEntry>`).
/// - `build(vm, R) -> JsValue` — the GC-capable marshal of `R` into the
///   records/entries Array, run with the whole batch already rooted.
///
/// The microtask drain runs **unconditionally** post-loop — matches
/// `deliver_mutation_records`' embedder-API parity contract: even an
/// empty `observer_ids` slice yields a microtask checkpoint so
/// chained promises queued from earlier in the same frame fire on
/// the same boundary.
pub(crate) fn deliver_to_observer_callbacks<R, L, P, B>(
    vm: &mut VmInner,
    observer_ids: &[u64],
    mut lookup: L,
    mut prepare: P,
    mut build: B,
) where
    L: FnMut(&mut VmInner, u64) -> Option<ObserverBinding>,
    P: FnMut(&mut VmInner, u64) -> Option<R>,
    B: FnMut(&mut VmInner, R) -> JsValue,
{
    // Phase 1 — resolve + BATCH-ROOT every binding this delivery will
    // touch. `push_stack_scope` roots an arbitrary number of values for
    // the scope's lifetime (each `frame.stack.push` is a GC root until
    // the scope drops), so both `ObjectId`s of every observer in the
    // batch stay rooted through the entire Phase-2 delivery loop — a GC
    // triggered by ANY observer's callback / build cannot prune a
    // later peer's binding row. `batch` is a separate Vec (no borrow
    // conflict with `frame`, which mutably borrows the VM).
    let mut frame = vm.push_stack_scope();
    let mut batch: Vec<(u64, ObserverBinding)> = Vec::with_capacity(observer_ids.len());
    for &observer_id in observer_ids {
        if let Some(binding) = lookup(&mut frame, observer_id) {
            frame.stack.push(JsValue::Object(binding.instance));
            frame.stack.push(JsValue::Object(binding.callback));
            batch.push((observer_id, binding));
        }
    }

    // Phase 2 — per-observer registry work + marshal + callback, all
    // under the batch root established in Phase 1.
    for (observer_id, binding) in &batch {
        let Some(prepared) = prepare(&mut frame, *observer_id) else {
            continue;
        };
        let observer_val = JsValue::Object(binding.instance);
        let records_arr = build(&mut frame, prepared);
        // Per-turn root for the freshly-built records array across the
        // call (its identity-assert holds because `call` restores the
        // stack). The batch root already covers `instance` + `callback`.
        let mut records_guard = frame.push_temp_root(records_arr);
        if let Err(err) =
            records_guard.call(binding.callback, observer_val, &[records_arr, observer_val])
        {
            // Embedder-side diagnostic channel is deferred to
            // `#11-embedder-diagnostic-channel`; until then this
            // matches the boa-side stderr report path so the
            // four observer-family callsites (Mutation / Resize
            // / Intersection / MediaQuery) stay consistent.
            eprintln!("[JS Observer Error] {err:?}");
        }
        drop(records_guard);
    }
    drop(frame);
    vm.drain_microtasks();
}

// ---------------------------------------------------------------------------
// WebIDL §3.10.7 dictionary-member read
// ---------------------------------------------------------------------------

/// Look up an own / prototype-chain property by `StringId`, returning
/// `None` for `undefined` (per WebIDL §3.10.7 dictionary semantics —
/// an `undefined` value means "member not present", default applies).
/// Other values pass through to the caller for type coercion.
///
/// Hoisted from the original site in `mutation_observer.rs` so all
/// three observer init-dict parsers (and any future surface
/// following the same WebIDL pattern) share one canonical form
/// rather than each inlining the same `get_property_value + matches!`
/// check.  Object-only API surface — primitives that should
/// ToObject-coerce per spec §3.10.7 are handled by the caller (most
/// callers reject primitives with a TypeError matching the Chrome /
/// Firefox shape, Phase 2 simplification tracked at
/// `#11-mutation-observer-extras`).
pub(crate) fn read_dict_field(
    ctx: &mut NativeContext<'_>,
    obj_id: ObjectId,
    name: StringId,
) -> Result<Option<JsValue>, VmError> {
    let value = ctx.get_property_value(obj_id, PropertyKey::String(name))?;
    if matches!(value, JsValue::Undefined) {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}
