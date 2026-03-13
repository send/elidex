//! CSS text and font property handler plugin (font-*, text-*, line-height,
//! white-space, color, writing-mode, direction, unicode-bidi, list-style-type).

use elidex_plugin::{
    css_resolve::{keyword_from, parse_length_unit, resolve_length},
    parse_css_keyword as parse_keyword, ComputedStyle, CssColor, CssPropertyHandler, CssValue,
    Direction, FontStyle, LengthUnit, LineHeight, ListStyleType, ParseError, PropertyDeclaration,
    ResolveContext, TextAlign, TextDecorationLine, TextDecorationStyle, TextOrientation,
    TextTransform, UnicodeBidi, WhiteSpace, WritingMode,
};

/// All property names handled by this handler.
const PROPERTY_NAMES: &[&str] = &[
    "color",
    "font-size",
    "font-weight",
    "font-style",
    "font-family",
    "line-height",
    "text-align",
    "text-transform",
    "white-space",
    "letter-spacing",
    "word-spacing",
    "text-decoration-line",
    "text-decoration-style",
    "text-decoration-color",
    "writing-mode",
    "text-orientation",
    "direction",
    "unicode-bidi",
    "list-style-type",
];

/// CSS text and font property handler.
#[derive(Clone)]
pub struct TextHandler;

impl TextHandler {
    /// Register this handler in a CSS property registry.
    pub fn register(registry: &mut elidex_plugin::CssPropertyRegistry) {
        elidex_plugin::register_css_handler(registry, Self);
    }
}

impl CssPropertyHandler for TextHandler {
    fn property_names(&self) -> &[&str] {
        PROPERTY_NAMES
    }

    fn parse(
        &self,
        name: &str,
        input: &mut cssparser::Parser<'_, '_>,
    ) -> Result<Vec<PropertyDeclaration>, ParseError> {
        let value = match name {
            "font-size" => parse_font_size(input)?,
            "font-weight" => parse_font_weight(input)?,
            "font-style" => parse_keyword(input, &["normal", "italic", "oblique"])?,
            "font-family" => parse_font_family(input)?,
            "line-height" => parse_line_height(input)?,
            "text-align" => parse_keyword(
                input,
                &["left", "right", "center", "justify", "start", "end"],
            )?,
            "text-transform" => {
                parse_keyword(input, &["none", "uppercase", "lowercase", "capitalize"])?
            }
            "white-space" => {
                parse_keyword(input, &["normal", "pre", "nowrap", "pre-wrap", "pre-line"])?
            }
            "letter-spacing" | "word-spacing" => parse_spacing(input)?,
            "text-decoration-line" => parse_text_decoration_line(input)?,
            "text-decoration-style" => {
                parse_keyword(input, &["solid", "double", "dotted", "dashed", "wavy"])?
            }
            "color" | "text-decoration-color" => parse_css_color(input)?,
            "writing-mode" => {
                parse_keyword(input, &["horizontal-tb", "vertical-rl", "vertical-lr"])?
            }
            "text-orientation" => parse_keyword(input, &["mixed", "upright", "sideways"])?,
            "direction" => parse_keyword(input, &["ltr", "rtl"])?,
            "unicode-bidi" => parse_keyword(
                input,
                &[
                    "normal",
                    "embed",
                    "bidi-override",
                    "isolate",
                    "isolate-override",
                    "plaintext",
                ],
            )?,
            "list-style-type" => {
                parse_keyword(input, &["disc", "circle", "square", "decimal", "none"])?
            }
            _ => return Ok(vec![]),
        };
        Ok(vec![PropertyDeclaration::new(name, value)])
    }

    fn resolve(
        &self,
        name: &str,
        value: &CssValue,
        ctx: &ResolveContext,
        style: &mut ComputedStyle,
    ) {
        match name {
            "color" => {
                if let CssValue::Color(c) = value {
                    style.color = *c;
                }
            }
            "font-size" => {
                style.font_size = resolve_font_size(value, style, ctx);
            }
            "font-weight" => {
                resolve_font_weight_value(value, style);
            }
            "font-style" => {
                if let CssValue::Keyword(ref k) = value {
                    style.font_style = FontStyle::from_keyword(k).unwrap_or_default();
                }
            }
            "font-family" => {
                resolve_font_family(value, style);
            }
            "line-height" => {
                resolve_line_height(value, style, ctx);
            }
            "text-align" => {
                if let CssValue::Keyword(ref k) = value {
                    style.text_align = TextAlign::from_keyword(k).unwrap_or_default();
                }
            }
            "text-transform" => {
                if let CssValue::Keyword(ref k) = value {
                    style.text_transform = TextTransform::from_keyword(k).unwrap_or_default();
                }
            }
            "white-space" => {
                if let CssValue::Keyword(ref k) = value {
                    style.white_space = WhiteSpace::from_keyword(k).unwrap_or_default();
                }
            }
            "letter-spacing" => {
                style.letter_spacing = resolve_spacing(value, ctx);
            }
            "word-spacing" => {
                style.word_spacing = resolve_spacing(value, ctx);
            }
            "text-decoration-line" => {
                resolve_text_decoration_line(value, style);
            }
            "text-decoration-style" => {
                if let CssValue::Keyword(ref k) = value {
                    style.text_decoration_style =
                        TextDecorationStyle::from_keyword(k).unwrap_or_default();
                }
            }
            "text-decoration-color" => match value {
                CssValue::Color(c) => style.text_decoration_color = Some(*c),
                CssValue::Keyword(ref k) if k.eq_ignore_ascii_case("currentcolor") => {
                    style.text_decoration_color = None;
                }
                _ => {}
            },
            "writing-mode" => {
                if let CssValue::Keyword(ref k) = value {
                    style.writing_mode = WritingMode::from_keyword(k).unwrap_or_default();
                }
            }
            "text-orientation" => {
                if let CssValue::Keyword(ref k) = value {
                    style.text_orientation = TextOrientation::from_keyword(k).unwrap_or_default();
                }
            }
            "direction" => {
                if let CssValue::Keyword(ref k) = value {
                    style.direction = Direction::from_keyword(k).unwrap_or_default();
                }
            }
            "unicode-bidi" => {
                if let CssValue::Keyword(ref k) = value {
                    style.unicode_bidi = UnicodeBidi::from_keyword(k).unwrap_or_default();
                }
            }
            "list-style-type" => {
                if let CssValue::Keyword(ref k) = value {
                    style.list_style_type = ListStyleType::from_keyword(k).unwrap_or_default();
                }
            }
            _ => {}
        }
    }

    fn initial_value(&self, name: &str) -> CssValue {
        match name {
            "color" => CssValue::Color(CssColor::BLACK),
            "font-size" => CssValue::Length(16.0, LengthUnit::Px),
            "font-weight" => CssValue::Number(400.0),
            "font-style" | "line-height" | "white-space" | "letter-spacing" | "word-spacing"
            | "unicode-bidi" => CssValue::Keyword("normal".to_string()),
            "font-family" => CssValue::List(vec![CssValue::Keyword("serif".to_string())]),
            "text-align" => CssValue::Keyword("start".to_string()),
            "text-transform" | "text-decoration-line" => CssValue::Keyword("none".to_string()),
            "text-decoration-style" => CssValue::Keyword("solid".to_string()),
            "text-decoration-color" => CssValue::Keyword("currentcolor".to_string()),
            "writing-mode" => CssValue::Keyword("horizontal-tb".to_string()),
            "text-orientation" => CssValue::Keyword("mixed".to_string()),
            "direction" => CssValue::Keyword("ltr".to_string()),
            "list-style-type" => CssValue::Keyword("disc".to_string()),
            _ => CssValue::Initial,
        }
    }

    fn is_inherited(&self, name: &str) -> bool {
        matches!(
            name,
            "color"
                | "font-size"
                | "font-weight"
                | "font-style"
                | "font-family"
                | "line-height"
                | "text-transform"
                | "text-align"
                | "white-space"
                | "list-style-type"
                | "writing-mode"
                | "text-orientation"
                | "direction"
                | "letter-spacing"
                | "word-spacing"
        )
    }

    fn affects_layout(&self, name: &str) -> bool {
        // All text/font properties can affect layout except color and
        // text-decoration-color.
        !matches!(name, "color" | "text-decoration-color")
    }

    fn get_computed(&self, name: &str, style: &ComputedStyle) -> CssValue {
        match name {
            "color" => CssValue::Color(style.color),
            "font-size" => CssValue::Length(style.font_size, LengthUnit::Px),
            "font-weight" => CssValue::Number(f32::from(style.font_weight)),
            "font-style" => keyword_from(&style.font_style),
            "font-family" => CssValue::List(
                style
                    .font_family
                    .iter()
                    .map(|s| CssValue::Keyword(s.clone()))
                    .collect(),
            ),
            "line-height" => match style.line_height {
                LineHeight::Normal => CssValue::Keyword("normal".to_string()),
                LineHeight::Number(n) => CssValue::Number(n),
                LineHeight::Px(px) => CssValue::Length(px, LengthUnit::Px),
            },
            "text-align" => keyword_from(&style.text_align),
            "text-transform" => keyword_from(&style.text_transform),
            "white-space" => keyword_from(&style.white_space),
            "letter-spacing" => match style.letter_spacing {
                None => CssValue::Keyword("normal".to_string()),
                Some(px) => CssValue::Length(px, LengthUnit::Px),
            },
            "word-spacing" => match style.word_spacing {
                None => CssValue::Keyword("normal".to_string()),
                Some(px) => CssValue::Length(px, LengthUnit::Px),
            },
            "text-decoration-line" => {
                let d = &style.text_decoration_line;
                let mut parts = Vec::new();
                if d.underline {
                    parts.push(CssValue::Keyword("underline".to_string()));
                }
                if d.overline {
                    parts.push(CssValue::Keyword("overline".to_string()));
                }
                if d.line_through {
                    parts.push(CssValue::Keyword("line-through".to_string()));
                }
                if parts.len() > 1 {
                    CssValue::List(parts)
                } else {
                    CssValue::Keyword(d.to_string())
                }
            }
            "text-decoration-style" => keyword_from(&style.text_decoration_style),
            "text-decoration-color" => match style.text_decoration_color {
                Some(c) => CssValue::Color(c),
                None => CssValue::Keyword("currentcolor".to_string()),
            },
            "writing-mode" => keyword_from(&style.writing_mode),
            "text-orientation" => keyword_from(&style.text_orientation),
            "direction" => keyword_from(&style.direction),
            "unicode-bidi" => keyword_from(&style.unicode_bidi),
            "list-style-type" => keyword_from(&style.list_style_type),
            _ => CssValue::Initial,
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

fn parse_css_color(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    elidex_css::parse_color_with_currentcolor(input)
}

/// CSS absolute font-size keyword values in pixels (CSS Fonts Level 4).
const FONT_SIZE_KEYWORDS: &[(&str, f32)] = &[
    ("xx-small", 9.0),
    ("x-small", 10.0),
    ("small", 13.0),
    ("medium", 16.0),
    ("large", 18.0),
    ("x-large", 24.0),
    ("xx-large", 32.0),
    ("xxx-large", 48.0),
];

fn parse_font_size(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    // Try keyword first.
    if let Ok(ident) = input.try_parse(|i| i.expect_ident().map(|s| s.to_ascii_lowercase())) {
        // Check absolute keywords.
        for (kw, _) in FONT_SIZE_KEYWORDS {
            if ident == *kw {
                return Ok(CssValue::Keyword(ident));
            }
        }
        // Relative keywords.
        if ident == "smaller" || ident == "larger" {
            return Ok(CssValue::Keyword(ident));
        }
        return Err(ParseError {
            property: "font-size".into(),
            input: ident,
            message: "unknown font-size keyword".into(),
        });
    }

    // Try length/percentage/number.
    parse_length_percentage_number(input, "font-size")
}

fn parse_font_weight(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    // Try number first.
    if let Ok(n) = input.try_parse(|i| {
        let token = i.next().map_err(|_| ())?;
        match *token {
            cssparser::Token::Number { value, .. } => Ok(value),
            _ => Err(()),
        }
    }) {
        return Ok(CssValue::Number(n.clamp(1.0, 1000.0)));
    }

    // Then keywords.
    parse_keyword(input, &["normal", "bold", "bolder", "lighter"])
}

fn parse_font_family(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    let mut families = Vec::new();
    loop {
        // Try quoted string.
        if let Ok(s) = input.try_parse(|i| i.expect_string().map(std::string::ToString::to_string))
        {
            families.push(CssValue::String(s));
        } else {
            // Unquoted: collect consecutive idents as a single family name.
            // Limit word count to prevent unbounded accumulation.
            const MAX_FAMILY_WORDS: usize = 32;
            let mut name_parts = Vec::new();
            while let Ok(ident) =
                input.try_parse(|i| i.expect_ident().map(std::string::ToString::to_string))
            {
                name_parts.push(ident);
                if name_parts.len() >= MAX_FAMILY_WORDS {
                    break;
                }
            }
            if name_parts.is_empty() {
                if families.is_empty() {
                    return Err(ParseError::simple("expected font family name"));
                }
                break;
            }
            let name = name_parts.join(" ");
            families.push(CssValue::Keyword(name));
        }
        // Expect comma or end.
        if input.try_parse(cssparser::Parser::expect_comma).is_err() {
            break;
        }
    }
    Ok(CssValue::List(families))
}

fn parse_line_height(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    // Try "normal" keyword.
    if input
        .try_parse(|i| i.expect_ident_matching("normal"))
        .is_ok()
    {
        return Ok(CssValue::Keyword("normal".to_string()));
    }

    // Try number/length/percentage.
    let token = input
        .next()
        .map_err(|_| ParseError::simple("expected line-height value"))?;
    match *token {
        cssparser::Token::Number { value, .. } => Ok(CssValue::Number(value)),
        cssparser::Token::Dimension {
            value, ref unit, ..
        } => {
            let unit = parse_length_unit(unit);
            Ok(CssValue::Length(value, unit))
        }
        cssparser::Token::Percentage { unit_value, .. } => {
            Ok(CssValue::Percentage(unit_value * 100.0))
        }
        _ => Err(ParseError::simple("expected line-height value")),
    }
}

fn parse_spacing(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    // Try "normal" keyword.
    if input
        .try_parse(|i| i.expect_ident_matching("normal"))
        .is_ok()
    {
        return Ok(CssValue::Keyword("normal".to_string()));
    }

    // Try length.
    let token = input
        .next()
        .map_err(|_| ParseError::simple("expected spacing value"))?;
    match *token {
        cssparser::Token::Dimension {
            value, ref unit, ..
        } => {
            let unit = parse_length_unit(unit);
            Ok(CssValue::Length(value, unit))
        }
        cssparser::Token::Number { value: 0.0, .. } => Ok(CssValue::Length(0.0, LengthUnit::Px)),
        _ => Err(ParseError::simple("expected length or 'normal'")),
    }
}

fn parse_text_decoration_line(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    // Try "none" first (must be the only keyword).
    if input.try_parse(|i| i.expect_ident_matching("none")).is_ok() {
        return Ok(CssValue::Keyword("none".to_string()));
    }

    // Collect one or more keywords.
    let allowed = ["underline", "overline", "line-through"];
    let mut values = Vec::new();
    loop {
        let ok = input.try_parse(|i| {
            let ident = i.expect_ident().map_err(|_| ())?;
            let lower = ident.to_ascii_lowercase();
            if allowed.contains(&lower.as_str()) {
                Ok(CssValue::Keyword(lower))
            } else {
                Err(())
            }
        });
        match ok {
            Ok(v) => values.push(v),
            Err(()) => break,
        }
    }

    if values.is_empty() {
        return Err(ParseError::simple("expected text-decoration-line keyword"));
    }
    if values.len() == 1 {
        Ok(values.into_iter().next().unwrap())
    } else {
        Ok(CssValue::List(values))
    }
}

fn parse_length_percentage_number(
    input: &mut cssparser::Parser<'_, '_>,
    property: &str,
) -> Result<CssValue, ParseError> {
    let token = input.next().map_err(|_| ParseError {
        property: property.into(),
        input: String::new(),
        message: "expected value".into(),
    })?;
    match *token {
        cssparser::Token::Dimension {
            value, ref unit, ..
        } => {
            let unit = parse_length_unit(unit);
            Ok(CssValue::Length(value, unit))
        }
        cssparser::Token::Percentage { unit_value, .. } => {
            Ok(CssValue::Percentage(unit_value * 100.0))
        }
        cssparser::Token::Number { value: 0.0, .. } => Ok(CssValue::Length(0.0, LengthUnit::Px)),
        _ => Err(ParseError {
            property: property.into(),
            input: String::new(),
            message: "expected length, percentage, or number".into(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Resolution helpers
// ---------------------------------------------------------------------------

/// Scale factor for the `smaller` relative font-size keyword (~5/6).
const SMALLER_FACTOR: f32 = 5.0 / 6.0;
/// Scale factor for the `larger` relative font-size keyword (~6/5).
const LARGER_FACTOR: f32 = 6.0 / 5.0;

fn resolve_font_size(value: &CssValue, style: &ComputedStyle, ctx: &ResolveContext) -> f32 {
    match value {
        CssValue::Length(v, unit) => {
            // For font-size, em is relative to parent (ctx.em_base already set
            // to parent font-size by the caller in elidex-style).
            resolve_length(*v, *unit, ctx)
        }
        CssValue::Percentage(p) => {
            let result = ctx.em_base * p / 100.0;
            if result.is_finite() {
                result
            } else {
                style.font_size
            }
        }
        CssValue::Keyword(kw) => {
            match kw.as_str() {
                "smaller" => ctx.em_base * SMALLER_FACTOR,
                "larger" => ctx.em_base * LARGER_FACTOR,
                _ => {
                    // Absolute keyword lookup.
                    FONT_SIZE_KEYWORDS
                        .iter()
                        .find(|(k, _)| *k == kw.as_str())
                        .map_or(style.font_size, |(_, v)| *v)
                }
            }
        }
        _ => style.font_size,
    }
}

fn resolve_font_weight_value(value: &CssValue, style: &mut ComputedStyle) {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    match value {
        CssValue::Number(n) if n.is_finite() => {
            style.font_weight = n.round().clamp(1.0, 1000.0) as u16;
        }
        CssValue::Keyword(ref k) => {
            style.font_weight = match k.as_str() {
                "normal" => 400,
                "bold" => 700,
                "bolder" => resolve_bolder(style.font_weight),
                "lighter" => resolve_lighter(style.font_weight),
                _ => style.font_weight,
            };
        }
        _ => {}
    }
}

/// Resolve `font-weight: bolder` per CSS Fonts Level 4 section 2.2.
fn resolve_bolder(parent: u16) -> u16 {
    match parent {
        1..=349 => 400,
        350..=549 => 700,
        _ => 900,
    }
}

/// Resolve `font-weight: lighter` per CSS Fonts Level 4 section 2.2.
fn resolve_lighter(parent: u16) -> u16 {
    match parent {
        0..=99 => parent,
        100..=549 => 100,
        550..=749 => 400,
        _ => 700,
    }
}

fn resolve_font_family(value: &CssValue, style: &mut ComputedStyle) {
    match value {
        CssValue::List(ref items) => {
            let names: Vec<String> = items
                .iter()
                .filter_map(|v| match v {
                    CssValue::String(s) => Some(s.clone()),
                    CssValue::Keyword(k) => Some(k.clone()),
                    _ => None,
                })
                .collect();
            if !names.is_empty() {
                style.font_family = names;
            }
        }
        CssValue::String(ref s) => {
            style.font_family = vec![s.clone()];
        }
        CssValue::Keyword(ref k) => {
            style.font_family = vec![k.clone()];
        }
        _ => {}
    }
}

fn resolve_line_height(value: &CssValue, style: &mut ComputedStyle, ctx: &ResolveContext) {
    style.line_height = match value {
        CssValue::Keyword(ref k) if k == "normal" => LineHeight::Normal,
        CssValue::Number(n) => LineHeight::Number(*n),
        CssValue::Length(v, unit) => LineHeight::Px(resolve_length(*v, *unit, ctx)),
        CssValue::Percentage(p) => LineHeight::Px(style.font_size * p / 100.0),
        _ => style.line_height,
    };
}

fn resolve_spacing(value: &CssValue, ctx: &ResolveContext) -> Option<f32> {
    match value {
        CssValue::Keyword(ref k) if k == "normal" => None,
        CssValue::Length(v, unit) => Some(resolve_length(*v, *unit, ctx)),
        _ => None,
    }
}

fn resolve_text_decoration_line(value: &CssValue, style: &mut ComputedStyle) {
    style.text_decoration_line = match value {
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

#[cfg(test)]
mod tests;
