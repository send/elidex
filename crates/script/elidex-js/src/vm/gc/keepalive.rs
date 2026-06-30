//! The keepalive-predicate seam (`#11-eventtarget-listener-keepalive-rooting`).
//!
//! A non-Node VM `EventTarget` (`MediaQueryList` / `AbortSignal` /
//! `WebSocket` / `EventSource` / the observers) lives in a side-store keyed
//! by its own `ObjectId`, with its listeners in
//! [`VmInner::vm_event_listeners`](super::super::VmInner::vm_event_listeners)
//! (callbacks rooted via `HostData::listener_store`).  The callback being a
//! root does NOT root the **target** — so a target anchored *only* by a
//! listener (e.g. `matchMedia(q).addEventListener('change', cb)` with no
//! retained reference) used to be swept, and its out-of-band state row pruned
//! with it (the `deliver_*` producer then silently skips the collected
//! target).
//!
//! ## The mechanism — a per-registrant predicate, NOT an any-listener root
//!
//! DOM §2.8 ("Observing event listeners") states that listener presence must
//! not be a general keepalive: there is **no** "an EventTarget with a listener
//! stays alive" rule.  Keepalive is a **per-interface opt-in**, and every real
//! GC-note gates on `<active/in-flight state> AND <type-restricted listener
//! subset>` (XHR §3.2 / WebSockets §7 / HTML §9.2.9 / DOM §3.2.1), never "any
//! listener of any type".  So the seam is a **keepalive PREDICATE**: one GC
//! mark pass ([`keepalive_survivors`]) that asks each registrant *its own*
//! spec-faithful question via [`KeepaliveClass`], and marks only the survivors.
//! An any-listener root would be over-rooting (a §2.8 violation in the leak
//! direction).
//!
//! ## Layering + ECS-native home
//!
//! The rooted thing is a per-VM `ObjectId` (the side-store→component rule's
//! per-VM-identity-handle exception (a) — `Vm::unbind`'s cross-DOM-aliasing
//! note): component-on-entity is world_id-gated and deferred
//! (`#11-eventtarget-keepalive-component-migration`).  ⚠ SUPERSEDED 2026-06-30:
//! world_id retracted → agent-scoped `EcsDom` World (PR #434,
//! `docs/plans/2026-06-agent-scoped-ecsdom-world.md`); under B1 (1-agent=1-World)
//! per-entity identity is stable, so that migration becomes safe without a
//! discriminator.  A predicate whose rule
//! is engine-independent stays so (the `MediaQueryList` arm reuses
//! [`vm_path_has_listener`], itself composed from the engine-independent
//! `EventListeners` API); a future `WebSocket` / `EventSource` arm marshals VM
//! state and delegates its tier rule to `elidex-api-ws`, and the observer arm
//! to `elidex-api-observers` (S5-3b/c).

#![cfg(feature = "engine")]

use super::super::host::event_target_dispatch_vm::vm_path_has_listener;
use super::super::host_data::HostData;
use super::super::value::ObjectId;
use super::super::VmInner;

/// The **listener-predicate** mechanism of the keepalive seam — one of the two
/// the seam ([`keepalive_survivors`]) composes.  The other is plain *membership*
/// rooting (registration in an in-flight registry *is* the anchor), which asks
/// no per-registrant question and so needs no dispatch — it is marked directly,
/// not through this enum.  `KeepaliveClass` selects the spec-faithful predicate
/// for each non-Node `EventTarget` whose keepalive *is* a listener test.
///
/// Static enum dispatch: these are all **built-in** EventTargets (no runtime /
/// user extension), so the rule is selected by a per-kind `match` arm, not a
/// `dyn` registry — CLAUDE.md Plugin-first "hot path built-in = static
/// dispatch".  Each arm encodes *which* listener types its interface's GC-note
/// keeps alive; the seam supplies the per-VM listener state.
///
/// S5-3a lands the `MediaQueryList` arm — the FLIP-precondition.  The remaining
/// non-Node EventTargets migrate their existing divergent roots onto this seam
/// **before** the S5-6 flip (the hard pre-flip gate
/// `#11-eventtarget-keepalive-registrant-coverage`): `WebSocket` / `EventSource`
/// (state-tiered listener subset, S5-3b — the tier rule delegated to
/// `elidex-api-ws`) and the Mutation / Resize / Intersection observers
/// (active-observation membership, S5-3c — delegated to `elidex-api-observers`).
/// A future `AbortSignalDependent` arm would root an `AbortSignal.any()`
/// composite under the DOM §3.2.1 dependent-signal predicate (non-aborted ∧
/// source-signals-non-empty ∧ listenered) — but that is a **behavior change**
/// (`any_composite_map` is a deliberate non-root today, `mark_roots` note (k)),
/// so it is NOT part of behavior-neutral S5-3a.  Likewise the
/// `AbortController`-internal signal stays a **trace** edge (`gc/trace.rs`,
/// reachable-from-the-controller), not a keepalive root — a root pass would
/// over-root a signal whose controller is already dead.  Adding a client is a
/// new arm here + its registrant loop in [`keepalive_survivors`], never a new
/// divergent root pass.
#[derive(Clone, Copy)]
enum KeepaliveClass {
    /// `MediaQueryList` (CSSOM-View §4.2).  The interface has **no** GC-note;
    /// the spec keeps an MQL alive through its document's MQL set.  elidex
    /// narrows that listener-independent membership to **has a live `change`
    /// listener** — pragmatic-faithful: a listener-less unreferenced MQL can
    /// deliver nothing (collecting it is GC-observably sound), and the narrowing
    /// avoids the over-root a raw registry-membership root would bake in.  The
    /// test reuses [`vm_path_has_listener`], the dispatch-time SSoT, so
    /// *kept-alive ⇔ would-actually-fire*: it counts a `change`
    /// `addEventListener` **or** a live `onchange` handler, and correctly
    /// EXCLUDES a cleared `onchange = null` handler (whose callable is retired
    /// from `listener_store`, so the registration metadata lingers but fires
    /// nobody).
    MediaQueryList,
}

impl KeepaliveClass {
    /// Whether `target` (a registrant of this class) must survive this GC.
    ///
    /// Takes `&VmInner` (not the [`GcRoots`](super::roots::GcRoots) snapshot) so
    /// the arm can reuse [`vm_path_has_listener`] rather than duplicate the
    /// listener-liveness walk into the snapshot.  Caller
    /// ([`keepalive_survivors`]) owns the per-class registrant set + document
    /// scope; this is the pure per-registrant rule.
    fn keepalive(self, vm: &VmInner, target: ObjectId) -> bool {
        match self {
            KeepaliveClass::MediaQueryList => vm_path_has_listener(vm, target, "change", false),
        }
    }
}

/// Collect every non-Node `EventTarget` `ObjectId` that this GC must keep
/// alive by its spec keepalive rule — the keepalive-predicate seam's mark set.
///
/// Returns an owned `Vec` (rather than marking in place) so the caller
/// ([`VmInner::collect_garbage`](super::super::VmInner::collect_garbage)) can
/// run it as an immutable `&VmInner` borrow that coexists with the live
/// `GcRoots` snapshot, then mark the survivors under the disjoint `&mut`
/// borrow of the mark bit-vectors.  Runs **before** `trace_work_list` so a
/// marked registrant's out-of-band state fan-out (e.g. an `AbortSignal`'s
/// `reason` + `abort` listener callbacks) is traced.
///
/// The seam composes two keepalive mechanisms (see the inline notes for the
/// per-mechanism rationale):
///
/// - **listener-predicate** registrants ([`KeepaliveClass`]) — survival is the
///   interface's own type-restricted rule.  `MediaQueryList` now; `WebSocket` /
///   `EventSource` / observers join before the flip (S5-3b/c).  Document-scoped
///   through `MediaQueryEntry::keepalive_worthy` — the GC-LIVENESS gate, which
///   delegates to `deliver`'s dispatch gate (`deliverable_to`) while bound but
///   keeps a `document`-tagged MQL alive across an unbound inter-batch GC so a
///   later same-document `deliver` can still fire it (liveness ≠ dispatch; the
///   inline note covers the cross-`EcsDom` deferral, dissolved by B1 — PR #434).
/// - **membership** registrants — registration in an in-flight registry *is*
///   the anchor.  `AbortSignal.timeout` signals (timer-pending; the
///   `timeout()` step note, DOM §3.2 `#dom-abortsignal-timeout` — distinct from
///   §3.2.1 Garbage collection, the *dependent*-signal predicate) — routed here
///   from `mark_roots` pass (j) so non-Node EventTarget
///   keepalive lives in one home (behavior-neutral: the same signal set is
///   marked).
pub(super) fn keepalive_survivors(vm: &VmInner) -> Vec<ObjectId> {
    let current_document = vm
        .host_data
        .as_deref()
        .and_then(HostData::document_entity_opt);

    // MediaQueryList — kept iff `MediaQueryEntry::keepalive_worthy` (the
    // GC-LIVENESS gate) holds AND it has a live `change` listener. Liveness ≠
    // dispatch deliverability (Codex R5): `keepalive_worthy` delegates to the
    // `deliver` gate (`deliverable_to`) while BOUND, but while UNBOUND it keeps
    // every `document`-tagged MQL — a listener-only MQL must survive an unbound
    // inter-batch GC so the next same-document rebind's `deliver` can fire it
    // (collecting it would reintroduce the lost-`change` bug the seam fixes). An
    // unbound-created MQL (`document == None`) is never deliverable → collected.
    // The cross-`EcsDom` rebind-with-index-collision case keepalive_worthy
    // cannot tell from a same-`EcsDom` rebind is `deliver`'s own pre-existing
    // raw-`Entity` exposure — inert until the S5-6 flip first drives `deliver`.
    // ⚠ SUPERSEDED 2026-06-30: dissolved by B1 (agent-scoped `EcsDom` World, PR
    // #434) not world_id — a `Vm` never rebinds across worlds, so the case does
    // not arise in production. See `MediaQueryEntry::keepalive_worthy`.
    let mut keep: Vec<ObjectId> = vm
        .media_query_list_registry
        .iter()
        .filter(|(_, entry)| entry.keepalive_worthy(current_document))
        .map(|(&id, _)| id)
        .filter(|&id| KeepaliveClass::MediaQueryList.keepalive(vm, id))
        .collect();

    // AbortSignal.timeout — membership (timer-pending). The signal is reachable
    // only via this map until the timer fires; the sweep tail
    // (`collect.rs`) prunes any entry whose signal was somehow collected.
    keep.extend(vm.pending_timeout_signals.values().copied());

    keep
}
