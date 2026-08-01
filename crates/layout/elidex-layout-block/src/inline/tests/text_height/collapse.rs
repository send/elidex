//! CSS Text 3 §4.1.1 Phase I collapse transform — text-level and
//! font-independent, asserted on the collected runs rather than on geometry.

use super::*;
use elidex_plugin::WhiteSpace;

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
    // CSS Text 3 §4.1.2: leading collapsible white space at the start of the inline
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
