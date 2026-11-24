//! DOM tree walking for style resolution.

use elidex_css::media::MediaEnvironment;
use elidex_css::{parse_stylesheet, Declaration, Origin, PseudoElement, Stylesheet};
use elidex_css_anim::resolve::{resolve_anim_property, ANIM_LONGHAND_NAMES};
use elidex_css_anim::style::AnimStyle;
use elidex_ecs::{
    Attributes, EcsDom, ElementState, Entity, PseudoElementMarker, ShadowRoot, SlotAssignment,
    TagType, TextContent, MAX_ANCESTOR_DEPTH,
};
use elidex_plugin::{ComputedStyle, CssValue, Display, Overflow, ViewportOverflow};

use crate::cascade::{collect_and_cascade, get_inline_declarations, ElementSheets, ShadowCascade};
use crate::pseudo::{generate_pseudo_entity, remove_pseudo_entities};
use crate::resolve::{build_style_for_subject, ResolveContext, StyleSubject};
use crate::slot::distribute_slots;

/// Maximum recursion depth for style tree walks.
const MAX_WALK_DEPTH: usize = MAX_ANCESTOR_DEPTH;

/// Mutable state carried through the style tree walk.
///
/// Bundles the shared parameters that are threaded through every walk function,
/// reducing argument counts and making recursive calls cleaner.
///
/// `ctx` is owned (not borrowed) because each recursive level computes a new
/// `ResolveContext` with an updated `em_base`/`root_font_size`. The struct is
/// only 16 bytes (4× f32) so copies are cheap.
pub(crate) struct WalkState<'a> {
    pub ctx: ResolveContext,
    /// The media-query environment (`@media` cascade gate, CSS Conditional §2).
    /// `Copy`, built once per resolve from the same `viewport` device-fact
    /// `ctx` derives from (a derived view, not a competing SoT). Constant across
    /// the walk — unlike `ctx`, it is not recomputed per level.
    pub media_env: MediaEnvironment,
    pub hint_generator: &'a dyn Fn(Entity, &EcsDom) -> Vec<Declaration>,
    pub depth: usize,
    pub effective_parent: Option<Display>,
    pub document_element: Option<Entity>,
    pub total_shadow_css: &'a mut usize,
}

/// Build an `AnimStyle` from cascade winners if any animation/transition properties are set.
///
/// Returns `Some(AnimStyle)` when at least one animation/transition property was found
/// in the winners map, `None` otherwise (to avoid inserting empty ECS components).
fn build_anim_style_from_winners(
    winners: &std::collections::HashMap<&str, &CssValue>,
) -> Option<AnimStyle> {
    let mut style = AnimStyle::default();
    let mut found = false;

    for &name in ANIM_LONGHAND_NAMES {
        if let Some(&value) = winners.get(name) {
            resolve_anim_property(name, value, &mut style);
            found = true;
        }
    }

    if found {
        Some(style)
    } else {
        None
    }
}

/// Whether an entity should be skipped during tree walking.
///
/// Returns `true` for pseudo-element entities (already styled). `<template>`
/// content needs no skip here: it is inert by construction, living in a
/// *detached* `TemplateContents` fragment (HTML §4.12.3) no light-tree walk
/// reaches; the `<template>` element itself is styled normally but has no
/// light children.
fn is_generated_pseudo(dom: &EcsDom, entity: Entity) -> bool {
    dom.world().get::<&PseudoElementMarker>(entity).is_ok()
}

/// Find root entities to start the tree walk.
///
/// Uses the cached document root from `EcsDom` when available (O(1)),
/// falling back to scanning all entities for backward compatibility.
pub(crate) fn find_roots(dom: &EcsDom) -> Vec<Entity> {
    if let Some(root) = dom.document_root() {
        return vec![root];
    }
    // Fallback: scan all entities that have no parent.
    let mut roots = Vec::new();
    for entity in &mut dom.world().query::<Entity>() {
        if dom.get_parent(entity).is_none() {
            roots.push(entity);
        }
    }
    roots
}

/// CSS Display §2.8 and Appendix B's HTML elements with suppressed contents.
fn element_subject(dom: &EcsDom, entity: Entity, document_element: Option<Entity>) -> StyleSubject {
    if Some(entity) == document_element {
        StyleSubject::DocumentElement
    } else if dom.is_html_namespace(entity)
        && dom.with_tag_name(entity, |tag| {
            matches!(
                tag,
                Some(
                    "br" | "wbr"
                        | "meter"
                        | "progress"
                        | "canvas"
                        | "embed"
                        | "object"
                        | "audio"
                        | "iframe"
                        | "img"
                        | "video"
                        | "frame"
                        | "frameset"
                        | "input"
                        | "textarea"
                        | "select"
                )
            )
        })
    {
        StyleSubject::UnusualHtml
    } else {
        StyleSubject::Element
    }
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
    stylesheets: ElementSheets<'_>,
    parent_style: &ComputedStyle,
    state: &WalkState<'_>,
    shadow_cascade: &ShadowCascade<'_>,
) -> ComputedStyle {
    if !dom.is_element(entity) {
        return parent_style.clone();
    }

    // Set LINK state for `<a href>` elements BEFORE cascade,
    // so :link pseudo-class matching can see the state.
    set_link_state(dom, entity);

    // Remove stale pseudo-element entities from previous resolution
    // BEFORE cascade, so :empty and other structural pseudo-classes
    // don't see generated content children.
    remove_pseudo_entities(dom, entity);

    let (mut style, anim_style) = compute_element_style(
        dom,
        entity,
        stylesheets,
        parent_style,
        &state.ctx,
        &state.media_env,
        state.hint_generator,
        shadow_cascade,
        element_subject(dom, entity, state.document_element),
        state.effective_parent,
    );
    let element_ctx = state.ctx.with_em_base(parent_style.font_size);
    // Apply implicit list-item counters for <ol>, <ul>, <li> (CSS Lists 3 §4.6).
    crate::counter::apply_implicit_list_counters_from_dom(dom, entity, &mut style);

    // Attach ComputedStyle to the entity.
    let _ = dom.world_mut().insert_one(entity, style.clone());

    // Attach AnimStyle if any animation/transition properties are set,
    // or remove stale AnimStyle so transition detection stops running.
    if let Some(anim_style) = anim_style {
        let _ = dom.world_mut().insert_one(entity, anim_style);
    } else {
        let _ = dom
            .world_mut()
            .remove_one::<elidex_css_anim::style::AnimStyle>(entity);
    }

    // Only generate pseudo-elements for visible elements.
    if style.display != Display::None {
        for pseudo in [PseudoElement::Before, PseudoElement::After] {
            generate_pseudo_entity(
                dom,
                entity,
                stylesheets,
                &style,
                &element_ctx,
                &state.media_env,
                child_display(style.display, state.effective_parent),
                pseudo,
            );
        }
    }

    style
}

#[allow(clippy::too_many_arguments)]
fn compute_element_style(
    dom: &EcsDom,
    entity: Entity,
    stylesheets: ElementSheets<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
    media_env: &MediaEnvironment,
    hint_generator: &dyn Fn(Entity, &EcsDom) -> Vec<Declaration>,
    shadow_cascade: &ShadowCascade<'_>,
    subject: StyleSubject,
    effective_parent: Option<Display>,
) -> (ComputedStyle, Option<AnimStyle>) {
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
        media_env,
    );

    // Build resolve context with parent's font-size.
    let element_ctx = ctx.with_em_base(parent_style.font_size);

    // Resolve values → ComputedStyle.
    let mut resolved = crate::resolve::resolve_candidates(&winners, parent_style);
    let custom = std::mem::take(&mut resolved.custom);
    let winners = resolved.ordinary();
    let mut style = build_style_for_subject(
        &winners,
        parent_style,
        &element_ctx,
        subject,
        effective_parent,
    );

    style.custom_properties = custom;

    (style, build_anim_style_from_winners(&winners))
}

pub(crate) fn preliminary_root(
    dom: &mut EcsDom,
    sheets: &[&Stylesheet],
    ctx: &ResolveContext,
    media: &MediaEnvironment,
    hints: &dyn Fn(Entity, &EcsDom) -> Vec<Declaration>,
) -> ComputedStyle {
    let parent = ComputedStyle::default();
    let Some(root) = dom.document_root().and_then(|r| dom.first_element_child(r)) else {
        return parent;
    };
    set_link_state(dom, root);
    remove_pseudo_entities(dom, root);
    let sheet = host_stylesheet(dom, root, &mut 0);
    let shadow = ShadowCascade {
        host: sheet.as_ref(),
        slot: None,
    };
    compute_element_style(
        dom,
        root,
        ElementSheets::document(sheets),
        &parent,
        ctx,
        media,
        hints,
        &shadow,
        StyleSubject::DocumentElement,
        None,
    )
    .0
}

/// Pre-order tree walk: resolve styles for `entity` then recurse into children.
///
/// Recursion is capped at `MAX_WALK_DEPTH` to prevent stack overflow on
/// deeply nested DOM trees.
pub(crate) fn walk_tree(
    dom: &mut EcsDom,
    entity: Entity,
    stylesheets: ElementSheets<'_>,
    parent_style: &ComputedStyle,
    state: &mut WalkState<'_>,
) {
    walk_node(dom, entity, stylesheets, None, None, parent_style, state);
}

fn child_display(display: Display, incoming: Option<Display>) -> Option<Display> {
    if display == Display::Contents {
        incoming
    } else {
        Some(display)
    }
}

/// Resolve a flat-tree node once, then choose its shadow, assigned or fallback children.
fn walk_node(
    dom: &mut EcsDom,
    entity: Entity,
    sheets: ElementSheets<'_>,
    shadow: Option<&ShadowWalkContext<'_>>,
    slot_sheet: Option<(Entity, &Stylesheet)>,
    parent: &ComputedStyle,
    state: &mut WalkState<'_>,
) {
    if state.depth > MAX_WALK_DEPTH || is_generated_pseudo(dom, entity) {
        return;
    }
    let own_sheet = host_stylesheet(dom, entity, state.total_shadow_css);
    let style = resolve_and_attach_style(
        dom,
        entity,
        sheets,
        parent,
        state,
        &ShadowCascade {
            slot: slot_sheet,
            host: own_sheet.as_ref(),
        },
    );
    let saved_ctx = state.ctx;
    let saved_display = state.effective_parent;
    if dom.is_element(entity) {
        state.effective_parent = child_display(style.display, saved_display);
    }
    // CSS Values §6.1.1: rem is relative to the document element.
    let root_font_size = if Some(entity) == state.document_element {
        style.font_size
    } else {
        saved_ctx.root_font_size
    };
    state.ctx = saved_ctx.with_em_and_root(style.font_size, root_font_size);
    state.depth += 1;
    if let Some(sheet) = own_sheet {
        distribute_slots(dom, entity);
        walk_shadow_tree(dom, entity, sheets, shadow, &sheet, &style, state);
    } else {
        let assignment = shadow.and_then(|scope| {
            let assigned = dom.world().get::<&SlotAssignment>(entity).ok()?;
            (!assigned.assigned_nodes.is_empty()).then(|| (scope, assigned.assigned_nodes.clone()))
        });
        if let Some((scope, nodes)) = assignment {
            for node in nodes {
                walk_node(
                    dom,
                    node,
                    scope.outer_sheets,
                    scope.outer_scope,
                    Some((scope.host, scope.shadow_sheet)),
                    &style,
                    state,
                );
            }
        } else {
            for child in dom.children(entity) {
                walk_node(dom, child, sheets, shadow, None, &style, state);
            }
        }
    }
    state.depth -= 1;
    state.effective_parent = saved_display;
    state.ctx = saved_ctx;
}

/// Context for shadow tree walking, bundling stylesheet references.
struct ShadowWalkContext<'a> {
    host: Entity,
    outer_scope: Option<&'a ShadowWalkContext<'a>>,
    /// UA + shadow-internal stylesheets.
    shadow_sheets: ElementSheets<'a>,
    /// Outer (light DOM) stylesheets for slotted node cascade.
    outer_sheets: ElementSheets<'a>,
    /// The shadow stylesheet itself (for `::slotted()` cascade).
    shadow_sheet: &'a Stylesheet,
}

/// Walk a shadow host's shadow tree with scoped stylesheets.
///
/// 1. Builds shadow stylesheet list (UA + shadow-internal).
/// 2. Walks shadow tree children with the shadow context.
/// 3. When a `<slot>` with assigned nodes is encountered, each slotted node is
///    resolved with its element sheets, the slot's sheet, and its own host sheet
///    when present. It inherits the slot's computed style.
fn walk_shadow_tree(
    dom: &mut EcsDom,
    host: Entity,
    outer_stylesheets: ElementSheets<'_>,
    outer_scope: Option<&ShadowWalkContext<'_>>,
    shadow_sheet: &Stylesheet,
    host_style: &ComputedStyle,
    state: &mut WalkState<'_>,
) {
    let Some(shadow_root) = dom.get_shadow_root(host) else {
        return;
    };

    // Build shadow stylesheet list: UA + shadow-internal only.
    let ua = crate::ua::ua_stylesheet();
    let shadow_ctx = ShadowWalkContext {
        host,
        outer_scope,
        shadow_sheets: ElementSheets {
            list: &[ua, shadow_sheet],
            author_host: Some(host),
        },
        outer_sheets: outer_stylesheets,
        shadow_sheet,
    };

    // Walk shadow root's children with shadow stylesheets.
    let children = dom.children(shadow_root);
    for child in children {
        walk_node(
            dom,
            child,
            shadow_ctx.shadow_sheets,
            Some(&shadow_ctx),
            None,
            host_style,
            state,
        );
    }
}

fn host_stylesheet(dom: &EcsDom, host: Entity, total_css: &mut usize) -> Option<Stylesheet> {
    let root = dom.get_shadow_root(host)?;
    Some(parse_stylesheet(
        &collect_shadow_styles(dom, root, total_css),
        Origin::Author,
    ))
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
            // Only authored text contributes to the sheet on a subsequent style pass.
            for text_child in dom.children(child) {
                if is_generated_pseudo(dom, text_child) {
                    continue;
                }
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
        .is_ok_and(|t| matches!(t.0.as_str(), "a" | "area" | "link"));
    if !is_link_tag {
        return false;
    }
    dom.world()
        .get::<&Attributes>(entity)
        .is_ok_and(|attrs| attrs.get("href").is_some())
}

// ---------------------------------------------------------------------------
// Root overflow propagation (CSS Overflow L3 §3.1)
// ---------------------------------------------------------------------------

/// Find the `<html>` element entity among the roots.
fn find_html_entity(dom: &EcsDom) -> Option<Entity> {
    let roots = find_roots(dom);
    for root in roots {
        // The root itself might be <html>.
        if dom.has_tag(root, "html") {
            return Some(root);
        }
        // Or <html> might be a child of the document root.
        for child in dom.children(root) {
            if dom.has_tag(child, "html") {
                return Some(child);
            }
        }
    }
    None
}

/// Find the first `<body>` child of `html`.
fn find_body_child(dom: &EcsDom, html: Entity) -> Option<Entity> {
    dom.children(html)
        .into_iter()
        .find(|&child| dom.has_tag(child, "body"))
}

/// Propagate root element overflow to the viewport (CSS Overflow L3 §3.1).
///
/// The algorithm:
/// 1. Find `<html>` element.
/// 2. If html has non-visible overflow → propagate html's overflow to viewport, reset html to visible.
/// 3. If html is visible on both axes → check first `<body>` child:
///    - If body has non-visible overflow → propagate body's overflow, reset body to visible.
/// 4. Otherwise → default viewport overflow (auto/auto).
///
/// Viewport normalization: `visible` → `auto`, `clip` → `hidden`.
pub(crate) fn propagate_root_overflow(dom: &mut EcsDom) -> ViewportOverflow {
    let Some(html) = find_html_entity(dom) else {
        return ViewportOverflow::default();
    };

    // Read html's computed style.
    let (html_display, html_over_x, html_over_y) = match dom.world().get::<&ComputedStyle>(html) {
        Ok(s) => (s.display, s.overflow_x, s.overflow_y),
        Err(_) => return ViewportOverflow::default(),
    };

    // CSS Overflow L3 §3.1: no propagation when root element has display:none.
    if html_display == Display::None {
        return ViewportOverflow::default();
    }

    // If html has non-visible overflow on either axis, propagate from html.
    if html_over_x != Overflow::Visible || html_over_y != Overflow::Visible {
        // Reset html's overflow to visible.
        if let Ok(mut s) = dom.world_mut().get::<&mut ComputedStyle>(html) {
            s.overflow_x = Overflow::Visible;
            s.overflow_y = Overflow::Visible;
        }
        return ViewportOverflow::from_propagated(html_over_x, html_over_y);
    }

    // html is visible on both axes — check body.
    // CSS Overflow L3 §3.1: body must have display not none for fallback.
    let Some(body) = find_body_child(dom, html) else {
        return ViewportOverflow::default();
    };

    let (body_display, body_over_x, body_over_y) = match dom.world().get::<&ComputedStyle>(body) {
        Ok(s) => (s.display, s.overflow_x, s.overflow_y),
        Err(_) => return ViewportOverflow::default(),
    };

    if body_display == Display::None {
        return ViewportOverflow::default();
    }

    if body_over_x != Overflow::Visible || body_over_y != Overflow::Visible {
        // Reset body's overflow to visible.
        if let Ok(mut s) = dom.world_mut().get::<&mut ComputedStyle>(body) {
            s.overflow_x = Overflow::Visible;
            s.overflow_y = Overflow::Visible;
        }
        return ViewportOverflow::from_propagated(body_over_x, body_over_y);
    }

    ViewportOverflow::default()
}
