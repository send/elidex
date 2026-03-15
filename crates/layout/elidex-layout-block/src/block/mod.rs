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
    resolve_min_max, sanitize_border, sanitize_padding, LayoutInput, MAX_LAYOUT_DEPTH,
};

pub use children::{resolve_block_height, stack_block_children, StackResult};
pub use margin::resolve_margin;

use children::shift_block_children;
use margin::{apply_margin_auto_centering, collapse_margins};
use replaced::{resolve_replaced_height, resolve_replaced_width};

/// Returns `true` if the display value establishes a block-level box.
// TODO: InlineBlock should participate in inline formatting
// context (CSS 2.1 §9.2.2), not force block context.
pub fn is_block_level(display: Display) -> bool {
    matches!(
        display,
        Display::Block
            | Display::InlineBlock
            | Display::Flex
            | Display::InlineFlex
            | Display::Grid
            | Display::InlineGrid
            | Display::ListItem
            | Display::Table
            | Display::InlineTable
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
/// When this returns `true` and inline children are also present, inline
/// content is currently skipped. CSS 2.1 §9.2.1.1 requires wrapping
/// consecutive inline runs in anonymous block boxes.
// TODO: generate anonymous block boxes for mixed block/inline content.
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

    // --- Sanitize padding and border (protect against NaN/infinity/negative) ---
    let padding = sanitize_padding(&style);
    let border = sanitize_border(&style);
    let h_pb = horizontal_pb(&padding, &border);

    // --- Resolve margins ---
    let margin_top = resolve_margin(style.margin_top, containing_width);
    let margin_bottom = resolve_margin(style.margin_bottom, containing_width);

    // --- Check for replaced element (e.g. <img> with decoded ImageData) ---
    // Also check for form controls with intrinsic dimensions.
    // TODO(R7): form_intrinsic_size returns f32 while ImageData uses u32.
    // Unify intrinsic size API to a common type when reworking replaced element sizing.
    let intrinsic = dom
        .world()
        .get::<&ImageData>(entity)
        .ok()
        .map(|img| (img.width, img.height))
        .or_else(|| {
            dom.world()
                .get::<&elidex_form::FormControlState>(entity)
                .ok()
                .map(|fcs| {
                    let (w, h) = elidex_form::form_intrinsic_size(&fcs);
                    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                    (w.max(0.0) as u32, h.max(0.0) as u32)
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
        };
        let is_bfc = establishes_bfc(&style);
        let result = stack_block_children(dom, &children, &child_input, layout_child, is_bfc);

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
        // TODO(Phase 4): full axis swap for vertical writing modes.
        let inline_size = if matches!(
            style.writing_mode,
            WritingMode::VerticalRl | WritingMode::VerticalLr
        ) {
            // Use containing height if known; otherwise use content_width as fallback.
            containing_height.unwrap_or(content_width)
        } else {
            content_width
        };
        layout_inline_context(dom, &children, inline_size, &style, font_db)
    };

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
        || style.overflow != Overflow::Visible
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
