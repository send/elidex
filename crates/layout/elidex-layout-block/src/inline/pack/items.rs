//! Pack-item construction: [`PackItem`], [`FlowMember`], and [`build_pack_items`].
//!
//! Splits each `InlineItem` into the break-opportunity-delimited pieces the
//! [`LinePacker`](super::LinePacker) greedily packs. Split out of `pack.rs` to keep
//! the line packer under the repo's ~1000-line convention; all items are
//! pack-internal.

use elidex_ecs::Entity;
use elidex_text::BreakOpportunity;

use super::InlineItem;

// ---------------------------------------------------------------------------
// Break segment — a piece of a StyledRun between break opportunities
// ---------------------------------------------------------------------------

/// A piece of inline content for line packing — either a text segment or an atomic box.
pub(in crate::inline) enum PackItem {
    /// Text segment between break opportunities.
    Text {
        /// Index into the items array for style lookup.
        item_index: usize,
        /// The text of this segment.
        text: String,
        /// Break opportunity at the end of this segment, if any.
        break_after: Option<BreakOpportunity>,
    },
    /// An atomic inline box (no break inside).
    Atomic {
        /// Index into the items array.
        item_index: usize,
    },
    /// Absolutely positioned placeholder — records static position, zero-width.
    Placeholder { entity: Entity },
}

/// What `place_item` records for a placed item (only when persisting —
/// `flow_align.is_some()`). A text segment coalesces into a contiguous same-entity
/// [`InlineFlowRun::Text`]; a *static* atomic inline-level box becomes its own
/// [`InlineFlowRun::AtomicBox`] flow member (render `walk()`s it at its
/// repositioned `LayoutBox`); a *positioned* (relative/sticky) atomic is recorded
/// in the separate `relpos_atomic_placements` reposition bucket — NOT a flow
/// member, because render paints it in Layer 6 and a flow member would double-paint
/// (`emit_inline_flow` walks every member in Layer 5 too; slice 3p-b-2).
///
/// [`InlineFlowRun::Text`]: elidex_ecs::InlineFlowRun
/// [`InlineFlowRun::AtomicBox`]: elidex_ecs::InlineFlowRun
#[derive(Clone, Copy)]
pub(super) enum FlowMember<'a> {
    /// A text segment that contributes its `&str` to a text run.
    Text(&'a str),
    /// A static atomic inline-level box (no text; carries its own box geometry).
    Atomic,
    /// A `position:relative`/`sticky` atomic inline-level box: in-flow (advances the
    /// cursor) but painted in render Layer 6 from its own `LayoutBox`. Recorded in
    /// the reposition bucket, not a flow member (avoids double-paint, slice 3p-b-2).
    PositionedAtomic,
}

/// Build pack items from inline items.
///
/// Text runs are split at break opportunities. Atomic boxes become single pack items.
pub(in crate::inline) fn build_pack_items(items: &[InlineItem]) -> Vec<PackItem> {
    use elidex_text::find_break_opportunities;
    let mut pack_items = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        match item {
            InlineItem::Text(run) => {
                if run.text.is_empty() {
                    continue;
                }
                let breaks = find_break_opportunities(&run.text);
                let mut prev_pos = 0;
                for &(bp, kind) in &breaks {
                    if bp > prev_pos {
                        pack_items.push(PackItem::Text {
                            item_index: idx,
                            text: run.text[prev_pos..bp].to_string(),
                            break_after: Some(kind),
                        });
                    }
                    prev_pos = bp;
                }
                if prev_pos < run.text.len() {
                    pack_items.push(PackItem::Text {
                        item_index: idx,
                        text: run.text[prev_pos..].to_string(),
                        break_after: None,
                    });
                }
            }
            InlineItem::Atomic { .. } => {
                pack_items.push(PackItem::Atomic { item_index: idx });
            }
            InlineItem::Placeholder(entity) => {
                pack_items.push(PackItem::Placeholder { entity: *entity });
            }
        }
    }
    pack_items
}
