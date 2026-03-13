use super::*;

#[test]
fn parse_time_seconds() {
    let mut pi = cssparser::ParserInput::new("0.3s");
    let mut parser = cssparser::Parser::new(&mut pi);
    assert_eq!(parse_time(&mut parser).unwrap(), 0.3);
}

#[test]
fn parse_time_milliseconds() {
    let mut pi = cssparser::ParserInput::new("200ms");
    let mut parser = cssparser::Parser::new(&mut pi);
    assert_eq!(parse_time(&mut parser).unwrap(), 0.2);
}

#[test]
fn parse_time_zero() {
    let mut pi = cssparser::ParserInput::new("0");
    let mut parser = cssparser::Parser::new(&mut pi);
    assert_eq!(parse_time(&mut parser).unwrap(), 0.0);
}

#[test]
fn parse_timing_ease() {
    let mut pi = cssparser::ParserInput::new("ease");
    let mut parser = cssparser::Parser::new(&mut pi);
    assert_eq!(
        parse_timing_function(&mut parser).unwrap(),
        TimingFunction::EASE
    );
}

#[test]
fn parse_timing_linear() {
    let mut pi = cssparser::ParserInput::new("linear");
    let mut parser = cssparser::Parser::new(&mut pi);
    assert_eq!(
        parse_timing_function(&mut parser).unwrap(),
        TimingFunction::Linear
    );
}

#[test]
fn parse_timing_cubic_bezier() {
    let mut pi = cssparser::ParserInput::new("cubic-bezier(0.1, 0.2, 0.3, 0.4)");
    let mut parser = cssparser::Parser::new(&mut pi);
    assert_eq!(
        parse_timing_function(&mut parser).unwrap(),
        TimingFunction::CubicBezier(0.1, 0.2, 0.3, 0.4)
    );
}

#[test]
fn parse_timing_cubic_bezier_out_of_range() {
    // x1 must be in [0, 1]
    let mut pi = cssparser::ParserInput::new("cubic-bezier(1.5, 0.0, 0.5, 1.0)");
    let mut parser = cssparser::Parser::new(&mut pi);
    assert!(parse_timing_function(&mut parser).is_err());
}

#[test]
fn parse_timing_steps() {
    let mut pi = cssparser::ParserInput::new("steps(4, start)");
    let mut parser = cssparser::Parser::new(&mut pi);
    assert_eq!(
        parse_timing_function(&mut parser).unwrap(),
        TimingFunction::Steps(4, StepPosition::JumpStart)
    );
}

#[test]
fn parse_timing_steps_default_end() {
    let mut pi = cssparser::ParserInput::new("steps(3)");
    let mut parser = cssparser::Parser::new(&mut pi);
    assert_eq!(
        parse_timing_function(&mut parser).unwrap(),
        TimingFunction::Steps(3, StepPosition::JumpEnd)
    );
}

#[test]
fn parse_timing_step_start() {
    let mut pi = cssparser::ParserInput::new("step-start");
    let mut parser = cssparser::Parser::new(&mut pi);
    assert_eq!(
        parse_timing_function(&mut parser).unwrap(),
        TimingFunction::Steps(1, StepPosition::JumpStart)
    );
}

#[test]
fn parse_transition_property_single() {
    let mut pi = cssparser::ParserInput::new("opacity");
    let mut parser = cssparser::Parser::new(&mut pi);
    let decls = parse_transition_property(&mut parser).unwrap();
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].value, CssValue::String("opacity".into()));
}

#[test]
fn parse_transition_property_all() {
    let mut pi = cssparser::ParserInput::new("all");
    let mut parser = cssparser::Parser::new(&mut pi);
    let decls = parse_transition_property(&mut parser).unwrap();
    assert_eq!(decls[0].value, CssValue::String("all".into()));
}

#[test]
fn parse_transition_property_multiple() {
    let mut pi = cssparser::ParserInput::new("opacity, width, color");
    let mut parser = cssparser::Parser::new(&mut pi);
    let decls = parse_transition_property(&mut parser).unwrap();
    assert_eq!(
        decls[0].value,
        CssValue::String("opacity, width, color".into())
    );
}

#[test]
fn parse_time_list_single() {
    let mut pi = cssparser::ParserInput::new("0.3s");
    let mut parser = cssparser::Parser::new(&mut pi);
    let decls = parse_time_list("transition-duration", &mut parser).unwrap();
    assert_eq!(decls[0].value, CssValue::Time(0.3));
}

#[test]
fn parse_time_list_multiple() {
    let mut pi = cssparser::ParserInput::new("0.3s, 0.5s");
    let mut parser = cssparser::Parser::new(&mut pi);
    let decls = parse_time_list("transition-duration", &mut parser).unwrap();
    match &decls[0].value {
        CssValue::List(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], CssValue::Time(0.3));
            assert_eq!(items[1], CssValue::Time(0.5));
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn parse_animation_name_single() {
    let mut pi = cssparser::ParserInput::new("fadeIn");
    let mut parser = cssparser::Parser::new(&mut pi);
    let decls = parse_animation_name(&mut parser).unwrap();
    assert_eq!(decls[0].value, CssValue::String("fadeIn".into()));
}

#[test]
fn parse_animation_name_none() {
    let mut pi = cssparser::ParserInput::new("none");
    let mut parser = cssparser::Parser::new(&mut pi);
    let decls = parse_animation_name(&mut parser).unwrap();
    assert_eq!(decls[0].value, CssValue::String("none".into()));
}

#[test]
fn parse_iteration_count_number() {
    let mut pi = cssparser::ParserInput::new("3");
    let mut parser = cssparser::Parser::new(&mut pi);
    let decls = parse_iteration_count(&mut parser).unwrap();
    assert_eq!(decls[0].value, CssValue::String("3".into()));
}

#[test]
fn parse_iteration_count_infinite() {
    let mut pi = cssparser::ParserInput::new("infinite");
    let mut parser = cssparser::Parser::new(&mut pi);
    let decls = parse_iteration_count(&mut parser).unwrap();
    assert_eq!(decls[0].value, CssValue::String("infinite".into()));
}

#[test]
fn parse_direction_keywords() {
    let mut pi = cssparser::ParserInput::new("alternate");
    let mut parser = cssparser::Parser::new(&mut pi);
    let decls = parse_animation_direction(&mut parser).unwrap();
    assert_eq!(decls[0].value, CssValue::String("alternate".into()));
}

#[test]
fn parse_fill_mode_both() {
    let mut pi = cssparser::ParserInput::new("both");
    let mut parser = cssparser::Parser::new(&mut pi);
    let decls = parse_animation_fill_mode(&mut parser).unwrap();
    assert_eq!(decls[0].value, CssValue::String("both".into()));
}

#[test]
fn parse_play_state_paused() {
    let mut pi = cssparser::ParserInput::new("paused");
    let mut parser = cssparser::Parser::new(&mut pi);
    let decls = parse_animation_play_state(&mut parser).unwrap();
    assert_eq!(decls[0].value, CssValue::String("paused".into()));
}

#[test]
fn keyframes_parse_basic() {
    let rule = parse_keyframes("fadeIn", "from { opacity: 0; } to { opacity: 1; }");
    assert_eq!(rule.name, "fadeIn");
    assert_eq!(rule.keyframes.len(), 2);
    assert_eq!(rule.keyframes[0].offset, 0.0);
    assert_eq!(rule.keyframes[1].offset, 1.0);
}

#[test]
fn keyframes_parse_percentage() {
    let rule = parse_keyframes(
        "slide",
        "0% { width: 0px; } 50% { width: 50px; } 100% { width: 100px; }",
    );
    assert_eq!(rule.keyframes.len(), 3);
    assert_eq!(rule.keyframes[0].offset, 0.0);
    assert_eq!(rule.keyframes[1].offset, 0.5);
    assert_eq!(rule.keyframes[2].offset, 1.0);
}

#[test]
fn keyframes_parse_multiple_selectors() {
    let rule = parse_keyframes("test", "0%, 100% { opacity: 1; } 50% { opacity: 0; }");
    // 0% and 100% share same block → 2 keyframes, plus 50% → 3 total
    assert_eq!(rule.keyframes.len(), 3);
}

#[test]
fn keyframes_parse_color() {
    let rule = parse_keyframes("color-anim", "from { color: red; } to { color: blue; }");
    assert_eq!(
        rule.keyframes[0].declarations[0].value,
        CssValue::Color(elidex_plugin::CssColor::RED)
    );
    assert_eq!(
        rule.keyframes[1].declarations[0].value,
        CssValue::Color(elidex_plugin::CssColor::BLUE)
    );
}

#[test]
fn transition_shorthand_basic() {
    let mut pi = cssparser::ParserInput::new("opacity 0.3s ease");
    let mut parser = cssparser::Parser::new(&mut pi);
    let decls = parse_transition_shorthand(&mut parser).unwrap();
    assert_eq!(decls.len(), 4);
    // property
    assert_eq!(decls[0].property, "transition-property");
    assert_eq!(decls[0].value, CssValue::String("opacity".into()));
    // duration
    assert_eq!(decls[1].property, "transition-duration");
    assert_eq!(decls[1].value, CssValue::Time(0.3));
}

#[test]
fn hex_color_parse() {
    let value = elidex_css::parse_raw_token_value("#ff0000");
    assert_eq!(value, CssValue::Color(elidex_plugin::CssColor::RED));
}

#[test]
fn hex_color_short() {
    let value = elidex_css::parse_raw_token_value("#f00");
    assert_eq!(
        value,
        CssValue::Color(elidex_plugin::CssColor::rgb(255, 0, 0))
    );
}

// F22: Multi-transition shorthand parse test.
//
// "opacity 0.3s, transform 0.5s ease-in" should produce two sets of
// transition longhand values, each property carrying two comma-separated
// entries.
#[test]
fn transition_shorthand_multi_value() {
    let mut pi = cssparser::ParserInput::new("opacity 0.3s, transform 0.5s ease-in");
    let mut parser = cssparser::Parser::new(&mut pi);
    let decls = parse_transition_shorthand(&mut parser).unwrap();

    // Should produce 4 longhands: property, duration, timing-function, delay
    assert_eq!(decls.len(), 4);

    // transition-property: "opacity, transform"
    assert_eq!(decls[0].property, "transition-property");
    assert_eq!(
        decls[0].value,
        CssValue::String("opacity, transform".into())
    );

    // transition-duration: list with 0.3s and 0.5s
    assert_eq!(decls[1].property, "transition-duration");
    match &decls[1].value {
        CssValue::List(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], CssValue::Time(0.3));
            assert_eq!(items[1], CssValue::Time(0.5));
        }
        other => panic!("expected List for transition-duration, got {other:?}"),
    }

    // transition-timing-function: the second entry should be ease-in.
    // ease-in is stored internally as CubicBezier(0.42, 0.0, 1.0, 1.0) and
    // serialized as "cubic-bezier(0.42, 0, 1, 1)".
    assert_eq!(decls[2].property, "transition-timing-function");
    let tf_str = match &decls[2].value {
        CssValue::String(s) => s.clone(),
        other => panic!("expected String for timing-function, got {other:?}"),
    };
    // The string contains two entries: the default "ease" for the first
    // transition and "cubic-bezier(0.42, 0, 1, 1)" (ease-in) for the second.
    assert!(
        tf_str.contains("cubic-bezier(0.42"),
        "second timing function should be the ease-in cubic-bezier: {tf_str}"
    );
}

// F23: steps(1, jump-none) should be rejected.
//
// Per CSS Easing Functions Level 2, jump-none requires at least 2 steps
// because jump-none produces (n-1) intervals and 0 intervals is invalid.
#[test]
fn steps_jump_none_count_one_rejected() {
    let mut pi = cssparser::ParserInput::new("steps(1, jump-none)");
    let mut parser = cssparser::Parser::new(&mut pi);
    assert!(
        parse_timing_function(&mut parser).is_err(),
        "steps(1, jump-none) should be rejected: n=1 with jump-none produces 0 intervals"
    );
}

// S4-1: !important in @keyframes should be stripped (CSS Animations §4.1).
#[test]
fn keyframes_important_stripped() {
    let rule = parse_keyframes("test", "from { opacity: 0 !important; } to { opacity: 1; }");
    assert_eq!(rule.keyframes.len(), 2);
    // parse_raw_token_value treats bare `0` as a zero-length (CSS unitless zero).
    assert_eq!(
        rule.keyframes[0].declarations[0].value,
        CssValue::Length(0.0, elidex_plugin::LengthUnit::Px),
        "!important should be stripped, leaving just the value"
    );
}

// S4-2: Missing from/to keyframes should be auto-generated.
#[test]
fn keyframes_auto_generate_from_to() {
    let rule = parse_keyframes("test", "50% { opacity: 0.5; }");
    assert_eq!(rule.keyframes.len(), 3, "should have from, 50%, and to");
    assert_eq!(rule.keyframes[0].offset, 0.0);
    assert!(
        rule.keyframes[0].declarations.is_empty(),
        "synthesized from should have empty declarations"
    );
    assert_eq!(rule.keyframes[1].offset, 0.5);
    assert_eq!(rule.keyframes[2].offset, 1.0);
    assert!(
        rule.keyframes[2].declarations.is_empty(),
        "synthesized to should have empty declarations"
    );
}

// S4-2: Existing from/to should not be duplicated.
#[test]
fn keyframes_no_duplicate_from_to() {
    let rule = parse_keyframes(
        "test",
        "from { opacity: 0; } 50% { opacity: 0.5; } to { opacity: 1; }",
    );
    assert_eq!(rule.keyframes.len(), 3);
    assert!(!rule.keyframes[0].declarations.is_empty());
    assert!(!rule.keyframes[2].declarations.is_empty());
}
