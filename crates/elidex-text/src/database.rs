//! Font database wrapping [`fontdb`] for font discovery and metric queries.

use fontdb::Database;

/// Wraps [`fontdb::Database`] for font lookup and metric extraction.
pub struct FontDatabase {
    db: Database,
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
    /// Font design units per em.
    pub units_per_em: u16,
}

impl FontDatabase {
    /// Creates a new font database loaded with system fonts.
    pub fn new() -> Self {
        let mut db = Database::new();
        db.load_system_fonts();
        Self { db }
    }

    /// Queries for a font matching any of the given family names.
    ///
    /// Returns the first match found, or `None` if no font matches.
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
    pub fn font_metrics(&self, id: fontdb::ID, font_size: f32) -> Option<FontMetrics> {
        self.db.with_face_data(id, |data, face_index| {
            let face = rustybuzz::Face::from_slice(data, face_index)?;
            let upem = face.units_per_em();
            // units_per_em is always positive and fits in u16 for valid fonts.
            let upem_u16 = u16::try_from(upem).ok()?;
            let scale = font_size / f32::from(upem_u16);

            // ttf_parser::Face methods via Deref: ascender(), descender(), line_gap()
            let ascent = f32::from(face.ascender()) * scale;
            let descent = f32::from(face.descender()) * scale;
            let line_gap = f32::from(face.line_gap()) * scale;

            Some(FontMetrics {
                ascent,
                descent,
                line_gap,
                units_per_em: upem_u16,
            })
        })?
    }

    /// Provides access to the underlying [`fontdb::Database`].
    ///
    /// Used by the shaping module to access raw font data.
    pub(crate) fn inner(&self) -> &Database {
        &self.db
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
        let Some(id) = db.query(&[
            "Arial",
            "Helvetica",
            "Liberation Sans",
            "DejaVu Sans",
            "Noto Sans",
            "Hiragino Sans",
        ]) else {
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
        let Some(id) = db.query(&[
            "Arial",
            "Helvetica",
            "Liberation Sans",
            "DejaVu Sans",
            "Noto Sans",
            "Hiragino Sans",
        ]) else {
            return;
        };
        let metrics = db.font_metrics(id, 16.0).unwrap();
        // Ascent should be positive
        assert!(metrics.ascent > 0.0);
        // Descent should be negative (below baseline)
        assert!(metrics.descent < 0.0);
        // units_per_em should be a common value (typically 1000 or 2048)
        assert!(metrics.units_per_em > 0);
    }
}
