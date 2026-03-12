//! Core plugin traits (Ch.7 §7.2).
//!
//! These traits define the extension points for the elidex browser engine.
//! All traits require `Send + Sync` for safe use in async and multi-threaded
//! contexts.

use std::collections::HashMap;

use crate::{
    ComputedStyle, CssSpecLevel, CssValue, DomSpecLevel, HtmlSpecLevel, HttpRequest, HttpResponse,
    LayoutContext, LayoutResult, NetworkError, WebApiSpecLevel,
};

/// Pre-resolve context passed to CSS property handlers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StyleContext {
    /// Root element font size in px.
    pub root_font_size_px: f32,
    /// Parent element font size in px.
    pub parent_font_size_px: f32,
    /// Viewport width in px.
    pub viewport_width_px: f32,
    /// Viewport height in px.
    pub viewport_height_px: f32,
}

impl Default for StyleContext {
    fn default() -> Self {
        Self {
            root_font_size_px: 16.0,
            parent_font_size_px: 16.0,
            viewport_width_px: 1280.0,
            viewport_height_px: 720.0,
        }
    }
}

/// Normalized value returned by a CSS property handler after resolution.
pub type ComputedValue = CssValue;

/// Metadata used to announce planned deprecation of a plugin handler.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DeprecationInfo {
    /// Project version where this handler became deprecated.
    pub since: String,
    /// Optional replacement feature name.
    pub replacement: Option<String>,
    /// Optional free-form migration notes.
    pub note: Option<String>,
}

/// Handler interface for CSS property parsing and resolution.
pub trait CssPropertyHandler: Send + Sync {
    /// Returns the canonical property name (e.g. `display`).
    fn property_name(&self) -> &str;

    /// Returns the specification level of this property handler.
    fn spec_level(&self) -> CssSpecLevel {
        CssSpecLevel::Standard
    }

    /// Parse raw CSS text into an internal value representation.
    fn parse(&self, value: &str) -> Result<CssValue, crate::ParseError>;

    /// Resolve a parsed value into a computed value.
    fn resolve(&self, value: &CssValue, ctx: &StyleContext) -> ComputedValue;

    /// Whether changing this property may affect layout.
    fn affects_layout(&self) -> bool {
        false
    }

    /// Optional deprecation metadata for this handler.
    fn deprecated_by(&self) -> Option<DeprecationInfo> {
        None
    }
}

/// Attribute map passed to HTML element handlers.
pub type Attributes = HashMap<String, String>;

/// A minimal CSS rule used by element handlers for default styles.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CssRule {
    /// Selector text (e.g. `p`, `h1`, `:host`).
    pub selector: String,
    /// Declaration pairs (`property`, `value`).
    pub declarations: Vec<(String, String)>,
}

/// Parsed element payload emitted by `HtmlElementHandler`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ElementData {
    /// Canonical HTML tag name (lowercase).
    pub tag_name: String,
    /// Element attributes.
    pub attributes: Attributes,
}

/// Parsing behavior hint for specialized elements.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum ParseBehavior {
    /// Standard tokenization/tree-construction behavior.
    #[default]
    Normal,
    /// Void element (no end tag, no children).
    Void,
    /// Raw text parsing mode (e.g. `script`, `style`).
    RawText,
    /// Escapable raw text parsing mode (e.g. `textarea`, `title`).
    EscapableRawText,
}

/// Accessibility role hint surfaced by HTML element handlers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AccessibilityRole {
    Generic,
    Button,
    Link,
    Image,
    Heading,
    Navigation,
    Main,
}

/// Handler interface for HTML tag-specific behavior.
pub trait HtmlElementHandler: Send + Sync {
    /// Returns the canonical tag name (e.g. `div`).
    fn tag_name(&self) -> &str;

    /// Returns the specification level for this tag.
    fn spec_level(&self) -> HtmlSpecLevel {
        HtmlSpecLevel::Html5
    }

    /// Returns default stylesheet rules for this tag.
    fn default_style(&self) -> &[CssRule] {
        &[]
    }

    /// Creates normalized element payload from parsed attributes.
    fn create_element(&self, attrs: &Attributes) -> ElementData;

    /// Optional parser behavior override.
    fn parse_behavior(&self) -> ParseBehavior {
        ParseBehavior::Normal
    }

    /// Optional default accessibility role.
    fn accessibility_role(&self) -> Option<AccessibilityRole> {
        None
    }
}

/// A layout tree node passed to `LayoutModel`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutNode {
    /// Stable node id for diagnostics and caching.
    pub node_id: u64,
    /// Resolved style attached to this node.
    pub style: ComputedStyle,
}

/// Constraints passed to layout algorithms.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Constraints {
    /// Available inline size (px), if known.
    pub available_width: Option<f32>,
    /// Available block size (px), if known.
    pub available_height: Option<f32>,
    /// Minimum width (px).
    pub min_width: f32,
    /// Maximum width (px), if bounded.
    pub max_width: Option<f32>,
    /// Minimum height (px).
    pub min_height: f32,
    /// Maximum height (px), if bounded.
    pub max_height: Option<f32>,
}

/// Handler interface for layout algorithms (block, flex, grid, table, etc.).
pub trait LayoutModel: Send + Sync {
    /// Human-readable layout model name.
    fn name(&self) -> &str;

    /// Returns the spec level classification for this layout model.
    fn spec_level(&self) -> DomSpecLevel {
        DomSpecLevel::Living
    }

    /// Runs layout for `node` and its already-resolved `children`.
    fn layout(
        &self,
        node: &LayoutNode,
        children: &[LayoutNode],
        constraints: &Constraints,
        ctx: &LayoutContext,
    ) -> LayoutResult;
}

/// Middleware for intercepting and modifying network requests/responses.
pub trait NetworkMiddleware: Send + Sync {
    /// Returns the middleware name.
    fn name(&self) -> &str;

    /// Returns the specification level of this network middleware.
    fn spec_level(&self) -> WebApiSpecLevel {
        WebApiSpecLevel::Modern
    }

    /// Called before a request is sent. Can modify the request in-place.
    ///
    /// # Errors
    ///
    /// Returns `NetworkError` if the request should be rejected.
    fn on_request(&self, request: &mut HttpRequest) -> Result<(), NetworkError>;

    /// Called after a response is received. Can modify the response in-place.
    ///
    /// # Errors
    ///
    /// Returns `NetworkError` if the response should be rejected.
    fn on_response(&self, response: &mut HttpResponse) -> Result<(), NetworkError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EdgeSizes, LayoutBox, ParseError, Rect, Size};

    struct WidthHandler;

    impl CssPropertyHandler for WidthHandler {
        fn property_name(&self) -> &'static str {
            "width"
        }

        fn parse(&self, value: &str) -> Result<CssValue, ParseError> {
            if value == "auto" {
                return Ok(CssValue::Auto);
            }
            Err(ParseError {
                property: "width".into(),
                input: value.into(),
                message: "only auto is supported in test".into(),
            })
        }

        fn resolve(&self, value: &CssValue, _ctx: &StyleContext) -> ComputedValue {
            value.clone()
        }

        fn affects_layout(&self) -> bool {
            true
        }
    }

    struct DivHandler;

    impl HtmlElementHandler for DivHandler {
        fn tag_name(&self) -> &'static str {
            "div"
        }

        fn create_element(&self, attrs: &Attributes) -> ElementData {
            ElementData {
                tag_name: self.tag_name().to_string(),
                attributes: attrs.clone(),
            }
        }
    }

    struct BlockLayout;

    impl LayoutModel for BlockLayout {
        fn name(&self) -> &'static str {
            "block"
        }

        fn layout(
            &self,
            _node: &LayoutNode,
            children: &[LayoutNode],
            constraints: &Constraints,
            _ctx: &LayoutContext,
        ) -> LayoutResult {
            let child_count = u16::try_from(children.len()).unwrap_or(u16::MAX);
            let h = f32::from(child_count) * 10.0;
            LayoutResult {
                bounds: Rect::new(0.0, 0.0, constraints.available_width.unwrap_or(0.0), h),
                margin: EdgeSizes::default(),
                padding: EdgeSizes::default(),
                border: EdgeSizes::default(),
            }
        }
    }

    #[test]
    fn css_property_handler_contract() {
        let handler = WidthHandler;
        assert_eq!(handler.property_name(), "width");
        assert_eq!(handler.spec_level(), CssSpecLevel::Standard);
        assert!(handler.affects_layout());
        assert_eq!(handler.parse("auto"), Ok(CssValue::Auto));
        assert_eq!(handler.deprecated_by(), None);
    }

    #[test]
    fn html_element_handler_defaults() {
        let mut attrs = Attributes::new();
        attrs.insert("class".into(), "container".into());
        let handler = DivHandler;
        assert_eq!(handler.spec_level(), HtmlSpecLevel::Html5);
        assert_eq!(handler.default_style(), &[]);
        assert_eq!(handler.parse_behavior(), ParseBehavior::Normal);
        assert_eq!(handler.accessibility_role(), None);
        let el = handler.create_element(&attrs);
        assert_eq!(el.tag_name, "div");
        assert_eq!(el.attributes.get("class"), Some(&"container".to_string()));
    }

    #[test]
    fn layout_model_contract() {
        let model = BlockLayout;
        let node = LayoutNode::default();
        let children = vec![LayoutNode::default(), LayoutNode::default()];
        let constraints = Constraints {
            available_width: Some(320.0),
            available_height: Some(200.0),
            min_width: 0.0,
            max_width: Some(320.0),
            min_height: 0.0,
            max_height: Some(200.0),
        };
        let ctx = LayoutContext {
            viewport: Size {
                width: 1280.0,
                height: 720.0,
            },
            containing_block: Size {
                width: 320.0,
                height: 200.0,
            },
        };
        assert_eq!(model.name(), "block");
        assert_eq!(model.spec_level(), DomSpecLevel::Living);
        let result = model.layout(&node, &children, &constraints, &ctx);
        assert_eq!(result.bounds.width, 320.0);
        assert_eq!(result.bounds.height, 20.0);
        let _ = LayoutBox::default();
    }
}
