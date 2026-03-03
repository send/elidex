//! Pre-order tree walk for display list building.

use elidex_ecs::{EcsDom, Entity, ImageData};
use elidex_plugin::{ComputedStyle, Display, LayoutBox, ListStyleType, Overflow};
use elidex_text::FontDatabase;

use crate::display_list::{DisplayItem, DisplayList};
use crate::font_cache::FontCache;

use super::{
    emit_background, emit_borders, emit_image, emit_inline_run, emit_list_marker_with_counter,
};

/// Pre-order walk: emit paint commands for this entity, then recurse.
///
/// Children are grouped into "inline runs" (consecutive non-block children)
/// and "block children" (those with a `LayoutBox`). Inline runs have their
/// text collected and rendered as a single item; block children are
/// recursed into normally.
pub(crate) fn walk(
    dom: &EcsDom,
    entity: Entity,
    font_db: &FontDatabase,
    font_cache: &mut FontCache,
    dl: &mut DisplayList,
) {
    // Check for display: none — skip this subtree entirely.
    // This check is independent of LayoutBox: an element without a LayoutBox
    // but with display:none should still be skipped.
    if let Ok(style) = dom.world().get::<&ComputedStyle>(entity) {
        if style.display == Display::None {
            return;
        }
    }

    // Emit background + borders for elements with a LayoutBox.
    let mut has_clip = false;
    if let Ok(lb) = dom.world().get::<&LayoutBox>(entity) {
        if let Ok(style) = dom.world().get::<&ComputedStyle>(entity) {
            emit_background(
                &lb,
                style.background_color,
                style.border_radius,
                style.opacity,
                dl,
            );
            emit_borders(&lb, &style, dl);

            // Emit image for replaced elements with decoded pixel data.
            if let Ok(image_data) = dom.world().get::<&ImageData>(entity) {
                if style.opacity > 0.0 {
                    emit_image(&lb, &image_data, style.opacity, dl);
                }
            }

            // overflow: hidden → clip children to padding box (CSS Overflow §3).
            if style.overflow == Overflow::Hidden {
                let pb = lb.padding_box();
                dl.push(DisplayItem::PushClip { rect: pb });
                has_clip = true;
            }

            // List marker rendering — counter is managed per-parent, passed down.
            // The walk function handles this instead (see below).
        }
    }

    // Process children in inline runs vs block children.
    let children: Vec<Entity> = dom.children_iter(entity).collect();
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
            if let Ok(child_style) = dom.world().get::<&ComputedStyle>(child) {
                if child_style.display == Display::ListItem {
                    list_counter += 1;
                    if child_style.list_style_type != ListStyleType::None {
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
            walk(dom, child, font_db, font_cache, dl);
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
}

/// Check whether a child entity is a block-level child (has a `LayoutBox`).
///
/// Block children are recursed into separately; non-block children (text
/// nodes and inline elements) are collected into inline runs.
pub(crate) fn is_block_child(dom: &EcsDom, entity: Entity) -> bool {
    dom.world().get::<&LayoutBox>(entity).is_ok()
}
