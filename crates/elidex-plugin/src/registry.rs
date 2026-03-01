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
    #[must_use]
    pub fn len(&self) -> usize {
        let dynamic_only = self
            .dynamic_lookup
            .keys()
            .filter(|k| !self.static_lookup.contains_key(k.as_str()))
            .count();
        self.static_lookup.len() + dynamic_only
    }

    /// Returns true if no handlers are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.static_lookup.is_empty() && self.dynamic_lookup.is_empty()
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
    use crate::{HttpRequest, HttpResponse, NetworkError, NetworkMiddleware};

    struct TestMiddleware {
        mw_name: &'static str,
    }

    impl NetworkMiddleware for TestMiddleware {
        fn name(&self) -> &str {
            self.mw_name
        }

        fn on_request(&self, _request: &mut HttpRequest) -> Result<(), NetworkError> {
            Ok(())
        }

        fn on_response(&self, _response: &mut HttpResponse) -> Result<(), NetworkError> {
            Ok(())
        }
    }

    #[test]
    fn empty_registry() {
        let registry: PluginRegistry<dyn NetworkMiddleware> = PluginRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.resolve("cors").is_none());
    }

    #[test]
    fn static_registration_and_resolve() {
        let mut registry: PluginRegistry<dyn NetworkMiddleware> = PluginRegistry::new();
        registry.register_static("cors", Box::new(TestMiddleware { mw_name: "cors" }));

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        let handler = registry.resolve("cors").unwrap();
        assert_eq!(handler.name(), "cors");
    }

    #[test]
    fn dynamic_registration_and_resolve() {
        let mut registry: PluginRegistry<dyn NetworkMiddleware> = PluginRegistry::new();
        registry.register_dynamic(
            "auth".to_string(),
            Box::new(TestMiddleware { mw_name: "auth" }),
        );

        let handler = registry.resolve("auth").unwrap();
        assert_eq!(handler.name(), "auth");
    }

    #[test]
    fn static_takes_priority_over_dynamic() {
        let mut registry: PluginRegistry<dyn NetworkMiddleware> = PluginRegistry::new();
        registry.register_static(
            "cors",
            Box::new(TestMiddleware {
                mw_name: "static-cors",
            }),
        );
        registry.register_dynamic(
            "cors".to_string(),
            Box::new(TestMiddleware {
                mw_name: "dynamic-cors",
            }),
        );

        let handler = registry.resolve("cors").unwrap();
        assert_eq!(handler.name(), "static-cors");
    }

    #[test]
    fn unregistered_returns_none() {
        let mut registry: PluginRegistry<dyn NetworkMiddleware> = PluginRegistry::new();
        registry.register_static("cors", Box::new(TestMiddleware { mw_name: "cors" }));
        assert!(registry.resolve("auth").is_none());
    }

    #[test]
    fn reregister_static_overwrites_previous() {
        let mut registry: PluginRegistry<dyn NetworkMiddleware> = PluginRegistry::new();
        registry.register_static(
            "cors",
            Box::new(TestMiddleware {
                mw_name: "old-cors",
            }),
        );
        registry.register_static(
            "cors",
            Box::new(TestMiddleware {
                mw_name: "new-cors",
            }),
        );

        assert_eq!(registry.len(), 1);
        let handler = registry.resolve("cors").unwrap();
        assert_eq!(handler.name(), "new-cors");
    }

    #[test]
    fn reregister_dynamic_overwrites_previous() {
        let mut registry: PluginRegistry<dyn NetworkMiddleware> = PluginRegistry::new();
        registry.register_dynamic(
            "cors".to_string(),
            Box::new(TestMiddleware {
                mw_name: "old-cors",
            }),
        );
        registry.register_dynamic(
            "cors".to_string(),
            Box::new(TestMiddleware {
                mw_name: "new-cors",
            }),
        );

        assert_eq!(registry.len(), 1);
        let handler = registry.resolve("cors").unwrap();
        assert_eq!(handler.name(), "new-cors");
    }

    #[test]
    fn shadowed_dynamic_counted_once() {
        let mut registry: PluginRegistry<dyn NetworkMiddleware> = PluginRegistry::new();
        registry.register_static("cors", Box::new(TestMiddleware { mw_name: "static" }));
        registry.register_dynamic(
            "cors".to_string(),
            Box::new(TestMiddleware { mw_name: "dynamic" }),
        );
        registry.register_dynamic(
            "auth".to_string(),
            Box::new(TestMiddleware { mw_name: "auth" }),
        );

        // Shadowed dynamic "cors" should not be double-counted.
        assert_eq!(registry.len(), 2);
    }
}
