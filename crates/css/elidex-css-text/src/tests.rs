use super::*;

fn parse_prop(name: &str, input_str: &str) -> Vec<PropertyDeclaration> {
    let handler = TextHandler;
    let mut pi = cssparser::ParserInput::new(input_str);
    let mut parser = cssparser::Parser::new(&mut pi);
    handler.parse(name, &mut parser).unwrap()
}

fn parse_value(name: &str, input_str: &str) -> CssValue {
    parse_prop(name, input_str)[0].value.clone()
}

#[test]
fn property_names_count() {
    assert_eq!(TextHandler.property_names().len(), 19);
}

#[test]
fn parse_color_named() {
    let val = parse_value("color", "red");
    assert_eq!(val, CssValue::Color(CssColor::new(255, 0, 0, 255)));
}

#[test]
fn parse_color_currentcolor() {
    let val = parse_value("text-decoration-color", "currentcolor");
    assert_eq!(val, CssValue::Keyword("currentcolor".to_string()));
}

#[test]
fn parse_font_size_keyword() {
    let val = parse_value("font-size", "xx-large");
    assert_eq!(val, CssValue::Keyword("xx-large".to_string()));
}

#[test]
fn parse_font_size_length() {
    let val = parse_value("font-size", "24px");
    assert_eq!(val, CssValue::Length(24.0, LengthUnit::Px));
}

#[test]
fn parse_font_size_relative() {
    let val = parse_value("font-size", "smaller");
    assert_eq!(val, CssValue::Keyword("smaller".to_string()));
}

#[test]
fn parse_font_weight_number() {
    let val = parse_value("font-weight", "600");
    assert_eq!(val, CssValue::Number(600.0));
}

#[test]
fn parse_font_weight_keyword() {
    let val = parse_value("font-weight", "bold");
    assert_eq!(val, CssValue::Keyword("bold".to_string()));
}

#[test]
fn parse_font_family_quoted_and_generic() {
    let val = parse_value("font-family", "'Courier New', monospace");
    match val {
        CssValue::List(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], CssValue::String("Courier New".to_string()));
            assert_eq!(items[1], CssValue::Keyword("monospace".to_string()));
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn parse_font_family_multi_word_unquoted() {
    let val = parse_value("font-family", "Times New Roman, serif");
    match val {
        CssValue::List(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], CssValue::Keyword("Times New Roman".to_string()));
            assert_eq!(items[1], CssValue::Keyword("serif".to_string()));
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn parse_line_height_normal() {
    let val = parse_value("line-height", "normal");
    assert_eq!(val, CssValue::Keyword("normal".to_string()));
}

#[test]
fn parse_line_height_number() {
    let val = parse_value("line-height", "1.5");
    assert_eq!(val, CssValue::Number(1.5));
}

#[test]
fn parse_line_height_px() {
    let val = parse_value("line-height", "24px");
    assert_eq!(val, CssValue::Length(24.0, LengthUnit::Px));
}

#[test]
fn parse_text_align_justify() {
    let val = parse_value("text-align", "justify");
    assert_eq!(val, CssValue::Keyword("justify".to_string()));
}

#[test]
fn parse_text_decoration_line_multiple() {
    let val = parse_value("text-decoration-line", "underline overline");
    match val {
        CssValue::List(items) => {
            assert_eq!(items.len(), 2);
            assert!(items.contains(&CssValue::Keyword("underline".to_string())));
            assert!(items.contains(&CssValue::Keyword("overline".to_string())));
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn parse_text_decoration_line_none() {
    let val = parse_value("text-decoration-line", "none");
    assert_eq!(val, CssValue::Keyword("none".to_string()));
}

#[test]
fn parse_letter_spacing_normal() {
    let val = parse_value("letter-spacing", "normal");
    assert_eq!(val, CssValue::Keyword("normal".to_string()));
}

#[test]
fn parse_letter_spacing_length() {
    let val = parse_value("letter-spacing", "2px");
    assert_eq!(val, CssValue::Length(2.0, LengthUnit::Px));
}

#[test]
fn parse_writing_mode() {
    let val = parse_value("writing-mode", "vertical-rl");
    assert_eq!(val, CssValue::Keyword("vertical-rl".to_string()));
}

#[test]
fn parse_unicode_bidi() {
    let val = parse_value("unicode-bidi", "isolate-override");
    assert_eq!(val, CssValue::Keyword("isolate-override".to_string()));
}

#[test]
fn resolve_color_to_style() {
    let handler = TextHandler;
    let ctx = ResolveContext::default();
    let mut style = ComputedStyle::default();
    let red = CssColor::new(255, 0, 0, 255);
    handler.resolve("color", &CssValue::Color(red), &ctx, &mut style);
    assert_eq!(style.color, red);
}

#[test]
fn resolve_font_size_keyword_medium() {
    let handler = TextHandler;
    let ctx = ResolveContext {
        em_base: 16.0,
        ..ResolveContext::default()
    };
    let mut style = ComputedStyle::default();
    handler.resolve(
        "font-size",
        &CssValue::Keyword("large".to_string()),
        &ctx,
        &mut style,
    );
    assert_eq!(style.font_size, 18.0);
}

#[test]
fn resolve_font_size_em() {
    let handler = TextHandler;
    let ctx = ResolveContext {
        em_base: 20.0,
        ..ResolveContext::default()
    };
    let mut style = ComputedStyle::default();
    handler.resolve(
        "font-size",
        &CssValue::Length(2.0, LengthUnit::Em),
        &ctx,
        &mut style,
    );
    assert_eq!(style.font_size, 40.0);
}

#[test]
fn resolve_font_weight_bolder() {
    let handler = TextHandler;
    let ctx = ResolveContext::default();
    let mut style = ComputedStyle {
        font_weight: 400,
        ..ComputedStyle::default()
    };
    handler.resolve(
        "font-weight",
        &CssValue::Keyword("bolder".to_string()),
        &ctx,
        &mut style,
    );
    assert_eq!(style.font_weight, 700);
}

#[test]
fn resolve_font_weight_lighter() {
    let handler = TextHandler;
    let ctx = ResolveContext::default();
    let mut style = ComputedStyle {
        font_weight: 700,
        ..ComputedStyle::default()
    };
    handler.resolve(
        "font-weight",
        &CssValue::Keyword("lighter".to_string()),
        &ctx,
        &mut style,
    );
    assert_eq!(style.font_weight, 400);
}

#[test]
fn resolve_text_decoration_line_combined() {
    let handler = TextHandler;
    let ctx = ResolveContext::default();
    let mut style = ComputedStyle::default();
    handler.resolve(
        "text-decoration-line",
        &CssValue::List(vec![
            CssValue::Keyword("underline".to_string()),
            CssValue::Keyword("line-through".to_string()),
        ]),
        &ctx,
        &mut style,
    );
    assert!(style.text_decoration_line.underline);
    assert!(style.text_decoration_line.line_through);
    assert!(!style.text_decoration_line.overline);
}

#[test]
fn resolve_line_height_number() {
    let handler = TextHandler;
    let ctx = ResolveContext::default();
    let mut style = ComputedStyle::default();
    handler.resolve("line-height", &CssValue::Number(1.5), &ctx, &mut style);
    assert_eq!(style.line_height, LineHeight::Number(1.5));
}

#[test]
fn resolve_letter_spacing_value() {
    let handler = TextHandler;
    let ctx = ResolveContext::default();
    let mut style = ComputedStyle::default();
    handler.resolve(
        "letter-spacing",
        &CssValue::Length(3.0, LengthUnit::Px),
        &ctx,
        &mut style,
    );
    assert_eq!(style.letter_spacing, Some(3.0));
}

#[test]
fn resolve_letter_spacing_normal() {
    let handler = TextHandler;
    let ctx = ResolveContext::default();
    let mut style = ComputedStyle {
        letter_spacing: Some(5.0),
        ..ComputedStyle::default()
    };
    handler.resolve(
        "letter-spacing",
        &CssValue::Keyword("normal".to_string()),
        &ctx,
        &mut style,
    );
    assert_eq!(style.letter_spacing, None);
}

#[test]
fn inheritance_flags() {
    let handler = TextHandler;
    assert!(handler.is_inherited("color"));
    assert!(handler.is_inherited("font-size"));
    assert!(handler.is_inherited("font-weight"));
    assert!(handler.is_inherited("font-style"));
    assert!(handler.is_inherited("font-family"));
    assert!(handler.is_inherited("line-height"));
    assert!(handler.is_inherited("text-align"));
    assert!(handler.is_inherited("text-transform"));
    assert!(handler.is_inherited("white-space"));
    assert!(handler.is_inherited("list-style-type"));
    assert!(handler.is_inherited("writing-mode"));
    assert!(handler.is_inherited("text-orientation"));
    assert!(handler.is_inherited("direction"));
    assert!(handler.is_inherited("letter-spacing"));
    assert!(handler.is_inherited("word-spacing"));
    // Non-inherited.
    assert!(!handler.is_inherited("text-decoration-line"));
    assert!(!handler.is_inherited("text-decoration-style"));
    assert!(!handler.is_inherited("text-decoration-color"));
    assert!(!handler.is_inherited("unicode-bidi"));
}

#[test]
fn get_computed_roundtrip() {
    let handler = TextHandler;
    let style = ComputedStyle {
        text_align: TextAlign::Justify,
        ..ComputedStyle::default()
    };
    let val = handler.get_computed("text-align", &style);
    assert_eq!(val, CssValue::Keyword("justify".to_string()));
}

#[test]
fn initial_values() {
    let handler = TextHandler;
    assert_eq!(
        handler.initial_value("font-size"),
        CssValue::Length(16.0, LengthUnit::Px)
    );
    assert_eq!(
        handler.initial_value("font-weight"),
        CssValue::Number(400.0)
    );
    assert_eq!(
        handler.initial_value("text-decoration-line"),
        CssValue::Keyword("none".to_string())
    );
}

#[test]
fn affects_layout_flags() {
    let handler = TextHandler;
    assert!(!handler.affects_layout("color"));
    assert!(!handler.affects_layout("text-decoration-color"));
    assert!(handler.affects_layout("font-size"));
    assert!(handler.affects_layout("letter-spacing"));
}

#[test]
fn resolve_direction_rtl() {
    let handler = TextHandler;
    let ctx = ResolveContext::default();
    let mut style = ComputedStyle::default();
    handler.resolve(
        "direction",
        &CssValue::Keyword("rtl".to_string()),
        &ctx,
        &mut style,
    );
    assert_eq!(style.direction, Direction::Rtl);
}

#[test]
fn resolve_writing_mode_vertical() {
    let handler = TextHandler;
    let ctx = ResolveContext::default();
    let mut style = ComputedStyle::default();
    handler.resolve(
        "writing-mode",
        &CssValue::Keyword("vertical-lr".to_string()),
        &ctx,
        &mut style,
    );
    assert_eq!(style.writing_mode, WritingMode::VerticalLr);
}

#[test]
fn resolve_unicode_bidi() {
    let handler = TextHandler;
    let ctx = ResolveContext::default();
    let mut style = ComputedStyle::default();
    handler.resolve(
        "unicode-bidi",
        &CssValue::Keyword("isolate".to_string()),
        &ctx,
        &mut style,
    );
    assert_eq!(style.unicode_bidi, UnicodeBidi::Isolate);
}
