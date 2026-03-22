//! Column filling algorithms: sequential and balanced.
//!
//! CSS Multi-column Layout Level 1 §7 (column-fill) and §8 (overflow columns).

use elidex_ecs::{EcsDom, Entity, ImageData};
use elidex_layout_block::block::stack_block_children;
use elidex_layout_block::{
    BreakToken, BreakTokenData, FragmentainerContext, FragmentationType, LayoutEnv, LayoutInput,
};
use elidex_plugin::{ComputedStyle, CssSize, LayoutBox, Overflow, Point, WritingModeContext};

use crate::algo::ColumnGeometry;
use crate::ColumnLayoutCtx;

/// Shared environment for column fill algorithms.
pub struct ColumnFillEnv<'a> {
    pub input: &'a LayoutInput<'a>,
    pub geom: &'a ColumnGeometry,
    pub env: &'a LayoutEnv<'a>,
    pub parent_entity: Entity,
    pub col_ctx: &'a ColumnLayoutCtx,
}

/// Result of laying out content into a single column.
#[derive(Clone, Debug)]
pub struct ColumnFragment {
    /// Entities laid out in this column (direct children).
    pub children: Vec<Entity>,
}

/// Maximum number of binary search iterations for balanced fill.
const MAX_BALANCE_ITERATIONS: u32 = 10;

/// Maximum number of overflow columns to prevent infinite loops.
const MAX_OVERFLOW_COLUMNS: u32 = 1000;

/// Extract `child_index` from a break token's mode data.
fn extract_child_index(token: Option<&BreakToken>) -> Option<usize> {
    token
        .and_then(|bt| bt.mode_data.as_ref())
        .and_then(|md| match md {
            BreakTokenData::Block { child_index, .. } => Some(*child_index),
            _ => None,
        })
}

/// Fill columns sequentially (column-fill: auto).
///
/// Each column is filled to `column_height` before overflowing to the next.
/// Overflow columns beyond `max_columns` are still created (CSS §8).
///
/// `max_columns` is capped at [`MAX_OVERFLOW_COLUMNS`] (1000) as a safety
/// limit. Pass `u32::MAX` for "unlimited" (overflow columns allowed up to
/// the cap), or `geom.count` to restrict to the computed column count
/// (used by balanced fill probes).
///
/// Uses `stack_block_children` with `FragmentainerContext` for proper
/// fragmentation. Break tokens are passed between columns so that
/// mid-child breaks (nested content) are correctly resumed.
pub fn fill_columns_sequential(
    dom: &mut EcsDom,
    children: &[Entity],
    env: &ColumnFillEnv<'_>,
    column_height: f32,
    max_columns: u32,
) -> Vec<ColumnFragment> {
    let mut fragments = Vec::new();
    let mut col_index: u32 = 0;
    let mut carry_break_token: Option<BreakToken> = None;

    // Safety limit to prevent infinite loops.
    let total_limit = max_columns.min(MAX_OVERFLOW_COLUMNS);

    loop {
        if col_index >= total_limit {
            break;
        }

        let frag_ctx = FragmentainerContext {
            available_block_size: column_height,
            fragmentation_type: FragmentationType::Column,
        };

        // Always lay out at column 0's position. position_column_fragments
        // in lib.rs shifts columns 1+ to their correct inline offset afterward.
        let col_pos = base_column_position(env.col_ctx);

        let col_input = build_column_input(
            env.input,
            env.geom,
            &frag_ctx,
            col_pos,
            env.col_ctx.wm,
            carry_break_token.as_ref(),
        );

        let result = stack_block_children(
            dom,
            children,
            &col_input,
            env.env.layout_child,
            true, // Each column establishes a BFC
            env.parent_entity,
        );

        // Determine which children ended up in this column.
        let next_child_idx =
            extract_child_index(result.break_token.as_ref()).unwrap_or(children.len());
        let prev_child_idx = extract_child_index(carry_break_token.as_ref()).unwrap_or(0);

        // Build the children list for this column fragment.
        //
        // Mid-child break (next == prev): the child is fragmented mid-way and
        // only partially laid out in this column. Include it so that
        // position_column_fragments can shift it with the rest of this column.
        //
        // When next > prev AND the child at next is mid-break, we do NOT
        // include it — the ECS model has one LayoutBox per entity, so the next
        // column's layout will overwrite it. Multi-fragment tracking is G11.
        let col_children = if next_child_idx == prev_child_idx
            && result.break_token.is_some()
            && next_child_idx < children.len()
        {
            // Mid-child break: only this partially-laid-out child.
            vec![children[next_child_idx]]
        } else {
            children[prev_child_idx.min(children.len())..next_child_idx.min(children.len())]
                .to_vec()
        };

        fragments.push(ColumnFragment {
            children: col_children,
        });

        carry_break_token = result.break_token;
        if carry_break_token.is_none() {
            break;
        }

        col_index += 1;
    }

    fragments
}

/// Fill columns with balanced heights (column-fill: balance).
///
/// Uses binary search to find the minimum column height that fits all
/// content into `geom.count` columns.
///
/// Returns `(fragments, resolved_column_height)`.
pub fn fill_columns_balanced(
    dom: &mut EcsDom,
    children: &[Entity],
    env: &ColumnFillEnv<'_>,
    max_height: Option<f32>,
) -> (Vec<ColumnFragment>, f32) {
    if children.is_empty() {
        return (Vec::new(), 0.0);
    }

    // Step 1: Unconstrained probe to get total content height.
    let total_height = probe_total_height(dom, children, env);

    if total_height <= 0.0 {
        return (Vec::new(), 0.0);
    }

    let tallest_monolithic = find_tallest_monolithic(dom, children, env.col_ctx.wm);

    // Step 2: Binary search bounds.
    #[allow(clippy::cast_precision_loss)]
    let mut low = (total_height / env.geom.count as f32)
        .max(tallest_monolithic)
        .max(1.0);
    let mut high = if let Some(max_h) = max_height {
        total_height.min(max_h)
    } else {
        total_height
    };

    if low > high {
        low = high;
    }

    // Step 3: Binary search — converge high toward the minimum fitting height.
    for _ in 0..MAX_BALANCE_ITERATIONS {
        if (high - low).abs() < 1.0 {
            break;
        }

        let mid = f32::midpoint(low, high);
        let frags = fill_columns_sequential(dom, children, env, mid, env.geom.count);

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let used_columns = frags.len() as u32;

        if used_columns <= env.geom.count {
            high = mid;
        } else {
            low = mid;
        }
    }

    // Final pass at converged `high` to produce the definitive fragments
    // and ensure children's LayoutBoxes match.
    let best_frags = fill_columns_sequential(dom, children, env, high, env.geom.count);

    (best_frags, high)
}

/// Probe total content height by laying out with infinite available block size.
fn probe_total_height(dom: &mut EcsDom, children: &[Entity], env: &ColumnFillEnv<'_>) -> f32 {
    let is_horizontal = env.col_ctx.wm.is_horizontal();
    let col_pos = base_column_position(env.col_ctx);

    let col_input = LayoutInput {
        containing: CssSize {
            width: if is_horizontal {
                env.geom.width
            } else {
                env.input.containing.width
            },
            height: if is_horizontal {
                None
            } else {
                Some(env.geom.width)
            },
        },
        offset: col_pos,
        viewport: env.input.viewport,
        ..LayoutInput::probe(env.env, env.geom.width)
    };

    let result = stack_block_children(
        dom,
        children,
        &col_input,
        env.env.layout_child,
        true,
        env.parent_entity,
    );
    result.height
}

/// Find the tallest monolithic child (lower bound for balanced column height).
///
/// Returns the largest block-axis extent among monolithic children.
/// Monolithic elements: overflow != visible, replaced elements (`ImageData`).
///
/// Note: absolute/fixed children are already excluded by `split_segments`
/// before reaching this function.
fn find_tallest_monolithic(dom: &EcsDom, children: &[Entity], wm: WritingModeContext) -> f32 {
    let is_horizontal = wm.is_horizontal();
    let mut tallest: f32 = 0.0;
    for &child in children {
        let is_monolithic = dom
            .world()
            .get::<&ComputedStyle>(child)
            .ok()
            .is_some_and(|style| {
                style.overflow_x != Overflow::Visible || style.overflow_y != Overflow::Visible
            })
            || dom.world().get::<&ImageData>(child).is_ok();

        if is_monolithic {
            if let Ok(lb) = dom.world().get::<&LayoutBox>(child) {
                let block_extent = if is_horizontal {
                    lb.content.size.height
                } else {
                    lb.content.size.width
                };
                tallest = tallest.max(block_extent);
            }
        }
    }
    tallest
}

/// Compute the base (column 0) physical position.
///
/// All columns are laid out at this position; `position_column_fragments`
/// in `lib.rs` shifts columns 1+ to their correct inline offset afterward.
fn base_column_position(ctx: &ColumnLayoutCtx) -> Point {
    if ctx.wm.is_horizontal() {
        Point::new(
            ctx.content_origin.x,
            ctx.content_origin.y + ctx.block_offset,
        )
    } else {
        Point::new(
            ctx.content_origin.x + ctx.block_offset,
            ctx.content_origin.y,
        )
    }
}

/// Build a `LayoutInput` for a single column.
fn build_column_input<'a>(
    input: &LayoutInput<'a>,
    geom: &ColumnGeometry,
    frag_ctx: &'a FragmentainerContext,
    col_pos: Point,
    wm: WritingModeContext,
    break_token: Option<&'a BreakToken>,
) -> LayoutInput<'a> {
    let is_horizontal = wm.is_horizontal();
    LayoutInput {
        containing: CssSize {
            width: if is_horizontal {
                geom.width
            } else {
                input.containing.width
            },
            // Column box is an independent formatting context (§3.1):
            // percentage heights resolve against the column height, not the parent's.
            height: if is_horizontal {
                Some(frag_ctx.available_block_size)
            } else {
                Some(geom.width)
            },
        },
        containing_inline_size: geom.width,
        offset: col_pos,
        font_db: input.font_db,
        depth: input.depth + 1,
        float_ctx: None, // Each column resets float context (§3)
        viewport: input.viewport,
        fragmentainer: Some(frag_ctx),
        break_token,
        subgrid: None,
        layout_generation: input.layout_generation,
    }
}
