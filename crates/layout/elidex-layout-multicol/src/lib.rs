//! CSS Multi-column Layout Level 1 implementation.
//!
//! Computes column geometry, fills columns sequentially or with balanced
//! heights, handles `column-span: all` spanning elements, and attaches
//! [`MulticolInfo`] for column rule rendering.

pub mod algo;
mod fill;

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::{
    adjust_min_max_for_border_box, clamp_min_max, composed_children_flat, resolve_dimension_value,
    resolve_padding, sanitize_border, ChildLayoutFn, LayoutInput, LayoutOutcome,
};
use elidex_plugin::{
    BoxSizing, ColumnFill, ColumnSpan, ComputedStyle, Dimension, Display, EdgeSizes, Float,
    LayoutBox, MulticolInfo, Position, Rect, WritingModeContext,
};

use algo::{compute_column_geometry, ColumnGeometry};
use fill::{fill_columns_balanced, fill_columns_sequential};

/// Maximum depth for descendant walks to prevent infinite loops on corrupted trees.
const MAX_DESCENDANT_DEPTH: usize = 10_000;

/// Segment of children between `column-span: all` elements.
enum Segment {
    /// Normal children to be laid out across columns.
    Normal(Vec<Entity>),
    /// A `column-span: all` element that spans the full container width.
    Spanner(Entity),
}

/// Shared layout context for column fill functions, reducing argument counts.
#[derive(Clone, Copy)]
pub(crate) struct ColumnLayoutCtx {
    pub(crate) content_x: f32,
    pub(crate) content_y: f32,
    pub(crate) block_offset: f32,
    pub(crate) wm: WritingModeContext,
}

/// Lay out a multicol container.
///
/// CSS Multi-column Layout Level 1: computes column geometry, fills columns
/// (balanced or sequential), handles `column-span: all`, and attaches
/// [`MulticolInfo`] for column rule rendering.
#[allow(clippy::too_many_lines)]
pub fn layout_multicol(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
    layout_child: ChildLayoutFn,
) -> LayoutOutcome {
    let style = elidex_layout_block::get_style(dom, entity);
    let wm = WritingModeContext::new(style.writing_mode, style.direction);
    let is_horizontal = wm.is_horizontal();
    let font_db = input.font_db;
    let depth = input.depth;

    // --- Resolve box model ---
    let containing_inline = input.containing_inline_size;
    let padding = resolve_padding(&style, containing_inline);
    let border = sanitize_border(&style);
    let i_pb = elidex_layout_block::inline_pb(&wm, &padding, &border);

    let margin_top =
        elidex_layout_block::block::resolve_margin(style.margin_top, containing_inline);
    let margin_bottom =
        elidex_layout_block::block::resolve_margin(style.margin_bottom, containing_inline);
    let margin_left =
        elidex_layout_block::block::resolve_margin(style.margin_left, containing_inline);
    let margin_right =
        elidex_layout_block::block::resolve_margin(style.margin_right, containing_inline);

    let (margin_inline_start, margin_inline_end) = if is_horizontal {
        (margin_left, margin_right)
    } else {
        (margin_top, margin_bottom)
    };
    let (margin_block_start, margin_block_end) = if is_horizontal {
        (margin_top, margin_bottom)
    } else if wm.is_block_reversed() {
        (margin_right, margin_left)
    } else {
        (margin_left, margin_right)
    };

    let available_inline = if is_horizontal {
        input.containing_width
    } else {
        input.containing_height.unwrap_or(input.containing_width)
    };

    // --- Resolve inline-size ---
    let inline_size_dim = if is_horizontal {
        style.width
    } else {
        style.height
    };
    let inline_extra = margin_inline_start + margin_inline_end + i_pb;
    let auto_inline = (available_inline - inline_extra).max(0.0);
    let mut content_inline =
        resolve_dimension_value(inline_size_dim, available_inline, auto_inline).max(0.0);
    // box-sizing: border-box — subtract inline p+b from specified inline-size.
    if style.box_sizing == BoxSizing::BorderBox {
        if let Dimension::Length(_) | Dimension::Percentage(_) = inline_size_dim {
            content_inline = (content_inline - i_pb).max(0.0);
        }
    }
    // Apply min/max inline-size constraints.
    let (min_inline_dim, max_inline_dim) = if is_horizontal {
        (style.min_width, style.max_width)
    } else {
        (style.min_height, style.max_height)
    };
    {
        let mut min_i =
            elidex_layout_block::resolve_min_max(min_inline_dim, containing_inline, 0.0);
        let mut max_i =
            elidex_layout_block::resolve_min_max(max_inline_dim, containing_inline, f32::INFINITY);
        if style.box_sizing == BoxSizing::BorderBox {
            adjust_min_max_for_border_box(&mut min_i, &mut max_i, i_pb);
        }
        content_inline = clamp_min_max(content_inline, min_i, max_i);
    }

    // --- Content position ---
    let (content_x, content_y) = if is_horizontal {
        (
            input.offset_x + margin_inline_start + border.left + padding.left,
            input.offset_y + margin_block_start + border.top + padding.top,
        )
    } else {
        let l_padding = elidex_plugin::LogicalEdges::from_physical(padding, wm);
        let l_border = elidex_plugin::LogicalEdges::from_physical(border, wm);
        (
            input.offset_x + margin_block_start + l_border.block_start + l_padding.block_start,
            input.offset_y + margin_inline_start + l_border.inline_start + l_padding.inline_start,
        )
    };

    // --- Column geometry ---
    let column_gap = resolve_dimension_value(style.column_gap, content_inline, 0.0).max(0.0);
    let geom = compute_column_geometry(
        content_inline,
        style.column_count,
        style.column_width,
        column_gap,
    );

    // --- Height determination ---
    let containing_block = if is_horizontal {
        input.containing_height
    } else {
        Some(input.containing_width)
    };
    let definite_height =
        resolve_definite_block_size(&style, containing_block, &padding, &border, wm);

    // Resolve max-block-size for balanced fill upper bound.
    let max_block_dim = if is_horizontal {
        style.max_height
    } else {
        style.max_width
    };
    let containing_for_minmax = if is_horizontal {
        input.containing_height.unwrap_or(0.0)
    } else {
        input.containing_width
    };
    let max_block_size =
        elidex_layout_block::resolve_min_max(max_block_dim, containing_for_minmax, f32::INFINITY);

    // --- Collect children and split into segments ---
    let children = composed_children_flat(dom, entity);
    let segments = split_segments(dom, &children);

    // Collect static positions for absolutely positioned children.
    let static_positions = elidex_layout_block::positioned::collect_abspos_static_positions(
        dom, &children, content_x, content_y,
    );

    // --- Lay out segments ---
    let mut block_cursor: f32 = 0.0;
    let mut multicol_segments: Vec<(u32, f32, f32)> = Vec::new();

    for segment in &segments {
        match segment {
            Segment::Normal(seg_children) => {
                if seg_children.is_empty() {
                    continue;
                }
                let col_ctx = ColumnLayoutCtx {
                    content_x,
                    content_y,
                    block_offset: block_cursor,
                    wm,
                };
                let balance_max = if max_block_size.is_finite() {
                    Some(max_block_size)
                } else {
                    None
                };
                let (frags, col_height) = layout_normal_segment(
                    dom,
                    seg_children,
                    input,
                    &geom,
                    definite_height,
                    balance_max,
                    &style,
                    &col_ctx,
                    layout_child,
                    entity,
                );

                // Position column fragments.
                #[allow(clippy::cast_possible_truncation)]
                let actual_count = frags.len().min(u32::MAX as usize) as u32;
                if actual_count > 0 {
                    multicol_segments.push((actual_count, block_cursor, col_height));
                }
                block_cursor += col_height;
            }
            Segment::Spanner(spanner_entity) => {
                // Lay out spanner at full container width.
                let (spanner_x, spanner_y) = if is_horizontal {
                    (content_x, content_y + block_cursor)
                } else {
                    (content_x + block_cursor, content_y)
                };
                let spanner_input = LayoutInput {
                    containing_width: if is_horizontal {
                        content_inline
                    } else {
                        input.containing_width
                    },
                    containing_height: if is_horizontal {
                        input.containing_height
                    } else {
                        Some(content_inline)
                    },
                    containing_inline_size: content_inline,
                    offset_x: spanner_x,
                    offset_y: spanner_y,
                    font_db: input.font_db,
                    depth: input.depth + 1,
                    float_ctx: None,
                    viewport: input.viewport,
                    fragmentainer: None,
                    break_token: None,
                    subgrid: None,
                };
                let outcome = layout_child(dom, *spanner_entity, &spanner_input);
                // Use margin_box() for spanner block extent.
                let mb = outcome.layout_box.margin_box();
                let spanner_block_extent = if is_horizontal { mb.height } else { mb.width };
                block_cursor += spanner_block_extent;
            }
        }
    }

    // --- Resolve final block size ---
    let content_block = definite_height.unwrap_or(block_cursor);

    // Apply min/max block-size constraints.
    let min_block_dim = if is_horizontal {
        style.min_height
    } else {
        style.min_width
    };
    let min_block = elidex_layout_block::resolve_min_max(min_block_dim, containing_for_minmax, 0.0);
    let final_block = clamp_min_max(content_block, min_block, max_block_size);

    // --- Build LayoutBox ---
    let (phys_width, phys_height) = if is_horizontal {
        (content_inline, final_block)
    } else {
        (final_block, content_inline)
    };

    let margin = if is_horizontal {
        EdgeSizes::new(
            margin_block_start,
            margin_inline_end,
            margin_block_end,
            margin_inline_start,
        )
    } else {
        let (phys_left, phys_right) = if wm.is_block_reversed() {
            (margin_block_end, margin_block_start)
        } else {
            (margin_block_start, margin_block_end)
        };
        EdgeSizes::new(
            margin_inline_start,
            phys_right,
            margin_inline_end,
            phys_left,
        )
    };

    let lb = LayoutBox {
        content: Rect::new(content_x, content_y, phys_width, phys_height),
        padding,
        border,
        margin,
        first_baseline: None,
    };

    let _ = dom.world_mut().insert_one(entity, lb.clone());

    // Attach MulticolInfo for column rule rendering.
    let info = MulticolInfo {
        column_width: geom.width,
        column_gap: geom.gap,
        writing_mode: style.writing_mode,
        segments: multicol_segments,
    };
    let _ = dom.world_mut().insert_one(entity, info);

    // --- Layout positioned descendants ---
    let is_root = dom.get_parent(entity).is_none();
    let is_cb = style.position != Position::Static || is_root || style.has_transform;
    if is_cb {
        let pb = lb.padding_box();
        elidex_layout_block::positioned::layout_positioned_children(
            dom,
            entity,
            &pb,
            input.viewport,
            &static_positions,
            font_db,
            layout_child,
            depth,
        );
    }

    lb.into()
}

/// Lay out a normal (non-spanner) segment across columns.
///
/// Returns the column fragments and the resolved column height.
#[allow(clippy::too_many_arguments)]
fn layout_normal_segment(
    dom: &mut EcsDom,
    children: &[Entity],
    input: &LayoutInput<'_>,
    geom: &ColumnGeometry,
    definite_height: Option<f32>,
    balance_max_height: Option<f32>,
    style: &ComputedStyle,
    col_ctx: &ColumnLayoutCtx,
    layout_child: ChildLayoutFn,
    parent_entity: Entity,
) -> (Vec<fill::ColumnFragment>, f32) {
    let fill_and_position =
        |dom: &mut EcsDom, frags: Vec<fill::ColumnFragment>| -> Vec<fill::ColumnFragment> {
            position_column_fragments(dom, &frags, geom, col_ctx.wm);
            frags
        };

    match (definite_height, style.column_fill) {
        // Definite height + auto fill → sequential
        (Some(h), ColumnFill::Auto) => {
            let frags = fill_columns_sequential(
                dom,
                children,
                input,
                geom,
                h,
                u32::MAX,
                layout_child,
                parent_entity,
                col_ctx,
            );
            let frags = fill_and_position(dom, frags);
            (frags, h)
        }
        // Definite height + balance → balanced with max_height
        (Some(h), ColumnFill::Balance) => {
            // Use min(definite_height, max-block-size) as balance upper bound.
            let max_h = balance_max_height.map_or(h, |m| h.min(m));
            let (frags, col_height) = fill_columns_balanced(
                dom,
                children,
                input,
                geom,
                Some(max_h),
                layout_child,
                parent_entity,
                col_ctx,
            );
            let frags = fill_and_position(dom, frags);
            (frags, col_height.min(h))
        }
        // Auto height → always balance (CSS Multi-column L1 §7)
        (None, _) => {
            let (frags, col_height) = fill_columns_balanced(
                dom,
                children,
                input,
                geom,
                balance_max_height,
                layout_child,
                parent_entity,
                col_ctx,
            );
            let frags = fill_and_position(dom, frags);
            (frags, col_height)
        }
    }
}

/// Position column fragment children by shifting them to their column's
/// physical position (writing-mode aware).
fn position_column_fragments(
    dom: &mut EcsDom,
    frags: &[fill::ColumnFragment],
    geom: &ColumnGeometry,
    wm: WritingModeContext,
) {
    for (i, frag) in frags.iter().enumerate() {
        if i == 0 {
            continue; // Column 0 has no inline offset.
        }
        #[allow(clippy::cast_precision_loss)]
        let inline_offset = i as f32 * (geom.width + geom.gap);
        let (dx, dy) = if wm.is_horizontal() {
            (inline_offset, 0.0)
        } else {
            (0.0, inline_offset)
        };

        // Shift all children in this column fragment.
        for &child in &frag.children {
            shift_entity_and_descendants(dom, child, dx, dy);
        }
    }
}

/// Shift an entity and all its descendants by `(dx, dy)`.
///
/// Depth is capped at [`MAX_DESCENDANT_DEPTH`] to prevent infinite loops.
fn shift_entity_and_descendants(dom: &mut EcsDom, entity: Entity, dx: f32, dy: f32) {
    if let Ok(mut lb) = dom.world_mut().get::<&mut LayoutBox>(entity) {
        lb.content.x += dx;
        lb.content.y += dy;
    }
    let mut stack: Vec<(Entity, usize)> = dom.children_iter(entity).map(|c| (c, 0)).collect();
    while let Some((e, depth)) = stack.pop() {
        if depth >= MAX_DESCENDANT_DEPTH {
            break;
        }
        if let Ok(mut lb) = dom.world_mut().get::<&mut LayoutBox>(e) {
            lb.content.x += dx;
            lb.content.y += dy;
        }
        for child in dom.children_iter(e) {
            stack.push((child, depth + 1));
        }
    }
}

/// Split children into segments around `column-span: all` elements.
///
/// CSS Multi-column L1 §5: `column-span: all` only applies to in-flow
/// block-level elements. Out-of-flow and inline-level are excluded.
///
/// Absolutely/fixedly positioned children are excluded from column content
/// entirely — they are laid out separately via `layout_positioned_children`.
fn split_segments(dom: &EcsDom, children: &[Entity]) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut current_normal: Vec<Entity> = Vec::new();

    for &child in children {
        let child_style = dom.world().get::<&ComputedStyle>(child).ok();

        // Skip display:none children — they generate no boxes.
        if child_style
            .as_ref()
            .is_some_and(|s| s.display == Display::None)
        {
            continue;
        }

        // Skip absolutely/fixedly positioned children — they're out-of-flow.
        let is_out_of_flow = child_style
            .as_ref()
            .is_some_and(|s| matches!(s.position, Position::Absolute | Position::Fixed));
        if is_out_of_flow {
            continue;
        }

        let is_spanner = child_style.is_some_and(|style| {
            style.column_span == ColumnSpan::All
                && style.display != Display::Inline
                && style.float == Float::None
        });

        if is_spanner {
            if !current_normal.is_empty() {
                segments.push(Segment::Normal(std::mem::take(&mut current_normal)));
            }
            segments.push(Segment::Spanner(child));
        } else {
            current_normal.push(child);
        }
    }

    if !current_normal.is_empty() {
        segments.push(Segment::Normal(current_normal));
    }

    segments
}

/// Resolve a definite block-size for the multicol container.
///
/// Returns `Some(content_height)` for explicit `height`/`width` (depending on
/// writing mode), `None` for `auto`.
fn resolve_definite_block_size(
    style: &ComputedStyle,
    containing_block: Option<f32>,
    padding: &EdgeSizes,
    border: &EdgeSizes,
    wm: WritingModeContext,
) -> Option<f32> {
    let block_size_dim = if wm.is_horizontal() {
        style.height
    } else {
        style.width
    };
    let b_pb = elidex_layout_block::block_pb(&wm, padding, border);

    match block_size_dim {
        Dimension::Length(px) if px.is_finite() => {
            if style.box_sizing == BoxSizing::BorderBox {
                Some((px - b_pb).max(0.0))
            } else {
                Some(px)
            }
        }
        Dimension::Percentage(pct) => containing_block.map(|cb| {
            let resolved = cb * pct / 100.0;
            if style.box_sizing == BoxSizing::BorderBox {
                (resolved - b_pb).max(0.0)
            } else {
                resolved
            }
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
