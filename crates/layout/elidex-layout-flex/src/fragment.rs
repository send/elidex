//! CSS Flexbox Level 1 §12: Fragmentation (row and column flex).

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::block::fragmentation::{
    find_best_break, is_avoid_break_inside, is_avoid_break_value, is_forced_break, BreakCandidate,
    BreakClass,
};
use elidex_layout_block::{get_intrinsic_size, BreakToken, BreakTokenData};

use super::{FlexContext, FlexItem, FlexLineInfo};

use elidex_layout_block::block::fragmentation::is_monolithic;

/// CSS Flexbox Level 1 §12: Fragmenting Flex Layout.
///
/// For **row flex** (`horizontal == true`), fragmentation happens between flex
/// lines along the cross axis (which is the block axis in `horizontal-tb`).
///
/// For **column flex** (`horizontal == false`), the main axis is vertical
/// (block direction in `horizontal-tb`), so fragmentation happens between
/// items within each line along the main axis.
///
/// **Writing mode limitation**: In vertical writing modes (`vertical-rl`,
/// `vertical-lr`), the block direction is horizontal. This function currently
/// assumes `horizontal-tb` mapping (row→cross=block, column→main=block).
/// Vertical writing mode interaction with flex fragmentation direction is not
/// yet handled and may produce incorrect results.
///
/// Returns `(break_token, propagated_break_before, propagated_break_after)`.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(crate) fn compute_flex_fragmentation(
    dom: &EcsDom,
    entity: Entity,
    items: &[FlexItem],
    line_ranges: &[(usize, usize)],
    line_cross_sizes: &[f32],
    ctx: &FlexContext,
    frag: elidex_layout_block::FragmentainerContext,
    resume_line: usize,
    resume_item: usize,
    _resume_child_bt: Option<Box<BreakToken>>,
) -> (
    Option<BreakToken>,
    Option<elidex_plugin::BreakValue>,
    Option<elidex_plugin::BreakValue>,
) {
    let frag_type = frag.fragmentation_type;

    // Propagated breaks from first/last items across all lines.
    let propagated_before = line_ranges.first().and_then(|&(s, e)| {
        if s < e {
            let first_style = elidex_layout_block::try_get_style(dom, items[s].entity)?;
            if is_forced_break(first_style.break_before, frag_type) {
                Some(first_style.break_before)
            } else {
                None
            }
        } else {
            None
        }
    });

    let propagated_after = line_ranges.last().and_then(|&(s, e)| {
        if s < e {
            let last_style = elidex_layout_block::try_get_style(dom, items[e - 1].entity)?;
            if is_forced_break(last_style.break_after, frag_type) {
                Some(last_style.break_after)
            } else {
                None
            }
        } else {
            None
        }
    });

    let line_info = FlexLineInfo {
        items,
        line_ranges,
        line_cross_sizes,
    };

    let break_token = if ctx.horizontal {
        // Row flex: fragment between lines along cross axis (block direction).
        fragment_row_flex(dom, entity, &line_info, ctx, frag, resume_line)
    } else {
        // Column flex: fragment between items within lines along main axis (block direction).
        fragment_column_flex(dom, entity, &line_info, ctx, frag, resume_line, resume_item)
    };

    (break_token, propagated_before, propagated_after)
}

/// Row flex fragmentation: break between flex lines along the cross axis.
#[allow(clippy::too_many_lines)]
fn fragment_row_flex(
    dom: &EcsDom,
    entity: Entity,
    info: &FlexLineInfo<'_>,
    ctx: &FlexContext,
    frag: elidex_layout_block::FragmentainerContext,
    resume_line: usize,
) -> Option<BreakToken> {
    let frag_type = frag.fragmentation_type;
    let available = frag.available_block_size;
    let items = info.items;
    let line_ranges = info.line_ranges;
    let line_cross_sizes = info.line_cross_sizes;

    let mut consumed: f32 = 0.0;

    // Account for lines before resume_line (already consumed in a prior fragment).
    for line_idx in 0..resume_line.min(line_ranges.len()) {
        consumed += line_cross_sizes.get(line_idx).copied().unwrap_or(0.0);
        if line_idx + 1 < line_ranges.len() {
            consumed += ctx.gap_cross;
        }
    }

    let mut candidates: Vec<BreakCandidate> = Vec::new();

    for line_idx in resume_line..line_ranges.len() {
        let &(start, end) = &line_ranges[line_idx];
        let line_cross = line_cross_sizes.get(line_idx).copied().unwrap_or(0.0);

        // Skip zero-height lines — they don't consume space or create break opportunities.
        if line_cross <= 0.0 {
            continue;
        }

        // Check forced break-before on the first item of this line (if not the first line).
        if line_idx > resume_line && start < end {
            if let Some(first_style) = elidex_layout_block::try_get_style(dom, items[start].entity)
            {
                if is_forced_break(first_style.break_before, frag_type) {
                    let bt = BreakToken {
                        entity,
                        consumed_block_size: consumed,
                        child_break_token: None,
                        mode_data: Some(BreakTokenData::Flex {
                            line_index: line_idx,
                            item_index: 0,
                            child_break_token: None,
                        }),
                    };
                    return Some(bt);
                }
            }
        }

        // Check forced break-after on the last item of the previous line.
        if line_idx > resume_line && line_idx > 0 {
            let prev = &line_ranges[line_idx - 1];
            if prev.1 > prev.0 {
                if let Some(last_style) =
                    elidex_layout_block::try_get_style(dom, items[prev.1 - 1].entity)
                {
                    if is_forced_break(last_style.break_after, frag_type) {
                        let bt = BreakToken {
                            entity,
                            consumed_block_size: consumed,
                            child_break_token: None,
                            mode_data: Some(BreakTokenData::Flex {
                                line_index: line_idx,
                                item_index: 0,
                                child_break_token: None,
                            }),
                        };
                        return Some(bt);
                    }
                }
            }
        }

        // Record a break candidate between lines (Class A break opportunity).
        if line_idx > resume_line {
            let violates_avoid =
                check_avoid_between_lines(dom, items, line_ranges, line_idx, frag_type);
            candidates.push(BreakCandidate {
                child_index: line_idx,
                class: BreakClass::A,
                cursor_block: consumed,
                violates_avoid,
                orphan_widow_penalty: false,
            });
        }

        // Add this line's cross size to consumed.
        consumed += line_cross;

        // Check if consumed exceeds available space (need to break).
        if consumed > available && !candidates.is_empty() {
            // Check if the current line is monolithic.
            let line_monolithic = items[start..end].iter().all(|item| {
                let s = elidex_layout_block::try_get_style(dom, item.entity);
                s.is_some_and(|st| {
                    is_monolithic(&st, get_intrinsic_size(dom, item.entity).is_some())
                })
            });
            if line_monolithic {
                // Monolithic line that overflows — can't break within it, continue.
                continue;
            }

            if let Some(best_idx) = find_best_break(&candidates, available) {
                let break_line = candidates[best_idx].child_index;
                let consumed_at_break = candidates[best_idx].cursor_block;
                let bt = BreakToken {
                    entity,
                    consumed_block_size: consumed_at_break,
                    child_break_token: None,
                    mode_data: Some(BreakTokenData::Flex {
                        line_index: break_line,
                        item_index: 0,
                        child_break_token: None,
                    }),
                };
                return Some(bt);
            }
        }

        // Add cross-axis gap after this line (before the next line).
        if line_idx + 1 < line_ranges.len() {
            consumed += ctx.gap_cross;
        }
    }

    // All lines fit — no break needed.
    None
}

/// Column flex fragmentation: break between items within lines along the main axis.
///
/// For `flex-direction: column`, the main axis is vertical (block direction in
/// `horizontal-tb`). Each item in a line is like a block sibling — we accumulate
/// `item.final_main` and break between items when the consumed size exceeds the
/// available block size.
#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
fn fragment_column_flex(
    dom: &EcsDom,
    entity: Entity,
    info: &FlexLineInfo<'_>,
    ctx: &FlexContext,
    frag: elidex_layout_block::FragmentainerContext,
    resume_line: usize,
    resume_item: usize,
) -> Option<BreakToken> {
    let frag_type = frag.fragmentation_type;
    let available = frag.available_block_size;
    let items = info.items;
    let line_ranges = info.line_ranges;

    // Walk each line, and within each line walk items along the main axis.
    // For column flex, lines are arranged along the cross axis (horizontal),
    // but fragmentation is along the main axis (vertical = block direction).
    // Each line is independently fragmented along the main axis.
    for line_idx in resume_line..line_ranges.len() {
        let &(start, end) = &line_ranges[line_idx];
        if start >= end {
            continue;
        }

        // Determine the first item index within this line to process.
        let first_item_in_line = if line_idx == resume_line {
            resume_item
        } else {
            0
        };

        let mut consumed: f32 = 0.0;

        // Account for items before first_item_in_line (already consumed).
        for item_rel in 0..first_item_in_line.min(end - start) {
            let item = &items[start + item_rel];
            consumed += item.final_main + item.margin_main;
            if item_rel + 1 < end - start {
                consumed += ctx.gap_main;
            }
        }

        let mut candidates: Vec<BreakCandidate> = Vec::new();

        for item_rel in first_item_in_line..(end - start) {
            let item_abs = start + item_rel;
            let item = &items[item_abs];
            let item_size = item.final_main + item.margin_main;

            // Skip zero-height items — they don't consume space.
            if item_size <= 0.0 {
                continue;
            }

            // Check forced break-before on this item (if not the first item).
            if item_rel > first_item_in_line {
                if let Some(st) = elidex_layout_block::try_get_style(dom, item.entity) {
                    if is_forced_break(st.break_before, frag_type) {
                        let bt = BreakToken {
                            entity,
                            consumed_block_size: consumed,
                            child_break_token: None,
                            mode_data: Some(BreakTokenData::Flex {
                                line_index: line_idx,
                                item_index: item_rel,
                                child_break_token: None,
                            }),
                        };
                        return Some(bt);
                    }
                }
            }

            // Check forced break-after on the previous item.
            if item_rel > first_item_in_line && item_abs > 0 {
                if let Some(prev_st) =
                    elidex_layout_block::try_get_style(dom, items[item_abs - 1].entity)
                {
                    if is_forced_break(prev_st.break_after, frag_type) {
                        let bt = BreakToken {
                            entity,
                            consumed_block_size: consumed,
                            child_break_token: None,
                            mode_data: Some(BreakTokenData::Flex {
                                line_index: line_idx,
                                item_index: item_rel,
                                child_break_token: None,
                            }),
                        };
                        return Some(bt);
                    }
                }
            }

            // Record a break candidate between items (Class A break opportunity).
            if item_rel > first_item_in_line {
                let violates_avoid =
                    check_avoid_between_column_items(dom, items, item_abs, frag_type);
                candidates.push(BreakCandidate {
                    child_index: item_rel,
                    class: BreakClass::A,
                    cursor_block: consumed,
                    violates_avoid,
                    orphan_widow_penalty: false,
                });
            }

            // Add this item's main-axis size to consumed.
            consumed += item_size;

            // Check if consumed exceeds available space.
            if consumed > available && !candidates.is_empty() {
                // Check if item is monolithic.
                let item_monolithic = elidex_layout_block::try_get_style(dom, item.entity)
                    .is_some_and(|st| {
                        is_monolithic(&st, get_intrinsic_size(dom, item.entity).is_some())
                    });
                if item_monolithic {
                    continue;
                }

                if let Some(best_idx) = find_best_break(&candidates, available) {
                    let break_item = candidates[best_idx].child_index;
                    let consumed_at_break = candidates[best_idx].cursor_block;
                    let bt = BreakToken {
                        entity,
                        consumed_block_size: consumed_at_break,
                        child_break_token: None,
                        mode_data: Some(BreakTokenData::Flex {
                            line_index: line_idx,
                            item_index: break_item,
                            child_break_token: None,
                        }),
                    };
                    return Some(bt);
                }
            }

            // Add main-axis gap after this item (before the next item).
            if item_rel + 1 < end - start {
                consumed += ctx.gap_main;
            }
        }
    }

    // All items fit — no break needed.
    None
}

/// Check whether breaking between column items at `item_abs` violates an avoid constraint.
fn check_avoid_between_column_items(
    dom: &EcsDom,
    items: &[FlexItem],
    item_abs: usize,
    frag_type: elidex_layout_block::FragmentationType,
) -> bool {
    // Check break-after on previous item.
    if item_abs > 0 {
        if let Some(prev_st) = elidex_layout_block::try_get_style(dom, items[item_abs - 1].entity) {
            if is_avoid_break_value(prev_st.break_after, frag_type) {
                return true;
            }
        }
    }

    // Check break-before on current item.
    if let Some(st) = elidex_layout_block::try_get_style(dom, items[item_abs].entity) {
        if is_avoid_break_value(st.break_before, frag_type) {
            return true;
        }
    }

    // Check break-inside on previous item.
    if item_abs > 0 {
        if let Some(prev_st) = elidex_layout_block::try_get_style(dom, items[item_abs - 1].entity) {
            if is_avoid_break_inside(prev_st.break_inside, frag_type) {
                return true;
            }
        }
    }

    false
}

/// Check whether breaking between lines at `line_idx` violates an avoid constraint.
///
/// Checks `break-after` on the last item of the previous line and `break-before`
/// on the first item of the current line, plus `break-inside` on the container.
fn check_avoid_between_lines(
    dom: &EcsDom,
    items: &[FlexItem],
    line_ranges: &[(usize, usize)],
    line_idx: usize,
    frag_type: elidex_layout_block::FragmentationType,
) -> bool {
    // Check break-inside on the container's parent style via items' styles.
    // Actually, the container's break-inside is checked by the caller (block layout).
    // Here we check the items at the boundary.

    // break-after on last item of previous line.
    if line_idx > 0 {
        let prev = &line_ranges[line_idx - 1];
        if prev.1 > prev.0 {
            if let Some(last_style) =
                elidex_layout_block::try_get_style(dom, items[prev.1 - 1].entity)
            {
                if is_avoid_break_value(last_style.break_after, frag_type) {
                    return true;
                }
            }
        }
    }

    // break-before on first item of current line.
    let &(start, end) = &line_ranges[line_idx];
    if start < end {
        if let Some(first_style) = elidex_layout_block::try_get_style(dom, items[start].entity) {
            if is_avoid_break_value(first_style.break_before, frag_type) {
                return true;
            }
        }
    }

    // Check break-inside: avoid on items spanning the boundary.
    // For flex, break-inside on the container is the relevant check,
    // but the container's style is handled by the parent fragmentainer.
    // We check break-inside on individual items that might be avoided.
    if line_idx > 0 {
        let prev = &line_ranges[line_idx - 1];
        for item in &items[prev.0..prev.1] {
            if let Some(st) = elidex_layout_block::try_get_style(dom, item.entity) {
                if is_avoid_break_inside(st.break_inside, frag_type) {
                    return true;
                }
            }
        }
    }

    false
}
