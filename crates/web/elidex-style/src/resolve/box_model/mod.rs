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
fn border_side_mut(
    style: &mut ComputedStyle,
    side: usize,
) -> Option<(&mut BorderStyle, &mut f32, &mut CssColor)> {
    match side {
        0 => Some((
            &mut style.border_top_style,
            &mut style.border_top_width,
            &mut style.border_top_color,
        )),
        1 => Some((
            &mut style.border_right_style,
            &mut style.border_right_width,
            &mut style.border_right_color,
        )),
        2 => Some((
            &mut style.border_bottom_style,
            &mut style.border_bottom_width,
            &mut style.border_bottom_color,
        )),
        3 => Some((
            &mut style.border_left_style,
            &mut style.border_left_width,
            &mut style.border_left_color,
        )),
        _ => None,
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
    let Some((s, w, c)) = border_side_mut(style, side) else {
        return;
    };
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
mod tests;
