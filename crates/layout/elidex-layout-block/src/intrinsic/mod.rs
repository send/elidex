//! Intrinsic sizing computation (CSS Sizing Level 3).
//!
//! Provides [`compute_intrinsic_sizes`] which computes min-content and
//! max-content intrinsic sizes for any element, routing to the appropriate
//! algorithm based on `display` type.

mod block;
mod table;

use crate::{
    composed_children_flat, get_style, inline_pb, sanitize_border, sanitize_padding,
    IntrinsicSizes, LayoutEnv, MAX_LAYOUT_DEPTH,
};
use elidex_ecs::{EcsDom, Entity};

use elidex_plugin::{BoxSizing, Dimension, Display, WritingModeContext};

use block::{compute_block_intrinsic, compute_flex_intrinsic};
use table::compute_table_intrinsic;

pub type GridIntrinsicFn = fn(&mut EcsDom, Entity, &[Entity], &LayoutEnv<'_>) -> IntrinsicSizes;

/// Compute min-content and max-content intrinsic inline sizes for an element.
///
/// Writing-mode-aware: uses the element's `writing-mode` to determine the
/// inline axis, so vertical writing modes compute intrinsic sizes along the
/// vertical (block-flow) direction.
///
/// Routes to display-specific intrinsic sizing:
/// - **Block**: inline children → inline min/max; block children → max of children.
/// - **Flex row**: nowrap → sum, wrap → max for min-content.
/// - **Flex column**: max of children's intrinsic widths.
/// - **Grid**: placement-based per-column track sizing via `elidex-layout-grid`.
/// - **Table**: cell → column → table (CSS 2.1 §17.5.2).
/// - **Inline/Text**: delegates to inline measurement.
pub fn compute_intrinsic_sizes(
    dom: &mut EcsDom,
    entity: Entity,
    env: &LayoutEnv<'_>,
    grid: GridIntrinsicFn,
) -> IntrinsicSizes {
    if env.depth >= MAX_LAYOUT_DEPTH {
        return IntrinsicSizes::default();
    }

    let style = get_style(dom, entity);

    // Container padding + border contribute to intrinsic inline size (CSS Sizing L3 §5).
    let padding = sanitize_padding(&style);
    let border = sanitize_border(&style);
    let wm = WritingModeContext::new(style.writing_mode, style.direction);
    let pb = inline_pb(&wm, &padding, &border);

    // Check for replaced elements or explicit sizes first.
    if let Some(sizes) = explicit_intrinsic(dom, entity, &style, pb) {
        return sizes;
    }

    let children = composed_children_flat(dom, entity);

    if children.is_empty() {
        return IntrinsicSizes {
            min_content: pb,
            max_content: pb,
        };
    }

    let content = match style.display {
        Display::Flex | Display::InlineFlex => {
            compute_flex_intrinsic(dom, entity, &children, env, grid)
        }
        Display::Grid | Display::InlineGrid => grid(dom, entity, &children, env),
        Display::Table | Display::InlineTable => {
            compute_table_intrinsic(dom, entity, &children, env, grid)
        }
        _ => compute_block_intrinsic(dom, entity, &children, env, grid),
    };

    IntrinsicSizes {
        min_content: content.min_content + pb,
        max_content: content.max_content + pb,
    }
}

/// Measure the inline content box, excluding the item's own intrinsic decorations.
pub fn inner_intrinsic_sizes(
    dom: &mut EcsDom,
    entity: Entity,
    env: &LayoutEnv<'_>,
    grid: GridIntrinsicFn,
) -> IntrinsicSizes {
    let sizes = compute_intrinsic_sizes(dom, entity, env, grid);
    let style = get_style(dom, entity);
    let wm = WritingModeContext::new(style.writing_mode, style.direction);
    let pb = inline_pb(&wm, &sanitize_padding(&style), &sanitize_border(&style));
    IntrinsicSizes {
        min_content: (sizes.min_content - pb).max(0.0),
        max_content: (sizes.max_content - pb).max(0.0),
    }
}

/// Return intrinsic sizes for replaced elements or elements with explicit inline-axis size.
///
/// Replaced elements (images, form controls) have intrinsic dimensions.
/// Elements with explicit inline-axis dimension (`width` in horizontal-tb,
/// `height` in vertical modes) use that as both min and max content.
fn explicit_intrinsic(
    dom: &EcsDom,
    entity: Entity,
    style: &elidex_plugin::ComputedStyle,
    pb: f32,
) -> Option<IntrinsicSizes> {
    let inline_horizontal =
        WritingModeContext::new(style.writing_mode, style.direction).is_horizontal();

    // Replaced elements: use intrinsic image/form dimensions (inline-axis).
    if let Some(intr) = crate::get_intrinsic_size(dom, entity) {
        let inline_size = if inline_horizontal {
            intr.width
        } else {
            intr.height
        };
        return Some(IntrinsicSizes {
            min_content: inline_size + pb,
            max_content: inline_size + pb,
        });
    }
    // Explicit inline-axis dimension: acts as both min-content and max-content.
    let explicit_dim = if inline_horizontal {
        style.width
    } else {
        style.height
    };
    if let Dimension::Length(len) = explicit_dim {
        // border-box: dimension already includes padding+border.
        let size = if style.box_sizing == BoxSizing::BorderBox {
            len
        } else {
            len + pb
        };
        return Some(IntrinsicSizes {
            min_content: size,
            max_content: size,
        });
    }
    None
}

/// Compute shrink-to-fit width for inline-level containers.
///
/// CSS 2.1 §10.3.5: shrink-to-fit width = min(max-content, max(min-content, `available_width`)).
#[must_use]
pub fn shrink_to_fit_width(intrinsic: &IntrinsicSizes, available_width: f32) -> f32 {
    intrinsic
        .max_content
        .min(intrinsic.min_content.max(available_width))
}
