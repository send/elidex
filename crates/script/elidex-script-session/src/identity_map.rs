//! Bidirectional mapping between ECS entities and JS object references.

use std::collections::HashMap;

use elidex_ecs::Entity;

use crate::types::{ComponentKind, JsObjectRef};

/// Bidirectional identity map between (Entity, [`ComponentKind`]) pairs and
/// [`JsObjectRef`] handles.
///
/// Ensures that the same entity/component pair always maps to the same JS
/// object reference (identity preservation), which is critical for JS `===`
/// semantics on DOM wrapper objects.
#[derive(Debug)]
pub struct IdentityMap {
    /// Forward map: (Entity, `ComponentKind`) → `JsObjectRef`.
    forward: HashMap<(Entity, ComponentKind), JsObjectRef>,
    /// Reverse map: `JsObjectRef` → (Entity, `ComponentKind`).
    reverse: HashMap<JsObjectRef, (Entity, ComponentKind)>,
    /// Monotonically increasing counter for generating unique references.
    next_id: u64,
}

impl IdentityMap {
    /// Create a new, empty identity map.
    pub fn new() -> Self {
        Self {
            forward: HashMap::new(),
            reverse: HashMap::new(),
            next_id: 1,
        }
    }

    /// Get or create a [`JsObjectRef`] for the given entity and component kind.
    ///
    /// If a mapping already exists, the existing reference is returned
    /// (idempotent). Otherwise a new unique reference is allocated.
    pub fn get_or_create(&mut self, entity: Entity, kind: ComponentKind) -> JsObjectRef {
        let next_id = &mut self.next_id;
        let reverse = &mut self.reverse;
        *self.forward.entry((entity, kind)).or_insert_with(|| {
            let id = *next_id;
            *next_id += 1;
            let obj_ref = JsObjectRef::from_raw(id);
            reverse.insert(obj_ref, (entity, kind));
            obj_ref
        })
    }

    /// Look up the (Entity, [`ComponentKind`]) for a given [`JsObjectRef`].
    pub fn get(&self, obj_ref: JsObjectRef) -> Option<(Entity, ComponentKind)> {
        self.reverse.get(&obj_ref).copied()
    }

    /// Release a single [`JsObjectRef`], removing it from both maps.
    ///
    /// Returns `true` if the reference existed and was removed.
    #[cfg(test)]
    fn release(&mut self, obj_ref: JsObjectRef) -> bool {
        if let Some(key) = self.reverse.remove(&obj_ref) {
            self.forward.remove(&key);
            true
        } else {
            false
        }
    }

    /// Release all references associated with the given entity.
    ///
    /// Returns the number of references released.
    pub fn release_entity(&mut self, entity: Entity) -> usize {
        // Collect refs to remove (avoid borrowing conflict).
        let refs: Vec<JsObjectRef> = self
            .forward
            .iter()
            .filter(|(&(e, _), _)| e == entity)
            .map(|(_, &r)| r)
            .collect();

        let count = refs.len();
        for r in refs {
            self.reverse.remove(&r);
        }
        self.forward.retain(|&(e, _), _| e != entity);
        count
    }

    /// Returns the number of active mappings.
    pub fn len(&self) -> usize {
        self.forward.len()
    }

    /// Returns `true` if the map contains no mappings.
    pub fn is_empty(&self) -> bool {
        self.forward.is_empty()
    }
}

impl Default for IdentityMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::EcsDom;

    fn make_entity(dom: &mut EcsDom) -> Entity {
        dom.create_element("div", elidex_ecs::Attributes::default())
    }

    #[test]
    fn get_or_create_idempotent() {
        let mut dom = EcsDom::new();
        let e = make_entity(&mut dom);
        let mut map = IdentityMap::new();

        let r1 = map.get_or_create(e, ComponentKind::Element);
        let r2 = map.get_or_create(e, ComponentKind::Element);
        assert_eq!(r1, r2);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn different_entities_get_different_refs() {
        let mut dom = EcsDom::new();
        let e1 = make_entity(&mut dom);
        let e2 = make_entity(&mut dom);
        let mut map = IdentityMap::new();

        let r1 = map.get_or_create(e1, ComponentKind::Element);
        let r2 = map.get_or_create(e2, ComponentKind::Element);
        assert_ne!(r1, r2);
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn different_kinds_get_different_refs() {
        let mut dom = EcsDom::new();
        let e = make_entity(&mut dom);
        let mut map = IdentityMap::new();

        let r1 = map.get_or_create(e, ComponentKind::Element);
        let r2 = map.get_or_create(e, ComponentKind::Style);
        assert_ne!(r1, r2);
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn release_single() {
        let mut dom = EcsDom::new();
        let e = make_entity(&mut dom);
        let mut map = IdentityMap::new();

        let r = map.get_or_create(e, ComponentKind::Element);
        assert!(map.release(r));
        assert!(map.is_empty());
        assert!(map.get(r).is_none());

        // Double release returns false.
        assert!(!map.release(r));
    }

    #[test]
    fn release_entity_removes_all_kinds() {
        let mut dom = EcsDom::new();
        let e = make_entity(&mut dom);
        let mut map = IdentityMap::new();

        map.get_or_create(e, ComponentKind::Element);
        map.get_or_create(e, ComponentKind::Style);
        map.get_or_create(e, ComponentKind::Attribute);
        assert_eq!(map.len(), 3);

        let count = map.release_entity(e);
        assert_eq!(count, 3);
        assert!(map.is_empty());
    }

    #[test]
    fn len_and_is_empty() {
        let map = IdentityMap::new();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }
}
