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
    Clear, ComputedStyle, CssValue, Dimension, Direction, Display, Float, LengthUnit, Position,
    TextOrientation, UnicodeBidi, VerticalAlign, Visibility, WritingMode,
};

use helpers::resolve_dimension;
pub(crate) use helpers::PropertyMap;

use var_resolution::{build_custom_properties, merge_winners, resolve_var_references};

use font::{
    resolve_background_color, resolve_color, resolve_font_and_text_properties, resolve_font_size,
};

use box_model::{
    resolve_border_properties, resolve_box_dimensions, resolve_box_model_extras, resolve_content,
    resolve_display, resolve_gap_properties, resolve_overflow, resolve_position,
    resolve_position_offsets, resolve_table_properties,
};

use flex::resolve_flex_properties;
use grid::resolve_grid_properties;

/// Context for resolving relative CSS values (re-exported from elidex-plugin).
pub(crate) type ResolveContext = elidex_plugin::ResolveContext;

/// Extract a property's computed value using the CSS property registry.
///
/// Delegates to the registered `CssPropertyHandler::get_computed` for known
/// properties. Custom properties and the `border-spacing` compound shorthand
/// are handled inline. Falls back to the initial value for unregistered
/// properties.
#[must_use]
pub fn get_computed_with_registry(
    property: &str,
    style: &ComputedStyle,
    registry: &elidex_plugin::CssPropertyRegistry,
) -> CssValue {
    use crate::inherit::get_initial_value;

    // Custom properties are not handled by plugin handlers.
    if property.starts_with("--") {
        return match style.custom_properties.get(property) {
            Some(raw) => CssValue::RawTokens(raw.clone()),
            None => CssValue::RawTokens(String::new()),
        };
    }
    // Compound shorthand: border-spacing (two longhands → single/pair value).
    if property == "border-spacing" {
        if (style.border_spacing_h - style.border_spacing_v).abs() < f32::EPSILON {
            return CssValue::Length(style.border_spacing_h, LengthUnit::Px);
        }
        return CssValue::List(vec![
            CssValue::Length(style.border_spacing_h, LengthUnit::Px),
            CssValue::Length(style.border_spacing_v, LengthUnit::Px),
        ]);
    }
    if let Some(handler) = registry.resolve(property) {
        return handler.get_computed(property, style);
    }
    get_initial_value(property)
}

// Display, Position, and BorderStyle implement AsRef<str> in elidex-plugin,
// so enum-to-string conversion is handled via .as_ref() directly.

/// Convert a [`Dimension`] back into a [`CssValue`] for CSS serialization.
///
/// Delegates to [`elidex_plugin::css_resolve::dimension_to_css_value`].
pub fn dimension_to_css_value(d: Dimension) -> CssValue {
    elidex_plugin::css_resolve::dimension_to_css_value(d)
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
    resolve_position_offsets(&mut style, &winners, parent_style, &elem_ctx);
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

    // Phase 11: Float, clear, visibility, vertical-align.
    resolve_float_visibility_properties(&mut style, &winners, parent_style, &elem_ctx);

    // Phase 12: Background layers (CSS Backgrounds Level 3).
    resolve_background_layers(&winners, &mut style);

    // Phase 13: Transform properties (CSS Transforms L1/L2, will-change).
    resolve_transform_properties(&mut style, &winners, parent_style, &elem_ctx);

    style
}

/// Resolve float, clear, visibility, and vertical-align properties.
fn resolve_float_visibility_properties(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    use helpers::{
        get_resolved_winner, resolve_inherited_keyword_enum, resolve_keyword_enum, resolve_length,
    };

    // visibility — inherited
    style.visibility = resolve_inherited_keyword_enum(
        "visibility",
        winners,
        parent_style,
        parent_style.visibility,
        Visibility::from_keyword,
    );

    // float — non-inherited
    if let Some(f) = resolve_keyword_enum("float", winners, parent_style, Float::from_keyword) {
        style.float = f;
    }

    // CSS 2.1 §9.7 (applied in spec order):
    // Step 2: position: absolute/fixed → blockify display AND force float to none.
    // Step 3: float is not 'none' → blockify display.
    if matches!(style.position, Position::Absolute | Position::Fixed) {
        style.display = blockify_display(style.display);
        style.float = Float::None;
    } else if style.float != Float::None {
        style.display = blockify_display(style.display);
    }

    // clear — non-inherited
    if let Some(c) = resolve_keyword_enum("clear", winners, parent_style, Clear::from_keyword) {
        style.clear = c;
    }

    // vertical-align — non-inherited, accepts keywords + length + percentage + calc()
    if let Some(value) = get_resolved_winner("vertical-align", winners, parent_style) {
        style.vertical_align = match &value {
            CssValue::Keyword(kw) => {
                VerticalAlign::from_keyword(kw).unwrap_or(VerticalAlign::Baseline)
            }
            CssValue::Length(v, unit) => VerticalAlign::Length(resolve_length(*v, *unit, ctx)),
            CssValue::Percentage(pct) => VerticalAlign::Percentage(*pct),
            CssValue::Calc(expr) => {
                // CSS 2.1 §10.8.1: percentages in vertical-align refer to
                // the element's own line-height.
                let lh_base = computed_line_height_px(style);
                VerticalAlign::Length(helpers::resolve_calc_expr(expr, lh_base, ctx))
            }
            _ => VerticalAlign::Baseline,
        };
    }
}

/// CSS Display Level 3 §2.8 / CSS 2.1 §9.7: Map inline-level display values
/// to their block-level equivalents when the element is floated or absolutely
/// positioned.
///
/// `display: contents` is excluded — it generates no box, so blockification
/// does not apply (browsers preserve `contents` when combined with `float`).
fn blockify_display(display: Display) -> Display {
    match display {
        // Inline-level and table-internal values become block.
        Display::Inline
        | Display::InlineBlock
        | Display::TableRow
        | Display::TableCell
        | Display::TableRowGroup
        | Display::TableHeaderGroup
        | Display::TableFooterGroup
        | Display::TableColumn
        | Display::TableColumnGroup
        | Display::TableCaption => Display::Block,
        Display::InlineFlex => Display::Flex,
        Display::InlineGrid => Display::Grid,
        Display::InlineTable => Display::Table,
        // Block, Flex, Grid, Table, ListItem, None — already block-level
        // or special, unchanged.
        other => other,
    }
}

/// Compute the element's line-height in pixels for percentage resolution.
///
/// CSS 2.1 §10.8.1: `vertical-align` percentages refer to the element's
/// computed `line-height`. Delegates to `LineHeight::resolve_px`.
fn computed_line_height_px(style: &ComputedStyle) -> f32 {
    style.line_height.resolve_px(style.font_size)
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

/// Resolve CSS Transforms L1/L2 and will-change properties.
///
/// Delegates to `TransformHandler::resolve()` for each of the 7 transform
/// properties that have cascade winners.
fn resolve_transform_properties(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    use elidex_plugin::CssPropertyHandler;

    const TRANSFORM_PROPS: &[&str] = &[
        "transform",
        "transform-origin",
        "perspective",
        "perspective-origin",
        "transform-style",
        "backface-visibility",
        "will-change",
    ];

    let handler = elidex_css_transform::TransformHandler;
    for &prop in TRANSFORM_PROPS {
        if let Some(value) = helpers::get_resolved_winner(prop, winners, parent_style) {
            handler.resolve(prop, &value, ctx, style);
        }
    }
}

/// Resolve background layer properties from the winning property map.
///
/// Reads `background-image`, `background-position`, `background-size`,
/// `background-repeat`, `background-origin`, `background-clip`, and
/// `background-attachment` from `winners` and assembles resolved
/// `BackgroundLayer` values into `style.background_layers`.
///
/// If all layers have `background-image: none`, the field is set to `None`
/// (zero-cost default).
fn resolve_background_layers(winners: &PropertyMap<'_>, style: &mut ComputedStyle) {
    use elidex_plugin::background::{BackgroundImage, BackgroundLayer};

    let image_value = winners.get("background-image").copied();
    let Some(img_val) = image_value else {
        return;
    };

    // Build image list
    let images: Vec<BackgroundImage> = match img_val {
        CssValue::List(items) => items
            .iter()
            .map(elidex_css_background::resolve_bg_image)
            .collect(),
        other => vec![elidex_css_background::resolve_bg_image(other)],
    };

    // If all images are None, no layers needed.
    if images.iter().all(|i| *i == BackgroundImage::None) {
        style.background_layers = None;
        return;
    }

    // Helper: get per-layer values from a winning property, cycling as needed.
    let get_list = |prop: &str| -> Vec<&CssValue> {
        match winners.get(prop).copied() {
            Some(CssValue::List(items)) => items.iter().collect(),
            Some(v) => vec![v],
            None => vec![],
        }
    };

    let positions = get_list("background-position");
    let sizes = get_list("background-size");
    let repeats = get_list("background-repeat");
    let origins = get_list("background-origin");
    let clips = get_list("background-clip");
    let attachments = get_list("background-attachment");

    let layers: Vec<BackgroundLayer> = images
        .into_iter()
        .enumerate()
        .map(|(i, image)| {
            let position = positions
                .get(i % positions.len().max(1))
                .map_or_else(Default::default, |v| {
                    elidex_css_background::resolve_bg_position(v)
                });
            let size = sizes
                .get(i % sizes.len().max(1))
                .map_or_else(Default::default, |v| {
                    elidex_css_background::resolve_bg_size(v)
                });
            let repeat = repeats
                .get(i % repeats.len().max(1))
                .map_or_else(Default::default, |v| {
                    elidex_css_background::resolve_bg_repeat(v)
                });
            let origin = origins
                .get(i % origins.len().max(1))
                .map_or_else(Default::default, |v| {
                    elidex_css_background::resolve_box_area_keyword(v)
                });
            let clip = clips.get(i % clips.len().max(1)).map_or_else(
                || elidex_plugin::background::BoxArea::BorderBox,
                |v| elidex_css_background::resolve_box_area_keyword(v),
            );
            let attachment = attachments
                .get(i % attachments.len().max(1))
                .map_or_else(Default::default, |v| {
                    elidex_css_background::resolve_bg_attachment(v)
                });

            BackgroundLayer {
                image,
                position,
                size,
                repeat,
                origin,
                clip,
                attachment,
            }
        })
        .collect();

    style.background_layers = if layers.is_empty() {
        None
    } else {
        Some(layers.into_boxed_slice())
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use elidex_plugin::{CssValue, Display, LineHeight};

    #[test]
    fn get_computed_display() {
        let style = ComputedStyle {
            display: Display::Flex,
            ..ComputedStyle::default()
        };
        let val = crate::get_computed("display", &style);
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
        let val = crate::get_computed("--my-color", &style);
        assert_eq!(val, CssValue::RawTokens("red".to_string()));
    }

    #[test]
    fn get_computed_custom_property_undefined() {
        let style = ComputedStyle::default();
        let val = crate::get_computed("--undefined", &style);
        assert_eq!(val, CssValue::RawTokens(String::new()));
    }

    #[test]
    fn get_computed_font_family() {
        let style = ComputedStyle {
            font_family: vec!["Arial".to_string(), "sans-serif".to_string()],
            ..ComputedStyle::default()
        };
        let val = crate::get_computed("font-family", &style);
        assert_eq!(
            val,
            CssValue::List(vec![
                CssValue::Keyword("Arial".to_string()),
                CssValue::Keyword("sans-serif".to_string()),
            ])
        );
    }

    // --- CSS 2.1 §9.7: blockify_display tests ---

    #[test]
    fn blockify_inline_to_block() {
        assert_eq!(blockify_display(Display::Inline), Display::Block);
        assert_eq!(blockify_display(Display::InlineBlock), Display::Block);
    }

    #[test]
    fn blockify_inline_flex_to_flex() {
        assert_eq!(blockify_display(Display::InlineFlex), Display::Flex);
    }

    #[test]
    fn blockify_inline_grid_to_grid() {
        assert_eq!(blockify_display(Display::InlineGrid), Display::Grid);
    }

    #[test]
    fn blockify_inline_table_to_table() {
        assert_eq!(blockify_display(Display::InlineTable), Display::Table);
    }

    #[test]
    fn blockify_block_unchanged() {
        assert_eq!(blockify_display(Display::Block), Display::Block);
        assert_eq!(blockify_display(Display::Flex), Display::Flex);
        assert_eq!(blockify_display(Display::Table), Display::Table);
    }

    #[test]
    fn blockify_contents_unchanged() {
        // CSS Display Level 3 §2.8: display:contents generates no box,
        // so blockification does not apply — value is preserved.
        assert_eq!(blockify_display(Display::Contents), Display::Contents);
    }

    #[test]
    fn blockify_table_internal_to_block() {
        assert_eq!(blockify_display(Display::TableRow), Display::Block);
        assert_eq!(blockify_display(Display::TableCell), Display::Block);
        assert_eq!(blockify_display(Display::TableCaption), Display::Block);
    }

    // CSS 2.1 §9.7 step 2: position:absolute/fixed forces float to none.
    #[test]
    fn absolute_position_forces_float_none() {
        use elidex_plugin::Float;

        let winners = HashMap::new();
        let parent = ComputedStyle::default();
        let ctx = ResolveContext {
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            em_base: 16.0,
            root_font_size: 16.0,
        };

        // Simulate: float:left + position:absolute → float becomes none, display blockified
        let mut style = ComputedStyle {
            float: Float::Left,
            position: Position::Absolute,
            display: Display::Inline,
            ..ComputedStyle::default()
        };
        resolve_float_visibility_properties(&mut style, &winners, &parent, &ctx);
        assert_eq!(
            style.float,
            Float::None,
            "position:absolute should force float:none"
        );
        assert_eq!(
            style.display,
            Display::Block,
            "display should be blockified"
        );
    }

    #[test]
    fn computed_line_height_px_variants() {
        let base = ComputedStyle {
            font_size: 16.0,
            ..ComputedStyle::default()
        };

        let s1 = ComputedStyle {
            line_height: LineHeight::Px(24.0),
            ..base.clone()
        };
        assert_eq!(computed_line_height_px(&s1), 24.0);

        let s2 = ComputedStyle {
            line_height: LineHeight::Number(1.5),
            ..base.clone()
        };
        assert_eq!(computed_line_height_px(&s2), 24.0);

        let s3 = ComputedStyle {
            line_height: LineHeight::Normal,
            ..base
        };
        assert!((computed_line_height_px(&s3) - 19.2).abs() < 0.01);
    }

    #[test]
    fn resolve_transform_from_cascade() {
        use elidex_plugin::{BackfaceVisibility, LengthUnit, TransformFunction};

        let mut winners = HashMap::new();
        let transform_val = CssValue::TransformList(vec![TransformFunction::Rotate(45.0)]);
        winners.insert("transform", &transform_val);
        let backface_val = CssValue::Keyword("hidden".to_string());
        winners.insert("backface-visibility", &backface_val);

        let parent = ComputedStyle::default();
        let ctx = ResolveContext {
            viewport_width: 800.0,
            viewport_height: 600.0,
            em_base: 16.0,
            root_font_size: 16.0,
        };
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!(style.has_transform);
        assert_eq!(style.transform.len(), 1);
        assert!(
            matches!(style.transform[0], TransformFunction::Rotate(deg) if (deg - 45.0).abs() < f32::EPSILON)
        );
        assert_eq!(style.backface_visibility, BackfaceVisibility::Hidden);
    }

    #[test]
    fn resolve_perspective_from_cascade() {
        let mut winners = HashMap::new();
        let perspective_val = CssValue::Length(800.0, LengthUnit::Px);
        winners.insert("perspective", &perspective_val);

        let parent = ComputedStyle::default();
        let ctx = ResolveContext {
            viewport_width: 800.0,
            viewport_height: 600.0,
            em_base: 16.0,
            root_font_size: 16.0,
        };
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!(style.has_perspective);
        assert_eq!(style.perspective, Some(800.0));
    }

    #[test]
    fn resolve_will_change_sets_stacking() {
        let mut winners = HashMap::new();
        let wc_val = CssValue::List(vec![CssValue::Keyword("transform".to_string())]);
        winners.insert("will-change", &wc_val);

        let parent = ComputedStyle::default();
        let ctx = ResolveContext {
            viewport_width: 800.0,
            viewport_height: 600.0,
            em_base: 16.0,
            root_font_size: 16.0,
        };
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!(style.will_change_stacking);
        assert_eq!(style.will_change, vec!["transform".to_string()]);
    }
}
