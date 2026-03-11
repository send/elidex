//! Core session state coordinating identity mapping and mutation buffering.

use elidex_ecs::{EcsDom, Entity};

use crate::identity_map::IdentityMap;
use crate::mutation::{apply_mutation, Mutation, MutationRecord};
use crate::types::{ComponentKind, JsObjectRef};

/// Central session state for a single script execution context.
///
/// `SessionCore` coordinates:
/// - **Identity mapping** — stable JS object references for ECS entities
/// - **Mutation buffering** — DOM changes are recorded and applied on [`flush()`](Self::flush)
///
/// In Phase 2's single-threaded model, `flush()` is called at the end of each
/// script task (microtask checkpoint).
#[derive(Debug)]
pub struct SessionCore {
    identity: IdentityMap,
    pending: Vec<Mutation>,
}

impl SessionCore {
    /// Create a new session with empty identity map and mutation buffer.
    pub fn new() -> Self {
        Self {
            identity: IdentityMap::new(),
            pending: Vec::new(),
        }
    }

    /// Get or create a JS object wrapper for the given entity and component kind.
    pub fn get_or_create_wrapper(&mut self, entity: Entity, kind: ComponentKind) -> JsObjectRef {
        self.identity.get_or_create(entity, kind)
    }

    /// Record a DOM mutation to be applied on the next [`flush()`](Self::flush).
    pub fn record_mutation(&mut self, mutation: Mutation) {
        self.pending.push(mutation);
    }

    /// Apply all pending mutations to the ECS DOM and return their records.
    ///
    /// The mutation buffer is drained regardless of individual success/failure.
    /// Each mutation is applied in order; failed mutations produce `None` in
    /// the returned vector.
    pub fn flush(&mut self, dom: &mut EcsDom) -> Vec<Option<MutationRecord>> {
        let mutations = std::mem::take(&mut self.pending);
        mutations.iter().map(|m| apply_mutation(m, dom)).collect()
    }

    /// Release all JS object references for the given entity.
    pub fn release(&mut self, entity: Entity) -> usize {
        self.identity.release_entity(entity)
    }

    /// Returns the number of pending (unflushed) mutations.
    #[cfg(test)]
    fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Returns a reference to the identity map.
    pub fn identity_map(&self) -> &IdentityMap {
        &self.identity
    }
}

impl Default for SessionCore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
        dom.create_element(tag, Attributes::default())
    }

    #[test]
    fn record_increments_pending() {
        let mut session = SessionCore::new();
        assert_eq!(session.pending_count(), 0);

        let mut dom = EcsDom::new();
        let p = elem(&mut dom, "div");
        let c = elem(&mut dom, "span");

        session.record_mutation(Mutation::AppendChild {
            parent: p,
            child: c,
        });
        assert_eq!(session.pending_count(), 1);
    }

    #[test]
    fn flush_applies_mutations() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "span");

        let mut session = SessionCore::new();
        session.record_mutation(Mutation::AppendChild { parent, child });

        let records = session.flush(&mut dom);
        assert_eq!(records.len(), 1);
        assert!(records[0].is_some());
        assert_eq!(dom.children(parent), vec![child]);
    }

    #[test]
    fn flush_clears_buffer() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "span");

        let mut session = SessionCore::new();
        session.record_mutation(Mutation::AppendChild { parent, child });
        session.flush(&mut dom);

        assert_eq!(session.pending_count(), 0);
    }

    #[test]
    fn flush_returns_none_for_failed_mutations() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "span");
        dom.append_child(parent, child);

        let mut session = SessionCore::new();
        // Try to append parent to its own child (cycle — will fail).
        session.record_mutation(Mutation::AppendChild {
            parent: child,
            child: parent,
        });
        let records = session.flush(&mut dom);
        assert_eq!(records.len(), 1);
        assert!(records[0].is_none());
    }

    #[test]
    fn multi_mutation_flush() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "p");

        let mut session = SessionCore::new();
        session.record_mutation(Mutation::AppendChild { parent, child: a });
        session.record_mutation(Mutation::AppendChild { parent, child: b });
        session.record_mutation(Mutation::SetAttribute {
            entity: parent,
            name: "class".into(),
            value: "container".into(),
        });

        let records = session.flush(&mut dom);
        assert_eq!(records.len(), 3);
        assert!(records.iter().all(Option::is_some));
        assert_eq!(dom.children(parent), vec![a, b]);

        let attrs = dom.world().get::<&Attributes>(parent).unwrap();
        assert_eq!(attrs.get("class"), Some("container"));
    }

    #[test]
    fn identity_wrapper_idempotent() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");

        let mut session = SessionCore::new();
        let r1 = session.get_or_create_wrapper(e, ComponentKind::Element);
        let r2 = session.get_or_create_wrapper(e, ComponentKind::Element);
        assert_eq!(r1, r2);
    }

    #[test]
    fn release_clears_identity() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");

        let mut session = SessionCore::new();
        session.get_or_create_wrapper(e, ComponentKind::Element);
        session.get_or_create_wrapper(e, ComponentKind::Style);

        let count = session.release(e);
        assert_eq!(count, 2);
        assert!(session.identity_map().is_empty());
    }

    #[test]
    fn multiple_flushes_independent() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "p");

        let mut session = SessionCore::new();

        // First flush
        session.record_mutation(Mutation::AppendChild { parent, child: a });
        let r1 = session.flush(&mut dom);
        assert_eq!(r1.len(), 1);

        // Second flush
        session.record_mutation(Mutation::AppendChild { parent, child: b });
        let r2 = session.flush(&mut dom);
        assert_eq!(r2.len(), 1);

        assert_eq!(dom.children(parent), vec![a, b]);
    }
}
