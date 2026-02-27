//! DOM component types stored on ECS entities.

use hecs::Entity;
use std::collections::HashMap;

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
