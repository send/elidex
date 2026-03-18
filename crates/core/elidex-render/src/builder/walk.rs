//! Pre-order tree walk for display list building.

use std::sync::Arc;

use elidex_ecs::{
    BackgroundImages, EcsDom, Entity, ImageData, TemplateContent, MAX_ANCESTOR_DEPTH,
};
use elidex_form::FormControlState;
use elidex_layout_block::paint_order::{collect_sc_participants, is_float_entity, is_positioned};
use elidex_plugin::background::{BgRepeat, BgRepeatAxis};
use elidex_plugin::transform_math::{resolve_child_perspective, Perspective};
use elidex_plugin::{ComputedStyle, Display, LayoutBox, ListStyleType, Visibility};
use elidex_text::FontDatabase;

use crate::display_list::{DisplayItem, DisplayList};
use crate::font_cache::FontCache;

use super::form::emit_form_control;
use super::transform::{element_transform, TransformResult};
use super::{emit_background, emit_borders, emit_inline_run, emit_list_marker_with_counter};

/// Shared mutable state threaded through the display list walk.
///
/// Groups the invariant references and mutable buffers that every
/// recursive `walk` call needs, reducing per-call argument counts.
pub(crate) struct PaintContext<'a> {
    pub(crate) dom: &'a EcsDom,
    pub(crate) font_db: &'a FontDatabase,
    pub(crate) font_cache: &'a mut FontCache,
    pub(crate) dl: &'a mut DisplayList,
    pub(crate) caret_visible: bool,
    /// Viewport scroll offset `(x, y)`.
    ///
    /// This is the same value used for the root-level `PushScrollOffset`
    /// in `build_display_list_with_scroll()`. Fixed elements re-push this
    /// value after their `PopScrollOffset`/walk/`PushScrollOffset` sequence.
    pub(crate) scroll_offset: (f32, f32),
}

/// Pre-order walk: emit paint commands for this entity, then recurse.
///
/// Children are grouped into "inline runs" (consecutive non-block children)
/// and "block children" (those with a `LayoutBox`). Inline runs have their
/// text collected and rendered as a single item; block children are
/// recursed into normally.
///
/// For stacking contexts, children are painted in CSS 2.1 Appendix E order:
/// Layer 2 (negative z) → Layer 3 (blocks) → Layer 4 (floats) →
/// Layer 5 (inline) → Layer 6 (positioned auto + z:0) → Layer 7 (positive z).
///
/// Recursion is capped at `MAX_ANCESTOR_DEPTH` to prevent stack overflow.
#[allow(clippy::too_many_lines)]
pub(crate) fn walk(
    ctx: &mut PaintContext,
    entity: Entity,
    depth: usize,
    parent_perspective: &Perspective,
    in_transform: bool,
) {
    if depth > MAX_ANCESTOR_DEPTH {
        return;
    }
    // Skip <template> elements — their content is inert.
    if ctx.dom.world().get::<&TemplateContent>(entity).is_ok() {
        return;
    }

    // Fetch ComputedStyle once for display/visibility/painting checks.
    let style_ref = ctx.dom.world().get::<&ComputedStyle>(entity).ok();

    // Check for display: none — skip this subtree entirely.
    if let Some(ref style) = style_ref {
        if style.display == Display::None {
            return;
        }
        // CSS 2.1 §11.2: visibility: collapse on table-row, table-column,
        // table-row-group, or table-column-group hides the entire row/column
        // (equivalent to display: none for rendering purposes).
        if style.visibility == Visibility::Collapse && style.display.is_table_internal() {
            return;
        }
    }

    // Check visibility — hidden elements skip painting but still occupy space
    // and children can override visibility, so we must recurse.
    // For non-table elements, 'collapse' is treated the same as 'hidden'.
    let is_visible = style_ref
        .as_ref()
        .is_none_or(|s| s.visibility == Visibility::Visible);

    // Emit background + borders + images for elements with a LayoutBox.
    let mut has_clip = false;
    let mut has_transform_push = false;
    // Cache border box for perspective-origin computation (avoids redundant ECS lookup).
    let mut cached_border_box = None;
    if let Ok(lb) = ctx.dom.world().get::<&LayoutBox>(entity) {
        cached_border_box = Some(lb.border_box());
        if let Some(ref style) = style_ref {
            // CSS Transforms: compute and emit PushTransform before any painting.
            match element_transform(style, &lb, parent_perspective) {
                TransformResult::BackfaceHidden => {
                    // CSS Transforms L2 §5: back-facing → skip entire subtree.
                    return;
                }
                TransformResult::Affine(affine) => {
                    ctx.dl.push(DisplayItem::PushTransform { affine });
                    has_transform_push = true;
                }
                TransformResult::None => {}
            }

            if is_visible {
                let bg_images = ctx.dom.world().get::<&BackgroundImages>(entity).ok();
                emit_background(
                    &lb,
                    style.background_color,
                    style.border_radii,
                    style.opacity,
                    bg_images.as_deref(),
                    style,
                    ctx.dl,
                );
                emit_borders(&lb, style, ctx.dl);

                // Emit image for replaced elements with decoded pixel data.
                if let Ok(image_data) = ctx.dom.world().get::<&ImageData>(entity) {
                    if style.opacity > 0.0 && image_data.width > 0 && image_data.height > 0 {
                        ctx.dl.push(DisplayItem::Image {
                            painting_area: lb.content,
                            pixels: Arc::clone(&image_data.pixels),
                            image_width: image_data.width,
                            image_height: image_data.height,
                            position: (0.0, 0.0),
                            size: (lb.content.width, lb.content.height),
                            repeat: BgRepeat {
                                x: BgRepeatAxis::NoRepeat,
                                y: BgRepeatAxis::NoRepeat,
                            },
                            opacity: style.opacity,
                        });
                    }
                }

                // Emit form control rendering.
                if let Ok(fcs) = ctx.dom.world().get::<&FormControlState>(entity) {
                    emit_form_control(
                        &lb,
                        &fcs,
                        style,
                        ctx.font_db,
                        ctx.font_cache,
                        ctx.dl,
                        ctx.dom
                            .world()
                            .get::<&elidex_ecs::ElementState>(entity)
                            .ok()
                            .is_some_and(|s| s.contains(elidex_ecs::ElementState::FOCUS)),
                        ctx.caret_visible,
                    );
                }
            }

            // overflow clipping → clip children to padding box (CSS Overflow §3).
            if style.clips_overflow() {
                let pb = lb.padding_box();
                ctx.dl.push(DisplayItem::PushClip {
                    rect: pb,
                    radii: [0.0; 4],
                });
                has_clip = true;
            }
        }
    }

    // Compute perspective to propagate to children.
    let child_perspective = match (&style_ref, cached_border_box) {
        (Some(style), Some(bb)) => resolve_child_perspective(style, &bb),
        _ => Perspective::default(),
    };

    // Process children: stacking context elements use 7-layer paint order.
    let children = elidex_layout_block::composed_children_flat(ctx.dom, entity);
    let is_sc = style_ref
        .as_ref()
        .is_some_and(|s| s.creates_stacking_context())
        || ctx.dom.get_parent(entity).is_none(); // root is always a SC

    let child_in_transform = in_transform || has_transform_push;

    if is_sc {
        paint_stacking_context_layers(
            ctx,
            entity,
            &children,
            depth,
            &child_perspective,
            child_in_transform,
        );
    } else {
        paint_non_sc(
            ctx,
            entity,
            &children,
            depth,
            &child_perspective,
            child_in_transform,
        );
    }

    if has_clip {
        ctx.dl.push(DisplayItem::PopClip);
    }
    if has_transform_push {
        ctx.dl.push(DisplayItem::PopTransform);
    }
}

/// Paint children in CSS 2.1 Appendix E stacking context layer order.
#[allow(clippy::similar_names)]
fn paint_stacking_context_layers(
    ctx: &mut PaintContext,
    entity: Entity,
    children: &[Entity],
    depth: usize,
    child_perspective: &Perspective,
    in_transform: bool,
) {
    let layers = collect_sc_participants(ctx.dom, children);

    // Layer 2: negative z stacking contexts (z ascending).
    for &child in &layers.negative_z {
        walk_child_with_fixed_check(ctx, child, depth, child_perspective, in_transform);
    }

    // Layer 3: in-flow non-positioned blocks (DOM order).
    let mut list_counter = 0_usize;
    for &child in &layers.in_flow_blocks {
        maybe_emit_list_marker(ctx, child, &mut list_counter);
        walk_child_with_fixed_check(ctx, child, depth, child_perspective, in_transform);
    }

    // Layer 4: non-positioned floats (DOM order).
    for &child in &layers.floats {
        walk_child_with_fixed_check(ctx, child, depth, child_perspective, in_transform);
    }

    // Layer 5: inline content (DOM order, positioned excluded).
    {
        let mut inline_run = Vec::new();
        for &child in &layers.all_children {
            if is_positioned(ctx.dom, child)
                || is_block_child(ctx.dom, child)
                || is_float_entity(ctx.dom, child)
            {
                if !inline_run.is_empty() {
                    emit_inline_run(
                        ctx.dom,
                        entity,
                        &inline_run,
                        ctx.font_db,
                        ctx.font_cache,
                        ctx.dl,
                    );
                    inline_run.clear();
                }
            } else {
                inline_run.push(child);
            }
        }
        if !inline_run.is_empty() {
            emit_inline_run(
                ctx.dom,
                entity,
                &inline_run,
                ctx.font_db,
                ctx.font_cache,
                ctx.dl,
            );
        }
    }

    // Layer 6: positioned auto + z:0 SC — DOM-order interleave.
    let mut layer6: Vec<Entity> = layers
        .positioned_auto
        .iter()
        .chain(layers.zero_z.iter())
        .copied()
        .collect();
    layer6.sort_by(|&a, &b| ctx.dom.tree_order_cmp(a, b));
    for &child in &layer6 {
        walk_child_with_fixed_check(ctx, child, depth, child_perspective, in_transform);
    }

    // Layer 7: positive z stacking contexts (z ascending).
    for &child in &layers.positive_z {
        walk_child_with_fixed_check(ctx, child, depth, child_perspective, in_transform);
    }
}

/// Paint children of a non-SC element in DOM order, skipping positioned
/// children (they are painted by the parent stacking context).
///
/// The `in_transform` flag is propagated to all children so that
/// `position: fixed` descendants inside a transform ancestor are
/// correctly treated as absolute (CSS Transforms L1 §2).
fn paint_non_sc(
    ctx: &mut PaintContext,
    entity: Entity,
    children: &[Entity],
    depth: usize,
    child_perspective: &Perspective,
    in_transform: bool,
) {
    let mut inline_run = Vec::new();
    let mut list_counter = 0_usize;

    for &child in children {
        // Skip positioned children — they're painted by the parent SC.
        if is_positioned(ctx.dom, child) {
            continue;
        }

        if is_block_child(ctx.dom, child) {
            // Flush any pending inline run before the block child.
            if !inline_run.is_empty() {
                emit_inline_run(
                    ctx.dom,
                    entity,
                    &inline_run,
                    ctx.font_db,
                    ctx.font_cache,
                    ctx.dl,
                );
                inline_run.clear();
            }

            maybe_emit_list_marker(ctx, child, &mut list_counter);

            // Recurse into block child.
            walk(ctx, child, depth + 1, child_perspective, in_transform);
        } else {
            // Text node or inline element — add to current run.
            inline_run.push(child);
        }
    }

    // Flush trailing inline run.
    if !inline_run.is_empty() {
        emit_inline_run(
            ctx.dom,
            entity,
            &inline_run,
            ctx.font_db,
            ctx.font_cache,
            ctx.dl,
        );
    }
}

/// Check whether a child entity is a block-level child.
///
/// Block children are recursed into separately; non-block children (text
/// nodes and inline elements) are collected into inline runs.
///
/// An entity is block-level if it has a `LayoutBox` AND a block-level
/// display type. Inline elements may also have a `LayoutBox` (assigned
/// during inline layout for background/border rendering) but should
/// still be treated as part of an inline run.
pub(crate) fn is_block_child(dom: &EcsDom, entity: Entity) -> bool {
    if dom.world().get::<&LayoutBox>(entity).is_err() {
        return false;
    }
    // Check display type — inline elements with LayoutBox are NOT block children.
    dom.world()
        .get::<&ComputedStyle>(entity)
        .ok()
        .is_some_and(|style| elidex_layout_block::block::is_block_level(style.display))
}

/// Walk a child entity, wrapping `position: fixed` (viewport-attached) elements
/// with `PopScrollOffset`/`PushScrollOffset` so they remain visually unscrolled.
///
/// The `PopScrollOffset`/`PushScrollOffset` pair must always be balanced:
/// both are emitted unconditionally when `is_viewport_fixed` is true,
/// and `walk()` never early-returns after the Pop has been emitted.
fn walk_child_with_fixed_check(
    ctx: &mut PaintContext,
    child: Entity,
    depth: usize,
    child_perspective: &Perspective,
    in_transform: bool,
) {
    let is_fixed_vp = is_viewport_fixed(ctx.dom, child, in_transform);
    if is_fixed_vp {
        ctx.dl.push(DisplayItem::PopScrollOffset);
    }
    walk(ctx, child, depth + 1, child_perspective, in_transform);
    if is_fixed_vp {
        ctx.dl.push(DisplayItem::PushScrollOffset {
            scroll_offset: ctx.scroll_offset,
        });
    }
}

/// `position: fixed` with no transform ancestor → viewport-attached (scroll excluded).
///
/// CSS Transforms L1 §2: a transform ancestor establishes a containing block
/// for fixed descendants, so they scroll with the transform ancestor.
#[must_use]
fn is_viewport_fixed(dom: &EcsDom, entity: Entity, in_transform: bool) -> bool {
    if in_transform {
        return false;
    }
    dom.world()
        .get::<&ComputedStyle>(entity)
        .ok()
        .is_some_and(|s| s.position == elidex_plugin::Position::Fixed)
}

/// Emit a list marker for a block child if it is a `list-item` with a visible marker.
fn maybe_emit_list_marker(ctx: &mut PaintContext, child: Entity, counter: &mut usize) {
    if let Ok(child_style) = ctx.dom.world().get::<&ComputedStyle>(child) {
        if child_style.display == Display::ListItem {
            *counter += 1;
            if child_style.list_style_type != ListStyleType::None
                && child_style.visibility == Visibility::Visible
            {
                if let Ok(child_lb) = ctx.dom.world().get::<&LayoutBox>(child) {
                    emit_list_marker_with_counter(
                        &child_lb,
                        &child_style,
                        *counter,
                        ctx.font_db,
                        ctx.font_cache,
                        ctx.dl,
                    );
                }
            }
        }
    }
}
