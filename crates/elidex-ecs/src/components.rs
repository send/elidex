//! DOM component types stored on ECS entities.

use hecs::Entity;
use std::collections::HashMap;

/// The HTML tag name of an element (e.g., "div", "span", "a").
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TagType(pub String);

/// Key-value attribute map for an element.
#[derive(Debug, Clone, Default)]
pub struct Attributes {
    map: HashMap<String, String>,
}

impl Attributes {
    /// Get an attribute value by name.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.map.get(name).map(String::as_str)
    }

    /// Set an attribute value. Returns the previous value if present.
    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) -> Option<String> {
        self.map.insert(name.into(), value.into())
    }

    /// Remove an attribute by name. Returns the removed value if present.
    pub fn remove(&mut self, name: &str) -> Option<String> {
        self.map.remove(name)
    }

    /// Returns `true` if the attribute exists.
    pub fn contains(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }

    /// Iterate over all attribute name-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.map.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

/// Tree structure relationships linking entities into a DOM tree.
///
/// Fields are `pub(crate)` to ensure tree mutations go through [`EcsDom`]
/// methods, which enforce invariants (no cycles, consistent sibling links).
///
/// [`EcsDom`]: crate::EcsDom
#[derive(Debug, Clone)]
pub struct TreeRelation {
    pub(crate) parent: Option<Entity>,
    pub(crate) first_child: Option<Entity>,
    pub(crate) last_child: Option<Entity>,
    pub(crate) next_sibling: Option<Entity>,
    pub(crate) prev_sibling: Option<Entity>,
}

impl TreeRelation {
    pub fn new() -> Self {
        Self {
            parent: None,
            first_child: None,
            last_child: None,
            next_sibling: None,
            prev_sibling: None,
        }
    }
}

impl Default for TreeRelation {
    fn default() -> Self {
        Self::new()
    }
}

/// Text content for text nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextContent(pub String);

/// Inline style declarations on an element.
///
/// Properties are stored in a `HashMap` to enforce uniqueness (last
/// declaration wins, matching CSS cascade behavior).
#[derive(Debug, Clone, Default)]
pub struct InlineStyle {
    properties: HashMap<String, String>,
}

impl InlineStyle {
    /// Get a style property value by name.
    pub fn get(&self, property: &str) -> Option<&str> {
        self.properties.get(property).map(String::as_str)
    }

    /// Set a style property. Returns the previous value if present.
    pub fn set(&mut self, property: impl Into<String>, value: impl Into<String>) -> Option<String> {
        self.properties.insert(property.into(), value.into())
    }

    /// Remove a style property. Returns the removed value if present.
    pub fn remove(&mut self, property: &str) -> Option<String> {
        self.properties.remove(property)
    }

    /// Returns `true` if the property exists.
    pub fn contains(&self, property: &str) -> bool {
        self.properties.contains_key(property)
    }

    /// Iterate over all property name-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.properties
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
    }
}
