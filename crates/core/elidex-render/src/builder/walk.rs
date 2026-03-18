//! Pre-order tree walk for display list building.

use std::sync::Arc;

use elidex_ecs::{
    BackgroundImages, EcsDom, Entity, ImageData, TemplateContent, MAX_ANCESTOR_DEPTH,
};
use elidex_form::FormControlState;
use elidex_plugin::background::{BgRepeat, BgRepeatAxis};
use elidex_plugin::transform_math::resolve_child_perspective;
use elidex_plugin::{ComputedStyle, Display, LayoutBox, ListStyleType, Visibility};
use elidex_text::FontDatabase;

use crate::display_list::{DisplayItem, DisplayList};
use crate::font_cache::FontCache;

use super::form::emit_form_control;
use super::transform::{element_transform, TransformResult};
use super::{emit_background, emit_borders, emit_inline_run, emit_list_marker_with_counter};

/// Pre-order walk: emit paint commands for this entity, then recurse.
///
/// Children are grouped into "inline runs" (consecutive non-block children)
/// and "block children" (those with a `LayoutBox`). Inline runs have their
/// text collected and rendered as a single item; block children are
/// recursed into normally.
///
/// Recursion is capped at `MAX_ANCESTOR_DEPTH` to prevent stack overflow.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub(crate) fn walk(
    dom: &EcsDom,
    entity: Entity,
    font_db: &FontDatabase,
    font_cache: &mut FontCache,
    dl: &mut DisplayList,
    depth: usize,
    caret_visible: bool,
    parent_perspective: Option<f32>,
    parent_perspective_origin: (f64, f64),
) {
    if depth > MAX_ANCESTOR_DEPTH {
        return;
    }
    // Skip <template> elements — their content is inert.
    if dom.world().get::<&TemplateContent>(entity).is_ok() {
        return;
    }

    // Fetch ComputedStyle once for display/visibility/painting checks.
    let style_ref = dom.world().get::<&ComputedStyle>(entity).ok();

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
    if let Ok(lb) = dom.world().get::<&LayoutBox>(entity) {
        cached_border_box = Some(lb.border_box());
        if let Some(ref style) = style_ref {
            // CSS Transforms: compute and emit PushTransform before any painting.
            match element_transform(style, &lb, parent_perspective, parent_perspective_origin) {
                TransformResult::BackfaceHidden => {
                    // CSS Transforms L2 §5: back-facing → skip entire subtree.
                    return;
                }
                TransformResult::Affine(affine) => {
                    dl.push(DisplayItem::PushTransform { affine });
                    has_transform_push = true;
                }
                TransformResult::None => {}
            }

            if is_visible {
                let bg_images = dom.world().get::<&BackgroundImages>(entity).ok();
                emit_background(
                    &lb,
                    style.background_color,
                    style.border_radii,
                    style.opacity,
                    bg_images.as_deref(),
                    style,
                    dl,
                );
                emit_borders(&lb, style, dl);

                // Emit image for replaced elements with decoded pixel data.
                if let Ok(image_data) = dom.world().get::<&ImageData>(entity) {
                    if style.opacity > 0.0 && image_data.width > 0 && image_data.height > 0 {
                        dl.push(DisplayItem::Image {
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
                if let Ok(fcs) = dom.world().get::<&FormControlState>(entity) {
                    emit_form_control(
                        &lb,
                        &fcs,
                        style,
                        font_db,
                        font_cache,
                        dl,
                        dom.world()
                            .get::<&elidex_ecs::ElementState>(entity)
                            .ok()
                            .is_some_and(|s| s.contains(elidex_ecs::ElementState::FOCUS)),
                        caret_visible,
                    );
                }
            }

            // overflow clipping → clip children to padding box (CSS Overflow §3).
            if style.clips_overflow() {
                let pb = lb.padding_box();
                dl.push(DisplayItem::PushClip {
                    rect: pb,
                    radii: [0.0; 4],
                });
                has_clip = true;
            }
        }
    }

    // Compute perspective to propagate to children.
    let (child_perspective, child_perspective_origin) = match (&style_ref, cached_border_box) {
        (Some(style), Some(bb)) => resolve_child_perspective(style, &bb),
        _ => (None, (0.0, 0.0)),
    };

    // Process children in inline runs vs block children.
    // Flatten display:contents children — they generate no box, their
    // children are promoted to this formatting context.
    let children = elidex_layout_block::composed_children_flat(dom, entity);
    let mut inline_run = Vec::new();
    let mut list_counter = 0_usize;

    for &child in &children {
        if is_block_child(dom, child) {
            // Flush any pending inline run before the block child.
            if !inline_run.is_empty() {
                emit_inline_run(dom, entity, &inline_run, font_db, font_cache, dl);
                inline_run.clear();
            }

            // Emit list marker for list-item children.
            // Counter increments for every list-item regardless of list-style-type;
            // list-style-type: none only suppresses marker rendering.
            // visibility: hidden also suppresses marker painting.
            if let Ok(child_style) = dom.world().get::<&ComputedStyle>(child) {
                if child_style.display == Display::ListItem {
                    list_counter += 1;
                    if child_style.list_style_type != ListStyleType::None
                        && child_style.visibility == Visibility::Visible
                    {
                        if let Ok(child_lb) = dom.world().get::<&LayoutBox>(child) {
                            emit_list_marker_with_counter(
                                &child_lb,
                                &child_style,
                                list_counter,
                                font_db,
                                font_cache,
                                dl,
                            );
                        }
                    }
                }
            }

            // Recurse into block child.
            walk(
                dom,
                child,
                font_db,
                font_cache,
                dl,
                depth + 1,
                caret_visible,
                child_perspective,
                child_perspective_origin,
            );
        } else {
            // Text node or inline element — add to current run.
            inline_run.push(child);
        }
    }

    // Flush trailing inline run.
    if !inline_run.is_empty() {
        emit_inline_run(dom, entity, &inline_run, font_db, font_cache, dl);
    }

    if has_clip {
        dl.push(DisplayItem::PopClip);
    }
    if has_transform_push {
        dl.push(DisplayItem::PopTransform);
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
        .is_some_and(|style| is_block_display(style.display))
}

/// Returns `true` for display values that generate block-level boxes.
///
/// Atomic inline-level boxes (`inline-block`, `inline-flex`, etc.) return
/// `false` — they participate in inline runs and are rendered inline.
fn is_block_display(display: Display) -> bool {
    matches!(
        display,
        Display::Block
            | Display::Flex
            | Display::Grid
            | Display::ListItem
            | Display::Table
            | Display::TableRow
            | Display::TableRowGroup
            | Display::TableHeaderGroup
            | Display::TableFooterGroup
            | Display::TableCell
            | Display::TableColumn
            | Display::TableColumnGroup
            | Display::TableCaption
    )
}
