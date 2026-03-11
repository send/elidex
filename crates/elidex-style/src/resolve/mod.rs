//! CSS value resolution: relative units → absolute pixels.
//!
//! Converts parsed [`CssValue`]s into concrete values for [`ComputedStyle`]
//! fields, resolving relative lengths, font-size keywords, `currentcolor`,
//! and the border-width/border-style interaction.

mod box_model;
mod flex;
mod font;
mod grid;
pub(crate) mod helpers;
mod var_resolution;

use elidex_plugin::{
    ComputedStyle, ContentItem, ContentValue, CssValue, Dimension, Direction, LengthUnit,
    LineHeight, TextOrientation, UnicodeBidi, WritingMode,
};

pub(crate) use helpers::PropertyMap;
use helpers::{keyword_from, resolve_dimension};

use var_resolution::{build_custom_properties, merge_winners, resolve_var_references};

use font::{
    resolve_background_color, resolve_color, resolve_font_and_text_properties, resolve_font_size,
};

use box_model::{
    resolve_border_properties, resolve_box_dimensions, resolve_box_model_extras, resolve_content,
    resolve_display, resolve_gap_properties, resolve_overflow, resolve_position,
    resolve_table_properties,
};

use flex::resolve_flex_properties;
use grid::{
    grid_line_to_css_value, resolve_grid_properties, track_list_to_css_value,
    track_size_to_css_value,
};

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

/// Extract a property's computed value back into a [`CssValue`] for inheritance.
///
/// Also used by `getComputedStyle()` DOM API.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn get_computed_as_css_value(property: &str, style: &ComputedStyle) -> CssValue {
    use crate::inherit::get_initial_value;

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
        "font-style" => keyword_from(&style.font_style),
        "line-height" => match style.line_height {
            LineHeight::Normal => CssValue::Keyword("normal".to_string()),
            LineHeight::Number(n) => CssValue::Number(n),
            LineHeight::Px(px) => CssValue::Length(px, LengthUnit::Px),
        },
        "text-transform" => keyword_from(&style.text_transform),
        "text-align" => keyword_from(&style.text_align),
        "white-space" => keyword_from(&style.white_space),
        "list-style-type" => keyword_from(&style.list_style_type),
        "direction" => keyword_from(&style.direction),
        "unicode-bidi" => keyword_from(&style.unicode_bidi),
        "writing-mode" => keyword_from(&style.writing_mode),
        "text-orientation" => keyword_from(&style.text_orientation),
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
        // Grid container
        "grid-template-columns" => track_list_to_css_value(&style.grid_template_columns),
        "grid-template-rows" => track_list_to_css_value(&style.grid_template_rows),
        "grid-auto-flow" => keyword_from(&style.grid_auto_flow),
        "grid-auto-columns" => track_size_to_css_value(&style.grid_auto_columns),
        "grid-auto-rows" => track_size_to_css_value(&style.grid_auto_rows),
        // Grid item
        "grid-column-start" => grid_line_to_css_value(style.grid_column_start),
        "grid-column-end" => grid_line_to_css_value(style.grid_column_end),
        "grid-row-start" => grid_line_to_css_value(style.grid_row_start),
        "grid-row-end" => grid_line_to_css_value(style.grid_row_end),

        // Table properties
        "border-collapse" => keyword_from(&style.border_collapse),
        "border-spacing" => {
            if (style.border_spacing_h - style.border_spacing_v).abs() < f32::EPSILON {
                CssValue::Length(style.border_spacing_h, LengthUnit::Px)
            } else {
                CssValue::List(vec![
                    CssValue::Length(style.border_spacing_h, LengthUnit::Px),
                    CssValue::Length(style.border_spacing_v, LengthUnit::Px),
                ])
            }
        }
        "border-spacing-h" => CssValue::Length(style.border_spacing_h, LengthUnit::Px),
        "border-spacing-v" => CssValue::Length(style.border_spacing_v, LengthUnit::Px),
        "table-layout" => keyword_from(&style.table_layout),
        "caption-side" => keyword_from(&style.caption_side),

        "content" => match &style.content {
            ContentValue::Normal => CssValue::Keyword("normal".to_string()),
            ContentValue::None => CssValue::Keyword("none".to_string()),
            ContentValue::Items(items) => {
                if items.len() == 1 {
                    match &items[0] {
                        ContentItem::String(s) => CssValue::String(s.clone()),
                        ContentItem::Attr(a) => CssValue::Keyword(format!("attr:{a}")),
                    }
                } else {
                    CssValue::List(
                        items
                            .iter()
                            .map(|item| match item {
                                ContentItem::String(s) => CssValue::String(s.clone()),
                                ContentItem::Attr(a) => CssValue::Keyword(format!("attr:{a}")),
                            })
                            .collect(),
                    )
                }
            }
        },
        _ => get_initial_value(property),
    }
}

// Display, Position, and BorderStyle implement AsRef<str> in elidex-plugin,
// so enum-to-string conversion is handled via .as_ref() directly.

/// Convert a [`Dimension`] back into a [`CssValue`] for CSS serialization.
pub fn dimension_to_css_value(d: Dimension) -> CssValue {
    match d {
        Dimension::Length(px) => CssValue::Length(px, LengthUnit::Px),
        Dimension::Percentage(p) => CssValue::Percentage(p),
        Dimension::Auto => CssValue::Auto,
    }
}

/// Build a [`ComputedStyle`] from the cascade winner map.
///
/// Resolution order: font-size first (dependencies), then color,
/// then all remaining properties.
#[must_use]
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
    resolve_overflow(&mut style, &winners, parent_style);

    // Phase 6: Box model — dimensions, margin, padding, border, extras.
    resolve_box_dimensions(&mut style, &winners, parent_style, &elem_ctx);
    resolve_border_properties(&mut style, &winners, parent_style, &elem_ctx);
    resolve_box_model_extras(&mut style, &winners, parent_style, &elem_ctx);

    // Phase 7: Flex, grid, and gap properties.
    let dim = |v: &CssValue| resolve_dimension(v, &elem_ctx);
    resolve_flex_properties(&mut style, &winners, parent_style, dim);
    resolve_grid_properties(&mut style, &winners, parent_style, &elem_ctx);
    resolve_gap_properties(&mut style, &winners, parent_style, &elem_ctx);

    // Phase 8: Content property (non-inherited).
    resolve_content(&mut style, &winners, parent_style);

    // Phase 9: Table properties.
    resolve_table_properties(&mut style, &winners, parent_style, &elem_ctx);

    // Phase 10: Writing mode / BiDi properties.
    resolve_writing_mode_properties(&mut style, &winners, parent_style);

    style
}

/// Resolve writing mode and bidi properties.
///
/// Uses `resolve_inherited_keyword_enum` / `resolve_keyword_enum` to
/// correctly handle `initial`, `unset`, and `inherit` global keywords
/// via `get_resolved_winner`.
fn resolve_writing_mode_properties(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
) {
    use helpers::{resolve_inherited_keyword_enum, resolve_keyword_enum};

    // direction — inherited
    style.direction = resolve_inherited_keyword_enum(
        "direction",
        winners,
        parent_style,
        parent_style.direction,
        Direction::from_keyword,
    );

    // unicode-bidi — non-inherited
    if let Some(u) = resolve_keyword_enum(
        "unicode-bidi",
        winners,
        parent_style,
        UnicodeBidi::from_keyword,
    ) {
        style.unicode_bidi = u;
    }

    // writing-mode — inherited
    style.writing_mode = resolve_inherited_keyword_enum(
        "writing-mode",
        winners,
        parent_style,
        parent_style.writing_mode,
        WritingMode::from_keyword,
    );

    // text-orientation — inherited
    style.text_orientation = resolve_inherited_keyword_enum(
        "text-orientation",
        winners,
        parent_style,
        parent_style.text_orientation,
        TextOrientation::from_keyword,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use elidex_plugin::{CssValue, Display};

    #[test]
    fn get_computed_display() {
        let style = ComputedStyle {
            display: Display::Flex,
            ..ComputedStyle::default()
        };
        let val = get_computed_as_css_value("display", &style);
        assert_eq!(val, CssValue::Keyword("flex".to_string()));
    }

    #[test]
    fn get_computed_custom_property() {
        let style = ComputedStyle {
            custom_properties: {
                let mut m = HashMap::new();
                m.insert("--my-color".to_string(), "red".to_string());
                m
            },
            ..ComputedStyle::default()
        };
        let val = get_computed_as_css_value("--my-color", &style);
        assert_eq!(val, CssValue::RawTokens("red".to_string()));
    }

    #[test]
    fn get_computed_custom_property_undefined() {
        let style = ComputedStyle::default();
        let val = get_computed_as_css_value("--undefined", &style);
        assert_eq!(val, CssValue::RawTokens(String::new()));
    }

    #[test]
    fn get_computed_font_family() {
        let style = ComputedStyle {
            font_family: vec!["Arial".to_string(), "sans-serif".to_string()],
            ..ComputedStyle::default()
        };
        let val = get_computed_as_css_value("font-family", &style);
        assert_eq!(
            val,
            CssValue::List(vec![
                CssValue::Keyword("Arial".to_string()),
                CssValue::Keyword("sans-serif".to_string()),
            ])
        );
    }
}
