//! Box model helpers, item collection, and automatic minimum size computation.

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::block::resolve_margin;
use elidex_layout_block::{
    adjust_min_max_for_border_box, clamp_min_max, effective_align, horizontal_pb, resolve_min_max,
    sanitize, sanitize_border, vertical_pb, LayoutInput,
};
use elidex_plugin::{BoxSizing, ComputedStyle, Dimension, Display, FlexBasis, Visibility};

use super::{is_reversed, resolve_flex_basis, FlexContext, FlexItem};

/// Compute padding+border on main and cross axes.
pub(crate) fn compute_pb(
    style: &ComputedStyle,
    horizontal: bool,
    containing_width: f32,
) -> (f32, f32) {
    let p = elidex_layout_block::resolve_padding(style, containing_width);
    let b = sanitize_border(style);
    let h = horizontal_pb(&p, &b);
    let v = vertical_pb(&p, &b);
    if horizontal {
        (h, v)
    } else {
        (v, h)
    }
}

/// Returns `(margin_main, margin_cross)`.
pub(crate) fn compute_margins(
    style: &ComputedStyle,
    horizontal: bool,
    containing_width: f32,
) -> (f32, f32) {
    let ml = resolve_margin(style.margin_left, containing_width);
    let mr = resolve_margin(style.margin_right, containing_width);
    let mt = resolve_margin(style.margin_top, containing_width);
    let mb = resolve_margin(style.margin_bottom, containing_width);
    if horizontal {
        (ml + mr, mt + mb)
    } else {
        (mt + mb, ml + mr)
    }
}

/// Collect flex items from children, resolving styles and computing metrics.
#[allow(clippy::too_many_lines)]
pub(crate) fn collect_flex_items(
    dom: &mut EcsDom,
    children: &[Entity],
    ctx: &FlexContext,
    env: &super::algo::LayoutEnv<'_>,
) -> Vec<FlexItem> {
    let mut items = Vec::new();
    for (source_order, &child) in children.iter().enumerate() {
        let Some(mut child_style) = elidex_layout_block::try_get_style(dom, child) else {
            continue;
        };
        if child_style.display == Display::None {
            continue;
        }
        // Absolutely positioned flex children are removed from flex layout.
        if elidex_layout_block::positioned::is_absolutely_positioned(&child_style) {
            continue;
        }

        // Flex §4.2: blockify flex items.
        let blockified = child_style.display.blockify();
        if blockified != child_style.display {
            child_style.display = blockified;
            let _ = dom.world_mut().insert_one(child, child_style.clone());
        }

        let (pb_main, pb_cross) = compute_pb(&child_style, ctx.horizontal, ctx.inline_containing);

        // Flex §8.1: detect auto margins before resolving to 0.
        let (
            margin_main_start_auto,
            margin_main_end_auto,
            margin_cross_start_auto,
            margin_cross_end_auto,
        ) = if ctx.horizontal {
            let (ms, me) = if is_reversed(ctx.direction) {
                (
                    child_style.margin_right == Dimension::Auto,
                    child_style.margin_left == Dimension::Auto,
                )
            } else {
                (
                    child_style.margin_left == Dimension::Auto,
                    child_style.margin_right == Dimension::Auto,
                )
            };
            (
                ms,
                me,
                child_style.margin_top == Dimension::Auto,
                child_style.margin_bottom == Dimension::Auto,
            )
        } else {
            let (ms, me) = if is_reversed(ctx.direction) {
                (
                    child_style.margin_bottom == Dimension::Auto,
                    child_style.margin_top == Dimension::Auto,
                )
            } else {
                (
                    child_style.margin_top == Dimension::Auto,
                    child_style.margin_bottom == Dimension::Auto,
                )
            };
            (
                ms,
                me,
                child_style.margin_left == Dimension::Auto,
                child_style.margin_right == Dimension::Auto,
            )
        };

        let (margin_main, margin_cross) =
            compute_margins(&child_style, ctx.horizontal, ctx.inline_containing);

        // Cross-start margin for baseline offset computation.
        let margin_cross_start = if ctx.horizontal {
            resolve_margin(child_style.margin_top, ctx.inline_containing)
        } else {
            resolve_margin(child_style.margin_left, ctx.inline_containing)
        };

        // Flex §4.4: visibility:collapse detection.
        let collapsed = child_style.visibility == Visibility::Collapse;

        // CSS spec: stretch only applies when the cross-size property is auto.
        let cross_size_auto = if ctx.horizontal {
            matches!(child_style.height, Dimension::Auto)
        } else {
            matches!(child_style.width, Dimension::Auto)
        };

        let basis = resolve_flex_basis(
            &child_style,
            ctx.direction,
            ctx.container_main,
            ctx.inline_containing,
            ctx.writing_mode,
        );
        // CSS Flexbox §9.2 step 3(B): flex-basis: content → max-content size.
        // flex-basis: auto with width: auto → layout at container width.
        let is_content_basis = matches!(child_style.flex_basis, FlexBasis::Content);
        let hypo_main = if let Some(px) = basis {
            sanitize(px).max(0.0)
        } else {
            // content: probe at very large width for max-content.
            // auto (width: auto): probe at container width.
            let probe_width = if is_content_basis {
                1e6
            } else {
                ctx.content_width
            };
            let child_input = LayoutInput {
                viewport: env.viewport,
                ..LayoutInput::probe(env, probe_width)
            };
            let child_lb = (env.layout_child)(dom, child, &child_input).layout_box;
            if ctx.horizontal {
                child_lb.content.size.width
            } else {
                child_lb.content.size.height
            }
        };

        // Resolve min/max constraints on the main axis.
        // For box-sizing: border-box, subtract padding+border from min/max
        // so they compare correctly with content-level hypo_main.
        let containing_main = ctx.container_main;
        let (raw_min_dim, mut max_main) = if ctx.horizontal {
            (
                child_style.min_width,
                resolve_min_max(child_style.max_width, containing_main, f32::INFINITY),
            )
        } else {
            // Column direction: items' containing block is the flex container itself.
            let ch = ctx.resolved_height.unwrap_or(0.0);
            (
                child_style.min_height,
                resolve_min_max(child_style.max_height, ch, f32::INFINITY),
            )
        };
        // Flex §4.5: auto min → automatic minimum size (content-based).
        let mut min_main = if raw_min_dim == Dimension::Auto {
            compute_automatic_minimum(dom, child, &child_style, ctx, env, pb_main)
        } else {
            resolve_min_max(raw_min_dim, containing_main, 0.0)
        };
        if child_style.box_sizing == BoxSizing::BorderBox {
            adjust_min_max_for_border_box(&mut min_main, &mut max_main, pb_main);
        }
        // Flex base size is pre-clamp (CSS Flexbox §9.2 step 3).
        let flex_base_size = hypo_main;
        // Clamp hypothetical main size by min/max (CSS §9.5 step 5).
        let hypo_main = clamp_min_max(hypo_main, min_main, max_main);

        items.push(FlexItem {
            entity: child,
            source_order,
            order: child_style.order,
            flex_base_size,
            hypo_main,
            grow: child_style.flex_grow,
            shrink: child_style.flex_shrink,
            margin_main,
            margin_cross,
            pb_main,
            pb_cross,
            final_main: hypo_main,
            final_cross: 0.0,
            align: effective_align(child_style.align_self, ctx.align_items),
            cross_size_auto,
            min_main,
            max_main,
            margin_main_start_auto,
            margin_main_end_auto,
            margin_cross_start_auto,
            margin_cross_end_auto,
            collapsed,
            first_baseline: None,
            margin_cross_start,
        });
    }
    items
}

/// Compute Flex §4.5 automatic minimum size for a flex item with `min-width: auto`.
///
/// `automatic_minimum` = min(`content_based_minimum`, `specified_size_suggestion`)
/// `content_based_minimum` = min-content size (content probe at 1px)
/// `specified_size_suggestion` = flex-basis or width/height if definite, else infinity
/// overflow != visible → return 0 (clamped minimum)
fn compute_automatic_minimum(
    dom: &mut EcsDom,
    child: Entity,
    child_style: &ComputedStyle,
    ctx: &FlexContext,
    env: &super::algo::LayoutEnv<'_>,
    pb_main: f32,
) -> f32 {
    use elidex_plugin::Overflow;

    // Flex §4.5: overflow != visible → clamped minimum (0).
    let overflow = if ctx.horizontal {
        child_style.overflow_x
    } else {
        child_style.overflow_y
    };
    if overflow != Overflow::Visible {
        return 0.0;
    }

    // Content-based minimum: probe layout at near-zero width.
    let probe_input = LayoutInput {
        viewport: env.viewport,
        ..LayoutInput::probe(env, 1.0)
    };
    let probe_lb = (env.layout_child)(dom, child, &probe_input).layout_box;
    let content_min = if ctx.horizontal {
        probe_lb.content.size.width
    } else {
        probe_lb.content.size.height
    };

    // Specified size suggestion (CSS Flexbox §4.5):
    // Comes from the item's computed main size property (width/height),
    // NOT from flex-basis.
    let specified_suggestion = {
        let dim = if ctx.horizontal {
            child_style.width
        } else {
            child_style.height
        };
        match dim {
            Dimension::Length(px) if px.is_finite() => {
                if child_style.box_sizing == BoxSizing::BorderBox {
                    (px - pb_main).max(0.0)
                } else {
                    px
                }
            }
            _ => f32::INFINITY,
        }
    };

    content_min.min(specified_suggestion)
}
