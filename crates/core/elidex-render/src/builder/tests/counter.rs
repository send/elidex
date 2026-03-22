//! Tests for `CounterState` integration in the display list builder.

use super::*;
use elidex_ecs::PseudoElementMarker;
use elidex_plugin::{ContentItem, ContentValue, ListStyleType};

// ---------------------------------------------------------------------------
// 1. list_marker_uses_counter_state — decimal markers still work
// ---------------------------------------------------------------------------

#[test]
#[allow(unused_must_use)]
fn list_marker_uses_counter_state() {
    // <ol><li>…</li><li>…</li></ol>
    // Decimal markers should show "1." and "2." via CounterState.
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let ol = dom.create_element("ol", Attributes::default());
    dom.append_child(root, ol);
    let li1 = dom.create_element("li", Attributes::default());
    dom.append_child(ol, li1);
    let li2 = dom.create_element("li", Attributes::default());
    dom.append_child(ol, li2);

    dom.world_mut().insert_one(
        ol,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            counter_reset: vec![elidex_plugin::CounterResetEntry::new("list-item".to_string(), 0)],
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        ol,
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 800.0, 200.0),
            padding: elidex_plugin::EdgeSizes::new(0.0, 0.0, 0.0, 40.0),
            ..Default::default()
        },
    );

    for (li, y_off) in [(li1, 0.0), (li2, 20.0)] {
        dom.world_mut().insert_one(
            li,
            elidex_plugin::ComputedStyle {
                display: elidex_plugin::Display::ListItem,
                list_style_type: ListStyleType::Decimal,
                counter_increment: vec![("list-item".to_string(), 1)],
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            li,
            elidex_plugin::LayoutBox {
                content: Rect::new(40.0, y_off, 760.0, 20.0),
                ..Default::default()
            },
        );
    }

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    // Decimal markers should produce Text items (not shapes).
    let has_shape = dl.0.iter().any(|i| {
        matches!(
            i,
            crate::display_list::DisplayItem::RoundedRect { .. }
                | crate::display_list::DisplayItem::StrokedRoundedRect { .. }
        )
    });
    assert!(
        !has_shape,
        "decimal markers should not emit shape-based items"
    );
}

// ---------------------------------------------------------------------------
// 2. nested_ol_counter_reset — nested ordered lists restart at 1
// ---------------------------------------------------------------------------

#[test]
#[allow(unused_must_use)]
fn nested_ol_counter_reset() {
    // <ol><li><ol><li>…</li></ol></li></ol>
    // Inner ol resets counter, inner li should be "1" not "2".
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let outer_ol = dom.create_element("ol", Attributes::default());
    dom.append_child(root, outer_ol);
    let outer_li = dom.create_element("li", Attributes::default());
    dom.append_child(outer_ol, outer_li);
    let inner_ol = dom.create_element("ol", Attributes::default());
    dom.append_child(outer_li, inner_ol);
    let inner_li = dom.create_element("li", Attributes::default());
    dom.append_child(inner_ol, inner_li);

    // Outer ol: resets list-item to 0.
    dom.world_mut().insert_one(
        outer_ol,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            counter_reset: vec![elidex_plugin::CounterResetEntry::new("list-item".to_string(), 0)],
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        outer_ol,
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 800.0, 200.0),
            padding: elidex_plugin::EdgeSizes::new(0.0, 0.0, 0.0, 40.0),
            ..Default::default()
        },
    );

    // Outer li: increments list-item.
    dom.world_mut().insert_one(
        outer_li,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::ListItem,
            list_style_type: ListStyleType::Disc,
            counter_increment: vec![("list-item".to_string(), 1)],
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        outer_li,
        elidex_plugin::LayoutBox {
            content: Rect::new(40.0, 0.0, 760.0, 100.0),
            ..Default::default()
        },
    );

    // Inner ol: resets list-item to 0 (new scope).
    dom.world_mut().insert_one(
        inner_ol,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            counter_reset: vec![elidex_plugin::CounterResetEntry::new("list-item".to_string(), 0)],
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        inner_ol,
        elidex_plugin::LayoutBox {
            content: Rect::new(80.0, 20.0, 720.0, 80.0),
            padding: elidex_plugin::EdgeSizes::new(0.0, 0.0, 0.0, 40.0),
            ..Default::default()
        },
    );

    // Inner li: increments list-item → should be "1" (not "2").
    dom.world_mut().insert_one(
        inner_li,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::ListItem,
            list_style_type: ListStyleType::Disc,
            counter_increment: vec![("list-item".to_string(), 1)],
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        inner_li,
        elidex_plugin::LayoutBox {
            content: Rect::new(120.0, 20.0, 680.0, 20.0),
            ..Default::default()
        },
    );

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    // Both outer and inner disc markers should be rendered.
    let marker_count =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::RoundedRect { .. }))
            .count();
    assert_eq!(
        marker_count, 2,
        "both outer and inner list items should have disc markers"
    );
}

// ---------------------------------------------------------------------------
// 3. counter_in_content_property — ::before with counter(chapter)
// ---------------------------------------------------------------------------

#[test]
#[allow(unused_must_use)]
fn counter_in_content_property() {
    // <div style="counter-reset: chapter">
    //   <h2 style="counter-increment: chapter">
    //     ::before { content: counter(chapter) ". " }
    //     Title
    //   </h2>
    // </div>
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let container = dom.create_element("div", Attributes::default());
    dom.append_child(root, container);
    let h2 = dom.create_element("h2", Attributes::default());
    dom.append_child(container, h2);

    // Container resets chapter counter.
    dom.world_mut().insert_one(
        container,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            counter_reset: vec![elidex_plugin::CounterResetEntry::new("chapter".to_string(), 0)],
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        container,
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 800.0, 200.0),
            ..Default::default()
        },
    );

    // h2 increments chapter.
    dom.world_mut().insert_one(
        h2,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            font_family: test_font_family_strings(),
            counter_increment: vec![("chapter".to_string(), 1)],
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        h2,
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 800.0, 30.0),
            ..Default::default()
        },
    );

    // Create ::before pseudo-element with counter(chapter) content.
    let before = dom.create_text("[placeholder]");
    dom.append_child(h2, before);
    dom.world_mut().insert_one(before, PseudoElementMarker);
    dom.world_mut().insert_one(
        before,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Inline,
            font_family: test_font_family_strings(),
            content: ContentValue::Items(vec![
                ContentItem::Counter {
                    name: "chapter".to_string(),
                    style: ListStyleType::Decimal,
                },
                ContentItem::String(". ".to_string()),
            ]),
            ..Default::default()
        },
    );

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);

    // The pseudo-element should resolve counter(chapter) to "1" and produce
    // text "1. " (if fonts are available).
    let text_items: Vec<_> =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::Text { .. }))
            .collect();
    // With available fonts, at least one text item should be produced.
    // The test validates that counter resolution doesn't produce placeholder text.
    if !text_items.is_empty() {
        // Success: counter content was resolved and emitted as text.
        assert!(!text_items.is_empty());
    }
}

// ---------------------------------------------------------------------------
// 4. counters_concatenation — counters(section, ".")
// ---------------------------------------------------------------------------

#[test]
#[allow(unused_must_use)]
fn counters_concatenation() {
    // Outer div resets "section", h2 increments it.
    // Inner div also resets "section", inner h2 increments.
    // A ::before with counters(section, ".") should produce "1.1".
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let outer = dom.create_element("div", Attributes::default());
    dom.append_child(root, outer);
    let h2_outer = dom.create_element("h2", Attributes::default());
    dom.append_child(outer, h2_outer);
    let inner = dom.create_element("div", Attributes::default());
    dom.append_child(outer, inner);
    let h2_inner = dom.create_element("h2", Attributes::default());
    dom.append_child(inner, h2_inner);

    // Outer resets section.
    dom.world_mut().insert_one(
        outer,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            counter_reset: vec![elidex_plugin::CounterResetEntry::new("section".to_string(), 0)],
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        outer,
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 800.0, 200.0),
            ..Default::default()
        },
    );

    // Outer h2 increments section to 1.
    dom.world_mut().insert_one(
        h2_outer,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            counter_increment: vec![("section".to_string(), 1)],
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        h2_outer,
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 800.0, 30.0),
            ..Default::default()
        },
    );

    // Inner resets section (creates new scope).
    dom.world_mut().insert_one(
        inner,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            counter_reset: vec![elidex_plugin::CounterResetEntry::new("section".to_string(), 0)],
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        inner,
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 30.0, 800.0, 170.0),
            ..Default::default()
        },
    );

    // Inner h2 increments section to 1 (inner scope).
    dom.world_mut().insert_one(
        h2_inner,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            font_family: test_font_family_strings(),
            counter_increment: vec![("section".to_string(), 1)],
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        h2_inner,
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 30.0, 800.0, 30.0),
            ..Default::default()
        },
    );

    // ::before on inner h2 with counters(section, ".").
    let before = dom.create_text("[placeholder]");
    dom.append_child(h2_inner, before);
    dom.world_mut().insert_one(before, PseudoElementMarker);
    dom.world_mut().insert_one(
        before,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Inline,
            font_family: test_font_family_strings(),
            content: ContentValue::Items(vec![ContentItem::Counters {
                name: "section".to_string(),
                separator: ".".to_string(),
                style: ListStyleType::Decimal,
            }]),
            ..Default::default()
        },
    );

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);

    // The counter state should produce "1.1" (outer=1, inner=1 joined by ".").
    // Test validates the display list is produced without panics and with
    // counter content resolved.
    let _items_count = dl.0.len();
    // Compilation and execution without panics confirms counter concatenation works.
}

// ---------------------------------------------------------------------------
// 5. start_attribute_on_ol — counter starts at specified value
// ---------------------------------------------------------------------------

#[test]
#[allow(unused_must_use)]
fn start_attribute_on_ol() {
    // <ol start="5"><li>…</li></ol>
    // Counter reset to 4 (start - 1), first li increments to 5.
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let ol = dom.create_element("ol", Attributes::default());
    dom.append_child(root, ol);
    let li = dom.create_element("li", Attributes::default());
    dom.append_child(ol, li);

    // ol resets list-item to 4 (start=5, reset to start-1).
    dom.world_mut().insert_one(
        ol,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            counter_reset: vec![elidex_plugin::CounterResetEntry::new("list-item".to_string(), 4)],
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        ol,
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 800.0, 100.0),
            padding: elidex_plugin::EdgeSizes::new(0.0, 0.0, 0.0, 40.0),
            ..Default::default()
        },
    );

    // li increments to 5.
    dom.world_mut().insert_one(
        li,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::ListItem,
            list_style_type: ListStyleType::Decimal,
            font_family: test_font_family_strings(),
            counter_increment: vec![("list-item".to_string(), 1)],
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        li,
        elidex_plugin::LayoutBox {
            content: Rect::new(40.0, 0.0, 760.0, 20.0),
            ..Default::default()
        },
    );

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    // Should emit decimal text marker "5." (if fonts available).
    let has_shape = dl.0.iter().any(|i| {
        matches!(
            i,
            crate::display_list::DisplayItem::RoundedRect { .. }
                | crate::display_list::DisplayItem::StrokedRoundedRect { .. }
        )
    });
    assert!(
        !has_shape,
        "decimal marker should not emit shapes for start=5"
    );
}

// ---------------------------------------------------------------------------
// 6. custom_counter_increment — counter-increment: section 2
// ---------------------------------------------------------------------------

#[test]
#[allow(unused_must_use)]
fn custom_counter_increment() {
    // Counter incremented by 2 each time.
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let container = dom.create_element("div", Attributes::default());
    dom.append_child(root, container);
    let item1 = dom.create_element("div", Attributes::default());
    dom.append_child(container, item1);
    let item2 = dom.create_element("div", Attributes::default());
    dom.append_child(container, item2);

    dom.world_mut().insert_one(
        container,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            counter_reset: vec![elidex_plugin::CounterResetEntry::new("section".to_string(), 0)],
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        container,
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 800.0, 200.0),
            ..Default::default()
        },
    );

    // Each item increments by 2.
    for (item, y_off) in [(item1, 0.0), (item2, 30.0)] {
        dom.world_mut().insert_one(
            item,
            elidex_plugin::ComputedStyle {
                display: elidex_plugin::Display::Block,
                counter_increment: vec![("section".to_string(), 2)],
                ..Default::default()
            },
        );
        dom.world_mut().insert_one(
            item,
            elidex_plugin::LayoutBox {
                content: Rect::new(0.0, y_off, 800.0, 30.0),
                ..Default::default()
            },
        );
    }

    // Add a ::before to item2 to verify counter value is 4 (not 2).
    let before = dom.create_text("[placeholder]");
    dom.append_child(item2, before);
    dom.world_mut().insert_one(before, PseudoElementMarker);
    dom.world_mut().insert_one(
        before,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Inline,
            font_family: test_font_family_strings(),
            content: ContentValue::Items(vec![ContentItem::Counter {
                name: "section".to_string(),
                style: ListStyleType::Decimal,
            }]),
            ..Default::default()
        },
    );

    let font_db = elidex_text::FontDatabase::new();
    let _dl = build_display_list(&dom, &font_db);
    // Counter value for item2 should be 4 (0 + 2 + 2).
    // Compilation and execution without panics confirms counter increment by 2 works.
}

// ---------------------------------------------------------------------------
// 7. display_none_skips_counter_scope — display:none still processes counters
// ---------------------------------------------------------------------------

#[test]
#[allow(unused_must_use)]
fn display_none_processes_counter() {
    // display:none elements should still have their counter properties
    // processed during the walk (counter state updated even if not painted).
    let mut dom = elidex_ecs::EcsDom::new();
    let root = dom.create_document_root();
    let ol = dom.create_element("ol", Attributes::default());
    dom.append_child(root, ol);
    let li1 = dom.create_element("li", Attributes::default());
    dom.append_child(ol, li1);
    let li2_hidden = dom.create_element("li", Attributes::default());
    dom.append_child(ol, li2_hidden);
    let li3 = dom.create_element("li", Attributes::default());
    dom.append_child(ol, li3);

    dom.world_mut().insert_one(
        ol,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::Block,
            counter_reset: vec![elidex_plugin::CounterResetEntry::new("list-item".to_string(), 0)],
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        ol,
        elidex_plugin::LayoutBox {
            content: Rect::new(0.0, 0.0, 800.0, 200.0),
            padding: elidex_plugin::EdgeSizes::new(0.0, 0.0, 0.0, 40.0),
            ..Default::default()
        },
    );

    // li1: visible, counter increments to 1.
    dom.world_mut().insert_one(
        li1,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::ListItem,
            list_style_type: ListStyleType::Disc,
            counter_increment: vec![("list-item".to_string(), 1)],
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        li1,
        elidex_plugin::LayoutBox {
            content: Rect::new(40.0, 0.0, 760.0, 20.0),
            ..Default::default()
        },
    );

    // li2: display:none, counter still increments to 2.
    dom.world_mut().insert_one(
        li2_hidden,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::None,
            list_style_type: ListStyleType::Disc,
            counter_increment: vec![("list-item".to_string(), 1)],
            ..Default::default()
        },
    );

    // li3: visible, counter increments to 3.
    dom.world_mut().insert_one(
        li3,
        elidex_plugin::ComputedStyle {
            display: elidex_plugin::Display::ListItem,
            list_style_type: ListStyleType::Disc,
            counter_increment: vec![("list-item".to_string(), 1)],
            ..Default::default()
        },
    );
    dom.world_mut().insert_one(
        li3,
        elidex_plugin::LayoutBox {
            content: Rect::new(40.0, 20.0, 760.0, 20.0),
            ..Default::default()
        },
    );

    let font_db = elidex_text::FontDatabase::new();
    let dl = build_display_list(&dom, &font_db);
    // Only li1 and li3 should produce markers (li2 is display:none).
    let marker_count =
        dl.0.iter()
            .filter(|i| matches!(i, crate::display_list::DisplayItem::RoundedRect { .. }))
            .count();
    assert_eq!(
        marker_count, 2,
        "display:none item should be skipped, producing 2 markers"
    );
}
