//! Font, text, and inherited keyword-enum property resolution.

use elidex_plugin::{
    ComputedStyle, CssColor, CssValue, FontStyle, LineHeight, ListStyleType, TextAlign,
    TextDecorationLine, TextDecorationStyle, TextTransform, WhiteSpace,
};

use super::helpers::{
    get_resolved_winner, resolve_inherited_keyword_enum, resolve_length, PropertyMap,
};
use super::ResolveContext;

/// CSS absolute font-size keyword values in pixels (CSS Fonts Level 4).
const FONT_SIZE_KEYWORD_PX: [(&str, f32); 9] = [
    ("xx-small", 9.0),
    ("x-small", 10.0),
    ("small", 13.0),
    ("medium", 16.0),
    ("large", 18.0),
    ("x-large", 24.0),
    ("xx-large", 32.0),
    ("xxx-large", 48.0),
    ("smaller", 0.0), // handled by separate match arm above; kept for table completeness
];

/// Scale factor for the `smaller` relative font-size keyword (~5/6).
const SMALLER_FACTOR: f32 = 5.0 / 6.0;
/// Scale factor for the `larger` relative font-size keyword (~6/5).
const LARGER_FACTOR: f32 = 6.0 / 5.0;

/// Resolve font-size keywords to pixel values.
pub(super) fn resolve_font_size_keyword(keyword: &str, parent_font_size: f32) -> Option<f32> {
    match keyword {
        "smaller" => Some(parent_font_size * SMALLER_FACTOR),
        "larger" => Some(parent_font_size * LARGER_FACTOR),
        _ => FONT_SIZE_KEYWORD_PX
            .iter()
            .find(|(k, _)| *k == keyword)
            .map(|(_, v)| *v),
    }
}

/// Resolve font, text, and inherited keyword-enum properties.
pub(super) fn resolve_font_and_text_properties(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    resolve_font_weight(style, winners, parent_style);
    style.font_style = resolve_inherited_keyword_enum(
        "font-style",
        winners,
        parent_style,
        parent_style.font_style,
        FontStyle::from_keyword,
    );
    resolve_font_family(style, winners, parent_style);
    resolve_line_height(style, winners, parent_style, ctx);
    style.text_transform = resolve_inherited_keyword_enum(
        "text-transform",
        winners,
        parent_style,
        parent_style.text_transform,
        TextTransform::from_keyword,
    );
    style.text_align = resolve_inherited_keyword_enum(
        "text-align",
        winners,
        parent_style,
        parent_style.text_align,
        TextAlign::from_keyword,
    );
    style.white_space = resolve_inherited_keyword_enum(
        "white-space",
        winners,
        parent_style,
        parent_style.white_space,
        WhiteSpace::from_keyword,
    );
    style.list_style_type = resolve_inherited_keyword_enum(
        "list-style-type",
        winners,
        parent_style,
        parent_style.list_style_type,
        ListStyleType::from_keyword,
    );
    // text-decoration-line is non-inherited.
    resolve_text_decoration_line(style, winners, parent_style);
    resolve_text_decoration_style(style, winners, parent_style);
    resolve_text_decoration_color(style, winners, parent_style);
    style.letter_spacing =
        resolve_inherited_spacing("letter-spacing", winners, parent_style, ctx, parent_style.letter_spacing);
    style.word_spacing =
        resolve_inherited_spacing("word-spacing", winners, parent_style, ctx, parent_style.word_spacing);
}

pub(super) fn resolve_font_size(
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) -> f32 {
    match get_resolved_winner("font-size", winners, parent_style) {
        Some(value) => resolve_font_size_value(&value, parent_style, ctx),
        None => parent_style.font_size,
    }
}

fn resolve_font_size_value(
    value: &CssValue,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) -> f32 {
    match value {
        CssValue::Length(v, unit) => {
            // For font-size, em is relative to parent, not self.
            resolve_length(*v, *unit, &ctx.with_em_base(parent_style.font_size))
        }
        CssValue::Percentage(p) => parent_style.font_size * p / 100.0,
        CssValue::Keyword(kw) => {
            resolve_font_size_keyword(kw, parent_style.font_size).unwrap_or(parent_style.font_size)
        }
        _ => parent_style.font_size,
    }
}

pub(super) fn resolve_color(winners: &PropertyMap<'_>, parent_style: &ComputedStyle) -> CssColor {
    match get_resolved_winner("color", winners, parent_style) {
        Some(CssValue::Color(c)) => c,
        Some(_) | None => parent_style.color,
    }
}

pub(super) fn resolve_background_color(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    match get_resolved_winner("background-color", winners, parent_style) {
        Some(CssValue::Color(c)) => style.background_color = c,
        Some(CssValue::Keyword(ref k)) if k.eq_ignore_ascii_case("currentcolor") => {
            style.background_color = style.color;
        }
        _ => {}
    }
}

fn resolve_font_family(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    match get_resolved_winner("font-family", winners, parent_style) {
        Some(CssValue::List(ref items)) => {
            style.font_family = extract_font_family_names(items);
        }
        Some(CssValue::RawTokens(ref raw) | CssValue::String(ref raw)) => {
            style.font_family = parse_font_family_from_raw(raw);
        }
        Some(CssValue::Keyword(ref k)) => {
            // A single keyword (e.g. from var() resolving to a generic family).
            style.font_family = vec![k.clone()];
        }
        _ => {
            style.font_family.clone_from(&parent_style.font_family);
        }
    }
}

/// Extract font family names from a parsed `CssValue::List`.
fn extract_font_family_names(items: &[CssValue]) -> Vec<String> {
    items
        .iter()
        .filter_map(|v| match v {
            CssValue::String(s) => Some(s.clone()),
            CssValue::Keyword(k) => Some(k.clone()),
            _ => None,
        })
        .collect()
}

/// Parse a comma-separated font-family string (from `var()` resolution)
/// into a list of family names.
///
/// Handles quoted names (`'SFMono-Regular'`, `"Courier New"`), unquoted
/// multi-word names (`Times New Roman`), and generic families (`monospace`).
pub(super) fn parse_font_family_from_raw(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| {
            let trimmed = s.trim();
            // Strip matching outer quotes (single or double).
            if (trimmed.starts_with('\'') && trimmed.ends_with('\''))
                || (trimmed.starts_with('"') && trimmed.ends_with('"'))
            {
                trimmed[1..trimmed.len() - 1].to_string()
            } else {
                trimmed.to_string()
            }
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn resolve_font_weight(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    style.font_weight = match get_resolved_winner("font-weight", winners, parent_style) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Some(CssValue::Number(n)) if n.is_finite() => n.round().clamp(1.0, 1000.0) as u16,
        Some(CssValue::Keyword(ref k)) => match k.as_str() {
            "normal" => 400,
            "bold" => 700,
            "bolder" => resolve_bolder(parent_style.font_weight),
            "lighter" => resolve_lighter(parent_style.font_weight),
            _ => parent_style.font_weight,
        },
        _ => parent_style.font_weight,
    };
}

/// Resolve `font-weight: bolder` per CSS Fonts Level 4 §2.2.
///
/// Maps the inherited weight to the next bolder weight:
/// - 1–349 → 400
/// - 350–549 → 700
/// - 550–899 → 900
/// - 900–1000 → 900 (clamped)
pub(super) fn resolve_bolder(parent: u16) -> u16 {
    match parent {
        1..=349 => 400,
        350..=549 => 700,
        _ => 900,
    }
}

/// Resolve `font-weight: lighter` per CSS Fonts Level 4 §2.2.
///
/// Maps the inherited weight to the next lighter weight:
/// - 1–99 → unchanged (already below minimum break point)
/// - 100–549 → 100
/// - 550–749 → 400
/// - 750–1000 → 700
pub(super) fn resolve_lighter(parent: u16) -> u16 {
    match parent {
        0..=99 => parent,
        100..=549 => 100,
        550..=749 => 400,
        _ => 700,
    }
}

fn resolve_line_height(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    style.line_height = match get_resolved_winner("line-height", winners, parent_style) {
        Some(CssValue::Keyword(ref k)) if k == "normal" => LineHeight::Normal,
        // Unitless number: inherited as-is, recomputed per element's font-size.
        Some(CssValue::Number(n)) => LineHeight::Number(n),
        // Absolute length: resolve to px.
        Some(CssValue::Length(v, unit)) => LineHeight::Px(resolve_length(v, unit, ctx)),
        // Percentage: resolve to px (relative to element's font-size).
        Some(CssValue::Percentage(p)) => LineHeight::Px(style.font_size * p / 100.0),
        _ => parent_style.line_height,
    };
}

fn resolve_text_decoration_line(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    let Some(value) = get_resolved_winner("text-decoration-line", winners, parent_style) else {
        // Non-inherited: keep default (none). Don't inherit from parent.
        return;
    };
    style.text_decoration_line = match &value {
        CssValue::Keyword(k) => keyword_to_decoration_line(k),
        CssValue::List(items) => {
            let mut result = TextDecorationLine::default();
            for item in items {
                if let CssValue::Keyword(k) = item {
                    match k.as_str() {
                        "underline" => result.underline = true,
                        "overline" => result.overline = true,
                        "line-through" => result.line_through = true,
                        _ => {}
                    }
                }
            }
            result
        }
        _ => TextDecorationLine::default(),
    };
}

fn keyword_to_decoration_line(k: &str) -> TextDecorationLine {
    match k {
        "underline" => TextDecorationLine {
            underline: true,
            ..TextDecorationLine::default()
        },
        "overline" => TextDecorationLine {
            overline: true,
            ..TextDecorationLine::default()
        },
        "line-through" => TextDecorationLine {
            line_through: true,
            ..TextDecorationLine::default()
        },
        _ => TextDecorationLine::default(),
    }
}

fn resolve_text_decoration_style(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    if let Some(CssValue::Keyword(ref k)) =
        get_resolved_winner("text-decoration-style", winners, parent_style)
    {
        if let Some(s) = TextDecorationStyle::from_keyword(k) {
            style.text_decoration_style = s;
        }
    }
}

fn resolve_text_decoration_color(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    if let Some(value) = get_resolved_winner("text-decoration-color", winners, parent_style) {
        match &value {
            CssValue::Color(c) => style.text_decoration_color = Some(*c),
            CssValue::Keyword(ref k) if k.eq_ignore_ascii_case("currentcolor") => {
                // None means use currentcolor (resolved at render time).
                style.text_decoration_color = None;
            }
            _ => {}
        }
    }
}

/// Resolve an inherited spacing property (`letter-spacing` or `word-spacing`).
///
/// Both properties accept `normal` (= 0.0) or a `<length>`, and inherit from parent.
fn resolve_inherited_spacing(
    property: &str,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
    parent_value: f32,
) -> f32 {
    match get_resolved_winner(property, winners, parent_style) {
        Some(CssValue::Length(v, unit)) => resolve_length(v, unit, ctx),
        Some(CssValue::Keyword(ref k)) if k == "normal" => 0.0,
        Some(_) | None => parent_value,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use elidex_plugin::{
        ComputedStyle, CssColor, CssValue, FontStyle, LengthUnit, LineHeight, ListStyleType,
        TextAlign, TextDecorationLine, TextDecorationStyle, TextTransform, WhiteSpace,
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
                (|s: &ComputedStyle| s.text_align == TextAlign::Center)
                    as fn(&ComputedStyle) -> bool,
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
                (|s: &ComputedStyle| s.text_align == TextAlign::Center)
                    as fn(&ComputedStyle) -> bool,
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
        let result = parse_font_family_from_raw(
            "'SFMono-Regular', Consolas, \"Liberation Mono\", monospace",
        );
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
        assert!((style.letter_spacing - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn letter_spacing_normal_is_zero() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let val = CssValue::Keyword("normal".to_string());
        let mut winners: PropertyMap = HashMap::new();
        winners.insert("letter-spacing", &val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!(style.letter_spacing.abs() < f32::EPSILON);
    }

    #[test]
    fn letter_spacing_inherits() {
        let parent = ComputedStyle {
            letter_spacing: 3.0,
            ..ComputedStyle::default()
        };
        let winners: PropertyMap = HashMap::new();
        let ctx = default_ctx();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!((style.letter_spacing - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn letter_spacing_computed_value() {
        let style = ComputedStyle {
            letter_spacing: 1.5,
            ..ComputedStyle::default()
        };
        assert_eq!(
            get_computed_as_css_value("letter-spacing", &style),
            CssValue::Length(1.5, LengthUnit::Px)
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
        assert!((style.word_spacing - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn word_spacing_inherits() {
        let parent = ComputedStyle {
            word_spacing: 4.0,
            ..ComputedStyle::default()
        };
        let winners: PropertyMap = HashMap::new();
        let ctx = default_ctx();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!((style.word_spacing - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn word_spacing_computed_value() {
        let style = ComputedStyle {
            word_spacing: 2.0,
            ..ComputedStyle::default()
        };
        assert_eq!(
            get_computed_as_css_value("word-spacing", &style),
            CssValue::Length(2.0, LengthUnit::Px)
        );
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
}
