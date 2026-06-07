use super::*;
use elidex_ecs::InlineFlow;
use elidex_plugin::TextTransform;

/// Lay out `text` with `text_transform` applied to the parent, returning the
/// persisted single-line run text (the slice's payoff: text-transform now
/// persists instead of gating to render's legacy path).
fn transformed_run_text(
    text: &str,
    transform: TextTransform,
    available_inline: f32,
) -> Option<String> {
    let (mut dom, parent, mut style, font_db) = setup_inline_test(text)?;
    style.text_transform = transform;
    let _ = dom.world_mut().insert_one(parent, style);
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        available_inline,
        parent,
        Point::ZERO,
        &env(&font_db),
    );
    let flow = dom
        .world()
        .get::<&InlineFlow>(key)
        .expect("text-transform run must now persist an InlineFlow");
    Some(
        flow.fragments[0].lines[0].runs[0]
            .text()
            .expect("persisted run must carry text")
            .to_string(),
    )
}

#[test]
fn text_transform_uppercase_persists_transformed_text() {
    // The slice payoff: layout transforms before measuring, so the run persists
    // (no gate) and the persisted text is the final, transformed glyphs.
    let Some(t) = transformed_run_text("hello", TextTransform::Uppercase, 800.0) else {
        return;
    };
    assert_eq!(t, "HELLO");
}

#[test]
fn text_transform_lowercase_persists_transformed_text() {
    let Some(t) = transformed_run_text("HELLO", TextTransform::Lowercase, 800.0) else {
        return;
    };
    assert_eq!(t, "hello");
}

#[test]
fn text_transform_capitalize_word_boundaries() {
    // CSS Text 3 §2.1.1: first typographic letter unit of each word.
    let Some(t) = transformed_run_text("hello world", TextTransform::Capitalize, 800.0) else {
        return;
    };
    assert_eq!(t, "Hello World");
}

#[test]
fn text_transform_capitalize_after_collapse() {
    // CSS Text 3 §2.1.2: transform runs AFTER §4.1.1 collapse, so word
    // boundaries are computed on the collapsed text.
    let Some(t) = transformed_run_text("  hello   world  ", TextTransform::Capitalize, 800.0)
    else {
        return;
    };
    assert!(
        t.contains("Hello World"),
        "collapsed-then-capitalized text should read 'Hello World', got {t:?}"
    );
    assert!(
        !t.contains("hello"),
        "first word must be capitalized: {t:?}"
    );
}

#[test]
fn text_transform_multi_line_each_line_transformed() {
    // Multi-line payoff: the legacy single-linear-pass mis-rendered wrapped
    // transformed runs; converged layout positions each line from the
    // transformed advances. Tiny container forces a wrap at the space.
    let Some((mut dom, parent, mut style, font_db)) = setup_inline_test("hello world") else {
        return;
    };
    style.text_transform = TextTransform::Uppercase;
    let _ = dom.world_mut().insert_one(parent, style);
    let children = dom.composed_children(parent);
    let key = run_start(&dom, parent);
    layout_inline_context(
        &mut dom,
        &children,
        1.0,
        parent,
        Point::ZERO,
        &env(&font_db),
    );

    let flow = dom.world().get::<&InlineFlow>(key).expect("should persist");
    assert_eq!(flow.fragments[0].lines.len(), 2, "wraps into two lines");
    assert!(flow.fragments[0].lines[0].runs[0]
        .text()
        .is_some_and(|t| t.starts_with("HELLO")));
    assert_eq!(flow.fragments[0].lines[1].runs[0].text(), Some("WORLD"));
    assert!(
        flow.fragments[0].lines[1].block_start > flow.fragments[0].lines[0].block_start,
        "second line below the first"
    );
}
