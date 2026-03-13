//! Background, border, image, and list marker painting.

use std::sync::Arc;

use elidex_ecs::{EcsDom, Entity, ImageData};
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

/// Emit a background rect (solid or rounded) with opacity applied.
pub(crate) fn emit_background(
    lb: &LayoutBox,
    bg: CssColor,
    border_radius: f32,
    opacity: f32,
    dl: &mut DisplayList,
) {
    let color = apply_opacity(bg, opacity);
    if color.a == 0 {
        return; // transparent
    }
    let rect = lb.border_box();
    if border_radius > 0.0 {
        dl.push(DisplayItem::RoundedRect {
            rect,
            radius: border_radius,
            color,
        });
    } else {
        dl.push(DisplayItem::SolidRect { rect, color });
    }
}

/// Emit border rectangles as `SolidRect` items.
///
/// Each side is drawn only when `border-style != none` and `border-width > 0`.
/// Styles other than `none` are rendered as solid rectangles; `dashed`/`dotted`
/// rendering is Phase 4 scope.
///
/// Top and bottom borders span the full width. Left and right borders are
/// inset by the top/bottom border widths to avoid overlapping at corners,
/// which would cause visible darkening when `opacity < 1.0`.
pub(crate) fn emit_borders(lb: &LayoutBox, style: &ComputedStyle, dl: &mut DisplayList) {
    let bb = lb.border_box();
    let opacity = style.opacity;

    // top (full width)
    if style.border_top.style != BorderStyle::None && lb.border.top > 0.0 {
        dl.push(DisplayItem::SolidRect {
            rect: elidex_plugin::Rect::new(bb.x, bb.y, bb.width, lb.border.top),
            color: apply_opacity(style.border_top.color, opacity),
        });
    }
    // bottom (full width)
    if style.border_bottom.style != BorderStyle::None && lb.border.bottom > 0.0 {
        dl.push(DisplayItem::SolidRect {
            rect: elidex_plugin::Rect::new(
                bb.x,
                bb.y + bb.height - lb.border.bottom,
                bb.width,
                lb.border.bottom,
            ),
            color: apply_opacity(style.border_bottom.color, opacity),
        });
    }
    // right (inset by top/bottom to avoid corner overlap)
    let v_inset_top = lb.border.top;
    let v_inset_bottom = lb.border.bottom;
    let v_height = (bb.height - v_inset_top - v_inset_bottom).max(0.0);
    if style.border_right.style != BorderStyle::None && lb.border.right > 0.0 && v_height > 0.0 {
        dl.push(DisplayItem::SolidRect {
            rect: elidex_plugin::Rect::new(
                bb.x + bb.width - lb.border.right,
                bb.y + v_inset_top,
                lb.border.right,
                v_height,
            ),
            color: apply_opacity(style.border_right.color, opacity),
        });
    }
    // left (inset by top/bottom to avoid corner overlap)
    if style.border_left.style != BorderStyle::None && lb.border.left > 0.0 && v_height > 0.0 {
        dl.push(DisplayItem::SolidRect {
            rect: elidex_plugin::Rect::new(bb.x, bb.y + v_inset_top, lb.border.left, v_height),
            color: apply_opacity(style.border_left.color, opacity),
        });
    }
}

/// Emit a `DisplayItem::Image` for a replaced element.
///
/// The image is drawn within the content rect of the layout box.
pub(crate) fn emit_image(
    lb: &LayoutBox,
    image_data: &ImageData,
    opacity: f32,
    dl: &mut DisplayList,
) {
    if image_data.width == 0 || image_data.height == 0 {
        return;
    }
    dl.push(DisplayItem::Image {
        rect: lb.content,
        pixels: Arc::clone(&image_data.pixels),
        image_width: image_data.width,
        image_height: image_data.height,
        opacity,
    });
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
            dl.push(DisplayItem::RoundedRect {
                rect: marker_rect,
                radius: marker_size / 2.0,
                color,
            });
        }
        ListStyleType::Circle => {
            dl.push(DisplayItem::StrokedRoundedRect {
                rect: marker_rect,
                radius: marker_size / 2.0,
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
