//! `MutationObserver` API (DOM Standard §4.3).
//!
//! Observes changes to the DOM tree (child list, attributes, character data).
//!
//! ECS-native model: the per-node **registered observer list** (WHATWG DOM
//! §4.3.1) lives as a `MutationObservedBy` component on the observed target
//! entity, mirroring the spec data model. `notify` reproduces §4.3.2 "Queuing a
//! mutation record" by walking the target's inclusive ancestors via
//! [`EcsDom::get_parent`] and reading each node's registered observer list.
//! Only the per-observer pending-record queue (the JS observer object's
//! `[[recordQueue]]` internal slot) is held registry-side, keyed by observer id.

use elidex_ecs::{EcsDom, Entity};

/// A unique identifier for a mutation observer registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// A single registered observer on a node (WHATWG DOM §4.3.1 "registered observer").
#[derive(Debug, Clone)]
struct MutationObservation {
    observer: MutationObserverId,
    init: MutationObserverInit,
}

/// The per-node **registered observer list** (WHATWG DOM §4.3.1), stored as a
/// component on the observed target entity. Removed automatically when the
/// entity is despawned.
#[derive(Debug, Default)]
struct MutationObservedBy(Vec<MutationObservation>);

/// Registry of active mutation observers.
///
/// Holds only the observer-owned pending-record queues; the observation
/// relationship lives on target entities as `MutationObservedBy` components.
#[derive(Debug, Default)]
pub struct MutationObserverRegistry {
    next_id: u64,
    records: std::collections::HashMap<MutationObserverId, Vec<MutationRecord>>,
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

    /// Start observing a target with the given options.
    ///
    /// Re-observing the same target replaces that observer's options (DOM
    /// §4.3.1 `observe()` step: existing registered observer is updated).
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
        if let Ok(mut comp) = dom.world_mut().get::<&mut MutationObservedBy>(target) {
            if let Some(existing) = comp.0.iter_mut().find(|o| o.observer == id) {
                existing.init = init;
            } else {
                comp.0.push(MutationObservation { observer: id, init });
            }
            return;
        }
        let _ = dom.world_mut().insert_one(
            target,
            MutationObservedBy(vec![MutationObservation { observer: id, init }]),
        );
    }

    /// Stop observing all targets for this observer.
    ///
    /// Per WHATWG DOM §4.3.3.3: empties both the node list and the record queue.
    pub fn disconnect(&mut self, dom: &mut EcsDom, id: MutationObserverId) {
        let mut emptied: Vec<Entity> = Vec::new();
        for (entity, comp) in &mut dom.world_mut().query::<(Entity, &mut MutationObservedBy)>() {
            comp.0.retain(|o| o.observer != id);
            if comp.0.is_empty() {
                emptied.push(entity);
            }
        }
        for entity in emptied {
            let _ = dom.world_mut().remove_one::<MutationObservedBy>(entity);
        }
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
    }

    /// Drain every observer's pending records without dropping observer ids.
    ///
    /// Called at a VM unbind boundary: records reference `Entity` values from
    /// the outgoing world, so they must not be delivered after rebind. Unlike
    /// the old `clear_all_targets`, no target-list scrub is needed — the
    /// observation components vanish with the despawned world, so there is no
    /// Entity-index-collision hazard.
    pub fn clear_pending_records(&mut self) {
        for queue in self.records.values_mut() {
            queue.clear();
        }
    }

    /// Queue matching records from a session-level `MutationRecord` to all
    /// interested observers.
    ///
    /// Implements WHATWG DOM §4.3.2 "Queuing a mutation record": walk the
    /// target's inclusive ancestors and, for each node's registered observer
    /// list, queue a record for observers whose options match (proper
    /// ancestors require `subtree`).
    pub fn notify(&mut self, dom: &EcsDom, record: &elidex_script_session::MutationRecord) {
        use elidex_script_session::MutationKind;

        if !dom.contains(record.target) {
            return;
        }

        let mut node = Some(record.target);
        let mut is_target = true;
        while let Some(n) = node {
            if let Ok(comp) = dom.world().get::<&MutationObservedBy>(n) {
                for obs in &comp.0 {
                    // Proper ancestors only match subtree observers.
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
                        // InlineStyle, CssRule, and future variants are not observed by MutationObserver.
                        MutationKind::InlineStyle | MutationKind::CssRule | _ => false,
                    };
                    if !kind_matches {
                        continue;
                    }

                    let mutation_type = match record.kind {
                        MutationKind::ChildList => "childList",
                        MutationKind::Attribute => "attributes",
                        MutationKind::CharacterData => "characterData",
                        _ => continue,
                    };

                    let old_value = match record.kind {
                        MutationKind::Attribute if init.attribute_old_value => {
                            record.old_value.clone()
                        }
                        MutationKind::CharacterData if init.character_data_old_value => {
                            record.old_value.clone()
                        }
                        _ => None,
                    };

                    if let Some(queue) = self.records.get_mut(&obs.observer) {
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
                    }
                }
            }
            node = dom.get_parent(n);
            is_target = false;
        }
    }

    /// Returns `true` if any observer has pending records.
    #[must_use]
    pub fn has_pending_records(&self) -> bool {
        self.records.values().any(|queue| !queue.is_empty())
    }

    /// Returns an iterator of observer IDs that have pending records.
    pub fn observers_with_records(&self) -> impl Iterator<Item = MutationObserverId> + '_ {
        self.records
            .iter()
            .filter(|(_, queue)| !queue.is_empty())
            .map(|(id, _)| *id)
    }
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
}
