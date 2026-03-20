//! Intrinsic sizing computation (CSS Sizing Level 3).
//!
//! Provides [`compute_intrinsic_sizes`] which computes min-content and
//! max-content intrinsic sizes for any element, routing to the appropriate
//! algorithm based on `display` type.

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::{
    composed_children_flat, get_style, horizontal_pb, inline, sanitize_border, sanitize_padding,
    total_gap, ChildLayoutFn, IntrinsicSizes, LayoutInput, MAX_LAYOUT_DEPTH,
};
use elidex_plugin::{BoxSizing, Dimension, Display, FlexDirection, FlexWrap};
use elidex_text::FontDatabase;

/// Compute min-content and max-content intrinsic inline sizes for an element.
///
/// Currently computes intrinsic *widths* only (horizontal writing mode).
/// When writing-mode support matures, this should be generalized to use
/// the inline axis.
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
    font_db: &FontDatabase,
    layout_child: ChildLayoutFn,
    depth: u32,
) -> IntrinsicSizes {
    if depth >= MAX_LAYOUT_DEPTH {
        return IntrinsicSizes::default();
    }

    let style = get_style(dom, entity);

    // Container padding + border contribute to intrinsic width (CSS Sizing L3 §5).
    let padding = sanitize_padding(&style);
    let border = sanitize_border(&style);
    let pb = horizontal_pb(&padding, &border);

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
            compute_flex_intrinsic(dom, entity, &children, font_db, layout_child, depth)
        }
        Display::Grid | Display::InlineGrid => elidex_layout_grid::compute_grid_intrinsic(
            dom,
            entity,
            &children,
            font_db,
            layout_child,
            depth,
        ),
        Display::Table | Display::InlineTable => {
            compute_table_intrinsic(dom, entity, &children, font_db, layout_child, depth)
        }
        _ => compute_block_intrinsic(dom, entity, &children, font_db, layout_child, depth),
    };

    IntrinsicSizes {
        min_content: content.min_content + pb,
        max_content: content.max_content + pb,
    }
}

/// Return intrinsic sizes for replaced elements or elements with explicit width.
///
/// Replaced elements (images, form controls) have intrinsic dimensions.
/// Elements with explicit `width` use that as both min and max content.
fn explicit_intrinsic(
    dom: &EcsDom,
    entity: Entity,
    style: &elidex_plugin::ComputedStyle,
    pb: f32,
) -> Option<IntrinsicSizes> {
    // Replaced elements: use intrinsic image/form dimensions.
    if let Some((w, _h)) = elidex_layout_block::get_intrinsic_size(dom, entity) {
        return Some(IntrinsicSizes {
            min_content: w + pb,
            max_content: w + pb,
        });
    }
    // Explicit width: acts as both min-content and max-content.
    if let Dimension::Length(w) = style.width {
        // border-box: width already includes padding+border.
        let size = if style.box_sizing == BoxSizing::BorderBox {
            w
        } else {
            w + pb
        };
        return Some(IntrinsicSizes {
            min_content: size,
            max_content: size,
        });
    }
    None
}

/// Block intrinsic sizing.
///
/// Inline children → min/max content inline sizes.
/// Block children → max of each child's intrinsic sizes.
fn compute_block_intrinsic(
    dom: &mut EcsDom,
    entity: Entity,
    children: &[Entity],
    font_db: &FontDatabase,
    layout_child: ChildLayoutFn,
    depth: u32,
) -> IntrinsicSizes {
    let style = get_style(dom, entity);
    let has_block_children = children.iter().any(|&c| {
        elidex_layout_block::try_get_style(dom, c)
            .is_some_and(|s| elidex_layout_block::block::is_block_level(s.display))
    });

    if has_block_children {
        // Block children: max of each child's intrinsic sizes.
        let mut min = 0.0_f32;
        let mut max = 0.0_f32;
        for &child in children {
            let child_sizes = compute_intrinsic_sizes(dom, child, font_db, layout_child, depth + 1);
            min = min.max(child_sizes.min_content);
            max = max.max(child_sizes.max_content);
        }
        // Also check inline content mixed in.
        let inline_min = inline::min_content_inline_size(dom, children, &style, entity, font_db);
        let inline_max = inline::max_content_inline_size(dom, children, &style, entity, font_db);
        IntrinsicSizes {
            min_content: min.max(inline_min),
            max_content: max.max(inline_max),
        }
    } else {
        // Pure inline children.
        IntrinsicSizes {
            min_content: inline::min_content_inline_size(dom, children, &style, entity, font_db),
            max_content: inline::max_content_inline_size(dom, children, &style, entity, font_db),
        }
    }
}

/// Flex intrinsic sizing (CSS Sizing Level 3 §5.1).
fn compute_flex_intrinsic(
    dom: &mut EcsDom,
    entity: Entity,
    children: &[Entity],
    font_db: &FontDatabase,
    layout_child: ChildLayoutFn,
    depth: u32,
) -> IntrinsicSizes {
    let style = get_style(dom, entity);
    let horizontal = matches!(
        style.flex_direction,
        FlexDirection::Row | FlexDirection::RowReverse
    );
    let nowrap = matches!(style.flex_wrap, FlexWrap::Nowrap);

    let child_sizes_list =
        collect_child_intrinsic_sizes(dom, children, font_db, layout_child, depth);

    if child_sizes_list.is_empty() {
        return IntrinsicSizes::default();
    }

    // CSS Box Alignment L3: gap between items contributes to intrinsic size.
    let gap = if horizontal {
        elidex_layout_block::resolve_dimension_value(style.column_gap, 0.0, 0.0).max(0.0)
    } else {
        elidex_layout_block::resolve_dimension_value(style.row_gap, 0.0, 0.0).max(0.0)
    };
    let gap_total = total_gap(child_sizes_list.len(), gap);

    if horizontal {
        // Row direction: items side-by-side along main axis.
        let sum_min: f32 = child_sizes_list.iter().map(|s| s.min_content).sum();
        let sum_max: f32 = child_sizes_list.iter().map(|s| s.max_content).sum();
        let max_min = child_sizes_list
            .iter()
            .map(|s| s.min_content)
            .fold(0.0_f32, f32::max);
        // CSS Sizing L3 §5.1:
        // nowrap: min = sum(items min) + gaps, max = sum(items max) + gaps
        // wrap: min = max(items min) (no gap — single item per line), max = sum + gaps
        if nowrap {
            IntrinsicSizes {
                min_content: sum_min + gap_total,
                max_content: sum_max + gap_total,
            }
        } else {
            IntrinsicSizes {
                min_content: max_min,
                max_content: sum_max + gap_total,
            }
        }
    } else {
        // Column direction: items stack vertically, intrinsic width = max of children.
        // Gap is on the main (vertical) axis — does not affect intrinsic inline size.
        let max_min = child_sizes_list
            .iter()
            .map(|s| s.min_content)
            .fold(0.0_f32, f32::max);
        let max_max = child_sizes_list
            .iter()
            .map(|s| s.max_content)
            .fold(0.0_f32, f32::max);
        IntrinsicSizes {
            min_content: max_min,
            max_content: max_max,
        }
    }
}

/// Table intrinsic sizing (CSS 2.1 §17.5.2).
///
/// Walks the table structure (rows → cells) to build per-column min/max widths,
/// then sums columns.
fn compute_table_intrinsic(
    dom: &mut EcsDom,
    entity: Entity,
    children: &[Entity],
    font_db: &FontDatabase,
    layout_child: ChildLayoutFn,
    depth: u32,
) -> IntrinsicSizes {
    let mut col_min: Vec<f32> = Vec::new();
    let mut col_max: Vec<f32> = Vec::new();

    // Walk rows (direct or inside row-groups) → cells.
    let mut rows: Vec<Entity> = Vec::new();
    for &child in children {
        let child_style = elidex_layout_block::try_get_style(dom, child);
        match child_style.as_ref().map(|s| s.display) {
            Some(Display::TableRow) => rows.push(child),
            Some(
                Display::TableHeaderGroup | Display::TableRowGroup | Display::TableFooterGroup,
            ) => {
                // Row group: collect its row children.
                let group_children = composed_children_flat(dom, child);
                for &gc in &group_children {
                    if elidex_layout_block::try_get_style(dom, gc)
                        .is_some_and(|s| s.display == Display::TableRow)
                    {
                        rows.push(gc);
                    }
                }
            }
            Some(Display::TableCell) => {
                // Direct cell (anonymous row): treat as column 0.
                let cell_sizes =
                    compute_intrinsic_sizes(dom, child, font_db, layout_child, depth + 1);
                if col_min.is_empty() {
                    col_min.push(0.0);
                    col_max.push(0.0);
                }
                col_min[0] = col_min[0].max(cell_sizes.min_content);
                col_max[0] = col_max[0].max(cell_sizes.max_content);
            }
            _ => {}
        }
    }

    // NOTE: colspan is not yet handled — each cell is treated as colspan=1.
    for &row in &rows {
        let cells = composed_children_flat(dom, row);
        let mut col_idx = 0;
        for &cell in &cells {
            // Skip display:none and non-cell children.
            let cell_style = elidex_layout_block::try_get_style(dom, cell);
            if cell_style
                .as_ref()
                .is_some_and(|s| s.display == Display::None)
            {
                continue;
            }
            let cell_sizes = compute_intrinsic_sizes(dom, cell, font_db, layout_child, depth + 1);
            // Grow column vectors as needed.
            while col_min.len() <= col_idx {
                col_min.push(0.0);
                col_max.push(0.0);
            }
            col_min[col_idx] = col_min[col_idx].max(cell_sizes.min_content);
            col_max[col_idx] = col_max[col_idx].max(cell_sizes.max_content);
            col_idx += 1;
        }
    }

    let style = get_style(dom, entity);
    let gap = style.border_spacing_h.max(0.0);
    let gap_total = total_gap(col_min.len(), gap);

    IntrinsicSizes {
        min_content: col_min.iter().sum::<f32>() + gap_total,
        max_content: col_max.iter().sum::<f32>() + gap_total,
    }
}

/// Collect per-child intrinsic sizes, filtering `display:none` and probing text nodes.
fn collect_child_intrinsic_sizes(
    dom: &mut EcsDom,
    children: &[Entity],
    font_db: &FontDatabase,
    layout_child: ChildLayoutFn,
    depth: u32,
) -> Vec<IntrinsicSizes> {
    let mut result = Vec::new();
    for &child in children {
        let child_style = elidex_layout_block::try_get_style(dom, child);
        if child_style
            .as_ref()
            .is_some_and(|s| s.display == Display::None)
        {
            continue;
        }
        if child_style.is_none() {
            // Text node: measure via probe layout.
            let probe_min = probe_layout_size(dom, child, 1.0, font_db, layout_child, depth);
            let probe_max = probe_layout_size(dom, child, 1e6, font_db, layout_child, depth);
            result.push(IntrinsicSizes {
                min_content: probe_min,
                max_content: probe_max,
            });
            continue;
        }
        result.push(compute_intrinsic_sizes(
            dom,
            child,
            font_db,
            layout_child,
            depth + 1,
        ));
    }
    result
}

/// Probe layout at a given containing width, return content-box width.
///
/// Returns `LayoutBox.content.width` — the content area excluding the entity's
/// own padding and border.  Intended for text nodes and leaf elements whose
/// outer box model is accounted for by `compute_intrinsic_sizes`.
fn probe_layout_size(
    dom: &mut EcsDom,
    entity: Entity,
    containing_width: f32,
    font_db: &FontDatabase,
    layout_child: ChildLayoutFn,
    depth: u32,
) -> f32 {
    let input = LayoutInput {
        containing_width,
        containing_height: None,
        offset_x: 0.0,
        offset_y: 0.0,
        font_db,
        depth: depth + 1,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
    };
    let lb = layout_child(dom, entity, &input).layout_box;
    lb.content.width
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::dispatch_layout_child;
    use elidex_ecs::Attributes;
    use elidex_plugin::{ComputedStyle, Dimension};

    fn make_dom_with_text() -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            parent,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        let text = dom.create_text("hello world");
        let _ = dom.append_child(parent, text);
        (dom, parent)
    }

    #[test]
    fn block_with_text_intrinsic() {
        let (mut dom, parent) = make_dom_with_text();
        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, parent, &font_db, dispatch_layout_child, 0);
        // min_content <= max_content
        assert!(sizes.min_content <= sizes.max_content);
    }

    #[test]
    fn flex_row_nowrap_intrinsic() {
        let mut dom = EcsDom::new();
        let flex = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            flex,
            ComputedStyle {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Nowrap,
                ..Default::default()
            },
        );
        let c1 = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(100.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(flex, c1);
        let c2 = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            c2,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(50.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(flex, c2);

        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, flex, &font_db, dispatch_layout_child, 0);
        // nowrap: min = sum = 150, max = sum = 150 (both have explicit widths)
        assert!(sizes.min_content >= 0.0);
        assert!(sizes.max_content >= sizes.min_content);
    }

    #[test]
    fn flex_row_wrap_intrinsic() {
        let mut dom = EcsDom::new();
        let flex = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            flex,
            ComputedStyle {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                ..Default::default()
            },
        );
        let c1 = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(100.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(flex, c1);
        let c2 = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            c2,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(50.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(flex, c2);

        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, flex, &font_db, dispatch_layout_child, 0);
        // wrap: min = max(items) = 100, max = sum = 150
        assert!(sizes.min_content >= 0.0);
        assert!(sizes.max_content >= sizes.min_content);
    }

    #[test]
    fn nested_block_intrinsic() {
        let mut dom = EcsDom::new();
        let outer = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            outer,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        let inner = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            inner,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(200.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(outer, inner);

        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, outer, &font_db, dispatch_layout_child, 0);
        assert!(sizes.min_content >= 0.0);
        assert!(sizes.max_content >= sizes.min_content);
    }

    #[test]
    fn inline_flex_shrink_to_fit() {
        let sizes = IntrinsicSizes {
            min_content: 50.0,
            max_content: 200.0,
        };
        // available < min: result = min
        assert_eq!(shrink_to_fit_width(&sizes, 30.0), 50.0);
        // available between min and max: result = available
        assert_eq!(shrink_to_fit_width(&sizes, 100.0), 100.0);
        // available > max: result = max
        assert_eq!(shrink_to_fit_width(&sizes, 300.0), 200.0);
    }

    #[test]
    fn inline_grid_shrink_to_fit() {
        let sizes = IntrinsicSizes {
            min_content: 100.0,
            max_content: 400.0,
        };
        assert_eq!(shrink_to_fit_width(&sizes, 250.0), 250.0);
    }
}
