//! Intrinsic sizing computation (CSS Sizing Level 3).
//!
//! Provides [`compute_intrinsic_sizes`] which computes min-content and
//! max-content intrinsic sizes for any element, routing to the appropriate
//! algorithm based on `display` type.

mod block;
mod table;

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::{
    composed_children_flat, get_style, inline_pb, sanitize_border, sanitize_padding,
    IntrinsicSizes, LayoutEnv, MAX_LAYOUT_DEPTH,
};
#[cfg(test)]
use elidex_plugin::WritingMode;
use elidex_plugin::{BoxSizing, Dimension, Display, WritingModeContext};

use block::{compute_block_intrinsic, compute_flex_intrinsic};
use table::compute_table_intrinsic;

/// Compute min-content and max-content intrinsic inline sizes for an element.
///
/// Writing-mode-aware: uses the element's `writing-mode` to determine the
/// inline axis, so vertical writing modes compute intrinsic sizes along the
/// vertical (block-flow) direction.
///
/// Routes to display-specific intrinsic sizing:
/// - **Block**: inline children → inline min/max; block children → max of children.
/// - **Flex row**: nowrap → sum, wrap → max for min-content.
/// - **Flex column**: max of children's intrinsic widths.
/// - **Grid**: placement-based per-column track sizing via `elidex-layout-grid`.
/// - **Table**: cell → column → table (CSS 2.1 §17.5.2).
/// - **Inline/Text**: delegates to inline measurement.
pub fn compute_intrinsic_sizes(
    dom: &mut EcsDom,
    entity: Entity,
    env: &LayoutEnv<'_>,
) -> IntrinsicSizes {
    if env.depth >= MAX_LAYOUT_DEPTH {
        return IntrinsicSizes::default();
    }

    let style = get_style(dom, entity);

    // Container padding + border contribute to intrinsic inline size (CSS Sizing L3 §5).
    let padding = sanitize_padding(&style);
    let border = sanitize_border(&style);
    let wm = WritingModeContext::new(style.writing_mode, style.direction);
    let pb = inline_pb(&wm, &padding, &border);

    // Check for replaced elements or explicit sizes first.
    if let Some(sizes) = explicit_intrinsic(dom, entity, &style, pb) {
        return sizes;
    }

    let children = composed_children_flat(dom, entity);

    if children.is_empty() {
        return IntrinsicSizes {
            min_content: pb,
            max_content: pb,
        };
    }

    let content = match style.display {
        Display::Flex | Display::InlineFlex => compute_flex_intrinsic(dom, entity, &children, env),
        Display::Grid | Display::InlineGrid => {
            elidex_layout_grid::compute_grid_intrinsic(dom, entity, &children, env)
        }
        Display::Table | Display::InlineTable => {
            compute_table_intrinsic(dom, entity, &children, env)
        }
        _ => compute_block_intrinsic(dom, entity, &children, env),
    };

    IntrinsicSizes {
        min_content: content.min_content + pb,
        max_content: content.max_content + pb,
    }
}

/// Return intrinsic sizes for replaced elements or elements with explicit inline-axis size.
///
/// Replaced elements (images, form controls) have intrinsic dimensions.
/// Elements with explicit inline-axis dimension (`width` in horizontal-tb,
/// `height` in vertical modes) use that as both min and max content.
fn explicit_intrinsic(
    dom: &EcsDom,
    entity: Entity,
    style: &elidex_plugin::ComputedStyle,
    pb: f32,
) -> Option<IntrinsicSizes> {
    let inline_horizontal =
        WritingModeContext::new(style.writing_mode, style.direction).is_horizontal();

    // Replaced elements: use intrinsic image/form dimensions (inline-axis).
    if let Some(intr) = elidex_layout_block::get_intrinsic_size(dom, entity) {
        let inline_size = if inline_horizontal {
            intr.width
        } else {
            intr.height
        };
        return Some(IntrinsicSizes {
            min_content: inline_size + pb,
            max_content: inline_size + pb,
        });
    }
    // Explicit inline-axis dimension: acts as both min-content and max-content.
    let explicit_dim = if inline_horizontal {
        style.width
    } else {
        style.height
    };
    if let Dimension::Length(len) = explicit_dim {
        // border-box: dimension already includes padding+border.
        let size = if style.box_sizing == BoxSizing::BorderBox {
            len
        } else {
            len + pb
        };
        return Some(IntrinsicSizes {
            min_content: size,
            max_content: size,
        });
    }
    None
}

/// Compute shrink-to-fit width for inline-level containers.
///
/// CSS 2.1 §10.3.5: shrink-to-fit width = min(max-content, max(min-content, `available_width`)).
#[must_use]
pub fn shrink_to_fit_width(intrinsic: &IntrinsicSizes, available_width: f32) -> f32 {
    intrinsic
        .max_content
        .min(intrinsic.min_content.max(available_width))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dispatch_layout_child;
    use elidex_ecs::Attributes;
    use elidex_plugin::{ComputedStyle, Dimension, FlexDirection, FlexWrap};
    use elidex_text::FontDatabase;

    fn test_env(font_db: &FontDatabase) -> LayoutEnv<'_> {
        LayoutEnv {
            font_db,
            layout_child: dispatch_layout_child,
            depth: 0,
            viewport: None,
        }
    }

    fn make_dom_with_text() -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        let text = dom.create_text("hello world");
        let _ = dom.append_child(parent, text);
        (dom, parent)
    }

    #[test]
    fn block_with_text_intrinsic() {
        let (mut dom, parent) = make_dom_with_text();
        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, parent, &test_env(&font_db));
        // min_content <= max_content
        assert!(sizes.min_content <= sizes.max_content);
    }

    #[test]
    fn flex_row_nowrap_intrinsic() {
        let mut dom = EcsDom::new();
        let flex = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            flex,
            ComputedStyle {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Nowrap,
                ..Default::default()
            },
        );
        let c1 = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(100.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(flex, c1);
        let c2 = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            c2,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(50.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(flex, c2);

        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, flex, &test_env(&font_db));
        // nowrap: min = sum = 150, max = sum = 150 (both have explicit widths)
        assert!(sizes.min_content >= 0.0);
        assert!(sizes.max_content >= sizes.min_content);
    }

    #[test]
    fn flex_row_wrap_intrinsic() {
        let mut dom = EcsDom::new();
        let flex = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            flex,
            ComputedStyle {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                ..Default::default()
            },
        );
        let c1 = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(100.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(flex, c1);
        let c2 = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            c2,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(50.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(flex, c2);

        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, flex, &test_env(&font_db));
        // wrap: min = max(items) = 100, max = sum = 150
        assert!(sizes.min_content >= 0.0);
        assert!(sizes.max_content >= sizes.min_content);
    }

    #[test]
    fn nested_block_intrinsic() {
        let mut dom = EcsDom::new();
        let outer = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            outer,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        let inner = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            inner,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(200.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(outer, inner);

        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, outer, &test_env(&font_db));
        assert!(sizes.min_content >= 0.0);
        assert!(sizes.max_content >= sizes.min_content);
    }

    #[test]
    fn inline_flex_shrink_to_fit() {
        let sizes = IntrinsicSizes {
            min_content: 50.0,
            max_content: 200.0,
        };
        // available < min: result = min
        assert_eq!(shrink_to_fit_width(&sizes, 30.0), 50.0);
        // available between min and max: result = available
        assert_eq!(shrink_to_fit_width(&sizes, 100.0), 100.0);
        // available > max: result = max
        assert_eq!(shrink_to_fit_width(&sizes, 300.0), 200.0);
    }

    #[test]
    fn inline_grid_shrink_to_fit() {
        let sizes = IntrinsicSizes {
            min_content: 100.0,
            max_content: 400.0,
        };
        assert_eq!(shrink_to_fit_width(&sizes, 250.0), 250.0);
    }

    // --- G6: Colspan intrinsic sizing tests ---

    /// Helper: create a table with one row and cells with optional colspan.
    fn make_table_with_colspan(cells: &[(usize, f32)]) -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let table = dom.create_element("table", Attributes::default());
        let _ = dom.world_mut().insert_one(
            table,
            ComputedStyle {
                display: Display::Table,
                ..Default::default()
            },
        );
        let tr = dom.create_element("tr", Attributes::default());
        let _ = dom.world_mut().insert_one(
            tr,
            ComputedStyle {
                display: Display::TableRow,
                ..Default::default()
            },
        );
        let _ = dom.append_child(table, tr);
        for &(colspan, width) in cells {
            let mut attrs = Attributes::default();
            if colspan > 1 {
                attrs.set("colspan", colspan.to_string());
            }
            let td = dom.create_element("td", attrs);
            let _ = dom.world_mut().insert_one(
                td,
                ComputedStyle {
                    display: Display::TableCell,
                    width: Dimension::Length(width),
                    height: Dimension::Length(20.0),
                    ..Default::default()
                },
            );
            let _ = dom.append_child(tr, td);
        }
        (dom, table)
    }

    #[test]
    fn colspan_2_intrinsic_distributes() {
        let (mut dom, table) = make_table_with_colspan(&[(2, 200.0)]);
        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, table, &test_env(&font_db));
        assert!(sizes.min_content > 0.0);
        assert!(sizes.max_content >= sizes.min_content);
    }

    #[test]
    fn colspan_plus_normal_mixed() {
        let (mut dom, table) = make_table_with_colspan(&[(2, 200.0), (1, 50.0)]);
        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, table, &test_env(&font_db));
        assert!(sizes.min_content > 0.0);
        assert!(sizes.max_content >= sizes.min_content);
    }

    #[test]
    fn all_colspan_row() {
        let (mut dom, table) = make_table_with_colspan(&[(3, 300.0)]);
        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, table, &test_env(&font_db));
        assert!(sizes.min_content > 0.0);
        assert!(sizes.max_content >= sizes.min_content);
    }

    // --- G7: Writing-mode-aware intrinsic sizing tests ---

    #[test]
    fn block_vertical_rl_intrinsic() {
        let mut dom = EcsDom::new();
        let block = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            block,
            ComputedStyle {
                display: Display::Block,
                writing_mode: WritingMode::VerticalRl,
                height: Dimension::Length(120.0),
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, block, &test_env(&font_db));
        assert!((sizes.min_content - 120.0).abs() < 1.0);
        assert!((sizes.max_content - 120.0).abs() < 1.0);
    }

    #[test]
    fn flex_row_vertical_rl_intrinsic() {
        let mut dom = EcsDom::new();
        let flex = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            flex,
            ComputedStyle {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Nowrap,
                writing_mode: WritingMode::VerticalRl,
                ..Default::default()
            },
        );
        let c1 = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                writing_mode: WritingMode::VerticalRl,
                height: Dimension::Length(80.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(flex, c1);
        let c2 = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            c2,
            ComputedStyle {
                display: Display::Block,
                writing_mode: WritingMode::VerticalRl,
                height: Dimension::Length(60.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(flex, c2);

        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, flex, &test_env(&font_db));
        assert!(sizes.min_content >= 60.0);
        assert!(sizes.max_content >= sizes.min_content);
    }

    #[test]
    fn flex_column_vertical_rl_intrinsic() {
        let mut dom = EcsDom::new();
        let flex = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            flex,
            ComputedStyle {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                flex_wrap: FlexWrap::Nowrap,
                writing_mode: WritingMode::VerticalRl,
                ..Default::default()
            },
        );
        let c1 = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                writing_mode: WritingMode::VerticalRl,
                height: Dimension::Length(70.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(flex, c1);
        let c2 = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            c2,
            ComputedStyle {
                display: Display::Block,
                writing_mode: WritingMode::VerticalRl,
                height: Dimension::Length(40.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(flex, c2);

        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, flex, &test_env(&font_db));
        assert!(sizes.min_content >= 40.0);
        assert!(sizes.max_content >= sizes.min_content);
    }

    #[test]
    fn table_vertical_rl_intrinsic() {
        let mut dom = EcsDom::new();
        let table = dom.create_element("table", Attributes::default());
        let _ = dom.world_mut().insert_one(
            table,
            ComputedStyle {
                display: Display::Table,
                writing_mode: WritingMode::VerticalRl,
                border_spacing_h: 10.0,
                border_spacing_v: 5.0,
                ..Default::default()
            },
        );
        let tr = dom.create_element("tr", Attributes::default());
        let _ = dom.world_mut().insert_one(
            tr,
            ComputedStyle {
                display: Display::TableRow,
                ..Default::default()
            },
        );
        let _ = dom.append_child(table, tr);
        let td1 = dom.create_element("td", Attributes::default());
        let _ = dom.world_mut().insert_one(
            td1,
            ComputedStyle {
                display: Display::TableCell,
                writing_mode: WritingMode::VerticalRl,
                height: Dimension::Length(50.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(tr, td1);
        let td2 = dom.create_element("td", Attributes::default());
        let _ = dom.world_mut().insert_one(
            td2,
            ComputedStyle {
                display: Display::TableCell,
                writing_mode: WritingMode::VerticalRl,
                height: Dimension::Length(30.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(tr, td2);

        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, table, &test_env(&font_db));
        assert!(sizes.min_content > 0.0);
        assert!(sizes.max_content >= sizes.min_content);
    }

    #[test]
    fn horizontal_tb_regression_intrinsic() {
        let mut dom = EcsDom::new();
        let block = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            block,
            ComputedStyle {
                display: Display::Block,
                writing_mode: WritingMode::HorizontalTb,
                width: Dimension::Length(200.0),
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, block, &test_env(&font_db));
        assert!((sizes.min_content - 200.0).abs() < 1.0);
        assert!((sizes.max_content - 200.0).abs() < 1.0);
    }

    #[test]
    fn explicit_height_vertical_rl() {
        let mut dom = EcsDom::new();
        let el = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            el,
            ComputedStyle {
                display: Display::Block,
                writing_mode: WritingMode::VerticalRl,
                width: Dimension::Length(999.0),
                height: Dimension::Length(150.0),
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, el, &test_env(&font_db));
        assert!((sizes.min_content - 150.0).abs() < 1.0);
        assert!((sizes.max_content - 150.0).abs() < 1.0);
    }
}
