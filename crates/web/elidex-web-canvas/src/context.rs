//! Canvas 2D rendering context implementation.
//!
//! Wraps a `tiny_skia::Pixmap` and provides the Canvas 2D API surface.
//! The pixel buffer can be extracted as RGBA8 data for integration with
//! elidex's `ImageData` component.

use elidex_plugin::CssColor;
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};

use crate::path::arc_to_beziers;
use crate::style::parse_color_string;

/// Convert a premultiplied-alpha pixel to straight alpha (RGBA8).
#[inline]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn premul_to_straight(src: &[u8]) -> [u8; 4] {
    debug_assert!(src.len() >= 4, "premul_to_straight requires 4-byte pixel");
    let a = src[3];
    if a == 0 {
        [0, 0, 0, 0]
    } else if a == 255 {
        [src[0], src[1], src[2], 255]
    } else {
        let af = f32::from(a) / 255.0;
        [
            (f32::from(src[0]) / af).round().min(255.0) as u8,
            (f32::from(src[1]) / af).round().min(255.0) as u8,
            (f32::from(src[2]) / af).round().min(255.0) as u8,
            a,
        ]
    }
}

/// Convert a straight-alpha pixel to premultiplied alpha (RGBA8).
#[inline]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn straight_to_premul(src: &[u8]) -> [u8; 4] {
    debug_assert!(src.len() >= 4, "straight_to_premul requires 4-byte pixel");
    let a = src[3];
    if a == 255 {
        [src[0], src[1], src[2], 255]
    } else if a == 0 {
        [0, 0, 0, 0]
    } else {
        let af = f32::from(a) / 255.0;
        [
            (f32::from(src[0]) * af).round() as u8,
            (f32::from(src[1]) * af).round() as u8,
            (f32::from(src[2]) * af).round() as u8,
            a,
        ]
    }
}

/// Default canvas width per HTML spec.
pub const DEFAULT_WIDTH: u32 = 300;
/// Default canvas height per HTML spec.
pub const DEFAULT_HEIGHT: u32 = 150;
/// Bytes per pixel in RGBA8 format.
const BYTES_PER_PIXEL: usize = 4;
/// Estimated character width in pixels at the default 10px font.
const ESTIMATED_CHAR_WIDTH: f32 = 6.0;

/// Saved drawing state for `save()`/`restore()`.
#[derive(Clone, Debug)]
struct DrawingState {
    fill_color: CssColor,
    stroke_color: CssColor,
    line_width: f32,
    global_alpha: f32,
    transform: Transform,
}

impl Default for DrawingState {
    fn default() -> Self {
        Self {
            fill_color: CssColor::BLACK,
            stroke_color: CssColor::BLACK,
            line_width: 1.0,
            global_alpha: 1.0,
            transform: Transform::identity(),
        }
    }
}

/// Canvas 2D rendering context backed by tiny-skia.
///
/// Provides drawing methods matching the HTML Canvas 2D API subset.
/// The rendered output is accessible as an RGBA8 pixel buffer via [`pixels()`](Self::pixels).
pub struct Canvas2dContext {
    pixmap: Pixmap,
    state_stack: Vec<DrawingState>,
    current: DrawingState,
    path_builder: PathBuilder,
}

impl Canvas2dContext {
    /// Create a new canvas context with the given dimensions.
    ///
    /// Returns `None` if the dimensions are zero or too large for tiny-skia.
    pub fn new(width: u32, height: u32) -> Option<Self> {
        let pixmap = Pixmap::new(width, height)?;
        Some(Self {
            pixmap,
            state_stack: Vec::new(),
            current: DrawingState::default(),
            path_builder: PathBuilder::new(),
        })
    }

    /// Width of the canvas in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.pixmap.width()
    }

    /// Height of the canvas in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.pixmap.height()
    }

    /// Get the RGBA8 pixel data (4 bytes per pixel, row-major).
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        self.pixmap.data()
    }

    // --- Style accessors ---

    /// Get the current fill color.
    #[must_use]
    pub fn fill_style(&self) -> CssColor {
        self.current.fill_color
    }

    /// Set the fill color from a CSS color string.
    ///
    /// If the string is not a valid CSS color, the current fill color is unchanged
    /// (matching browser behavior).
    pub fn set_fill_style(&mut self, color_str: &str) {
        if let Some(c) = parse_color_string(color_str) {
            self.current.fill_color = c;
        }
    }

    /// Get the current stroke color.
    #[must_use]
    pub fn stroke_style(&self) -> CssColor {
        self.current.stroke_color
    }

    /// Set the stroke color from a CSS color string.
    pub fn set_stroke_style(&mut self, color_str: &str) {
        if let Some(c) = parse_color_string(color_str) {
            self.current.stroke_color = c;
        }
    }

    /// Get the current line width.
    #[must_use]
    pub fn line_width(&self) -> f32 {
        self.current.line_width
    }

    /// Set the line width. Values ≤ 0, infinity, and NaN are ignored.
    pub fn set_line_width(&mut self, width: f32) {
        if width.is_finite() && width > 0.0 {
            self.current.line_width = width;
        }
    }

    /// Get the current global alpha.
    #[must_use]
    pub fn global_alpha(&self) -> f32 {
        self.current.global_alpha
    }

    /// Set the global alpha (0.0–1.0). Out-of-range values are ignored.
    pub fn set_global_alpha(&mut self, alpha: f32) {
        if alpha.is_finite() && (0.0..=1.0).contains(&alpha) {
            self.current.global_alpha = alpha;
        }
    }

    // --- State stack ---

    /// Save the current drawing state onto the stack.
    pub fn save(&mut self) {
        self.state_stack.push(self.current.clone());
    }

    /// Restore the most recently saved drawing state.
    ///
    /// Does nothing if the stack is empty.
    pub fn restore(&mut self) {
        if let Some(state) = self.state_stack.pop() {
            self.current = state;
        }
    }

    // --- Transform ---

    /// Apply a translation transform.
    ///
    /// Per WHATWG spec, non-finite values are silently ignored.
    pub fn translate(&mut self, tx: f32, ty: f32) {
        if any_non_finite_2(tx, ty) {
            return;
        }
        self.current.transform = self.current.transform.pre_translate(tx, ty);
    }

    /// Apply a rotation transform (angle in radians).
    ///
    /// Per WHATWG spec, non-finite values are silently ignored.
    pub fn rotate(&mut self, angle: f32) {
        if !angle.is_finite() {
            return;
        }
        let cos = angle.cos();
        let sin = angle.sin();
        let rot = Transform::from_row(cos, sin, -sin, cos, 0.0, 0.0);
        self.current.transform = self.current.transform.pre_concat(rot);
    }

    /// Apply a scale transform.
    ///
    /// Per WHATWG spec, non-finite values are silently ignored.
    pub fn scale(&mut self, sx: f32, sy: f32) {
        if any_non_finite_2(sx, sy) {
            return;
        }
        self.current.transform = self.current.transform.pre_scale(sx, sy);
    }

    // --- Rectangle methods ---

    /// Fill a rectangle with the current fill style.
    ///
    /// Per WHATWG spec: non-finite args → noop, zero w/h → noop,
    /// negative dimensions are normalized.
    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        if any_non_finite_4(x, y, w, h) {
            return;
        }
        if w == 0.0 || h == 0.0 {
            return;
        }
        let (x, y, w, h) = normalize_rect(x, y, w, h);
        let Some(rect) = Rect::from_xywh(x, y, w, h) else {
            return;
        };
        let paint = self.fill_paint();
        self.pixmap
            .fill_rect(rect, &paint, self.current.transform, None);
    }

    /// Stroke a rectangle with the current stroke style.
    ///
    /// Per WHATWG spec: non-finite args → noop, both zero → noop,
    /// one zero dimension → stroke a line, negative dimensions are normalized.
    pub fn stroke_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        if any_non_finite_4(x, y, w, h) {
            return;
        }
        if w == 0.0 && h == 0.0 {
            return;
        }
        let (x, y, w, h) = normalize_rect(x, y, w, h);
        let mut pb = PathBuilder::new();
        if w == 0.0 || h == 0.0 {
            // One zero dimension → stroke a line.
            pb.move_to(x, y);
            pb.line_to(x + w, y + h);
        } else {
            let Some(rect) = Rect::from_xywh(x, y, w, h) else {
                return;
            };
            pb.push_rect(rect);
        }
        let Some(path) = pb.finish() else { return };
        let paint = self.stroke_paint();
        let stroke = self.current_stroke();
        self.pixmap
            .stroke_path(&path, &paint, &stroke, self.current.transform, None);
    }

    /// Clear a rectangle to transparent black.
    ///
    /// Per WHATWG spec: non-finite args → noop, zero w/h → noop,
    /// negative dimensions are normalized.
    pub fn clear_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        if any_non_finite_4(x, y, w, h) {
            return;
        }
        if w == 0.0 || h == 0.0 {
            return;
        }
        let (x, y, w, h) = normalize_rect(x, y, w, h);
        let Some(rect) = Rect::from_xywh(x, y, w, h) else {
            return;
        };
        let mut paint = Paint::default();
        paint.set_color(tiny_skia::Color::from_rgba8(0, 0, 0, 0));
        paint.blend_mode = tiny_skia::BlendMode::Source;
        self.pixmap
            .fill_rect(rect, &paint, self.current.transform, None);
    }

    // --- Path methods ---

    /// Begin a new sub-path (resets the path builder).
    pub fn begin_path(&mut self) {
        self.path_builder = PathBuilder::new();
    }

    /// Move the current point to (x, y) without drawing.
    ///
    /// Per WHATWG spec, non-finite values are silently ignored.
    pub fn move_to(&mut self, x: f32, y: f32) {
        if any_non_finite_2(x, y) {
            return;
        }
        self.path_builder.move_to(x, y);
    }

    /// Draw a line from the current point to (x, y).
    ///
    /// Per WHATWG spec, non-finite values are silently ignored.
    pub fn line_to(&mut self, x: f32, y: f32) {
        if any_non_finite_2(x, y) {
            return;
        }
        if self.path_builder.last_point().is_none() {
            self.path_builder.move_to(x, y);
        } else {
            self.path_builder.line_to(x, y);
        }
    }

    /// Close the current sub-path.
    pub fn close_path(&mut self) {
        self.path_builder.close();
    }

    /// Add a rectangular sub-path.
    ///
    /// Per Canvas 2D spec, creates a closed subpath with corners at
    /// (x,y), (x+w,y), (x+w,y+h), (x,y+h) — handles negative dimensions.
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        if any_non_finite_4(x, y, w, h) {
            return;
        }
        self.path_builder.move_to(x, y);
        self.path_builder.line_to(x + w, y);
        self.path_builder.line_to(x + w, y + h);
        self.path_builder.line_to(x, y + h);
        self.path_builder.close();
    }

    /// Add an arc sub-path.
    ///
    /// Approximates the arc using cubic Bezier curves.
    pub fn arc(
        &mut self,
        x: f32,
        y: f32,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        anticlockwise: bool,
    ) {
        arc_to_beziers(
            &mut self.path_builder,
            x,
            y,
            radius,
            start_angle,
            end_angle,
            anticlockwise,
        );
    }

    /// Fill the current path with the current fill style.
    ///
    /// Per Canvas 2D spec, the path is preserved after fill — only
    /// `beginPath()` clears it.
    pub fn fill(&mut self) {
        let Some(path) = self.try_finish_path() else {
            return;
        };
        let paint = self.fill_paint();
        self.pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            self.current.transform,
            None,
        );
    }

    /// Stroke the current path with the current stroke style.
    ///
    /// Per Canvas 2D spec, the path is preserved after stroke — only
    /// `beginPath()` clears it.
    pub fn stroke(&mut self) {
        let Some(path) = self.try_finish_path() else {
            return;
        };
        let paint = self.stroke_paint();
        let stroke = self.current_stroke();
        self.pixmap
            .stroke_path(&path, &paint, &stroke, self.current.transform, None);
    }

    /// Clone the current path builder and finish it.
    ///
    /// Returns `None` if the path is empty or invalid. The original
    /// path builder is preserved (Canvas 2D spec requires path persistence).
    fn try_finish_path(&self) -> Option<tiny_skia::Path> {
        self.path_builder.clone().finish()
    }

    // --- Image data methods ---

    /// Get pixel data for a rectangular region.
    ///
    /// Returns RGBA8 pixel data for the given region. Pixels outside the
    /// canvas bounds are returned as transparent black.
    #[must_use]
    #[allow(
        clippy::cast_possible_wrap,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::similar_names
    )]
    pub fn get_image_data(&self, sx: i32, sy: i32, sw: u32, sh: u32) -> Vec<u8> {
        let Some(total) = (sw as usize)
            .checked_mul(sh as usize)
            .and_then(|n| n.checked_mul(BYTES_PER_PIXEL))
        else {
            return Vec::new();
        };
        // Safe: tiny-skia Pixmap dimensions are bounded well below i32::MAX.
        let w = self.pixmap.width() as i32;
        let h = self.pixmap.height() as i32;
        let mut data = vec![0u8; total];

        for dy in 0..sh as i32 {
            for dx in 0..sw as i32 {
                let px = sx + dx;
                let py = sy + dy;
                if px >= 0 && px < w && py >= 0 && py < h {
                    let src_offset =
                        ((py as usize) * (w as usize) + (px as usize)) * BYTES_PER_PIXEL;
                    let dst_offset =
                        ((dy as usize) * (sw as usize) + (dx as usize)) * BYTES_PER_PIXEL;
                    let pixel = premul_to_straight(
                        &self.pixmap.data()[src_offset..src_offset + BYTES_PER_PIXEL],
                    );
                    data[dst_offset..dst_offset + BYTES_PER_PIXEL].copy_from_slice(&pixel);
                }
            }
        }
        data
    }

    /// Put pixel data into the canvas at the given position.
    ///
    /// Input data must be RGBA8 (straight alpha), which is converted to
    /// premultiplied alpha for tiny-skia.
    #[allow(
        clippy::cast_possible_wrap,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::similar_names
    )]
    pub fn put_image_data(&mut self, data: &[u8], dx: i32, dy: i32, sw: u32, sh: u32) {
        // Safe: tiny-skia Pixmap dimensions are bounded well below i32::MAX.
        let w = self.pixmap.width() as i32;
        let h = self.pixmap.height() as i32;
        let pixels = self.pixmap.data_mut();

        for sy in 0..sh as i32 {
            for sx in 0..sw as i32 {
                let px = dx + sx;
                let py = dy + sy;
                if px >= 0 && px < w && py >= 0 && py < h {
                    let src_offset =
                        ((sy as usize) * (sw as usize) + (sx as usize)) * BYTES_PER_PIXEL;
                    if src_offset + BYTES_PER_PIXEL > data.len() {
                        continue;
                    }
                    let dst_offset =
                        ((py as usize) * (w as usize) + (px as usize)) * BYTES_PER_PIXEL;
                    let pixel = straight_to_premul(&data[src_offset..src_offset + BYTES_PER_PIXEL]);
                    pixels[dst_offset..dst_offset + BYTES_PER_PIXEL].copy_from_slice(&pixel);
                }
            }
        }
    }

    /// Create a blank image data buffer of the given dimensions.
    ///
    /// Returns a `Vec<u8>` of transparent black pixels (RGBA8).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn create_image_data(width: u32, height: u32) -> Vec<u8> {
        let Some(total) = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(BYTES_PER_PIXEL))
        else {
            return Vec::new();
        };
        vec![0u8; total]
    }

    /// Measure text width using a rough per-character estimate.
    ///
    /// Returns an approximate width based on the default 10px sans-serif font
    /// at `ESTIMATED_CHAR_WIDTH` (6px) per character. This is sufficient for
    /// basic layout calculations but will not match browser `measureText()`
    /// output for variable-width fonts, CJK text, or non-default font sizes.
    ///
    /// Integrating with `elidex-shaping` for accurate font metrics requires:
    /// 1. Tracking the current canvas font (via `ctx.font = "..."` parsing)
    /// 2. Resolving the font against `FontDatabase` (same as CSS text layout)
    /// 3. Calling `measure_text()` from `elidex-shaping` with the resolved face
    ///
    /// This is blocked on `fillText`/`strokeText` implementation, which needs
    /// the same font resolution pipeline. Both will be addressed together.
    #[must_use]
    pub fn measure_text(&self, text: &str) -> f32 {
        let char_count = text.chars().count();
        #[allow(clippy::cast_precision_loss)]
        let width = char_count as f32 * ESTIMATED_CHAR_WIDTH;
        width
    }

    /// Convert the canvas pixel buffer to straight-alpha RGBA8 suitable
    /// for the `ImageData` ECS component.
    ///
    /// tiny-skia internally uses premultiplied alpha, so this method
    /// performs the conversion.
    #[must_use]
    pub fn to_rgba8_straight(&self) -> Vec<u8> {
        let src = self.pixmap.data();
        debug_assert_eq!(
            src.len() % BYTES_PER_PIXEL,
            0,
            "Pixmap buffer length must be a multiple of {BYTES_PER_PIXEL}"
        );
        let mut dst = vec![0u8; src.len()];
        for i in (0..src.len()).step_by(BYTES_PER_PIXEL) {
            let pixel = premul_to_straight(&src[i..i + BYTES_PER_PIXEL]);
            dst[i..i + BYTES_PER_PIXEL].copy_from_slice(&pixel);
        }
        dst
    }

    // --- Internal helpers ---

    // (module-level helpers below impl block)
}

/// Per WHATWG Canvas 2D spec, non-finite arguments are silently ignored.
#[inline]
fn any_non_finite_2(a: f32, b: f32) -> bool {
    !a.is_finite() || !b.is_finite()
}

/// Per WHATWG Canvas 2D spec, non-finite arguments are silently ignored.
#[inline]
fn any_non_finite_4(a: f32, b: f32, c: f32, d: f32) -> bool {
    !a.is_finite() || !b.is_finite() || !c.is_finite() || !d.is_finite()
}

/// Normalize rectangle dimensions per WHATWG Canvas spec.
///
/// Negative width → x += w, w = -w. Negative height → y += h, h = -h.
fn normalize_rect(x: f32, y: f32, w: f32, h: f32) -> (f32, f32, f32, f32) {
    let (x, w) = if w < 0.0 { (x + w, -w) } else { (x, w) };
    let (y, h) = if h < 0.0 { (y + h, -h) } else { (y, h) };
    (x, y, w, h)
}

impl Canvas2dContext {
    /// Build a `Paint` from the given color and the current global alpha.
    #[must_use]
    fn make_paint(&self, color: CssColor) -> Paint<'static> {
        let mut paint = Paint::default();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let a = (f32::from(color.a) * self.current.global_alpha)
            .round()
            .clamp(0.0, 255.0) as u8;
        paint.set_color(tiny_skia::Color::from_rgba8(color.r, color.g, color.b, a));
        paint.anti_alias = true;
        paint
    }

    /// Build a fill `Paint` from the current drawing state.
    #[must_use]
    fn fill_paint(&self) -> Paint<'static> {
        self.make_paint(self.current.fill_color)
    }

    /// Build a stroke `Paint` from the current drawing state.
    #[must_use]
    fn stroke_paint(&self) -> Paint<'static> {
        self.make_paint(self.current.stroke_color)
    }

    /// Build a `Stroke` from the current line width.
    #[must_use]
    fn current_stroke(&self) -> Stroke {
        Stroke {
            width: self.current.line_width,
            ..Stroke::default()
        }
    }
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod context_tests;
