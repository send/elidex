//! CSS float, clear, visibility, and vertical-align property handler.

use elidex_plugin::{
    css_resolve::{keyword_from, resolve_length},
    Clear, ComputedStyle, CssPropertyHandler, CssValue, Float, LengthUnit,
    ParseError, PropertyDeclaration, ResolveContext, VerticalAlign, Visibility,
};

/// CSS float/clear/visibility/vertical-align property handler.
pub struct FloatHandler;

impl FloatHandler {
    /// Register this handler in a CSS property registry.
    pub fn register(registry: &mut elidex_plugin::CssPropertyRegistry) {
        for name in Self.property_names() {
            registry.register_static(name, Box::new(Self));
        }
    }
}

impl CssPropertyHandler for FloatHandler {
    fn property_names(&self) -> &[&str] {
        &["float", "clear", "visibility", "vertical-align"]
    }

    fn parse(
        &self,
        name: &str,
        input: &mut cssparser::Parser<'_, '_>,
    ) -> Result<Vec<PropertyDeclaration>, ParseError> {
        let value = match name {
            "float" => parse_keyword(input, &["none", "left", "right"])?,
            "clear" => parse_keyword(input, &["none", "left", "right", "both"])?,
            "visibility" => parse_keyword(input, &["visible", "hidden", "collapse"])?,
            "vertical-align" => parse_vertical_align(input)?,
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
            "float" => {
                if let CssValue::Keyword(ref k) = value {
                    style.float = Float::from_keyword(k).unwrap_or_default();
                }
            }
            "clear" => {
                if let CssValue::Keyword(ref k) = value {
                    style.clear = Clear::from_keyword(k).unwrap_or_default();
                }
            }
            "visibility" => {
                if let CssValue::Keyword(ref k) = value {
                    style.visibility = Visibility::from_keyword(k).unwrap_or_default();
                }
            }
            "vertical-align" => {
                style.vertical_align = match value {
                    CssValue::Keyword(kw) => {
                        VerticalAlign::from_keyword(kw).unwrap_or(VerticalAlign::Baseline)
                    }
                    CssValue::Length(v, unit) => {
                        VerticalAlign::Length(resolve_length(*v, *unit, ctx))
                    }
                    CssValue::Percentage(pct) => VerticalAlign::Percentage(*pct),
                    _ => VerticalAlign::Baseline,
                };
            }
            _ => {}
        }
    }

    fn initial_value(&self, name: &str) -> CssValue {
        match name {
            "float" | "clear" => CssValue::Keyword("none".to_string()),
            "visibility" => CssValue::Keyword("visible".to_string()),
            "vertical-align" => CssValue::Keyword("baseline".to_string()),
            _ => CssValue::Initial,
        }
    }

    fn is_inherited(&self, name: &str) -> bool {
        name == "visibility"
    }

    fn affects_layout(&self, _name: &str) -> bool {
        true
    }

    fn get_computed(&self, name: &str, style: &ComputedStyle) -> CssValue {
        match name {
            "float" => keyword_from(&style.float),
            "clear" => keyword_from(&style.clear),
            "visibility" => keyword_from(&style.visibility),
            "vertical-align" => match &style.vertical_align {
                VerticalAlign::Length(px) => CssValue::Length(*px, LengthUnit::Px),
                VerticalAlign::Percentage(pct) => CssValue::Percentage(*pct),
                other => CssValue::Keyword(other.to_string()),
            },
            _ => CssValue::Initial,
        }
    }
}

fn parse_keyword(
    input: &mut cssparser::Parser<'_, '_>,
    allowed: &[&str],
) -> Result<CssValue, ParseError> {
    let ident = input.expect_ident().map_err(|_| ParseError {
        property: String::new(),
        input: String::new(),
        message: "expected identifier".into(),
    })?;
    let lower = ident.to_ascii_lowercase();
    if allowed.contains(&lower.as_str()) {
        Ok(CssValue::Keyword(lower))
    } else {
        Err(ParseError {
            property: String::new(),
            input: lower,
            message: "unexpected keyword".into(),
        })
    }
}

fn parse_vertical_align(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    // Try keyword first
    if input.try_parse(|i| i.expect_ident_matching("baseline")).is_ok() {
        return Ok(CssValue::Keyword("baseline".to_string()));
    }
    for kw in &[
        "sub", "super", "top", "text-top", "middle", "bottom", "text-bottom",
    ] {
        if input
            .try_parse(|i| i.expect_ident_matching(kw))
            .is_ok()
        {
            return Ok(CssValue::Keyword((*kw).to_string()));
        }
    }

    // Try length/percentage
    if let Ok(value) = input.try_parse(|i| {
        let token = i.next().map_err(|_| ())?;
        match *token {
            cssparser::Token::Dimension { value, ref unit, .. } => {
                let unit = elidex_plugin::css_resolve::parse_length_unit(unit);
                Ok(CssValue::Length(value, unit))
            }
            cssparser::Token::Percentage { unit_value, .. } => {
                Ok(CssValue::Percentage(unit_value * 100.0))
            }
            cssparser::Token::Number { value: 0.0, .. } => {
                Ok(CssValue::Length(0.0, LengthUnit::Px))
            }
            _ => Err(()),
        }
    }) {
        return Ok(value);
    }

    Err(ParseError {
        property: "vertical-align".into(),
        input: String::new(),
        message: "expected keyword, length, or percentage".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_handler_property_names() {
        let handler = FloatHandler;
        assert_eq!(
            handler.property_names(),
            &["float", "clear", "visibility", "vertical-align"]
        );
    }

    #[test]
    fn float_handler_parse_float() {
        let handler = FloatHandler;
        let mut pi = cssparser::ParserInput::new("left");
        let mut parser = cssparser::Parser::new(&mut pi);
        let result = handler.parse("float", &mut parser).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].property, "float");
        assert_eq!(result[0].value, CssValue::Keyword("left".to_string()));
    }

    #[test]
    fn float_handler_parse_visibility() {
        let handler = FloatHandler;
        let mut pi = cssparser::ParserInput::new("hidden");
        let mut parser = cssparser::Parser::new(&mut pi);
        let result = handler.parse("visibility", &mut parser).unwrap();
        assert_eq!(result[0].value, CssValue::Keyword("hidden".to_string()));
    }

    #[test]
    fn float_handler_resolve_float() {
        let handler = FloatHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        handler.resolve("float", &CssValue::Keyword("left".into()), &ctx, &mut style);
        assert_eq!(style.float, Float::Left);
    }

    #[test]
    fn float_handler_inheritance() {
        let handler = FloatHandler;
        assert!(handler.is_inherited("visibility"));
        assert!(!handler.is_inherited("float"));
        assert!(!handler.is_inherited("clear"));
        assert!(!handler.is_inherited("vertical-align"));
    }

    #[test]
    fn float_handler_initial_values() {
        let handler = FloatHandler;
        assert_eq!(
            handler.initial_value("float"),
            CssValue::Keyword("none".to_string())
        );
        assert_eq!(
            handler.initial_value("visibility"),
            CssValue::Keyword("visible".to_string())
        );
    }

    #[test]
    fn float_handler_get_computed() {
        let handler = FloatHandler;
        let style = ComputedStyle {
            float: Float::Right,
            ..ComputedStyle::default()
        };
        assert_eq!(
            handler.get_computed("float", &style),
            CssValue::Keyword("right".to_string())
        );
    }

    #[test]
    fn vertical_align_parse_keyword() {
        let handler = FloatHandler;
        let mut pi = cssparser::ParserInput::new("middle");
        let mut parser = cssparser::Parser::new(&mut pi);
        let result = handler.parse("vertical-align", &mut parser).unwrap();
        assert_eq!(result[0].value, CssValue::Keyword("middle".to_string()));
    }

    #[test]
    fn vertical_align_parse_length() {
        let handler = FloatHandler;
        let mut pi = cssparser::ParserInput::new("10px");
        let mut parser = cssparser::Parser::new(&mut pi);
        let result = handler.parse("vertical-align", &mut parser).unwrap();
        assert_eq!(result[0].value, CssValue::Length(10.0, LengthUnit::Px));
    }

    #[test]
    fn vertical_align_parse_percentage() {
        let handler = FloatHandler;
        let mut pi = cssparser::ParserInput::new("50%");
        let mut parser = cssparser::Parser::new(&mut pi);
        let result = handler.parse("vertical-align", &mut parser).unwrap();
        assert_eq!(result[0].value, CssValue::Percentage(50.0));
    }
}
