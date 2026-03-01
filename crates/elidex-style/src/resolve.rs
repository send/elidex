//! CSS value resolution: relative units → absolute pixels.
//!
//! Converts parsed [`CssValue`]s into concrete values for [`ComputedStyle`]
//! fields, resolving relative lengths, font-size keywords, `currentcolor`,
//! and the border-width/border-style interaction.

use std::collections::{HashMap, HashSet};

use elidex_plugin::{
    AlignContent, AlignItems, AlignSelf, BorderStyle, ComputedStyle, CssColor, CssValue, Dimension,
    Display, FlexDirection, FlexWrap, JustifyContent, LengthUnit, LineHeight, Position,
    TextDecorationLine, TextTransform,
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
    // Custom properties: return the raw token string.
    if property.starts_with("--") {
        return match style.custom_properties.get(property) {
            Some(raw) => CssValue::RawTokens(raw.clone()),
            None => CssValue::RawTokens(String::new()),
        };
    }

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
        "font-weight" => CssValue::Number(f32::from(style.font_weight)),
        "line-height" => match style.line_height {
            LineHeight::Normal => CssValue::Keyword("normal".to_string()),
            LineHeight::Number(n) => CssValue::Number(n),
            LineHeight::Px(px) => CssValue::Length(px, LengthUnit::Px),
        },
        "text-transform" => CssValue::Keyword(style.text_transform.as_ref().to_string()),
        "text-decoration-line" => {
            let d = &style.text_decoration_line;
            if d.underline && d.line_through {
                CssValue::List(vec![
                    CssValue::Keyword("underline".to_string()),
                    CssValue::Keyword("line-through".to_string()),
                ])
            } else {
                CssValue::Keyword(d.to_string())
            }
        }
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

    // --- Phase 1: Build custom properties map (inherit from parent + override) ---
    let custom_props = build_custom_properties(winners, parent_style);
    style.custom_properties = custom_props;

    // --- Phase 2: Resolve var() references in winners ---
    let resolved_winners = resolve_var_references(winners, &style.custom_properties);
    let winners = merge_winners(winners, &resolved_winners);

    // --- Font properties (inherit by default) ---
    // Font-size must be resolved first: em units in all other properties depend on it.
    let element_font_size = resolve_font_size(&winners, parent_style, ctx);
    style.font_size = element_font_size;
    let elem_ctx = ctx.with_em_base(element_font_size);
    resolve_font_weight(&mut style, &winners, parent_style);
    resolve_font_family(&mut style, &winners, parent_style);
    resolve_line_height(&mut style, &winners, parent_style, &elem_ctx);
    resolve_text_transform(&mut style, &winners, parent_style);

    // --- Text decoration (non-inherited) ---
    resolve_text_decoration_line(&mut style, &winners, parent_style);

    // --- Color properties ---
    // Color must precede border-color (initial = currentcolor) and background-color.
    style.color = resolve_color(&winners, parent_style);
    resolve_background_color(&mut style, &winners, parent_style);

    // --- Display & positioning ---
    resolve_display(&mut style, &winners, parent_style);
    resolve_position(&mut style, &winners, parent_style);

    // --- Box model: dimensions, margin, padding ---
    let dim = |v: &CssValue| resolve_dimension(v, &elem_ctx);
    resolve_prop("width", &winners, parent_style, dim, |d| style.width = d);
    resolve_prop("height", &winners, parent_style, dim, |d| {
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
        resolve_prop(prop, &winners, parent_style, dim, |d| setter(&mut style, d));
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
        resolve_prop(prop, &winners, parent_style, px, |v| setter(&mut style, v));
    }

    // --- Border properties (style → width → color) ---
    resolve_border_properties(&mut style, &winners, parent_style, &elem_ctx);

    // --- Flex properties ---
    resolve_flex_properties(&mut style, &winners, parent_style, dim);

    style
}

// --- Custom property resolution ---

/// Build the custom properties map: inherit all from parent, then override
/// with any custom properties declared on this element.
///
/// Per CSS Variables Level 1:
/// - `RawTokens`: set the property to the raw value.
/// - `Initial`: remove the property (custom properties have no initial value;
///   their initial value is the "guaranteed-invalid value").
/// - `Inherit`/`Unset`: keep the inherited value (custom properties are
///   always inherited, so both behave as `inherit` — already handled by the
///   parent clone).
fn build_custom_properties(
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) -> HashMap<String, String> {
    let mut props = parent_style.custom_properties.clone();
    for (name, value) in winners {
        if name.starts_with("--") {
            match value {
                CssValue::RawTokens(raw) => {
                    props.insert((*name).to_string(), raw.clone());
                }
                CssValue::Initial => {
                    props.remove(*name);
                }
                // Inherit/Unset: keep inherited value (already cloned from parent).
                _ => {}
            }
        }
    }
    props
}

/// Maximum recursion depth for resolving `var()` references (cycle protection).
const MAX_VAR_DEPTH: usize = 32;

/// Resolve all `CssValue::Var` references in the winners map.
///
/// Returns a map of property name → resolved `CssValue` for properties that
/// had `var()` references.
fn resolve_var_references(
    winners: &PropertyMap<'_>,
    custom_props: &HashMap<String, String>,
) -> HashMap<String, CssValue> {
    let mut resolved = HashMap::new();
    for (name, value) in winners {
        if name.starts_with("--") {
            continue; // Custom properties themselves don't get var-resolved here.
        }
        if let CssValue::Var(..) = value {
            let mut visited = HashSet::new();
            if let Some(val) = resolve_var_value(value, custom_props, 0, &mut visited) {
                resolved.insert((*name).to_string(), val);
            }
        }
    }
    resolved
}

/// Resolve a single `CssValue::Var` to a concrete value.
///
/// Uses both depth limiting and a visited set for cycle detection.
/// If a custom property name is already in the visited set, the reference
/// is circular and resolution fails (returns `None`).
#[must_use]
fn resolve_var_value(
    value: &CssValue,
    custom_props: &HashMap<String, String>,
    depth: usize,
    visited: &mut HashSet<String>,
) -> Option<CssValue> {
    if depth > MAX_VAR_DEPTH {
        return None;
    }

    let CssValue::Var(ref name, ref fallback) = value else {
        return Some(value.clone());
    };

    // Cycle detection: if we've already visited this property, bail out.
    if !visited.insert(name.clone()) {
        return None;
    }

    // Look up the custom property.
    if let Some(raw) = custom_props.get(name) {
        // The raw value itself might contain var() references.
        let parsed = parse_raw_value(raw);
        if let CssValue::Var(..) = &parsed {
            // Recursively resolve nested var().
            let result = resolve_var_value(&parsed, custom_props, depth + 1, visited);
            visited.remove(name);
            return result;
        }
        visited.remove(name);
        return Some(parsed);
    }

    visited.remove(name);

    // Property not found: use fallback if available.
    match fallback {
        Some(fb) => {
            if let CssValue::Var(..) = fb.as_ref() {
                resolve_var_value(fb, custom_props, depth + 1, visited)
            } else {
                Some(*fb.clone())
            }
        }
        None => None, // No fallback, var() unresolvable.
    }
}

/// Parse a raw token string into a typed [`CssValue`].
///
/// Delegates to `elidex_css::parse_raw_token_value()` which uses cssparser
/// internally.
fn parse_raw_value(raw: &str) -> CssValue {
    elidex_css::parse_raw_token_value(raw)
}

/// Merge resolved `var()` values back into the winners map.
fn merge_winners<'a>(
    original: &PropertyMap<'a>,
    resolved: &'a HashMap<String, CssValue>,
) -> HashMap<&'a str, &'a CssValue> {
    let mut merged: HashMap<&str, &CssValue> = HashMap::new();
    // Copy originals, replacing var()-resolved entries and dropping unresolved
    // Var values. Unresolved var() should make the property "invalid at
    // computed-value time" (CSS Variables Level 1 §5), which means downstream
    // resolvers see no entry and apply inherited/initial as appropriate.
    for (name, value) in original {
        if name.starts_with("--") {
            continue;
        }
        if let Some(resolved_val) = resolved.get(*name) {
            merged.insert(name, resolved_val);
        } else if !matches!(value, CssValue::Var(..)) {
            merged.insert(name, value);
        }
        // Unresolved CssValue::Var is intentionally dropped.
    }
    merged
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

fn resolve_font_weight(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    let Some(value) = winners.get("font-weight") else {
        style.font_weight = parent_style.font_weight;
        return;
    };
    let value = resolve_keyword_or_clone("font-weight", value, parent_style);
    style.font_weight = match &value {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        CssValue::Number(n) => n.round().clamp(1.0, 1000.0) as u16,
        CssValue::Keyword(k) => match k.as_str() {
            "normal" => 400,
            "bold" => 700,
            _ => parent_style.font_weight,
        },
        _ => parent_style.font_weight,
    };
}

fn resolve_line_height(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    let Some(value) = winners.get("line-height") else {
        style.line_height = parent_style.line_height;
        return;
    };
    let value = resolve_keyword_or_clone("line-height", value, parent_style);
    style.line_height = match &value {
        CssValue::Keyword(k) if k == "normal" => LineHeight::Normal,
        // Unitless number: inherited as-is, recomputed per element's font-size.
        CssValue::Number(n) => LineHeight::Number(*n),
        // Absolute length: resolve to px.
        CssValue::Length(v, unit) => LineHeight::Px(resolve_length(*v, *unit, ctx)),
        // Percentage: resolve to px (relative to element's font-size).
        CssValue::Percentage(p) => LineHeight::Px(style.font_size * p / 100.0),
        _ => parent_style.line_height,
    };
}

fn resolve_text_transform(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    let Some(value) = winners.get("text-transform") else {
        // Inherited: use parent's value.
        style.text_transform = parent_style.text_transform;
        return;
    };
    let value = resolve_keyword_or_clone("text-transform", value, parent_style);
    style.text_transform = match value {
        CssValue::Keyword(ref k) => match k.as_str() {
            "none" => TextTransform::None,
            "uppercase" => TextTransform::Uppercase,
            "lowercase" => TextTransform::Lowercase,
            "capitalize" => TextTransform::Capitalize,
            _ => parent_style.text_transform,
        },
        _ => parent_style.text_transform,
    };
}

fn resolve_text_decoration_line(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    let Some(value) = winners.get("text-decoration-line") else {
        // Non-inherited: keep default (none). Don't inherit from parent.
        return;
    };
    let value = resolve_keyword_or_clone("text-decoration-line", value, parent_style);
    style.text_decoration_line = match &value {
        CssValue::Keyword(k) => match k.as_str() {
            "underline" => TextDecorationLine {
                underline: true,
                line_through: false,
            },
            "line-through" => TextDecorationLine {
                underline: false,
                line_through: true,
            },
            // "none" and unrecognized keywords.
            _ => TextDecorationLine::default(),
        },
        CssValue::List(items) => {
            let mut result = TextDecorationLine::default();
            for item in items {
                if let CssValue::Keyword(k) = item {
                    match k.as_str() {
                        "underline" => result.underline = true,
                        "line-through" => result.line_through = true,
                        _ => {}
                    }
                }
            }
            result
        }
        _ => TextDecorationLine::default(),
    };
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

    // --- Custom property + var() resolution tests (M3-0) ---

    #[test]
    fn custom_properties_from_winners() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let raw = CssValue::RawTokens("#0d1117".into());
        winners.insert("--bg", &raw);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(
            style.custom_properties.get("--bg"),
            Some(&"#0d1117".to_string())
        );
    }

    #[test]
    fn custom_properties_inherited_from_parent() {
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--text-color".into(), "#e6edf3".into());
        let ctx = default_ctx();
        let winners: HashMap<&str, &CssValue> = HashMap::new();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(
            style.custom_properties.get("--text-color"),
            Some(&"#e6edf3".to_string())
        );
    }

    #[test]
    fn custom_property_override() {
        let mut parent = ComputedStyle::default();
        parent.custom_properties.insert("--bg".into(), "red".into());
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let raw = CssValue::RawTokens("blue".into());
        winners.insert("--bg", &raw);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(
            style.custom_properties.get("--bg"),
            Some(&"blue".to_string())
        );
    }

    #[test]
    fn custom_property_initial_removes_inherited() {
        // `--bg: initial` should remove the inherited value.
        let mut parent = ComputedStyle::default();
        parent.custom_properties.insert("--bg".into(), "red".into());
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let initial = CssValue::Initial;
        winners.insert("--bg", &initial);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.custom_properties.get("--bg"), None);
    }

    #[test]
    fn unresolved_var_treated_as_invalid() {
        // An unresolvable var() should be treated as "invalid at computed-value
        // time" — the property falls back to inherited/initial, not to Var.
        let parent = ComputedStyle {
            display: Display::Block,
            ..ComputedStyle::default()
        };
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let var_val = CssValue::Var("--undefined".into(), None);
        // display is non-inherited, so invalid → initial (Inline).
        winners.insert("display", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.display, Display::Inline);
    }

    #[test]
    fn var_resolution_color() {
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--text".into(), "#ff0000".into());
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let var_val = CssValue::Var("--text".into(), None);
        winners.insert("color", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.color, CssColor::RED);
    }

    #[test]
    fn var_resolution_with_fallback() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let var_val = CssValue::Var(
            "--undefined".into(),
            Some(Box::new(CssValue::Color(CssColor::BLUE))),
        );
        winners.insert("color", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.color, CssColor::BLUE);
    }

    #[test]
    fn var_resolution_undefined_no_fallback() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let var_val = CssValue::Var("--undefined".into(), None);
        winners.insert("color", &var_val);
        // Undefined with no fallback → color stays at parent/default value.
        let style = build_computed_style(&winners, &parent, &ctx);
        // color inherits from parent (BLACK by default).
        assert_eq!(style.color, CssColor::BLACK);
    }

    #[test]
    fn var_resolution_background_color() {
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--bg".into(), "#0d1117".into());
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let var_val = CssValue::Var("--bg".into(), None);
        winners.insert("background-color", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.background_color, CssColor::new(0x0d, 0x11, 0x17, 255));
    }

    #[test]
    fn var_resolution_display() {
        let mut parent = ComputedStyle::default();
        parent.custom_properties.insert("--d".into(), "flex".into());
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let var_val = CssValue::Var("--d".into(), None);
        winners.insert("display", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.display, Display::Flex);
    }

    #[test]
    fn parse_raw_value_color() {
        let val = parse_raw_value("#ff0000");
        assert_eq!(val, CssValue::Color(CssColor::RED));
    }

    #[test]
    fn parse_raw_value_keyword() {
        let val = parse_raw_value("block");
        assert_eq!(val, CssValue::Keyword("block".into()));
    }

    #[test]
    fn parse_raw_value_length() {
        let val = parse_raw_value("16px");
        assert_eq!(val, CssValue::Length(16.0, LengthUnit::Px));
    }

    #[test]
    fn parse_raw_value_multi_token() {
        let val = parse_raw_value("\"Courier New\", monospace");
        assert!(matches!(val, CssValue::RawTokens(_)));
    }

    #[test]
    fn var_circular_reference_returns_none() {
        // --a references --b, --b references --a → cycle.
        let mut custom_props = HashMap::new();
        custom_props.insert("--a".into(), "var(--b)".into());
        custom_props.insert("--b".into(), "var(--a)".into());

        let var_a = CssValue::Var("--a".into(), None);
        let mut visited = HashSet::new();
        let result = resolve_var_value(&var_a, &custom_props, 0, &mut visited);
        assert!(result.is_none(), "circular var() should resolve to None");
    }

    #[test]
    fn var_self_reference_returns_none() {
        // --x references itself → cycle.
        let mut custom_props = HashMap::new();
        custom_props.insert("--x".into(), "var(--x)".into());

        let var_x = CssValue::Var("--x".into(), None);
        let mut visited = HashSet::new();
        let result = resolve_var_value(&var_x, &custom_props, 0, &mut visited);
        assert!(
            result.is_none(),
            "self-referencing var() should resolve to None"
        );
    }

    #[test]
    fn get_computed_custom_property() {
        let mut style = ComputedStyle::default();
        style
            .custom_properties
            .insert("--bg".into(), "#0d1117".into());

        let val = get_computed_as_css_value("--bg", &style);
        assert_eq!(val, CssValue::RawTokens("#0d1117".into()));
    }

    #[test]
    fn get_computed_custom_property_undefined() {
        let style = ComputedStyle::default();
        let val = get_computed_as_css_value("--undefined", &style);
        assert_eq!(val, CssValue::RawTokens(String::new()));
    }
}
