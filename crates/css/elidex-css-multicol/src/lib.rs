//! CSS multi-column layout property handler plugin (column-count, column-width,
//! column-fill, column-span, column-rule-*, columns shorthand).

use elidex_plugin::{
    css_resolve::{keyword_from, parse_length_unit, resolve_to_px},
    parse_css_keyword as parse_keyword, BorderStyle, ColumnFill, ColumnSpan, ComputedStyle,
    CssColor, CssPropertyHandler, CssValue, Dimension, LengthUnit, ParseError, PropertyDeclaration,
    ResolveContext,
};

/// CSS multi-column property handler.
#[derive(Clone)]
pub struct MulticolHandler;

impl MulticolHandler {
    /// Register this handler in a CSS property registry.
    pub fn register(registry: &mut elidex_plugin::CssPropertyRegistry) {
        elidex_plugin::register_css_handler(registry, Self);
    }
}

const MULTICOL_PROPERTIES: &[&str] = &[
    "column-count",
    "column-width",
    "column-fill",
    "column-span",
    "column-rule-width",
    "column-rule-style",
    "column-rule-color",
];

const BORDER_STYLE_KEYWORDS: &[&str] = &[
    "none", "hidden", "solid", "dashed", "dotted", "double", "groove", "ridge", "inset", "outset",
];

impl CssPropertyHandler for MulticolHandler {
    fn property_names(&self) -> &[&str] {
        MULTICOL_PROPERTIES
    }

    fn parse(
        &self,
        name: &str,
        input: &mut cssparser::Parser<'_, '_>,
    ) -> Result<Vec<PropertyDeclaration>, ParseError> {
        match name {
            "column-count" => {
                let value = parse_column_count(input)?;
                Ok(vec![PropertyDeclaration::new(name, value)])
            }
            "column-width" => {
                let value = parse_column_width(input)?;
                Ok(vec![PropertyDeclaration::new(name, value)])
            }
            "column-fill" => {
                let value = parse_keyword(input, &["balance", "auto"])?;
                Ok(vec![PropertyDeclaration::new(name, value)])
            }
            "column-span" => {
                let value = parse_keyword(input, &["none", "all"])?;
                Ok(vec![PropertyDeclaration::new(name, value)])
            }
            "column-rule-width" => {
                let value = parse_border_width(input)?;
                Ok(vec![PropertyDeclaration::new(name, value)])
            }
            "column-rule-style" => {
                let value = parse_keyword(input, BORDER_STYLE_KEYWORDS)?;
                Ok(vec![PropertyDeclaration::new(name, value)])
            }
            "column-rule-color" => {
                let value = elidex_css::parse_color_with_currentcolor(input)?;
                Ok(vec![PropertyDeclaration::new(name, value)])
            }
            _ => Ok(vec![]),
        }
    }

    fn resolve(
        &self,
        name: &str,
        value: &CssValue,
        ctx: &ResolveContext,
        style: &mut ComputedStyle,
    ) {
        match name {
            "column-count" => {
                style.column_count = match value {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    CssValue::Number(n) => Some((*n as u32).max(1)),
                    _ => None,
                };
            }
            "column-width" => {
                style.column_width = match value {
                    CssValue::Auto | CssValue::Keyword(_) => Dimension::Auto,
                    _ => {
                        let px = resolve_to_px(value, ctx).max(0.0);
                        Dimension::Length(px)
                    }
                };
            }
            "column-fill" => {
                elidex_plugin::resolve_keyword!(value, style.column_fill, ColumnFill);
            }
            "column-span" => {
                elidex_plugin::resolve_keyword!(value, style.column_span, ColumnSpan);
            }
            "column-rule-width" => {
                style.column_rule_width = resolve_to_px(value, ctx).max(0.0);
            }
            "column-rule-style" => {
                elidex_plugin::resolve_keyword!(value, style.column_rule_style, BorderStyle);
            }
            "column-rule-color" => {
                style.column_rule_color = resolve_rule_color(value, style.color);
            }
            _ => {}
        }
    }

    fn initial_value(&self, name: &str) -> CssValue {
        match name {
            "column-count" | "column-width" => CssValue::Auto,
            "column-fill" => CssValue::Keyword("balance".to_string()),
            "column-span" | "column-rule-style" => CssValue::Keyword("none".to_string()),
            "column-rule-width" => CssValue::Length(3.0, LengthUnit::Px),
            "column-rule-color" => CssValue::Keyword("currentcolor".to_string()),
            _ => CssValue::Initial,
        }
    }

    fn is_inherited(&self, _name: &str) -> bool {
        false
    }

    fn affects_layout(&self, name: &str) -> bool {
        !matches!(name, "column-rule-color" | "column-rule-style")
    }

    fn get_computed(&self, name: &str, style: &ComputedStyle) -> CssValue {
        match name {
            #[allow(clippy::cast_precision_loss)]
            "column-count" => match style.column_count {
                Some(n) => CssValue::Number(n as f32),
                None => CssValue::Auto,
            },
            "column-width" => match style.column_width {
                Dimension::Auto => CssValue::Auto,
                Dimension::Length(px) => CssValue::Length(px, LengthUnit::Px),
                Dimension::Percentage(p) => CssValue::Percentage(p),
            },
            "column-fill" => keyword_from(&style.column_fill),
            "column-span" => keyword_from(&style.column_span),
            "column-rule-width" => CssValue::Length(style.column_rule_width, LengthUnit::Px),
            "column-rule-style" => keyword_from(&style.column_rule_style),
            "column-rule-color" => CssValue::Color(style.column_rule_color),
            _ => CssValue::Initial,
        }
    }

    /// Omit-initial shorthand serialization (CSSOM §6.7.2) for the Multicol
    /// family — first per-family increment under slot `#11-style-shorthand-expand`.
    fn serialize_shorthand(&self, property: &str, longhands: &[(&str, &str)]) -> Option<String> {
        match property {
            // Both are omit-initial `||` shorthands over their (already
            // canonical-ordered) longhands — one body, no per-family branch.
            "column-rule" | "columns" => {
                let initials: Vec<String> = longhands
                    .iter()
                    .map(|(name, _)| self.initial_value(name).to_css_string())
                    .collect();
                let components: Vec<(&str, &str)> = longhands
                    .iter()
                    .zip(&initials)
                    .map(|((_, value), initial)| (*value, initial.as_str()))
                    .collect();
                elidex_plugin::serialize_omit_initial(&components)
            }
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Parse `column-count`: `auto` | positive integer.
fn parse_column_count(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    if input.try_parse(|i| i.expect_ident_matching("auto")).is_ok() {
        return Ok(CssValue::Auto);
    }
    let n = input.expect_integer().map_err(|_| ParseError {
        property: "column-count".into(),
        input: String::new(),
        message: "expected 'auto' or positive integer".into(),
    })?;
    if n < 1 {
        return Err(ParseError {
            property: "column-count".into(),
            input: n.to_string(),
            message: "column-count must be >= 1".into(),
        });
    }
    #[allow(clippy::cast_precision_loss)]
    Ok(CssValue::Number(n as f32))
}

/// Parse `column-width`: `auto` | non-negative length.
fn parse_column_width(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    if input.try_parse(|i| i.expect_ident_matching("auto")).is_ok() {
        return Ok(CssValue::Auto);
    }
    let token = input.next().map_err(|_| ParseError {
        property: "column-width".into(),
        input: String::new(),
        message: "expected 'auto' or length".into(),
    })?;
    match *token {
        cssparser::Token::Dimension {
            value, ref unit, ..
        } if value >= 0.0 => {
            let u = parse_length_unit(unit);
            Ok(CssValue::Length(value, u))
        }
        cssparser::Token::Number { value: 0.0, .. } => Ok(CssValue::Length(0.0, LengthUnit::Px)),
        _ => Err(ParseError {
            property: "column-width".into(),
            input: String::new(),
            message: "expected non-negative length".into(),
        }),
    }
}

/// Parse border-width: `thin` | `medium` | `thick` | non-negative length.
fn parse_border_width(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    if let Ok(ident) = input.try_parse(|i| i.expect_ident().map(|s| s.to_ascii_lowercase())) {
        return match ident.as_str() {
            "thin" => Ok(CssValue::Length(1.0, LengthUnit::Px)),
            "medium" => Ok(CssValue::Length(3.0, LengthUnit::Px)),
            "thick" => Ok(CssValue::Length(5.0, LengthUnit::Px)),
            _ => Err(ParseError {
                property: "column-rule-width".into(),
                input: ident,
                message: "expected border width".into(),
            }),
        };
    }
    let token = input.next().map_err(|_| ParseError {
        property: "column-rule-width".into(),
        input: String::new(),
        message: "expected border width".into(),
    })?;
    match *token {
        cssparser::Token::Dimension {
            value, ref unit, ..
        } if value >= 0.0 => {
            let u = parse_length_unit(unit);
            Ok(CssValue::Length(value, u))
        }
        cssparser::Token::Number { value: 0.0, .. } => Ok(CssValue::Length(0.0, LengthUnit::Px)),
        _ => Err(ParseError {
            property: "column-rule-width".into(),
            input: String::new(),
            message: "expected non-negative length".into(),
        }),
    }
}

/// Resolve a color value, handling `currentcolor` keyword.
fn resolve_rule_color(value: &CssValue, current_color: CssColor) -> CssColor {
    match value {
        CssValue::Keyword(k) if k.eq_ignore_ascii_case("currentcolor") => current_color,
        CssValue::Color(c) => *c,
        _ => current_color,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_prop(name: &str, css: &str) -> Vec<PropertyDeclaration> {
        let handler = MulticolHandler;
        let mut pi = cssparser::ParserInput::new(css);
        let mut parser = cssparser::Parser::new(&mut pi);
        handler.parse(name, &mut parser).unwrap()
    }

    #[test]
    fn parse_column_count_auto() {
        let result = parse_prop("column-count", "auto");
        assert_eq!(result[0].value, CssValue::Auto);
    }

    #[test]
    fn parse_column_count_integer() {
        let result = parse_prop("column-count", "3");
        assert_eq!(result[0].value, CssValue::Number(3.0));
    }

    #[test]
    fn parse_column_count_rejects_zero() {
        let handler = MulticolHandler;
        let mut pi = cssparser::ParserInput::new("0");
        let mut parser = cssparser::Parser::new(&mut pi);
        assert!(handler.parse("column-count", &mut parser).is_err());
    }

    #[test]
    fn parse_column_width_auto() {
        let result = parse_prop("column-width", "auto");
        assert_eq!(result[0].value, CssValue::Auto);
    }

    #[test]
    fn parse_column_width_length() {
        let result = parse_prop("column-width", "200px");
        assert_eq!(result[0].value, CssValue::Length(200.0, LengthUnit::Px));
    }

    #[test]
    fn parse_column_fill() {
        let result = parse_prop("column-fill", "balance");
        assert_eq!(result[0].value, CssValue::Keyword("balance".to_string()));
        let result = parse_prop("column-fill", "auto");
        assert_eq!(result[0].value, CssValue::Keyword("auto".to_string()));
    }

    #[test]
    fn parse_column_span() {
        let result = parse_prop("column-span", "none");
        assert_eq!(result[0].value, CssValue::Keyword("none".to_string()));
        let result = parse_prop("column-span", "all");
        assert_eq!(result[0].value, CssValue::Keyword("all".to_string()));
    }

    #[test]
    fn parse_column_rule_width_keywords() {
        let result = parse_prop("column-rule-width", "thin");
        assert_eq!(result[0].value, CssValue::Length(1.0, LengthUnit::Px));
        let result = parse_prop("column-rule-width", "medium");
        assert_eq!(result[0].value, CssValue::Length(3.0, LengthUnit::Px));
        let result = parse_prop("column-rule-width", "thick");
        assert_eq!(result[0].value, CssValue::Length(5.0, LengthUnit::Px));
    }

    #[test]
    fn parse_column_rule_style() {
        let result = parse_prop("column-rule-style", "solid");
        assert_eq!(result[0].value, CssValue::Keyword("solid".to_string()));
    }

    #[test]
    fn parse_column_rule_color() {
        let result = parse_prop("column-rule-color", "red");
        assert_eq!(
            result[0].value,
            CssValue::Color(CssColor {
                r: 255,
                g: 0,
                b: 0,
                a: 255
            })
        );
    }

    #[test]
    fn resolve_column_count() {
        let handler = MulticolHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        handler.resolve("column-count", &CssValue::Number(4.0), &ctx, &mut style);
        assert_eq!(style.column_count, Some(4));
    }

    #[test]
    fn resolve_column_width() {
        let handler = MulticolHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        handler.resolve(
            "column-width",
            &CssValue::Length(150.0, LengthUnit::Px),
            &ctx,
            &mut style,
        );
        assert_eq!(style.column_width, Dimension::Length(150.0));
    }

    #[test]
    fn resolve_column_fill() {
        let handler = MulticolHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        handler.resolve(
            "column-fill",
            &CssValue::Keyword("auto".into()),
            &ctx,
            &mut style,
        );
        assert_eq!(style.column_fill, ColumnFill::Auto);
    }

    #[test]
    fn resolve_column_span() {
        let handler = MulticolHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        handler.resolve(
            "column-span",
            &CssValue::Keyword("all".into()),
            &ctx,
            &mut style,
        );
        assert_eq!(style.column_span, ColumnSpan::All);
    }

    #[test]
    fn initial_values() {
        let handler = MulticolHandler;
        assert_eq!(handler.initial_value("column-count"), CssValue::Auto);
        assert_eq!(handler.initial_value("column-width"), CssValue::Auto);
        assert_eq!(
            handler.initial_value("column-fill"),
            CssValue::Keyword("balance".into())
        );
        assert_eq!(
            handler.initial_value("column-span"),
            CssValue::Keyword("none".into())
        );
    }

    #[test]
    fn not_inherited() {
        let handler = MulticolHandler;
        for name in handler.property_names() {
            assert!(!handler.is_inherited(name), "{name}");
        }
    }

    #[test]
    fn get_computed_roundtrip() {
        let handler = MulticolHandler;
        let style = ComputedStyle {
            column_count: Some(3),
            column_width: Dimension::Length(200.0),
            column_fill: ColumnFill::Auto,
            column_span: ColumnSpan::All,
            column_rule_width: 2.0,
            column_rule_style: BorderStyle::Solid,
            ..ComputedStyle::default()
        };
        assert_eq!(
            handler.get_computed("column-count", &style),
            CssValue::Number(3.0)
        );
        assert_eq!(
            handler.get_computed("column-width", &style),
            CssValue::Length(200.0, LengthUnit::Px)
        );
        assert_eq!(
            handler.get_computed("column-fill", &style),
            CssValue::Keyword("auto".into())
        );
        assert_eq!(
            handler.get_computed("column-span", &style),
            CssValue::Keyword("all".into())
        );
        assert_eq!(
            handler.get_computed("column-rule-width", &style),
            CssValue::Length(2.0, LengthUnit::Px)
        );
        assert_eq!(
            handler.get_computed("column-rule-style", &style),
            CssValue::Keyword("solid".into())
        );
    }

    // --- serialize_shorthand: §2 corner matrix (omit-initial, CSSOM §6.7.2) ---
    //
    // Each `(name, value)` pair is the longhand as elidex ACTUALLY stores it
    // (border-width keyword → px, named color → #rrggbb; see plan Risks R2), fed
    // in the canonical grammar order the coordinator supplies. Initials come from
    // `MulticolHandler::initial_value` (width=3px, style=none, color=currentcolor;
    // column-width/count=auto), so the arm needs no duplicated initials.

    fn ser(property: &str, longhands: &[(&str, &str)]) -> Option<String> {
        MulticolHandler.serialize_shorthand(property, longhands)
    }

    #[test]
    fn shorthand_column_rule_corner1_style_only() {
        // `column-rule: solid` — width+color initial, keep style.
        let lh = [
            ("column-rule-width", "3px"),
            ("column-rule-style", "solid"),
            ("column-rule-color", "currentcolor"),
        ];
        assert_eq!(ser("column-rule", &lh), Some("solid".to_string()));
    }

    #[test]
    fn shorthand_column_rule_corner2_style_gap() {
        // `column-rule: thick blue` — style=initial in the MIDDLE; keep width
        // then color in order (no re-sort). thick→5px, blue→#0000ff (R2).
        let lh = [
            ("column-rule-width", "5px"),
            ("column-rule-style", "none"),
            ("column-rule-color", "#0000ff"),
        ];
        assert_eq!(ser("column-rule", &lh), Some("5px #0000ff".to_string()));
    }

    #[test]
    fn shorthand_column_rule_corner2b_leading_width_omitted() {
        // `column-rule: dashed green` — width=initial (medium→3px) dropped;
        // style+color survive in order. green→#008000 (R2).
        let lh = [
            ("column-rule-width", "3px"),
            ("column-rule-style", "dashed"),
            ("column-rule-color", "#008000"),
        ];
        assert_eq!(ser("column-rule", &lh), Some("dashed #008000".to_string()));
    }

    #[test]
    fn shorthand_column_rule_corner3_all_initial_keeps_width() {
        // `column-rule: medium none currentcolor` — ALL initial ⇒ keep FIRST
        // (width). `3px` vs Blink's `medium` is the pre-existing R2 divergence.
        let lh = [
            ("column-rule-width", "3px"),
            ("column-rule-style", "none"),
            ("column-rule-color", "currentcolor"),
        ];
        assert_eq!(ser("column-rule", &lh), Some("3px".to_string()));
    }

    #[test]
    fn shorthand_column_rule_corner3b_all_non_initial() {
        // `column-rule: thick solid red` — all kept, canonical order.
        // thick→5px, red→#ff0000 (R2).
        let lh = [
            ("column-rule-width", "5px"),
            ("column-rule-style", "solid"),
            ("column-rule-color", "#ff0000"),
        ];
        assert_eq!(
            ser("column-rule", &lh),
            Some("5px solid #ff0000".to_string())
        );
    }

    #[test]
    fn shorthand_columns_corner4_all_initial_keeps_width() {
        // `columns: auto auto` — both initial ⇒ keep FIRST (width) ⇒ single `auto`.
        let lh = [("column-width", "auto"), ("column-count", "auto")];
        assert_eq!(ser("columns", &lh), Some("auto".to_string()));
    }

    #[test]
    fn shorthand_columns_corner5_width_only() {
        // `columns: 200px` — count=initial (auto) dropped.
        let lh = [("column-width", "200px"), ("column-count", "auto")];
        assert_eq!(ser("columns", &lh), Some("200px".to_string()));
    }

    #[test]
    fn shorthand_columns_corner5b_count_only() {
        // `columns: 3` — width=initial (auto) dropped; Number(3.0)→"3" (R3).
        let lh = [("column-width", "auto"), ("column-count", "3")];
        assert_eq!(ser("columns", &lh), Some("3".to_string()));
    }

    #[test]
    fn shorthand_columns_corner5c_both() {
        // `columns: 200px 3` — both kept, canonical order width→count.
        let lh = [("column-width", "200px"), ("column-count", "3")];
        assert_eq!(ser("columns", &lh), Some("200px 3".to_string()));
    }

    #[test]
    fn shorthand_unknown_property_is_none() {
        // The handler opts IN only to the shorthands it owns; anything else →
        // None (the coordinator maps that to "" — a CSSOM-valid non-serialize).
        let lh = [("margin-top", "1px")];
        assert_eq!(ser("margin", &lh), None);
    }
}
