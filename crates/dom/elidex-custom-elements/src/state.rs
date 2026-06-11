//! Custom element state tracking (WHATWG HTML §4.13.3 "Core concepts")
//! and the canonical creation-time state derivation (WHATWG DOM §4.9
//! "create an element" step 6.3).

use elidex_ecs::Namespace;

use crate::validation::is_valid_custom_element_name;

/// Custom element lifecycle state per WHATWG HTML §4.13.3.
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

    /// The element's *is value* slot view (DOM §4.9 "create an
    /// element" / HTML §13.3): `Some(definition_name)` when this state
    /// marks a **customized built-in** (the definition name differs
    /// from the element's local name — autonomous custom elements key
    /// on the tag itself and have a null is value in this data model),
    /// else `None`.
    ///
    /// This is the single home of the "customized built-in ⇔
    /// definition_name ≠ local name" discriminator — consumers
    /// (serializer is-value compensation, engine upgrade routing)
    /// must not inline the comparison.  KNOWN CONFLATION: the
    /// component stores one name, so `createElement('my-el',
    /// {is:'my-other'})` loses the is value to the autonomous branch
    /// (slot `#11-custom-element-is-value-slot-separation`).
    #[must_use]
    pub fn is_value(&self, local_name: &str) -> Option<&str> {
        (self.definition_name != local_name).then_some(self.definition_name.as_str())
    }

    /// The canonical creation-time custom-element-state derivation,
    /// shared by every element-creation path (parser `element_init`,
    /// `document.createElement` handler).
    ///
    /// Implements WHATWG DOM §4.9 "create an element" step 6.3: *"If
    /// namespace is the HTML namespace, and either localName is a
    /// valid custom element name or is is non-null, then set result's
    /// custom element state to 'undefined'."*
    ///
    /// - `local_name` must already be lower-cased by the caller (the
    ///   creation path owns tag canonicalization).
    /// - `is_value` is the *is* value from markup (`is=` content
    ///   attribute at parse time) or `ElementCreationOptions.is` —
    ///   per the spec condition its **validity is not required**: an
    ///   invalid non-null `is` still yields `Undefined` (the element
    ///   simply never matches a definition, observably `:defined`
    ///   non-matching), because `customElements.define()` rejects
    ///   invalid names so no lookup can ever succeed.
    /// - Foreign (SVG / MathML) elements never get a state — custom
    ///   element names are HTML-namespace-scoped.
    ///
    /// Returns the `Undefined` state carrying the definition name the
    /// upgrade machinery keys on: the local name for autonomous
    /// custom elements, the *is* value for customized built-ins.
    ///
    /// Cloning never calls this: per DOM §4.4 "clone a single node"
    /// step 2.4 a clone *propagates* the source's is value (the
    /// existing component's `definition_name`) rather than
    /// re-deriving from attributes.
    #[must_use]
    pub fn for_created_element(
        local_name: &str,
        is_value: Option<&str>,
        namespace: Namespace,
    ) -> Option<Self> {
        if namespace != Namespace::Html {
            return None;
        }
        if is_valid_custom_element_name(local_name) {
            return Some(Self::undefined(local_name));
        }
        is_value.map(Self::undefined)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn for_created_element_autonomous_valid_name() {
        let ce = CustomElementState::for_created_element("my-el", None, Namespace::Html)
            .expect("valid custom element name marks Undefined");
        assert_eq!(ce.state, CEState::Undefined);
        assert_eq!(ce.definition_name, "my-el");
    }

    #[test]
    fn for_created_element_customized_builtin_valid_is() {
        let ce = CustomElementState::for_created_element("button", Some("my-btn"), Namespace::Html)
            .expect("non-null is marks Undefined");
        assert_eq!(ce.definition_name, "my-btn");
    }

    #[test]
    fn for_created_element_invalid_is_still_marks_undefined() {
        // DOM §4.9 "create an element" step 6.3 requires only that
        // `is` be non-null — validity is NOT a condition. The element
        // simply never upgrades (define() rejects invalid names).
        let ce =
            CustomElementState::for_created_element("button", Some("x-invalid"), Namespace::Html)
                .expect("invalid but non-null is still marks Undefined");
        assert_eq!(ce.state, CEState::Undefined);
        assert_eq!(ce.definition_name, "x-invalid");
    }

    #[test]
    fn for_created_element_foreign_namespace_never_marks() {
        assert!(
            CustomElementState::for_created_element("my-foo", None, Namespace::Svg).is_none(),
            "custom element names are HTML-namespace-scoped"
        );
        assert!(
            CustomElementState::for_created_element("circle", Some("my-x"), Namespace::Svg)
                .is_none()
        );
    }

    #[test]
    fn for_created_element_plain_builtin_no_is_unmarked() {
        assert!(CustomElementState::for_created_element("div", None, Namespace::Html).is_none());
    }

    #[test]
    fn for_created_element_valid_name_wins_over_is() {
        // Autonomous branch takes precedence: a hyphenated local name
        // keys the definition on the tag itself.
        let ce =
            CustomElementState::for_created_element("my-el", Some("my-other"), Namespace::Html)
                .expect("valid local name marks autonomous");
        assert_eq!(ce.definition_name, "my-el");
    }
}
