//! DOM API handler trait for script-engine method dispatch.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{DomSpecLevel, JsValue};

use crate::session::SessionCore;
use crate::types::DomApiError;

/// Handler for a single DOM API method.
///
/// Implementations of this trait register with a [`PluginRegistry`] and are
/// dispatched by the script engine when JS code calls a DOM method.
///
/// [`PluginRegistry`]: elidex_plugin::PluginRegistry
pub trait DomApiHandler: Send + Sync {
    /// Returns the DOM method name (e.g. `"appendChild"`, `"setAttribute"`).
    fn method_name(&self) -> &str;

    /// Returns the specification level of this DOM API method.
    fn spec_level(&self) -> DomSpecLevel {
        DomSpecLevel::Living
    }

    /// Invoke the DOM method.
    ///
    /// # Parameters
    ///
    /// - `this` — The entity on which the method is called.
    /// - `args` — JS arguments passed to the method.
    /// - `session` — The session core for identity mapping and mutation recording.
    /// - `dom` — The ECS DOM for direct reads and entity creation.
    ///
    /// # Errors
    ///
    /// Returns `DomApiError` if the operation fails (e.g. wrong argument types,
    /// node not found, hierarchy constraint violation).
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DomApiErrorKind;
    use elidex_ecs::Attributes;
    use elidex_plugin::PluginRegistry;

    struct MockGetTagName;

    impl DomApiHandler for MockGetTagName {
        fn method_name(&self) -> &'static str {
            "tagName"
        }

        fn invoke(
            &self,
            this: Entity,
            _args: &[JsValue],
            _session: &mut SessionCore,
            dom: &mut EcsDom,
        ) -> Result<JsValue, DomApiError> {
            let tag = dom
                .world()
                .get::<&elidex_ecs::TagType>(this)
                .map_err(|_| DomApiError {
                    kind: DomApiErrorKind::NotFoundError,
                    message: "entity has no tag".into(),
                })?;
            Ok(JsValue::String(tag.0.to_uppercase()))
        }
    }

    #[test]
    fn mock_handler_invoke() {
        let mut dom = EcsDom::new();
        let div = dom.create_element("div", Attributes::default());
        let mut session = SessionCore::new();

        let handler = MockGetTagName;
        let result = handler.invoke(div, &[], &mut session, &mut dom);
        assert_eq!(result.unwrap(), JsValue::String("DIV".into()));
    }

    #[test]
    fn mock_handler_in_registry() {
        let mut registry: PluginRegistry<dyn DomApiHandler> = PluginRegistry::new();
        registry.register_static("tagName", Box::new(MockGetTagName));

        let handler = registry.resolve("tagName").unwrap();
        assert_eq!(handler.method_name(), "tagName");
    }

    #[test]
    fn default_spec_level() {
        let handler = MockGetTagName;
        assert_eq!(handler.spec_level(), DomSpecLevel::Living);
    }
}
