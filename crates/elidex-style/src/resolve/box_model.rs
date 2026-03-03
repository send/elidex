//! Box model resolution: dimensions, margin, padding, border, extras, gap.

use elidex_plugin::{
    BorderCollapse, BorderStyle, BoxSizing, CaptionSide, ComputedStyle, CssColor, CssValue,
    Dimension, Overflow, Position, TableLayout,
};

use super::helpers::resolve_keyword_enum_prop;
use super::helpers::{
    get_resolved_winner, resolve_border_style_value, resolve_dimension, resolve_prop,
    resolve_to_px, PropertyMap,
};
use super::ResolveContext;

/// Resolve dimensions, margins, and padding.
pub(super) fn resolve_box_dimensions(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    let dim = |v: &CssValue| resolve_dimension(v, ctx);
    resolve_prop("width", winners, parent_style, dim, |d| style.width = d);
    resolve_prop("height", winners, parent_style, dim, |d| {
        style.height = d;
    });
    resolve_prop("min-width", winners, parent_style, dim, |d| {
        style.min_width = d;
    });
    resolve_prop("max-width", winners, parent_style, dim, |d| {
        style.max_width = d;
    });
    resolve_prop("min-height", winners, parent_style, dim, |d| {
        style.min_height = d;
    });
    resolve_prop("max-height", winners, parent_style, dim, |d| {
        style.max_height = d;
    });
    for (prop, setter) in [
        (
            "margin-top",
            (|s: &mut ComputedStyle, d| s.margin_top = d) as fn(&mut ComputedStyle, Dimension),
        ),
        ("margin-right", |s, d| s.margin_right = d),
        ("margin-bottom", |s, d| s.margin_bottom = d),
        ("margin-left", |s, d| s.margin_left = d),
    ] {
        resolve_prop(prop, winners, parent_style, dim, |d| setter(style, d));
    }
    // CSS Box Model §4: padding cannot be negative.
    let px = |v: &CssValue| resolve_to_px(v, ctx).max(0.0);
    for (prop, setter) in [
        (
            "padding-top",
            (|s: &mut ComputedStyle, v| s.padding_top = v) as fn(&mut ComputedStyle, f32),
        ),
        ("padding-right", |s, v| s.padding_right = v),
        ("padding-bottom", |s, v| s.padding_bottom = v),
        ("padding-left", |s, v| s.padding_left = v),
    ] {
        resolve_prop(prop, winners, parent_style, px, |v| setter(style, v));
    }
}

/// Resolve box-sizing, border-radius, and opacity.
pub(super) fn resolve_box_model_extras(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    resolve_keyword_enum_prop!(
        "box-sizing",
        winners,
        parent_style,
        style.box_sizing,
        BoxSizing::from_keyword
    );
    let px = |v: &CssValue| resolve_to_px(v, ctx);
    resolve_prop("border-radius", winners, parent_style, px, |v| {
        style.border_radius = v.max(0.0);
    });
    resolve_prop(
        "opacity",
        winners,
        parent_style,
        |v| match v {
            CssValue::Number(n) if n.is_finite() => n.clamp(0.0, 1.0),
            _ => 1.0,
        },
        |v| style.opacity = v,
    );
}

/// Resolve row-gap and column-gap.
///
/// NOTE: gap percentages resolve to 0 because `resolve_to_px` has no
/// containing block width. Proper percentage gap requires layout-time
/// resolution with Dimension storage (Phase 4).
pub(super) fn resolve_gap_properties(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    let px = |v: &CssValue| resolve_to_px(v, ctx);
    resolve_prop("row-gap", winners, parent_style, px, |v| {
        style.row_gap = v.max(0.0);
    });
    resolve_prop("column-gap", winners, parent_style, px, |v| {
        style.column_gap = v.max(0.0);
    });
}

// --- Border resolution ---

/// Border property names for all four sides, indexed by side (0=top, 1=right, 2=bottom, 3=left).
const BORDER_STYLE_PROPS: [&str; 4] = [
    "border-top-style",
    "border-right-style",
    "border-bottom-style",
    "border-left-style",
];
const BORDER_WIDTH_PROPS: [&str; 4] = [
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
];
const BORDER_COLOR_PROPS: [&str; 4] = [
    "border-top-color",
    "border-right-color",
    "border-bottom-color",
    "border-left-color",
];

/// Return mutable references to the border (style, width, color) fields for
/// the given side index (0=top, 1=right, 2=bottom, 3=left).
///
/// # Panics
///
/// Panics if `side >= 4`.
fn border_side_mut(
    style: &mut ComputedStyle,
    side: usize,
) -> (&mut BorderStyle, &mut f32, &mut CssColor) {
    match side {
        0 => (
            &mut style.border_top_style,
            &mut style.border_top_width,
            &mut style.border_top_color,
        ),
        1 => (
            &mut style.border_right_style,
            &mut style.border_right_width,
            &mut style.border_right_color,
        ),
        2 => (
            &mut style.border_bottom_style,
            &mut style.border_bottom_width,
            &mut style.border_bottom_color,
        ),
        3 => (
            &mut style.border_left_style,
            &mut style.border_left_width,
            &mut style.border_left_color,
        ),
        _ => unreachable!("border side index must be 0..4"),
    }
}

/// Get the border-style for a side by index (0=top, 1=right, 2=bottom, 3=left).
fn border_style_by_side(style: &ComputedStyle, side: usize) -> BorderStyle {
    match side {
        0 => style.border_top_style,
        1 => style.border_right_style,
        2 => style.border_bottom_style,
        3 => style.border_left_style,
        _ => BorderStyle::None,
    }
}

/// Set border-style, border-width, and border-color for a side by index.
fn set_border_side(
    style: &mut ComputedStyle,
    side: usize,
    bs: Option<BorderStyle>,
    bw: Option<f32>,
    bc: Option<CssColor>,
) {
    let (s, w, c) = border_side_mut(style, side);
    if let Some(v) = bs {
        *s = v;
    }
    if let Some(v) = bw {
        *w = v;
    }
    if let Some(v) = bc {
        *c = v;
    }
}

/// Resolve all border properties (style, width, color) for all four sides.
///
/// Resolution order: style first (width depends on style being none), then
/// width, then color. Each group iterates over top/right/bottom/left.
pub(super) fn resolve_border_properties(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    // Border styles must be resolved before border widths (width = 0 when style = none).
    for (side, prop) in BORDER_STYLE_PROPS.iter().enumerate() {
        if let Some(value) = get_resolved_winner(prop, winners, parent_style) {
            set_border_side(
                style,
                side,
                Some(resolve_border_style_value(&value)),
                None,
                None,
            );
        }
    }

    // Border widths (special: 0 when style = none).
    for (side, prop) in BORDER_WIDTH_PROPS.iter().enumerate() {
        // CSS spec: computed border-width is 0 when border-style is none or hidden.
        let px = if matches!(
            border_style_by_side(style, side),
            BorderStyle::None | BorderStyle::Hidden
        ) {
            0.0
        } else {
            match get_resolved_winner(prop, winners, parent_style) {
                // CSS Backgrounds §4.3: border-width cannot be negative.
                Some(value) => resolve_to_px(&value, ctx).max(0.0),
                None => 3.0, // medium
            }
        };
        set_border_side(style, side, None, Some(px), None);
    }

    // Border colors (initial = currentcolor).
    let current_color = style.color;
    for (side, prop) in BORDER_COLOR_PROPS.iter().enumerate() {
        let color = match get_resolved_winner(prop, winners, parent_style) {
            Some(CssValue::Color(c)) => c,
            Some(CssValue::Keyword(ref k)) if k.eq_ignore_ascii_case("currentcolor") => {
                current_color
            }
            _ => current_color,
        };
        set_border_side(style, side, None, None, Some(color));
    }
}

// TODO(Phase 4): float/clear layout (CSS 2.1 §9.5).
// TODO(Phase 4): visibility: hidden/collapse (CSS 2.1 §11.2).
// TODO(Phase 4): vertical-align on inline elements (CSS 2.1 §10.8).

// --- Display, position, overflow ---

pub(super) fn resolve_display(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    use elidex_plugin::Display;
    resolve_keyword_enum_prop!(
        "display",
        winners,
        parent_style,
        style.display,
        Display::from_keyword
    );
}

pub(super) fn resolve_position(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    resolve_keyword_enum_prop!(
        "position",
        winners,
        parent_style,
        style.position,
        Position::from_keyword
    );
}

pub(super) fn resolve_overflow(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    resolve_keyword_enum_prop!(
        "overflow",
        winners,
        parent_style,
        style.overflow,
        Overflow::from_keyword
    );
}

// --- Content property resolution ---

/// Resolve the `content` property to a [`ContentValue`].
///
/// `content` is non-inherited. Keywords `normal` and `none` map directly;
/// string values produce `ContentValue::Items`. `attr:name` convention
/// (from the parser) becomes `ContentItem::Attr`.
pub(super) fn resolve_content(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    use elidex_plugin::{ContentItem, ContentValue};

    let Some(value) = get_resolved_winner("content", winners, parent_style) else {
        return; // not declared → default (Normal)
    };
    style.content = match &value {
        CssValue::Keyword(k) => match k.as_str() {
            "none" => ContentValue::None,
            "normal" => ContentValue::Normal,
            k if k.starts_with("attr:") => {
                ContentValue::Items(vec![ContentItem::Attr(k["attr:".len()..].to_string())])
            }
            _ => ContentValue::Normal,
        },
        CssValue::String(s) => ContentValue::Items(vec![ContentItem::String(s.clone())]),
        CssValue::List(items) => {
            let content_items: Vec<ContentItem> = items
                .iter()
                .filter_map(|item| match item {
                    CssValue::String(s) => Some(ContentItem::String(s.clone())),
                    CssValue::Keyword(k) if k.starts_with("attr:") => {
                        Some(ContentItem::Attr(k["attr:".len()..].to_string()))
                    }
                    _ => None,
                })
                .collect();
            if content_items.is_empty() {
                ContentValue::Normal
            } else {
                ContentValue::Items(content_items)
            }
        }
        _ => ContentValue::Normal,
    };
}

// --- Table property resolution ---

/// Resolve table-related CSS properties.
///
/// `border-collapse`, `border-spacing`, `caption-side` are inherited.
/// `table-layout` is non-inherited.
pub(super) fn resolve_table_properties(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    use super::helpers::resolve_inherited_keyword_enum;

    // border-collapse (inherited)
    style.border_collapse = resolve_inherited_keyword_enum(
        "border-collapse",
        winners,
        parent_style,
        parent_style.border_collapse,
        BorderCollapse::from_keyword,
    );

    // border-spacing (inherited, 2 longhands)
    let has_h = winners.contains_key("border-spacing-h");
    let has_v = winners.contains_key("border-spacing-v");
    if has_h || has_v {
        let px = |v: &CssValue| resolve_to_px(v, ctx).max(0.0);
        if has_h {
            resolve_prop("border-spacing-h", winners, parent_style, px, |v| {
                style.border_spacing_h = v;
            });
        } else {
            style.border_spacing_h = parent_style.border_spacing_h;
        }
        if has_v {
            resolve_prop("border-spacing-v", winners, parent_style, px, |v| {
                style.border_spacing_v = v;
            });
        } else {
            style.border_spacing_v = parent_style.border_spacing_v;
        }
    } else {
        // Inherited from parent.
        style.border_spacing_h = parent_style.border_spacing_h;
        style.border_spacing_v = parent_style.border_spacing_v;
    }

    // table-layout (non-inherited)
    resolve_keyword_enum_prop!(
        "table-layout",
        winners,
        parent_style,
        style.table_layout,
        TableLayout::from_keyword
    );

    // caption-side (inherited)
    style.caption_side = resolve_inherited_keyword_enum(
        "caption-side",
        winners,
        parent_style,
        parent_style.caption_side,
        CaptionSide::from_keyword,
    );
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use elidex_plugin::{
        BorderCollapse, BoxSizing, CaptionSide, ComputedStyle, CssColor, CssValue, Dimension,
        Display, LengthUnit, Overflow, Position, TableLayout,
    };

    use crate::resolve::helpers::PropertyMap;
    use crate::resolve::{build_computed_style, get_computed_as_css_value, ResolveContext};

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
        assert_eq!(style.border_top_width, 0.0);
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
        assert_eq!(style.border_top_width, 5.0);
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
        assert_eq!(style.padding_top, 10.0);
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
            get_computed_as_css_value("box-sizing", &style),
            CssValue::Keyword("border-box".to_string())
        );
        assert_eq!(
            get_computed_as_css_value("border-radius", &style),
            CssValue::Length(10.0, LengthUnit::Px)
        );
        assert_eq!(
            get_computed_as_css_value("opacity", &style),
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
            get_computed_as_css_value("row-gap", &style),
            CssValue::Length(8.0, LengthUnit::Px)
        );
        assert_eq!(
            get_computed_as_css_value("column-gap", &style),
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
            get_computed_as_css_value("overflow", &style),
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
            get_computed_as_css_value("max-width", &style),
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
            get_computed_as_css_value("border-spacing", &style),
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
            get_computed_as_css_value("border-collapse", &style),
            CssValue::Keyword("collapse".into())
        );
        assert_eq!(
            get_computed_as_css_value("border-spacing", &style),
            CssValue::List(vec![
                CssValue::Length(5.0, LengthUnit::Px),
                CssValue::Length(10.0, LengthUnit::Px),
            ])
        );
        assert_eq!(
            get_computed_as_css_value("table-layout", &style),
            CssValue::Keyword("fixed".into())
        );
        assert_eq!(
            get_computed_as_css_value("caption-side", &style),
            CssValue::Keyword("bottom".into())
        );
    }
}
