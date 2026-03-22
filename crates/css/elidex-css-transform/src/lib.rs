//! CSS Transforms L1/L2 and will-change property handler.
//!
//! Properties: `transform`, `transform-origin`, `perspective`,
//! `perspective-origin`, `transform-style`, `backface-visibility`, `will-change`.

mod parse;

#[cfg(test)]
mod tests;

use elidex_plugin::{
    css_resolve::resolve_length, BackfaceVisibility, ComputedStyle, CssPropertyHandler, CssValue,
    Dimension, LengthUnit, ParseError, PropertyDeclaration, ResolveContext, TransformFunction,
    TransformStyle,
};

/// CSS Transforms L1/L2 + will-change property handler.
#[derive(Clone)]
pub struct TransformHandler;

impl TransformHandler {
    /// Register this handler in a CSS property registry.
    pub fn register(registry: &mut elidex_plugin::CssPropertyRegistry) {
        elidex_plugin::register_css_handler(registry, Self);
    }
}

impl CssPropertyHandler for TransformHandler {
    fn property_names(&self) -> &[&str] {
        &[
            "transform",
            "transform-origin",
            "perspective",
            "perspective-origin",
            "transform-style",
            "backface-visibility",
            "will-change",
        ]
    }

    fn parse(
        &self,
        name: &str,
        input: &mut cssparser::Parser<'_, '_>,
    ) -> Result<Vec<PropertyDeclaration>, ParseError> {
        let value = match name {
            "transform" => parse::parse_transform(input)?,
            "transform-origin" => parse::parse_transform_origin(input)?,
            "perspective-origin" => parse::parse_origin(input)?,
            "perspective" => parse::parse_perspective_property(input)?,
            "transform-style" => parse::parse_keyword(input, &["flat", "preserve-3d"])?,
            "backface-visibility" => parse::parse_keyword(input, &["visible", "hidden"])?,
            "will-change" => parse::parse_will_change(input)?,
            _ => return Ok(vec![]),
        };
        Ok(vec![PropertyDeclaration::new(name, value)])
    }

    fn resolve(
        &self,
        name: &str,
        value: &CssValue,
        ctx: &ResolveContext,
        style: &mut ComputedStyle,
    ) {
        match name {
            "transform" => match value {
                CssValue::Keyword(k) if k == "none" => {
                    style.transform.clear();
                    style.has_transform = false;
                }
                CssValue::TransformList(funcs) => {
                    style.transform = funcs
                        .iter()
                        .map(|f| resolve_transform_func(f, ctx))
                        .collect();
                    style.has_transform = !style.transform.is_empty();
                }
                _ => {}
            },
            "transform-origin" => {
                if let CssValue::List(parts) = value {
                    if let Some(x) = parts.first() {
                        style.transform_origin.0 = resolve_origin_value(x, ctx);
                    }
                    if let Some(y) = parts.get(1) {
                        style.transform_origin.1 = resolve_origin_value(y, ctx);
                    }
                    // CSS Transforms L2 §4: 3rd value is Z offset (<length> only, default 0).
                    if let Some(CssValue::Length(v, unit)) = parts.get(2) {
                        let z = resolve_length(*v, *unit, ctx);
                        style.transform_origin.2 = if z.is_finite() { z } else { 0.0 };
                    } else {
                        style.transform_origin.2 = 0.0;
                    }
                }
            }
            "perspective" => match value {
                CssValue::Keyword(k) if k == "none" => {
                    style.perspective = None;
                    style.has_perspective = false;
                }
                CssValue::Length(v, unit) => {
                    let px = resolve_length(*v, *unit, ctx);
                    if px > 0.0 {
                        style.perspective = Some(px);
                        style.has_perspective = true;
                    } else {
                        style.perspective = None;
                        style.has_perspective = false;
                    }
                }
                _ => {}
            },
            "perspective-origin" => {
                if let CssValue::List(parts) = value {
                    if let Some(x) = parts.first() {
                        style.perspective_origin.0 = resolve_origin_value(x, ctx);
                    }
                    if let Some(y) = parts.get(1) {
                        style.perspective_origin.1 = resolve_origin_value(y, ctx);
                    }
                }
            }
            "transform-style" => {
                if let CssValue::Keyword(k) = value {
                    style.transform_style = match k.as_str() {
                        "preserve-3d" => TransformStyle::Preserve3d,
                        _ => TransformStyle::Flat,
                    };
                }
            }
            "backface-visibility" => {
                if let CssValue::Keyword(k) = value {
                    style.backface_visibility = match k.as_str() {
                        "hidden" => BackfaceVisibility::Hidden,
                        _ => BackfaceVisibility::Visible,
                    };
                }
            }
            "will-change" => match value {
                CssValue::Keyword(k) if k == "auto" => {
                    style.will_change.clear();
                    style.will_change_stacking = false;
                }
                CssValue::List(items) => {
                    let props: Vec<String> = items
                        .iter()
                        .filter_map(|v| {
                            if let CssValue::Keyword(k) = v {
                                Some(k.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    style.will_change_stacking =
                        elidex_plugin::transform_math::will_change_creates_stacking(&props);
                    style.will_change = props;
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn initial_value(&self, name: &str) -> CssValue {
        match name {
            "transform" | "perspective" => CssValue::Keyword("none".to_string()),
            "transform-origin" => CssValue::List(vec![
                CssValue::Percentage(50.0),
                CssValue::Percentage(50.0),
                CssValue::Length(0.0, LengthUnit::Px),
            ]),
            "perspective-origin" => {
                CssValue::List(vec![CssValue::Percentage(50.0), CssValue::Percentage(50.0)])
            }
            "transform-style" => CssValue::Keyword("flat".to_string()),
            "backface-visibility" => CssValue::Keyword("visible".to_string()),
            "will-change" => CssValue::Keyword("auto".to_string()),
            _ => CssValue::Initial,
        }
    }

    fn is_inherited(&self, _name: &str) -> bool {
        false
    }

    fn affects_layout(&self, _name: &str) -> bool {
        false
    }

    fn get_computed(&self, name: &str, style: &ComputedStyle) -> CssValue {
        match name {
            "transform" => {
                if style.transform.is_empty() {
                    CssValue::Keyword("none".to_string())
                } else {
                    CssValue::TransformList(style.transform.clone())
                }
            }
            "transform-origin" => {
                let mut parts = vec![
                    dim_to_css(style.transform_origin.0),
                    dim_to_css(style.transform_origin.1),
                ];
                // Only include Z when non-zero (CSS serialization convention).
                if style.transform_origin.2.abs() > f32::EPSILON {
                    parts.push(CssValue::Length(style.transform_origin.2, LengthUnit::Px));
                }
                CssValue::List(parts)
            }
            "perspective" => match style.perspective {
                Some(px) => CssValue::Length(px, LengthUnit::Px),
                None => CssValue::Keyword("none".to_string()),
            },
            "perspective-origin" => CssValue::List(vec![
                dim_to_css(style.perspective_origin.0),
                dim_to_css(style.perspective_origin.1),
            ]),
            "transform-style" => match style.transform_style {
                TransformStyle::Flat => CssValue::Keyword("flat".to_string()),
                TransformStyle::Preserve3d => CssValue::Keyword("preserve-3d".to_string()),
            },
            "backface-visibility" => match style.backface_visibility {
                BackfaceVisibility::Visible => CssValue::Keyword("visible".to_string()),
                BackfaceVisibility::Hidden => CssValue::Keyword("hidden".to_string()),
            },
            "will-change" => {
                if style.will_change.is_empty() {
                    CssValue::Keyword("auto".to_string())
                } else {
                    CssValue::List(
                        style
                            .will_change
                            .iter()
                            .map(|s| CssValue::Keyword(s.clone()))
                            .collect(),
                    )
                }
            }
            _ => CssValue::Initial,
        }
    }
}

/// Resolve relative length units (em, rem, vw, etc.) inside a `TransformFunction`.
///
/// Translate arguments may contain `CssValue::Length` with relative units that
/// must be resolved to px during style resolution. Other functions (rotate, scale,
/// skew, matrix) use angles or numbers and need no resolution.
fn resolve_transform_func(func: &TransformFunction, ctx: &ResolveContext) -> TransformFunction {
    match func {
        TransformFunction::Translate(x, y) => {
            TransformFunction::Translate(resolve_css_length(x, ctx), resolve_css_length(y, ctx))
        }
        TransformFunction::TranslateX(x) => {
            TransformFunction::TranslateX(resolve_css_length(x, ctx))
        }
        TransformFunction::TranslateY(y) => {
            TransformFunction::TranslateY(resolve_css_length(y, ctx))
        }
        TransformFunction::TranslateZ(z) => {
            TransformFunction::TranslateZ(resolve_css_length(z, ctx))
        }
        TransformFunction::Translate3d(x, y, z) => TransformFunction::Translate3d(
            resolve_css_length(x, ctx),
            resolve_css_length(y, ctx),
            resolve_css_length(z, ctx),
        ),
        // All other functions use angles, numbers, or raw matrices — no resolution needed.
        other => other.clone(),
    }
}

/// Resolve a `CssValue::Length` with relative units to px.
/// Percentages and already-px values pass through unchanged.
fn resolve_css_length(val: &CssValue, ctx: &ResolveContext) -> CssValue {
    match val {
        CssValue::Length(v, unit) => {
            let px = resolve_length(*v, *unit, ctx);
            // Guard against NaN/Infinity from degenerate inputs (e.g. extreme em/vw values).
            CssValue::Length(if px.is_finite() { px } else { 0.0 }, LengthUnit::Px)
        }
        // Percentages stay as-is (resolved against element's box at render time).
        other => other.clone(),
    }
}

fn dim_to_css(d: Dimension) -> CssValue {
    match d {
        Dimension::Length(px) => CssValue::Length(px, LengthUnit::Px),
        Dimension::Percentage(p) => CssValue::Percentage(p),
        Dimension::Auto => CssValue::Percentage(50.0),
    }
}

fn resolve_origin_value(value: &CssValue, ctx: &ResolveContext) -> Dimension {
    match value {
        CssValue::Keyword(k) => match k.as_str() {
            "left" | "top" => Dimension::Percentage(0.0),
            "right" | "bottom" => Dimension::Percentage(100.0),
            _ => Dimension::Percentage(50.0),
        },
        CssValue::Length(v, unit) => Dimension::Length(resolve_length(*v, *unit, ctx)),
        CssValue::Percentage(p) => Dimension::Percentage(*p),
        _ => Dimension::Percentage(50.0),
    }
}
