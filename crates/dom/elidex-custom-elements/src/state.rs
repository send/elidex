//! Custom element state tracking (WHATWG HTML §4.13.2).

/// Custom element lifecycle state per WHATWG HTML §4.13.2.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum CEState {
    /// Element created before `customElements.define()` was called.
    #[default]
    Undefined,
    /// Constructor invocation threw an error during upgrade.
    Failed,
    /// Built-in element not associated with a custom element definition.
    Uncustomized,
    /// Temporary state during constructor execution (upgrade in progress).
    Precustomized,
    /// Element fully initialized with lifecycle callbacks active.
    Custom,
}

/// ECS component tracking the custom element state of an entity.
///
/// Attached to elements whose tag name is a valid custom element name
/// (contains a hyphen) or that have an `is` attribute.
#[derive(Clone, Debug)]
pub struct CustomElementState {
    /// Current lifecycle state.
    pub state: CEState,
    /// The custom element definition name (e.g., "my-element").
    pub definition_name: String,
}

impl CustomElementState {
    /// Create a new state in `Undefined` (awaiting upgrade).
    #[must_use]
    pub fn undefined(name: impl Into<String>) -> Self {
        Self {
            state: CEState::Undefined,
            definition_name: name.into(),
        }
    }

    /// Create a new state in `Custom` (fully upgraded).
    #[must_use]
    pub fn custom(name: impl Into<String>) -> Self {
        Self {
            state: CEState::Custom,
            definition_name: name.into(),
        }
    }
}
