//! Display list builder: converts a laid-out DOM into paint commands.
//!
//! Walks the DOM tree in pre-order (painter's order) and emits
//! [`DisplayItem`]s for background rectangles and text content.

use elidex_ecs::{EcsDom, Entity, TextContent};
use elidex_plugin::{ComputedStyle, CssColor, Display, LayoutBox};
use elidex_text::{shape_text, FontDatabase};

use crate::display_list::{DisplayItem, DisplayList, GlyphEntry};
use crate::font_cache::FontCache;

/// Build a display list from a laid-out DOM tree.
///
/// Each element with a [`LayoutBox`] component is visited in pre-order.
/// Background colors produce [`DisplayItem::SolidRect`] entries; text
/// nodes produce [`DisplayItem::Text`] entries via re-shaping.
///
/// # Prerequisites
///
/// `elidex_layout::layout_tree()` must have been called first so that
/// every visible element has a [`LayoutBox`] component.
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

    // Check if this entity is a text node.
    // Clone and drop the ECS borrow before calling emit_text, which borrows
    // other components (ComputedStyle, LayoutBox) from the same world.
    let text_owned = dom
        .world()
        .get::<&TextContent>(entity)
        .ok()
        .map(|tc| tc.0.clone());
    if let Some(ref text) = text_owned {
        if let Some(parent) = dom.get_parent(entity) {
            emit_text(dom, parent, text, font_db, font_cache, dl);
        }
    }

    // Recurse into children.
    for child in dom.children_iter(entity) {
        walk(dom, child, font_db, font_cache, dl);
    }
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
    color: CssColor,
    content_x: f32,
    content_y: f32,
}

/// Gather parent style and layout info needed for text rendering.
///
/// Returns `None` if the parent lacks the required components.
fn text_context(dom: &EcsDom, parent: Entity) -> Option<TextContext> {
    let style = dom.world().get::<&ComputedStyle>(parent).ok()?;
    let parent_lb = dom.world().get::<&LayoutBox>(parent).ok()?;
    Some(TextContext {
        font_family: style.font_family.clone(),
        font_size: style.font_size,
        color: style.color,
        content_x: parent_lb.content.x,
        content_y: parent_lb.content.y,
    })
}

/// Emit `Text` display items for a text node.
///
/// Uses the parent element's `ComputedStyle` for font properties and
/// `LayoutBox` for position. Text is re-shaped to obtain glyph IDs and
/// positions.
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

    let families: Vec<&str> = ctx.font_family.iter().map(String::as_str).collect();
    let font_size = ctx.font_size;
    let Some(font_id) = font_db.query(&families) else {
        return;
    };
    let Some(shaped) = shape_text(font_db, font_id, font_size, text) else {
        return;
    };
    let Some((font_blob, font_index)) = font_cache.get(font_db, font_id) else {
        return;
    };

    // Get font metrics for baseline positioning.
    let ascent = font_db
        .font_metrics(font_id, font_size)
        .map_or(font_size, |m| m.ascent);

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

    dl.push(DisplayItem::Text {
        glyphs,
        font_blob,
        font_index,
        font_size,
        color: ctx.color,
    });
}

#[cfg(test)]
#[allow(unused_must_use)]
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
        if font_db.query(&families_ref).is_none() {
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
}
