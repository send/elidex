use std::sync::Arc;

use super::*;
use elidex_ecs::Attributes;
use elidex_plugin::{EdgeSizes, Rect};

mod basic;
mod display_types;
mod styling;

/// Font families used across tests. Covers common system fonts on
/// Linux, macOS, and Windows so that at least one is available on CI.
const TEST_FONT_FAMILIES: &[&str] = &[
    "Arial",
    "Helvetica",
    "Liberation Sans",
    "DejaVu Sans",
    "Noto Sans",
    "Hiragino Sans",
];

/// Build a `Vec<String>` from [`TEST_FONT_FAMILIES`] for `ComputedStyle`.
fn test_font_family_strings() -> Vec<String> {
    TEST_FONT_FAMILIES
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

/// Common test setup: creates a DOM with a root, one block element with a
/// [`ComputedStyle`] and [`LayoutBox`], and returns `(dom, element)`.
///
/// `style_fn` receives a default `ComputedStyle` with `display: Block` and
/// `test_font_family_strings()` pre-filled; callers can override fields.
fn setup_block_element(
    style: elidex_plugin::ComputedStyle,
    layout: elidex_plugin::LayoutBox,
) -> (elidex_ecs::EcsDom, elidex_ecs::Entity) {
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let elem = dom.create_element("div", Attributes::default());
    let _ = dom.append_child(root, elem);
    let _ = dom.world_mut().insert_one(elem, style);
    let _ = dom.world_mut().insert_one(elem, layout);
    (dom, elem)
}

fn make_segment(text: &str) -> StyledTextSegment {
    StyledTextSegment {
        text: text.to_string(),
        color: elidex_plugin::CssColor::BLACK,
        font_family: vec!["serif".to_string()],
        font_size: 16.0,
        font_weight: 400,
        font_style: elidex_plugin::FontStyle::Normal,
        text_transform: elidex_plugin::TextTransform::None,
        text_decoration_line: elidex_plugin::TextDecorationLine::default(),
        text_decoration_style: elidex_plugin::TextDecorationStyle::Solid,
        text_decoration_color: None,
        letter_spacing: 0.0,
        word_spacing: 0.0,
        opacity: 1.0,
    }
}
