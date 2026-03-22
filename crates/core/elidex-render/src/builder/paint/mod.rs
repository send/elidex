//! Background, border, image, and list marker painting.

#[cfg(test)]
mod tests;

use std::sync::Arc;

use elidex_ecs::{BackgroundImages, EcsDom, Entity};
use elidex_plugin::background::{
    BackgroundImage, BackgroundLayer, BgPosition, BgPositionAxis, BgSize, BgSizeDimension,
};
use elidex_plugin::{
    BorderStyle, ComputedStyle, CssColor, LayoutBox, ListStyleType, Point, Rect, Size,
};
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

/// Resolve background-size to concrete pixel dimensions.
///
/// CSS Backgrounds Level 3 §3.9: the concrete size depends on the intrinsic
/// image dimensions and the painting area.
#[must_use]
fn resolve_bg_size(size: &BgSize, painting_area: &Rect, img_w: u32, img_h: u32) -> Size {
    #[allow(clippy::cast_precision_loss)]
    let iw = img_w as f32;
    #[allow(clippy::cast_precision_loss)]
    let ih = img_h as f32;
    let pw = painting_area.size.width;
    let ph = painting_area.size.height;

    match size {
        BgSize::Cover => {
            if iw <= 0.0 || ih <= 0.0 {
                return Size::new(pw, ph);
            }
            let scale = (pw / iw).max(ph / ih);
            Size::new(iw * scale, ih * scale)
        }
        BgSize::Contain => {
            if iw <= 0.0 || ih <= 0.0 {
                return Size::new(pw, ph);
            }
            let scale = (pw / iw).min(ph / ih);
            Size::new(iw * scale, ih * scale)
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
                (Some(w), Some(h)) => Size::new(w, h),
                (Some(w), None) => {
                    // auto height: preserve aspect ratio
                    if iw > 0.0 {
                        Size::new(w, w * ih / iw)
                    } else {
                        Size::new(w, ih)
                    }
                }
                (None, Some(h)) => {
                    if ih > 0.0 {
                        Size::new(h * iw / ih, h)
                    } else {
                        Size::new(iw, h)
                    }
                }
                (None, None) => Size::new(iw, ih), // auto auto = intrinsic size
            }
        }
    }
}

/// Resolve background-position to an offset within the painting area.
///
/// CSS Backgrounds Level 3 §3.6: percentage positions refer to the difference
/// between the painting area and image size.
#[must_use]
fn resolve_bg_position(pos: &BgPosition, painting_area: &Rect, img_size: Size) -> Point {
    let x = resolve_position_axis(&pos.x, painting_area.size.width, img_size.width);
    let y = resolve_position_axis(&pos.y, painting_area.size.height, img_size.height);
    Point::new(x, y)
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
            let center = painting_area.point_at_pct(rg.center);
            let rx = if rg.radii.width > 0.0 {
                rg.radii.width
            } else {
                let dx = painting_area.size.width.max(0.0);
                let dy = painting_area.size.height.max(0.0);
                (dx * dx + dy * dy).sqrt() / 2.0
            };
            let ry = if rg.radii.height > 0.0 {
                rg.radii.height
            } else {
                rx
            };
            let stops: Vec<(f32, CssColor)> = rg
                .stops
                .iter()
                .map(|s| (s.position, apply_opacity(s.color, opacity)))
                .collect();
            dl.push(DisplayItem::RadialGradient {
                painting_area,
                center,
                radii: Size::new(rx, ry),
                stops,
                repeating: rg.repeating,
                opacity,
            });
        }
        BackgroundImage::ConicGradient(cg) => {
            let center = painting_area.point_at_pct(cg.center);
            let stops: Vec<(f32, CssColor)> = cg
                .stops
                .iter()
                .map(|s| (s.position, apply_opacity(s.color, opacity)))
                .collect();
            dl.push(DisplayItem::ConicGradient {
                painting_area,
                center,
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
        emit_h_line(
            style.border_top.style,
            bb.origin.x,
            bb.right(),
            bb.origin.y,
            lb.border.top,
            color,
            dl,
        );
    }
    // bottom (full width)
    if style.border_bottom.style != BorderStyle::None && lb.border.bottom > 0.0 {
        let color = apply_opacity(style.border_bottom.color, opacity);
        emit_h_line(
            style.border_bottom.style,
            bb.origin.x,
            bb.right(),
            bb.bottom() - lb.border.bottom,
            lb.border.bottom,
            color,
            dl,
        );
    }
    // right (inset by top/bottom to avoid corner overlap)
    let v_inset_top = lb.border.top;
    let v_inset_bottom = lb.border.bottom;
    let v_height = (bb.size.height - v_inset_top - v_inset_bottom).max(0.0);
    if style.border_right.style != BorderStyle::None && lb.border.right > 0.0 && v_height > 0.0 {
        let color = apply_opacity(style.border_right.color, opacity);
        emit_v_line(
            style.border_right.style,
            bb.origin.y + v_inset_top,
            bb.origin.y + v_inset_top + v_height,
            bb.right() - lb.border.right,
            lb.border.right,
            color,
            dl,
        );
    }
    // left (inset by top/bottom to avoid corner overlap)
    if style.border_left.style != BorderStyle::None && lb.border.left > 0.0 && v_height > 0.0 {
        let color = apply_opacity(style.border_left.color, opacity);
        emit_v_line(
            style.border_left.style,
            bb.origin.y + v_inset_top,
            bb.origin.y + v_inset_top + v_height,
            bb.origin.x,
            lb.border.left,
            color,
            dl,
        );
    }
}

/// Emit a horizontal border/rule line (top or bottom edge).
///
/// `x_start`/`x_end` are the horizontal extent, `y` is the line center,
/// and `w` is the line thickness.
fn emit_h_line(
    border_style: BorderStyle,
    x_start: f32,
    x_end: f32,
    y: f32,
    w: f32,
    color: CssColor,
    dl: &mut DisplayList,
) {
    emit_border_side(
        border_style,
        Point::new(x_start, y + w / 2.0),
        Point::new(x_end, y + w / 2.0),
        w,
        Rect::new(x_start, y, x_end - x_start, w),
        color,
        dl,
    );
}

/// Emit a vertical border/rule line (left or right edge).
///
/// `y_start`/`y_end` are the vertical extent, `x` is the line center,
/// and `w` is the line thickness.
fn emit_v_line(
    border_style: BorderStyle,
    y_start: f32,
    y_end: f32,
    x: f32,
    w: f32,
    color: CssColor,
    dl: &mut DisplayList,
) {
    emit_border_side(
        border_style,
        Point::new(x + w / 2.0, y_start),
        Point::new(x + w / 2.0, y_end),
        w,
        Rect::new(x, y_start, w, y_end - y_start),
        color,
        dl,
    );
}

/// Emit a single border side as either a `StyledBorderSegment` (dashed/dotted)
/// or a `SolidRect` (all other visible styles).
fn emit_border_side(
    border_style: BorderStyle,
    start: Point,
    end: Point,
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
/// - `decimal` and other text markers: rendered as "N." text to the left.
///
/// `marker_text` is the pre-formatted counter string from `CounterState::evaluate_counter`.
///
/// The marker is positioned in the element's left padding area, vertically
/// centered on the first line (approximated by font ascent).
pub(crate) fn emit_list_marker_with_counter(
    lb: &LayoutBox,
    style: &ComputedStyle,
    marker_text: &str,
    font_db: &FontDatabase,
    font_cache: &mut FontCache,
    dl: &mut DisplayList,
) {
    let marker_size = style.font_size * MARKER_SIZE_FACTOR;
    let marker_x = lb.content.origin.x - style.font_size * MARKER_X_OFFSET_FACTOR;

    let families = families_as_refs(&style.font_family);
    let font_style = to_fontdb_style(style.font_style);
    let ascent = font_db
        .query(&families, style.font_weight, font_style)
        .and_then(|fid| font_db.font_metrics(fid, style.font_size))
        .map_or(style.font_size, |m| m.ascent);
    let marker_y = lb.content.origin.y + ascent * MARKER_Y_CENTER_FACTOR
        - marker_size * MARKER_Y_CENTER_FACTOR;

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
        ListStyleType::None => {}
        // Decimal and all other variants: render as "N." text marker.
        _ => {
            emit_text_marker(
                lb,
                style,
                marker_text,
                &families,
                font_style,
                ascent,
                color,
                font_db,
                font_cache,
                dl,
            );
        }
    }
}

/// Emit a text-based list marker (e.g. "1.", "2.") to the left of the content box.
///
/// `marker_text` is the pre-formatted counter string (e.g. "1", "ii", "a").
/// A trailing period is appended for display.
#[allow(clippy::too_many_arguments)]
fn emit_text_marker(
    lb: &LayoutBox,
    style: &ComputedStyle,
    marker_text: &str,
    families: &[&str],
    font_style: fontdb::Style,
    ascent: f32,
    color: CssColor,
    font_db: &FontDatabase,
    font_cache: &mut FontCache,
    dl: &mut DisplayList,
) {
    let marker_text = format!("{marker_text}.");
    let Some(font_id) = font_db.query(families, style.font_weight, font_style) else {
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
    let baseline_y = lb.content.origin.y + ascent;
    let mut text_x = lb.content.origin.x - text_width - style.font_size * DECIMAL_MARKER_GAP_FACTOR;
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

/// Emit column rules between columns of a multicol container.
///
/// CSS Multi-column L1 §4: column rules are drawn between columns that both
/// have content. For spanners, rules are not drawn in the spanner area.
///
/// Writing-mode aware:
/// - `horizontal-tb`: vertical rules between horizontal columns.
/// - `vertical-rl`/`vertical-lr`: horizontal rules between vertical columns.
pub(crate) fn emit_column_rules(
    lb: &LayoutBox,
    style: &ComputedStyle,
    info: &elidex_plugin::MulticolInfo,
    dl: &mut DisplayList,
) {
    // Skip if no rule style or zero width.
    if style.column_rule_style == BorderStyle::None || style.column_rule_width <= 0.0 {
        return;
    }

    let rule_width = style.column_rule_width;
    // currentColor is already resolved to the text color during style resolution
    // (ComputedStyle initialises column_rule_color = color).
    let color = apply_opacity(style.column_rule_color, style.opacity);

    let pb = lb.padding_box();
    let is_horizontal = info.writing_mode.is_horizontal();

    for &(actual_count, seg_start, seg_extent) in &info.segments {
        if actual_count <= 1 {
            continue;
        }

        for i in 1..actual_count {
            #[allow(clippy::cast_precision_loss)]
            let rule_inline_pos =
                i as f32 * (info.column_width + info.column_gap) - info.column_gap / 2.0;

            if is_horizontal {
                // Vertical rule line.
                let rule_x = pb.origin.x + rule_inline_pos;
                let rule_y = pb.origin.y + seg_start;
                emit_v_line(
                    style.column_rule_style,
                    rule_y,
                    rule_y + seg_extent,
                    rule_x - rule_width / 2.0,
                    rule_width,
                    color,
                    dl,
                );
            } else {
                // Horizontal rule line (vertical writing mode).
                let rule_y = pb.origin.y + rule_inline_pos;
                let rule_x = pb.origin.x + seg_start;
                emit_h_line(
                    style.column_rule_style,
                    rule_x,
                    rule_x + seg_extent,
                    rule_y - rule_width / 2.0,
                    rule_width,
                    color,
                    dl,
                );
            }
        }
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
