//! Inline run collection and emission.

use elidex_ecs::{
    EcsDom, Entity, InlineFlow, InlineFlowRun, InlineFragment, PseudoElementMarker, TextContent,
};
use elidex_plugin::transform_math::Perspective;
use elidex_plugin::{
    ComputedStyle, CssColor, Direction, Display, FontStyle as PluginFontStyle, LayoutBox, Point,
    TextAlign, TextDecorationLine, TextDecorationStyle, TextOrientation, TextTransform, Visibility,
    WritingMode,
};
use elidex_text::FontDatabase;

use crate::display_list::{DisplayItem, DisplayList, GlyphEntry};
use crate::font_cache::FontCache;

use super::{
    apply_opacity, bidi_visual_order, collapse_segments, compute_text_align_offset,
    find_nearest_layout_box, place_glyphs, place_glyphs_vertical, resolve_text_align, walk,
    PaintContext, DECORATION_THICKNESS_DIVISOR, DEFAULT_DESCENT_FACTOR,
    LINE_THROUGH_POSITION_FACTOR, OVERLINE_POSITION_FACTOR, UNDERLINE_POSITION_FACTOR,
};
use elidex_text::{shape_text, shape_text_vertical, shape_text_vertical_sideways};

/// A segment of text with its own style properties.
///
/// `font_family` is an owned `Vec<String>` rather than a reference because
/// segments outlive the `ComputedStyle` borrows they are created from
/// (collected into a `Vec` and consumed after the DOM walk). Switching to
/// `&[String]` would require a lifetime parameter that propagates through
/// `collapse_segments`, `bidi_visual_order`, and the display-list builder,
/// so the allocation is accepted here.
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
            letter_spacing: style.letter_spacing.unwrap_or(0.0),
            word_spacing: style.word_spacing.unwrap_or(0.0),
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

/// Whether a persisted [`InlineFragment`] belongs to the page currently being
/// walked: every fragment off the paged path (`expected == None` — a
/// non-fragmented flow has exactly one), else only the one stamped with this
/// page's generation. The consume gate and the `emit_inline_flow` paint loop
/// share this predicate so they never disagree on which fragments paint (D4).
fn fragment_matches_page(frag: &InlineFragment, expected: Option<u32>) -> bool {
    expected.is_none_or(|g| frag.generation == g)
}

/// Collect styled text segments from an inline run and render them.
///
/// An inline run is a sequence of non-block children (text nodes and
/// inline elements). Each text segment preserves its element's style
/// (color, font, etc.), allowing `<span style="color:red">` to render
/// in the correct color.
pub(crate) fn emit_inline_run(
    ctx: &mut PaintContext,
    parent: Entity,
    run: &[Entity],
    depth: usize,
    child_perspective: &Perspective,
    in_transform: bool,
) {
    // Converged path: if layout persisted an `InlineFlow` on this run's start
    // entity (`run[0]`, the same key both passes derive), consume its collapsed +
    // positioned members instead of re-collecting/re-collapsing/re-measuring the
    // DOM. The generation check reads each fragment's OWN `generation`; off the
    // paged path (`expected_generation == None`) it is a no-op and presence alone
    // gates, because layout explicitly clears the flow when a run becomes
    // non-persistable. On the paged path we consume iff SOME fragment matches this
    // page (`emit_inline_flow` then paints only the matching fragment(s)). Read the
    // gate as a bool so the `InlineFlow` borrow drops before `emit_inline_flow`
    // takes `&mut ctx` (it re-gets the flow internally).
    if let Some(&first) = run.first() {
        let expected = ctx.expected_generation;
        let consume = ctx.dom.world().get::<&InlineFlow>(first).is_ok_and(|flow| {
            flow.fragments
                .iter()
                .any(|frag| fragment_matches_page(frag, expected))
        });
        if consume {
            // The horizontal/vertical dispatch needs only the parent's `writing_mode`
            // + `text_orientation` (Copy enums) — read those WITHOUT cloning the parent
            // `ComputedStyle`. Render must interpret the persisted coordinates with the
            // SAME writing mode layout used to project them — i.e. the IFC parent's — so
            // the dispatch reads the parent's, not each member's. A styleless parent →
            // horizontal default, mirroring layout's `get_style` `unwrap_or_default`
            // tolerance (it persisted under the same default), so the flow still paints.
            let (writing_mode, text_orientation) = ctx
                .dom
                .world()
                .get::<&ComputedStyle>(parent)
                .map_or((WritingMode::HorizontalTb, TextOrientation::Mixed), |s| {
                    (s.writing_mode, s.text_orientation)
                });
            emit_inline_flow(
                ctx,
                first,
                writing_mode,
                text_orientation,
                depth,
                child_perspective,
                in_transform,
            );
            return;
        }
    }

    // Legacy path: needs the full parent style for collect/collapse/measure.
    let parent_style = match ctx.dom.world().get::<&ComputedStyle>(parent) {
        Ok(s) => s.clone(),
        Err(_) => return,
    };
    let Some(lb) = find_nearest_layout_box(ctx.dom, parent) else {
        return;
    };

    let segments = collect_styled_inline_text(ctx.dom, run, &parent_style, 0);
    if segments.is_empty() {
        return;
    }

    // Check if all segments are whitespace-only after cross-segment collapsing.
    let collapsed = collapse_segments(&segments, parent_style.white_space);
    if collapsed.is_empty() {
        return;
    }

    let run_ctx = InlineRunContext {
        segments: &segments,
        collapsed: &collapsed,
        lb: &lb,
        parent_style: &parent_style,
    };
    emit_styled_segments(&run_ctx, ctx.font_db, ctx.font_cache, ctx.dl);
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

            // Pseudo-element: emit its resolved generated text with its own style
            // (skip child recursion). `content` — including counter() / counters()
            // — has already been resolved into the pseudo's `TextContent` by the
            // pre-layout generated-content pass (the single resolver), so render
            // just reads it.
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

    let is_vertical = !parent_style.writing_mode.is_horizontal();

    if is_vertical {
        emit_styled_segments_vertical(ctx, font_db, font_cache, dl);
        return;
    }

    let align_result = compute_text_align_offset(
        parent_style.text_align,
        parent_style.direction,
        lb.content.size.width,
        collapsed,
        segments,
        font_db,
    );

    // Reorder segments for visual display (bidi algorithm).
    let visual_order = bidi_visual_order(collapsed, parent_style.direction);

    // Emit display items (single shaping pass per segment).
    let mut cursor_x = lb.content.origin.x + align_result.offset;

    for &vi in &visual_order {
        let Some((ref text, idx)) = collapsed.get(vi) else {
            continue;
        };
        let Some(seg) = segments.get(*idx) else {
            continue;
        };
        // The whole horizontal run shares one line at the parent content-box top
        // (this legacy path does not line-break — see `emit_inline_flow` for the
        // converged, per-line-positioned path).
        emit_text_segment(
            text,
            seg,
            &mut cursor_x,
            lb.content.origin.y,
            align_result.justify_extra_word_spacing,
            font_db,
            font_cache,
            dl,
        );
    }
}

/// Shape one collapsed text segment and emit its glyphs + text-decoration at the
/// current inline cursor, on the line whose top edge is `line_top_y`.
///
/// Shared by the legacy single-line `emit_styled_segments` and the converged
/// `emit_inline_flow` (per-line positioned). The baseline is `line_top_y + ascent`
/// (CSS 2 §10.8.1 leading is not yet modelled — preserved from the legacy path).
/// `cursor_x` is advanced past the segment so the caller can place the next one;
/// the converged path sets it explicitly per run instead.
#[allow(clippy::too_many_arguments)]
fn emit_text_segment(
    text: &str,
    seg: &StyledTextSegment,
    cursor_x: &mut f32,
    line_top_y: f32,
    justify_extra_word_spacing: f32,
    font_db: &FontDatabase,
    font_cache: &mut FontCache,
    dl: &mut DisplayList,
) {
    let Some((transformed, font_id)) = super::query_segment_font(text, seg, font_db) else {
        return;
    };
    let Some(shaped) = shape_text(font_db, font_id, seg.font_size, &transformed) else {
        return;
    };
    let Some((font_blob, font_index)) = font_cache.get(font_db, font_id) else {
        return;
    };
    let text_color = apply_opacity(seg.color, seg.opacity);

    let metrics = font_db.font_metrics(font_id, seg.font_size);
    let ascent = metrics.map_or(seg.font_size, |m| m.ascent);
    let descent = metrics.map_or(-seg.font_size * DEFAULT_DESCENT_FACTOR, |m| m.descent);
    let baseline_y = line_top_y + ascent;

    let seg_start_x = *cursor_x;
    let glyphs = place_glyphs(
        &shaped.glyphs,
        cursor_x,
        baseline_y,
        seg.letter_spacing,
        seg.word_spacing + justify_extra_word_spacing,
        &transformed,
    );
    let seg_width = *cursor_x - seg_start_x;

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

/// Consume a layout-produced [`InlineFlow`]: paint each line's members at their
/// absolute, already-projected coordinates. This is the converged path — render no
/// longer re-collects / re-collapses / re-measures the DOM, and (unlike the legacy
/// single-line `emit_styled_segments`) honours layout's per-line line-breaking and
/// per-line `text-align`. Two member kinds:
/// - [`InlineFlowRun::Text`]: shaped + emitted at `inline_start` with the run
///   entity's `ComputedStyle`.
/// - [`InlineFlowRun::AtomicBox`]: an atomic inline-level box, painted by
///   `walk()`-ing the entity (chrome + descendants + its own inner IFC) at the
///   `LayoutBox` layout repositioned to the member's line position. Atomic members
///   are collected during the borrow-scoped text loop and walked **after** the
///   `InlineFlow` borrow drops — `walk()` needs `&mut ctx`, which conflicts with the
///   read borrow; render is read-only on the DOM, so this avoids cloning the flow.
///
/// Layout applied the writing-mode projection at persist, so each scalar holds the
/// absolute physical coordinate for its axis; render reads them without a transform
/// and only branches the per-run glyph emit on `writing_mode` — the IFC parent's,
/// i.e. the writing mode layout used when projecting these coordinates (the caller
/// reads it from the parent, not per member):
/// - **horizontal**: `inline_start` = physical x, `block_start` = physical line top.
/// - **vertical**: `block_start`/`block_size` give the glyph-column center x;
///   `inline_start` = physical y (pen top). `text_orientation` selects the shaping.
fn emit_inline_flow(
    ctx: &mut PaintContext,
    first: Entity,
    writing_mode: WritingMode,
    text_orientation: TextOrientation,
    depth: usize,
    child_perspective: &Perspective,
    in_transform: bool,
) {
    let vertical = !writing_mode.is_horizontal();
    let orient = vertical_text_orientation(writing_mode, text_orientation);
    // On the paged path, paint only the fragment(s) belonging to this page
    // (`expected_generation`); off it (`None`), paint every fragment — a
    // non-fragmented flow has exactly one (generation 0). The matching
    // gate mirrors the consume check in `emit_inline_run`.
    let expected = ctx.expected_generation;
    // Atomic members collected here, walked after the `InlineFlow` borrow drops.
    let mut atomics: Vec<Entity> = Vec::new();
    {
        // Copy the shared `&EcsDom` out so reads borrow the DOM (not `ctx`), leaving
        // `ctx.font_db`/`font_cache`/`dl` free for the text emit (disjoint fields).
        let dom = ctx.dom;
        let Ok(flow) = dom.world().get::<&InlineFlow>(first) else {
            return;
        };
        for line in flow
            .fragments
            .iter()
            .filter(|frag| fragment_matches_page(frag, expected))
            .flat_map(|frag| frag.lines.iter())
        {
            for run in &line.runs {
                match run {
                    InlineFlowRun::Text {
                        entity,
                        text,
                        inline_start,
                    } => {
                        let Ok(style) = dom.world().get::<&ComputedStyle>(*entity) else {
                            continue;
                        };
                        // visibility: hidden text occupies space but is not painted.
                        if style.visibility != Visibility::Visible {
                            continue;
                        }
                        // Style-only segment: the collapsed text comes from the flow
                        // member (`text`), so the segment's own text field is unused.
                        // Layout already applied `text-transform` before measuring, so
                        // the persisted `text` is final — paint it verbatim (force the
                        // segment's transform to `None` so the shared emit path does not
                        // re-transform; CSS Text 3 §2.1, render = paint-only).
                        let mut seg = StyledTextSegment::from_style(String::new(), &style);
                        seg.text_transform = TextTransform::None;
                        if vertical {
                            let center_x = line.block_start + line.block_size / 2.0;
                            let mut cursor_y = *inline_start;
                            emit_vertical_text_segment(
                                text,
                                &seg,
                                orient,
                                center_x,
                                &mut cursor_y,
                                ctx.font_db,
                                ctx.font_cache,
                                ctx.dl,
                            );
                        } else {
                            let mut cursor_x = *inline_start;
                            emit_text_segment(
                                text,
                                &seg,
                                &mut cursor_x,
                                line.block_start,
                                0.0,
                                ctx.font_db,
                                ctx.font_cache,
                                ctx.dl,
                            );
                        }
                    }
                    InlineFlowRun::AtomicBox { entity, .. } => atomics.push(*entity),
                }
            }
        }
    }
    // Paint each atomic inline-level box by walking it at its (layout-repositioned)
    // `LayoutBox` — the same depth/perspective/in_transform a block child gets
    // (`paint_non_sc` walks block children at `depth + 1`).
    for atomic in atomics {
        walk(ctx, atomic, depth + 1, child_perspective, in_transform);
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
        lb.content.size.height,
        collapsed,
        segments,
        font_db,
    );

    // A1: Apply BiDi visual reordering (same as horizontal path).
    // Note: BiDi reorder on vertical text reorders top-to-bottom runs.
    // CSS Writing Modes Level 4 §3.1 (Introduction to Vertical Writing) says
    // text in vertical modes reads top-to-bottom (inline direction = TTB);
    // current behaviour is correct for LTR vertical text.
    let visual_order = bidi_visual_order(collapsed, parent_style.direction);

    let text_orientation =
        vertical_text_orientation(parent_style.writing_mode, parent_style.text_orientation);

    // Vertical: cursor_y advances downward, center_x is the column center.
    let center_x = lb.content.center().x;
    let mut cursor_y = lb.content.origin.y + align_offset;

    for &vi in &visual_order {
        let Some((ref text, idx)) = collapsed.get(vi) else {
            continue;
        };
        let Some(seg) = segments.get(*idx) else {
            continue;
        };
        emit_vertical_text_segment(
            text,
            seg,
            text_orientation,
            center_x,
            &mut cursor_y,
            font_db,
            font_cache,
            dl,
        );
    }
}

/// CSS Writing Modes Level 4 §5.1: `sideways-rl`/`sideways-lr` force all glyphs
/// sideways; otherwise the `text-orientation` property (mixed/upright/sideways)
/// selects the shaping strategy. Shared by the legacy single-column path and the
/// converged `emit_inline_flow` vertical branch.
fn vertical_text_orientation(
    writing_mode: WritingMode,
    text_orientation: TextOrientation,
) -> TextOrientation {
    if matches!(
        writing_mode,
        WritingMode::SidewaysRl | WritingMode::SidewaysLr
    ) {
        TextOrientation::Sideways
    } else {
        text_orientation
    }
}

/// Shape and emit one collapsed text segment vertically (downward), advancing
/// `cursor_y` past it. Shared by the legacy single-column
/// `emit_styled_segments_vertical` and the converged per-line vertical branch of
/// [`emit_inline_flow`] (mirrors the horizontal [`emit_text_segment`]).
/// `center_x` is the glyph-column center; `text_orientation` selects the shaping
/// strategy (CSS Writing Modes Level 4 §5.1).
#[allow(clippy::too_many_arguments)]
fn emit_vertical_text_segment(
    text: &str,
    seg: &StyledTextSegment,
    text_orientation: TextOrientation,
    center_x: f32,
    cursor_y: &mut f32,
    font_db: &FontDatabase,
    font_cache: &mut FontCache,
    dl: &mut DisplayList,
) {
    let Some((transformed, font_id)) = super::query_segment_font(text, seg, font_db) else {
        return;
    };
    let Some((font_blob, font_index)) = font_cache.get(font_db, font_id) else {
        return;
    };
    let text_color = apply_opacity(seg.color, seg.opacity);

    let seg_start_y = *cursor_y;

    // CSS Writing Modes Level 4 §5.1: text-orientation determines shaping.
    // - Sideways: shape horizontally, rotate 90° CW (x-advance → y-advance).
    // - Upright/Mixed: shape with TTB + OpenType vert feature.
    let shaped_opt = match text_orientation {
        TextOrientation::Sideways => {
            shape_text_vertical_sideways(font_db, font_id, seg.font_size, &transformed)
        }
        _ => shape_text_vertical(font_db, font_id, seg.font_size, &transformed),
    };

    let Some(shaped) = shaped_opt else {
        // Fallback to horizontal shaping if vertical shaping fails.
        let Some(shaped) = shape_text(font_db, font_id, seg.font_size, &transformed) else {
            return;
        };
        // Place horizontally-shaped glyphs vertically (one per line).
        // Glyph y_offset from horizontal shaping may have incorrect sign
        // for vertical layout (diacritics direction), but is acceptable
        // for most Latin/CJK text.
        for glyph in &shaped.glyphs {
            let x = center_x + glyph.x_offset - glyph.x_advance / 2.0;
            let y = *cursor_y + glyph.y_offset;
            dl.push(DisplayItem::Text {
                glyphs: vec![GlyphEntry {
                    glyph_id: u32::from(glyph.glyph_id),
                    position: Point::new(x, y),
                }],
                font_blob: font_blob.clone(),
                font_index,
                font_size: seg.font_size,
                color: text_color,
            });
            *cursor_y += glyph.x_advance;
        }
        return;
    };
    let glyphs = place_glyphs_vertical(&shaped.glyphs, center_x, cursor_y);

    dl.push(DisplayItem::Text {
        glyphs,
        font_blob,
        font_index,
        font_size: seg.font_size,
        color: text_color,
    });

    // CSS Writing Modes Level 4 §7.1: vertical text-decoration runs vertically
    // alongside the glyph column. Placement here is writing-mode-agnostic —
    // underline on the +x side of the column center, overline on the −x side,
    // line-through through the center — NOT yet the spec's vertical-rl/lr-dependent
    // under/over sides (deferred with the other vertical-rl physical-correctness
    // work; same family as the InlineFlow no-block-reversal limitation).
    let seg_height = *cursor_y - seg_start_y;
    if seg_height > 0.0 {
        let ascent = seg.font_size;
        let decoration_thickness = ascent / DECORATION_THICKNESS_DIVISOR;
        let decoration_color =
            apply_opacity(seg.text_decoration_color.unwrap_or(seg.color), seg.opacity);
        // Underline: +x side of the column center (writing-mode-agnostic, see above).
        if seg.text_decoration_line.underline {
            let x = center_x + ascent * UNDERLINE_POSITION_FACTOR;
            emit_vertical_decoration_line(
                dl,
                x,
                seg_start_y,
                seg_height,
                decoration_thickness,
                decoration_color,
                seg.text_decoration_style,
            );
        }
        if seg.text_decoration_line.overline {
            let x = center_x - ascent * OVERLINE_POSITION_FACTOR;
            emit_vertical_decoration_line(
                dl,
                x,
                seg_start_y,
                seg_height,
                decoration_thickness,
                decoration_color,
                seg.text_decoration_style,
            );
        }
        if seg.text_decoration_line.line_through {
            let x = center_x;
            emit_vertical_decoration_line(
                dl,
                x,
                seg_start_y,
                seg_height,
                decoration_thickness,
                decoration_color,
                seg.text_decoration_style,
            );
        }
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
            if !total_height.is_finite() {
                return 0.0;
            }
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
    // Guard: skip entirely if any coordinate is non-finite or size is non-positive.
    if !x.is_finite()
        || !y.is_finite()
        || !width.is_finite()
        || !thickness.is_finite()
        || width <= 0.0
        || thickness <= 0.0
    {
        return;
    }
    match style {
        TextDecorationStyle::Solid | TextDecorationStyle::Wavy => {
            dl.push(DisplayItem::SolidRect {
                rect: elidex_plugin::Rect::new(x, y, width, thickness),
                color,
            });
        }
        TextDecorationStyle::Double => {
            let thin = (thickness * 0.5).max(1.0);
            dl.push(DisplayItem::SolidRect {
                rect: elidex_plugin::Rect::new(x, y, width, thin),
                color,
            });
            dl.push(DisplayItem::SolidRect {
                rect: elidex_plugin::Rect::new(x, y + thickness, width, thin),
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

/// Emit a vertical text-decoration line (CSS Writing Modes Level 4 §7.1).
///
/// Like [`emit_decoration_line`] but oriented vertically: the line runs along
/// the y-axis with `height` extent and `thickness` in the x-axis.
fn emit_vertical_decoration_line(
    dl: &mut DisplayList,
    x: f32,
    y: f32,
    height: f32,
    thickness: f32,
    color: CssColor,
    style: TextDecorationStyle,
) {
    if !x.is_finite()
        || !y.is_finite()
        || !height.is_finite()
        || !thickness.is_finite()
        || height <= 0.0
        || thickness <= 0.0
    {
        return;
    }
    match style {
        TextDecorationStyle::Solid | TextDecorationStyle::Wavy => {
            dl.push(DisplayItem::SolidRect {
                rect: elidex_plugin::Rect::new(x, y, thickness, height),
                color,
            });
        }
        TextDecorationStyle::Double => {
            let thin = (thickness * 0.5).max(1.0);
            dl.push(DisplayItem::SolidRect {
                rect: elidex_plugin::Rect::new(x, y, thin, height),
                color,
            });
            dl.push(DisplayItem::SolidRect {
                rect: elidex_plugin::Rect::new(x + thickness, y, thin, height),
                color,
            });
        }
        TextDecorationStyle::Dotted => {
            emit_vertical_repeating_decoration(dl, x, y, height, thickness, thickness, color);
        }
        TextDecorationStyle::Dashed => {
            emit_vertical_repeating_decoration(dl, x, y, height, thickness * 3.0, thickness, color);
        }
    }
}

/// Emit a vertical repeating decoration pattern (dots or dashes).
fn emit_vertical_repeating_decoration(
    dl: &mut DisplayList,
    x: f32,
    y: f32,
    height: f32,
    mark_height: f32,
    thickness: f32,
    color: CssColor,
) {
    let step = mark_height + thickness;
    if step <= 0.0 || !step.is_finite() || !x.is_finite() || !y.is_finite() {
        return;
    }
    let mut cy = y;
    let end = y + height;
    let mut count = 0usize;
    while cy < end && count < MAX_DECORATION_MARKS {
        let h = mark_height.min(end - cy);
        dl.push(DisplayItem::SolidRect {
            rect: elidex_plugin::Rect::new(x, cy, thickness, h),
            color,
        });
        cy += step;
        count += 1;
    }
}

/// Emit a repeating decoration pattern (dots or dashes).
///
/// Each mark has `mark_width` inline extent and `thickness` block extent,
/// separated by gaps equal to `thickness`.
/// Maximum number of marks in a repeating decoration (dotted/dashed) to prevent
/// memory exhaustion on extreme inputs.
const MAX_DECORATION_MARKS: usize = 10_000;

fn emit_repeating_decoration(
    dl: &mut DisplayList,
    x: f32,
    y: f32,
    width: f32,
    mark_width: f32,
    thickness: f32,
    color: CssColor,
) {
    // Note: -0.0 inputs are safe here — `step <= 0.0` catches -0.0 + -0.0,
    // and the MAX_DECORATION_MARKS cap prevents runaway loops in all cases.
    let step = mark_width + thickness;
    if step <= 0.0 || !step.is_finite() || !x.is_finite() || !y.is_finite() {
        return;
    }
    let mut cx = x;
    let end = x + width;
    let mut count = 0usize;
    while cx < end && count < MAX_DECORATION_MARKS {
        let w = mark_width.min(end - cx);
        dl.push(DisplayItem::SolidRect {
            rect: elidex_plugin::Rect::new(cx, y, w, thickness),
            color,
        });
        cx += step;
        count += 1;
    }
}
