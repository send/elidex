//! `column-span` / spanner segmentation tests.

use super::*;

#[test]
fn column_span_all_splits_segments() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    // Block A
    add_block_child(&mut dom, container, 40.0);

    // Spanner
    let spanner = elem(&mut dom, "div");
    dom.append_child(container, spanner);
    let sp_style = ComputedStyle {
        display: Display::Block,
        column_span: ColumnSpan::All,
        height: Dimension::Length(20.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(spanner, sp_style);

    // Block B
    add_block_child(&mut dom, container, 40.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    // Total: segment1(40px balanced) + spanner(20px) + segment2(40px balanced)
    assert!(lb.content.size.height > 20.0);

    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    // Two normal segments (spanner excluded).
    assert_eq!(info.segments.len(), 2);
}

#[test]
fn spanner_first_child() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    // Spanner as first child
    let spanner = elem(&mut dom, "div");
    dom.append_child(container, spanner);
    let sp_style = ComputedStyle {
        display: Display::Block,
        column_span: ColumnSpan::All,
        height: Dimension::Length(30.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(spanner, sp_style);

    add_block_child(&mut dom, container, 40.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    assert!(lb.content.size.height >= 30.0);
}

#[test]
fn spanner_last_child() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 40.0);

    // Spanner as last child
    let spanner = elem(&mut dom, "div");
    dom.append_child(container, spanner);
    let sp_style = ComputedStyle {
        display: Display::Block,
        column_span: ColumnSpan::All,
        height: Dimension::Length(30.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(spanner, sp_style);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let lb = dom.world().get::<&LayoutBox>(container).unwrap();
    assert!(lb.content.size.height >= 30.0);
}

#[test]
fn multiple_spanners() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 30.0);

    // Spanner 1
    let s1 = elem(&mut dom, "div");
    dom.append_child(container, s1);
    let sp1 = ComputedStyle {
        display: Display::Block,
        column_span: ColumnSpan::All,
        height: Dimension::Length(20.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(s1, sp1);

    add_block_child(&mut dom, container, 30.0);

    // Spanner 2
    let s2 = elem(&mut dom, "div");
    dom.append_child(container, s2);
    let sp2 = ComputedStyle {
        display: Display::Block,
        column_span: ColumnSpan::All,
        height: Dimension::Length(10.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(s2, sp2);

    add_block_child(&mut dom, container, 30.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    // Three normal segments (two spanners excluded).
    assert_eq!(info.segments.len(), 3);
}

// --- Spanner edge cases ---
#[test]
fn inline_spanner_not_treated_as_spanner() {
    // column-span: all on inline-level element should NOT create a spanner
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 40.0);

    // Inline element with column-span: all — should be ignored
    let inline_span = elem(&mut dom, "span");
    dom.append_child(container, inline_span);
    let span_style = ComputedStyle {
        display: Display::Inline,
        column_span: ColumnSpan::All,
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(inline_span, span_style);

    add_block_child(&mut dom, container, 40.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    // Should have only 1 normal segment (no spanner split)
    assert_eq!(info.segments.len(), 1);
}

#[test]
fn floated_spanner_not_treated_as_spanner() {
    // column-span: all on floated element should NOT create a spanner
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 40.0);

    let float_span = elem(&mut dom, "div");
    dom.append_child(container, float_span);
    let fs_style = ComputedStyle {
        display: Display::Block,
        column_span: ColumnSpan::All,
        float: Float::Left,
        height: Dimension::Length(20.0),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(float_span, fs_style);

    add_block_child(&mut dom, container, 40.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    // Floated element not treated as spanner → 1 segment
    assert_eq!(info.segments.len(), 1);
}

#[test]
fn display_none_excluded_from_segments() {
    let mut dom = EcsDom::new();
    let container = elem(&mut dom, "div");
    let style = ComputedStyle {
        display: Display::Block,
        column_count: Some(2),
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(container, style);

    add_block_child(&mut dom, container, 40.0);

    // display:none child — should be excluded from segments entirely
    let hidden = elem(&mut dom, "div");
    dom.append_child(container, hidden);
    let hidden_style = ComputedStyle {
        display: Display::None,
        column_span: ColumnSpan::All,
        ..ComputedStyle::default()
    };
    let _ = dom.world_mut().insert_one(hidden, hidden_style);

    add_block_child(&mut dom, container, 40.0);

    let font_db = make_font_db();
    let input = make_input(&font_db);
    layout_multicol(&mut dom, container, &input, layout_child_fn);

    let info = dom.world().get::<&MulticolInfo>(container).unwrap();
    // display:none with column-span:all should NOT split into segments
    assert_eq!(info.segments.len(), 1);
}
