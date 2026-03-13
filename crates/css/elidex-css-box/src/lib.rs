//! CSS box model property handler plugin (display, position, margin, padding,
//! border, opacity, box-sizing, overflow, background-color, content, gap).

use elidex_plugin::{
    css_resolve::{
        keyword_from, parse_non_negative_length_or_percentage, resolve_dimension, resolve_to_px,
    },
    parse_css_keyword as parse_keyword, BorderStyle, BoxSizing, ComputedStyle, ContentItem,
    ContentValue, CssColor, CssPropertyHandler, CssValue, Dimension, Display, LengthUnit, Overflow,
    ParseError, Position, PropertyDeclaration, ResolveContext,
};

/// CSS box model property handler.
#[derive(Clone)]
pub struct BoxHandler;

impl BoxHandler {
    /// Register this handler in a CSS property registry.
    pub fn register(registry: &mut elidex_plugin::CssPropertyRegistry) {
        elidex_plugin::register_css_handler(registry, Self);
    }
}

/// All display keywords accepted by the parser.
const DISPLAY_KEYWORDS: &[&str] = &[
    "block",
    "inline",
    "inline-block",
    "none",
    "flex",
    "inline-flex",
    "list-item",
    "grid",
    "inline-grid",
    "table",
    "inline-table",
    "table-caption",
    "table-row",
    "table-cell",
    "table-row-group",
    "table-header-group",
    "table-footer-group",
    "table-column",
    "table-column-group",
    "contents",
];

const POSITION_KEYWORDS: &[&str] = &["static", "relative", "absolute", "fixed", "sticky"];

const BORDER_STYLE_KEYWORDS: &[&str] = &[
    "none", "hidden", "solid", "dashed", "dotted", "double", "groove", "ridge", "inset", "outset",
];

const OVERFLOW_KEYWORDS: &[&str] = &["visible", "hidden", "scroll", "auto", "clip"];

const BOX_SIZING_KEYWORDS: &[&str] = &["content-box", "border-box"];

/// All property names handled by [`BoxHandler`].
const BOX_PROPERTIES: &[&str] = &[
    "display",
    "position",
    "width",
    "height",
    "min-width",
    "min-height",
    "max-width",
    "max-height",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
    "border-top-style",
    "border-right-style",
    "border-bottom-style",
    "border-left-style",
    "border-top-color",
    "border-right-color",
    "border-bottom-color",
    "border-left-color",
    "box-sizing",
    "border-radius",
    "opacity",
    "overflow",
    "background-color",
    "content",
    "row-gap",
    "column-gap",
];

impl CssPropertyHandler for BoxHandler {
    fn property_names(&self) -> &[&str] {
        BOX_PROPERTIES
    }

    fn parse(
        &self,
        name: &str,
        input: &mut cssparser::Parser<'_, '_>,
    ) -> Result<Vec<PropertyDeclaration>, ParseError> {
        let value = match name {
            "display" => parse_keyword(input, DISPLAY_KEYWORDS)?,
            "position" => parse_keyword(input, POSITION_KEYWORDS)?,
            "box-sizing" => parse_keyword(input, BOX_SIZING_KEYWORDS)?,
            "overflow" => parse_keyword(input, OVERFLOW_KEYWORDS)?,

            "border-top-style"
            | "border-right-style"
            | "border-bottom-style"
            | "border-left-style" => parse_keyword(input, BORDER_STYLE_KEYWORDS)?,

            "width" | "height" | "max-width" | "max-height" => {
                parse_length_percentage_auto_or_none(input)?
            }

            "min-width" | "min-height" => parse_length_percentage_auto(input)?,

            "border-radius" | "row-gap" | "column-gap" | "padding-top" | "padding-right"
            | "padding-bottom" | "padding-left" => parse_non_negative_length_percentage(input)?,

            "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => {
                parse_length_percentage_auto(input)?
            }

            "border-top-width"
            | "border-right-width"
            | "border-bottom-width"
            | "border-left-width" => parse_border_width(input)?,

            "border-top-color"
            | "border-right-color"
            | "border-bottom-color"
            | "border-left-color"
            | "background-color" => parse_color_value(input)?,

            "opacity" => parse_opacity(input)?,
            "content" => parse_content(input)?,

            _ => return Ok(vec![]),
        };
        Ok(vec![PropertyDeclaration::new(name, value)])
    }

    #[allow(clippy::too_many_lines)]
    fn resolve(
        &self,
        name: &str,
        value: &CssValue,
        ctx: &ResolveContext,
        style: &mut ComputedStyle,
    ) {
        match name {
            "display" => {
                if let CssValue::Keyword(ref k) = value {
                    style.display = Display::from_keyword(k).unwrap_or_default();
                }
            }
            "position" => {
                if let CssValue::Keyword(ref k) = value {
                    style.position = Position::from_keyword(k).unwrap_or_default();
                }
            }
            "box-sizing" => {
                if let CssValue::Keyword(ref k) = value {
                    style.box_sizing = BoxSizing::from_keyword(k).unwrap_or_default();
                }
            }
            "overflow" => {
                if let CssValue::Keyword(ref k) = value {
                    // scroll, auto, clip all map to Hidden
                    style.overflow = match k.as_str() {
                        "visible" => Overflow::Visible,
                        _ => Overflow::Hidden,
                    };
                }
            }

            "width" => style.width = resolve_dimension(value, ctx),
            "height" => style.height = resolve_dimension(value, ctx),
            "min-width" => style.min_width = resolve_dimension(value, ctx),
            "min-height" => style.min_height = resolve_dimension(value, ctx),
            "max-width" => style.max_width = resolve_max_dimension(value, ctx),
            "max-height" => style.max_height = resolve_max_dimension(value, ctx),

            "margin-top" => style.margin_top = resolve_dimension(value, ctx),
            "margin-right" => style.margin_right = resolve_dimension(value, ctx),
            "margin-bottom" => style.margin_bottom = resolve_dimension(value, ctx),
            "margin-left" => style.margin_left = resolve_dimension(value, ctx),

            "padding-top" => style.padding_top = resolve_to_px(value, ctx).max(0.0),
            "padding-right" => style.padding_right = resolve_to_px(value, ctx).max(0.0),
            "padding-bottom" => style.padding_bottom = resolve_to_px(value, ctx).max(0.0),
            "padding-left" => style.padding_left = resolve_to_px(value, ctx).max(0.0),

            "border-top-width" => style.border_top_width = resolve_to_px(value, ctx).max(0.0),
            "border-right-width" => style.border_right_width = resolve_to_px(value, ctx).max(0.0),
            "border-bottom-width" => {
                style.border_bottom_width = resolve_to_px(value, ctx).max(0.0);
            }
            "border-left-width" => style.border_left_width = resolve_to_px(value, ctx).max(0.0),

            "border-top-style" => {
                resolve_border_style_and_zero_width(
                    value,
                    &mut style.border_top_style,
                    &mut style.border_top_width,
                );
            }
            "border-right-style" => {
                resolve_border_style_and_zero_width(
                    value,
                    &mut style.border_right_style,
                    &mut style.border_right_width,
                );
            }
            "border-bottom-style" => {
                resolve_border_style_and_zero_width(
                    value,
                    &mut style.border_bottom_style,
                    &mut style.border_bottom_width,
                );
            }
            "border-left-style" => {
                resolve_border_style_and_zero_width(
                    value,
                    &mut style.border_left_style,
                    &mut style.border_left_width,
                );
            }

            "border-top-color" => style.border_top_color = resolve_color(value, style.color),
            "border-right-color" => style.border_right_color = resolve_color(value, style.color),
            "border-bottom-color" => style.border_bottom_color = resolve_color(value, style.color),
            "border-left-color" => style.border_left_color = resolve_color(value, style.color),

            "border-radius" => style.border_radius = resolve_to_px(value, ctx).max(0.0),

            "opacity" => {
                if let CssValue::Number(n) = value {
                    style.opacity = n.clamp(0.0, 1.0);
                }
            }

            "background-color" => style.background_color = resolve_color(value, style.color),
            "content" => resolve_content(value, &mut style.content),
            "row-gap" => style.row_gap = resolve_to_px(value, ctx).max(0.0),
            "column-gap" => style.column_gap = resolve_to_px(value, ctx).max(0.0),

            _ => {}
        }
    }

    fn initial_value(&self, name: &str) -> CssValue {
        match name {
            "display" => CssValue::Keyword("inline".to_string()),
            "position" => CssValue::Keyword("static".to_string()),

            "width" | "height" | "max-width" | "max-height" | "min-width" | "min-height" => {
                CssValue::Auto
            }

            "margin-top" | "margin-right" | "margin-bottom" | "margin-left" | "padding-top"
            | "padding-right" | "padding-bottom" | "padding-left" | "border-radius" | "row-gap"
            | "column-gap" => CssValue::Length(0.0, LengthUnit::Px),

            "border-top-width"
            | "border-right-width"
            | "border-bottom-width"
            | "border-left-width" => CssValue::Length(3.0, LengthUnit::Px),

            "border-top-style"
            | "border-right-style"
            | "border-bottom-style"
            | "border-left-style" => CssValue::Keyword("none".to_string()),

            "border-top-color"
            | "border-right-color"
            | "border-bottom-color"
            | "border-left-color" => CssValue::Keyword("currentcolor".to_string()),

            "box-sizing" => CssValue::Keyword("content-box".to_string()),
            "opacity" => CssValue::Number(1.0),
            "overflow" => CssValue::Keyword("visible".to_string()),
            "background-color" => CssValue::Color(CssColor::TRANSPARENT),
            "content" => CssValue::Keyword("normal".to_string()),

            _ => CssValue::Initial,
        }
    }

    fn is_inherited(&self, _name: &str) -> bool {
        false
    }

    fn affects_layout(&self, name: &str) -> bool {
        !matches!(
            name,
            "opacity"
                | "background-color"
                | "border-top-color"
                | "border-right-color"
                | "border-bottom-color"
                | "border-left-color"
        )
    }

    fn get_computed(&self, name: &str, style: &ComputedStyle) -> CssValue {
        match name {
            "display" => keyword_from(&style.display),
            "position" => keyword_from(&style.position),
            "box-sizing" => keyword_from(&style.box_sizing),
            "overflow" => keyword_from(&style.overflow),

            "width" => dimension_to_css_value(style.width),
            "height" => dimension_to_css_value(style.height),
            "min-width" => dimension_to_css_value(style.min_width),
            "min-height" => dimension_to_css_value(style.min_height),
            "max-width" => match style.max_width {
                Dimension::Auto => CssValue::Keyword("none".to_string()),
                other => dimension_to_css_value(other),
            },
            "max-height" => match style.max_height {
                Dimension::Auto => CssValue::Keyword("none".to_string()),
                other => dimension_to_css_value(other),
            },

            "margin-top" => dimension_to_css_value(style.margin_top),
            "margin-right" => dimension_to_css_value(style.margin_right),
            "margin-bottom" => dimension_to_css_value(style.margin_bottom),
            "margin-left" => dimension_to_css_value(style.margin_left),

            "padding-top" => CssValue::Length(style.padding_top, LengthUnit::Px),
            "padding-right" => CssValue::Length(style.padding_right, LengthUnit::Px),
            "padding-bottom" => CssValue::Length(style.padding_bottom, LengthUnit::Px),
            "padding-left" => CssValue::Length(style.padding_left, LengthUnit::Px),

            "border-top-width" => CssValue::Length(style.border_top_width, LengthUnit::Px),
            "border-right-width" => CssValue::Length(style.border_right_width, LengthUnit::Px),
            "border-bottom-width" => CssValue::Length(style.border_bottom_width, LengthUnit::Px),
            "border-left-width" => CssValue::Length(style.border_left_width, LengthUnit::Px),

            "border-top-style" => keyword_from(&style.border_top_style),
            "border-right-style" => keyword_from(&style.border_right_style),
            "border-bottom-style" => keyword_from(&style.border_bottom_style),
            "border-left-style" => keyword_from(&style.border_left_style),

            "border-top-color" => CssValue::Color(style.border_top_color),
            "border-right-color" => CssValue::Color(style.border_right_color),
            "border-bottom-color" => CssValue::Color(style.border_bottom_color),
            "border-left-color" => CssValue::Color(style.border_left_color),

            "border-radius" => CssValue::Length(style.border_radius, LengthUnit::Px),
            "opacity" => CssValue::Number(style.opacity),

            "background-color" => CssValue::Color(style.background_color),

            "content" => match &style.content {
                ContentValue::Normal => CssValue::Keyword("normal".to_string()),
                ContentValue::None => CssValue::Keyword("none".to_string()),
                ContentValue::Items(items) => {
                    let parts: Vec<CssValue> = items
                        .iter()
                        .map(|item| match item {
                            ContentItem::String(s) => CssValue::String(s.clone()),
                            ContentItem::Attr(a) => CssValue::Keyword(format!("attr:{a}")),
                        })
                        .collect();
                    if parts.len() == 1 {
                        parts.into_iter().next().unwrap_or(CssValue::Initial)
                    } else {
                        CssValue::List(parts)
                    }
                }
            },

            "row-gap" => CssValue::Length(style.row_gap, LengthUnit::Px),
            "column-gap" => CssValue::Length(style.column_gap, LengthUnit::Px),

            _ => CssValue::Initial,
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Parse a length, percentage, or `auto` keyword.
fn parse_length_percentage_auto(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    // Try `auto` keyword first.
    if input.try_parse(|i| i.expect_ident_matching("auto")).is_ok() {
        return Ok(CssValue::Auto);
    }
    parse_length_percentage(input)
}

/// Parse a length, percentage, `auto`, or `none` (for max-width/max-height).
fn parse_length_percentage_auto_or_none(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    if input.try_parse(|i| i.expect_ident_matching("auto")).is_ok() {
        return Ok(CssValue::Auto);
    }
    if input.try_parse(|i| i.expect_ident_matching("none")).is_ok() {
        return Ok(CssValue::Auto); // `none` maps to Auto (unconstrained)
    }
    parse_length_percentage(input)
}

/// Parse a length or percentage value.
fn parse_length_percentage(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    elidex_plugin::css_resolve::parse_length_or_percentage(input)
}

/// Parse a non-negative length or percentage value.
fn parse_non_negative_length_percentage(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    parse_non_negative_length_or_percentage(input)
}

/// Parse a border width value: length or keyword (thin/medium/thick).
fn parse_border_width(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    // Try keywords first
    if let Ok(ident) = input.try_parse(|i| i.expect_ident().map(|s| s.to_ascii_lowercase())) {
        return match ident.as_str() {
            "thin" => Ok(CssValue::Length(1.0, LengthUnit::Px)),
            "medium" => Ok(CssValue::Length(3.0, LengthUnit::Px)),
            "thick" => Ok(CssValue::Length(5.0, LengthUnit::Px)),
            _ => Err(ParseError {
                property: String::new(),
                input: ident,
                message: "unexpected border-width keyword".into(),
            }),
        };
    }
    parse_non_negative_length_percentage(input)
}

/// Parse a CSS color value, including the `currentcolor` keyword.
fn parse_color_value(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    elidex_css::parse_color_with_currentcolor(input)
}

/// Parse opacity: a number clamped to 0.0..=1.0.
fn parse_opacity(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    let token = input.next().map_err(|_| ParseError {
        property: String::new(),
        input: String::new(),
        message: "expected number".into(),
    })?;
    match *token {
        cssparser::Token::Number { value, .. } => Ok(CssValue::Number(value.clamp(0.0, 1.0))),
        cssparser::Token::Percentage { unit_value, .. } => {
            // CSS allows percentage for opacity
            Ok(CssValue::Number(unit_value.clamp(0.0, 1.0)))
        }
        _ => Err(ParseError {
            property: String::new(),
            input: String::new(),
            message: "expected number for opacity".into(),
        }),
    }
}

/// Parse the CSS `content` property.
fn parse_content(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    const MAX_CONTENT_ITEMS: usize = 256;
    // Try keywords
    if input
        .try_parse(|i| i.expect_ident_matching("normal"))
        .is_ok()
    {
        return Ok(CssValue::Keyword("normal".to_string()));
    }
    if input.try_parse(|i| i.expect_ident_matching("none")).is_ok() {
        return Ok(CssValue::Keyword("none".to_string()));
    }

    // Collect content items (strings and attr())
    let mut items: Vec<CssValue> = Vec::new();
    loop {
        if items.len() >= MAX_CONTENT_ITEMS {
            break;
        }
        // Try quoted string
        if let Ok(s) = input.try_parse(|i| i.expect_string().map(std::string::ToString::to_string))
        {
            items.push(CssValue::String(s));
            continue;
        }
        // Try attr() function
        if let Ok(attr_name) = input.try_parse(|i| {
            i.expect_function_matching("attr")?;
            i.parse_nested_block(|nested| -> Result<String, cssparser::ParseError<'_, ()>> {
                let name = nested.expect_ident().map_err(cssparser::ParseError::from)?;
                Ok(name.as_ref().to_string())
            })
        }) {
            items.push(CssValue::Keyword(format!("attr({attr_name})")));
            continue;
        }
        break;
    }

    if items.is_empty() {
        Err(ParseError {
            property: "content".into(),
            input: String::new(),
            message: "expected content value".into(),
        })
    } else if items.len() == 1 {
        Ok(items.into_iter().next().unwrap())
    } else {
        Ok(CssValue::List(items))
    }
}

// ---------------------------------------------------------------------------
// Resolution helpers
// ---------------------------------------------------------------------------

/// Resolve a border-style keyword into a `BorderStyle` field.
fn resolve_border_style(value: &CssValue, target: &mut BorderStyle) {
    if let CssValue::Keyword(ref k) = value {
        *target = BorderStyle::from_keyword(k).unwrap_or_default();
    }
}

/// Resolve a border-style value and zero the corresponding width when the
/// style is `none` or `hidden` (CSS 2.1 §8.5.1).
fn resolve_border_style_and_zero_width(value: &CssValue, style: &mut BorderStyle, width: &mut f32) {
    resolve_border_style(value, style);
    if matches!(*style, BorderStyle::None | BorderStyle::Hidden) {
        *width = 0.0;
    }
}

/// Resolve `max-width`/`max-height`: `none` keyword maps to `Auto`.
fn resolve_max_dimension(value: &CssValue, ctx: &ResolveContext) -> Dimension {
    match value {
        CssValue::Keyword(k) if k == "none" => Dimension::Auto,
        _ => resolve_dimension(value, ctx),
    }
}

/// Resolve a color value, with `currentcolor` mapping to the element's `color`.
// NOTE: similar currentcolor resolution in elidex-style/src/resolve/font.rs
fn resolve_color(value: &CssValue, current_color: CssColor) -> CssColor {
    match value {
        CssValue::Color(c) => *c,
        CssValue::Keyword(k) if k.eq_ignore_ascii_case("currentcolor") => current_color,
        _ => current_color,
    }
}

/// Resolve the `content` property value.
fn resolve_content(value: &CssValue, target: &mut ContentValue) {
    match value {
        CssValue::Keyword(k) => match k.as_str() {
            "normal" => *target = ContentValue::Normal,
            "none" => *target = ContentValue::None,
            kw if kw.starts_with("attr(") && kw.ends_with(')') => {
                if let Some(attr_name) = kw.strip_prefix("attr(").and_then(|s| s.strip_suffix(')'))
                {
                    *target = ContentValue::Items(vec![ContentItem::Attr(attr_name.to_string())]);
                }
            }
            _ => {}
        },
        CssValue::String(s) => {
            *target = ContentValue::Items(vec![ContentItem::String(s.clone())]);
        }
        CssValue::List(items) => {
            let content_items: Vec<ContentItem> = items
                .iter()
                .filter_map(|item| match item {
                    CssValue::String(s) => Some(ContentItem::String(s.clone())),
                    CssValue::Keyword(kw) if kw.starts_with("attr(") && kw.ends_with(')') => kw
                        .strip_prefix("attr(")
                        .and_then(|s| s.strip_suffix(')'))
                        .map(|attr_name| ContentItem::Attr(attr_name.to_string())),
                    _ => None,
                })
                .collect();
            if !content_items.is_empty() {
                *target = ContentValue::Items(content_items);
            }
        }
        _ => {}
    }
}

/// Convert a [`Dimension`] to a [`CssValue`].
// NOTE: also defined in elidex-style/src/resolve/mod.rs
fn dimension_to_css_value(d: Dimension) -> CssValue {
    match d {
        Dimension::Length(px) => CssValue::Length(px, LengthUnit::Px),
        Dimension::Percentage(p) => CssValue::Percentage(p),
        Dimension::Auto => CssValue::Auto,
    }
}

#[cfg(test)]
mod tests;
