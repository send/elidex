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
//! `EventListeners` API; the `WebSocket` / `EventSource` arms [S5-3b] marshal VM
//! state and delegate their tier rule to `elidex-api-ws::{ws_keepalive,
//! es_keepalive}`).  The remaining observer arm marshals to
//! `elidex-api-observers` (S5-3c).

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
/// S5-3a landed the `MediaQueryList` arm (the FLIP-precondition); S5-3b adds the
/// `WebSocket` / `EventSource` arms (state-tiered listener subset, the tier rule
/// delegated to `elidex-api-ws::{ws_keepalive, es_keepalive}`).  The remaining
/// non-Node EventTargets migrate their existing divergent roots onto this seam
/// **before** the S5-6 flip (the hard pre-flip gate
/// `#11-eventtarget-keepalive-registrant-coverage`): the Mutation / Resize /
/// Intersection observers (active-observation membership, S5-3c — delegated to
/// `elidex-api-observers`).
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
    /// `WebSocket` (WebSockets §7 Garbage collection).  **Pure state-tiered
    /// listener check**: kept while readyState ∈ {CONNECTING, OPEN, CLOSING} with
    /// the per-state listener subset.  The §7 no-listener `data-queued` clause is
    /// **OMITTED as vacuous** in elidex (F1): outbound bytes are broker-owned FIFO
    /// once `send()` emits (they transmit ahead of any GC-emitted close whether the
    /// wrapper survives or not), and `buffered_amount` is incremented
    /// unconditionally (incl. never-transmitted CLOSING/CLOSED sends) so keying on
    /// it would over-root a listener-less CLOSING socket into an indefinite leak.
    /// The arm marshals **only** the readyState from `HostData::websocket_states` +
    /// a typed-listener closure over [`vm_path_has_listener`], and delegates the
    /// tier rule to [`elidex_api_ws::ws_keepalive`].  A genuine orphan (no
    /// in-tier listener) or a CLOSED wrapper is NOT kept → the `collect.rs` sweep
    /// prunes it and force-closes the connection (the spec's GC-while-open close).
    WebSocket,
    /// `EventSource` (HTML §9.2.9 Garbage collection).  **State-tiered OR
    /// task-queued**: kept while readyState ∈ {CONNECTING, OPEN} with the per-state
    /// listener subset, **OR** while an inbound SSE event is buffered for this
    /// conn awaiting dispatch — the §9.2.9 no-listener "task queued on the remote
    /// event task source" clause, **INCLUDED** because it is the GC root for the
    /// inbound buffer window (F3): an event buffers between
    /// `drain_fetch_responses_only` and `drain_events`, and a mid-turn GC that
    /// collects a named-event-only wrapper in that window would silently drop the
    /// event via a reverse-map miss.  The arm marshals `EventSourceState`
    /// (readyState + `conn_id`) and derives `has_queued_task` by peeking the
    /// `NetworkHandle` buffer (`has_pending_event_for_conn`) +
    /// [`vm_path_has_listener`], delegating to [`elidex_api_ws::es_keepalive`].
    EventSource,
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
            KeepaliveClass::WebSocket => {
                // Marshal ONLY the readyState from `WebSocketState`, then delegate
                // the §7 pure tier rule — no `buffered_amount` input (the §7
                // data-queued clause is dropped as vacuous/F1; see
                // `elidex_api_ws::ws_keepalive`). (Copy the scalar out so the
                // `host_data` borrow is dropped before the listener closure.)
                let Some(ready_state) = vm
                    .host_data
                    .as_deref()
                    .and_then(|hd| hd.websocket_states.get(&target))
                    .map(|s| s.ready_state)
                else {
                    return false;
                };
                elidex_api_ws::ws_keepalive(ready_state, |t| {
                    vm_path_has_listener(vm, target, t, false)
                })
            }
            KeepaliveClass::EventSource => {
                // Marshal readyState + conn_id from `EventSourceState`, then derive
                // `has_queued_task` from the NetworkHandle buffer peek (the §9.2.9
                // task-queued clause, F3). (Copy the scalars out so the `host_data`
                // borrow is dropped before the peek + the listener closure, which
                // both re-borrow `&VmInner`.)
                let Some((ready_state, conn_id)) = vm
                    .host_data
                    .as_deref()
                    .and_then(|hd| hd.event_source_states.get(&target))
                    .map(|s| (s.ready_state, s.conn_id))
                else {
                    return false;
                };
                // Is an inbound SSE event buffered for this conn awaiting drain?
                // `network_handle` lives on `VmInner` (installed post-construction);
                // absent in standalone/test-less mode → no queued task.
                let has_queued_task = vm
                    .network_handle
                    .as_ref()
                    .is_some_and(|h| h.has_pending_event_for_conn(conn_id));
                elidex_api_ws::es_keepalive(ready_state, has_queued_task, |t| {
                    vm_path_has_listener(vm, target, t, false)
                })
            }
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
///   interface's own type-restricted rule.  `MediaQueryList` + `WebSocket` /
///   `EventSource` now (S5-3a/b); the observers join before the flip (S5-3c).
///   `WebSocket` / `EventSource` are state-tiered (WebSockets §7 / HTML §9.2.9,
///   delegated to `elidex-api-ws`) over the per-VM `HostData::websocket_states` /
///   `event_source_states` side-stores; a kept connection survives the
///   `collect.rs` sweep (so it is NOT force-closed and keeps delivering), the
///   un-kept orphan/CLOSED wrapper is swept + force-closed (the sweep is the
///   predicate's `false` else-branch — see [`KeepaliveClass::WebSocket`]).
///   `MediaQueryList` is document-scoped
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

    // WebSocket / EventSource — state-tiered listener predicate (WebSockets §7 /
    // HTML §9.2.9), delegated to `elidex-api-ws`. A listener-held non-CLOSED WS
    // (pure tier — the §7 data-queued clause is OMITTED as vacuous/F1), or a
    // listener-held OR buffer-window (`has_queued_task`, §9.2.9/F3) non-CLOSED ES,
    // survives this GC and keeps delivering; a genuine orphan (no in-tier
    // listener, and for ES no queued task) or a CLOSED wrapper is NOT marked here,
    // so the `collect.rs` sweep tail prunes its state row AND emits the broker
    // `WebSocketClose` / `EventSourceClose` (the spec's GC-while-open closing
    // handshake / fetch-abort). That sweep keys purely on the mark bit, so it is
    // already the predicate's `false` else-branch — no edit there. Keys collected
    // first so the `host_data` borrow is dropped before the per-id `keepalive`
    // calls (themselves `&VmInner`-borrowing).
    let (ws_ids, es_ids): (Vec<ObjectId>, Vec<ObjectId>) = match vm.host_data.as_deref() {
        Some(hd) => (
            hd.websocket_states.keys().copied().collect(),
            hd.event_source_states.keys().copied().collect(),
        ),
        None => (Vec::new(), Vec::new()),
    };
    keep.extend(
        ws_ids
            .into_iter()
            .filter(|&id| KeepaliveClass::WebSocket.keepalive(vm, id)),
    );
    keep.extend(
        es_ids
            .into_iter()
            .filter(|&id| KeepaliveClass::EventSource.keepalive(vm, id)),
    );

    keep
}
