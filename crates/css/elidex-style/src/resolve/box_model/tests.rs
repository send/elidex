use std::collections::HashMap;

use elidex_plugin::{
    BorderCollapse, BoxSizing, CaptionSide, ComputedStyle, CssColor, CssValue, Dimension, Display,
    LengthUnit, Overflow, Position, TableLayout,
};

use crate::resolve::helpers::PropertyMap;
use crate::resolve::{build_computed_style, ResolveContext};

fn default_ctx() -> ResolveContext {
    ResolveContext {
        viewport_width: 1920.0,
        viewport_height: 1080.0,
        em_base: 16.0,
        root_font_size: 16.0,
    }
}

// --- Border width/style interaction ---

#[test]
fn border_width_zero_when_style_none() {
    let parent = ComputedStyle::default();
    let ctx = default_ctx();
    let mut winners: PropertyMap = HashMap::new();
    let width = CssValue::Length(5.0, LengthUnit::Px);
    winners.insert("border-top-width", &width);
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.border_top.width, 0.0);
}

#[test]
fn border_width_preserved_when_style_solid() {
    let parent = ComputedStyle::default();
    let ctx = default_ctx();
    let mut winners: PropertyMap = HashMap::new();
    let width = CssValue::Length(5.0, LengthUnit::Px);
    let solid = CssValue::Keyword("solid".to_string());
    winners.insert("border-top-width", &width);
    winners.insert("border-top-style", &solid);
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.border_top.width, 5.0);
}

// --- Display, position, overflow ---

#[test]
fn position_defaults_to_static() {
    let parent = ComputedStyle::default();
    let winners: PropertyMap = HashMap::new();
    let ctx = default_ctx();
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.position, Position::Static);
}

#[test]
fn overflow_defaults_to_visible() {
    let parent = ComputedStyle::default();
    let winners: PropertyMap = HashMap::new();
    let ctx = default_ctx();
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.overflow, Overflow::Visible);
}

// --- Box model ---

#[test]
fn build_style_resolves_padding() {
    let parent = ComputedStyle::default();
    let padding = CssValue::Length(10.0, LengthUnit::Px);
    let mut winners: PropertyMap = HashMap::new();
    winners.insert("padding-top", &padding);
    let ctx = default_ctx();
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.padding.top, 10.0);
}

#[test]
fn inherit_keyword_returns_parent_value() {
    let parent = ComputedStyle {
        color: CssColor::RED,
        ..ComputedStyle::default()
    };
    let inherit = CssValue::Inherit;
    let mut winners: PropertyMap = HashMap::new();
    winners.insert("color", &inherit);
    let ctx = default_ctx();
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.color, CssColor::RED);
}

// --- M3-2: Box model resolution ---

#[test]
fn resolve_box_sizing_border_box() {
    let parent = ComputedStyle::default();
    let ctx = default_ctx();
    let mut winners: PropertyMap = HashMap::new();
    let val = CssValue::Keyword("border-box".to_string());
    winners.insert("box-sizing", &val);
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.box_sizing, BoxSizing::BorderBox);
}

#[test]
fn resolve_box_sizing_not_inherited() {
    let parent = ComputedStyle {
        box_sizing: BoxSizing::BorderBox,
        ..ComputedStyle::default()
    };
    let ctx = default_ctx();
    let winners: PropertyMap = HashMap::new();
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.box_sizing, BoxSizing::ContentBox);
}

#[test]
fn resolve_border_radius_px() {
    let parent = ComputedStyle::default();
    let ctx = default_ctx();
    let mut winners: PropertyMap = HashMap::new();
    let val = CssValue::Length(8.0, LengthUnit::Px);
    winners.insert("border-radius", &val);
    let style = build_computed_style(&winners, &parent, &ctx);
    assert!((style.border_radius - 8.0).abs() < f32::EPSILON);
}

#[test]
fn resolve_opacity() {
    for (input, expected) in [(0.5, 0.5), (2.0, 1.0), (-0.5, 0.0)] {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let val = CssValue::Number(input);
        winners.insert("opacity", &val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!(
            (style.opacity - expected).abs() < f32::EPSILON,
            "opacity {input} -> {expected}, got {}",
            style.opacity
        );
    }
}

#[test]
fn get_computed_box_model_properties() {
    let style = ComputedStyle {
        box_sizing: BoxSizing::BorderBox,
        border_radius: 10.0,
        opacity: 0.75,
        ..ComputedStyle::default()
    };
    assert_eq!(
        crate::get_computed("box-sizing", &style),
        CssValue::Keyword("border-box".to_string())
    );
    assert_eq!(
        crate::get_computed("border-radius", &style),
        CssValue::Length(10.0, LengthUnit::Px)
    );
    assert_eq!(
        crate::get_computed("opacity", &style),
        CssValue::Number(0.75)
    );
}

// --- M3-5: gap resolution ---

#[test]
fn resolve_row_gap_px() {
    let ctx = default_ctx();
    let val = CssValue::Length(10.0, LengthUnit::Px);
    let mut winners: PropertyMap = HashMap::new();
    winners.insert("row-gap", &val);
    let parent = ComputedStyle::default();
    let style = build_computed_style(&winners, &parent, &ctx);
    assert!((style.row_gap - 10.0).abs() < f32::EPSILON);
}

#[test]
fn resolve_column_gap_negative_clamped() {
    let ctx = default_ctx();
    let val = CssValue::Length(-5.0, LengthUnit::Px);
    let mut winners: PropertyMap = HashMap::new();
    winners.insert("column-gap", &val);
    let parent = ComputedStyle::default();
    let style = build_computed_style(&winners, &parent, &ctx);
    assert!((style.column_gap).abs() < f32::EPSILON);
}

#[test]
fn resolve_gap_computed_value() {
    let style = ComputedStyle {
        row_gap: 8.0,
        column_gap: 16.0,
        ..ComputedStyle::default()
    };
    assert_eq!(
        crate::get_computed("row-gap", &style),
        CssValue::Length(8.0, LengthUnit::Px)
    );
    assert_eq!(
        crate::get_computed("column-gap", &style),
        CssValue::Length(16.0, LengthUnit::Px)
    );
}

#[test]
fn resolve_gap_inherit_from_parent() {
    let ctx = default_ctx();
    let parent = ComputedStyle {
        row_gap: 8.0,
        column_gap: 16.0,
        ..ComputedStyle::default()
    };
    let mut winners: PropertyMap = HashMap::new();
    let inherit_val = CssValue::Inherit;
    winners.insert("row-gap", &inherit_val);
    winners.insert("column-gap", &inherit_val);
    let style = build_computed_style(&winners, &parent, &ctx);
    assert!(
        (style.row_gap - 8.0).abs() < f32::EPSILON,
        "expected row-gap=8 from parent, got {}",
        style.row_gap
    );
    assert!(
        (style.column_gap - 16.0).abs() < f32::EPSILON,
        "expected column-gap=16 from parent, got {}",
        style.column_gap
    );
}

// --- Overflow resolution ---

#[test]
fn resolve_overflow_keyword() {
    let ctx = default_ctx();
    let parent = ComputedStyle::default();
    let mut winners: PropertyMap = HashMap::new();
    let val = CssValue::Keyword("hidden".to_string());
    winners.insert("overflow", &val);
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.overflow, Overflow::Hidden);
}

#[test]
fn resolve_overflow_computed_value() {
    let style = ComputedStyle {
        overflow: Overflow::Hidden,
        ..ComputedStyle::default()
    };
    assert_eq!(
        crate::get_computed("overflow", &style),
        CssValue::Keyword("hidden".to_string())
    );
}

// --- min/max width/height ---

#[test]
fn resolve_min_width() {
    let ctx = default_ctx();
    let parent = ComputedStyle::default();
    let mut winners: PropertyMap = HashMap::new();
    let val = CssValue::Length(100.0, LengthUnit::Px);
    winners.insert("min-width", &val);
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.min_width, Dimension::Length(100.0));
}

#[test]
fn resolve_max_width_none() {
    let ctx = default_ctx();
    let parent = ComputedStyle::default();
    let mut winners: PropertyMap = HashMap::new();
    let val = CssValue::Auto;
    winners.insert("max-width", &val);
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.max_width, Dimension::Auto);
}

#[test]
fn resolve_max_width_computed_none() {
    let style = ComputedStyle {
        max_width: Dimension::Auto,
        ..ComputedStyle::default()
    };
    assert_eq!(
        crate::get_computed("max-width", &style),
        CssValue::Keyword("none".to_string())
    );
}

#[test]
fn resolve_min_height_percentage() {
    let ctx = default_ctx();
    let parent = ComputedStyle::default();
    let mut winners: PropertyMap = HashMap::new();
    let val = CssValue::Percentage(25.0);
    winners.insert("min-height", &val);
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.min_height, Dimension::Percentage(25.0));
}

// --- Display variants ---

#[test]
fn resolve_display_table_variants() {
    let ctx = default_ctx();
    let parent = ComputedStyle::default();
    for (keyword, expected) in [
        ("block", Display::Block),
        ("list-item", Display::ListItem),
        ("grid", Display::Grid),
        ("inline-grid", Display::InlineGrid),
        ("table", Display::Table),
        ("inline-table", Display::InlineTable),
        ("table-caption", Display::TableCaption),
        ("table-row", Display::TableRow),
        ("table-cell", Display::TableCell),
        ("table-row-group", Display::TableRowGroup),
        ("table-header-group", Display::TableHeaderGroup),
        ("table-footer-group", Display::TableFooterGroup),
        ("table-column", Display::TableColumn),
        ("table-column-group", Display::TableColumnGroup),
    ] {
        let mut winners: PropertyMap = HashMap::new();
        let val = CssValue::Keyword(keyword.into());
        winners.insert("display", &val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.display, expected, "display: {keyword}");
    }
}

// --- Table properties ---

#[test]
fn resolve_table_keyword_enums() {
    let ctx = default_ctx();
    let parent = ComputedStyle::default();
    for (prop, keyword, check_fn) in [
        (
            "border-collapse",
            "collapse",
            (|s: &ComputedStyle| s.border_collapse == BorderCollapse::Collapse)
                as fn(&ComputedStyle) -> bool,
        ),
        ("table-layout", "fixed", |s: &ComputedStyle| {
            s.table_layout == TableLayout::Fixed
        }),
        ("caption-side", "bottom", |s: &ComputedStyle| {
            s.caption_side == CaptionSide::Bottom
        }),
    ] {
        let mut winners: PropertyMap = HashMap::new();
        let val = CssValue::Keyword(keyword.to_string());
        winners.insert(prop, &val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!(check_fn(&style), "{prop}: {keyword}");
    }
}

#[test]
fn resolve_border_collapse_inherited() {
    let ctx = default_ctx();
    let parent = ComputedStyle {
        border_collapse: BorderCollapse::Collapse,
        ..Default::default()
    };
    let winners: PropertyMap = HashMap::new();
    let style = build_computed_style(&winners, &parent, &ctx);
    assert_eq!(style.border_collapse, BorderCollapse::Collapse);
}

#[test]
fn resolve_border_spacing() {
    let ctx = default_ctx();
    let parent = ComputedStyle::default();
    let mut winners: PropertyMap = HashMap::new();
    let val_h = CssValue::Length(5.0, LengthUnit::Px);
    let val_v = CssValue::Length(10.0, LengthUnit::Px);
    winners.insert("border-spacing-h", &val_h);
    winners.insert("border-spacing-v", &val_v);
    let style = build_computed_style(&winners, &parent, &ctx);
    assert!((style.border_spacing_h - 5.0).abs() < f32::EPSILON);
    assert!((style.border_spacing_v - 10.0).abs() < f32::EPSILON);
}

#[test]
fn resolve_border_spacing_inherited() {
    let ctx = default_ctx();
    let parent = ComputedStyle {
        border_spacing_h: 2.0,
        border_spacing_v: 4.0,
        ..Default::default()
    };
    let winners: PropertyMap = HashMap::new();
    let style = build_computed_style(&winners, &parent, &ctx);
    assert!((style.border_spacing_h - 2.0).abs() < f32::EPSILON);
    assert!((style.border_spacing_v - 4.0).abs() < f32::EPSILON);
}

#[test]
fn get_computed_border_spacing_single_value() {
    let style = ComputedStyle {
        border_spacing_h: 5.0,
        border_spacing_v: 5.0,
        ..Default::default()
    };
    assert_eq!(
        crate::get_computed("border-spacing", &style),
        CssValue::Length(5.0, LengthUnit::Px)
    );
}

#[test]
fn get_computed_table_properties() {
    let style = ComputedStyle {
        border_collapse: BorderCollapse::Collapse,
        border_spacing_h: 5.0,
        border_spacing_v: 10.0,
        table_layout: TableLayout::Fixed,
        caption_side: CaptionSide::Bottom,
        ..Default::default()
    };
    assert_eq!(
        crate::get_computed("border-collapse", &style),
        CssValue::Keyword("collapse".into())
    );
    assert_eq!(
        crate::get_computed("border-spacing", &style),
        CssValue::List(vec![
            CssValue::Length(5.0, LengthUnit::Px),
            CssValue::Length(10.0, LengthUnit::Px),
        ])
    );
    assert_eq!(
        crate::get_computed("table-layout", &style),
        CssValue::Keyword("fixed".into())
    );
    assert_eq!(
        crate::get_computed("caption-side", &style),
        CssValue::Keyword("bottom".into())
    );
}
