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

    /// Build the [`MediaEnvironment`](elidex_css::media::MediaEnvironment) the
    /// canonical evaluator reads: viewport from the caller's `width`/`height`,
    /// device facts from the bridge's cached fields (written by the `SetDeviceFacts`
    /// / construction setters) — `resolution_dppx` (Media Queries L5 §5.1) +
    /// `color_scheme` (Media Queries L5 §12.5). The single env-builder both
    /// `matchMedia` (`globals/window/media_query`) and the report-changes re-eval
    /// route through, so the two callers cannot diverge — the boa mirror of the VM's
    /// `media_environment` (#360 one-builder-one-evaluator). Self-borrows are short
    /// and released before any caller's `borrow_mut`.
    pub(crate) fn media_environment(
        &self,
        width: f32,
        height: f32,
    ) -> elidex_css::media::MediaEnvironment {
        elidex_css::media::MediaEnvironment {
            viewport_width: f64::from(width),
            viewport_height: f64::from(height),
            // Feed the lossless `f64` dppx (the bridge stores the full winit scale, C3 R3).
            // The query/device precision alignment for the exact `<resolution>` comparison
            // lives in the engine-independent evaluator (`elidex_css::media::eval`
            // `range_feature_value`), shared by every producer — not duplicated here.
            resolution_dppx: self.device_pixel_ratio(),
            color_scheme: self.color_scheme(),
            ..elidex_css::media::MediaEnvironment::default()
        }
    }

    /// Re-evaluate all media queries against the given viewport dimensions.
    ///
    /// Returns a list of `(id, new_matches)` for entries whose result changed.
    /// Updates the cached `matches` value for each changed entry.
    pub fn re_evaluate_media_queries(&self, width: f32, height: f32) -> Vec<(u64, bool)> {
        // Build the env (immutable self-borrows) BEFORE the entries' `borrow_mut`.
        let env = self.media_environment(width, height);
        let mut inner = self.inner.borrow_mut();
        let mut changed = Vec::new();
        for (&id, entry) in &mut inner.media_queries {
            let new_matches = evaluate_media_query_raw(&entry.query, &env);
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
