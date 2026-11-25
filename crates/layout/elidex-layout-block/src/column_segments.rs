//! Source order for column layout and continuation comparisons.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{ColumnSpan, ComputedStyle, Display, Float, Position};

/// Segment of children between `column-span: all` elements.
pub enum Segment {
    /// Normal children to be laid out across columns.
    Normal(Vec<Entity>),
    /// A `column-span: all` element that spans the full container width.
    Spanner(Entity),
}

/// Split children into segments around `column-span: all` elements.
///
/// CSS Multi-column L1 §5: `column-span: all` only applies to in-flow
/// block-level elements. Out-of-flow and inline-level are excluded.
///
/// Absolutely/fixedly positioned children are excluded from column content
/// entirely — they are laid out separately via `layout_positioned_children`.
pub fn split_segments(dom: &EcsDom, children: &[Entity]) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut current_normal: Vec<Entity> = Vec::new();

    for &child in children {
        let child_style = dom.world().get::<&ComputedStyle>(child).ok();

        // Skip display:none children — they generate no boxes.
        if child_style
            .as_ref()
            .is_some_and(|s| s.display == Display::None)
        {
            continue;
        }

        // Skip absolutely/fixedly positioned children — they're out-of-flow.
        let is_out_of_flow = child_style
            .as_ref()
            .is_some_and(|s| matches!(s.position, Position::Absolute | Position::Fixed));
        if is_out_of_flow {
            continue;
        }

        let is_spanner = child_style.is_some_and(|style| {
            style.column_span == ColumnSpan::All
                && style.display != Display::Inline
                && style.float == Float::None
        });

        if is_spanner {
            if !current_normal.is_empty() {
                segments.push(Segment::Normal(std::mem::take(&mut current_normal)));
            }
            segments.push(Segment::Spanner(child));
        } else {
            current_normal.push(child);
        }
    }

    if !current_normal.is_empty() {
        segments.push(Segment::Normal(current_normal));
    }

    segments
}

/// Only a multicol's direct formatting children have their slice drained by fill.
pub(crate) fn is_column_item(dom: &EcsDom, entity: Entity) -> bool {
    let mut current = entity;
    for _ in 0..crate::MAX_LAYOUT_DEPTH {
        let Some(parent) = dom.get_parent(current) else {
            return false;
        };
        let slot = dom
            .world()
            .get::<&elidex_ecs::SlottedMarker>(current)
            .ok()
            .map(|m| m.slot);
        current = slot.or_else(|| dom.shadow_host(parent)).unwrap_or(parent);
        if let Ok(style) = dom.world().get::<&ComputedStyle>(current) {
            if style.display != Display::Contents {
                return elidex_plugin::is_multicol(&style);
            }
        } else {
            return false;
        }
    }
    false
}
