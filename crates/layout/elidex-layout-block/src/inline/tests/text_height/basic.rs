//! Basic inline text height, and the `white-space` values that preserve
//! newlines or blank lines as line boxes.

use super::*;
use elidex_plugin::WhiteSpace;

#[test]
fn empty_text_zero_height() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("p", Attributes::default());
    let text = dom.create_text("");
    dom.append_child(parent, text);

    let font_db = FontDatabase::new();
    let children = dom.composed_children(parent);

    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let h = layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env).height;
    assert!(h.abs() < f32::EPSILON);
}

#[test]
fn no_children_zero_height() {
    let mut dom = EcsDom::new();
    let parent_entity = Entity::DANGLING;
    let font_db = FontDatabase::new();

    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let h = layout_inline_context(&mut dom, &[], 800.0, parent_entity, Point::ZERO, &env).height;
    assert!(h.abs() < f32::EPSILON);
}

#[test]
fn single_line_text() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("Hello") else {
        return;
    };

    let css_line_height = style.line_height.resolve_px(style.font_size);
    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let h = layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env).height;
    assert!((h - css_line_height).abs() < f32::EPSILON);
}

#[test]
fn normal_collapses_newline_to_space() {
    // CSS Text 3 §4.1.1 / §4.1.3: under `white-space: normal` a segment break is
    // collapsible and is transformed to a space, so "line1\nline2" is one line.
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("line1\nline2") else {
        return;
    };

    let css_line_height = style.line_height.resolve_px(style.font_size);
    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let h = layout_inline_context(&mut dom, &children, 8000.0, parent, Point::ZERO, &env).height;
    assert!(
        (h - css_line_height).abs() < f32::EPSILON,
        "normal white-space collapses the newline to a space (one line, {css_line_height}), got {h}",
    );
}

#[test]
fn pre_preserves_newline_as_break() {
    // CSS Text 3 §4.1.3: under `white-space: pre` a segment break is preserved as a
    // forced line break, so "line1\nline2" is two lines.
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("line1\nline2") else {
        return;
    };
    style.white_space = WhiteSpace::Pre;
    let _ = dom.world_mut().insert_one(parent, style.clone());

    let css_line_height = style.line_height.resolve_px(style.font_size);
    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let h = layout_inline_context(&mut dom, &children, 8000.0, parent, Point::ZERO, &env).height;
    assert!(
        (h - css_line_height * 2.0).abs() < f32::EPSILON,
        "pre preserves the newline as a forced break (two lines), got {h}",
    );
}

#[test]
fn pre_blank_line_keeps_height() {
    // A blank line in `<pre>` ("a\n\nb") still generates a line box with height: the
    // forced-break path marks the line as rendered content (three lines total).
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("a\n\nb") else {
        return;
    };
    style.white_space = WhiteSpace::Pre;
    let _ = dom.world_mut().insert_one(parent, style.clone());

    let css_line_height = style.line_height.resolve_px(style.font_size);
    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let h = layout_inline_context(&mut dom, &children, 8000.0, parent, Point::ZERO, &env).height;
    assert!(
        (h - css_line_height * 3.0).abs() < f32::EPSILON,
        "pre keeps the blank line's height (three lines), got {h}",
    );
}

#[test]
fn pre_newline_only_keeps_line_height() {
    // `<pre>` whose content is a single newline still generates a line box. The
    // end-of-text segment break is filtered out of `find_break_opportunities`, so
    // `force_break` never runs — the segment must be marked as rendered content
    // directly, otherwise the line is incorrectly suppressed to zero height.
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("\n") else {
        return;
    };
    style.white_space = WhiteSpace::Pre;
    let _ = dom.world_mut().insert_one(parent, style.clone());

    let css_line_height = style.line_height.resolve_px(style.font_size);
    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let h = layout_inline_context(&mut dom, &children, 8000.0, parent, Point::ZERO, &env).height;
    assert!(
        (h - css_line_height).abs() < f32::EPSILON,
        "pre newline-only content keeps its line height (one line), got {h}",
    );
}

#[test]
fn pre_spaces_only_keeps_line_height() {
    // `<pre>   </pre>`: preserved spaces are rendered content and give the line its
    // height — the box-suppression (CSS 2 §9.2.2.1) applies only to *collapsible*
    // white space, so a preserved spaces-only line is NOT suppressed.
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("   ") else {
        return;
    };
    style.white_space = WhiteSpace::Pre;
    let _ = dom.world_mut().insert_one(parent, style.clone());

    let css_line_height = style.line_height.resolve_px(style.font_size);
    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let h = layout_inline_context(&mut dom, &children, 8000.0, parent, Point::ZERO, &env).height;
    assert!(
        (h - css_line_height).abs() < f32::EPSILON,
        "pre spaces-only line keeps its height (one line), got {h}",
    );
}

#[test]
fn collapsible_whitespace_only_generates_no_line_box() {
    // CSS 2 §9.2.2.1: a line of only collapsible white space generates no box — not
    // a zero-height one — so `line_count` is 0, not a phantom 1.
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("   ") else {
        return;
    };
    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let result = layout_inline_context(&mut dom, &children, 8000.0, parent, Point::ZERO, &env);
    assert_eq!(
        result.line_count, 0,
        "collapsible whitespace-only content generates no line box",
    );
    assert!(result.height.abs() < f32::EPSILON);
    // No box ⇒ no first baseline captured from the suppressed whitespace segment.
    assert!(
        result.first_baseline.is_none(),
        "suppressed whitespace must not set first_baseline",
    );
}

#[test]
fn nbsp_only_line_generates_a_box() {
    // A no-break space (U+00A0) renders and gives the line its height: it is not
    // collapsible white space, so unlike a regular-space-only line it generates a
    // box (CSS 2 §9.2.2.1 applies only to collapsible white space).
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("\u{00A0}") else {
        return;
    };
    let css_line_height = style.line_height.resolve_px(style.font_size);
    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let result = layout_inline_context(&mut dom, &children, 8000.0, parent, Point::ZERO, &env);
    assert_eq!(
        result.line_count, 1,
        "nbsp-only content generates a line box"
    );
    assert!((result.height - css_line_height).abs() < f32::EPSILON);
}

#[test]
fn trailing_nbsp_does_not_hang_for_overflow() {
    // CSS Text 3 §4.1.2 trailing-hang covers collapsible white space (and, under
    // `pre-wrap`, preserved spaces) — but not U+00A0, which §4.1 excludes from
    // "other space separators"
    // (ASCII space/tab). A trailing no-break space (U+00A0) is non-collapsible and
    // counts toward overflow, so trimmed_width == full width for an NBSP-terminated
    // segment, whereas an ASCII-space-terminated segment hangs (trimmed < full).
    let Some((_dom, _parent, style, font_db)) = setup_inline_test("x") else {
        return;
    };
    let params = TextMeasureParams {
        families: TEST_FAMILIES,
        font_size: style.font_size,
        weight: 400,
        style: elidex_text::FontStyle::Normal,
        letter_spacing: 0.0,
        word_spacing: 0.0,
    };
    let (full_space, trimmed_space) =
        super::super::measure::measure_segment_widths(&font_db, &params, "a ");
    assert!(
        trimmed_space < full_space,
        "trailing ASCII space should hang (trimmed {trimmed_space} < full {full_space})",
    );
    let (full_nbsp, trimmed_nbsp) =
        super::super::measure::measure_segment_widths(&font_db, &params, "a\u{00A0}");
    assert!(
        (trimmed_nbsp - full_nbsp).abs() < f32::EPSILON,
        "trailing NBSP must not hang (trimmed {trimmed_nbsp} == full {full_nbsp})",
    );
}

#[test]
fn collapse_preserves_form_feed_as_glyph() {
    // CSS Text 3 §4 (#white-space-processing): U+000C FORM FEED is a Cc control character —
    // not a tab/LF/CR — so it is rendered as a visible glyph, NOT treated as a
    // segment break or collapsible white space. It must survive collapsing intact.
    let Some((dom, parent, style, _font_db)) = setup_inline_test("a\u{000C}b") else {
        return;
    };
    let children = dom.composed_children(parent);
    let runs = collect_styled_runs(&dom, &children, &style, parent);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "a\u{000C}b");
}
