use super::*;
use elidex_plugin::ComputedStyle;

#[test]
fn basic_counter_reset_and_increment() {
    let mut cs = CounterState::new();

    // Simulate: element resets counter "c" to 0.
    let mut style = ComputedStyle::default();
    style
        .counter_reset
        .push(CounterResetEntry::new("c".to_string(), 0));
    cs.push_scope();
    cs.process_element(&style, false);
    assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "0");

    // Simulate: child increments counter "c" by 1.
    let mut style2 = ComputedStyle::default();
    style2.counter_increment.push(("c".to_string(), 1));
    cs.push_scope();
    cs.process_element(&style2, false);
    assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "1");
}

#[test]
fn counter_reset_creates_scope() {
    let mut cs = CounterState::new();

    // Outer reset to 5.
    let mut style = ComputedStyle::default();
    style
        .counter_reset
        .push(CounterResetEntry::new("c".to_string(), 5));
    cs.push_scope();
    cs.process_element(&style, false);
    assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "5");

    // Inner reset to 10 — creates a new scope entry, shadowing outer.
    let mut style2 = ComputedStyle::default();
    style2
        .counter_reset
        .push(CounterResetEntry::new("c".to_string(), 10));
    cs.push_scope();
    cs.process_element(&style2, false);
    assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "10");
}

#[test]
fn pop_scope_removes_counters() {
    let mut cs = CounterState::new();

    // Outer scope: reset to 5.
    let mut style = ComputedStyle::default();
    style
        .counter_reset
        .push(CounterResetEntry::new("c".to_string(), 5));
    cs.push_scope();
    cs.process_element(&style, false);

    // Inner scope: reset to 10.
    let mut style2 = ComputedStyle::default();
    style2
        .counter_reset
        .push(CounterResetEntry::new("c".to_string(), 10));
    cs.push_scope();
    cs.process_element(&style2, false);
    assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "10");

    // Pop inner scope — should restore to 5.
    cs.pop_scope();
    assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "5");
}

#[test]
fn nested_counter_scopes() {
    let mut cs = CounterState::new();

    // ol resets list-item to 0.
    let mut ol_style = ComputedStyle::default();
    ol_style
        .counter_reset
        .push(CounterResetEntry::new("list-item".to_string(), 0));
    cs.push_scope();
    cs.process_element(&ol_style, false);

    // li increments.
    let mut li_style = ComputedStyle::default();
    li_style
        .counter_increment
        .push(("list-item".to_string(), 1));
    cs.push_scope();
    cs.process_element(&li_style, false);
    assert_eq!(
        cs.evaluate_counter("list-item", ListStyleType::Decimal),
        "1"
    );
    cs.pop_scope();

    // Nested ol resets again.
    let mut inner_ol = ComputedStyle::default();
    inner_ol
        .counter_reset
        .push(CounterResetEntry::new("list-item".to_string(), 0));
    cs.push_scope();
    cs.process_element(&inner_ol, false);

    // Inner li increments.
    cs.push_scope();
    cs.process_element(&li_style, false);
    assert_eq!(
        cs.evaluate_counter("list-item", ListStyleType::Decimal),
        "1"
    );
    cs.pop_scope();

    // Pop inner ol scope.
    cs.pop_scope();

    // Second outer li — increments the outer counter.
    cs.push_scope();
    cs.process_element(&li_style, false);
    assert_eq!(
        cs.evaluate_counter("list-item", ListStyleType::Decimal),
        "2"
    );
    cs.pop_scope();
}

#[test]
fn counters_concatenation() {
    let mut cs = CounterState::new();

    // Outer reset.
    let mut reset = ComputedStyle::default();
    reset
        .counter_reset
        .push(CounterResetEntry::new("c".to_string(), 0));
    cs.push_scope();
    cs.process_element(&reset, false);

    // Increment to 1.
    let mut inc = ComputedStyle::default();
    inc.counter_increment.push(("c".to_string(), 1));
    cs.push_scope();
    cs.process_element(&inc, false);

    // Nested reset.
    cs.push_scope();
    cs.process_element(&reset, false);

    // Nested increment to 1.
    cs.push_scope();
    cs.process_element(&inc, false);

    // counters() should show "1.1".
    assert_eq!(
        cs.evaluate_counters("c", ".", ListStyleType::Decimal),
        "1.1"
    );

    cs.pop_scope();
    cs.pop_scope();
    cs.pop_scope();

    // After popping inner scopes, only outer "1" remains.
    assert_eq!(cs.evaluate_counters("c", ".", ListStyleType::Decimal), "1");
}

#[test]
fn counter_set_overwrites_value() {
    let mut cs = CounterState::new();

    // Reset to 0.
    let mut style = ComputedStyle::default();
    style
        .counter_reset
        .push(CounterResetEntry::new("c".to_string(), 0));
    cs.push_scope();
    cs.process_element(&style, false);

    // Set to 42.
    let mut set_style = ComputedStyle::default();
    set_style.counter_set.push(("c".to_string(), 42));
    cs.push_scope();
    cs.process_element(&set_style, false);
    assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "42");
}

#[test]
fn counter_set_creates_if_not_exists() {
    let mut cs = CounterState::new();

    // counter-set on a non-existent counter creates it (§4.2).
    let mut style = ComputedStyle::default();
    style.counter_set.push(("newc".to_string(), 7));
    cs.push_scope();
    cs.process_element(&style, false);
    assert_eq!(cs.evaluate_counter("newc", ListStyleType::Decimal), "7");
}

#[test]
fn increment_skipped_on_continuation() {
    let mut cs = CounterState::new();

    let mut style = ComputedStyle::default();
    style
        .counter_reset
        .push(CounterResetEntry::new("c".to_string(), 0));
    cs.push_scope();
    cs.process_element(&style, false);

    // Normal increment.
    let mut inc = ComputedStyle::default();
    inc.counter_increment.push(("c".to_string(), 1));
    cs.push_scope();
    cs.process_element(&inc, false);
    assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "1");
    cs.pop_scope();

    // Continuation — increment skipped.
    cs.push_scope();
    cs.process_element(&inc, true);
    assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "1");
    cs.pop_scope();

    // Normal increment again.
    cs.push_scope();
    cs.process_element(&inc, false);
    assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "2");
}

#[test]
fn multiple_counters_on_one_element() {
    let mut cs = CounterState::new();

    let mut style = ComputedStyle::default();
    style
        .counter_reset
        .push(CounterResetEntry::new("a".to_string(), 10));
    style
        .counter_reset
        .push(CounterResetEntry::new("b".to_string(), 20));
    style.counter_increment.push(("a".to_string(), 5));
    cs.push_scope();
    cs.process_element(&style, false);

    // a was reset to 10 then incremented by 5.
    assert_eq!(cs.evaluate_counter("a", ListStyleType::Decimal), "15");
    // b was reset to 20, no increment.
    assert_eq!(cs.evaluate_counter("b", ListStyleType::Decimal), "20");
}

#[test]
fn format_decimal_and_alpha() {
    assert_eq!(format_counter_value(1, ListStyleType::Decimal), "1");
    assert_eq!(format_counter_value(42, ListStyleType::Decimal), "42");
    assert_eq!(
        format_counter_value(1, ListStyleType::DecimalLeadingZero),
        "01"
    );
    assert_eq!(
        format_counter_value(10, ListStyleType::DecimalLeadingZero),
        "10"
    );

    // Alpha: 1=a, 26=z, 27=aa.
    assert_eq!(format_counter_value(1, ListStyleType::LowerAlpha), "a");
    assert_eq!(format_counter_value(26, ListStyleType::LowerAlpha), "z");
    assert_eq!(format_counter_value(27, ListStyleType::LowerAlpha), "aa");
    assert_eq!(format_counter_value(1, ListStyleType::UpperAlpha), "A");

    // Roman.
    assert_eq!(format_counter_value(4, ListStyleType::LowerRoman), "iv");
    assert_eq!(format_counter_value(9, ListStyleType::UpperRoman), "IX");
    assert_eq!(
        format_counter_value(2024, ListStyleType::LowerRoman),
        "mmxxiv"
    );

    // Bullets.
    assert_eq!(format_counter_value(1, ListStyleType::Disc), "\u{2022}");
    assert_eq!(format_counter_value(1, ListStyleType::Circle), "\u{25E6}");
    assert_eq!(format_counter_value(1, ListStyleType::Square), "\u{25AA}");

    // None.
    assert_eq!(format_counter_value(1, ListStyleType::None), "");

    // Decimal-leading-zero: negative values use plain decimal (no zero-pad).
    assert_eq!(
        format_counter_value(-1, ListStyleType::DecimalLeadingZero),
        "-1"
    );
    assert_eq!(
        format_counter_value(-10, ListStyleType::DecimalLeadingZero),
        "-10"
    );
    // Zero still gets padded.
    assert_eq!(
        format_counter_value(0, ListStyleType::DecimalLeadingZero),
        "00"
    );

    // Non-positive alpha/roman fall back to decimal.
    assert_eq!(format_counter_value(0, ListStyleType::LowerAlpha), "0");
    assert_eq!(format_counter_value(-1, ListStyleType::LowerRoman), "-1");
}

#[test]
fn reversed_counter_negates_increment() {
    let mut cs = CounterState::new();

    // Reset reversed counter to 5.
    let mut style = ComputedStyle::default();
    style
        .counter_reset
        .push(CounterResetEntry::reversed("c", 5));
    cs.push_scope();
    cs.process_element(&style, false);
    assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "5");

    // Increment by 1 → reversed negates to -1 → value becomes 4.
    let mut inc = ComputedStyle::default();
    inc.counter_increment.push(("c".to_string(), 1));
    cs.push_scope();
    cs.process_element(&inc, false);
    assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "4");
    cs.pop_scope();

    // Increment again → 3.
    cs.push_scope();
    cs.process_element(&inc, false);
    assert_eq!(cs.evaluate_counter("c", ListStyleType::Decimal), "3");
}

#[test]
fn reversed_ol_implicit_counters() {
    // Simulate <ol reversed> with 3 <li> children (no start attribute).
    let mut cs = CounterState::new();

    // <ol reversed> → reversed counter reset to li_count (3).
    let mut ol_style = ComputedStyle::default();
    let mut attrs = elidex_ecs::Attributes::default();
    attrs.set("reversed", "");
    apply_implicit_list_counters(&mut ol_style, "ol", &attrs, 3);

    assert!(ol_style.counter_reset[0].reversed);
    // Initial value = li_count + 1 = 4, so first <li> decrements to 3.
    // Same pattern as normal: normal init=0, first <li> increments to 1.
    assert_eq!(ol_style.counter_reset[0].value, 4);

    cs.push_scope();
    cs.process_element(&ol_style, false);
    let mut li_style = ComputedStyle::default();
    li_style
        .counter_increment
        .push(("list-item".to_string(), 1));

    // First <li>
    cs.push_scope();
    cs.process_element(&li_style, false);
    assert_eq!(
        cs.evaluate_counter("list-item", ListStyleType::Decimal),
        "3",
        "first li in reversed ol with 3 items should show 3"
    );
    cs.pop_scope();

    // Second <li>
    cs.push_scope();
    cs.process_element(&li_style, false);
    assert_eq!(
        cs.evaluate_counter("list-item", ListStyleType::Decimal),
        "2"
    );
    cs.pop_scope();

    // Third <li>
    cs.push_scope();
    cs.process_element(&li_style, false);
    assert_eq!(
        cs.evaluate_counter("list-item", ListStyleType::Decimal),
        "1"
    );
}
