//! Display list types for the rendering pipeline.
//!
//! A [`DisplayList`] is a flat list of [`DisplayItem`]s in painter's order
//! (back-to-front). The layout engine produces `LayoutBox` components; the
//! display list builder converts those into paint commands that can be
//! submitted to a GPU renderer.

use std::sync::Arc;

use elidex_plugin::{CssColor, Point, Rect, Size, Vector};

/// A positioned glyph in the display list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphEntry {
    /// Glyph ID in the font.
    pub glyph_id: u32,
    /// Position in pixels.
    pub position: Point,
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
        /// Position within the painting area.
        position: Point,
        /// Rendered size in pixels.
        size: Size,
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
        /// Center position in pixels relative to painting area.
        center: Point,
        /// Ellipse radii in pixels.
        radii: Size,
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
        /// Center position in pixels relative to painting area.
        center: Point,
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
    /// Draw a border ring between an outer and inner rounded rectangle.
    /// Used when border-radius is set with uniform border color.
    RoundedBorderRing {
        /// Outer rectangle (border-box).
        outer_rect: Rect,
        /// Outer corner radii `[top-left, top-right, bottom-right, bottom-left]`.
        outer_radii: [f32; 4],
        /// Inner rectangle (padding-box).
        inner_rect: Rect,
        /// Inner corner radii `[top-left, top-right, bottom-right, bottom-left]`.
        inner_radii: [f32; 4],
        /// Border color (uniform for all sides).
        color: CssColor,
    },
    /// Draw a border segment with a specific line style (dashed, dotted).
    ///
    /// The segment is rendered as a stroked line along the center of the
    /// border edge. Dash patterns and cap styles control the appearance.
    StyledBorderSegment {
        /// Start point of the border line center.
        start: Point,
        /// End point of the border line center.
        end: Point,
        /// Border width (stroke width).
        width: f32,
        /// Dash pattern: `[dash_length, gap_length]`. Empty = solid.
        dashes: Vec<f32>,
        /// Whether to use round caps (for dotted: dots are circles).
        round_caps: bool,
        /// The border color.
        color: CssColor,
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
    /// Begin a 2D affine transform region (CSS Transforms L1/L2, 3D projected to 2D).
    /// Coefficients `[a, b, c, d, e, f]` = `| a c e | / | b d f | / | 0 0 1 |`
    PushTransform {
        /// The projected 2D affine transform.
        affine: [f64; 6],
    },
    /// End a transform region.
    PopTransform,
    /// Begin viewport scroll translation.
    ///
    /// Translates all subsequent items by `(-offset.0, -offset.1)` until
    /// the matching `PopScrollOffset`. Fixed-position elements emit a
    /// `PopScrollOffset`/`PushScrollOffset` pair around themselves to
    /// cancel the scroll translation.
    PushScrollOffset {
        /// Scroll offset `(x, y)` in pixels (displacement).
        scroll_offset: Vector,
    },
    /// End viewport scroll translation.
    PopScrollOffset,
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

    /// Patch all `PushScrollOffset` items with new scroll values.
    ///
    /// This is the fast path for scroll-only updates: the display list
    /// structure (including fixed-element exclusion pairs) is preserved,
    /// and only the scroll offset values are updated in place.
    pub fn update_scroll_offset(&mut self, scroll_offset: Vector) {
        debug_assert!(scroll_offset.is_finite(), "scroll offset must be finite");
        for item in &mut self.0 {
            if let DisplayItem::PushScrollOffset { scroll_offset: so } = item {
                *so = scroll_offset;
            }
        }
    }
}

/// Multi-page display list for paged media output.
///
/// Each element of `pages` is a complete [`DisplayList`] for one printed
/// page, including margin box content. The `page_size` is the physical
/// page dimensions (before margins).
#[derive(Clone, Debug)]
pub struct PagedDisplayList {
    /// One display list per page, in page order.
    pub pages: Vec<DisplayList>,
    /// Physical page size in pixels.
    pub page_size: Size,
}

impl PagedDisplayList {
    /// Returns the total number of pages.
    #[must_use]
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Returns `true` if there are no pages.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
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

    #[test]
    fn push_scroll_offset() {
        let mut dl = DisplayList::default();
        dl.push(DisplayItem::PushScrollOffset {
            scroll_offset: Vector::new(10.0, 20.0),
        });
        assert_eq!(dl.len(), 1);
        assert!(matches!(
            dl.iter().next(),
            Some(DisplayItem::PushScrollOffset {
                scroll_offset,
            }) if (scroll_offset.x - 10.0).abs() < f32::EPSILON && (scroll_offset.y - 20.0).abs() < f32::EPSILON
        ));
    }

    #[test]
    fn update_scroll_offset_patches() {
        let mut dl = DisplayList::default();
        dl.push(DisplayItem::PushScrollOffset {
            scroll_offset: Vector::<f32>::ZERO,
        });
        dl.push(DisplayItem::SolidRect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            color: CssColor::RED,
        });
        dl.push(DisplayItem::PopScrollOffset);
        // A second PushScrollOffset (e.g. from fixed re-push).
        dl.push(DisplayItem::PushScrollOffset {
            scroll_offset: Vector::<f32>::ZERO,
        });
        dl.push(DisplayItem::PopScrollOffset);

        dl.update_scroll_offset(Vector::new(50.0, 100.0));

        let pushes: Vec<_> = dl
            .iter()
            .filter_map(|item| match item {
                DisplayItem::PushScrollOffset { scroll_offset } => Some(*scroll_offset),
                _ => None,
            })
            .collect();
        assert_eq!(pushes.len(), 2);
        for p in pushes {
            assert!((p.x - 50.0).abs() < f32::EPSILON);
            assert!((p.y - 100.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn update_scroll_offset_noop() {
        let mut dl = DisplayList::default();
        dl.push(DisplayItem::SolidRect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            color: CssColor::RED,
        });
        // No PushScrollOffset items — should not panic.
        dl.update_scroll_offset(Vector::new(10.0, 20.0));
        assert_eq!(dl.len(), 1);
    }
}
