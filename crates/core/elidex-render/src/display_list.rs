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
    /// Fill a rounded rectangle with a solid color.
    RoundedRect {
        /// The rectangle to fill.
        rect: Rect,
        /// Per-corner radii `[top-left, top-right, bottom-right, bottom-left]` in pixels.
        radii: [f32; 4],
        /// The fill color.
        color: CssColor,
    },
    /// Stroke (outline) a rounded rectangle.
    StrokedRoundedRect {
        /// The rectangle to stroke.
        rect: Rect,
        /// Per-corner radii `[top-left, top-right, bottom-right, bottom-left]` in pixels.
        radii: [f32; 4],
        /// Stroke line width in pixels.
        stroke_width: f32,
        /// The stroke color.
        color: CssColor,
    },
    /// Draw a decoded image.
    Image {
        /// The painting area (clip boundary).
        painting_area: Rect,
        /// Decoded RGBA8 pixel data.
        pixels: Arc<Vec<u8>>,
        /// Image width in pixels.
        image_width: u32,
        /// Image height in pixels.
        image_height: u32,
        /// Position within the painting area `(x, y)`.
        position: (f32, f32),
        /// Rendered size `(width, height)` in pixels.
        size: (f32, f32),
        /// Repeat mode for tiling.
        repeat: elidex_plugin::background::BgRepeat,
        /// Element opacity (0.0–1.0).
        opacity: f32,
    },
    /// Draw a linear gradient.
    LinearGradient {
        /// The painting area.
        painting_area: Rect,
        /// Gradient line angle in degrees (0 = to top, 90 = to right).
        angle: f32,
        /// Resolved color stops with normalized positions (0.0–1.0).
        stops: Vec<(f32, CssColor)>,
        /// Whether this is a repeating gradient.
        repeating: bool,
        /// Element opacity (0.0–1.0).
        opacity: f32,
    },
    /// Draw a radial gradient.
    RadialGradient {
        /// The painting area.
        painting_area: Rect,
        /// Center position `(x, y)` in pixels relative to painting area.
        center: (f32, f32),
        /// Ellipse radii `(rx, ry)` in pixels.
        radii: (f32, f32),
        /// Resolved color stops with normalized positions (0.0–1.0).
        stops: Vec<(f32, CssColor)>,
        /// Whether this is a repeating gradient.
        repeating: bool,
        /// Element opacity (0.0–1.0).
        opacity: f32,
    },
    /// Draw a conic gradient.
    ConicGradient {
        /// The painting area.
        painting_area: Rect,
        /// Center position `(x, y)` in pixels relative to painting area.
        center: (f32, f32),
        /// Start angle in degrees.
        start_angle: f32,
        /// End angle in degrees.
        end_angle: f32,
        /// Resolved angular color stops with positions in degrees.
        stops: Vec<(f32, CssColor)>,
        /// Whether this is a repeating gradient.
        repeating: bool,
        /// Element opacity (0.0–1.0).
        opacity: f32,
    },
    /// Begin a clip region (for `overflow: hidden` or background-clip).
    PushClip {
        /// The clipping rectangle.
        rect: Rect,
        /// Per-corner radii `[top-left, top-right, bottom-right, bottom-left]`.
        /// `[0.0; 4]` = rectangular clip.
        radii: [f32; 4],
    },
    /// End a clip region.
    PopClip,
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
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            color: CssColor::RED,
        });
        assert_eq!(dl.0.len(), 1);
    }
}
