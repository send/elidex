//! DOM tree walking for style resolution.

use elidex_css::{Declaration, PseudoElement, Stylesheet};
use elidex_ecs::{Attributes, EcsDom, ElementState, Entity, PseudoElementMarker, TagType};
use elidex_plugin::{ComputedStyle, Display};

use crate::cascade::{collect_and_cascade, get_inline_declarations};
use crate::pseudo::{generate_pseudo_entity, remove_pseudo_entities};
use crate::resolve::{build_computed_style, ResolveContext};

/// Find root entities to start the tree walk.
///
/// Currently scans all entities.
/// TODO: track the document root entity directly in `EcsDom`.
pub(crate) fn find_roots(dom: &EcsDom) -> Vec<Entity> {
    // Collect all entities that have no parent.
    let mut roots = Vec::new();
    for (entity, ()) in &mut dom.world().query::<()>() {
        if dom.get_parent(entity).is_none() {
            roots.push(entity);
        }
    }
    roots
}

/// Pre-order tree walk: resolve styles for `entity` then recurse into children.
pub(crate) fn walk_tree(
    dom: &mut EcsDom,
    entity: Entity,
    stylesheets: &[&Stylesheet],
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
    hint_generator: &dyn Fn(Entity, &EcsDom) -> Vec<Declaration>,
) {
    // Only resolve styles for element nodes (those with TagType).
    let is_element = dom.world().get::<&TagType>(entity).is_ok();

    let entity_style = if is_element {
        // Set LINK state for `<a href>` elements BEFORE cascade,
        // so :link pseudo-class matching can see the state.
        if is_link_element(dom, entity) {
            let mut state = dom
                .world()
                .get::<&ElementState>(entity)
                .ok()
                .map_or(ElementState::default(), |s| *s);
            // :link and :visited are mutually exclusive (Selectors §3.2).
            state.insert(ElementState::LINK);
            state.remove(ElementState::VISITED);
            let _ = dom.world_mut().insert_one(entity, state);
        }

        // Remove stale pseudo-element entities from previous resolution
        // BEFORE cascade, so :empty and other structural pseudo-classes
        // don't see generated content children.
        remove_pseudo_entities(dom, entity);

        // Collect inline style declarations.
        let inline_decls = get_inline_declarations(entity, dom);

        // Generate presentational hints for this entity.
        let extra_decls = hint_generator(entity, dom);

        // Cascade: collect matching declarations and determine winners.
        let winners = collect_and_cascade(entity, dom, stylesheets, &inline_decls, &extra_decls);

        // Build resolve context with parent's font-size.
        let element_ctx = ctx.with_em_base(parent_style.font_size);

        // Resolve values → ComputedStyle.
        let style = build_computed_style(&winners, parent_style, &element_ctx);

        // Attach ComputedStyle to the entity.
        let _ = dom.world_mut().insert_one(entity, style.clone());

        // Only generate pseudo-elements for visible elements.
        if style.display != Display::None {
            generate_pseudo_entity(
                dom,
                entity,
                stylesheets,
                &style,
                &element_ctx,
                PseudoElement::Before,
            );
            generate_pseudo_entity(
                dom,
                entity,
                stylesheets,
                &style,
                &element_ctx,
                PseudoElement::After,
            );
        }

        style
    } else {
        // Non-element nodes (text, document root) inherit parent style.
        parent_style.clone()
    };

    // Update root_font_size for children: if this is the root element (html),
    // its font-size becomes the root font-size for rem resolution.
    let root_fs = if is_root_element(dom, entity) {
        entity_style.font_size
    } else {
        ctx.root_font_size
    };
    let child_ctx = ctx.with_em_and_root(entity_style.font_size, root_fs);

    // Recurse into children (re-collect since pseudo entities may have been added).
    let children = dom.children(entity);
    for child in children {
        // Skip pseudo-element entities — they already have their ComputedStyle.
        if dom.world().get::<&PseudoElementMarker>(child).is_ok() {
            continue;
        }
        walk_tree(
            dom,
            child,
            stylesheets,
            &entity_style,
            &child_ctx,
            hint_generator,
        );
    }
}

/// Check if an entity is a link element per Selectors §3.2.
///
/// Matches `<a>`, `<area>`, and `<link>` elements that have an `href` attribute.
fn is_link_element(dom: &EcsDom, entity: Entity) -> bool {
    let is_link_tag = dom
        .world()
        .get::<&TagType>(entity)
        .ok()
        .is_some_and(|t| matches!(t.0.as_str(), "a" | "area" | "link"));
    if !is_link_tag {
        return false;
    }
    dom.world()
        .get::<&Attributes>(entity)
        .ok()
        .is_some_and(|attrs| attrs.get("href").is_some())
}

/// Check if entity is the `<html>` root element (tag name only).
///
/// Simplified check for the style tree walk — only needs the tag name since
/// the tree walk already processes elements in document order.
/// See also `elidex_css::selector::is_root_element` which additionally
/// verifies the parent is a document root (for selector matching).
fn is_root_element(dom: &EcsDom, entity: Entity) -> bool {
    dom.world()
        .get::<&TagType>(entity)
        .ok()
        .is_some_and(|t| t.0 == "html")
}
