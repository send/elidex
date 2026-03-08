//! Pre-populated registries for standard DOM/CSSOM API handlers.
//!
//! These factory functions create registries that contain all built-in handlers,
//! enabling engine-agnostic dispatch by name rather than direct handler references.

use elidex_plugin::PluginRegistry;
use elidex_script_session::{CssomApiHandler, DomApiHandler};

/// Type alias for a registry of DOM API handlers.
pub type DomHandlerRegistry = PluginRegistry<dyn DomApiHandler>;

/// Type alias for a registry of CSSOM API handlers.
pub type CssomHandlerRegistry = PluginRegistry<dyn CssomApiHandler>;

/// Create a registry pre-populated with all standard DOM API handlers.
#[must_use]
pub fn create_dom_registry() -> DomHandlerRegistry {
    let mut r: DomHandlerRegistry = PluginRegistry::new();
    // Document
    r.register_static("querySelector", Box::new(super::QuerySelector));
    r.register_static("getElementById", Box::new(super::GetElementById));
    r.register_static("createElement", Box::new(super::CreateElement));
    r.register_static("createTextNode", Box::new(super::CreateTextNode));
    // Element — child mutations
    r.register_static("appendChild", Box::new(super::AppendChild));
    r.register_static("insertBefore", Box::new(super::InsertBefore));
    r.register_static("removeChild", Box::new(super::RemoveChild));
    // Element — attributes
    r.register_static("getAttribute", Box::new(super::GetAttribute));
    r.register_static("setAttribute", Box::new(super::SetAttribute));
    r.register_static("removeAttribute", Box::new(super::RemoveAttribute));
    // Element — content
    r.register_static("textContent.get", Box::new(super::GetTextContent));
    r.register_static("textContent.set", Box::new(super::SetTextContent));
    r.register_static("innerHTML.get", Box::new(super::GetInnerHtml));
    // Style
    r.register_static("style.setProperty", Box::new(super::StyleSetProperty));
    r.register_static(
        "style.getPropertyValue",
        Box::new(super::StyleGetPropertyValue),
    );
    r.register_static(
        "style.removeProperty",
        Box::new(super::StyleRemoveProperty),
    );
    // ClassList
    r.register_static("classList.add", Box::new(super::ClassListAdd));
    r.register_static("classList.remove", Box::new(super::ClassListRemove));
    r.register_static("classList.toggle", Box::new(super::ClassListToggle));
    r.register_static("classList.contains", Box::new(super::ClassListContains));
    r
}

/// Create a registry pre-populated with all standard CSSOM handlers.
#[must_use]
pub fn create_cssom_registry() -> CssomHandlerRegistry {
    let mut r: CssomHandlerRegistry = PluginRegistry::new();
    r.register_static("getComputedStyle", Box::new(super::GetComputedStyle));
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_script_session::{CssomApiHandler, DomApiHandler};

    const EXPECTED_DOM_HANDLERS: [&str; 20] = [
        "querySelector",
        "getElementById",
        "createElement",
        "createTextNode",
        "appendChild",
        "insertBefore",
        "removeChild",
        "getAttribute",
        "setAttribute",
        "removeAttribute",
        "textContent.get",
        "textContent.set",
        "innerHTML.get",
        "style.setProperty",
        "style.getPropertyValue",
        "style.removeProperty",
        "classList.add",
        "classList.remove",
        "classList.toggle",
        "classList.contains",
    ];

    #[test]
    fn dom_registry_has_all_handlers() {
        let registry = create_dom_registry();
        assert_eq!(registry.len(), EXPECTED_DOM_HANDLERS.len());
        for name in EXPECTED_DOM_HANDLERS {
            assert!(
                registry.resolve(name).is_some(),
                "handler '{name}' not found in DOM registry"
            );
        }
    }

    #[test]
    fn dom_registry_method_names_match_keys() {
        let registry = create_dom_registry();
        for name in EXPECTED_DOM_HANDLERS {
            let handler = registry.resolve(name).unwrap();
            assert_eq!(
                handler.method_name(),
                name,
                "method_name() mismatch for '{name}'"
            );
        }
    }

    #[test]
    fn cssom_registry_has_get_computed_style() {
        let registry = create_cssom_registry();
        assert_eq!(registry.len(), 1);
        let handler = registry.resolve("getComputedStyle").unwrap();
        assert_eq!(handler.method_name(), "getComputedStyle");
    }

    #[test]
    fn unknown_name_returns_none() {
        let registry = create_dom_registry();
        assert!(registry.resolve("nonExistentMethod").is_none());
    }
}
