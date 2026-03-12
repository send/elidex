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
    style.letter_spacing = resolve_inherited_spacing(
        "letter-spacing",
        winners,
        parent_style,
        ctx,
        parent_style.letter_spacing,
    );
    style.word_spacing = resolve_inherited_spacing(
        "word-spacing",
        winners,
        parent_style,
        ctx,
        parent_style.word_spacing,
    );
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
        CssValue::Percentage(p) => {
            let result = parent_style.font_size * p / 100.0;
            if result.is_finite() {
                result
            } else {
                parent_style.font_size
            }
        }
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
/// Returns `None` for `normal`, `Some(px)` for an explicit `<length>`.
/// Inherits from parent when no declaration is present.
fn resolve_inherited_spacing(
    property: &str,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
    parent_value: Option<f32>,
) -> Option<f32> {
    match get_resolved_winner(property, winners, parent_style) {
        Some(CssValue::Length(v, unit)) => Some(resolve_length(v, unit, ctx)),
        Some(CssValue::Calc(expr)) if !expr.contains_percentage() => {
            Some(super::helpers::resolve_calc_expr(expr.as_ref(), 0.0, ctx))
        }
        Some(CssValue::Keyword(ref k)) if k == "normal" => None,
        Some(_) | None => parent_value,
    }
}

#[cfg(test)]
#[path = "font_tests.rs"]
mod tests;
