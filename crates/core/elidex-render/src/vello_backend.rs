//! Vello GPU rendering backend.
//!
//! Converts a [`DisplayList`] into a Vello [`Scene`] and renders it to
//! a `wgpu::Texture` using Vello's GPU compute pipeline.

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

use vello::kurbo::{Affine, BezPath, Rect as VelloRect, Shape, Stroke};
use vello::peniko::{
    Blob, Color, Extend, Fill, FontData, Gradient, ImageAlphaType, ImageData as PenikoImageData,
    ImageFormat, Mix,
};
use vello::{AaConfig, AaSupport, Glyph, RenderParams, Renderer, RendererOptions, Scene};
use wgpu::{Device, Queue, Texture, TextureDescriptor, TextureFormat, TextureUsages};

use elidex_plugin::{CssColor, Rect};

use crate::display_list::{DisplayItem, DisplayList};

/// Convert an elidex [`Rect`] to a Vello [`kurbo::Rect`].
#[must_use]
fn to_vello_rect(r: &Rect) -> VelloRect {
    VelloRect::new(
        f64::from(r.x),
        f64::from(r.y),
        f64::from(r.x + r.width),
        f64::from(r.y + r.height),
    )
}

/// GPU renderer backed by Vello.
///
/// Holds the Vello `Renderer`, a reusable `Scene`, and a persistent font
/// cache to avoid per-frame allocations and preserve Vello's glyph cache
/// across frames.
pub struct VelloRenderer {
    renderer: Renderer,
    scene: Scene,
    /// Persistent font data cache keyed by `Arc<Vec<u8>>` pointer identity.
    /// Keeping the same `FontData` (with the same `Blob` ID) across frames
    /// allows Vello's internal glyph cache to hit.
    font_cache: HashMap<*const Vec<u8>, FontData>,
}

impl VelloRenderer {
    /// Create a new Vello renderer for the given wgpu device.
    ///
    /// Returns an error if Vello's GPU pipeline creation fails.
    pub fn new(device: &Device) -> Result<Self, vello::Error> {
        let renderer = Renderer::new(
            device,
            RendererOptions {
                use_cpu: false,
                antialiasing_support: AaSupport::area_only(),
                num_init_threads: NonZeroUsize::new(1),
                pipeline_cache: None,
            },
        )?;

        Ok(Self {
            renderer,
            scene: Scene::new(),
            font_cache: HashMap::new(),
        })
    }

    /// Render a display list to a newly created `Rgba8Unorm` texture.
    ///
    /// The returned texture has `STORAGE_BINDING | TEXTURE_BINDING` usage
    /// flags, suitable for blitting to a surface via [`wgpu::util::TextureBlitter`].
    ///
    /// Zero-size dimensions are clamped to 1 pixel.
    pub fn render(
        &mut self,
        device: &Device,
        queue: &Queue,
        display_list: &DisplayList,
        width: u32,
        height: u32,
    ) -> Result<Texture, vello::Error> {
        // Clamp to 1×1 to avoid wgpu validation errors on zero-size textures.
        let width = width.max(1);
        let height = height.max(1);

        // Build the Vello scene from the display list.
        self.scene.reset();
        build_scene(&mut self.scene, display_list, &mut self.font_cache);

        // Create the render target texture.
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("vello_render_target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.renderer.render_to_texture(
            device,
            queue,
            &self.scene,
            &view,
            &RenderParams {
                base_color: Color::from_rgb8(255, 255, 255),
                width,
                height,
                antialiasing_method: AaConfig::Area,
            },
        )?;

        Ok(texture)
    }
}

/// Convert a [`CssColor`] to a Vello [`Color`].
#[must_use]
fn convert_color(c: CssColor) -> Color {
    Color::from_rgba8(c.r, c.g, c.b, c.a)
}

/// Convert per-corner radii `[tl, tr, br, bl]` to a Vello-compatible tuple.
#[must_use]
fn to_vello_radii(radii: &[f32; 4]) -> (f64, f64, f64, f64) {
    (
        f64::from(radii[0]),
        f64::from(radii[1]),
        f64::from(radii[2]),
        f64::from(radii[3]),
    )
}

/// Maximum number of tiles to prevent excessive rendering.
const MAX_TILES: usize = 10_000;

/// Compute tile positions for background-image repeat modes.
///
/// Returns a list of `(x, y)` offsets relative to the painting area origin.
/// CSS Backgrounds Level 3 §3.8:
/// - `no-repeat`: single placement at `position`
/// - `repeat`: tile from `position`, covering the painting area
/// - `space`: whole tiles only, excess space distributed evenly
/// - `round`: adjust tile size so tiles fill exactly
#[must_use]
#[allow(clippy::trivially_copy_pass_by_ref)] // kept as ref for consistency with DisplayItem fields
fn compute_tile_positions(
    painting_area: &Rect,
    position: &(f32, f32),
    size: &(f32, f32),
    repeat: &elidex_plugin::background::BgRepeat,
) -> Vec<(f32, f32)> {
    use elidex_plugin::background::BgRepeatAxis;

    let pw = painting_area.width;
    let ph = painting_area.height;

    // Compute tile size, adjusting for `round`
    let tile_w = match repeat.x {
        BgRepeatAxis::Round if size.0 > 0.0 => {
            let n = (pw / size.0).round().max(1.0);
            pw / n
        }
        _ => size.0,
    };
    let tile_h = match repeat.y {
        BgRepeatAxis::Round if size.1 > 0.0 => {
            let n = (ph / size.1).round().max(1.0);
            ph / n
        }
        _ => size.1,
    };

    if tile_w <= 0.0 || tile_h <= 0.0 {
        return vec![];
    }

    // Compute x positions
    let xs = axis_tile_positions(repeat.x, position.0, tile_w, pw);
    // Compute y positions
    let ys = axis_tile_positions(repeat.y, position.1, tile_h, ph);

    // Cartesian product, capped at MAX_TILES
    let mut positions = Vec::with_capacity(xs.len() * ys.len());
    for &y in &ys {
        for &x in &xs {
            positions.push((x, y));
            if positions.len() >= MAX_TILES {
                return positions;
            }
        }
    }
    positions
}

/// Compute tile positions along a single axis.
fn axis_tile_positions(
    mode: elidex_plugin::background::BgRepeatAxis,
    origin: f32,
    tile_size: f32,
    area_size: f32,
) -> Vec<f32> {
    use elidex_plugin::background::BgRepeatAxis;

    match mode {
        BgRepeatAxis::NoRepeat => vec![origin],
        BgRepeatAxis::Repeat | BgRepeatAxis::Round => {
            // Tile from origin backwards and forwards to cover the area
            let mut positions = Vec::new();
            // First tile going left/up from origin
            let mut pos = origin;
            while pos > -tile_size {
                pos -= tile_size;
            }
            // Now tile forward to cover the area
            while pos < area_size {
                positions.push(pos);
                pos += tile_size;
            }
            positions
        }
        BgRepeatAxis::Space => {
            if tile_size <= 0.0 {
                return vec![origin];
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let count = (area_size / tile_size).floor() as usize;
            if count == 0 {
                return vec![];
            }
            if count == 1 {
                // Single tile: use background-position (CSS Backgrounds L3 §3.8).
                return vec![origin];
            }
            #[allow(clippy::cast_precision_loss)]
            let spacing = (area_size - tile_size * count as f32) / (count - 1) as f32;
            #[allow(clippy::cast_precision_loss)]
            (0..count)
                .map(|i| i as f32 * (tile_size + spacing))
                .collect()
        }
    }
}

/// Compute gradient line start and end points from an angle and painting area.
///
/// CSS Images Level 3 §3.4.1: The gradient line passes through the center of
/// the painting area at the given angle, extending to the corners.
#[must_use]
fn gradient_line_from_angle(
    angle_deg: f32,
    area: &Rect,
) -> (vello::kurbo::Point, vello::kurbo::Point) {
    use std::f64::consts::PI;
    let cx = f64::from(area.x + area.width / 2.0);
    let cy = f64::from(area.y + area.height / 2.0);
    let w = f64::from(area.width);
    let h = f64::from(area.height);

    // CSS angle: 0deg = to top, 90deg = to right (clockwise from top)
    let rad = f64::from(angle_deg) * PI / 180.0;
    let sin = rad.sin();
    let cos = rad.cos();

    // Half-length of gradient line (extends to box corners)
    let half_len = f64::midpoint(w * sin.abs(), h * cos.abs());

    let start = vello::kurbo::Point::new(cx - sin * half_len, cy + cos * half_len);
    let end = vello::kurbo::Point::new(cx + sin * half_len, cy - cos * half_len);
    (start, end)
}

/// Convert a [`DisplayList`] into a Vello [`Scene`].
///
/// The `font_cache` maps `Arc<Vec<u8>>` pointer identity to Vello `FontData`.
/// Keeping this cache across frames preserves Vello's internal glyph cache,
/// which keys on `Blob` ID.
///
/// # Safety of raw pointer keys
///
/// The `Arc<Vec<u8>>` references in `display_list` must outlive the returned
/// `FontData` entries (they do, since `FontData` clones the data into its own
/// `Blob`).
#[allow(clippy::too_many_lines)]
// Single match dispatcher over display list item variants.
pub(crate) fn build_scene(
    scene: &mut Scene,
    display_list: &DisplayList,
    font_cache: &mut HashMap<*const Vec<u8>, FontData>,
) {
    debug_assert_eq!(
        display_list
            .iter()
            .filter(|i| matches!(i, DisplayItem::PushClip { .. }))
            .count(),
        display_list
            .iter()
            .filter(|i| matches!(i, DisplayItem::PopClip))
            .count(),
        "PushClip/PopClip must be balanced in display list"
    );
    for item in display_list.iter() {
        match item {
            DisplayItem::SolidRect { rect, color } => {
                let vello_rect = to_vello_rect(rect);
                let vello_color = convert_color(*color);
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    vello_color,
                    None,
                    &vello_rect,
                );
            }
            DisplayItem::RoundedRect { rect, radii, color } => {
                let vello_rect = to_vello_rect(rect);
                let rounded = vello_rect.to_rounded_rect(to_vello_radii(radii));
                let vello_color = convert_color(*color);
                scene.fill(Fill::NonZero, Affine::IDENTITY, vello_color, None, &rounded);
            }
            DisplayItem::StrokedRoundedRect {
                rect,
                radii,
                stroke_width,
                color,
            } => {
                let vello_rect = to_vello_rect(rect);
                let rounded = vello_rect.to_rounded_rect(to_vello_radii(radii));
                let vello_color = convert_color(*color);
                let stroke = Stroke::new(f64::from(*stroke_width));
                scene.stroke(&stroke, Affine::IDENTITY, vello_color, None, &rounded);
            }
            DisplayItem::StyledBorderSegment {
                start,
                end,
                width,
                dashes,
                round_caps,
                color,
            } => {
                let vello_color = convert_color(*color);
                let mut stroke = Stroke::new(f64::from(*width));
                if !dashes.is_empty() {
                    let dash_pattern: Vec<f64> = dashes.iter().map(|d| f64::from(*d)).collect();
                    stroke = stroke.with_dashes(0.0, &dash_pattern);
                }
                if *round_caps {
                    stroke = stroke.with_caps(vello::kurbo::Cap::Round);
                }
                let mut path = BezPath::new();
                path.move_to((f64::from(start.0), f64::from(start.1)));
                path.line_to((f64::from(end.0), f64::from(end.1)));
                scene.stroke(&stroke, Affine::IDENTITY, vello_color, None, &path);
            }
            DisplayItem::RoundedBorderRing {
                outer_rect,
                outer_radii,
                inner_rect,
                inner_radii,
                color,
            } => {
                let outer = to_vello_rect(outer_rect).to_rounded_rect(to_vello_radii(outer_radii));
                let inner = to_vello_rect(inner_rect).to_rounded_rect(to_vello_radii(inner_radii));
                let mut path = BezPath::new();
                // Outer path (clockwise)
                for el in outer.path_elements(0.1) {
                    path.push(el);
                }
                // Inner path — same winding direction as outer, but EvenOdd fill
                // treats the overlapping region as "outside", creating the ring.
                for el in inner.path_elements(0.1) {
                    path.push(el);
                }
                let vello_color = convert_color(*color);
                scene.fill(Fill::EvenOdd, Affine::IDENTITY, vello_color, None, &path);
            }
            DisplayItem::Image {
                painting_area,
                pixels,
                image_width,
                image_height,
                position,
                size,
                repeat,
                opacity,
            } => {
                if *image_width > 0 && *image_height > 0 && size.0 > 0.0 && size.1 > 0.0 {
                    let clip = to_vello_rect(painting_area);
                    // Always clip to painting area (tiled images must not overflow)
                    scene.push_layer(
                        Fill::NonZero,
                        Mix::Normal,
                        if *opacity < 1.0 { *opacity } else { 1.0 },
                        Affine::IDENTITY,
                        &clip,
                    );
                    let blob = Blob::from(pixels.as_ref().clone());
                    let image = PenikoImageData {
                        data: blob,
                        format: ImageFormat::Rgba8,
                        alpha_type: ImageAlphaType::Alpha,
                        width: *image_width,
                        height: *image_height,
                    };
                    let scale_x = f64::from(size.0) / f64::from(*image_width);
                    let scale_y = f64::from(size.1) / f64::from(*image_height);

                    let tile_positions =
                        compute_tile_positions(painting_area, position, size, repeat);
                    for (tx, ty) in tile_positions {
                        let draw_x = f64::from(painting_area.x + tx);
                        let draw_y = f64::from(painting_area.y + ty);
                        let transform = Affine::translate((draw_x, draw_y))
                            * Affine::scale_non_uniform(scale_x, scale_y);
                        scene.draw_image(&image, transform);
                    }
                    scene.pop_layer();
                }
            }
            DisplayItem::LinearGradient {
                painting_area,
                angle,
                stops,
                repeating,
                opacity,
            } => {
                let rect = to_vello_rect(painting_area);
                let (start, end) = gradient_line_from_angle(*angle, painting_area);
                let vello_stops: Vec<(f32, Color)> =
                    stops.iter().map(|(p, c)| (*p, convert_color(*c))).collect();
                let mut grad = Gradient::new_linear(start, end).with_stops(vello_stops.as_slice());
                if *repeating {
                    grad = grad.with_extend(Extend::Repeat);
                }
                let needs_layer = *opacity < 1.0;
                if needs_layer {
                    scene.push_layer(
                        Fill::NonZero,
                        Mix::Normal,
                        *opacity,
                        Affine::IDENTITY,
                        &rect,
                    );
                }
                scene.fill(Fill::NonZero, Affine::IDENTITY, &grad, None, &rect);
                if needs_layer {
                    scene.pop_layer();
                }
            }
            DisplayItem::RadialGradient {
                painting_area,
                center,
                radii,
                stops,
                repeating,
                opacity,
            } => {
                let rect = to_vello_rect(painting_area);
                let cx = f64::from(center.0);
                let cy = f64::from(center.1);
                let rx = f64::from(radii.0).max(0.001);
                let ry = f64::from(radii.1).max(0.001);
                let vello_stops: Vec<(f32, Color)> =
                    stops.iter().map(|(p, c)| (*p, convert_color(*c))).collect();
                // Use circular gradient with aspect transform for ellipses
                #[allow(clippy::cast_possible_truncation)]
                let r = rx as f32;
                let mut grad = Gradient::new_radial((cx, cy), r).with_stops(vello_stops.as_slice());
                if *repeating {
                    grad = grad.with_extend(Extend::Repeat);
                }
                let needs_layer = *opacity < 1.0;
                if needs_layer {
                    scene.push_layer(
                        Fill::NonZero,
                        Mix::Normal,
                        *opacity,
                        Affine::IDENTITY,
                        &rect,
                    );
                }
                // Apply aspect ratio transform for ellipse
                let aspect = ry / rx;
                let transform = Affine::translate((cx, cy))
                    * Affine::scale_non_uniform(1.0, aspect)
                    * Affine::translate((-cx, -cy));
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &grad,
                    Some(transform),
                    &rect,
                );
                if needs_layer {
                    scene.pop_layer();
                }
            }
            DisplayItem::ConicGradient {
                painting_area,
                center,
                start_angle,
                end_angle,
                stops,
                repeating,
                opacity,
            } => {
                let rect = to_vello_rect(painting_area);
                let cx = f64::from(center.0);
                let cy = f64::from(center.1);
                // Convert degrees to turns for vello sweep gradient
                let vello_stops: Vec<(f32, Color)> = stops
                    .iter()
                    .map(|(p, c)| {
                        // p is in degrees; normalize to 0.0-1.0 within start..end range
                        let range = end_angle - start_angle;
                        let t = if range > 0.0 {
                            (p - start_angle) / range
                        } else {
                            0.0
                        };
                        (t, convert_color(*c))
                    })
                    .collect();
                let mut grad = Gradient::new_sweep((cx, cy), *start_angle, *end_angle)
                    .with_stops(vello_stops.as_slice());
                if *repeating {
                    grad = grad.with_extend(Extend::Repeat);
                }
                let needs_layer = *opacity < 1.0;
                if needs_layer {
                    scene.push_layer(
                        Fill::NonZero,
                        Mix::Normal,
                        *opacity,
                        Affine::IDENTITY,
                        &rect,
                    );
                }
                scene.fill(Fill::NonZero, Affine::IDENTITY, &grad, None, &rect);
                if needs_layer {
                    scene.pop_layer();
                }
            }
            DisplayItem::PushClip { rect, radii } => {
                let clip = to_vello_rect(rect);
                let all_zero = radii.iter().all(|r| *r == 0.0);
                if all_zero {
                    scene.push_layer(Fill::NonZero, Mix::Normal, 1.0, Affine::IDENTITY, &clip);
                } else {
                    let rounded = clip.to_rounded_rect(to_vello_radii(radii));
                    scene.push_layer(Fill::NonZero, Mix::Normal, 1.0, Affine::IDENTITY, &rounded);
                }
            }
            DisplayItem::PopClip => {
                scene.pop_layer();
            }
            DisplayItem::Text {
                glyphs,
                font_blob,
                font_index,
                font_size,
                color,
            } => {
                let ptr = Arc::as_ptr(font_blob);
                let font_data = font_cache
                    .entry(ptr)
                    .or_insert_with(|| {
                        let blob = Blob::from(font_blob.as_ref().clone());
                        FontData::new(blob, *font_index)
                    })
                    .clone();
                let vello_color = convert_color(*color);

                let vello_glyphs: Vec<Glyph> = glyphs
                    .iter()
                    .map(|g| Glyph {
                        id: g.glyph_id,
                        x: g.x,
                        y: g.y,
                    })
                    .collect();

                scene
                    .draw_glyphs(&font_data)
                    .font_size(*font_size)
                    .brush(vello_color)
                    .draw(Fill::NonZero, vello_glyphs.into_iter());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_display_list_builds_empty_scene() {
        let mut scene = Scene::new();
        let mut fc = HashMap::new();
        let dl = DisplayList::default();
        build_scene(&mut scene, &dl, &mut fc);
        // Scene was constructed without panic — smoke test passes.
    }

    #[test]
    fn solid_rect_builds_scene() {
        let mut scene = Scene::new();
        let mut fc = HashMap::new();
        let dl = DisplayList(vec![DisplayItem::SolidRect {
            rect: Rect::new(10.0, 20.0, 100.0, 50.0),
            color: CssColor::RED,
        }]);
        build_scene(&mut scene, &dl, &mut fc);
        // Scene contains data (encoding is non-empty).
    }

    #[test]
    fn image_builds_scene() {
        use elidex_plugin::background::{BgRepeat, BgRepeatAxis};
        let mut scene = Scene::new();
        let mut fc = HashMap::new();
        let dl = DisplayList(vec![DisplayItem::Image {
            painting_area: Rect::new(10.0, 20.0, 100.0, 50.0),
            pixels: Arc::new(vec![255u8; 4 * 2 * 2]), // 2×2 white
            image_width: 2,
            image_height: 2,
            position: (0.0, 0.0),
            size: (100.0, 50.0),
            repeat: BgRepeat {
                x: BgRepeatAxis::NoRepeat,
                y: BgRepeatAxis::NoRepeat,
            },
            opacity: 1.0,
        }]);
        build_scene(&mut scene, &dl, &mut fc);
        // Should not panic — smoke test.
    }

    #[test]
    fn rounded_rect_builds_scene() {
        let mut scene = Scene::new();
        let mut fc = HashMap::new();
        let dl = DisplayList(vec![DisplayItem::RoundedRect {
            rect: Rect::new(10.0, 20.0, 100.0, 50.0),
            radii: [8.0, 8.0, 8.0, 8.0],
            color: CssColor::BLUE,
        }]);
        build_scene(&mut scene, &dl, &mut fc);
        // Should not panic — smoke test.
    }

    #[test]
    fn stroked_rounded_rect_builds_scene() {
        let mut scene = Scene::new();
        let mut fc = HashMap::new();
        let dl = DisplayList(vec![DisplayItem::StrokedRoundedRect {
            rect: Rect::new(10.0, 20.0, 8.0, 8.0),
            radii: [4.0, 4.0, 4.0, 4.0],
            stroke_width: 1.0,
            color: CssColor::BLACK,
        }]);
        build_scene(&mut scene, &dl, &mut fc);
        // Should not panic — smoke test for stroked rounded rect.
    }

    #[test]
    fn rounded_border_ring_builds_scene() {
        let mut scene = Scene::new();
        let mut fc = HashMap::new();
        let dl = DisplayList(vec![DisplayItem::RoundedBorderRing {
            outer_rect: Rect::new(0.0, 0.0, 104.0, 54.0),
            outer_radii: [10.0, 10.0, 10.0, 10.0],
            inner_rect: Rect::new(2.0, 2.0, 100.0, 50.0),
            inner_radii: [8.0, 8.0, 8.0, 8.0],
            color: CssColor::BLACK,
        }]);
        build_scene(&mut scene, &dl, &mut fc);
        // Should not panic — smoke test for rounded border ring.
    }

    #[test]
    fn push_pop_clip_builds_scene() {
        let mut scene = Scene::new();
        let mut fc = HashMap::new();
        let dl = DisplayList(vec![
            DisplayItem::PushClip {
                rect: Rect::new(0.0, 0.0, 200.0, 100.0),
                radii: [0.0; 4],
            },
            DisplayItem::SolidRect {
                rect: Rect::new(10.0, 10.0, 50.0, 50.0),
                color: CssColor::RED,
            },
            DisplayItem::PopClip,
        ]);
        build_scene(&mut scene, &dl, &mut fc);
        // Should not panic — smoke test for clip layer.
    }

    #[test]
    fn image_repeat_builds_scene() {
        use elidex_plugin::background::{BgRepeat, BgRepeatAxis};
        let mut scene = Scene::new();
        let mut fc = HashMap::new();
        let dl = DisplayList(vec![DisplayItem::Image {
            painting_area: Rect::new(0.0, 0.0, 200.0, 200.0),
            pixels: Arc::new(vec![255u8; 4 * 2 * 2]),
            image_width: 2,
            image_height: 2,
            position: (0.0, 0.0),
            size: (50.0, 50.0),
            repeat: BgRepeat {
                x: BgRepeatAxis::Repeat,
                y: BgRepeatAxis::Repeat,
            },
            opacity: 1.0,
        }]);
        build_scene(&mut scene, &dl, &mut fc);
    }

    #[test]
    fn tile_positions_no_repeat() {
        use elidex_plugin::background::{BgRepeat, BgRepeatAxis};
        let area = Rect::new(0.0, 0.0, 400.0, 300.0);
        let repeat = BgRepeat {
            x: BgRepeatAxis::NoRepeat,
            y: BgRepeatAxis::NoRepeat,
        };
        let positions = compute_tile_positions(&area, &(10.0, 20.0), &(100.0, 50.0), &repeat);
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0], (10.0, 20.0));
    }

    #[test]
    fn tile_positions_repeat() {
        use elidex_plugin::background::{BgRepeat, BgRepeatAxis};
        let area = Rect::new(0.0, 0.0, 200.0, 100.0);
        let repeat = BgRepeat {
            x: BgRepeatAxis::Repeat,
            y: BgRepeatAxis::Repeat,
        };
        let positions = compute_tile_positions(&area, &(0.0, 0.0), &(50.0, 50.0), &repeat);
        // Must cover the entire painting area — at least 4 columns × 2 rows
        assert!(positions.len() >= 8);
        // All visible tiles must intersect the painting area
        for &(x, y) in &positions {
            assert!(x < 200.0 && y < 100.0);
        }
    }

    #[test]
    fn tile_positions_space() {
        use elidex_plugin::background::{BgRepeat, BgRepeatAxis};
        let area = Rect::new(0.0, 0.0, 250.0, 100.0);
        let repeat = BgRepeat {
            x: BgRepeatAxis::Space,
            y: BgRepeatAxis::NoRepeat,
        };
        let positions = compute_tile_positions(&area, &(0.0, 0.0), &(100.0, 50.0), &repeat);
        // floor(250/100) = 2 tiles in x, 1 in y → 2 tiles
        assert_eq!(positions.len(), 2);
        // First tile at x=0, second at x=150 (50px space between)
        assert!((positions[0].0).abs() < 0.1);
        assert!((positions[1].0 - 150.0).abs() < 0.1);
    }

    #[test]
    fn tile_positions_round() {
        use elidex_plugin::background::{BgRepeat, BgRepeatAxis};
        let area = Rect::new(0.0, 0.0, 250.0, 100.0);
        let repeat = BgRepeat {
            x: BgRepeatAxis::Round,
            y: BgRepeatAxis::NoRepeat,
        };
        // round(250/100) = 3 tiles, each 250/3 ≈ 83.3px
        let positions = compute_tile_positions(&area, &(0.0, 0.0), &(100.0, 50.0), &repeat);
        // Must have at least 3 tiles covering the 250px area with ~83px tiles
        assert!(positions.len() >= 3);
    }

    #[test]
    fn styled_border_segment_dashed_builds_scene() {
        let mut scene = Scene::new();
        let mut fc = HashMap::new();
        let dl = DisplayList(vec![DisplayItem::StyledBorderSegment {
            start: (0.0, 1.0),
            end: (100.0, 1.0),
            width: 2.0,
            dashes: vec![6.0, 2.0],
            round_caps: false,
            color: CssColor::RED,
        }]);
        build_scene(&mut scene, &dl, &mut fc);
        // Should not panic — smoke test for dashed border segment.
    }

    #[test]
    fn styled_border_segment_dotted_builds_scene() {
        let mut scene = Scene::new();
        let mut fc = HashMap::new();
        let dl = DisplayList(vec![DisplayItem::StyledBorderSegment {
            start: (1.5, 0.0),
            end: (1.5, 50.0),
            width: 3.0,
            dashes: vec![0.001, 6.0],
            round_caps: true,
            color: CssColor::BLUE,
        }]);
        build_scene(&mut scene, &dl, &mut fc);
        // Should not panic — smoke test for dotted border segment.
    }
}
