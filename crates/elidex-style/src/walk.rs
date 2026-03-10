//! DOM tree walking for style resolution.

use elidex_css::{parse_stylesheet, Declaration, Origin, PseudoElement, Stylesheet};
use elidex_ecs::{
    Attributes, EcsDom, ElementState, Entity, PseudoElementMarker, ShadowHost, ShadowRoot,
    SlotAssignment, TagType, TemplateContent, TextContent, MAX_ANCESTOR_DEPTH,
};
use elidex_plugin::{ComputedStyle, Display};

use crate::cascade::{collect_and_cascade, get_inline_declarations, ShadowCascade};
use crate::pseudo::{generate_pseudo_entity, remove_pseudo_entities};
use crate::resolve::{build_computed_style, ResolveContext};
use crate::slot::distribute_slots;

/// Maximum recursion depth for style tree walks.
const MAX_WALK_DEPTH: usize = MAX_ANCESTOR_DEPTH;

/// Build a child `ResolveContext` from a resolved entity style.
///
/// Updates `em_base` from the entity's font-size and `root_font_size`
/// if the entity is the `<html>` root element.
fn child_context(dom: &EcsDom, entity: Entity, style: &ComputedStyle, ctx: &ResolveContext) -> ResolveContext {
    let root_fs = if dom.has_tag(entity, "html") {
        style.font_size
    } else {
        ctx.root_font_size
    };
    ctx.with_em_and_root(style.font_size, root_fs)
}

/// Whether an entity should be skipped during tree walking.
///
/// Returns `true` for pseudo-element entities (already styled) and
/// `<template>` elements (inert content, not rendered/styled).
fn should_skip_child(dom: &EcsDom, entity: Entity) -> bool {
    dom.world().get::<&PseudoElementMarker>(entity).is_ok()
        || dom.world().get::<&TemplateContent>(entity).is_ok()
}

/// Walk children of `parent`, resolving styles with `walk_tree`.
///
/// Filters out pseudo-element and template entities.
///
/// When the `parallel` feature is enabled, the cascade phase runs
/// sequentially (requires `&EcsDom`), then `build_computed_style` runs
/// in parallel across siblings via rayon, and finally the results are
/// applied and children recursed sequentially.
#[allow(clippy::too_many_arguments)]
fn walk_children(
    dom: &mut EcsDom,
    parent: Entity,
    stylesheets: &[&Stylesheet],
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
    hint_generator: &dyn Fn(Entity, &EcsDom) -> Vec<Declaration>,
    depth: usize,
    total_shadow_css: &mut usize,
) {
    #[cfg(feature = "parallel")]
    {
        walk_children_parallel(
            dom,
            parent,
            stylesheets,
            parent_style,
            ctx,
            hint_generator,
            depth,
            total_shadow_css,
        );
    }

    #[cfg(not(feature = "parallel"))]
    {
        let children = dom.children(parent);
        for child in children {
            if should_skip_child(dom, child) {
                continue;
            }
            walk_tree(
                dom,
                child,
                stylesheets,
                parent_style,
                ctx,
                hint_generator,
                depth + 1,
                total_shadow_css,
            );
        }
    }
}

/// Parallel sibling resolution: cascade sequentially, resolve in parallel,
/// then apply and recurse sequentially.
#[cfg(feature = "parallel")]
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn walk_children_parallel(
    dom: &mut EcsDom,
    parent: Entity,
    stylesheets: &[&Stylesheet],
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
    hint_generator: &dyn Fn(Entity, &EcsDom) -> Vec<Declaration>,
    depth: usize,
    total_shadow_css: &mut usize,
) {
    use crate::parallel::{par_resolve_siblings, to_owned_map, OwnedPropertyMap};

    if depth > MAX_WALK_DEPTH {
        return;
    }

    let children = dom.children(parent);
    let walkable: Vec<Entity> = children
        .into_iter()
        .filter(|&c| !should_skip_child(dom, c))
        .collect();

    if walkable.is_empty() {
        return;
    }

    // Check for shadow hosts — they need special handling, fall back to sequential.
    let has_shadow_hosts = walkable
        .iter()
        .any(|&c| dom.world().get::<&ShadowHost>(c).is_ok());

    if has_shadow_hosts {
        // Fall back to sequential walk_tree for shadow DOM subtrees.
        for child in walkable {
            walk_tree(
                dom,
                child,
                stylesheets,
                parent_style,
                ctx,
                hint_generator,
                depth + 1,
                total_shadow_css,
            );
        }
        return;
    }

    // Separate elements from non-elements. Non-elements get parent_style.clone()
    // (matching the sequential path in resolve_and_attach_style).
    // Elements go through cascade + parallel build_computed_style.
    let mut child_entries: Vec<(Entity, Option<usize>)> = Vec::with_capacity(walkable.len());
    let mut cascade_inputs: Vec<OwnedPropertyMap> = Vec::new();

    // Phase 1: Sequential cascade (requires &EcsDom).
    for &child in &walkable {
        let is_element = dom.world().get::<&TagType>(child).is_ok();
        if !is_element {
            child_entries.push((child, None));
            continue;
        }

        set_link_state(dom, child);

        remove_pseudo_entities(dom, child);

        let inline_decls = get_inline_declarations(child, dom);
        let extra_decls = hint_generator(child, dom);
        let winners = collect_and_cascade(
            child,
            dom,
            stylesheets,
            &inline_decls,
            &extra_decls,
            &ShadowCascade::Outer,
        );
        let idx = cascade_inputs.len();
        cascade_inputs.push(to_owned_map(&winners));
        child_entries.push((child, Some(idx)));
    }

    // Phase 2: Parallel build_computed_style (elements only).
    let element_ctx = ctx.with_em_base(parent_style.font_size);
    let styles = par_resolve_siblings(&cascade_inputs, parent_style, &element_ctx);

    // Phase 3: Sequential apply + pseudo + recurse.
    for &(child, cascade_idx) in &child_entries {
        let style: &ComputedStyle = if let Some(idx) = cascade_idx {
            // Element: attach parallel-resolved style (single clone).
            let _ = dom.world_mut().insert_one(child, styles[idx].clone());

            if styles[idx].display != Display::None {
                generate_pseudo_entity(
                    dom,
                    child,
                    stylesheets,
                    &styles[idx],
                    &element_ctx,
                    PseudoElement::Before,
                );
                generate_pseudo_entity(
                    dom,
                    child,
                    stylesheets,
                    &styles[idx],
                    &element_ctx,
                    PseudoElement::After,
                );
            }
            &styles[idx]
        } else {
            // Non-element: inherit parent style without attaching ComputedStyle
            // (matches resolve_and_attach_style, which returns early for non-elements
            // without inserting a component — CSSOM getComputedStyle is element-only).
            parent_style
        };

        let child_ctx = child_context(dom, child, style, ctx);
        walk_children(
            dom,
            child,
            stylesheets,
            style,
            &child_ctx,
            hint_generator,
            depth + 1,
            total_shadow_css,
        );
    }
}

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

/// Resolve style for a single entity and attach it as a `ComputedStyle` component.
///
/// For element nodes: runs cascade (with shadow cascade context), resolves values,
/// generates pseudo-elements, and attaches `ComputedStyle`.
/// For non-element nodes: inherits the parent style.
///
/// Returns the resolved `ComputedStyle` for use as the parent of child elements.
fn resolve_and_attach_style(
    dom: &mut EcsDom,
    entity: Entity,
    stylesheets: &[&Stylesheet],
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
    hint_generator: &dyn Fn(Entity, &EcsDom) -> Vec<Declaration>,
    shadow_cascade: &ShadowCascade<'_>,
) -> ComputedStyle {
    let is_element = dom.world().get::<&TagType>(entity).is_ok();

    if !is_element {
        return parent_style.clone();
    }

    // Set LINK state for `<a href>` elements BEFORE cascade,
    // so :link pseudo-class matching can see the state.
    set_link_state(dom, entity);

    // Remove stale pseudo-element entities from previous resolution
    // BEFORE cascade, so :empty and other structural pseudo-classes
    // don't see generated content children.
    remove_pseudo_entities(dom, entity);

    // Collect inline style declarations.
    let inline_decls = get_inline_declarations(entity, dom);

    // Generate presentational hints for this entity.
    let extra_decls = hint_generator(entity, dom);

    // Cascade: collect matching declarations and determine winners.
    let winners = collect_and_cascade(
        entity,
        dom,
        stylesheets,
        &inline_decls,
        &extra_decls,
        shadow_cascade,
    );

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
}

/// Pre-order tree walk: resolve styles for `entity` then recurse into children.
///
/// Recursion is capped at `MAX_WALK_DEPTH` to prevent stack overflow on
/// deeply nested DOM trees.
#[allow(clippy::too_many_arguments)]
pub(crate) fn walk_tree(
    dom: &mut EcsDom,
    entity: Entity,
    stylesheets: &[&Stylesheet],
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
    hint_generator: &dyn Fn(Entity, &EcsDom) -> Vec<Declaration>,
    depth: usize,
    total_shadow_css: &mut usize,
) {
    if depth > MAX_WALK_DEPTH {
        return;
    }
    // A2: If this entity is a shadow host, we need to resolve its style with
    // the shadow stylesheet's :host rules participating in the cascade.
    // H2: Parse shadow CSS once and reuse for both :host cascade and child walk.
    // Extract shadow_root Entity before dropping the Ref to release the borrow.
    let shadow_root_entity = dom
        .world()
        .get::<&ShadowHost>(entity)
        .ok()
        .map(|sh| sh.shadow_root);
    let shadow_sheet_owned = shadow_root_entity.map(|sr| {
        let shadow_css = collect_shadow_styles(dom, sr, total_shadow_css);
        parse_stylesheet(&shadow_css, Origin::Author)
    });

    let entity_style = if let Some(ref shadow_sheet) = shadow_sheet_owned {
        resolve_and_attach_style(
            dom,
            entity,
            stylesheets,
            parent_style,
            ctx,
            hint_generator,
            &ShadowCascade::Host(shadow_sheet),
        )
    } else {
        resolve_and_attach_style(
            dom,
            entity,
            stylesheets,
            parent_style,
            ctx,
            hint_generator,
            &ShadowCascade::Outer,
        )
    };

    let child_ctx = child_context(dom, entity, &entity_style, ctx);

    // Shadow DOM: if this entity is a shadow host, distribute slots and
    // walk the shadow tree with shadow-internal stylesheets.
    if let Some(shadow_sheet) = shadow_sheet_owned {
        distribute_slots(dom, entity);
        walk_shadow_tree(
            dom,
            entity,
            stylesheets,
            &shadow_sheet,
            &entity_style,
            &child_ctx,
            hint_generator,
            depth,
            total_shadow_css,
        );
        return;
    }

    // Recurse into children (re-collect since pseudo entities may have been added).
    walk_children(
        dom,
        entity,
        stylesheets,
        &entity_style,
        &child_ctx,
        hint_generator,
        depth,
        total_shadow_css,
    );
}

/// Context for shadow tree walking, bundling stylesheet references.
struct ShadowWalkContext<'a> {
    /// UA + shadow-internal stylesheets.
    shadow_sheets: Vec<&'a Stylesheet>,
    /// Outer (light DOM) stylesheets for slotted node cascade.
    outer_sheets: &'a [&'a Stylesheet],
    /// The shadow stylesheet itself (for `::slotted()` cascade).
    shadow_sheet: &'a Stylesheet,
}

/// Walk a shadow host's shadow tree with scoped stylesheets.
///
/// 1. Builds shadow stylesheet list (UA + shadow-internal).
/// 2. Walks shadow tree children with the shadow context.
/// 3. When a `<slot>` with assigned nodes is encountered, each slotted node is
///    resolved with outer stylesheets + `ShadowCascade::Slotted` for `::slotted()`
///    rule participation, inheriting from the slot's computed style.
#[allow(clippy::too_many_arguments)]
fn walk_shadow_tree(
    dom: &mut EcsDom,
    host: Entity,
    outer_stylesheets: &[&Stylesheet],
    shadow_sheet: &Stylesheet,
    host_style: &ComputedStyle,
    ctx: &ResolveContext,
    hint_generator: &dyn Fn(Entity, &EcsDom) -> Vec<Declaration>,
    depth: usize,
    total_shadow_css: &mut usize,
) {
    let Some(shadow_root) = dom.get_shadow_root(host) else {
        return;
    };

    // Build shadow stylesheet list: UA + shadow-internal only.
    let ua = crate::ua::ua_stylesheet();
    let shadow_ctx = ShadowWalkContext {
        shadow_sheets: vec![ua, shadow_sheet],
        outer_sheets: outer_stylesheets,
        shadow_sheet,
    };

    // Walk shadow root's children with shadow stylesheets.
    let children = dom.children(shadow_root);
    for child in children {
        walk_shadow_child(
            dom,
            child,
            &shadow_ctx,
            host_style,
            ctx,
            hint_generator,
            depth + 1,
            total_shadow_css,
        );
    }
}

/// Recursively walk a child within a shadow tree.
///
/// A1: This function is fully recursive — nested `<slot>` elements and their
/// descendants are all handled by recursive calls.
///
/// A3: For `<slot>` elements with assigned nodes, each slotted node is resolved
/// with the outer stylesheets + `ShadowCascade::Slotted(&shadow_sheet)` so that
/// `::slotted()` rules from the shadow stylesheet participate in the cascade.
///
/// Regular shadow tree children are resolved with shadow stylesheets only.
#[allow(clippy::too_many_arguments)]
fn walk_shadow_child(
    dom: &mut EcsDom,
    entity: Entity,
    shadow_ctx: &ShadowWalkContext<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
    hint_generator: &dyn Fn(Entity, &EcsDom) -> Vec<Declaration>,
    depth: usize,
    total_shadow_css: &mut usize,
) {
    if depth > MAX_WALK_DEPTH {
        return;
    }

    // Skip <style> elements in shadow tree (already collected).
    if dom.has_tag(entity, "style") {
        return;
    }

    if should_skip_child(dom, entity) {
        return;
    }

    // Nested shadow host: resolve with its own :host rules and walk its shadow tree.
    let nested_shadow_root = dom
        .world()
        .get::<&ShadowHost>(entity)
        .ok()
        .map(|sh| sh.shadow_root);
    if let Some(inner_sr) = nested_shadow_root {
        // Parse the nested shadow's stylesheet for :host cascade.
        let inner_shadow_css = collect_shadow_styles(dom, inner_sr, total_shadow_css);
        let inner_shadow_sheet = parse_stylesheet(&inner_shadow_css, Origin::Author);

        // Resolve host style: shadow_sheets act as "outer" for this nested host,
        // and the inner shadow's :host rules participate via ShadowCascade::Host.
        let entity_style = resolve_and_attach_style(
            dom,
            entity,
            &shadow_ctx.shadow_sheets,
            parent_style,
            ctx,
            hint_generator,
            &ShadowCascade::Host(&inner_shadow_sheet),
        );

        let child_ctx = ctx.with_em_base(entity_style.font_size);
        distribute_slots(dom, entity);
        walk_shadow_tree(
            dom,
            entity,
            &shadow_ctx.shadow_sheets,
            &inner_shadow_sheet,
            &entity_style,
            &child_ctx,
            hint_generator,
            depth,
            total_shadow_css,
        );
        return;
    }

    // Resolve style for this shadow tree element using shadow stylesheets.
    let entity_style = resolve_and_attach_style(
        dom,
        entity,
        &shadow_ctx.shadow_sheets,
        parent_style,
        ctx,
        hint_generator,
        &ShadowCascade::Outer,
    );

    let child_ctx = ctx.with_em_base(entity_style.font_size);

    // A3: If this is a <slot>, resolve assigned (slotted) nodes with outer
    // stylesheets + ShadowCascade::Slotted so ::slotted() rules participate.
    let assigned = dom
        .world()
        .get::<&SlotAssignment>(entity)
        .ok()
        .map(|sa| sa.assigned_nodes.clone());
    if let Some(ref assigned) = assigned {
        if !assigned.is_empty() {
            for node in assigned {
                let node_style = resolve_and_attach_style(
                    dom,
                    *node,
                    shadow_ctx.outer_sheets,
                    &entity_style,
                    &child_ctx,
                    hint_generator,
                    &ShadowCascade::Slotted(shadow_ctx.shadow_sheet),
                );
                // Recurse into slotted node's children with outer stylesheets.
                let node_ctx = child_ctx.with_em_base(node_style.font_size);
                walk_children(
                    dom,
                    *node,
                    shadow_ctx.outer_sheets,
                    &node_style,
                    &node_ctx,
                    hint_generator,
                    depth,
                    total_shadow_css,
                );
            }
            // L6: Skip fallback children when assigned nodes are present.
            // Only walk shadow tree children (non-fallback) below.
            return;
        }
    }

    // A1: Recurse into shadow tree children (handles nested <slot> elements).
    // For slots with no assigned nodes, this walks fallback content.
    let children = dom.children(entity);
    for child in children {
        walk_shadow_child(
            dom,
            child,
            shadow_ctx,
            &entity_style,
            &child_ctx,
            hint_generator,
            depth + 1,
            total_shadow_css,
        );
    }
}

/// Maximum total CSS text size collected across all shadow roots (1 MB).
///
/// This limit is global (tracked via a cumulative counter passed through
/// the style walk), preventing deeply nested shadow trees from allocating
/// unbounded CSS text.
const MAX_SHADOW_CSS_SIZE: usize = 1_000_000;

fn collect_shadow_styles(dom: &EcsDom, shadow_root: Entity, total_css: &mut usize) -> String {
    let mut css = String::new();
    collect_shadow_styles_recursive(dom, shadow_root, &mut css, *total_css, 0);
    *total_css += css.len();
    css
}

fn collect_shadow_styles_recursive(
    dom: &EcsDom,
    entity: Entity,
    css: &mut String,
    total_css: usize,
    depth: usize,
) {
    if depth > MAX_WALK_DEPTH {
        return;
    }
    for child in dom.children(entity) {
        if total_css + css.len() >= MAX_SHADOW_CSS_SIZE {
            return;
        }
        // Stop at nested shadow roots — their styles belong to their own scope.
        if dom.world().get::<&ShadowRoot>(child).is_ok() {
            continue;
        }
        if dom.has_tag(child, "style") {
            // Collect text content from <style>'s children.
            for text_child in dom.children(child) {
                if let Ok(tc) = dom.world().get::<&TextContent>(text_child) {
                    css.push_str(&tc.0);
                }
            }
        } else {
            collect_shadow_styles_recursive(dom, child, css, total_css, depth + 1);
        }
    }
}

/// Set `:link` state on an element if it is a link element per Selectors §3.2.
///
/// `:link` and `:visited` are mutually exclusive; we always set `:link`
/// and clear `:visited` (privacy-preserving — all links treated as unvisited).
fn set_link_state(dom: &mut EcsDom, entity: Entity) {
    if !is_link_element(dom, entity) {
        return;
    }
    let mut state = dom
        .world()
        .get::<&ElementState>(entity)
        .ok()
        .map_or(ElementState::default(), |s| *s);
    state.insert(ElementState::LINK);
    state.remove(ElementState::VISITED);
    let _ = dom.world_mut().insert_one(entity, state);
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
