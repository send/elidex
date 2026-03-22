//! CSS box model property handler plugin (display, position, margin, padding,
//! border, opacity, box-sizing, overflow, content, gap).

use elidex_plugin::{
    css_resolve::{
        dimension_to_css_value, keyword_from, parse_length_percentage_auto,
        parse_length_percentage_auto_or_none, parse_non_negative_length_or_percentage,
        resolve_color, resolve_dimension, resolve_to_px,
    },
    parse_css_keyword as parse_keyword, BorderStyle, BoxDecorationBreak, BoxSizing,
    BreakInsideValue, BreakValue, ComputedStyle, ContentItem, ContentValue, CssPropertyHandler,
    CssValue, Dimension, Display, LengthUnit, ListStyleType, Overflow, ParseError, Position,
    PropertyDeclaration, ResolveContext,
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
    "border-top-left-radius",
    "border-top-right-radius",
    "border-bottom-right-radius",
    "border-bottom-left-radius",
    "opacity",
    "overflow",
    "overflow-x",
    "overflow-y",
    "content",
    "row-gap",
    "column-gap",
    "top",
    "right",
    "bottom",
    "left",
    "z-index",
    "break-before",
    "break-after",
    "break-inside",
    "box-decoration-break",
    "orphans",
    "widows",
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
            "overflow-x" | "overflow-y" => parse_keyword(input, OVERFLOW_KEYWORDS)?,

            "border-top-style"
            | "border-right-style"
            | "border-bottom-style"
            | "border-left-style" => parse_keyword(input, BORDER_STYLE_KEYWORDS)?,

            "width" | "height" | "max-width" | "max-height" => {
                parse_length_percentage_auto_or_none(input)?
            }

            "min-width" | "min-height" => parse_length_percentage_auto(input)?,

            "border-radius" | "row-gap" | "column-gap" | "padding-top" | "padding-right"
            | "padding-bottom" | "padding-left" => parse_non_negative_length_or_percentage(input)?,

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
            | "border-left-color" => elidex_css::parse_color_with_currentcolor(input)?,

            "opacity" => parse_opacity(input)?,
            "content" => parse_content(input)?,

            "break-before" | "break-after" => parse_keyword(
                input,
                &[
                    "auto",
                    "avoid",
                    "avoid-page",
                    "avoid-column",
                    "page",
                    "column",
                    "left",
                    "right",
                    "recto",
                    "verso",
                ],
            )?,
            "break-inside" => {
                parse_keyword(input, &["auto", "avoid", "avoid-page", "avoid-column"])?
            }
            "box-decoration-break" => parse_keyword(input, &["slice", "clone"])?,
            "orphans" | "widows" => parse_positive_integer(input, name)?,

            // "overflow" shorthand is expanded into overflow-x/y by elidex-css
            // before reaching this handler. It remains in BOX_PROPERTIES for
            // get_computed() / initial_value() shorthand queries.
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
                elidex_plugin::resolve_keyword!(value, style.display, Display);
            }
            "position" => {
                elidex_plugin::resolve_keyword!(value, style.position, Position);
            }
            "box-sizing" => {
                elidex_plugin::resolve_keyword!(value, style.box_sizing, BoxSizing);
            }
            "overflow-x" => {
                if let CssValue::Keyword(ref k) = value {
                    style.overflow_x = Overflow::from_keyword(k).unwrap_or_default();
                }
            }
            "overflow-y" => {
                if let CssValue::Keyword(ref k) = value {
                    style.overflow_y = Overflow::from_keyword(k).unwrap_or_default();
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

            "padding-top" => style.padding.top = resolve_padding_dimension(value, ctx),
            "padding-right" => style.padding.right = resolve_padding_dimension(value, ctx),
            "padding-bottom" => style.padding.bottom = resolve_padding_dimension(value, ctx),
            "padding-left" => style.padding.left = resolve_padding_dimension(value, ctx),

            "border-top-width" => style.border_top.width = resolve_to_px(value, ctx).max(0.0),
            "border-right-width" => style.border_right.width = resolve_to_px(value, ctx).max(0.0),
            "border-bottom-width" => {
                style.border_bottom.width = resolve_to_px(value, ctx).max(0.0);
            }
            "border-left-width" => style.border_left.width = resolve_to_px(value, ctx).max(0.0),

            "border-top-style" => {
                resolve_border_style_and_zero_width(
                    value,
                    &mut style.border_top.style,
                    &mut style.border_top.width,
                );
            }
            "border-right-style" => {
                resolve_border_style_and_zero_width(
                    value,
                    &mut style.border_right.style,
                    &mut style.border_right.width,
                );
            }
            "border-bottom-style" => {
                resolve_border_style_and_zero_width(
                    value,
                    &mut style.border_bottom.style,
                    &mut style.border_bottom.width,
                );
            }
            "border-left-style" => {
                resolve_border_style_and_zero_width(
                    value,
                    &mut style.border_left.style,
                    &mut style.border_left.width,
                );
            }

            "border-top-color" => style.border_top.color = resolve_color(value, style.color),
            "border-right-color" => style.border_right.color = resolve_color(value, style.color),
            "border-bottom-color" => style.border_bottom.color = resolve_color(value, style.color),
            "border-left-color" => style.border_left.color = resolve_color(value, style.color),

            "border-radius" => {
                let r = resolve_to_px(value, ctx).max(0.0);
                style.border_radii = [r; 4];
            }
            "border-top-left-radius" => {
                style.border_radii[0] = resolve_to_px(value, ctx).max(0.0);
            }
            "border-top-right-radius" => {
                style.border_radii[1] = resolve_to_px(value, ctx).max(0.0);
            }
            "border-bottom-right-radius" => {
                style.border_radii[2] = resolve_to_px(value, ctx).max(0.0);
            }
            "border-bottom-left-radius" => {
                style.border_radii[3] = resolve_to_px(value, ctx).max(0.0);
            }

            "opacity" => {
                if let CssValue::Number(n) = value {
                    style.opacity = n.clamp(0.0, 1.0);
                }
            }

            "content" => resolve_content(value, &mut style.content),
            "row-gap" => style.row_gap = resolve_gap_dimension(value, ctx),
            "column-gap" => style.column_gap = resolve_gap_dimension(value, ctx),

            "top" => style.top = resolve_dimension(value, ctx),
            "right" => style.right = resolve_dimension(value, ctx),
            "bottom" => style.bottom = resolve_dimension(value, ctx),
            "left" => style.left = resolve_dimension(value, ctx),
            "z-index" => {
                style.z_index = if let CssValue::Number(n) = value {
                    #[allow(clippy::cast_possible_truncation)]
                    Some(*n as i32)
                } else {
                    None
                };
            }

            "break-before" => {
                elidex_plugin::resolve_keyword!(value, style.break_before, BreakValue);
            }
            "break-after" => {
                elidex_plugin::resolve_keyword!(value, style.break_after, BreakValue);
            }
            "break-inside" => {
                elidex_plugin::resolve_keyword!(value, style.break_inside, BreakInsideValue);
            }
            "box-decoration-break" => {
                elidex_plugin::resolve_keyword!(
                    value,
                    style.box_decoration_break,
                    BoxDecorationBreak
                );
            }
            "orphans" => {
                if let CssValue::Number(n) = value {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    {
                        style.orphans = (*n as u32).max(1);
                    }
                }
            }
            "widows" => {
                if let CssValue::Number(n) = value {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    {
                        style.widows = (*n as u32).max(1);
                    }
                }
            }

            _ => {}
        }
    }

    fn initial_value(&self, name: &str) -> CssValue {
        match name {
            "display" => CssValue::Keyword("inline".to_string()),
            "position" => CssValue::Keyword("static".to_string()),

            "width" | "height" | "max-width" | "max-height" | "top" | "right" | "bottom"
            | "left" | "z-index" => CssValue::Auto,

            "min-width"
            | "min-height"
            | "margin-top"
            | "margin-right"
            | "margin-bottom"
            | "margin-left"
            | "padding-top"
            | "padding-right"
            | "padding-bottom"
            | "padding-left"
            | "border-radius"
            | "border-top-left-radius"
            | "border-top-right-radius"
            | "border-bottom-right-radius"
            | "border-bottom-left-radius"
            | "row-gap"
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
            "overflow" | "overflow-x" | "overflow-y" => CssValue::Keyword("visible".to_string()),
            "content" => CssValue::Keyword("normal".to_string()),

            "break-before" | "break-after" | "break-inside" => {
                CssValue::Keyword("auto".to_string())
            }
            "box-decoration-break" => CssValue::Keyword("slice".to_string()),
            "orphans" | "widows" => CssValue::Number(2.0),

            _ => CssValue::Initial,
        }
    }

    fn is_inherited(&self, name: &str) -> bool {
        matches!(name, "orphans" | "widows")
    }

    fn affects_layout(&self, name: &str) -> bool {
        !matches!(
            name,
            "opacity"
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
            "overflow" => {
                if style.overflow_x == style.overflow_y {
                    keyword_from(&style.overflow_x)
                } else {
                    CssValue::Keyword(format!("{} {}", style.overflow_x, style.overflow_y))
                }
            }
            "overflow-x" => keyword_from(&style.overflow_x),
            "overflow-y" => keyword_from(&style.overflow_y),

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

            "padding-top" => dimension_to_css_value(style.padding.top),
            "padding-right" => dimension_to_css_value(style.padding.right),
            "padding-bottom" => dimension_to_css_value(style.padding.bottom),
            "padding-left" => dimension_to_css_value(style.padding.left),

            "border-top-width" => CssValue::Length(style.border_top.width, LengthUnit::Px),
            "border-right-width" => CssValue::Length(style.border_right.width, LengthUnit::Px),
            "border-bottom-width" => CssValue::Length(style.border_bottom.width, LengthUnit::Px),
            "border-left-width" => CssValue::Length(style.border_left.width, LengthUnit::Px),

            "border-top-style" => keyword_from(&style.border_top.style),
            "border-right-style" => keyword_from(&style.border_right.style),
            "border-bottom-style" => keyword_from(&style.border_bottom.style),
            "border-left-style" => keyword_from(&style.border_left.style),

            "border-top-color" => CssValue::Color(style.border_top.color),
            "border-right-color" => CssValue::Color(style.border_right.color),
            "border-bottom-color" => CssValue::Color(style.border_bottom.color),
            "border-left-color" => CssValue::Color(style.border_left.color),

            "border-radius" => {
                // Return uniform value if all corners are equal.
                CssValue::Length(style.border_radii[0], LengthUnit::Px)
            }
            "border-top-left-radius" => CssValue::Length(style.border_radii[0], LengthUnit::Px),
            "border-top-right-radius" => CssValue::Length(style.border_radii[1], LengthUnit::Px),
            "border-bottom-right-radius" => CssValue::Length(style.border_radii[2], LengthUnit::Px),
            "border-bottom-left-radius" => CssValue::Length(style.border_radii[3], LengthUnit::Px),
            "opacity" => CssValue::Number(style.opacity),

            "content" => computed_content(style),

            "row-gap" => dimension_to_css_value(style.row_gap),
            "column-gap" => dimension_to_css_value(style.column_gap),

            "top" => dimension_to_css_value(style.top),
            "right" => dimension_to_css_value(style.right),
            "bottom" => dimension_to_css_value(style.bottom),
            "left" => dimension_to_css_value(style.left),
            #[allow(clippy::cast_precision_loss)]
            "z-index" => match style.z_index {
                Some(z) => CssValue::Number(z as f32),
                None => CssValue::Auto,
            },

            "break-before" => keyword_from(&style.break_before),
            "break-after" => keyword_from(&style.break_after),
            "break-inside" => keyword_from(&style.break_inside),
            "box-decoration-break" => keyword_from(&style.box_decoration_break),
            #[allow(clippy::cast_precision_loss)]
            "orphans" => CssValue::Number(style.orphans as f32),
            #[allow(clippy::cast_precision_loss)]
            "widows" => CssValue::Number(style.widows as f32),

            _ => CssValue::Initial,
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

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
    parse_non_negative_length_or_percentage(input)
}

/// Parse opacity: a number clamped to 0.0..=1.0.
fn parse_positive_integer(
    input: &mut cssparser::Parser<'_, '_>,
    prop: &str,
) -> Result<CssValue, ParseError> {
    let n = input.expect_integer().map_err(|_| ParseError {
        property: prop.into(),
        input: String::new(),
        message: "expected positive integer".into(),
    })?;
    if n < 1 {
        return Err(ParseError {
            property: prop.into(),
            input: n.to_string(),
            message: "value must be >= 1".into(),
        });
    }
    #[allow(clippy::cast_precision_loss)]
    Ok(CssValue::Number(n as f32))
}

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
        // len checked above; pop cannot fail.
        Ok(items.pop().expect("len == 1"))
    } else {
        Ok(CssValue::List(items))
    }
}

// ---------------------------------------------------------------------------
// Resolution helpers
// ---------------------------------------------------------------------------

/// Resolve a border-style keyword into a `BorderStyle` field.
fn resolve_border_style(value: &CssValue, target: &mut BorderStyle) {
    elidex_plugin::resolve_keyword!(value, *target, BorderStyle);
}

/// Resolve a border-style value and zero the corresponding width when the
/// style is `none` or `hidden` (CSS 2.1 §8.5.1).
fn resolve_border_style_and_zero_width(value: &CssValue, style: &mut BorderStyle, width: &mut f32) {
    resolve_border_style(value, style);
    if matches!(*style, BorderStyle::None | BorderStyle::Hidden) {
        *width = 0.0;
    }
}

/// Resolve a padding value to a `Dimension`, preserving percentages.
///
/// CSS Box Model §4: padding cannot be negative, and `auto` is invalid.
fn resolve_padding_dimension(value: &CssValue, ctx: &ResolveContext) -> Dimension {
    match resolve_dimension(value, ctx) {
        Dimension::Length(px) => Dimension::Length(px.max(0.0)),
        Dimension::Percentage(p) => Dimension::Percentage(p.max(0.0)),
        Dimension::Auto => Dimension::ZERO,
    }
}

/// Resolve a gap value to a `Dimension`, preserving percentages.
fn resolve_gap_dimension(value: &CssValue, ctx: &ResolveContext) -> Dimension {
    match resolve_dimension(value, ctx) {
        Dimension::Length(px) => Dimension::Length(px.max(0.0)),
        Dimension::Percentage(p) => Dimension::Percentage(p.max(0.0)),
        Dimension::Auto => Dimension::ZERO,
    }
}

/// Resolve `max-width`/`max-height`: `none` keyword maps to `Auto`.
fn resolve_max_dimension(value: &CssValue, ctx: &ResolveContext) -> Dimension {
    match value {
        CssValue::Keyword(k) if k == "none" => Dimension::Auto,
        _ => resolve_dimension(value, ctx),
    }
}

/// Convert `ComputedStyle.content` to a `CssValue` for `get_computed`.
fn computed_content(style: &ComputedStyle) -> CssValue {
    match &style.content {
        ContentValue::Normal => CssValue::Keyword("normal".to_string()),
        ContentValue::None => CssValue::Keyword("none".to_string()),
        ContentValue::Items(items) => {
            let mut parts: Vec<CssValue> = items
                .iter()
                .map(|item| match item {
                    ContentItem::String(s) => CssValue::String(s.clone()),
                    ContentItem::Attr(a) => CssValue::Keyword(format!("attr:{a}")),
                    ContentItem::Counter { name, style } => {
                        CssValue::Keyword(format!("counter:{name}:{style}"))
                    }
                    ContentItem::Counters {
                        name,
                        separator,
                        style,
                    } => CssValue::Keyword(format!("counters:{name}:{separator}:{style}")),
                })
                .collect();
            if parts.len() == 1 {
                parts.pop().expect("len == 1")
            } else {
                CssValue::List(parts)
            }
        }
    }
}

/// Resolve the `content` property value.
fn resolve_content(value: &CssValue, target: &mut ContentValue) {
    match value {
        CssValue::Keyword(k) => match k.as_str() {
            "normal" => *target = ContentValue::Normal,
            "none" => *target = ContentValue::None,
            kw if kw.starts_with("attr:") => {
                if let Some(attr_name) = kw.strip_prefix("attr:") {
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
                    CssValue::Keyword(kw) if kw.starts_with("attr:") => kw
                        .strip_prefix("attr:")
                        .map(|attr_name| ContentItem::Attr(attr_name.to_string())),
                    CssValue::Keyword(kw) if kw.starts_with("counter:") => {
                        parse_counter_keyword_to_item(kw)
                    }
                    CssValue::Keyword(kw) if kw.starts_with("counters:") => {
                        parse_counters_keyword_to_item(kw)
                    }
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

/// Parse a `counter:name:style` keyword into a `ContentItem::Counter`.
fn parse_counter_keyword_to_item(k: &str) -> Option<ContentItem> {
    let rest = k.strip_prefix("counter:")?;
    let mut parts = rest.splitn(2, ':');
    let name = parts.next()?.to_string();
    let style_str = parts.next().unwrap_or("decimal");
    let style = ListStyleType::from_keyword(style_str).unwrap_or(ListStyleType::Decimal);
    Some(ContentItem::Counter { name, style })
}

/// Parse a `counters:name:separator:style` keyword into a `ContentItem::Counters`.
fn parse_counters_keyword_to_item(k: &str) -> Option<ContentItem> {
    let rest = k.strip_prefix("counters:")?;
    let mut parts = rest.splitn(3, ':');
    let name = parts.next()?.to_string();
    let separator = parts.next().unwrap_or(".").to_string();
    let style_str = parts.next().unwrap_or("decimal");
    let style = ListStyleType::from_keyword(style_str).unwrap_or(ListStyleType::Decimal);
    Some(ContentItem::Counters {
        name,
        separator,
        style,
    })
}

#[cfg(test)]
mod tests;
