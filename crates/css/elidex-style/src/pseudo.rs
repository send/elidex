//! Pseudo-element (`::before`/`::after`) generation for style resolution.

use std::fmt::Write;

use elidex_css::{PseudoElement, Stylesheet};
use elidex_ecs::{Attributes, EcsDom, Entity, PseudoElementMarker};
use elidex_plugin::{ComputedStyle, ContentItem, ContentValue, Display};

use crate::cascade::collect_and_cascade_pseudo;
use crate::resolve::{build_computed_style, ResolveContext};

/// Remove all pseudo-element child entities from a parent.
///
/// Called before generating new pseudo-elements to avoid stale entities
/// from a previous style resolution pass.
pub(crate) fn remove_pseudo_entities(dom: &mut EcsDom, parent: Entity) {
    let pseudo_children: Vec<Entity> = dom
        .children_iter(parent)
        .filter(|&child| dom.world().get::<&PseudoElementMarker>(child).is_ok())
        .collect();
    for pe in pseudo_children {
        let _ = dom.destroy_entity(pe);
    }
}

/// Generate a pseudo-element entity (`::before` or `::after`) if the
/// originating element has matching CSS rules with non-empty content.
pub(crate) fn generate_pseudo_entity(
    dom: &mut EcsDom,
    entity: Entity,
    stylesheets: &[&Stylesheet],
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
    pseudo: PseudoElement,
) {
    // Cascade pseudo-element rules.
    let winners = collect_and_cascade_pseudo(entity, dom, stylesheets, pseudo);
    if winners.is_empty() {
        return;
    }

    // Build computed style for the pseudo-element (inherits from originating element).
    let pe_style = build_computed_style(&winners, parent_style, ctx);

    // CSS Generated Content §2: on pseudo-elements, `content: normal` computes
    // to `none`.  Both `normal` and `none` suppress generation.
    let text = match &pe_style.content {
        ContentValue::Items(ref items) => resolve_content_text(items, entity, dom),
        // Normal or None → no generation.
        _ => return,
    };

    // Create the pseudo-element entity with inline display.
    let mut style = pe_style;
    // Pseudo-elements default to inline display unless explicitly set.
    if !winners.contains_key("display") {
        style.display = Display::Inline;
    }

    // Use create_text() to ensure the entity has a TreeRelation component,
    // which is required for EcsDom tree operations (append_child, destroy_entity, etc.).
    let pe_entity = dom.create_text(text);
    let _ = dom.world_mut().insert_one(pe_entity, style);
    let _ = dom.world_mut().insert_one(pe_entity, PseudoElementMarker);

    // Insert as first child (::before) or last child (::after).
    match pseudo {
        PseudoElement::Before => {
            let first_child = dom.get_first_child(entity);
            if let Some(fc) = first_child {
                let _ = dom.insert_before(entity, pe_entity, fc);
            } else {
                let _ = dom.append_child(entity, pe_entity);
            }
        }
        PseudoElement::After => {
            let _ = dom.append_child(entity, pe_entity);
        }
    }
}

/// Resolve content items to a text string.
fn resolve_content_text(items: &[ContentItem], entity: Entity, dom: &EcsDom) -> String {
    let mut result = String::new();
    for item in items {
        match item {
            ContentItem::String(s) => result.push_str(s),
            ContentItem::Attr(name) => {
                if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
                    if let Some(val) = attrs.get(name) {
                        result.push_str(val);
                    }
                }
            }
            // Counter values require counter state from the document tree;
            // placeholder output until counter evaluation is implemented.
            ContentItem::Counter { name, .. } => {
                write!(result, "[counter:{name}]").unwrap();
            }
            ContentItem::Counters {
                name, separator, ..
            } => {
                write!(result, "[counters:{name},{separator}]").unwrap();
            }
        }
    }
    result
}
