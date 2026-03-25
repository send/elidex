//! Built-in [`HtmlElementHandler`] implementations for representative tags.

use crate::{
    AccessibilityRole, Attributes, CssRule, ElementData, HtmlElementHandler, ParseBehavior,
    PluginRegistry,
};

fn default_create_element(tag: &str, attrs: &Attributes) -> ElementData {
    ElementData {
        tag_name: tag.into(),
        attributes: attrs.clone(),
    }
}

// ---------------------------------------------------------------------------
// DivHandler
// ---------------------------------------------------------------------------

struct DivHandler;

impl HtmlElementHandler for DivHandler {
    fn tag_name(&self) -> &'static str {
        "div"
    }

    fn create_element(&self, attrs: &Attributes) -> ElementData {
        default_create_element("div", attrs)
    }
}

// ---------------------------------------------------------------------------
// AnchorHandler
// ---------------------------------------------------------------------------

struct AnchorHandler {
    default_style: Vec<CssRule>,
}

impl AnchorHandler {
    fn new() -> Self {
        Self {
            default_style: vec![CssRule {
                selector: "a".into(),
                declarations: vec![
                    ("color".into(), "blue".into()),
                    ("text-decoration".into(), "underline".into()),
                ],
            }],
        }
    }
}

impl HtmlElementHandler for AnchorHandler {
    fn tag_name(&self) -> &'static str {
        "a"
    }

    fn default_style(&self) -> &[CssRule] {
        &self.default_style
    }

    fn accessibility_role(&self) -> Option<AccessibilityRole> {
        Some(AccessibilityRole::Link)
    }

    fn create_element(&self, attrs: &Attributes) -> ElementData {
        default_create_element("a", attrs)
    }
}

// ---------------------------------------------------------------------------
// ImgHandler
// ---------------------------------------------------------------------------

struct ImgHandler;

impl HtmlElementHandler for ImgHandler {
    fn tag_name(&self) -> &'static str {
        "img"
    }

    fn parse_behavior(&self) -> ParseBehavior {
        ParseBehavior::Void
    }

    fn accessibility_role(&self) -> Option<AccessibilityRole> {
        Some(AccessibilityRole::Image)
    }

    fn create_element(&self, attrs: &Attributes) -> ElementData {
        default_create_element("img", attrs)
    }
}

// ---------------------------------------------------------------------------
// ScriptHandler
// ---------------------------------------------------------------------------

struct ScriptHandler;

impl HtmlElementHandler for ScriptHandler {
    fn tag_name(&self) -> &'static str {
        "script"
    }

    fn parse_behavior(&self) -> ParseBehavior {
        ParseBehavior::RawText
    }

    fn create_element(&self, attrs: &Attributes) -> ElementData {
        default_create_element("script", attrs)
    }
}

// ---------------------------------------------------------------------------
// ButtonHandler
// ---------------------------------------------------------------------------

struct ButtonHandler;

impl HtmlElementHandler for ButtonHandler {
    fn tag_name(&self) -> &'static str {
        "button"
    }

    fn accessibility_role(&self) -> Option<AccessibilityRole> {
        Some(AccessibilityRole::Button)
    }

    fn create_element(&self, attrs: &Attributes) -> ElementData {
        default_create_element("button", attrs)
    }
}

// ---------------------------------------------------------------------------
// IframeHandler
// ---------------------------------------------------------------------------

struct IframeHandler {
    default_style: Vec<CssRule>,
}

impl IframeHandler {
    fn new() -> Self {
        Self {
            default_style: vec![CssRule {
                selector: "iframe".into(),
                declarations: vec![
                    ("display".into(), "inline-block".into()),
                    ("width".into(), "300px".into()),
                    ("height".into(), "150px".into()),
                    ("border-width".into(), "2px".into()),
                    ("border-style".into(), "inset".into()),
                ],
            }],
        }
    }
}

impl HtmlElementHandler for IframeHandler {
    fn tag_name(&self) -> &'static str {
        "iframe"
    }

    fn parse_behavior(&self) -> ParseBehavior {
        // WHATWG HTML §4.8.5: content model is "nothing".
        // html5ever handles this; text inside <iframe> is raw text (fallback for legacy UAs).
        ParseBehavior::Normal
    }

    fn default_style(&self) -> &[CssRule] {
        &self.default_style
    }

    fn create_element(&self, attrs: &Attributes) -> ElementData {
        default_create_element("iframe", attrs)
    }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Creates a [`PluginRegistry`] pre-populated with built-in HTML element handlers.
///
/// Registers handlers for: `div`, `a`, `img`, `script`, `button`, `iframe`.
#[must_use]
pub fn create_html_element_registry() -> PluginRegistry<dyn HtmlElementHandler> {
    let mut registry: PluginRegistry<dyn HtmlElementHandler> = PluginRegistry::new();
    registry.register_static("div", Box::new(DivHandler));
    registry.register_static("a", Box::new(AnchorHandler::new()));
    registry.register_static("img", Box::new(ImgHandler));
    registry.register_static("script", Box::new(ScriptHandler));
    registry.register_static("button", Box::new(ButtonHandler));
    registry.register_static("iframe", Box::new(IframeHandler::new()));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn div_handler_defaults() {
        let h = DivHandler;
        assert_eq!(h.tag_name(), "div");
        assert_eq!(h.parse_behavior(), ParseBehavior::Normal);
        assert_eq!(h.accessibility_role(), None);
        assert!(h.default_style().is_empty());
    }

    #[test]
    fn anchor_handler_role_and_style() {
        let h = AnchorHandler::new();
        assert_eq!(h.tag_name(), "a");
        assert_eq!(h.accessibility_role(), Some(AccessibilityRole::Link));
        assert!(!h.default_style().is_empty());
    }

    #[test]
    fn img_handler_void() {
        let h = ImgHandler;
        assert_eq!(h.tag_name(), "img");
        assert_eq!(h.parse_behavior(), ParseBehavior::Void);
        assert_eq!(h.accessibility_role(), Some(AccessibilityRole::Image));
    }

    #[test]
    fn script_handler_raw_text() {
        let h = ScriptHandler;
        assert_eq!(h.tag_name(), "script");
        assert_eq!(h.parse_behavior(), ParseBehavior::RawText);
        assert_eq!(h.accessibility_role(), None);
    }

    #[test]
    fn button_handler_role() {
        let h = ButtonHandler;
        assert_eq!(h.tag_name(), "button");
        assert_eq!(h.parse_behavior(), ParseBehavior::Normal);
        assert_eq!(h.accessibility_role(), Some(AccessibilityRole::Button));
    }

    #[test]
    fn html_registry_factory() {
        let registry = create_html_element_registry();
        assert_eq!(registry.len(), 6);
        let div = registry.resolve("div").unwrap();
        assert_eq!(div.tag_name(), "div");
        let a = registry.resolve("a").unwrap();
        assert_eq!(a.accessibility_role(), Some(AccessibilityRole::Link));
        assert!(registry.resolve("unknown").is_none());
    }

    struct MyWidgetHandler;
    impl HtmlElementHandler for MyWidgetHandler {
        fn tag_name(&self) -> &'static str {
            "my-widget"
        }
        fn create_element(&self, attrs: &Attributes) -> ElementData {
            ElementData {
                tag_name: "my-widget".into(),
                attributes: attrs.clone(),
            }
        }
    }

    #[test]
    fn html_registry_dynamic_custom_tag() {
        let mut registry = create_html_element_registry();
        registry.register_dynamic("my-widget".into(), Box::new(MyWidgetHandler));
        assert_eq!(registry.len(), 7);
        let handler = registry.resolve("my-widget").unwrap();
        let el = handler.create_element(&HashMap::new());
        assert_eq!(el.tag_name, "my-widget");
    }
}
