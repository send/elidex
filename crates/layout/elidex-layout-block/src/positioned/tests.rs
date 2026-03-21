use super::*;
use elidex_ecs::Attributes;
use elidex_plugin::{Direction, LayoutBox};

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < f32::EPSILON * 100.0
}

// --- resolve_offset tests ---

#[test]
fn resolve_offset_length() {
    assert_eq!(resolve_offset(&Dimension::Length(20.0), 100.0), Some(20.0));
}

#[test]
fn resolve_offset_percentage() {
    let result = resolve_offset(&Dimension::Percentage(50.0), 200.0);
    assert!(approx_eq(result.unwrap(), 100.0));
}

#[test]
fn resolve_offset_auto() {
    assert_eq!(resolve_offset(&Dimension::Auto, 200.0), None);
}

// --- is_absolutely_positioned tests ---

#[test]
fn is_absolutely_positioned_checks() {
    let make = |pos| ComputedStyle {
        position: pos,
        ..Default::default()
    };
    assert!(is_absolutely_positioned(&make(Position::Absolute)));
    assert!(is_absolutely_positioned(&make(Position::Fixed)));
    assert!(!is_absolutely_positioned(&make(Position::Relative)));
    assert!(!is_absolutely_positioned(&make(Position::Static)));
    assert!(!is_absolutely_positioned(&make(Position::Sticky)));
}

// --- collect_positioned_descendants tests ---

fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
    dom.create_element(tag, Attributes::default())
}

fn set_style(dom: &mut EcsDom, entity: Entity, pos: Position) {
    let _ = dom.world_mut().insert_one(
        entity,
        ComputedStyle {
            display: Display::Block,
            position: pos,
            ..Default::default()
        },
    );
}

#[test]
fn collect_abs_direct_child() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let child = elem(&mut dom, "div");
    dom.append_child(parent, child);
    set_style(&mut dom, parent, Position::Relative);
    set_style(&mut dom, child, Position::Absolute);

    let (abs, fixed) = collect_positioned_descendants(&dom, parent);
    assert_eq!(abs.len(), 1);
    assert_eq!(abs[0], child);
    assert!(fixed.is_empty());
}

#[test]
fn collect_abs_through_static() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let wrapper = elem(&mut dom, "div");
    let child = elem(&mut dom, "div");
    dom.append_child(parent, wrapper);
    dom.append_child(wrapper, child);
    set_style(&mut dom, parent, Position::Relative);
    set_style(&mut dom, wrapper, Position::Static);
    set_style(&mut dom, child, Position::Absolute);

    let (abs, _) = collect_positioned_descendants(&dom, parent);
    assert_eq!(abs.len(), 1);
    assert_eq!(abs[0], child);
}

#[test]
fn collect_stops_at_positioned() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let rel = elem(&mut dom, "div");
    let inner_abs = elem(&mut dom, "div");
    dom.append_child(parent, rel);
    dom.append_child(rel, inner_abs);
    set_style(&mut dom, parent, Position::Relative);
    set_style(&mut dom, rel, Position::Relative);
    set_style(&mut dom, inner_abs, Position::Absolute);

    let (abs, _) = collect_positioned_descendants(&dom, parent);
    // inner_abs should NOT be collected — rel owns it.
    assert!(abs.is_empty());
}

#[test]
fn collect_fixed_separate() {
    let mut dom = EcsDom::new();
    let parent = elem(&mut dom, "div");
    let abs_child = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    dom.append_child(parent, abs_child);
    dom.append_child(parent, fixed_child);
    set_style(&mut dom, parent, Position::Relative);
    set_style(&mut dom, abs_child, Position::Absolute);
    set_style(&mut dom, fixed_child, Position::Fixed);

    let (abs, fixed) = collect_positioned_descendants(&dom, parent);
    assert_eq!(abs.len(), 1);
    assert_eq!(abs[0], abs_child);
    assert_eq!(fixed.len(), 1);
    assert_eq!(fixed[0], fixed_child);
}

// --- apply_relative_offset tests ---

fn make_lb(x: f32, y: f32, w: f32, h: f32) -> LayoutBox {
    LayoutBox {
        content: Rect::new(x, y, w, h),
        ..Default::default()
    }
}

#[test]
fn apply_relative_offset_top_left() {
    let mut lb = make_lb(10.0, 20.0, 100.0, 50.0);
    let style = ComputedStyle {
        position: Position::Relative,
        top: Dimension::Length(5.0),
        left: Dimension::Length(10.0),
        ..Default::default()
    };
    apply_relative_offset(&mut lb, &style, 800.0, Some(600.0));
    assert!(approx_eq(lb.content.x, 20.0));
    assert!(approx_eq(lb.content.y, 25.0));
}

#[test]
fn relative_top_offset() {
    let mut lb = make_lb(0.0, 0.0, 100.0, 50.0);
    let style = ComputedStyle {
        position: Position::Relative,
        top: Dimension::Length(20.0),
        ..Default::default()
    };
    apply_relative_offset(&mut lb, &style, 800.0, Some(600.0));
    assert!(approx_eq(lb.content.y, 20.0));
    assert!(approx_eq(lb.content.x, 0.0)); // unchanged
}

#[test]
fn relative_left_offset() {
    let mut lb = make_lb(0.0, 0.0, 100.0, 50.0);
    let style = ComputedStyle {
        position: Position::Relative,
        left: Dimension::Length(10.0),
        ..Default::default()
    };
    apply_relative_offset(&mut lb, &style, 800.0, None);
    assert!(approx_eq(lb.content.x, 10.0));
}

#[test]
fn relative_bottom_when_top_auto() {
    let mut lb = make_lb(0.0, 100.0, 100.0, 50.0);
    let style = ComputedStyle {
        position: Position::Relative,
        bottom: Dimension::Length(10.0),
        ..Default::default()
    };
    apply_relative_offset(&mut lb, &style, 800.0, Some(600.0));
    // bottom:10px → move up by 10
    assert!(approx_eq(lb.content.y, 90.0));
}

#[test]
fn relative_right_when_left_auto() {
    let mut lb = make_lb(100.0, 0.0, 100.0, 50.0);
    let style = ComputedStyle {
        position: Position::Relative,
        right: Dimension::Length(10.0),
        ..Default::default()
    };
    apply_relative_offset(&mut lb, &style, 800.0, None);
    // right:10px → move left by 10
    assert!(approx_eq(lb.content.x, 90.0));
}

#[test]
fn relative_top_wins_over_bottom() {
    let mut lb = make_lb(0.0, 0.0, 100.0, 50.0);
    let style = ComputedStyle {
        position: Position::Relative,
        top: Dimension::Length(20.0),
        bottom: Dimension::Length(10.0),
        ..Default::default()
    };
    apply_relative_offset(&mut lb, &style, 800.0, Some(600.0));
    // top always wins
    assert!(approx_eq(lb.content.y, 20.0));
}

#[test]
fn relative_left_wins_over_right_ltr() {
    let mut lb = make_lb(0.0, 0.0, 100.0, 50.0);
    let style = ComputedStyle {
        position: Position::Relative,
        left: Dimension::Length(30.0),
        right: Dimension::Length(10.0),
        direction: Direction::Ltr,
        ..Default::default()
    };
    apply_relative_offset(&mut lb, &style, 800.0, None);
    // LTR: left wins
    assert!(approx_eq(lb.content.x, 30.0));
}

#[test]
fn relative_right_wins_over_left_rtl() {
    let mut lb = make_lb(0.0, 0.0, 100.0, 50.0);
    let style = ComputedStyle {
        position: Position::Relative,
        left: Dimension::Length(30.0),
        right: Dimension::Length(10.0),
        direction: Direction::Rtl,
        ..Default::default()
    };
    apply_relative_offset(&mut lb, &style, 800.0, None);
    // RTL: right wins → -10
    assert!(approx_eq(lb.content.x, -10.0));
}

#[test]
fn relative_percentage_offset() {
    let mut lb = make_lb(0.0, 0.0, 100.0, 50.0);
    let style = ComputedStyle {
        position: Position::Relative,
        top: Dimension::Percentage(50.0),
        ..Default::default()
    };
    // containing height = 200 → top = 100
    apply_relative_offset(&mut lb, &style, 800.0, Some(200.0));
    assert!(approx_eq(lb.content.y, 100.0));
}

// --- Inline axis constraint equation tests ---

fn make_inline_props(
    start: Option<f32>,
    size: Option<f32>,
    end: Option<f32>,
    margin_start_raw: f32,
    margin_end_raw: f32,
    margin_start_auto: bool,
    margin_end_auto: bool,
    pb: f32,
    containing: f32,
    static_offset: f32,
) -> InlineAxisProps {
    InlineAxisProps {
        start,
        end,
        size,
        margin_start_raw,
        margin_end_raw,
        margin_start_auto,
        margin_end_auto,
        pb,
        containing,
        static_offset,
    }
}

fn make_block_props(
    start: Option<f32>,
    end: Option<f32>,
    size: Option<f32>,
    content_size: Option<f32>,
    margin_start_raw: f32,
    margin_end_raw: f32,
    margin_start_auto: bool,
    margin_end_auto: bool,
    pb: f32,
    containing: f32,
    static_offset: f32,
) -> BlockAxisProps {
    BlockAxisProps {
        start,
        end,
        size,
        content_size,
        margin_start_raw,
        margin_end_raw,
        margin_start_auto,
        margin_end_auto,
        pb,
        containing,
        static_offset,
    }
}

#[test]
fn inline_all_three_auto() {
    // start, size, end all auto → shrink-to-fit, start = static pos
    let r = resolve_inline_axis(
        &make_inline_props(None, None, None, 0.0, 0.0, false, false, 0.0, 800.0, 50.0),
        || 200.0,
    );
    assert!(approx_eq(r.size, 200.0));
    assert!(approx_eq(r.offset, 50.0)); // static position
}

#[test]
fn inline_size_auto_stretch() {
    // start + end specified, size auto → stretch
    let r = resolve_inline_axis(
        &make_inline_props(
            Some(10.0),
            None,
            Some(20.0),
            0.0,
            0.0,
            false,
            false,
            0.0,
            800.0,
            0.0,
        ),
        || 100.0,
    );
    assert!(approx_eq(r.size, 770.0)); // 800 - 10 - 20
    assert!(approx_eq(r.offset, 10.0));
}

#[test]
fn inline_overconstrained_centering() {
    // All three specified + margin auto → centering
    let r = resolve_inline_axis(
        &make_inline_props(
            Some(0.0),
            Some(200.0),
            Some(0.0),
            0.0,
            0.0,
            true,
            true,
            0.0,
            800.0,
            0.0,
        ),
        || 200.0,
    );
    assert!(approx_eq(r.size, 200.0));
    assert!(approx_eq(r.offset, 0.0));
    // margins split remaining 600 equally
    assert!(approx_eq(r.margin_start, 300.0));
    assert!(approx_eq(r.margin_end, 300.0));
}

// --- Block axis constraint equation tests ---

#[test]
fn block_start_end_stretch() {
    // start + end specified, size auto (no content) → stretch
    let r = resolve_block_axis(&make_block_props(
        Some(10.0),
        Some(20.0),
        None,
        None,
        0.0,
        0.0,
        false,
        false,
        0.0,
        600.0,
        0.0,
    ));
    assert!(approx_eq(r.size, 570.0)); // 600 - 10 - 20
    assert!(approx_eq(r.offset, 10.0));
}

#[test]
fn block_margin_centering() {
    // start + size + end all specified + margin auto → centering
    let r = resolve_block_axis(&make_block_props(
        Some(0.0),
        Some(0.0),
        Some(200.0),
        None,
        0.0,
        0.0,
        true,
        true,
        0.0,
        600.0,
        0.0,
    ));
    assert!(approx_eq(r.size, 200.0));
    assert!(approx_eq(r.offset, 0.0));
    assert!(approx_eq(r.margin_start, 200.0));
    assert!(approx_eq(r.margin_end, 200.0));
}

#[test]
fn inline_overconstrained_ignores_end() {
    // Over-constrained with no auto margins: inline-end is ignored, start kept.
    let r = resolve_inline_axis(
        &make_inline_props(
            Some(10.0),
            Some(200.0),
            Some(50.0),
            0.0,
            0.0,
            false,
            false,
            0.0,
            800.0,
            0.0,
        ),
        || 200.0,
    );
    assert!(approx_eq(r.size, 200.0));
    assert!(approx_eq(r.offset, 10.0)); // start kept as-is
}

#[test]
fn block_centering_negative_space_equal_margins() {
    // Block axis: margin-start = margin-end always, even when negative.
    let r = resolve_block_axis(&make_block_props(
        Some(0.0),
        Some(0.0),
        Some(500.0),
        None,
        0.0,
        0.0,
        true,
        true,
        0.0,
        400.0,
        0.0,
    ));
    assert!(approx_eq(r.size, 500.0));
    assert!(approx_eq(r.offset, 0.0));
    // Both margins should be equal: -100 / 2 = -50 each
    assert!(approx_eq(r.margin_start, -50.0));
    assert!(approx_eq(r.margin_end, -50.0));
}

// --- Integration: absolute layout via block layout ---

fn setup_block_with_abs() -> (EcsDom, Entity, Entity, Entity) {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let normal = elem(&mut dom, "div");
    let abs_child = elem(&mut dom, "div");
    dom.append_child(root, normal);
    dom.append_child(root, abs_child);

    // Root: relative positioned, 800x600
    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            position: Position::Relative,
            width: Dimension::Length(800.0),
            height: Dimension::Length(600.0),
            ..Default::default()
        },
    );
    // Normal child: 800x100
    let _ = dom.world_mut().insert_one(
        normal,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(800.0),
            height: Dimension::Length(100.0),
            ..Default::default()
        },
    );
    (dom, root, normal, abs_child)
}

#[test]
fn absolute_removed_from_flow() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let sibling = elem(&mut dom, "div");
    dom.append_child(root, sibling);

    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(200.0),
            height: Dimension::Length(200.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        sibling,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(800.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    // Sibling should be at y=100 (after normal child), not y=300 (after abs)
    let sib_lb = dom.world().get::<&LayoutBox>(sibling).unwrap();
    assert!(approx_eq(sib_lb.content.y, 100.0));
}

#[test]
fn absolute_top_left_zero() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(200.0),
            height: Dimension::Length(100.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    assert!(approx_eq(lb.content.x, 0.0));
    assert!(approx_eq(lb.content.y, 0.0));
}

#[test]
fn absolute_bottom_right() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            bottom: Dimension::Length(0.0),
            right: Dimension::Length(0.0),
            width: Dimension::Length(200.0),
            height: Dimension::Length(100.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // Should be at right-bottom corner of CB (800x600)
    assert!(approx_eq(lb.content.x, 600.0)); // 800 - 200
    assert!(approx_eq(lb.content.y, 500.0)); // 600 - 100
}

#[test]
fn absolute_width_auto_stretch() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            top: Dimension::Length(0.0),
            left: Dimension::Length(50.0),
            right: Dimension::Length(50.0),
            height: Dimension::Length(100.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // width auto + left/right specified → stretch: 800 - 50 - 50 = 700
    assert!(approx_eq(lb.content.width, 700.0));
    assert!(approx_eq(lb.content.x, 50.0));
}

#[test]
fn absolute_percentage_offsets() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            top: Dimension::Percentage(10.0),  // 10% of 600 = 60
            left: Dimension::Percentage(25.0), // 25% of 800 = 200
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    assert!(approx_eq(lb.content.x, 200.0));
    assert!(approx_eq(lb.content.y, 60.0));
}

#[test]
fn absolute_cb_is_positioned_ancestor() {
    // Absolute child's CB is the nearest positioned ancestor's padding box.
    let mut dom = EcsDom::new();
    let outer = elem(&mut dom, "div");
    let rel = elem(&mut dom, "div");
    let abs_child = elem(&mut dom, "div");
    dom.append_child(outer, rel);
    dom.append_child(rel, abs_child);

    let _ = dom.world_mut().insert_one(
        outer,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(1000.0),
            height: Dimension::Length(800.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        rel,
        ComputedStyle {
            display: Display::Block,
            position: Position::Relative,
            width: Dimension::Length(400.0),
            height: Dimension::Length(300.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, outer, 1000.0, 0.0, 0.0, &font_db());

    let rel_lb = dom.world().get::<&LayoutBox>(rel).unwrap();
    let abs_lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // abs_child should be at rel's padding box origin
    assert!(approx_eq(abs_lb.content.x, rel_lb.content.x));
    assert!(approx_eq(abs_lb.content.y, rel_lb.content.y));
}

// --- Fixed positioning test ---

#[test]
fn fixed_uses_viewport() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    dom.append_child(root, fixed_child);

    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(800.0),
            height: Dimension::Length(2000.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        fixed_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Fixed,
            bottom: Dimension::Length(0.0),
            right: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    // Layout with viewport 800x600
    let input = LayoutInput {
        containing_width: 800.0,
        containing_height: None,
        containing_inline_size: 800.0,
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some((800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };
    crate::block::layout_block_inner(&mut dom, root, &input, crate::layout_block_only);

    let lb = dom.world().get::<&LayoutBox>(fixed_child).unwrap();
    // Fixed to viewport bottom-right: (800-100, 600-50) = (700, 550)
    assert!(approx_eq(lb.content.x, 700.0));
    assert!(approx_eq(lb.content.y, 550.0));
}

#[test]
fn fixed_removed_from_flow() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    let sibling = elem(&mut dom, "div");
    dom.append_child(root, fixed_child);
    dom.append_child(root, sibling);

    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(800.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        fixed_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Fixed,
            width: Dimension::Length(200.0),
            height: Dimension::Length(200.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        sibling,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(800.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    let input = LayoutInput {
        containing_width: 800.0,
        containing_height: None,
        containing_inline_size: 800.0,
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some((800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };
    crate::block::layout_block_inner(&mut dom, root, &input, crate::layout_block_only);

    // Sibling should be at y=0, not y=200
    let sib_lb = dom.world().get::<&LayoutBox>(sibling).unwrap();
    assert!(approx_eq(sib_lb.content.y, 0.0));
}

#[test]
fn fixed_top_left_zero() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    dom.append_child(root, fixed_child);

    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(800.0),
            height: Dimension::Length(2000.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        fixed_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Fixed,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    let input = LayoutInput {
        containing_width: 800.0,
        containing_height: None,
        containing_inline_size: 800.0,
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some((800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };
    crate::block::layout_block_inner(&mut dom, root, &input, crate::layout_block_only);

    let lb = dom.world().get::<&LayoutBox>(fixed_child).unwrap();
    assert!(approx_eq(lb.content.x, 0.0));
    assert!(approx_eq(lb.content.y, 0.0));
}

#[test]
fn fixed_percentage_against_viewport() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    dom.append_child(root, fixed_child);

    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(800.0),
            height: Dimension::Length(2000.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        fixed_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Fixed,
            top: Dimension::Percentage(10.0),  // 10% of 600 = 60
            left: Dimension::Percentage(25.0), // 25% of 800 = 200
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    let input = LayoutInput {
        containing_width: 800.0,
        containing_height: None,
        containing_inline_size: 800.0,
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some((800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };
    crate::block::layout_block_inner(&mut dom, root, &input, crate::layout_block_only);

    let lb = dom.world().get::<&LayoutBox>(fixed_child).unwrap();
    assert!(approx_eq(lb.content.x, 200.0));
    assert!(approx_eq(lb.content.y, 60.0));
}

#[test]
fn fixed_inside_relative() {
    // Fixed child should use viewport, not the relative parent.
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let rel = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    dom.append_child(root, rel);
    dom.append_child(rel, fixed_child);

    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(800.0),
            height: Dimension::Length(2000.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        rel,
        ComputedStyle {
            display: Display::Block,
            position: Position::Relative,
            width: Dimension::Length(400.0),
            height: Dimension::Length(300.0),
            top: Dimension::Length(100.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        fixed_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Fixed,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    let input = LayoutInput {
        containing_width: 800.0,
        containing_height: None,
        containing_inline_size: 800.0,
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some((800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };
    crate::block::layout_block_inner(&mut dom, root, &input, crate::layout_block_only);

    let lb = dom.world().get::<&LayoutBox>(fixed_child).unwrap();
    // Should be at viewport (0,0), not relative to rel parent
    assert!(approx_eq(lb.content.x, 0.0));
    assert!(approx_eq(lb.content.y, 0.0));
}

/// CSS Transforms L1 §2: static element with transform establishes CB for
/// fixed descendants, even when a positioned ancestor is in between.
///
/// ```text
/// root (static, transform: rotate(10deg))   ← CB for fixed
///   └─ container (position: relative)
///       └─ fixed-child (position: fixed)
/// ```
///
/// fixed-child should use root's padding box as CB, not viewport.
#[test]
fn fixed_inside_static_transform_ancestor_uses_transform_cb() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let root = elem(&mut dom, "div");
    let container = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    dom.append_child(doc, root);
    dom.append_child(root, container);
    dom.append_child(container, fixed_child);

    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            position: Position::Static,
            has_transform: true, // transform ancestor
            width: Dimension::Length(600.0),
            height: Dimension::Length(400.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            position: Position::Relative,
            width: Dimension::Length(400.0),
            height: Dimension::Length(300.0),
            top: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        fixed_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Fixed,
            top: Dimension::Length(10.0),
            left: Dimension::Length(20.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    let input = LayoutInput {
        containing_width: 800.0,
        containing_height: None,
        containing_inline_size: 800.0,
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some((800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };
    crate::block::layout_block_inner(&mut dom, root, &input, crate::layout_block_only);

    let lb = dom.world().get::<&LayoutBox>(fixed_child).unwrap();
    // Fixed child should be positioned relative to root's padding box (transform CB),
    // not the viewport. Root's padding box starts at (0, 0) with width 600.
    assert!(approx_eq(lb.content.x, 20.0));
    assert!(approx_eq(lb.content.y, 10.0));
}

/// Fixed element with NO transform ancestor should still use viewport.
#[test]
fn fixed_no_transform_uses_viewport() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let root = elem(&mut dom, "div");
    let container = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    dom.append_child(doc, root);
    dom.append_child(root, container);
    dom.append_child(container, fixed_child);

    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            position: Position::Static,
            // No transform
            width: Dimension::Length(600.0),
            height: Dimension::Length(400.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Block,
            position: Position::Relative,
            width: Dimension::Length(400.0),
            height: Dimension::Length(300.0),
            top: Dimension::Length(50.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        fixed_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Fixed,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    let input = LayoutInput {
        containing_width: 800.0,
        containing_height: None,
        containing_inline_size: 800.0,
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some((800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };
    crate::block::layout_block_inner(&mut dom, root, &input, crate::layout_block_only);

    let lb = dom.world().get::<&LayoutBox>(fixed_child).unwrap();
    // No transform → viewport is CB
    assert!(approx_eq(lb.content.x, 0.0));
    assert!(approx_eq(lb.content.y, 0.0));
}

/// `collect_positioned_descendants` stops fixed collection at transform boundary.
#[test]
fn collect_stops_fixed_at_transform_boundary() {
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    let parent = elem(&mut dom, "div");
    let transform_el = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    dom.append_child(doc, parent);
    dom.append_child(parent, transform_el);
    dom.append_child(transform_el, fixed_child);

    let _ = dom.world_mut().insert_one(
        parent,
        ComputedStyle {
            display: Display::Block,
            position: Position::Relative,
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        transform_el,
        ComputedStyle {
            display: Display::Block,
            position: Position::Static,
            has_transform: true,
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        fixed_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Fixed,
            ..Default::default()
        },
    );

    let (abs, fixed) = collect_positioned_descendants(&dom, parent);
    // The fixed child should NOT be collected by parent because
    // transform_el (static + has_transform) is the CB for fixed descendants.
    assert!(abs.is_empty());
    assert!(fixed.is_empty());

    // transform_el should collect the fixed child.
    let (abs2, fixed2) = collect_positioned_descendants(&dom, transform_el);
    assert!(abs2.is_empty());
    assert_eq!(fixed2.len(), 1);
    assert_eq!(fixed2[0], fixed_child);
}

// --- Writing-mode-aware positioned layout tests ---

/// Regression: horizontal-tb absolute positioning is unchanged.
#[test]
fn horizontal_tb_absolute_regression() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::HorizontalTb,
            top: Dimension::Length(10.0),
            left: Dimension::Length(20.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    assert!(approx_eq(lb.content.x, 20.0));
    assert!(approx_eq(lb.content.y, 10.0));
    assert!(approx_eq(lb.content.width, 100.0));
    assert!(approx_eq(lb.content.height, 50.0));
}

/// In vertical-rl mode, percentage margins/padding resolve against cb.height
/// (the containing block's inline size in vertical modes).
#[test]
fn vertical_rl_absolute_percentage_margin_resolves_against_cb_height() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let abs_child = elem(&mut dom, "div");
    dom.append_child(root, abs_child);

    // Root: 800x600, relative positioned
    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            position: Position::Relative,
            width: Dimension::Length(800.0),
            height: Dimension::Length(600.0),
            ..Default::default()
        },
    );

    // Abs child: vertical-rl, margin-left: 10% (should resolve against cb.height=600)
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            margin_left: Dimension::Percentage(10.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // In vertical-rl, inline_containing = cb.height = 600.
    // margin-left: 10% of 600 = 60.
    // x = cb.x(0) + left(0) + margin_left(60) + border(0) + padding(0) = 60
    assert!(approx_eq(lb.content.x, 60.0));
    assert!(approx_eq(lb.content.y, 0.0));
}

/// vertical-rl: absolute child with top/left offsets should position correctly.
#[test]
fn vertical_rl_absolute_basic_positioning() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(10.0),
            left: Dimension::Length(20.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    assert!(approx_eq(lb.content.x, 20.0));
    assert!(approx_eq(lb.content.y, 10.0));
    assert!(approx_eq(lb.content.width, 100.0));
    assert!(approx_eq(lb.content.height, 50.0));
}

/// vertical-rl: auto margins on an axis should center the element.
#[test]
fn vertical_rl_absolute_auto_margins_center() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            right: Dimension::Length(0.0),
            width: Dimension::Length(200.0),
            height: Dimension::Length(50.0),
            margin_left: Dimension::Auto,
            margin_right: Dimension::Auto,
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // Centered horizontally: (800 - 200) / 2 = 300 per auto margin.
    assert!(approx_eq(lb.content.width, 200.0));
    assert!(approx_eq(lb.margin.left, 300.0));
    assert!(approx_eq(lb.margin.right, 300.0));
}

/// vertical-rl: stretch with left+right and auto width.
#[test]
fn vertical_rl_absolute_stretch() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(10.0),
            left: Dimension::Length(50.0),
            right: Dimension::Length(50.0),
            height: Dimension::Length(40.0),
            // width: auto → stretch
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // width = cb(800) - left(50) - right(50) - h_pb(0) - margin(0) = 700
    assert!(approx_eq(lb.content.width, 700.0));
    assert!(approx_eq(lb.content.x, 50.0));
}

/// vertical-lr: basic absolute positioning.
#[test]
fn vertical_lr_absolute_basic_positioning() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalLr,
            top: Dimension::Length(15.0),
            left: Dimension::Length(25.0),
            width: Dimension::Length(80.0),
            height: Dimension::Length(60.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    assert!(approx_eq(lb.content.x, 25.0));
    assert!(approx_eq(lb.content.y, 15.0));
    assert!(approx_eq(lb.content.width, 80.0));
    assert!(approx_eq(lb.content.height, 60.0));
}

/// vertical-rl: percentage padding should resolve against cb.height (inline size).
#[test]
fn vertical_rl_absolute_padding_pct_resolves_against_inline_size() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let abs_child = elem(&mut dom, "div");
    dom.append_child(root, abs_child);

    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            position: Position::Relative,
            width: Dimension::Length(800.0),
            height: Dimension::Length(600.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            padding: elidex_plugin::EdgeSizes {
                top: Dimension::Percentage(10.0),
                right: Dimension::ZERO,
                bottom: Dimension::ZERO,
                left: Dimension::ZERO,
            },
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // vertical-rl: inline_containing = cb.height = 600. 10% of 600 = 60.
    assert!(
        approx_eq(lb.padding.top, 60.0),
        "padding-top should be 60 (10% of 600), got {}",
        lb.padding.top,
    );
}

/// vertical-rl: percentage height should resolve against cb.height.
#[test]
fn vertical_rl_absolute_percentage_height() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(0.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Percentage(50.0), // 50% of cb.height=600 = 300
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    assert!(
        approx_eq(lb.content.height, 300.0),
        "height should be 300 (50% of 600), got {}",
        lb.content.height,
    );
}

/// apply_relative_offset works correctly in vertical-rl mode.
/// In vertical-rl: inline axis = top/bottom (top is inline-start for LTR),
/// block axis = left/right (right is block-start for rl).
#[test]
fn vertical_rl_relative_offset() {
    use elidex_plugin::EdgeSizes;
    let mut lb = LayoutBox {
        content: Rect::new(100.0, 200.0, 50.0, 30.0),
        padding: EdgeSizes::default(),
        border: EdgeSizes::default(),
        margin: EdgeSizes::default(),
        first_baseline: None,
    };
    let style = ComputedStyle {
        position: Position::Relative,
        writing_mode: elidex_plugin::WritingMode::VerticalRl,
        top: Dimension::Length(10.0),  // inline-start offset → dy = +10
        left: Dimension::Length(20.0), // block-end offset (vertical-rl: block-start=right)
        ..Default::default()
    };
    // vertical-rl LTR:
    //   inline axis (top/bottom): top(inline-start) wins → dy = +10
    //   block axis (left/right): left only specified → dx = +20
    apply_relative_offset(&mut lb, &style, 800.0, Some(600.0));
    assert!(approx_eq(lb.content.x, 120.0)); // 100 + 20
    assert!(approx_eq(lb.content.y, 210.0)); // 200 + 10
}

/// vertical-rl: top+bottom stretch (auto height).
#[test]
fn vertical_rl_absolute_top_bottom_stretch_height() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(100.0),
            bottom: Dimension::Length(100.0),
            left: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            // height: auto → stretch between top and bottom
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // height = cb(600) - top(100) - bottom(100) - v_pb(0) - margins(0) = 400
    assert!(
        approx_eq(lb.content.height, 400.0),
        "height should be 400 (stretched), got {}",
        lb.content.height,
    );
}

/// vertical-rl: over-constrained with all three specified (left+width+right).
/// In vertical-rl: block axis = horizontal, block-start = right, block-end = left.
/// Over-constrained block axis: block-end (left) is ignored, block-start (right) wins.
#[test]
fn vertical_rl_absolute_over_constrained() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(0.0),
            left: Dimension::Length(50.0),
            right: Dimension::Length(50.0),
            width: Dimension::Length(600.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // vertical-rl block axis: block-start = right(50), block-end = left(50) (ignored).
    // x = cb_width(800) - right(50) - width(600) = 150
    assert!(
        approx_eq(lb.content.x, 150.0),
        "expected x=150 (right wins in vertical-rl), got {}",
        lb.content.x,
    );
    assert!(approx_eq(lb.content.width, 600.0));
}

/// Horizontal-tb relative offset regression.
#[test]
fn horizontal_tb_relative_offset_regression() {
    use elidex_plugin::EdgeSizes;
    let mut lb = LayoutBox {
        content: Rect::new(0.0, 0.0, 100.0, 50.0),
        padding: EdgeSizes::default(),
        border: EdgeSizes::default(),
        margin: EdgeSizes::default(),
        first_baseline: None,
    };
    let style = ComputedStyle {
        position: Position::Relative,
        writing_mode: elidex_plugin::WritingMode::HorizontalTb,
        top: Dimension::Length(10.0),
        left: Dimension::Length(20.0),
        ..Default::default()
    };
    apply_relative_offset(&mut lb, &style, 800.0, Some(600.0));
    assert!(approx_eq(lb.content.x, 20.0));
    assert!(approx_eq(lb.content.y, 10.0));
    assert!(approx_eq(lb.content.width, 100.0));
    assert!(approx_eq(lb.content.height, 50.0));
}

// ---------------------------------------------------------------------------
// Step 9: Writing-mode-aware positioned layout — additional tests
// ---------------------------------------------------------------------------

/// vertical-rl: min/max constraints on block-size (physical width).
#[test]
fn vertical_rl_absolute_min_max_block_size() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(0.0),
            right: Dimension::Length(0.0),
            // width (block-size): auto → stretch between right(0) and left(auto)
            // but min-width = 500 should clamp
            min_width: Dimension::Length(500.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    assert!(
        approx_eq(lb.content.width, 500.0),
        "min-width should clamp to 500, got {}",
        lb.content.width,
    );
}

/// vertical-rl: min/max constraints on inline-size (physical height).
#[test]
fn vertical_rl_absolute_min_max_inline_size() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(0.0),
            right: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            max_height: Dimension::Length(30.0), // max-height clamps inline-size
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    assert!(
        approx_eq(lb.content.height, 30.0),
        "max-height should clamp inline-size to 30, got {}",
        lb.content.height,
    );
}

/// RTL + vertical-rl: inline direction is reversed (inline-start = bottom).
/// bottom offset should be treated as inline-start.
#[test]
fn vertical_rl_rtl_absolute_positioning() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            direction: Direction::Rtl,
            // In vertical-rl RTL: inline-start = bottom, inline-end = top.
            // block-start = right, block-end = left.
            bottom: Dimension::Length(20.0), // inline-start offset
            right: Dimension::Length(30.0),  // block-start offset
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // Physical y = cb.height(600) - bottom(20) - height(50) = 530
    assert!(
        approx_eq(lb.content.y, 530.0),
        "vertical-rl RTL: y should be 530 (bottom=20 from bottom edge), got {}",
        lb.content.y,
    );
    // Physical x: block-start = right(30). x = cb_width(800) - right(30) - width(100) = 670
    assert!(
        approx_eq(lb.content.x, 670.0),
        "vertical-rl RTL: x should be 670 (right=30), got {}",
        lb.content.x,
    );
}

/// vertical-rl: auto margin centering on inline axis (top/bottom).
#[test]
fn vertical_rl_absolute_auto_margins_center_inline_axis() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(0.0),
            bottom: Dimension::Length(0.0),
            right: Dimension::Length(0.0),
            width: Dimension::Length(100.0),
            height: Dimension::Length(200.0),
            margin_top: Dimension::Auto,
            margin_bottom: Dimension::Auto,
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // Inline axis centering: (600 - 200) / 2 = 200 per auto margin.
    // In vertical-rl: inline-axis margins are physical top/bottom.
    // Negative centering absorbs into inline-end (bottom in LTR).
    assert!(
        approx_eq(lb.content.height, 200.0),
        "height should be 200, got {}",
        lb.content.height,
    );
    assert!(
        approx_eq(lb.margin.top, 200.0),
        "margin-top should be 200 (centered), got {}",
        lb.margin.top,
    );
    assert!(
        approx_eq(lb.margin.bottom, 200.0),
        "margin-bottom should be 200 (centered), got {}",
        lb.margin.bottom,
    );
}

/// vertical-lr: block-start = left (not reversed). left+right+width over-constrained
/// should ignore block-end (right).
#[test]
fn vertical_lr_absolute_over_constrained() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalLr,
            top: Dimension::Length(0.0),
            left: Dimension::Length(50.0),
            right: Dimension::Length(50.0),
            width: Dimension::Length(600.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    // vertical-lr: block-start = left(50), block-end = right(50) (ignored).
    // x = left(50)
    assert!(
        approx_eq(lb.content.x, 50.0),
        "vertical-lr over-constrained: x should be 50 (left wins), got {}",
        lb.content.x,
    );
    assert!(approx_eq(lb.content.width, 600.0));
}

/// vertical-rl: relative offset with both top+bottom specified.
/// In vertical-rl LTR: inline axis = top/bottom, top (inline-start) wins.
#[test]
fn vertical_rl_relative_offset_both_top_bottom() {
    use elidex_plugin::EdgeSizes;
    let mut lb = LayoutBox {
        content: Rect::new(100.0, 200.0, 50.0, 30.0),
        padding: EdgeSizes::default(),
        border: EdgeSizes::default(),
        margin: EdgeSizes::default(),
        first_baseline: None,
    };
    let style = ComputedStyle {
        position: Position::Relative,
        writing_mode: elidex_plugin::WritingMode::VerticalRl,
        top: Dimension::Length(10.0),
        bottom: Dimension::Length(20.0),
        ..Default::default()
    };
    // vertical-rl LTR: top (inline-start) wins over bottom (inline-end).
    apply_relative_offset(&mut lb, &style, 800.0, Some(600.0));
    assert!(
        approx_eq(lb.content.y, 210.0), // 200 + 10 (top wins)
        "top (inline-start) should win, got y={}",
        lb.content.y,
    );
}

/// vertical-rl: relative offset with both left+right specified.
/// In vertical-rl: block axis = left/right, block-start = right. Right wins.
#[test]
fn vertical_rl_relative_offset_both_left_right() {
    use elidex_plugin::EdgeSizes;
    let mut lb = LayoutBox {
        content: Rect::new(100.0, 200.0, 50.0, 30.0),
        padding: EdgeSizes::default(),
        border: EdgeSizes::default(),
        margin: EdgeSizes::default(),
        first_baseline: None,
    };
    let style = ComputedStyle {
        position: Position::Relative,
        writing_mode: elidex_plugin::WritingMode::VerticalRl,
        left: Dimension::Length(10.0),
        right: Dimension::Length(20.0),
        ..Default::default()
    };
    // vertical-rl: block-start = right. When both specified, block-start (right) wins.
    apply_relative_offset(&mut lb, &style, 800.0, Some(600.0));
    assert!(
        approx_eq(lb.content.x, 80.0), // 100 + (-20) = 80, right wins
        "right (block-start) should win in vertical-rl, got x={}",
        lb.content.x,
    );
}

/// vertical-lr: relative offset with both left+right specified.
/// In vertical-lr: block-start = left. Left wins.
#[test]
fn vertical_lr_relative_offset_both_left_right() {
    use elidex_plugin::EdgeSizes;
    let mut lb = LayoutBox {
        content: Rect::new(100.0, 200.0, 50.0, 30.0),
        padding: EdgeSizes::default(),
        border: EdgeSizes::default(),
        margin: EdgeSizes::default(),
        first_baseline: None,
    };
    let style = ComputedStyle {
        position: Position::Relative,
        writing_mode: elidex_plugin::WritingMode::VerticalLr,
        left: Dimension::Length(10.0),
        right: Dimension::Length(20.0),
        ..Default::default()
    };
    // vertical-lr: block-start = left. Left wins.
    apply_relative_offset(&mut lb, &style, 800.0, Some(600.0));
    assert!(
        approx_eq(lb.content.x, 110.0), // 100 + 10 (left wins)
        "left (block-start) should win in vertical-lr, got x={}",
        lb.content.x,
    );
}

/// Fixed positioning in vertical-rl should work correctly.
#[test]
fn vertical_rl_fixed_positioning() {
    let mut dom = EcsDom::new();
    let root = elem(&mut dom, "div");
    let fixed_child = elem(&mut dom, "div");
    dom.append_child(root, fixed_child);

    let _ = dom.world_mut().insert_one(
        root,
        ComputedStyle {
            display: Display::Block,
            width: Dimension::Length(800.0),
            height: Dimension::Length(600.0),
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        fixed_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Fixed,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            right: Dimension::Length(10.0), // block-start
            top: Dimension::Length(20.0),   // inline-start
            width: Dimension::Length(100.0),
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    // Use viewport 1024x768.
    let input = LayoutInput {
        containing_width: 1024.0,
        containing_height: None,
        containing_inline_size: 1024.0,
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some((1024.0, 768.0)),
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };
    crate::block::layout_block_inner(&mut dom, root, &input, crate::layout_block_only);

    let lb = dom.world().get::<&LayoutBox>(fixed_child).unwrap();
    // vertical-rl: block-start = right(10). x = viewport_w(1024) - right(10) - width(100) = 914
    assert!(
        approx_eq(lb.content.x, 914.0),
        "fixed vertical-rl: x should be 914, got {}",
        lb.content.x,
    );
    // inline-start = top(20). y = 20
    assert!(
        approx_eq(lb.content.y, 20.0),
        "fixed vertical-rl: y should be 20, got {}",
        lb.content.y,
    );
}

/// vertical-rl: percentage width (block-size) resolves against cb.width.
#[test]
fn vertical_rl_absolute_percentage_width() {
    let (mut dom, root, _normal, abs_child) = setup_block_with_abs();
    let _ = dom.world_mut().insert_one(
        abs_child,
        ComputedStyle {
            display: Display::Block,
            position: Position::Absolute,
            writing_mode: elidex_plugin::WritingMode::VerticalRl,
            top: Dimension::Length(0.0),
            right: Dimension::Length(0.0),
            width: Dimension::Percentage(50.0), // 50% of cb.width=800 = 400
            height: Dimension::Length(50.0),
            ..Default::default()
        },
    );

    crate::block::layout_block(&mut dom, root, 800.0, 0.0, 0.0, &font_db());

    let lb = dom.world().get::<&LayoutBox>(abs_child).unwrap();
    assert!(
        approx_eq(lb.content.width, 400.0),
        "width should be 400 (50% of 800), got {}",
        lb.content.width,
    );
}

/// horizontal-tb: inline over-constrained negative centering absorbs into end-side.
#[test]
fn horizontal_tb_inline_negative_centering() {
    let r = resolve_inline_axis(
        &make_inline_props(
            Some(0.0),
            Some(900.0),
            Some(0.0),
            0.0,
            0.0,
            true,
            true,
            0.0,
            800.0,
            0.0,
        ),
        || 900.0,
    );
    // size(900) > containing(800): available = 800 - 0 - 900 - 0 - 0 = -100
    // Negative centering: margin_start = 0, margin_end absorbs = -100.
    assert!(approx_eq(r.size, 900.0));
    assert!(approx_eq(r.margin_start, 0.0));
    assert!(approx_eq(r.margin_end, -100.0));
}

fn font_db() -> elidex_text::FontDatabase {
    elidex_text::FontDatabase::new()
}
