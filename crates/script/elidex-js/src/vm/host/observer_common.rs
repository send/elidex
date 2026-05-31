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
//!   delivery loop: lookup binding, temp-root the observer instance +
//!   records array, invoke the JS callback with `(records, observer)`,
//!   report exceptions via `eprintln!`, then drain microtasks at end.
//!   Each per-kind `deliver_*` method on `VmInner` is now a thin
//!   shell that supplies (a) the binding-lookup closure (per-kind
//!   HashMap), (b) a `prepare` closure that returns `(binding,
//!   records_array)` for each observer with work.
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
/// Both `ObjectId`s are rooted via
/// [`super::super::host_data::HostData::gc_root_object_ids`] so the
/// JS callback + observer wrapper survive any GC cycle while the
/// observer is registered.  Retained across `Vm::unbind` because the
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

/// Iterate `observer_ids` and invoke each observer's JS callback
/// with the records array `prepare` produces for it.  Handles the
/// temp-root discipline (instance + records array rooted across the
/// `call` so a GC triggered by user code cannot collect either),
/// callback-exception reporting (matches the boa-side
/// `eprintln!("[JS … Observer Error] {err}")` form), and the trailing
/// microtask checkpoint (HTML §8.1.7.3 — perform a microtask checkpoint; chained `.then` reactions
/// fire before this call returns).
///
/// `prepare(vm, observer_id) -> Option<(binding, records_array)>` is
/// called once per id.  Returning `None` skips that observer (the
/// per-kind callsites use this to drop ids whose registry has no
/// pending records, or whose `(callback, instance)` lookup failed
/// — both legitimate "race with unobserve / disconnect" outcomes).
/// The closure runs to completion before the helper temp-roots its
/// returned array, so any allocations the closure makes can use the
/// closure's own temp-root scope; once the closure returns, the
/// helper takes over and roots the array across the JS callback.
///
/// The microtask drain runs **unconditionally** post-loop — matches
/// `deliver_mutation_records`' embedder-API parity contract: even an
/// empty `observer_ids` slice yields a microtask checkpoint so
/// chained promises queued from earlier in the same frame fire on
/// the same boundary.
pub(crate) fn deliver_to_observer_callbacks<F>(
    vm: &mut VmInner,
    observer_ids: &[u64],
    mut prepare: F,
) where
    F: FnMut(&mut VmInner, u64) -> Option<(ObserverBinding, JsValue)>,
{
    for &observer_id in observer_ids {
        let Some((binding, records_arr)) = prepare(vm, observer_id) else {
            continue;
        };
        let observer_val = JsValue::Object(binding.instance);
        let mut observer_guard = vm.push_temp_root(observer_val);
        let mut records_guard = observer_guard.push_temp_root(records_arr);
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
        drop(observer_guard);
    }
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
