//! CSS value resolution: relative units → absolute pixels.
//!
//! Converts parsed [`CssValue`]s into concrete values for [`ComputedStyle`]
//! fields, resolving relative lengths, font-size keywords, `currentcolor`,
//! and the border-width/border-style interaction.

use elidex_plugin::{
    BorderStyle, ComputedStyle, CssColor, CssValue, Dimension, Display, LengthUnit, Position,
};

use crate::inherit::{get_initial_value, is_inherited};

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
pub(crate) fn resolve_keyword(
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
        "display" => CssValue::Keyword(display_to_string(style.display).to_string()),
        "position" => CssValue::Keyword(position_to_string(style.position).to_string()),
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
        "border-top-style" => {
            CssValue::Keyword(border_style_to_string(style.border_top_style).to_string())
        }
        "border-right-style" => {
            CssValue::Keyword(border_style_to_string(style.border_right_style).to_string())
        }
        "border-bottom-style" => {
            CssValue::Keyword(border_style_to_string(style.border_bottom_style).to_string())
        }
        "border-left-style" => {
            CssValue::Keyword(border_style_to_string(style.border_left_style).to_string())
        }
        "border-top-color" => CssValue::Color(style.border_top_color),
        "border-right-color" => CssValue::Color(style.border_right_color),
        "border-bottom-color" => CssValue::Color(style.border_bottom_color),
        "border-left-color" => CssValue::Color(style.border_left_color),
        _ => get_initial_value(property),
    }
}

fn display_to_string(d: Display) -> &'static str {
    match d {
        Display::Block => "block",
        Display::Inline => "inline",
        Display::InlineBlock => "inline-block",
        Display::None => "none",
        Display::Flex => "flex",
    }
}

fn position_to_string(p: Position) -> &'static str {
    match p {
        Position::Static => "static",
        Position::Relative => "relative",
        Position::Absolute => "absolute",
        Position::Fixed => "fixed",
    }
}

fn border_style_to_string(s: BorderStyle) -> &'static str {
    match s {
        BorderStyle::None => "none",
        BorderStyle::Solid => "solid",
        BorderStyle::Dashed => "dashed",
        BorderStyle::Dotted => "dotted",
    }
}

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
pub(crate) fn build_computed_style(
    winners: &std::collections::HashMap<&str, &CssValue>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) -> ComputedStyle {
    let mut style = ComputedStyle::default();

    // Phase 1: resolve font-size (needed by em units in other properties).
    let element_font_size = resolve_font_size(winners, parent_style, ctx);
    style.font_size = element_font_size;

    // Create an element-level context with the resolved font-size.
    let elem_ctx = ResolveContext {
        viewport_width: ctx.viewport_width,
        viewport_height: ctx.viewport_height,
        em_base: element_font_size,
        root_font_size: ctx.root_font_size,
    };

    // Phase 2: resolve color (needed by currentcolor).
    style.color = resolve_color(winners, parent_style);

    // Phase 3: resolve all other properties.
    resolve_font_family(&mut style, winners, parent_style);
    resolve_display(&mut style, winners, parent_style);
    resolve_position(&mut style, winners, parent_style);
    resolve_background_color(&mut style, winners, parent_style);
    resolve_dimension_prop(&mut style, "width", winners, parent_style, &elem_ctx);
    resolve_dimension_prop(&mut style, "height", winners, parent_style, &elem_ctx);
    resolve_dimension_prop(&mut style, "margin-top", winners, parent_style, &elem_ctx);
    resolve_dimension_prop(&mut style, "margin-right", winners, parent_style, &elem_ctx);
    resolve_dimension_prop(&mut style, "margin-bottom", winners, parent_style, &elem_ctx);
    resolve_dimension_prop(&mut style, "margin-left", winners, parent_style, &elem_ctx);
    resolve_padding_prop(&mut style, "padding-top", winners, parent_style, &elem_ctx);
    resolve_padding_prop(&mut style, "padding-right", winners, parent_style, &elem_ctx);
    resolve_padding_prop(&mut style, "padding-bottom", winners, parent_style, &elem_ctx);
    resolve_padding_prop(&mut style, "padding-left", winners, parent_style, &elem_ctx);

    // Border styles must be resolved before border widths (width = 0 when style = none).
    resolve_border_style_prop(&mut style, "border-top-style", winners, parent_style);
    resolve_border_style_prop(&mut style, "border-right-style", winners, parent_style);
    resolve_border_style_prop(&mut style, "border-bottom-style", winners, parent_style);
    resolve_border_style_prop(&mut style, "border-left-style", winners, parent_style);

    resolve_border_width_prop(&mut style, "border-top-width", winners, parent_style, &elem_ctx);
    resolve_border_width_prop(
        &mut style,
        "border-right-width",
        winners,
        parent_style,
        &elem_ctx,
    );
    resolve_border_width_prop(
        &mut style,
        "border-bottom-width",
        winners,
        parent_style,
        &elem_ctx,
    );
    resolve_border_width_prop(
        &mut style,
        "border-left-width",
        winners,
        parent_style,
        &elem_ctx,
    );

    resolve_border_color_prop(&mut style, "border-top-color", winners, parent_style);
    resolve_border_color_prop(&mut style, "border-right-color", winners, parent_style);
    resolve_border_color_prop(&mut style, "border-bottom-color", winners, parent_style);
    resolve_border_color_prop(&mut style, "border-left-color", winners, parent_style);

    style
}

// --- Individual property resolvers ---

fn resolve_font_size(
    winners: &std::collections::HashMap<&str, &CssValue>,
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
            let font_ctx = ResolveContext {
                viewport_width: ctx.viewport_width,
                viewport_height: ctx.viewport_height,
                em_base: parent_style.font_size,
                root_font_size: ctx.root_font_size,
            };
            resolve_length(*v, *unit, &font_ctx)
        }
        CssValue::Percentage(p) => parent_style.font_size * p / 100.0,
        CssValue::Keyword(kw) => {
            resolve_font_size_keyword(kw, parent_style.font_size).unwrap_or(parent_style.font_size)
        }
        _ => parent_style.font_size,
    }
}

fn resolve_color(
    winners: &std::collections::HashMap<&str, &CssValue>,
    parent_style: &ComputedStyle,
) -> CssColor {
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
    winners: &std::collections::HashMap<&str, &CssValue>,
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

fn resolve_display(
    style: &mut ComputedStyle,
    winners: &std::collections::HashMap<&str, &CssValue>,
    parent_style: &ComputedStyle,
) {
    let Some(value) = winners.get("display") else {
        return; // Non-inherited: keep default.
    };
    let value = resolve_keyword_or_clone("display", value, parent_style);
    style.display = match value {
        CssValue::Keyword(ref k) => match k.as_str() {
            "block" => Display::Block,
            "inline" => Display::Inline,
            "inline-block" => Display::InlineBlock,
            "none" => Display::None,
            "flex" => Display::Flex,
            _ => Display::default(),
        },
        _ => Display::default(),
    };
}

fn resolve_position(
    style: &mut ComputedStyle,
    winners: &std::collections::HashMap<&str, &CssValue>,
    parent_style: &ComputedStyle,
) {
    let Some(value) = winners.get("position") else {
        return;
    };
    let value = resolve_keyword_or_clone("position", value, parent_style);
    style.position = match value {
        CssValue::Keyword(ref k) => match k.as_str() {
            "static" => Position::Static,
            "relative" => Position::Relative,
            "absolute" => Position::Absolute,
            "fixed" => Position::Fixed,
            _ => Position::default(),
        },
        _ => Position::default(),
    };
}

fn resolve_background_color(
    style: &mut ComputedStyle,
    winners: &std::collections::HashMap<&str, &CssValue>,
    parent_style: &ComputedStyle,
) {
    let Some(value) = winners.get("background-color") else {
        return;
    };
    let value = resolve_keyword_or_clone("background-color", value, parent_style);
    if let CssValue::Color(c) = value {
        style.background_color = c;
    }
}

fn resolve_dimension_prop(
    style: &mut ComputedStyle,
    property: &str,
    winners: &std::collections::HashMap<&str, &CssValue>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    let Some(value) = winners.get(property) else {
        return;
    };
    let value = resolve_keyword_or_clone(property, value, parent_style);
    let dim = resolve_dimension(&value, ctx);
    match property {
        "width" => style.width = dim,
        "height" => style.height = dim,
        "margin-top" => style.margin_top = dim,
        "margin-right" => style.margin_right = dim,
        "margin-bottom" => style.margin_bottom = dim,
        "margin-left" => style.margin_left = dim,
        _ => {}
    }
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

fn resolve_padding_prop(
    style: &mut ComputedStyle,
    property: &str,
    winners: &std::collections::HashMap<&str, &CssValue>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    let Some(value) = winners.get(property) else {
        return;
    };
    let value = resolve_keyword_or_clone(property, value, parent_style);
    let px = resolve_to_px(&value, ctx);
    match property {
        "padding-top" => style.padding_top = px,
        "padding-right" => style.padding_right = px,
        "padding-bottom" => style.padding_bottom = px,
        "padding-left" => style.padding_left = px,
        _ => {}
    }
}

fn resolve_border_style_prop(
    style: &mut ComputedStyle,
    property: &str,
    winners: &std::collections::HashMap<&str, &CssValue>,
    parent_style: &ComputedStyle,
) {
    let Some(value) = winners.get(property) else {
        return;
    };
    let value = resolve_keyword_or_clone(property, value, parent_style);
    let bs = match value {
        CssValue::Keyword(ref k) => match k.as_str() {
            "none" => BorderStyle::None,
            "solid" => BorderStyle::Solid,
            "dashed" => BorderStyle::Dashed,
            "dotted" => BorderStyle::Dotted,
            _ => BorderStyle::default(),
        },
        _ => BorderStyle::default(),
    };
    match property {
        "border-top-style" => style.border_top_style = bs,
        "border-right-style" => style.border_right_style = bs,
        "border-bottom-style" => style.border_bottom_style = bs,
        "border-left-style" => style.border_left_style = bs,
        _ => {}
    }
}

fn resolve_border_width_prop(
    style: &mut ComputedStyle,
    property: &str,
    winners: &std::collections::HashMap<&str, &CssValue>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    // CSS spec: computed border-width is 0 when border-style is none.
    let border_style = match property {
        "border-top-width" => style.border_top_style,
        "border-right-width" => style.border_right_style,
        "border-bottom-width" => style.border_bottom_style,
        "border-left-width" => style.border_left_style,
        _ => BorderStyle::None,
    };

    let px = if border_style == BorderStyle::None {
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

    match property {
        "border-top-width" => style.border_top_width = px,
        "border-right-width" => style.border_right_width = px,
        "border-bottom-width" => style.border_bottom_width = px,
        "border-left-width" => style.border_left_width = px,
        _ => {}
    }
}

fn resolve_border_color_prop(
    style: &mut ComputedStyle,
    property: &str,
    winners: &std::collections::HashMap<&str, &CssValue>,
    parent_style: &ComputedStyle,
) {
    let color = match winners.get(property) {
        Some(value) => {
            let value = resolve_keyword_or_clone(property, value, parent_style);
            match value {
                CssValue::Color(c) => c,
                CssValue::Keyword(ref k) if k == "currentcolor" => style.color,
                _ => style.color, // fallback to currentcolor behavior
            }
        }
        None => style.color, // initial = currentcolor
    };

    match property {
        "border-top-color" => style.border_top_color = color,
        "border-right-color" => style.border_right_color = color,
        "border-bottom-color" => style.border_bottom_color = color,
        "border-left-color" => style.border_left_color = color,
        _ => {}
    }
}

/// Resolve a [`CssValue`] to a pixel value (for padding/border-width).
fn resolve_to_px(value: &CssValue, ctx: &ResolveContext) -> f32 {
    match value {
        CssValue::Length(v, unit) => resolve_length(*v, *unit, ctx),
        CssValue::Percentage(p) => *p, // percentages kept as-is for now (resolved in layout)
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
        let resolved = resolve_keyword("display", &CssValue::Inherit, &parent).unwrap();
        assert_eq!(resolved, CssValue::Keyword("block".to_string()));
    }

    #[test]
    fn unset_inherited_uses_parent() {
        let parent = ComputedStyle {
            color: CssColor::RED,
            ..ComputedStyle::default()
        };
        let resolved = resolve_keyword("color", &CssValue::Unset, &parent).unwrap();
        assert_eq!(resolved, CssValue::Color(CssColor::RED));
    }

    #[test]
    fn unset_non_inherited_uses_initial() {
        let parent = ComputedStyle {
            display: Display::Block,
            ..ComputedStyle::default()
        };
        let resolved = resolve_keyword("display", &CssValue::Unset, &parent).unwrap();
        assert_eq!(resolved, CssValue::Keyword("inline".to_string()));
    }
}
