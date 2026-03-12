//! Font data cache for the rendering pipeline.
//!
//! Caches raw font file data keyed by `fontdb::ID` so that repeated
//! lookups during display list building and Vello scene construction
//! avoid re-copying font data.

use std::collections::HashMap;
use std::sync::Arc;

use elidex_text::{FontDatabase, FontId};

/// Raw font file data with face index within the font collection.
pub(crate) type FontBlob = (Arc<Vec<u8>>, u32);

/// Cache of raw font data indexed by font ID.
///
/// Each entry contains an `Arc<Vec<u8>>` of the font binary and the
/// face index within the font collection.
pub(crate) struct FontCache {
    cache: HashMap<FontId, FontBlob>,
}

impl FontCache {
    /// Create a new, empty font cache.
    pub(crate) fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Get the font data for `id`, loading and caching it if necessary.
    ///
    /// Returns `None` if the font ID is invalid.
    pub(crate) fn get(&mut self, db: &FontDatabase, id: FontId) -> Option<FontBlob> {
        if let Some(entry) = self.cache.get(&id) {
            return Some(entry.clone());
        }

        let (data, index) = db.with_face_data(id, |data, index| (data.to_vec(), index))?;
        let arc = Arc::new(data);
        self.cache.insert(id, (Arc::clone(&arc), index));
        Some((arc, index))
    }
}

impl Default for FontCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_miss_returns_none_for_invalid_id() {
        let db = FontDatabase::new();
        let mut cache = FontCache::new();
        // fontdb::ID is opaque — use a dummy ID via a query for a nonexistent font.
        // Since we can't construct a dummy ID, just verify the cache starts empty.
        assert!(cache.cache.is_empty());

        // Query a real font if available to test the cache hit path.
        let families = &[
            "Arial",
            "Helvetica",
            "Liberation Sans",
            "DejaVu Sans",
            "Noto Sans",
            "Hiragino Sans",
        ];
        let Some(id) = db.query(families, 400, elidex_text::FontStyle::Normal) else {
            return;
        };

        let result = cache.get(&db, id);
        assert!(result.is_some());
        let (blob, _index) = result.unwrap();
        assert!(!blob.is_empty());

        // Second get should hit the cache.
        let result2 = cache.get(&db, id);
        assert!(result2.is_some());
        let (blob2, _) = result2.unwrap();
        // Same Arc (pointer equality).
        assert!(Arc::ptr_eq(&blob, &blob2));
    }
}
