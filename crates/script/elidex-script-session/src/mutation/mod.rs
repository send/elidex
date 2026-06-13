//! Buffered DOM mutations and their application to the ECS DOM.
//!
//! The WHATWG HTML §8.5 HTML-fragment setters (`innerHTML` / `setHTMLUnsafe` /
//! `outerHTML` / `insertAdjacentHTML`) live in the [`html_fragment`] submodule;
//! everything else (the [`Mutation`] queue + the generic `apply_*` node
//! mutations) stays here.

use elidex_ecs::{Attributes, EcsDom, Entity, TextContent};

mod html_fragment;
use html_fragment::apply_insert_adjacent_html;
pub use html_fragment::{
    apply_set_inner_html, apply_set_outer_html, OuterHtmlError, SetInnerHtmlOptions,
};

/// A buffered DOM mutation recorded by script code.
///
/// Mutations are collected in [`SessionCore`](crate::SessionCore) and applied
/// atomically via [`flush()`](crate::SessionCore::flush).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Mutation {
    /// Append `child` as the last child of `parent`.
    AppendChild {
        /// Parent entity.
        parent: Entity,
        /// Child entity to append.
        child: Entity,
    },
    /// Insert `new_child` before `ref_child` under `parent`.
    InsertBefore {
        /// Parent entity.
        parent: Entity,
        /// New child entity to insert.
        new_child: Entity,
        /// Existing child to insert before.
        ref_child: Entity,
    },
    /// Remove `child` from `parent`.
    RemoveChild {
        /// Parent entity.
        parent: Entity,
        /// Child entity to remove.
        child: Entity,
    },
    /// Replace `old_child` with `new_child` under `parent`.
    ReplaceChild {
        /// Parent entity.
        parent: Entity,
        /// New child entity.
        new_child: Entity,
        /// Old child entity to replace.
        old_child: Entity,
    },
    /// Set an attribute on an element.
    SetAttribute {
        /// Target entity.
        entity: Entity,
        /// Attribute name.
        name: String,
        /// Attribute value.
        value: String,
    },
    /// Remove an attribute from an element.
    RemoveAttribute {
        /// Target entity.
        entity: Entity,
        /// Attribute name to remove.
        name: String,
    },
    /// Set the text content of a text node.
    ///
    /// Currently only updates the [`TextContent`](elidex_ecs::TextContent)
    /// component directly. Full DOM `textContent` setter semantics for
    /// element nodes (removing all children and inserting a single text
    /// node) will be implemented in a later milestone.
    SetTextContent {
        /// Target entity.
        entity: Entity,
        /// New text content.
        text: String,
    },
    /// Set innerHTML — parses HTML fragment and replaces children.
    SetInnerHtml {
        /// Target entity.
        entity: Entity,
        /// HTML string to parse and insert.
        html: String,
    },
    /// Insert parsed HTML at a position relative to an element.
    ///
    /// Position: `"beforebegin"`, `"afterbegin"`, `"beforeend"`, `"afterend"`.
    InsertAdjacentHtml {
        /// Target entity.
        entity: Entity,
        /// Insertion position.
        position: String,
        /// HTML string to parse and insert.
        html: String,
    },
    /// Insert a CSS rule into a stylesheet (legacy variant, CSSOM uses bridge).
    InsertCssRule {
        /// Stylesheet entity.
        stylesheet: Entity,
        /// Index at which to insert.
        index: usize,
        /// CSS rule text.
        rule: String,
    },
    /// Delete a CSS rule from a stylesheet (legacy variant, CSSOM uses bridge).
    DeleteCssRule {
        /// Stylesheet entity.
        stylesheet: Entity,
        /// Index of the rule to delete.
        index: usize,
    },
}

/// The kind of mutation that was applied, for observer notifications.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MutationKind {
    /// A child was added, removed, or replaced.
    ChildList,
    /// An attribute was set or removed.
    Attribute,
    /// Text content was changed.
    CharacterData,
    /// A CSS rule was inserted or deleted.
    CssRule,
}

/// Record of a successfully applied mutation (WHATWG DOM §4.3.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutationRecord {
    /// The kind of mutation.
    pub kind: MutationKind,
    /// The primary target entity.
    pub target: Entity,
    /// Nodes added (for `ChildList` mutations).
    pub added_nodes: Vec<Entity>,
    /// Nodes removed (for `ChildList` mutations).
    pub removed_nodes: Vec<Entity>,
    /// The previous sibling of the mutation site.
    pub previous_sibling: Option<Entity>,
    /// The next sibling of the mutation site.
    pub next_sibling: Option<Entity>,
    /// The attribute name (for `Attribute` mutations).
    pub attribute_name: Option<String>,
    /// The old value (for `Attribute` or `CharacterData` mutations when requested).
    pub old_value: Option<String>,
}

/// Apply a single [`Mutation`] to the ECS DOM.
///
/// This is a low-level function. Prefer recording mutations via
/// [`SessionCore::record_mutation()`](crate::SessionCore::record_mutation)
/// and applying them with [`SessionCore::flush()`](crate::SessionCore::flush)
/// to ensure consistent buffering and future `MutationObserver` support.
///
/// Returns `Some(MutationRecord)` on success, `None` if the operation failed
/// (e.g. entity not found, tree constraint violation, or stub operation).
pub fn apply_mutation(mutation: &Mutation, dom: &mut EcsDom) -> Option<MutationRecord> {
    match mutation {
        Mutation::AppendChild { parent, child } => apply_append_child(dom, *parent, *child),
        Mutation::InsertBefore {
            parent,
            new_child,
            ref_child,
        } => apply_insert_before(dom, *parent, *new_child, *ref_child),
        Mutation::RemoveChild { parent, child } => apply_remove_child(dom, *parent, *child),
        Mutation::ReplaceChild {
            parent,
            new_child,
            old_child,
        } => apply_replace_child(dom, *parent, *new_child, *old_child),
        Mutation::SetAttribute {
            entity,
            name,
            value,
        } => apply_set_attribute(dom, *entity, name, value),
        Mutation::RemoveAttribute { entity, name } => apply_remove_attribute(dom, *entity, name),
        Mutation::SetTextContent { entity, text } => apply_set_text(dom, *entity, text),
        Mutation::SetInnerHtml { entity, html } => {
            apply_set_inner_html(dom, *entity, html, SetInnerHtmlOptions::default())
        }
        Mutation::InsertAdjacentHtml {
            entity,
            position,
            html,
        } => apply_insert_adjacent_html(dom, *entity, position, html),
        // CSS rule mutations are handled directly by the HostBridge CSSOM layer
        // (not through the EcsDom mutation system). These variants are kept for
        // backward compat but are no longer reached in normal operation.
        Mutation::InsertCssRule { .. } | Mutation::DeleteCssRule { .. } => None,
    }
}

pub(super) fn empty_record(kind: MutationKind, target: Entity) -> MutationRecord {
    MutationRecord {
        kind,
        target,
        added_nodes: Vec::new(),
        removed_nodes: Vec::new(),
        previous_sibling: None,
        next_sibling: None,
        attribute_name: None,
        old_value: None,
    }
}

fn apply_append_child(dom: &mut EcsDom, parent: Entity, child: Entity) -> Option<MutationRecord> {
    // Capture previous sibling before mutation (the current last
    // exposed child). `get_last_child` would return a `ShadowRoot`
    // entity on a shadow host with no light-tree children, leaking
    // it via `MutationRecord.previousSibling`; `children_iter_rev`
    // skips internal ShadowRoot entities so the captured sibling
    // matches the DOM-visible chain — same encapsulation invariant
    // the insert/remove/replace paths enforce via
    // `prev_exposed_sibling` / `next_exposed_sibling`.
    let prev_sibling = dom.children_iter_rev(parent).next();
    if !dom.append_child(parent, child) {
        return None;
    }
    Some(MutationRecord {
        added_nodes: vec![child],
        previous_sibling: prev_sibling,
        ..empty_record(MutationKind::ChildList, parent)
    })
}

fn apply_insert_before(
    dom: &mut EcsDom,
    parent: Entity,
    new_child: Entity,
    ref_child: Entity,
) -> Option<MutationRecord> {
    // Sibling fields surface to JS via MutationObserver; use the
    // exposed-sibling helpers so a closed `ShadowRoot` (which IS a real
    // ECS sibling but is filtered out of the DOM children view per
    // §4.8 encapsulation) cannot leak as `previousSibling`/`nextSibling`.
    let prev_sibling = dom.prev_exposed_sibling(ref_child);
    if !dom.insert_before(parent, new_child, ref_child) {
        return None;
    }
    Some(MutationRecord {
        added_nodes: vec![new_child],
        previous_sibling: prev_sibling,
        next_sibling: Some(ref_child),
        ..empty_record(MutationKind::ChildList, parent)
    })
}

fn apply_remove_child(dom: &mut EcsDom, parent: Entity, child: Entity) -> Option<MutationRecord> {
    let prev_sibling = dom.prev_exposed_sibling(child);
    let next_sibling = dom.next_exposed_sibling(child);
    if !dom.remove_child(parent, child) {
        return None;
    }
    Some(MutationRecord {
        removed_nodes: vec![child],
        previous_sibling: prev_sibling,
        next_sibling,
        ..empty_record(MutationKind::ChildList, parent)
    })
}

fn apply_replace_child(
    dom: &mut EcsDom,
    parent: Entity,
    new_child: Entity,
    old_child: Entity,
) -> Option<MutationRecord> {
    let prev_sibling = dom.prev_exposed_sibling(old_child);
    let next_sibling = dom.next_exposed_sibling(old_child);
    if !dom.replace_child(parent, new_child, old_child) {
        return None;
    }
    Some(MutationRecord {
        added_nodes: vec![new_child],
        removed_nodes: vec![old_child],
        previous_sibling: prev_sibling,
        next_sibling,
        ..empty_record(MutationKind::ChildList, parent)
    })
}

fn apply_set_attribute(
    dom: &mut EcsDom,
    entity: Entity,
    name: &str,
    value: &str,
) -> Option<MutationRecord> {
    let mut attrs = dom.world_mut().get::<&mut Attributes>(entity).ok()?;
    let name = name.to_ascii_lowercase();
    let old_value = attrs.get(&name).map(str::to_owned);
    attrs.set(name.clone(), value.to_owned());
    drop(attrs);
    // This deferred-flush path mutates `Attributes` directly instead of
    // entering `EcsDom::set_attribute`, so it must run that chokepoint's
    // attribute-derived-component reconcile: drop a stale `InlineStyle` on a
    // buffered `style` write (else a later CSSOM write could resurrect the old
    // declarations — Codex #335 R10 F31) AND re-derive `IframeData` on a
    // buffered iframe-attribute write (else a flushed `setAttribute("src", …)`
    // would leave the component stale).
    dom.reconcile_attribute_derived_components(entity, &name);
    dom.rev_version(entity);
    Some(MutationRecord {
        attribute_name: Some(name),
        old_value,
        ..empty_record(MutationKind::Attribute, entity)
    })
}

fn apply_remove_attribute(dom: &mut EcsDom, entity: Entity, name: &str) -> Option<MutationRecord> {
    let mut attrs = dom.world_mut().get::<&mut Attributes>(entity).ok()?;
    let name = name.to_ascii_lowercase();
    let old_value = attrs.get(&name).map(str::to_owned);
    attrs.remove(&name);
    drop(attrs);
    // Same attribute-derived-component reconcile as `apply_set_attribute`
    // (Codex #335 R10 F31) — a buffered `removeAttribute("style")` drops the
    // hydrated `InlineStyle`, and a buffered iframe-attribute removal re-derives
    // `IframeData`.
    dom.reconcile_attribute_derived_components(entity, &name);
    dom.rev_version(entity);
    Some(MutationRecord {
        attribute_name: Some(name),
        old_value,
        ..empty_record(MutationKind::Attribute, entity)
    })
}

fn apply_set_text(dom: &mut EcsDom, entity: Entity, text: &str) -> Option<MutationRecord> {
    let old_value = dom
        .world()
        .get::<&TextContent>(entity)
        .ok()
        .map(|tc| tc.0.clone());
    // `set_text_data` bumps `rev_version(entity)` internally, so we
    // do not call it here.
    dom.set_text_data(entity, text)?;
    Some(MutationRecord {
        old_value,
        ..empty_record(MutationKind::CharacterData, entity)
    })
}

#[cfg(test)]
#[allow(unused_must_use)] // Test setup calls dom.append_child() etc. without checking return values
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    fn elem(dom: &mut EcsDom, tag: &str) -> Entity {
        dom.create_element(tag, Attributes::default())
    }

    #[test]
    fn apply_append_child() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let child = elem(&mut dom, "span");

        let m = Mutation::AppendChild { parent, child };
        let record = apply_mutation(&m, &mut dom).expect("should succeed");
        assert_eq!(record.kind, MutationKind::ChildList);
        assert_eq!(record.target, parent);
        assert_eq!(record.added_nodes, vec![child]);
        assert!(record.removed_nodes.is_empty());
        assert_eq!(record.previous_sibling, None);
        assert_eq!(dom.children(parent), vec![child]);
    }

    #[test]
    fn apply_append_child_records_previous_sibling() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let first = elem(&mut dom, "span");
        let second = elem(&mut dom, "p");
        dom.append_child(parent, first);

        let m = Mutation::AppendChild {
            parent,
            child: second,
        };
        let record = apply_mutation(&m, &mut dom).expect("should succeed");
        assert_eq!(record.previous_sibling, Some(first));
        assert_eq!(record.added_nodes, vec![second]);
    }

    #[test]
    fn apply_insert_before() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "span");
        dom.append_child(parent, b);

        let m = Mutation::InsertBefore {
            parent,
            new_child: a,
            ref_child: b,
        };
        let record = apply_mutation(&m, &mut dom).expect("should succeed");
        assert_eq!(record.kind, MutationKind::ChildList);
        assert_eq!(record.added_nodes, vec![a]);
        assert_eq!(record.next_sibling, Some(b));
        assert_eq!(dom.children(parent), vec![a, b]);
    }

    #[test]
    fn apply_set_attribute() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");

        let m = Mutation::SetAttribute {
            entity: e,
            name: "class".into(),
            value: "active".into(),
        };
        let record = apply_mutation(&m, &mut dom).expect("should succeed");
        assert_eq!(record.kind, MutationKind::Attribute);
        assert_eq!(record.attribute_name.as_deref(), Some("class"));
        assert_eq!(record.old_value, None);

        let attrs = dom.world().get::<&Attributes>(e).unwrap();
        assert_eq!(attrs.get("class"), Some("active"));
    }

    #[test]
    fn apply_set_attribute_records_old_value() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(e).unwrap();
            attrs.set("class", "old");
        }

        let m = Mutation::SetAttribute {
            entity: e,
            name: "class".into(),
            value: "new".into(),
        };
        let record = apply_mutation(&m, &mut dom).expect("should succeed");
        assert_eq!(record.old_value.as_deref(), Some("old"));
    }

    #[test]
    fn apply_remove_attribute() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(e).unwrap();
            attrs.set("id", "test");
        }

        let m = Mutation::RemoveAttribute {
            entity: e,
            name: "id".into(),
        };
        let record = apply_mutation(&m, &mut dom).expect("should succeed");
        assert_eq!(record.kind, MutationKind::Attribute);
        assert_eq!(record.attribute_name.as_deref(), Some("id"));
        assert_eq!(record.old_value.as_deref(), Some("test"));

        let attrs = dom.world().get::<&Attributes>(e).unwrap();
        assert!(!attrs.contains("id"));
    }

    #[test]
    fn apply_set_text_content() {
        let mut dom = EcsDom::new();
        let text = dom.create_text("hello");

        let m = Mutation::SetTextContent {
            entity: text,
            text: "world".into(),
        };
        let record = apply_mutation(&m, &mut dom).expect("should succeed");
        assert_eq!(record.kind, MutationKind::CharacterData);
        assert_eq!(record.old_value.as_deref(), Some("hello"));

        let tc = dom.world().get::<&TextContent>(text).unwrap();
        assert_eq!(tc.0, "world");
    }

    #[test]
    fn apply_remove_child() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let a = elem(&mut dom, "span");
        let b = elem(&mut dom, "p");
        dom.append_child(parent, a);
        dom.append_child(parent, b);

        let m = Mutation::RemoveChild { parent, child: a };
        let record = apply_mutation(&m, &mut dom).expect("should succeed");
        assert_eq!(record.kind, MutationKind::ChildList);
        assert_eq!(record.target, parent);
        assert_eq!(record.removed_nodes, vec![a]);
        assert_eq!(record.previous_sibling, None);
        assert_eq!(record.next_sibling, Some(b));
        assert_eq!(dom.children(parent), vec![b]);
    }

    #[test]
    fn apply_replace_child() {
        let mut dom = EcsDom::new();
        let parent = elem(&mut dom, "div");
        let old = elem(&mut dom, "span");
        let new = elem(&mut dom, "p");
        dom.append_child(parent, old);

        let m = Mutation::ReplaceChild {
            parent,
            new_child: new,
            old_child: old,
        };
        let record = apply_mutation(&m, &mut dom).expect("should succeed");
        assert_eq!(record.kind, MutationKind::ChildList);
        assert_eq!(record.added_nodes, vec![new]);
        assert_eq!(record.removed_nodes, vec![old]);
        assert_eq!(dom.children(parent), vec![new]);
        assert_eq!(dom.get_parent(old), None);
    }

    #[test]
    fn apply_append_child_does_not_leak_shadow_root_as_previous_sibling() {
        // PR201 Copilot R4 / F3 regression: `apply_append_child` was
        // capturing `prev_sibling` via raw `get_last_child(parent)`,
        // which returns the internal ShadowRoot when the host has no
        // light-tree children yet. The fix walks via
        // `children_iter_rev` (which skips ShadowRoot entities).
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        let shadow_root = dom
            .attach_shadow(host, elidex_ecs::ShadowRootMode::Closed)
            .expect("attach closed shadow");
        // Sanity: raw `get_last_child(host)` IS the shadow root —
        // confirms the helper would leak without the fix.
        assert_eq!(
            dom.get_last_child(host),
            Some(shadow_root),
            "shadow root is the only sibling at this point"
        );
        let new_child = elem(&mut dom, "span");
        let m = Mutation::AppendChild {
            parent: host,
            child: new_child,
        };
        let record = apply_mutation(&m, &mut dom).expect("append should succeed");
        assert_ne!(
            record.previous_sibling,
            Some(shadow_root),
            "MutationRecord.previous_sibling must not leak shadow root"
        );
        assert_eq!(
            record.previous_sibling, None,
            "no exposed prev sibling (shadow root skipped)"
        );
    }

    #[test]
    fn apply_remove_child_does_not_leak_shadow_root_as_previous_sibling() {
        // Pre-existing apply_remove_child path now uses
        // `prev_exposed_sibling` too. Lock the no-leak invariant.
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        let shadow_root = dom
            .attach_shadow(host, elidex_ecs::ShadowRootMode::Closed)
            .expect("attach closed shadow");
        let child = elem(&mut dom, "span");
        let _ = dom.append_child(host, child);
        assert_eq!(dom.get_prev_sibling(child), Some(shadow_root));
        let m = Mutation::RemoveChild {
            parent: host,
            child,
        };
        let record = apply_mutation(&m, &mut dom).expect("remove should succeed");
        assert_ne!(record.previous_sibling, Some(shadow_root));
        assert_eq!(record.previous_sibling, None);
    }

    /// Codex #335 R10 F31: a buffered `style` attribute mutation applied via
    /// the deferred flush (which bypasses `EcsDom::set_attribute`) must
    /// still invalidate a lazily-hydrated `InlineStyle` cache, else a later
    /// CSSOM read resurrects stale declarations.
    #[test]
    fn apply_style_attribute_invalidates_inline_style_cache() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");
        {
            let mut attrs = dom.world_mut().get::<&mut Attributes>(e).unwrap();
            attrs.set("style", "color: red");
        }
        // Simulate a prior `el.style.*` read that hydrated the cache.
        let mut style = elidex_ecs::InlineStyle::default();
        style.set("color", "red");
        dom.world_mut().insert_one(e, style).unwrap();
        assert!(dom.world().get::<&elidex_ecs::InlineStyle>(e).is_ok());

        // A buffered SetAttribute("style", …) must drop the stale cache.
        let m = Mutation::SetAttribute {
            entity: e,
            name: "style".into(),
            value: "color: blue".into(),
        };
        apply_mutation(&m, &mut dom).expect("should succeed");
        assert!(
            dom.world().get::<&elidex_ecs::InlineStyle>(e).is_err(),
            "buffered SetAttribute('style') left a stale InlineStyle cache"
        );

        // Re-hydrate, then a buffered RemoveAttribute must also drop it.
        let mut style = elidex_ecs::InlineStyle::default();
        style.set("color", "blue");
        dom.world_mut().insert_one(e, style).unwrap();
        let m = Mutation::RemoveAttribute {
            entity: e,
            name: "style".into(),
        };
        apply_mutation(&m, &mut dom).expect("should succeed");
        assert!(
            dom.world().get::<&elidex_ecs::InlineStyle>(e).is_err(),
            "buffered RemoveAttribute('style') left a stale InlineStyle cache"
        );
    }
}
