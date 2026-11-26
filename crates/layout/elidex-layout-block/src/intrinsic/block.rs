//! Block and flex intrinsic sizing.

use crate::{get_style, inline, resolve_dimension_value, total_gap, IntrinsicSizes, LayoutEnv};
use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{FlexDirection, FlexWrap};

use super::intrinsic_on_parent_axis;

/// Block intrinsic sizing.
///
/// Inline children → min/max content inline sizes.
/// Block children → max of each child's intrinsic sizes.
pub(super) fn compute_block_intrinsic(
    dom: &mut EcsDom,
    entity: Entity,
    children: &[Entity],
    env: &LayoutEnv<'_>,
    grid: super::GridIntrinsicFn,
) -> IntrinsicSizes {
    let style = get_style(dom, entity);
    let has_block_children = children.iter().any(|&c| {
        crate::try_get_style(dom, c).is_some_and(|s| crate::block::is_block_level(s.display))
    });

    if has_block_children {
        // Block children: max of each child's intrinsic sizes.
        let mut min = 0.0_f32;
        let mut max = 0.0_f32;
        let deeper = env.deeper();
        for &child in children {
            let child_sizes = intrinsic_on_parent_axis(
                dom,
                child,
                style.writing_mode.is_horizontal(),
                &deeper,
                grid,
            );
            let outer = with_inline_margins(
                child_sizes,
                &get_style(dom, child),
                style.writing_mode.is_horizontal(),
            );
            min = min.max(outer.min_content);
            max = max.max(outer.max_content);
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
    grid: super::GridIntrinsicFn,
) -> IntrinsicSizes {
    let style = get_style(dom, entity);
    // CSS Flexbox 1 §5.1: row follows the inline axis in every writing mode.
    let row = matches!(
        style.flex_direction,
        FlexDirection::Row | FlexDirection::RowReverse
    );
    let nowrap = matches!(style.flex_wrap, FlexWrap::Nowrap);

    let child_sizes_list = collect_child_intrinsic_sizes(dom, entity, children, env, grid, &style);

    if child_sizes_list.is_empty() {
        return IntrinsicSizes::default();
    }

    if row {
        let gap = resolve_dimension_value(style.column_gap, 0.0, 0.0).max(0.0);
        let gap_total = total_gap(child_sizes_list.len(), gap);
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
        // In a column, main-axis gaps do not contribute to intrinsic inline size.
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

fn with_inline_margins(
    mut sizes: IntrinsicSizes,
    style: &elidex_plugin::ComputedStyle,
    horizontal_inline: bool,
) -> IntrinsicSizes {
    let margins = crate::resolve_margins(style, 0.);
    let inline = if horizontal_inline {
        margins.horizontal()
    } else {
        margins.vertical()
    };
    sizes.min_content += inline;
    sizes.max_content += inline;
    sizes
}

/// Collect per-child intrinsic sizes, including fixed inline margins and probing text nodes.
fn collect_child_intrinsic_sizes(
    dom: &mut EcsDom,
    parent: Entity,
    children: &[Entity],
    env: &LayoutEnv<'_>,
    grid: super::GridIntrinsicFn,
    parent_style: &elidex_plugin::ComputedStyle,
) -> Vec<IntrinsicSizes> {
    let horizontal_inline = parent_style.writing_mode.is_horizontal();
    let mut result = Vec::new();
    // Adjacent text nodes form one anonymous flex item (Flexbox 1 §4).
    let mut remaining = children;
    while let Some((&child, rest)) = remaining.split_first() {
        if let Some(style) = crate::try_get_style(dom, child) {
            let sizes =
                intrinsic_on_parent_axis(dom, child, horizontal_inline, &env.deeper(), grid);
            result.push(with_inline_margins(sizes, &style, horizontal_inline));
            remaining = rest;
        } else {
            let end = 1 + rest
                .iter()
                .take_while(|&&node| crate::try_get_style(dom, node).is_none())
                .count();
            let (text, tail) = remaining.split_at(end);
            let visible = text.iter().any(|&node| {
                dom.world()
                    .get::<&elidex_ecs::TextContent>(node)
                    .is_ok_and(|t| !t.0.trim_matches([' ', '\t', '\r', '\n', '\x0c']).is_empty())
            });
            if visible {
                result.push(IntrinsicSizes {
                    min_content: inline::min_content_inline_size(
                        dom,
                        text,
                        parent_style,
                        parent,
                        env.font_db,
                    ),
                    max_content: inline::max_content_inline_size(
                        dom,
                        text,
                        parent_style,
                        parent,
                        env.font_db,
                    ),
                });
            }
            remaining = tail;
        }
    }
    result
}
