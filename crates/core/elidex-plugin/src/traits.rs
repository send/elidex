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

/// Context for resolving relative CSS values to computed values.
///
/// Provides the base sizes needed for resolving em, rem, vw, vh, and
/// percentage units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolveContext {
    /// Viewport width in px.
    pub viewport_width: f32,
    /// Viewport height in px.
    pub viewport_height: f32,
    /// Base value for `em` unit resolution. For font-size this is the
    /// parent's font-size; for all other properties it is the element's
    /// own computed font-size.
    pub em_base: f32,
    /// Root element font size in px (for `rem` resolution).
    pub root_font_size: f32,
}

impl ResolveContext {
    /// Return a copy with a different `em_base` value.
    #[must_use]
    pub fn with_em_base(&self, em_base: f32) -> Self {
        Self { em_base, ..*self }
    }

    /// Return a copy with both `em_base` and `root_font_size` overridden.
    #[must_use]
    pub fn with_em_and_root(&self, em_base: f32, root_font_size: f32) -> Self {
        Self {
            em_base,
            root_font_size,
            ..*self
        }
    }
}

impl Default for ResolveContext {
    fn default() -> Self {
        Self {
            viewport_width: 1280.0,
            viewport_height: 720.0,
            em_base: 16.0,
            root_font_size: 16.0,
        }
    }
}

/// A parsed property declaration returned by [`CssPropertyHandler::parse`].
///
/// Represents a single property-value pair after shorthand expansion.
/// Importance (`!important`) is handled by the cascade, not by handlers.
#[derive(Clone, Debug, PartialEq)]
pub struct PropertyDeclaration {
    /// Longhand property name (e.g. `"margin-top"`).
    pub property: String,
    /// Parsed value.
    pub value: CssValue,
}

impl PropertyDeclaration {
    /// Create a new property declaration.
    #[must_use]
    pub fn new(property: impl Into<String>, value: CssValue) -> Self {
        Self {
            property: property.into(),
            value,
        }
    }
}

/// Handler interface for CSS property parsing and resolution.
///
/// Each handler covers one or more related CSS properties. Handlers
/// are registered in a [`CssPropertyRegistry`](crate::CssPropertyRegistry)
/// and dispatched by property name during parsing and resolution.
pub trait CssPropertyHandler: Send + Sync {
    /// Returns the CSS property names this handler covers.
    ///
    /// Longhand names only — shorthand expansion is handled internally
    /// by [`parse`](Self::parse).
    fn property_names(&self) -> &[&str];

    /// Returns the specification level of this property handler.
    fn spec_level(&self) -> CssSpecLevel {
        CssSpecLevel::Standard
    }

    /// Parse a CSS property value using a cssparser tokenizer.
    ///
    /// For shorthand properties, returns multiple longhand declarations.
    /// For longhands, returns a single-element vec.
    #[allow(clippy::elidable_lifetime_names)]
    fn parse<'i>(
        &self,
        name: &str,
        input: &mut cssparser::Parser<'i, '_>,
    ) -> Result<Vec<PropertyDeclaration>, crate::ParseError>;

    /// Resolve a parsed value into a computed value on `style`.
    ///
    /// Writes the resolved value directly to the appropriate
    /// [`ComputedStyle`] field(s).
    fn resolve(
        &self,
        name: &str,
        value: &CssValue,
        ctx: &ResolveContext,
        style: &mut ComputedStyle,
    );

    /// Returns the CSS initial value for `name`.
    fn initial_value(&self, name: &str) -> CssValue;

    /// Returns `true` if `name` is an inherited property.
    fn is_inherited(&self, name: &str) -> bool;

    /// Whether changing this property may affect layout.
    fn affects_layout(&self, _name: &str) -> bool {
        false
    }

    /// Extract the current computed value from `style` as a [`CssValue`].
    ///
    /// Used by `getComputedStyle()` and for inheritance serialization.
    fn get_computed(&self, name: &str, style: &ComputedStyle) -> CssValue;
}

/// Type alias for a registry of CSS property handlers.
pub type CssPropertyRegistry = crate::PluginRegistry<dyn CssPropertyHandler>;

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

    struct TestWidthHandler;

    impl CssPropertyHandler for TestWidthHandler {
        fn property_names(&self) -> &[&str] {
            &["width"]
        }

        fn parse<'i>(
            &self,
            _name: &str,
            input: &mut cssparser::Parser<'i, '_>,
        ) -> Result<Vec<PropertyDeclaration>, ParseError> {
            let location = input.current_source_location();
            if input.try_parse(|i| i.expect_ident_matching("auto")).is_ok() {
                return Ok(vec![PropertyDeclaration::new("width", CssValue::Auto)]);
            }
            Err(ParseError {
                property: "width".into(),
                input: format!("line {}", location.line),
                message: "only auto is supported in test".into(),
            })
        }

        fn resolve(
            &self,
            _name: &str,
            value: &CssValue,
            _ctx: &ResolveContext,
            style: &mut ComputedStyle,
        ) {
            if let CssValue::Auto = value {
                style.width = crate::Dimension::Auto;
            }
        }

        fn initial_value(&self, _name: &str) -> CssValue {
            CssValue::Auto
        }

        fn is_inherited(&self, _name: &str) -> bool {
            false
        }

        fn affects_layout(&self, _name: &str) -> bool {
            true
        }

        fn get_computed(&self, _name: &str, _style: &ComputedStyle) -> CssValue {
            CssValue::Auto
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
        let handler = TestWidthHandler;
        assert_eq!(handler.property_names(), &["width"]);
        assert_eq!(handler.spec_level(), CssSpecLevel::Standard);
        assert!(handler.affects_layout("width"));
        assert!(!handler.is_inherited("width"));
        assert_eq!(handler.initial_value("width"), CssValue::Auto);
    }

    #[test]
    fn css_property_handler_parse() {
        let handler = TestWidthHandler;
        let mut parser_input = cssparser::ParserInput::new("auto");
        let mut parser = cssparser::Parser::new(&mut parser_input);
        let result = handler.parse("width", &mut parser).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].property, "width");
        assert_eq!(result[0].value, CssValue::Auto);
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

    #[test]
    fn property_declaration_new() {
        let decl = PropertyDeclaration::new("color", CssValue::Color(crate::CssColor::RED));
        assert_eq!(decl.property, "color");
        assert_eq!(decl.value, CssValue::Color(crate::CssColor::RED));
    }

    #[test]
    fn resolve_context_default() {
        let ctx = ResolveContext::default();
        assert_eq!(ctx.viewport_width, 1280.0);
        assert_eq!(ctx.viewport_height, 720.0);
        assert_eq!(ctx.em_base, 16.0);
        assert_eq!(ctx.root_font_size, 16.0);
    }
}
