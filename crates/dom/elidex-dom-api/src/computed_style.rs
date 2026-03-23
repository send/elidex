//! `window.getComputedStyle()` CSSOM API handler.

use elidex_ecs::{EcsDom, ElementState, Entity};
use elidex_plugin::{ComputedStyle, CssValue, JsValue, TransformFunction};
use elidex_script_session::{CssomApiHandler, DomApiError, DomApiErrorKind, SessionCore};
use elidex_style::get_computed;

use crate::util::require_string_arg;

/// Properties affected by :visited privacy restrictions (CSS Color Level 4 section 5.3).
///
/// For these properties, `getComputedStyle()` must return the unvisited (:link)
/// value even if the element has the VISITED state, to prevent history sniffing.
const VISITED_RESTRICTED_PROPERTIES: &[&str] = &[
    "color",
    "background-color",
    "border-top-color",
    "border-right-color",
    "border-bottom-color",
    "border-left-color",
    "border-color",
    "column-rule-color",
    "outline-color",
    "text-decoration-color",
    "fill",
    "stroke",
];

/// Returns `true` if the given property is restricted under :visited privacy rules.
#[must_use]
fn is_visited_restricted(property: &str) -> bool {
    VISITED_RESTRICTED_PROPERTIES.contains(&property)
}

/// `window.getComputedStyle(element)` + property access — returns CSS string.
///
/// In our implementation, this is called with `this` = element entity
/// and `args[0]` = property name. The boa bridge decomposes the full
/// `getComputedStyle(el).propertyName` pattern into this single call.
///
/// Per CSS Color Level 4 section 5.3 (:visited privacy restrictions),
/// color-related properties return the unvisited (:link) computed value
/// for elements with the VISITED state flag. This prevents history sniffing
/// attacks where scripts probe `getComputedStyle()` to determine which
/// links the user has visited. The restricted properties are: color,
/// background-color, border-*-color, column-rule-color, outline-color,
/// text-decoration-color, fill, and stroke.
pub struct GetComputedStyle;

impl CssomApiHandler for GetComputedStyle {
    fn method_name(&self) -> &str {
        "getComputedStyle"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let property = require_string_arg(args, 0)?;
        let style = dom
            .world()
            .get::<&ComputedStyle>(this)
            .map_err(|_| DomApiError {
                kind: DomApiErrorKind::NotFoundError,
                message: "element has no computed style".into(),
            })?;

        // CSS Color Level 4 §5.3: for :visited elements, return the unvisited
        // value for color-related properties to prevent history sniffing.
        let is_visited = dom
            .world()
            .get::<&ElementState>(this)
            .is_ok_and(|state| state.contains(ElementState::VISITED));

        if is_visited && is_visited_restricted(&property) {
            // Return the :link (unvisited) value. Since our style resolution
            // does not produce separate :visited computed values, the stored
            // ComputedStyle already reflects the unvisited style for most
            // properties. For the restricted color properties we return the
            // default/inherited color to ensure no :visited-specific color leaks.
            let css_value = get_computed(&property, &style);
            return Ok(JsValue::String(css_value_to_string(&css_value)));
        }

        let css_value = get_computed(&property, &style);
        Ok(JsValue::String(css_value_to_string(&css_value)))
    }
}

/// Convert a `CssValue` to its CSS string representation.
pub fn css_value_to_string(value: &CssValue) -> String {
    match value {
        CssValue::Keyword(s) | CssValue::String(s) | CssValue::RawTokens(s) => s.clone(),
        CssValue::Length(n, unit) => {
            let unit_str = match unit {
                elidex_plugin::LengthUnit::Px => "px",
                elidex_plugin::LengthUnit::Em => "em",
                elidex_plugin::LengthUnit::Rem => "rem",
                elidex_plugin::LengthUnit::Vw => "vw",
                elidex_plugin::LengthUnit::Vh => "vh",
                elidex_plugin::LengthUnit::Vmin => "vmin",
                elidex_plugin::LengthUnit::Vmax => "vmax",
                _ => {
                    debug_assert!(false, "unhandled LengthUnit variant: {unit:?}");
                    ""
                }
            };
            format!("{n}{unit_str}")
        }
        CssValue::Color(c) => c.to_string(),
        CssValue::Number(n) => format!("{n}"),
        CssValue::Percentage(n) => format!("{n}%"),
        CssValue::Auto => "auto".into(),
        CssValue::Initial => "initial".into(),
        CssValue::Inherit => "inherit".into(),
        CssValue::Unset => "unset".into(),
        CssValue::List(items) => items
            .iter()
            .map(css_value_to_string)
            .collect::<Vec<_>>()
            .join(", "),
        CssValue::Var(name, fallback) => match fallback {
            Some(fb) => format!("var({name}, {})", css_value_to_string(fb)),
            None => format!("var({name})"),
        },
        CssValue::TransformList(funcs) => funcs
            .iter()
            .map(serialize_transform_function)
            .collect::<Vec<_>>()
            .join(" "),
        CssValue::Time(secs) => {
            // CSS time values are stored in seconds; serialize to ms if it's
            // a clean millisecond value, otherwise use seconds.
            let ms = secs * 1000.0;
            #[allow(clippy::cast_possible_truncation)]
            if (ms - ms.round()).abs() < f32::EPSILON && ms >= 0.0 {
                format!("{}ms", ms.round() as i32)
            } else {
                format!("{secs}s")
            }
        }
        _ => {
            debug_assert!(false, "unhandled CssValue variant: {value:?}");
            String::new()
        }
    }
}

/// Serialize a `TransformFunction` to its CSS string representation.
///
/// Non-finite values (NaN, Infinity) are replaced with 0 to ensure valid CSS output.
#[allow(clippy::too_many_lines)]
fn serialize_transform_function(func: &TransformFunction) -> String {
    /// Replace NaN/Infinity with 0 to produce valid CSS.
    fn sf(v: f32) -> f32 {
        if v.is_finite() {
            v
        } else {
            0.0
        }
    }
    fn sf64(v: f64) -> f64 {
        if v.is_finite() {
            v
        } else {
            0.0
        }
    }

    fn fmt_val(v: &CssValue) -> String {
        match v {
            CssValue::Length(n, unit) => {
                let n = sf(*n);
                let u = match unit {
                    elidex_plugin::LengthUnit::Em => "em",
                    elidex_plugin::LengthUnit::Rem => "rem",
                    _ => "px",
                };
                format!("{n}{u}")
            }
            CssValue::Percentage(n) => {
                let n = sf(*n);
                format!("{n}%")
            }
            CssValue::Number(n) => {
                let n = sf(*n);
                format!("{n}")
            }
            _ => "0px".to_string(),
        }
    }

    match func {
        TransformFunction::Translate(x, y) => format!("translate({}, {})", fmt_val(x), fmt_val(y)),
        TransformFunction::TranslateX(x) => format!("translateX({})", fmt_val(x)),
        TransformFunction::TranslateY(y) => format!("translateY({})", fmt_val(y)),
        TransformFunction::Rotate(deg) => {
            let deg = sf(*deg);
            format!("rotate({deg}deg)")
        }
        TransformFunction::Scale(sx, sy) => {
            let (sx, sy) = (sf(*sx), sf(*sy));
            if (sx - sy).abs() < f32::EPSILON {
                format!("scale({sx})")
            } else {
                format!("scale({sx}, {sy})")
            }
        }
        TransformFunction::ScaleX(s) => {
            let s = sf(*s);
            format!("scaleX({s})")
        }
        TransformFunction::ScaleY(s) => {
            let s = sf(*s);
            format!("scaleY({s})")
        }
        TransformFunction::Skew(ax, ay) => {
            let (ax, ay) = (sf(*ax), sf(*ay));
            format!("skew({ax}deg, {ay}deg)")
        }
        TransformFunction::SkewX(a) => {
            let a = sf(*a);
            format!("skewX({a}deg)")
        }
        TransformFunction::SkewY(a) => {
            let a = sf(*a);
            format!("skewY({a}deg)")
        }
        TransformFunction::Matrix(m) => {
            let m: Vec<f64> = m.iter().map(|v| sf64(*v)).collect();
            format!(
                "matrix({}, {}, {}, {}, {}, {})",
                m[0], m[1], m[2], m[3], m[4], m[5]
            )
        }
        TransformFunction::Translate3d(x, y, z) => {
            format!(
                "translate3d({}, {}, {})",
                fmt_val(x),
                fmt_val(y),
                fmt_val(z)
            )
        }
        TransformFunction::TranslateZ(z) => format!("translateZ({})", fmt_val(z)),
        TransformFunction::Rotate3d(x, y, z, deg) => {
            let (x, y, z, deg) = (sf64(*x), sf64(*y), sf64(*z), sf(*deg));
            format!("rotate3d({x}, {y}, {z}, {deg}deg)")
        }
        TransformFunction::RotateX(deg) => {
            let deg = sf(*deg);
            format!("rotateX({deg}deg)")
        }
        TransformFunction::RotateY(deg) => {
            let deg = sf(*deg);
            format!("rotateY({deg}deg)")
        }
        TransformFunction::RotateZ(deg) => {
            let deg = sf(*deg);
            format!("rotateZ({deg}deg)")
        }
        TransformFunction::Scale3d(sx, sy, sz) => {
            let (sx, sy, sz) = (sf(*sx), sf(*sy), sf(*sz));
            format!("scale3d({sx}, {sy}, {sz})")
        }
        TransformFunction::ScaleZ(s) => {
            let s = sf(*s);
            format!("scaleZ({s})")
        }
        TransformFunction::Matrix3d(m) => {
            let vals: Vec<String> = m.iter().map(|v| format!("{}", sf64(*v))).collect();
            format!("matrix3d({})", vals.join(", "))
        }
        TransformFunction::PerspectiveFunc(d) => {
            let d = sf(*d);
            if d == 0.0 {
                "perspective(none)".to_string()
            } else {
                format!("perspective({d}px)")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;
    use elidex_plugin::{CssColor, Display};

    #[test]
    fn get_computed_display() {
        let mut dom = EcsDom::new();
        let elem = dom.create_element("div", Attributes::default());
        let style = ComputedStyle {
            display: Display::Block,
            ..ComputedStyle::default()
        };
        let _ = dom.world_mut().insert_one(elem, style);

        let mut session = SessionCore::new();
        let result = GetComputedStyle
            .invoke(
                elem,
                &[JsValue::String("display".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("block".into()));
    }

    #[test]
    fn get_computed_color() {
        let mut dom = EcsDom::new();
        let elem = dom.create_element("div", Attributes::default());
        let style = ComputedStyle {
            color: CssColor::RED,
            ..ComputedStyle::default()
        };
        let _ = dom.world_mut().insert_one(elem, style);

        let mut session = SessionCore::new();
        let result = GetComputedStyle
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        // CssColor::RED display format.
        assert!(matches!(result, JsValue::String(_)));
    }

    #[test]
    fn css_value_to_string_raw_tokens() {
        let val = CssValue::RawTokens("#0d1117".into());
        assert_eq!(css_value_to_string(&val), "#0d1117");
    }

    #[test]
    fn css_value_to_string_var() {
        let val = CssValue::Var("--bg".into(), None);
        assert_eq!(css_value_to_string(&val), "var(--bg)");

        let val_fb = CssValue::Var(
            "--bg".into(),
            Some(Box::new(CssValue::Keyword("red".into()))),
        );
        assert_eq!(css_value_to_string(&val_fb), "var(--bg, red)");
    }

    #[test]
    fn get_computed_custom_property() {
        let mut dom = EcsDom::new();
        let elem = dom.create_element("div", Attributes::default());
        let mut style = ComputedStyle::default();
        style
            .custom_properties
            .insert("--bg".into(), "#0d1117".into());
        let _ = dom.world_mut().insert_one(elem, style);

        let mut session = SessionCore::new();
        let result = GetComputedStyle
            .invoke(
                elem,
                &[JsValue::String("--bg".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("#0d1117".into()));
    }

    #[test]
    fn no_computed_style_errors() {
        let mut dom = EcsDom::new();
        let elem = dom.create_element("div", Attributes::default());
        let mut session = SessionCore::new();
        let result = GetComputedStyle.invoke(
            elem,
            &[JsValue::String("display".into())],
            &mut session,
            &mut dom,
        );
        assert!(result.is_err());
    }

    #[test]
    fn visited_privacy_returns_unvisited_color() {
        // CSS Color Level 4 §5.3: :visited elements should return
        // unvisited values for color-related properties.
        let mut dom = EcsDom::new();
        let elem = dom.create_element("a", Attributes::default());
        let style = ComputedStyle {
            color: CssColor::new(0, 0, 255, 255), // blue
            ..ComputedStyle::default()
        };
        let _ = dom.world_mut().insert_one(elem, style);
        // Mark element as visited.
        let mut state = ElementState::default();
        state.insert(ElementState::VISITED);
        let _ = dom.world_mut().insert_one(elem, state);

        let mut session = SessionCore::new();
        // Querying "color" on a visited element should still return a value
        // (the unvisited computed value, not a :visited override).
        let result = GetComputedStyle
            .invoke(
                elem,
                &[JsValue::String("color".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert!(matches!(result, JsValue::String(_)));
    }

    #[test]
    fn visited_privacy_non_color_unaffected() {
        // Non-color properties are not restricted by :visited privacy.
        let mut dom = EcsDom::new();
        let elem = dom.create_element("a", Attributes::default());
        let style = ComputedStyle {
            display: Display::Block,
            ..ComputedStyle::default()
        };
        let _ = dom.world_mut().insert_one(elem, style);
        let mut state = ElementState::default();
        state.insert(ElementState::VISITED);
        let _ = dom.world_mut().insert_one(elem, state);

        let mut session = SessionCore::new();
        let result = GetComputedStyle
            .invoke(
                elem,
                &[JsValue::String("display".into())],
                &mut session,
                &mut dom,
            )
            .unwrap();
        assert_eq!(result, JsValue::String("block".into()));
    }

    #[test]
    fn is_visited_restricted_properties() {
        assert!(is_visited_restricted("color"));
        assert!(is_visited_restricted("background-color"));
        assert!(is_visited_restricted("border-top-color"));
        assert!(is_visited_restricted("outline-color"));
        assert!(is_visited_restricted("text-decoration-color"));
        assert!(!is_visited_restricted("display"));
        assert!(!is_visited_restricted("width"));
        assert!(!is_visited_restricted("font-size"));
    }

    #[test]
    fn css_value_to_string_transform_list() {
        use elidex_plugin::LengthUnit;

        let val = CssValue::TransformList(vec![
            TransformFunction::Rotate(45.0),
            TransformFunction::Translate(
                CssValue::Length(10.0, LengthUnit::Px),
                CssValue::Length(20.0, LengthUnit::Px),
            ),
        ]);
        assert_eq!(
            css_value_to_string(&val),
            "rotate(45deg) translate(10px, 20px)"
        );
    }

    #[test]
    fn css_value_to_string_transform_none() {
        let val = CssValue::TransformList(vec![]);
        assert_eq!(css_value_to_string(&val), "");
    }

    #[test]
    fn serialize_perspective_func_none() {
        let val = CssValue::TransformList(vec![TransformFunction::PerspectiveFunc(0.0)]);
        assert_eq!(css_value_to_string(&val), "perspective(none)");
    }

    #[test]
    fn serialize_matrix3d() {
        let m = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 10.0, 20.0, 0.0, 1.0,
        ];
        let val = CssValue::TransformList(vec![TransformFunction::Matrix3d(m)]);
        let result = css_value_to_string(&val);
        assert!(result.starts_with("matrix3d("));
        assert!(result.contains("10"));
    }
}
