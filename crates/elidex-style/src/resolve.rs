//! CSS value resolution: relative units → absolute pixels.
//!
//! Converts parsed [`CssValue`]s into concrete values for [`ComputedStyle`]
//! fields, resolving relative lengths, font-size keywords, `currentcolor`,
//! and the border-width/border-style interaction.

use elidex_plugin::{
    BorderStyle, ComputedStyle, CssColor, CssValue, Dimension, Display, LengthUnit, Position,
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
// Sync: When adding a property, also update build_computed_style and get_initial_value.
fn get_computed_as_css_value(property: &str, style: &ComputedStyle) -> CssValue {
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
        _ => get_initial_value(property),
    }
}

// Display, Position, and BorderStyle implement AsRef<str> in elidex-plugin,
// so enum-to-string conversion is handled via .as_ref() directly.

fn dimension_to_css_value(d: Dimension) -> CssValue {
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

    // Phase 1: resolve font-size (needed by em units in other properties).
    let element_font_size = resolve_font_size(winners, parent_style, ctx);
    style.font_size = element_font_size;

    // Create an element-level context with the resolved font-size.
    let elem_ctx = ctx.with_em_base(element_font_size);

    // Phase 2: resolve color (needed by currentcolor).
    style.color = resolve_color(winners, parent_style);

    // Phase 3: resolve all other properties.
    resolve_font_family(&mut style, winners, parent_style);
    resolve_display(&mut style, winners, parent_style);
    resolve_position(&mut style, winners, parent_style);
    resolve_background_color(&mut style, winners, parent_style);
    // Dimension properties (width/height/margin).
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

    // Padding properties.
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

    // Border styles must be resolved before border widths (width = 0 when style = none).
    let bs = |v: &CssValue| resolve_border_style_value(v);
    for (prop, setter) in [
        (
            "border-top-style",
            (|s: &mut ComputedStyle, v| s.border_top_style = v)
                as fn(&mut ComputedStyle, BorderStyle),
        ),
        ("border-right-style", |s, v| s.border_right_style = v),
        ("border-bottom-style", |s, v| s.border_bottom_style = v),
        ("border-left-style", |s, v| s.border_left_style = v),
    ] {
        resolve_prop(prop, winners, parent_style, bs, |v| setter(&mut style, v));
    }

    // Border widths (special: 0 when style = none).
    for prop in &[
        "border-top-width",
        "border-right-width",
        "border-bottom-width",
        "border-left-width",
    ] {
        resolve_border_width_prop(&mut style, prop, winners, parent_style, &elem_ctx);
    }

    // Border colors (initial = currentcolor).
    let current_color = style.color;
    for prop in &[
        "border-top-color",
        "border-right-color",
        "border-bottom-color",
        "border-left-color",
    ] {
        resolve_border_color_prop(&mut style, prop, winners, parent_style, current_color);
    }

    style
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
    if let Some(d) = resolve_keyword_enum("display", winners, parent_style, |k| match k {
        "block" => Some(Display::Block),
        "inline" => Some(Display::Inline),
        "inline-block" => Some(Display::InlineBlock),
        "none" => Some(Display::None),
        "flex" => Some(Display::Flex),
        _ => None,
    }) {
        style.display = d;
    }
}

fn resolve_position(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    if let Some(p) = resolve_keyword_enum("position", winners, parent_style, |k| match k {
        "static" => Some(Position::Static),
        "relative" => Some(Position::Relative),
        "absolute" => Some(Position::Absolute),
        "fixed" => Some(Position::Fixed),
        _ => None,
    }) {
        style.position = p;
    }
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
}
