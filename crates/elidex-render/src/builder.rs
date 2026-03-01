//! Display list builder: converts a laid-out DOM into paint commands.
//!
//! Walks the DOM tree in pre-order (painter's order) and emits
//! [`DisplayItem`]s for background rectangles and text content.
//!
//! Text processing follows CSS `white-space: normal` rules: newlines
//! and tabs are replaced with spaces, and runs of spaces are collapsed
//! to a single space. Whitespace-only text is discarded.

use std::borrow::Cow;

use elidex_ecs::{EcsDom, Entity, TextContent};
use elidex_plugin::{
    ComputedStyle, CssColor, Display, LayoutBox, LineHeight, TextDecorationLine, TextTransform,
};
use elidex_text::{shape_text, FontDatabase};

use crate::display_list::{DisplayItem, DisplayList, GlyphEntry};
use crate::font_cache::FontCache;

/// Build a display list from a laid-out DOM tree.
///
/// Each element with a [`LayoutBox`] component is visited in pre-order.
/// Background colors produce [`DisplayItem::SolidRect`] entries; text
/// nodes produce [`DisplayItem::Text`] entries via re-shaping.
///
/// Children of each element are processed in "inline runs": consecutive
/// non-block children (text nodes and inline elements) have their text
/// collected, whitespace-collapsed, and rendered as a single text item.
/// This avoids position overlap when multiple text nodes share the same
/// block ancestor.
///
/// # Prerequisites
///
/// `elidex_layout::layout_tree()` must have been called first so that
/// every visible element has a [`LayoutBox`] component.
#[must_use]
pub fn build_display_list(dom: &EcsDom, font_db: &FontDatabase) -> DisplayList {
    let mut dl = DisplayList::default();
    let mut font_cache = FontCache::new();

    let roots = find_roots(dom);
    for root in roots {
        walk(dom, root, font_db, &mut font_cache, &mut dl);
    }

    dl
}

/// Find root entities for rendering: parentless entities with layout or children.
fn find_roots(dom: &EcsDom) -> Vec<Entity> {
    dom.root_entities()
        .into_iter()
        .filter(|&e| dom.world().get::<&LayoutBox>(e).is_ok() || dom.get_first_child(e).is_some())
        .collect()
}

/// Pre-order walk: emit paint commands for this entity, then recurse.
///
/// Children are grouped into "inline runs" (consecutive non-block children)
/// and "block children" (those with a `LayoutBox`). Inline runs have their
/// text collected and rendered as a single item; block children are
/// recursed into normally.
fn walk(
    dom: &EcsDom,
    entity: Entity,
    font_db: &FontDatabase,
    font_cache: &mut FontCache,
    dl: &mut DisplayList,
) {
    // Check for display: none — skip this subtree entirely.
    // This check is independent of LayoutBox: an element without a LayoutBox
    // but with display:none should still be skipped.
    if let Ok(style) = dom.world().get::<&ComputedStyle>(entity) {
        if style.display == Display::None {
            return;
        }
    }

    // Emit background color for elements with a LayoutBox.
    if let Ok(lb) = dom.world().get::<&LayoutBox>(entity) {
        if let Ok(style) = dom.world().get::<&ComputedStyle>(entity) {
            emit_background(&lb, style.background_color, dl);
        }
    }

    // Process children in inline runs vs block children.
    let children: Vec<Entity> = dom.children_iter(entity).collect();
    let mut inline_run = Vec::new();

    for &child in &children {
        if is_block_child(dom, child) {
            // Flush any pending inline run before the block child.
            if !inline_run.is_empty() {
                emit_inline_run(dom, entity, &inline_run, font_db, font_cache, dl);
                inline_run.clear();
            }
            // Recurse into block child.
            walk(dom, child, font_db, font_cache, dl);
        } else {
            // Text node or inline element — add to current run.
            inline_run.push(child);
        }
    }

    // Flush trailing inline run.
    if !inline_run.is_empty() {
        emit_inline_run(dom, entity, &inline_run, font_db, font_cache, dl);
    }
}

/// Check whether a child entity is a block-level child (has a `LayoutBox`).
///
/// Block children are recursed into separately; non-block children (text
/// nodes and inline elements) are collected into inline runs.
fn is_block_child(dom: &EcsDom, entity: Entity) -> bool {
    dom.world().get::<&LayoutBox>(entity).is_ok()
}

/// Collect text from an inline run and render it as a single text item.
///
/// An inline run is a sequence of non-block children (text nodes and
/// inline elements). Text is collected recursively from inline elements,
/// whitespace-collapsed per CSS `white-space: normal`, and rendered at
/// the parent's content position.
fn emit_inline_run(
    dom: &EcsDom,
    parent: Entity,
    run: &[Entity],
    font_db: &FontDatabase,
    font_cache: &mut FontCache,
    dl: &mut DisplayList,
) {
    let raw_text = collect_inline_text(dom, run);
    let text = collapse_whitespace(&raw_text);
    if text.is_empty() {
        return;
    }

    emit_text(dom, parent, &text, font_db, font_cache, dl);
}

/// Recursively collect text content from a list of inline entities.
///
/// Text nodes contribute their content directly. Inline elements (those
/// with `ComputedStyle` but no `LayoutBox`) contribute their children's
/// text, flattened. `display: none` elements are skipped.
fn collect_inline_text(dom: &EcsDom, entities: &[Entity]) -> String {
    collect_inline_text_inner(dom, entities, 0)
}

/// Maximum recursion depth for inline text collection.
const MAX_INLINE_DEPTH: u32 = 100;

fn collect_inline_text_inner(dom: &EcsDom, entities: &[Entity], depth: u32) -> String {
    if depth >= MAX_INLINE_DEPTH {
        return String::new();
    }
    let mut text = String::new();
    for &entity in entities {
        // Check for display: none on elements.
        if let Ok(style) = dom.world().get::<&ComputedStyle>(entity) {
            if style.display == Display::None {
                continue;
            }
        }

        // Text node: contribute content directly.
        if let Ok(tc) = dom.world().get::<&TextContent>(entity) {
            text.push_str(&tc.0);
            continue;
        }

        // Inline element: recurse into children.
        let children: Vec<Entity> = dom.children_iter(entity).collect();
        text.push_str(&collect_inline_text_inner(dom, &children, depth + 1));
    }
    text
}

/// Collapse whitespace per CSS `white-space: normal` rules.
///
/// Replaces newlines, carriage returns, and tabs with spaces, then
/// collapses runs of consecutive spaces to a single space. Strips
/// leading and trailing whitespace (inter-element whitespace).
fn collapse_whitespace(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_was_space = false;
    for ch in text.chars() {
        if ch == '\n' || ch == '\r' || ch == '\t' || ch == ' ' {
            if !prev_was_space {
                result.push(' ');
                prev_was_space = true;
            }
        } else {
            result.push(ch);
            prev_was_space = false;
        }
    }

    // Trim leading/trailing whitespace (handles inter-element whitespace).
    let trimmed = result.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed.to_string()
}

/// Emit a `SolidRect` for the element's background color.
fn emit_background(lb: &LayoutBox, bg: CssColor, dl: &mut DisplayList) {
    if bg.a == 0 {
        return; // transparent
    }
    dl.push(DisplayItem::SolidRect {
        rect: lb.border_box(),
        color: bg,
    });
}

/// Parent style and layout info needed for text rendering.
struct TextContext {
    font_family: Vec<String>,
    font_size: f32,
    font_weight: u16,
    /// CSS line-height (for future multi-line text rendering).
    #[allow(dead_code)]
    line_height: LineHeight,
    color: CssColor,
    content_x: f32,
    content_y: f32,
    text_transform: TextTransform,
    text_decoration_line: TextDecorationLine,
}

/// Gather style and layout info from `parent` for text rendering.
///
/// Font properties (family, size, color) come from `parent`'s
/// [`ComputedStyle`]. Position comes from the nearest ancestor with
/// a [`LayoutBox`] — which may be `parent` itself (block element) or
/// a further-up block ancestor when `parent` is an inline element
/// that has no `LayoutBox` in Phase 2.
fn text_context(dom: &EcsDom, parent: Entity) -> Option<TextContext> {
    let style = dom.world().get::<&ComputedStyle>(parent).ok()?;
    let lb = find_nearest_layout_box(dom, parent)?;

    Some(TextContext {
        font_family: style.font_family.clone(),
        font_size: style.font_size,
        font_weight: style.font_weight,
        line_height: style.line_height,
        color: style.color,
        content_x: lb.content.x,
        content_y: lb.content.y,
        text_transform: style.text_transform,
        text_decoration_line: style.text_decoration_line,
    })
}

/// Walk up the ancestor chain to find the nearest entity with a `LayoutBox`.
///
/// Starts with `entity` itself, then checks its parent, grandparent, etc.
/// Returns `None` if no ancestor has a `LayoutBox` (capped at 1000 depth).
fn find_nearest_layout_box(dom: &EcsDom, entity: Entity) -> Option<LayoutBox> {
    let mut current = entity;
    for _ in 0..1000 {
        if let Ok(lb) = dom.world().get::<&LayoutBox>(current) {
            return Some((*lb).clone());
        }
        current = dom.get_parent(current)?;
    }
    None
}

/// Emit a `Text` display item.
///
/// Uses `parent`'s `ComputedStyle` for font properties and nearest
/// ancestor's `LayoutBox` for position. Text is shaped to obtain glyph
/// IDs and positions.
fn emit_text(
    dom: &EcsDom,
    parent: Entity,
    text: &str,
    font_db: &FontDatabase,
    font_cache: &mut FontCache,
    dl: &mut DisplayList,
) {
    if text.is_empty() {
        return;
    }

    let Some(ctx) = text_context(dom, parent) else {
        return;
    };

    // Apply text-transform before shaping.
    let transformed = apply_text_transform(text, ctx.text_transform);
    let text: &str = &transformed;

    let families: Vec<&str> = ctx.font_family.iter().map(String::as_str).collect();
    let font_size = ctx.font_size;
    let Some(font_id) = font_db.query(&families, ctx.font_weight) else {
        return;
    };
    let Some(shaped) = shape_text(font_db, font_id, font_size, text) else {
        return;
    };
    let Some((font_blob, font_index)) = font_cache.get(font_db, font_id) else {
        return;
    };

    // Get font metrics for baseline positioning.
    let metrics = font_db.font_metrics(font_id, font_size);
    let ascent = metrics.map_or(font_size, |m| m.ascent);
    let descent = metrics.map_or(-font_size * 0.25, |m| m.descent);

    let baseline_y = ctx.content_y + ascent;
    let mut cursor_x = ctx.content_x;
    let mut glyphs = Vec::with_capacity(shaped.glyphs.len());

    for glyph in &shaped.glyphs {
        let x = cursor_x + glyph.x_offset;
        let y = baseline_y - glyph.y_offset;
        glyphs.push(GlyphEntry {
            glyph_id: u32::from(glyph.glyph_id),
            x,
            y,
        });
        cursor_x += glyph.x_advance;
    }

    let text_width = cursor_x - ctx.content_x;

    dl.push(DisplayItem::Text {
        glyphs,
        font_blob,
        font_index,
        font_size,
        color: ctx.color,
    });

    // Emit text decoration (underline / line-through) as SolidRect items.
    let decoration_thickness = (font_size / 16.0).max(1.0);
    if ctx.text_decoration_line.underline {
        // Position underline just below the baseline.
        let y = baseline_y - descent * 0.5;
        dl.push(DisplayItem::SolidRect {
            rect: elidex_plugin::Rect {
                x: ctx.content_x,
                y,
                width: text_width,
                height: decoration_thickness,
            },
            color: ctx.color,
        });
    }
    if ctx.text_decoration_line.line_through {
        // Position line-through at approximately half x-height (midpoint of ascent).
        let y = baseline_y - ascent * 0.4;
        dl.push(DisplayItem::SolidRect {
            rect: elidex_plugin::Rect {
                x: ctx.content_x,
                y,
                width: text_width,
                height: decoration_thickness,
            },
            color: ctx.color,
        });
    }
}

/// Apply CSS `text-transform` to a string before shaping.
fn apply_text_transform(text: &str, transform: TextTransform) -> Cow<'_, str> {
    match transform {
        TextTransform::None => Cow::Borrowed(text),
        TextTransform::Uppercase => Cow::Owned(text.to_uppercase()),
        TextTransform::Lowercase => Cow::Owned(text.to_lowercase()),
        TextTransform::Capitalize => Cow::Owned(capitalize_words(text)),
    }
}

/// Capitalize the first letter of each word (whitespace-delimited).
fn capitalize_words(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_was_whitespace = true;
    for ch in text.chars() {
        if prev_was_whitespace && ch.is_alphabetic() {
            for upper in ch.to_uppercase() {
                result.push(upper);
            }
            prev_was_whitespace = false;
        } else {
            result.push(ch);
            prev_was_whitespace = ch.is_whitespace();
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;
    use elidex_plugin::{EdgeSizes, Rect};

    #[test]
    fn empty_dom_empty_display_list() {
        let dom = EcsDom::new();
        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        assert!(dl.0.is_empty());
    }

    #[test]
    #[allow(unused_must_use)]
    fn background_color_emits_solid_rect() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(root, div);

        dom.world_mut().insert_one(
            div,
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::RED,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            div,
            LayoutBox {
                content: Rect {
                    x: 10.0,
                    y: 10.0,
                    width: 100.0,
                    height: 50.0,
                },
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        assert_eq!(dl.0.len(), 1);
        let DisplayItem::SolidRect { rect, color } = &dl.0[0] else {
            panic!("expected SolidRect");
        };
        assert_eq!(*color, CssColor::RED);
        assert!((rect.width - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    #[allow(unused_must_use)]
    fn transparent_background_no_item() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(root, div);

        dom.world_mut().insert_one(
            div,
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::TRANSPARENT,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            div,
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 50.0,
                },
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        assert!(dl.0.is_empty());
    }

    #[test]
    #[allow(unused_must_use)]
    fn text_node_emits_text_item() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let div = dom.create_element("div", Attributes::default());
        let text = dom.create_text("Hello");
        dom.append_child(root, div);
        dom.append_child(div, text);

        let test_families = vec![
            "Arial".to_string(),
            "Helvetica".to_string(),
            "Liberation Sans".to_string(),
            "DejaVu Sans".to_string(),
            "Noto Sans".to_string(),
            "Hiragino Sans".to_string(),
        ];

        dom.world_mut().insert_one(
            div,
            ComputedStyle {
                display: Display::Block,
                font_family: test_families,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            div,
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();

        // Early return if no font available (CI)
        let families_ref: Vec<&str> = vec![
            "Arial",
            "Helvetica",
            "Liberation Sans",
            "DejaVu Sans",
            "Noto Sans",
            "Hiragino Sans",
        ];
        if font_db.query(&families_ref, 400).is_none() {
            return;
        }

        let dl = build_display_list(&dom, &font_db);
        let text_items: Vec<_> =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::Text { .. }))
                .collect();
        assert_eq!(text_items.len(), 1);
        let DisplayItem::Text {
            glyphs, font_size, ..
        } = &text_items[0]
        else {
            unreachable!();
        };
        assert_eq!(glyphs.len(), 5); // "Hello" = 5 glyphs
        assert!((*font_size - 16.0).abs() < f32::EPSILON);
    }

    #[test]
    #[allow(unused_must_use)]
    fn nested_elements_painter_order() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let outer = dom.create_element("div", Attributes::default());
        let inner = dom.create_element("div", Attributes::default());
        dom.append_child(root, outer);
        dom.append_child(outer, inner);

        dom.world_mut().insert_one(
            outer,
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::RED,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            outer,
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 200.0,
                    height: 100.0,
                },
                ..Default::default()
            },
        );

        dom.world_mut().insert_one(
            inner,
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::BLUE,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            inner,
            LayoutBox {
                content: Rect {
                    x: 10.0,
                    y: 10.0,
                    width: 180.0,
                    height: 80.0,
                },
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);

        // Painter's order: outer first, inner second.
        assert_eq!(dl.0.len(), 2);
        match (&dl.0[0], &dl.0[1]) {
            (
                DisplayItem::SolidRect {
                    color: c1,
                    rect: r1,
                },
                DisplayItem::SolidRect {
                    color: c2,
                    rect: r2,
                },
            ) => {
                assert_eq!(*c1, CssColor::RED);
                assert_eq!(*c2, CssColor::BLUE);
                assert!((r1.width - 200.0).abs() < f32::EPSILON);
                assert!((r2.width - 180.0).abs() < f32::EPSILON);
            }
            _ => panic!("expected two SolidRects"),
        }
    }

    #[test]
    #[allow(unused_must_use)]
    fn display_none_skipped() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let visible = dom.create_element("div", Attributes::default());
        let hidden = dom.create_element("div", Attributes::default());
        dom.append_child(root, visible);
        dom.append_child(root, hidden);

        dom.world_mut().insert_one(
            visible,
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::RED,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            visible,
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 50.0,
                },
                ..Default::default()
            },
        );

        dom.world_mut().insert_one(
            hidden,
            ComputedStyle {
                display: Display::None,
                background_color: CssColor::BLUE,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            hidden,
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 50.0,
                    width: 100.0,
                    height: 50.0,
                },
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        assert_eq!(dl.0.len(), 1);
        let DisplayItem::SolidRect { color, .. } = &dl.0[0] else {
            panic!("expected SolidRect");
        };
        assert_eq!(*color, CssColor::RED);
    }

    #[test]
    #[allow(unused_must_use)]
    fn background_uses_border_box() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(root, div);

        dom.world_mut().insert_one(
            div,
            ComputedStyle {
                display: Display::Block,
                background_color: CssColor::GREEN,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            div,
            LayoutBox {
                content: Rect {
                    x: 20.0,
                    y: 20.0,
                    width: 100.0,
                    height: 50.0,
                },
                padding: EdgeSizes {
                    top: 5.0,
                    right: 5.0,
                    bottom: 5.0,
                    left: 5.0,
                },
                border: EdgeSizes {
                    top: 2.0,
                    right: 2.0,
                    bottom: 2.0,
                    left: 2.0,
                },
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        assert_eq!(dl.0.len(), 1);
        let DisplayItem::SolidRect { rect, .. } = &dl.0[0] else {
            panic!("expected SolidRect");
        };
        // border box: x = 20 - 5 - 2 = 13, width = 100 + 10 + 4 = 114
        assert!((rect.x - 13.0).abs() < f32::EPSILON);
        assert!((rect.y - 13.0).abs() < f32::EPSILON);
        assert!((rect.width - 114.0).abs() < f32::EPSILON);
        assert!((rect.height - 64.0).abs() < f32::EPSILON);
    }

    // --- Whitespace processing tests ---

    #[test]
    fn collapse_whitespace_newlines_and_tabs() {
        assert_eq!(collapse_whitespace("hello\n  world"), "hello world");
        assert_eq!(collapse_whitespace("a\t\tb"), "a b");
        assert_eq!(collapse_whitespace("a\r\nb"), "a b");
    }

    #[test]
    fn collapse_whitespace_multiple_spaces() {
        assert_eq!(collapse_whitespace("hello   world"), "hello world");
    }

    #[test]
    fn collapse_whitespace_trims() {
        assert_eq!(collapse_whitespace("  hello  "), "hello");
        assert_eq!(collapse_whitespace("\n  \n"), "");
    }

    #[test]
    fn collapse_whitespace_only() {
        assert_eq!(collapse_whitespace("   "), "");
        assert_eq!(collapse_whitespace("\n\t\r "), "");
    }

    #[test]
    fn collapse_whitespace_preserves_content() {
        assert_eq!(collapse_whitespace("hello"), "hello");
        assert_eq!(collapse_whitespace("hello world"), "hello world");
    }

    #[test]
    #[allow(unused_must_use)]
    fn whitespace_only_text_node_skipped() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let div = dom.create_element("div", Attributes::default());
        let ws = dom.create_text("   \n   ");
        dom.append_child(root, div);
        dom.append_child(div, ws);

        dom.world_mut().insert_one(
            div,
            ComputedStyle {
                display: Display::Block,
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            div,
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let dl = build_display_list(&dom, &font_db);
        // Whitespace-only text should produce no display items.
        assert!(dl.0.is_empty());
    }

    #[test]
    #[allow(unused_must_use)]
    fn inline_elements_text_collected() {
        // <p>Hello <strong>world</strong>!</p>
        // Should produce a single "Hello world!" text item.
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let p = dom.create_element("p", Attributes::default());
        let t1 = dom.create_text("Hello ");
        let strong = dom.create_element("strong", Attributes::default());
        let t2 = dom.create_text("world");
        let t3 = dom.create_text("!");
        dom.append_child(root, p);
        dom.append_child(p, t1);
        dom.append_child(p, strong);
        dom.append_child(strong, t2);
        dom.append_child(p, t3);

        let test_families = vec![
            "Arial".to_string(),
            "Helvetica".to_string(),
            "Liberation Sans".to_string(),
            "DejaVu Sans".to_string(),
            "Noto Sans".to_string(),
            "Hiragino Sans".to_string(),
        ];

        dom.world_mut().insert_one(
            p,
            ComputedStyle {
                display: Display::Block,
                font_family: test_families.clone(),
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            p,
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );
        // strong is inline — no LayoutBox, but has ComputedStyle.
        dom.world_mut().insert_one(
            strong,
            ComputedStyle {
                display: Display::Inline,
                font_family: test_families,
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let families_ref: Vec<&str> = vec![
            "Arial",
            "Helvetica",
            "Liberation Sans",
            "DejaVu Sans",
            "Noto Sans",
            "Hiragino Sans",
        ];
        if font_db.query(&families_ref, 400).is_none() {
            return;
        }

        let dl = build_display_list(&dom, &font_db);
        let text_items: Vec<_> =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::Text { .. }))
                .collect();
        // Should be a single text item (not three overlapping ones).
        assert_eq!(text_items.len(), 1);
        let DisplayItem::Text { glyphs, .. } = &text_items[0] else {
            unreachable!();
        };
        // "Hello world!" = 12 glyphs.
        assert_eq!(glyphs.len(), 12);
    }

    // --- M3-1: text-transform tests ---

    #[test]
    fn apply_text_transform_uppercase() {
        assert_eq!(
            super::apply_text_transform("hello", TextTransform::Uppercase),
            "HELLO"
        );
    }

    #[test]
    fn apply_text_transform_lowercase() {
        assert_eq!(
            super::apply_text_transform("HELLO", TextTransform::Lowercase),
            "hello"
        );
    }

    #[test]
    fn apply_text_transform_capitalize() {
        assert_eq!(
            super::apply_text_transform("hello world", TextTransform::Capitalize),
            "Hello World"
        );
    }

    #[test]
    fn apply_text_transform_none() {
        assert_eq!(
            super::apply_text_transform("Hello", TextTransform::None),
            "Hello"
        );
    }

    #[test]
    #[allow(unused_must_use)]
    fn text_decoration_underline_emits_solid_rect() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let div = dom.create_element("div", Attributes::default());
        let text = dom.create_text("Hello");
        dom.append_child(root, div);
        dom.append_child(div, text);

        let test_families = vec![
            "Arial".to_string(),
            "Helvetica".to_string(),
            "Liberation Sans".to_string(),
            "DejaVu Sans".to_string(),
            "Noto Sans".to_string(),
            "Hiragino Sans".to_string(),
        ];

        dom.world_mut().insert_one(
            div,
            ComputedStyle {
                display: Display::Block,
                font_family: test_families,
                text_decoration_line: TextDecorationLine {
                    underline: true,
                    line_through: false,
                },
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            div,
            LayoutBox {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 20.0,
                },
                ..Default::default()
            },
        );

        let font_db = FontDatabase::new();
        let families_ref: Vec<&str> = vec![
            "Arial",
            "Helvetica",
            "Liberation Sans",
            "DejaVu Sans",
            "Noto Sans",
            "Hiragino Sans",
        ];
        if font_db.query(&families_ref, 400).is_none() {
            return;
        }

        let dl = build_display_list(&dom, &font_db);
        // Should have: Text item + SolidRect for underline.
        let text_count =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::Text { .. }))
                .count();
        let rect_count =
            dl.0.iter()
                .filter(|i| matches!(i, DisplayItem::SolidRect { .. }))
                .count();
        assert_eq!(text_count, 1);
        // At least 1 rect for underline (no background since transparent).
        assert!(rect_count >= 1, "expected underline rect, got {rect_count}");
    }
}
