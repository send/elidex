//! element.style DOM API handlers: setProperty, getPropertyValue, removeProperty.

use elidex_ecs::{EcsDom, Entity, InlineStyle};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiError, DomApiErrorKind, DomApiHandler, SessionCore};

use crate::util::require_string_arg;

// ---------------------------------------------------------------------------
// style.setProperty
// ---------------------------------------------------------------------------

/// `element.style.setProperty(property, value)` — sets an inline style.
pub struct StyleSetProperty;

impl DomApiHandler for StyleSetProperty {
    fn method_name(&self) -> &str {
        "style.setProperty"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let property = require_string_arg(args, 0)?;
        let value = require_string_arg(args, 1)?;

        // Insert InlineStyle component if missing.
        if dom.world_mut().get::<&InlineStyle>(this).is_err() {
            let _ = dom.world_mut().insert_one(this, InlineStyle::default());
        }

        let mut style = dom
            .world_mut()
            .get::<&mut InlineStyle>(this)
            .map_err(|_| DomApiError {
                kind: DomApiErrorKind::NotFoundError,
                message: "element not found".into(),
            })?;
        style.set(property, value);
        Ok(JsValue::Undefined)
    }
}

// ---------------------------------------------------------------------------
// style.getPropertyValue
// ---------------------------------------------------------------------------

/// `element.style.getPropertyValue(property)` — gets an inline style value.
pub struct StyleGetPropertyValue;

impl DomApiHandler for StyleGetPropertyValue {
    fn method_name(&self) -> &str {
        "style.getPropertyValue"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let property = require_string_arg(args, 0)?;
        match dom.world().get::<&InlineStyle>(this) {
            Ok(style) => match style.get(&property) {
                Some(val) => Ok(JsValue::String(val.to_string())),
                None => Ok(JsValue::String(String::new())),
            },
            Err(_) => Ok(JsValue::String(String::new())),
        }
    }
}

// ---------------------------------------------------------------------------
// style.removeProperty
// ---------------------------------------------------------------------------

/// `element.style.removeProperty(property)` — removes an inline style.
pub struct StyleRemoveProperty;

impl DomApiHandler for StyleRemoveProperty {
    fn method_name(&self) -> &str {
        "style.removeProperty"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let property = require_string_arg(args, 0)?;
        match dom.world_mut().get::<&mut InlineStyle>(this) {
            Ok(mut style) => {
                let old_value = style.remove(&property).unwrap_or_default();
                Ok(JsValue::String(old_value))
            }
            Err(_) => Ok(JsValue::String(String::new())),
        }
    }
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    fn setup() -> (EcsDom, Entity, SessionCore) {
        let mut dom = EcsDom::new();
        let elem = dom.create_element("div", Attributes::default());
        let session = SessionCore::new();
        (dom, elem, session)
    }

    #[test]
    fn set_and_get_property() {
        let (mut dom, elem, mut session) = setup();
        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("color".into()),
                    JsValue::String("red".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let result = StyleGetPropertyValue
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("red".into()));
    }

    #[test]
    fn get_nonexistent_property() {
        let (mut dom, elem, mut session) = setup();
        let result = StyleGetPropertyValue
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String(String::new()));
    }

    #[test]
    fn remove_property() {
        let (mut dom, elem, mut session) = setup();
        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("color".into()),
                    JsValue::String("blue".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();

        let result = StyleRemoveProperty
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("blue".into()));

        // Verify it's gone.
        let result = StyleGetPropertyValue
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String(String::new()));
    }

    #[test]
    fn auto_creates_inline_style_component() {
        let (mut dom, elem, mut session) = setup();
        // No InlineStyle component initially.
        assert!(dom.world().get::<&InlineStyle>(elem).is_err());

        StyleSetProperty
            .invoke(
                elem,
                &[
                    JsValue::String("display".into()),
                    JsValue::String("none".into()),
                ],
                &mut session,
                &mut dom,
            )
            .unwrap();

        // InlineStyle component should now exist.
        assert!(dom.world().get::<&InlineStyle>(elem).is_ok());
    }
}
