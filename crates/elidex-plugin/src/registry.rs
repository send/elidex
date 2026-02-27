//! Generic plugin registry with static and dynamic dispatch (Ch.7 §7.3.3).

use std::collections::HashMap;
use std::fmt;

/// A registry that maps names to plugin handlers.
///
/// Supports two lookup paths:
/// - **Static**: keyed by `&'static str` for compile-time-known plugins.
/// - **Dynamic**: keyed by `String` for runtime-registered plugins.
///
/// Static entries are checked first during resolution.
pub struct PluginRegistry<T: Send + Sync + ?Sized> {
    static_lookup: HashMap<&'static str, Box<T>>,
    dynamic_lookup: HashMap<String, Box<T>>,
}

impl<T: Send + Sync + ?Sized> PluginRegistry<T> {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            static_lookup: HashMap::new(),
            dynamic_lookup: HashMap::new(),
        }
    }

    /// Resolve a handler by name.
    ///
    /// Static entries take priority over dynamic entries.
    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<&T> {
        self.static_lookup
            .get(name)
            .or_else(|| self.dynamic_lookup.get(name))
            .map(AsRef::as_ref)
    }

    /// Register a handler with a static (compile-time) name.
    pub fn register_static(&mut self, name: &'static str, handler: Box<T>) {
        self.static_lookup.insert(name, handler);
    }

    /// Register a handler with a dynamic (runtime) name.
    pub fn register_dynamic(&mut self, name: String, handler: Box<T>) {
        self.dynamic_lookup.insert(name, handler);
    }

    /// Returns the number of unique handler names.
    ///
    /// If the same name is registered both statically and dynamically,
    /// it is counted once (static takes priority during resolution).
    pub fn len(&self) -> usize {
        let dynamic_only = self
            .dynamic_lookup
            .keys()
            .filter(|k| !self.static_lookup.contains_key(k.as_str()))
            .count();
        self.static_lookup.len() + dynamic_only
    }

    /// Returns true if no handlers are registered.
    pub fn is_empty(&self) -> bool {
        self.static_lookup.is_empty() && self.dynamic_lookup.is_empty()
    }

    /// Returns an iterator over all unique handler names.
    ///
    /// Static names are yielded first, then dynamic names that do not
    /// overlap with static entries.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.static_lookup.keys().copied().chain(
            self.dynamic_lookup
                .keys()
                .filter(|k| !self.static_lookup.contains_key(k.as_str()))
                .map(String::as_str),
        )
    }
}

impl<T: Send + Sync + ?Sized> fmt::Debug for PluginRegistry<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PluginRegistry")
            .field("static_count", &self.static_lookup.len())
            .field("dynamic_count", &self.dynamic_lookup.len())
            .finish()
    }
}

impl<T: Send + Sync + ?Sized> Default for PluginRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ComputedValue, CssPropertyHandler, CssValue, ParseError, StyleContext};

    struct TestHandler {
        name: &'static str,
    }

    impl CssPropertyHandler for TestHandler {
        fn property_name(&self) -> &str {
            self.name
        }

        fn parse(&self, _value: &str) -> Result<CssValue, ParseError> {
            Ok(CssValue::Keyword("test".into()))
        }

        fn resolve(&self, _value: &CssValue, _context: &StyleContext) -> ComputedValue {
            ComputedValue::Keyword("test".into())
        }
    }

    #[test]
    fn empty_registry() {
        let registry: PluginRegistry<dyn CssPropertyHandler> = PluginRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.resolve("color").is_none());
    }

    #[test]
    fn static_registration_and_resolve() {
        let mut registry: PluginRegistry<dyn CssPropertyHandler> = PluginRegistry::new();
        registry.register_static("color", Box::new(TestHandler { name: "color" }));

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        let handler = registry.resolve("color").unwrap();
        assert_eq!(handler.property_name(), "color");
    }

    #[test]
    fn dynamic_registration_and_resolve() {
        let mut registry: PluginRegistry<dyn CssPropertyHandler> = PluginRegistry::new();
        registry.register_dynamic(
            "background".to_string(),
            Box::new(TestHandler { name: "background" }),
        );

        let handler = registry.resolve("background").unwrap();
        assert_eq!(handler.property_name(), "background");
    }

    #[test]
    fn static_takes_priority_over_dynamic() {
        let mut registry: PluginRegistry<dyn CssPropertyHandler> = PluginRegistry::new();
        registry.register_static(
            "color",
            Box::new(TestHandler {
                name: "static-color",
            }),
        );
        registry.register_dynamic(
            "color".to_string(),
            Box::new(TestHandler {
                name: "dynamic-color",
            }),
        );

        let handler = registry.resolve("color").unwrap();
        assert_eq!(handler.property_name(), "static-color");
    }

    #[test]
    fn unregistered_returns_none() {
        let mut registry: PluginRegistry<dyn CssPropertyHandler> = PluginRegistry::new();
        registry.register_static("color", Box::new(TestHandler { name: "color" }));
        assert!(registry.resolve("font-size").is_none());
    }

    #[test]
    fn reregister_static_overwrites_previous() {
        let mut registry: PluginRegistry<dyn CssPropertyHandler> = PluginRegistry::new();
        registry.register_static("color", Box::new(TestHandler { name: "old-color" }));
        registry.register_static("color", Box::new(TestHandler { name: "new-color" }));

        assert_eq!(registry.len(), 1);
        let handler = registry.resolve("color").unwrap();
        assert_eq!(handler.property_name(), "new-color");
    }

    #[test]
    fn reregister_dynamic_overwrites_previous() {
        let mut registry: PluginRegistry<dyn CssPropertyHandler> = PluginRegistry::new();
        registry.register_dynamic(
            "color".to_string(),
            Box::new(TestHandler { name: "old-color" }),
        );
        registry.register_dynamic(
            "color".to_string(),
            Box::new(TestHandler { name: "new-color" }),
        );

        assert_eq!(registry.len(), 1);
        let handler = registry.resolve("color").unwrap();
        assert_eq!(handler.property_name(), "new-color");
    }

    #[test]
    fn names_iterator() {
        let mut registry: PluginRegistry<dyn CssPropertyHandler> = PluginRegistry::new();
        registry.register_static("color", Box::new(TestHandler { name: "color" }));
        registry.register_dynamic(
            "background".to_string(),
            Box::new(TestHandler { name: "background" }),
        );

        let mut names: Vec<&str> = registry.names().collect();
        names.sort_unstable();
        assert_eq!(names, vec!["background", "color"]);
    }

    #[test]
    fn shadowed_dynamic_excluded_from_names() {
        let mut registry: PluginRegistry<dyn CssPropertyHandler> = PluginRegistry::new();
        registry.register_static("color", Box::new(TestHandler { name: "static" }));
        registry.register_dynamic(
            "color".to_string(),
            Box::new(TestHandler { name: "dynamic" }),
        );
        registry.register_dynamic(
            "background".to_string(),
            Box::new(TestHandler { name: "background" }),
        );

        // Shadowed dynamic "color" should not appear twice.
        assert_eq!(registry.len(), 2);
        let names: Vec<&str> = registry.names().collect();
        assert_eq!(names.iter().filter(|n| **n == "color").count(), 1);
        assert!(names.contains(&"background"));
    }
}
