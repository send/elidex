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
pub use shift::shift_descendants;
pub use stack::stack_block_children;

/// Result of stacking block children, including margin info for parent-child collapse.
pub struct StackResult {
    /// Total content height consumed by stacked children.
    pub height: f32,
    /// Top margin of the first block child (for parent-child collapse).
    pub first_child_margin_top: Option<f32>,
    /// Bottom margin of the last block child (for parent-child collapse).
    pub last_child_margin_bottom: Option<f32>,
    /// Static positions for absolutely positioned descendants (CSS 2.1 §10.6.5).
    pub static_positions: HashMap<Entity, Point>,
    /// First baseline from children (CSS 2.1 §10.8.1).
    ///
    /// For block formatting contexts: the first in-flow child's baseline,
    /// offset-adjusted to the parent's content area.
    /// For anonymous inline runs: the inline layout's first baseline.
    pub first_baseline: Option<f32>,
    /// Break token if layout was interrupted by a fragmentainer break.
    pub break_token: Option<BreakToken>,
    /// Propagated forced break-before from first child (CSS Frag L3 §3.2).
    pub propagated_break_before: Option<BreakValue>,
    /// Propagated forced break-after from last child (CSS Frag L3 §3.2).
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
    inline_break_line: Option<usize>,
    child_break_token: Option<Box<BreakToken>>,
) -> BreakToken {
    BreakToken {
        entity: parent_entity,
        consumed_block_size,
        child_break_token,
        mode_data: Some(BreakTokenData::Block {
            child_index,
            inline_break_line,
        }),
    }
}
