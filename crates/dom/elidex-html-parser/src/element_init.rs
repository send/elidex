//! Parser-derived ECS component attachment — the canonical derivation
//! point shared by both design-doc §11.3 tiers (strict Tier-1 and
//! tolerant Tier-2) and, subtree-scoped, the future fragment path.
//!
//! One-issue-one-way: the strict and tolerant backends produce identical
//! derived-component sets because both funnel through
//! [`derive_element_components`]. The strict tree-builder
//! (`elidex-html-parser-strict`) stays a pure WHATWG §13.2 tree-builder
//! with no DOM-semantic deps; this module — one layer up, where the
//! `elidex-custom-elements` dep already exists — owns the derivation.
//!
//! Components derived purely from (tag, namespace, attributes):
//! - `CustomElementState` — WHATWG DOM §4.9 "create an element" step
//!   6.3 (HTML namespace ∧ (valid custom element name ∨ `is` non-null)
//!   → state "undefined"), derived by the canonical
//!   `CustomElementState::for_created_element`; name dfns per WHATWG
//!   HTML §4.13.3 "Core concepts". HTML-namespace only: custom element
//!   names are HTML-namespace-scoped, so a hyphenated foreign
//!   (SVG / MathML) local name must not be marked.
//! - [`IframeData`] — WHATWG HTML §4.8.5 `<iframe>`. HTML-namespace only.
//!
//! `FormControlState` is intentionally NOT derived here: it lives in
//! `elidex-form`, which sits above this crate (cycle: elidex-form →
//! elidex-dom-api → elidex-script-session → elidex-html-parser). The
//! parse-path FCS attach is `elidex_form::init_form_controls` (bulk walk
//! invoked from the shell's `build_pipeline_from_loaded`); dynamic
//! insertion is handled by `elidex_form::FormControlReconciler`.
//!
//! `InlineStyle` is likewise NOT derived here: the cascade reads inline
//! declarations from `attrs("style")` directly, and registry-backed
//! properties (`transform` …) need the property registry which lives in
//! `elidex-style` (above this crate). The CSSOM `InlineStyle` component
//! is materialized lazily, registry-aware, on first `el.style.*` access
//! by `elidex_dom_api`'s `ensure_inline_style` — the single
//! materialization point (One-issue-one-way).

use elidex_ecs::{Attributes, EcsDom, Entity, TagType};

/// Attach parser-derived components to every element in `root`'s
/// shadow-inclusive subtree.
///
/// Two-phase (collect entities under the read-only
/// [`EcsDom::for_each_shadow_inclusive_descendant`] walker, then mutate)
/// because the walker borrows `&self` while component insertion needs
/// `&mut`. Mirrors `elidex_form::init_form_controls`. Declarative-shadow
/// content is covered (the walker is shadow-inclusive), so custom
/// elements inside a `<template shadowrootmode>` tree are marked too.
pub(crate) fn derive_element_components(dom: &mut EcsDom, root: Entity) {
    let mut entities = Vec::new();
    dom.for_each_shadow_inclusive_descendant(root, &mut |e| entities.push(e));
    for entity in entities {
        attach_derived(dom, entity);
    }
}

/// Derive and attach components for a single element entity. A no-op for
/// non-element entities (those without a [`TagType`], e.g. the Document
/// node, text, comments).
///
/// Called per-node at element-creation time by the tolerant backend
/// (`convert.rs::convert_node`, before any `append_child` so the
/// components exist when a dispatcher-bound `innerHTML` insert fires) and
/// per-subtree by [`derive_element_components`] for the strict backend.
pub(crate) fn attach_derived(dom: &mut EcsDom, entity: Entity) {
    // `namespace_of` returns `Html` by absence, so plain HTML elements (no
    // `Namespace` component) pass; foreign elements derive neither component.
    let namespace = dom.namespace_of(entity);

    // Phase 1: read (tag, is, attrs) under shared borrows and build the
    // derived-component set via the single canonical derivation (shared with
    // the `elidex_dom_api` `createElement` handler); hold no borrow across the
    // mutating inserts below. The parse-time `is` content attribute IS the is
    // value at creation (DOM §4.9 step 6.3).
    let components = {
        let world = dom.world();
        let Ok(tag_ref) = world.get::<&TagType>(entity) else {
            return; // not an element
        };
        let attrs = world.get::<&Attributes>(entity);
        let is_value = attrs.as_ref().ok().and_then(|a| a.get("is"));
        let empty = Attributes::default();
        let attrs_ref = attrs.as_deref().unwrap_or(&empty);
        elidex_custom_elements::derive_created_element_components(
            &tag_ref.0, namespace, is_value, attrs_ref,
        )
    };

    if let Some(ce_state) = components.custom_element_state {
        let _ = dom.world_mut().insert_one(entity, ce_state);
    }
    if let Some(iframe_data) = components.iframe_data {
        let _ = dom.world_mut().insert_one(entity, iframe_data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_custom_elements::{CEState, CustomElementState};
    use elidex_ecs::{IframeData, InlineStyle, Namespace};

    /// Parse a conforming document through the RAW strict (Tier-1) backend
    /// (no derivation) and run [`derive_element_components`] explicitly, so
    /// these tests exercise the derivation logic in isolation. (The public
    /// `crate::parse_strict` wrapper derives on its own — covered separately
    /// in `lib.rs`.) Returns the dom + document root.
    fn parse_and_derive(html: &str) -> (EcsDom, Entity) {
        let mut result = elidex_html_parser_strict::parse_strict(html).expect("conforming HTML5");
        derive_element_components(&mut result.dom, result.document);
        (result.dom, result.document)
    }

    /// Find the first element with tag `tag` anywhere in `root`'s
    /// shadow-inclusive subtree (namespace-agnostic — matches `TagType`).
    fn find_by_tag(dom: &EcsDom, root: Entity, tag: &str) -> Option<Entity> {
        let mut found = None;
        dom.for_each_shadow_inclusive_descendant(root, &mut |e| {
            if found.is_none() {
                if let Ok(t) = dom.world().get::<&TagType>(e) {
                    if t.0 == tag {
                        found = Some(e);
                    }
                }
            }
        });
        found
    }

    #[test]
    fn strict_autonomous_custom_element_marked_undefined() {
        // The shipped bug: strict-parsed autonomous custom elements never
        // received the `CustomElementState::undefined` marker, so the
        // upgrade consumer ignored them. They must now carry it.
        let (dom, doc) = parse_and_derive(
            "<!DOCTYPE html><html><head></head><body><my-widget></my-widget></body></html>",
        );
        let el = find_by_tag(&dom, doc, "my-widget").expect("my-widget element");
        let ce = dom
            .world()
            .get::<&CustomElementState>(el)
            .expect("CustomElementState attached to strict-parsed custom element");
        assert_eq!(ce.state, CEState::Undefined);
        assert_eq!(ce.definition_name, "my-widget");
    }

    #[test]
    fn strict_customized_builtin_via_is_attr_marked() {
        let (dom, doc) = parse_and_derive(
            r#"<!DOCTYPE html><html><head></head><body><button is="my-btn"></button></body></html>"#,
        );
        let el = find_by_tag(&dom, doc, "button").expect("button element");
        let ce = dom
            .world()
            .get::<&CustomElementState>(el)
            .expect("CustomElementState attached via is= customized built-in");
        assert_eq!(ce.state, CEState::Undefined);
        assert_eq!(ce.definition_name, "my-btn");
    }

    #[test]
    fn strict_inline_style_not_attached_at_parse() {
        // The parser preserves the `style` attribute but does NOT attach
        // an `InlineStyle` component — the CSSOM component materializes
        // lazily (registry-aware) in elidex-dom-api on first access, and
        // the cascade reads `attrs("style")` directly.
        let (dom, doc) = parse_and_derive(
            r#"<!DOCTYPE html><html><head></head><body><div style="color: red"></div></body></html>"#,
        );
        let el = find_by_tag(&dom, doc, "div").expect("div element");
        assert!(
            dom.world().get::<&InlineStyle>(el).is_err(),
            "InlineStyle must not be attached at parse time"
        );
        let attrs = dom.world().get::<&Attributes>(el).unwrap();
        assert_eq!(attrs.get("style"), Some("color: red"));
    }

    #[test]
    fn strict_iframe_data_attached() {
        let (dom, doc) = parse_and_derive(
            r#"<!DOCTYPE html><html><head></head><body><iframe src="x"></iframe></body></html>"#,
        );
        let el = find_by_tag(&dom, doc, "iframe").expect("iframe element");
        assert!(
            dom.world().get::<&IframeData>(el).is_ok(),
            "IframeData attached to strict-parsed <iframe>",
        );
    }

    #[test]
    fn strict_foreign_hyphenated_name_not_marked_custom() {
        // Namespace guard: an SVG-namespaced <my-foo> must NOT receive
        // `CustomElementState` — custom element names are HTML-namespace
        // scoped (WHATWG HTML §4.13.3).
        let (dom, doc) = parse_and_derive(
            "<!DOCTYPE html><html><head></head><body><svg><my-foo></my-foo></svg></body></html>",
        );
        let el = find_by_tag(&dom, doc, "my-foo").expect("my-foo element in svg");
        assert_eq!(dom.namespace_of(el), Namespace::Svg);
        assert!(
            dom.world().get::<&CustomElementState>(el).is_err(),
            "foreign-namespace hyphenated element must not be marked custom",
        );
    }

    #[test]
    fn strict_invalid_is_attr_still_marked_undefined() {
        // DOM §4.9 "create an element" step 6.3: non-null `is` marks
        // the element regardless of name validity — `<button
        // is="notvalid">` is state "undefined" (observably `:defined`
        // non-matching) even though no definition can ever match.
        let (dom, doc) = parse_and_derive(
            r#"<!DOCTYPE html><html><head></head><body><button is="notvalid"></button></body></html>"#,
        );
        let el = find_by_tag(&dom, doc, "button").expect("button element");
        let ce = dom
            .world()
            .get::<&CustomElementState>(el)
            .expect("non-null is must mark Undefined (validity-free per step 6.3)");
        assert_eq!(ce.state, CEState::Undefined);
        assert_eq!(ce.definition_name, "notvalid");
    }
}
