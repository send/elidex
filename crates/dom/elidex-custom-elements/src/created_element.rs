//! WHATWG DOM "create an element" intrinsic-component derivation — the
//! single canonical `(local name, namespace, is, attributes) → components`
//! projection shared by the two element-creation paths:
//! `elidex_html_parser::element_init::attach_derived` (the parser) and the
//! `elidex_dom_api` `createElement` handler.
//!
//! One-issue-one-way: both paths attach an identical derived-component set
//! rather than each hand-rolling its own subset. The pure derivation lives
//! here in `elidex-custom-elements` because it is the lowest crate that both
//! sees [`IframeData`] (via its `elidex-ecs` dep) and owns
//! [`CustomElementState`], and is depended on by both consumers.
//!
//! Components derived purely from `(local_name, namespace, is, attributes)`:
//! - [`CustomElementState`] — WHATWG DOM §4.9 "create an element" step 6.3
//!   (HTML namespace ∧ (valid custom element name ∨ non-null `is`) → state
//!   "undefined"), via the canonical [`CustomElementState::for_created_element`].
//! - [`IframeData`] — WHATWG HTML §4.8.5 "the iframe element". HTML-namespace
//!   `<iframe>` only.
//!
//! `FormControlState` and `InlineStyle` are deliberately NOT derived here
//! (see `elidex_html_parser::element_init` module doc): `FormControlState`
//! is attached on *insertion* by `elidex_form::FormControlReconciler`;
//! `InlineStyle` is lazily materialized on first `el.style.*` access by
//! `elidex_dom_api`. So the creation-time intrinsic set is exactly
//! **CustomElementState + IframeData**.

use elidex_ecs::{Attributes, IframeData, Namespace};

use crate::state::CustomElementState;

/// The intrinsic ECS components derived at element-creation time. Either may
/// be `None` (e.g. a plain `<div>` derives neither; a foreign-namespace
/// element derives neither).
#[derive(Debug, Default)]
pub struct CreatedElementComponents {
    /// WHATWG DOM §4.9 custom-element state — set for HTML-namespace custom
    /// (or `is`-bearing) elements.
    pub custom_element_state: Option<CustomElementState>,
    /// WHATWG HTML §4.8.5 iframe projection — set for an HTML-namespace
    /// `<iframe>`.
    pub iframe_data: Option<IframeData>,
}

/// Derive the creation-time intrinsic components for an element from its
/// `(local_name, namespace, is, attributes)`.
///
/// Pure and HTML-namespace-scoped: a non-HTML-namespace element (a foreign
/// SVG / MathML look-alike) derives neither component. `is` is the element's
/// `is` value at creation (the parser reads it from the `is` content
/// attribute; `createElement` takes it from the options argument). `attrs`
/// feeds [`IframeData::from_attributes`]; on the `createElement` path the
/// attribute set is empty at creation, so an `<iframe>` derives
/// `IframeData::default()` — a *present* component (the point), which the
/// `setAttribute` reconcile seam then populates.
#[must_use]
pub fn derive_created_element_components(
    local_name: &str,
    namespace: Namespace,
    is_value: Option<&str>,
    attrs: &Attributes,
) -> CreatedElementComponents {
    // Both components are HTML-namespace-scoped; bail early for foreign
    // elements (mirrors the single namespace gate the parser applied).
    if namespace != Namespace::Html {
        return CreatedElementComponents::default();
    }
    CreatedElementComponents {
        custom_element_state: CustomElementState::for_created_element(
            local_name, is_value, namespace,
        ),
        iframe_data: (local_name == "iframe").then(|| IframeData::from_attributes(attrs)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::CEState;

    fn attrs(pairs: &[(&str, &str)]) -> Attributes {
        let mut a = Attributes::default();
        for (k, v) in pairs {
            a.set((*k).to_string(), (*v).to_string());
        }
        a
    }

    #[test]
    fn plain_html_element_derives_neither() {
        let c =
            derive_created_element_components("div", Namespace::Html, None, &Attributes::default());
        assert!(c.custom_element_state.is_none());
        assert!(c.iframe_data.is_none());
    }

    #[test]
    fn autonomous_custom_element_derives_ce_state() {
        let c = derive_created_element_components(
            "my-el",
            Namespace::Html,
            None,
            &Attributes::default(),
        );
        let ce = c
            .custom_element_state
            .expect("custom element name marks CE state");
        assert_eq!(ce.state, CEState::Undefined);
        assert!(c.iframe_data.is_none());
    }

    #[test]
    fn is_value_derives_ce_state_on_builtin() {
        let c = derive_created_element_components(
            "p",
            Namespace::Html,
            Some("my-p"),
            &Attributes::default(),
        );
        assert!(
            c.custom_element_state.is_some(),
            "non-null is marks a builtin"
        );
    }

    #[test]
    fn iframe_with_empty_attrs_derives_present_default_iframe_data() {
        // createElement path: empty attrs at creation → a PRESENT (default)
        // IframeData so the setAttribute reconcile seam can later populate it.
        let c = derive_created_element_components(
            "iframe",
            Namespace::Html,
            None,
            &Attributes::default(),
        );
        let data = c
            .iframe_data
            .expect("iframe derives IframeData even with no attrs");
        // Present but unpopulated — no src/srcdoc until setAttribute fires.
        assert!(data.src.is_none());
        assert!(data.srcdoc.is_none());
        assert!(c.custom_element_state.is_none());
    }

    #[test]
    fn iframe_with_attrs_projects_src_srcdoc() {
        // parser path: real attributes project into IframeData.
        let c = derive_created_element_components(
            "iframe",
            Namespace::Html,
            None,
            &attrs(&[("src", "child.html")]),
        );
        let data = c.iframe_data.expect("iframe derives IframeData");
        assert_eq!(data.src.as_deref(), Some("child.html"));
    }

    #[test]
    fn foreign_namespace_derives_neither() {
        // An SVG/MathML look-alike (e.g. a foreign `<iframe>` or hyphenated
        // local name) derives neither HTML-namespace-scoped component.
        let c = derive_created_element_components(
            "iframe",
            Namespace::Svg,
            None,
            &Attributes::default(),
        );
        assert!(c.custom_element_state.is_none());
        assert!(c.iframe_data.is_none());
        let c2 = derive_created_element_components(
            "my-el",
            Namespace::MathMl,
            None,
            &Attributes::default(),
        );
        assert!(c2.custom_element_state.is_none());
    }
}
