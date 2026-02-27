//! Display list types for the rendering pipeline.
//!
//! A [`DisplayList`] is a flat list of [`DisplayItem`]s in painter's order
//! (back-to-front). The layout engine produces `LayoutBox` components; the
//! display list builder converts those into paint commands that can be
//! submitted to a GPU renderer.

use std::sync::Arc;

use elidex_plugin::{CssColor, Rect};

/// A positioned glyph in the display list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphEntry {
    /// Glyph ID in the font.
    pub glyph_id: u32,
    /// Horizontal position.
    pub x: f32,
    /// Vertical position.
    pub y: f32,
}

/// A single paint operation in the display list.
#[derive(Clone, Debug)]
pub enum DisplayItem {
    /// Fill a rectangle with a solid color.
    SolidRect {
        /// The rectangle to fill.
        rect: Rect,
        /// The fill color.
        color: CssColor,
    },
    /// Draw shaped text glyphs.
    Text {
        /// Positioned glyphs.
        glyphs: Vec<GlyphEntry>,
        /// Raw font file data (shared via `Arc`).
        font_blob: Arc<Vec<u8>>,
        /// Face index within the font collection.
        font_index: u32,
        /// Font size in pixels.
        font_size: f32,
        /// Text color.
        color: CssColor,
    },
}

/// A flat list of [`DisplayItem`]s in painter's order.
#[derive(Clone, Debug, Default)]
pub struct DisplayList(pub(crate) Vec<DisplayItem>);

impl DisplayList {
    /// Append a display item.
    pub(crate) fn push(&mut self, item: DisplayItem) {
        self.0.push(item);
    }

    /// Iterate over display items in painter's order.
    pub fn iter(&self) -> impl Iterator<Item = &DisplayItem> {
        self.0.iter()
    }

    /// Returns the number of display items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the display list has no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_display_list() {
        let dl = DisplayList::default();
        assert!(dl.0.is_empty());
    }

    #[test]
    fn push_solid_rect() {
        let mut dl = DisplayList::default();
        dl.0.push(DisplayItem::SolidRect {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
            color: CssColor::RED,
        });
        assert_eq!(dl.0.len(), 1);
    }
}
