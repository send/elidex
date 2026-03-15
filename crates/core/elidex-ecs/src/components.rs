//! DOM component types stored on ECS entities.

use std::collections::HashMap;
use std::sync::Arc;

use hecs::Entity;

/// Generate string-keyed map accessor methods for a struct wrapping a `HashMap<String, String>`.
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
                self.$field.remove(name)
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attributes {
    map: HashMap<String, String>,
}

impl_string_map!(Attributes, map, "attribute");

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
}

/// Text content for text nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextContent(pub String);

/// Inline style declarations on an element.
///
/// Properties are stored in a `HashMap` to enforce uniqueness (last
/// declaration wins, matching CSS cascade behavior).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InlineStyle {
    properties: HashMap<String, String>,
}

impl_string_map!(InlineStyle, properties, "style property");

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

/// Marker: this entity is a shadow root.
///
/// A shadow root is a document fragment attached to a host element.
/// It provides style encapsulation and DOM isolation.
///
// TODO(L2): WHATWG DOM §4.8 specifies additional fields:
// - `delegatesFocus: bool` (focus delegation to first focusable element)
// - `slotAssignment: "manual" | "named"` (slot assignment mode)
// Currently omitted because no consumer exists.
#[derive(Clone, Copy, Debug)]
pub struct ShadowRoot {
    /// Open or closed mode.
    pub mode: ShadowRootMode,
    /// The host element that owns this shadow root.
    pub host: Entity,
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
