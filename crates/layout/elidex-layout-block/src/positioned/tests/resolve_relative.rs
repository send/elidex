use super::*;

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
    assert!(approx_eq(lb.content.origin.x, 20.0));
    assert!(approx_eq(lb.content.origin.y, 25.0));
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
    assert!(approx_eq(lb.content.origin.y, 20.0));
    assert!(approx_eq(lb.content.origin.x, 0.0)); // unchanged
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
    assert!(approx_eq(lb.content.origin.x, 10.0));
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
    assert!(approx_eq(lb.content.origin.y, 90.0));
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
    assert!(approx_eq(lb.content.origin.x, 90.0));
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
    assert!(approx_eq(lb.content.origin.y, 20.0));
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
    assert!(approx_eq(lb.content.origin.x, 30.0));
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
    assert!(approx_eq(lb.content.origin.x, -10.0));
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
    assert!(approx_eq(lb.content.origin.y, 100.0));
}
