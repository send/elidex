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
    /// The custom element definition name the upgrade machinery keys
    /// on: the local name for autonomous custom elements, the *is*
    /// value for customized built-ins.
    pub definition_name: String,
    /// The element's *is value* slot (DOM §4.9 "create an element" —
    /// set from `ElementCreationOptions.is` / the parse-time `is`
    /// content attribute, immutable thereafter). Kept SEPARATE from
    /// `definition_name`: an autonomous custom element created with a
    /// non-null `is` retains it here (the autonomous branch keys the
    /// definition on the tag, but HTML §13.3 serialization must still
    /// emit the is value), and an is value equal to the local name is
    /// still non-null and must serialize.
    pub is_value: Option<String>,
}

impl CustomElementState {
    /// Create a new state in `Undefined` (awaiting upgrade).
    #[must_use]
    pub fn undefined(name: impl Into<String>) -> Self {
        Self {
            state: CEState::Undefined,
            definition_name: name.into(),
            is_value: None,
        }
    }

    /// Create a new state in `Custom` (fully upgraded).
    #[must_use]
    pub fn custom(name: impl Into<String>) -> Self {
        Self {
            state: CEState::Custom,
            definition_name: name.into(),
            is_value: None,
        }
    }

    /// The element's *is value* slot (DOM §4.9): the raw `is` the
    /// element was created with, or `None`. This is what HTML §13.3
    /// serialization emits — including when it equals the local name
    /// or coexists with a valid autonomous tag (the slot is
    /// independent of which name keys the upgrade).
    #[must_use]
    pub fn is_value(&self) -> Option<&str> {
        self.is_value.as_deref()
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
    /// step 2.4 a clone *propagates* the source's slots (the existing
    /// component's `definition_name` + `is_value`) rather than
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
            // Autonomous: the tag keys the definition; a non-null is
            // value is still retained in its own slot (DOM §4.9 sets
            // it independently; HTML §13.3 serializes it).
            return Some(Self {
                state: CEState::Undefined,
                definition_name: local_name.to_string(),
                is_value: is_value.map(str::to_string),
            });
        }
        is_value.map(|is| Self {
            state: CEState::Undefined,
            definition_name: is.to_string(),
            is_value: Some(is.to_string()),
        })
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
            CustomElementState::for_created_element("button", Some("notvalid"), Namespace::Html)
                .expect("invalid but non-null is still marks Undefined");
        assert_eq!(ce.state, CEState::Undefined);
        assert_eq!(ce.definition_name, "notvalid");
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
    fn for_created_element_valid_name_keys_definition_but_keeps_is() {
        // Autonomous branch keys the definition on the tag itself,
        // while the non-null is value is retained in its own slot
        // (DOM §4.9 sets the is value independently of step 6.3).
        let ce =
            CustomElementState::for_created_element("my-el", Some("my-other"), Namespace::Html)
                .expect("valid local name marks autonomous");
        assert_eq!(ce.definition_name, "my-el");
        assert_eq!(ce.is_value(), Some("my-other"));
    }

    #[test]
    fn for_created_element_is_equal_to_local_name_is_still_non_null() {
        // `createElement("button", {is: "button"})`: the is value is
        // non-null regardless of equality with the local name — HTML
        // §13.3 must serialize it, so the slot must record it.
        let ce = CustomElementState::for_created_element("button", Some("button"), Namespace::Html)
            .expect("non-null is marks Undefined");
        assert_eq!(ce.is_value(), Some("button"));
    }
}
