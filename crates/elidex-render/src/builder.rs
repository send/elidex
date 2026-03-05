//! Display list builder: converts a laid-out DOM into paint commands.
//!
//! Walks the DOM tree in pre-order (painter's order) and emits
//! [`DisplayItem`]s for background rectangles and text content.
//!
//! Text processing follows CSS `white-space: normal` rules: newlines
//! and tabs are replaced with spaces, and runs of spaces are collapsed
//! to a single space. Whitespace-only text is discarded.

use std::borrow::Cow;
use std::sync::Arc;

use elidex_ecs::{EcsDom, Entity, ImageData, TextContent};
use elidex_plugin::{
    BorderStyle, ComputedStyle, CssColor, Display, LayoutBox, ListStyleType, Overflow, Rect,
    TextAlign, TextDecorationLine, TextTransform, WhiteSpace,
};
use elidex_text::{shape_text, FontDatabase};

use crate::display_list::{DisplayItem, DisplayList, GlyphEntry};
use crate::font_cache::FontCache;

// ---------------------------------------------------------------------------
// Named constants for list marker layout (R4)
// ---------------------------------------------------------------------------

/// List marker size as a fraction of `font_size`.
const MARKER_SIZE_FACTOR: f32 = 0.35;

/// Horizontal offset of the list marker from the content box left edge,
/// as a fraction of `font_size`.
const MARKER_X_OFFSET_FACTOR: f32 = 0.75;

/// Vertical center of the marker relative to the font ascent.
const MARKER_Y_CENTER_FACTOR: f32 = 0.5;

/// Gap between a decimal marker's trailing edge and the content box,
/// as a fraction of `font_size`.
const DECIMAL_MARKER_GAP_FACTOR: f32 = 0.3;

// ---------------------------------------------------------------------------
// Named constants for text metrics (R5)
// ---------------------------------------------------------------------------

/// Default descent as a fraction of `font_size` when font metrics are
/// unavailable (negative direction).
const DEFAULT_DESCENT_FACTOR: f32 = 0.25;

/// Underline position as a fraction of the descent below the baseline.
const UNDERLINE_POSITION_FACTOR: f32 = 0.5;

/// Line-through position as a fraction of the ascent above the baseline.
const LINE_THROUGH_POSITION_FACTOR: f32 = 0.4;

/// Minimum text decoration thickness divisor: `font_size / DECORATION_THICKNESS_DIVISOR`.
const DECORATION_THICKNESS_DIVISOR: f32 = 16.0;

/// Build a display list from a laid-out DOM tree.
///
/// Each element with a [`LayoutBox`] component is visited in pre-order.
/// Background colors produce [`DisplayItem::SolidRect`] entries; text
/// nodes produce [`DisplayItem::Text`] entries via re-shaping.
///
/// Children of each element are processed in "inline runs": consecutive
/// non-block children (text nodes and inline elements) have their text
/// collected, whitespace-collapsed, and rendered as a single text item.
/// This avoids position overlap when multiple text nodes share the same
/// block ancestor.
///
/// # Prerequisites
///
/// `elidex_layout::layout_tree()` must have been called first so that
/// every visible element has a [`LayoutBox`] component.
#[must_use]
pub fn build_display_list(dom: &EcsDom, font_db: &FontDatabase) -> DisplayList {
    let mut dl = DisplayList::default();
    let mut font_cache = FontCache::new();

    let roots = find_roots(dom);
    for root in roots {
        walk(dom, root, font_db, &mut font_cache, &mut dl);
    }

    dl
}

/// Find root entities for rendering: parentless entities with layout or children.
fn find_roots(dom: &EcsDom) -> Vec<Entity> {
    dom.root_entities()
        .into_iter()
        .filter(|&e| dom.world().get::<&LayoutBox>(e).is_ok() || dom.get_first_child(e).is_some())
        .collect()
}

/// Pre-order walk: emit paint commands for this entity, then recurse.
///
/// Children are grouped into "inline runs" (consecutive non-block children)
/// and "block children" (those with a `LayoutBox`). Inline runs have their
/// text collected and rendered as a single item; block children are
/// recursed into normally.
fn walk(
    dom: &EcsDom,
    entity: Entity,
    font_db: &FontDatabase,
    font_cache: &mut FontCache,
    dl: &mut DisplayList,
) {
    // Check for display: none — skip this subtree entirely.
    // This check is independent of LayoutBox: an element without a LayoutBox
    // but with display:none should still be skipped.
    if let Ok(style) = dom.world().get::<&ComputedStyle>(entity) {
        if style.display == Display::None {
            return;
        }
    }

    // Emit background + borders for elements with a LayoutBox.
    let mut has_clip = false;
    if let Ok(lb) = dom.world().get::<&LayoutBox>(entity) {
        if let Ok(style) = dom.world().get::<&ComputedStyle>(entity) {
            emit_background(
                &lb,
                style.background_color,
                style.border_radius,
                style.opacity,
                dl,
            );
            emit_borders(&lb, &style, dl);

            // Emit image for replaced elements with decoded pixel data.
            if let Ok(image_data) = dom.world().get::<&ImageData>(entity) {
                if style.opacity > 0.0 {
                    emit_image(&lb, &image_data, style.opacity, dl);
                }
            }

            // overflow: hidden → clip children to padding box (CSS Overflow §3).
            if style.overflow == Overflow::Hidden {
                let pb = lb.padding_box();
                dl.push(DisplayItem::PushClip { rect: pb });
                has_clip = true;
            }

            // List marker rendering — counter is managed per-parent, passed down.
            // The walk function handles this instead (see below).
        }
    }

    // Process children in inline runs vs block children.
    let children: Vec<Entity> = dom.children_iter(entity).collect();
    let mut inline_run = Vec::new();
    let mut list_counter = 0_usize;

    for &child in &children {
        if is_block_child(dom, child) {
            // Flush any pending inline run before the block child.
            if !inline_run.is_empty() {
                emit_inline_run(dom, entity, &inline_run, font_db, font_cache, dl);
                inline_run.clear();
            }

            // Emit list marker for list-item children.
            // Counter increments for every list-item regardless of list-style-type;
            // list-style-type: none only suppresses marker rendering.
            if let Ok(child_style) = dom.world().get::<&ComputedStyle>(child) {
                if child_style.display == Display::ListItem {
                    list_counter += 1;
                    if child_style.list_style_type != ListStyleType::None {
                        if let Ok(child_lb) = dom.world().get::<&LayoutBox>(child) {
                            emit_list_marker_with_counter(
                                &child_lb,
                                &child_style,
                                list_counter,
                                font_db,
                                font_cache,
                                dl,
                            );
                        }
                    }
                }
            }

            // Recurse into block child.
            walk(dom, child, font_db, font_cache, dl);
        } else {
            // Text node or inline element — add to current run.
            inline_run.push(child);
        }
    }

    // Flush trailing inline run.
    if !inline_run.is_empty() {
        emit_inline_run(dom, entity, &inline_run, font_db, font_cache, dl);
    }

    if has_clip {
        dl.push(DisplayItem::PopClip);
    }
}

/// Check whether a child entity is a block-level child (has a `LayoutBox`).
///
/// Block children are recursed into separately; non-block children (text
/// nodes and inline elements) are collected into inline runs.
fn is_block_child(dom: &EcsDom, entity: Entity) -> bool {
    dom.world().get::<&LayoutBox>(entity).is_ok()
}

/// A segment of text with its own style properties.
struct StyledTextSegment {
    text: String,
    color: CssColor,
    font_family: Vec<String>,
    font_size: f32,
    font_weight: u16,
    text_transform: TextTransform,
    text_decoration_line: TextDecorationLine,
    opacity: f32,
}

/// Collect styled text segments from an inline run and render them.
///
/// An inline run is a sequence of non-block children (text nodes and
/// inline elements). Each text segment preserves its element's style
/// (color, font, etc.), allowing `<span style="color:red">` to render
/// in the correct color.
fn emit_inline_run(
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

/// Maximum recursion depth for inline text collection.
const MAX_INLINE_DEPTH: u32 = 100;

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
            // Inline element: use this element's style for its children.
            let children: Vec<Entity> = dom.children_iter(entity).collect();
            segments.extend(collect_styled_inline_text(
                dom,
                &children,
                &style,
                depth + 1,
            ));
            continue;
        }

        // Text node: produce a segment with the parent's style.
        if let Ok(tc) = dom.world().get::<&TextContent>(entity) {
            if !tc.0.is_empty() {
                segments.push(StyledTextSegment {
                    text: tc.0.clone(),
                    color: parent_style.color,
                    font_family: parent_style.font_family.clone(),
                    font_size: parent_style.font_size,
                    font_weight: parent_style.font_weight,
                    text_transform: parent_style.text_transform,
                    text_decoration_line: parent_style.text_decoration_line,
                    opacity: parent_style.opacity,
                });
            }
        }
    }
    segments
}

/// Grouped parameters for [`emit_styled_segments`], reducing argument count.
struct InlineRunContext<'a> {
    segments: &'a [StyledTextSegment],
    collapsed: &'a [(String, usize)],
    lb: &'a LayoutBox,
    parent_style: &'a ComputedStyle,
}

/// Emit styled text segments as display items.
///
/// Each segment is independently shaped and rendered. Segments are placed
/// sequentially along the x-axis. Text-align is applied to the total run width.
#[allow(clippy::too_many_lines)]
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
    let align_offset = compute_text_align_offset(
        parent_style.text_align,
        lb.content.width,
        collapsed,
        segments,
        font_db,
    );

    // Emit display items (single shaping pass per segment).
    let mut cursor_x = lb.content.x + align_offset;

    for (text, idx) in collapsed {
        let seg = &segments[*idx];
        let transformed = apply_text_transform(text, seg.text_transform);
        let families = families_as_refs(&seg.font_family);
        let Some(font_id) = font_db.query(&families, seg.font_weight) else {
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
        let glyphs = place_glyphs(&shaped.glyphs, &mut cursor_x, baseline_y);
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
        if seg.text_decoration_line.underline {
            let y = baseline_y - descent * UNDERLINE_POSITION_FACTOR;
            dl.push(DisplayItem::SolidRect {
                rect: elidex_plugin::Rect {
                    x: seg_start_x,
                    y,
                    width: seg_width,
                    height: decoration_thickness,
                },
                color: text_color,
            });
        }
        if seg.text_decoration_line.line_through {
            let y = baseline_y - ascent * LINE_THROUGH_POSITION_FACTOR;
            dl.push(DisplayItem::SolidRect {
                rect: elidex_plugin::Rect {
                    x: seg_start_x,
                    y,
                    width: seg_width,
                    height: decoration_thickness,
                },
                color: text_color,
            });
        }
    }
}

/// Normalize line endings per CSS Text §4.1 Phase I.
///
/// Converts `\r\n` sequences to `\n` first, then any remaining bare `\r` to `\n`.
fn normalize_line_endings(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// Collapse whitespace across segments according to `white-space` mode.
///
/// | Mode    | collapse spaces | collapse newlines | wrap  |
/// |---------|:---:|:---:|:---:|
/// | Normal  | Yes | Yes | Yes |
/// | Pre     | No  | No  | No  |
/// | NoWrap  | Yes | Yes | No  |
/// | PreWrap | No  | No  | Yes |
/// | PreLine | Yes | No  | Yes |
fn collapse_segments(
    segments: &[StyledTextSegment],
    white_space: WhiteSpace,
) -> Vec<(String, usize)> {
    let collapse_spaces = matches!(
        white_space,
        WhiteSpace::Normal | WhiteSpace::NoWrap | WhiteSpace::PreLine
    );
    let collapse_newlines = matches!(white_space, WhiteSpace::Normal | WhiteSpace::NoWrap);

    // Pre / PreWrap: preserve text, but still normalize \r\n → \n (CSS Text §4.1).
    if !collapse_spaces && !collapse_newlines {
        return segments
            .iter()
            .enumerate()
            .filter(|(_, seg)| !seg.text.is_empty())
            .map(|(idx, seg)| {
                let text = normalize_line_endings(&seg.text);
                (text, idx)
            })
            .collect();
    }

    let mut result: Vec<(String, usize)> = Vec::new();
    let mut prev_was_space = true; // Leading whitespace is trimmed.
    for (idx, seg) in segments.iter().enumerate() {
        // CSS Text §4.1 Phase I: normalize \r\n → \n, bare \r → \n.
        let normalized = normalize_line_endings(&seg.text);
        let mut seg_text = String::new();
        for ch in normalized.chars() {
            let is_newline = ch == '\n';
            let is_space = ch == ' ' || ch == '\t';

            if is_newline {
                if collapse_newlines {
                    // Treat newlines as spaces (Normal / NoWrap).
                    if collapse_spaces && !prev_was_space {
                        seg_text.push(' ');
                        prev_was_space = true;
                    }
                } else {
                    // PreLine: preserve newlines; strip spaces/tabs immediately
                    // before the forced break (CSS Text §4).
                    let trimmed = seg_text.trim_end_matches([' ', '\t']);
                    seg_text.truncate(trimmed.len());
                    seg_text.push('\n');
                    prev_was_space = true; // Reset space state after newline.
                }
            } else if is_space {
                if collapse_spaces {
                    if !prev_was_space {
                        seg_text.push(' ');
                        prev_was_space = true;
                    }
                } else {
                    seg_text.push(ch);
                    prev_was_space = false;
                }
            } else {
                seg_text.push(ch);
                prev_was_space = false;
            }
        }
        if !seg_text.is_empty() {
            result.push((seg_text, idx));
        }
    }
    // Trim trailing/leading whitespace from the result.
    // For PreLine: only trim spaces/tabs, preserve newlines.
    if collapse_newlines {
        if let Some(last) = result.last_mut() {
            last.0 = last.0.trim_end().to_string();
        }
        if let Some(first) = result.first_mut() {
            first.0 = first.0.trim_start().to_string();
        }
    } else {
        // PreLine: trim only spaces/tabs, not newlines.
        if let Some(last) = result.last_mut() {
            last.0 = last.0.trim_end_matches([' ', '\t']).to_string();
        }
        if let Some(first) = result.first_mut() {
            first.0 = first.0.trim_start_matches([' ', '\t']).to_string();
        }
    }
    result.retain(|(text, _)| !text.is_empty());
    result
}

/// Compute the horizontal offset for `text-align` within a content box.
///
/// For `Left`, returns `0.0` immediately (no measurement needed).
/// For `Center`/`Right`, measures the total width of all collapsed segments
/// and returns the appropriate offset within `container_width`.
fn compute_text_align_offset(
    align: TextAlign,
    container_width: f32,
    collapsed: &[(String, usize)],
    segments: &[StyledTextSegment],
    font_db: &FontDatabase,
) -> f32 {
    match align {
        TextAlign::Left => 0.0,
        TextAlign::Center | TextAlign::Right => {
            let total_width: f32 = collapsed
                .iter()
                .map(|(text, idx)| measure_segment_width(text, &segments[*idx], font_db))
                .sum();
            let free = (container_width - total_width).max(0.0);
            match align {
                TextAlign::Center => free / 2.0,
                // TextAlign::Right and any future variants.
                _ => free,
            }
        }
    }
}

/// Measure a segment's text width after text-transform.
#[must_use]
fn measure_segment_width(text: &str, seg: &StyledTextSegment, font_db: &FontDatabase) -> f32 {
    let transformed = apply_text_transform(text, seg.text_transform);
    let families = families_as_refs(&seg.font_family);
    let Some(font_id) = font_db.query(&families, seg.font_weight) else {
        return 0.0;
    };
    let Some(shaped) = shape_text(font_db, font_id, seg.font_size, &transformed) else {
        return 0.0;
    };
    shaped.glyphs.iter().map(|g| g.x_advance).sum()
}

/// Place shaped glyphs into a `Vec<GlyphEntry>`, advancing `cursor_x`.
///
/// Returns the placed glyphs. `cursor_x` is updated to reflect the total advance.
#[must_use]
fn place_glyphs(
    shaped_glyphs: &[elidex_text::ShapedGlyph],
    cursor_x: &mut f32,
    baseline_y: f32,
) -> Vec<GlyphEntry> {
    let mut glyphs = Vec::with_capacity(shaped_glyphs.len());
    for glyph in shaped_glyphs {
        let x = *cursor_x + glyph.x_offset;
        let y = baseline_y - glyph.y_offset;
        glyphs.push(GlyphEntry {
            glyph_id: u32::from(glyph.glyph_id),
            x,
            y,
        });
        *cursor_x += glyph.x_advance;
    }
    glyphs
}

/// Convert a `Vec<String>` of font family names to a `Vec<&str>` for font queries.
#[must_use]
fn families_as_refs(families: &[String]) -> Vec<&str> {
    families.iter().map(String::as_str).collect()
}

/// Apply opacity to a color by multiplying its alpha channel.
#[must_use]
fn apply_opacity(color: CssColor, opacity: f32) -> CssColor {
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
fn emit_background(
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
fn emit_borders(lb: &LayoutBox, style: &ComputedStyle, dl: &mut DisplayList) {
    let bb = lb.border_box();
    let opacity = style.opacity;

    // top (full width)
    if style.border_top_style != BorderStyle::None && lb.border.top > 0.0 {
        dl.push(DisplayItem::SolidRect {
            rect: elidex_plugin::Rect {
                x: bb.x,
                y: bb.y,
                width: bb.width,
                height: lb.border.top,
            },
            color: apply_opacity(style.border_top_color, opacity),
        });
    }
    // bottom (full width)
    if style.border_bottom_style != BorderStyle::None && lb.border.bottom > 0.0 {
        dl.push(DisplayItem::SolidRect {
            rect: elidex_plugin::Rect {
                x: bb.x,
                y: bb.y + bb.height - lb.border.bottom,
                width: bb.width,
                height: lb.border.bottom,
            },
            color: apply_opacity(style.border_bottom_color, opacity),
        });
    }
    // right (inset by top/bottom to avoid corner overlap)
    let v_inset_top = lb.border.top;
    let v_inset_bottom = lb.border.bottom;
    let v_height = (bb.height - v_inset_top - v_inset_bottom).max(0.0);
    if style.border_right_style != BorderStyle::None && lb.border.right > 0.0 && v_height > 0.0 {
        dl.push(DisplayItem::SolidRect {
            rect: elidex_plugin::Rect {
                x: bb.x + bb.width - lb.border.right,
                y: bb.y + v_inset_top,
                width: lb.border.right,
                height: v_height,
            },
            color: apply_opacity(style.border_right_color, opacity),
        });
    }
    // left (inset by top/bottom to avoid corner overlap)
    if style.border_left_style != BorderStyle::None && lb.border.left > 0.0 && v_height > 0.0 {
        dl.push(DisplayItem::SolidRect {
            rect: elidex_plugin::Rect {
                x: bb.x,
                y: bb.y + v_inset_top,
                width: lb.border.left,
                height: v_height,
            },
            color: apply_opacity(style.border_left_color, opacity),
        });
    }
}

/// Emit a `DisplayItem::Image` for a replaced element.
///
/// The image is drawn within the content rect of the layout box.
fn emit_image(lb: &LayoutBox, image_data: &ImageData, opacity: f32, dl: &mut DisplayList) {
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
fn emit_list_marker_with_counter(
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
    let ascent = font_db
        .query(&families, style.font_weight)
        .and_then(|fid| font_db.font_metrics(fid, style.font_size))
        .map_or(style.font_size, |m| m.ascent);
    let marker_y =
        lb.content.y + ascent * MARKER_Y_CENTER_FACTOR - marker_size * MARKER_Y_CENTER_FACTOR;

    let color = apply_opacity(style.color, style.opacity);

    // Common marker rect for disc/circle/square (R2: hoisted before match).
    let marker_rect = Rect {
        x: marker_x,
        y: marker_y,
        width: marker_size,
        height: marker_size,
    };

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
            let Some(font_id) = font_db.query(&families, style.font_weight) else {
                return;
            };
            let Some(shaped) = shape_text(font_db, font_id, style.font_size, &marker_text) else {
                return;
            };
            let Some((font_blob, font_index)) = font_cache.get(font_db, font_id) else {
                return;
            };
            let text_width: f32 = shaped.glyphs.iter().map(|g| g.x_advance).sum();
            let baseline_y = lb.content.y + ascent;
            let mut text_x =
                lb.content.x - text_width - style.font_size * DECIMAL_MARKER_GAP_FACTOR;
            let glyphs = place_glyphs(&shaped.glyphs, &mut text_x, baseline_y);
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

/// Maximum depth for ancestor walks to prevent infinite loops on corrupted trees.
const MAX_ANCESTOR_DEPTH: u32 = 1000;

/// Walk up the ancestor chain to find the nearest entity with a `LayoutBox`.
///
/// Starts with `entity` itself, then checks its parent, grandparent, etc.
/// Returns `None` if no ancestor has a `LayoutBox` (capped at [`MAX_ANCESTOR_DEPTH`]).
#[must_use]
fn find_nearest_layout_box(dom: &EcsDom, entity: Entity) -> Option<LayoutBox> {
    let mut current = entity;
    for _ in 0..MAX_ANCESTOR_DEPTH {
        if let Ok(lb) = dom.world().get::<&LayoutBox>(current) {
            return Some((*lb).clone());
        }
        current = dom.get_parent(current)?;
    }
    None
}

/// Apply CSS `text-transform` to a string before shaping.
#[must_use]
fn apply_text_transform(text: &str, transform: TextTransform) -> Cow<'_, str> {
    match transform {
        TextTransform::None => Cow::Borrowed(text),
        TextTransform::Uppercase => Cow::Owned(text.to_uppercase()),
        TextTransform::Lowercase => Cow::Owned(text.to_lowercase()),
        TextTransform::Capitalize => Cow::Owned(capitalize_words(text)),
    }
}

/// Capitalize the first letter of each word (whitespace-delimited).
#[must_use]
fn capitalize_words(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_was_whitespace = true;
    for ch in text.chars() {
        if prev_was_whitespace && ch.is_alphabetic() {
            for upper in ch.to_uppercase() {
                result.push(upper);
            }
            prev_was_whitespace = false;
        } else {
            result.push(ch);
            prev_was_whitespace = ch.is_whitespace();
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;
    use elidex_plugin::{EdgeSizes, Rect};

    /// Font families used across tests. Covers common system fonts on
    /// Linux, macOS, and Windows so that at least one is available on CI.
    const TEST_FONT_FAMILIES: &[&str] = &[
        "Arial",
        "Helvetica",
        "Liberation Sans",
        "DejaVu Sans",
        "Noto Sans",
        "Hiragino Sans",
    ];

    /// Build a `Vec<String>` from [`TEST_FONT_FAMILIES`] for `ComputedStyle`.
    fn test_font_family_strings() -> Vec<String> {
        TEST_FONT_FAMILIES
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    }

    /// Common test setup: creates a DOM with a root, one block element with a
    /// [`ComputedStyle`] and [`LayoutBox`], and returns `(dom, element)`.
    ///
    /// `style_fn` receives a default `ComputedStyle` with `display: Block` and
    /// `test_font_family_strings()` pre-filled; callers can override fields.
    fn setup_block_element(style: ComputedStyle, layout: LayoutBox) -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let elem = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(root, elem);
        let _ = dom.world_mut().insert_one(elem, style);
        let _ = dom.world_mut().insert_one(elem, layout);
        (dom, elem)
    }

    /// Return `true` if test fonts are available on this system.
    fn fonts_available(font_db: &FontDatabase) -> bool {
        font_db.query(TEST_FONT_FAMILIES, 400).is_some()
    }

    #[test]
    fn empty_dom_empty_display_list() {
        let dom = EcsDom::new();
        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        assert!(dl.0.is_empty());
    }

    #[test]
    fn background_color_emits_solid_rect() {
        let (dom, _) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::RED,
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 10.0,
                    y: 10.0,
                    width: 100.0,
                    height: 50.0,
                },
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        assert_eq!(dl.0.len(), 1);
        let DisplayItem::SolidRect { rect, color } = &dl.0[0] else {
            panic!("expected SolidRect");
        };
        assert_eq!(*color, CssColor::RED);
        assert!((rect.width - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn transparent_background_no_item() {
        let (dom, _) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::TRANSPARENT,
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 50.0,
                },
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        assert!(dl.0.is_empty());
    }

    #[test]
    #[allow(unused_must_use)]
    fn text_node_emits_text_item() {
        let (mut dom, div) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                font_family: test_font_family_strings(),
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );
        let text = dom.create_text("Hello");
        dom.append_child(div, text);

        let font_db = FontDatabase::new();
        if !fonts_available(&font_db) {
            return;
        }

        let dl = build_display_list(&dom, &font_db);
        let text_items: Vec<_> =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::Text { .. }))
                .collect();
        assert_eq!(text_items.len(), 1);
        let DisplayItem::Text {
            glyphs, font_size, ..
        } = &text_items[0]
        else {
            unreachable!();
        };
        assert_eq!(glyphs.len(), 5); // "Hello" = 5 glyphs
        assert!((*font_size - 16.0).abs() < f32::EPSILON);
    }

    #[test]
    #[allow(unused_must_use)]
    fn nested_elements_painter_order() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let outer = dom.create_element("div", Attributes::default());
        let inner = dom.create_element("div", Attributes::default());
        dom.append_child(root, outer);
        dom.append_child(outer, inner);

        dom.world_mut().insert_one(
            outer,
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::RED,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            outer,
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 200.0,
                    height: 100.0,
                },
                ..Default::default()
            },
        );

        dom.world_mut().insert_one(
            inner,
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::BLUE,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            inner,
            LayoutBox {
                content: Rect {
                    x: 10.0,
                    y: 10.0,
                    width: 180.0,
                    height: 80.0,
                },
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);

        // Painter's order: outer first, inner second.
        assert_eq!(dl.0.len(), 2);
        match (&dl.0[0], &dl.0[1]) {
            (
                DisplayItem::SolidRect {
                    color: c1,
                    rect: r1,
                },
                DisplayItem::SolidRect {
                    color: c2,
                    rect: r2,
                },
            ) => {
                assert_eq!(*c1, CssColor::RED);
                assert_eq!(*c2, CssColor::BLUE);
                assert!((r1.width - 200.0).abs() < f32::EPSILON);
                assert!((r2.width - 180.0).abs() < f32::EPSILON);
            }
            _ => panic!("expected two SolidRects"),
        }
    }

    #[test]
    #[allow(unused_must_use)]
    fn display_none_skipped() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let visible = dom.create_element("div", Attributes::default());
        let hidden = dom.create_element("div", Attributes::default());
        dom.append_child(root, visible);
        dom.append_child(root, hidden);

        dom.world_mut().insert_one(
            visible,
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::RED,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            visible,
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 50.0,
                },
                ..Default::default()
            },
        );

        dom.world_mut().insert_one(
            hidden,
            ComputedStyle {
                display: Display::None,
                background_color: CssColor::BLUE,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            hidden,
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 50.0,
                    width: 100.0,
                    height: 50.0,
                },
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        assert_eq!(dl.0.len(), 1);
        let DisplayItem::SolidRect { color, .. } = &dl.0[0] else {
            panic!("expected SolidRect");
        };
        assert_eq!(*color, CssColor::RED);
    }

    #[test]
    fn background_uses_border_box() {
        let (dom, _) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::GREEN,
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 20.0,
                    y: 20.0,
                    width: 100.0,
                    height: 50.0,
                },
                padding: EdgeSizes {
                    top: 5.0,
                    right: 5.0,
                    bottom: 5.0,
                    left: 5.0,
                },
                border: EdgeSizes {
                    top: 2.0,
                    right: 2.0,
                    bottom: 2.0,
                    left: 2.0,
                },
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        assert_eq!(dl.0.len(), 1);
        let DisplayItem::SolidRect { rect, .. } = &dl.0[0] else {
            panic!("expected SolidRect");
        };
        // border box: x = 20 - 5 - 2 = 13, width = 100 + 10 + 4 = 114
        assert!((rect.x - 13.0).abs() < f32::EPSILON);
        assert!((rect.y - 13.0).abs() < f32::EPSILON);
        assert!((rect.width - 114.0).abs() < f32::EPSILON);
        assert!((rect.height - 64.0).abs() < f32::EPSILON);
    }

    #[test]
    #[allow(unused_must_use)]
    fn whitespace_only_text_node_skipped() {
        let (mut dom, div) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );
        let ws = dom.create_text("   \n   ");
        dom.append_child(div, ws);

        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        // Whitespace-only text should produce no display items.
        assert!(dl.0.is_empty());
    }

    #[test]
    #[allow(unused_must_use)]
    fn inline_elements_text_collected() {
        // <p>Hello <strong>world</strong>!</p>
        // Should produce a single "Hello world!" text item.
        let (mut dom, p) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                font_family: test_font_family_strings(),
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );
        let t1 = dom.create_text("Hello ");
        let strong = dom.create_element("strong", Attributes::default());
        let t2 = dom.create_text("world");
        let t3 = dom.create_text("!");
        dom.append_child(p, t1);
        dom.append_child(p, strong);
        dom.append_child(strong, t2);
        dom.append_child(p, t3);
        // strong is inline — no LayoutBox, but has ComputedStyle.
        dom.world_mut().insert_one(
            strong,
            ComputedStyle {
                display: Display::Inline,
                font_family: test_font_family_strings(),
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        if !fonts_available(&font_db) {
            return;
        }

        let dl = build_display_list(&dom, &font_db);
        let text_items: Vec<_> =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::Text { .. }))
                .collect();
        // Styled inline runs: one text item per styled segment.
        // "Hello " (parent style), "world" (strong style), "!" (parent style).
        assert_eq!(text_items.len(), 3);
        let total_glyphs: usize = text_items
            .iter()
            .map(|item| {
                let DisplayItem::Text { glyphs, .. } = item else {
                    unreachable!();
                };
                glyphs.len()
            })
            .sum();
        // "Hello world!" = 12 glyphs total across 3 segments.
        assert_eq!(total_glyphs, 12);
    }

    // L9: text-align center/right in builder
    #[test]
    #[allow(unused_must_use)]
    fn text_align_center_offsets_text() {
        let (mut dom, p) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                text_align: TextAlign::Center,
                font_family: test_font_family_strings(),
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 400.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );
        let txt = dom.create_text("Hi");
        dom.append_child(p, txt);

        let font_db = FontDatabase::new();
        if !fonts_available(&font_db) {
            return;
        }
        let dl = build_display_list(&dom, &font_db);
        let text_items: Vec<_> =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::Text { .. }))
                .collect();
        assert!(!text_items.is_empty(), "should have text items");
        // Center-aligned text: first glyph should be shifted right from 0.
        // Exact offset = (400 - text_width) / 2, which is > 0 for any short text.
        if let DisplayItem::Text { glyphs, .. } = text_items[0] {
            assert!(
                glyphs[0].x > 0.0 && glyphs[0].x < 400.0,
                "center-aligned text should be between 0 and container width, got x={}",
                glyphs[0].x
            );
        }
    }

    #[test]
    #[allow(unused_must_use)]
    fn text_align_right_offsets_text() {
        let (mut dom, p) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                text_align: TextAlign::Right,
                font_family: test_font_family_strings(),
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 400.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );
        let txt = dom.create_text("Hi");
        dom.append_child(p, txt);

        let font_db = FontDatabase::new();
        if !fonts_available(&font_db) {
            return;
        }
        let dl = build_display_list(&dom, &font_db);
        let text_items: Vec<_> =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::Text { .. }))
                .collect();
        assert!(!text_items.is_empty(), "should have text items");
        // Right-aligned: offset = 400 - text_width, so glyph x > center offset.
        if let DisplayItem::Text { glyphs, .. } = text_items[0] {
            assert!(
                glyphs[0].x > 0.0 && glyphs[0].x < 400.0,
                "right-aligned text should be between 0 and container width, got x={}",
                glyphs[0].x
            );
            // Right offset should be > center offset for the same text.
            // "Hi" in 400px: right offset ≈ 380+, center offset ≈ 190+.
            assert!(
                glyphs[0].x > 200.0,
                "right-aligned text should be in right half, got x={}",
                glyphs[0].x
            );
        }
    }

    // --- M3-2: border rendering tests ---

    #[test]
    fn emit_borders_four_sides() {
        let (dom, _) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::TRANSPARENT,
                border_top_style: BorderStyle::Solid,
                border_right_style: BorderStyle::Solid,
                border_bottom_style: BorderStyle::Solid,
                border_left_style: BorderStyle::Solid,
                border_top_color: CssColor::RED,
                border_right_color: CssColor::RED,
                border_bottom_color: CssColor::RED,
                border_left_color: CssColor::RED,
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 12.0,
                    y: 12.0,
                    width: 100.0,
                    height: 50.0,
                },
                border: EdgeSizes {
                    top: 2.0,
                    right: 2.0,
                    bottom: 2.0,
                    left: 2.0,
                },
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        // 4 border SolidRects (no background since transparent).
        let rect_count =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::SolidRect { .. }))
                .count();
        assert_eq!(rect_count, 4);
    }

    #[test]
    fn emit_borders_style_none_skipped() {
        let (dom, _) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::TRANSPARENT,
                // Only top border is solid; others are none (default).
                border_top_style: BorderStyle::Solid,
                border_top_color: CssColor::RED,
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 2.0,
                    y: 2.0,
                    width: 100.0,
                    height: 50.0,
                },
                border: EdgeSizes {
                    top: 2.0,
                    right: 2.0,
                    bottom: 2.0,
                    left: 2.0,
                },
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        // Only 1 border (top), others skipped because style=none.
        assert_eq!(dl.0.len(), 1);
    }

    #[test]
    fn emit_borders_zero_width_skipped() {
        let (dom, _) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::TRANSPARENT,
                border_top_style: BorderStyle::Solid,
                border_top_color: CssColor::RED,
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 50.0,
                },
                border: EdgeSizes {
                    top: 0.0,
                    ..Default::default()
                }, // zero width, should be skipped
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        assert!(dl.0.is_empty());
    }

    #[test]
    fn background_with_border_radius_emits_rounded_rect() {
        let (dom, _) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::RED,
                border_radius: 10.0,
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 50.0,
                },
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        assert_eq!(dl.0.len(), 1);
        assert!(
            matches!(&dl.0[0], DisplayItem::RoundedRect { radius, .. } if (*radius - 10.0).abs() < f32::EPSILON)
        );
    }

    #[test]
    fn background_without_border_radius_emits_solid_rect() {
        let (dom, _) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::RED,
                border_radius: 0.0,
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 50.0,
                },
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        assert_eq!(dl.0.len(), 1);
        assert!(matches!(&dl.0[0], DisplayItem::SolidRect { .. }));
    }

    #[test]
    fn opacity_half_halves_alpha() {
        let color = CssColor::new(255, 0, 0, 200);
        let result = apply_opacity(color, 0.5);
        assert_eq!(result.r, 255);
        assert_eq!(result.g, 0);
        assert_eq!(result.b, 0);
        assert_eq!(result.a, 100);
    }

    #[test]
    fn opacity_zero_makes_transparent() {
        let color = CssColor::RED;
        let result = apply_opacity(color, 0.0);
        assert_eq!(result.a, 0);
    }

    #[test]
    fn opacity_one_unchanged() {
        let color = CssColor::RED;
        let result = apply_opacity(color, 1.0);
        assert_eq!(result, CssColor::RED);
    }

    /// Known Phase 4 limitation: when both `border-radius` and `border` are
    /// set, the background is a `RoundedRect` but borders are axis-aligned
    /// `SolidRect` items. Borders do not follow rounded corners.
    #[test]
    fn border_radius_with_border_known_limitation() {
        let (dom, _) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::RED,
                border_radius: 10.0,
                border_top_style: BorderStyle::Solid,
                border_right_style: BorderStyle::Solid,
                border_bottom_style: BorderStyle::Solid,
                border_left_style: BorderStyle::Solid,
                border_top_color: CssColor::BLACK,
                border_right_color: CssColor::BLACK,
                border_bottom_color: CssColor::BLACK,
                border_left_color: CssColor::BLACK,
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 2.0,
                    y: 2.0,
                    width: 100.0,
                    height: 50.0,
                },
                border: EdgeSizes {
                    top: 2.0,
                    right: 2.0,
                    bottom: 2.0,
                    left: 2.0,
                },
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        // 1 RoundedRect (background) + 4 SolidRect (borders).
        let rounded =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::RoundedRect { .. }))
                .count();
        let rects =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::SolidRect { .. }))
                .count();
        assert_eq!(rounded, 1);
        assert_eq!(rects, 4);
    }

    #[test]
    fn border_corners_no_overlap() {
        // Verify that left/right borders are inset by top/bottom widths.
        let (dom, _) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::TRANSPARENT,
                border_top_style: BorderStyle::Solid,
                border_right_style: BorderStyle::Solid,
                border_bottom_style: BorderStyle::Solid,
                border_left_style: BorderStyle::Solid,
                border_top_color: CssColor::RED,
                border_right_color: CssColor::RED,
                border_bottom_color: CssColor::RED,
                border_left_color: CssColor::RED,
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 5.0,
                    y: 5.0,
                    width: 100.0,
                    height: 50.0,
                },
                border: EdgeSizes {
                    top: 3.0,
                    right: 2.0,
                    bottom: 3.0,
                    left: 2.0,
                },
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        let rects: Vec<_> =
            dl.0.iter()
                .filter_map(|i| match i {
                    DisplayItem::SolidRect { rect, .. } => Some(rect),
                    _ => None,
                })
                .collect();
        assert_eq!(rects.len(), 4);
        // border-box: x=3, y=2, w=104, h=56 (content 100x50 + border 2+2 / 3+3)
        // top: full width, y=2, h=3
        let top = rects[0];
        assert!((top.y - 2.0).abs() < f32::EPSILON);
        assert!((top.height - 3.0).abs() < f32::EPSILON);
        assert!((top.width - 104.0).abs() < f32::EPSILON);
        // bottom: full width, y=55, h=3
        let bottom = rects[1];
        assert!((bottom.y - 55.0).abs() < f32::EPSILON);
        assert!((bottom.height - 3.0).abs() < f32::EPSILON);
        // right: inset by top(3)+bottom(3), height=50
        let right = rects[2];
        assert!((right.y - 5.0).abs() < f32::EPSILON); // 2 + 3
        assert!((right.height - 50.0).abs() < f32::EPSILON); // 56 - 3 - 3
                                                             // left: same inset
        let left = rects[3];
        assert!((left.y - 5.0).abs() < f32::EPSILON);
        assert!((left.height - 50.0).abs() < f32::EPSILON);
    }

    // --- M3-1: text-transform tests ---

    #[test]
    fn apply_text_transform_uppercase() {
        assert_eq!(
            super::apply_text_transform("hello", TextTransform::Uppercase),
            "HELLO"
        );
    }

    #[test]
    fn apply_text_transform_lowercase() {
        assert_eq!(
            super::apply_text_transform("HELLO", TextTransform::Lowercase),
            "hello"
        );
    }

    #[test]
    fn apply_text_transform_capitalize() {
        assert_eq!(
            super::apply_text_transform("hello world", TextTransform::Capitalize),
            "Hello World"
        );
    }

    #[test]
    fn apply_text_transform_none() {
        assert_eq!(
            super::apply_text_transform("Hello", TextTransform::None),
            "Hello"
        );
    }

    // --- M3-4: image rendering tests ---

    #[test]
    fn image_data_emits_image_item() {
        let (mut dom, img) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::TRANSPARENT,
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 10.0,
                    y: 10.0,
                    width: 200.0,
                    height: 100.0,
                },
                ..Default::default()
            },
        );
        let _ = dom.world_mut().insert_one(
            img,
            elidex_ecs::ImageData {
                pixels: Arc::new(vec![255u8; 4]), // 1x1 white pixel
                width: 1,
                height: 1,
            },
        );

        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        let image_items: Vec<_> =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::Image { .. }))
                .collect();
        assert_eq!(image_items.len(), 1);
        match &image_items[0] {
            DisplayItem::Image {
                rect,
                image_width,
                image_height,
                ..
            } => {
                assert!((rect.width - 200.0).abs() < f32::EPSILON);
                assert!((rect.height - 100.0).abs() < f32::EPSILON);
                assert_eq!(*image_width, 1);
                assert_eq!(*image_height, 1);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn no_image_data_no_image_item() {
        let (dom, _) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::RED,
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 50.0,
                },
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        let image_count =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::Image { .. }))
                .count();
        assert_eq!(image_count, 0);
    }

    #[test]
    fn image_opacity_zero_skipped() {
        let (mut dom, img) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::TRANSPARENT,
                opacity: 0.0,
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 50.0,
                },
                ..Default::default()
            },
        );
        let _ = dom.world_mut().insert_one(
            img,
            elidex_ecs::ImageData {
                pixels: Arc::new(vec![255u8; 4]),
                width: 1,
                height: 1,
            },
        );

        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        let image_count =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::Image { .. }))
                .count();
        assert_eq!(image_count, 0);
    }

    #[test]
    #[allow(unused_must_use)]
    fn text_decoration_underline_emits_solid_rect() {
        let (mut dom, div) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                font_family: test_font_family_strings(),
                text_decoration_line: TextDecorationLine {
                    underline: true,
                    line_through: false,
                },
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );
        let text = dom.create_text("Hello");
        dom.append_child(div, text);

        let font_db = FontDatabase::new();
        if !fonts_available(&font_db) {
            return;
        }

        let dl = build_display_list(&dom, &font_db);
        // Should have: Text item + SolidRect for underline.
        let text_count =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::Text { .. }))
                .count();
        let rect_count =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::SolidRect { .. }))
                .count();
        assert_eq!(text_count, 1);
        // At least 1 rect for underline (no background since transparent).
        assert!(rect_count >= 1, "expected underline rect, got {rect_count}");
    }

    // --- M3-5: styled inline runs ---

    #[test]
    #[allow(unused_must_use)]
    fn styled_span_color_preserved() {
        // <p><span style="color:red">red</span> normal</p>
        // The span text should have a different color from the parent text.
        let (mut dom, p) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                font_family: test_font_family_strings(),
                color: CssColor {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                },
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );
        let span = dom.create_element("span", Attributes::default());
        let t_red = dom.create_text("red");
        let t_normal = dom.create_text(" normal");
        dom.append_child(p, span);
        dom.append_child(span, t_red);
        dom.append_child(p, t_normal);
        dom.world_mut().insert_one(
            span,
            ComputedStyle {
                display: Display::Inline,
                font_family: test_font_family_strings(),
                color: CssColor {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                },
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        if !fonts_available(&font_db) {
            return;
        }

        let dl = build_display_list(&dom, &font_db);
        let text_items: Vec<_> =
            dl.0.iter()
                .filter_map(|i| {
                    if let DisplayItem::Text { color, .. } = i {
                        Some(*color)
                    } else {
                        None
                    }
                })
                .collect();
        // Should have 2 text items (span "red" + parent "normal").
        assert_eq!(text_items.len(), 2);
        // First text item from span should be red.
        assert_eq!(
            text_items[0],
            CssColor {
                r: 255,
                g: 0,
                b: 0,
                a: 255
            }
        );
        // Second text item from parent should be black.
        assert_eq!(
            text_items[1],
            CssColor {
                r: 0,
                g: 0,
                b: 0,
                a: 255
            }
        );
    }

    #[test]
    #[allow(unused_must_use)]
    fn display_none_inline_skipped() {
        // <p>visible <span style="display:none">hidden</span></p>
        let (mut dom, p) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                font_family: test_font_family_strings(),
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );
        let span = dom.create_element("span", Attributes::default());
        let t1 = dom.create_text("visible ");
        let t2 = dom.create_text("hidden");
        dom.append_child(p, t1);
        dom.append_child(p, span);
        dom.append_child(span, t2);
        dom.world_mut().insert_one(
            span,
            ComputedStyle {
                display: Display::None,
                font_family: test_font_family_strings(),
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        if !fonts_available(&font_db) {
            return;
        }

        let dl = build_display_list(&dom, &font_db);
        let text_items: Vec<_> =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::Text { .. }))
                .collect();
        // Only "visible" — hidden span is skipped.
        assert_eq!(text_items.len(), 1);
    }

    #[test]
    #[allow(unused_must_use)]
    fn styled_segments_x_consecutive() {
        // <p><span>A</span><span>B</span></p>
        // Two segments: A and B should have consecutive x positions.
        let (mut dom, p) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                font_family: test_font_family_strings(),
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );
        let s1 = dom.create_element("span", Attributes::default());
        let s2 = dom.create_element("span", Attributes::default());
        let t1 = dom.create_text("A");
        let t2 = dom.create_text("B");
        dom.append_child(p, s1);
        dom.append_child(s1, t1);
        dom.append_child(p, s2);
        dom.append_child(s2, t2);
        for &span in &[s1, s2] {
            dom.world_mut().insert_one(
                span,
                ComputedStyle {
                    display: Display::Inline,
                    font_family: test_font_family_strings(),
                    ..Default::default()
                },
            );
        }

        let font_db = FontDatabase::new();
        if !fonts_available(&font_db) {
            return;
        }

        let dl = build_display_list(&dom, &font_db);
        let text_first_x: Vec<f32> =
            dl.0.iter()
                .filter_map(|i| {
                    if let DisplayItem::Text { glyphs, .. } = i {
                        glyphs.first().map(|g| g.x)
                    } else {
                        None
                    }
                })
                .collect();
        // Two text items, second starts after first.
        assert_eq!(text_first_x.len(), 2);
        assert!(
            text_first_x[1] > text_first_x[0],
            "second segment x={} should be > first x={}",
            text_first_x[1],
            text_first_x[0]
        );
    }

    // --- M3-6: overflow: hidden → PushClip/PopClip ---

    #[test]
    fn overflow_hidden_emits_clip() {
        let (dom, _) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                overflow: Overflow::Hidden,
                background_color: CssColor::WHITE,
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 10.0,
                    y: 10.0,
                    width: 100.0,
                    height: 50.0,
                },
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        let has_push_clip =
            dl.0.iter()
                .any(|i| matches!(i, DisplayItem::PushClip { .. }));
        let has_pop_clip = dl.0.iter().any(|i| matches!(i, DisplayItem::PopClip));
        assert!(has_push_clip, "overflow:hidden should emit PushClip");
        assert!(has_pop_clip, "overflow:hidden should emit PopClip");
    }

    #[test]
    fn overflow_visible_no_clip() {
        let (dom, _) = setup_block_element(
            ComputedStyle {
                display: Display::Block,
                overflow: Overflow::Visible,
                background_color: CssColor::RED,
                ..Default::default()
            },
            LayoutBox {
                content: Rect {
                    x: 10.0,
                    y: 10.0,
                    width: 100.0,
                    height: 50.0,
                },
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        let has_push_clip =
            dl.0.iter()
                .any(|i| matches!(i, DisplayItem::PushClip { .. }));
        assert!(!has_push_clip, "overflow:visible should not emit PushClip");
    }

    // --- M3-6: list markers ---

    #[test]
    #[allow(unused_must_use)]
    fn list_item_disc_emits_marker() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let ul = dom.create_element("ul", Attributes::default());
        dom.append_child(root, ul);
        let li = dom.create_element("li", Attributes::default());
        dom.append_child(ul, li);

        dom.world_mut().insert_one(
            ul,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            ul,
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 100.0,
                },
                padding: EdgeSizes::new(0.0, 0.0, 0.0, 40.0),
                ..Default::default()
            },
        );

        dom.world_mut().insert_one(
            li,
            ComputedStyle {
                display: Display::ListItem,
                list_style_type: ListStyleType::Disc,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            li,
            LayoutBox {
                content: Rect {
                    x: 40.0,
                    y: 0.0,
                    width: 760.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        // Disc marker should emit a RoundedRect.
        let has_marker =
            dl.0.iter()
                .any(|i| matches!(i, DisplayItem::RoundedRect { .. }));
        assert!(has_marker, "disc list marker should emit RoundedRect");
    }

    #[test]
    #[allow(unused_must_use)]
    fn list_item_square_emits_solid_rect_marker() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let ul = dom.create_element("ul", Attributes::default());
        dom.append_child(root, ul);
        let li = dom.create_element("li", Attributes::default());
        dom.append_child(ul, li);

        dom.world_mut().insert_one(
            ul,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            ul,
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 100.0,
                },
                padding: EdgeSizes::new(0.0, 0.0, 0.0, 40.0),
                ..Default::default()
            },
        );

        dom.world_mut().insert_one(
            li,
            ComputedStyle {
                display: Display::ListItem,
                list_style_type: ListStyleType::Square,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            li,
            LayoutBox {
                content: Rect {
                    x: 40.0,
                    y: 0.0,
                    width: 760.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        // The first SolidRect with a very small width is the square marker.
        let small_rects: Vec<_> =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::SolidRect { rect, .. } if rect.width < 10.0))
                .collect();
        assert!(
            !small_rects.is_empty(),
            "square list marker should emit small SolidRect"
        );
    }

    #[test]
    #[allow(unused_must_use)]
    fn list_item_none_no_marker() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let ul = dom.create_element("ul", Attributes::default());
        dom.append_child(root, ul);
        let li = dom.create_element("li", Attributes::default());
        dom.append_child(ul, li);

        dom.world_mut().insert_one(
            ul,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            ul,
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 100.0,
                },
                ..Default::default()
            },
        );

        dom.world_mut().insert_one(
            li,
            ComputedStyle {
                display: Display::ListItem,
                list_style_type: ListStyleType::None,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            li,
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        // list-style-type: none should not emit any marker shapes.
        let has_rounded =
            dl.0.iter()
                .any(|i| matches!(i, DisplayItem::RoundedRect { .. }));
        assert!(!has_rounded, "list-style-type:none should not emit marker");
    }

    #[test]
    #[allow(unused_must_use)]
    fn list_item_circle_emits_stroked_marker() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let ul = dom.create_element("ul", Attributes::default());
        dom.append_child(root, ul);
        let li = dom.create_element("li", Attributes::default());
        dom.append_child(ul, li);

        dom.world_mut().insert_one(
            ul,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            ul,
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 100.0,
                },
                padding: EdgeSizes::new(0.0, 0.0, 0.0, 40.0),
                ..Default::default()
            },
        );

        dom.world_mut().insert_one(
            li,
            ComputedStyle {
                display: Display::ListItem,
                list_style_type: ListStyleType::Circle,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            li,
            LayoutBox {
                content: Rect {
                    x: 40.0,
                    y: 0.0,
                    width: 760.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        // Circle marker should emit StrokedRoundedRect (outline), not filled RoundedRect.
        let has_stroked =
            dl.0.iter()
                .any(|i| matches!(i, DisplayItem::StrokedRoundedRect { .. }));
        assert!(
            has_stroked,
            "circle list marker should emit StrokedRoundedRect"
        );
        let has_filled =
            dl.0.iter()
                .any(|i| matches!(i, DisplayItem::RoundedRect { .. }));
        assert!(
            !has_filled,
            "circle list marker should not emit filled RoundedRect"
        );
    }

    #[test]
    #[allow(unused_must_use)]
    fn list_item_decimal_emits_text_marker() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let ol = dom.create_element("ol", Attributes::default());
        dom.append_child(root, ol);
        let li = dom.create_element("li", Attributes::default());
        dom.append_child(ol, li);

        dom.world_mut().insert_one(
            ol,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            ol,
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 100.0,
                },
                padding: EdgeSizes::new(0.0, 0.0, 0.0, 40.0),
                ..Default::default()
            },
        );

        dom.world_mut().insert_one(
            li,
            ComputedStyle {
                display: Display::ListItem,
                list_style_type: ListStyleType::Decimal,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            li,
            LayoutBox {
                content: Rect {
                    x: 40.0,
                    y: 0.0,
                    width: 760.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        // Decimal marker emits Text items (if fonts are available) or nothing
        // (graceful fallback). It should never emit shape-based markers.
        let has_shape = dl.0.iter().any(|i| {
            matches!(
                i,
                DisplayItem::RoundedRect { .. } | DisplayItem::StrokedRoundedRect { .. }
            )
        });
        assert!(
            !has_shape,
            "decimal list marker should not emit shape-based markers"
        );
        // If system fonts are available, a Text item should be emitted.
        let has_text = dl.0.iter().any(|i| matches!(i, DisplayItem::Text { .. }));
        if font_db.query(&["serif"], 400).is_some() {
            assert!(
                has_text,
                "decimal marker should emit Text when fonts available"
            );
        }
    }

    // --- M3-6: white-space collapse tests ---

    fn make_segment(text: &str) -> StyledTextSegment {
        StyledTextSegment {
            text: text.to_string(),
            color: CssColor::BLACK,
            font_family: vec!["serif".to_string()],
            font_size: 16.0,
            font_weight: 400,
            text_transform: TextTransform::None,
            text_decoration_line: TextDecorationLine::default(),
            opacity: 1.0,
        }
    }

    #[test]
    fn collapse_segments_normal_collapses_spaces_and_newlines() {
        let segments = vec![make_segment("hello  \n  world")];
        let result = collapse_segments(&segments, WhiteSpace::Normal);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "hello world");
    }

    #[test]
    fn collapse_segments_pre_preserves_all() {
        let segments = vec![make_segment("hello  \n  world")];
        let result = collapse_segments(&segments, WhiteSpace::Pre);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "hello  \n  world");
    }

    #[test]
    fn collapse_segments_pre_line_collapses_spaces_preserves_newlines() {
        let segments = vec![make_segment("hello   \n   world")];
        let result = collapse_segments(&segments, WhiteSpace::PreLine);
        assert_eq!(result.len(), 1);
        // Spaces collapsed, but newline preserved.
        assert!(
            result[0].0.contains('\n'),
            "pre-line should preserve newlines"
        );
        assert!(
            !result[0].0.contains("   "),
            "pre-line should collapse spaces"
        );
    }

    #[test]
    fn collapse_segments_nowrap_same_as_normal() {
        let segments = vec![make_segment("hello  \n  world")];
        let result = collapse_segments(&segments, WhiteSpace::NoWrap);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "hello world");
    }

    #[test]
    fn collapse_segments_pre_wrap_preserves_all() {
        let segments = vec![make_segment("  hello  \n  world  ")];
        let result = collapse_segments(&segments, WhiteSpace::PreWrap);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "  hello  \n  world  ");
    }

    #[test]
    fn collapse_segments_pre_line_preserves_trailing_newline() {
        // pre-line: spaces collapse, newlines preserved; trailing newline kept.
        let segments = vec![make_segment("hello\n")];
        let result = collapse_segments(&segments, WhiteSpace::PreLine);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "hello\n");
    }

    #[test]
    fn collapse_segments_pre_line_trims_trailing_spaces_only() {
        let segments = vec![make_segment("hello   ")];
        let result = collapse_segments(&segments, WhiteSpace::PreLine);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "hello");
    }

    #[test]
    fn collapse_segments_pre_line_strips_spaces_before_newline() {
        // CSS Text §4: spaces/tabs immediately preceding a forced break are removed.
        let segments = vec![make_segment("hello   \nworld")];
        let result = collapse_segments(&segments, WhiteSpace::PreLine);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "hello\nworld");
    }

    #[test]
    fn collapse_segments_crlf_normalized_to_lf() {
        // CSS Text §4.1 Phase I: \r\n → single \n.
        let segments = vec![make_segment("hello\r\nworld")];
        let result = collapse_segments(&segments, WhiteSpace::PreLine);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "hello\nworld");
    }

    #[test]
    fn collapse_segments_bare_cr_normalized_to_lf() {
        // CSS Text §4.1 Phase I: bare \r → \n.
        let segments = vec![make_segment("hello\rworld")];
        let result = collapse_segments(&segments, WhiteSpace::Pre);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "hello\nworld");
    }

    // --- L11: nested overflow:hidden ---

    #[test]
    #[allow(unused_must_use)]
    fn nested_overflow_hidden_balanced_clips() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let outer = dom.create_element("div", Attributes::default());
        dom.append_child(root, outer);
        let inner = dom.create_element("div", Attributes::default());
        dom.append_child(outer, inner);

        for (entity, w, h) in [(outer, 200.0, 100.0), (inner, 100.0, 50.0)] {
            dom.world_mut().insert_one(
                entity,
                ComputedStyle {
                    display: Display::Block,
                    overflow: Overflow::Hidden,
                    background_color: CssColor::WHITE,
                    ..Default::default()
                },
            );
            dom.world_mut().insert_one(
                entity,
                LayoutBox {
                    content: Rect {
                        x: 0.0,
                        y: 0.0,
                        width: w,
                        height: h,
                    },
                    ..Default::default()
                },
            );
        }

        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        let push_count =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::PushClip { .. }))
                .count();
        let pop_count =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::PopClip))
                .count();
        assert_eq!(push_count, 2, "should have 2 PushClip for nested overflow");
        assert_eq!(pop_count, 2, "should have 2 PopClip for nested overflow");
        assert_eq!(push_count, pop_count, "PushClip/PopClip must be balanced");
    }

    // --- L14: multi-item list counter ---

    #[test]
    #[allow(unused_must_use)]
    fn list_item_counter_increments() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let ol = dom.create_element("ol", Attributes::default());
        dom.append_child(root, ol);
        let li1 = dom.create_element("li", Attributes::default());
        dom.append_child(ol, li1);
        let li2 = dom.create_element("li", Attributes::default());
        dom.append_child(ol, li2);

        dom.world_mut().insert_one(
            ol,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            ol,
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 200.0,
                },
                padding: EdgeSizes::new(0.0, 0.0, 0.0, 40.0),
                ..Default::default()
            },
        );

        for (li, y_off) in [(li1, 0.0), (li2, 20.0)] {
            dom.world_mut().insert_one(
                li,
                ComputedStyle {
                    display: Display::ListItem,
                    list_style_type: ListStyleType::Disc,
                    ..Default::default()
                },
            );
            dom.world_mut().insert_one(
                li,
                LayoutBox {
                    content: Rect {
                        x: 40.0,
                        y: y_off,
                        width: 760.0,
                        height: 20.0,
                    },
                    ..Default::default()
                },
            );
        }

        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        // Both list items should emit a marker (RoundedRect for disc).
        let marker_count =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::RoundedRect { .. }))
                .count();
        assert_eq!(marker_count, 2, "two disc markers for two list items");
    }
}
