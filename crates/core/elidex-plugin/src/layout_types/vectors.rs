//! Vector and Point types for layout geometry.

use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

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
