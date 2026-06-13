//! Buffered DOM mutations and their application to the ECS DOM.

use elidex_ecs::{Attributes, EcsDom, Entity, TextContent};

/// Options controlling [`apply_set_inner_html`] fragment-parse semantics.
///
/// Per HTML §4.4.5 (innerHTML setter) vs §4.4.7 (setHTMLUnsafe), the only
/// behavioural difference is whether `<template shadowrootmode>` children
/// are converted into declarative shadow roots on their parent host or
/// left as plain `<template>` elements. The `_unsafe` JS API name refers
/// to Trusted Types sanitization which is unrelated to engine semantics;
/// from the algorithm's perspective the distinction is purely
/// "honour declarative shadow root markup yes/no".
#[derive(Default, Clone, Copy, Debug)]
pub struct SetInnerHtmlOptions {
    /// When true, `<template shadowrootmode="open|closed">` children
    /// attach as a shadow root on the parent host (per HTML §4.12.3
    /// `<template shadowrootmode>` declarative shadow DOM algorithm). When false (the default,
    /// matching plain `innerHTML` semantics), the templates are left
    /// as ordinary `<template>` elements.
    pub allow_declarative_shadow: bool,
}

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

fn empty_record(kind: MutationKind, target: Entity) -> MutationRecord {
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
    // entering `EcsDom::set_attribute`, so it must preserve that
    // chokepoint's `InlineStyle` invalidation invariant: a buffered `style`
    // write would otherwise leave a lazily-hydrated `InlineStyle` stale and
    // a later CSSOM write could resurrect the old declarations (Codex
    // #335 R10 F31).
    dom.invalidate_inline_style_cache(entity, &name);
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
    // Same `InlineStyle` invalidation invariant as `apply_set_attribute`
    // (Codex #335 R10 F31) — a buffered `removeAttribute("style")` must
    // drop the hydrated component too.
    dom.invalidate_inline_style_cache(entity, &name);
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

/// Apply the `innerHTML` / `setHTMLUnsafe` setter algorithm: remove all
/// existing children, parse `html` as a fragment in the element's tag
/// context, and append the parsed nodes as new children.
///
/// The single `opts` parameter selects between the two JS-visible APIs:
/// `opts.allow_declarative_shadow = false` (the default) implements
/// `innerHTML`, while `true` implements `setHTMLUnsafe` (which honours
/// `<template shadowrootmode>` markup per HTML §4.12.3).
///
/// `pub` so VM bindings can invoke the algorithm directly (CLAUDE.md
/// layering mandate — DOM mutation logic lives engine-indep, not in
/// `vm/host/`); the [`Mutation::SetInnerHtml`] queue path also routes
/// through here for boa-compat (defaulted opts).
#[allow(clippy::unnecessary_wraps)] // Signature matches apply_mutation's Option<> convention.
pub fn apply_set_inner_html(
    dom: &mut EcsDom,
    entity: Entity,
    html: &str,
    opts: SetInnerHtmlOptions,
) -> Option<MutationRecord> {
    let context_tag = dom
        .world()
        .get::<&elidex_ecs::TagType>(entity)
        .ok()
        .map_or_else(|| "div".to_string(), |t| t.0.clone());

    // Remove all existing children.
    let removed: Vec<Entity> = dom.children(entity);
    for &child in &removed {
        let _ = dom.remove_child(entity, child);
    }

    // Parse the HTML fragment and append new children.
    let parse_opts = elidex_html_parser::ParseFragmentOptions {
        allow_declarative_shadow: opts.allow_declarative_shadow,
    };
    let added =
        elidex_html_parser::parse_html_fragment(html, &context_tag, entity, dom, parse_opts);

    Some(MutationRecord {
        added_nodes: added,
        removed_nodes: removed,
        ..empty_record(MutationKind::ChildList, entity)
    })
}

/// Error variants for [`apply_set_outer_html`] per WHATWG HTML §4.4.5
/// `outerHTML` setter algorithm.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum OuterHtmlError {
    /// The target entity has no parent, or its parent is the Document
    /// (Document children cannot be replaced via `outerHTML`).
    /// Surfaces as `DOMException("NoModificationAllowedError")` in JS.
    NoModificationAllowed,
}

/// Apply the `outerHTML` setter algorithm (HTML §4.4.5): parse `html`
/// in the parent's context, then replace `entity` with the parsed
/// fragment in the parent's child list.
///
/// Returns the resulting `MutationRecord` on success (`ChildList`
/// targeting the parent, with `removed_nodes = [entity]` and
/// `added_nodes` = the freshly parsed fragment roots).
///
/// When the parent is a `DocumentFragment` we use a synthetic
/// `<body>` fragment context: per HTML §4.4.5 the spec's fragment
/// algorithm reads the parent's `localName`, and `DocumentFragment`
/// has no tag — `<body>` is the spec's documented fallback context
/// for this case and matches Blink / Gecko behaviour.
pub fn apply_set_outer_html(
    dom: &mut EcsDom,
    entity: Entity,
    html: &str,
) -> Result<MutationRecord, OuterHtmlError> {
    let parent = dom
        .get_parent(entity)
        .ok_or(OuterHtmlError::NoModificationAllowed)?;
    // Document parent is rejected per spec — the Document's children
    // are the doctype + root element and cannot be replaced wholesale
    // via outerHTML on an arbitrary descendant.
    if matches!(dom.node_kind(parent), Some(elidex_ecs::NodeKind::Document)) {
        return Err(OuterHtmlError::NoModificationAllowed);
    }
    let context_tag = if matches!(
        dom.node_kind(parent),
        Some(elidex_ecs::NodeKind::DocumentFragment)
    ) {
        "body".to_string()
    } else {
        dom.world()
            .get::<&elidex_ecs::TagType>(parent)
            .ok()
            .map_or_else(|| "div".to_string(), |t| t.0.clone())
    };
    // Capture exposed siblings BEFORE parse_html_fragment runs — the
    // parser appends the new roots to the end of `parent` first
    // (they are moved into place below), so a post-parse
    // `next_exposed_sibling(entity)` would observe the freshly
    // appended parse output instead of the pre-mutation sibling when
    // `entity` was already the last child. The exposed-sibling
    // helpers also skip internal `ShadowRoot` entities so
    // `MutationRecord.previousSibling/.nextSibling` never leak a
    // closed shadow across the §4.8 encapsulation boundary.
    let prev_sibling = dom.prev_exposed_sibling(entity);
    let next_sibling = dom.next_exposed_sibling(entity);
    let parse_opts = elidex_html_parser::ParseFragmentOptions::default();
    let added =
        elidex_html_parser::parse_html_fragment(html, &context_tag, parent, dom, parse_opts);
    // parse_html_fragment appended the parsed roots at the end of
    // `parent`; relocate them to `entity`'s slot, then unhook entity.
    for &node in &added {
        let _ = dom.remove_child(parent, node);
        let _ = dom.insert_before(parent, node, entity);
    }
    let _ = dom.remove_child(parent, entity);
    Ok(MutationRecord {
        added_nodes: added,
        removed_nodes: vec![entity],
        previous_sibling: prev_sibling,
        next_sibling,
        ..empty_record(MutationKind::ChildList, parent)
    })
}

/// Apply `insertAdjacentHTML`: parse HTML fragment and insert at position.
#[allow(clippy::unnecessary_wraps)]
fn apply_insert_adjacent_html(
    dom: &mut EcsDom,
    entity: Entity,
    position: &str,
    html: &str,
) -> Option<MutationRecord> {
    let tag_of = |e: Entity, dom: &EcsDom| -> String {
        dom.world()
            .get::<&elidex_ecs::TagType>(e)
            .ok()
            .map_or_else(|| "div".to_string(), |t| t.0.clone())
    };

    // insertAdjacentHTML does not honour declarative shadow root markup
    // per HTML §4.4.7 — only setHTMLUnsafe / innerHTML w/ that opt-in do.
    let parse_opts = elidex_html_parser::ParseFragmentOptions::default();
    let added = match position {
        "beforebegin" => {
            let parent = dom.get_parent(entity)?;
            let context_tag = tag_of(parent, dom);
            let nodes = elidex_html_parser::parse_html_fragment(
                html,
                &context_tag,
                parent,
                dom,
                parse_opts,
            );
            for &node in &nodes {
                let _ = dom.remove_child(parent, node);
                let _ = dom.insert_before(parent, node, entity);
            }
            nodes
        }
        "afterbegin" => {
            let context_tag = tag_of(entity, dom);
            let first_child = dom.get_first_child(entity);
            let nodes = elidex_html_parser::parse_html_fragment(
                html,
                &context_tag,
                entity,
                dom,
                parse_opts,
            );
            if let Some(ref_child) = first_child {
                for &node in &nodes {
                    let _ = dom.remove_child(entity, node);
                    let _ = dom.insert_before(entity, node, ref_child);
                }
            }
            nodes
        }
        "beforeend" => {
            let context_tag = tag_of(entity, dom);
            elidex_html_parser::parse_html_fragment(html, &context_tag, entity, dom, parse_opts)
        }
        "afterend" => {
            let parent = dom.get_parent(entity)?;
            let context_tag = tag_of(parent, dom);
            let next = dom.get_next_sibling(entity);
            let nodes = elidex_html_parser::parse_html_fragment(
                html,
                &context_tag,
                parent,
                dom,
                parse_opts,
            );
            if let Some(ref_child) = next {
                // Natural order with constant ref_child preserves document order:
                // insert A before ref → [... A ref], insert B before ref → [... A B ref].
                // Each node goes immediately before ref_child, accumulating in order.
                for &node in &nodes {
                    let _ = dom.remove_child(parent, node);
                    let _ = dom.insert_before(parent, node, ref_child);
                }
            }
            nodes
        }
        _ => return None,
    };

    let target = match position {
        "beforebegin" | "afterend" => dom.get_parent(entity).unwrap_or(entity),
        _ => entity,
    };

    Some(MutationRecord {
        added_nodes: added,
        ..empty_record(MutationKind::ChildList, target)
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
    fn set_inner_html_parses_and_replaces() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let e = elem(&mut dom, "div");
        let _ = dom.append_child(root, e);
        let old_child = dom.create_text("old");
        let _ = dom.append_child(e, old_child);

        let m = Mutation::SetInnerHtml {
            entity: e,
            html: "<p>new</p>".into(),
        };
        let record = apply_mutation(&m, &mut dom);
        assert!(record.is_some(), "SetInnerHtml should return a record");
        let record = record.unwrap();
        assert_eq!(record.removed_nodes.len(), 1, "should remove old child");
        assert_eq!(record.added_nodes.len(), 1, "should add <p>");
    }

    // -------------------------------------------------------------
    // D-15 PR-B: declarative shadow root + outerHTML coverage
    // -------------------------------------------------------------

    #[test]
    fn set_inner_html_default_opts_leaves_template_shadowroot_as_plain_element() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        let _ = apply_set_inner_html(
            &mut dom,
            host,
            r#"<template shadowrootmode="open"><p>x</p></template>"#,
            SetInnerHtmlOptions::default(),
        );
        // Default opts → no shadow attached, template remains as a
        // child element of the host.
        assert!(dom.get_shadow_root(host).is_none());
        let kids = dom.children(host);
        assert_eq!(kids.len(), 1);
        let tag = dom
            .world()
            .get::<&elidex_ecs::TagType>(kids[0])
            .expect("template entity should exist")
            .0
            .clone();
        assert_eq!(tag, "template");
    }

    #[test]
    fn set_inner_html_allow_declarative_shadow_attaches_open_shadow_root() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        let _ = apply_set_inner_html(
            &mut dom,
            host,
            r#"<template shadowrootmode="open"><p>shadow content</p></template>"#,
            SetInnerHtmlOptions {
                allow_declarative_shadow: true,
            },
        );
        let sr = dom
            .get_shadow_root(host)
            .expect("declarative shadow should attach");
        let sr_component = dom.world().get::<&elidex_ecs::ShadowRoot>(sr).unwrap();
        assert_eq!(sr_component.mode, elidex_ecs::ShadowRootMode::Open);
        // The <template> element itself is consumed; the host's light
        // tree has no template child. `EcsDom::children` filters
        // ShadowRoot entities out of the view (they are internal-only
        // siblings), so `host_kids` here is the light-tree-visible
        // set — we just check that no <template> survived.
        let host_kids = dom.children(host);
        for kid in &host_kids {
            if let Ok(tag) = dom.world().get::<&elidex_ecs::TagType>(*kid) {
                assert_ne!(tag.0, "template", "template should be consumed");
            }
        }
        // The shadow tree contains the parsed contents (a <p>).
        let shadow_kids = dom.children(sr);
        assert!(
            shadow_kids.iter().any(|c| dom
                .world()
                .get::<&elidex_ecs::TagType>(*c)
                .is_ok_and(|t| t.0 == "p")),
            "shadow root should hold the parsed <p>"
        );
    }

    #[test]
    fn set_inner_html_allow_declarative_shadow_attaches_closed_shadow_root() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        let _ = apply_set_inner_html(
            &mut dom,
            host,
            r#"<template shadowrootmode="closed"></template>"#,
            SetInnerHtmlOptions {
                allow_declarative_shadow: true,
            },
        );
        let sr = dom.get_shadow_root(host).expect("should attach");
        let mode = dom.world().get::<&elidex_ecs::ShadowRoot>(sr).unwrap().mode;
        assert_eq!(mode, elidex_ecs::ShadowRootMode::Closed);
    }

    #[test]
    fn set_inner_html_invalid_shadowrootmode_leaves_template_as_element() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        let _ = apply_set_inner_html(
            &mut dom,
            host,
            r#"<template shadowrootmode="weird"></template>"#,
            SetInnerHtmlOptions {
                allow_declarative_shadow: true,
            },
        );
        // Invalid mode → silently no shadow + template remains.
        assert!(dom.get_shadow_root(host).is_none());
        let kids = dom.children(host);
        assert_eq!(kids.len(), 1);
        let tag = dom.world().get::<&elidex_ecs::TagType>(kids[0]).unwrap();
        assert_eq!(tag.0, "template");
    }

    #[test]
    fn set_inner_html_declarative_shadow_already_attached_falls_back_silently() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        // Pre-attach a shadow root so the declarative attach is rejected.
        let existing = dom
            .attach_shadow(host, elidex_ecs::ShadowRootMode::Open)
            .expect("first attach");
        let _ = apply_set_inner_html(
            &mut dom,
            host,
            r#"<template shadowrootmode="open"><p>x</p></template>"#,
            SetInnerHtmlOptions {
                allow_declarative_shadow: true,
            },
        );
        // The existing shadow root is unchanged; the template becomes a
        // plain child of the host.
        assert_eq!(
            dom.get_shadow_root(host),
            Some(existing),
            "pre-existing shadow root must be preserved"
        );
    }

    #[test]
    fn set_inner_html_case_insensitive_shadowrootmode() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        let _ = apply_set_inner_html(
            &mut dom,
            host,
            r#"<template shadowrootmode="OPEN"></template>"#,
            SetInnerHtmlOptions {
                allow_declarative_shadow: true,
            },
        );
        let sr = dom
            .get_shadow_root(host)
            .expect("case-insensitive 'OPEN' should attach");
        let mode = dom.world().get::<&elidex_ecs::ShadowRoot>(sr).unwrap().mode;
        assert_eq!(mode, elidex_ecs::ShadowRootMode::Open);
    }

    #[test]
    fn apply_set_outer_html_replaces_element_in_parent() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let parent = elem(&mut dom, "div");
        let _ = dom.append_child(root, parent);
        let target = elem(&mut dom, "span");
        let _ = dom.append_child(parent, target);

        let record =
            apply_set_outer_html(&mut dom, target, "<p>new</p>").expect("outer html should apply");
        assert_eq!(record.removed_nodes, vec![target]);
        assert_eq!(record.added_nodes.len(), 1);
        assert_ne!(
            dom.get_parent(target),
            Some(parent),
            "target should be unhooked from parent"
        );
        // The new <p> is now a child of parent.
        let kids = dom.children(parent);
        assert_eq!(kids.len(), 1);
        let tag = dom.world().get::<&elidex_ecs::TagType>(kids[0]).unwrap();
        assert_eq!(tag.0, "p");
    }

    #[test]
    fn apply_set_outer_html_no_parent_returns_error() {
        let mut dom = EcsDom::new();
        let orphan = elem(&mut dom, "div");
        let err = apply_set_outer_html(&mut dom, orphan, "<p></p>").unwrap_err();
        assert_eq!(err, OuterHtmlError::NoModificationAllowed);
    }

    #[test]
    fn apply_set_outer_html_document_fragment_parent_uses_body_context() {
        // PR201 Copilot R14 regression: `apply_set_outer_html` falls
        // back to the synthetic `<body>` fragment-parse context when
        // parent is a `DocumentFragment` (per HTML §4.4.5 — fragment
        // parents have no tag name, so the spec uses `<body>` as the
        // fallback context). The branch was added without a direct
        // test; this case locks it so a future refactor cannot
        // silently regress to e.g. `<div>` context (which would parse
        // table-context fragments incorrectly).
        let mut dom = EcsDom::new();
        let frag = dom.create_document_fragment();
        let target = elem(&mut dom, "span");
        let _ = dom.append_child(frag, target);

        let record = apply_set_outer_html(&mut dom, target, "<p>new</p>")
            .expect("outerHTML on a DocumentFragment child should apply");
        assert_eq!(record.removed_nodes, vec![target]);
        assert_eq!(record.added_nodes.len(), 1, "one parsed root expected");
        // Verify the parsed root is a `<p>` element — confirming the
        // fragment-parse context didn't drop or restructure the input.
        let kids = dom.children(frag);
        assert_eq!(kids.len(), 1);
        let tag = dom.world().get::<&elidex_ecs::TagType>(kids[0]).unwrap();
        assert_eq!(tag.0, "p");
    }

    #[test]
    fn apply_set_outer_html_document_parent_returns_error() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let html_elem = elem(&mut dom, "html");
        let _ = dom.append_child(doc, html_elem);
        let err = apply_set_outer_html(&mut dom, html_elem, "<html></html>").unwrap_err();
        assert_eq!(err, OuterHtmlError::NoModificationAllowed);
    }

    // -------------------------------------------------------------
    // PR201 Copilot R2: MutationRecord sibling fields must not leak
    // internal ShadowRoot entities (encapsulation lock).
    // -------------------------------------------------------------

    #[test]
    fn apply_set_outer_html_captures_next_sibling_before_parsing() {
        // PR201 Copilot R10 regression: `apply_set_outer_html` used
        // to capture `prev_sibling` / `next_sibling` AFTER calling
        // `parse_html_fragment`, but the parser appends the new roots
        // at the end of `parent` first (they are moved into place
        // immediately after). If `entity` is already the last child,
        // a post-parse `next_exposed_sibling(entity)` would observe
        // the freshly appended parse output instead of the
        // pre-mutation next sibling (None). Lock the corrected order:
        // capture sibling fields BEFORE parsing so `MutationRecord`
        // reflects the pre-mutation tree state per the §4.4.5
        // `outerHTML` replacement algorithm.
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let parent = elem(&mut dom, "div");
        let _ = dom.append_child(root, parent);
        let first = elem(&mut dom, "span");
        let _ = dom.append_child(parent, first);
        // `target` is the LAST child of `parent` — the bug case.
        let target = elem(&mut dom, "p");
        let _ = dom.append_child(parent, target);

        let record =
            apply_set_outer_html(&mut dom, target, "<b>new</b>").expect("outer html should apply");
        assert_eq!(
            record.next_sibling, None,
            "MutationRecord.next_sibling for the last-child case must \
             stay None — must not surface the parser's temporary append"
        );
        assert_eq!(
            record.previous_sibling,
            Some(first),
            "previous_sibling reflects pre-mutation tree"
        );
    }

    #[test]
    fn apply_set_outer_html_does_not_leak_shadow_root_as_previous_sibling() {
        // Scenario: parent is a shadow host; the shadow root entity
        // becomes the FIRST child of the host (attach_shadow appends
        // it before light tree children). Then a light child is
        // appended. When that light child is replaced via outerHTML,
        // its raw `prev_sibling` would be the ShadowRoot entity —
        // surfacing as `MutationRecord.previousSibling` would leak the
        // closed shadow to JS. The fix walks past ShadowRoot entities
        // via `prev_exposed_sibling`.
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let host = elem(&mut dom, "div");
        let _ = dom.append_child(root, host);
        let shadow_root = dom
            .attach_shadow(host, elidex_ecs::ShadowRootMode::Closed)
            .expect("attach closed shadow");
        let target = elem(&mut dom, "span");
        let _ = dom.append_child(host, target);
        // Sanity: raw prev_sibling for `target` IS the shadow root —
        // confirms the helper would leak without the fix.
        assert_eq!(
            dom.get_prev_sibling(target),
            Some(shadow_root),
            "shadow root sits between host and target in the raw sibling chain"
        );
        let record =
            apply_set_outer_html(&mut dom, target, "<p>new</p>").expect("outer html should apply");
        assert_ne!(
            record.previous_sibling,
            Some(shadow_root),
            "MutationRecord.previous_sibling must not leak shadow root"
        );
        assert_eq!(
            record.previous_sibling, None,
            "no exposed prev sibling for target (shadow root skipped)"
        );
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
