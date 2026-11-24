//! Block child stacking, shifting, and height resolution.

mod helpers;
mod shift;
mod stack;

use std::collections::HashMap;

use elidex_ecs::Entity;
use elidex_plugin::{BreakValue, Point};

use crate::{BreakToken, BreakTokenData};

pub(crate) use helpers::max_content_width;
pub use helpers::resolve_block_height;
pub(in crate::block) use shift::shift_block_children;
pub use shift::{shift_descendants, shift_descendants_excluding_own_fragments};
pub use stack::stack_block_children;

/// Result of stacking block children, including margin info for parent-child collapse.
pub struct StackResult {
    pub contribution: crate::source_progress::SourceProgress,
    /// Total content height consumed by stacked children.
    pub height: f32,
    /// Top margin of the first block child (for parent-child collapse).
    pub first_child_margin_top: Option<f32>,
    /// Bottom margin of the last block child (for parent-child collapse).
    pub last_child_margin_bottom: Option<f32>,
    /// Static positions for absolutely positioned descendants
    /// (CSS 2.1 §10.3.7 left/right / §10.6.4 top). Physical x/y, as stored.
    pub static_positions: HashMap<Entity, Point>,
    /// First baseline from children (CSS 2.1 §10.8.1).
    ///
    /// For block formatting contexts: the first in-flow child's baseline,
    /// offset-adjusted to the parent's content area.
    /// For anonymous inline runs: the inline layout's first baseline.
    pub first_baseline: Option<f32>,
    /// Break token if layout was interrupted by a fragmentainer break.
    pub break_token: Option<BreakToken>,
    /// Propagated forced break-before from first child (CSS Frag L3 §3.1.1).
    pub propagated_break_before: Option<BreakValue>,
    /// Propagated forced break-after from last child (CSS Frag L3 §3.1.1).
    pub propagated_break_after: Option<BreakValue>,
}

/// Build a `BreakToken` for a block break point.
///
/// DRY helper — avoids repeating the `BreakToken` / `BreakTokenData::Block`
/// construction at every break site in `stack_block_children`.
fn make_block_break_token(
    parent_entity: Entity,
    consumed_block_size: f32,
    child_index: usize,
    continuation: crate::BlockContinuation,
) -> BreakToken {
    BreakToken {
        entity: parent_entity,
        consumed_block_size,
        mode_data: Some(BreakTokenData::Block {
            coverage: crate::source_progress::SourceProgress::default(),
            child_index,
            continuation,
        }),
    }
}

fn participates(dom: &elidex_ecs::EcsDom, child: Entity) -> bool {
    crate::try_get_style(dom, child).is_none_or(|style| {
        style.display != elidex_plugin::Display::None
            && !crate::positioned::is_absolutely_positioned(&style)
    })
}

/// Validate a caller-supplied Block continuation before publishing the next fragment.
pub(super) fn validate_route(
    dom: &elidex_ecs::EcsDom,
    owner: Entity,
    children: &[Entity],
    token: Option<&BreakToken>,
) {
    let Some(token) = token else { return };
    let Some(BreakTokenData::Block {
        child_index,
        continuation,
        coverage,
    }) = &token.mode_data
    else {
        return;
    };
    assert_eq!(token.entity, owner, "Block continuation owner mismatch");
    let (prefix, rest) = children.split_at(*child_index);
    let index = coverage.index();
    assert!(
        prefix
            .iter()
            .all(|&c| !participates(dom, c) || index.is_finished(owner, c)),
        "Block route skips unconsumed source"
    );
    match continuation {
        crate::BlockContinuation::ChildBoundary => {
            if let Some(&child) = rest.iter().find(|&&c| participates(dom, c)) {
                assert!(
                    !index.is_finished(owner, child),
                    "Block route restarts consumed source"
                );
            }
        }
        crate::BlockContinuation::InterruptedChild(nested) => {
            assert_eq!(
                rest.first(),
                Some(&nested.entity),
                "interrupted child identity mismatch"
            );
            assert!(
                !index.is_finished(owner, nested.entity),
                "interrupted child already complete"
            );
        }
        crate::BlockContinuation::LocalRun { first, .. } => {
            assert_eq!(rest.first(), Some(first), "local run identity mismatch");
        }
    }
}
