//! Align-content and justify-content offset computation (CSS Flexbox §9.4, §9.5).

use elidex_layout_block::total_gap;
use elidex_plugin::{AlignContent, AlignmentSafety, FlexWrap, JustifyContent};

/// Apply safety fallback for justify-content.
///
/// CSS Box Alignment L3 §5.4: when `safe` is specified and free space is negative,
/// the alignment falls back to `flex-start`.
pub(crate) fn apply_justify_safety(
    justify: JustifyContent,
    free_space: f32,
    safety: AlignmentSafety,
) -> JustifyContent {
    if safety == AlignmentSafety::Safe && free_space < 0.0 {
        JustifyContent::FlexStart
    } else {
        justify
    }
}

/// Apply safety fallback for align-content.
pub(crate) fn apply_align_content_safety(
    align_content: AlignContent,
    container_cross: f32,
    line_cross_sizes: &[f32],
    gap_cross: f32,
    safety: AlignmentSafety,
) -> AlignContent {
    if safety == AlignmentSafety::Safe {
        let total: f32 = line_cross_sizes.iter().sum();
        let total_gap = total_gap(line_cross_sizes.len(), gap_cross);
        let free = container_cross - total - total_gap;
        if free < 0.0 {
            return AlignContent::FlexStart;
        }
    }
    align_content
}

/// Compute justify-content start offset and gap.
#[allow(clippy::cast_precision_loss)] // item counts are small
pub(crate) fn compute_justify_offsets(
    justify: JustifyContent,
    free_space: f32,
    count: usize,
) -> (f32, f32) {
    if count == 0 {
        return (0.0, 0.0);
    }
    let n = count as f32;
    match justify {
        JustifyContent::FlexStart | JustifyContent::Stretch | JustifyContent::Normal => (0.0, 0.0),
        JustifyContent::FlexEnd => (free_space, 0.0),
        JustifyContent::Center => (free_space / 2.0, 0.0),
        JustifyContent::SpaceBetween => {
            if count <= 1 {
                (0.0, 0.0)
            } else {
                (0.0, free_space / (n - 1.0))
            }
        }
        JustifyContent::SpaceAround => {
            let gap = free_space / n;
            (gap / 2.0, gap)
        }
        JustifyContent::SpaceEvenly => {
            let gap = free_space / (n + 1.0);
            (gap, gap)
        }
    }
}

/// Result of align-content distribution.
pub(crate) struct AlignContentResult {
    /// Starting cross offset for each line.
    pub(crate) offsets: Vec<f32>,
    /// Effective cross sizes for each line (may be increased by stretch).
    pub(crate) effective_line_sizes: Vec<f32>,
}

#[allow(clippy::cast_precision_loss)] // line counts are small
pub(crate) fn compute_align_content_offsets(
    line_cross_sizes: &[f32],
    container_cross: f32,
    align_content: AlignContent,
    wrap: FlexWrap,
    gap_cross: f32,
) -> AlignContentResult {
    let n = line_cross_sizes.len();
    if n == 0 {
        return AlignContentResult {
            offsets: Vec::new(),
            effective_line_sizes: Vec::new(),
        };
    }
    if matches!(wrap, FlexWrap::Nowrap) {
        return AlignContentResult {
            offsets: vec![0.0],
            effective_line_sizes: line_cross_sizes.to_vec(),
        };
    }

    let total: f32 = line_cross_sizes.iter().sum();
    let total_cross_gap = total_gap(n, gap_cross);
    let free = (container_cross - total - total_cross_gap).max(0.0);
    let nf = n as f32;

    let mut cursor = match align_content {
        AlignContent::FlexEnd => free,
        AlignContent::Center => free / 2.0,
        AlignContent::SpaceAround => free / (2.0 * nf),
        AlignContent::SpaceEvenly => free / (nf + 1.0),
        AlignContent::FlexStart
        | AlignContent::SpaceBetween
        | AlignContent::Stretch
        | AlignContent::Normal => 0.0,
    };

    let gap = match align_content {
        AlignContent::SpaceBetween => {
            if n <= 1 {
                0.0
            } else {
                free / (nf - 1.0)
            }
        }
        AlignContent::SpaceAround => free / nf,
        AlignContent::SpaceEvenly => free / (nf + 1.0),
        _ => 0.0,
    };

    let stretch_extra = if matches!(align_content, AlignContent::Stretch | AlignContent::Normal) {
        free / nf
    } else {
        0.0
    };

    let mut offsets = Vec::with_capacity(n);
    let mut effective_line_sizes = Vec::with_capacity(n);
    for (i, &line_size) in line_cross_sizes.iter().enumerate() {
        offsets.push(cursor);
        effective_line_sizes.push(line_size + stretch_extra);
        cursor += line_size + stretch_extra;
        if i < n - 1 {
            cursor += gap + gap_cross;
        }
    }
    AlignContentResult {
        offsets,
        effective_line_sizes,
    }
}
