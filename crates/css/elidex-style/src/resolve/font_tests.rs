use std::collections::HashMap;

use elidex_plugin::{
    ComputedStyle, CssColor, CssValue, FontStyle, LengthUnit, LineHeight, ListStyleType, TextAlign,
    TextDecorationLine, TextDecorationStyle, TextTransform, WhiteSpace,
};

use super::*;
use crate::resolve::helpers::PropertyMap;
use crate::resolve::{build_computed_style, get_computed_as_css_value, ResolveContext};

fn default_ctx() -> ResolveContext {
    ResolveContext {
        viewport_width: 1920.0,
        viewport_height: 1080.0,
        em_base: 16.0,
        root_font_size: 16.0,
    }
}

// --- Font-size resolution ---

#[test]
fn font_size_keywords() {
    let parent = ComputedStyle::default();
    let ctx = default_ctx();
    let mut winners: PropertyMap = HashMap::new();
    let medium = CssValue::Keyword("medium".to_string());
    winners.insert("font-size", &medium);
    assert_eq!(resolve_font_size(&winners, &parent, &ctx), 16.0);
}

#[test]
fn font_size_keyword_mapping() {
    for (kw, parent_fs, expected) in [
        ("medium", 16.0, Some(16.0)),
        ("xx-small", 16.0, Some(9.0)),
        ("xx-large", 16.0, Some(32.0)),
        ("xxx-large", 16.0, Some(48.0)),
        ("unknown", 16.0, None),
    ] {
        let result = resolve_font_size_keyword(kw, parent_fs);
        match expected {
            Some(val) => assert_eq!(result, Some(val), "keyword: {kw}"),
            None => assert!(result.is_none(), "keyword: {kw} should be None"),
        }
    }
    // smaller/larger depend on parent_fs
    let smaller = resolve_font_size_keyword("smaller", 20.0).unwrap();
    assert!((smaller - 16.66).abs() < 0.1, "smaller(20.0)");
    let larger = resolve_font_size_keyword("larger", 20.0).unwrap();
    assert_eq!(larger, 24.0, "larger(20.0)");
}

#[test]
fn font_size_relative_smaller() {
    let parent = ComputedStyle {
        font_size: 24.0,
        ..ComputedStyle::default()
    };
    let ctx = default_ctx();
    let mut winners: PropertyMap = HashMap::new();
    let smaller = CssValue::Keyword("smaller".to_string());
    winners.insert("font-size", &smaller);
    let result = resolve_font_size(&winners, &parent, &ctx);
    assert!((result - 20.0).abs() < 0.1);
}

#[test]
fn font_size_xxx_large() {
    let parent = ComputedStyle::default();
    let ctx = default_ctx();
    let mut winners: PropertyMap = HashMap::new();
    let xxx_large = CssValue::Keyword("xxx-large".to_string());
    winners.insert("font-size", &xxx_large);
    assert_eq!(resolve_font_size(&winners, &parent, &ctx), 48.0);
}

#[test]
fn font_size_em_uses_parent() {
    let parent = ComputedStyle {
        font_size: 20.0,
        ..ComputedStyle::default()
    };
    let ctx = default_ctx();
    let mut winners: PropertyMap = HashMap::new();
    let val = CssValue::Length(2.0, LengthUnit::Em);
    winners.insert("font-size", &val);
    let fs = resolve_font_size(&winners, &parent, &ctx);
    assert_eq!(fs, 40.0);
}

#[test]
fn font_size_percentage() {
    let parent = ComputedStyle {
        font_size: 20.0,
        ..ComputedStyle::default()
    };
    let ctx = default_ctx();
    let mut winners: PropertyMap = HashMap::new();
    let val = CssValue::Percentage(150.0);
    winners.insert("font-size", &val);
    let fs = resolve_font_size(&winners, &parent, &ctx);
    assert_eq!(fs, 30.0);
}

// --- Font-weight resolution ---

#[test]
fn font_weight_bolder() {
    for (input, expected) in [
        (100, 400),
        (300, 400),
        (400, 700),
        (500, 700),
        (700, 900),
        (900, 900),
    ] {
        assert_eq!(resolve_bolder(input), expected, "bolder({input})");
    }
}

#[test]
fn font_weight_lighter() {
    for (input, expected) in [(50, 50), (100, 100), (400, 100), (700, 400), (900, 700)] {
        assert_eq!(resolve_lighter(input), expected, "lighter({input})");
    }
}

#[test]
fn font_weight_numeric_resolved() {
    let parent = ComputedStyle::default();
    let weight = CssValue::Number(700.0);
    let mut winners: PropertyMap = HashMap::new();
    winners.insert("font-weight", &weight);
    let ctx = default_ctx();
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.font_weight, 700);
}

#[test]
fn font_weight_nan_inherits_from_parent() {
    let parent = ComputedStyle {
        font_weight: 600,
        ..ComputedStyle::default()
    };
    let weight = CssValue::Number(f32::NAN);
    let mut winners: PropertyMap = HashMap::new();
    winners.insert("font-weight", &weight);
    let ctx = default_ctx();
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.font_weight, 600);
}

#[test]
fn font_weight_infinity_inherits_from_parent() {
    let parent = ComputedStyle {
        font_weight: 300,
        ..ComputedStyle::default()
    };
    let weight = CssValue::Number(f32::INFINITY);
    let mut winners: PropertyMap = HashMap::new();
    winners.insert("font-weight", &weight);
    let ctx = default_ctx();
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.font_weight, 300);
}

// --- Inherited keyword enum resolution ---

#[test]
fn inherited_keyword_enum_resolve() {
    let ctx = default_ctx();
    let parent = ComputedStyle::default();
    for (prop, keyword, check_fn) in [
        (
            "text-align",
            "center",
            (|s: &ComputedStyle| s.text_align == TextAlign::Center) as fn(&ComputedStyle) -> bool,
        ),
        ("white-space", "pre", |s: &ComputedStyle| {
            s.white_space == WhiteSpace::Pre
        }),
        ("list-style-type", "decimal", |s: &ComputedStyle| {
            s.list_style_type == ListStyleType::Decimal
        }),
        ("font-style", "italic", |s: &ComputedStyle| {
            s.font_style == FontStyle::Italic
        }),
        ("text-transform", "uppercase", |s: &ComputedStyle| {
            s.text_transform == TextTransform::Uppercase
        }),
    ] {
        let val = CssValue::Keyword(keyword.into());
        let mut winners: PropertyMap = HashMap::new();
        winners.insert(prop, &val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!(check_fn(&style), "{prop}: {keyword} not resolved correctly");
    }
}

#[test]
fn inherited_keyword_enum_inherits() {
    let ctx = default_ctx();
    for (check_fn, parent_mod) in [
        (
            (|s: &ComputedStyle| s.text_align == TextAlign::Center) as fn(&ComputedStyle) -> bool,
            ComputedStyle {
                text_align: TextAlign::Center,
                ..ComputedStyle::default()
            },
        ),
        (
            |s: &ComputedStyle| s.text_transform == TextTransform::Uppercase,
            ComputedStyle {
                text_transform: TextTransform::Uppercase,
                ..ComputedStyle::default()
            },
        ),
        (
            |s: &ComputedStyle| s.white_space == WhiteSpace::NoWrap,
            ComputedStyle {
                white_space: WhiteSpace::NoWrap,
                ..ComputedStyle::default()
            },
        ),
        (
            |s: &ComputedStyle| s.list_style_type == ListStyleType::Square,
            ComputedStyle {
                list_style_type: ListStyleType::Square,
                ..ComputedStyle::default()
            },
        ),
        (
            |s: &ComputedStyle| s.font_style == FontStyle::Oblique,
            ComputedStyle {
                font_style: FontStyle::Oblique,
                ..ComputedStyle::default()
            },
        ),
    ] {
        let winners: PropertyMap = HashMap::new();
        let style = build_computed_style(&winners, &parent_mod, &ctx);
        assert!(check_fn(&style), "inheritance failed for {parent_mod:?}");
    }
}

#[test]
fn inherited_keyword_enum_computed_value() {
    for (prop, style, expected_kw) in [
        (
            "text-align",
            ComputedStyle {
                text_align: TextAlign::Center,
                ..ComputedStyle::default()
            },
            "center",
        ),
        (
            "white-space",
            ComputedStyle {
                white_space: WhiteSpace::PreWrap,
                ..ComputedStyle::default()
            },
            "pre-wrap",
        ),
        (
            "list-style-type",
            ComputedStyle {
                list_style_type: ListStyleType::Circle,
                ..ComputedStyle::default()
            },
            "circle",
        ),
        (
            "font-style",
            ComputedStyle {
                font_style: FontStyle::Italic,
                ..ComputedStyle::default()
            },
            "italic",
        ),
        (
            "text-transform",
            ComputedStyle {
                text_transform: TextTransform::Capitalize,
                ..ComputedStyle::default()
            },
            "capitalize",
        ),
    ] {
        assert_eq!(
            get_computed_as_css_value(prop, &style),
            CssValue::Keyword(expected_kw.to_string()),
            "{prop}"
        );
    }
}

// --- Color + currentcolor ---

#[test]
fn build_style_inherits_color() {
    let parent = ComputedStyle {
        color: CssColor::RED,
        ..ComputedStyle::default()
    };
    let winners: PropertyMap = HashMap::new();
    let ctx = default_ctx();
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.color, CssColor::RED);
}

#[test]
fn currentcolor_resolution() {
    let parent = ComputedStyle::default();
    let ctx = default_ctx();
    let mut winners: PropertyMap = HashMap::new();
    let red = CssValue::Color(CssColor::RED);
    winners.insert("color", &red);
    let style = build_computed_style(&winners, &parent, &ctx);
    // border-*-color initial = currentcolor → should be RED
    assert_eq!(style.border_top_color, CssColor::RED);
}

// --- Text properties ---

#[test]
fn text_decoration_line_none_default() {
    let parent = ComputedStyle {
        text_decoration_line: TextDecorationLine {
            underline: true,
            ..TextDecorationLine::default()
        },
        ..ComputedStyle::default()
    };
    let winners: PropertyMap = HashMap::new();
    let ctx = default_ctx();
    let style = build_computed_style(&winners, &parent, &ctx);
    assert!(!style.text_decoration_line.underline);
}

#[test]
fn text_decoration_line_inherits_when_inherit_keyword() {
    let parent = ComputedStyle {
        text_decoration_line: TextDecorationLine {
            underline: true,
            ..TextDecorationLine::default()
        },
        ..ComputedStyle::default()
    };
    let inherit = CssValue::Inherit;
    let mut winners: PropertyMap = HashMap::new();
    winners.insert("text-decoration-line", &inherit);
    let ctx = default_ctx();
    let style = build_computed_style(&winners, &parent, &ctx);
    assert!(style.text_decoration_line.underline);
}

#[test]
fn line_height_number_inherits() {
    let parent = ComputedStyle {
        line_height: LineHeight::Number(1.5),
        ..ComputedStyle::default()
    };
    let winners: PropertyMap = HashMap::new();
    let ctx = default_ctx();
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.line_height, LineHeight::Number(1.5));
}

#[test]
fn parse_font_family_from_raw_mixed_quotes() {
    let result =
        parse_font_family_from_raw("'SFMono-Regular', Consolas, \"Liberation Mono\", monospace");
    assert_eq!(
        result,
        vec![
            "SFMono-Regular".to_string(),
            "Consolas".to_string(),
            "Liberation Mono".to_string(),
            "monospace".to_string(),
        ]
    );
}

#[test]
fn parse_font_family_from_raw_empty() {
    let result = parse_font_family_from_raw("");
    assert!(result.is_empty());
}

// --- M4-1: letter-spacing resolution ---

#[test]
fn letter_spacing_length_resolved() {
    let parent = ComputedStyle::default();
    let ctx = default_ctx();
    let val = CssValue::Length(2.0, LengthUnit::Px);
    let mut winners: PropertyMap = HashMap::new();
    winners.insert("letter-spacing", &val);
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.letter_spacing, Some(2.0));
}

#[test]
fn letter_spacing_normal_is_none() {
    let parent = ComputedStyle::default();
    let ctx = default_ctx();
    let val = CssValue::Keyword("normal".to_string());
    let mut winners: PropertyMap = HashMap::new();
    winners.insert("letter-spacing", &val);
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.letter_spacing, None);
}

#[test]
fn letter_spacing_inherits() {
    let parent = ComputedStyle {
        letter_spacing: Some(3.0),
        ..ComputedStyle::default()
    };
    let winners: PropertyMap = HashMap::new();
    let ctx = default_ctx();
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.letter_spacing, Some(3.0));
}

#[test]
fn letter_spacing_computed_value() {
    let style = ComputedStyle {
        letter_spacing: Some(1.5),
        ..ComputedStyle::default()
    };
    assert_eq!(
        get_computed_as_css_value("letter-spacing", &style),
        CssValue::Length(1.5, LengthUnit::Px)
    );
}

#[test]
fn letter_spacing_normal_computed_value() {
    let style = ComputedStyle::default();
    assert_eq!(
        get_computed_as_css_value("letter-spacing", &style),
        CssValue::Keyword("normal".to_string())
    );
}

#[test]
fn letter_spacing_zero_px_computed_value() {
    let style = ComputedStyle {
        letter_spacing: Some(0.0),
        ..ComputedStyle::default()
    };
    assert_eq!(
        get_computed_as_css_value("letter-spacing", &style),
        CssValue::Length(0.0, LengthUnit::Px)
    );
}

// --- M4-1: word-spacing resolution ---

#[test]
fn word_spacing_length_resolved() {
    let parent = ComputedStyle::default();
    let ctx = default_ctx();
    let val = CssValue::Length(5.0, LengthUnit::Px);
    let mut winners: PropertyMap = HashMap::new();
    winners.insert("word-spacing", &val);
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.word_spacing, Some(5.0));
}

#[test]
fn word_spacing_inherits() {
    let parent = ComputedStyle {
        word_spacing: Some(4.0),
        ..ComputedStyle::default()
    };
    let winners: PropertyMap = HashMap::new();
    let ctx = default_ctx();
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.word_spacing, Some(4.0));
}

#[test]
fn word_spacing_computed_value() {
    let style = ComputedStyle {
        word_spacing: Some(2.0),
        ..ComputedStyle::default()
    };
    assert_eq!(
        get_computed_as_css_value("word-spacing", &style),
        CssValue::Length(2.0, LengthUnit::Px)
    );
}

#[test]
fn letter_spacing_calc_resolved() {
    use elidex_plugin::CalcExpr;
    let parent = ComputedStyle::default();
    let ctx = default_ctx();
    let val = CssValue::Calc(Box::new(CalcExpr::Add(
        Box::new(CalcExpr::Length(2.0, LengthUnit::Px)),
        Box::new(CalcExpr::Length(3.0, LengthUnit::Px)),
    )));
    let mut winners: PropertyMap = HashMap::new();
    winners.insert("letter-spacing", &val);
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.letter_spacing, Some(5.0));
}

// --- M4-1: text-decoration-style resolution ---

#[test]
fn text_decoration_style_resolved() {
    let parent = ComputedStyle::default();
    let ctx = default_ctx();
    let val = CssValue::Keyword("dashed".to_string());
    let mut winners: PropertyMap = HashMap::new();
    winners.insert("text-decoration-style", &val);
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.text_decoration_style, TextDecorationStyle::Dashed);
}

#[test]
fn text_decoration_style_not_inherited() {
    let parent = ComputedStyle {
        text_decoration_style: TextDecorationStyle::Dotted,
        ..ComputedStyle::default()
    };
    let winners: PropertyMap = HashMap::new();
    let ctx = default_ctx();
    let style = build_computed_style(&winners, &parent, &ctx);
    // text-decoration-style is NOT inherited → defaults to Solid.
    assert_eq!(style.text_decoration_style, TextDecorationStyle::Solid);
}

#[test]
fn text_decoration_style_computed_value() {
    let style = ComputedStyle {
        text_decoration_style: TextDecorationStyle::Double,
        ..ComputedStyle::default()
    };
    assert_eq!(
        get_computed_as_css_value("text-decoration-style", &style),
        CssValue::Keyword("double".to_string())
    );
}

// --- M4-1: text-decoration-color resolution ---

#[test]
fn text_decoration_color_explicit() {
    let parent = ComputedStyle::default();
    let ctx = default_ctx();
    let val = CssValue::Color(CssColor::RED);
    let mut winners: PropertyMap = HashMap::new();
    winners.insert("text-decoration-color", &val);
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.text_decoration_color, Some(CssColor::RED));
}

#[test]
fn text_decoration_color_currentcolor_is_none() {
    let parent = ComputedStyle::default();
    let ctx = default_ctx();
    let val = CssValue::Keyword("currentcolor".to_string());
    let mut winners: PropertyMap = HashMap::new();
    winners.insert("text-decoration-color", &val);
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.text_decoration_color, None);
}

#[test]
fn text_decoration_color_not_inherited() {
    let parent = ComputedStyle {
        text_decoration_color: Some(CssColor::BLUE),
        ..ComputedStyle::default()
    };
    let winners: PropertyMap = HashMap::new();
    let ctx = default_ctx();
    let style = build_computed_style(&winners, &parent, &ctx);
    // text-decoration-color is NOT inherited → defaults to None.
    assert_eq!(style.text_decoration_color, None);
}

// --- M4-1: overline in text-decoration-line ---

#[test]
fn text_decoration_line_overline_resolved() {
    let parent = ComputedStyle::default();
    let ctx = default_ctx();
    let val = CssValue::Keyword("overline".to_string());
    let mut winners: PropertyMap = HashMap::new();
    winners.insert("text-decoration-line", &val);
    let style = build_computed_style(&winners, &parent, &ctx);
    assert!(style.text_decoration_line.overline);
    assert!(!style.text_decoration_line.underline);
    assert!(!style.text_decoration_line.line_through);
}
