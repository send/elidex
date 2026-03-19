//! Flexbox property resolution.

use elidex_plugin::{
    AlignContent, AlignItems, AlignSelf, ComputedStyle, CssValue, Dimension, FlexDirection,
    FlexWrap, JustifyContent,
};

use super::helpers::resolve_keyword_enum_prop;
use super::helpers::{resolve_i32, resolve_non_negative_f32, resolve_prop, PropertyMap};

/// Resolve all flex-related properties.
pub(super) fn resolve_flex_properties(
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
        FlexDirection::from_keyword
    );
    resolve_keyword_enum_prop!(
        "flex-wrap",
        winners,
        parent_style,
        style.flex_wrap,
        FlexWrap::from_keyword
    );
    resolve_keyword_enum_prop!(
        "justify-content",
        winners,
        parent_style,
        style.justify_content,
        JustifyContent::from_keyword
    );
    resolve_keyword_enum_prop!(
        "align-items",
        winners,
        parent_style,
        style.align_items,
        AlignItems::from_keyword
    );
    resolve_keyword_enum_prop!(
        "align-content",
        winners,
        parent_style,
        style.align_content,
        AlignContent::from_keyword
    );
    resolve_keyword_enum_prop!(
        "align-self",
        winners,
        parent_style,
        style.align_self,
        AlignSelf::from_keyword
    );
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use elidex_plugin::{
        AlignContent, AlignItems, AlignSelf, ComputedStyle, CssValue, Dimension, FlexDirection,
        FlexWrap, JustifyContent, LengthUnit,
    };

    use crate::resolve::helpers::PropertyMap;
    use crate::resolve::{build_computed_style, ResolveContext};

    fn default_ctx() -> ResolveContext {
        ResolveContext {
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            em_base: 16.0,
            root_font_size: 16.0,
        }
    }

    #[test]
    fn resolve_flex_direction() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let val = CssValue::Keyword("column-reverse".to_string());
        winners.insert("flex-direction", &val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.flex_direction, FlexDirection::ColumnReverse);
    }

    #[test]
    fn resolve_flex_grow_shrink_clamping() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
        let grow = CssValue::Number(-5.0);
        let shrink = CssValue::Number(3.0);
        winners.insert("flex-grow", &grow);
        winners.insert("flex-shrink", &shrink);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.flex_grow, 0.0);
        assert_eq!(style.flex_shrink, 3.0);
    }

    #[test]
    fn resolve_flex_props() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let mut winners: PropertyMap = HashMap::new();
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
        let winners: PropertyMap = HashMap::new();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.flex_direction, FlexDirection::Row);
        assert_eq!(style.flex_wrap, FlexWrap::Nowrap);
        assert_eq!(style.justify_content, JustifyContent::Normal);
        assert_eq!(style.align_items, AlignItems::Stretch);
        assert_eq!(style.align_content, AlignContent::Normal);
        assert_eq!(style.flex_grow, 0.0);
        assert_eq!(style.flex_shrink, 1.0);
        assert_eq!(style.flex_basis, Dimension::Auto);
        assert_eq!(style.order, 0);
        assert_eq!(style.align_self, AlignSelf::Auto);
    }
}
