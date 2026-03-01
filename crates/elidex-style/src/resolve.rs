//! CSS value resolution: relative units → absolute pixels.
//!
//! Converts parsed [`CssValue`]s into concrete values for [`ComputedStyle`]
//! fields, resolving relative lengths, font-size keywords, `currentcolor`,
//! and the border-width/border-style interaction.

use std::collections::{HashMap, HashSet};

use elidex_plugin::{
    AlignContent, AlignItems, AlignSelf, BorderStyle, BoxSizing, ComputedStyle, CssColor, CssValue,
    Dimension, Display, FlexDirection, FlexWrap, JustifyContent, LengthUnit, LineHeight,
    ListStyleType, Overflow, Position, TextAlign, TextDecorationLine, TextTransform, WhiteSpace,
};

use crate::inherit::{get_initial_value, is_inherited};

/// Cascade winner map: property name → winning CSS value.
type PropertyMap<'a> = HashMap<&'a str, &'a CssValue>;

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

/// CSS absolute font-size keyword values in pixels (CSS Values Level 3).
const FONT_SIZE_XX_SMALL: f32 = 9.0;
const FONT_SIZE_X_SMALL: f32 = 10.0;
const FONT_SIZE_SMALL: f32 = 13.0;
const FONT_SIZE_MEDIUM: f32 = 16.0;
const FONT_SIZE_LARGE: f32 = 18.0;
const FONT_SIZE_X_LARGE: f32 = 24.0;
const FONT_SIZE_XX_LARGE: f32 = 32.0;

/// Scale factor for the `smaller` relative font-size keyword (~5/6).
const FONT_SIZE_SMALLER_RATIO: f32 = 0.833;
/// Scale factor for the `larger` relative font-size keyword (~6/5).
const FONT_SIZE_LARGER_RATIO: f32 = 1.2;

/// Resolve font-size keywords to pixel values.
fn resolve_font_size_keyword(keyword: &str, parent_font_size: f32) -> Option<f32> {
    match keyword {
        "xx-small" => Some(FONT_SIZE_XX_SMALL),
        "x-small" => Some(FONT_SIZE_X_SMALL),
        "small" => Some(FONT_SIZE_SMALL),
        "medium" => Some(FONT_SIZE_MEDIUM),
        "large" => Some(FONT_SIZE_LARGE),
        "x-large" => Some(FONT_SIZE_X_LARGE),
        "xx-large" => Some(FONT_SIZE_XX_LARGE),
        "smaller" => Some(parent_font_size * FONT_SIZE_SMALLER_RATIO),
        "larger" => Some(parent_font_size * FONT_SIZE_LARGER_RATIO),
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

/// Wrap an `AsRef<str>` value in `CssValue::Keyword`.
///
/// Shorthand for `CssValue::Keyword(val.as_ref().to_string())`, used to
/// convert keyword-enum fields back into CSS values for inheritance and
/// `getComputedStyle()`.
fn keyword_from<T: AsRef<str>>(val: &T) -> CssValue {
    CssValue::Keyword(val.as_ref().to_string())
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
        "text-transform" => keyword_from(&style.text_transform),
        "text-align" => keyword_from(&style.text_align),
        "white-space" => keyword_from(&style.white_space),
        "list-style-type" => keyword_from(&style.list_style_type),
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
        "display" => keyword_from(&style.display),
        "position" => keyword_from(&style.position),
        "overflow" => keyword_from(&style.overflow),
        "background-color" => CssValue::Color(style.background_color),
        "width" => dimension_to_css_value(style.width),
        "height" => dimension_to_css_value(style.height),
        "min-width" => dimension_to_css_value(style.min_width),
        "max-width" => {
            if style.max_width == Dimension::Auto {
                CssValue::Keyword("none".to_string())
            } else {
                dimension_to_css_value(style.max_width)
            }
        }
        "min-height" => dimension_to_css_value(style.min_height),
        "max-height" => {
            if style.max_height == Dimension::Auto {
                CssValue::Keyword("none".to_string())
            } else {
                dimension_to_css_value(style.max_height)
            }
        }
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
        "border-top-style" => keyword_from(&style.border_top_style),
        "border-right-style" => keyword_from(&style.border_right_style),
        "border-bottom-style" => keyword_from(&style.border_bottom_style),
        "border-left-style" => keyword_from(&style.border_left_style),
        "border-top-color" => CssValue::Color(style.border_top_color),
        "border-right-color" => CssValue::Color(style.border_right_color),
        "border-bottom-color" => CssValue::Color(style.border_bottom_color),
        "border-left-color" => CssValue::Color(style.border_left_color),
        "flex-direction" => keyword_from(&style.flex_direction),
        "flex-wrap" => keyword_from(&style.flex_wrap),
        "justify-content" => keyword_from(&style.justify_content),
        "align-items" => keyword_from(&style.align_items),
        "align-content" => keyword_from(&style.align_content),
        "align-self" => keyword_from(&style.align_self),
        "flex-grow" => CssValue::Number(style.flex_grow),
        "flex-shrink" => CssValue::Number(style.flex_shrink),
        "flex-basis" => dimension_to_css_value(style.flex_basis),
        #[allow(clippy::cast_precision_loss)]
        "order" => CssValue::Number(style.order as f32),
        "row-gap" => CssValue::Length(style.row_gap, LengthUnit::Px),
        "column-gap" => CssValue::Length(style.column_gap, LengthUnit::Px),
        "box-sizing" => keyword_from(&style.box_sizing),
        "border-radius" => CssValue::Length(style.border_radius, LengthUnit::Px),
        "opacity" => CssValue::Number(style.opacity),
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
    // Phase 1: Build custom properties map (inherit from parent + override).
    let mut style = ComputedStyle {
        custom_properties: build_custom_properties(winners, parent_style),
        ..ComputedStyle::default()
    };

    // Phase 2: Resolve var() references in winners.
    let resolved_winners = resolve_var_references(winners, &style.custom_properties);
    let winners = merge_winners(winners, &resolved_winners);

    // Phase 3: Font-size first (em units in all other properties depend on it).
    let element_font_size = resolve_font_size(&winners, parent_style, ctx);
    style.font_size = element_font_size;
    let elem_ctx = ctx.with_em_base(element_font_size);

    // Phase 4: Font, text, and color properties.
    resolve_font_and_text_properties(&mut style, &winners, parent_style, &elem_ctx);
    style.color = resolve_color(&winners, parent_style);
    resolve_background_color(&mut style, &winners, parent_style);

    // Phase 5: Display, positioning, overflow.
    resolve_display(&mut style, &winners, parent_style);
    resolve_position(&mut style, &winners, parent_style);
    resolve_keyword_enum_prop!(
        "overflow",
        &winners,
        parent_style,
        style.overflow,
        |k| match k {
            "visible" => Some(Overflow::Visible),
            "hidden" => Some(Overflow::Hidden),
            _ => None,
        }
    );

    // Phase 6: Box model — dimensions, margin, padding, border, extras.
    resolve_box_dimensions(&mut style, &winners, parent_style, &elem_ctx);
    resolve_border_properties(&mut style, &winners, parent_style, &elem_ctx);
    resolve_box_model_extras(&mut style, &winners, parent_style, &elem_ctx);

    // Phase 7: Flex and gap properties.
    let dim = |v: &CssValue| resolve_dimension(v, &elem_ctx);
    resolve_flex_properties(&mut style, &winners, parent_style, dim);
    resolve_gap_properties(&mut style, &winners, parent_style, &elem_ctx);

    style
}

/// Resolve font, text, and inherited keyword-enum properties.
fn resolve_font_and_text_properties(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    resolve_font_weight(style, winners, parent_style);
    resolve_font_family(style, winners, parent_style);
    resolve_line_height(style, winners, parent_style, ctx);
    style.text_transform = resolve_inherited_keyword_enum(
        "text-transform",
        winners,
        parent_style,
        parent_style.text_transform,
        |k| match k {
            "none" => Some(TextTransform::None),
            "uppercase" => Some(TextTransform::Uppercase),
            "lowercase" => Some(TextTransform::Lowercase),
            "capitalize" => Some(TextTransform::Capitalize),
            _ => None,
        },
    );
    style.text_align = resolve_inherited_keyword_enum(
        "text-align",
        winners,
        parent_style,
        parent_style.text_align,
        |k| match k {
            "left" => Some(TextAlign::Left),
            "center" => Some(TextAlign::Center),
            "right" => Some(TextAlign::Right),
            _ => None,
        },
    );
    style.white_space = resolve_inherited_keyword_enum(
        "white-space",
        winners,
        parent_style,
        parent_style.white_space,
        |k| match k {
            "normal" => Some(WhiteSpace::Normal),
            "pre" => Some(WhiteSpace::Pre),
            "nowrap" => Some(WhiteSpace::NoWrap),
            "pre-wrap" => Some(WhiteSpace::PreWrap),
            "pre-line" => Some(WhiteSpace::PreLine),
            _ => None,
        },
    );
    style.list_style_type = resolve_inherited_keyword_enum(
        "list-style-type",
        winners,
        parent_style,
        parent_style.list_style_type,
        |k| match k {
            "disc" => Some(ListStyleType::Disc),
            "circle" => Some(ListStyleType::Circle),
            "square" => Some(ListStyleType::Square),
            "decimal" => Some(ListStyleType::Decimal),
            "none" => Some(ListStyleType::None),
            _ => None,
        },
    );
    // text-decoration-line is non-inherited.
    resolve_text_decoration_line(style, winners, parent_style);
}

/// Resolve dimensions, margins, and padding.
fn resolve_box_dimensions(
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
    let px = |v: &CssValue| resolve_to_px(v, ctx);
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
fn resolve_box_model_extras(
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
        |k| match k {
            "content-box" => Some(BoxSizing::ContentBox),
            "border-box" => Some(BoxSizing::BorderBox),
            _ => None,
        }
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
            CssValue::Number(n) => n.clamp(0.0, 1.0),
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
fn resolve_gap_properties(
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
                // Expand shorthand property names to their longhands so that
                // downstream resolvers (which only look up longhand keys) can
                // find the value.  When var() is the entire shorthand value,
                // parse_property_value stores it under the shorthand name
                // (e.g. "background") because var() is detected before the
                // shorthand match.  After resolution we re-key it here.
                expand_resolved_shorthand(&mut resolved, name, val);
            }
        }
    }
    resolved
}

/// Expand a resolved shorthand value into longhand entries.
///
/// For simple shorthands where the resolved value maps to a single longhand
/// (e.g. `background` → `background-color`), insert under the longhand key.
/// For per-side border shorthands (`border-top`, etc.), insert under the 3
/// longhand keys with appropriate defaults.
fn expand_resolved_shorthand(resolved: &mut HashMap<String, CssValue>, name: &str, val: CssValue) {
    match name {
        "background" => {
            resolved.insert("background-color".to_string(), val);
        }
        "border-top" | "border-right" | "border-bottom" | "border-left" => {
            // The resolved value is a single color/keyword — treat it as the
            // most common case: `border-side: <width> <style> <color>` where
            // only one component was specified via var().
            // For a single resolved color, store it as the border-side-color.
            let side = &name["border-".len()..];
            match &val {
                CssValue::Color(_) => {
                    resolved.insert(format!("border-{side}-color"), val);
                }
                CssValue::Keyword(k) => {
                    // Could be a style keyword (solid, dashed, none, etc.)
                    let style_keywords = [
                        "none", "hidden", "dotted", "dashed", "solid", "double", "groove", "ridge",
                        "inset", "outset",
                    ];
                    if style_keywords.contains(&k.as_str()) {
                        resolved.insert(format!("border-{side}-style"), val);
                    } else {
                        resolved.insert(format!("border-{side}-color"), val);
                    }
                }
                CssValue::Length(_, _) => {
                    resolved.insert(format!("border-{side}-width"), val);
                }
                _ => {
                    // Fallback: store under the shorthand name as-is.
                    resolved.insert(name.to_string(), val);
                }
            }
        }
        _ => {
            resolved.insert((*name).to_string(), val);
        }
    }
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
    // Include resolved entries with new keys (from shorthand expansion,
    // e.g. "background" → "background-color").
    for (name, value) in resolved {
        if !merged.contains_key(name.as_str()) {
            merged.insert(name.as_str(), value);
        }
    }
    merged
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

// --- Shared helpers ---

/// Get the cascade winner for a property, resolving `inherit`/`initial`/`unset`
/// keywords. Returns `None` if the property is not in the winners map (caller
/// should apply inheritance or initial value as appropriate).
fn get_resolved_winner(
    property: &str,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) -> Option<CssValue> {
    let value = winners.get(property)?;
    Some(resolve_keyword_or_clone(property, value, parent_style))
}

// --- Individual property resolvers ---

fn resolve_font_size(
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) -> f32 {
    match get_resolved_winner("font-size", winners, parent_style) {
        Some(value) => resolve_font_size_value(&value, parent_style, ctx),
        None => parent_style.font_size,
    }
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
    match get_resolved_winner("color", winners, parent_style) {
        Some(CssValue::Color(c)) => c,
        Some(_) | None => parent_style.color,
    }
}

fn resolve_font_family(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    match get_resolved_winner("font-family", winners, parent_style) {
        Some(CssValue::List(ref items)) => {
            style.font_family = extract_font_family_names(items);
        }
        Some(CssValue::RawTokens(ref raw) | CssValue::String(ref raw)) => {
            style.font_family = parse_font_family_from_raw(raw);
        }
        Some(CssValue::Keyword(ref k)) => {
            // A single keyword (e.g. from var() resolving to a generic family).
            style.font_family = vec![k.clone()];
        }
        _ => {
            style.font_family.clone_from(&parent_style.font_family);
        }
    }
}

/// Extract font family names from a parsed `CssValue::List`.
fn extract_font_family_names(items: &[CssValue]) -> Vec<String> {
    items
        .iter()
        .filter_map(|v| match v {
            CssValue::String(s) => Some(s.clone()),
            CssValue::Keyword(k) => Some(k.clone()),
            _ => None,
        })
        .collect()
}

/// Parse a comma-separated font-family string (from `var()` resolution)
/// into a list of family names.
///
/// Handles quoted names (`'SFMono-Regular'`, `"Courier New"`), unquoted
/// multi-word names (`Times New Roman`), and generic families (`monospace`).
fn parse_font_family_from_raw(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| {
            let trimmed = s.trim();
            // Strip matching outer quotes (single or double).
            if (trimmed.starts_with('\'') && trimmed.ends_with('\''))
                || (trimmed.starts_with('"') && trimmed.ends_with('"'))
            {
                trimmed[1..trimmed.len() - 1].to_string()
            } else {
                trimmed.to_string()
            }
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn resolve_font_weight(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    style.font_weight = match get_resolved_winner("font-weight", winners, parent_style) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Some(CssValue::Number(n)) => n.round().clamp(1.0, 1000.0) as u16,
        Some(CssValue::Keyword(ref k)) => match k.as_str() {
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
    style.line_height = match get_resolved_winner("line-height", winners, parent_style) {
        Some(CssValue::Keyword(ref k)) if k == "normal" => LineHeight::Normal,
        // Unitless number: inherited as-is, recomputed per element's font-size.
        Some(CssValue::Number(n)) => LineHeight::Number(n),
        // Absolute length: resolve to px.
        Some(CssValue::Length(v, unit)) => LineHeight::Px(resolve_length(v, unit, ctx)),
        // Percentage: resolve to px (relative to element's font-size).
        Some(CssValue::Percentage(p)) => LineHeight::Px(style.font_size * p / 100.0),
        _ => parent_style.line_height,
    };
}

/// Resolve an inherited keyword-enum property.
///
/// If present in the winners map, converts the keyword via `from_keyword`.
/// If absent, inherits from `parent_value`. This unifies the common pattern
/// used by text-transform, text-align, white-space, and list-style-type.
fn resolve_inherited_keyword_enum<T: Copy>(
    property: &str,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    parent_value: T,
    from_keyword: impl Fn(&str) -> Option<T>,
) -> T {
    match get_resolved_winner(property, winners, parent_style) {
        Some(CssValue::Keyword(ref k)) => from_keyword(k).unwrap_or(parent_value),
        Some(_) | None => parent_value,
    }
}

fn resolve_text_decoration_line(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    let Some(value) = get_resolved_winner("text-decoration-line", winners, parent_style) else {
        // Non-inherited: keep default (none). Don't inherit from parent.
        return;
    };
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

/// Resolve a non-inherited keyword enum property from the winners map.
///
/// Returns `None` if the property is absent from the winners (caller keeps
/// the default). Returns `Some(T)` when a value is found, mapping the keyword
/// through `from_keyword` or falling back to `T::default()`.
fn resolve_keyword_enum<T: Default>(
    property: &str,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    from_keyword: impl Fn(&str) -> Option<T>,
) -> Option<T> {
    let value = get_resolved_winner(property, winners, parent_style)?;
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
            "list-item" => Some(Display::ListItem),
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
    match get_resolved_winner("background-color", winners, parent_style) {
        Some(CssValue::Color(c)) => style.background_color = c,
        Some(CssValue::Keyword(ref k)) if k.eq_ignore_ascii_case("currentcolor") => {
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
    if let Some(value) = get_resolved_winner(property, winners, parent_style) {
        set(convert(&value));
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

/// Resolve a border-style keyword to a [`BorderStyle`] enum value.
///
/// Phase 1 supports: `none`, `solid`, `dashed`, `dotted`.
// TODO(Phase 4): support double, groove, ridge, inset, outset
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
fn resolve_border_properties(
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
        // CSS spec: computed border-width is 0 when border-style is none.
        let px = if border_style_by_side(style, side) == BorderStyle::None {
            0.0
        } else {
            match get_resolved_winner(prop, winners, parent_style) {
                Some(value) => resolve_to_px(&value, ctx),
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
/// Percentage values are not yet supported (Phase 4) and resolve to `0.0`.
fn resolve_to_px(value: &CssValue, ctx: &ResolveContext) -> f32 {
    match value {
        CssValue::Length(v, unit) => resolve_length(*v, *unit, ctx),
        // TODO(Phase 4): resolve CssValue::Percentage against containing block width
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

    // --- M3-2: Box model resolution ---

    #[test]
    fn resolve_box_sizing_border_box() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
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
        let winners: HashMap<&str, &CssValue> = HashMap::new();
        let style = build_computed_style(&winners, &parent, &ctx);
        // Non-inherited: should be initial (content-box), not parent's border-box.
        assert_eq!(style.box_sizing, BoxSizing::ContentBox);
    }

    #[test]
    fn resolve_border_radius_px() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let val = CssValue::Length(8.0, LengthUnit::Px);
        winners.insert("border-radius", &val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!((style.border_radius - 8.0).abs() < f32::EPSILON);
    }

    #[test]
    fn resolve_opacity() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let val = CssValue::Number(0.5);
        winners.insert("opacity", &val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!((style.opacity - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn resolve_opacity_clamped() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let val = CssValue::Number(2.0);
        winners.insert("opacity", &val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!((style.opacity - 1.0).abs() < f32::EPSILON);
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

    // --- M3-5: gap + text-align resolution ---

    #[test]
    fn resolve_row_gap_px() {
        let ctx = default_ctx();
        let val = CssValue::Length(10.0, LengthUnit::Px);
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        winners.insert("row-gap", &val);
        let parent = ComputedStyle::default();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!((style.row_gap - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn resolve_column_gap_negative_clamped() {
        let ctx = default_ctx();
        let val = CssValue::Length(-5.0, LengthUnit::Px);
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        winners.insert("column-gap", &val);
        let parent = ComputedStyle::default();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!((style.column_gap).abs() < f32::EPSILON);
    }

    #[test]
    fn resolve_text_align_center() {
        let ctx = default_ctx();
        let val = CssValue::Keyword("center".into());
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        winners.insert("text-align", &val);
        let parent = ComputedStyle::default();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.text_align, TextAlign::Center);
    }

    #[test]
    fn resolve_text_align_inherited() {
        let ctx = default_ctx();
        let parent = ComputedStyle {
            text_align: TextAlign::Right,
            ..ComputedStyle::default()
        };
        let winners: HashMap<&str, &CssValue> = HashMap::new();
        let style = build_computed_style(&winners, &parent, &ctx);
        // text-align is inherited — child inherits Right from parent.
        assert_eq!(style.text_align, TextAlign::Right);
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
    fn resolve_text_align_computed_value() {
        let style = ComputedStyle {
            text_align: TextAlign::Center,
            ..ComputedStyle::default()
        };
        assert_eq!(
            get_computed_as_css_value("text-align", &style),
            CssValue::Keyword("center".to_string())
        );
    }

    #[test]
    fn resolve_row_gap_length_value() {
        let ctx = default_ctx();
        let parent = ComputedStyle::default();
        let mut winners: PropertyMap<'_> = HashMap::new();
        let gap_val = CssValue::Length(12.0, LengthUnit::Px);
        winners.insert("row-gap", &gap_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!((style.row_gap - 12.0).abs() < f32::EPSILON);
    }

    // L3: gap: inherit resolves from parent
    #[test]
    fn resolve_gap_inherit_from_parent() {
        let ctx = default_ctx();
        let parent = ComputedStyle {
            row_gap: 8.0,
            column_gap: 16.0,
            ..ComputedStyle::default()
        };
        let mut winners: PropertyMap<'_> = HashMap::new();
        let inherit_val = CssValue::Inherit;
        winners.insert("row-gap", &inherit_val);
        winners.insert("column-gap", &inherit_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        // gap is non-inherited, but `inherit` keyword forces parent value.
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

    // --- M3-6: white-space resolution ---

    #[test]
    fn resolve_white_space_keyword() {
        let ctx = default_ctx();
        let parent = ComputedStyle::default();
        let mut winners: PropertyMap<'_> = HashMap::new();
        let val = CssValue::Keyword("pre".to_string());
        winners.insert("white-space", &val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.white_space, WhiteSpace::Pre);
    }

    #[test]
    fn resolve_white_space_inherits_from_parent() {
        let ctx = default_ctx();
        let parent = ComputedStyle {
            white_space: WhiteSpace::NoWrap,
            ..ComputedStyle::default()
        };
        let winners: PropertyMap<'_> = HashMap::new();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.white_space, WhiteSpace::NoWrap);
    }

    #[test]
    fn resolve_white_space_computed_value() {
        let style = ComputedStyle {
            white_space: WhiteSpace::PreWrap,
            ..ComputedStyle::default()
        };
        assert_eq!(
            get_computed_as_css_value("white-space", &style),
            CssValue::Keyword("pre-wrap".to_string())
        );
    }

    // --- M3-6: overflow resolution ---

    #[test]
    fn resolve_overflow_keyword() {
        let ctx = default_ctx();
        let parent = ComputedStyle::default();
        let mut winners: PropertyMap<'_> = HashMap::new();
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

    // --- M3-6: min/max width/height resolution ---

    #[test]
    fn resolve_min_width() {
        let ctx = default_ctx();
        let parent = ComputedStyle::default();
        let mut winners: PropertyMap<'_> = HashMap::new();
        let val = CssValue::Length(100.0, LengthUnit::Px);
        winners.insert("min-width", &val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.min_width, Dimension::Length(100.0));
    }

    #[test]
    fn resolve_max_width_none() {
        let ctx = default_ctx();
        let parent = ComputedStyle::default();
        let mut winners: PropertyMap<'_> = HashMap::new();
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
        let mut winners: PropertyMap<'_> = HashMap::new();
        let val = CssValue::Percentage(25.0);
        winners.insert("min-height", &val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.min_height, Dimension::Percentage(25.0));
    }

    // --- M3-6: list-style-type resolution ---

    #[test]
    fn resolve_list_style_type_keyword() {
        let ctx = default_ctx();
        let parent = ComputedStyle::default();
        let mut winners: PropertyMap<'_> = HashMap::new();
        let val = CssValue::Keyword("decimal".to_string());
        winners.insert("list-style-type", &val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.list_style_type, ListStyleType::Decimal);
    }

    #[test]
    fn resolve_list_style_type_inherits() {
        let ctx = default_ctx();
        let parent = ComputedStyle {
            list_style_type: ListStyleType::Square,
            ..ComputedStyle::default()
        };
        let winners: PropertyMap<'_> = HashMap::new();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.list_style_type, ListStyleType::Square);
    }

    #[test]
    fn resolve_list_style_type_computed_value() {
        let style = ComputedStyle {
            list_style_type: ListStyleType::Circle,
            ..ComputedStyle::default()
        };
        assert_eq!(
            get_computed_as_css_value("list-style-type", &style),
            CssValue::Keyword("circle".to_string())
        );
    }

    #[test]
    fn resolve_display_list_item() {
        let ctx = default_ctx();
        let parent = ComputedStyle::default();
        let mut winners: PropertyMap<'_> = HashMap::new();
        let val = CssValue::Keyword("list-item".to_string());
        winners.insert("display", &val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.display, Display::ListItem);
    }

    // --- var() resolution for font-family ---

    #[test]
    fn var_resolution_font_family_comma_list() {
        // Simulates: --fonts: 'SFMono-Regular', Consolas, monospace
        //            font-family: var(--fonts)
        let mut parent = ComputedStyle::default();
        parent.custom_properties.insert(
            "--fonts".into(),
            "'SFMono-Regular', Consolas, monospace".into(),
        );
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let var_val = CssValue::Var("--fonts".into(), None);
        winners.insert("font-family", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(
            style.font_family,
            vec![
                "SFMono-Regular".to_string(),
                "Consolas".to_string(),
                "monospace".to_string(),
            ]
        );
    }

    #[test]
    fn var_resolution_font_family_double_quoted() {
        // Double-quoted family names should also be unquoted.
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--fonts".into(), "\"Courier New\", monospace".into());
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let var_val = CssValue::Var("--fonts".into(), None);
        winners.insert("font-family", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(
            style.font_family,
            vec!["Courier New".to_string(), "monospace".to_string()]
        );
    }

    #[test]
    fn var_resolution_font_family_single_generic() {
        // A single generic family from var() should also work.
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--mono".into(), "monospace".into());
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        let var_val = CssValue::Var("--mono".into(), None);
        winners.insert("font-family", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.font_family, vec!["monospace".to_string()]);
    }

    #[test]
    fn parse_font_family_from_raw_mixed_quotes() {
        let result = parse_font_family_from_raw(
            "'SFMono-Regular', Consolas, \"Liberation Mono\", monospace",
        );
        assert_eq!(
            result,
            vec![
                "SFMono-Regular".to_string(),
                "Consolas".to_string(),
                "Liberation Mono".to_string(),
                "monospace".to_string(),
            ]
        );
    }

    #[test]
    fn parse_font_family_from_raw_empty() {
        let result = parse_font_family_from_raw("");
        assert!(result.is_empty());
    }

    #[test]
    fn var_background_shorthand_expands_to_background_color() {
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--bg".into(), "#0d1117".into());
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        // Simulates `background: var(--bg)` — stored under "background" key.
        let var_val = CssValue::Var("--bg".into(), None);
        winners.insert("background", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.background_color, CssColor::new(0x0d, 0x11, 0x17, 255));
    }

    #[test]
    fn var_border_side_shorthand_expands_color() {
        let mut parent = ComputedStyle::default();
        parent
            .custom_properties
            .insert("--border".into(), "#30363d".into());
        let ctx = default_ctx();
        let mut winners: HashMap<&str, &CssValue> = HashMap::new();
        // Simulates `border-bottom: var(--border)` — stored under "border-bottom" key.
        let var_val = CssValue::Var("--border".into(), None);
        winners.insert("border-bottom", &var_val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(
            style.border_bottom_color,
            CssColor::new(0x30, 0x36, 0x3d, 255)
        );
    }
}
