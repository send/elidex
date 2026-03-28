//! Table fragmentation logic (CSS 2.1 §17.5.4).

use elidex_ecs::EcsDom;
use elidex_ecs::Entity;
use elidex_layout_block::block::fragmentation::{
    find_best_break, is_avoid_break_value, is_forced_break, BreakCandidate, BreakClass,
};
use elidex_layout_block::{BreakToken, BreakTokenData};

use crate::grid::RowInfo;

/// CSS 2.1 §17.5.4: Fragmenting Table Layout.
///
/// When the table is inside a fragmentainer, row heights are checked against
/// the available block size. Between rows, break opportunities are evaluated
/// (forced breaks, avoid constraints). `thead`/`tfoot` row groups are repeated
/// in each continuation fragment, reducing the available space for body rows.
///
/// Returns `(break_token, propagated_break_before, propagated_break_after)`.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(crate) fn compute_table_fragmentation(
    dom: &EcsDom,
    entity: Entity,
    all_rows: &[RowInfo],
    row_heights: &[f32],
    spacing_v: f32,
    frag: elidex_layout_block::FragmentainerContext,
    resume_row: usize,
    thead_entity: Option<Entity>,
    tfoot_entity: Option<Entity>,
) -> (
    Option<BreakToken>,
    Option<elidex_plugin::BreakValue>,
    Option<elidex_plugin::BreakValue>,
) {
    let frag_type = frag.fragmentation_type;
    let mut available = frag.available_block_size;
    let num_rows = all_rows.len();

    // Propagated break-before from the first row.
    let propagated_before = all_rows.first().and_then(|ri| {
        let st = elidex_layout_block::try_get_style(dom, ri.entity)?;
        if is_forced_break(st.break_before, frag_type) {
            Some(st.break_before)
        } else {
            None
        }
    });

    // Propagated break-after from the last row.
    let propagated_after = all_rows.last().and_then(|ri| {
        let st = elidex_layout_block::try_get_style(dom, ri.entity)?;
        if is_forced_break(st.break_after, frag_type) {
            Some(st.break_after)
        } else {
            None
        }
    });

    if num_rows == 0 {
        return (None, propagated_before, propagated_after);
    }

    // In continuation fragments, reduce available space for thead/tfoot repetition.
    // The actual repeated layout is handled by the caller, but we account for the
    // space they consume when computing break points.
    let is_continuation = resume_row > 0;
    let mut thead_height = 0.0_f32;
    let mut tfoot_height = 0.0_f32;
    if is_continuation {
        // Estimate thead/tfoot height from row heights of their contained rows.
        if let Some(thead) = thead_entity {
            for (idx, ri) in all_rows.iter().enumerate() {
                if dom.get_parent(ri.entity) == Some(thead) {
                    thead_height += row_heights.get(idx).copied().unwrap_or(0.0) + spacing_v;
                }
            }
        }
        if let Some(tfoot) = tfoot_entity {
            for (idx, ri) in all_rows.iter().enumerate() {
                if dom.get_parent(ri.entity) == Some(tfoot) {
                    tfoot_height += row_heights.get(idx).copied().unwrap_or(0.0) + spacing_v;
                }
            }
        }
        available = (available - thead_height - tfoot_height).max(0.0);
    }

    // Accumulate consumed block size from body rows.
    let mut consumed: f32 = 0.0;

    // Account for rows before resume_row (already consumed in prior fragment).
    for row_idx in 0..resume_row.min(num_rows) {
        consumed += row_heights.get(row_idx).copied().unwrap_or(0.0);
        if row_idx + 1 < num_rows {
            consumed += spacing_v;
        }
    }

    let mut candidates: Vec<BreakCandidate> = Vec::new();

    for row_idx in resume_row..num_rows {
        let row_h = row_heights.get(row_idx).copied().unwrap_or(0.0);

        // Check forced break-before on this row (not the resume row).
        if row_idx > resume_row {
            if let Some(row_style) =
                elidex_layout_block::try_get_style(dom, all_rows[row_idx].entity)
            {
                if is_forced_break(row_style.break_before, frag_type) {
                    let bt = BreakToken {
                        entity,
                        consumed_block_size: consumed,
                        child_break_token: None,
                        mode_data: Some(BreakTokenData::Table {
                            row_index: row_idx,
                            thead_entity,
                            tfoot_entity,
                        }),
                    };
                    return (Some(bt), propagated_before, propagated_after);
                }
            }
        }

        // Check forced break-after on the previous row.
        if row_idx > resume_row && row_idx > 0 {
            if let Some(prev_style) =
                elidex_layout_block::try_get_style(dom, all_rows[row_idx - 1].entity)
            {
                if is_forced_break(prev_style.break_after, frag_type) {
                    let bt = BreakToken {
                        entity,
                        consumed_block_size: consumed,
                        child_break_token: None,
                        mode_data: Some(BreakTokenData::Table {
                            row_index: row_idx,
                            thead_entity,
                            tfoot_entity,
                        }),
                    };
                    return (Some(bt), propagated_before, propagated_after);
                }
            }
        }

        // Record a break candidate between rows.
        if row_idx > resume_row {
            let violates_avoid = check_avoid_between_table_rows(dom, all_rows, row_idx, frag_type);
            candidates.push(BreakCandidate {
                child_index: row_idx,
                class: BreakClass::A,
                cursor_block: consumed,
                violates_avoid,
                orphan_widow_penalty: false,
            });
        }

        // Add this row's height to consumed.
        consumed += row_h;

        // Check if consumed exceeds available space.
        if consumed > available && !candidates.is_empty() {
            if let Some(best_idx) = find_best_break(&candidates, available) {
                let break_row = candidates[best_idx].child_index;
                let consumed_at_break = candidates[best_idx].cursor_block;
                let bt = BreakToken {
                    entity,
                    consumed_block_size: consumed_at_break,
                    child_break_token: None,
                    mode_data: Some(BreakTokenData::Table {
                        row_index: break_row,
                        thead_entity,
                        tfoot_entity,
                    }),
                };
                return (Some(bt), propagated_before, propagated_after);
            }
        }

        // Add row spacing after this row.
        if row_idx + 1 < num_rows {
            consumed += spacing_v;
        }
    }

    // All rows fit — no break needed.
    (None, propagated_before, propagated_after)
}

/// Check whether breaking between rows at `row_idx` violates an avoid constraint.
fn check_avoid_between_table_rows(
    dom: &EcsDom,
    all_rows: &[RowInfo],
    row_idx: usize,
    frag_type: elidex_layout_block::FragmentationType,
) -> bool {
    // break-after on the previous row.
    if row_idx > 0 {
        if let Some(st) = elidex_layout_block::try_get_style(dom, all_rows[row_idx - 1].entity) {
            if is_avoid_break_value(st.break_after, frag_type) {
                return true;
            }
        }
    }

    // break-before on the current row.
    if let Some(st) = elidex_layout_block::try_get_style(dom, all_rows[row_idx].entity) {
        if is_avoid_break_value(st.break_before, frag_type) {
            return true;
        }
    }

    false
}
