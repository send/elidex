//! `MutationObserver` API (DOM Standard §4.3).
//!
//! Observes changes to the DOM tree (child list, attributes, character data).

use elidex_ecs::Entity;

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

/// Registry for active mutation observers.
#[derive(Debug, Default)]
pub struct MutationObserverRegistry {
    next_id: u64,
    observers: Vec<MutationObserverEntry>,
}

#[derive(Debug)]
struct MutationObserverEntry {
    id: MutationObserverId,
    targets: Vec<(Entity, MutationObserverInit)>,
    records: Vec<MutationRecord>,
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
        self.observers.push(MutationObserverEntry {
            id,
            targets: Vec::new(),
            records: Vec::new(),
        });
        id
    }

    /// Start observing a target with the given options.
    pub fn observe(&mut self, id: MutationObserverId, target: Entity, init: MutationObserverInit) {
        if let Some(entry) = self.observers.iter_mut().find(|e| e.id == id) {
            // Replace existing observation on the same target.
            if let Some(existing) = entry.targets.iter_mut().find(|(e, _)| *e == target) {
                existing.1 = init;
            } else {
                entry.targets.push((target, init));
            }
        }
    }

    /// Stop observing all targets for this observer.
    ///
    /// Per WHATWG DOM §4.3.3.3: empties both the node list and the record queue.
    pub fn disconnect(&mut self, id: MutationObserverId) {
        if let Some(entry) = self.observers.iter_mut().find(|e| e.id == id) {
            entry.targets.clear();
            entry.records.clear();
        }
    }

    /// Take all pending records for this observer.
    pub fn take_records(&mut self, id: MutationObserverId) -> Vec<MutationRecord> {
        self.observers
            .iter_mut()
            .find(|e| e.id == id)
            .map(|e| std::mem::take(&mut e.records))
            .unwrap_or_default()
    }

    /// Remove the observer entirely.
    pub fn unregister(&mut self, id: MutationObserverId) {
        self.observers.retain(|e| e.id != id);
    }

    /// Remove a destroyed entity from all observer target lists.
    ///
    /// Call this when an entity is removed from the DOM to prevent stale references.
    pub fn remove_entity(&mut self, entity: Entity) {
        for entry in &mut self.observers {
            entry.targets.retain(|(e, _)| *e != entity);
        }
    }

    /// Drain every observer's target list and pending records,
    /// keeping the observer IDs registered.  Called from a VM
    /// `unbind` step so a retained `MutationObserver` reference can
    /// be re-bound to a fresh DOM without inheriting stale `Entity`
    /// indices from the prior world (two `EcsDom::new()` worlds
    /// share `Entity` index space — without this the post-rebind
    /// `notify` would match a `target` Entity that happens to
    /// collide with an old observation).  Observer IDs themselves
    /// are monotonic per-VM and stay live so post-unbind method
    /// calls on retained instances continue to brand-check.
    pub fn clear_all_targets(&mut self) {
        for entry in &mut self.observers {
            entry.targets.clear();
            entry.records.clear();
        }
    }

    /// Queue matching records from a session-level `MutationRecord` to all interested observers.
    ///
    /// `is_descendant_fn` checks whether a `target` is a descendant of an `ancestor`
    /// (needed for `subtree` option matching).
    pub fn notify(
        &mut self,
        record: &elidex_script_session::MutationRecord,
        is_descendant_fn: &dyn Fn(Entity, Entity) -> bool,
    ) {
        use elidex_script_session::MutationKind;

        for entry in &mut self.observers {
            for &(observed_target, ref init) in &entry.targets {
                // Check if this observer's target matches the record target.
                let target_matches = record.target == observed_target
                    || (init.subtree && is_descendant_fn(record.target, observed_target));
                if !target_matches {
                    continue;
                }

                // Check if the mutation kind is observed.
                let kind_matches = match record.kind {
                    MutationKind::ChildList => init.child_list,
                    MutationKind::Attribute => {
                        if !init.attributes {
                            false
                        } else if let (Some(ref filter), Some(ref attr)) =
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
                    MutationKind::Attribute if init.attribute_old_value => record.old_value.clone(),
                    MutationKind::CharacterData if init.character_data_old_value => {
                        record.old_value.clone()
                    }
                    _ => None,
                };

                entry.records.push(MutationRecord {
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

    /// Returns `true` if any observer has pending records.
    #[must_use]
    pub fn has_pending_records(&self) -> bool {
        self.observers.iter().any(|e| !e.records.is_empty())
    }

    /// Returns an iterator of observer IDs that have pending records.
    pub fn observers_with_records(&self) -> impl Iterator<Item = MutationObserverId> + '_ {
        self.observers
            .iter()
            .filter(|e| !e.records.is_empty())
            .map(|e| e.id)
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

        reg.notify(&session_record, &|_, _| false);

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
        reg.notify(&record_class, &|_, _| false);

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
        reg.notify(&record_id, &|_, _| false);

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
        reg.notify(&record, &|_, _| false);

        reg.disconnect(id);
        assert!(!reg.has_pending_records());
        assert!(reg.take_records(id).is_empty());
    }

    #[test]
    fn remove_entity_cleans_targets() {
        let mut dom = EcsDom::new();
        let el = elem(&mut dom, "div");

        let mut reg = MutationObserverRegistry::new();
        let id = reg.register();
        reg.observe(
            id,
            el,
            MutationObserverInit {
                child_list: true,
                ..Default::default()
            },
        );

        reg.remove_entity(el);

        // Notify should not match since the target is removed.
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
        reg.notify(&record, &|_, _| false);
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
            id,
            parent,
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
            old_value: None,
        };
        // is_descendant_fn returns true for child→parent.
        reg.notify(&record, &|target, ancestor| {
            target == child && ancestor == parent
        });

        let records = reg.take_records(id);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].target, child);
    }
}
