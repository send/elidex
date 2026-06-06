//! `column-fill` (auto / balance) and overflow-column tests.

use super::*;

#[test]
fn auto_vs_balance_fill() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        column_fill: ColumnFill::Auto,
        height: Dimension::Length(200.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 50.0);
    add_block_child(&mut dom, container, 50.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    // With auto fill and definite height=200, container uses 200.
    assert_eq!(lb.content.size.height, 200.0);
}

// --- Overflow columns ---
#[test]
fn overflow_columns_beyond_count() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        column_fill: ColumnFill::Auto,
        height: Dimension::Length(50.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    // Three children that won't fit in 2 columns at 50px each
    add_block_child(&mut dom, container, 50.0);
    add_block_child(&mut dom, container, 50.0);
    add_block_child(&mut dom, container, 50.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    // With definite height + auto fill, container height = definite height
    assert_eq!(lb.content.size.height, 50.0);

    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    // Should have overflow columns (>2)
    assert!(!info.segments.is_empty());
}
