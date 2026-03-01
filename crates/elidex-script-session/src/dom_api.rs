//! DOM API handler trait for script-engine method dispatch.

define_api_handler!(
    /// Handler for a single DOM API method.
    ///
    /// Implementations of this trait register with a [`PluginRegistry`] and are
    /// dispatched by the script engine when JS code calls a DOM method.
    ///
    /// [`PluginRegistry`]: elidex_plugin::PluginRegistry
    DomApiHandler,
    elidex_plugin::DomSpecLevel,
    elidex_plugin::DomSpecLevel::Living
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionCore;
    use crate::types::DomApiError;
    use elidex_ecs::{Attributes, EcsDom, Entity};
    use elidex_plugin::{DomSpecLevel, JsValue, PluginRegistry};

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
                .map_err(|_| DomApiError::not_found("entity has no tag"))?;
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
