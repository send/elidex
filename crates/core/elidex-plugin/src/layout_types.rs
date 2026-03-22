//! Layout types for the box model and layout algorithms.

use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

use crate::computed_style::Dimension;

/// A 2D displacement (delta / offset).
///
/// Unlike [`Point`] (which represents an absolute position), `Vector`
/// represents a relative displacement. The type system enforces correct
/// semantics: `Point + Vector = Point`, `Point - Point = Vector`.
///
/// The default type parameter is `f32` (CSS pixel coordinates). Use
/// `Vector<f64>` for high-precision intermediate calculations (e.g. scroll
/// deltas from the windowing system).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector<T = f32> {
    /// X component.
    pub x: T,
    /// Y component.
    pub y: T,
}

impl<T> Vector<T> {
    /// Create a new vector.
    #[must_use]
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl Vector<f32> {
    /// The zero vector.
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    /// X-axis only displacement.
    #[must_use]
    pub fn x_only(x: f32) -> Self {
        Self { x, y: 0.0 }
    }

    /// Y-axis only displacement.
    #[must_use]
    pub fn y_only(y: f32) -> Self {
        Self { x: 0.0, y }
    }

    /// Convert to a point (interpret as absolute position).
    #[must_use]
    pub fn to_point(self) -> Point {
        Point::new(self.x, self.y)
    }

    /// Returns `true` if both components are finite (not NaN or infinite).
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    /// Widen to `Vector<f64>`.
    #[must_use]
    pub fn to_f64(self) -> Vector<f64> {
        Vector::new(f64::from(self.x), f64::from(self.y))
    }
}

/// f64 precision methods for `Vector<f64>`.
impl Vector<f64> {
    /// The zero vector (f64).
    ///
    /// Use as `Vector::<f64>::ZERO` to avoid ambiguity with `Vector::<f32>::ZERO`.
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    /// Returns `true` if both components are finite (not NaN or infinite).
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    /// Convert to `(x, y)` tuple for interop with APIs that take `(f64, f64)`.
    #[must_use]
    pub fn to_tuple(self) -> (f64, f64) {
        (self.x, self.y)
    }

    /// Convert to a point (interpret as absolute position).
    #[must_use]
    pub fn to_point(self) -> Point<f64> {
        Point::new(self.x, self.y)
    }

    /// Narrow to `Vector<f32>` (truncating).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn to_f32(self) -> Vector<f32> {
        Vector::new(self.x as f32, self.y as f32)
    }
}

impl<T: Add<Output = T>> Add for Vector<T> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<T: Sub<Output = T>> Sub for Vector<T> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<T: AddAssign> AddAssign for Vector<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<T: Mul<Output = T> + Copy> Mul<T> for Vector<T> {
    type Output = Self;
    fn mul(self, rhs: T) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

/// Component-wise `Vector * Vector` (Hadamard product / scale).
impl<T: Mul<Output = T>> Mul for Vector<T> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}

/// Component-wise `Vector / Vector`.
impl<T: Div<Output = T>> Div for Vector<T> {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
        }
    }
}

/// Scalar `Vector / T`.
impl<T: Div<Output = T> + Copy> Div<T> for Vector<T> {
    type Output = Self;
    fn div(self, rhs: T) -> Self {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl<T: SubAssign> SubAssign for Vector<T> {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<T: Neg<Output = T>> Neg for Vector<T> {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

/// A 2D point (absolute position).
///
/// Use [`Vector`] for displacements / deltas.
///
/// The default type parameter is `f32` (CSS pixel coordinates). Use
/// `Point<f64>` for windowing-system coordinates (`HiDPI` physical pixels)
/// or transform-math intermediate values.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point<T = f32> {
    /// X coordinate.
    pub x: T,
    /// Y coordinate.
    pub y: T,
}

impl<T> Point<T> {
    /// Create a new point.
    #[must_use]
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl Point<f32> {
    /// The origin point (0, 0).
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    /// Convert to a vector from the origin.
    #[must_use]
    pub fn to_vector(self) -> Vector {
        Vector::new(self.x, self.y)
    }

    /// Returns `true` if both components are finite (not NaN or infinite).
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    /// Widen to `Point<f64>`.
    #[must_use]
    pub fn to_f64(self) -> Point<f64> {
        Point::new(f64::from(self.x), f64::from(self.y))
    }

    /// Convert to `(x, y)` tuple.
    #[must_use]
    pub fn to_tuple(self) -> (f32, f32) {
        (self.x, self.y)
    }
}

/// f64 precision methods for `Point<f64>`.
impl Point<f64> {
    /// Returns `true` if both components are finite (not NaN or infinite).
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    /// Narrow to `Point<f32>` (truncating).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn to_f32(self) -> Point<f32> {
        Point::new(self.x as f32, self.y as f32)
    }

    /// Convert to a vector from the origin.
    #[must_use]
    pub fn to_vector(self) -> Vector<f64> {
        Vector::new(self.x, self.y)
    }

    /// Convert to `(x, y)` tuple for interop with APIs that take `(f64, f64)`.
    #[must_use]
    pub fn to_tuple(self) -> (f64, f64) {
        (self.x, self.y)
    }
}

/// `Point + Vector = Point`
impl<T: Add<Output = T>> Add<Vector<T>> for Point<T> {
    type Output = Self;
    fn add(self, rhs: Vector<T>) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

/// `Point - Point = Vector`
impl<T: Sub<Output = T>> Sub for Point<T> {
    type Output = Vector<T>;
    fn sub(self, rhs: Self) -> Vector<T> {
        Vector {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

/// `Point - Vector = Point`
impl<T: Sub<Output = T>> Sub<Vector<T>> for Point<T> {
    type Output = Self;
    fn sub(self, rhs: Vector<T>) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

/// `Point += Vector`
impl<T: AddAssign> AddAssign<Vector<T>> for Point<T> {
    fn add_assign(&mut self, rhs: Vector<T>) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

/// `Point -= Vector`
impl<T: SubAssign> SubAssign<Vector<T>> for Point<T> {
    fn sub_assign(&mut self, rhs: Vector<T>) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

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

/// Edge sizes for padding, border, and margin.
///
/// The default type parameter `f32` is used for used/layout values.
/// `EdgeSizes<Dimension>` stores computed values that may contain percentages.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeSizes<T = f32> {
    /// Top edge size.
    pub top: T,
    /// Right edge size.
    pub right: T,
    /// Bottom edge size.
    pub bottom: T,
    /// Left edge size.
    pub left: T,
}

impl Default for EdgeSizes<f32> {
    fn default() -> Self {
        Self {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }
}

impl Default for EdgeSizes<Dimension> {
    fn default() -> Self {
        Self {
            top: Dimension::ZERO,
            right: Dimension::ZERO,
            bottom: Dimension::ZERO,
            left: Dimension::ZERO,
        }
    }
}

impl EdgeSizes {
    /// Create edge sizes with individual values for each side.
    #[must_use]
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Create edge sizes with the same value on all sides.
    #[must_use]
    pub fn uniform(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Sum of left and right edges.
    #[must_use]
    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    /// Sum of top and bottom edges.
    #[must_use]
    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
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

/// A box in the layout tree.
///
/// Represents the CSS box model with content, padding, border, and margin areas.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutBox {
    /// The content area.
    pub content: Rect,
    /// Padding between content and border.
    pub padding: EdgeSizes,
    /// Border widths.
    pub border: EdgeSizes,
    /// Margin outside the border.
    pub margin: EdgeSizes,
    /// First baseline offset from content box top edge (`None` if no baseline).
    ///
    /// CSS 2.1 §10.8.1: the first baseline of a box is the first baseline
    /// of its first in-flow line box or block child that has a baseline.
    pub first_baseline: Option<f32>,
}

impl LayoutBox {
    /// Returns the padding box (content + padding).
    #[must_use]
    pub fn padding_box(&self) -> Rect {
        self.content.expand(self.padding)
    }

    /// Returns the border box (content + padding + border).
    #[must_use]
    pub fn border_box(&self) -> Rect {
        self.padding_box().expand(self.border)
    }

    /// Returns the margin box (content + padding + border + margin).
    ///
    /// Note: negative margins can produce a `Rect` with negative width or height.
    #[must_use]
    pub fn margin_box(&self) -> Rect {
        self.border_box().expand(self.margin)
    }
}

/// Context available to a layout algorithm.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutContext {
    /// The viewport size.
    pub viewport: Size,
    /// The containing block size.
    pub containing_block: Size,
}

/// The result of a layout pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutResult {
    /// The positioned bounding rectangle.
    pub bounds: Rect,
    /// The computed margins.
    pub margin: EdgeSizes,
    /// The computed padding.
    pub padding: EdgeSizes,
    /// The computed border widths.
    pub border: EdgeSizes,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_arithmetic() {
        let a = Vector::new(1.0, 2.0);
        let b = Vector::new(3.0, 4.0);
        assert_eq!(a + b, Vector::new(4.0, 6.0));
        assert_eq!(b - a, Vector::new(2.0, 2.0));
        assert_eq!(-a, Vector::new(-1.0, -2.0));
        assert_eq!(a * 3.0, Vector::new(3.0, 6.0));

        let mut c = a;
        c += b;
        assert_eq!(c, Vector::new(4.0, 6.0));

        // Component-wise mul/div
        assert_eq!(a * b, Vector::new(3.0, 8.0));
        assert_eq!(b / a, Vector::new(3.0, 2.0));

        assert_eq!(Vector::<f32>::ZERO, Vector::new(0.0, 0.0));
        assert_eq!(Vector::default(), Vector::<f32>::ZERO);
    }

    #[test]
    fn point_vector_arithmetic() {
        let p = Point::new(10.0_f32, 20.0);
        let v = Vector::new(3.0_f32, 4.0);

        // Point + Vector = Point
        assert_eq!(p + v, Point::new(13.0, 24.0));
        // Point - Vector = Point
        assert_eq!(p - v, Point::new(7.0, 16.0));
        // Point - Point = Vector
        assert_eq!(Point::new(5.0, 8.0) - p, Vector::new(-5.0, -12.0));

        let mut q = p;
        q += v;
        assert_eq!(q, Point::new(13.0, 24.0));
        q -= v;
        assert_eq!(q, Point::new(10.0, 20.0));

        // to_vector
        assert_eq!(p.to_vector(), Vector::new(10.0_f32, 20.0));

        assert_eq!(Point::ZERO, Point::new(0.0_f32, 0.0));
        assert_eq!(Point::default(), Point::ZERO);
    }

    #[test]
    fn is_finite() {
        assert!(Point::new(1.0_f32, 2.0).is_finite());
        assert!(!Point::new(f32::NAN, 0.0).is_finite());
        assert!(!Point::new(0.0_f32, f32::INFINITY).is_finite());
        assert!(Vector::new(1.0_f32, 2.0).is_finite());
        assert!(!Vector::new(f32::NAN, 0.0).is_finite());
    }

    #[test]
    fn vector_to_point() {
        let v = Vector::new(5.0_f32, 10.0);
        assert_eq!(v.to_point(), Point::new(5.0, 10.0));
    }

    #[test]
    fn point_f64_conversion() {
        let p = Point::new(10.0_f32, 20.0);
        let p64 = p.to_f64();
        assert_eq!(p64, Point::new(10.0_f64, 20.0));
        let back = p64.to_f32();
        assert_eq!(back, p);
    }

    #[test]
    fn size_scale_from() {
        let target = Size::new(200.0, 100.0);
        let source = Size::new(400.0, 200.0);
        let scale = target.scale_from(source);
        assert!((scale.x - 0.5).abs() < f64::EPSILON);
        assert!((scale.y - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn rect_point_at_pct() {
        let r = Rect::new(10.0, 20.0, 200.0, 100.0);
        let center = r.point_at_pct(Point::new(50.0, 50.0));
        assert_eq!(center, Point::new(110.0, 70.0));
        let top_left = r.point_at_pct(Point::ZERO);
        assert_eq!(top_left, r.origin);
    }

    #[test]
    #[allow(clippy::many_single_char_names)]
    fn rect_intersection() {
        let a = Rect::new(0.0, 0.0, 100.0, 100.0);
        let b = Rect::new(50.0, 50.0, 100.0, 100.0);
        let inter = a.intersection(&b).unwrap();
        assert_eq!(inter, Rect::new(50.0, 50.0, 50.0, 50.0));

        // No overlap
        let c = Rect::new(200.0, 200.0, 10.0, 10.0);
        assert!(a.intersection(&c).is_none());

        // Edge-touching (zero area)
        let d = Rect::new(100.0, 0.0, 50.0, 50.0);
        assert!(a.intersection(&d).is_none());

        // Full containment
        let e = Rect::new(10.0, 10.0, 20.0, 20.0);
        assert_eq!(a.intersection(&e), Some(e));
    }

    #[test]
    fn rect_inset() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(r.inset(5.0), Rect::new(15.0, 25.0, 90.0, 40.0));
        // Over-inset clamps to zero size
        assert_eq!(r.inset(60.0), Rect::new(70.0, 80.0, 0.0, 0.0));
    }

    #[test]
    fn size_new_and_zero() {
        assert_eq!(
            Size::new(10.0, 20.0),
            Size {
                width: 10.0,
                height: 20.0
            }
        );
        assert_eq!(Size::ZERO, Size::default());
    }

    #[test]
    fn rect_from_origin_size() {
        let r = Rect::from_origin_size(Point::new(5.0, 10.0), Size::new(100.0, 50.0));
        assert_eq!(r, Rect::new(5.0, 10.0, 100.0, 50.0));
    }

    #[test]
    fn rect_edges_and_center() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(r.right(), 110.0);
        assert_eq!(r.bottom(), 70.0);
        assert_eq!(r.max_point(), Point::new(110.0, 70.0));
        assert_eq!(r.center(), Point::new(60.0, 45.0));
    }

    #[test]
    fn rect_contains() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        // Inside
        assert!(r.contains(Point::new(50.0, 40.0)));
        // Top-left corner (inclusive)
        assert!(r.contains(Point::new(10.0, 20.0)));
        // Bottom-right (exclusive)
        assert!(!r.contains(Point::new(110.0, 70.0)));
        // Just inside bottom-right
        assert!(r.contains(Point::new(109.99, 69.99)));
        // Outside
        assert!(!r.contains(Point::new(5.0, 40.0)));
        assert!(!r.contains(Point::new(50.0, 80.0)));
    }

    #[test]
    fn rect_default() {
        let r = Rect::default();
        assert_eq!(r.origin.x, 0.0);
        assert_eq!(r.origin.y, 0.0);
        assert_eq!(r.size.width, 0.0);
        assert_eq!(r.size.height, 0.0);
    }

    #[test]
    fn size_default() {
        let s = Size::default();
        assert_eq!(s.width, 0.0);
        assert_eq!(s.height, 0.0);
    }

    #[test]
    fn edge_sizes_default() {
        let e = EdgeSizes::<f32>::default();
        assert_eq!(e.top, 0.0);
        assert_eq!(e.right, 0.0);
        assert_eq!(e.bottom, 0.0);
        assert_eq!(e.left, 0.0);
    }

    #[test]
    fn layout_box_padding_box() {
        let b = LayoutBox {
            content: Rect::new(20.0, 20.0, 100.0, 50.0),
            padding: EdgeSizes {
                top: 10.0,
                right: 10.0,
                bottom: 10.0,
                left: 10.0,
            },
            ..Default::default()
        };
        let pb = b.padding_box();
        assert_eq!(pb.origin.x, 10.0);
        assert_eq!(pb.origin.y, 10.0);
        assert_eq!(pb.size.width, 120.0);
        assert_eq!(pb.size.height, 70.0);
    }

    #[test]
    fn layout_box_border_box() {
        let b = LayoutBox {
            content: Rect::new(25.0, 25.0, 100.0, 50.0),
            padding: EdgeSizes {
                top: 10.0,
                right: 10.0,
                bottom: 10.0,
                left: 10.0,
            },
            border: EdgeSizes {
                top: 5.0,
                right: 5.0,
                bottom: 5.0,
                left: 5.0,
            },
            ..Default::default()
        };
        let bb = b.border_box();
        assert_eq!(bb.origin.x, 10.0);
        assert_eq!(bb.origin.y, 10.0);
        assert_eq!(bb.size.width, 130.0);
        assert_eq!(bb.size.height, 80.0);
    }

    #[test]
    fn layout_box_margin_box() {
        let b = LayoutBox {
            content: Rect::new(30.0, 30.0, 100.0, 50.0),
            padding: EdgeSizes {
                top: 10.0,
                right: 10.0,
                bottom: 10.0,
                left: 10.0,
            },
            border: EdgeSizes {
                top: 5.0,
                right: 5.0,
                bottom: 5.0,
                left: 5.0,
            },
            margin: EdgeSizes {
                top: 5.0,
                right: 5.0,
                bottom: 5.0,
                left: 5.0,
            },
            first_baseline: None,
        };
        let mb = b.margin_box();
        assert_eq!(mb.origin.x, 10.0); // 30 - 10(pad) - 5(border) - 5(margin)
        assert_eq!(mb.origin.y, 10.0); // 30 - 10(pad) - 5(border) - 5(margin)
        assert_eq!(mb.size.width, 140.0);
        assert_eq!(mb.size.height, 90.0);
    }

    #[test]
    fn layout_box_default_all_zero() {
        let b = LayoutBox::default();
        let mb = b.margin_box();
        assert_eq!(mb, Rect::default());
    }

    #[test]
    fn layout_box_asymmetric_edges() {
        let b = LayoutBox {
            content: Rect::new(20.0, 10.0, 200.0, 100.0),
            padding: EdgeSizes {
                top: 5.0,
                right: 15.0,
                bottom: 10.0,
                left: 10.0,
            },
            border: EdgeSizes {
                top: 1.0,
                right: 2.0,
                bottom: 3.0,
                left: 4.0,
            },
            margin: EdgeSizes {
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            },
            first_baseline: None,
        };
        let bb = b.border_box();
        assert_eq!(bb.origin.x, 6.0); // 20 - 10 (pad.left) - 4 (border.left)
        assert_eq!(bb.origin.y, 4.0); // 10 - 5 (pad.top) - 1 (border.top)
        assert_eq!(bb.size.width, 231.0); // 200 + 10 + 15 + 4 + 2
        assert_eq!(bb.size.height, 119.0); // 100 + 5 + 10 + 1 + 3
    }

    #[test]
    fn layout_context_default() {
        let ctx = LayoutContext::default();
        assert_eq!(ctx.viewport, Size::default());
        assert_eq!(ctx.containing_block, Size::default());
    }

    #[test]
    fn layout_result_default() {
        let r = LayoutResult::default();
        assert_eq!(r.bounds, Rect::default());
        assert_eq!(r.margin, EdgeSizes::default());
    }

    #[test]
    fn vector_scalar_div() {
        let v = Vector::new(10.0_f32, 20.0);
        assert_eq!(v / 2.0, Vector::new(5.0, 10.0));

        let v64 = Vector::new(9.0_f64, 12.0);
        assert_eq!(v64 / 3.0, Vector::new(3.0, 4.0));
    }

    #[test]
    fn vector_sub_assign() {
        let mut v = Vector::new(10.0_f32, 20.0);
        v -= Vector::new(3.0, 5.0);
        assert_eq!(v, Vector::new(7.0, 15.0));
    }

    #[test]
    fn point_f32_to_tuple() {
        let p = Point::new(1.5_f32, 2.5);
        assert_eq!(p.to_tuple(), (1.5, 2.5));
    }

    #[test]
    fn vector_f64_zero() {
        assert_eq!(Vector::<f64>::ZERO, Vector::new(0.0_f64, 0.0));
        assert_eq!(Vector::<f64>::default(), Vector::<f64>::ZERO);
    }

    #[test]
    fn vector_f64_to_point() {
        let v = Vector::new(3.0_f64, 4.0);
        assert_eq!(v.to_point(), Point::new(3.0_f64, 4.0));
    }

    #[test]
    fn vector_f64_to_f32() {
        let v = Vector::new(1.5_f64, 2.5);
        assert_eq!(v.to_f32(), Vector::new(1.5_f32, 2.5));
    }

    #[test]
    fn size_scalar_mul() {
        let s = Size::new(10.0, 20.0);
        assert_eq!(s * 2.0, Size::new(20.0, 40.0));
    }

    #[test]
    fn size_scalar_div() {
        let s = Size::new(10.0, 20.0);
        assert_eq!(s / 2.0, Size::new(5.0, 10.0));
    }
}
