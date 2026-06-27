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

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom};
    use elidex_script_session::{MutationKind, MutationRecord as SessionRecord};

    fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
        dom.create_element(tag, Attributes::default())
    }

    #[test]
    fn notify_child_list() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "span");

        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        reg.observe(
            &mut dom,
            id,
            parent,
            MutationObserverInit {
                child_list: true,
                ..Default::default()
            },
        );

        let session_record = SessionRecord {
            kind: MutationKind::ChildList,
            target: parent,
            added_nodes: vec![child],
            removed_nodes: vec![],
            previous_sibling: None,
            next_sibling: None,
            attribute_name: None,
            old_value: None,
        };

        reg.notify(&dom, &session_record);

        assert!(reg.has_pending_records());
        let records = reg.take_records(id);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].mutation_type, "childList");
        assert_eq!(records[0].added_nodes, vec![child]);
    }

    #[test]
    fn notify_attribute_with_filter() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        reg.observe(
            &mut dom,
            id,
            el,
            MutationObserverInit {
                attributes: true,
                attribute_filter: Some(vec!["class".to_string()]),
                attribute_old_value: true,
                ..Default::default()
            },
        );

        // "class" attribute should match.
        let record_class = SessionRecord {
            kind: MutationKind::Attribute,
            target: el,
            added_nodes: vec![],
            removed_nodes: vec![],
            previous_sibling: None,
            next_sibling: None,
            attribute_name: Some("class".to_string()),
            old_value: Some("old".to_string()),
        };
        reg.notify(&dom, &record_class);

        // "id" attribute should NOT match the filter.
        let record_id = SessionRecord {
            kind: MutationKind::Attribute,
            target: el,
            added_nodes: vec![],
            removed_nodes: vec![],
            previous_sibling: None,
            next_sibling: None,
            attribute_name: Some("id".to_string()),
            old_value: Some("test".to_string()),
        };
        reg.notify(&dom, &record_id);

        let records = reg.take_records(id);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].attribute_name, Some("class".to_string()));
        assert_eq!(records[0].old_value, Some("old".to_string()));
    }

    #[test]
    fn disconnect_clears_records_and_targets() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        reg.observe(
            &mut dom,
            id,
            el,
            MutationObserverInit {
                child_list: true,
                ..Default::default()
            },
        );

        let record = SessionRecord {
            kind: MutationKind::ChildList,
            target: el,
            added_nodes: vec![],
            removed_nodes: vec![],
            previous_sibling: None,
            next_sibling: None,
            attribute_name: None,
            old_value: None,
        };
        reg.notify(&dom, &record);

        reg.disconnect(&mut dom, id);
        assert!(!reg.has_pending_records());
        assert!(reg.take_records(id).is_empty());

        // Targets cleared: a post-disconnect notify matches nothing.
        reg.notify(&dom, &record);
        assert!(!reg.has_pending_records());
    }

    #[test]
    fn despawn_auto_cleans_targets() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        reg.observe(
            &mut dom,
            id,
            el,
            MutationObserverInit {
                child_list: true,
                ..Default::default()
            },
        );

        // Despawning the entity drops its MutationObservedBy component, so the
        // observer no longer matches — no manual scrub needed.
        let _ = dom.destroy_entity(el);

        let record = SessionRecord {
            kind: MutationKind::ChildList,
            target: el,
            added_nodes: vec![],
            removed_nodes: vec![],
            previous_sibling: None,
            next_sibling: None,
            attribute_name: None,
            old_value: None,
        };
        reg.notify(&dom, &record);
        assert!(!reg.has_pending_records());
    }

    #[test]
    fn subtree_observer() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "span");
        let _ = dom.append_child(parent, child);

        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        reg.observe(
            &mut dom,
            id,
            parent,
            MutationObserverInit {
                attributes: true,
                subtree: true,
                ..Default::default()
            },
        );

        // Record on the child is matched via inclusive-ancestor walk to the
        // parent's subtree observer.
        let record = SessionRecord {
            kind: MutationKind::Attribute,
            target: child,
            added_nodes: vec![],
            removed_nodes: vec![],
            previous_sibling: None,
            next_sibling: None,
            attribute_name: Some("class".to_string()),
            old_value: None,
        };
        reg.notify(&dom, &record);

        let records = reg.take_records(id);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].target, child);
    }

    #[test]
    fn two_observers_same_target() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = MutationObserverRegistry::new();
        let id_a = reg.register();
        let id_b = reg.register();
        let init = MutationObserverInit {
            child_list: true,
            ..Default::default()
        };
        reg.observe(&mut dom, id_a, el, init.clone());
        reg.observe(&mut dom, id_b, el, init);

        let record = SessionRecord {
            kind: MutationKind::ChildList,
            target: el,
            added_nodes: vec![],
            removed_nodes: vec![],
            previous_sibling: None,
            next_sibling: None,
            attribute_name: None,
            old_value: None,
        };
        reg.notify(&dom, &record);

        assert_eq!(reg.take_records(id_a).len(), 1);
        assert_eq!(reg.take_records(id_b).len(), 1);
    }

    #[test]
    fn observe_unregistered_id_is_noop() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = MutationObserverRegistry::new();
        // `ghost` is never register()'d — observe must not attach a stale
        // component nor leave registry state inconsistent.
        let ghost = MutationObserverId::from_raw(999);
        reg.observe(
            &mut dom,
            ghost,
            el,
            MutationObserverInit {
                child_list: true,
                ..Default::default()
            },
        );
        assert!(
            dom.world().get::<&MutationObservedBy>(el).is_err(),
            "observe on an unregistered id must not attach a component"
        );
    }

    #[test]
    fn take_pending_observers_drains_in_append_order() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = MutationObserverRegistry::new();
        let init = MutationObserverInit {
            child_list: true,
            ..Default::default()
        };
        let mut ids = Vec::new();
        for _ in 0..4 {
            let id = reg.register();
            reg.observe(&mut dom, id, el, init.clone());
            ids.push(id.raw());
        }

        let record = SessionRecord {
            kind: MutationKind::ChildList,
            target: el,
            added_nodes: vec![],
            removed_nodes: vec![],
            previous_sibling: None,
            next_sibling: None,
            attribute_name: None,
            old_value: None,
        };
        reg.notify(&dom, &record);

        // One record on one target enqueues all four in registered-observer-list
        // (= walk = registration) order; here that is also `ids`. The id-vs-append
        // *divergence* is covered by `pending_observers_preserve_append_order_not_id_order`.
        let got: Vec<u64> = reg
            .take_pending_observers()
            .into_iter()
            .map(MutationObserverId::raw)
            .collect();
        assert_eq!(got, ids, "delivery follows append (registration) order");
        // Draining the notifySet leaves it empty (step 3 "empty pending").
        assert!(reg.take_pending_observers().is_empty());
    }

    /// Codex R1 (P2): the §4.3 notifySet is the pending-mutation-observers set,
    /// NOT derived from record-queue contents — so `takeRecords()` draining an
    /// observer's queue before the microtask must NOT drop it from `notifySet`
    /// (else its step-6.3 transient clear is skipped and the transient leaks).
    #[test]
    fn pending_observer_survives_take_records() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");
        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        reg.observe(
            &mut dom,
            id,
            el,
            MutationObserverInit {
                child_list: true,
                ..Default::default()
            },
        );

        let child = elem(&mut dom, "span");
        let record = child_list_added(el, vec![child]);
        reg.notify(&dom, &record);

        // The page calls takeRecords() before the microtask — empties the queue
        // but must not remove the observer from the pending notifySet.
        assert_eq!(reg.take_records(id).len(), 1);
        assert!(!reg.has_pending_records(), "record queue drained");
        assert_eq!(
            reg.take_pending_observers(),
            vec![id],
            "observer is still in the notifySet after takeRecords()"
        );
    }

    /// Codex R1 (P2) + R8-L1: observing a node that holds ONLY a transient
    /// registration matches it in §4.3.1 step 7 (a transient IS a registered
    /// observer), so step 8 does NOT add a spurious permanent — the transient is
    /// updated in place and lapses on the next microtask. The step-7.1 clear is
    /// source-keyed, so an unrelated removed subtree's transient is NOT cleared.
    #[test]
    fn observe_on_transient_only_node_keeps_other_transients() {
        let mut dom = EcsDom::new();
        let root = elem(&mut dom, "div");
        let a = elem(&mut dom, "div");
        let b = elem(&mut dom, "div");
        let _ = dom.append_child(root, a);
        let _ = dom.append_child(root, b);

        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        reg.observe(
            &mut dom,
            id,
            root,
            MutationObserverInit {
                child_list: true,
                subtree: true,
                ..Default::default()
            },
        );

        // Remove both children (coalesced removal) → each gets a transient for id.
        assert!(dom.remove_child(root, a));
        assert!(dom.remove_child(root, b));
        reg.add_transient_observers(&mut dom, root, &[a, b]);
        assert_eq!(count_entries(&dom, a, id, true), 1);
        assert_eq!(count_entries(&dom, b, id, true), 1);

        // Observe `a` (which has ONLY a transient for id) before the microtask.
        reg.observe(
            &mut dom,
            id,
            a,
            MutationObserverInit {
                child_list: true,
                ..Default::default()
            },
        );
        // §4.3.1 step 7 matches `a`'s transient → step 8 does NOT add a permanent;
        // `a` stays transient (and lapses next microtask). `b`'s unrelated transient
        // (different source) is NOT cleared.
        assert_eq!(
            count_entries(&dom, a, id, false),
            0,
            "no spurious permanent added (step 7 matched the transient)"
        );
        assert_eq!(
            count_entries(&dom, a, id, true),
            1,
            "a's transient updated in place"
        );
        assert_eq!(
            count_entries(&dom, b, id, true),
            1,
            "b's transient survives"
        );
    }

    // ---- Transient registered observers (DOM §4.3 / §4.2.3 step 15) ----

    /// A `ChildList` mutation record on `target` adding `added`.
    fn child_list_added(target: Entity, added: Vec<Entity>) -> SessionRecord {
        SessionRecord {
            kind: MutationKind::ChildList,
            target,
            added_nodes: added,
            removed_nodes: vec![],
            previous_sibling: None,
            next_sibling: None,
            attribute_name: None,
            old_value: None,
        }
    }

    /// Count entries on `node`'s registered observer list matching `observer`
    /// and `transient`-ness.
    fn count_entries(
        dom: &EcsDom,
        node: Entity,
        observer: MutationObserverId,
        transient: bool,
    ) -> usize {
        dom.world().get::<&MutationObservedBy>(node).map_or(0, |c| {
            c.0.iter()
                .filter(|o| o.observer == observer && o.transient == transient)
                .count()
        })
    }

    /// Headline §1 scenario: remove a subtree-observed mid-node, then mutate the
    /// detached subtree — the mutation still reaches the ancestor's observer via
    /// the transient, and is cleared after delivery.
    #[test]
    fn transient_keeps_detached_subtree_observed_until_cleared() {
        let mut dom = EcsDom::new();
        let grandparent = elem(&mut dom, "div");
        let parent = elem(&mut dom, "div");
        let mid = elem(&mut dom, "div");
        let _ = dom.append_child(grandparent, parent);
        let _ = dom.append_child(parent, mid);

        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        reg.observe(
            &mut dom,
            id,
            grandparent,
            MutationObserverInit {
                child_list: true,
                subtree: true,
                ..Default::default()
            },
        );

        // removeChild(mid): detach, then create transients (the notify_one order).
        assert!(dom.remove_child(parent, mid));
        reg.add_transient_observers(&mut dom, parent, &[mid]);

        // mid carries one transient for `id` carrying the original subtree init.
        {
            let comp = dom
                .world()
                .get::<&MutationObservedBy>(mid)
                .expect("transient appended to removed node");
            let t = comp
                .0
                .iter()
                .find(|o| o.observer == id && o.transient)
                .expect("transient entry for the ancestor observer");
            assert!(t.init.subtree, "transient carries original subtree:true");
            assert!(t.init.child_list);
        }

        // Mutating the detached subtree (mid.appendChild(x)) reaches `id`.
        let x = elem(&mut dom, "span");
        let record = child_list_added(mid, vec![x]);
        assert!(reg.notify(&dom, &record));
        assert_eq!(reg.take_records(id).len(), 1);

        // Clearing the transient (delivery step 6.3) stops further delivery.
        clear_transient_observers(&mut dom, id);
        assert_eq!(count_entries(&dom, mid, id, true), 0);
        assert!(!reg.notify(&dom, &record));
    }

    /// The transient is appended to the removed node ONLY (not its descendants);
    /// a mutation deep inside the detached subtree still reaches the observer
    /// because `notify` walks the mutated node's inclusive ancestors up through
    /// the removed node, which now carries the transient (subtree:true).
    #[test]
    fn transient_on_removed_node_catches_deep_descendant_mutation() {
        let mut dom = EcsDom::new();
        let root = elem(&mut dom, "div");
        let mid = elem(&mut dom, "div");
        let child = elem(&mut dom, "div");
        let grandchild = elem(&mut dom, "div");
        let _ = dom.append_child(root, mid);
        let _ = dom.append_child(mid, child);
        let _ = dom.append_child(child, grandchild);

        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        reg.observe(
            &mut dom,
            id,
            root,
            MutationObserverInit {
                child_list: true,
                subtree: true,
                ..Default::default()
            },
        );

        assert!(dom.remove_child(root, mid));
        reg.add_transient_observers(&mut dom, root, &[mid]);
        // Only `mid` carries the transient — child/grandchild do not.
        assert_eq!(count_entries(&dom, mid, id, true), 1);
        assert_eq!(count_entries(&dom, child, id, true), 0);
        assert_eq!(count_entries(&dom, grandchild, id, true), 0);

        // A mutation two levels below the removed node still reaches the observer.
        let x = elem(&mut dom, "span");
        let record = child_list_added(grandchild, vec![x]);
        assert!(reg.notify(&dom, &record));
        assert_eq!(reg.take_records(id).len(), 1);
    }

    /// A non-`subtree` ancestor observer creates no transient (step 15 gates on
    /// `options["subtree"]`).
    #[test]
    fn no_transient_for_non_subtree_ancestor() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let mid = elem(&mut dom, "div");
        let _ = dom.append_child(parent, mid);

        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        reg.observe(
            &mut dom,
            id,
            parent,
            MutationObserverInit {
                child_list: true,
                ..Default::default()
            },
        );

        assert!(dom.remove_child(parent, mid));
        reg.add_transient_observers(&mut dom, parent, &[mid]);
        assert!(
            dom.world().get::<&MutationObservedBy>(mid).is_err(),
            "no transient component on the removed node"
        );
    }

    /// A removed node's OWN registered observer is left permanent — only the
    /// inherited ancestor observation is appended as transient.
    #[test]
    fn own_observer_not_made_transient() {
        let mut dom = EcsDom::new();
        let grandparent = elem(&mut dom, "div");
        let parent = elem(&mut dom, "div");
        let mid = elem(&mut dom, "div");
        let _ = dom.append_child(grandparent, parent);
        let _ = dom.append_child(parent, mid);

        let mut reg = MutationObserverRegistry::new();
        let id_own = reg.register();
        let id_anc = reg.register();
        reg.observe(
            &mut dom,
            id_own,
            mid,
            MutationObserverInit {
                child_list: true,
                ..Default::default()
            },
        );
        reg.observe(
            &mut dom,
            id_anc,
            grandparent,
            MutationObserverInit {
                child_list: true,
                subtree: true,
                ..Default::default()
            },
        );

        assert!(dom.remove_child(parent, mid));
        reg.add_transient_observers(&mut dom, parent, &[mid]);

        assert_eq!(
            count_entries(&dom, mid, id_own, false),
            1,
            "own stays permanent"
        );
        assert_eq!(
            count_entries(&dom, mid, id_own, true),
            0,
            "own not transient"
        );
        assert_eq!(
            count_entries(&dom, mid, id_anc, true),
            1,
            "ancestor is transient"
        );
    }

    /// `clear_transient_observers` removes only the named observer's transients.
    #[test]
    fn clear_targets_only_the_named_observer() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let mid = elem(&mut dom, "div");
        let _ = dom.append_child(parent, mid);

        let mut reg = MutationObserverRegistry::new();
        let id_a = reg.register();
        let id_b = reg.register();
        let init = MutationObserverInit {
            child_list: true,
            subtree: true,
            ..Default::default()
        };
        reg.observe(&mut dom, id_a, parent, init.clone());
        reg.observe(&mut dom, id_b, parent, init);

        assert!(dom.remove_child(parent, mid));
        reg.add_transient_observers(&mut dom, parent, &[mid]);
        assert_eq!(count_entries(&dom, mid, id_a, true), 1);
        assert_eq!(count_entries(&dom, mid, id_b, true), 1);

        clear_transient_observers(&mut dom, id_a);
        assert_eq!(count_entries(&dom, mid, id_a, true), 0, "a cleared");
        assert_eq!(count_entries(&dom, mid, id_b, true), 1, "b kept");
    }

    /// A coalesced removal (replaceChildren / replaceChild) carries its removed
    /// set on one record — every removed subtree-root gets a transient (§2.4).
    #[test]
    fn coalesced_removal_creates_transient_per_removed_node() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let c1 = elem(&mut dom, "div");
        let c2 = elem(&mut dom, "div");
        let _ = dom.append_child(parent, c1);
        let _ = dom.append_child(parent, c2);

        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        reg.observe(
            &mut dom,
            id,
            parent,
            MutationObserverInit {
                child_list: true,
                subtree: true,
                ..Default::default()
            },
        );

        // replaceChildren(): both children removed under one coalesced record.
        assert!(dom.remove_child(parent, c1));
        assert!(dom.remove_child(parent, c2));
        reg.add_transient_observers(&mut dom, parent, &[c1, c2]);

        assert_eq!(count_entries(&dom, c1, id, true), 1);
        assert_eq!(count_entries(&dom, c2, id, true), 1);
    }

    /// §4.3.2 step 1 collapse: a removed node holding both a permanent and a
    /// transient registration for the SAME observer yields ONE record.
    #[test]
    fn notify_collapses_permanent_and_transient_same_observer() {
        let mut dom = EcsDom::new();
        let grandparent = elem(&mut dom, "div");
        let parent = elem(&mut dom, "div");
        let mid = elem(&mut dom, "div");
        let _ = dom.append_child(grandparent, parent);
        let _ = dom.append_child(parent, mid);

        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        // Same observer registered on the ancestor (subtree) AND directly on mid.
        reg.observe(
            &mut dom,
            id,
            grandparent,
            MutationObserverInit {
                child_list: true,
                subtree: true,
                ..Default::default()
            },
        );
        reg.observe(
            &mut dom,
            id,
            mid,
            MutationObserverInit {
                child_list: true,
                ..Default::default()
            },
        );

        assert!(dom.remove_child(parent, mid));
        reg.add_transient_observers(&mut dom, parent, &[mid]);
        // mid now has permanent + transient entries for `id`.
        assert_eq!(count_entries(&dom, mid, id, false), 1);
        assert_eq!(count_entries(&dom, mid, id, true), 1);

        let x = elem(&mut dom, "span");
        let record = child_list_added(mid, vec![x]);
        assert!(reg.notify(&dom, &record));
        assert_eq!(
            reg.take_records(id).len(),
            1,
            "duplicate matches collapse to one record per observer"
        );
    }

    /// §4.3.2 step 3.2.3 old-value union under collapse: when one observer
    /// matches via a registration that requests the old value AND one that does
    /// not, the collapsed record carries the old value regardless of walk order
    /// (a non-requesting match never resets a value a requesting match set).
    #[test]
    fn notify_collapse_old_value_union_is_order_independent() {
        let mut dom = EcsDom::new();
        let grandparent = elem(&mut dom, "div");
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "div");
        let _ = dom.append_child(grandparent, parent);
        let _ = dom.append_child(parent, child);

        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        // Walk order is child→parent→grandparent. The requesting registration is
        // on `parent` (processed FIRST), the non-requesting on `grandparent`
        // (processed SECOND) — so this exercises "later non-requesting match must
        // not reset the already-set old value".
        reg.observe(
            &mut dom,
            id,
            parent,
            MutationObserverInit {
                attributes: true,
                attribute_old_value: true,
                subtree: true,
                ..Default::default()
            },
        );
        reg.observe(
            &mut dom,
            id,
            grandparent,
            MutationObserverInit {
                attributes: true,
                subtree: true,
                ..Default::default()
            },
        );

        let record = SessionRecord {
            kind: MutationKind::Attribute,
            target: child,
            added_nodes: vec![],
            removed_nodes: vec![],
            previous_sibling: None,
            next_sibling: None,
            attribute_name: Some("class".to_string()),
            old_value: Some("old".to_string()),
        };
        assert!(reg.notify(&dom, &record));
        let records = reg.take_records(id);
        assert_eq!(records.len(), 1, "collapsed to one record");
        assert_eq!(
            records[0].old_value,
            Some("old".to_string()),
            "the requesting registration's old value survives the non-requesting match"
        );
    }

    /// §4.3.2 step 1 collapse (pre-existing dup fix): one observer registered on
    /// two inclusive ancestors of the target yields ONE record per mutation.
    #[test]
    fn notify_collapses_observer_on_two_ancestors() {
        let mut dom = EcsDom::new();
        let grandparent = elem(&mut dom, "div");
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "div");
        let _ = dom.append_child(grandparent, parent);
        let _ = dom.append_child(parent, child);

        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        let init = MutationObserverInit {
            child_list: true,
            subtree: true,
            ..Default::default()
        };
        reg.observe(&mut dom, id, grandparent, init.clone());
        reg.observe(&mut dom, id, parent, init);

        let x = elem(&mut dom, "span");
        let record = child_list_added(child, vec![x]);
        assert!(reg.notify(&dom, &record));
        assert_eq!(reg.take_records(id).len(), 1);
    }

    /// Re-observing a target clears that observer's outstanding transients
    /// (§4.3.1 observe step 7.1, source→observer-id collapse).
    #[test]
    fn reobserve_clears_outstanding_transients() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let mid = elem(&mut dom, "div");
        let _ = dom.append_child(parent, mid);

        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        let init = MutationObserverInit {
            child_list: true,
            subtree: true,
            ..Default::default()
        };
        reg.observe(&mut dom, id, parent, init.clone());

        assert!(dom.remove_child(parent, mid));
        reg.add_transient_observers(&mut dom, parent, &[mid]);
        assert_eq!(count_entries(&dom, mid, id, true), 1);

        // Re-observe (any target carrying a registration for `id` triggers 7.1).
        reg.observe(&mut dom, id, parent, init);
        assert_eq!(
            count_entries(&dom, mid, id, true),
            0,
            "re-observe cleared the observer's transients"
        );
    }

    /// Codex R3 (P2): observe() step 7.1 is **source-keyed**. When one observer
    /// holds permanent registrations on two inclusive ancestors, a removed node
    /// gets two transients with different sources; re-observing ONE ancestor must
    /// clear only that ancestor's transient and preserve the other-sourced one.
    #[test]
    fn reobserve_preserves_other_source_transient() {
        let mut dom = EcsDom::new();
        let grandparent = elem(&mut dom, "div");
        let parent = elem(&mut dom, "div");
        let mid = elem(&mut dom, "div");
        let _ = dom.append_child(grandparent, parent);
        let _ = dom.append_child(parent, mid);

        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        let init = MutationObserverInit {
            child_list: true,
            subtree: true,
            ..Default::default()
        };
        // Same observer registered on BOTH inclusive ancestors of `mid`.
        reg.observe(&mut dom, id, grandparent, init.clone());
        reg.observe(&mut dom, id, parent, init.clone());

        assert!(dom.remove_child(parent, mid));
        reg.add_transient_observers(&mut dom, parent, &[mid]);
        // Two transients for `id` on `mid` — sourced from `parent` and `grandparent`.
        assert_eq!(count_entries(&dom, mid, id, true), 2);

        // Re-observe `parent` only → clears the `parent`-sourced transient,
        // leaves the `grandparent`-sourced one.
        reg.observe(&mut dom, id, parent, init);
        assert_eq!(
            count_entries(&dom, mid, id, true),
            1,
            "only the re-observed ancestor's transient is cleared (source-keyed §7.1)"
        );
        // A mutation in the still-detached subtree still reaches `id` via the
        // surviving grandparent-sourced transient.
        let x = elem(&mut dom, "span");
        assert!(reg.notify(&dom, &child_list_added(mid, vec![x])));
        assert_eq!(reg.take_records(id).len(), 1);
    }

    /// Codex R3 (P2): `clear_all_transient_observers` (the unbind scrub) drops
    /// every transient across the tree while leaving permanent registrations.
    #[test]
    fn clear_all_transient_observers_scrubs_transients_keeps_permanent() {
        let mut dom = EcsDom::new();
        let root = elem(&mut dom, "div");
        let mid = elem(&mut dom, "div");
        let _ = dom.append_child(root, mid);

        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        reg.observe(
            &mut dom,
            id,
            root,
            MutationObserverInit {
                child_list: true,
                subtree: true,
                ..Default::default()
            },
        );
        assert!(dom.remove_child(root, mid));
        reg.add_transient_observers(&mut dom, root, &[mid]);
        assert_eq!(count_entries(&dom, mid, id, true), 1);
        assert_eq!(count_entries(&dom, root, id, false), 1);

        clear_all_transient_observers(&mut dom);
        assert_eq!(count_entries(&dom, mid, id, true), 0, "transient scrubbed");
        assert_eq!(
            count_entries(&dom, root, id, false),
            1,
            "permanent registration untouched"
        );
    }

    /// Codex R4-H1 + R8-L1: when one observer holds TWO registrations on a single
    /// node (a permanent + a chained transient, both subtree), the transients they
    /// spawn on a removed descendant carry distinct `source` reg-ids. Re-observing
    /// that node matches BOTH registrations (§4.3.1 step 7 — a transient is a
    /// registered observer), so step 7.1 clears the transients sourced from each →
    /// both chained child transients are cleared. (The reg-id source key is what
    /// makes per-registration step-7.1 possible at all; the ancestor-entity key
    /// could not distinguish the two child transients, both `source == mid`.)
    #[test]
    fn reobserve_matching_both_registrations_clears_both_chained_sources() {
        let mut dom = EcsDom::new();
        let grandparent = elem(&mut dom, "div");
        let parent = elem(&mut dom, "div");
        let mid = elem(&mut dom, "div");
        let child = elem(&mut dom, "div");
        let _ = dom.append_child(grandparent, parent);
        let _ = dom.append_child(parent, mid);
        let _ = dom.append_child(mid, child);

        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        let init = MutationObserverInit {
            child_list: true,
            subtree: true,
            ..Default::default()
        };
        // Same observer on grandparent AND mid.
        reg.observe(&mut dom, id, grandparent, init.clone());
        reg.observe(&mut dom, id, mid, init);

        // Remove mid from parent → mid gets a transient sourced from grandparent's
        // registration (mid also keeps its own permanent registration).
        assert!(dom.remove_child(parent, mid));
        reg.add_transient_observers(&mut dom, parent, &[mid]);
        // Remove child from the now-detached mid → child gets TWO transients: one
        // sourced from mid's permanent registration, one from mid's transient.
        assert!(dom.remove_child(mid, child));
        reg.add_transient_observers(&mut dom, mid, &[child]);
        assert_eq!(count_entries(&dom, child, id, true), 2);

        // Re-observe mid → step 7 matches BOTH mid's permanent and mid's transient
        // (both have observer == id); step 7.1 clears the child transients sourced
        // from each → both cleared.
        reg.observe(
            &mut dom,
            id,
            mid,
            MutationObserverInit {
                child_list: true,
                subtree: true,
                ..Default::default()
            },
        );
        assert_eq!(
            count_entries(&dom, child, id, true),
            0,
            "both child transients (sourced from mid's permanent + transient) cleared"
        );
        // The doubly-detached child no longer reaches the observer.
        let x = elem(&mut dom, "span");
        assert!(!reg.notify(&dom, &child_list_added(child, vec![x])));
    }

    /// Codex R4 (P2, H2): the pending mutation observers set delivers in spec
    /// **append order** (the order each observer first received a record this
    /// cycle), NOT observer-id order — observable cross-observer callback ordering.
    #[test]
    fn pending_observers_preserve_append_order_not_id_order() {
        let mut dom = EcsDom::new();
        let el_a = elem(&mut dom, "div");
        let el_b = elem(&mut dom, "div");

        let mut reg = MutationObserverRegistry::new();
        let id_a = reg.register(); // lower id
        let id_b = reg.register(); // higher id
        let init = MutationObserverInit {
            child_list: true,
            ..Default::default()
        };
        reg.observe(&mut dom, id_a, el_a, init.clone());
        reg.observe(&mut dom, id_b, el_b, init);

        // The higher-id observer (B, on el_b) receives a record FIRST.
        let x = elem(&mut dom, "span");
        reg.notify(&dom, &child_list_added(el_b, vec![x]));
        let y = elem(&mut dom, "span");
        reg.notify(&dom, &child_list_added(el_a, vec![y]));

        assert_eq!(
            reg.take_pending_observers(),
            vec![id_b, id_a],
            "delivery follows append order (B then A), not id order (A then B)"
        );
    }
}
