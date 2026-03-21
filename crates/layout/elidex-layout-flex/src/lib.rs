//! CSS Flexbox layout algorithm (CSS Flexbox Level 1, simplified).
//!
//! Implements the core flexbox algorithm: item collection, line splitting,
//! flexible length resolution, and cross/main axis alignment.

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::{
    adjust_min_max_for_border_box, block::resolve_margin, clamp_min_max, effective_align,
    horizontal_pb, resolve_explicit_height, resolve_min_max, sanitize, sanitize_border,
    vertical_pb, ChildLayoutFn, EmptyContainerParams, LayoutInput, MAX_LAYOUT_DEPTH,
};
use elidex_plugin::{
    AlignContent, AlignItems, AlignmentSafety, BoxSizing, ComputedStyle, Dimension, Direction,
    Display, EdgeSizes, FlexBasis, FlexDirection, FlexWrap, JustifyContent, LayoutBox, Rect,
    Visibility, WritingMode,
};
/// Sentinel value representing an indefinite container main-axis size.
///
/// Uses `f32::MAX / 2.0` (approximately 1.7e38) to avoid overflow when adding margins/padding,
/// while remaining large enough to never be reached by real layout values.
const INDEFINITE_MAIN_SIZE: f32 = f32::MAX / 2.0;

mod algo;
mod align;
mod baseline;

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;

// ---------------------------------------------------------------------------
// Axis helpers
// ---------------------------------------------------------------------------

/// Returns `true` if the flex main axis is horizontal.
///
/// In `horizontal-tb`, row = horizontal main axis; in vertical writing modes,
/// row = vertical main axis (CSS Flexbox §9.1, CSS Writing Modes §6.3).
fn is_main_horizontal(dir: FlexDirection, wm: WritingMode) -> bool {
    let horizontal_wm = wm.is_horizontal();
    match dir {
        FlexDirection::Row | FlexDirection::RowReverse => horizontal_wm,
        FlexDirection::Column | FlexDirection::ColumnReverse => !horizontal_wm,
    }
}

/// Returns `true` if the main axis direction is reversed.
pub(crate) fn is_reversed(dir: FlexDirection) -> bool {
    matches!(
        dir,
        FlexDirection::RowReverse | FlexDirection::ColumnReverse
    )
}

/// Resolve a percentage dimension against the containing main-axis size.
///
/// Returns `None` if the container is indefinite (percentage cannot be resolved).
fn resolve_percentage_main(pct: f32, containing_main: f32) -> Option<f32> {
    if containing_main >= INDEFINITE_MAIN_SIZE {
        None
    } else {
        Some(containing_main * pct / 100.0)
    }
}

/// Resolve flex-basis to a main-axis size in pixels.
///
/// Returns `None` when content sizing is needed (auto with auto main size,
/// or `content` keyword).
/// When `box-sizing: border-box`, subtracts main-axis padding and border
/// from the resolved value so it represents content size.
fn resolve_flex_basis(
    style: &ComputedStyle,
    direction: FlexDirection,
    containing_main: f32,
    inline_containing: f32,
    wm: WritingMode,
) -> Option<f32> {
    let raw = match style.flex_basis {
        FlexBasis::Length(px) => Some(px),
        FlexBasis::Percentage(pct) => resolve_percentage_main(pct, containing_main),
        FlexBasis::Content => None,
        FlexBasis::Auto => {
            let fallback = if is_main_horizontal(direction, wm) {
                style.width
            } else {
                style.height
            };
            match fallback {
                Dimension::Length(px) => Some(px),
                Dimension::Percentage(pct) => resolve_percentage_main(pct, containing_main),
                Dimension::Auto => None,
            }
        }
    };
    let px = raw?;
    // Adjust for box-sizing: border-box — convert from border-box to content size.
    if style.box_sizing == BoxSizing::BorderBox {
        let p = elidex_layout_block::resolve_padding(style, inline_containing);
        let b = sanitize_border(style);
        let pb = if is_main_horizontal(direction, wm) {
            horizontal_pb(&p, &b)
        } else {
            vertical_pb(&p, &b)
        };
        return Some((px - pb).max(0.0));
    }
    Some(px)
}

// ---------------------------------------------------------------------------
// Flex item
// ---------------------------------------------------------------------------

/// A flex item with resolved metrics.
#[allow(clippy::struct_excessive_bools)] // auto margin + collapsed flags are spec-mandated
pub(crate) struct FlexItem {
    pub(crate) entity: Entity,
    pub(crate) source_order: usize,
    pub(crate) order: i32,
    /// Flex base size before min/max clamping (CSS Flexbox §9.2 step 3).
    pub(crate) flex_base_size: f32,
    /// Hypothetical main size: flex base size clamped by min/max (§9.5 step 5).
    pub(crate) hypo_main: f32,
    pub(crate) grow: f32,
    pub(crate) shrink: f32,
    /// Total margin on the main axis (start + end).
    pub(crate) margin_main: f32,
    /// Total margin on the cross axis (start + end).
    pub(crate) margin_cross: f32,
    pub(crate) pb_main: f32,
    pub(crate) pb_cross: f32,
    pub(crate) final_main: f32,
    pub(crate) final_cross: f32,
    pub(crate) align: AlignItems,
    /// Whether the item's cross-size dimension is `auto` (stretch only applies when true).
    pub(crate) cross_size_auto: bool,
    /// Minimum content size on the main axis (from min-width/min-height).
    pub(crate) min_main: f32,
    /// Maximum content size on the main axis (from max-width/max-height).
    pub(crate) max_main: f32,
    /// Flex §8.1: auto margin flags for free-space distribution.
    pub(crate) margin_main_start_auto: bool,
    pub(crate) margin_main_end_auto: bool,
    pub(crate) margin_cross_start_auto: bool,
    pub(crate) margin_cross_end_auto: bool,
    /// Flex §4.4: visibility:collapse — item participates in sizing but renders at zero main size.
    pub(crate) collapsed: bool,
    /// First baseline offset from the item's margin-box cross-start edge.
    ///
    /// Populated after `layout_items_cross` by reading the child's `LayoutBox`.
    /// Used for baseline alignment (CSS Flexbox §9.4, §9.6).
    pub(crate) first_baseline: Option<f32>,
    /// Cross-start margin (top for row, left for column).
    ///
    /// Needed to compute the baseline offset from the margin-box edge.
    pub(crate) margin_cross_start: f32,
}

impl FlexItem {
    /// Returns the item's baseline for alignment, or a synthesized baseline
    /// at the border-box bottom (CSS Flexbox §9.4) when no intrinsic baseline exists.
    pub(crate) fn baseline_or_synthesized(&self) -> f32 {
        self.first_baseline
            .unwrap_or(self.margin_cross_start + self.final_cross)
    }
}

// ---------------------------------------------------------------------------
// Container context — shared state for the layout pass
// ---------------------------------------------------------------------------

pub(crate) struct FlexContext {
    pub(crate) content_x: f32,
    pub(crate) content_y: f32,
    pub(crate) content_width: f32,
    pub(crate) horizontal: bool,
    pub(crate) container_main: f32,
    pub(crate) direction: FlexDirection,
    pub(crate) wrap: FlexWrap,
    pub(crate) justify: JustifyContent,
    pub(crate) align_items: AlignItems,
    pub(crate) align_content: AlignContent,
    pub(crate) containing_width: f32,
    pub(crate) containing_height: Option<f32>,
    /// Containing block's inline size (for margin/padding % resolution).
    pub(crate) inline_containing: f32,
    /// The container's own definite height (for children's percentage height resolution).
    pub(crate) container_definite_height: Option<f32>,
    /// Gap between items on the main axis.
    pub(crate) gap_main: f32,
    /// Gap between lines on the cross axis.
    pub(crate) gap_cross: f32,
    /// CSS `direction` property (LTR/RTL) — affects main-axis order for row layouts.
    pub(crate) css_direction: Direction,
    /// CSS Box Alignment L3: safe/unsafe modifier for justify-content.
    pub(crate) justify_content_safety: AlignmentSafety,
    /// CSS Box Alignment L3: safe/unsafe modifier for align-content.
    pub(crate) align_content_safety: AlignmentSafety,
    /// Container's writing mode (CSS Writing Modes §6.3).
    pub(crate) writing_mode: WritingMode,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Resolve the container main-axis size.
fn resolve_container_main(
    style: &ComputedStyle,
    horizontal: bool,
    content_width: f32,
    containing_height: Option<f32>,
) -> f32 {
    if horizontal {
        content_width
    } else {
        match style.height {
            Dimension::Length(px) => sanitize(px),
            Dimension::Percentage(pct) => {
                containing_height.map_or(INDEFINITE_MAIN_SIZE, |ch| sanitize(ch * pct / 100.0))
            }
            Dimension::Auto => INDEFINITE_MAIN_SIZE,
        }
    }
}

/// Layout a flex container and return its `LayoutBox`.
#[allow(clippy::too_many_lines)]
// Sequential algorithm phases sharing extensive local state; splitting would add indirection without improving clarity.
pub fn layout_flex(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
    layout_child: ChildLayoutFn,
) -> LayoutBox {
    let containing_width = input.containing_width;
    let containing_height = input.containing_height;
    let offset_x = input.offset_x;
    let offset_y = input.offset_y;
    let font_db = input.font_db;
    let depth = input.depth;
    let style = elidex_layout_block::get_style(dom, entity);

    let inline_containing = input.containing_inline_size;
    let padding = elidex_layout_block::resolve_padding(&style, inline_containing);
    let border = sanitize_border(&style);
    let margin_top = resolve_margin(style.margin_top, inline_containing);
    let margin_bottom = resolve_margin(style.margin_bottom, inline_containing);
    let margin_left = resolve_margin(style.margin_left, inline_containing);
    let margin_right = resolve_margin(style.margin_right, inline_containing);
    let h_pb = horizontal_pb(&padding, &border);
    let content_width = elidex_layout_block::resolve_content_width(
        &style,
        containing_width,
        h_pb,
        margin_left + margin_right,
    );
    let content_x = offset_x + margin_left + border.left + padding.left;
    let content_y = offset_y + margin_top + border.top + padding.top;
    let margin = EdgeSizes::new(margin_top, margin_right, margin_bottom, margin_left);
    let horizontal = is_main_horizontal(style.flex_direction, style.writing_mode);
    let container_main =
        resolve_container_main(&style, horizontal, content_width, containing_height);

    // --- Early return for empty containers ---
    let children = elidex_layout_block::composed_children_flat(dom, entity);
    if children.is_empty() || depth >= MAX_LAYOUT_DEPTH {
        return elidex_layout_block::empty_container_box(
            dom,
            entity,
            &EmptyContainerParams {
                style: &style,
                content_x,
                content_y,
                content_width,
                containing_height,
                padding,
                border,
                margin,
            },
        );
    }

    // Resolve gap: row-gap applies between rows, column-gap between columns.
    // For flex-direction: row, main axis is horizontal → gap_main = column_gap, gap_cross = row_gap.
    // For flex-direction: column, main axis is vertical → gap_main = row_gap, gap_cross = column_gap.
    let resolve_gap = |dim: elidex_plugin::Dimension| {
        elidex_layout_block::resolve_dimension_value(dim, content_width, 0.0).max(0.0)
    };
    let (gap_main, gap_cross) = if horizontal {
        (resolve_gap(style.column_gap), resolve_gap(style.row_gap))
    } else {
        (resolve_gap(style.row_gap), resolve_gap(style.column_gap))
    };

    // Container's own definite height for children's percentage height resolution.
    let container_definite_height = resolve_explicit_height(&style, containing_height);

    let ctx = FlexContext {
        content_x,
        content_y,
        content_width,
        horizontal,
        container_main,
        direction: style.flex_direction,
        wrap: style.flex_wrap,
        justify: style.justify_content,
        align_items: style.align_items,
        align_content: style.align_content,
        containing_width,
        containing_height,
        inline_containing,
        container_definite_height,
        gap_main,
        gap_cross,
        css_direction: style.direction,
        justify_content_safety: style.justify_content_safety,
        align_content_safety: style.align_content_safety,
        writing_mode: style.writing_mode,
    };

    let env = algo::LayoutEnv {
        font_db,
        layout_child,
        depth,
        input_viewport: input.viewport,
    };

    // --- Collect, sort, flex-resolve, layout, position ---
    let mut items = collect_flex_items(dom, &children, &ctx, &env);
    items.sort_by(|a, b| {
        a.order
            .cmp(&b.order)
            .then(a.source_order.cmp(&b.source_order))
    });

    let line_ranges = algo::build_line_ranges(&items, ctx.container_main, ctx.wrap, ctx.gap_main);
    for &(start, end) in &line_ranges {
        algo::resolve_flexible_lengths(&mut items[start..end], ctx.container_main, ctx.gap_main);
    }

    // Flex §4.4: After flexible length resolution, collapsed items have their
    // main size and main-axis margins zeroed so they occupy no main-axis space,
    // but they still participate in cross-size computation (act as a strut).
    for item in items.iter_mut().filter(|i| i.collapsed) {
        item.final_main = 0.0;
        item.margin_main = 0.0;
        // Clear auto margin flags so collapsed items don't participate
        // in auto margin free-space distribution in position_items.
        item.margin_main_start_auto = false;
        item.margin_main_end_auto = false;
    }

    algo::layout_items_cross(dom, &mut items, &ctx, &env);

    // Read baselines from children after cross-size layout.
    baseline::read_item_baselines(dom, &mut items, &ctx);

    let line_baselines = baseline::compute_line_baselines(&items, &line_ranges, horizontal);
    let (line_cross_sizes, total_line_cross) =
        algo::compute_line_cross_sizes(&items, &line_ranges, &line_baselines);

    let container_cross =
        algo::resolve_container_cross(&style, &ctx, containing_width, total_line_cross);
    let align_content = align::apply_align_content_safety(
        ctx.align_content,
        container_cross,
        &line_cross_sizes,
        ctx.gap_cross,
        ctx.align_content_safety,
    );
    let align_result = align::compute_align_content_offsets(
        &line_cross_sizes,
        container_cross,
        align_content,
        ctx.wrap,
        ctx.gap_cross,
    );

    // Stretch items using effective line sizes (includes align-content stretch extra).
    algo::stretch_items(&mut items, &line_ranges, &align_result.effective_line_sizes);

    let lines = algo::LineGeometry {
        line_ranges: &line_ranges,
        line_cross_sizes: &align_result.effective_line_sizes,
        line_offsets: &align_result.offsets,
        line_baselines: &line_baselines,
        container_cross,
    };
    algo::position_items(dom, &items, &lines, &ctx, &env);

    // --- Container LayoutBox ---
    let content_height =
        algo::compute_container_height(&style, &ctx, &items, &line_ranges, total_line_cross);

    // Flex container baseline (CSS Flexbox §9.4): first line's max baseline.
    let wrap_reverse = matches!(ctx.wrap, FlexWrap::WrapReverse);
    let first_baseline = if !line_baselines.is_empty() && line_baselines[0] > 0.0 {
        // First logical line's max baseline, content-box relative.
        // wrap-reverse mirrors cross positions: visual offset = container_cross - off - line_cross.
        align_result.offsets.first().map(|&off| {
            let cross_off = if wrap_reverse {
                container_cross - off - align_result.effective_line_sizes[0]
            } else {
                off
            };
            cross_off + line_baselines[0]
        })
    } else if !items.is_empty() {
        // Fallback: first item's own baseline.
        let first_item = &items[0];
        dom.world()
            .get::<&LayoutBox>(first_item.entity)
            .ok()
            .and_then(|lb| lb.first_baseline.map(|bl| lb.content.y - content_y + bl))
    } else {
        None
    };

    let lb = LayoutBox {
        content: Rect::new(content_x, content_y, content_width, content_height),
        padding,
        border,
        margin,
        first_baseline,
    };
    let _ = dom.world_mut().insert_one(entity, lb.clone());

    // Layout positioned descendants owned by this containing block.
    // CSS Flexbox §4.1: the flex container establishes a CB for absolute children
    // when it is itself positioned (or is the root).
    // CSS Transforms L1 §2: transform establishes CB for all descendants.
    let is_root = dom.get_parent(entity).is_none();
    let is_cb = style.position != elidex_plugin::Position::Static || is_root || style.has_transform;
    if is_cb {
        let static_positions = elidex_layout_block::positioned::collect_abspos_static_positions(
            dom, &children, content_x, content_y,
        );
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

    lb
}

// ---------------------------------------------------------------------------
// Helpers — box model
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Item collection
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn collect_flex_items(
    dom: &mut EcsDom,
    children: &[Entity],
    ctx: &FlexContext,
    env: &algo::LayoutEnv<'_>,
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
                containing_width: probe_width,
                containing_height: None,
                containing_inline_size: probe_width,
                offset_x: 0.0,
                offset_y: 0.0,
                font_db: env.font_db,
                depth: env.depth + 1,
                float_ctx: None,
                viewport: env.input_viewport,
                fragmentainer: None,
                break_token: None,
                subgrid: None,
            };
            let child_lb = (env.layout_child)(dom, child, &child_input).layout_box;
            if ctx.horizontal {
                child_lb.content.width
            } else {
                child_lb.content.height
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
            let ch = ctx.container_definite_height.unwrap_or(0.0);
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
    env: &algo::LayoutEnv<'_>,
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
        containing_width: 1.0,
        containing_height: None,
        containing_inline_size: 1.0,
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: env.font_db,
        depth: env.depth + 1,
        float_ctx: None,
        viewport: env.input_viewport,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };
    let probe_lb = (env.layout_child)(dom, child, &probe_input).layout_box;
    let content_min = if ctx.horizontal {
        probe_lb.content.width
    } else {
        probe_lb.content.height
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

fn compute_pb(style: &ComputedStyle, horizontal: bool, containing_width: f32) -> (f32, f32) {
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
fn compute_margins(style: &ComputedStyle, horizontal: bool, containing_width: f32) -> (f32, f32) {
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
