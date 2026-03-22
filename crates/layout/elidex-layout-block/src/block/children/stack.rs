//! Block child stacking along the block axis with margin collapse.

use std::cell::RefCell;
use std::collections::HashMap;

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{BreakValue, Clear, Display, Float, Point, WritingModeContext};

use crate::{BreakToken, BreakTokenData, LayoutInput};

use super::super::float::FloatContext;
use super::super::fragmentation;
use super::super::is_block_level;
use super::super::margin::{collapse_margins, resolve_margin};
use super::helpers::{flush_inline_run, layout_float};
use super::{make_block_break_token, StackResult};

/// Stack block-level children along the block axis with margin collapse.
///
/// Writing-mode-aware: the block axis is vertical in `horizontal-tb` and
/// horizontal in vertical writing modes. Children are stacked along the
/// block axis, with margin collapse applied on block-axis margins.
///
/// Shared by block children layout and document-root layout. Returns
/// the total block extent consumed and first/last child margin info for
/// parent-child collapse (CSS 2.1 §8.3.1).
///
/// Floated children (CSS 2.1 §9.5) are removed from normal flow and
/// placed via the float context. Cleared children advance past floats.
///
/// Consecutive non-block children (text nodes and inline elements) are
/// wrapped in anonymous block boxes per CSS 2.1 §9.2.1.1 — their inline
/// content is laid out via [`layout_inline_context`](crate::inline::layout_inline_context).
#[allow(clippy::too_many_lines)]
pub fn stack_block_children(
    dom: &mut EcsDom,
    children: &[Entity],
    input: &LayoutInput<'_>,
    layout_child: crate::ChildLayoutFn,
    is_bfc: bool,
    parent_entity: Entity,
) -> StackResult {
    let env = crate::LayoutEnv::from_input(input, layout_child);
    let parent_style = crate::get_style(dom, parent_entity);
    let wm = WritingModeContext::new(parent_style.writing_mode, parent_style.direction);
    let is_horizontal = wm.is_horizontal();

    // Block-axis cursor: tracks Y in horizontal-tb, X in vertical modes.
    let mut cursor_block = if is_horizontal {
        input.offset.y
    } else {
        input.offset.x
    };
    let cursor_block_origin = cursor_block;

    let mut prev_margin_block_end: Option<f32> = None;
    let mut first_child_margin_block_start: Option<f32> = None;
    let mut last_child_margin_block_end: Option<f32> = None;

    // CSS 2.1 §9.5: BFC-establishing elements create their own FloatContext.
    // FloatContext works in abstract 2D space: "containing_width" = inline-axis size,
    // "y" positions = block-axis positions.
    let inline_containing = input.containing_inline_size;
    let local_ctx = RefCell::new(FloatContext::new(inline_containing));
    let float_ctx: &RefCell<FloatContext> = if let Some(ancestor_ctx) = input.float_ctx {
        if is_bfc {
            &local_ctx
        } else {
            ancestor_ctx
        }
    } else {
        &local_ctx
    };
    let mut inline_run: Vec<Entity> = Vec::new();
    // Array index of the first child in the current inline run.
    // Used for break token child_index when an inline run is fragmented.
    let mut inline_run_start_idx: Option<usize> = None;
    let mut static_positions: HashMap<Entity, Point> = HashMap::new();
    let mut first_baseline: Option<f32> = None;

    // --- Fragmentation: resume from break token ---
    let (start_index, resume_child_break_token, resume_inline_break_line) =
        if let Some(bt) = input.break_token {
            if let Some(BreakTokenData::Block {
                child_index,
                inline_break_line,
            }) = &bt.mode_data
            {
                (
                    *child_index,
                    bt.child_break_token.as_deref(),
                    *inline_break_line,
                )
            } else {
                (0, None, None)
            }
        } else {
            (0, None, None)
        };
    let is_continuation = input.break_token.is_some();
    let frag_ctx = input.fragmentainer;

    // CSS Fragmentation L3 §3.1: margins not collapsed across a fragmentation break.
    // In continuation fragments, suppress collapse with "previous" content.
    if is_continuation {
        prev_margin_block_end = None;
        first_child_margin_block_start = Some(0.0);
    }

    debug_assert!(
        start_index <= children.len(),
        "BreakToken child_index ({start_index}) exceeds children count ({})",
        children.len()
    );

    // Break candidates for overflow-based break selection.
    let mut break_candidates: Vec<fragmentation::BreakCandidate> = Vec::new();
    let mut propagated_break_before: Option<BreakValue> = None;
    let mut propagated_break_after: Option<BreakValue> = None;
    let mut result_break_token: Option<BreakToken> = None;

    // Track previous block child's break-after for avoid penalty (CSS Frag §3.4).
    let mut prev_break_after: Option<BreakValue> = None;
    // Track whether we've seen any block children (for break candidate recording).
    let mut seen_block_child = false;

    for (idx, &child) in children.iter().enumerate() {
        // Skip children before the resume point.
        if idx < start_index {
            continue;
        }
        let child_style = crate::try_get_style(dom, child);

        // Skip display: none entirely.
        if child_style
            .as_ref()
            .is_some_and(|s| s.display == Display::None)
        {
            continue;
        }

        // CSS 2.1 §9.3.1/§9.6: absolutely positioned elements are removed from flow.
        // Record static position before skipping (CSS 2.1 §10.6.5).
        if child_style
            .as_ref()
            .is_some_and(crate::positioned::is_absolutely_positioned)
        {
            let static_pos = if is_horizontal {
                Point::new(input.offset.x, cursor_block)
            } else {
                Point::new(cursor_block, input.offset.y)
            };
            static_positions.insert(child, static_pos);
            continue;
        }

        let is_block = child_style
            .as_ref()
            .is_some_and(|s| is_block_level(s.display));

        if !is_block {
            // Text node or inline element — collect for anonymous block box.
            if inline_run.is_empty() {
                inline_run_start_idx = Some(idx);
            }
            inline_run.push(child);
            continue;
        }

        // Flush any pending inline run before this block child (CSS 2.1 §9.2.1.1).
        if !inline_run.is_empty() {
            let skip_lines = if inline_run_start_idx == Some(start_index) {
                resume_inline_break_line.unwrap_or(0)
            } else {
                0
            };
            let run_result = flush_inline_run(
                dom,
                &inline_run,
                parent_entity,
                input,
                wm,
                cursor_block,
                cursor_block_origin,
                &env,
                &mut static_positions,
                skip_lines,
            );
            if first_baseline.is_none() {
                first_baseline = run_result
                    .first_baseline
                    .map(|bl| (cursor_block - cursor_block_origin) + bl);
            }
            cursor_block += run_result.block_extent;
            // Anonymous block box has zero margins.
            if first_child_margin_block_start.is_none() {
                first_child_margin_block_start = Some(0.0);
            }
            prev_margin_block_end = Some(0.0);
            last_child_margin_block_end = Some(0.0);
            // Handle inline fragmentation break.
            if let Some(break_line) = run_result.break_after_line {
                result_break_token = Some(make_block_break_token(
                    parent_entity,
                    (cursor_block - cursor_block_origin).max(0.0),
                    inline_run_start_idx.unwrap_or(idx),
                    Some(break_line),
                    None,
                ));
                inline_run.clear();
                inline_run_start_idx = None;
                break;
            }
            inline_run.clear();
            inline_run_start_idx = None;
        }

        let child_style = child_style.unwrap();
        let child_float = child_style.float;
        let child_clear = child_style.clear;

        // --- Fragmentation: forced break-before (§3.1) ---
        if let Some(frag) = frag_ctx {
            let effective_break_before = child_style.break_before;
            if fragmentation::is_forced_break(effective_break_before, frag.fragmentation_type) {
                // First child's forced break-before propagates to parent (§3.2).
                if idx == start_index {
                    propagated_break_before = Some(effective_break_before);
                }
                result_break_token = Some(make_block_break_token(
                    parent_entity,
                    (cursor_block - cursor_block_origin).max(0.0),
                    idx,
                    None,
                    None,
                ));
                break;
            }
        }

        // Block-start margin of the child (for collapse).
        // CSS WM: block-start = top (horizontal), right (vertical-rl), left (vertical-lr).
        let child_margin_block_start_dim = if is_horizontal {
            child_style.margin_top
        } else if wm.is_block_reversed() {
            child_style.margin_right
        } else {
            child_style.margin_left
        };

        // --- Clear: advance past floats (CSS 2.1 §9.5.2) ---
        // FloatContext clear_y operates on block-axis positions.
        let has_clearance = if child_clear == Clear::None {
            false
        } else {
            let new_block = float_ctx.borrow().clear_y(child_clear, cursor_block);
            let cleared = new_block > cursor_block;
            cursor_block = new_block;
            cleared
        };

        // --- Floated children: out of normal flow (CSS 2.1 §9.5) ---
        // TODO(G9): Float fragmentation — push float to next fragmentainer if it
        // overflows. Requires fragmentainer-aware float placement (CSS Frag §3.3).
        if child_float != Float::None {
            layout_float(
                dom,
                child,
                child_float,
                float_ctx,
                input,
                wm,
                cursor_block,
                layout_child,
            );
            prev_break_after = Some(child_style.break_after);
            continue;
        }

        // Margin collapse between adjacent block siblings (CSS 2.1 §8.3.1).
        let child_margin_block_start =
            resolve_margin(child_margin_block_start_dim, input.containing_inline_size);
        if first_child_margin_block_start.is_none() && !has_clearance {
            first_child_margin_block_start = Some(child_margin_block_start);
        }
        if let Some(prev_mbe) = prev_margin_block_end {
            if !has_clearance {
                let collapsed = collapse_margins(prev_mbe, child_margin_block_start);
                cursor_block -= prev_mbe + child_margin_block_start - collapsed;
            }
        }

        // --- Fragmentation: record Class A break candidate between siblings ---
        if let Some(frag) = frag_ctx {
            if seen_block_child {
                // §3.4: penalty from parent's break-inside, this child's break-before,
                // AND the previous child's break-after.
                let violates_avoid = fragmentation::is_avoid_break_inside(
                    parent_style.break_inside,
                    frag.fragmentation_type,
                ) || fragmentation::is_avoid_break_value(
                    child_style.break_before,
                    frag.fragmentation_type,
                ) || prev_break_after.is_some_and(|ba| {
                    fragmentation::is_avoid_break_value(ba, frag.fragmentation_type)
                });
                break_candidates.push(fragmentation::BreakCandidate {
                    child_index: idx,
                    class: fragmentation::BreakClass::A,
                    cursor_block: (cursor_block - cursor_block_origin).max(0.0),
                    violates_avoid,
                    orphan_widow_penalty: false,
                });
            }
        }

        // --- Fragmentation: check monolithic overflow before layout ---
        if let Some(frag) = frag_ctx {
            let child_has_intrinsic = crate::get_intrinsic_size(dom, child).is_some();
            if fragmentation::is_monolithic(&child_style, child_has_intrinsic) {
                let consumed = (cursor_block - cursor_block_origin).max(0.0);
                // CSS Frag §4: if prior content exists and no remaining space,
                // defer monolithic child to next fragmentainer.
                if consumed > 0.0 && consumed >= frag.available_block_size {
                    result_break_token = Some(make_block_break_token(
                        parent_entity,
                        consumed,
                        idx,
                        None,
                        None,
                    ));
                    break;
                }
            }
        }

        // Dispatch child layout via callback.
        // Set the block-axis offset in the child input.
        // Pass child break token only to the first child being resumed.
        let child_bt = if idx == start_index {
            resume_child_break_token
        } else {
            None
        };
        let child_input = if is_horizontal {
            LayoutInput {
                offset: elidex_plugin::Point::new(input.offset.x, cursor_block),
                float_ctx: Some(float_ctx),
                break_token: child_bt,
                ..*input
            }
        } else {
            LayoutInput {
                offset: elidex_plugin::Point::new(cursor_block, input.offset.y),
                float_ctx: Some(float_ctx),
                break_token: child_bt,
                ..*input
            }
        };
        let child_outcome = layout_child(dom, child, &child_input);
        let child_box = child_outcome.layout_box;

        // --- Fragmentation: propagate child's break token upward ---
        // If the child itself was fragmented, produce a break at this child_index
        // wrapping the child's break token.
        if let Some(child_bt) = child_outcome.break_token {
            // Track propagated break-before from first child.
            if idx == start_index {
                if let Some(bp) = child_outcome.propagated_break_before {
                    propagated_break_before = Some(bp);
                }
            }
            // Note: propagated_break_after from a fragmented child is NOT propagated
            // here — the child hasn't finished, so its last-child break info is premature.
            result_break_token = Some(make_block_break_token(
                parent_entity,
                (cursor_block - cursor_block_origin).max(0.0) + child_bt.consumed_block_size,
                idx,
                None,
                Some(Box::new(child_bt)),
            ));
            break;
        }

        // Capture baseline from first in-flow block child (CSS 2.1 §10.8.1).
        if first_baseline.is_none() {
            if let Some(child_bl) = child_box.first_baseline {
                let child_block_pos = if is_horizontal {
                    child_box.content.origin.y
                } else {
                    child_box.content.origin.x
                };
                first_baseline = Some(child_block_pos - cursor_block_origin + child_bl);
            }
        }

        // Advance cursor by the child's block extent (margin box).
        let child_block_extent = if is_horizontal {
            child_box.margin_box().size.height
        } else {
            child_box.margin_box().size.width
        };
        cursor_block += child_block_extent;

        // --- Fragmentation: overflow detection ---
        if let Some(frag) = frag_ctx {
            let consumed = (cursor_block - cursor_block_origin).max(0.0);
            if consumed > frag.available_block_size {
                // Select best break from candidates.
                if let Some(best_idx) =
                    fragmentation::find_best_break(&break_candidates, frag.available_block_size)
                {
                    let best = &break_candidates[best_idx];
                    result_break_token = Some(make_block_break_token(
                        parent_entity,
                        best.cursor_block,
                        best.child_index,
                        None,
                        None,
                    ));
                    break;
                }
                // No candidates: break after current child (overflow accepted).
                result_break_token = Some(make_block_break_token(
                    parent_entity,
                    consumed,
                    idx + 1,
                    None,
                    None,
                ));
                break;
            }
        }

        // Track block-end margin for collapse with next sibling.
        let child_margin_block_end = if is_horizontal {
            child_box.margin.bottom
        } else if wm.is_block_reversed() {
            child_box.margin.left
        } else {
            child_box.margin.right
        };
        prev_margin_block_end = Some(child_margin_block_end);
        last_child_margin_block_end = Some(child_margin_block_end);

        // --- Fragmentation: forced break-after (§3.1) ---
        if let Some(frag) = frag_ctx {
            if fragmentation::is_forced_break(child_style.break_after, frag.fragmentation_type) {
                // §3.2: Only propagate break-after if this is the last content child.
                let remaining_has_content = children[idx + 1..].iter().any(|&c| {
                    crate::try_get_style(dom, c).is_none_or(|s| {
                        s.display != Display::None
                            && !crate::positioned::is_absolutely_positioned(&s)
                    })
                });
                if !remaining_has_content {
                    propagated_break_after = Some(child_style.break_after);
                }
                result_break_token = Some(make_block_break_token(
                    parent_entity,
                    (cursor_block - cursor_block_origin).max(0.0),
                    idx + 1,
                    None,
                    None,
                ));
                break;
            }
        }

        prev_break_after = Some(child_style.break_after);
        seen_block_child = true;
    }

    // Flush trailing inline run (CSS 2.1 §9.2.1.1) — only if no break was produced.
    if result_break_token.is_none() && !inline_run.is_empty() {
        let skip_lines = if inline_run_start_idx == Some(start_index) {
            resume_inline_break_line.unwrap_or(0)
        } else {
            0
        };
        let run_result = flush_inline_run(
            dom,
            &inline_run,
            parent_entity,
            input,
            wm,
            cursor_block,
            cursor_block_origin,
            &env,
            &mut static_positions,
            skip_lines,
        );
        if first_baseline.is_none() {
            first_baseline = run_result
                .first_baseline
                .map(|bl| (cursor_block - cursor_block_origin) + bl);
        }
        cursor_block += run_result.block_extent;
        if first_child_margin_block_start.is_none() {
            first_child_margin_block_start = Some(0.0);
        }
        last_child_margin_block_end = Some(0.0);
        // Handle inline fragmentation break from trailing run.
        if let Some(break_line) = run_result.break_after_line {
            result_break_token = Some(make_block_break_token(
                parent_entity,
                (cursor_block - cursor_block_origin).max(0.0),
                inline_run_start_idx.unwrap_or(children.len()),
                Some(break_line),
                None,
            ));
        }
    }

    // CSS Fragmentation L3 §3.1: margins not collapsed at fragment end.
    if result_break_token.is_some() {
        last_child_margin_block_end = Some(0.0);
    }

    // CSS 2.1 §10.6.7: Only elements that establish a BFC have their
    // block extent increased to contain floats.
    let normal_extent = cursor_block - cursor_block_origin;

    // H3: When fragmented, clamp height to the break point's consumed size
    // (the cursor may have advanced past the break point for overflow detection).
    let height = if let Some(ref bt) = result_break_token {
        bt.consumed_block_size.min(normal_extent)
    } else if is_bfc {
        let float_bottom = float_ctx.borrow().float_bottom();
        let float_extend = if float_bottom > 0.0 {
            (float_bottom - cursor_block_origin).max(0.0)
        } else {
            0.0
        };
        normal_extent.max(float_extend)
    } else {
        normal_extent
    };

    StackResult {
        height,
        first_child_margin_top: first_child_margin_block_start,
        last_child_margin_bottom: last_child_margin_block_end,
        static_positions,
        first_baseline,
        break_token: result_break_token,
        propagated_break_before,
        propagated_break_after,
    }
}
