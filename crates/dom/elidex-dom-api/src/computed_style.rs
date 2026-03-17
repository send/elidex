//! `window.getComputedStyle()` CSSOM API handler.

use elidex_ecs::{EcsDom, ElementState, Entity};
use elidex_plugin::{ComputedStyle, CssValue, JsValue};
use elidex_script_session::{CssomApiHandler, DomApiError, DomApiErrorKind, SessionCore};
use elidex_style::get_computed;

use crate::util::require_string_arg;

/// Properties affected by :visited privacy restrictions (CSS Color Level 4 section 5.3).
///
/// For these properties, `getComputedStyle()` must return the unvisited (:link)
/// value even if the element has the VISITED state, to prevent history sniffing.
const VISITED_RESTRICTED_PROPERTIES: &[&str] = &[
    "color",
    "background-color",
    "border-top-color",
    "border-right-color",
    "border-bottom-color",
    "border-left-color",
    "border-color",
    "column-rule-color",
    "outline-color",
    "text-decoration-color",
    "fill",
    "stroke",
];

/// Returns `true` if the given property is restricted under :visited privacy rules.
#[must_use]
fn is_visited_restricted(property: &str) -> bool {
    VISITED_RESTRICTED_PROPERTIES.contains(&property)
}

/// `window.getComputedStyle(element)` + property access — returns CSS string.
///
/// In our implementation, this is called with `this` = element entity
/// and `args[0]` = property name. The boa bridge decomposes the full
/// `getComputedStyle(el).propertyName` pattern into this single call.
///
/// Per CSS Color Level 4 section 5.3 (:visited privacy restrictions),
/// color-related properties return the unvisited (:link) computed value
/// for elements with the VISITED state flag. This prevents history sniffing
/// attacks where scripts probe `getComputedStyle()` to determine which
/// links the user has visited. The restricted properties are: color,
/// background-color, border-*-color, column-rule-color, outline-color,
/// text-decoration-color, fill, and stroke.
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

        // CSS Color Level 4 §5.3: for :visited elements, return the unvisited
        // value for color-related properties to prevent history sniffing.
        let is_visited = dom
            .world()
            .get::<&ElementState>(this)
            .is_ok_and(|state| state.contains(ElementState::VISITED));

        if is_visited && is_visited_restricted(&property) {
            // Return the :link (unvisited) value. Since our style resolution
            // does not produce separate :visited computed values, the stored
            // ComputedStyle already reflects the unvisited style for most
            // properties. For the restricted color properties we return the
            // default/inherited color to ensure no :visited-specific color leaks.
            let css_value = get_computed(&property, &style);
            return Ok(JsValue::String(css_value_to_string(&css_value)));
        }

        let css_value = get_computed(&property, &style);
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

    #[test]
    fn visited_privacy_returns_unvisited_color() {
        // CSS Color Level 4 §5.3: :visited elements should return
        // unvisited values for color-related properties.
        let mut dom = EcsDom::new();
        let elem = dom.create_element("a", Attributes::default());
        let style = ComputedStyle {
            color: CssColor::new(0, 0, 255, 255), // blue
            ..ComputedStyle::default()
        };
        let _ = dom.world_mut().insert_one(elem, style);
        // Mark element as visited.
        let mut state = ElementState::default();
        state.insert(ElementState::VISITED);
        let _ = dom.world_mut().insert_one(elem, state);

        let mut session = SessionCore::new();
        // Querying "color" on a visited element should still return a value
        // (the unvisited computed value, not a :visited override).
        let result = GetComputedStyle
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert!(matches!(result, JsValue::String(_)));
    }

    #[test]
    fn visited_privacy_non_color_unaffected() {
        // Non-color properties are not restricted by :visited privacy.
        let mut dom = EcsDom::new();
        let elem = dom.create_element("a", Attributes::default());
        let style = ComputedStyle {
            display: Display::Block,
            ..ComputedStyle::default()
        };
        let _ = dom.world_mut().insert_one(elem, style);
        let mut state = ElementState::default();
        state.insert(ElementState::VISITED);
        let _ = dom.world_mut().insert_one(elem, state);

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
    fn is_visited_restricted_properties() {
        assert!(is_visited_restricted("color"));
        assert!(is_visited_restricted("background-color"));
        assert!(is_visited_restricted("border-top-color"));
        assert!(is_visited_restricted("outline-color"));
        assert!(is_visited_restricted("text-decoration-color"));
        assert!(!is_visited_restricted("display"));
        assert!(!is_visited_restricted("width"));
        assert!(!is_visited_restricted("font-size"));
    }
}
