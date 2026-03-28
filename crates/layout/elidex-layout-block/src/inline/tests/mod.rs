use super::*;
use elidex_ecs::Attributes;
use elidex_plugin::{Dimension, Point, WritingMode};

mod baseline;
mod text_height;

/// Collect only text runs from inline items (for tests that don't need atomics).
fn collect_styled_runs(
    dom: &EcsDom,
    children: &[Entity],
    parent_style: &ComputedStyle,
    parent_entity: Entity,
) -> Vec<StyledRun> {
    collect_inline_items(dom, children, parent_style, parent_entity)
        .into_iter()
        .filter_map(|item| match item {
            InlineItem::Text(run) => Some(run),
            InlineItem::Atomic { .. } | InlineItem::Placeholder(_) => None,
        })
        .collect()
}

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
fn setup_inline_test(text_content: &str) -> Option<(EcsDom, Entity, ComputedStyle, FontDatabase)> {
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
    let _ = dom.world_mut().insert_one(parent, style.clone());
    Some((dom, parent, style, font_db))
}
