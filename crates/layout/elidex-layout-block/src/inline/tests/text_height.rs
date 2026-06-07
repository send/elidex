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
    // CSS Text §4.1.2 trailing-hang applies only to collapsible white space
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
    // CSS Text 3 §4 (#segment-break): U+000C FORM FEED is a Cc control character —
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

// --- §4.1.1 collapse transform (text-level, font-independent) ---

#[test]
fn collapse_normal_collapses_whitespace_runs_to_single_space() {
    // §4.1.1 steps 2-4: tab → space, segment break → space (normal), and a run of
    // collapsible spaces collapses to a single space.
    let Some((dom, parent, style, _font_db)) = setup_inline_test("a \t\n  b") else {
        return;
    };
    let children = dom.composed_children(parent);
    let runs = collect_styled_runs(&dom, &children, &style, parent);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "a b");
}

#[test]
fn collapse_pre_preserves_whitespace() {
    let Some((dom, parent, mut style, _font_db)) = setup_inline_test("a \t\n  b") else {
        return;
    };
    style.white_space = WhiteSpace::Pre;
    let children = dom.composed_children(parent);
    let runs = collect_styled_runs(&dom, &children, &style, parent);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "a \t\n  b");
}

#[test]
fn collapse_pre_line_preserves_newline_collapses_spaces() {
    // pre-line: collapsible spaces collapse and the spaces around the preserved
    // segment break are removed (§4.1.1 step 1), but the break itself is kept.
    let Some((dom, parent, mut style, _font_db)) = setup_inline_test("a  \n  b") else {
        return;
    };
    style.white_space = WhiteSpace::PreLine;
    let children = dom.composed_children(parent);
    let runs = collect_styled_runs(&dom, &children, &style, parent);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "a\nb");
}

#[test]
fn collapse_normal_trims_leading_whitespace_at_ifc_start() {
    // CSS Text §4.1.2: leading collapsible white space at the start of the inline
    // formatting context collapses away — it does not become a leading space that
    // shifts content.
    let Some((dom, parent, style, _font_db)) = setup_inline_test("  hello") else {
        return;
    };
    let children = dom.composed_children(parent);
    let runs = collect_styled_runs(&dom, &children, &style, parent);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "hello");
}

#[test]
fn collapse_across_adjacent_text_runs_yields_single_space() {
    // Cross-run collapse (§4.1.1 step 4: a collapsible space following another
    // collapsible space — even across inline boundaries within the same IFC —
    // collapses): three adjacent text nodes "x" / "\n  " / "y" collapse so the
    // inter-run whitespace becomes a single space, not dropped or doubled.
    let Some((mut dom, parent, style, _font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }
    for t in ["x", "\n  ", "y"] {
        let tx = dom.create_text(t);
        dom.append_child(parent, tx);
    }
    let children = dom.composed_children(parent);
    let runs = collect_styled_runs(&dom, &children, &style, parent);
    let texts: Vec<&str> = runs.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(texts, vec!["x", " ", "y"]);
}

#[test]
fn collapse_pre_line_normalizes_cr_to_preserved_break() {
    // CSS Text 3 §4.1.3: CRLF and bare CR normalize to the segment break `\n`.
    // Under pre-line that break is preserved — CR must NOT be treated as a
    // collapsible space.
    let Some((dom, parent, mut style, _font_db)) = setup_inline_test("a\r\nb\rc") else {
        return;
    };
    style.white_space = WhiteSpace::PreLine;
    let children = dom.composed_children(parent);
    let runs = collect_styled_runs(&dom, &children, &style, parent);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "a\nb\nc");
}

#[test]
fn collapse_normal_normalizes_cr_then_collapses_to_space() {
    // Under normal, the normalized segment breaks collapse to spaces.
    let Some((dom, parent, style, _font_db)) = setup_inline_test("a\r\nb\rc") else {
        return;
    };
    let children = dom.composed_children(parent);
    let runs = collect_styled_runs(&dom, &children, &style, parent);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "a b c");
}

#[test]
fn collapse_pre_line_trims_cross_run_space_before_break() {
    // pre-line, §4.1.1 step 1 across a run boundary: a collapsible space left at
    // the end of one run, immediately before a preserved segment break beginning
    // the next run, is removed from the previous run.
    let Some((mut dom, parent, mut style, _font_db)) = setup_inline_test("") else {
        return;
    };
    style.white_space = WhiteSpace::PreLine;
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }
    for t in ["a ", "\nb"] {
        let tx = dom.create_text(t);
        dom.append_child(parent, tx);
    }
    let children = dom.composed_children(parent);
    let runs = collect_styled_runs(&dom, &children, &style, parent);
    let texts: Vec<&str> = runs.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(texts, vec!["a", "\nb"]);
}

#[test]
fn collapse_pre_line_trims_past_empty_intermediate_run() {
    // pre-line cross-run trim must target the run that emitted the pending space,
    // not an intermediate run that collapsed to empty: "a " / "  " / "\nb" →
    // "a" / "" / "\nb" (the trailing space is removed from the first run, even
    // though the middle run collapses away).
    let Some((mut dom, parent, mut style, _font_db)) = setup_inline_test("") else {
        return;
    };
    style.white_space = WhiteSpace::PreLine;
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }
    for t in ["a ", "  ", "\nb"] {
        let tx = dom.create_text(t);
        dom.append_child(parent, tx);
    }
    let children = dom.composed_children(parent);
    let runs = collect_styled_runs(&dom, &children, &style, parent);
    let texts: Vec<&str> = runs.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(texts, vec!["a", "", "\nb"]);
}

#[test]
fn text_wrapping_increases_height() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("hello world foo bar baz")
    else {
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
    let h = layout_inline_context(&mut dom, &children, 1.0, parent, Point::ZERO, &env).height;
    assert!(h > css_line_height);
}

// --- Vertical writing mode ---

#[test]
fn vertical_mode_uses_font_size_line_advance() {
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("Hello") else {
        return;
    };
    style.writing_mode = WritingMode::VerticalRl;
    let _ = dom.world_mut().insert_one(parent, style.clone());

    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let block_dim =
        layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env).height;
    assert!(
        (block_dim - style.font_size).abs() < f32::EPSILON,
        "vertical single line should be font_size ({}), got {}",
        style.font_size,
        block_dim,
    );
}

#[test]
fn vertical_lr_same_as_vertical_rl_for_height() {
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("Hello") else {
        return;
    };
    style.writing_mode = WritingMode::VerticalLr;
    let _ = dom.world_mut().insert_one(parent, style.clone());

    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let block_dim =
        layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env).height;
    assert!(
        (block_dim - style.font_size).abs() < f32::EPSILON,
        "vertical-lr single line should be font_size ({}), got {}",
        style.font_size,
        block_dim,
    );
}

#[test]
fn horizontal_tb_uses_line_height() {
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
    assert!(
        (h - css_line_height).abs() < f32::EPSILON,
        "horizontal-tb single line should be line-height ({css_line_height}), got {h}",
    );
}

// --- Multi-style inline layout ---

#[test]
fn styled_runs_collect_from_nested_span() {
    let Some((mut dom, parent, style, _font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }

    let text1 = dom.create_text("Hello ");
    dom.append_child(parent, text1);
    let span = dom.create_element("span", Attributes::default());
    let span_style = ComputedStyle {
        font_size: 24.0,
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(span, span_style);
    dom.append_child(parent, span);
    let text2 = dom.create_text("World");
    dom.append_child(span, text2);

    let children = dom.composed_children(parent);
    let runs = collect_styled_runs(&dom, &children, &style, parent);
    assert_eq!(runs.len(), 2, "should have 2 runs");
    assert_eq!(runs[0].text, "Hello ");
    assert!((runs[0].font_size - style.font_size).abs() < f32::EPSILON);
    assert_eq!(runs[1].text, "World");
    assert!((runs[1].font_size - 24.0).abs() < f32::EPSILON);
}

#[test]
fn multi_style_line_height_uses_max() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }

    let text1 = dom.create_text("A");
    dom.append_child(parent, text1);
    let span = dom.create_element("span", Attributes::default());
    let big_style = ComputedStyle {
        font_size: 32.0,
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let big_line_height = big_style.line_height.resolve_px(big_style.font_size);
    let _ = dom.world_mut().insert_one(span, big_style);
    dom.append_child(parent, span);
    let text2 = dom.create_text("B");
    dom.append_child(span, text2);

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
    assert!(
        (h - big_line_height).abs() < 1.0,
        "line height should be the bigger style's line-height ({big_line_height}), got {h}",
    );
}

#[test]
fn display_none_child_skipped_in_runs() {
    let Some((mut dom, parent, style, _font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }

    let text1 = dom.create_text("visible");
    dom.append_child(parent, text1);
    let hidden = dom.create_element("span", Attributes::default());
    let hidden_style = ComputedStyle {
        display: Display::None,
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(hidden, hidden_style);
    dom.append_child(parent, hidden);
    let text2 = dom.create_text("hidden");
    dom.append_child(hidden, text2);

    let children = dom.composed_children(parent);
    let runs = collect_styled_runs(&dom, &children, &style, parent);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "visible");
}

// --- Inline elements get LayoutBox ---

#[test]
fn inline_span_gets_layout_box() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }

    let text1 = dom.create_text("Hello ");
    dom.append_child(parent, text1);
    let span = dom.create_element("span", Attributes::default());
    let span_style = ComputedStyle {
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(span, span_style);
    dom.append_child(parent, span);
    let text2 = dom.create_text("World");
    dom.append_child(span, text2);

    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let _result = layout_inline_context(
        &mut dom,
        &children,
        800.0,
        parent,
        Point::new(10.0, 20.0),
        &env,
    );

    let lb = dom.world().get::<&LayoutBox>(span);
    assert!(
        lb.is_ok(),
        "inline span should have a LayoutBox after layout"
    );
    let lb = lb.unwrap();
    assert!(
        lb.content.origin.x >= 10.0,
        "span x should be >= content_origin.x"
    );
    assert!(
        (lb.content.origin.y - 20.0).abs() < f32::EPSILON,
        "span y should be content_origin.y"
    );
    assert!(
        lb.content.size.width > 0.0,
        "span should have positive width"
    );
    assert!(
        lb.content.size.height > 0.0,
        "span should have positive height"
    );
}

#[test]
fn whitespace_only_inline_span_gets_no_layout_box() {
    // A span whose only content is collapsible whitespace generates no box — its
    // line is suppressed (CSS 2 §9.2.2.1) — so it must NOT get a phantom LayoutBox /
    // getClientRects geometry. The per-line rects are discarded on suppression.
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }
    let span = dom.create_element("span", Attributes::default());
    let span_style = ComputedStyle {
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(span, span_style);
    dom.append_child(parent, span);
    let ws = dom.create_text("   ");
    dom.append_child(span, ws);

    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let _ = layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env);

    assert!(
        dom.world().get::<&LayoutBox>(span).is_err(),
        "whitespace-only inline span must not get a phantom LayoutBox",
    );
}

#[test]
fn multi_line_inline_box_unions_fragment_bounds() {
    // A `<span>` spanning two lines must get a LayoutBox whose width encloses the
    // WIDER line, not just the last (narrow) fragment — `getBoundingClientRect` is
    // the union of the fragment rects. `white-space: pre` forces the break.
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }
    let span = dom.create_element("span", Attributes::default());
    let span_style = ComputedStyle {
        white_space: WhiteSpace::Pre,
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(span, span_style);
    dom.append_child(parent, span);
    let text = dom.create_text("WIDE\nx");
    dom.append_child(span, text);

    let children = dom.composed_children(parent);
    let env = crate::LayoutEnv {
        font_db: &font_db,
        layout_child: crate::layout_block_only,
        depth: 0,
        viewport: None,
        layout_generation: 0,
        is_probe: false,
    };
    let _ = layout_inline_context(&mut dom, &children, 8000.0, parent, Point::ZERO, &env);

    let lb = dom.world().get::<&LayoutBox>(span);
    assert!(lb.is_ok(), "multi-line span should get a LayoutBox");
    let span_width = lb.unwrap().content.size.width;

    let params = TextMeasureParams {
        families: TEST_FAMILIES,
        font_size: style.font_size,
        weight: 400,
        style: elidex_text::FontStyle::Normal,
        letter_spacing: 0.0,
        word_spacing: 0.0,
    };
    let narrow = measure_text(&font_db, &params, "x").map_or(0.0, |m| m.width);
    // Union ⇒ the box width reflects the wide first line, so it is strictly wider
    // than the narrow last line alone (the overwrite-bug result).
    assert!(
        span_width > narrow + 1.0,
        "multi-line span box width {span_width} must enclose the wider line, not just the narrow last line ({narrow})",
    );
}

#[test]
fn parent_entity_does_not_get_inline_layout_box() {
    let Some((mut dom, parent, _style, font_db)) = setup_inline_test("Hello") else {
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
    let _result = layout_inline_context(&mut dom, &children, 800.0, parent, Point::ZERO, &env);

    assert!(
        dom.world().get::<&LayoutBox>(parent).is_err(),
        "parent entity should not get LayoutBox from inline layout"
    );
}

// --- Atomic inline boxes (InlineBlock) ---

#[test]
fn inline_block_participates_in_ifc() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }

    let text1 = dom.create_text("Hello ");
    dom.append_child(parent, text1);
    let ib = dom.create_element("span", Attributes::default());
    let ib_style = ComputedStyle {
        display: Display::InlineBlock,
        width: Dimension::Length(50.0),
        height: Dimension::Length(30.0),
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(ib, ib_style);
    dom.append_child(parent, ib);
    let ib_text = dom.create_text("X");
    dom.append_child(ib, ib_text);
    let text2 = dom.create_text(" World");
    dom.append_child(parent, text2);

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

    let ib_lb = dom.world().get::<&LayoutBox>(ib);
    assert!(ib_lb.is_ok(), "inline-block should have a LayoutBox");
    let ib_lb = ib_lb.unwrap();
    assert!(
        (ib_lb.content.size.width - 50.0).abs() < f32::EPSILON,
        "inline-block width should be 50px, got {}",
        ib_lb.content.size.width
    );

    assert!(
        h >= 30.0,
        "line height should be >= inline-block height (30px), got {h}"
    );
}

#[test]
fn inline_block_not_block_level() {
    assert!(
        !crate::block::is_block_level(Display::InlineBlock),
        "InlineBlock should not be block-level"
    );
    assert!(
        !crate::block::is_block_level(Display::InlineFlex),
        "InlineFlex should not be block-level"
    );
    assert!(
        !crate::block::is_block_level(Display::InlineGrid),
        "InlineGrid should not be block-level"
    );
    assert!(
        !crate::block::is_block_level(Display::InlineTable),
        "InlineTable should not be block-level"
    );
}

// --- Anonymous block boxes (CSS 2.1 §9.2.1.1) ---

#[test]
fn mixed_block_inline_anonymous_box() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }

    let _ = dom.world_mut().insert_one(parent, style.clone());

    let text1 = dom.create_text("Hello ");
    dom.append_child(parent, text1);
    let block = dom.create_element("p", Attributes::default());
    let block_style = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(40.0),
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(block, block_style);
    dom.append_child(parent, block);
    let text2 = dom.create_text(" World");
    dom.append_child(parent, text2);

    let children_list = dom.composed_children(parent);
    let input = crate::LayoutInput {
        containing: elidex_plugin::CssSize::width_only(800.0),
        containing_inline_size: 800.0,
        offset: Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
        is_probe: false,
    };
    let result = crate::block::stack_block_children(
        &mut dom,
        &children_list,
        &input,
        crate::layout_block_only,
        false,
        parent,
    );

    assert!(
        result.height >= 40.0,
        "height should be at least block child height (40), got {}",
        result.height
    );
    let line_h = style.line_height.resolve_px(style.font_size);
    let expected_min = 40.0 + line_h;
    assert!(
        result.height >= expected_min,
        "height should include anonymous box height ({expected_min}), got {}",
        result.height
    );
}

#[test]
fn block_only_children_no_anonymous_boxes() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }
    let _ = dom.world_mut().insert_one(parent, style.clone());

    let block1 = dom.create_element("div", Attributes::default());
    let block_style = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(20.0),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(block1, block_style.clone());
    dom.append_child(parent, block1);

    let block2 = dom.create_element("div", Attributes::default());
    let _ = dom.world_mut().insert_one(block2, block_style);
    dom.append_child(parent, block2);

    let children_list = dom.composed_children(parent);
    let input = crate::LayoutInput {
        containing: elidex_plugin::CssSize::width_only(800.0),
        containing_inline_size: 800.0,
        offset: Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
        is_probe: false,
    };
    let result = crate::block::stack_block_children(
        &mut dom,
        &children_list,
        &input,
        crate::layout_block_only,
        false,
        parent,
    );

    assert!(
        (result.height - 40.0).abs() < f32::EPSILON,
        "height should be 40.0 (2 x 20), got {}",
        result.height
    );
}

#[test]
fn display_none_skipped_in_block_context() {
    let Some((mut dom, parent, style, font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }
    let _ = dom.world_mut().insert_one(parent, style.clone());

    let hidden = dom.create_element("span", Attributes::default());
    let hidden_style = ComputedStyle {
        display: Display::None,
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(hidden, hidden_style);
    dom.append_child(parent, hidden);
    let hidden_text = dom.create_text("invisible");
    dom.append_child(hidden, hidden_text);

    let block = dom.create_element("div", Attributes::default());
    let block_style = ComputedStyle {
        display: Display::Block,
        height: Dimension::Length(30.0),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(block, block_style);
    dom.append_child(parent, block);

    let children_list = dom.composed_children(parent);
    let input = crate::LayoutInput {
        containing: elidex_plugin::CssSize::width_only(800.0),
        containing_inline_size: 800.0,
        offset: Point::ZERO,
        font_db: &font_db,
        depth: 0,
        float_ctx: None,
        viewport: None,
        fragmentainer: None,
        break_token: None,
        subgrid: None,
        layout_generation: 0,
        is_probe: false,
    };
    let result = crate::block::stack_block_children(
        &mut dom,
        &children_list,
        &input,
        crate::layout_block_only,
        false,
        parent,
    );

    assert!(
        (result.height - 30.0).abs() < f32::EPSILON,
        "height should be 30.0 (block only), got {}",
        result.height
    );
}

#[test]
fn atomic_inline_skipped_in_styled_runs() {
    let Some((mut dom, parent, style, _font_db)) = setup_inline_test("") else {
        return;
    };
    let children = dom.composed_children(parent);
    for &c in &children {
        dom.remove_child(parent, c);
    }

    let text1 = dom.create_text("Hello ");
    dom.append_child(parent, text1);
    let ib = dom.create_element("span", Attributes::default());
    let ib_style = ComputedStyle {
        display: Display::InlineBlock,
        font_family: style.font_family.clone(),
        ..Default::default()
    };
    let _ = dom.world_mut().insert_one(ib, ib_style);
    dom.append_child(parent, ib);
    let ib_text = dom.create_text("IB");
    dom.append_child(ib, ib_text);
    let text2 = dom.create_text(" World");
    dom.append_child(parent, text2);

    let children = dom.composed_children(parent);
    let runs = collect_styled_runs(&dom, &children, &style, parent);
    assert_eq!(runs.len(), 2, "should have 2 text runs (Hello + World)");
    assert_eq!(runs[0].text, "Hello ");
    assert_eq!(runs[1].text, " World");
}
