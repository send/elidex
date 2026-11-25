use super::source_contributions::contribution_input;
use super::*;

#[test]
fn balanced_multicol_reports_empty_block_to_its_parent() {
    let mut dom = EcsDom::new();
    let mut node = |parent, columns| {
        let id = dom.create_element_ownerless_fixture("div", Attributes::default());
        dom.world_mut().insert_one(
            id,
            ComputedStyle {
                display: Display::Block,
                column_count: columns,
                ..Default::default()
            },
        );
        if let Some(owner) = parent {
            dom.append_child(owner, id);
        }
        id
    };
    let owner = node(None, None);
    let multi = node(Some(owner), Some(2));
    let empty = node(Some(multi), None);
    let fonts = FontDatabase::new();
    let frag = elidex_layout_block::FragmentainerContext {
        available_block_size: 100.0,
        fragmentation_type: elidex_layout_block::FragmentationType::Page,
    };
    let input = contribution_input(&fonts, &frag);
    for entry in [multi, owner] {
        let outcome = dispatch_layout_child(&mut dom, entry, &input);
        assert!(outcome.end.is_complete());
        assert!(outcome.contribution.is_finished(multi, empty));
        if entry == owner {
            assert!(outcome.contribution.is_finished(owner, multi));
        }
    }
}

#[test]
fn whole_spanners_and_floats_forward_nested_source() {
    use elidex_plugin::{ColumnSpan, Float};
    let mut dom = EcsDom::new();
    let mut node = |tag, display, parent| {
        let id = dom.create_element_ownerless_fixture(tag, Attributes::default());
        dom.world_mut().insert_one(
            id,
            ComputedStyle {
                display,
                ..Default::default()
            },
        );
        if let Some(owner) = parent {
            dom.append_child(owner, id);
        }
        id
    };
    let owner = node("div", Display::Block, None);
    let multi = node("div", Display::Block, Some(owner));
    let table = node("table", Display::Table, Some(multi));
    let row = node("tr", Display::TableRow, Some(table));
    node("td", Display::TableCell, Some(row));
    let block = node("div", Display::Block, Some(multi));
    let inner = node("div", Display::Block, Some(block));
    let float = node("div", Display::Block, Some(owner));
    let floated = node("div", Display::Block, Some(float));
    let flex_span = node("div", Display::Flex, Some(multi));
    let span_item = node("div", Display::Block, Some(flex_span));
    let flex_float = node("div", Display::Flex, Some(owner));
    let float_item = node("div", Display::Block, Some(flex_float));
    dom.world_mut()
        .get::<&mut ComputedStyle>(flex_float)
        .unwrap()
        .float = Float::Left;
    dom.world_mut()
        .get::<&mut ComputedStyle>(multi)
        .unwrap()
        .column_count = Some(2);
    for spanner in [table, block, flex_span] {
        dom.world_mut()
            .get::<&mut ComputedStyle>(spanner)
            .unwrap()
            .column_span = ColumnSpan::All;
    }
    dom.world_mut()
        .get::<&mut ComputedStyle>(float)
        .unwrap()
        .float = Float::Left;
    let fonts = FontDatabase::new();
    let frag = elidex_layout_block::FragmentainerContext {
        available_block_size: 100.0,
        fragmentation_type: elidex_layout_block::FragmentationType::Page,
    };
    let outcome = dispatch_layout_child(&mut dom, owner, &contribution_input(&fonts, &frag));
    for (parent, child) in [
        (owner, multi),
        (flex_span, span_item),
        (flex_float, float_item),
        (multi, table),
        (table, row),
        (multi, block),
        (block, inner),
        (owner, float),
        (float, floated),
    ] {
        assert!(
            outcome.contribution.is_finished(parent, child),
            "missing {parent:?}/{child:?}"
        );
    }
}

#[test]
fn nested_screen_multicol_keeps_overflow_columns_at_the_inner_pitch() {
    let mut dom = EcsDom::new();
    let mut multi = |parent| {
        let id = dom.create_element_ownerless_fixture("div", Attributes::default());
        dom.world_mut().insert_one(
            id,
            ComputedStyle {
                display: Display::Block,
                column_count: Some(2),
                column_gap: Dimension::Length(0.0),
                column_fill: elidex_plugin::ColumnFill::Auto,
                height: Dimension::Length(40.0),
                ..Default::default()
            },
        );
        if let Some(owner) = parent {
            dom.append_child(owner, id);
        }
        id
    };
    let outer = multi(None);
    let inner = multi(Some(outer));
    let mut last = None;
    for i in 0..1001 {
        let id = dom.create_element_ownerless_fixture("div", Attributes::default());
        dom.append_child(inner, id);
        dom.world_mut().insert_one(
            id,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(1.0),
                break_after: if i < 1000 {
                    elidex_plugin::BreakValue::Column
                } else {
                    elidex_plugin::BreakValue::Auto
                },
                ..Default::default()
            },
        );
        last = Some(id);
    }
    let fonts = FontDatabase::new();
    let frag = elidex_layout_block::FragmentainerContext {
        available_block_size: 40.0,
        fragmentation_type: elidex_layout_block::FragmentationType::Column,
    };
    let input = elidex_layout_block::LayoutInput {
        medium: elidex_plugin::Medium::Screen,
        placement: elidex_layout_block::FragmentPlacement::Normal,
        containing: CssSize::definite(600.0, 800.0),
        containing_inline_size: 600.0,
        fragmentainer: None,
        ..contribution_input(&fonts, &frag)
    };
    let outcome = dispatch_layout_child(&mut dom, outer, &input);
    let last = last.unwrap();
    assert!(outcome.contribution.is_finished(inner, last));
    assert_eq!(
        dom.world()
            .get::<&elidex_plugin::MulticolInfo>(inner)
            .unwrap()
            .segments[0]
            .0,
        1001
    );
    assert_eq!(
        dom.world()
            .get::<&elidex_plugin::LayoutBox>(last)
            .unwrap()
            .content
            .origin
            .x,
        150_000.0
    );
}

#[test]
fn stalled_multicol_remains_incomplete_through_whole_children_and_page_counting() {
    use elidex_layout_block::{FragmentEnd, FragmentainerContext, FragmentationType};
    use elidex_plugin::{BreakValue, ColumnFill, ColumnSpan, Float, Medium};

    for (float, span) in [
        (Float::None, ColumnSpan::None),
        (Float::Left, ColumnSpan::None),
        (Float::None, ColumnSpan::All),
    ] {
        let mut dom = EcsDom::new();
        let mut node = |parent, style| {
            let id = dom.create_element_ownerless_fixture("div", Attributes::default());
            dom.world_mut().insert_one(id, style);
            if let Some(owner) = parent {
                dom.append_child(owner, id);
            }
            id
        };
        let block = ComputedStyle {
            display: Display::Block,
            ..Default::default()
        };
        let root = node(None, block.clone());
        let wrapper = node(
            Some(root),
            ComputedStyle {
                column_count: (span == ColumnSpan::All).then_some(2),
                ..block.clone()
            },
        );
        let inner = node(
            Some(wrapper),
            ComputedStyle {
                column_count: Some(2),
                height: Dimension::Length(40.0),
                column_fill: ColumnFill::Auto,
                float,
                column_span: span,
                ..block.clone()
            },
        );
        let pending = node(
            Some(inner),
            ComputedStyle {
                break_before: BreakValue::Column,
                height: Dimension::Length(1.0),
                ..block
            },
        );
        let fonts = FontDatabase::new();
        let page = FragmentainerContext {
            available_block_size: 100.0,
            fragmentation_type: FragmentationType::Page,
        };
        let input = elidex_layout_block::LayoutInput {
            medium: Medium::Print,
            placement: elidex_layout_block::FragmentPlacement::Normal,
            ..contribution_input(&fonts, &page)
        };
        for owner in [inner, wrapper, root] {
            let fragments = layout_fragmented(&mut dom, owner, &input, page);
            assert_eq!(fragments.len(), 1);
            let outcome = &fragments[0];
            assert!(matches!(outcome.end, FragmentEnd::Incomplete));
            for (parent, child) in [(inner, pending), (wrapper, inner), (root, wrapper)] {
                assert!(!outcome.contribution.is_finished(parent, child));
            }
        }
    }
}

#[test]
fn whole_print_multicol_finishes_all_columns_in_floats_and_spanners() {
    use elidex_plugin::{BreakValue, ColumnFill, ColumnSpan, Float, Medium};
    let fonts = FontDatabase::new();
    for spanner in [false, true] {
        let mut dom = EcsDom::new();
        let mut node = |parent, style| {
            let id = dom.create_element_ownerless_fixture("div", Attributes::default());
            dom.world_mut().insert_one(id, style);
            if let Some(parent) = parent {
                dom.append_child(parent, id);
            }
            id
        };
        let block = ComputedStyle {
            display: Display::Block,
            ..Default::default()
        };
        let root = node(
            None,
            ComputedStyle {
                column_count: spanner.then_some(2),
                ..block.clone()
            },
        );
        let inner = node(
            Some(root),
            ComputedStyle {
                float: if spanner { Float::None } else { Float::Left },
                column_span: if spanner {
                    ColumnSpan::All
                } else {
                    ColumnSpan::None
                },
                column_count: Some(2),
                height: Dimension::Length(40.0),
                column_fill: ColumnFill::Auto,
                ..block.clone()
            },
        );
        let children: Vec<_> = (0..1001)
            .map(|i| {
                node(
                    Some(inner),
                    ComputedStyle {
                        height: Dimension::Length(1.0),
                        break_after: if i < 1000 {
                            BreakValue::Column
                        } else {
                            BreakValue::Auto
                        },
                        ..block.clone()
                    },
                )
            })
            .collect();
        let sibling = node(Some(root), block);
        let page = elidex_layout_block::FragmentainerContext {
            available_block_size: 100.0,
            fragmentation_type: elidex_layout_block::FragmentationType::Page,
        };
        let input = elidex_layout_block::LayoutInput {
            medium: Medium::Print,
            placement: elidex_layout_block::FragmentPlacement::Normal,
            ..contribution_input(&fonts, &page)
        };
        let pages = layout_fragmented(&mut dom, root, &input, page);
        assert_eq!(pages.len(), 1);
        let outcome = &pages[0];
        assert!(outcome.end.is_complete());
        for child in children {
            assert!(outcome.contribution.is_finished(inner, child));
        }
        assert!(outcome.contribution.is_finished(root, inner));
        assert!(outcome.contribution.is_finished(root, sibling));
    }
}
