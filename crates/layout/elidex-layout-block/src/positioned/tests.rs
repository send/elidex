use super::*;
use elidex_ecs::Attributes;
use elidex_plugin::LayoutBox;

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

// --- Horizontal constraint equation tests ---

#[test]
fn horizontal_all_three_auto() {
    // left, width, right all auto → shrink-to-fit, left = static pos
    let (w, _ml, _mr, l) = resolve_horizontal(
        None,
        None,
        None,
        0.0,
        0.0,
        0.0,
        800.0,
        50.0,
        &ComputedStyle::default(),
        || 200.0,
    );
    assert!(approx_eq(w, 200.0));
    assert!(approx_eq(l, 50.0)); // static position
}

#[test]
fn horizontal_width_auto_stretch() {
    // left + right specified, width auto → stretch
    let (w, _ml, _mr, l) = resolve_horizontal(
        Some(10.0),
        None,
        Some(20.0),
        0.0,
        0.0,
        0.0,
        800.0,
        0.0,
        &ComputedStyle::default(),
        || 100.0,
    );
    assert!(approx_eq(w, 770.0)); // 800 - 10 - 20
    assert!(approx_eq(l, 10.0));
}

#[test]
fn horizontal_overconstrained_centering() {
    // All three specified + margin auto → centering
    let style = ComputedStyle {
        margin_left: Dimension::Auto,
        margin_right: Dimension::Auto,
        ..Default::default()
    };
    let (w, ml, mr, l) = resolve_horizontal(
        Some(0.0),
        Some(200.0),
        Some(0.0),
        0.0,
        0.0,
        0.0,
        800.0,
        0.0,
        &style,
        || 200.0,
    );
    assert!(approx_eq(w, 200.0));
    assert!(approx_eq(l, 0.0));
    // margins split remaining 600 equally
    assert!(approx_eq(ml, 300.0));
    assert!(approx_eq(mr, 300.0));
}

// --- Vertical constraint equation tests ---

#[test]
fn vertical_top_bottom_stretch() {
    // top + bottom specified, height auto (no content) → stretch
    let (h, _mt, _mb, t) = resolve_vertical(
        Some(10.0),
        Some(20.0),
        None,
        None,
        0.0,
        0.0,
        0.0,
        600.0,
        0.0,
        &ComputedStyle::default(),
    );
    assert!(approx_eq(h, 570.0)); // 600 - 10 - 20
    assert!(approx_eq(t, 10.0));
}

#[test]
fn vertical_margin_centering() {
    // top + height + bottom all specified + margin auto → centering
    let style = ComputedStyle {
        margin_top: Dimension::Auto,
        margin_bottom: Dimension::Auto,
        ..Default::default()
    };
    let (h, mt, mb, t) = resolve_vertical(
        Some(0.0),
        Some(0.0),
        Some(200.0),
        None,
        0.0,
        0.0,
        0.0,
        600.0,
        0.0,
        &style,
    );
    assert!(approx_eq(h, 200.0));
    assert!(approx_eq(t, 0.0));
    assert!(approx_eq(mt, 200.0));
    assert!(approx_eq(mb, 200.0));
}

#[test]
fn horizontal_overconstrained_rtl_ignores_left() {
    // CSS 2.1 §10.3.7: RTL over-constrained → ignore left, solve for left from right.
    let style = ComputedStyle {
        direction: Direction::Rtl,
        ..Default::default()
    };
    let (w, _ml, _mr, l) = resolve_horizontal(
        Some(10.0),  // left (should be ignored in RTL)
        Some(200.0), // width
        Some(50.0),  // right
        0.0,
        0.0,
        0.0,
        800.0,
        0.0,
        &style,
        || 200.0,
    );
    assert!(approx_eq(w, 200.0));
    // RTL: left = cb_width - right - width - h_pb - ml - mr = 800 - 50 - 200 = 550
    assert!(approx_eq(l, 550.0));
}

#[test]
fn horizontal_overconstrained_ltr_ignores_right() {
    // CSS 2.1 §10.3.7: LTR over-constrained → ignore right, keep left.
    let (w, _ml, _mr, l) = resolve_horizontal(
        Some(10.0),  // left (kept in LTR)
        Some(200.0), // width
        Some(50.0),  // right (ignored)
        0.0,
        0.0,
        0.0,
        800.0,
        0.0,
        &ComputedStyle::default(),
        || 200.0,
    );
    assert!(approx_eq(w, 200.0));
    assert!(approx_eq(l, 10.0)); // left kept as-is
}

#[test]
fn vertical_centering_negative_space_equal_margins() {
    // CSS 2.1 §10.6.4: margin-top = margin-bottom always, even when negative.
    // (Unlike horizontal §10.3.7 which has directional asymmetry.)
    let style = ComputedStyle {
        margin_top: Dimension::Auto,
        margin_bottom: Dimension::Auto,
        ..Default::default()
    };
    // height(500) > cb_height(400) → available = 400 - 0 - 500 - 0 - 0 = -100
    let (h, mt, mb, t) = resolve_vertical(
        Some(0.0),
        Some(0.0),
        Some(500.0),
        None,
        0.0,
        0.0,
        0.0,
        400.0,
        0.0,
        &style,
    );
    assert!(approx_eq(h, 500.0));
    assert!(approx_eq(t, 0.0));
    // Both margins should be equal: -100 / 2 = -50 each
    assert!(approx_eq(mt, -50.0));
    assert!(approx_eq(mb, -50.0));
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
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some((800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
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
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some((800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
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
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some((800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
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
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some((800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
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
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some((800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
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
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some((800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
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
        offset_x: 0.0,
        offset_y: 0.0,
        font_db: &font_db(),
        depth: 0,
        float_ctx: None,
        viewport: Some((800.0, 600.0)),
        fragmentainer: None,
        break_token: None,
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

fn font_db() -> elidex_text::FontDatabase {
    elidex_text::FontDatabase::new()
}
