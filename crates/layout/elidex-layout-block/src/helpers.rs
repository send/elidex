//! Shared layout helpers used across layout algorithm crates.

use elidex_ecs::{EcsDom, Entity, ImageData};
use elidex_plugin::{
    AlignItems, AlignSelf, BoxSizing, ComputedStyle, Dimension, Display, EdgeSizes, Size,
    WritingMode, WritingModeContext,
};

use super::MAX_LAYOUT_DEPTH;

/// Compute the inline-axis containing size from the element's writing mode.
///
/// Used when a layout function constructs `LayoutInput` for its children.
/// In `horizontal-tb`, inline size equals physical width. In vertical writing
/// modes, inline size equals physical height.
#[must_use]
pub fn compute_inline_containing(wm: WritingMode, width: f32, height: Option<f32>) -> f32 {
    if wm.is_horizontal() {
        width
    } else {
        height.unwrap_or(width)
    }
}

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

/// Resolve padding from a computed style against the containing block's inline size.
///
/// CSS Box Model L3 5.3: padding percentages (including block-axis sides) refer
/// to the **inline size** of the containing block. In `horizontal-tb` this equals
/// the physical width; in vertical writing modes it equals the physical height.
/// Callers must pass `containing_inline_size` (not necessarily physical width).
/// The result is clamped to non-negative.
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

/// Resolve all box model edges (padding, border, margin) for an element.
///
/// Returns `(padding, border, margin)` all resolved against `containing_inline_size`
/// per CSS Box Model L3 5.3.
#[must_use]
pub fn resolve_box_model(
    style: &ComputedStyle,
    containing_inline_size: f32,
) -> (EdgeSizes, EdgeSizes, EdgeSizes) {
    let padding = resolve_padding(style, containing_inline_size);
    let border = sanitize_border(style);
    let margin = EdgeSizes::new(
        crate::block::resolve_margin(style.margin_top, containing_inline_size),
        crate::block::resolve_margin(style.margin_right, containing_inline_size),
        crate::block::resolve_margin(style.margin_bottom, containing_inline_size),
        crate::block::resolve_margin(style.margin_left, containing_inline_size),
    );
    (padding, border, margin)
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

/// Sum of inline-axis (inline-start + inline-end) padding and border.
///
/// In `horizontal-tb` this equals `horizontal_pb`. In vertical writing modes
/// this equals `vertical_pb`.
#[must_use]
pub fn inline_pb(wm: &WritingModeContext, padding: &EdgeSizes, border: &EdgeSizes) -> f32 {
    if wm.is_horizontal() {
        horizontal_pb(padding, border)
    } else {
        vertical_pb(padding, border)
    }
}

/// Sum of block-axis (block-start + block-end) padding and border.
///
/// In `horizontal-tb` this equals `vertical_pb`. In vertical writing modes
/// this equals `horizontal_pb`.
#[must_use]
pub fn block_pb(wm: &WritingModeContext, padding: &EdgeSizes, border: &EdgeSizes) -> f32 {
    if wm.is_horizontal() {
        vertical_pb(padding, border)
    } else {
        horizontal_pb(padding, border)
    }
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
#[must_use]
pub fn effective_align(item_align: AlignSelf, container_align: AlignItems) -> AlignItems {
    match item_align {
        AlignSelf::Auto => container_align,
        AlignSelf::Stretch => AlignItems::Stretch,
        AlignSelf::FlexStart => AlignItems::FlexStart,
        AlignSelf::FlexEnd => AlignItems::FlexEnd,
        AlignSelf::Center => AlignItems::Center,
        AlignSelf::Baseline => AlignItems::Baseline,
    }
}

/// Save all descendant `ComputedStyle` components under `entity`.
///
/// Layout probes (e.g. min-content at `containing_width: 1.0`) can mutate
/// descendant styles via flex/grid `position_items`. This function captures
/// the styles so they can be restored after the probe.
#[must_use]
pub fn save_descendant_styles(dom: &EcsDom, entity: Entity) -> Vec<(Entity, ComputedStyle)> {
    let mut result = Vec::new();
    let mut stack = Vec::new();
    let mut child = dom.get_first_child(entity);
    while let Some(c) = child {
        stack.push(c);
        child = dom.get_next_sibling(c);
    }
    while let Some(e) = stack.pop() {
        if let Ok(style) = dom.world().get::<&ComputedStyle>(e) {
            result.push((e, (*style).clone()));
        }
        let mut c = dom.get_first_child(e);
        while let Some(ch) = c {
            stack.push(ch);
            c = dom.get_next_sibling(ch);
        }
    }
    result
}

/// Restore previously saved `ComputedStyle` components.
pub fn restore_descendant_styles(dom: &mut EcsDom, saved: &[(Entity, ComputedStyle)]) {
    for (entity, style) in saved {
        let _ = dom.world_mut().insert_one(*entity, style.clone());
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
/// CSS Display Level 3 2.8: `display: contents` elements do not generate
/// a box -- their children participate in the parent's formatting context
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

/// Detect intrinsic dimensions from `ImageData`, `FormControlState`, or `IframeData`.
///
/// Returns `Some(Size)` for replaced elements (images, form controls, iframes),
/// `None` otherwise.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn get_intrinsic_size(dom: &EcsDom, entity: Entity) -> Option<Size> {
    dom.world()
        .get::<&ImageData>(entity)
        .ok()
        .map(|img| Size::new(img.width as f32, img.height as f32))
        .or_else(|| {
            dom.world()
                .get::<&elidex_form::FormControlState>(entity)
                .ok()
                .map(|fcs| {
                    let s = elidex_form::form_intrinsic_size(&fcs);
                    Size::new(s.width.max(0.0), s.height.max(0.0))
                })
        })
        .or_else(|| {
            dom.world()
                .get::<&elidex_ecs::IframeData>(entity)
                .ok()
                .map(|iframe| Size::new(iframe.width as f32, iframe.height as f32))
        })
}
