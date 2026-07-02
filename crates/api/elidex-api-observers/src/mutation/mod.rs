//! `MutationObserver` API (DOM Standard §4.3).
//!
//! Observes changes to the DOM tree (child list, attributes, character data).
//!
//! ECS-native model: the per-node **registered observer list** (WHATWG DOM
//! §4.3) lives as a `MutationObservedBy` component on the observed target
//! entity, mirroring the spec data model. `notify` reproduces §4.3.2 "Queuing a
//! mutation record" by walking the target's inclusive ancestors via
//! [`EcsDom::get_parent`] and reading each node's registered observer list.
//! Only the per-observer pending-record queue (the JS observer object's
//! `[[recordQueue]]` internal slot) is held registry-side, keyed by observer id.

use elidex_ecs::{EcsDom, Entity, MAX_ANCESTOR_DEPTH};

/// A unique identifier for a mutation observer registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MutationObserverId(u64);

impl MutationObserverId {
    /// Create an ID from a raw u64 value.
    #[must_use]
    pub fn from_raw(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw u64 value.
    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// Options for `MutationObserver.observe()`.
#[allow(clippy::struct_excessive_bools)] // Mirrors DOM Standard MutationObserverInit dictionary
#[derive(Debug, Clone, Default)]
pub struct MutationObserverInit {
    /// Observe child list changes.
    pub child_list: bool,
    /// Observe attribute changes.
    pub attributes: bool,
    /// Observe character data changes.
    pub character_data: bool,
    /// Observe the entire subtree.
    pub subtree: bool,
    /// Record old attribute values.
    pub attribute_old_value: bool,
    /// Record old character data values.
    pub character_data_old_value: bool,
    /// Filter to specific attribute names.
    pub attribute_filter: Option<Vec<String>>,
}

/// A single mutation record delivered to the observer callback.
#[derive(Debug, Clone)]
pub struct MutationRecord {
    /// The type of mutation: "childList", "attributes", or "characterData".
    pub mutation_type: String,
    /// The target node of the mutation.
    pub target: Entity,
    /// Nodes added (for childList mutations).
    pub added_nodes: Vec<Entity>,
    /// Nodes removed (for childList mutations).
    pub removed_nodes: Vec<Entity>,
    /// The previous sibling of added/removed nodes.
    pub previous_sibling: Option<Entity>,
    /// The next sibling of added/removed nodes.
    pub next_sibling: Option<Entity>,
    /// The attribute name (for attribute mutations).
    pub attribute_name: Option<String>,
    /// The old value (if requested).
    pub old_value: Option<String>,
}

/// A single registered observer on a node (WHATWG DOM §4.3 "registered observer").
///
/// A **transient registered observer** (`transient == true`) is appended to a
/// node when it is removed from a subtree observed by an ancestor's
/// `subtree:true` observer (WHATWG DOM §4.2.3 "remove" step 15), so mutations in
/// the now-detached subtree keep reaching that observer until the next microtask
/// delivery clears it (§4.3 "notify mutation observers" step 6.3).
///
/// Each registration carries a stable identity `reg_id` (the spec's
/// registered-observer object identity). The spec's `source` (the *originating*
/// registered observer of a transient, WHATWG DOM §4.2.3 "remove" step 15) is
/// `source: Some(src_reg_id)` — the `reg_id` of the ancestor registration that
/// spawned it (`None` for a permanent registration). The two clearings key
/// differently, matching the spec: notify step 6.3 removes transients **by
/// observer** ("observer is mo"), while observe() step 7.1 removes them **by
/// source** ("source is registered") — once per registration of this observer on
/// the re-observed node (§4.3.1 step 7 matches every registration for `this`,
/// transient or permanent). Keying `source` on the registration `reg_id` (not the
/// ancestor entity) correctly distinguishes multiple registrations of the same
/// observer on one node (a permanent + a chained transient).
#[derive(Debug, Clone)]
struct MutationObservation {
    observer: MutationObserverId,
    init: MutationObserverInit,
    /// `true` for a transient registered observer (cleared every microtask
    /// delivery); `false` for a permanent registration created by `observe()`.
    transient: bool,
    /// For a transient observer, the `reg_id` of the registered observer that
    /// spawned it (WHATWG DOM §4.2.3 "remove" step 15 `source`); `None` for a
    /// permanent registration.
    source: Option<u64>,
    /// Stable identity of this registration (the spec's registered-observer
    /// object identity), allocated from `MutationObserverRegistry::next_reg_id`.
    reg_id: u64,
}

/// The per-node **registered observer list** (WHATWG DOM §4.3), stored as a
/// component on the observed target entity. Removed automatically when the
/// entity is despawned.
#[derive(Debug, Default)]
struct MutationObservedBy(Vec<MutationObservation>);

/// Registry of active mutation observers.
///
/// Holds the observer-owned pending-record queues and the agent's "pending
/// mutation observers" set; the observation relationship lives on target
/// entities as `MutationObservedBy` components.
#[derive(Debug, Default)]
pub struct MutationObserverRegistry {
    next_id: u64,
    /// Monotonic allocator for `MutationObservation::reg_id` (registered-observer
    /// identity), used as the transient `source` key.
    next_reg_id: u64,
    /// Per-observer pending-record queues, keyed by observer id (a lookup map;
    /// delivery order is the spec's append-ordered `pending` set below, not this
    /// map's key order).
    records: std::collections::BTreeMap<MutationObserverId, Vec<MutationRecord>>,
    /// The agent's **pending mutation observers** (WHATWG DOM §4.3.2 "queue a
    /// mutation record" step 4.3 appends here; §4.3 "notify mutation observers"
    /// step 2 clones it as `notifySet`, step 3 empties it). A WHATWG **ordered
    /// set** — stored as an append-ordered `Vec` (dedup on insert) so callback
    /// delivery and the interleaved step-6.3 transient clearing run in spec
    /// append order (the order each observer first received a record this cycle),
    /// not observer-id order. **Distinct from `records`**: `takeRecords()` empties
    /// an observer's record queue but does **not** remove it here, so a microtask
    /// still runs step 6.3 for it.
    pending: Vec<MutationObserverId>,
}

impl MutationObserverRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new mutation observer, returning its ID.
    pub fn register(&mut self) -> MutationObserverId {
        let id = MutationObserverId(self.next_id);
        self.next_id += 1;
        self.records.insert(id, Vec::new());
        id
    }

    /// Allocate a fresh registered-observer identity (`MutationObservation::reg_id`).
    fn alloc_reg_id(&mut self) -> u64 {
        let reg_id = self.next_reg_id;
        self.next_reg_id += 1;
        reg_id
    }

    /// Append transient registered observers to the removed nodes (WHATWG DOM
    /// §4.2.3 "remove" step 15).
    ///
    /// For each inclusive ancestor of `parent` and each `subtree:true` registered
    /// observer on it (permanent **or** transient — the spec walks the full
    /// registered observer list, so chained transients propagate), append a
    /// transient copy to each removed node's registered observer list, tagged with
    /// `source = Some(<that registration's reg_id>)` (the spec's step-15 `source`)
    /// and a fresh `reg_id` of its own. A coalesced removal (replace / replace-all)
    /// carries its removed set on a single `ChildList` record, so the caller drives
    /// this once per such record. Not gated by `suppressObservers` (only the
    /// removal *record* is — step 16).
    ///
    /// A registry method (not a free fn) because it allocates fresh `reg_id`s from
    /// `next_reg_id`; the per-node registered observer lists still live entirely on
    /// the `MutationObservedBy` components (ECS-native).
    ///
    /// Appends one transient per matching ancestor registration without dedup
    /// (spec "append a new transient registered observer"); any same-observer
    /// duplicates this produces on a node collapse to a single record in
    /// [`Self::notify`] (§4.3.2 interestedObservers map).
    pub fn add_transient_observers(
        &mut self,
        dom: &mut EcsDom,
        parent: Entity,
        removed: &[Entity],
    ) {
        if removed.is_empty() {
            return;
        }
        // Phase 1 (immutable reads): collect (observer, options, source reg_id)
        // from the `subtree:true` registered observers on `parent`'s inclusive
        // ancestors. Collecting first lets the shared `&MutationObservedBy` reads
        // finish before Phase 2's `&mut` appends. The bounded walk mirrors
        // `notify`'s `MAX_ANCESTOR_DEPTH` cycle guard.
        let mut sources: Vec<(MutationObserverId, MutationObserverInit, u64)> = Vec::new();
        let mut node = Some(parent);
        let mut depth = 0usize;
        while let Some(n) = node {
            if depth >= MAX_ANCESTOR_DEPTH {
                break;
            }
            depth += 1;
            if let Ok(comp) = dom.world().get::<&MutationObservedBy>(n) {
                for obs in &comp.0 {
                    if obs.init.subtree {
                        sources.push((obs.observer, obs.init.clone(), obs.reg_id));
                    }
                }
            }
            node = dom.get_parent(n);
        }
        if sources.is_empty() {
            return;
        }
        // Phase 2 (`&mut` appends): append a transient (with a fresh `reg_id` and
        // `source` = the originating registration's reg_id) to each removed node's
        // registered observer list, inserting the component when absent.
        for &rn in removed {
            if !dom.contains(rn) {
                continue;
            }
            let new_entries: Vec<MutationObservation> = sources
                .iter()
                .map(|(observer, init, src_reg)| MutationObservation {
                    observer: *observer,
                    init: init.clone(),
                    transient: true,
                    source: Some(*src_reg),
                    reg_id: self.alloc_reg_id(),
                })
                .collect();
            // Split the existence check from the mutation so the `Ok`-arm's borrow
            // does not extend into the insert arm (NLL temporary lifetime).
            let has_comp = dom.world().get::<&MutationObservedBy>(rn).is_ok();
            if has_comp {
                if let Ok(mut comp) = dom.world_mut().get::<&mut MutationObservedBy>(rn) {
                    comp.0.extend(new_entries);
                }
            } else {
                let _ = dom
                    .world_mut()
                    .insert_one(rn, MutationObservedBy(new_entries));
            }
        }
    }

    /// Start observing a target with the given options.
    ///
    /// Re-observing the same target replaces that observer's options (WHATWG DOM
    /// §4.3.1 `observe()` step 7.2: existing registered observer is updated).
    pub fn observe(
        &mut self,
        dom: &mut EcsDom,
        id: MutationObserverId,
        target: Entity,
        init: MutationObserverInit,
    ) {
        // Ignore observe for an unregistered observer id (no record queue):
        // keeps registry state and per-entity registrations consistent so a
        // stale `MutationObservedBy` can't accumulate with its notifications
        // silently dropped. Restores the pre-refactor registry-lookup guard.
        if !self.records.contains_key(&id) {
            return;
        }
        // §4.3.1 `observe()` step 7: "for each registered of target's registered
        // observer list, if registered's observer is this" — a **transient**
        // registered observer is a registered observer, so step 7 matches it too
        // (not just a permanent one). Collect every matching registration's
        // `reg_id` on `target` (immutable read first). Step 8 ("Otherwise") adds a
        // new permanent registration **only if NONE matched** — so observing a node
        // that already carries a (transient or permanent) registration for this
        // observer updates that registration in place and does NOT add a spurious
        // permanent (which would otherwise outlive the next microtask's step-6.3
        // clear of the transient).
        let matched_reg_ids: Vec<u64> = dom
            .world()
            .get::<&MutationObservedBy>(target)
            .map(|c| {
                c.0.iter()
                    .filter(|o| o.observer == id)
                    .map(|o| o.reg_id)
                    .collect()
            })
            .unwrap_or_default();

        if matched_reg_ids.is_empty() {
            // step 8: add a new permanent registration. Split the existence check
            // from the mutation so the `Ok`-arm borrow does not extend into the
            // insert arm (NLL temporary lifetime).
            let reg_id = self.alloc_reg_id();
            let entry = MutationObservation {
                observer: id,
                init,
                transient: false,
                source: None,
                reg_id,
            };
            let has_comp = dom.world().get::<&MutationObservedBy>(target).is_ok();
            if has_comp {
                if let Ok(mut comp) = dom.world_mut().get::<&mut MutationObservedBy>(target) {
                    comp.0.push(entry);
                }
            } else {
                let _ = dom
                    .world_mut()
                    .insert_one(target, MutationObservedBy(vec![entry]));
            }
            return;
        }
        // step 7.1: for each matched registration, remove the transient registered
        // observers whose source is it (keyed on that registration's `reg_id`),
        // tree-wide. (A matched permanent's reg_id sources the transients spawned
        // from it; a matched transient's reg_id sources any chained transients.)
        // The tree-wide sweep is equivalent to the spec's "for each node of this's
        // node list" scope by construction: any node carrying a transient with
        // `source == reg_id` necessarily carries a registration for this observer,
        // so it is in the (query-derived) node list — the spec's notify step 6.3
        // is node-list-scoped for the same reason.
        for reg_id in &matched_reg_ids {
            clear_transient_observers_by_source(dom, *reg_id);
        }
        // step 7.2: set each surviving matched registration's options to `init`
        // (the matched registration keeps its `transient`-ness and `reg_id`).
        if let Ok(mut comp) = dom.world_mut().get::<&mut MutationObservedBy>(target) {
            for o in comp.0.iter_mut().filter(|o| o.observer == id) {
                o.init = init.clone();
            }
        }
    }

    /// Stop observing all targets for this observer.
    ///
    /// Per WHATWG DOM §4.3.1: empties both the node list and the record queue.
    pub fn disconnect(&mut self, dom: &mut EcsDom, id: MutationObserverId) {
        retain_observations(dom, |o| o.observer != id);
        if let Some(queue) = self.records.get_mut(&id) {
            queue.clear();
        }
    }

    /// Take all pending records for this observer.
    pub fn take_records(&mut self, id: MutationObserverId) -> Vec<MutationRecord> {
        self.records
            .get_mut(&id)
            .map(std::mem::take)
            .unwrap_or_default()
    }

    /// Remove the observer entirely (drops its registrations and record queue).
    pub fn unregister(&mut self, dom: &mut EcsDom, id: MutationObserverId) {
        self.disconnect(dom, id);
        self.records.remove(&id);
        self.pending.retain(|o| *o != id);
    }

    /// Drop the registry-internal bookkeeping for an observer whose JS wrapper
    /// was GC-collected (binding row swept). Dom-free: a GC-collected observer is
    /// guaranteed non-observing (its observation components are already gone), so
    /// no target-list scrub is needed — mirrors [`Self::clear_pending_records`]'s
    /// rationale. Called only from the `gc/collect.rs` binding-row sweep.
    ///
    /// Removes the (necessarily empty — a collected observer is non-pending, so
    /// its record queue drained) `records` row and its `pending` membership.
    /// `next_id` is left untouched (monotonic id allocator).
    ///
    /// **Internal VM-integration helper — not a supported public API** (hence
    /// `#[doc(hidden)]`). The GC-only precondition — the observer is already
    /// proven collected (non-observing + non-pending) — is the caller's
    /// obligation; call only from the `gc/collect.rs` binding-row sweep.
    #[doc(hidden)]
    pub fn retire_collected(&mut self, id: MutationObserverId) {
        self.records.remove(&id);
        self.pending.retain(|o| *o != id);
    }

    /// Number of registry-internal `records` rows (one per still-registered
    /// observer). VM-integration + test oracle for the GC-sweep
    /// [`Self::retire_collected`] retirement — the private `records` map has no
    /// public reader. `#[doc(hidden)]` (not part of the supported API surface),
    /// mirroring [`Self::clear_pending_records`]'s VM-integration-helper marking.
    #[doc(hidden)]
    #[must_use]
    pub fn records_len(&self) -> usize {
        self.records.len()
    }

    /// Drain every observer's pending records without dropping observer ids.
    ///
    /// **Internal VM-integration helper — not a supported public API** (hence
    /// `#[doc(hidden)]`); call only from a VM bind/unbind cycle. Records
    /// reference `Entity` values from the outgoing world, so they must not be
    /// delivered after rebind. Unlike the old `clear_all_targets`, no
    /// target-list scrub is needed — the observation components vanish with the
    /// despawned world, so there is no Entity-index-collision hazard.
    #[doc(hidden)]
    pub fn clear_pending_records(&mut self) {
        for queue in self.records.values_mut() {
            queue.clear();
        }
        // Drop the pending-mutation-observers set too: its ids reference the
        // outgoing world's delivery cycle and must not survive a rebind.
        self.pending.clear();
    }

    /// Queue matching records from a session-level `MutationRecord` to all
    /// interested observers.
    ///
    /// Implements WHATWG DOM §4.3.2 "queue a mutation record" (the section title
    /// is "Queuing a mutation record"): walk the target's
    /// inclusive ancestors and, for each node's registered observer list,
    /// determine the interested observers (proper ancestors require `subtree`),
    /// then enqueue **one** record per interested observer.
    ///
    /// Per §4.3.2 step 1, `interestedObservers` is a **map keyed by observer**:
    /// when the same observer matches more than once along the walk — a permanent
    /// registration plus a transient registered observer on a removed node
    /// (§4.2.3 "remove" step 15), or the same observer registered on two
    /// inclusive ancestors — the matches **collapse to a single record** (step 4
    /// enqueues once per observer). The mapped value is the per-observer old
    /// value (step 3.2.3), set when any matching registration requested it for
    /// this record's kind. The map is a WHATWG **ordered map** (insertion =
    /// walk order: target first, then ancestors), so the per-record enqueue —
    /// and thus the `pending` append order that drives §4.3 delivery — follows
    /// the spec's order, not observer-id order.
    ///
    /// Returns `true` if at least one observer's record queue received a record
    /// (so the caller knows whether to "queue a mutation observer microtask",
    /// WHATWG DOM §4.3.2 step 5). `false` means no interested observer — the
    /// caller can skip scheduling the (otherwise no-op) microtask.
    pub fn notify(&mut self, dom: &EcsDom, record: &elidex_script_session::MutationRecord) -> bool {
        use elidex_script_session::MutationKind;

        if !dom.contains(record.target) {
            return false;
        }

        // §4.3.2 step 1: interestedObservers — an insertion-ordered map
        // (observer → mapped old value), preserving walk order for spec-faithful
        // delivery order. Linear search dedups (per-record observer count is tiny).
        let mut interested: Vec<(MutationObserverId, Option<String>)> = Vec::new();
        let mut node = Some(record.target);
        let mut is_target = true;
        // Cap the ancestor walk against a corrupted tree (cycle / self-parent),
        // matching the `MAX_ANCESTOR_DEPTH` guard `EcsDom::is_ancestor_or_self`
        // applies to its own upward walk.
        let mut depth = 0usize;
        while let Some(n) = node {
            if depth >= MAX_ANCESTOR_DEPTH {
                break;
            }
            depth += 1;
            if let Ok(comp) = dom.world().get::<&MutationObservedBy>(n) {
                for obs in &comp.0 {
                    // Proper ancestors only match subtree observers (step 3.2
                    // "node is not target and options['subtree'] is false").
                    if !is_target && !obs.init.subtree {
                        continue;
                    }
                    let init = &obs.init;

                    let kind_matches = match record.kind {
                        MutationKind::ChildList => init.child_list,
                        MutationKind::Attribute => {
                            if !init.attributes {
                                false
                            } else if let (Some(filter), Some(attr)) =
                                (&init.attribute_filter, &record.attribute_name)
                            {
                                filter.iter().any(|f| f == attr)
                            } else {
                                init.attributes
                            }
                        }
                        MutationKind::CharacterData => init.character_data,
                        // CssRule (and future variants) are not observed by
                        // MutationObserver. `el.style.*` mutations write back
                        // through `sync_to_attribute` → `EcsDom::set_attribute`
                        // (CSSOM §6.6 "update style attribute for"), and will
                        // surface as Attribute records once ECS
                        // attribute-change events are translated into session
                        // MutationRecords — today only session-level mutation
                        // producers reach this registry.
                        _ => false,
                    };
                    if !kind_matches {
                        continue;
                    }
                    // Only observers with a live record queue are interested (the
                    // JS observer object still exists / is registered).
                    if !self.records.contains_key(&obs.observer) {
                        continue;
                    }

                    // step 3.2.2: default the entry to null old value if absent
                    // (insert at end → walk-order preserved).
                    let idx =
                        if let Some(i) = interested.iter().position(|(o, _)| *o == obs.observer) {
                            i
                        } else {
                            interested.push((obs.observer, None));
                            interested.len() - 1
                        };
                    let slot = &mut interested[idx].1;
                    // step 3.2.3: this registration requests the old value for
                    // this kind → record it. The record-level `old_value` is the
                    // same for every matching registration, so a later match never
                    // changes a value already set, and a non-requesting match
                    // never resets one (matches the spec's "set ... to oldValue").
                    let wants_old = match record.kind {
                        MutationKind::Attribute => init.attribute_old_value,
                        MutationKind::CharacterData => init.character_data_old_value,
                        _ => false,
                    };
                    if wants_old {
                        slot.clone_from(&record.old_value);
                    }
                }
            }
            node = dom.get_parent(n);
            is_target = false;
        }

        if interested.is_empty() {
            return false;
        }
        let mutation_type = match record.kind {
            MutationKind::ChildList => "childList",
            MutationKind::Attribute => "attributes",
            MutationKind::CharacterData => "characterData",
            // Unreachable: `interested` only gains entries for the three observed
            // kinds above (`CssRule` never sets `kind_matches`).
            _ => return false,
        };
        // step 4: enqueue one record per interested observer, and append it to
        // the agent's pending mutation observers (step 4.3) preserving append
        // order (ordered set — push only if not already present).
        let mut enqueued = false;
        for (observer, old_value) in interested {
            if let Some(queue) = self.records.get_mut(&observer) {
                queue.push(MutationRecord {
                    mutation_type: mutation_type.to_string(),
                    target: record.target,
                    added_nodes: record.added_nodes.clone(),
                    removed_nodes: record.removed_nodes.clone(),
                    previous_sibling: record.previous_sibling,
                    next_sibling: record.next_sibling,
                    attribute_name: record.attribute_name.clone(),
                    old_value,
                });
                if !self.pending.contains(&observer) {
                    self.pending.push(observer);
                }
                enqueued = true;
            }
        }
        enqueued
    }

    /// Returns `true` if any observer has pending records.
    #[must_use]
    pub fn has_pending_records(&self) -> bool {
        self.records.values().any(|queue| !queue.is_empty())
    }

    /// The raw ids of the observers that have ≥1 **pending undelivered record**
    /// — i.e. a NON-EMPTY entry in the per-observer record queue (`records`).
    ///
    /// This is the second GC-keepalive clause the observer arm marshals (S5-3c):
    /// an observer with a queued record awaiting the "notify mutation observers"
    /// microtask (WHATWG DOM §4.3.2 "Queuing a mutation record" enqueues the
    /// record + queues the §4.3 "notify mutation observers" microtask) must stay
    /// alive to deliver it, even if its last *observation* just ended (its target
    /// despawned / it was `disconnect()`ed after the record queued but before the
    /// microtask ran). Losing the wrapper in that window would drop the queued
    /// records (the delivery path takes the records, then a missing binding row
    /// silently discards them) = observable data loss. This is the exact analogue
    /// of the SSE §9.2.9 "task queued on the remote event task source" strong-
    /// reference clause (`es_keepalive`'s `has_queued_task`).
    ///
    /// Keyed on NON-EMPTY `records`, **not** on `pending` membership: `takeRecords()`
    /// empties an observer's record queue (`take_records`) but deliberately does
    /// NOT remove it from `pending` (so the microtask still runs the step-6.3
    /// transient clear for it). An observer whose queue the page already drained
    /// via `takeRecords()` has nothing left to deliver, so it does NOT need the
    /// pending-record keepalive — keying on `records` (the precise "has undelivered
    /// data" signal) avoids over-keeping a stale-pending, empty-queue observer.
    /// This reads only the registry (HostData), NO World access — so it is valid
    /// regardless of bound / unbound.
    #[must_use]
    pub fn observers_with_pending_records(&self) -> std::collections::HashSet<u64> {
        self.records
            .iter()
            .filter(|(_, queue)| !queue.is_empty())
            .map(|(id, _)| id.raw())
            .collect()
    }

    /// Take the agent's **pending mutation observers** as `notifySet` (WHATWG DOM
    /// §4.3 "notify mutation observers" step 2 clone + step 3 empty), in **append
    /// order** (the ordered-set order each observer first received a record this
    /// cycle), matching the spec's notifySet iteration — not observer-id order.
    ///
    /// This is the delivery-cycle's observer set, **not** derived from record-queue
    /// non-emptiness: an observer whose queue the page drained via `takeRecords()`
    /// is still returned here, so the microtask runs step 6.3 (transient clearing)
    /// for it. Draining the set models steps 2–3 (clone then empty); records
    /// queued during a callback re-populate it for the next microtask.
    pub fn take_pending_observers(&mut self) -> Vec<MutationObserverId> {
        std::mem::take(&mut self.pending)
    }
}

/// Drop registered-observer entries failing `keep` from every node, removing any
/// `MutationObservedBy` component left empty.
///
/// The shared scaffold for the whole-tree retain queries
/// ([`MutationObserverRegistry::disconnect`], [`clear_transient_observers`]),
/// keeping the empty-component GC invariant in one place. No observer→nodes
/// reverse index — the registered observers live on the nodes (ECS-native).
fn retain_observations(dom: &mut EcsDom, mut keep: impl FnMut(&MutationObservation) -> bool) {
    let mut emptied: Vec<Entity> = Vec::new();
    for (entity, comp) in &mut dom.world_mut().query::<(Entity, &mut MutationObservedBy)>() {
        comp.0.retain(&mut keep);
        if comp.0.is_empty() {
            emptied.push(entity);
        }
    }
    for entity in emptied {
        let _ = dom.world_mut().remove_one::<MutationObservedBy>(entity);
    }
}

/// Remove every transient registered observer whose observer is `observer`
/// (WHATWG DOM §4.3 "notify mutation observers" step 6.3 — "remove all transient
/// registered observers whose observer is mo").
///
/// One ECS query over `MutationObservedBy` (mirroring
/// [`MutationObserverRegistry::disconnect`]'s retain-query) drops the matching
/// transient entries and removes any component left empty. No observer→nodes
/// reverse index is needed — the transients live on the nodes themselves.
pub fn clear_transient_observers(dom: &mut EcsDom, observer: MutationObserverId) {
    retain_observations(dom, |o| !(o.transient && o.observer == observer));
}

/// Remove the transient registered observers whose `source` is the registration
/// identified by `source_reg_id` (WHATWG DOM §4.3.1 `observe()` step 7.1 —
/// "remove all transient registered observers whose source is registered").
/// Keying on the registration `reg_id` (not the ancestor entity) distinguishes
/// multiple registrations of the same observer on one node, so transients sourced
/// from a *different* registration survive.
pub fn clear_transient_observers_by_source(dom: &mut EcsDom, source_reg_id: u64) {
    retain_observations(dom, |o| !(o.transient && o.source == Some(source_reg_id)));
}

/// Remove **all** transient registered observers from every node, regardless of
/// observer. The delivery cycle that owns them was discarded (a `Vm::unbind`
/// before the notify microtask cleared them), so leaving them would let a
/// same-DOM rebind deliver future detached-subtree mutations through a stale
/// transient. Permanent registrations are untouched (they are despawned with the
/// outgoing world, or legitimately persist for a same-DOM rebind).
pub fn clear_all_transient_observers(dom: &mut EcsDom) {
    retain_observations(dom, |o| !o.transient);
}

/// The raw ids of the mutation observers that currently have ≥1 active
/// observation — the WHATWG DOM §4.3 **registered-observer-list membership**
/// (an observer is reachable from every node whose registered observer list it
/// is in). Derived in one hecs archetype query over the live per-entity
/// `MutationObservedBy` components, flat-mapping every observation's `observer`
/// id (permanent **or** transient — a transient registered observer is a
/// registered observer, §4.3) into the set.
///
/// This is the **membership disjunct** of the GC-keepalive predicate
/// `elidex-js` marshals (S5-3c): a `MutationObserver` wrapper stays rooted if
/// its id is in this set OR in `observers_with_pending_records` (the
/// queued-undelivered-record clause; the full predicate lives at the seam,
/// `elidex-js` `gc/keepalive.rs`). **Despawn-
/// safe by construction** — a despawned entity's `MutationObservedBy` is gone
/// with the entity, so a stale (observer, despawned-target) pair is never
/// scanned; the observer's membership drops to zero the instant its sole
/// observed entity despawns, with no registry decrement hook.
#[must_use]
pub fn observing_observer_ids(dom: &EcsDom) -> std::collections::HashSet<u64> {
    let mut ids = std::collections::HashSet::new();
    for (_entity, comp) in &mut dom.world().query::<(Entity, &MutationObservedBy)>() {
        for obs in &comp.0 {
            ids.insert(obs.observer.raw());
        }
    }
    ids
}

#[cfg(test)]
mod tests_core;

#[cfg(test)]
mod tests_transient;
