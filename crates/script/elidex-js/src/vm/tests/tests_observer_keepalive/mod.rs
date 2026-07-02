//! S5-3c — the observer (Mutation / Resize / Intersection) GC-keepalive arm
//! (`#11-eventtarget-listener-keepalive-rooting`, the observer keepalive
//! predicate).
//!
//! **The oracle is FLIPPED vs S5-3a/b** (the WS/ES/MQL under-root fixes, whose
//! headline asserts a listener-held target *survives*). Observers were
//! **OVER-rooted** — the `(callback, instance)` binding was rooted at
//! construction for life (`gc_root_object_ids`), and `disconnect()` never
//! released it → immortal-until-`Vm::unbind` leak. S5-3c routes the observers
//! through the keepalive seam with the spec predicate **"has ≥1 active
//! observation"** (DOM §4.3 registered-observer-list / RO §3.5 / IO §3.3
//! Lifetime) — OR-ed, MutationObserver only, with **"has ≥1 pending
//! undelivered record"** (full predicate = `gc/keepalive.rs`
//! `keepalive_survivors`); a never-observed / disconnected unreferenced
//! observer becomes
//! **collectible** (its binding-map row is pruned in the sweep). So the headline
//! here asserts an idle observer **IS collected**.
//!
//! Companion unit tests for the engine-indep membership query
//! (`observing_observer_ids`) live in `elidex-api-observers`.
//!
//! Split by scenario group (touch-time 1000-line convention):
//!
//! - [`collection`] — (a) headline never-observed-is-collected flip +
//!   the JS-referenced negative control + binding-row prune-by-id.
//! - [`delivery`] — (b) observing survives GC + still fires.
//! - [`disconnect_despawn`] — (c) observe-then-disconnect / unobserve +
//!   (d) despawn-of-sole-target discriminator.
//! - [`pending_records`] — (d') pending-records keepalive clause +
//!   drained-queue negative control.
//! - [`mid_delivery_gc`] — mid-delivery GC temp-root / batch-root fixes.
//! - [`unbound_rebind`] — (e) unbound-GC keep-all fail-safe + rebind-resume.
//! - [`transient`] — transient registered-observer membership.
//! - [`registry_retirement`] — (f) registry-side `retire_collected` sweep.

#![cfg(feature = "engine")]

mod collection;
mod delivery;
mod disconnect_despawn;
mod mid_delivery_gc;
mod pending_records;
mod registry_retirement;
mod transient;
mod unbound_rebind;

use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::{MutationKind, MutationRecord as SessionRecord};

use super::super::Vm;

// --- shared fixtures --------------------------------------------------------

pub(super) fn build_doc(dom: &mut EcsDom) -> Entity {
    let doc = dom.create_document_root();
    let html = dom.create_element("html", elidex_ecs::Attributes::default());
    let body = dom.create_element("body", elidex_ecs::Attributes::default());
    assert!(dom.append_child(doc, html));
    assert!(dom.append_child(html, body));
    doc
}

pub(super) fn body_of(dom: &EcsDom, doc: Entity) -> Entity {
    dom.first_child_with_tag(dom.first_child_with_tag(doc, "html").unwrap(), "body")
        .unwrap()
}

/// Counts of the three `*_observer_bindings` maps (the sweep-prune oracle).
pub(super) fn binding_counts(vm: &Vm) -> (usize, usize, usize) {
    let hd = vm.inner.host_data.as_deref().unwrap();
    (
        hd.mutation_observer_bindings.len(),
        hd.resize_observer_bindings.len(),
        hd.intersection_observer_bindings.len(),
    )
}

/// Registry-internal row counts (mutation `records` / resize `registered` /
/// intersection `observers`) — the SECOND-HALF leak oracle: the GC sweep must
/// `retire_collected` these alongside the binding rows so no registry-side
/// residual survives a collection.
pub(super) fn registry_counts(vm: &Vm) -> (usize, usize, usize) {
    let hd = vm.inner.host_data.as_deref().unwrap();
    (
        hd.mutation_observers.records_len(),
        hd.resize_observers.registered_len(),
        hd.intersection_observers.observers_len(),
    )
}

/// A `ChildList` record adding `added` to `target`.
pub(super) fn child_list_added(target: Entity, added: Entity) -> SessionRecord {
    SessionRecord {
        kind: MutationKind::ChildList,
        target,
        added_nodes: vec![added],
        removed_nodes: vec![],
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    }
}
