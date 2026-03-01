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
use crate::{resolve_dimension_value, sanitize_edge_values, MAX_LAYOUT_DEPTH};

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

/// Returns `true` if the display value establishes a block-level box.
// TODO(Phase 2): InlineBlock should participate in inline formatting
// context (CSS 2.1 §9.2.2), not force block context.
fn is_block_level(display: Display) -> bool {
    matches!(
        display,
        Display::Block | Display::InlineBlock | Display::Flex | Display::InlineFlex
    )
}

/// Returns `true` if any child is block-level (block formatting context).
///
/// When this returns `true` and inline children are also present, inline
/// content is currently skipped. CSS 2.1 §9.2.1.1 requires wrapping
/// consecutive inline runs in anonymous block boxes — this is deferred to
/// Phase 2.
// TODO(Phase 2): generate anonymous block boxes for mixed block/inline content.
fn children_are_block(dom: &EcsDom, children: &[Entity]) -> bool {
    children.iter().any(|&child| {
        dom.world()
            .get::<&ComputedStyle>(child)
            .is_ok_and(|s| is_block_level(s.display))
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
#[allow(clippy::too_many_lines)]
fn layout_block_inner(
    dom: &mut EcsDom,
    entity: Entity,
    containing_width: f32,
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
            offset_x,
            offset_y,
            font_db,
            depth,
        );
    }

    // --- Sanitize padding and border (protect against NaN/infinity/negative) ---
    let padding = sanitize_edge_values(
        style.padding_top,
        style.padding_right,
        style.padding_bottom,
        style.padding_left,
    );
    let border = sanitize_edge_values(
        style.border_top_width,
        style.border_right_width,
        style.border_bottom_width,
        style.border_left_width,
    );

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
    let horizontal_extra = margin_left_raw
        + margin_right_raw
        + padding.left
        + padding.right
        + border.left
        + border.right;
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
            let pb_horizontal = padding.left + padding.right + border.left + border.right;
            content_width = (content_width - pb_horizontal).max(0.0);
        }
    }

    // --- Horizontal margin auto centering ---
    let used_horizontal = content_width + padding.left + padding.right + border.left + border.right;
    let (margin_left, margin_right) =
        if matches!(style.width, Dimension::Auto) && intrinsic.is_none() {
            (margin_left_raw, margin_right_raw)
        } else {
            apply_margin_auto_centering(&style, containing_width, used_horizontal)
        };

    // --- Content rect position ---
    let content_x = offset_x + margin_left + border.left + padding.left;
    let content_y = offset_y + margin_top + border.top + padding.top;

    // --- Layout children (stop recursion at depth limit) ---
    let children = dom.children(entity);
    let content_height = if let Some((iw, ih)) = intrinsic {
        // Replaced element: use intrinsic/CSS height, no child layout.
        resolve_replaced_height(&style, content_width, iw, ih, &padding, &border)
    } else if children.is_empty() || depth >= MAX_LAYOUT_DEPTH {
        0.0
    } else if children_are_block(dom, &children) {
        stack_block_children(
            dom,
            &children,
            content_width,
            content_x,
            content_y,
            font_db,
            depth + 1,
        )
    } else {
        layout_inline_context(dom, &children, content_width, &style, font_db)
    };

    let height = if intrinsic.is_some() {
        // Replaced element height already resolved above.
        content_height
    } else {
        match style.height {
            Dimension::Length(px) if px.is_finite() => {
                if style.box_sizing == BoxSizing::BorderBox {
                    let pb_vertical = padding.top + padding.bottom + border.top + border.bottom;
                    (px - pb_vertical).max(0.0)
                } else {
                    px
                }
            }
            _ => content_height,
        }
    };

    let lb = LayoutBox {
        content: Rect {
            x: content_x,
            y: content_y,
            width: content_width,
            height,
        },
        padding,
        border,
        margin: EdgeSizes::new(margin_top, margin_right, margin_bottom, margin_left),
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
    let pb_horizontal = padding.left + padding.right + border.left + border.right;

    if style.width == Dimension::Auto {
        match style.height {
            Dimension::Length(h) if h.is_finite() && ih > 0.0 => {
                // height specified, width auto: compute from aspect ratio.
                let css_h = if style.box_sizing == BoxSizing::BorderBox {
                    let pb_vertical = padding.top + padding.bottom + border.top + border.bottom;
                    (h - pb_vertical).max(0.0)
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
            (raw - pb_horizontal).max(0.0)
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
                let pb_vertical = padding.top + padding.bottom + border.top + border.bottom;
                (h - pb_vertical).max(0.0)
            } else {
                h
            }
        }
        _ => ih,
    }
}

/// Stack block-level children with vertical margin collapse.
///
/// Shared by block children layout and document-root layout. Returns
/// the total height consumed (`cursor_y` − `offset_y`).
// TODO(Phase 2): implement parent-child margin collapse (CSS 2.1 §8.3.1)
// when parent has no border-top/padding-top (first child) or
// border-bottom/padding-bottom (last child).
pub(crate) fn stack_block_children(
    dom: &mut EcsDom,
    children: &[Entity],
    containing_width: f32,
    offset_x: f32,
    offset_y: f32,
    font_db: &FontDatabase,
    depth: u32,
) -> f32 {
    let mut cursor_y = offset_y;
    let mut prev_margin_bottom: Option<f32> = None;

    for &child in children {
        let (child_display, child_margin_top_dim) = match dom.world().get::<&ComputedStyle>(child) {
            Ok(s) => (s.display, s.margin_top),
            Err(_) => continue, // text node in block context: skip
        };

        if !is_block_level(child_display) {
            continue;
        }

        // Margin collapse between adjacent block siblings (CSS 2.1 §8.3.1).
        // Both positive → max, both negative → min, mixed → sum.
        let child_margin_top = resolve_margin(child_margin_top_dim, containing_width);
        if let Some(prev_mb) = prev_margin_bottom {
            let collapsed = collapse_margins(prev_mb, child_margin_top);
            cursor_y -= prev_mb + child_margin_top - collapsed;
        }

        let child_box = if matches!(child_display, Display::Flex | Display::InlineFlex) {
            crate::flex::layout_flex(
                dom,
                child,
                containing_width,
                offset_x,
                cursor_y,
                font_db,
                depth,
            )
        } else {
            layout_block_inner(
                dom,
                child,
                containing_width,
                offset_x,
                cursor_y,
                font_db,
                depth,
            )
        };
        cursor_y += child_box.margin_box().height;
        prev_margin_bottom = Some(child_box.margin.bottom);
    }

    cursor_y - offset_y
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
}
