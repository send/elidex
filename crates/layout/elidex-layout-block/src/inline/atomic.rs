//! Layout of atomic inline items (inline-block, inline-flex, etc.).

use elidex_ecs::EcsDom;
use elidex_plugin::Point;
use elidex_text::FontDatabase;

use super::InlineItem;

/// Layout all atomic inline items (`inline-block`, etc.) and fill their dimensions.
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
) {
    for item in items.iter_mut() {
        if let InlineItem::Atomic {
            entity,
            inline_size,
            block_size,
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
            };
            let lb = layout_child(dom, *entity, &input).layout_box;
            let margin_box = lb.margin_box();
            if is_vertical {
                *inline_size = margin_box.size.height;
                *block_size = margin_box.size.width;
            } else {
                *inline_size = margin_box.size.width;
                *block_size = margin_box.size.height;
            }
        }
    }
}
