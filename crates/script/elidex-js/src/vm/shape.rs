//! Hidden Classes (Shape/Transition) for O(1) property lookup.
//!
//! A [`Shape`] describes the "type" of an object: which property names exist,
//! in what order, with what attributes, and at which slot index each value is
//! stored.  Objects that were created by the same constructor and had the same
//! sequence of property additions share the same Shape, enabling inline caches
//! to skip property lookup entirely.
//!
//! ## Design
//!
//! - **`ordered_entries`** is the single source of truth for property names,
//!   insertion order, and attributes.  `ordered_entries[i]` corresponds to
//!   `Object.slots[i]`.
//! - **`property_map`** provides O(1) key→slot-index lookup for random access.
//! - **`transitions`** caches child Shapes so that the same property addition
//!   or attribute reconfiguration reuses an existing Shape.
//!
//! Two kinds of transitions exist:
//! - [`TransitionKey::Add`]: a new property is added (slot count grows by 1).
//! - [`TransitionKey::Reconfigure`]: an existing property's attributes change
//!   (slot count stays the same).  This covers `Object.defineProperty` attribute
//!   changes and data↔accessor conversions without falling back to Dictionary
//!   mode.

use std::collections::HashMap;

use super::value::PropertyKey;

/// Index into `VmInner.shapes`.
pub type ShapeId = u32;

/// The root (empty) shape.  Always `shapes[0]`.
pub const ROOT_SHAPE: ShapeId = 0;

/// Property attributes tracked per slot in the Shape.
///
/// These mirror the ES2020 property descriptor flags plus `is_accessor` to
/// distinguish data properties from accessor properties at the shape level
/// (needed for correct transition keying).
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct PropertyAttrs {
    pub writable: bool,
    pub enumerable: bool,
    pub configurable: bool,
    pub is_accessor: bool,
}

impl PropertyAttrs {
    /// Default attributes for a user-created data property (`{W, E, C}`).
    pub const DATA: Self = Self {
        writable: true,
        enumerable: true,
        configurable: true,
        is_accessor: false,
    };

    /// Attributes for a built-in property (`{¬W, ¬E, ¬C}`).
    pub const BUILTIN: Self = Self {
        writable: false,
        enumerable: false,
        configurable: false,
        is_accessor: false,
    };

    /// Attributes for a built-in method (`{W, ¬E, C}`).
    pub const METHOD: Self = Self {
        writable: true,
        enumerable: false,
        configurable: true,
        is_accessor: false,
    };

    /// Attributes for a writable, non-enumerable, non-configurable data property
    /// (e.g., RegExp `lastIndex`, Function `.prototype`).
    pub const WRITABLE_HIDDEN: Self = Self {
        writable: true,
        enumerable: false,
        configurable: false,
        is_accessor: false,
    };
}

/// Key for the transition table.
///
/// - `Add`: the property does not yet exist on the current Shape.
/// - `Reconfigure`: the property already exists but its attributes are changing.
#[derive(Hash, Eq, PartialEq, Debug)]
pub enum TransitionKey {
    Add(PropertyKey, PropertyAttrs),
    Reconfigure(PropertyKey, PropertyAttrs),
}

/// A Shape (hidden class) describes the property layout of a set of objects.
///
/// All fields are `pub(crate)` — only VM internals access them directly.
pub struct Shape {
    /// Cached child-shape transitions.
    pub(crate) transitions: HashMap<TransitionKey, ShapeId>,
    /// O(1) key → slot index lookup.
    pub(crate) property_map: HashMap<PropertyKey, u16>,
    /// Insertion-ordered (key, attrs) pairs.  `ordered_entries[i]` corresponds
    /// to `Object.slots[i]`.  Attrs here are the single source of truth.
    pub(crate) ordered_entries: Vec<(PropertyKey, PropertyAttrs)>,
}

impl Shape {
    /// Create the root (empty) shape.
    pub fn root() -> Self {
        Self {
            transitions: HashMap::new(),
            property_map: HashMap::new(),
            ordered_entries: Vec::new(),
        }
    }

    /// Number of properties described by this shape.
    #[inline]
    pub fn property_count(&self) -> u16 {
        self.ordered_entries.len() as u16
    }

    /// Look up a property's slot index and attributes.
    #[inline]
    pub fn lookup(&self, key: PropertyKey) -> Option<(u16, PropertyAttrs)> {
        self.property_map.get(&key).map(|&idx| {
            let (_, attrs) = self.ordered_entries[idx as usize];
            (idx, attrs)
        })
    }

    /// Check whether a property exists in this shape.
    #[inline]
    pub fn has(&self, key: PropertyKey) -> bool {
        self.property_map.contains_key(&key)
    }
}
