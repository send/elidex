//! Pseudo-element (`::before`/`::after`) generation for style resolution.

use elidex_css::{PseudoElement, Stylesheet};
use elidex_ecs::{EcsDom, Entity, PseudoElementMarker};
use elidex_plugin::{ComputedStyle, ContentValue, Display};

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
    if !matches!(pe_style.content, ContentValue::Items(_)) {
        return;
    }

    // Create the pseudo-element entity with inline display.
    let mut style = pe_style;
    // Pseudo-elements default to inline display unless explicitly set.
    if !winners.contains_key("display") {
        style.display = Display::Inline;
    }

    // Create the entity with empty text — the pre-layout generated-content pass
    // (`generated_content::resolve_generated_content`) is the single resolver of
    // `content` (string / attr() / counter() / counters()) and fills the
    // `TextContent` in document order. `create_text` gives the entity a
    // `TreeRelation` (required for `append_child` / `destroy_entity`).
    let pe_entity = dom.create_text(String::new());
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
