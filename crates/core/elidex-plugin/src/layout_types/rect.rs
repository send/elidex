//! Rectangle, `Size`, and `CssSize` types.

use std::ops::{Div, Mul};

use super::boxes::EdgeSizes;
use super::vectors::{Point, Vector};

/// An axis-aligned rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// Position of the top-left corner.
    pub origin: Point,
    /// Size of the rectangle.
    pub size: Size,
}

/// A 2D size (width and height).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    /// Width in pixels.
    pub width: f32,
    /// Height in pixels.
    pub height: f32,
}

/// A CSS containing block size: width is always definite, height may be
/// indefinite (`None` = auto / content-dependent).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CssSize {
    /// Width (always definite).
    pub width: f32,
    /// Height (`None` if content-dependent / auto).
    pub height: Option<f32>,
}

impl CssSize {
    /// Create a fully definite size.
    #[must_use]
    pub fn definite(width: f32, height: f32) -> Self {
        Self {
            width,
            height: Some(height),
        }
    }

    /// Create a size with indefinite height.
    #[must_use]
    pub fn width_only(width: f32) -> Self {
        Self {
            width,
            height: None,
        }
    }

    /// Height or 0 if indefinite.
    #[must_use]
    pub fn height_or_zero(&self) -> f32 {
        self.height.unwrap_or(0.0)
    }

    /// Height or `width` as fallback (for vertical writing modes where
    /// the block axis maps to the physical width).
    #[must_use]
    pub fn height_or_width(&self) -> f32 {
        self.height.unwrap_or(self.width)
    }

    /// Resolve a percentage against the height, returning `None` if height
    /// is indefinite.
    #[must_use]
    pub fn resolve_height_pct(&self, pct: f32) -> Option<f32> {
        self.height.map(|h| h * pct / 100.0)
    }
}

impl Default for CssSize {
    fn default() -> Self {
        Self {
            width: 0.0,
            height: None,
        }
    }
}

impl Size {
    /// A zero size.
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };

    /// Create a new size.
    #[must_use]
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    /// Widen both components to a `Vector<f64>` (width → x, height → y).
    #[must_use]
    pub fn to_vector_f64(self) -> Vector<f64> {
        Vector::new(f64::from(self.width), f64::from(self.height))
    }

    /// Area as `f64` (avoids `f32` overflow for large sizes).
    #[must_use]
    pub fn area_f64(self) -> f64 {
        let v = self.to_vector_f64();
        v.x * v.y
    }

    /// Per-axis scale factors to map `from` to `self`.
    #[must_use]
    pub fn scale_from(self, from: Self) -> Vector<f64> {
        self.to_vector_f64() / from.to_vector_f64()
    }
}

impl Mul<f32> for Size {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self {
            width: self.width * rhs,
            height: self.height * rhs,
        }
    }
}

impl Div<f32> for Size {
    type Output = Self;
    fn div(self, rhs: f32) -> Self {
        Self {
            width: self.width / rhs,
            height: self.height / rhs,
        }
    }
}

impl Rect {
    /// Create a rectangle from position and size.
    #[must_use]
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    /// Create a rectangle from origin point and size.
    #[must_use]
    pub fn from_origin_size(origin: Point, size: Size) -> Self {
        Self { origin, size }
    }

    /// X coordinate of the right edge (`origin.x + size.width`).
    #[must_use]
    pub fn right(&self) -> f32 {
        self.origin.x + self.size.width
    }

    /// Y coordinate of the bottom edge (`origin.y + size.height`).
    #[must_use]
    pub fn bottom(&self) -> f32 {
        self.origin.y + self.size.height
    }

    /// The bottom-right corner point.
    #[must_use]
    pub fn max_point(&self) -> Point {
        Point::new(self.right(), self.bottom())
    }

    /// Center point of the rectangle.
    #[must_use]
    pub fn center(&self) -> Point {
        Point::new(
            self.origin.x + self.size.width * 0.5,
            self.origin.y + self.size.height * 0.5,
        )
    }

    /// Returns `true` if the point is inside the rectangle (half-open: includes
    /// top-left, excludes bottom-right).
    #[must_use]
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.origin.x && p.x < self.right() && p.y >= self.origin.y && p.y < self.bottom()
    }

    /// Returns the intersection of two rectangles, or `None` if they don't overlap.
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let x = self.origin.x.max(other.origin.x);
        let y = self.origin.y.max(other.origin.y);
        let w = self.right().min(other.right()) - x;
        let h = self.bottom().min(other.bottom()) - y;
        if w > 0.0 && h > 0.0 {
            Some(Self::new(x, y, w, h))
        } else {
            None
        }
    }

    /// Map a percentage-based point (0–100 per axis) to absolute coordinates
    /// within this rectangle.
    #[must_use]
    pub fn point_at_pct(&self, pct: Point) -> Point {
        Point::new(
            self.origin.x + self.size.width * pct.x / 100.0,
            self.origin.y + self.size.height * pct.y / 100.0,
        )
    }

    /// Returns `(x0, y0, x1, y1)` as `f64` for interop with external f64
    /// rectangle types (Vello `kurbo::Rect`, AccessKit `Rect`, etc.).
    #[must_use]
    pub fn to_f64_bounds(&self) -> (f64, f64, f64, f64) {
        (
            f64::from(self.origin.x),
            f64::from(self.origin.y),
            f64::from(self.right()),
            f64::from(self.bottom()),
        )
    }

    /// Returns a new rectangle shrunk inward by `d` on each side.
    ///
    /// Width and height are clamped to 0 to prevent negative sizes.
    #[must_use]
    pub fn inset(self, d: f32) -> Self {
        Self::new(
            self.origin.x + d,
            self.origin.y + d,
            (self.size.width - d * 2.0).max(0.0),
            (self.size.height - d * 2.0).max(0.0),
        )
    }

    /// Returns a new rectangle expanded outward by the given edge sizes.
    #[must_use]
    pub fn expand(self, edges: EdgeSizes) -> Self {
        Self {
            origin: self.origin - Vector::new(edges.left, edges.top),
            size: Size::new(
                self.size.width + edges.left + edges.right,
                self.size.height + edges.top + edges.bottom,
            ),
        }
    }
}
