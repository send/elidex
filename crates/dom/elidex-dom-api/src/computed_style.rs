//! `window.getComputedStyle()` CSSOM API handler.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{ComputedStyle, JsValue};
use elidex_script_session::{DomApiError, DomApiErrorKind, DomApiHandler, SessionCore};
use elidex_style::serialize_resolved_value;

use crate::util::require_string_arg;

/// `window.getComputedStyle(element)` + property access — returns CSS string.
///
/// In our implementation, this is called with `this` = element entity
/// and `args[0]` = property name. The boa bridge decomposes the full
/// `getComputedStyle(el).propertyName` pattern into this single call.
///
/// Per Selectors Level 4 §8.2 "The Link History Pseudo-classes" (:visited
/// privacy restrictions),
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

        // Selectors 4 §8.2 (:link/:visited privacy): the restricted
        // color properties (color / background-color / border-*-color /
        // outline-color / text-decoration-color / fill / stroke …) must
        // return the unvisited (:link) value. Our style resolution does
        // not produce separate :visited computed values — the stored
        // ComputedStyle already reflects the unvisited style — so no
        // divergence is needed until :visited styling exists.
        // CSSOM-1 §9: a color longhand's resolved value is its used value,
        // serialized in the `rgb()`/`rgba()` form (CSS Color 4 §16.2.2) — NOT
        // the declared `#rrggbb` form. `serialize_resolved_value` applies that
        // resolved-value transform; non-color properties are unchanged.
        Ok(JsValue::String(serialize_resolved_value(&property, &style)))
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
        // CSSOM resolved value = used value, serialized as rgb() (CSS Color 4
        // §16.2.2) — NOT the declared #rrggbb form.
        assert_eq!(result, JsValue::String("rgb(255, 0, 0)".into()));
    }

    #[test]
    fn get_computed_color_translucent() {
        let mut dom = EcsDom::new();
        let elem = dom.create_element("div", Attributes::default());
        let style = ComputedStyle {
            // 50% alpha (u8 128) → "0.5" per CSS Color 4 §16.1.
            background_color: CssColor::new(0, 0, 0, 128),
            ..ComputedStyle::default()
        };
        let _ = dom.world_mut().insert_one(elem, style);

        let mut session = SessionCore::new();
        let result = GetComputedStyle
            .invoke(
                elem,
                &[JsValue::String("background-color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("rgba(0, 0, 0, 0.5)".into()));
    }

    #[test]
    fn get_computed_text_decoration_color_currentcolor() {
        // text-decoration-color default (None = currentcolor) resolves to the
        // element's own `color` used value (CSSOM-1 §9).
        let mut dom = EcsDom::new();
        let elem = dom.create_element("div", Attributes::default());
        let style = ComputedStyle {
            color: CssColor::BLUE,
            // text_decoration_color stays None (default) = currentcolor.
            ..ComputedStyle::default()
        };
        let _ = dom.world_mut().insert_one(elem, style);

        let mut session = SessionCore::new();
        let result = GetComputedStyle
            .invoke(
                elem,
                &[JsValue::String("text-decoration-color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("rgb(0, 0, 255)".into()));
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
        // Selectors 4 §8.2: :visited elements should return
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
