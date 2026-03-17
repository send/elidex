//! Background, border, image, and list marker painting.

use std::sync::Arc;

use elidex_ecs::{BackgroundImages, EcsDom, Entity};
use elidex_plugin::background::{
    BackgroundImage, BackgroundLayer, BgPosition, BgPositionAxis, BgSize, BgSizeDimension,
};
use elidex_plugin::{BorderStyle, ComputedStyle, CssColor, LayoutBox, ListStyleType, Rect};
use elidex_text::{shape_text, to_fontdb_style, FontDatabase};

use crate::display_list::{DisplayItem, DisplayList};
use crate::font_cache::FontCache;

use super::{
    families_as_refs, place_glyphs, DECIMAL_MARKER_GAP_FACTOR, MARKER_SIZE_FACTOR,
    MARKER_X_OFFSET_FACTOR, MARKER_Y_CENTER_FACTOR,
};

/// Maximum depth for ancestor walks to prevent infinite loops on corrupted trees.
const MAX_ANCESTOR_DEPTH: u32 = 1000;

/// Apply opacity to a color by multiplying its alpha channel.
#[must_use]
pub(crate) fn apply_opacity(color: CssColor, opacity: f32) -> CssColor {
    if opacity >= 1.0 {
        return color;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let a = (f32::from(color.a) * opacity).round() as u8;
    CssColor {
        r: color.r,
        g: color.g,
        b: color.b,
        a,
    }
}

/// Emit background color + image layers with opacity applied.
pub(crate) fn emit_background(
    lb: &LayoutBox,
    bg: CssColor,
    border_radii: [f32; 4],
    opacity: f32,
    bg_images: Option<&BackgroundImages>,
    style: &ComputedStyle,
    dl: &mut DisplayList,
) {
    // 1. Background color
    let color = apply_opacity(bg, opacity);
    if color.a > 0 {
        let rect = lb.border_box();
        let has_radius = border_radii.iter().any(|r| *r > 0.0);
        if has_radius {
            dl.push(DisplayItem::RoundedRect {
                rect,
                radii: border_radii,
                color,
            });
        } else {
            dl.push(DisplayItem::SolidRect { rect, color });
        }
    }

    // 2. Background image layers (bottom-most first, CSS Backgrounds §2.11.1)
    let Some(ref layers) = style.background_layers else {
        return;
    };
    let painting_area = lb.padding_box();

    for (i, layer) in layers.iter().enumerate() {
        emit_bg_layer(layer, i, painting_area, opacity, bg_images, dl);
    }
}

/// Resolve background-size to concrete `(width, height)` in pixels.
///
/// CSS Backgrounds Level 3 §3.9: the concrete size depends on the intrinsic
/// image dimensions and the painting area.
#[must_use]
fn resolve_bg_size(size: &BgSize, painting_area: &Rect, img_w: u32, img_h: u32) -> (f32, f32) {
    #[allow(clippy::cast_precision_loss)]
    let iw = img_w as f32;
    #[allow(clippy::cast_precision_loss)]
    let ih = img_h as f32;
    let pw = painting_area.width;
    let ph = painting_area.height;

    match size {
        BgSize::Cover => {
            if iw <= 0.0 || ih <= 0.0 {
                return (pw, ph);
            }
            let scale = (pw / iw).max(ph / ih);
            (iw * scale, ih * scale)
        }
        BgSize::Contain => {
            if iw <= 0.0 || ih <= 0.0 {
                return (pw, ph);
            }
            let scale = (pw / iw).min(ph / ih);
            (iw * scale, ih * scale)
        }
        BgSize::Explicit(w, h) => {
            let resolved_w = w.as_ref().map(|d| match d {
                BgSizeDimension::Length(v) => *v,
                BgSizeDimension::Percentage(p) => pw * *p / 100.0,
            });
            let resolved_h = h.as_ref().map(|d| match d {
                BgSizeDimension::Length(v) => *v,
                BgSizeDimension::Percentage(p) => ph * *p / 100.0,
            });
            match (resolved_w, resolved_h) {
                (Some(w), Some(h)) => (w, h),
                (Some(w), None) => {
                    // auto height: preserve aspect ratio
                    if iw > 0.0 {
                        (w, w * ih / iw)
                    } else {
                        (w, ih)
                    }
                }
                (None, Some(h)) => {
                    if ih > 0.0 {
                        (h * iw / ih, h)
                    } else {
                        (iw, h)
                    }
                }
                (None, None) => (iw, ih), // auto auto = intrinsic size
            }
        }
    }
}

/// Resolve background-position to `(x, y)` offset within the painting area.
///
/// CSS Backgrounds Level 3 §3.6: percentage positions refer to the difference
/// between the painting area and image size.
#[must_use]
fn resolve_bg_position(pos: &BgPosition, painting_area: &Rect, img_size: (f32, f32)) -> (f32, f32) {
    let x = resolve_position_axis(&pos.x, painting_area.width, img_size.0);
    let y = resolve_position_axis(&pos.y, painting_area.height, img_size.1);
    (x, y)
}

fn resolve_position_axis(axis: &BgPositionAxis, area_dim: f32, img_dim: f32) -> f32 {
    match axis {
        BgPositionAxis::Length(v) => *v,
        BgPositionAxis::Percentage(p) => (area_dim - img_dim) * *p / 100.0,
        BgPositionAxis::Edge(edge, offset) => {
            use elidex_plugin::background::PositionEdge;
            match edge {
                PositionEdge::Left | PositionEdge::Top => *offset,
                PositionEdge::Right | PositionEdge::Bottom => area_dim - img_dim - *offset,
            }
        }
    }
}

/// Emit a single background image layer.
fn emit_bg_layer(
    layer: &BackgroundLayer,
    index: usize,
    painting_area: Rect,
    opacity: f32,
    bg_images: Option<&BackgroundImages>,
    dl: &mut DisplayList,
) {
    match &layer.image {
        BackgroundImage::Url(_) => {
            if let Some(bg_imgs) = bg_images {
                if let Some(Some(img_data)) = bg_imgs.layers.get(index) {
                    if img_data.width > 0 && img_data.height > 0 {
                        let size = resolve_bg_size(
                            &layer.size,
                            &painting_area,
                            img_data.width,
                            img_data.height,
                        );
                        let position = resolve_bg_position(&layer.position, &painting_area, size);
                        dl.push(DisplayItem::Image {
                            painting_area,
                            pixels: Arc::clone(&img_data.pixels),
                            image_width: img_data.width,
                            image_height: img_data.height,
                            position,
                            size,
                            repeat: layer.repeat.clone(),
                            opacity,
                        });
                    }
                }
            }
        }
        BackgroundImage::LinearGradient(lg) => {
            let stops: Vec<(f32, CssColor)> = lg
                .stops
                .iter()
                .map(|s| (s.position, apply_opacity(s.color, opacity)))
                .collect();
            dl.push(DisplayItem::LinearGradient {
                painting_area,
                angle: lg.angle,
                stops,
                repeating: lg.repeating,
                opacity,
            });
        }
        BackgroundImage::RadialGradient(rg) => {
            let cx = painting_area.x + painting_area.width * rg.center.0 / 100.0;
            let cy = painting_area.y + painting_area.height * rg.center.1 / 100.0;
            let rx = if rg.radii.0 > 0.0 {
                rg.radii.0
            } else {
                let dx = painting_area.width.max(0.0);
                let dy = painting_area.height.max(0.0);
                (dx * dx + dy * dy).sqrt() / 2.0
            };
            let ry = if rg.radii.1 > 0.0 { rg.radii.1 } else { rx };
            let stops: Vec<(f32, CssColor)> = rg
                .stops
                .iter()
                .map(|s| (s.position, apply_opacity(s.color, opacity)))
                .collect();
            dl.push(DisplayItem::RadialGradient {
                painting_area,
                center: (cx, cy),
                radii: (rx, ry),
                stops,
                repeating: rg.repeating,
                opacity,
            });
        }
        BackgroundImage::ConicGradient(cg) => {
            let cx = painting_area.x + painting_area.width * cg.center.0 / 100.0;
            let cy = painting_area.y + painting_area.height * cg.center.1 / 100.0;
            let stops: Vec<(f32, CssColor)> = cg
                .stops
                .iter()
                .map(|s| (s.position, apply_opacity(s.color, opacity)))
                .collect();
            dl.push(DisplayItem::ConicGradient {
                painting_area,
                center: (cx, cy),
                start_angle: cg.start_angle,
                end_angle: cg.end_angle,
                stops,
                repeating: cg.repeating,
                opacity,
            });
        }
        BackgroundImage::None | _ => {}
    }
}

/// Compute inner (padding-box) corner radii per CSS Backgrounds 3 §5.2.
///
/// Each inner radius is computed per-axis: horizontal is reduced by the
/// adjacent horizontal border width, vertical by the adjacent vertical
/// border width. The result is clamped to zero. Since `RoundedRect` only
/// supports circular per-corner radii, we use `min(horizontal, vertical)`
/// as a documented simplification.
#[must_use]
fn compute_inner_radii(outer_radii: [f32; 4], border: &elidex_plugin::EdgeSizes) -> [f32; 4] {
    let per_axis = compute_inner_radii_per_axis(outer_radii, border);
    [
        per_axis[0].0.min(per_axis[0].1), // top-left
        per_axis[1].0.min(per_axis[1].1), // top-right
        per_axis[2].0.min(per_axis[2].1), // bottom-right
        per_axis[3].0.min(per_axis[3].1), // bottom-left
    ]
}

/// Compute per-axis inner radii as `(horizontal, vertical)` per corner.
///
/// CSS Backgrounds 3 §5.2: for each corner, the inner horizontal radius is
/// `max(0, outer - adjacent_horizontal_border)` and the inner vertical
/// radius is `max(0, outer - adjacent_vertical_border)`.
#[must_use]
fn compute_inner_radii_per_axis(
    outer_radii: [f32; 4],
    border: &elidex_plugin::EdgeSizes,
) -> [(f32, f32); 4] {
    [
        // top-left: horizontal reduced by border-left, vertical by border-top
        (
            (outer_radii[0] - border.left).max(0.0),
            (outer_radii[0] - border.top).max(0.0),
        ),
        // top-right: horizontal reduced by border-right, vertical by border-top
        (
            (outer_radii[1] - border.right).max(0.0),
            (outer_radii[1] - border.top).max(0.0),
        ),
        // bottom-right: horizontal reduced by border-right, vertical by border-bottom
        (
            (outer_radii[2] - border.right).max(0.0),
            (outer_radii[2] - border.bottom).max(0.0),
        ),
        // bottom-left: horizontal reduced by border-left, vertical by border-bottom
        (
            (outer_radii[3] - border.left).max(0.0),
            (outer_radii[3] - border.bottom).max(0.0),
        ),
    ]
}

/// Emit border display items, or a `RoundedBorderRing` when `border-radius`
/// is set and all border colors are uniform with solid style.
///
/// Each side is drawn only when `border-style != none` and `border-width > 0`.
/// - `Dashed`: stroked line with dash pattern `[3*width, width]` (CSS recommendation).
/// - `Dotted`: stroked line with round caps and near-zero dash length (circular dots).
/// - All other visible styles: solid rectangles (`SolidRect`).
///
/// When `border-radius > 0` and all four border colors match (and all active
/// styles are `Solid`), a single `RoundedBorderRing` is emitted instead of
/// four axis-aligned rectangles.
///
/// **Limitation:** Dashed/dotted borders with `border-radius` fall back to
/// straight-line segments (not curved). Curved dashed/dotted borders require
/// path-based stroke rendering, which is deferred to a future phase.
///
/// Top and bottom borders span the full width. Left and right borders are
/// inset by the top/bottom border widths to avoid overlapping at corners,
/// which would cause visible darkening when `opacity < 1.0`.
pub(crate) fn emit_borders(lb: &LayoutBox, style: &ComputedStyle, dl: &mut DisplayList) {
    let bb = lb.border_box();
    let opacity = style.opacity;

    // Collect which sides are active (style != None && width > 0).
    let sides = [
        (&style.border_top, lb.border.top),
        (&style.border_right, lb.border.right),
        (&style.border_bottom, lb.border.bottom),
        (&style.border_left, lb.border.left),
    ];
    let active_sides: Vec<_> = sides
        .iter()
        .filter(|(s, w)| s.style != BorderStyle::None && *w > 0.0)
        .collect();

    // Try rounded border ring when border-radius is set.
    let has_radius = style.border_radii.iter().any(|r| *r > 0.0);
    if has_radius && !active_sides.is_empty() {
        // Check uniform color and all-solid styles.
        let first_color = active_sides[0].0.color;
        let uniform_color = active_sides.iter().all(|(s, _)| s.color == first_color);
        let all_solid = active_sides
            .iter()
            .all(|(s, _)| s.style == BorderStyle::Solid);

        if uniform_color && all_solid {
            let inner_rect = lb.padding_box();
            let inner_radii = compute_inner_radii(style.border_radii, &lb.border);
            dl.push(DisplayItem::RoundedBorderRing {
                outer_rect: bb,
                outer_radii: style.border_radii,
                inner_rect,
                inner_radii,
                color: apply_opacity(first_color, opacity),
            });
            return;
        }
    }

    // Per-side border emission (SolidRect or StyledBorderSegment).
    // top (full width)
    if style.border_top.style != BorderStyle::None && lb.border.top > 0.0 {
        let color = apply_opacity(style.border_top.color, opacity);
        let w = lb.border.top;
        emit_border_side(
            style.border_top.style,
            (bb.x, bb.y + w / 2.0),
            (bb.x + bb.width, bb.y + w / 2.0),
            w,
            elidex_plugin::Rect::new(bb.x, bb.y, bb.width, w),
            color,
            dl,
        );
    }
    // bottom (full width)
    if style.border_bottom.style != BorderStyle::None && lb.border.bottom > 0.0 {
        let color = apply_opacity(style.border_bottom.color, opacity);
        let w = lb.border.bottom;
        emit_border_side(
            style.border_bottom.style,
            (bb.x, bb.y + bb.height - w / 2.0),
            (bb.x + bb.width, bb.y + bb.height - w / 2.0),
            w,
            elidex_plugin::Rect::new(bb.x, bb.y + bb.height - w, bb.width, w),
            color,
            dl,
        );
    }
    // right (inset by top/bottom to avoid corner overlap)
    let v_inset_top = lb.border.top;
    let v_inset_bottom = lb.border.bottom;
    let v_height = (bb.height - v_inset_top - v_inset_bottom).max(0.0);
    if style.border_right.style != BorderStyle::None && lb.border.right > 0.0 && v_height > 0.0 {
        let color = apply_opacity(style.border_right.color, opacity);
        let w = lb.border.right;
        emit_border_side(
            style.border_right.style,
            (bb.x + bb.width - w / 2.0, bb.y + v_inset_top),
            (bb.x + bb.width - w / 2.0, bb.y + v_inset_top + v_height),
            w,
            elidex_plugin::Rect::new(bb.x + bb.width - w, bb.y + v_inset_top, w, v_height),
            color,
            dl,
        );
    }
    // left (inset by top/bottom to avoid corner overlap)
    if style.border_left.style != BorderStyle::None && lb.border.left > 0.0 && v_height > 0.0 {
        let color = apply_opacity(style.border_left.color, opacity);
        let w = lb.border.left;
        emit_border_side(
            style.border_left.style,
            (bb.x + w / 2.0, bb.y + v_inset_top),
            (bb.x + w / 2.0, bb.y + v_inset_top + v_height),
            w,
            elidex_plugin::Rect::new(bb.x, bb.y + v_inset_top, w, v_height),
            color,
            dl,
        );
    }
}

/// Emit a single border side as either a `StyledBorderSegment` (dashed/dotted)
/// or a `SolidRect` (all other visible styles).
fn emit_border_side(
    border_style: BorderStyle,
    start: (f32, f32),
    end: (f32, f32),
    width: f32,
    solid_rect: Rect,
    color: CssColor,
    dl: &mut DisplayList,
) {
    match border_style {
        BorderStyle::Dashed => {
            dl.push(DisplayItem::StyledBorderSegment {
                start,
                end,
                width,
                dashes: vec![3.0 * width, width],
                round_caps: false,
                color,
            });
        }
        BorderStyle::Dotted => {
            // Near-zero dash with round caps produces circular dots.
            // Gap of 2*width ensures dot edges are separated by ~1 dot diameter.
            dl.push(DisplayItem::StyledBorderSegment {
                start,
                end,
                width,
                dashes: vec![0.001, 2.0 * width],
                round_caps: true,
                color,
            });
        }
        BorderStyle::None | BorderStyle::Hidden => {
            // Should not reach here (caller checks for None), but guard anyway.
        }
        _ => {
            // Solid, Double, Groove, Ridge, Inset, Outset — all rendered as solid rect.
            dl.push(DisplayItem::SolidRect {
                rect: solid_rect,
                color,
            });
        }
    }
}

/// Emit a list marker for a `display: list-item` element.
///
/// - `disc`/`circle`/`square`: small shape rendered to the left of the content box.
/// - `decimal`: rendered as "N." text to the left.
///
/// The marker is positioned in the element's left padding area, vertically
/// centered on the first line (approximated by font ascent).
pub(crate) fn emit_list_marker_with_counter(
    lb: &LayoutBox,
    style: &ComputedStyle,
    counter: usize,
    font_db: &FontDatabase,
    font_cache: &mut FontCache,
    dl: &mut DisplayList,
) {
    let marker_size = style.font_size * MARKER_SIZE_FACTOR;
    let marker_x = lb.content.x - style.font_size * MARKER_X_OFFSET_FACTOR;

    let families = families_as_refs(&style.font_family);
    let font_style = to_fontdb_style(style.font_style);
    let ascent = font_db
        .query(&families, style.font_weight, font_style)
        .and_then(|fid| font_db.font_metrics(fid, style.font_size))
        .map_or(style.font_size, |m| m.ascent);
    let marker_y =
        lb.content.y + ascent * MARKER_Y_CENTER_FACTOR - marker_size * MARKER_Y_CENTER_FACTOR;

    let color = apply_opacity(style.color, style.opacity);

    // Common marker rect for disc/circle/square (R2: hoisted before match).
    let marker_rect = Rect::new(marker_x, marker_y, marker_size, marker_size);

    match style.list_style_type {
        ListStyleType::Disc => {
            let r = marker_size / 2.0;
            dl.push(DisplayItem::RoundedRect {
                rect: marker_rect,
                radii: [r, r, r, r],
                color,
            });
        }
        ListStyleType::Circle => {
            let r = marker_size / 2.0;
            dl.push(DisplayItem::StrokedRoundedRect {
                rect: marker_rect,
                radii: [r, r, r, r],
                stroke_width: 1.0,
                color,
            });
        }
        ListStyleType::Square => {
            dl.push(DisplayItem::SolidRect {
                rect: marker_rect,
                color,
            });
        }
        ListStyleType::Decimal => {
            let marker_text = format!("{counter}.");
            let Some(font_id) = font_db.query(&families, style.font_weight, font_style) else {
                return;
            };
            let Some(shaped) = shape_text(font_db, font_id, style.font_size, &marker_text) else {
                return;
            };
            let Some((font_blob, font_index)) = font_cache.get(font_db, font_id) else {
                return;
            };
            let text_width: f32 = shaped.glyphs.iter().map(|g| g.x_advance).sum();
            if !text_width.is_finite() {
                return;
            }
            let baseline_y = lb.content.y + ascent;
            let mut text_x =
                lb.content.x - text_width - style.font_size * DECIMAL_MARKER_GAP_FACTOR;
            let glyphs = place_glyphs(
                &shaped.glyphs,
                &mut text_x,
                baseline_y,
                0.0,
                0.0,
                &marker_text,
            );
            dl.push(DisplayItem::Text {
                glyphs,
                font_blob,
                font_index,
                font_size: style.font_size,
                color,
            });
        }
        ListStyleType::None => {}
    }
}

/// Walk up the ancestor chain to find the nearest entity with a `LayoutBox`.
///
/// Starts with `entity` itself, then checks its parent, grandparent, etc.
/// Returns `None` if no ancestor has a `LayoutBox` (capped at [`MAX_ANCESTOR_DEPTH`]).
#[must_use]
pub(crate) fn find_nearest_layout_box(dom: &EcsDom, entity: Entity) -> Option<LayoutBox> {
    let mut current = entity;
    for _ in 0..MAX_ANCESTOR_DEPTH {
        if let Ok(lb) = dom.world().get::<&LayoutBox>(current) {
            return Some((*lb).clone());
        }
        current = dom.get_parent(current)?;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::background::PositionEdge;

    fn area(w: f32, h: f32) -> Rect {
        Rect::new(0.0, 0.0, w, h)
    }

    // --- resolve_bg_size ---

    #[test]
    fn bg_size_auto_auto_uses_intrinsic() {
        let size = resolve_bg_size(&BgSize::default(), &area(400.0, 300.0), 100, 50);
        assert_eq!(size, (100.0, 50.0));
    }

    #[test]
    fn bg_size_cover() {
        // 100x50 image in 400x300 area → scale by max(4.0, 6.0) = 6.0
        let size = resolve_bg_size(&BgSize::Cover, &area(400.0, 300.0), 100, 50);
        assert!((size.0 - 600.0).abs() < 0.1);
        assert!((size.1 - 300.0).abs() < 0.1);
    }

    #[test]
    fn bg_size_contain() {
        // 100x50 image in 400x300 area → scale by min(4.0, 6.0) = 4.0
        let size = resolve_bg_size(&BgSize::Contain, &area(400.0, 300.0), 100, 50);
        assert!((size.0 - 400.0).abs() < 0.1);
        assert!((size.1 - 200.0).abs() < 0.1);
    }

    #[test]
    fn bg_size_explicit_both() {
        let size = resolve_bg_size(
            &BgSize::Explicit(
                Some(BgSizeDimension::Length(200.0)),
                Some(BgSizeDimension::Length(100.0)),
            ),
            &area(400.0, 300.0),
            100,
            50,
        );
        assert_eq!(size, (200.0, 100.0));
    }

    #[test]
    fn bg_size_explicit_width_auto_height() {
        // width=200px, height=auto → height = 200 * 50 / 100 = 100
        let size = resolve_bg_size(
            &BgSize::Explicit(Some(BgSizeDimension::Length(200.0)), None),
            &area(400.0, 300.0),
            100,
            50,
        );
        assert_eq!(size, (200.0, 100.0));
    }

    #[test]
    fn bg_size_percentage() {
        let size = resolve_bg_size(
            &BgSize::Explicit(
                Some(BgSizeDimension::Percentage(50.0)),
                Some(BgSizeDimension::Percentage(50.0)),
            ),
            &area(400.0, 300.0),
            100,
            50,
        );
        assert_eq!(size, (200.0, 150.0));
    }

    // --- resolve_bg_position ---

    #[test]
    fn bg_position_default_zero() {
        let pos = resolve_bg_position(&BgPosition::default(), &area(400.0, 300.0), (100.0, 50.0));
        // 0% of (400-100) = 0.0, 0% of (300-50) = 0.0
        assert_eq!(pos, (0.0, 0.0));
    }

    #[test]
    fn bg_position_center() {
        let pos = resolve_bg_position(
            &BgPosition {
                x: BgPositionAxis::Percentage(50.0),
                y: BgPositionAxis::Percentage(50.0),
            },
            &area(400.0, 300.0),
            (100.0, 50.0),
        );
        // 50% of (400-100) = 150.0, 50% of (300-50) = 125.0
        assert!((pos.0 - 150.0).abs() < 0.1);
        assert!((pos.1 - 125.0).abs() < 0.1);
    }

    #[test]
    fn bg_position_length() {
        let pos = resolve_bg_position(
            &BgPosition {
                x: BgPositionAxis::Length(10.0),
                y: BgPositionAxis::Length(20.0),
            },
            &area(400.0, 300.0),
            (100.0, 50.0),
        );
        assert_eq!(pos, (10.0, 20.0));
    }

    #[test]
    fn bg_position_right_bottom_edge() {
        let pos = resolve_bg_position(
            &BgPosition {
                x: BgPositionAxis::Edge(PositionEdge::Right, 10.0),
                y: BgPositionAxis::Edge(PositionEdge::Bottom, 20.0),
            },
            &area(400.0, 300.0),
            (100.0, 50.0),
        );
        // right 10px → 400 - 100 - 10 = 290
        // bottom 20px → 300 - 50 - 20 = 230
        assert!((pos.0 - 290.0).abs() < 0.1);
        assert!((pos.1 - 230.0).abs() < 0.1);
    }

    // --- compute_inner_radii ---

    #[test]
    fn inner_radii_uniform_border() {
        use elidex_plugin::EdgeSizes;
        let border = EdgeSizes {
            top: 3.0,
            right: 3.0,
            bottom: 3.0,
            left: 3.0,
        };
        let inner = compute_inner_radii([10.0; 4], &border);
        // Per-axis: (10-3, 10-3) = (7, 7) → min = 7
        assert_eq!(inner, [7.0, 7.0, 7.0, 7.0]);
    }

    #[test]
    fn inner_radii_asymmetric_border() {
        use elidex_plugin::EdgeSizes;
        let border = EdgeSizes {
            top: 5.0,
            right: 2.0,
            bottom: 3.0,
            left: 8.0,
        };
        let inner = compute_inner_radii([10.0, 10.0, 10.0, 10.0], &border);
        // top-left: h=10-8=2, v=10-5=5 → min=2
        // top-right: h=10-2=8, v=10-5=5 → min=5
        // bottom-right: h=10-2=8, v=10-3=7 → min=7
        // bottom-left: h=10-8=2, v=10-3=7 → min=2
        assert_eq!(inner, [2.0, 5.0, 7.0, 2.0]);
    }

    #[test]
    fn inner_radii_per_axis_asymmetric() {
        use elidex_plugin::EdgeSizes;
        let border = EdgeSizes {
            top: 2.0,
            right: 2.0,
            bottom: 2.0,
            left: 10.0,
        };
        let per_axis = compute_inner_radii_per_axis([10.0; 4], &border);
        // top-left: h=10-10=0, v=10-2=8
        assert_eq!(per_axis[0], (0.0, 8.0));
        // top-right: h=10-2=8, v=10-2=8
        assert_eq!(per_axis[1], (8.0, 8.0));
        // bottom-right: h=10-2=8, v=10-2=8
        assert_eq!(per_axis[2], (8.0, 8.0));
        // bottom-left: h=10-10=0, v=10-2=8
        assert_eq!(per_axis[3], (0.0, 8.0));
        // min(h,v) used for RoundedRect
        let inner = compute_inner_radii([10.0; 4], &border);
        assert_eq!(inner, [0.0, 8.0, 8.0, 0.0]);
    }

    #[test]
    fn inner_radii_clamped_to_zero() {
        use elidex_plugin::EdgeSizes;
        let border = EdgeSizes {
            top: 15.0,
            right: 15.0,
            bottom: 15.0,
            left: 15.0,
        };
        let inner = compute_inner_radii([10.0; 4], &border);
        // 10 - 15 = -5, clamped to 0
        assert_eq!(inner, [0.0, 0.0, 0.0, 0.0]);
    }

    // --- emit_borders: dashed/dotted ---

    /// Helper to build a `LayoutBox` + `ComputedStyle` with uniform border for testing.
    fn make_bordered_box(
        border_width: f32,
        border_style: BorderStyle,
        border_color: CssColor,
    ) -> (LayoutBox, ComputedStyle) {
        use elidex_plugin::{BorderSide, EdgeSizes};
        let lb = LayoutBox {
            content: Rect::new(10.0, 10.0, 100.0, 50.0),
            padding: EdgeSizes::default(),
            border: EdgeSizes {
                top: border_width,
                right: border_width,
                bottom: border_width,
                left: border_width,
            },
            margin: EdgeSizes::default(),
        };
        let side = BorderSide {
            width: border_width,
            style: border_style,
            color: border_color,
        };
        let style = ComputedStyle {
            border_top: side,
            border_right: side,
            border_bottom: side,
            border_left: side,
            ..ComputedStyle::default()
        };
        (lb, style)
    }

    #[test]
    fn emit_borders_dashed_produces_styled_segments() {
        let (lb, style) = make_bordered_box(2.0, BorderStyle::Dashed, CssColor::RED);
        let mut dl = DisplayList::default();
        emit_borders(&lb, &style, &mut dl);
        // 4 sides → 4 StyledBorderSegment items
        assert_eq!(dl.len(), 4);
        for item in dl.iter() {
            match item {
                DisplayItem::StyledBorderSegment {
                    width,
                    dashes,
                    round_caps,
                    ..
                } => {
                    assert!((width - 2.0).abs() < f32::EPSILON);
                    assert_eq!(dashes.len(), 2);
                    // Dash pattern: [3*width, width] = [6.0, 2.0]
                    assert!((dashes[0] - 6.0).abs() < f32::EPSILON);
                    assert!((dashes[1] - 2.0).abs() < f32::EPSILON);
                    assert!(!round_caps);
                }
                other => panic!("Expected StyledBorderSegment, got {other:?}"),
            }
        }
    }

    #[test]
    fn emit_borders_dotted_produces_round_cap_segments() {
        let (lb, style) = make_bordered_box(3.0, BorderStyle::Dotted, CssColor::BLUE);
        let mut dl = DisplayList::default();
        emit_borders(&lb, &style, &mut dl);
        assert_eq!(dl.len(), 4);
        for item in dl.iter() {
            match item {
                DisplayItem::StyledBorderSegment {
                    width,
                    dashes,
                    round_caps,
                    ..
                } => {
                    assert!((width - 3.0).abs() < f32::EPSILON);
                    assert_eq!(dashes.len(), 2);
                    assert!(dashes[0] < 0.01); // near-zero dash
                    assert!((dashes[1] - 6.0).abs() < f32::EPSILON); // 2*width gap
                    assert!(round_caps);
                }
                other => panic!("Expected StyledBorderSegment, got {other:?}"),
            }
        }
    }

    #[test]
    fn emit_borders_solid_still_produces_solid_rect() {
        let (lb, style) = make_bordered_box(2.0, BorderStyle::Solid, CssColor::BLACK);
        let mut dl = DisplayList::default();
        emit_borders(&lb, &style, &mut dl);
        // All 4 sides should be SolidRect
        assert_eq!(dl.len(), 4);
        for item in dl.iter() {
            assert!(
                matches!(item, DisplayItem::SolidRect { .. }),
                "Expected SolidRect, got {item:?}"
            );
        }
    }

    #[test]
    fn emit_borders_none_produces_nothing() {
        let (lb, style) = make_bordered_box(2.0, BorderStyle::None, CssColor::BLACK);
        let mut dl = DisplayList::default();
        emit_borders(&lb, &style, &mut dl);
        assert!(dl.is_empty());
    }
}
