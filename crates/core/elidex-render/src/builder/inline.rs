//! Inline run collection and emission.

use elidex_ecs::{EcsDom, Entity, PseudoElementMarker, TextContent};
use elidex_plugin::{
    ComputedStyle, CssColor, Direction, Display, FontStyle as PluginFontStyle, LayoutBox,
    TextAlign, TextDecorationLine, TextDecorationStyle, TextTransform, Visibility, WritingMode,
};
use elidex_text::FontDatabase;

use crate::display_list::{DisplayItem, DisplayList, GlyphEntry};
use crate::font_cache::FontCache;

use super::{
    apply_opacity, apply_text_transform, bidi_visual_order, collapse_segments,
    compute_text_align_offset, families_as_refs, find_nearest_layout_box, place_glyphs,
    place_glyphs_vertical, resolve_text_align, DECORATION_THICKNESS_DIVISOR,
    DEFAULT_DESCENT_FACTOR, LINE_THROUGH_POSITION_FACTOR, OVERLINE_POSITION_FACTOR,
    UNDERLINE_POSITION_FACTOR,
};
use elidex_text::{shape_text, shape_text_vertical, to_fontdb_style};

/// A segment of text with its own style properties.
pub(crate) struct StyledTextSegment {
    pub(crate) text: String,
    pub(crate) color: CssColor,
    pub(crate) font_family: Vec<String>,
    pub(crate) font_size: f32,
    pub(crate) font_weight: u16,
    pub(crate) font_style: PluginFontStyle,
    pub(crate) text_transform: TextTransform,
    pub(crate) text_decoration_line: TextDecorationLine,
    pub(crate) text_decoration_style: TextDecorationStyle,
    pub(crate) text_decoration_color: Option<CssColor>,
    pub(crate) letter_spacing: f32,
    pub(crate) word_spacing: f32,
    pub(crate) opacity: f32,
}

impl StyledTextSegment {
    /// Create a segment from text content and a computed style.
    fn from_style(text: String, style: &ComputedStyle) -> Self {
        Self {
            text,
            color: style.color,
            font_family: style.font_family.clone(),
            font_size: style.font_size,
            font_weight: style.font_weight,
            font_style: style.font_style,
            text_transform: style.text_transform,
            text_decoration_line: style.text_decoration_line,
            text_decoration_style: style.text_decoration_style,
            text_decoration_color: style.text_decoration_color,
            letter_spacing: style.letter_spacing,
            word_spacing: style.word_spacing,
            opacity: style.opacity,
        }
    }
}

/// Grouped parameters for [`emit_styled_segments`], reducing argument count.
pub(crate) struct InlineRunContext<'a> {
    pub(crate) segments: &'a [StyledTextSegment],
    pub(crate) collapsed: &'a [(String, usize)],
    pub(crate) lb: &'a LayoutBox,
    pub(crate) parent_style: &'a ComputedStyle,
}

/// Maximum recursion depth for inline text collection.
const MAX_INLINE_DEPTH: u32 = 100;

/// Collect styled text segments from an inline run and render them.
///
/// An inline run is a sequence of non-block children (text nodes and
/// inline elements). Each text segment preserves its element's style
/// (color, font, etc.), allowing `<span style="color:red">` to render
/// in the correct color.
pub(crate) fn emit_inline_run(
    dom: &EcsDom,
    parent: Entity,
    run: &[Entity],
    font_db: &FontDatabase,
    font_cache: &mut FontCache,
    dl: &mut DisplayList,
) {
    let parent_style = match dom.world().get::<&ComputedStyle>(parent) {
        Ok(s) => (*s).clone(),
        Err(_) => return,
    };
    let Some(lb) = find_nearest_layout_box(dom, parent) else {
        return;
    };

    let segments = collect_styled_inline_text(dom, run, &parent_style, 0);
    if segments.is_empty() {
        return;
    }

    // Check if all segments are whitespace-only after cross-segment collapsing.
    let collapsed = collapse_segments(&segments, parent_style.white_space);
    if collapsed.is_empty() {
        return;
    }

    let ctx = InlineRunContext {
        segments: &segments,
        collapsed: &collapsed,
        lb: &lb,
        parent_style: &parent_style,
    };
    emit_styled_segments(&ctx, font_db, font_cache, dl);
}

/// Recursively collect styled text segments from inline entities.
///
/// Text nodes produce segments inheriting their closest element ancestor's style.
/// Inline elements (with `ComputedStyle` but no `LayoutBox`) use their own style
/// for their children's text. `display: none` elements are skipped.
fn collect_styled_inline_text(
    dom: &EcsDom,
    entities: &[Entity],
    parent_style: &ComputedStyle,
    depth: u32,
) -> Vec<StyledTextSegment> {
    if depth >= MAX_INLINE_DEPTH {
        return Vec::new();
    }
    let mut segments = Vec::new();
    for &entity in entities {
        // Check for display: none on elements.
        if let Ok(style) = dom.world().get::<&ComputedStyle>(entity) {
            if style.display == Display::None {
                continue;
            }
            // visibility: hidden — skip text but recurse children
            // (children can override visibility).
            let visible = style.visibility == Visibility::Visible;

            // Pseudo-element: emit text with own style (skip child recursion).
            if dom.world().get::<&PseudoElementMarker>(entity).is_ok() {
                if visible {
                    if let Ok(tc) = dom.world().get::<&TextContent>(entity) {
                        if !tc.0.is_empty() {
                            segments.push(StyledTextSegment::from_style(tc.0.clone(), &style));
                        }
                    }
                }
                continue;
            }
            // Inline element: use this element's style for its children.
            let children: Vec<Entity> = dom.composed_children(entity);
            segments.extend(collect_styled_inline_text(
                dom,
                &children,
                &style,
                depth + 1,
            ));
            continue;
        }

        // Text node: produce a segment with the parent's style.
        // Inherits parent's visibility.
        if parent_style.visibility == Visibility::Visible {
            if let Ok(tc) = dom.world().get::<&TextContent>(entity) {
                if !tc.0.is_empty() {
                    segments.push(StyledTextSegment::from_style(tc.0.clone(), parent_style));
                }
            }
        }
    }
    segments
}

/// Emit styled text segments as display items.
///
/// Each segment is independently shaped and rendered. For horizontal writing
/// modes, segments are placed left-to-right; for vertical modes, top-to-bottom.
/// Text-align is applied to the total run width (horizontal) or height (vertical).
#[allow(clippy::too_many_lines)]
// Vertical path already extracted; horizontal path is a single linear pass.
fn emit_styled_segments(
    ctx: &InlineRunContext<'_>,
    font_db: &FontDatabase,
    font_cache: &mut FontCache,
    dl: &mut DisplayList,
) {
    let InlineRunContext {
        segments,
        collapsed,
        lb,
        parent_style,
    } = *ctx;

    let is_vertical = matches!(
        parent_style.writing_mode,
        WritingMode::VerticalRl | WritingMode::VerticalLr
    );

    if is_vertical {
        emit_styled_segments_vertical(ctx, font_db, font_cache, dl);
        return;
    }

    let align_offset = compute_text_align_offset(
        parent_style.text_align,
        parent_style.direction,
        lb.content.width,
        collapsed,
        segments,
        font_db,
    );

    // Reorder segments for visual display (bidi algorithm).
    let visual_order = bidi_visual_order(collapsed, parent_style.direction);

    // Emit display items (single shaping pass per segment).
    let mut cursor_x = lb.content.x + align_offset;

    for &vi in &visual_order {
        let Some((ref text, idx)) = collapsed.get(vi) else {
            continue;
        };
        let Some(seg) = segments.get(*idx) else {
            continue;
        };
        let transformed = apply_text_transform(text, seg.text_transform);
        let families = families_as_refs(&seg.font_family);
        let style = to_fontdb_style(seg.font_style);
        let Some(font_id) = font_db.query(&families, seg.font_weight, style) else {
            continue;
        };
        let Some(shaped) = shape_text(font_db, font_id, seg.font_size, &transformed) else {
            continue;
        };
        let Some((font_blob, font_index)) = font_cache.get(font_db, font_id) else {
            continue;
        };

        let metrics = font_db.font_metrics(font_id, seg.font_size);
        let ascent = metrics.map_or(seg.font_size, |m| m.ascent);
        let descent = metrics.map_or(-seg.font_size * DEFAULT_DESCENT_FACTOR, |m| m.descent);
        let baseline_y = lb.content.y + ascent;

        let seg_start_x = cursor_x;
        let glyphs = place_glyphs(
            &shaped.glyphs,
            &mut cursor_x,
            baseline_y,
            seg.letter_spacing,
            seg.word_spacing,
            &transformed,
        );
        let seg_width = cursor_x - seg_start_x;
        let text_color = apply_opacity(seg.color, seg.opacity);

        dl.push(DisplayItem::Text {
            glyphs,
            font_blob,
            font_index,
            font_size: seg.font_size,
            color: text_color,
        });

        // Text decoration.
        let decoration_thickness = (seg.font_size / DECORATION_THICKNESS_DIVISOR).max(1.0);
        let decoration_color =
            apply_opacity(seg.text_decoration_color.unwrap_or(seg.color), seg.opacity);
        if seg.text_decoration_line.underline {
            let y = baseline_y - descent * UNDERLINE_POSITION_FACTOR;
            emit_decoration_line(
                dl,
                seg_start_x,
                y,
                seg_width,
                decoration_thickness,
                decoration_color,
                seg.text_decoration_style,
            );
        }
        if seg.text_decoration_line.overline {
            let y = baseline_y - ascent * OVERLINE_POSITION_FACTOR;
            emit_decoration_line(
                dl,
                seg_start_x,
                y,
                seg_width,
                decoration_thickness,
                decoration_color,
                seg.text_decoration_style,
            );
        }
        if seg.text_decoration_line.line_through {
            let y = baseline_y - ascent * LINE_THROUGH_POSITION_FACTOR;
            emit_decoration_line(
                dl,
                seg_start_x,
                y,
                seg_width,
                decoration_thickness,
                decoration_color,
                seg.text_decoration_style,
            );
        }
    }
}

/// Emit styled text segments vertically (top-to-bottom).
///
/// Vertical writing mode: glyphs advance downward, each segment is shaped
/// with `shape_text_vertical` and placed using `y_advance`.
/// `BiDi` visual reordering is applied, and text-align offsets the cursor along
/// the block axis (vertical).
fn emit_styled_segments_vertical(
    ctx: &InlineRunContext<'_>,
    font_db: &FontDatabase,
    font_cache: &mut FontCache,
    dl: &mut DisplayList,
) {
    let InlineRunContext {
        segments,
        collapsed,
        lb,
        parent_style,
    } = *ctx;

    // A2: Compute vertical text-align offset (container height vs total advance).
    let align_offset = compute_vertical_text_align_offset(
        parent_style.text_align,
        parent_style.direction,
        lb.content.height,
        collapsed,
        segments,
        font_db,
    );

    // A1: Apply BiDi visual reordering (same as horizontal path).
    // TODO(Phase 4): BiDi reorder on vertical text segments reorders
    // top-to-bottom runs, but CSS Writing Modes Level 3 §4.1 says
    // inline direction in vertical modes is TTB; BiDi should reorder
    // within that axis. Current behaviour is likely correct for LTR
    // but needs verification for RTL vertical text.
    let visual_order = bidi_visual_order(collapsed, parent_style.direction);

    // Vertical: cursor_y advances downward, center_x is the column center.
    let center_x = lb.content.x + lb.content.width / 2.0;
    let mut cursor_y = lb.content.y + align_offset;

    for &vi in &visual_order {
        let Some((ref text, idx)) = collapsed.get(vi) else {
            continue;
        };
        let Some(seg) = segments.get(*idx) else {
            continue;
        };
        let transformed = apply_text_transform(text, seg.text_transform);
        let families = families_as_refs(&seg.font_family);
        let style = to_fontdb_style(seg.font_style);
        let Some(font_id) = font_db.query(&families, seg.font_weight, style) else {
            continue;
        };
        let Some(shaped) = shape_text_vertical(font_db, font_id, seg.font_size, &transformed)
        else {
            // Fallback to horizontal shaping if vertical shaping fails.
            let Some(shaped) = shape_text(font_db, font_id, seg.font_size, &transformed) else {
                continue;
            };
            let Some((font_blob, font_index)) = font_cache.get(font_db, font_id) else {
                continue;
            };
            let text_color = apply_opacity(seg.color, seg.opacity);
            // Place horizontally-shaped glyphs vertically (one per line).
            // TODO(Phase 4): glyph.y_offset was derived from horizontal
            // shaping, so its sign may be incorrect for vertical layout
            // (e.g. diacritics could be offset in the wrong direction).
            for glyph in &shaped.glyphs {
                let x = center_x + glyph.x_offset - glyph.x_advance / 2.0;
                let y = cursor_y + glyph.y_offset;
                dl.push(DisplayItem::Text {
                    glyphs: vec![GlyphEntry {
                        glyph_id: u32::from(glyph.glyph_id),
                        x,
                        y,
                    }],
                    font_blob: font_blob.clone(),
                    font_index,
                    font_size: seg.font_size,
                    color: text_color,
                });
                // A4: Use glyph x_advance (proportional to glyph width) instead of
                // fixed font_size for proper proportional spacing in vertical fallback.
                cursor_y += glyph.x_advance;
            }
            continue;
        };
        let Some((font_blob, font_index)) = font_cache.get(font_db, font_id) else {
            continue;
        };
        let text_color = apply_opacity(seg.color, seg.opacity);
        let glyphs = place_glyphs_vertical(&shaped.glyphs, center_x, &mut cursor_y);

        dl.push(DisplayItem::Text {
            glyphs,
            font_blob,
            font_index,
            font_size: seg.font_size,
            color: text_color,
        });

        // TODO(Phase 4): Render text-decoration (underline/line-through) for
        // vertical writing modes. Vertical underline runs along the inline
        // (block-start) side of the glyph column, not below the baseline.
    }
}

/// Compute the vertical text-align offset within a content box.
///
/// Analogous to [`compute_text_align_offset`] but for vertical writing modes:
/// measures total vertical advance of all segments and offsets within
/// `container_height`. `start`/`end` are resolved via `direction`.
fn compute_vertical_text_align_offset(
    align: TextAlign,
    direction: Direction,
    container_height: f32,
    collapsed: &[(String, usize)],
    segments: &[StyledTextSegment],
    font_db: &FontDatabase,
) -> f32 {
    // Resolve start/end using the same direction logic as horizontal.
    // In vertical modes, Left maps to top-aligned, Right to bottom-aligned.
    let resolved = resolve_text_align(align, direction);
    match resolved {
        TextAlign::Left | TextAlign::Start => 0.0,
        _ => {
            let total_height: f32 = collapsed
                .iter()
                .filter_map(|(text, idx)| {
                    segments
                        .get(*idx)
                        .map(|seg| measure_segment_height(text, seg, font_db))
                })
                .sum();
            let free = (container_height - total_height).max(0.0);
            match resolved {
                TextAlign::Center => free / 2.0,
                _ => free,
            }
        }
    }
}

/// Measure a segment's vertical advance after text-transform.
///
/// Tries vertical shaping first; falls back to horizontal shaping
/// (using `x_advance` sum as the vertical extent).
#[must_use]
fn measure_segment_height(text: &str, seg: &StyledTextSegment, font_db: &FontDatabase) -> f32 {
    let Some((transformed, font_id)) = super::query_segment_font(text, seg, font_db) else {
        return 0.0;
    };
    // Prefer vertical shaping (total_advance = sum of y_advance).
    if let Some(shaped) = shape_text_vertical(font_db, font_id, seg.font_size, &transformed) {
        return shaped.total_advance;
    }
    // Fallback: horizontal shaping, use x_advance sum as vertical extent.
    let Some(shaped) = shape_text(font_db, font_id, seg.font_size, &transformed) else {
        return 0.0;
    };
    shaped.glyphs.iter().map(|g| g.x_advance).sum()
}

/// Emit a text decoration line using the given style.
///
/// - `Solid`: single `SolidRect`
/// - `Double`: two thin `SolidRect`s separated by `thickness`
/// - `Dotted`: repeating square dots
/// - `Dashed`: repeating dashes (3:1 ratio)
/// - `Wavy`: falls back to solid (Vello path drawing needed for true wave)
fn emit_decoration_line(
    dl: &mut DisplayList,
    x: f32,
    y: f32,
    width: f32,
    thickness: f32,
    color: CssColor,
    style: TextDecorationStyle,
) {
    // Guard: skip entirely if width or thickness is non-finite or non-positive.
    if !width.is_finite() || !thickness.is_finite() || width <= 0.0 || thickness <= 0.0 {
        return;
    }
    match style {
        TextDecorationStyle::Solid | TextDecorationStyle::Wavy => {
            dl.push(DisplayItem::SolidRect {
                rect: elidex_plugin::Rect {
                    x,
                    y,
                    width,
                    height: thickness,
                },
                color,
            });
        }
        TextDecorationStyle::Double => {
            let thin = (thickness * 0.5).max(1.0);
            dl.push(DisplayItem::SolidRect {
                rect: elidex_plugin::Rect {
                    x,
                    y,
                    width,
                    height: thin,
                },
                color,
            });
            dl.push(DisplayItem::SolidRect {
                rect: elidex_plugin::Rect {
                    x,
                    y: y + thickness,
                    width,
                    height: thin,
                },
                color,
            });
        }
        TextDecorationStyle::Dotted => {
            emit_repeating_decoration(dl, x, y, width, thickness, thickness, color);
        }
        TextDecorationStyle::Dashed => {
            emit_repeating_decoration(dl, x, y, width, thickness * 3.0, thickness, color);
        }
    }
}

/// Emit a repeating decoration pattern (dots or dashes).
///
/// Each mark has `mark_width` inline extent and `thickness` block extent,
/// separated by gaps equal to `thickness`.
fn emit_repeating_decoration(
    dl: &mut DisplayList,
    x: f32,
    y: f32,
    width: f32,
    mark_width: f32,
    thickness: f32,
    color: CssColor,
) {
    let step = mark_width + thickness;
    if step <= 0.0 || !step.is_finite() {
        return;
    }
    let mut cx = x;
    let end = x + width;
    while cx < end {
        let w = mark_width.min(end - cx);
        dl.push(DisplayItem::SolidRect {
            rect: elidex_plugin::Rect {
                x: cx,
                y,
                width: w,
                height: thickness,
            },
            color,
        });
        cx += step;
    }
}
