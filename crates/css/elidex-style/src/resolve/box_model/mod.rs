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
    // CSS Box Model §4: padding cannot be negative. Preserve percentages for
    // layout-time resolution (CSS 2.1 §8.4: % refers to containing block width).
    let dim_nn = |v: &CssValue| match resolve_dimension(v, ctx) {
        Dimension::Length(px) => Dimension::Length(px.max(0.0)),
        Dimension::Percentage(p) => Dimension::Percentage(p.max(0.0)),
        Dimension::Auto => Dimension::ZERO, // padding cannot be auto
    };
    for (prop, setter) in [
        (
            "padding-top",
            (|s: &mut ComputedStyle, d| s.padding.top = d) as fn(&mut ComputedStyle, Dimension),
        ),
        ("padding-right", |s, d| s.padding.right = d),
        ("padding-bottom", |s, d| s.padding.bottom = d),
        ("padding-left", |s, d| s.padding.left = d),
    ] {
        resolve_prop(prop, winners, parent_style, dim_nn, |d| setter(style, d));
    }
}

/// Resolve box-sizing, border-radius, and opacity.
#[allow(clippy::needless_borrows_for_generic_args)]
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
    resolve_prop("border-radius", winners, parent_style, &px, |v| {
        let r = v.max(0.0);
        style.border_radii = [r; 4];
    });
    resolve_prop("border-top-left-radius", winners, parent_style, &px, |v| {
        style.border_radii[0] = v.max(0.0);
    });
    resolve_prop("border-top-right-radius", winners, parent_style, &px, |v| {
        style.border_radii[1] = v.max(0.0);
    });
    resolve_prop(
        "border-bottom-right-radius",
        winners,
        parent_style,
        &px,
        |v| {
            style.border_radii[2] = v.max(0.0);
        },
    );
    resolve_prop(
        "border-bottom-left-radius",
        winners,
        parent_style,
        &px,
        |v| {
            style.border_radii[3] = v.max(0.0);
        },
    );
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
/// Preserves percentages for layout-time resolution against the containing
/// block size (CSS Box Alignment §6).
pub(super) fn resolve_gap_properties(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    let dim_nn = |v: &CssValue| match resolve_dimension(v, ctx) {
        Dimension::Length(px) => Dimension::Length(px.max(0.0)),
        Dimension::Percentage(p) => Dimension::Percentage(p.max(0.0)),
        Dimension::Auto => Dimension::ZERO,
    };
    resolve_prop("row-gap", winners, parent_style, dim_nn, |d| {
        style.row_gap = d;
    });
    resolve_prop("column-gap", winners, parent_style, dim_nn, |d| {
        style.column_gap = d;
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
            &mut style.border_top.style,
            &mut style.border_top.width,
            &mut style.border_top.color,
        )),
        1 => Some((
            &mut style.border_right.style,
            &mut style.border_right.width,
            &mut style.border_right.color,
        )),
        2 => Some((
            &mut style.border_bottom.style,
            &mut style.border_bottom.width,
            &mut style.border_bottom.color,
        )),
        3 => Some((
            &mut style.border_left.style,
            &mut style.border_left.width,
            &mut style.border_left.color,
        )),
        _ => None,
    }
}

/// Get the border-style for a side by index (0=top, 1=right, 2=bottom, 3=left).
fn border_style_by_side(style: &ComputedStyle, side: usize) -> BorderStyle {
    match side {
        0 => style.border_top.style,
        1 => style.border_right.style,
        2 => style.border_bottom.style,
        3 => style.border_left.style,
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

/// Resolve position offset properties (top/right/bottom/left) and z-index.
#[allow(clippy::needless_borrows_for_generic_args)]
pub(super) fn resolve_position_offsets(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    let dim = |v: &CssValue| resolve_dimension(v, ctx);
    resolve_prop("top", winners, parent_style, &dim, |d| style.top = d);
    resolve_prop("right", winners, parent_style, &dim, |d| style.right = d);
    resolve_prop("bottom", winners, parent_style, &dim, |d| style.bottom = d);
    resolve_prop("left", winners, parent_style, &dim, |d| style.left = d);
    if let Some(CssValue::Number(n)) = get_resolved_winner("z-index", winners, parent_style) {
        if n.is_finite() {
            #[allow(clippy::cast_possible_truncation)]
            {
                style.z_index = Some(n as i32);
            }
        }
    }
}

pub(super) fn resolve_overflow(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    resolve_keyword_enum_prop!(
        "overflow-x",
        winners,
        parent_style,
        style.overflow_x,
        Overflow::from_keyword
    );
    resolve_keyword_enum_prop!(
        "overflow-y",
        winners,
        parent_style,
        style.overflow_y,
        Overflow::from_keyword
    );

    // CSS Overflow L3 §3.2: If one axis is `visible` or `clip` and the other
    // is neither `visible` nor `clip`, then `visible` computes to `auto` and
    // `clip` computes to `hidden`.
    let (ox, oy) = (style.overflow_x, style.overflow_y);
    let other_is_scrollable = |o: Overflow| o != Overflow::Visible && o != Overflow::Clip;
    if ox == Overflow::Visible && other_is_scrollable(oy) {
        style.overflow_x = Overflow::Auto;
    } else if ox == Overflow::Clip && other_is_scrollable(oy) {
        style.overflow_x = Overflow::Hidden;
    }
    if oy == Overflow::Visible && other_is_scrollable(ox) {
        style.overflow_y = Overflow::Auto;
    } else if oy == Overflow::Clip && other_is_scrollable(ox) {
        style.overflow_y = Overflow::Hidden;
    }
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
