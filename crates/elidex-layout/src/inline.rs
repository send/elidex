//! Inline formatting context layout algorithm.
//!
//! Handles text measurement and line breaking for inline content.
//! Text nodes are measured via `elidex-text::measure_text()` and
//! greedily packed into lines that fit the containing block width.

use elidex_ecs::{EcsDom, Entity, TextContent};
use elidex_plugin::{ComputedStyle, Display};
use elidex_text::{find_break_opportunities, measure_text, BreakOpportunity, FontDatabase};

use crate::MAX_LAYOUT_DEPTH;

/// Recursively collect text content from inline children.
///
/// Text nodes contribute their content directly. Inline elements
/// contribute their children's text (flattened). `display: none`
/// elements are skipped. Recursion stops at [`MAX_LAYOUT_DEPTH`].
// TODO: preserve per-element styles (font-weight, font-size, etc.)
// by returning styled text runs instead of a flat string.
fn collect_text(dom: &EcsDom, children: &[Entity]) -> String {
    collect_text_inner(dom, children, 0)
}

fn collect_text_inner(dom: &EcsDom, children: &[Entity], depth: u32) -> String {
    if depth >= MAX_LAYOUT_DEPTH {
        return String::new();
    }
    let mut text = String::new();
    for &child in children {
        if let Some(style) = crate::try_get_style(dom, child) {
            if style.display == Display::None {
                continue;
            }
            // Inline element: collect text from its children
            let grandchildren = dom.children(child);
            text.push_str(&collect_text_inner(dom, &grandchildren, depth + 1));
        } else if let Ok(tc) = dom.world().get::<&TextContent>(child) {
            text.push_str(&tc.0);
        }
    }
    text
}

/// Measure a segment's full and trimmed widths.
///
/// Returns `(full_width, trimmed_width)` where `trimmed_width` excludes trailing
/// whitespace per CSS Text Level 3 §4.1.2 (trailing spaces "hang" and don't
/// trigger line overflow).
fn measure_segment_widths(
    font_db: &FontDatabase,
    families: &[&str],
    font_size: f32,
    font_weight: u16,
    segment: &str,
) -> (f32, f32) {
    let seg_width =
        measure_text(font_db, families, font_size, font_weight, segment).map_or(0.0, |m| m.width);
    let trimmed = segment.trim_end();
    let trimmed_width = if trimmed.len() == segment.len() {
        seg_width
    } else if trimmed.is_empty() {
        0.0
    } else {
        measure_text(font_db, families, font_size, font_weight, trimmed).map_or(0.0, |m| m.width)
    };
    (seg_width, trimmed_width)
}

/// Layout inline content (text nodes and inline elements) within a line box.
///
/// Returns the total height consumed by all line boxes.
/// Text is measured using the parent element's font properties.
// TODO: use caller-provided offsets to position line boxes and assign
// LayoutBox components to inline elements and text nodes.
pub(crate) fn layout_inline_context(
    dom: &EcsDom,
    children: &[Entity],
    containing_width: f32,
    parent_style: &ComputedStyle,
    font_db: &FontDatabase,
) -> f32 {
    let text = collect_text(dom, children);
    if text.is_empty() {
        return 0.0;
    }

    let families: Vec<&str> = parent_style
        .font_family
        .iter()
        .map(String::as_str)
        .collect();
    let font_size = parent_style.font_size;
    let font_weight = parent_style.font_weight;

    // Use CSS line-height (resolved to px via the element's font-size).
    let line_height = parent_style.line_height.resolve_px(font_size);

    // Verify a font is available (needed for segment width measurement).
    if measure_text(font_db, &families, font_size, font_weight, "x").is_none() {
        return 0.0; // no font available
    }

    // Find break opportunities and build segments with their trailing break type.
    let breaks = find_break_opportunities(&text);

    // Build (start, end, break_after) tuples. break_after is the BreakOpportunity
    // at position `end`, or None for the final segment.
    let mut segments: Vec<(usize, usize, Option<BreakOpportunity>)> = Vec::new();
    let mut prev_pos = 0;
    for &(bp, kind) in &breaks {
        if bp > prev_pos {
            segments.push((prev_pos, bp, Some(kind)));
        }
        prev_pos = bp;
    }
    if prev_pos < text.len() {
        segments.push((prev_pos, text.len(), None));
    }

    // Greedy line packing: accumulate segment widths until overflow or mandatory break.
    let mut total_height = 0.0_f32;
    let mut current_line_width = 0.0_f32;
    let mut on_line = false;

    for &(start, end, break_kind) in &segments {
        let segment = &text[start..end];
        if segment.is_empty() {
            continue;
        }

        let (seg_width, trimmed_width) =
            measure_segment_widths(font_db, &families, font_size, font_weight, segment);

        if current_line_width + trimmed_width > containing_width && on_line {
            // Current line overflows — wrap to next line.
            total_height += line_height;
            current_line_width = seg_width;
        } else {
            current_line_width += seg_width;
        }
        on_line = true;

        // Mandatory break: force a new line after this segment.
        if break_kind == Some(BreakOpportunity::Mandatory) {
            total_height += line_height;
            current_line_width = 0.0;
            on_line = false;
        }
    }

    // Account for the last line (if not already ended by mandatory break).
    if on_line {
        total_height += line_height;
    }

    total_height
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    const TEST_FAMILIES: &[&str] = &[
        "Arial",
        "Helvetica",
        "Liberation Sans",
        "DejaVu Sans",
        "Noto Sans",
        "Hiragino Sans",
    ];

    #[test]
    fn empty_text_zero_height() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("p", Attributes::default());
        let text = dom.create_text("");
        dom.append_child(parent, text);

        let style = ComputedStyle::default();
        let font_db = FontDatabase::new();
        let children = dom.children(parent);

        let h = layout_inline_context(&dom, &children, 800.0, &style, &font_db);
        assert!(h.abs() < f32::EPSILON);
    }

    #[test]
    fn no_children_zero_height() {
        let dom = EcsDom::new();
        let style = ComputedStyle::default();
        let font_db = FontDatabase::new();

        let h = layout_inline_context(&dom, &[], 800.0, &style, &font_db);
        assert!(h.abs() < f32::EPSILON);
    }

    #[test]
    fn single_line_text() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("p", Attributes::default());
        let text = dom.create_text("Hello");
        dom.append_child(parent, text);

        let style = ComputedStyle {
            font_family: TEST_FAMILIES.iter().map(|&s| s.to_string()).collect(),
            ..Default::default()
        };
        let font_db = FontDatabase::new();

        // Early return if no font available
        if measure_text(&font_db, TEST_FAMILIES, style.font_size, 400, "x").is_none() {
            return;
        }

        let css_line_height = style.line_height.resolve_px(style.font_size);
        let children = dom.children(parent);
        let h = layout_inline_context(&dom, &children, 800.0, &style, &font_db);
        assert!((h - css_line_height).abs() < f32::EPSILON);
    }

    #[test]
    fn mandatory_newline_break() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("p", Attributes::default());
        let text = dom.create_text("line1\nline2");
        dom.append_child(parent, text);

        let style = ComputedStyle {
            font_family: TEST_FAMILIES.iter().map(|&s| s.to_string()).collect(),
            ..Default::default()
        };
        let font_db = FontDatabase::new();

        if measure_text(&font_db, TEST_FAMILIES, style.font_size, 400, "x").is_none() {
            return;
        }

        let css_line_height = style.line_height.resolve_px(style.font_size);
        // Wide container: should still produce 2 lines due to \n
        let children = dom.children(parent);
        let h = layout_inline_context(&dom, &children, 8000.0, &style, &font_db);
        assert!((h - css_line_height * 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn text_wrapping_increases_height() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("p", Attributes::default());
        let text = dom.create_text("hello world foo bar baz");
        dom.append_child(parent, text);

        let style = ComputedStyle {
            font_family: TEST_FAMILIES.iter().map(|&s| s.to_string()).collect(),
            ..Default::default()
        };
        let font_db = FontDatabase::new();

        if measure_text(&font_db, TEST_FAMILIES, style.font_size, 400, "x").is_none() {
            return;
        }

        let css_line_height = style.line_height.resolve_px(style.font_size);
        // Use a very narrow width to force wrapping
        let children = dom.children(parent);
        let h = layout_inline_context(&dom, &children, 1.0, &style, &font_db);
        assert!(h > css_line_height);
    }
}
