//! Node-classification queries for [`EcsDom`] — "what kind of node is this?".
//!
//! Extracted from `dom/mod.rs` when it was over the 1000-line review
//! convention (CLAUDE.md "1000-line debt = touch-time split").  The seam is
//! the read-only brand / kind / namespace / tag-name predicates that answer a
//! classification question about a single entity:
//!
//! - kind: [`EcsDom::node_kind`], [`EcsDom::node_kind_inferred`],
//!   [`EcsDom::is_element`], [`EcsDom::is_document_fragment`]
//! - namespace: [`EcsDom::namespace_of`], [`EcsDom::is_html_namespace`]
//! - HTML tag-name predicates: [`EcsDom::is_base_element`],
//!   [`EcsDom::is_template_element`]
//!
//! Boundary: node factories (`create_*`), tree/attribute mutation, and
//! queries that merely *use* a classification (e.g. `is_contenteditable`,
//! `resolve_attribute_qname`) stay in the parent module. Two predicates of
//! the same shape live in `tree/navigation.rs`, not here: `is_shadow_root`
//! (which `is_document_fragment` calls) and `has_tag` (a case-sensitive
//! tag-name match, unlike the ASCII case-insensitive ones above).

use hecs::Entity;

use super::EcsDom;
use crate::components::{CommentData, DocTypeData, Namespace, NodeKind, TagType, TextContent};

impl EcsDom {
    /// HTML tag-name predicate: returns `true` iff `entity` is an
    /// `<base>` element (HTML §4.2.3).  Used by event consumers (e.g.
    /// `BaseUrlMaintainer` in `elidex-dom-api`) to filter
    /// `MutationEvent::AttributeChange` events to `<base>.href`
    /// writes only.
    ///
    /// HTML-spec knowledge co-located with the `shadow.rs` tag-whitelist
    /// precedent (`VALID_SHADOW_HOST_TAGS` and `is_valid_shadow_host`) per
    /// layering mandate (small HTML predicates in elidex-ecs are OK; large
    /// algorithms hoist to elidex-dom-api).
    #[must_use]
    pub fn is_base_element(&self, entity: Entity) -> bool {
        // WHATWG HTML §4.2.3 restricts `<base>` to the HTML namespace, so
        // a foreign `<base>` (SVG / MathML) is not the document base.
        self.world
            .get::<&TagType>(entity)
            .is_ok_and(|t| t.0.eq_ignore_ascii_case("base"))
            && self.is_html_namespace(entity)
    }

    /// HTML tag-name predicate: returns `true` iff `entity` is a
    /// `<template>` element (HTML §4.12.3). Tree walkers implementing the
    /// §2.4.3 "first base element in the document" carve-out use this to
    /// **skip recursing into a `<template>`'s light children** — a `<base>`
    /// programmatically appended to a template element (e.g. via
    /// `template.appendChild(base)`) must not affect the host document's base
    /// URL. (Parsed template *content* lands in the detached
    /// [`TemplateContents`](crate::TemplateContents) fragment, which no
    /// light-tree walk reaches anyway; this predicate covers the light-child
    /// case the fragment model does not.)
    ///
    /// Tag-string compare matches the [`Self::is_base_element`] precedent.
    #[must_use]
    pub fn is_template_element(&self, entity: Entity) -> bool {
        self.world
            .get::<&TagType>(entity)
            .is_ok_and(|t| t.0.eq_ignore_ascii_case("template"))
    }

    /// The [`Namespace`] of an element entity.
    ///
    /// Returns the element's explicit [`Namespace`] component if present,
    /// otherwise [`Namespace::Html`] — the default-by-absence sentinel
    /// (most elements are HTML, so the component marks only the foreign
    /// exception).  For non-element entities the value is meaningless;
    /// callers gate on [`is_element`](Self::is_element) first, as
    /// [`Self::is_html_namespace`](Self::is_html_namespace) does.
    #[must_use]
    pub fn namespace_of(&self, entity: Entity) -> Namespace {
        self.world
            .get::<&Namespace>(entity)
            .map_or(Namespace::Html, |n| *n)
    }

    /// Returns `true` if the entity is an element node.
    ///
    /// Checks `NodeKind` first, falls back to `TagType` presence for
    /// backwards compatibility with entities created before `NodeKind` was added.
    #[must_use]
    pub fn is_element(&self, entity: Entity) -> bool {
        if let Ok(kind) = self.world.get::<&NodeKind>(entity) {
            return *kind == NodeKind::Element;
        }
        self.world.get::<&TagType>(entity).is_ok()
    }

    /// Returns the `NodeKind` of an entity, if it has one.
    #[must_use]
    pub fn node_kind(&self, entity: Entity) -> Option<NodeKind> {
        self.world.get::<&NodeKind>(entity).ok().map(|k| *k)
    }

    /// Returns `true` if the entity is a `DocumentFragment` node — and is **not**
    /// a [`ShadowRoot`](crate::ShadowRoot).
    ///
    /// The canonical brand-check for the kind whose children are *expanded* into
    /// the tree by the WHATWG DOM §4.2.3 "insert" algorithm (step 1) rather than
    /// the node itself being linked (mirrors [`is_element`](Self::is_element)).
    ///
    /// **ShadowRoot exclusion**: a shadow root is spawned with
    /// [`NodeKind::DocumentFragment`](crate::NodeKind::DocumentFragment) (it
    /// inherits the `DocumentFragment` interface per DOM), but it is **not** an
    /// expandable fragment — it must never be moved/expanded into a tree (it
    /// violates the ShadowRoot non-move invariant + §4.2.3 host-including-ancestor
    /// handling). Excluding it here keeps every shared apply-layer entry point
    /// (`apply_*` / `DomApiHandler`, not just the VM `node_proto` frontend guard)
    /// from treating a shadow root as a fragment.
    #[must_use]
    pub fn is_document_fragment(&self, entity: Entity) -> bool {
        matches!(self.node_kind(entity), Some(NodeKind::DocumentFragment))
            && !self.is_shadow_root(entity)
    }

    /// Whether `entity` is an element in the HTML namespace.
    ///
    /// `true` iff `entity` is an element AND its [`namespace_of`] is
    /// [`Namespace::Html`] (the default-by-absence case — most elements
    /// carry no [`Namespace`] component). Foreign (SVG / MathML) elements
    /// created via [`create_element_ns`](Self::create_element_ns) return
    /// `false`; non-elements return `false` (defensive).
    ///
    /// Used by `Range::create_contextual_fragment` in `elidex-dom-api`
    /// (HTML §8.5.7 `createContextualFragment()` step 6) to gate the
    /// `<html>`-as-context override (only HTML-namespace `<html>` is
    /// rewritten to `<body>` for parser-scope selection; SVG / MathML
    /// `<html>` must stay as-is), and by the `<base>` / `<datalist>`
    /// element predicates which are HTML-namespace-restricted (HTML
    /// §4.2.3 / §4.10.8).
    ///
    /// [`namespace_of`]: Self::namespace_of
    #[must_use]
    pub fn is_html_namespace(&self, entity: Entity) -> bool {
        self.is_element(entity) && self.namespace_of(entity) == Namespace::Html
    }

    /// Effective `NodeKind` — returns the explicit component when
    /// present, otherwise infers from payload components for legacy
    /// entities that predate the `NodeKind` component (same rules as
    /// [`clone_node_shallow`](Self::clone_node_shallow) and
    /// `HostData::prototype_kind_for`): `TagType` ⇒ `Element`,
    /// `TextContent` ⇒ `Text`, `CommentData` ⇒ `Comment`,
    /// `DocTypeData` ⇒ `DocumentType`.  Returns `None` only when no
    /// kind component and no DOM payload is present.
    ///
    /// Use this in any code path that has to treat legacy entities
    /// as real DOM nodes — e.g. `splitText` brand checks,
    /// `isEqualNode` equality, variadic argument normalisation.
    #[must_use]
    pub fn node_kind_inferred(&self, entity: Entity) -> Option<NodeKind> {
        if let Some(kind) = self.node_kind(entity) {
            return Some(kind);
        }
        if self.world.get::<&TagType>(entity).is_ok() {
            return Some(NodeKind::Element);
        }
        if self.world.get::<&TextContent>(entity).is_ok() {
            return Some(NodeKind::Text);
        }
        if self.world.get::<&CommentData>(entity).is_ok() {
            return Some(NodeKind::Comment);
        }
        if self.world.get::<&DocTypeData>(entity).is_ok() {
            return Some(NodeKind::DocumentType);
        }
        None
    }
}
