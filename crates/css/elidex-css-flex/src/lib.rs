//! CSS flexbox property handler plugin (flex-direction, flex-wrap,
//! justify-content, align-items/self/content, flex-grow/shrink/basis, order).

use elidex_plugin::{
    css_resolve::{keyword_from, parse_length_or_percentage, resolve_dimension},
    parse_css_keyword as parse_keyword, AlignContent, AlignItems, AlignSelf, ComputedStyle,
    CssPropertyHandler, CssValue, Dimension, FlexDirection, FlexWrap, JustifyContent, LengthUnit,
    ParseError, PropertyDeclaration, ResolveContext,
};

/// CSS flexbox property handler.
#[derive(Clone)]
pub struct FlexHandler;

impl FlexHandler {
    /// Register this handler in a CSS property registry.
    pub fn register(registry: &mut elidex_plugin::CssPropertyRegistry) {
        elidex_plugin::register_css_handler(registry, Self);
    }
}

impl CssPropertyHandler for FlexHandler {
    fn property_names(&self) -> &[&str] {
        &[
            "flex-direction",
            "flex-wrap",
            "justify-content",
            "align-items",
            "align-content",
            "align-self",
            "flex-grow",
            "flex-shrink",
            "flex-basis",
            "order",
        ]
    }

    fn parse(
        &self,
        name: &str,
        input: &mut cssparser::Parser<'_, '_>,
    ) -> Result<Vec<PropertyDeclaration>, ParseError> {
        let value = match name {
            "flex-direction" => {
                parse_keyword(input, &["row", "row-reverse", "column", "column-reverse"])?
            }
            "flex-wrap" => parse_keyword(input, &["nowrap", "wrap", "wrap-reverse"])?,
            "justify-content" => parse_keyword(
                input,
                &[
                    "normal",
                    "flex-start",
                    "flex-end",
                    "center",
                    "space-between",
                    "space-around",
                    "space-evenly",
                ],
            )?,
            "align-items" => parse_keyword(
                input,
                &["stretch", "flex-start", "flex-end", "center", "baseline"],
            )?,
            "align-content" => parse_keyword(
                input,
                &[
                    "normal",
                    "stretch",
                    "flex-start",
                    "flex-end",
                    "center",
                    "space-between",
                    "space-around",
                    "space-evenly",
                ],
            )?,
            "align-self" => parse_keyword(
                input,
                &[
                    "auto",
                    "stretch",
                    "flex-start",
                    "flex-end",
                    "center",
                    "baseline",
                ],
            )?,
            "flex-grow" => parse_non_negative_number(input, "flex-grow")?,
            "flex-shrink" => parse_non_negative_number(input, "flex-shrink")?,
            "flex-basis" => parse_flex_basis(input)?,
            "order" => parse_order(input)?,
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
            "flex-direction" => {
                elidex_plugin::resolve_keyword!(value, style.flex_direction, FlexDirection);
            }
            "flex-wrap" => {
                elidex_plugin::resolve_keyword!(value, style.flex_wrap, FlexWrap);
            }
            "justify-content" => {
                elidex_plugin::resolve_keyword!(value, style.justify_content, JustifyContent);
            }
            "align-items" => {
                elidex_plugin::resolve_keyword!(value, style.align_items, AlignItems);
            }
            "align-content" => {
                elidex_plugin::resolve_keyword!(value, style.align_content, AlignContent);
            }
            "align-self" => {
                elidex_plugin::resolve_keyword!(value, style.align_self, AlignSelf);
            }
            "flex-grow" => {
                if let CssValue::Number(n) = value {
                    style.flex_grow = n.max(0.0);
                }
            }
            "flex-shrink" => {
                if let CssValue::Number(n) = value {
                    style.flex_shrink = n.max(0.0);
                }
            }
            "flex-basis" => {
                style.flex_basis = resolve_dimension(value, ctx);
            }
            "order" => {
                if let CssValue::Number(n) = value {
                    if n.is_finite() {
                        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
                        {
                            style.order = n.clamp(i32::MIN as f32, i32::MAX as f32) as i32;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn initial_value(&self, name: &str) -> CssValue {
        match name {
            "flex-direction" => CssValue::Keyword("row".to_string()),
            "flex-wrap" => CssValue::Keyword("nowrap".to_string()),
            // CSS Box Alignment Level 3: initial value of justify-content and
            // align-content is `normal`, which behaves as `flex-start`/`stretch`
            // respectively in flex containers.
            "justify-content" | "align-content" => CssValue::Keyword("normal".to_string()),
            "align-items" => CssValue::Keyword("stretch".to_string()),
            "align-self" | "flex-basis" => CssValue::Keyword("auto".to_string()),
            "flex-grow" | "order" => CssValue::Number(0.0),
            "flex-shrink" => CssValue::Number(1.0),
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
            "flex-direction" => keyword_from(&style.flex_direction),
            "flex-wrap" => keyword_from(&style.flex_wrap),
            "justify-content" => keyword_from(&style.justify_content),
            "align-items" => keyword_from(&style.align_items),
            "align-content" => keyword_from(&style.align_content),
            "align-self" => keyword_from(&style.align_self),
            "flex-grow" => CssValue::Number(style.flex_grow),
            "flex-shrink" => CssValue::Number(style.flex_shrink),
            "flex-basis" => match style.flex_basis {
                Dimension::Auto => CssValue::Keyword("auto".to_string()),
                Dimension::Length(px) => CssValue::Length(px, LengthUnit::Px),
                Dimension::Percentage(pct) => CssValue::Percentage(pct),
            },
            #[allow(clippy::cast_precision_loss)]
            "order" => CssValue::Number(style.order as f32),
            _ => CssValue::Initial,
        }
    }
}

fn parse_non_negative_number(
    input: &mut cssparser::Parser<'_, '_>,
    prop: &str,
) -> Result<CssValue, ParseError> {
    let token = input.next().map_err(|_| ParseError {
        property: prop.into(),
        input: String::new(),
        message: "expected number".into(),
    })?;
    match *token {
        cssparser::Token::Number { value, .. } => Ok(CssValue::Number(value.max(0.0))),
        _ => Err(ParseError {
            property: prop.into(),
            input: String::new(),
            message: "expected number".into(),
        }),
    }
}

fn parse_flex_basis(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    // Try "auto" keyword
    if input.try_parse(|i| i.expect_ident_matching("auto")).is_ok() {
        return Ok(CssValue::Keyword("auto".to_string()));
    }

    // Try length/percentage/zero
    parse_length_or_percentage(input).map_err(|mut e| {
        e.property = "flex-basis".into();
        e.message = "expected auto, length, or percentage".into();
        e
    })
}

fn parse_order(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    let token = input.next().map_err(|_| ParseError {
        property: "order".into(),
        input: String::new(),
        message: "expected integer".into(),
    })?;
    match *token {
        cssparser::Token::Number { value, .. } => Ok(CssValue::Number(value)),
        _ => Err(ParseError {
            property: "order".into(),
            input: String::new(),
            message: "expected integer".into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(name: &str, css: &str) -> Vec<PropertyDeclaration> {
        let handler = FlexHandler;
        let mut pi = cssparser::ParserInput::new(css);
        let mut parser = cssparser::Parser::new(&mut pi);
        handler.parse(name, &mut parser).unwrap()
    }

    #[test]
    fn property_names_count() {
        let handler = FlexHandler;
        assert_eq!(handler.property_names().len(), 10);
    }

    #[test]
    fn parse_flex_direction_keywords() {
        for kw in &["row", "row-reverse", "column", "column-reverse"] {
            let result = parse("flex-direction", kw);
            assert_eq!(result[0].value, CssValue::Keyword(kw.to_string()));
        }
    }

    #[test]
    fn parse_flex_wrap_keywords() {
        for kw in &["nowrap", "wrap", "wrap-reverse"] {
            let result = parse("flex-wrap", kw);
            assert_eq!(result[0].value, CssValue::Keyword(kw.to_string()));
        }
    }

    #[test]
    fn parse_justify_content_keywords() {
        for kw in &[
            "flex-start",
            "flex-end",
            "center",
            "space-between",
            "space-around",
            "space-evenly",
        ] {
            let result = parse("justify-content", kw);
            assert_eq!(result[0].value, CssValue::Keyword(kw.to_string()));
        }
    }

    #[test]
    fn parse_flex_grow_number() {
        let result = parse("flex-grow", "2.5");
        assert_eq!(result[0].value, CssValue::Number(2.5));
    }

    #[test]
    fn parse_flex_grow_negative_clamped() {
        let result = parse("flex-grow", "-1");
        assert_eq!(result[0].value, CssValue::Number(0.0));
    }

    #[test]
    fn parse_flex_shrink_number() {
        let result = parse("flex-shrink", "0");
        assert_eq!(result[0].value, CssValue::Number(0.0));
    }

    #[test]
    fn parse_flex_basis_auto() {
        let result = parse("flex-basis", "auto");
        assert_eq!(result[0].value, CssValue::Keyword("auto".to_string()));
    }

    #[test]
    fn parse_flex_basis_length() {
        let result = parse("flex-basis", "100px");
        assert_eq!(result[0].value, CssValue::Length(100.0, LengthUnit::Px));
    }

    #[test]
    fn parse_flex_basis_percentage() {
        let result = parse("flex-basis", "50%");
        assert_eq!(result[0].value, CssValue::Percentage(50.0));
    }

    #[test]
    fn parse_order_integer() {
        let result = parse("order", "-3");
        assert_eq!(result[0].value, CssValue::Number(-3.0));
    }

    #[test]
    fn parse_invalid_keyword_rejected() {
        let handler = FlexHandler;
        let mut pi = cssparser::ParserInput::new("invalid");
        let mut parser = cssparser::Parser::new(&mut pi);
        assert!(handler.parse("flex-direction", &mut parser).is_err());
    }

    #[test]
    fn resolve_flex_direction() {
        let handler = FlexHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        handler.resolve(
            "flex-direction",
            &CssValue::Keyword("column".into()),
            &ctx,
            &mut style,
        );
        assert_eq!(style.flex_direction, FlexDirection::Column);
    }

    #[test]
    fn resolve_flex_grow_shrink() {
        let handler = FlexHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        handler.resolve("flex-grow", &CssValue::Number(2.0), &ctx, &mut style);
        handler.resolve("flex-shrink", &CssValue::Number(0.5), &ctx, &mut style);
        assert_eq!(style.flex_grow, 2.0);
        assert_eq!(style.flex_shrink, 0.5);
    }

    #[test]
    fn resolve_flex_basis_length() {
        let handler = FlexHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        handler.resolve(
            "flex-basis",
            &CssValue::Length(200.0, LengthUnit::Px),
            &ctx,
            &mut style,
        );
        assert_eq!(style.flex_basis, Dimension::Length(200.0));
    }

    #[test]
    fn resolve_order() {
        let handler = FlexHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        handler.resolve("order", &CssValue::Number(-2.0), &ctx, &mut style);
        assert_eq!(style.order, -2);
    }

    #[test]
    fn initial_values() {
        let handler = FlexHandler;
        assert_eq!(
            handler.initial_value("flex-direction"),
            CssValue::Keyword("row".into())
        );
        assert_eq!(handler.initial_value("flex-grow"), CssValue::Number(0.0));
        assert_eq!(handler.initial_value("flex-shrink"), CssValue::Number(1.0));
        assert_eq!(
            handler.initial_value("flex-basis"),
            CssValue::Keyword("auto".into())
        );
        assert_eq!(handler.initial_value("order"), CssValue::Number(0.0));
    }

    #[test]
    fn no_properties_inherited() {
        let handler = FlexHandler;
        for name in handler.property_names() {
            assert!(
                !handler.is_inherited(name),
                "{name} should not be inherited"
            );
        }
    }

    #[test]
    fn all_affect_layout() {
        let handler = FlexHandler;
        for name in handler.property_names() {
            assert!(handler.affects_layout(name), "{name} should affect layout");
        }
    }

    #[test]
    fn get_computed_roundtrip() {
        let handler = FlexHandler;
        let style = ComputedStyle {
            flex_direction: FlexDirection::ColumnReverse,
            flex_wrap: FlexWrap::WrapReverse,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            align_content: AlignContent::SpaceEvenly,
            align_self: AlignSelf::FlexEnd,
            flex_grow: 3.0,
            flex_shrink: 0.0,
            flex_basis: Dimension::Percentage(50.0),
            order: 5,
            ..ComputedStyle::default()
        };
        assert_eq!(
            handler.get_computed("flex-direction", &style),
            CssValue::Keyword("column-reverse".into())
        );
        assert_eq!(
            handler.get_computed("flex-wrap", &style),
            CssValue::Keyword("wrap-reverse".into())
        );
        assert_eq!(
            handler.get_computed("justify-content", &style),
            CssValue::Keyword("space-between".into())
        );
        assert_eq!(
            handler.get_computed("align-items", &style),
            CssValue::Keyword("center".into())
        );
        assert_eq!(
            handler.get_computed("align-content", &style),
            CssValue::Keyword("space-evenly".into())
        );
        assert_eq!(
            handler.get_computed("align-self", &style),
            CssValue::Keyword("flex-end".into())
        );
        assert_eq!(
            handler.get_computed("flex-grow", &style),
            CssValue::Number(3.0)
        );
        assert_eq!(
            handler.get_computed("flex-shrink", &style),
            CssValue::Number(0.0)
        );
        assert_eq!(
            handler.get_computed("flex-basis", &style),
            CssValue::Percentage(50.0)
        );
        assert_eq!(handler.get_computed("order", &style), CssValue::Number(5.0));
    }
}
