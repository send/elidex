//! Cached form ancestor lookups to avoid O(n*d) traversal per radio group search.

use elidex_ecs::{EcsDom, Entity};
use std::collections::HashMap;

/// Maximum cache entries before automatic eviction.
const MAX_CACHE_ENTRIES: usize = 10_000;

/// Caches form owner lookups for form controls.
///
/// Invalidated when the DOM tree changes (via `MutationObserver` or explicit call).
/// Bounded to `MAX_CACHE_ENTRIES` to prevent unbounded growth.
#[derive(Debug, Default)]
pub struct AncestorCache {
    form_owner: HashMap<Entity, Option<Entity>>,
}

impl AncestorCache {
    /// Create a new empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the form owner entity for a given entity, using cache.
    ///
    /// Per HTML §4.10.18.3: checks the `form` content attribute first
    /// (cross-tree association by ID), then falls back to ancestor `<form>`.
    #[must_use]
    pub fn get_form_owner(&mut self, entity: Entity, dom: &EcsDom) -> Option<Entity> {
        if let Some(cached) = self.form_owner.get(&entity) {
            return *cached;
        }
        // Evict all entries if cache grows too large to prevent unbounded memory.
        if self.form_owner.len() >= MAX_CACHE_ENTRIES {
            self.form_owner.clear();
        }
        let owner = crate::radio::resolve_form_owner_public(dom, entity);
        self.form_owner.insert(entity, owner);
        owner
    }

    /// Invalidate all cached entries (call on DOM mutation).
    pub fn invalidate_all(&mut self) {
        self.form_owner.clear();
    }

    /// Invalidate a specific entity's cached entry.
    pub fn invalidate(&mut self, entity: Entity) {
        self.form_owner.remove(&entity);
    }

    /// Returns the number of cached entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.form_owner.len()
    }

    /// Returns whether the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.form_owner.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom};

    #[test]
    fn cache_miss_then_hit() {
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let input = dom.create_element("input", Attributes::default());
        let _ = dom.append_child(form, input);

        let mut cache = AncestorCache::new();
        // First call: cache miss, performs traversal
        let owner = cache.get_form_owner(input, &dom);
        assert_eq!(owner, Some(form));
        assert_eq!(cache.len(), 1);

        // Second call: cache hit
        let owner2 = cache.get_form_owner(input, &dom);
        assert_eq!(owner2, Some(form));
    }

    #[test]
    fn no_form_ancestor() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());

        let mut cache = AncestorCache::new();
        let owner = cache.get_form_owner(div, &dom);
        assert!(owner.is_none());
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn invalidate_all_clears() {
        let mut dom = EcsDom::new();
        let form = dom.create_element("form", Attributes::default());
        let input = dom.create_element("input", Attributes::default());
        let _ = dom.append_child(form, input);

        let mut cache = AncestorCache::new();
        let _ = cache.get_form_owner(input, &dom);
        assert!(!cache.is_empty());

        cache.invalidate_all();
        assert!(cache.is_empty());
    }
}
