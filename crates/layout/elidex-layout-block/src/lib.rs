//! Block formatting context layout and shared layout helpers.
//!
//! Provides the block layout algorithm, inline formatting context,
//! and shared utilities (sanitize, box model helpers, etc.) used by
//! all layout algorithm crates.

pub mod block;
pub mod helpers;
pub mod inline;
pub mod paint_order;
pub mod positioned;

use std::cell::RefCell;

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{CssSize, EdgeSizes, LayoutBox, Point, Size};
use elidex_text::FontDatabase;

use crate::block::float::FloatContext;

// Re-export all helpers for backward compatibility.
pub use helpers::{
    adjust_min_max_for_border_box, block_pb, clamp_min_max, composed_children_flat,
    compute_inline_containing, effective_align, flatten_contents, get_intrinsic_size, get_style,
    horizontal_pb, inline_pb, resolve_box_model, resolve_content_width, resolve_dimension_value,
    resolve_explicit_height, resolve_min_max, resolve_padding, restore_descendant_styles, sanitize,
    sanitize_border, sanitize_edge_values, sanitize_non_negative, sanitize_padding,
    save_descendant_styles, total_gap, try_get_style, vertical_pb,
};

// ---------------------------------------------------------------------------
// Fragmentation types (CSS Fragmentation Level 3)
// ---------------------------------------------------------------------------

/// Type of fragmentation context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FragmentationType {
    /// Page-based fragmentation (CSS Paged Media).
    Page,
    /// Column-based fragmentation (CSS Multi-column).
    Column,
}

/// Context passed into layout when inside a fragmentation container.
#[derive(Clone, Copy, Debug)]
pub struct FragmentainerContext {
    /// Available block-axis size before the next break opportunity.
    pub available_block_size: f32,
    /// Type of fragmentation.
    pub fragmentation_type: FragmentationType,
}

/// Result of a layout pass, including an optional break token for fragmentation.
#[derive(Clone, Debug)]
pub struct LayoutOutcome {
    /// The layout box produced by this fragment.
    pub layout_box: LayoutBox,
    /// If layout was interrupted by a fragmentainer break, the token to resume.
    pub break_token: Option<BreakToken>,
    /// CSS Fragmentation L3 3.2: propagated forced break-before from first child.
    pub propagated_break_before: Option<elidex_plugin::BreakValue>,
    /// CSS Fragmentation L3 3.2: propagated forced break-after from last child.
    pub propagated_break_after: Option<elidex_plugin::BreakValue>,
}

impl From<LayoutBox> for LayoutOutcome {
    fn from(lb: LayoutBox) -> Self {
        Self {
            layout_box: lb,
            break_token: None,
            propagated_break_before: None,
            propagated_break_after: None,
        }
    }
}

/// Token that records where layout was interrupted, allowing resumption.
#[derive(Clone, Debug)]
pub struct BreakToken {
    /// The entity whose layout was interrupted.
    pub entity: Entity,
    /// How much block-axis size was consumed before the break.
    pub consumed_block_size: f32,
    /// Nested break token from a child that was itself interrupted.
    pub child_break_token: Option<Box<BreakToken>>,
    /// Layout-mode-specific data for resumption.
    pub mode_data: Option<BreakTokenData>,
}

/// Layout-mode-specific data stored in a [`BreakToken`].
#[derive(Clone, Debug)]
pub enum BreakTokenData {
    /// Block layout: index of the next child to lay out.
    Block {
        child_index: usize,
        /// If the break occurred within an inline run at this `child_index`,
        /// the number of line boxes to skip when resuming.
        inline_break_line: Option<usize>,
    },
    /// Flex layout: line and item indices, with optional child break token
    /// for flex items that contain fragmentable content.
    Flex {
        line_index: usize,
        item_index: usize,
        /// If the item at `item_index` was itself fragmented (e.g., contains
        /// block children), this token resumes its child layout.
        child_break_token: Option<Box<BreakToken>>,
    },
    /// Grid layout: row index and spanning item break tokens.
    Grid {
        row_index: usize,
        /// Spanning items that were fragmented at this break row.
        /// Each entry is `(item_entity, child_break_token)` for resumption.
        child_break_tokens: Vec<(Entity, Box<BreakToken>)>,
    },
    /// Table layout: row index and optional header/footer entities for
    /// repetition in continuation fragments.
    Table {
        row_index: usize,
        /// Entity of the thead row group to re-layout at the top of each fragment.
        thead_entity: Option<Entity>,
        /// Entity of the tfoot row group to re-layout at the bottom of each fragment.
        tfoot_entity: Option<Entity>,
    },
}

// ---------------------------------------------------------------------------
// Intrinsic sizing (CSS Sizing Level 3)
// ---------------------------------------------------------------------------

/// Min-content and max-content intrinsic sizes for an element.
///
/// Used by flex (4.5 automatic minimum), grid (12.3-12.6 track sizing),
/// and shrink-to-fit width calculations (CSS 2.1 10.3.5).
#[derive(Clone, Copy, Debug, Default)]
pub struct IntrinsicSizes {
    /// The narrowest an element can be without overflow (longest unbreakable segment).
    pub min_content: f32,
    /// The widest an element would be given infinite available space (no line breaks).
    pub max_content: f32,
}

// ---------------------------------------------------------------------------
// Subgrid context (CSS Grid Level 2 2)
// ---------------------------------------------------------------------------

/// Parent grid track context for subgrid children (CSS Grid Level 2 2).
///
/// When a grid item uses `subgrid` on one or both axes, the parent grid's
/// resolved track sizes and line names are passed down via this context.
#[derive(Clone, Debug)]
pub struct SubgridContext {
    /// Resolved parent column track sizes (subgrid span only). `None` if not subgridded on columns.
    pub col_sizes: Option<Vec<f32>>,
    /// Resolved parent row track sizes (subgrid span only). `None` if not subgridded on rows.
    pub row_sizes: Option<Vec<f32>>,
    /// Parent column line names (for merging with subgrid's own names).
    pub col_line_names: Vec<Vec<String>>,
    /// Parent row line names (for merging with subgrid's own names).
    pub row_line_names: Vec<Vec<String>>,
    /// Parent column gap (CSS Grid L2 2.4: subgridded axis inherits parent gap).
    pub col_gap: Option<f32>,
    /// Parent row gap (CSS Grid L2 2.4: subgridded axis inherits parent gap).
    pub row_gap: Option<f32>,
}

// ---------------------------------------------------------------------------
// Layout input and dispatch
// ---------------------------------------------------------------------------

/// Contextual parameters for a single child layout invocation.
#[derive(Debug, Clone, Copy)]
pub struct LayoutInput<'a> {
    /// Containing block size (width always definite, height may be indefinite).
    pub containing: CssSize,
    /// Inline-axis size of the containing block (for margin/padding % resolution).
    ///
    /// CSS Box Model Level 3 5.3: margin/padding percentages refer to the
    /// containing block's **inline size**. In `horizontal-tb`, this equals
    /// `containing_width`. In vertical writing modes, this equals the
    /// physical height of the containing block.
    pub containing_inline_size: f32,
    /// Offset from the containing block origin.
    pub offset: Point,
    /// Font database for text measurement.
    pub font_db: &'a FontDatabase,
    /// Recursion depth guard.
    pub depth: u32,
    /// Float context from the nearest ancestor BFC.
    ///
    /// Non-BFC blocks forward this to children for float propagation
    /// (CSS 2.1 9.5). BFC-establishing elements create their own
    /// `FloatContext` and ignore this field.
    pub float_ctx: Option<&'a RefCell<FloatContext>>,
    /// Viewport dimensions for fixed positioning.
    ///
    /// Set at the root layout and propagated downward. Fixed-positioned
    /// elements use this as their containing block (CSS 2.1 10.1).
    pub viewport: Option<Size>,
    /// Fragmentation context (if inside a fragmentainer).
    pub fragmentainer: Option<&'a FragmentainerContext>,
    /// Break token from a previous fragment (for resumption).
    pub break_token: Option<&'a BreakToken>,
    /// Parent grid context for subgrid items (CSS Grid Level 2 2).
    pub subgrid: Option<&'a SubgridContext>,
}

impl<'a> LayoutInput<'a> {
    /// Create a probe `LayoutInput` for intrinsic sizing (min/max-content).
    ///
    /// Sets `offset` to zero, all optional context fields to `None`.
    /// Use struct update syntax to override individual fields if needed:
    /// ```ignore
    /// let input = LayoutInput { containing_height: Some(h), ..LayoutInput::probe(&env, w) };
    /// ```
    #[must_use]
    pub fn probe(env: &LayoutEnv<'a>, containing_width: f32) -> Self {
        Self {
            containing: CssSize::width_only(containing_width),
            containing_inline_size: containing_width,
            offset: Point::ZERO,
            font_db: env.font_db,
            depth: env.depth + 1,
            float_ctx: None,
            viewport: env.viewport,
            fragmentainer: None,
            break_token: None,
            subgrid: None,
        }
    }
}

/// Shared layout environment: immutable resources passed through layout call chains.
///
/// Groups `font_db`, `layout_child`, `depth`, and `viewport` to reduce
/// parameter count in layout functions.
#[derive(Clone, Copy)]
pub struct LayoutEnv<'a> {
    /// Font database for text measurement.
    pub font_db: &'a FontDatabase,
    /// Dispatch function for child layout by display type.
    pub layout_child: ChildLayoutFn,
    /// Recursion depth guard.
    pub depth: u32,
    /// Viewport dimensions for fixed positioning.
    pub viewport: Option<Size>,
}

impl<'a> LayoutEnv<'a> {
    /// Create from a [`LayoutInput`].
    #[must_use]
    pub fn from_input(input: &LayoutInput<'a>, layout_child: ChildLayoutFn) -> Self {
        Self {
            font_db: input.font_db,
            layout_child,
            depth: input.depth,
            viewport: input.viewport,
        }
    }

    /// Return a copy with `depth` incremented by 1.
    #[must_use]
    pub fn deeper(&self) -> Self {
        Self {
            depth: self.depth + 1,
            ..*self
        }
    }
}

/// Callback type for dispatching child layout by display type.
///
/// The orchestrator (`elidex-layout`) provides a dispatch function that routes
/// to block, flex, or grid layout based on the child's `display` value.
/// Within standalone block-only scenarios, [`layout_block_only`] can be used.
pub type ChildLayoutFn = fn(&mut EcsDom, Entity, &LayoutInput<'_>) -> LayoutOutcome;

/// Maximum recursion depth for layout tree walking.
///
/// Prevents stack overflow on deeply nested DOMs. Shared between
/// block, inline, and flex layout modules.
pub const MAX_LAYOUT_DEPTH: u32 = 1000;

/// Block-only layout dispatch (no flex/grid routing).
///
/// A [`ChildLayoutFn`] implementation that always uses block layout.
/// Used for standalone tests and scenarios where flex/grid dispatch is not needed.
pub fn layout_block_only(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
) -> LayoutOutcome {
    block::layout_block_inner(dom, entity, input, layout_block_only)
}

/// Parameters for [`empty_container_box`].
pub struct EmptyContainerParams<'a> {
    /// Content area origin.
    pub content_origin: Point,
    /// Content area width.
    pub content_width: f32,
    /// Containing block height (if known).
    pub containing_height: Option<f32>,
    /// Resolved padding.
    pub padding: EdgeSizes,
    /// Resolved border.
    pub border: EdgeSizes,
    /// Resolved margin.
    pub margin: EdgeSizes,
    /// Computed style of the container.
    pub style: &'a elidex_plugin::ComputedStyle,
}

/// Build a layout box for an empty or depth-limited container.
///
/// Used when a container has no children or maximum layout depth is reached.
#[must_use]
pub fn empty_container_box(
    dom: &mut EcsDom,
    entity: Entity,
    params: &EmptyContainerParams<'_>,
) -> LayoutBox {
    let lb = LayoutBox {
        content: elidex_plugin::Rect::new(
            params.content_origin.x,
            params.content_origin.y,
            params.content_width,
            resolve_explicit_height(params.style, params.containing_height).unwrap_or(0.0),
        ),
        padding: params.padding,
        border: params.border,
        margin: params.margin,
        first_baseline: None,
    };
    let _ = dom.world_mut().insert_one(entity, lb.clone());
    lb
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::{Direction, LayoutBox, WritingMode, WritingModeContext};

    #[test]
    fn compute_inline_containing_horizontal() {
        // horizontal-tb: inline size = physical width
        assert_eq!(
            compute_inline_containing(WritingMode::HorizontalTb, 800.0, Some(600.0)),
            800.0
        );
        assert_eq!(
            compute_inline_containing(WritingMode::HorizontalTb, 800.0, None),
            800.0
        );
    }

    #[test]
    fn compute_inline_containing_vertical() {
        // vertical-rl/lr: inline size = physical height
        assert_eq!(
            compute_inline_containing(WritingMode::VerticalRl, 800.0, Some(600.0)),
            600.0
        );
        assert_eq!(
            compute_inline_containing(WritingMode::VerticalLr, 800.0, Some(400.0)),
            400.0
        );
        // When height is None, falls back to width
        assert_eq!(
            compute_inline_containing(WritingMode::VerticalRl, 800.0, None),
            800.0
        );
    }

    #[test]
    fn inline_pb_horizontal() {
        let wm = WritingModeContext::new(WritingMode::HorizontalTb, Direction::Ltr);
        let padding = EdgeSizes::new(10.0, 20.0, 30.0, 40.0);
        let border = EdgeSizes::new(1.0, 2.0, 3.0, 4.0);
        // Horizontal: inline = left + right = (40+20) + (4+2) = 66
        assert_eq!(inline_pb(&wm, &padding, &border), 66.0);
        // Horizontal: block = top + bottom = (10+30) + (1+3) = 44
        assert_eq!(block_pb(&wm, &padding, &border), 44.0);
    }

    #[test]
    fn inline_pb_vertical() {
        let wm = WritingModeContext::new(WritingMode::VerticalRl, Direction::Ltr);
        let padding = EdgeSizes::new(10.0, 20.0, 30.0, 40.0);
        let border = EdgeSizes::new(1.0, 2.0, 3.0, 4.0);
        // Vertical: inline = top + bottom = (10+30) + (1+3) = 44
        assert_eq!(inline_pb(&wm, &padding, &border), 44.0);
        // Vertical: block = left + right = (40+20) + (4+2) = 66
        assert_eq!(block_pb(&wm, &padding, &border), 66.0);
    }

    #[test]
    fn intrinsic_sizes_default() {
        let sizes = IntrinsicSizes::default();
        assert_eq!(sizes.min_content, 0.0);
        assert_eq!(sizes.max_content, 0.0);
    }

    #[test]
    fn intrinsic_sizes_with_values() {
        let sizes = IntrinsicSizes {
            min_content: 50.0,
            max_content: 200.0,
        };
        assert_eq!(sizes.min_content, 50.0);
        assert_eq!(sizes.max_content, 200.0);
        // Verify Clone + Copy
        let copy = sizes;
        assert_eq!(copy.min_content, 50.0);
    }

    #[test]
    fn layout_outcome_from_layout_box() {
        let lb = LayoutBox::default();
        let outcome: LayoutOutcome = lb.clone().into();
        assert!(outcome.break_token.is_none());
        assert_eq!(outcome.layout_box.content.size.width, lb.content.size.width);
    }

    #[test]
    fn layout_outcome_no_break_default() {
        let outcome = LayoutOutcome::from(LayoutBox::default());
        assert!(outcome.break_token.is_none());
    }

    #[test]
    fn break_token_nested() {
        let inner = BreakToken {
            entity: Entity::DANGLING,
            consumed_block_size: 50.0,
            child_break_token: None,
            mode_data: Some(BreakTokenData::Block {
                child_index: 2,
                inline_break_line: None,
            }),
        };
        let outer = BreakToken {
            entity: Entity::DANGLING,
            consumed_block_size: 100.0,
            child_break_token: Some(Box::new(inner)),
            mode_data: Some(BreakTokenData::Flex {
                line_index: 0,
                item_index: 3,
                child_break_token: None,
            }),
        };
        let child = outer.child_break_token.as_ref().unwrap();
        assert_eq!(child.consumed_block_size, 50.0);
        assert!(matches!(
            child.mode_data,
            Some(BreakTokenData::Block { child_index: 2, .. })
        ));
    }

    #[test]
    fn break_token_data_variants() {
        let block = BreakTokenData::Block {
            child_index: 5,
            inline_break_line: None,
        };
        let flex = BreakTokenData::Flex {
            line_index: 1,
            item_index: 2,
            child_break_token: None,
        };
        let grid = BreakTokenData::Grid {
            row_index: 3,
            child_break_tokens: Vec::new(),
        };
        let table = BreakTokenData::Table {
            row_index: 0,
            thead_entity: None,
            tfoot_entity: None,
        };
        // Ensure all variants are constructible and cloneable.
        let _ = block.clone();
        let _ = flex.clone();
        let _ = grid.clone();
        let _ = table.clone();
    }
}
