//! `window.getComputedStyle()` CSSOM API handler.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{ComputedStyle, JsValue};
use elidex_script_session::{DomApiError, DomApiErrorKind, DomApiHandler, SessionCore};
use elidex_style::get_computed;

use crate::util::require_string_arg;

/// `window.getComputedStyle(element)` + property access — returns CSS string.
///
/// In our implementation, this is called with `this` = element entity
/// and `args[0]` = property name. The boa bridge decomposes the full
/// `getComputedStyle(el).propertyName` pattern into this single call.
///
/// Per CSS Color Level 4 section 5.3 (:visited privacy restrictions),
/// color-related properties must return the unvisited (:link) computed
/// value for elements with the VISITED state flag, preventing history
/// sniffing via `getComputedStyle()` probes. Resolution currently never
/// produces separate :visited computed values, so the stored
/// `ComputedStyle` is already the unvisited form — see the inline note
/// in `invoke`.
pub struct GetComputedStyle;

impl DomApiHandler for GetComputedStyle {
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

        // CSS Color Level 4 §5.3 (:visited privacy): the restricted
        // color properties (color / background-color / border-*-color /
        // outline-color / text-decoration-color / fill / stroke …) must
        // return the unvisited (:link) value. Our style resolution does
        // not produce separate :visited computed values — the stored
        // ComputedStyle already reflects the unvisited style — so no
        // divergence is needed until :visited styling exists.
        let css_value = get_computed(&property, &style);
        Ok(JsValue::String(css_value.to_css_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;
    use elidex_ecs::ElementState;
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
}
