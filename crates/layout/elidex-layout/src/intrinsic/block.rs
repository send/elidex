//! Block and flex intrinsic sizing.

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::{
    get_style, inline, resolve_dimension_value, total_gap, IntrinsicSizes, LayoutEnv, LayoutInput,
};
use elidex_plugin::{Display, FlexDirection, FlexWrap, WritingModeContext};

use super::compute_intrinsic_sizes;

/// Block intrinsic sizing.
///
/// Inline children → min/max content inline sizes.
/// Block children → max of each child's intrinsic sizes.
pub(super) fn compute_block_intrinsic(
    dom: &mut EcsDom,
    entity: Entity,
    children: &[Entity],
    env: &LayoutEnv<'_>,
) -> IntrinsicSizes {
    let style = get_style(dom, entity);
    let has_block_children = children.iter().any(|&c| {
        elidex_layout_block::try_get_style(dom, c)
            .is_some_and(|s| elidex_layout_block::block::is_block_level(s.display))
    });

    if has_block_children {
        // Block children: max of each child's intrinsic sizes.
        let mut min = 0.0_f32;
        let mut max = 0.0_f32;
        let deeper = env.deeper();
        for &child in children {
            let child_sizes = compute_intrinsic_sizes(dom, child, &deeper);
            min = min.max(child_sizes.min_content);
            max = max.max(child_sizes.max_content);
        }
        // Also check inline content mixed in.
        let inline_min =
            inline::min_content_inline_size(dom, children, &style, entity, env.font_db);
        let inline_max =
            inline::max_content_inline_size(dom, children, &style, entity, env.font_db);
        IntrinsicSizes {
            min_content: min.max(inline_min),
            max_content: max.max(inline_max),
        }
    } else {
        // Pure inline children.
        IntrinsicSizes {
            min_content: inline::min_content_inline_size(
                dom,
                children,
                &style,
                entity,
                env.font_db,
            ),
            max_content: inline::max_content_inline_size(
                dom,
                children,
                &style,
                entity,
                env.font_db,
            ),
        }
    }
}

/// Flex intrinsic sizing (CSS Sizing Level 3 §5.1).
pub(super) fn compute_flex_intrinsic(
    dom: &mut EcsDom,
    entity: Entity,
    children: &[Entity],
    env: &LayoutEnv<'_>,
) -> IntrinsicSizes {
    let style = get_style(dom, entity);
    // Determine whether the flex main axis maps to the physical horizontal axis,
    // taking writing mode into account: row directions follow the inline axis,
    // column directions follow the block axis.
    let horizontal_wm = style.writing_mode.is_horizontal();
    let horizontal = match style.flex_direction {
        FlexDirection::Row | FlexDirection::RowReverse => horizontal_wm,
        FlexDirection::Column | FlexDirection::ColumnReverse => !horizontal_wm,
    };
    let nowrap = matches!(style.flex_wrap, FlexWrap::Nowrap);

    let child_sizes_list = collect_child_intrinsic_sizes(dom, children, env);

    if child_sizes_list.is_empty() {
        return IntrinsicSizes::default();
    }

    // CSS Box Alignment L3: gap between items contributes to intrinsic size.
    // Use the main-axis gap (column-gap for row flex, row-gap for column flex)
    // but select the physical axis based on writing mode.
    let gap = if horizontal {
        resolve_dimension_value(style.column_gap, 0.0, 0.0).max(0.0)
    } else {
        resolve_dimension_value(style.row_gap, 0.0, 0.0).max(0.0)
    };
    let gap_total = total_gap(child_sizes_list.len(), gap);

    if horizontal {
        // Row direction: items side-by-side along main axis.
        let sum_min: f32 = child_sizes_list.iter().map(|s| s.min_content).sum();
        let sum_max: f32 = child_sizes_list.iter().map(|s| s.max_content).sum();
        let max_min = child_sizes_list
            .iter()
            .map(|s| s.min_content)
            .fold(0.0_f32, f32::max);
        // CSS Sizing L3 §5.1:
        // nowrap: min = sum(items min) + gaps, max = sum(items max) + gaps
        // wrap: min = max(items min) (no gap — single item per line), max = sum + gaps
        if nowrap {
            IntrinsicSizes {
                min_content: sum_min + gap_total,
                max_content: sum_max + gap_total,
            }
        } else {
            IntrinsicSizes {
                min_content: max_min,
                max_content: sum_max + gap_total,
            }
        }
    } else {
        // Column direction: items stack vertically, intrinsic width = max of children.
        // Gap is on the main (vertical) axis — does not affect intrinsic inline size.
        let max_min = child_sizes_list
            .iter()
            .map(|s| s.min_content)
            .fold(0.0_f32, f32::max);
        let max_max = child_sizes_list
            .iter()
            .map(|s| s.max_content)
            .fold(0.0_f32, f32::max);
        IntrinsicSizes {
            min_content: max_min,
            max_content: max_max,
        }
    }
}

/// Collect per-child intrinsic sizes, filtering `display:none` and probing text nodes.
pub(super) fn collect_child_intrinsic_sizes(
    dom: &mut EcsDom,
    children: &[Entity],
    env: &LayoutEnv<'_>,
) -> Vec<IntrinsicSizes> {
    let mut result = Vec::new();
    for &child in children {
        let child_style = elidex_layout_block::try_get_style(dom, child);
        if child_style
            .as_ref()
            .is_some_and(|s| s.display == Display::None)
        {
            continue;
        }
        if child_style.is_none() {
            // Text node: measure via probe layout.
            let probe_min = probe_layout_size(dom, child, 1.0, env);
            let probe_max = probe_layout_size(dom, child, 1e6, env);
            result.push(IntrinsicSizes {
                min_content: probe_min,
                max_content: probe_max,
            });
            continue;
        }
        result.push(compute_intrinsic_sizes(dom, child, &env.deeper()));
    }
    result
}

/// Probe layout at a given containing width, return content-box inline-axis size.
///
/// Returns the inline-axis content dimension (`width` in horizontal-tb,
/// `height` in vertical modes), excluding the entity's own padding and border.
/// Intended for text nodes and leaf elements whose outer box model is
/// accounted for by `compute_intrinsic_sizes`.
pub(super) fn probe_layout_size(
    dom: &mut EcsDom,
    entity: Entity,
    containing_width: f32,
    env: &LayoutEnv<'_>,
) -> f32 {
    let input = LayoutInput::probe(env, containing_width);
    let lb = (env.layout_child)(dom, entity, &input).layout_box;
    // Determine inline axis from the parent's writing mode context.
    // For text/leaf nodes without a style, check ancestor style if available;
    // fallback to horizontal (width).
    let inline_horizontal = elidex_layout_block::try_get_style(dom, entity)
        .is_none_or(|s| WritingModeContext::new(s.writing_mode, s.direction).is_horizontal());
    if inline_horizontal {
        lb.content.size.width
    } else {
        lb.content.size.height
    }
}
