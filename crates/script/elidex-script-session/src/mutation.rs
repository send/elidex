//! Buffered DOM mutations and their application to the ECS DOM.

use elidex_ecs::{Attributes, EcsDom, Entity, InlineStyle, TextContent};

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
    /// Set innerHTML (stub — returns `false` until HTML parser integration).
    SetInnerHtml {
        /// Target entity.
        entity: Entity,
        /// HTML string to parse and insert.
        html: String,
    },
    /// Set an inline style property.
    SetInlineStyle {
        /// Target entity.
        entity: Entity,
        /// CSS property name.
        property: String,
        /// CSS property value.
        value: String,
    },
    /// Remove an inline style property.
    RemoveInlineStyle {
        /// Target entity.
        entity: Entity,
        /// CSS property name to remove.
        property: String,
    },
    /// Insert a CSS rule into a stylesheet (stub for CSSOM).
    InsertCssRule {
        /// Stylesheet entity.
        stylesheet: Entity,
        /// Index at which to insert.
        index: usize,
        /// CSS rule text.
        rule: String,
    },
    /// Delete a CSS rule from a stylesheet (stub for CSSOM).
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
    /// An inline style property was changed.
    InlineStyle,
    /// A CSS rule was inserted or deleted.
    CssRule,
}

/// Record of a successfully applied mutation.
///
/// Future use: `MutationObserver` notifications.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutationRecord {
    /// The kind of mutation.
    pub kind: MutationKind,
    /// The primary target entity.
    pub target: Entity,
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
        Mutation::AppendChild { parent, child } => {
            child_list_record(dom.append_child(*parent, *child), *parent)
        }
        Mutation::InsertBefore {
            parent,
            new_child,
            ref_child,
        } => child_list_record(dom.insert_before(*parent, *new_child, *ref_child), *parent),
        Mutation::RemoveChild { parent, child } => {
            child_list_record(dom.remove_child(*parent, *child), *parent)
        }
        Mutation::ReplaceChild {
            parent,
            new_child,
            old_child,
        } => child_list_record(dom.replace_child(*parent, *new_child, *old_child), *parent),
        Mutation::SetAttribute {
            entity,
            name,
            value,
        } => apply_set_attribute(dom, *entity, name, value),
        Mutation::RemoveAttribute { entity, name } => apply_remove_attribute(dom, *entity, name),
        Mutation::SetTextContent { entity, text } => apply_set_text(dom, *entity, text),
        Mutation::SetInlineStyle {
            entity,
            property,
            value,
        } => apply_set_inline_style(dom, *entity, property, value),
        Mutation::RemoveInlineStyle { entity, property } => {
            apply_remove_inline_style(dom, *entity, property)
        }
        // Stubs — SetInnerHtml needs HTML parser (M2-3), CSS rules need CSSOM (M2-4+).
        Mutation::SetInnerHtml { .. }
        | Mutation::InsertCssRule { .. }
        | Mutation::DeleteCssRule { .. } => None,
    }
}

fn child_list_record(ok: bool, parent: Entity) -> Option<MutationRecord> {
    ok.then_some(MutationRecord {
        kind: MutationKind::ChildList,
        target: parent,
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
    attrs.set(name, value.to_owned());
    Some(MutationRecord {
        kind: MutationKind::Attribute,
        target: entity,
    })
}

fn apply_remove_attribute(dom: &mut EcsDom, entity: Entity, name: &str) -> Option<MutationRecord> {
    let mut attrs = dom.world_mut().get::<&mut Attributes>(entity).ok()?;
    let name = name.to_ascii_lowercase();
    attrs.remove(&name);
    Some(MutationRecord {
        kind: MutationKind::Attribute,
        target: entity,
    })
}

fn apply_set_text(dom: &mut EcsDom, entity: Entity, text: &str) -> Option<MutationRecord> {
    let mut tc = dom.world_mut().get::<&mut TextContent>(entity).ok()?;
    text.clone_into(&mut tc.0);
    Some(MutationRecord {
        kind: MutationKind::CharacterData,
        target: entity,
    })
}

fn apply_set_inline_style(
    dom: &mut EcsDom,
    entity: Entity,
    property: &str,
    value: &str,
) -> Option<MutationRecord> {
    // Insert InlineStyle component if missing.
    if dom.world_mut().get::<&mut InlineStyle>(entity).is_err()
        && dom
            .world_mut()
            .insert_one(entity, InlineStyle::default())
            .is_err()
    {
        return None;
    }
    let mut style = dom.world_mut().get::<&mut InlineStyle>(entity).ok()?;
    style.set(property.to_owned(), value.to_owned());
    Some(MutationRecord {
        kind: MutationKind::InlineStyle,
        target: entity,
    })
}

fn apply_remove_inline_style(
    dom: &mut EcsDom,
    entity: Entity,
    property: &str,
) -> Option<MutationRecord> {
    let mut style = dom.world_mut().get::<&mut InlineStyle>(entity).ok()?;
    style.remove(property);
    Some(MutationRecord {
        kind: MutationKind::InlineStyle,
        target: entity,
    })
}

#[cfg(test)]
#[allow(unused_must_use)]
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
        assert_eq!(dom.children(parent), vec![child]);
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

        let attrs = dom.world().get::<&Attributes>(e).unwrap();
        assert_eq!(attrs.get("class"), Some("active"));
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

        let tc = dom.world().get::<&TextContent>(text).unwrap();
        assert_eq!(tc.0, "world");
    }

    #[test]
    fn apply_set_inline_style() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");

        let m = Mutation::SetInlineStyle {
            entity: e,
            property: "color".into(),
            value: "red".into(),
        };
        let record = apply_mutation(&m, &mut dom).expect("should succeed");
        assert_eq!(record.kind, MutationKind::InlineStyle);

        let style = dom.world().get::<&InlineStyle>(e).unwrap();
        assert_eq!(style.get("color"), Some("red"));
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
        assert_eq!(dom.children(parent), vec![new]);
        assert_eq!(dom.get_parent(old), None);
    }

    #[test]
    fn apply_remove_inline_style() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");

        // First set a style property.
        let set = Mutation::SetInlineStyle {
            entity: e,
            property: "color".into(),
            value: "red".into(),
        };
        apply_mutation(&set, &mut dom);

        // Now remove it.
        let m = Mutation::RemoveInlineStyle {
            entity: e,
            property: "color".into(),
        };
        let record = apply_mutation(&m, &mut dom).expect("should succeed");
        assert_eq!(record.kind, MutationKind::InlineStyle);

        let style = dom.world().get::<&InlineStyle>(e).unwrap();
        assert_eq!(style.get("color"), None);
    }

    #[test]
    fn set_inner_html_stub_returns_none() {
        let mut dom = EcsDom::new();
        let e = elem(&mut dom, "div");

        let m = Mutation::SetInnerHtml {
            entity: e,
            html: "<p>test</p>".into(),
        };
        assert!(apply_mutation(&m, &mut dom).is_none());
    }
}
