//! `MediaQueryList` methods for `HostBridge`.

use boa_engine::JsObject;

use super::{evaluate_media_query_raw, HostBridge, MediaQueryEntry};

impl HostBridge {
    /// Create a `MediaQueryList` entry and return its unique ID.
    pub fn create_media_query(&self, query: &str, matches: bool) -> u64 {
        let mut inner = self.inner.borrow_mut();
        let id = inner.media_query_next_id;
        inner.media_query_next_id += 1;
        inner.media_queries.insert(
            id,
            MediaQueryEntry {
                query: query.to_string(),
                matches,
                listeners: Vec::new(),
            },
        );
        id
    }

    /// Add a "change" event listener to a `MediaQueryList`.
    pub fn add_media_query_listener(&self, id: u64, callback: JsObject) {
        let mut inner = self.inner.borrow_mut();
        if let Some(entry) = inner.media_queries.get_mut(&id) {
            entry.listeners.push(callback);
        }
    }

    /// Remove a "change" event listener from a `MediaQueryList` by reference identity.
    pub fn remove_media_query_listener(&self, id: u64, callback: &JsObject) {
        let mut inner = self.inner.borrow_mut();
        if let Some(entry) = inner.media_queries.get_mut(&id) {
            entry
                .listeners
                .retain(|stored| !JsObject::equals(stored, callback));
        }
    }

    /// Re-evaluate all media queries against the given viewport dimensions.
    ///
    /// Returns a list of `(id, new_matches)` for entries whose result changed.
    /// Updates the cached `matches` value for each changed entry.
    pub fn re_evaluate_media_queries(&self, width: f32, height: f32) -> Vec<(u64, bool)> {
        let mut inner = self.inner.borrow_mut();
        let mut changed = Vec::new();
        for (&id, entry) in &mut inner.media_queries {
            let new_matches = evaluate_media_query_raw(&entry.query, width, height);
            if new_matches != entry.matches {
                entry.matches = new_matches;
                changed.push((id, new_matches));
            }
        }
        changed
    }

    /// Get the current `matches` value for a media query.
    pub fn media_query_matches(&self, id: u64) -> bool {
        self.inner
            .borrow()
            .media_queries
            .get(&id)
            .is_some_and(|e| e.matches)
    }

    /// Get the listener callbacks for a media query (cloned for dispatch).
    pub fn media_query_listeners(&self, id: u64) -> Vec<JsObject> {
        self.inner
            .borrow()
            .media_queries
            .get(&id)
            .map_or_else(Vec::new, |e| e.listeners.clone())
    }

    /// Get the query string for a media query entry.
    pub fn media_query_string(&self, id: u64) -> Option<String> {
        self.inner
            .borrow()
            .media_queries
            .get(&id)
            .map(|e| e.query.clone())
    }
}
