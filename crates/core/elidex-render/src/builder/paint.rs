//! Background, border, image, and list marker painting.

use std::sync::Arc;

use elidex_ecs::{BackgroundImages, EcsDom, Entity};
use elidex_plugin::background::{BackgroundImage, BgRepeat, BgRepeatAxis};
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
        emit_bg_layer(&layer.image, i, painting_area, opacity, bg_images, dl);
    }
}

/// Emit a single background image layer.
fn emit_bg_layer(
    image: &BackgroundImage,
    index: usize,
    painting_area: Rect,
    opacity: f32,
    bg_images: Option<&BackgroundImages>,
    dl: &mut DisplayList,
) {
    match image {
        BackgroundImage::Url(_) => {
            if let Some(bg_imgs) = bg_images {
                if let Some(Some(img_data)) = bg_imgs.layers.get(index) {
                    if img_data.width > 0 && img_data.height > 0 {
                        dl.push(DisplayItem::Image {
                            painting_area,
                            pixels: Arc::clone(&img_data.pixels),
                            image_width: img_data.width,
                            image_height: img_data.height,
                            position: (0.0, 0.0),
                            size: (painting_area.width, painting_area.height),
                            repeat: BgRepeat {
                                x: BgRepeatAxis::NoRepeat,
                                y: BgRepeatAxis::NoRepeat,
                            },
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
