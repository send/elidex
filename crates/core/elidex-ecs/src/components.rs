//! DOM component types stored on ECS entities.

use std::sync::Arc;

use hecs::Entity;
use indexmap::IndexMap;

use elidex_plugin::{Size, Vector};

/// Generate string-keyed map accessor methods for a struct wrapping an `IndexMap<String, String>`.
macro_rules! impl_string_map {
    ($type:ty, $field:ident, $key_label:literal) => {
        impl $type {
            #[doc = concat!("Get a ", $key_label, " value by name.")]
            pub fn get(&self, name: &str) -> Option<&str> {
                self.$field.get(name).map(String::as_str)
            }

            #[doc = concat!("Set a ", $key_label, " value. Returns the previous value if present.")]
            pub fn set(
                &mut self,
                name: impl Into<String>,
                value: impl Into<String>,
            ) -> Option<String> {
                self.$field.insert(name.into(), value.into())
            }

            #[doc = concat!("Remove a ", $key_label, " by name. Returns the removed value if present.")]
            pub fn remove(&mut self, name: &str) -> Option<String> {
                self.$field.shift_remove(name)
            }

            #[doc = concat!("Returns `true` if the ", $key_label, " exists.")]
            pub fn contains(&self, name: &str) -> bool {
                self.$field.contains_key(name)
            }

            #[doc = concat!("Iterate over all ", $key_label, " name-value pairs.")]
            pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
                self.$field.iter().map(|(k, v)| (k.as_str(), v.as_str()))
            }
        }
    };
}

/// The HTML tag name of an element (e.g., "div", "span", "a").
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TagType(pub String);

/// Key-value attribute map for an element.
///
/// Uses `IndexMap` to preserve insertion order, matching the WHATWG DOM spec
/// requirement that `getAttributeNames()` returns attributes in insertion order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attributes {
    map: IndexMap<String, String>,
}

impl_string_map!(Attributes, map, "attribute");

impl Attributes {
    /// Returns the number of attributes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if there are no attributes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Returns attribute names in insertion order.
    #[must_use]
    pub fn keys(&self) -> Vec<&str> {
        self.map.keys().map(String::as_str).collect()
    }
}

/// Tree structure relationships linking entities into a DOM tree.
///
/// Fields are `pub(crate)` to ensure tree mutations go through [`EcsDom`]
/// methods, which enforce invariants (no cycles, consistent sibling links).
///
/// **Warning:** `Clone` is derived for internal snapshotting only. Inserting a
/// cloned `TreeRelation` as a component on a different entity will break tree
/// invariants. Always use [`EcsDom`] mutation methods instead.
///
/// [`EcsDom`]: crate::EcsDom
#[derive(Debug, Clone, Default)]
pub struct TreeRelation {
    pub(crate) parent: Option<Entity>,
    pub(crate) first_child: Option<Entity>,
    pub(crate) last_child: Option<Entity>,
    pub(crate) next_sibling: Option<Entity>,
    pub(crate) prev_sibling: Option<Entity>,
    /// Monotonically increasing version counter for live collection cache invalidation.
    /// Bumped on any mutation (child add/remove, attribute change) that affects
    /// the subtree rooted at this entity. Propagated to ancestors via `rev_version()`.
    pub(crate) inclusive_descendants_version: u64,
}

/// Text content for text nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextContent(pub String);

/// Inline style declarations on an element.
///
/// Properties are stored in an `IndexMap` to preserve insertion order
/// (matching CSSOM `style.cssText` serialization order) while enforcing
/// uniqueness (last declaration wins, matching CSS cascade behavior).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InlineStyle {
    properties: IndexMap<String, String>,
}

impl_string_map!(InlineStyle, properties, "style property");

impl InlineStyle {
    /// Returns the number of properties.
    #[must_use]
    pub fn len(&self) -> usize {
        self.properties.len()
    }

    /// Returns `true` if there are no properties.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }

    /// Serialize all properties to a CSS text string.
    #[must_use]
    pub fn css_text(&self) -> String {
        self.properties
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Get the property name at the given index (insertion order).
    #[must_use]
    pub fn property_at(&self, index: usize) -> Option<&str> {
        self.properties.keys().nth(index).map(String::as_str)
    }
}

/// Marker component for pseudo-element entities (`::before`, `::after`).
///
/// Pseudo-element entities are generated during style resolution and
/// inserted as children of the originating element. They carry a
/// `ComputedStyle` and `TextContent` but are not real DOM elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PseudoElementMarker;

/// Dynamic element state flags for CSS pseudo-class matching.
///
/// Tracks whether an element is hovered, focused, active, or a link.
/// Used by the selector engine to match `:hover`, `:focus`, `:active`,
/// `:link`, `:visited`, and form-related pseudo-classes.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ElementState(pub u16);

impl ElementState {
    pub const HOVER: u16 = 0x0001;
    pub const FOCUS: u16 = 0x0002;
    pub const ACTIVE: u16 = 0x0004;
    pub const LINK: u16 = 0x0008;
    pub const VISITED: u16 = 0x0010;
    pub const DISABLED: u16 = 0x0020;
    pub const CHECKED: u16 = 0x0040;
    pub const REQUIRED: u16 = 0x0080;
    pub const VALID: u16 = 0x0100;
    pub const INVALID: u16 = 0x0200;
    pub const READ_ONLY: u16 = 0x0400;
    pub const INDETERMINATE: u16 = 0x0800;

    /// Returns `true` if the given flag is set.
    #[must_use]
    pub fn contains(self, flag: u16) -> bool {
        self.0 & flag != 0
    }

    /// Set the given flag.
    pub fn insert(&mut self, flag: u16) {
        self.0 |= flag;
    }

    /// Clear the given flag.
    pub fn remove(&mut self, flag: u16) {
        self.0 &= !flag;
    }

    /// Set or clear the given flag based on `value`.
    pub fn set(&mut self, flag: u16, value: bool) {
        if value {
            self.insert(flag);
        } else {
            self.remove(flag);
        }
    }

    /// Returns `true` if no flags are set.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// Shadow root mode (WHATWG DOM §4.8).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShadowRootMode {
    /// Shadow root is accessible via `element.shadowRoot`.
    Open,
    /// Shadow root is not accessible via `element.shadowRoot`.
    Closed,
}

/// Slot assignment mode for shadow roots (WHATWG DOM §4.8).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SlotAssignmentMode {
    /// Slots are assigned by matching `<slot name>` attributes (default).
    #[default]
    Named,
    /// Slots are assigned manually via `slot.assign()`.
    Manual,
}

/// Marker: this entity is a shadow root.
///
/// A shadow root is a document fragment attached to a host element.
/// It provides style encapsulation and DOM isolation (WHATWG DOM §4.8).
#[derive(Clone, Copy, Debug)]
pub struct ShadowRoot {
    /// Open or closed mode.
    pub mode: ShadowRootMode,
    /// The host element that owns this shadow root.
    pub host: Entity,
    /// Whether focus is delegated to the first focusable element in the shadow tree.
    pub delegates_focus: bool,
    /// How slots are assigned to light DOM children.
    pub slot_assignment: SlotAssignmentMode,
}

/// Marker: this element hosts a shadow root.
///
/// Attached to elements that have had `attachShadow()` called on them.
#[derive(Clone, Copy, Debug)]
pub struct ShadowHost {
    /// The shadow root entity.
    pub shadow_root: Entity,
}

/// Slot assignment for distributed nodes.
///
/// Attached to `<slot>` entities in the shadow tree.
/// Contains the list of light DOM nodes distributed to this slot.
#[derive(Debug, Default)]
pub struct SlotAssignment {
    /// Light DOM nodes assigned to this slot, in order.
    pub assigned_nodes: Vec<Entity>,
}

/// Marker attached to light DOM nodes that have been distributed to a slot.
///
/// Added by `distribute_slots()` for O(1) slotted-element checks in
/// selector matching and event retargeting.
#[derive(Clone, Copy, Debug)]
pub struct SlottedMarker;

/// Marker for `<template>` elements (inert — not rendered/styled).
///
/// Template content is not part of the rendered document. Elements
/// with this marker are excluded from style resolution and rendering.
#[derive(Clone, Copy, Debug)]
pub struct TemplateContent;

/// Decoded image pixel data for `<img>` elements.
///
/// Stored as a component on image entities after the image has been
/// fetched and decoded. Pixel data is RGBA8 format (4 bytes per pixel).
#[derive(Debug, Clone)]
pub struct ImageData {
    /// RGBA8 pixel data (width × height × 4 bytes).
    ///
    /// Wrapped in `Arc` so that the display list can share the data
    /// without cloning the entire pixel buffer every frame.
    pub pixels: Arc<Vec<u8>>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

/// Decoded background image layers for CSS `background-image`.
///
/// Each entry corresponds to a background layer. `None` entries indicate
/// layers that are not URL-based (e.g. gradients, or `none`).
#[derive(Debug, Clone)]
pub struct BackgroundImages {
    /// Per-layer decoded image data. `None` = gradient or none.
    pub layers: Vec<Option<Arc<ImageData>>>,
}

/// The kind of DOM node (WHATWG DOM §4.4 Node.nodeType).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NodeKind {
    /// Element node (nodeType = 1).
    Element,
    /// Attribute node (nodeType = 2).
    Attribute,
    /// Text node (nodeType = 3).
    Text,
    /// CDATA section node (nodeType = 4).
    CdataSection,
    /// Processing instruction node (nodeType = 7).
    ProcessingInstruction,
    /// Comment node (nodeType = 8).
    Comment,
    /// Document node (nodeType = 9).
    Document,
    /// Document type node (nodeType = 10).
    DocumentType,
    /// Document fragment node (nodeType = 11).
    DocumentFragment,
}

impl NodeKind {
    /// Returns the WHATWG `Node.nodeType` numeric value.
    #[must_use]
    pub fn node_type(self) -> u32 {
        match self {
            Self::Element => 1,
            Self::Attribute => 2,
            Self::Text => 3,
            Self::CdataSection => 4,
            Self::ProcessingInstruction => 7,
            Self::Comment => 8,
            Self::Document => 9,
            Self::DocumentType => 10,
            Self::DocumentFragment => 11,
        }
    }

    /// Create a `NodeKind` from a WHATWG `Node.nodeType` numeric value.
    #[must_use]
    pub fn from_node_type(node_type: u32) -> Option<Self> {
        match node_type {
            1 => Some(Self::Element),
            2 => Some(Self::Attribute),
            3 => Some(Self::Text),
            4 => Some(Self::CdataSection),
            7 => Some(Self::ProcessingInstruction),
            8 => Some(Self::Comment),
            9 => Some(Self::Document),
            10 => Some(Self::DocumentType),
            11 => Some(Self::DocumentFragment),
            _ => None,
        }
    }
}

/// Data for a comment node (`<!-- ... -->`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentData(pub String);

/// Data for a document type node (`<!DOCTYPE ...>`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocTypeData {
    /// The `name` part of the doctype (e.g. `"html"`).
    pub name: String,
    /// The `publicId` part of the doctype.
    pub public_id: String,
    /// The `systemId` part of the doctype.
    pub system_id: String,
}

/// Scroll state for elements with `overflow: scroll | auto | hidden`.
///
/// Tracks the current scroll position and content/client dimensions
/// for scroll containers (CSS Overflow L3 §3).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScrollState {
    /// Current scroll offset in CSS pixels (displacement from origin).
    pub scroll_offset: Vector,
    /// Total scrollable content size.
    pub scroll_size: Size,
    /// Visible client area size (padding box minus scrollbar).
    pub client_size: Size,
}

impl ScrollState {
    /// Create a new `ScrollState` with the given dimensions.
    #[must_use]
    pub fn new(
        scroll_width: f32,
        scroll_height: f32,
        client_width: f32,
        client_height: f32,
    ) -> Self {
        Self {
            scroll_offset: Vector::<f32>::ZERO,
            scroll_size: Size {
                width: scroll_width,
                height: scroll_height,
            },
            client_size: Size {
                width: client_width,
                height: client_height,
            },
        }
    }

    /// Maximum horizontal scroll offset (clamped to 0).
    #[must_use]
    pub fn max_scroll_x(&self) -> f32 {
        (self.scroll_size.width - self.client_size.width).max(0.0)
    }

    /// Maximum vertical scroll offset (clamped to 0).
    #[must_use]
    pub fn max_scroll_y(&self) -> f32 {
        (self.scroll_size.height - self.client_size.height).max(0.0)
    }

    /// Clamp scroll offsets to valid range.
    pub fn clamp_scroll(&mut self) {
        self.scroll_offset.x = self.scroll_offset.x.clamp(0.0, self.max_scroll_x());
        self.scroll_offset.y = self.scroll_offset.y.clamp(0.0, self.max_scroll_y());
    }
}

/// Marker for anonymous table objects (CSS 2.1 §17.2.1).
///
/// Re-layout reuses existing entities with this marker to prevent
/// duplicate anonymous box generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnonymousTableMarker;

/// Data for an Attr node (WHATWG DOM §4.9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttrData {
    /// The attribute's local name.
    pub local_name: String,
    /// The attribute's value.
    pub value: String,
    /// The element that owns this attribute, if any.
    pub owner_element: Option<Entity>,
}

/// Cache mapping attribute names to their `Attr` entity representations.
///
/// Attached to element entities so that `getAttributeNode("x")` returns the
/// same `Attr` entity on repeated calls (WHATWG DOM identity semantics).
#[derive(Debug, Clone, Default)]
pub struct AttrEntityCache {
    /// Maps lowercase attribute name to the `Attr` entity.
    pub entries: std::collections::HashMap<String, Entity>,
}

/// Loading attribute for `<iframe>` and `<img>` elements (WHATWG HTML §4.8.5).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LoadingAttribute {
    /// Load immediately (default).
    #[default]
    Eager,
    /// Defer loading until near the viewport (lazy loading).
    Lazy,
}

/// Data for an `<iframe>` element (WHATWG HTML §4.8.5).
///
/// Stored as an ECS component on iframe entities. Used by layout for
/// intrinsic sizing (replaced element model) and by the shell for
/// iframe lifecycle management.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct IframeData {
    /// URL to load (`src` attribute).
    pub src: Option<String>,
    /// Inline HTML content (`srcdoc` attribute).
    pub srcdoc: Option<String>,
    /// Raw sandbox attribute value.
    pub sandbox: Option<String>,
    /// Iframe width in CSS pixels (HTML attribute, default 300).
    pub width: u32,
    /// Iframe height in CSS pixels (HTML attribute, default 150).
    pub height: u32,
    /// Frame name for targeting (`name` attribute).
    pub name: Option<String>,
    /// Loading strategy.
    pub loading: LoadingAttribute,
    /// Whether fullscreen is allowed (`allowfullscreen` attribute).
    pub allow_fullscreen: bool,
    /// Referrer policy (`referrerpolicy` attribute).
    pub referrer_policy: Option<String>,
    /// Permissions policy (`allow` attribute).
    pub allow: Option<String>,
    /// Whether credentials are suppressed (`credentialless` attribute).
    pub credentialless: bool,
}

impl Default for IframeData {
    fn default() -> Self {
        Self {
            src: None,
            srcdoc: None,
            sandbox: None,
            width: 300,
            height: 150,
            name: None,
            loading: LoadingAttribute::Eager,
            allow_fullscreen: false,
            referrer_policy: None,
            allow: None,
            credentialless: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_state_new_and_defaults() {
        let s = ScrollState::new(500.0, 1000.0, 300.0, 400.0);
        assert_eq!(s.scroll_offset, Vector::<f32>::ZERO);
        assert_eq!(s.scroll_size.width, 500.0);
        assert_eq!(s.scroll_size.height, 1000.0);
        assert_eq!(s.client_size.width, 300.0);
        assert_eq!(s.client_size.height, 400.0);

        let d = ScrollState::default();
        assert_eq!(d.scroll_offset, Vector::<f32>::ZERO);
        assert_eq!(d.scroll_size.width, 0.0);
    }

    #[test]
    fn scroll_state_clamp() {
        let mut s = ScrollState::new(500.0, 1000.0, 300.0, 400.0);
        s.scroll_offset = Vector::new(999.0, -10.0);
        s.clamp_scroll();
        assert!((s.scroll_offset.x - 200.0).abs() < f32::EPSILON);
        assert_eq!(s.scroll_offset.y, 0.0);
    }

    #[test]
    fn scroll_state_max_scroll_zero_content() {
        let s = ScrollState::new(100.0, 50.0, 200.0, 100.0);
        assert_eq!(s.max_scroll_x(), 0.0);
        assert_eq!(s.max_scroll_y(), 0.0);
    }
}
