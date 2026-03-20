//! Block formatting context layout and shared layout helpers.
//!
//! Provides the block layout algorithm, inline formatting context,
//! and shared utilities (sanitize, box model helpers, etc.) used by
//! all layout algorithm crates.

pub mod block;
pub mod inline;
pub mod paint_order;
pub mod positioned;

use std::cell::RefCell;

use elidex_ecs::{EcsDom, Entity, ImageData};
use elidex_plugin::{
    AlignItems, AlignSelf, BoxSizing, ComputedStyle, Dimension, Display, EdgeSizes, LayoutBox,
};
use elidex_text::FontDatabase;

use crate::block::float::FloatContext;

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
}

impl From<LayoutBox> for LayoutOutcome {
    fn from(lb: LayoutBox) -> Self {
        Self {
            layout_box: lb,
            break_token: None,
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
    Block { child_index: usize },
    /// Flex layout: line and item indices.
    Flex {
        line_index: usize,
        item_index: usize,
    },
    /// Grid layout: row index.
    Grid { row_index: usize },
    /// Table layout: row index and optional header/footer tokens.
    Table {
        row_index: usize,
        thead: Option<Box<BreakToken>>,
        tfoot: Option<Box<BreakToken>>,
    },
}

// ---------------------------------------------------------------------------
// Intrinsic sizing (CSS Sizing Level 3)
// ---------------------------------------------------------------------------

/// Min-content and max-content intrinsic sizes for an element.
///
/// Used by flex (§4.5 automatic minimum), grid (§12.3-12.6 track sizing),
/// and shrink-to-fit width calculations (CSS 2.1 §10.3.5).
#[derive(Clone, Copy, Debug, Default)]
pub struct IntrinsicSizes {
    /// The narrowest an element can be without overflow (longest unbreakable segment).
    pub min_content: f32,
    /// The widest an element would be given infinite available space (no line breaks).
    pub max_content: f32,
}

// ---------------------------------------------------------------------------
// Layout input and dispatch
// ---------------------------------------------------------------------------

/// Contextual parameters for a single child layout invocation.
#[derive(Debug, Clone, Copy)]
pub struct LayoutInput<'a> {
    /// Width of the containing block.
    pub containing_width: f32,
    /// Height of the containing block (if known).
    pub containing_height: Option<f32>,
    /// Horizontal offset from the containing block origin.
    pub offset_x: f32,
    /// Vertical offset from the containing block origin.
    pub offset_y: f32,
    /// Font database for text measurement.
    pub font_db: &'a FontDatabase,
    /// Recursion depth guard.
    pub depth: u32,
    /// Float context from the nearest ancestor BFC.
    ///
    /// Non-BFC blocks forward this to children for float propagation
    /// (CSS 2.1 §9.5). BFC-establishing elements create their own
    /// `FloatContext` and ignore this field.
    pub float_ctx: Option<&'a RefCell<FloatContext>>,
    /// Viewport dimensions for fixed positioning.
    ///
    /// Set at the root layout and propagated downward. Fixed-positioned
    /// elements use this as their containing block (CSS 2.1 §10.1).
    pub viewport: Option<(f32, f32)>,
    /// Fragmentation context (if inside a fragmentainer).
    pub fragmentainer: Option<&'a FragmentainerContext>,
    /// Break token from a previous fragment (for resumption).
    pub break_token: Option<&'a BreakToken>,
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
    block::layout_block_inner(dom, entity, input, layout_block_only).into()
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Replace non-finite f32 values (NaN, infinity) with 0.0.
#[must_use]
pub fn sanitize(v: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

/// Clamp a single value to non-negative: negative, NaN, and infinity become `0.0`.
#[must_use]
pub fn sanitize_non_negative(v: f32) -> f32 {
    if v.is_finite() && v > 0.0 {
        v
    } else {
        0.0
    }
}

/// Clamp edge values to non-negative: negative values become `0.0`,
/// zero and positive values are preserved as-is. NaN and infinity also become `0.0`.
#[must_use]
pub fn sanitize_edge_values(top: f32, right: f32, bottom: f32, left: f32) -> EdgeSizes {
    EdgeSizes {
        top: sanitize_non_negative(top),
        right: sanitize_non_negative(right),
        bottom: sanitize_non_negative(bottom),
        left: sanitize_non_negative(left),
    }
}

/// Resolve padding from a computed style against the containing block width.
///
/// CSS 2.1 §8.4: padding percentages (including top/bottom) refer to the
/// **width** of the containing block. The result is clamped to non-negative.
#[must_use]
pub fn resolve_padding(style: &ComputedStyle, containing_width: f32) -> EdgeSizes {
    EdgeSizes {
        top: sanitize_non_negative(resolve_dimension_value(
            style.padding.top,
            containing_width,
            0.0,
        )),
        right: sanitize_non_negative(resolve_dimension_value(
            style.padding.right,
            containing_width,
            0.0,
        )),
        bottom: sanitize_non_negative(resolve_dimension_value(
            style.padding.bottom,
            containing_width,
            0.0,
        )),
        left: sanitize_non_negative(resolve_dimension_value(
            style.padding.left,
            containing_width,
            0.0,
        )),
    }
}

/// Sanitize padding from a computed style (non-negative, finite).
///
/// Backward-compatible helper that resolves percentages against 0
/// (i.e. treats percentages as 0). Prefer [`resolve_padding`] when
/// the containing block width is available.
#[must_use]
pub fn sanitize_padding(style: &ComputedStyle) -> EdgeSizes {
    resolve_padding(style, 0.0)
}

/// Sanitize border widths from a computed style (non-negative, finite).
#[must_use]
pub fn sanitize_border(style: &ComputedStyle) -> EdgeSizes {
    sanitize_edge_values(
        style.border_top.width,
        style.border_right.width,
        style.border_bottom.width,
        style.border_left.width,
    )
}

/// Sum of horizontal (left + right) padding and border.
#[must_use]
pub fn horizontal_pb(padding: &EdgeSizes, border: &EdgeSizes) -> f32 {
    padding.left + padding.right + border.left + border.right
}

/// Sum of vertical (top + bottom) padding and border.
#[must_use]
pub fn vertical_pb(padding: &EdgeSizes, border: &EdgeSizes) -> f32 {
    padding.top + padding.bottom + border.top + border.bottom
}

/// Resolve a CSS dimension to a pixel value.
/// - Length: use directly
/// - Percentage: relative to `containing`
/// - Auto: returns `auto_value`
#[must_use]
pub fn resolve_dimension_value(dim: Dimension, containing: f32, auto_value: f32) -> f32 {
    match dim {
        Dimension::Length(px) => px,
        Dimension::Percentage(pct) => containing * pct / 100.0,
        Dimension::Auto => auto_value,
    }
}

/// Resolve a `Dimension` to a pixel value for min/max constraints.
///
/// `Auto` returns `default_value` (0.0 for min-*, infinity for max-*).
/// Percentages against indefinite or non-positive containing sizes return
/// `default_value`. Negative results are clamped to 0.
#[must_use]
pub fn resolve_min_max(dim: Dimension, containing: f32, default_value: f32) -> f32 {
    match dim {
        Dimension::Length(px) if px.is_finite() => px.max(0.0),
        Dimension::Percentage(pct) => {
            // Guard against indefinite containing sizes (flex) and zero/negative.
            if containing > 0.0 && containing < f32::MAX / 2.0 {
                sanitize(containing * pct / 100.0).max(0.0)
            } else {
                default_value
            }
        }
        _ => default_value,
    }
}

/// Clamp `value` between `min` and `max`, with `min` winning on conflict.
///
/// Equivalent to `value.max(min).min(max).max(min)`.
#[must_use]
pub fn clamp_min_max(value: f32, min: f32, max: f32) -> f32 {
    value.max(min).min(max).max(min)
}

/// Compute total gap size for `count` items with `gap` between each pair.
///
/// Returns `gap * (count - 1)` when `count > 1`, otherwise `0.0`.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn total_gap(count: usize, gap: f32) -> f32 {
    if count > 1 {
        gap * (count - 1) as f32
    } else {
        0.0
    }
}

/// Resolve the content-box width for a container (flex, grid, table).
///
/// Handles `auto` (filling available space), percentage, and length values,
/// with `box-sizing: border-box` adjustment.
#[must_use]
pub fn resolve_content_width(
    style: &ComputedStyle,
    containing_width: f32,
    h_pb: f32,
    h_margin: f32,
) -> f32 {
    let auto_value = (containing_width - h_margin - h_pb).max(0.0);
    let mut w = sanitize(resolve_dimension_value(
        style.width,
        containing_width,
        auto_value,
    ));
    if style.box_sizing == BoxSizing::BorderBox {
        if let Dimension::Length(_) | Dimension::Percentage(_) = style.width {
            w = (w - h_pb).max(0.0);
        }
    }
    w
}

/// Adjust min/max constraint values for `box-sizing: border-box`.
///
/// Subtracts `pb` (padding + border sum on the relevant axis) from both
/// `min` and `max`, clamping to 0. `max` is only adjusted when finite
/// (infinity means no constraint).
pub fn adjust_min_max_for_border_box(min: &mut f32, max: &mut f32, pb: f32) {
    *min = (*min - pb).max(0.0);
    if *max < f32::INFINITY {
        *max = (*max - pb).max(0.0);
    }
}

/// Resolve the effective cross-axis alignment for an item.
///
/// `AlignSelf::Auto` inherits from the container's `align-items`.
/// `Baseline` is treated as `FlexStart` (baseline alignment not yet implemented).
#[must_use]
pub fn effective_align(item_align: AlignSelf, container_align: AlignItems) -> AlignItems {
    let resolved = match item_align {
        AlignSelf::Auto => container_align,
        AlignSelf::Stretch => AlignItems::Stretch,
        AlignSelf::FlexStart => AlignItems::FlexStart,
        AlignSelf::FlexEnd => AlignItems::FlexEnd,
        AlignSelf::Center => AlignItems::Center,
        AlignSelf::Baseline => AlignItems::Baseline,
    };
    // Baseline not yet implemented — treat as flex-start.
    if resolved == AlignItems::Baseline {
        AlignItems::FlexStart
    } else {
        resolved
    }
}

/// Resolve an explicit height (Length or Percentage) to content-box pixels.
///
/// Returns `None` for `auto`. For `border-box`, subtracts vertical padding + border.
/// Used by both block and flex layout for height resolution.
#[must_use]
pub fn resolve_explicit_height(
    style: &ComputedStyle,
    containing_height: Option<f32>,
) -> Option<f32> {
    let bb_pb = || {
        let p = sanitize_padding(style);
        let b = sanitize_border(style);
        vertical_pb(&p, &b)
    };
    match style.height {
        Dimension::Length(px) if px.is_finite() => {
            if style.box_sizing == BoxSizing::BorderBox {
                Some((px - bb_pb()).max(0.0))
            } else {
                Some(px)
            }
        }
        Dimension::Percentage(pct) => containing_height.map(|ch| {
            let resolved = ch * pct / 100.0;
            if style.box_sizing == BoxSizing::BorderBox {
                (resolved - bb_pb()).max(0.0)
            } else {
                resolved
            }
        }),
        _ => None,
    }
}

/// Parameters for [`empty_container_box`].
pub struct EmptyContainerParams<'a> {
    /// Computed style of the container.
    pub style: &'a ComputedStyle,
    /// Content area X coordinate.
    pub content_x: f32,
    /// Content area Y coordinate.
    pub content_y: f32,
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
            params.content_x,
            params.content_y,
            params.content_width,
            resolve_explicit_height(params.style, params.containing_height).unwrap_or(0.0),
        ),
        padding: params.padding,
        border: params.border,
        margin: params.margin,
    };
    let _ = dom.world_mut().insert_one(entity, lb.clone());
    lb
}

/// Get composed children with `display: contents` flattened.
///
/// Combines `EcsDom::composed_children()` and `flatten_contents()` into a single call.
#[must_use]
pub fn composed_children_flat(dom: &EcsDom, entity: Entity) -> Vec<Entity> {
    let raw = dom.composed_children(entity);
    flatten_contents(dom, &raw)
}

/// Flatten `display: contents` entities in a child list.
///
/// CSS Display Level 3 §2.8: `display: contents` elements do not generate
/// a box — their children participate in the parent's formatting context
/// as if the element did not exist.
///
/// This function replaces each `display: contents` child with its own
/// `composed_children`, recursively expanding nested `display: contents`.
/// Recursion is capped at `MAX_LAYOUT_DEPTH` to prevent stack overflow.
#[must_use]
pub fn flatten_contents(dom: &EcsDom, children: &[Entity]) -> Vec<Entity> {
    flatten_contents_impl(dom, children, 0)
}

fn flatten_contents_impl(dom: &EcsDom, children: &[Entity], depth: u32) -> Vec<Entity> {
    let mut result = Vec::with_capacity(children.len());
    if depth >= MAX_LAYOUT_DEPTH {
        return result;
    }
    for &child in children {
        if try_get_style(dom, child).is_some_and(|s| s.display == Display::Contents) {
            let grandchildren = dom.composed_children(child);
            result.extend(flatten_contents_impl(dom, &grandchildren, depth + 1));
        } else {
            result.push(child);
        }
    }
    result
}

/// Get the computed style for an entity, or a default if none is attached.
#[must_use]
pub fn get_style(dom: &EcsDom, entity: Entity) -> ComputedStyle {
    try_get_style(dom, entity).unwrap_or_default()
}

/// Try to get the computed style for an entity. Returns `None` for text nodes
/// or entities without a style component.
#[must_use]
pub fn try_get_style(dom: &EcsDom, entity: Entity) -> Option<ComputedStyle> {
    dom.world()
        .get::<&ComputedStyle>(entity)
        .ok()
        .map(|s| (*s).clone())
}

/// Detect intrinsic dimensions from `ImageData` or `FormControlState`.
///
/// Returns `Some((width, height))` for replaced elements (images, form controls),
/// `None` otherwise.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn get_intrinsic_size(dom: &EcsDom, entity: Entity) -> Option<(f32, f32)> {
    dom.world()
        .get::<&ImageData>(entity)
        .ok()
        .map(|img| (img.width as f32, img.height as f32))
        .or_else(|| {
            dom.world()
                .get::<&elidex_form::FormControlState>(entity)
                .ok()
                .map(|fcs| {
                    let (w, h) = elidex_form::form_intrinsic_size(&fcs);
                    (w.max(0.0), h.max(0.0))
                })
        })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::LayoutBox;

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
        assert_eq!(outcome.layout_box.content.width, lb.content.width);
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
            mode_data: Some(BreakTokenData::Block { child_index: 2 }),
        };
        let outer = BreakToken {
            entity: Entity::DANGLING,
            consumed_block_size: 100.0,
            child_break_token: Some(Box::new(inner)),
            mode_data: Some(BreakTokenData::Flex {
                line_index: 0,
                item_index: 3,
            }),
        };
        let child = outer.child_break_token.as_ref().unwrap();
        assert_eq!(child.consumed_block_size, 50.0);
        assert!(matches!(
            child.mode_data,
            Some(BreakTokenData::Block { child_index: 2 })
        ));
    }

    #[test]
    fn break_token_data_variants() {
        let block = BreakTokenData::Block { child_index: 5 };
        let flex = BreakTokenData::Flex {
            line_index: 1,
            item_index: 2,
        };
        let grid = BreakTokenData::Grid { row_index: 3 };
        let table = BreakTokenData::Table {
            row_index: 0,
            thead: None,
            tfoot: None,
        };
        // Ensure all variants are constructible and cloneable.
        let _ = block.clone();
        let _ = flex.clone();
        let _ = grid.clone();
        let _ = table.clone();
    }
}
