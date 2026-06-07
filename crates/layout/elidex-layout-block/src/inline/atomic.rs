//! Layout of atomic inline items (inline-block, inline-flex, etc.).

use std::collections::HashMap;

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::Point;
use elidex_text::FontDatabase;

use super::InlineItem;

/// Layout all atomic inline items (`inline-block`, etc.) and fill their dimensions.
///
/// Returns a per-atomic map `entity → un-offset margin-box origin`: the origin the
/// box has BEFORE any relative offset, taken from the un-offset `LayoutBox`
/// `layout_child` returns (`dispatch_layout_child` bakes `apply_relative_offset`
/// into the *ECS* box for `position:relative` but returns the un-offset box). The
/// persist block uses this as the reposition delta basis (`reposition_atomic_box`):
/// `delta = on-line target − un-offset origin` repositions a relpos atomic to its
/// on-line position while preserving its applied offset, and is byte-identical to
/// reading the ECS box origin for a static/sticky atomic (no offset baked) — so the
/// static path is unchanged (slice 3p-a) and the positioned path converges
/// (slice 3p-b-2). The basis is the un-offset origin (not `content_origin`) so a
/// vertical-rl asymmetric box does not shift (its un-offset `margin_box().origin`
/// differs from `content_origin`).
#[allow(clippy::too_many_arguments)]
pub(super) fn layout_atomic_items(
    dom: &mut EcsDom,
    items: &mut [InlineItem],
    containing_inline_size: f32,
    content_origin: Point,
    font_db: &FontDatabase,
    layout_child: crate::ChildLayoutFn,
    is_vertical: bool,
    layout_generation: u32,
    is_probe: bool,
) -> HashMap<Entity, Point> {
    let mut unoffset_origins = HashMap::new();
    for item in items.iter_mut() {
        if let InlineItem::Atomic {
            entity,
            inline_size,
            block_size,
            ..
        } = item
        {
            let input = crate::LayoutInput {
                containing: elidex_plugin::CssSize::width_only(containing_inline_size),
                containing_inline_size,
                offset: content_origin,
                font_db,
                depth: 0,
                float_ctx: None,
                viewport: None,
                fragmentainer: None,
                break_token: None,
                subgrid: None,
                layout_generation,
                is_probe,
            };
            let lb = layout_child(dom, *entity, &input).layout_box;
            let margin_box = lb.margin_box();
            // The returned box is un-offset (dispatch returns the original box;
            // the relative offset is inserted into ECS separately). Record its
            // origin as the reposition delta basis (see doc above).
            unoffset_origins.insert(*entity, margin_box.origin);
            if is_vertical {
                *inline_size = margin_box.size.height;
                *block_size = margin_box.size.width;
            } else {
                *inline_size = margin_box.size.width;
                *block_size = margin_box.size.height;
            }
        }
    }
    unoffset_origins
}
