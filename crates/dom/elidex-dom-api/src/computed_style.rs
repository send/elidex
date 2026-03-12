//! `window.getComputedStyle()` CSSOM API handler.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{ComputedStyle, CssValue, JsValue};
use elidex_script_session::{CssomApiHandler, DomApiError, DomApiErrorKind, SessionCore};
use elidex_style::get_computed_as_css_value;

use crate::util::require_string_arg;

/// `window.getComputedStyle(element)` + property access — returns CSS string.
///
/// In our implementation, this is called with `this` = element entity
/// and `args[0]` = property name. The boa bridge decomposes the full
/// `getComputedStyle(el).propertyName` pattern into this single call.
/// TODO(Phase 4): Per CSS Color Level 4 §5.3 :visited privacy restrictions,
/// `getComputedStyle()` should return the :link style (not :visited) for
/// properties like color, background-color, border-*-color, column-rule-color,
/// outline-color, and text-decoration-color, to prevent history sniffing.
pub struct GetComputedStyle;

impl CssomApiHandler for GetComputedStyle {
    fn method_name(&self) -> &str {
        "getComputedStyle"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let property = require_string_arg(args, 0)?;
        let style = dom
            .world()
            .get::<&ComputedStyle>(this)
            .map_err(|_| DomApiError {
                kind: DomApiErrorKind::NotFoundError,
                message: "element has no computed style".into(),
            })?;
        let css_value = get_computed_as_css_value(&property, &style);
        Ok(JsValue::String(css_value_to_string(&css_value)))
    }
}

/// Convert a `CssValue` to its CSS string representation.
fn css_value_to_string(value: &CssValue) -> String {
    match value {
        CssValue::Keyword(s) | CssValue::String(s) | CssValue::RawTokens(s) => s.clone(),
        CssValue::Length(n, unit) => {
            let unit_str = match unit {
                elidex_plugin::LengthUnit::Px => "px",
                elidex_plugin::LengthUnit::Em => "em",
                elidex_plugin::LengthUnit::Rem => "rem",
                elidex_plugin::LengthUnit::Vw => "vw",
                elidex_plugin::LengthUnit::Vh => "vh",
                elidex_plugin::LengthUnit::Vmin => "vmin",
                elidex_plugin::LengthUnit::Vmax => "vmax",
                _ => {
                    debug_assert!(false, "unhandled LengthUnit variant: {unit:?}");
                    ""
                }
            };
            format!("{n}{unit_str}")
        }
        CssValue::Color(c) => c.to_string(),
        CssValue::Number(n) => format!("{n}"),
        CssValue::Percentage(n) => format!("{n}%"),
        CssValue::Auto => "auto".into(),
        CssValue::Initial => "initial".into(),
        CssValue::Inherit => "inherit".into(),
        CssValue::Unset => "unset".into(),
        CssValue::List(items) => items
            .iter()
            .map(css_value_to_string)
            .collect::<Vec<_>>()
            .join(", "),
        CssValue::Var(name, fallback) => match fallback {
            Some(fb) => format!("var({name}, {})", css_value_to_string(fb)),
            None => format!("var({name})"),
        },
        _ => {
            debug_assert!(false, "unhandled CssValue variant: {value:?}");
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;
    use elidex_plugin::{CssColor, Display};

    #[test]
    fn get_computed_display() {
        let mut dom = EcsDom::new();
        let elem = dom.create_element("div", Attributes::default());
        let style = ComputedStyle {
            display: Display::Block,
            ..ComputedStyle::default()
        };
        let _ = dom.world_mut().insert_one(elem, style);

        let mut session = SessionCore::new();
        let result = GetComputedStyle
            .invoke(
                elem,
                &[JsValue::String("display".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("block".into()));
    }

    #[test]
    fn get_computed_color() {
        let mut dom = EcsDom::new();
        let elem = dom.create_element("div", Attributes::default());
        let style = ComputedStyle {
            color: CssColor::RED,
            ..ComputedStyle::default()
        };
        let _ = dom.world_mut().insert_one(elem, style);

        let mut session = SessionCore::new();
        let result = GetComputedStyle
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        // CssColor::RED display format.
        assert!(matches!(result, JsValue::String(_)));
    }

    #[test]
    fn css_value_to_string_raw_tokens() {
        let val = CssValue::RawTokens("#0d1117".into());
        assert_eq!(css_value_to_string(&val), "#0d1117");
    }

    #[test]
    fn css_value_to_string_var() {
        let val = CssValue::Var("--bg".into(), None);
        assert_eq!(css_value_to_string(&val), "var(--bg)");

        let val_fb = CssValue::Var(
            "--bg".into(),
            Some(Box::new(CssValue::Keyword("red".into()))),
        );
        assert_eq!(css_value_to_string(&val_fb), "var(--bg, red)");
    }

    #[test]
    fn get_computed_custom_property() {
        let mut dom = EcsDom::new();
        let elem = dom.create_element("div", Attributes::default());
        let mut style = ComputedStyle::default();
        style
            .custom_properties
            .insert("--bg".into(), "#0d1117".into());
        let _ = dom.world_mut().insert_one(elem, style);

        let mut session = SessionCore::new();
        let result = GetComputedStyle
            .invoke(
                elem,
                &[JsValue::String("--bg".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("#0d1117".into()));
    }

    #[test]
    fn no_computed_style_errors() {
        let mut dom = EcsDom::new();
        let elem = dom.create_element("div", Attributes::default());
        let mut session = SessionCore::new();
        let result = GetComputedStyle.invoke(
            elem,
            &[JsValue::String("display".into())],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
    }
}
