//! Inline formatting context layout algorithm.
//!
//! Handles text measurement and line breaking for inline content.
//! Text nodes are measured via `elidex-text::measure_text()` and
//! greedily packed into lines that fit the containing block width.

use elidex_ecs::{EcsDom, Entity, PseudoElementMarker, TextContent};
use elidex_plugin::{ComputedStyle, Display, WritingMode};
use elidex_text::{
    find_break_opportunities, measure_text, to_fontdb_style, BreakOpportunity, FontDatabase,
    TextMeasureParams,
};

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
            // Pseudo-element: use text directly (skip child recursion).
            if dom.world().get::<&PseudoElementMarker>(child).is_ok() {
                if let Ok(tc) = dom.world().get::<&TextContent>(child) {
                    text.push_str(&tc.0);
                }
                continue;
            }
            // Inline element: collect text from its children.
            let grandchildren = dom.composed_children(child);
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
    params: &TextMeasureParams<'_>,
    segment: &str,
) -> (f32, f32) {
    let seg_width = measure_text(font_db, params, segment).map_or(0.0, |m| m.width);
    let trimmed = segment.trim_end();
    let trimmed_width = if trimmed.len() == segment.len() {
        seg_width
    } else if trimmed.is_empty() {
        0.0
    } else {
        measure_text(font_db, params, trimmed).map_or(0.0, |m| m.width)
    };
    (seg_width, trimmed_width)
}

/// Layout inline content (text nodes and inline elements) within a line box.
///
/// Returns the total block-axis dimension consumed by all line boxes.
/// For `horizontal-tb` this is the total height; for vertical writing
/// modes (`vertical-rl`/`vertical-lr`) this is the total width.
///
/// `containing_inline_size` is the available inline-axis space
/// (width for horizontal, height for vertical).
// TODO: use caller-provided offsets to position line boxes and assign
// LayoutBox components to inline elements and text nodes.
pub fn layout_inline_context(
    dom: &EcsDom,
    children: &[Entity],
    containing_inline_size: f32,
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
    let params = TextMeasureParams {
        families: &families,
        font_size: parent_style.font_size,
        weight: parent_style.font_weight,
        style: to_fontdb_style(parent_style.font_style),
        letter_spacing: parent_style.letter_spacing.unwrap_or(0.0),
        word_spacing: parent_style.word_spacing.unwrap_or(0.0),
    };

    // Use CSS line-height (resolved to px via the element's font-size).
    let line_height = parent_style.line_height.resolve_px(params.font_size);

    // Verify a font is available (needed for segment width measurement).
    if measure_text(font_db, &params, "x").is_none() {
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

    let is_vertical = !matches!(parent_style.writing_mode, WritingMode::HorizontalTb);

    // For vertical writing: each "line" stacks along the block axis (X),
    // and the line advance is the font-size (column width), not line-height.
    // For horizontal: "line" stacks along Y, line advance = line-height.
    let line_advance = if is_vertical {
        params.font_size
    } else {
        line_height
    };

    // Greedy line packing: accumulate segment inline sizes until overflow
    // or mandatory break.
    let mut total_block = 0.0_f32;
    let mut current_inline = 0.0_f32;
    let mut on_line = false;

    for &(start, end, break_kind) in &segments {
        let Some(segment) = text.get(start..end) else {
            continue;
        };
        if segment.is_empty() {
            continue;
        }

        // TODO(Phase 4): use vertical shaping metrics for vertical modes.
        let (seg_width, trimmed_width) = measure_segment_widths(font_db, &params, segment);

        if current_inline + trimmed_width > containing_inline_size && on_line {
            // Current line overflows — wrap to next line.
            total_block += line_advance;
            current_inline = seg_width;
        } else {
            current_inline += seg_width;
        }
        on_line = true;

        // Mandatory break: force a new line after this segment.
        if break_kind == Some(BreakOpportunity::Mandatory) {
            total_block += line_advance;
            current_inline = 0.0;
            on_line = false;
        }
    }

    // Account for the last line (if not already ended by mandatory break).
    if on_line {
        total_block += line_advance;
    }

    total_block
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

    /// Setup a DOM with a `<p>` parent and a text child, a default `ComputedStyle`
    /// with test font families, and a `FontDatabase`. Returns `None` if no font is available.
    fn setup_inline_test(
        text_content: &str,
    ) -> Option<(EcsDom, Entity, ComputedStyle, FontDatabase)> {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("p", Attributes::default());
        let text = dom.create_text(text_content);
        dom.append_child(parent, text);

        let style = ComputedStyle {
            font_family: TEST_FAMILIES.iter().map(|&s| s.to_string()).collect(),
            ..Default::default()
        };
        let font_db = FontDatabase::new();
        let params = TextMeasureParams {
            families: TEST_FAMILIES,
            font_size: style.font_size,
            weight: 400,
            style: elidex_text::FontStyle::Normal,
            letter_spacing: 0.0,
            word_spacing: 0.0,
        };
        measure_text(&font_db, &params, "x")?;
        Some((dom, parent, style, font_db))
    }

    #[test]
    fn empty_text_zero_height() {
        let mut dom = EcsDom::new();
        let parent = dom.create_element("p", Attributes::default());
        let text = dom.create_text("");
        dom.append_child(parent, text);

        let style = ComputedStyle::default();
        let font_db = FontDatabase::new();
        let children = dom.composed_children(parent);

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
        let Some((dom, parent, style, font_db)) = setup_inline_test("Hello") else {
            return;
        };

        let css_line_height = style.line_height.resolve_px(style.font_size);
        let children = dom.composed_children(parent);
        let h = layout_inline_context(&dom, &children, 800.0, &style, &font_db);
        assert!((h - css_line_height).abs() < f32::EPSILON);
    }

    #[test]
    fn mandatory_newline_break() {
        let Some((dom, parent, style, font_db)) = setup_inline_test("line1\nline2") else {
            return;
        };

        let css_line_height = style.line_height.resolve_px(style.font_size);
        // Wide container: should still produce 2 lines due to \n
        let children = dom.composed_children(parent);
        let h = layout_inline_context(&dom, &children, 8000.0, &style, &font_db);
        assert!((h - css_line_height * 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn text_wrapping_increases_height() {
        let Some((dom, parent, style, font_db)) = setup_inline_test("hello world foo bar baz")
        else {
            return;
        };

        let css_line_height = style.line_height.resolve_px(style.font_size);
        // Use a very narrow width to force wrapping
        let children = dom.composed_children(parent);
        let h = layout_inline_context(&dom, &children, 1.0, &style, &font_db);
        assert!(h > css_line_height);
    }

    // --- M3.5-4: Vertical writing mode ---

    #[test]
    fn vertical_mode_uses_font_size_line_advance() {
        let Some((dom, parent, mut style, font_db)) = setup_inline_test("Hello") else {
            return;
        };
        style.writing_mode = WritingMode::VerticalRl;

        // In vertical mode, the block-axis advance per line is font_size, not line-height.
        let children = dom.composed_children(parent);
        let block_dim = layout_inline_context(&dom, &children, 800.0, &style, &font_db);
        // Single line: block dimension should be font_size.
        assert!(
            (block_dim - style.font_size).abs() < f32::EPSILON,
            "vertical single line should be font_size ({}), got {}",
            style.font_size,
            block_dim,
        );
    }

    #[test]
    fn vertical_lr_same_as_vertical_rl_for_height() {
        let Some((dom, parent, mut style, font_db)) = setup_inline_test("Hello") else {
            return;
        };
        style.writing_mode = WritingMode::VerticalLr;

        let children = dom.composed_children(parent);
        let block_dim = layout_inline_context(&dom, &children, 800.0, &style, &font_db);
        assert!(
            (block_dim - style.font_size).abs() < f32::EPSILON,
            "vertical-lr single line should be font_size ({}), got {}",
            style.font_size,
            block_dim,
        );
    }

    #[test]
    fn horizontal_tb_uses_line_height() {
        let Some((dom, parent, style, font_db)) = setup_inline_test("Hello") else {
            return;
        };
        // Default writing_mode is HorizontalTb, no modification needed.

        let css_line_height = style.line_height.resolve_px(style.font_size);
        let children = dom.composed_children(parent);
        let h = layout_inline_context(&dom, &children, 800.0, &style, &font_db);
        assert!(
            (h - css_line_height).abs() < f32::EPSILON,
            "horizontal-tb single line should be line-height ({css_line_height}), got {h}",
        );
    }
}
