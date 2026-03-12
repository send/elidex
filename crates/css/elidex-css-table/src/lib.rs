//! CSS table property handler plugin (border-collapse, table-layout,
//! caption-side, border-spacing).

use elidex_plugin::{
    css_resolve::{keyword_from, parse_length_unit, resolve_to_px},
    BorderCollapse, CaptionSide, ComputedStyle, CssPropertyHandler, CssValue, LengthUnit,
    ParseError, PropertyDeclaration, ResolveContext, TableLayout,
};

/// CSS table property handler.
pub struct TableHandler;

impl TableHandler {
    /// Register this handler in a CSS property registry.
    pub fn register(registry: &mut elidex_plugin::CssPropertyRegistry) {
        for name in Self.property_names() {
            registry.register_static(name, Box::new(Self));
        }
    }
}

impl CssPropertyHandler for TableHandler {
    fn property_names(&self) -> &[&str] {
        &[
            "border-collapse",
            "border-spacing-h",
            "border-spacing-v",
            "table-layout",
            "caption-side",
        ]
    }

    fn parse(
        &self,
        name: &str,
        input: &mut cssparser::Parser<'_, '_>,
    ) -> Result<Vec<PropertyDeclaration>, ParseError> {
        let value = match name {
            "border-collapse" => parse_keyword(input, &["separate", "collapse"])?,
            "border-spacing-h" | "border-spacing-v" => parse_non_negative_length(input)?,
            "table-layout" => parse_keyword(input, &["auto", "fixed"])?,
            "caption-side" => parse_keyword(input, &["top", "bottom"])?,
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
            "border-collapse" => {
                if let CssValue::Keyword(ref k) = value {
                    style.border_collapse =
                        BorderCollapse::from_keyword(k).unwrap_or_default();
                }
            }
            "border-spacing-h" => {
                let px = resolve_to_px(value, ctx).max(0.0);
                style.border_spacing_h = px;
            }
            "border-spacing-v" => {
                let px = resolve_to_px(value, ctx).max(0.0);
                style.border_spacing_v = px;
            }
            "table-layout" => {
                if let CssValue::Keyword(ref k) = value {
                    style.table_layout =
                        TableLayout::from_keyword(k).unwrap_or_default();
                }
            }
            "caption-side" => {
                if let CssValue::Keyword(ref k) = value {
                    style.caption_side =
                        CaptionSide::from_keyword(k).unwrap_or_default();
                }
            }
            _ => {}
        }
    }

    fn initial_value(&self, name: &str) -> CssValue {
        match name {
            "border-collapse" => CssValue::Keyword("separate".to_string()),
            "border-spacing-h" | "border-spacing-v" => CssValue::Length(0.0, LengthUnit::Px),
            "table-layout" => CssValue::Keyword("auto".to_string()),
            "caption-side" => CssValue::Keyword("top".to_string()),
            _ => CssValue::Initial,
        }
    }

    fn is_inherited(&self, name: &str) -> bool {
        matches!(
            name,
            "border-collapse" | "border-spacing-h" | "border-spacing-v" | "caption-side"
        )
    }

    fn affects_layout(&self, _name: &str) -> bool {
        true
    }

    fn get_computed(&self, name: &str, style: &ComputedStyle) -> CssValue {
        match name {
            "border-collapse" => keyword_from(&style.border_collapse),
            "border-spacing-h" => CssValue::Length(style.border_spacing_h, LengthUnit::Px),
            "border-spacing-v" => CssValue::Length(style.border_spacing_v, LengthUnit::Px),
            "table-layout" => keyword_from(&style.table_layout),
            "caption-side" => keyword_from(&style.caption_side),
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

fn parse_non_negative_length(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    let token = input.next().map_err(|_| ParseError {
        property: String::new(),
        input: String::new(),
        message: "expected length value".into(),
    })?;
    match *token {
        cssparser::Token::Dimension { value, ref unit, .. } => {
            if value < 0.0 {
                return Err(ParseError {
                    property: String::new(),
                    input: format!("{value}{unit}"),
                    message: "negative length not allowed".into(),
                });
            }
            let unit = parse_length_unit(unit);
            Ok(CssValue::Length(value, unit))
        }
        cssparser::Token::Number { value: 0.0, .. } => {
            Ok(CssValue::Length(0.0, LengthUnit::Px))
        }
        _ => Err(ParseError {
            property: String::new(),
            input: String::new(),
            message: "expected length value".into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_helper(name: &str, css: &str) -> Vec<PropertyDeclaration> {
        let handler = TableHandler;
        let mut pi = cssparser::ParserInput::new(css);
        let mut parser = cssparser::Parser::new(&mut pi);
        handler.parse(name, &mut parser).unwrap()
    }

    #[test]
    fn property_names() {
        let handler = TableHandler;
        let names = handler.property_names();
        assert!(names.contains(&"border-collapse"));
        assert!(names.contains(&"border-spacing-h"));
        assert!(names.contains(&"border-spacing-v"));
        assert!(names.contains(&"table-layout"));
        assert!(names.contains(&"caption-side"));
    }

    #[test]
    fn parse_border_collapse() {
        let result = parse_helper("border-collapse", "collapse");
        assert_eq!(result[0].value, CssValue::Keyword("collapse".to_string()));

        let result = parse_helper("border-collapse", "separate");
        assert_eq!(result[0].value, CssValue::Keyword("separate".to_string()));
    }

    #[test]
    fn parse_border_collapse_invalid() {
        let handler = TableHandler;
        let mut pi = cssparser::ParserInput::new("none");
        let mut parser = cssparser::Parser::new(&mut pi);
        assert!(handler.parse("border-collapse", &mut parser).is_err());
    }

    #[test]
    fn parse_border_spacing_length() {
        let result = parse_helper("border-spacing-h", "10px");
        assert_eq!(result[0].value, CssValue::Length(10.0, LengthUnit::Px));

        let result = parse_helper("border-spacing-v", "2em");
        assert_eq!(result[0].value, CssValue::Length(2.0, LengthUnit::Em));
    }

    #[test]
    fn parse_border_spacing_zero() {
        let result = parse_helper("border-spacing-h", "0");
        assert_eq!(result[0].value, CssValue::Length(0.0, LengthUnit::Px));
    }

    #[test]
    fn parse_border_spacing_negative_rejected() {
        let handler = TableHandler;
        let mut pi = cssparser::ParserInput::new("-5px");
        let mut parser = cssparser::Parser::new(&mut pi);
        assert!(handler.parse("border-spacing-h", &mut parser).is_err());
    }

    #[test]
    fn parse_table_layout_and_caption_side() {
        let result = parse_helper("table-layout", "fixed");
        assert_eq!(result[0].value, CssValue::Keyword("fixed".to_string()));

        let result = parse_helper("caption-side", "bottom");
        assert_eq!(result[0].value, CssValue::Keyword("bottom".to_string()));
    }

    #[test]
    fn resolve_all_properties() {
        let handler = TableHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();

        handler.resolve(
            "border-collapse",
            &CssValue::Keyword("collapse".into()),
            &ctx,
            &mut style,
        );
        assert_eq!(style.border_collapse, BorderCollapse::Collapse);

        handler.resolve(
            "border-spacing-h",
            &CssValue::Length(8.0, LengthUnit::Px),
            &ctx,
            &mut style,
        );
        assert_eq!(style.border_spacing_h, 8.0);

        handler.resolve(
            "border-spacing-v",
            &CssValue::Length(4.0, LengthUnit::Px),
            &ctx,
            &mut style,
        );
        assert_eq!(style.border_spacing_v, 4.0);

        handler.resolve(
            "table-layout",
            &CssValue::Keyword("fixed".into()),
            &ctx,
            &mut style,
        );
        assert_eq!(style.table_layout, TableLayout::Fixed);

        handler.resolve(
            "caption-side",
            &CssValue::Keyword("bottom".into()),
            &ctx,
            &mut style,
        );
        assert_eq!(style.caption_side, CaptionSide::Bottom);
    }

    #[test]
    fn inheritance_flags() {
        let handler = TableHandler;
        assert!(handler.is_inherited("border-collapse"));
        assert!(handler.is_inherited("border-spacing-h"));
        assert!(handler.is_inherited("border-spacing-v"));
        assert!(handler.is_inherited("caption-side"));
        assert!(!handler.is_inherited("table-layout"));
    }

    #[test]
    fn initial_values() {
        let handler = TableHandler;
        assert_eq!(
            handler.initial_value("border-collapse"),
            CssValue::Keyword("separate".to_string())
        );
        assert_eq!(
            handler.initial_value("border-spacing-h"),
            CssValue::Length(0.0, LengthUnit::Px)
        );
        assert_eq!(
            handler.initial_value("table-layout"),
            CssValue::Keyword("auto".to_string())
        );
        assert_eq!(
            handler.initial_value("caption-side"),
            CssValue::Keyword("top".to_string())
        );
    }

    #[test]
    fn get_computed_roundtrip() {
        let handler = TableHandler;
        let style = ComputedStyle {
            border_collapse: BorderCollapse::Collapse,
            border_spacing_h: 5.0,
            border_spacing_v: 3.0,
            table_layout: TableLayout::Fixed,
            caption_side: CaptionSide::Bottom,
            ..ComputedStyle::default()
        };
        assert_eq!(
            handler.get_computed("border-collapse", &style),
            CssValue::Keyword("collapse".to_string())
        );
        assert_eq!(
            handler.get_computed("border-spacing-h", &style),
            CssValue::Length(5.0, LengthUnit::Px)
        );
        assert_eq!(
            handler.get_computed("border-spacing-v", &style),
            CssValue::Length(3.0, LengthUnit::Px)
        );
        assert_eq!(
            handler.get_computed("table-layout", &style),
            CssValue::Keyword("fixed".to_string())
        );
        assert_eq!(
            handler.get_computed("caption-side", &style),
            CssValue::Keyword("bottom".to_string())
        );
    }

    #[test]
    fn resolve_border_spacing_clamps_negative() {
        let handler = TableHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        // Even if somehow a negative value gets through parsing,
        // resolve clamps to 0.0
        handler.resolve(
            "border-spacing-h",
            &CssValue::Length(-10.0, LengthUnit::Px),
            &ctx,
            &mut style,
        );
        assert_eq!(style.border_spacing_h, 0.0);
    }
}
