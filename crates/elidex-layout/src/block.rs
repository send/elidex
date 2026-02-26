//! Block formatting context layout algorithm.
//!
//! Computes the CSS box model (content, padding, border, margin) for
//! block-level elements, handling width/height resolution, margin auto
//! centering, and vertical stacking of child blocks.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{ComputedStyle, Dimension, Display, EdgeSizes, LayoutBox, Rect};
use elidex_text::FontDatabase;

use crate::inline::layout_inline_context;

/// Maximum recursion depth for layout. Prevents stack overflow on
/// deeply nested DOMs. Matches elidex-ecs's ancestor walk depth cap.
const MAX_LAYOUT_DEPTH: u32 = 1000;

/// Replace non-finite f32 values (NaN, infinity) with 0.0.
fn sanitize(v: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

/// Resolve a `Dimension` margin value to pixels.
///
/// `Auto` returns 0.0 here; horizontal auto centering is handled separately.
/// Non-finite results are replaced with 0.0.
pub(crate) fn resolve_margin(dim: Dimension, containing_width: f32) -> f32 {
    sanitize(match dim {
        Dimension::Length(px) => px,
        Dimension::Percentage(pct) => containing_width * pct / 100.0,
        Dimension::Auto => 0.0,
    })
}

/// Resolve content width from the `width` property.
///
/// `horizontal_extra` is the sum of resolved margins, padding, and border widths.
/// Non-finite results are replaced with 0.0.
fn resolve_width(style: &ComputedStyle, containing_width: f32, horizontal_extra: f32) -> f32 {
    sanitize(match style.width {
        Dimension::Length(px) => px,
        Dimension::Percentage(pct) => containing_width * pct / 100.0,
        Dimension::Auto => (containing_width - horizontal_extra).max(0.0),
    })
}

/// Resolve content height. Returns `None` for auto/percentage (shrink-to-content).
///
/// Non-finite lengths are treated as auto.
fn resolve_height(style: &ComputedStyle) -> Option<f32> {
    match style.height {
        Dimension::Length(px) if px.is_finite() => Some(px),
        Dimension::Length(_) | Dimension::Percentage(_) | Dimension::Auto => None,
    }
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
            let mr = resolve_margin(style.margin_right, containing_width);
            if ml + mr + used_horizontal > containing_width {
                // Overconstrained (LTR): margin-right absorbs the excess.
                (ml, containing_width - used_horizontal - ml)
            } else {
                (ml, mr)
            }
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

/// Returns `true` if any child is block-level (block formatting context).
///
/// When this returns `true` and inline children are also present, inline
/// content is currently skipped. CSS 2.1 §9.2.1.1 requires wrapping
/// consecutive inline runs in anonymous block boxes — this is deferred to
/// Phase 2.
// TODO(Phase 2): generate anonymous block boxes for mixed block/inline content.
fn children_are_block(dom: &EcsDom, children: &[Entity]) -> bool {
    for &child in children {
        if let Ok(style) = dom.world().get::<&ComputedStyle>(child) {
            match style.display {
                // TODO(Phase 2): InlineBlock should participate in inline formatting
                // context (CSS 2.1 §9.2.2), not force block context.
                Display::Block | Display::InlineBlock | Display::Flex => return true,
                Display::None | Display::Inline => {}
            }
        }
    }
    false
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
fn layout_block_inner(
    dom: &mut EcsDom,
    entity: Entity,
    containing_width: f32,
    offset_x: f32,
    offset_y: f32,
    font_db: &FontDatabase,
    depth: u32,
) -> LayoutBox {
    let style = dom
        .world()
        .get::<&ComputedStyle>(entity)
        .map(|s| (*s).clone())
        .unwrap_or_default();

    // --- Sanitize padding and border (protect against NaN/infinity) ---
    let pad_top = sanitize(style.padding_top);
    let pad_right = sanitize(style.padding_right);
    let pad_bottom = sanitize(style.padding_bottom);
    let pad_left = sanitize(style.padding_left);
    let bdr_top = sanitize(style.border_top_width);
    let bdr_right = sanitize(style.border_right_width);
    let bdr_bottom = sanitize(style.border_bottom_width);
    let bdr_left = sanitize(style.border_left_width);

    // --- Resolve margins ---
    let margin_top = resolve_margin(style.margin_top, containing_width);
    let margin_bottom = resolve_margin(style.margin_bottom, containing_width);

    // --- Resolve width ---
    let margin_left_raw = resolve_margin(style.margin_left, containing_width);
    let margin_right_raw = resolve_margin(style.margin_right, containing_width);
    let horizontal_extra =
        margin_left_raw + margin_right_raw + pad_left + pad_right + bdr_left + bdr_right;
    let content_width = resolve_width(&style, containing_width, horizontal_extra);

    // --- Horizontal margin auto centering ---
    let used_horizontal = content_width + pad_left + pad_right + bdr_left + bdr_right;
    let (margin_left, margin_right) = if matches!(style.width, Dimension::Auto) {
        (margin_left_raw, margin_right_raw)
    } else {
        apply_margin_auto_centering(&style, containing_width, used_horizontal)
    };

    // --- Content rect position ---
    let content_x = offset_x + margin_left + bdr_left + pad_left;
    let content_y = offset_y + margin_top + bdr_top + pad_top;

    // --- Layout children (stop recursion at depth limit) ---
    let children = dom.children(entity);
    let content_height = if children.is_empty() || depth >= MAX_LAYOUT_DEPTH {
        0.0
    } else if children_are_block(dom, &children) {
        layout_block_children_inner(
            dom,
            &children,
            content_width,
            content_x,
            content_y,
            font_db,
            depth + 1,
        )
    } else {
        layout_inline_context(
            dom,
            &children,
            content_width,
            content_x,
            content_y,
            &style,
            font_db,
        )
    };

    let height = resolve_height(&style).unwrap_or(content_height);

    let lb = LayoutBox {
        content: Rect {
            x: content_x,
            y: content_y,
            width: content_width,
            height,
        },
        padding: EdgeSizes {
            top: pad_top,
            right: pad_right,
            bottom: pad_bottom,
            left: pad_left,
        },
        border: EdgeSizes {
            top: bdr_top,
            right: bdr_right,
            bottom: bdr_bottom,
            left: bdr_left,
        },
        margin: EdgeSizes {
            top: margin_top,
            right: margin_right,
            bottom: margin_bottom,
            left: margin_left,
        },
    };

    let _ = dom.world_mut().insert_one(entity, lb.clone());
    lb
}

/// Layout block-level children with vertical stacking and margin collapse.
///
/// Currently only collapses margins between adjacent block siblings.
// TODO(Phase 2): implement parent-child margin collapse (CSS 2.1 §8.3.1)
// when parent has no border-top/padding-top (first child) or
// border-bottom/padding-bottom (last child).
fn layout_block_children_inner(
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
        let child_style = match dom.world().get::<&ComputedStyle>(child) {
            Ok(s) => (*s).clone(),
            Err(_) => continue, // text node in block context: skip
        };

        if child_style.display == Display::None || child_style.display == Display::Inline {
            continue;
        }

        // Margin collapse between adjacent block siblings (CSS 2.1 §8.3.1).
        // Both positive → max, both negative → min, mixed → sum.
        let child_margin_top = resolve_margin(child_style.margin_top, containing_width);
        if let Some(prev_mb) = prev_margin_bottom {
            let collapsed = collapse_margins(prev_mb, child_margin_top);
            cursor_y -= prev_mb + child_margin_top - collapsed;
        }

        let child_box = layout_block_inner(
            dom,
            child,
            containing_width,
            offset_x,
            cursor_y,
            font_db,
            depth,
        );
        cursor_y += child_box.margin_box().height;
        prev_margin_bottom = Some(child_box.margin.bottom);
    }

    cursor_y - offset_y
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    fn make_dom_with_block_div(style: ComputedStyle) -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        dom.world_mut().insert_one(div, style);
        (dom, div)
    }

    #[test]
    fn width_auto_fills_containing_block() {
        let style = ComputedStyle {
            display: Display::Block,
            ..Default::default()
        };
        let (mut dom, div) = make_dom_with_block_div(style);
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

        let parent_style = ComputedStyle {
            display: Display::Block,
            ..Default::default()
        };
        let child_style = ComputedStyle {
            display: Display::Block,
            height: Dimension::Length(50.0),
            ..Default::default()
        };

        dom.world_mut().insert_one(parent, parent_style);
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

        dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
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

        dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
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

        dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
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

        dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
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
}
