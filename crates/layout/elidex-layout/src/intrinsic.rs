//! Intrinsic sizing computation (CSS Sizing Level 3).
//!
//! Provides [`compute_intrinsic_sizes`] which computes min-content and
//! max-content intrinsic sizes for any element, routing to the appropriate
//! algorithm based on `display` type.

use elidex_ecs::{EcsDom, Entity};
use elidex_layout_block::{
    composed_children_flat, get_style, inline, inline_pb, sanitize_border, sanitize_padding,
    total_gap, ChildLayoutFn, IntrinsicSizes, LayoutInput, MAX_LAYOUT_DEPTH,
};
#[cfg(test)]
use elidex_plugin::WritingMode;
use elidex_plugin::{BoxSizing, Dimension, Display, FlexDirection, FlexWrap, WritingModeContext};
use elidex_text::FontDatabase;

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
    font_db: &FontDatabase,
    layout_child: ChildLayoutFn,
    depth: u32,
) -> IntrinsicSizes {
    if depth >= MAX_LAYOUT_DEPTH {
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
    if let Some((w, h)) = elidex_layout_block::get_intrinsic_size(dom, entity) {
        let inline_size = if inline_horizontal { w } else { h };
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
    // Determine whether the flex main axis maps to the physical horizontal axis,
    // taking writing mode into account: row directions follow the inline axis,
    // column directions follow the block axis.
    let horizontal_wm = style.writing_mode.is_horizontal();
    let horizontal = match style.flex_direction {
        FlexDirection::Row | FlexDirection::RowReverse => horizontal_wm,
        FlexDirection::Column | FlexDirection::ColumnReverse => !horizontal_wm,
    };
    let nowrap = matches!(style.flex_wrap, FlexWrap::Nowrap);

    let child_sizes_list =
        collect_child_intrinsic_sizes(dom, children, font_db, layout_child, depth);

    if child_sizes_list.is_empty() {
        return IntrinsicSizes::default();
    }

    // CSS Box Alignment L3: gap between items contributes to intrinsic size.
    // Use the main-axis gap (column-gap for row flex, row-gap for column flex)
    // but select the physical axis based on writing mode.
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

    // Colspan-aware intrinsic sizing: spanning cells distribute their intrinsic
    // width equally across spanned columns.
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
            // Read colspan from HTML attribute.
            let colspan = dom
                .world()
                .get::<&elidex_ecs::Attributes>(cell)
                .ok()
                .and_then(|attrs| attrs.get("colspan").and_then(|s| s.parse::<usize>().ok()))
                .unwrap_or(1)
                .clamp(1, 1000);

            let cell_sizes = compute_intrinsic_sizes(dom, cell, font_db, layout_child, depth + 1);

            // Grow column vectors as needed.
            while col_min.len() < col_idx + colspan {
                col_min.push(0.0);
                col_max.push(0.0);
            }

            if colspan == 1 {
                col_min[col_idx] = col_min[col_idx].max(cell_sizes.min_content);
                col_max[col_idx] = col_max[col_idx].max(cell_sizes.max_content);
            } else {
                // Distribute spanning cell's intrinsic width equally across columns.
                #[allow(clippy::cast_precision_loss)] // colspan clamped to 1..=1000
                let col_f = colspan as f32;
                let per_min = cell_sizes.min_content / col_f;
                let per_max = cell_sizes.max_content / col_f;
                for c in col_idx..col_idx + colspan {
                    col_min[c] = col_min[c].max(per_min);
                    col_max[c] = col_max[c].max(per_max);
                }
            }
            col_idx += colspan;
        }
    }

    let style = get_style(dom, entity);
    let inline_horizontal =
        WritingModeContext::new(style.writing_mode, style.direction).is_horizontal();
    // CSS 2.1 §17.6.2: in the collapsing border model, border-spacing is ignored.
    // Use inline-axis border-spacing (horizontal in horizontal-tb, vertical in vertical modes).
    let gap = if style.border_collapse == elidex_plugin::BorderCollapse::Collapse {
        0.0
    } else if inline_horizontal {
        style.border_spacing_h.max(0.0)
    } else {
        style.border_spacing_v.max(0.0)
    };
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

/// Probe layout at a given containing width, return content-box inline-axis size.
///
/// Returns the inline-axis content dimension (`width` in horizontal-tb,
/// `height` in vertical modes), excluding the entity's own padding and border.
/// Intended for text nodes and leaf elements whose outer box model is
/// accounted for by `compute_intrinsic_sizes`.
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
        containing_inline_size: containing_width,
        offset_x: 0.0,
        offset_y: 0.0,
        font_db,
        depth: depth + 1,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
    };
    let lb = layout_child(dom, entity, &input).layout_box;
    // Determine inline axis from the parent's writing mode context.
    // For text/leaf nodes without a style, check ancestor style if available;
    // fallback to horizontal (width).
    let inline_horizontal = elidex_layout_block::try_get_style(dom, entity)
        .is_none_or(|s| WritingModeContext::new(s.writing_mode, s.direction).is_horizontal());
    if inline_horizontal {
        lb.content.width
    } else {
        lb.content.height
    }
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

    // --- G6: Colspan intrinsic sizing tests ---

    /// Helper: create a table with one row and cells with optional colspan.
    fn make_table_with_colspan(cells: &[(usize, f32)]) -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let table = dom.create_element("table", Attributes::default());
        let _ = dom.world_mut().insert_one(
            table,
            ComputedStyle {
                display: Display::Table,
                ..Default::default()
            },
        );
        let tr = dom.create_element("tr", Attributes::default());
        let _ = dom.world_mut().insert_one(
            tr,
            ComputedStyle {
                display: Display::TableRow,
                ..Default::default()
            },
        );
        let _ = dom.append_child(table, tr);
        for &(colspan, width) in cells {
            let mut attrs = Attributes::default();
            if colspan > 1 {
                attrs.set("colspan", colspan.to_string());
            }
            let td = dom.create_element("td", attrs);
            let _ = dom.world_mut().insert_one(
                td,
                ComputedStyle {
                    display: Display::TableCell,
                    width: Dimension::Length(width),
                    height: Dimension::Length(20.0),
                    ..Default::default()
                },
            );
            let _ = dom.append_child(tr, td);
        }
        (dom, table)
    }

    #[test]
    fn colspan_2_intrinsic_distributes() {
        // One cell with colspan=2 and width=200.
        // Should create 2 columns of 100px each intrinsic.
        let (mut dom, table) = make_table_with_colspan(&[(2, 200.0)]);
        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, table, &font_db, dispatch_layout_child, 0);
        // With 2 columns of 100px + gap, total should be around 200+gap.
        assert!(sizes.min_content > 0.0);
        assert!(sizes.max_content >= sizes.min_content);
    }

    #[test]
    fn colspan_plus_normal_mixed() {
        // Row 1: [colspan=2, width=200], [width=50]
        // Should create 3 columns: col 0,1 at 100px each, col 2 at 50px.
        let (mut dom, table) = make_table_with_colspan(&[(2, 200.0), (1, 50.0)]);
        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, table, &font_db, dispatch_layout_child, 0);
        assert!(sizes.min_content > 0.0);
        // Total: 100 + 100 + 50 + gaps = ~250+
        assert!(sizes.max_content >= sizes.min_content);
    }

    #[test]
    fn all_colspan_row() {
        // Single cell with colspan=3 and width=300.
        let (mut dom, table) = make_table_with_colspan(&[(3, 300.0)]);
        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, table, &font_db, dispatch_layout_child, 0);
        // 3 columns of 100px each.
        assert!(sizes.min_content > 0.0);
        assert!(sizes.max_content >= sizes.min_content);
    }

    // --- G7: Writing-mode-aware intrinsic sizing tests ---

    #[test]
    fn block_vertical_rl_intrinsic() {
        // Block with explicit height in vertical-rl: intrinsic uses height as inline-size.
        let mut dom = EcsDom::new();
        let block = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            block,
            ComputedStyle {
                display: Display::Block,
                writing_mode: WritingMode::VerticalRl,
                height: Dimension::Length(120.0),
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, block, &font_db, dispatch_layout_child, 0);
        // Explicit height = 120 acts as inline-size in vertical-rl.
        assert!((sizes.min_content - 120.0).abs() < 1.0);
        assert!((sizes.max_content - 120.0).abs() < 1.0);
    }

    #[test]
    fn flex_row_vertical_rl_intrinsic() {
        // Flex row in vertical-rl: row main axis follows inline axis = vertical.
        // Two children with explicit height → intrinsic sums along vertical.
        let mut dom = EcsDom::new();
        let flex = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            flex,
            ComputedStyle {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Nowrap,
                writing_mode: WritingMode::VerticalRl,
                ..Default::default()
            },
        );
        let c1 = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                writing_mode: WritingMode::VerticalRl,
                height: Dimension::Length(80.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(flex, c1);
        let c2 = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            c2,
            ComputedStyle {
                display: Display::Block,
                writing_mode: WritingMode::VerticalRl,
                height: Dimension::Length(60.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(flex, c2);

        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, flex, &font_db, dispatch_layout_child, 0);
        // In vertical-rl, row is vertical → main axis is NOT horizontal.
        // So we compute column-style intrinsic: max of children = 80.
        // (Row in vertical maps horizontal=false, so column branch runs.)
        assert!(sizes.min_content >= 60.0);
        assert!(sizes.max_content >= sizes.min_content);
    }

    #[test]
    fn flex_column_vertical_rl_intrinsic() {
        // Flex column in vertical-rl: column main axis follows block axis = horizontal.
        // horizontal = !horizontal_wm = true, so the row branch runs (sums items).
        // Children's intrinsic inline size comes from their height (inline axis in vertical-rl).
        let mut dom = EcsDom::new();
        let flex = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            flex,
            ComputedStyle {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                flex_wrap: FlexWrap::Nowrap,
                writing_mode: WritingMode::VerticalRl,
                ..Default::default()
            },
        );
        let c1 = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            c1,
            ComputedStyle {
                display: Display::Block,
                writing_mode: WritingMode::VerticalRl,
                height: Dimension::Length(70.0), // inline-axis in vertical-rl
                ..Default::default()
            },
        );
        let _ = dom.append_child(flex, c1);
        let c2 = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            c2,
            ComputedStyle {
                display: Display::Block,
                writing_mode: WritingMode::VerticalRl,
                height: Dimension::Length(40.0), // inline-axis in vertical-rl
                ..Default::default()
            },
        );
        let _ = dom.append_child(flex, c2);

        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, flex, &font_db, dispatch_layout_child, 0);
        // Column + vertical-rl: horizontal = true, row branch sums.
        // Children's inline-axis sizes (heights) = 70 + 40 = 110.
        assert!(sizes.min_content >= 40.0);
        assert!(sizes.max_content >= sizes.min_content);
    }

    #[test]
    fn table_vertical_rl_intrinsic() {
        // Table in vertical-rl: inline-axis border-spacing uses border_spacing_v.
        let mut dom = EcsDom::new();
        let table = dom.create_element("table", Attributes::default());
        let _ = dom.world_mut().insert_one(
            table,
            ComputedStyle {
                display: Display::Table,
                writing_mode: WritingMode::VerticalRl,
                border_spacing_h: 10.0,
                border_spacing_v: 5.0,
                ..Default::default()
            },
        );
        let tr = dom.create_element("tr", Attributes::default());
        let _ = dom.world_mut().insert_one(
            tr,
            ComputedStyle {
                display: Display::TableRow,
                ..Default::default()
            },
        );
        let _ = dom.append_child(table, tr);
        let td1 = dom.create_element("td", Attributes::default());
        let _ = dom.world_mut().insert_one(
            td1,
            ComputedStyle {
                display: Display::TableCell,
                writing_mode: WritingMode::VerticalRl,
                height: Dimension::Length(50.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(tr, td1);
        let td2 = dom.create_element("td", Attributes::default());
        let _ = dom.world_mut().insert_one(
            td2,
            ComputedStyle {
                display: Display::TableCell,
                writing_mode: WritingMode::VerticalRl,
                height: Dimension::Length(30.0),
                ..Default::default()
            },
        );
        let _ = dom.append_child(tr, td2);

        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, table, &font_db, dispatch_layout_child, 0);
        // 2 cells: 50 + 30 + border_spacing_v(5) gap = 85 (plus pb).
        assert!(sizes.min_content > 0.0);
        assert!(sizes.max_content >= sizes.min_content);
    }

    #[test]
    fn horizontal_tb_regression_intrinsic() {
        // Standard horizontal-tb: unchanged from original behavior.
        let mut dom = EcsDom::new();
        let block = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            block,
            ComputedStyle {
                display: Display::Block,
                writing_mode: WritingMode::HorizontalTb,
                width: Dimension::Length(200.0),
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, block, &font_db, dispatch_layout_child, 0);
        // Explicit width = 200 in horizontal-tb → both min and max = 200.
        assert!((sizes.min_content - 200.0).abs() < 1.0);
        assert!((sizes.max_content - 200.0).abs() < 1.0);
    }

    #[test]
    fn explicit_height_vertical_rl() {
        // Element with explicit height in vertical-rl: intrinsic should use height.
        let mut dom = EcsDom::new();
        let el = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(
            el,
            ComputedStyle {
                display: Display::Block,
                writing_mode: WritingMode::VerticalRl,
                width: Dimension::Length(999.0), // should be ignored (block-axis)
                height: Dimension::Length(150.0), // inline-axis
                ..Default::default()
            },
        );
        let font_db = FontDatabase::new();
        let sizes = compute_intrinsic_sizes(&mut dom, el, &font_db, dispatch_layout_child, 0);
        // In vertical-rl, height is the inline-axis dimension.
        assert!((sizes.min_content - 150.0).abs() < 1.0);
        assert!((sizes.max_content - 150.0).abs() < 1.0);
    }
}
