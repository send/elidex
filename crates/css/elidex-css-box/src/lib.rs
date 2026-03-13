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

            "min-width" | "min-height" | "border-radius" | "row-gap" | "column-gap"
            | "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => {
                parse_non_negative_length_percentage(input)?
            }

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

            "width" | "height" | "max-width" | "max-height" => CssValue::Auto,

            "min-width" | "min-height" | "margin-top" | "margin-right" | "margin-bottom"
            | "margin-left" | "padding-top" | "padding-right" | "padding-bottom"
            | "padding-left" | "border-radius" | "row-gap" | "column-gap" => {
                CssValue::Length(0.0, LengthUnit::Px)
            }

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

    fn affects_layout(&self, _name: &str) -> bool {
        true
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
fn dimension_to_css_value(d: Dimension) -> CssValue {
    match d {
        Dimension::Length(px) => CssValue::Length(px, LengthUnit::Px),
        Dimension::Percentage(p) => CssValue::Percentage(p),
        Dimension::Auto => CssValue::Auto,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handler() -> BoxHandler {
        BoxHandler
    }

    fn parse(name: &str, css: &str) -> Vec<PropertyDeclaration> {
        let h = handler();
        let mut pi = cssparser::ParserInput::new(css);
        let mut parser = cssparser::Parser::new(&mut pi);
        h.parse(name, &mut parser).unwrap()
    }

    fn default_ctx() -> ResolveContext {
        ResolveContext {
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            em_base: 16.0,
            root_font_size: 16.0,
        }
    }

    // --- Parse tests ---

    #[test]
    fn parse_display_keywords() {
        for kw in DISPLAY_KEYWORDS {
            let decls = parse("display", kw);
            assert_eq!(decls.len(), 1);
            assert_eq!(decls[0].value, CssValue::Keyword(kw.to_string()));
        }
    }

    #[test]
    fn parse_position_keywords() {
        for kw in POSITION_KEYWORDS {
            let decls = parse("position", kw);
            assert_eq!(decls[0].value, CssValue::Keyword(kw.to_string()));
        }
    }

    #[test]
    fn parse_width_auto() {
        let decls = parse("width", "auto");
        assert_eq!(decls[0].value, CssValue::Auto);
    }

    #[test]
    fn parse_width_length() {
        let decls = parse("width", "100px");
        assert_eq!(decls[0].value, CssValue::Length(100.0, LengthUnit::Px));
    }

    #[test]
    fn parse_max_width_none() {
        let decls = parse("max-width", "none");
        assert_eq!(decls[0].value, CssValue::Auto);
    }

    #[test]
    fn parse_margin_percentage() {
        let decls = parse("margin-top", "50%");
        assert_eq!(decls[0].value, CssValue::Percentage(50.0));
    }

    #[test]
    fn parse_padding_rejects_negative() {
        let h = handler();
        let mut pi = cssparser::ParserInput::new("-10px");
        let mut parser = cssparser::Parser::new(&mut pi);
        assert!(h.parse("padding-top", &mut parser).is_err());
    }

    #[test]
    fn parse_border_width_keywords() {
        let decls = parse("border-top-width", "thin");
        assert_eq!(decls[0].value, CssValue::Length(1.0, LengthUnit::Px));

        let decls = parse("border-top-width", "medium");
        assert_eq!(decls[0].value, CssValue::Length(3.0, LengthUnit::Px));

        let decls = parse("border-top-width", "thick");
        assert_eq!(decls[0].value, CssValue::Length(5.0, LengthUnit::Px));
    }

    #[test]
    fn parse_border_style() {
        let decls = parse("border-left-style", "dashed");
        assert_eq!(decls[0].value, CssValue::Keyword("dashed".to_string()));
    }

    #[test]
    fn parse_border_color_currentcolor() {
        let decls = parse("border-top-color", "currentcolor");
        assert_eq!(
            decls[0].value,
            CssValue::Keyword("currentcolor".to_string())
        );
    }

    #[test]
    fn parse_border_color_hex() {
        let decls = parse("border-top-color", "#ff0000");
        assert_eq!(
            decls[0].value,
            CssValue::Color(CssColor {
                r: 255,
                g: 0,
                b: 0,
                a: 255
            })
        );
    }

    #[test]
    fn parse_opacity_clamp() {
        let decls = parse("opacity", "1.5");
        assert_eq!(decls[0].value, CssValue::Number(1.0));

        let decls = parse("opacity", "-0.5");
        assert_eq!(decls[0].value, CssValue::Number(0.0));

        let decls = parse("opacity", "0.5");
        assert_eq!(decls[0].value, CssValue::Number(0.5));
    }

    #[test]
    fn parse_background_color() {
        let decls = parse("background-color", "red");
        assert_eq!(
            decls[0].value,
            CssValue::Color(CssColor {
                r: 255,
                g: 0,
                b: 0,
                a: 255
            })
        );
    }

    #[test]
    fn parse_content_string() {
        let decls = parse("content", "\"hello\"");
        assert_eq!(decls[0].value, CssValue::String("hello".to_string()));
    }

    #[test]
    fn parse_content_normal_none() {
        let decls = parse("content", "normal");
        assert_eq!(decls[0].value, CssValue::Keyword("normal".to_string()));

        let decls = parse("content", "none");
        assert_eq!(decls[0].value, CssValue::Keyword("none".to_string()));
    }

    #[test]
    fn parse_content_attr() {
        let decls = parse("content", "attr(title)");
        assert_eq!(decls[0].value, CssValue::Keyword("attr(title)".to_string()));
    }

    #[test]
    fn parse_box_sizing() {
        let decls = parse("box-sizing", "border-box");
        assert_eq!(decls[0].value, CssValue::Keyword("border-box".to_string()));
    }

    #[test]
    fn parse_overflow_keywords() {
        for kw in OVERFLOW_KEYWORDS {
            let decls = parse("overflow", kw);
            assert_eq!(decls[0].value, CssValue::Keyword(kw.to_string()));
        }
    }

    #[test]
    fn parse_row_gap() {
        let decls = parse("row-gap", "10px");
        assert_eq!(decls[0].value, CssValue::Length(10.0, LengthUnit::Px));
    }

    // --- Resolve tests ---

    #[test]
    fn resolve_display() {
        let h = handler();
        let ctx = default_ctx();
        let mut style = ComputedStyle::default();
        h.resolve(
            "display",
            &CssValue::Keyword("flex".into()),
            &ctx,
            &mut style,
        );
        assert_eq!(style.display, Display::Flex);
    }

    #[test]
    fn resolve_width_and_margin() {
        let h = handler();
        let ctx = default_ctx();
        let mut style = ComputedStyle::default();
        h.resolve(
            "width",
            &CssValue::Length(200.0, LengthUnit::Px),
            &ctx,
            &mut style,
        );
        assert_eq!(style.width, Dimension::Length(200.0));

        h.resolve("margin-left", &CssValue::Auto, &ctx, &mut style);
        assert_eq!(style.margin_left, Dimension::Auto);
    }

    #[test]
    fn resolve_padding_non_negative() {
        let h = handler();
        let ctx = default_ctx();
        let mut style = ComputedStyle::default();
        h.resolve(
            "padding-top",
            &CssValue::Length(10.0, LengthUnit::Px),
            &ctx,
            &mut style,
        );
        assert_eq!(style.padding_top, 10.0);
    }

    #[test]
    fn resolve_border_color_currentcolor() {
        let h = handler();
        let ctx = default_ctx();
        let mut style = ComputedStyle {
            color: CssColor {
                r: 0,
                g: 128,
                b: 255,
                a: 255,
            },
            ..ComputedStyle::default()
        };
        h.resolve(
            "border-top-color",
            &CssValue::Keyword("currentcolor".into()),
            &ctx,
            &mut style,
        );
        assert_eq!(
            style.border_top_color,
            CssColor {
                r: 0,
                g: 128,
                b: 255,
                a: 255
            }
        );
    }

    #[test]
    fn resolve_opacity() {
        let h = handler();
        let ctx = default_ctx();
        let mut style = ComputedStyle::default();
        h.resolve("opacity", &CssValue::Number(0.75), &ctx, &mut style);
        assert_eq!(style.opacity, 0.75);
    }

    #[test]
    fn resolve_content_string() {
        let h = handler();
        let ctx = default_ctx();
        let mut style = ComputedStyle::default();
        h.resolve("content", &CssValue::String(">>".into()), &ctx, &mut style);
        assert_eq!(
            style.content,
            ContentValue::Items(vec![ContentItem::String(">>".into())])
        );
    }

    // --- Initial value tests ---

    #[test]
    fn initial_values() {
        let h = handler();
        assert_eq!(
            h.initial_value("display"),
            CssValue::Keyword("inline".to_string())
        );
        assert_eq!(h.initial_value("width"), CssValue::Auto);
        assert_eq!(
            h.initial_value("padding-top"),
            CssValue::Length(0.0, LengthUnit::Px)
        );
        assert_eq!(
            h.initial_value("border-top-width"),
            CssValue::Length(3.0, LengthUnit::Px)
        );
        assert_eq!(h.initial_value("opacity"), CssValue::Number(1.0));
        assert_eq!(
            h.initial_value("background-color"),
            CssValue::Color(CssColor::TRANSPARENT)
        );
    }

    // --- Inheritance ---

    #[test]
    fn no_properties_inherited() {
        let h = handler();
        for name in BOX_PROPERTIES {
            assert!(!h.is_inherited(name), "{name} should not be inherited");
        }
    }

    // --- get_computed ---

    #[test]
    fn get_computed_display() {
        let h = handler();
        let style = ComputedStyle {
            display: Display::Flex,
            ..ComputedStyle::default()
        };
        assert_eq!(
            h.get_computed("display", &style),
            CssValue::Keyword("flex".to_string())
        );
    }

    #[test]
    fn get_computed_max_width_none() {
        let h = handler();
        let style = ComputedStyle {
            max_width: Dimension::Auto,
            ..ComputedStyle::default()
        };
        assert_eq!(
            h.get_computed("max-width", &style),
            CssValue::Keyword("none".to_string())
        );
    }

    #[test]
    fn get_computed_padding() {
        let h = handler();
        let style = ComputedStyle {
            padding_left: 20.0,
            ..ComputedStyle::default()
        };
        assert_eq!(
            h.get_computed("padding-left", &style),
            CssValue::Length(20.0, LengthUnit::Px)
        );
    }

    #[test]
    fn get_computed_content_items() {
        let h = handler();
        let style = ComputedStyle {
            content: ContentValue::Items(vec![
                ContentItem::String("a".into()),
                ContentItem::Attr("title".into()),
            ]),
            ..ComputedStyle::default()
        };
        assert_eq!(
            h.get_computed("content", &style),
            CssValue::List(vec![
                CssValue::String("a".into()),
                CssValue::Keyword("attr:title".into()),
            ])
        );
    }

    #[test]
    fn resolve_overflow_scroll_maps_to_hidden() {
        let h = handler();
        let ctx = default_ctx();
        let mut style = ComputedStyle::default();
        h.resolve(
            "overflow",
            &CssValue::Keyword("scroll".into()),
            &ctx,
            &mut style,
        );
        assert_eq!(style.overflow, Overflow::Hidden);
    }

    #[test]
    fn resolve_em_units() {
        let h = handler();
        let ctx = ResolveContext {
            em_base: 20.0,
            ..default_ctx()
        };
        let mut style = ComputedStyle::default();
        h.resolve(
            "width",
            &CssValue::Length(2.0, LengthUnit::Em),
            &ctx,
            &mut style,
        );
        assert_eq!(style.width, Dimension::Length(40.0));
    }
}
