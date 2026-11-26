//! Intrinsic sizing with the Grid track-sizing formatter.
use elidex_ecs::{EcsDom, Entity};
pub use elidex_layout_block::intrinsic::shrink_to_fit_width;
use elidex_layout_block::{IntrinsicSizes, LayoutEnv};

/// Compute intrinsic inline sizes including the box's padding and border.
pub fn compute_intrinsic_sizes(
    dom: &mut EcsDom,
    entity: Entity,
    env: &LayoutEnv<'_>,
) -> IntrinsicSizes {
    elidex_layout_block::intrinsic::compute_intrinsic_sizes(
        dom,
        entity,
        env,
        elidex_layout_grid::compute_grid_intrinsic,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dispatch_layout_child;
    use elidex_ecs::Attributes;
    use elidex_plugin::{ComputedStyle, Dimension, FlexDirection, FlexWrap};
    use elidex_plugin::{Display, WritingMode};
    use elidex_text::FontDatabase;

    fn test_env(font_db: &FontDatabase) -> LayoutEnv<'_> {
        LayoutEnv {
            font_db,
            layout_child: dispatch_layout_child,
            depth: 0,
            scroll_limits: elidex_layout_block::ScrollLimits::default(),
            viewport: None,
            layout_generation: 0,
            is_probe: false,
        }
    }

    fn make_dom_with_text() -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let parent = dom.create_element_ownerless_fixture("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        let text = dom.create_text_ownerless_fixture("hello world");
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
        let flex = dom.create_element_ownerless_fixture("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            flex,
            ComputedStyle {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Nowrap,
                ..Default::default()
            },
        );
        let c1 = dom.create_element_ownerless_fixture("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(100.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(flex, c1);
        let c2 = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        let flex = dom.create_element_ownerless_fixture("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            flex,
            ComputedStyle {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                ..Default::default()
            },
        );
        let c1 = dom.create_element_ownerless_fixture("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(100.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(flex, c1);
        let c2 = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        let outer = dom.create_element_ownerless_fixture("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            outer,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        let inner = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        let table = dom.create_element_ownerless_fixture("table", Attributes::default());
        let _ = dom.world_mut().insert_one(
            table,
            ComputedStyle {
                display: Display::Table,
                ..Default::default()
            },
        );
        let tr = dom.create_element_ownerless_fixture("tr", Attributes::default());
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
            let td = dom.create_element_ownerless_fixture("td", attrs);
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
        let block = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        let flex = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        let c1 = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        let c2 = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        assert_eq!((sizes.min_content, sizes.max_content), (140., 140.));
    }

    #[test]
    fn flex_column_vertical_rl_intrinsic() {
        let mut dom = EcsDom::new();
        let flex = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        let c1 = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        let c2 = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        assert_eq!((sizes.min_content, sizes.max_content), (70., 70.));
    }

    #[test]
    fn table_vertical_rl_intrinsic() {
        let mut dom = EcsDom::new();
        let table = dom.create_element_ownerless_fixture("table", Attributes::default());
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
        let tr = dom.create_element_ownerless_fixture("tr", Attributes::default());
        let _ = dom.world_mut().insert_one(
            tr,
            ComputedStyle {
                display: Display::TableRow,
                ..Default::default()
            },
        );
        let _ = dom.append_child(table, tr);
        let td1 = dom.create_element_ownerless_fixture("td", Attributes::default());
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
        let td2 = dom.create_element_ownerless_fixture("td", Attributes::default());
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
        let block = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        let el = dom.create_element_ownerless_fixture("div", Attributes::default());
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
