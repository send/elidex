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
        let elem = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        let elem = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        let elem = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        let elem = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        use elidex_plugin::component_values::{ComponentValues, ComputedCustom};
        let mut dom = EcsDom::new();
        let elem = dom.create_element_ownerless_fixture("div", Attributes::default());
        let mut style = ComputedStyle::default();
        for (name, value) in [
            (
                "--bg",
                ComputedCustom::Value(ComponentValues::capture("#0d1117").unwrap()),
            ),
            (
                "--empty",
                ComputedCustom::Value(ComponentValues::capture("").unwrap()),
            ),
            ("--bad", ComputedCustom::Invalid),
        ] {
            style.custom_properties.insert(name.into(), value);
        }
        let _ = dom.world_mut().insert_one(elem, style);
        let mut session = SessionCore::new();
        for (name, expected) in [("--bg", "#0d1117"), ("--empty", " "), ("--bad", "")] {
            let result = GetComputedStyle
                .invoke(
                    elem,
                    &[JsValue::String(name.into())],
                    &mut session,
                    &mut dom,
                )
                .unwrap();
            assert_eq!(result, JsValue::String(expected.into()));
        }
    }

    #[test]
    fn computed_custom_copy_through_public_cssom_and_style_resolution() {
        use crate::style::{StyleGetPropertyValue, StyleSetProperty};
        use elidex_plugin::component_values::{ComponentValues, ComputedCustom};
        use elidex_plugin::Size;
        for (authored, pending, expected, width) in [
            ("2", "var(--x)px", Some("2/**/px"), "auto"),
            ("2px", "var(--x)", Some("2px"), "2px"),
            ("", "var(--x)", Some(""), "auto"),
            ("initial", "var(--x)", None, "7px"),
            ("<", "f(var(--x)!--)", Some("f(</**/!--)"), "auto"),
            ("<!--", "f(var(--x))", Some("f(<!--)"), "auto"),
            (r"\61", "var(--x) b", Some(r"\61/**/ b"), "auto"),
            ("a/*x*", "var(--x), serif", Some("a/*x**/, serif"), "auto"),
            (r"\61 ", "var(--x) b", Some(r"\61  b"), "auto"),
            ("\"a", "var(--x), serif", Some("\"a\", serif"), "auto"),
            ("a/*b*/", "var(--x)*", Some("a/*b*/*"), "auto"),
        ] {
            let mut dom = EcsDom::new();
            let root = dom.create_document_root();
            let parent = dom.create_element_ownerless_fixture("html", Attributes::default());
            let child = dom.create_element_ownerless_fixture("div", Attributes::default());
            let target = dom.create_element_ownerless_fixture("div", Attributes::default());
            assert!(dom.append_child(root, parent));
            assert!(dom.append_child(parent, child));
            assert!(dom.append_child(parent, target));
            dom.set_attribute(parent, "style", &format!("--x:{authored}"));
            dom.set_attribute(child, "style", &format!("--m:{pending}"));
            dom.set_attribute(target, "style", "--copy:old; width:var(--copy,7px)");
            let viewport = Size::new(800., 600.);
            elidex_style::resolve_styles(&mut dom, &[], viewport);
            let mut session = SessionCore::new();
            let property = [JsValue::String("--m".into())];
            assert_eq!(
                StyleGetPropertyValue
                    .invoke(child, &property, &mut session, &mut dom)
                    .unwrap(),
                JsValue::String(pending.into())
            );
            let value = GetComputedStyle
                .invoke(child, &property, &mut session, &mut dom)
                .unwrap();
            let readback = match expected {
                Some("") => " ",
                Some(css) => css,
                None => "",
            };
            assert_eq!(value, JsValue::String(readback.into()), "{pending}");
            StyleSetProperty
                .invoke(
                    target,
                    &[JsValue::String("--copy".into()), value],
                    &mut session,
                    &mut dom,
                )
                .unwrap();
            elidex_style::resolve_styles(&mut dom, &[], viewport);
            let computed = dom.world().get::<&ComputedStyle>(target).unwrap();
            let copy =
                expected.map(|css| ComputedCustom::Value(ComponentValues::capture(css).unwrap()));
            assert_eq!(
                computed.custom_properties.get("--copy"),
                copy.as_ref(),
                "{pending}"
            );
            assert_eq!(
                serialize_resolved_value("width", &computed),
                width,
                "{pending}"
            );
        }
    }

    #[test]
    fn no_computed_style_errors() {
        let mut dom = EcsDom::new();
        let elem = dom.create_element_ownerless_fixture("div", Attributes::default());
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
        let elem = dom.create_element_ownerless_fixture("a", Attributes::default());
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
        let elem = dom.create_element_ownerless_fixture("a", Attributes::default());
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
