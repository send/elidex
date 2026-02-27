//! Font database wrapping [`fontdb`] for font discovery and metric queries.

/// Wraps [`fontdb::Database`] for font lookup and metric extraction.
pub struct FontDatabase {
    db: fontdb::Database,
}

impl std::fmt::Debug for FontDatabase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontDatabase").finish_non_exhaustive()
    }
}

/// Font metrics scaled to pixel units for a given font size.
#[derive(Clone, Copy, Debug)]
pub struct FontMetrics {
    /// Ascent in pixels (positive, above baseline).
    pub ascent: f32,
    /// Descent in pixels (negative, below baseline).
    pub descent: f32,
    /// Line gap in pixels.
    pub line_gap: f32,
}

/// Compute pixel scale factor from a parsed font face and desired font size.
///
/// Returns `None` if `units_per_em` is not a valid `u16` (should not happen
/// for well-formed fonts, since the OpenType spec defines it as `u16`).
pub(crate) fn pixel_scale(face: &rustybuzz::Face, font_size: f32) -> Option<f32> {
    let upem = u16::try_from(face.units_per_em()).ok()?;
    Some(font_size / f32::from(upem))
}

impl FontDatabase {
    /// Creates a new font database loaded with system fonts.
    pub fn new() -> Self {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        Self { db }
    }

    /// Queries for a font matching any of the given family names.
    ///
    /// Returns the first match found, or `None` if no font matches.
    #[must_use]
    pub fn query(&self, families: &[&str]) -> Option<fontdb::ID> {
        let family_list: Vec<fontdb::Family<'_>> = families
            .iter()
            .map(|name| fontdb::Family::Name(name))
            .collect();
        let query = fontdb::Query {
            families: &family_list,
            ..fontdb::Query::default()
        };
        self.db.query(&query)
    }

    /// Returns pixel-scaled font metrics for the given font and size.
    ///
    /// Pixel conversion: `pixel = design_units * font_size / units_per_em`
    #[must_use]
    pub fn font_metrics(&self, id: fontdb::ID, font_size: f32) -> Option<FontMetrics> {
        self.db.with_face_data(id, |data, face_index| {
            let face = rustybuzz::Face::from_slice(data, face_index)?;
            let scale = pixel_scale(&face, font_size)?;

            // ttf_parser::Face methods via Deref: ascender(), descender(), line_gap()
            let ascent = f32::from(face.ascender()) * scale;
            let descent = f32::from(face.descender()) * scale;
            let line_gap = f32::from(face.line_gap()) * scale;

            Some(FontMetrics {
                ascent,
                descent,
                line_gap,
            })
        })?
    }

    /// Provides raw font data access for a given font ID.
    ///
    /// The callback receives the font binary data and the face index within the
    /// font collection. Returns `None` if the font ID is invalid.
    ///
    /// Used by `elidex-render` to create Vello `FontData` instances.
    #[must_use]
    pub fn with_face_data<R>(&self, id: fontdb::ID, f: impl FnOnce(&[u8], u32) -> R) -> Option<R> {
        self.db.with_face_data(id, f)
    }
}

impl Default for FontDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_does_not_panic() {
        let _db = FontDatabase::new();
    }

    #[test]
    fn nonexistent_font_returns_none() {
        let db = FontDatabase::new();
        assert!(db.query(&["__nonexistent_font_family_12345__"]).is_none());
    }

    #[test]
    fn query_system_font() {
        let db = FontDatabase::new();
        let Some(id) = db.query(crate::TEST_FONT_FAMILIES) else {
            // CI environment may not have fonts installed
            return;
        };
        // If we found a font, metrics should also work
        let metrics = db.font_metrics(id, 16.0);
        assert!(metrics.is_some());
    }

    #[test]
    fn font_metrics_pixel_scaling() {
        let db = FontDatabase::new();
        let Some(id) = db.query(crate::TEST_FONT_FAMILIES) else {
            return;
        };
        let metrics = db.font_metrics(id, 16.0).unwrap();
        // Ascent should be positive
        assert!(metrics.ascent > 0.0);
        // Descent should be negative (below baseline)
        assert!(metrics.descent < 0.0);
    }
}
