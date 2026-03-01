//! CSS value resolution: relative units → absolute pixels.
//!
//! Converts parsed [`CssValue`]s into concrete values for [`ComputedStyle`]
//! fields, resolving relative lengths, font-size keywords, `currentcolor`,
//! and the border-width/border-style interaction.

use elidex_plugin::{
    AlignContent, AlignItems, AlignSelf, BorderStyle, ComputedStyle, CssColor, CssValue, Dimension,
    Display, FlexDirection, FlexWrap, JustifyContent, LengthUnit, Position,
};

use crate::inherit::{get_initial_value, is_inherited};

/// Cascade winner map: property name → winning CSS value.
type PropertyMap<'a> = std::collections::HashMap<&'a str, &'a CssValue>;

/// Context for resolving relative CSS values.
pub(crate) struct ResolveContext {
    pub viewport_width: f32,
    pub viewport_height: f32,
    /// Base value for `em` unit resolution. For font-size resolution this is
    /// the parent's font-size; for all other properties it is the element's
    /// own computed font-size.
    pub em_base: f32,
    pub root_font_size: f32,
}

impl ResolveContext {
    /// Return a copy with a different `em_base` value.
    pub fn with_em_base(&self, em_base: f32) -> Self {
        Self { em_base, ..*self }
    }

    /// Return a copy with both `em_base` and `root_font_size` overridden.
    pub fn with_em_and_root(&self, em_base: f32, root_font_size: f32) -> Self {
        Self {
            em_base,
            root_font_size,
            ..*self
        }
    }
}

/// Resolve a CSS length value to pixels.
pub(crate) fn resolve_length(value: f32, unit: LengthUnit, ctx: &ResolveContext) -> f32 {
    match unit {
        LengthUnit::Em => value * ctx.em_base,
        LengthUnit::Rem => value * ctx.root_font_size,
        LengthUnit::Vw => value * ctx.viewport_width / 100.0,
        LengthUnit::Vh => value * ctx.viewport_height / 100.0,
        LengthUnit::Vmin => value * ctx.viewport_width.min(ctx.viewport_height) / 100.0,
        LengthUnit::Vmax => value * ctx.viewport_width.max(ctx.viewport_height) / 100.0,
        // Px and any future units: return as-is.
        _ => value,
    }
}

/// Resolve font-size keywords to pixel values.
fn resolve_font_size_keyword(keyword: &str, parent_font_size: f32) -> Option<f32> {
    match keyword {
        "xx-small" => Some(9.0),
        "x-small" => Some(10.0),
        "small" => Some(13.0),
        "medium" => Some(16.0),
        "large" => Some(18.0),
        "x-large" => Some(24.0),
        "xx-large" => Some(32.0),
        "smaller" => Some(parent_font_size * 0.833),
        "larger" => Some(parent_font_size * 1.2),
        _ => None,
    }
}

/// Resolve `inherit` / `initial` / `unset` keywords, returning the effective
/// [`CssValue`] to use for further resolution.
fn resolve_keyword(
    property: &str,
    value: &CssValue,
    parent_style: &ComputedStyle,
) -> Option<CssValue> {
    match value {
        CssValue::Inherit => Some(get_computed_as_css_value(property, parent_style)),
        CssValue::Initial => Some(get_initial_value(property)),
        CssValue::Unset => {
            if is_inherited(property) {
                Some(get_computed_as_css_value(property, parent_style))
            } else {
                Some(get_initial_value(property))
            }
        }
        _ => None,
    }
}

/// Extract a property's computed value back into a [`CssValue`] for inheritance.
///
/// Also used by `getComputedStyle()` DOM API.
// Sync: When adding a property, also update build_computed_style and get_initial_value.
pub fn get_computed_as_css_value(property: &str, style: &ComputedStyle) -> CssValue {
    match property {
        "color" => CssValue::Color(style.color),
        "font-size" => CssValue::Length(style.font_size, LengthUnit::Px),
        "font-family" => CssValue::List(
            style
                .font_family
                .iter()
                .map(|s| CssValue::Keyword(s.clone()))
                .collect(),
        ),
        "display" => CssValue::Keyword(style.display.as_ref().to_string()),
        "position" => CssValue::Keyword(style.position.as_ref().to_string()),
        "background-color" => CssValue::Color(style.background_color),
        "width" => dimension_to_css_value(style.width),
        "height" => dimension_to_css_value(style.height),
        "margin-top" => dimension_to_css_value(style.margin_top),
        "margin-right" => dimension_to_css_value(style.margin_right),
        "margin-bottom" => dimension_to_css_value(style.margin_bottom),
        "margin-left" => dimension_to_css_value(style.margin_left),
        "padding-top" => CssValue::Length(style.padding_top, LengthUnit::Px),
        "padding-right" => CssValue::Length(style.padding_right, LengthUnit::Px),
        "padding-bottom" => CssValue::Length(style.padding_bottom, LengthUnit::Px),
        "padding-left" => CssValue::Length(style.padding_left, LengthUnit::Px),
        "border-top-width" => CssValue::Length(style.border_top_width, LengthUnit::Px),
        "border-right-width" => CssValue::Length(style.border_right_width, LengthUnit::Px),
        "border-bottom-width" => CssValue::Length(style.border_bottom_width, LengthUnit::Px),
        "border-left-width" => CssValue::Length(style.border_left_width, LengthUnit::Px),
        "border-top-style" => CssValue::Keyword(style.border_top_style.as_ref().to_string()),
        "border-right-style" => CssValue::Keyword(style.border_right_style.as_ref().to_string()),
        "border-bottom-style" => CssValue::Keyword(style.border_bottom_style.as_ref().to_string()),
        "border-left-style" => CssValue::Keyword(style.border_left_style.as_ref().to_string()),
        "border-top-color" => CssValue::Color(style.border_top_color),
        "border-right-color" => CssValue::Color(style.border_right_color),
        "border-bottom-color" => CssValue::Color(style.border_bottom_color),
        "border-left-color" => CssValue::Color(style.border_left_color),
        "flex-direction" => CssValue::Keyword(style.flex_direction.as_ref().to_string()),
        "flex-wrap" => CssValue::Keyword(style.flex_wrap.as_ref().to_string()),
        "justify-content" => CssValue::Keyword(style.justify_content.as_ref().to_string()),
        "align-items" => CssValue::Keyword(style.align_items.as_ref().to_string()),
        "align-content" => CssValue::Keyword(style.align_content.as_ref().to_string()),
        "align-self" => CssValue::Keyword(style.align_self.as_ref().to_string()),
        "flex-grow" => CssValue::Number(style.flex_grow),
        "flex-shrink" => CssValue::Number(style.flex_shrink),
        "flex-basis" => dimension_to_css_value(style.flex_basis),
        #[allow(clippy::cast_precision_loss)]
        "order" => CssValue::Number(style.order as f32),
        _ => get_initial_value(property),
    }
}

// Display, Position, and BorderStyle implement AsRef<str> in elidex-plugin,
// so enum-to-string conversion is handled via .as_ref() directly.

/// Convert a [`Dimension`] back into a [`CssValue`] for CSS serialization.
pub fn dimension_to_css_value(d: Dimension) -> CssValue {
    match d {
        Dimension::Length(v) => CssValue::Length(v, LengthUnit::Px),
        Dimension::Percentage(v) => CssValue::Percentage(v),
        Dimension::Auto => CssValue::Auto,
    }
}

/// Build a [`ComputedStyle`] from the cascade winner map.
///
/// Resolution order: font-size first (dependencies), then color,
/// then all remaining properties.
// Sync: When adding a property, also update get_computed_as_css_value and get_initial_value.
pub(crate) fn build_computed_style(
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) -> ComputedStyle {
    let mut style = ComputedStyle::default();

    // --- Font properties (inherit by default) ---
    // Font-size must be resolved first: em units in all other properties depend on it.
    let element_font_size = resolve_font_size(winners, parent_style, ctx);
    style.font_size = element_font_size;
    let elem_ctx = ctx.with_em_base(element_font_size);
    resolve_font_family(&mut style, winners, parent_style);

    // --- Color properties ---
    // Color must precede border-color (initial = currentcolor) and background-color.
    style.color = resolve_color(winners, parent_style);
    resolve_background_color(&mut style, winners, parent_style);

    // --- Display & positioning ---
    resolve_display(&mut style, winners, parent_style);
    resolve_position(&mut style, winners, parent_style);

    // --- Box model: dimensions, margin, padding ---
    let dim = |v: &CssValue| resolve_dimension(v, &elem_ctx);
    resolve_prop("width", winners, parent_style, dim, |d| style.width = d);
    resolve_prop("height", winners, parent_style, dim, |d| {
        style.height = d;
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
        resolve_prop(prop, winners, parent_style, dim, |d| setter(&mut style, d));
    }
    let px = |v: &CssValue| resolve_to_px(v, &elem_ctx);
    for (prop, setter) in [
        (
            "padding-top",
            (|s: &mut ComputedStyle, v| s.padding_top = v) as fn(&mut ComputedStyle, f32),
        ),
        ("padding-right", |s, v| s.padding_right = v),
        ("padding-bottom", |s, v| s.padding_bottom = v),
        ("padding-left", |s, v| s.padding_left = v),
    ] {
        resolve_prop(prop, winners, parent_style, px, |v| setter(&mut style, v));
    }

    // --- Border properties (style → width → color) ---
    resolve_border_properties(&mut style, winners, parent_style, &elem_ctx);

    // --- Flex properties ---
    resolve_flex_properties(&mut style, winners, parent_style, dim);

    style
}

/// Resolve a CSS keyword property to its corresponding enum variant.
///
/// Matches the keyword string from a `CssValue::Keyword` against a list of
/// `(keyword_str, EnumVariant)` pairs via `$parser`, falling back to the
/// enum's `Default` if no match. The result is assigned to `$field` only
/// when the property is present in the cascade winners.
///
/// # Example
///
/// ```ignore
/// resolve_keyword_enum_prop!(
///     "display", winners, parent_style, style.display,
///     |k| match k {
///         "none" => Some(Display::None),
///         "inline" => Some(Display::Inline),
///         "flex" => Some(Display::Flex),
///         _ => None,
///     }
/// );
/// ```
macro_rules! resolve_keyword_enum_prop {
    ($prop:expr, $winners:expr, $parent:expr, $field:expr, $parser:expr) => {
        if let Some(val) = resolve_keyword_enum($prop, $winners, $parent, $parser) {
            $field = val;
        }
    };
}

/// Resolve all flex-related properties.
fn resolve_flex_properties(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    dim: impl Fn(&CssValue) -> Dimension,
) {
    resolve_flex_keyword_enums(style, winners, parent_style);

    resolve_prop(
        "flex-grow",
        winners,
        parent_style,
        |v| resolve_non_negative_f32(v, 0.0),
        |v| style.flex_grow = v,
    );
    resolve_prop(
        "flex-shrink",
        winners,
        parent_style,
        |v| resolve_non_negative_f32(v, 1.0),
        |v| style.flex_shrink = v,
    );
    resolve_prop("flex-basis", winners, parent_style, &dim, |d| {
        style.flex_basis = d;
    });
    resolve_prop(
        "order",
        winners,
        parent_style,
        |v| resolve_i32(v, 0),
        |v| style.order = v,
    );
}

/// Resolve flex keyword-enum properties (direction, wrap, alignment).
fn resolve_flex_keyword_enums(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    resolve_keyword_enum_prop!(
        "flex-direction",
        winners,
        parent_style,
        style.flex_direction,
        |k| match k {
            "row" => Some(FlexDirection::Row),
            "row-reverse" => Some(FlexDirection::RowReverse),
            "column" => Some(FlexDirection::Column),
            "column-reverse" => Some(FlexDirection::ColumnReverse),
            _ => None,
        }
    );
    resolve_keyword_enum_prop!(
        "flex-wrap",
        winners,
        parent_style,
        style.flex_wrap,
        |k| match k {
            "nowrap" => Some(FlexWrap::Nowrap),
            "wrap" => Some(FlexWrap::Wrap),
            "wrap-reverse" => Some(FlexWrap::WrapReverse),
            _ => None,
        }
    );
    resolve_keyword_enum_prop!(
        "justify-content",
        winners,
        parent_style,
        style.justify_content,
        |k| match k {
            "flex-start" => Some(JustifyContent::FlexStart),
            "flex-end" => Some(JustifyContent::FlexEnd),
            "center" => Some(JustifyContent::Center),
            "space-between" => Some(JustifyContent::SpaceBetween),
            "space-around" => Some(JustifyContent::SpaceAround),
            "space-evenly" => Some(JustifyContent::SpaceEvenly),
            _ => None,
        }
    );
    resolve_keyword_enum_prop!(
        "align-items",
        winners,
        parent_style,
        style.align_items,
        |k| match k {
            "stretch" => Some(AlignItems::Stretch),
            "flex-start" => Some(AlignItems::FlexStart),
            "flex-end" => Some(AlignItems::FlexEnd),
            "center" => Some(AlignItems::Center),
            "baseline" => Some(AlignItems::Baseline),
            _ => None,
        }
    );
    resolve_keyword_enum_prop!(
        "align-content",
        winners,
        parent_style,
        style.align_content,
        |k| match k {
            "stretch" => Some(AlignContent::Stretch),
            "flex-start" => Some(AlignContent::FlexStart),
            "flex-end" => Some(AlignContent::FlexEnd),
            "center" => Some(AlignContent::Center),
            "space-between" => Some(AlignContent::SpaceBetween),
            "space-around" => Some(AlignContent::SpaceAround),
            _ => None,
        }
    );
    resolve_keyword_enum_prop!(
        "align-self",
        winners,
        parent_style,
        style.align_self,
        |k| match k {
            "auto" => Some(AlignSelf::Auto),
            "stretch" => Some(AlignSelf::Stretch),
            "flex-start" => Some(AlignSelf::FlexStart),
            "flex-end" => Some(AlignSelf::FlexEnd),
            "center" => Some(AlignSelf::Center),
            "baseline" => Some(AlignSelf::Baseline),
            _ => None,
        }
    );
}

// --- Individual property resolvers ---

fn resolve_font_size(
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) -> f32 {
    let Some(value) = winners.get("font-size") else {
        // No declaration — inherit from parent.
        return parent_style.font_size;
    };
    let value = resolve_keyword_or_clone("font-size", value, parent_style);
    resolve_font_size_value(&value, parent_style, ctx)
}

fn resolve_font_size_value(
    value: &CssValue,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) -> f32 {
    match value {
        CssValue::Length(v, unit) => {
            // For font-size, em is relative to parent, not self.
            resolve_length(*v, *unit, &ctx.with_em_base(parent_style.font_size))
        }
        CssValue::Percentage(p) => parent_style.font_size * p / 100.0,
        CssValue::Keyword(kw) => {
            resolve_font_size_keyword(kw, parent_style.font_size).unwrap_or(parent_style.font_size)
        }
        _ => parent_style.font_size,
    }
}

fn resolve_color(winners: &PropertyMap<'_>, parent_style: &ComputedStyle) -> CssColor {
    let Some(value) = winners.get("color") else {
        return parent_style.color;
    };
    let value = resolve_keyword_or_clone("color", value, parent_style);
    match value {
        CssValue::Color(c) => c,
        _ => parent_style.color,
    }
}

fn resolve_font_family(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    let Some(value) = winners.get("font-family") else {
        style.font_family.clone_from(&parent_style.font_family);
        return;
    };
    let value = resolve_keyword_or_clone("font-family", value, parent_style);
    match value {
        CssValue::List(items) => {
            style.font_family = items
                .iter()
                .filter_map(|v| match v {
                    CssValue::String(s) => Some(s.clone()),
                    CssValue::Keyword(k) => Some(k.clone()),
                    _ => None,
                })
                .collect();
        }
        _ => {
            style.font_family.clone_from(&parent_style.font_family);
        }
    }
}

/// Resolve a keyword enum property from the winners map.
fn resolve_keyword_enum<T: Default>(
    property: &str,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    from_keyword: impl Fn(&str) -> Option<T>,
) -> Option<T> {
    let value = winners.get(property)?;
    let value = resolve_keyword_or_clone(property, value, parent_style);
    Some(match value {
        CssValue::Keyword(ref k) => from_keyword(k).unwrap_or_default(),
        _ => T::default(),
    })
}

fn resolve_display(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    resolve_keyword_enum_prop!(
        "display",
        winners,
        parent_style,
        style.display,
        |k| match k {
            "block" => Some(Display::Block),
            "inline" => Some(Display::Inline),
            "inline-block" => Some(Display::InlineBlock),
            "none" => Some(Display::None),
            "flex" => Some(Display::Flex),
            "inline-flex" => Some(Display::InlineFlex),
            _ => None,
        }
    );
}

fn resolve_position(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    resolve_keyword_enum_prop!(
        "position",
        winners,
        parent_style,
        style.position,
        |k| match k {
            "static" => Some(Position::Static),
            "relative" => Some(Position::Relative),
            "absolute" => Some(Position::Absolute),
            "fixed" => Some(Position::Fixed),
            _ => None,
        }
    );
}

fn resolve_background_color(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    let Some(value) = winners.get("background-color") else {
        return;
    };
    let value = resolve_keyword_or_clone("background-color", value, parent_style);
    match value {
        CssValue::Color(c) => style.background_color = c,
        CssValue::Keyword(ref k) if k.eq_ignore_ascii_case("currentcolor") => {
            style.background_color = style.color;
        }
        _ => {}
    }
}

/// Resolve a property and apply the result via a setter closure.
fn resolve_prop<T>(
    property: &str,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    convert: impl Fn(&CssValue) -> T,
    set: impl FnOnce(T),
) {
    let Some(value) = winners.get(property) else {
        return;
    };
    let value = resolve_keyword_or_clone(property, value, parent_style);
    set(convert(&value));
}

fn resolve_dimension(value: &CssValue, ctx: &ResolveContext) -> Dimension {
    match value {
        CssValue::Length(v, unit) => Dimension::Length(resolve_length(*v, *unit, ctx)),
        CssValue::Percentage(p) => Dimension::Percentage(*p),
        CssValue::Number(n) if *n == 0.0 => Dimension::Length(0.0),
        // Auto and anything else → Auto.
        _ => Dimension::Auto,
    }
}

/// Resolve a border-style keyword to a [`BorderStyle`] enum value.
///
/// Phase 1 supports: `none`, `solid`, `dashed`, `dotted`.
// TODO(Phase 2): support double, groove, ridge, inset, outset
fn resolve_border_style_value(value: &CssValue) -> BorderStyle {
    match value {
        CssValue::Keyword(ref k) => match k.as_str() {
            "none" => BorderStyle::None,
            "solid" => BorderStyle::Solid,
            "dashed" => BorderStyle::Dashed,
            "dotted" => BorderStyle::Dotted,
            _ => BorderStyle::default(),
        },
        _ => BorderStyle::default(),
    }
}

/// Get the border-style for the side corresponding to a border-width property.
fn border_style_for_width(style: &ComputedStyle, width_prop: &str) -> BorderStyle {
    match width_prop {
        "border-top-width" => style.border_top_style,
        "border-right-width" => style.border_right_style,
        "border-bottom-width" => style.border_bottom_style,
        "border-left-width" => style.border_left_style,
        _ => BorderStyle::None,
    }
}

/// Set a border-width field by property name.
fn set_border_width(style: &mut ComputedStyle, prop: &str, value: f32) {
    match prop {
        "border-top-width" => style.border_top_width = value,
        "border-right-width" => style.border_right_width = value,
        "border-bottom-width" => style.border_bottom_width = value,
        "border-left-width" => style.border_left_width = value,
        _ => {}
    }
}

/// Set a border-color field by property name.
fn set_border_color(style: &mut ComputedStyle, prop: &str, color: CssColor) {
    match prop {
        "border-top-color" => style.border_top_color = color,
        "border-right-color" => style.border_right_color = color,
        "border-bottom-color" => style.border_bottom_color = color,
        "border-left-color" => style.border_left_color = color,
        _ => {}
    }
}

/// Resolve all border properties (style, width, color) for all four sides.
///
/// Resolution order: style first (width depends on style being none), then
/// width, then color. Each group iterates over top/right/bottom/left.
fn resolve_border_properties(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    const SIDES: [&str; 4] = ["top", "right", "bottom", "left"];

    // Border styles must be resolved before border widths (width = 0 when style = none).
    let bs = |v: &CssValue| resolve_border_style_value(v);
    for side in &SIDES {
        let prop = format!("border-{side}-style");
        resolve_prop(&prop, winners, parent_style, bs, |v| {
            set_border_style(style, side, v);
        });
    }

    // Border widths (special: 0 when style = none).
    for side in &SIDES {
        let prop = format!("border-{side}-width");
        resolve_border_width_prop(style, &prop, winners, parent_style, ctx);
    }

    // Border colors (initial = currentcolor).
    let current_color = style.color;
    for side in &SIDES {
        let prop = format!("border-{side}-color");
        resolve_border_color_prop(style, &prop, winners, parent_style, current_color);
    }
}

/// Set a border-style field by side name.
fn set_border_style(style: &mut ComputedStyle, side: &str, value: BorderStyle) {
    match side {
        "top" => style.border_top_style = value,
        "right" => style.border_right_style = value,
        "bottom" => style.border_bottom_style = value,
        "left" => style.border_left_style = value,
        _ => {}
    }
}

fn resolve_border_width_prop(
    style: &mut ComputedStyle,
    property: &str,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    // CSS spec: computed border-width is 0 when border-style is none.
    let px = if border_style_for_width(style, property) == BorderStyle::None {
        0.0
    } else {
        match winners.get(property) {
            Some(value) => {
                let value = resolve_keyword_or_clone(property, value, parent_style);
                resolve_to_px(&value, ctx)
            }
            None => 3.0, // medium
        }
    };
    set_border_width(style, property, px);
}

fn resolve_border_color_prop(
    style: &mut ComputedStyle,
    property: &str,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    current_color: CssColor,
) {
    let color = match winners.get(property) {
        Some(value) => {
            let value = resolve_keyword_or_clone(property, value, parent_style);
            match value {
                CssValue::Color(c) => c,
                CssValue::Keyword(ref k) if k.eq_ignore_ascii_case("currentcolor") => current_color,
                _ => current_color,
            }
        }
        None => current_color,
    };
    set_border_color(style, property, color);
}

/// Resolve a [`CssValue::Number`] to a non-negative `f32`.
fn resolve_non_negative_f32(value: &CssValue, default: f32) -> f32 {
    match value {
        CssValue::Number(n) => n.max(0.0),
        _ => default,
    }
}

/// Resolve a [`CssValue::Number`] to an `i32`.
fn resolve_i32(value: &CssValue, default: i32) -> i32 {
    match value {
        #[allow(clippy::cast_possible_truncation)]
        CssValue::Number(n) => *n as i32,
        _ => default,
    }
}

/// Resolve a [`CssValue`] to a pixel value (for padding/border-width).
///
/// Percentage values are not yet supported (Phase 2) and resolve to `0.0`.
fn resolve_to_px(value: &CssValue, ctx: &ResolveContext) -> f32 {
    match value {
        CssValue::Length(v, unit) => resolve_length(*v, *unit, ctx),
        // TODO(Phase 2): resolve CssValue::Percentage against containing block width
        CssValue::Number(n) if *n == 0.0 => 0.0,
        _ => 0.0,
    }
}

/// If the value is `inherit`/`initial`/`unset`, resolve it; otherwise clone.
fn resolve_keyword_or_clone(
    property: &str,
    value: &CssValue,
    parent_style: &ComputedStyle,
) -> CssValue {
    resolve_keyword(property, value, parent_style).unwrap_or_else(|| value.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn default_ctx() -> ResolveContext {
        ResolveContext {
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            em_base: 16.0,
            root_font_size: 16.0,
        }
    }

    #[test]
    fn resolve_px() {
        let ctx = default_ctx();
        assert_eq!(resolve_length(10.0, LengthUnit::Px, &ctx), 10.0);
    }

    #[test]
    fn resolve_em() {
        let ctx = ResolveContext {
            em_base: 20.0,
            ..default_ctx()
        };
        assert_eq!(resolve_length(2.0, LengthUnit::Em, &ctx), 40.0);
    }

    #[test]
    fn resolve_rem() {
        let ctx = ResolveContext {
            root_font_size: 18.0,
            ..default_ctx()
        };
        assert_eq!(resolve_length(2.0, LengthUnit::Rem, &ctx), 36.0);
    }

    #[test]
    fn resolve_vw() {
        let ctx = default_ctx();
        assert_eq!(resolve_length(50.0, LengthUnit::Vw, &ctx), 960.0);
    }

    #[test]
    fn resolve_vh() {
        let ctx = default_ctx();
        assert_eq!(resolve_length(50.0, LengthUnit::Vh, &ctx), 540.0);
    }

    #[test]
    fn resolve_vmin_vmax() {
        let ctx = default_ctx(); // 1920x1080
        assert_eq!(resolve_length(10.0, LengthUnit::Vmin, &ctx), 108.0);
        assert_eq!(resolve_length(10.0, LengthUnit::Vmax, &ctx), 192.0);
    }

    #[test]
    fn font_size_keywords() {
        assert_eq!(resolve_font_size_keyword("medium", 16.0), Some(16.0));
        assert_eq!(resolve_font_size_keyword("xx-small", 16.0), Some(9.0));
        assert_eq!(resolve_font_size_keyword("xx-large", 16.0), Some(32.0));
        assert_eq!(resolve_font_size_keyword("unknown", 16.0), None);
    }

    #[test]
    fn font_size_smaller_larger() {
        let smaller = resolve_font_size_keyword("smaller", 20.0).unwrap();
        assert!((smaller - 16.66).abs() < 0.1);
        let larger = resolve_font_size_keyword("larger", 20.0).unwrap();
        assert_eq!(larger, 24.0);
    }

    #[test]
    fn font_size_em_uses_parent() {
        let parent = ComputedStyle {
            font_size: 20.0,
            ..ComputedStyle::default()
        };
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let val = CssValue::Length(2.0, LengthUnit::Em);
        winners.insert("font-size", &val);
        let fs = resolve_font_size(&winners, &parent, &ctx);
        assert_eq!(fs, 40.0);
    }

    #[test]
    fn font_size_percentage() {
        let parent = ComputedStyle {
            font_size: 20.0,
            ..ComputedStyle::default()
        };
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let val = CssValue::Percentage(150.0);
        winners.insert("font-size", &val);
        let fs = resolve_font_size(&winners, &parent, &ctx);
        assert_eq!(fs, 30.0);
    }

    #[test]
    fn currentcolor_resolution() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let red = CssValue::Color(CssColor::RED);
        winners.insert("color", &red);
        let style = build_computed_style(&winners, &parent, &ctx);
        // border-*-color initial = currentcolor → should be RED
        assert_eq!(style.border_top_color, CssColor::RED);
    }

    #[test]
    fn border_width_zero_when_style_none() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let width = CssValue::Length(5.0, LengthUnit::Px);
        // style is none (no border-top-style set, default = none)
        winners.insert("border-top-width", &width);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.border_top_width, 0.0);
    }

    #[test]
    fn border_width_preserved_when_style_solid() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let width = CssValue::Length(5.0, LengthUnit::Px);
        let solid = CssValue::Keyword("solid".to_string());
        winners.insert("border-top-width", &width);
        winners.insert("border-top-style", &solid);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.border_top_width, 5.0);
    }

    #[test]
    fn inherit_keyword_resolves_to_parent() {
        let parent = ComputedStyle {
            display: Display::Block,
            ..ComputedStyle::default()
        };
        let resolved = resolve_keyword_or_clone("display", &CssValue::Inherit, &parent);
        assert_eq!(resolved, CssValue::Keyword("block".to_string()));
    }

    #[test]
    fn unset_inherited_uses_parent() {
        let parent = ComputedStyle {
            color: CssColor::RED,
            ..ComputedStyle::default()
        };
        let resolved = resolve_keyword_or_clone("color", &CssValue::Unset, &parent);
        assert_eq!(resolved, CssValue::Color(CssColor::RED));
    }

    #[test]
    fn unset_non_inherited_uses_initial() {
        let parent = ComputedStyle {
            display: Display::Block,
            ..ComputedStyle::default()
        };
        let resolved = resolve_keyword_or_clone("display", &CssValue::Unset, &parent);
        assert_eq!(resolved, CssValue::Keyword("inline".to_string()));
    }

    #[test]
    fn resolve_flex_direction() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let val = CssValue::Keyword("column-reverse".to_string());
        winners.insert("flex-direction", &val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.flex_direction, FlexDirection::ColumnReverse);
    }

    #[test]
    fn resolve_flex_grow_shrink_clamping() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let grow = CssValue::Number(-5.0);
        let shrink = CssValue::Number(3.0);
        winners.insert("flex-grow", &grow);
        winners.insert("flex-shrink", &shrink);
        let style = build_computed_style(&winners, &parent, &ctx);
        // Negative clamped to 0.
        assert_eq!(style.flex_grow, 0.0);
        assert_eq!(style.flex_shrink, 3.0);
    }

    #[test]
    fn resolve_flex_properties() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let wrap = CssValue::Keyword("wrap".to_string());
        let justify = CssValue::Keyword("center".to_string());
        let align = CssValue::Keyword("flex-end".to_string());
        let basis = CssValue::Length(100.0, LengthUnit::Px);
        let order = CssValue::Number(2.0);
        winners.insert("flex-wrap", &wrap);
        winners.insert("justify-content", &justify);
        winners.insert("align-items", &align);
        winners.insert("flex-basis", &basis);
        winners.insert("order", &order);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.flex_wrap, FlexWrap::Wrap);
        assert_eq!(style.justify_content, JustifyContent::Center);
        assert_eq!(style.align_items, AlignItems::FlexEnd);
        assert_eq!(style.flex_basis, Dimension::Length(100.0));
        assert_eq!(style.order, 2);
    }

    #[test]
    fn resolve_flex_defaults() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let winners: HashMap<&str, &CssValue> = HashMap::new();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.flex_direction, FlexDirection::Row);
        assert_eq!(style.flex_wrap, FlexWrap::Nowrap);
        assert_eq!(style.justify_content, JustifyContent::FlexStart);
        assert_eq!(style.align_items, AlignItems::Stretch);
        assert_eq!(style.align_content, AlignContent::Stretch);
        assert_eq!(style.flex_grow, 0.0);
        assert_eq!(style.flex_shrink, 1.0);
        assert_eq!(style.flex_basis, Dimension::Auto);
        assert_eq!(style.order, 0);
        assert_eq!(style.align_self, AlignSelf::Auto);
    }
}
