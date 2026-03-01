//! Block formatting context layout algorithm.
//!
//! Computes the CSS box model (content, padding, border, margin) for
//! block-level elements, handling width/height resolution, margin auto
//! centering, and vertical stacking of child blocks.

use elidex_ecs::{EcsDom, Entity, ImageData};
use elidex_plugin::{BoxSizing, ComputedStyle, Dimension, Display, EdgeSizes, LayoutBox, Rect};
use elidex_text::FontDatabase;

use crate::inline::layout_inline_context;
use crate::sanitize;
use crate::{
    adjust_min_max_for_border_box, clamp_min_max, horizontal_pb, resolve_dimension_value,
    resolve_min_max, sanitize_border, sanitize_padding, vertical_pb, MAX_LAYOUT_DEPTH,
};

/// Resolve a `Dimension` margin value to pixels.
///
/// `Auto` returns 0.0 here; horizontal auto centering is handled separately.
/// Non-finite results are replaced with 0.0. Margins may be negative (unlike
/// padding/border), so `sanitize()` is used instead of clamping to non-negative.
pub(crate) fn resolve_margin(dim: Dimension, containing_width: f32) -> f32 {
    sanitize(resolve_dimension_value(dim, containing_width, 0.0))
}

/// Apply horizontal `margin: auto` centering (CSS 2.1 §10.3.3).
///
/// `used_horizontal` = `content_width` + padding + border (already sanitized).
/// When the box is overconstrained (used width + padding + border > containing
/// width), LTR direction sets `margin-left` to the specified value (or 0 for
/// auto) and `margin-right` absorbs the negative remainder.
fn apply_margin_auto_centering(
    style: &ComputedStyle,
    containing_width: f32,
    used_horizontal: f32,
) -> (f32, f32) {
    let remaining = containing_width - used_horizontal;
    let left_auto = matches!(style.margin_left, Dimension::Auto);
    let right_auto = matches!(style.margin_right, Dimension::Auto);

    match (left_auto, right_auto) {
        (true, true) => {
            if remaining >= 0.0 {
                (remaining / 2.0, remaining / 2.0)
            } else {
                // Overconstrained (LTR): margin-left = 0, margin-right absorbs overflow.
                (0.0, remaining)
            }
        }
        (true, false) => {
            let mr = resolve_margin(style.margin_right, containing_width);
            (remaining - mr, mr)
        }
        (false, true) => {
            let ml = resolve_margin(style.margin_left, containing_width);
            (ml, remaining - ml)
        }
        (false, false) => {
            let ml = resolve_margin(style.margin_left, containing_width);
            // CSS 2.1 §10.3.3: When no margins are auto, the system is
            // over-constrained. In LTR, margin-right is recalculated to
            // satisfy the constraint equation.
            (ml, containing_width - used_horizontal - ml)
        }
    }
}

/// Collapse two adjacent margins per CSS 2.1 §8.3.1.
///
/// - Both positive: the larger wins.
/// - Both negative: the more negative (smaller) wins.
/// - Mixed: they are summed.
pub(crate) fn collapse_margins(a: f32, b: f32) -> f32 {
    if a >= 0.0 && b >= 0.0 {
        a.max(b)
    } else if a < 0.0 && b < 0.0 {
        a.min(b)
    } else {
        a + b
    }
}

/// Resolve the final height for a block element.
///
/// Handles CSS height property (Length/Percentage/Auto), border-box adjustment,
/// and min-height/max-height constraints. `content_height` is used when the
/// height is auto.
fn resolve_block_height(
    style: &ComputedStyle,
    content_height: f32,
    containing_height: Option<f32>,
    padding: &EdgeSizes,
    border: &EdgeSizes,
    is_replaced: bool,
) -> f32 {
    let mut height = if is_replaced {
        content_height
    } else {
        match style.height {
            Dimension::Length(px) if px.is_finite() => {
                if style.box_sizing == BoxSizing::BorderBox {
                    (px - vertical_pb(padding, border)).max(0.0)
                } else {
                    px
                }
            }
            Dimension::Percentage(pct) => {
                containing_height.map_or(content_height, |ch| {
                    let resolved = ch * pct / 100.0;
                    if style.box_sizing == BoxSizing::BorderBox {
                        (resolved - vertical_pb(padding, border)).max(0.0)
                    } else {
                        resolved
                    }
                })
            }
            _ => content_height,
        }
    };

    // Apply min-height / max-height constraints.
    let ch = containing_height.unwrap_or(0.0);
    let mut min_h = resolve_min_max(style.min_height, ch, 0.0);
    let mut max_h = resolve_min_max(style.max_height, ch, f32::INFINITY);
    if style.box_sizing == BoxSizing::BorderBox && !is_replaced {
        adjust_min_max_for_border_box(&mut min_h, &mut max_h, vertical_pb(padding, border));
    }
    height = clamp_min_max(height, min_h, max_h);
    height
}

/// Returns `true` if the display value establishes a block-level box.
// TODO: InlineBlock should participate in inline formatting
// context (CSS 2.1 §9.2.2), not force block context.
fn is_block_level(display: Display) -> bool {
    matches!(
        display,
        Display::Block
            | Display::InlineBlock
            | Display::Flex
            | Display::InlineFlex
            | Display::ListItem
    )
}

/// Returns `true` if any child is block-level (block formatting context).
///
/// When this returns `true` and inline children are also present, inline
/// content is currently skipped. CSS 2.1 §9.2.1.1 requires wrapping
/// consecutive inline runs in anonymous block boxes.
// TODO: generate anonymous block boxes for mixed block/inline content.
fn children_are_block(dom: &EcsDom, children: &[Entity]) -> bool {
    children.iter().any(|&child| {
        crate::try_get_style(dom, child).is_some_and(|s| is_block_level(s.display))
    })
}

/// Layout a block-level element, inserting `LayoutBox` on it and all descendants.
pub(crate) fn layout_block(
    dom: &mut EcsDom,
    entity: Entity,
    containing_width: f32,
    offset_x: f32,
    offset_y: f32,
    font_db: &FontDatabase,
) -> LayoutBox {
    layout_block_inner(
        dom,
        entity,
        containing_width,
        None,
        offset_x,
        offset_y,
        font_db,
        0,
    )
}

/// Layout a block-level element with an explicit containing height.
///
/// Used when the parent has a definite height (e.g. `height: 500px`) so that
/// children with `height: 50%` can be resolved.
pub(crate) fn layout_block_with_height(
    dom: &mut EcsDom,
    entity: Entity,
    containing_width: f32,
    containing_height: Option<f32>,
    offset_x: f32,
    offset_y: f32,
    font_db: &FontDatabase,
) -> LayoutBox {
    layout_block_inner(
        dom,
        entity,
        containing_width,
        containing_height,
        offset_x,
        offset_y,
        font_db,
        0,
    )
}

/// Inner recursive implementation with depth tracking.
///
/// If the entity is a flex container (`display: Flex/InlineFlex`), delegates
/// to [`crate::flex::layout_flex`] so that its children are laid out as flex
/// items rather than block children.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
fn layout_block_inner(
    dom: &mut EcsDom,
    entity: Entity,
    containing_width: f32,
    containing_height: Option<f32>,
    offset_x: f32,
    offset_y: f32,
    font_db: &FontDatabase,
    depth: u32,
) -> LayoutBox {
    let style = crate::get_style(dom, entity);

    // A flex container reached via layout_block (e.g. a flex item that is itself
    // a flex container) must use the flex algorithm for its own children.
    if matches!(style.display, Display::Flex | Display::InlineFlex) {
        return crate::flex::layout_flex(
            dom,
            entity,
            containing_width,
            containing_height,
            offset_x,
            offset_y,
            font_db,
            depth,
        );
    }

    // --- Sanitize padding and border (protect against NaN/infinity/negative) ---
    let padding = sanitize_padding(&style);
    let border = sanitize_border(&style);

    // --- Resolve margins ---
    let margin_top = resolve_margin(style.margin_top, containing_width);
    let margin_bottom = resolve_margin(style.margin_bottom, containing_width);

    // --- Check for replaced element (e.g. <img> with decoded ImageData) ---
    let intrinsic = dom
        .world()
        .get::<&ImageData>(entity)
        .ok()
        .map(|img| (img.width, img.height));

    // --- Resolve width ---
    let margin_left_raw = resolve_margin(style.margin_left, containing_width);
    let margin_right_raw = resolve_margin(style.margin_right, containing_width);
    let horizontal_extra = margin_left_raw + margin_right_raw + horizontal_pb(&padding, &border);
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
            content_width = (content_width - horizontal_pb(&padding, &border)).max(0.0);
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
            adjust_min_max_for_border_box(&mut min_w, &mut max_w, horizontal_pb(&padding, &border));
        }
        content_width = clamp_min_max(content_width, min_w, max_w);
    }

    // --- Horizontal margin auto centering ---
    let used_horizontal = content_width + horizontal_pb(&padding, &border);
    let (margin_left, margin_right) =
        if matches!(style.width, Dimension::Auto) && intrinsic.is_none() {
            (margin_left_raw, margin_right_raw)
        } else {
            apply_margin_auto_centering(&style, containing_width, used_horizontal)
        };

    // --- Content rect position ---
    let content_x = offset_x + margin_left + border.left + padding.left;
    let mut content_y = offset_y + margin_top + border.top + padding.top;

    // --- Layout children (stop recursion at depth limit) ---
    let children = dom.children(entity);
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
        let result = stack_block_children(
            dom,
            &children,
            content_width,
            child_containing_height,
            content_x,
            content_y,
            font_db,
            depth + 1,
        );

        // Parent-child margin collapse (CSS 2.1 §8.3.1):
        // First child's top margin collapses with parent's top margin
        // when parent has no border-top and no padding-top.
        if border.top == 0.0 && padding.top == 0.0 {
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
        // when parent has no border-bottom and no padding-bottom and
        // the parent's height is auto (CSS 2.1 §8.3.1).
        if border.bottom == 0.0 && padding.bottom == 0.0 && matches!(style.height, Dimension::Auto)
        {
            if let Some(last_mb) = result.last_child_margin_bottom {
                collapsed_margin_bottom = collapse_margins(margin_bottom, last_mb);
            }
        }

        result.height
    } else {
        layout_inline_context(dom, &children, content_width, &style, font_db)
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
        content: Rect {
            x: content_x,
            y: content_y,
            width: content_width,
            height,
        },
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

/// Resolve width for a replaced element (e.g. `<img>`).
///
/// CSS 2.1 §10.3.2: replaced elements with `width: auto` use intrinsic width.
/// When only height is specified, width is computed from the aspect ratio.
#[allow(clippy::cast_precision_loss)]
fn resolve_replaced_width(
    style: &ComputedStyle,
    containing_width: f32,
    intrinsic_w: u32,
    intrinsic_h: u32,
    padding: &EdgeSizes,
    border: &EdgeSizes,
) -> f32 {
    let iw = intrinsic_w as f32;
    let ih = intrinsic_h as f32;

    if style.width == Dimension::Auto {
        match style.height {
            Dimension::Length(h) if h.is_finite() && ih > 0.0 => {
                // height specified, width auto: compute from aspect ratio.
                let css_h = if style.box_sizing == BoxSizing::BorderBox {
                    (h - vertical_pb(padding, border)).max(0.0)
                } else {
                    h
                };
                (css_h * iw / ih).max(0.0)
            }
            _ => iw, // Both auto or height auto: use intrinsic width.
        }
    } else {
        let raw = sanitize(resolve_dimension_value(style.width, containing_width, iw));
        if style.box_sizing == BoxSizing::BorderBox {
            (raw - horizontal_pb(padding, border)).max(0.0)
        } else {
            raw
        }
    }
}

/// Resolve height for a replaced element (e.g. `<img>`).
///
/// CSS 2.1 §10.6.2: replaced elements with `height: auto` use intrinsic height.
/// When only width is specified, height is computed from the aspect ratio.
#[allow(clippy::cast_precision_loss)]
fn resolve_replaced_height(
    style: &ComputedStyle,
    used_width: f32,
    intrinsic_w: u32,
    intrinsic_h: u32,
    padding: &EdgeSizes,
    border: &EdgeSizes,
) -> f32 {
    let iw = intrinsic_w as f32;
    let ih = intrinsic_h as f32;

    match style.height {
        Dimension::Auto => {
            if !matches!(style.width, Dimension::Auto) && iw > 0.0 {
                // width specified, height auto: compute from aspect ratio.
                (used_width * ih / iw).max(0.0)
            } else {
                ih // Both auto: use intrinsic height.
            }
        }
        Dimension::Length(h) if h.is_finite() => {
            if style.box_sizing == BoxSizing::BorderBox {
                (h - vertical_pb(padding, border)).max(0.0)
            } else {
                h
            }
        }
        _ => ih,
    }
}

/// Shift all block-level children's `LayoutBox.content.y` by `delta`,
/// recursively including descendants.
///
/// Used after parent-child margin collapse to reposition children that were
/// laid out before the collapse was detected.
fn shift_block_children(dom: &mut EcsDom, children: &[Entity], delta: f32) {
    if delta.abs() < f32::EPSILON {
        return;
    }
    for &child in children {
        let is_block =
            crate::try_get_style(dom, child).is_some_and(|s| is_block_level(s.display));
        if !is_block {
            continue;
        }
        if let Ok(mut lb) = dom.world_mut().get::<&mut LayoutBox>(child) {
            lb.content.y += delta;
        }
        // Recurse into descendants so nested layout positions stay consistent.
        let grandchildren = dom.children(child);
        if !grandchildren.is_empty() {
            shift_block_children(dom, &grandchildren, delta);
        }
    }
}

/// Result of stacking block children, including margin info for parent-child collapse.
pub(crate) struct StackResult {
    /// Total content height consumed by stacked children.
    pub height: f32,
    /// Top margin of the first block child (for parent-child collapse).
    pub first_child_margin_top: Option<f32>,
    /// Bottom margin of the last block child (for parent-child collapse).
    pub last_child_margin_bottom: Option<f32>,
}

/// Stack block-level children with vertical margin collapse.
///
/// Shared by block children layout and document-root layout. Returns
/// the total height consumed and first/last child margin info for
/// parent-child collapse (CSS 2.1 §8.3.1).
#[allow(clippy::too_many_arguments)]
pub(crate) fn stack_block_children(
    dom: &mut EcsDom,
    children: &[Entity],
    containing_width: f32,
    containing_height: Option<f32>,
    offset_x: f32,
    offset_y: f32,
    font_db: &FontDatabase,
    depth: u32,
) -> StackResult {
    let mut cursor_y = offset_y;
    let mut prev_margin_bottom: Option<f32> = None;
    let mut first_child_margin_top: Option<f32> = None;
    let mut last_child_margin_bottom: Option<f32> = None;

    for &child in children {
        let Some(child_style) = crate::try_get_style(dom, child) else {
            continue; // text node in block context: skip
        };
        let (child_display, child_margin_top_dim) = (child_style.display, child_style.margin_top);

        if !is_block_level(child_display) {
            continue;
        }

        // Margin collapse between adjacent block siblings (CSS 2.1 §8.3.1).
        // Both positive → max, both negative → min, mixed → sum.
        let child_margin_top = resolve_margin(child_margin_top_dim, containing_width);
        if first_child_margin_top.is_none() {
            first_child_margin_top = Some(child_margin_top);
        }
        if let Some(prev_mb) = prev_margin_bottom {
            let collapsed = collapse_margins(prev_mb, child_margin_top);
            cursor_y -= prev_mb + child_margin_top - collapsed;
        }

        // layout_block_inner handles flex dispatch internally
        // (Flex/InlineFlex containers are routed to layout_flex).
        let child_box = layout_block_inner(
            dom,
            child,
            containing_width,
            containing_height,
            offset_x,
            cursor_y,
            font_db,
            depth,
        );
        cursor_y += child_box.margin_box().height;
        prev_margin_bottom = Some(child_box.margin.bottom);
        last_child_margin_bottom = Some(child_box.margin.bottom);
    }

    StackResult {
        height: cursor_y - offset_y,
        first_child_margin_top,
        last_child_margin_bottom,
    }
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use elidex_ecs::Attributes;

    fn block_style() -> ComputedStyle {
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        }
    }

    fn make_dom_with_block_div(style: ComputedStyle) -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        dom.world_mut().insert_one(div, style);
        (dom, div)
    }

    #[test]
    fn width_auto_fills_containing_block() {
        let (mut dom, div) = make_dom_with_block_div(block_style());
        let font_db = FontDatabase::new();

        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        assert!((lb.content.width - 800.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fixed_width_with_padding_border() {
        let style = ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(200.0),
            padding_left: 10.0,
            padding_right: 10.0,
            border_left_width: 2.0,
            border_right_width: 2.0,
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();

        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        assert!((lb.content.width - 200.0).abs() < f32::EPSILON);
        assert!((lb.padding.left - 10.0).abs() < f32::EPSILON);
        assert!((lb.border.left - 2.0).abs() < f32::EPSILON);
        // border box width = 200 + 10 + 10 + 2 + 2 = 224
        let bb = lb.border_box();
        assert!((bb.width - 224.0).abs() < f32::EPSILON);
    }

    #[test]
    fn margin_auto_centering() {
        let style = ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(400.0),
            margin_left: Dimension::Auto,
            margin_right: Dimension::Auto,
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();

        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        assert!((lb.content.width - 400.0).abs() < f32::EPSILON);
        assert!((lb.margin.left - 200.0).abs() < f32::EPSILON);
        assert!((lb.margin.right - 200.0).abs() < f32::EPSILON);
    }

    #[test]
    fn vertical_stacking_two_divs() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let child1 = dom.create_element("div", Attributes::default());
        let child2 = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child1);
        dom.append_child(parent, child2);

        let child_style = ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(50.0),
            ..Default::default()
        };

        dom.world_mut().insert_one(parent, block_style());
        dom.world_mut().insert_one(child1, child_style.clone());
        dom.world_mut().insert_one(child2, child_style);

        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
        assert!((lb.content.height - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn display_none_excluded() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let visible = dom.create_element("div", Attributes::default());
        let hidden = dom.create_element("div", Attributes::default());
        dom.append_child(parent, visible);
        dom.append_child(parent, hidden);

        dom.world_mut().insert_one(parent, block_style());
        dom.world_mut().insert_one(
            visible,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(50.0),
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            hidden,
            ComputedStyle {
                display: Display::None,
                height: Dimension::Length(100.0),
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
        assert!((lb.content.height - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn margin_collapse_adjacent_siblings() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let child1 = dom.create_element("div", Attributes::default());
        let child2 = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child1);
        dom.append_child(parent, child2);

        dom.world_mut().insert_one(parent, block_style());
        dom.world_mut().insert_one(
            child1,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(40.0),
                margin_bottom: Dimension::Length(20.0),
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            child2,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(40.0),
                margin_top: Dimension::Length(30.0),
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
        // Without collapse: 40 + 20 + 30 + 40 = 130
        // With collapse: 40 + max(20,30) + 40 = 110
        assert!((lb.content.height - 110.0).abs() < f32::EPSILON);
    }

    #[test]
    fn margin_collapse_negative() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let child1 = dom.create_element("div", Attributes::default());
        let child2 = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child1);
        dom.append_child(parent, child2);

        dom.world_mut().insert_one(parent, block_style());
        // Both negative: collapsed = min(-10, -20) = -20
        dom.world_mut().insert_one(
            child1,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(40.0),
                margin_bottom: Dimension::Length(-10.0),
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            child2,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(40.0),
                margin_top: Dimension::Length(-20.0),
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
        // Without collapse: 40 + (-10) + (-20) + 40 = 50
        // With collapse (both neg): 40 + min(-10,-20) + 40 = 40 + (-20) + 40 = 60
        assert!((lb.content.height - 60.0).abs() < f32::EPSILON);
    }

    #[test]
    fn margin_collapse_mixed() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let child1 = dom.create_element("div", Attributes::default());
        let child2 = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child1);
        dom.append_child(parent, child2);

        dom.world_mut().insert_one(parent, block_style());
        // Mixed: collapsed = 20 + (-10) = 10
        dom.world_mut().insert_one(
            child1,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(40.0),
                margin_bottom: Dimension::Length(20.0),
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            child2,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(40.0),
                margin_top: Dimension::Length(-10.0),
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
        // Without collapse: 40 + 20 + (-10) + 40 = 90
        // With collapse (mixed): 40 + (20 + (-10)) + 40 = 90
        // Actually same here because sum == collapse for mixed.
        // But the key difference: old code did max(20, -10) = 20, giving 100.
        assert!((lb.content.height - 90.0).abs() < f32::EPSILON);
    }

    #[test]
    fn margin_auto_overconstrained() {
        // CSS 2.1 §10.3.3: overconstrained LTR — margin-left=0, margin-right absorbs overflow.
        let style = ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(900.0),
            margin_left: Dimension::Auto,
            margin_right: Dimension::Auto,
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();

        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        assert!((lb.content.width - 900.0).abs() < f32::EPSILON);
        // margin-left = 0 (overconstrained LTR)
        assert!(lb.margin.left.abs() < f32::EPSILON);
        // margin-right = 800 - 900 = -100
        assert!((lb.margin.right - (-100.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn overconstrained_non_auto_margins() {
        // CSS 2.1 §10.3.3: no auto margins, overconstrained LTR
        // margin-right recalculated to satisfy constraint equation.
        let style = ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(900.0),
            margin_left: Dimension::Length(10.0),
            margin_right: Dimension::Length(10.0),
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();

        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        assert!((lb.content.width - 900.0).abs() < f32::EPSILON);
        assert!((lb.margin.left - 10.0).abs() < f32::EPSILON);
        // margin-right = 800 - 900 - 10 = -110
        assert!((lb.margin.right - (-110.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn percentage_width_and_margin() {
        let style = ComputedStyle {
            display: Display::Block,
            width: Dimension::Percentage(50.0),
            margin_left: Dimension::Percentage(10.0),
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();

        let lb = layout_block(&mut dom, div, 1000.0, 0.0, 0.0, &font_db);
        assert!((lb.content.width - 500.0).abs() < f32::EPSILON);
        assert!((lb.margin.left - 100.0).abs() < f32::EPSILON);
    }

    // --- M3-2: box-sizing: border-box ---

    #[test]
    fn box_sizing_content_box_default() {
        // Default content-box: content_width = specified width.
        let style = ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(200.0),
            padding_left: 10.0,
            padding_right: 10.0,
            border_left_width: 2.0,
            border_right_width: 2.0,
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();

        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        assert!((lb.content.width - 200.0).abs() < f32::EPSILON);
        // border box = 200 + 20 + 4 = 224
        let bb = lb.border_box();
        assert!((bb.width - 224.0).abs() < f32::EPSILON);
    }

    #[test]
    fn box_sizing_border_box_width() {
        // border-box: specified 200px includes padding + border.
        let style = ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(200.0),
            padding_left: 10.0,
            padding_right: 10.0,
            border_left_width: 2.0,
            border_right_width: 2.0,
            box_sizing: BoxSizing::BorderBox,
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();

        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        // content = 200 - 10 - 10 - 2 - 2 = 176
        assert!((lb.content.width - 176.0).abs() < f32::EPSILON);
        // border box = 176 + 20 + 4 = 200
        let bb = lb.border_box();
        assert!((bb.width - 200.0).abs() < f32::EPSILON);
    }

    #[test]
    fn box_sizing_border_box_height() {
        let style = ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(100.0),
            padding_top: 10.0,
            padding_bottom: 10.0,
            border_top_width: 2.0,
            border_bottom_width: 2.0,
            box_sizing: BoxSizing::BorderBox,
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();

        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        // content height = 100 - 10 - 10 - 2 - 2 = 76
        assert!((lb.content.height - 76.0).abs() < f32::EPSILON);
    }

    #[test]
    fn box_sizing_border_box_percentage_width() {
        let style = ComputedStyle {
            display: Display::Block,
            width: Dimension::Percentage(50.0), // 50% of 800 = 400
            padding_left: 20.0,
            padding_right: 20.0,
            box_sizing: BoxSizing::BorderBox,
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();

        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        // content = 400 - 20 - 20 = 360
        assert!((lb.content.width - 360.0).abs() < f32::EPSILON);
    }

    #[test]
    fn box_sizing_border_box_auto_width_unchanged() {
        // auto width should not be affected by border-box.
        let style = ComputedStyle {
            display: Display::Block,
            width: Dimension::Auto,
            padding_left: 20.0,
            padding_right: 20.0,
            box_sizing: BoxSizing::BorderBox,
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();

        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        // auto: content_width = 800 - 20 - 20 = 760 (no border-box subtraction).
        assert!((lb.content.width - 760.0).abs() < f32::EPSILON);
    }

    // --- M3-4: replaced element (image) layout ---

    fn make_dom_with_image(style: ComputedStyle, img_w: u32, img_h: u32) -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let img = dom.create_element("img", Attributes::default());
        dom.world_mut().insert_one(img, style);
        dom.world_mut().insert_one(
            img,
            ImageData {
                pixels: Arc::new(vec![0u8; (img_w * img_h * 4) as usize]),
                width: img_w,
                height: img_h,
            },
        );
        (dom, img)
    }

    #[test]
    fn replaced_element_intrinsic_size() {
        // width:auto, height:auto → use intrinsic dimensions.
        let style = ComputedStyle {
            display: Display::Block,
            ..Default::default()
        };
        let (mut dom, img) = make_dom_with_image(style, 200, 100);
        let font_db = FontDatabase::new();

        let lb = layout_block(&mut dom, img, 800.0, 0.0, 0.0, &font_db);
        assert!((lb.content.width - 200.0).abs() < f32::EPSILON);
        assert!((lb.content.height - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn replaced_element_css_width_aspect_ratio() {
        // width:300px, height:auto → height computed from aspect ratio.
        let style = ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(300.0),
            ..Default::default()
        };
        let (mut dom, img) = make_dom_with_image(style, 200, 100);
        let font_db = FontDatabase::new();

        let lb = layout_block(&mut dom, img, 800.0, 0.0, 0.0, &font_db);
        assert!((lb.content.width - 300.0).abs() < f32::EPSILON);
        // height = 300 * 100/200 = 150
        assert!((lb.content.height - 150.0).abs() < f32::EPSILON);
    }

    #[test]
    fn replaced_element_css_height_aspect_ratio() {
        // width:auto, height:200px → width computed from aspect ratio.
        let style = ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(200.0),
            ..Default::default()
        };
        let (mut dom, img) = make_dom_with_image(style, 300, 100);
        let font_db = FontDatabase::new();

        let lb = layout_block(&mut dom, img, 800.0, 0.0, 0.0, &font_db);
        // width = 200 * 300/100 = 600
        assert!((lb.content.width - 600.0).abs() < f32::EPSILON);
        assert!((lb.content.height - 200.0).abs() < f32::EPSILON);
    }

    #[test]
    fn replaced_element_both_dimensions_specified() {
        // width:400px, height:300px → both used as-is.
        let style = ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(400.0),
            height: Dimension::Length(300.0),
            ..Default::default()
        };
        let (mut dom, img) = make_dom_with_image(style, 200, 100);
        let font_db = FontDatabase::new();

        let lb = layout_block(&mut dom, img, 800.0, 0.0, 0.0, &font_db);
        assert!((lb.content.width - 400.0).abs() < f32::EPSILON);
        assert!((lb.content.height - 300.0).abs() < f32::EPSILON);
    }

    #[test]
    fn replaced_element_border_box() {
        // box-sizing: border-box with padding.
        let style = ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(220.0),
            height: Dimension::Length(120.0),
            padding_left: 10.0,
            padding_right: 10.0,
            padding_top: 10.0,
            padding_bottom: 10.0,
            box_sizing: BoxSizing::BorderBox,
            ..Default::default()
        };
        let (mut dom, img) = make_dom_with_image(style, 200, 100);
        let font_db = FontDatabase::new();

        let lb = layout_block(&mut dom, img, 800.0, 0.0, 0.0, &font_db);
        // content = 220 - 10 - 10 = 200
        assert!((lb.content.width - 200.0).abs() < f32::EPSILON);
        // content height = 120 - 10 - 10 = 100
        assert!((lb.content.height - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn no_image_data_normal_layout() {
        // Element without ImageData → normal block layout.
        let style = ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(400.0),
            height: Dimension::Length(200.0),
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();

        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        assert!((lb.content.width - 400.0).abs() < f32::EPSILON);
        assert!((lb.content.height - 200.0).abs() < f32::EPSILON);
    }

    // --- M3-5: Parent-child margin collapse ---

    #[test]
    fn parent_child_first_child_margin_collapse() {
        // Parent margin-top=10, first child margin-top=20, no border/padding.
        // Collapsed margin = max(10, 20) = 20.
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child);
        dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                margin_top: Dimension::Length(10.0),
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                margin_top: Dimension::Length(20.0),
                height: Dimension::Length(50.0),
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
        // Parent's margin-top should collapse with child's: max(10, 20) = 20.
        assert!(
            (lb.margin.top - 20.0).abs() < f32::EPSILON,
            "expected collapsed margin-top=20, got {}",
            lb.margin.top
        );
        // Child's content.y should reflect the collapsed margin (20), not original (10).
        let child_lb = dom.world().get::<&LayoutBox>(child).unwrap();
        assert!(
            (child_lb.content.y - 20.0).abs() < f32::EPSILON,
            "expected child content.y=20 (collapsed margin), got {}",
            child_lb.content.y
        );
    }

    #[test]
    fn parent_child_margin_collapse_shifts_grandchildren() {
        // Parent margin-top=10, child margin-top=20, grandchild inside child.
        // After collapse, both child and grandchild must shift.
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_element("div", Attributes::default());
        let grandchild = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child);
        dom.append_child(child, grandchild);
        dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                margin_top: Dimension::Length(10.0),
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                margin_top: Dimension::Length(20.0),
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            grandchild,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(30.0),
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
        assert!(
            (lb.margin.top - 20.0).abs() < f32::EPSILON,
            "collapsed margin-top should be 20, got {}",
            lb.margin.top
        );
        let child_lb = dom.world().get::<&LayoutBox>(child).unwrap();
        let grandchild_lb = dom.world().get::<&LayoutBox>(grandchild).unwrap();
        // Grandchild should be inside child, which is at content.y=20.
        assert!(
            (grandchild_lb.content.y - child_lb.content.y).abs() < f32::EPSILON,
            "grandchild should be at child's content.y={}, got {}",
            child_lb.content.y,
            grandchild_lb.content.y
        );
    }

    #[test]
    fn parent_child_last_child_margin_collapse() {
        // Parent margin-bottom=5, last child margin-bottom=15, no border/padding,
        // height:auto → bottom margin collapses.
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child);
        dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                margin_bottom: Dimension::Length(5.0),
                // height defaults to Auto
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                margin_bottom: Dimension::Length(15.0),
                height: Dimension::Length(50.0),
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
        // Parent's margin-bottom should collapse with child's: max(5, 15) = 15.
        assert!(
            (lb.margin.bottom - 15.0).abs() < f32::EPSILON,
            "expected collapsed margin-bottom=15, got {}",
            lb.margin.bottom
        );
        // Child's content.y should be at top (no top margin on either).
        let child_lb = dom.world().get::<&LayoutBox>(child).unwrap();
        assert!(
            child_lb.content.y.abs() < f32::EPSILON,
            "expected child content.y=0, got {}",
            child_lb.content.y
        );
    }

    #[test]
    fn parent_child_no_bottom_collapse_with_explicit_height() {
        // Parent has height: 200px → bottom margin does NOT collapse (CSS 2.1 §8.3.1).
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child);
        dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                margin_bottom: Dimension::Length(5.0),
                height: Dimension::Length(200.0),
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                margin_bottom: Dimension::Length(15.0),
                height: Dimension::Length(50.0),
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
        // height is explicit → no bottom collapse. Parent keeps its own margin-bottom.
        assert!(
            (lb.margin.bottom - 5.0).abs() < f32::EPSILON,
            "expected margin-bottom=5 (no collapse), got {}",
            lb.margin.bottom
        );
    }

    #[test]
    fn parent_child_no_collapse_with_border() {
        // Parent has border-top > 0 → no first-child collapse.
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child);
        dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                margin_top: Dimension::Length(10.0),
                border_top_width: 1.0,
                border_top_style: elidex_plugin::BorderStyle::Solid,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                margin_top: Dimension::Length(20.0),
                height: Dimension::Length(50.0),
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
        // border-top prevents collapse: parent keeps its own margin-top.
        assert!(
            (lb.margin.top - 10.0).abs() < f32::EPSILON,
            "expected margin-top=10 (no collapse), got {}",
            lb.margin.top
        );
    }

    #[test]
    fn parent_child_no_collapse_with_padding() {
        // Parent has padding-top > 0 → no first-child collapse.
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child);
        dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                margin_top: Dimension::Length(10.0),
                padding_top: 5.0,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                margin_top: Dimension::Length(20.0),
                height: Dimension::Length(50.0),
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);
        // padding-top prevents collapse: parent keeps its own margin-top.
        assert!(
            (lb.margin.top - 10.0).abs() < f32::EPSILON,
            "expected margin-top=10 (no collapse), got {}",
            lb.margin.top
        );
    }

    // --- M3-5: Percentage heights ---

    #[test]
    fn percentage_height_with_definite_parent() {
        // Parent height=200, child height=50% → 100.
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child);
        dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(200.0),
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Percentage(50.0),
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);

        let child_lb = dom
            .world()
            .get::<&LayoutBox>(child)
            .map(|lb| (*lb).clone())
            .expect("child LayoutBox");
        assert!(
            (child_lb.content.height - 100.0).abs() < f32::EPSILON,
            "expected height=100 (50% of 200), got {}",
            child_lb.content.height
        );
    }

    #[test]
    fn percentage_height_without_definite_parent() {
        // Parent height=auto, child height=50% → falls back to auto (content height).
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(parent, child);
        dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                // height: Auto (default)
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Percentage(50.0),
                // No content → height = 0.
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        layout_block(&mut dom, parent, 800.0, 0.0, 0.0, &font_db);

        let child_lb = dom
            .world()
            .get::<&LayoutBox>(child)
            .map(|lb| (*lb).clone())
            .expect("child LayoutBox");
        // Auto parent → percentage height unresolvable → auto → content height (0).
        assert!(
            child_lb.content.height.abs() < f32::EPSILON,
            "expected height=0 (auto fallback), got {}",
            child_lb.content.height
        );
    }

    #[test]
    fn percentage_height_nested_blocks() {
        // Grandparent height=400, parent height=50% (=200), child height=50% (=100).
        let mut dom = EcsDom::new();
        let gp = dom.create_element("div", Attributes::default());
        let parent = dom.create_element("div", Attributes::default());
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(gp, parent);
        dom.append_child(parent, child);
        dom.world_mut().insert_one(
            gp,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(400.0),
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Percentage(50.0),
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Percentage(50.0),
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        layout_block(&mut dom, gp, 800.0, 0.0, 0.0, &font_db);

        let parent_lb = dom
            .world()
            .get::<&LayoutBox>(parent)
            .map(|lb| (*lb).clone())
            .expect("parent LayoutBox");
        let child_lb = dom
            .world()
            .get::<&LayoutBox>(child)
            .map(|lb| (*lb).clone())
            .expect("child LayoutBox");
        assert!(
            (parent_lb.content.height - 200.0).abs() < f32::EPSILON,
            "parent height = 50% of 400 = 200, got {}",
            parent_lb.content.height
        );
        assert!(
            (child_lb.content.height - 100.0).abs() < f32::EPSILON,
            "child height = 50% of 200 = 100, got {}",
            child_lb.content.height
        );
    }

    // --- M3-6: min-width / max-width / min-height / max-height ---

    #[test]
    fn min_width_constrains_auto() {
        // width: auto in 800px container, min-width: 900px → width = 900.
        let style = ComputedStyle {
            display: Display::Block,
            min_width: Dimension::Length(900.0),
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        assert!(
            (lb.content.width - 900.0).abs() < 1.0,
            "min-width should force width to 900, got {}",
            lb.content.width
        );
    }

    #[test]
    fn max_width_constrains_auto() {
        // width: auto in 800px container, max-width: 500px → width = 500.
        let style = ComputedStyle {
            display: Display::Block,
            max_width: Dimension::Length(500.0),
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        assert!(
            (lb.content.width - 500.0).abs() < 1.0,
            "max-width should limit width to 500, got {}",
            lb.content.width
        );
    }

    #[test]
    fn min_width_overrides_max_width() {
        // CSS spec: when min > max, min wins.
        let style = ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(400.0),
            min_width: Dimension::Length(600.0),
            max_width: Dimension::Length(500.0),
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        assert!(
            (lb.content.width - 600.0).abs() < 1.0,
            "min-width wins over max-width, got {}",
            lb.content.width
        );
    }

    #[test]
    fn max_width_none_is_unconstrained() {
        // max-width: Auto (=none) should not constrain.
        let style = ComputedStyle {
            display: Display::Block,
            max_width: Dimension::Auto,
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        assert!(
            (lb.content.width - 800.0).abs() < 1.0,
            "max-width: none should not constrain, got {}",
            lb.content.width
        );
    }

    #[test]
    fn min_height_constrains_auto() {
        // Block with no children → auto height = 0, min-height: 200px → height = 200.
        let style = ComputedStyle {
            display: Display::Block,
            min_height: Dimension::Length(200.0),
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        assert!(
            (lb.content.height - 200.0).abs() < 1.0,
            "min-height should force height to 200, got {}",
            lb.content.height
        );
    }

    #[test]
    fn max_height_constrains_explicit() {
        // height: 400px, max-height: 300px → height = 300.
        let style = ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(400.0),
            max_height: Dimension::Length(300.0),
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        assert!(
            (lb.content.height - 300.0).abs() < 1.0,
            "max-height should limit height to 300, got {}",
            lb.content.height
        );
    }

    #[test]
    fn min_width_percentage() {
        // min-width: 50% of 800px = 400px, width: 200px → constrained to 400.
        let style = ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(200.0),
            min_width: Dimension::Percentage(50.0),
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        assert!(
            (lb.content.width - 400.0).abs() < 1.0,
            "min-width 50% of 800 = 400, got {}",
            lb.content.width
        );
    }

    // --- M3-6: Display::ListItem is block-level ---

    #[test]
    fn list_item_is_block_level() {
        assert!(is_block_level(Display::ListItem));
    }

    // --- L15: border-box + min/max ---

    #[test]
    fn min_width_border_box_subtracts_padding() {
        // border-box: min-width: 200px with 20px padding each side.
        // Content min-width = 200 - 40 = 160. width: auto → fills 800 - 40 = 760 > 160, no effect.
        // width: 100px → content 100, but min-width 160 wins.
        let style = ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(100.0),
            min_width: Dimension::Length(200.0),
            box_sizing: BoxSizing::BorderBox,
            padding_left: 20.0,
            padding_right: 20.0,
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        // border-box width: 100px, padding: 40px → content = 60px.
        // border-box min-width: 200px → content min = 200 - 40 = 160.
        // Final content width = max(60, 160) = 160.
        assert!(
            (lb.content.width - 160.0).abs() < 1.0,
            "border-box min-width should subtract padding, got {}",
            lb.content.width
        );
    }

    #[test]
    fn max_width_border_box_subtracts_padding() {
        // border-box: max-width: 200px with 20px padding each side.
        // Content max-width = 200 - 40 = 160.
        let style = ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(300.0),
            max_width: Dimension::Length(200.0),
            box_sizing: BoxSizing::BorderBox,
            padding_left: 20.0,
            padding_right: 20.0,
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
        let font_db = FontDatabase::new();
        let lb = layout_block(&mut dom, div, 800.0, 0.0, 0.0, &font_db);
        // border-box width: 300px → content = 260px.
        // border-box max-width: 200px → content max = 160.
        // Final content width = min(260, 160) = 160.
        assert!(
            (lb.content.width - 160.0).abs() < 1.0,
            "border-box max-width should subtract padding, got {}",
            lb.content.width
        );
    }
}
