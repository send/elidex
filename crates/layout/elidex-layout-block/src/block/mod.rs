//! Block formatting context layout algorithm.
//!
//! Computes the CSS box model (content, padding, border, margin) for
//! block-level elements, handling width/height resolution, margin auto
//! centering, and vertical stacking of child blocks.

mod children;
pub mod float;
mod margin;
mod replaced;

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;

use elidex_ecs::{EcsDom, Entity, ImageData};
use elidex_plugin::{
    BoxSizing, Dimension, Display, EdgeSizes, Float, LayoutBox, Overflow, Position, Rect,
    WritingMode,
};
use elidex_text::FontDatabase;

use crate::inline::layout_inline_context;
use crate::sanitize;
use crate::{
    adjust_min_max_for_border_box, clamp_min_max, horizontal_pb, resolve_dimension_value,
    resolve_min_max, sanitize_border, LayoutInput, MAX_LAYOUT_DEPTH,
};

pub use children::{resolve_block_height, stack_block_children, StackResult};
pub use margin::resolve_margin;

use children::shift_block_children;
use margin::{apply_margin_auto_centering, collapse_margins};
use replaced::{resolve_replaced_height, resolve_replaced_width};

/// Returns `true` if the display value establishes a block-level box.
///
/// Atomic inline-level boxes (`inline-block`, `inline-flex`, `inline-grid`,
/// `inline-table`) are NOT block-level — they participate in inline
/// formatting context (CSS 2.1 §9.2.2) and are laid out as atomic units
/// within inline flow.
pub fn is_block_level(display: Display) -> bool {
    matches!(
        display,
        Display::Block
            | Display::Flex
            | Display::Grid
            | Display::ListItem
            | Display::Table
            | Display::TableCaption
            | Display::TableRowGroup
            | Display::TableHeaderGroup
            | Display::TableFooterGroup
            | Display::TableRow
            | Display::TableCell
            | Display::TableColumn
            | Display::TableColumnGroup
    )
}

/// Returns `true` if any child is block-level (block formatting context).
///
/// When this returns `true` and inline children are also present,
/// [`stack_block_children`] wraps consecutive inline runs in anonymous
/// block boxes (CSS 2.1 §9.2.1.1).
fn children_are_block(dom: &EcsDom, children: &[Entity]) -> bool {
    children
        .iter()
        .any(|&child| crate::try_get_style(dom, child).is_some_and(|s| is_block_level(s.display)))
}

/// Layout a block-level element, inserting `LayoutBox` on it and all descendants.
pub fn layout_block(
    dom: &mut EcsDom,
    entity: Entity,
    containing_width: f32,
    offset_x: f32,
    offset_y: f32,
    font_db: &FontDatabase,
) -> LayoutBox {
    let input = LayoutInput {
        containing_width,
        containing_height: None,
        offset_x,
        offset_y,
        font_db,
        depth: 0,
        float_ctx: None,
    };
    layout_block_inner(dom, entity, &input, crate::layout_block_only)
}

/// Layout a block-level element with an explicit containing height.
///
/// Used when the parent has a definite height (e.g. `height: 500px`) so that
/// children with `height: 50%` can be resolved.
pub fn layout_block_with_height(
    dom: &mut EcsDom,
    entity: Entity,
    containing_width: f32,
    containing_height: Option<f32>,
    offset_x: f32,
    offset_y: f32,
    font_db: &FontDatabase,
) -> LayoutBox {
    let input = LayoutInput {
        containing_width,
        containing_height,
        offset_x,
        offset_y,
        font_db,
        depth: 0,
        float_ctx: None,
    };
    layout_block_inner(dom, entity, &input, crate::layout_block_only)
}

/// Inner recursive implementation with depth tracking.
///
/// Uses the provided `layout_child` callback to dispatch child layout,
/// which allows the orchestrator to route flex/grid containers to
/// their respective layout algorithms.
#[allow(clippy::too_many_lines)]
// Sequential algorithm phases sharing extensive local state; splitting would add indirection without improving clarity.
pub fn layout_block_inner(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
    layout_child: crate::ChildLayoutFn,
) -> LayoutBox {
    let containing_width = input.containing_width;
    let containing_height = input.containing_height;
    let offset_x = input.offset_x;
    let offset_y = input.offset_y;
    let font_db = input.font_db;
    let depth = input.depth;

    let style = crate::get_style(dom, entity);
    let is_vertical = !matches!(style.writing_mode, WritingMode::HorizontalTb);

    // --- Resolve padding and border (protect against NaN/infinity/negative) ---
    // CSS 2.1 §8.4: padding % refers to containing block width.
    let padding = crate::resolve_padding(&style, containing_width);
    let border = sanitize_border(&style);
    let h_pb = horizontal_pb(&padding, &border);

    // --- Resolve margins ---
    let margin_top = resolve_margin(style.margin_top, containing_width);
    let margin_bottom = resolve_margin(style.margin_bottom, containing_width);

    // --- Check for replaced element (e.g. <img> with decoded ImageData) ---
    // Also check for form controls with intrinsic dimensions.
    #[allow(clippy::cast_precision_loss)]
    let intrinsic: Option<(f32, f32)> = dom
        .world()
        .get::<&ImageData>(entity)
        .ok()
        .map(|img| (img.width as f32, img.height as f32))
        .or_else(|| {
            dom.world()
                .get::<&elidex_form::FormControlState>(entity)
                .ok()
                .map(|fcs| {
                    let (w, h) = elidex_form::form_intrinsic_size(&fcs);
                    (w.max(0.0), h.max(0.0))
                })
        });

    // --- Resolve width ---
    let margin_left_raw = resolve_margin(style.margin_left, containing_width);
    let margin_right_raw = resolve_margin(style.margin_right, containing_width);
    let horizontal_extra = margin_left_raw + margin_right_raw + h_pb;
    let mut content_width = if let Some((iw, ih)) = intrinsic {
        resolve_replaced_width(&style, containing_width, iw, ih, &padding, &border)
    } else {
        sanitize(resolve_dimension_value(
            style.width,
            containing_width,
            (containing_width - horizontal_extra).max(0.0),
        ))
    };
    // box-sizing: border-box — subtract padding + border from specified width.
    // Only for non-replaced elements or replaced elements with explicit dimensions.
    if style.box_sizing == BoxSizing::BorderBox && intrinsic.is_none() {
        if let Dimension::Length(_) | Dimension::Percentage(_) = style.width {
            content_width = (content_width - h_pb).max(0.0);
        }
    }

    // --- Apply min-width / max-width constraints (CSS 2.1 §10.4) ---
    // min-width wins over max-width when they conflict.
    // For box-sizing: border-box, min/max are specified as border-box values,
    // so subtract padding+border to compare with content_width.
    {
        let mut min_w = resolve_min_max(style.min_width, containing_width, 0.0);
        let mut max_w = resolve_min_max(style.max_width, containing_width, f32::INFINITY);
        if style.box_sizing == BoxSizing::BorderBox && intrinsic.is_none() {
            adjust_min_max_for_border_box(&mut min_w, &mut max_w, h_pb);
        }
        content_width = clamp_min_max(content_width, min_w, max_w);
    }

    // --- Horizontal margin auto centering ---
    let used_horizontal = content_width + h_pb;
    let (margin_left, margin_right) =
        if matches!(style.width, Dimension::Auto) && intrinsic.is_none() {
            (margin_left_raw, margin_right_raw)
        } else {
            apply_margin_auto_centering(&style, containing_width, used_horizontal, style.direction)
        };

    // --- Content rect position ---
    let content_x = offset_x + margin_left + border.left + padding.left;
    let mut content_y = offset_y + margin_top + border.top + padding.top;

    // --- Layout children (stop recursion at depth limit) ---
    // Flatten display:contents children — they don't generate boxes,
    // their children participate in this element's formatting context.
    let children = crate::composed_children_flat(dom, entity);
    let mut collapsed_margin_top = margin_top;
    let mut collapsed_margin_bottom = margin_bottom;

    // Compute the definite height of this element (if any) for children's percentage heights.
    let child_containing_height = crate::resolve_explicit_height(&style, containing_height);

    // CSS Writing Modes Level 3 §3.1: In vertical modes, the block-axis result
    // from inline layout is the total column width (physical width), not height.
    // This variable captures the override when an axis swap is needed.
    let mut vertical_width_override: Option<f32> = None;

    let content_height = if let Some((iw, ih)) = intrinsic {
        // Replaced element: use intrinsic/CSS height, no child layout.
        resolve_replaced_height(&style, content_width, iw, ih, &padding, &border)
    } else if children.is_empty() || depth >= MAX_LAYOUT_DEPTH {
        0.0
    } else if children_are_block(dom, &children) {
        let child_input = LayoutInput {
            containing_width: content_width,
            containing_height: child_containing_height,
            offset_x: content_x,
            offset_y: content_y,
            font_db,
            depth: depth + 1,
            float_ctx: input.float_ctx,
        };
        let is_bfc = establishes_bfc(&style);
        let result =
            stack_block_children(dom, &children, &child_input, layout_child, is_bfc, entity);

        // Parent-child margin collapse (CSS 2.1 §8.3.1):
        // First child's top margin collapses with parent's top margin
        // when parent has no border-top, no padding-top, and the parent
        // does not establish a new block formatting context.
        if border.top == 0.0 && padding.top == 0.0 && !is_bfc {
            if let Some(first_mt) = result.first_child_margin_top {
                let new_margin_top = collapse_margins(margin_top, first_mt);
                // Parent content origin shifts by (new_margin - old_margin), and
                // the first child's margin is absorbed into the parent, so all
                // children shift by that delta minus the first child's margin.
                let delta = (new_margin_top - collapsed_margin_top) - first_mt;
                content_y = offset_y + new_margin_top + border.top + padding.top;
                shift_block_children(dom, &children, delta);
                collapsed_margin_top = new_margin_top;
            }
        }
        // Last child's bottom margin collapses with parent's bottom margin
        // when parent has no border-bottom, no padding-bottom, height is auto,
        // and the parent does not establish a new BFC (CSS 2.1 §8.3.1).
        if border.bottom == 0.0
            && padding.bottom == 0.0
            && matches!(style.height, Dimension::Auto)
            && !is_bfc
        {
            if let Some(last_mb) = result.last_child_margin_bottom {
                collapsed_margin_bottom = collapse_margins(margin_bottom, last_mb);
            }
        }

        result.height
    } else {
        // Inline context: the first argument is the available inline-axis space.
        // Horizontal: inline axis = X (width). Vertical: inline axis = Y (height).
        let inline_size = if is_vertical {
            // CSS Writing Modes Level 3 §3.1: In vertical modes, the inline axis
            // is vertical (height). Use containing height when known.
            containing_height.unwrap_or(content_width)
        } else {
            content_width
        };
        let block_result = layout_inline_context(
            dom,
            &children,
            inline_size,
            &style,
            font_db,
            entity,
            (content_x, content_y),
            layout_child,
        );
        if is_vertical {
            // block_result = total column width (block-axis in vertical mode).
            // Store it to override content_width below. Return the inline-axis
            // size (physical height) as content_height for resolve_block_height.
            vertical_width_override = Some(block_result);
            inline_size
        } else {
            block_result
        }
    };

    // CSS Writing Modes Level 3 §3.1: Apply vertical axis swap.
    // In vertical modes, inline layout's block-axis result is the physical width.
    if let Some(vw) = vertical_width_override {
        if matches!(style.width, Dimension::Auto) {
            content_width = vw;
        }
    }

    let height = resolve_block_height(
        &style,
        content_height,
        containing_height,
        &padding,
        &border,
        intrinsic.is_some(),
    );

    let lb = LayoutBox {
        content: Rect::new(content_x, content_y, content_width, height),
        padding,
        border,
        margin: EdgeSizes::new(
            collapsed_margin_top,
            margin_right,
            collapsed_margin_bottom,
            margin_left,
        ),
    };

    let _ = dom.world_mut().insert_one(entity, lb.clone());
    lb
}

/// CSS 2.1 §9.4.1: Does this element establish a new block formatting context?
///
/// Elements that establish a BFC prevent margin collapse with children.
fn establishes_bfc(style: &elidex_plugin::ComputedStyle) -> bool {
    style.float != Float::None
        || matches!(style.position, Position::Absolute | Position::Fixed)
        || style.overflow_x != Overflow::Visible
        || style.overflow_y != Overflow::Visible
        || matches!(
            style.display,
            Display::InlineBlock
                | Display::Flex
                | Display::InlineFlex
                | Display::Grid
                | Display::InlineGrid
                | Display::Table
                | Display::InlineTable
                | Display::TableCell
        )
}
