use super::*;

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
    // height:auto -> bottom margin collapses.
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
    // Parent has height: 200px -> bottom margin does NOT collapse (CSS 2.1 $8.3.1).
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
    // height is explicit -> no bottom collapse. Parent keeps its own margin-bottom.
    assert!(
        (lb.margin.bottom - 5.0).abs() < f32::EPSILON,
        "expected margin-bottom=5 (no collapse), got {}",
        lb.margin.bottom
    );
}

#[test]
fn parent_child_no_collapse_with_border() {
    // Parent has border-top > 0 -> no first-child collapse.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            margin_top: Dimension::Length(10.0),
            border_top: BorderSide { width: 1.0, style: BorderStyle::Solid, ..BorderSide::NONE },
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
    // Parent has padding-top > 0 -> no first-child collapse.
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let child = dom.create_element("div", Attributes::default());
    dom.append_child(parent, child);
    dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            margin_top: Dimension::Length(10.0),
            padding: EdgeSizes { top: 5.0, right: 0.0, bottom: 0.0, left: 0.0 },
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
