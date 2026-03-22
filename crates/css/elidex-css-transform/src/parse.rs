//! Parser functions for CSS Transforms L1/L2 properties.

use elidex_plugin::{
    css_resolve::{parse_length_or_percentage, parse_non_negative_length},
    CssValue, LengthUnit, ParseError, TransformFunction,
};

pub(crate) fn parse_keyword(
    input: &mut cssparser::Parser<'_, '_>,
    allowed: &[&str],
) -> Result<CssValue, ParseError> {
    elidex_plugin::parse_css_keyword(input, allowed)
}

// ---------------------------------------------------------------------------
// transform parsing
// ---------------------------------------------------------------------------

/// Maximum number of transform functions in a single `transform` declaration.
/// Prevents unbounded memory growth from pathological inputs.
const MAX_TRANSFORM_FUNCTIONS: usize = 256;

pub(crate) fn parse_transform(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    // Try `none` first
    if input.try_parse(|i| i.expect_ident_matching("none")).is_ok() {
        return Ok(CssValue::Keyword("none".to_string()));
    }

    let mut functions = Vec::new();
    loop {
        if functions.len() >= MAX_TRANSFORM_FUNCTIONS {
            break;
        }
        let func = input.try_parse(parse_transform_function);
        match func {
            Ok(f) => functions.push(f),
            Err(_) => break,
        }
    }
    if functions.is_empty() {
        return Err(ParseError::simple("expected transform function or 'none'"));
    }
    Ok(CssValue::TransformList(functions))
}

fn parse_transform_function(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<TransformFunction, ParseError> {
    let function = input
        .expect_function()
        .map_err(|_| ParseError::simple("expected function"))?
        .to_ascii_lowercase();

    // We can't use parse_nested_block with our ParseError type directly,
    // so we parse manually by consuming the block tokens.
    parse_transform_function_args(input, &function)
}

fn parse_transform_function_args(
    input: &mut cssparser::Parser<'_, '_>,
    function: &str,
) -> Result<TransformFunction, ParseError> {
    input
        .parse_nested_block(
            |args| -> Result<TransformFunction, cssparser::ParseError<'_, ()>> {
                let result = parse_function_inner(args, function);
                result.map_err(|_| args.new_custom_error(()))
            },
        )
        .map_err(|_| ParseError::simple("failed to parse transform function"))
}

#[allow(clippy::too_many_lines)]
fn parse_function_inner(
    args: &mut cssparser::Parser<'_, '_>,
    function: &str,
) -> Result<TransformFunction, ParseError> {
    match function {
        "translate" => {
            let x = parse_length_or_percentage(args)?;
            let y = if args.try_parse(cssparser::Parser::expect_comma).is_ok() {
                parse_length_or_percentage(args)?
            } else {
                CssValue::Length(0.0, LengthUnit::Px)
            };
            Ok(TransformFunction::Translate(x, y))
        }
        "translatex" => {
            let x = parse_length_or_percentage(args)?;
            Ok(TransformFunction::TranslateX(x))
        }
        "translatey" => {
            let y = parse_length_or_percentage(args)?;
            Ok(TransformFunction::TranslateY(y))
        }
        "translatez" => {
            let z = parse_length_only(args)?;
            Ok(TransformFunction::TranslateZ(z))
        }
        "translate3d" => {
            let x = parse_length_or_percentage(args)?;
            args.expect_comma()
                .map_err(|_| ParseError::simple("expected comma"))?;
            let y = parse_length_or_percentage(args)?;
            args.expect_comma()
                .map_err(|_| ParseError::simple("expected comma"))?;
            let z = parse_length_only(args)?;
            Ok(TransformFunction::Translate3d(x, y, z))
        }
        "rotate" => {
            let deg = parse_angle(args)?;
            Ok(TransformFunction::Rotate(deg))
        }
        "rotatex" => {
            let deg = parse_angle(args)?;
            Ok(TransformFunction::RotateX(deg))
        }
        "rotatey" => {
            let deg = parse_angle(args)?;
            Ok(TransformFunction::RotateY(deg))
        }
        "rotatez" => {
            let deg = parse_angle(args)?;
            Ok(TransformFunction::RotateZ(deg))
        }
        "rotate3d" => {
            let x = parse_number(args)?;
            args.expect_comma()
                .map_err(|_| ParseError::simple("expected comma"))?;
            let y = parse_number(args)?;
            args.expect_comma()
                .map_err(|_| ParseError::simple("expected comma"))?;
            let z = parse_number(args)?;
            args.expect_comma()
                .map_err(|_| ParseError::simple("expected comma"))?;
            let deg = parse_angle(args)?;
            Ok(TransformFunction::Rotate3d(
                f64::from(x),
                f64::from(y),
                f64::from(z),
                deg,
            ))
        }
        "scale" => {
            let sx = parse_number(args)?;
            let sy = if args.try_parse(cssparser::Parser::expect_comma).is_ok() {
                parse_number(args)?
            } else {
                sx
            };
            Ok(TransformFunction::Scale(sx, sy))
        }
        "scalex" => {
            let sx = parse_number(args)?;
            Ok(TransformFunction::ScaleX(sx))
        }
        "scaley" => {
            let sy = parse_number(args)?;
            Ok(TransformFunction::ScaleY(sy))
        }
        "scalez" => {
            let sz = parse_number(args)?;
            Ok(TransformFunction::ScaleZ(sz))
        }
        "scale3d" => {
            let sx = parse_number(args)?;
            args.expect_comma()
                .map_err(|_| ParseError::simple("expected comma"))?;
            let sy = parse_number(args)?;
            args.expect_comma()
                .map_err(|_| ParseError::simple("expected comma"))?;
            let sz = parse_number(args)?;
            Ok(TransformFunction::Scale3d(sx, sy, sz))
        }
        "skew" => {
            let ax = parse_angle(args)?;
            let ay = if args.try_parse(cssparser::Parser::expect_comma).is_ok() {
                parse_angle(args)?
            } else {
                0.0
            };
            Ok(TransformFunction::Skew(ax, ay))
        }
        "skewx" => {
            let ax = parse_angle(args)?;
            Ok(TransformFunction::SkewX(ax))
        }
        "skewy" => {
            let ay = parse_angle(args)?;
            Ok(TransformFunction::SkewY(ay))
        }
        "matrix" => {
            let mut vals = [0.0_f64; 6];
            for (i, v) in vals.iter_mut().enumerate() {
                if i > 0 {
                    args.expect_comma()
                        .map_err(|_| ParseError::simple("expected comma"))?;
                }
                *v = f64::from(parse_number(args)?);
            }
            Ok(TransformFunction::Matrix(vals))
        }
        "matrix3d" => {
            let mut vals = [0.0_f64; 16];
            for (i, v) in vals.iter_mut().enumerate() {
                if i > 0 {
                    args.expect_comma()
                        .map_err(|_| ParseError::simple("expected comma"))?;
                }
                *v = f64::from(parse_number(args)?);
            }
            Ok(TransformFunction::Matrix3d(vals))
        }
        // CSS Transforms L2 §7.1: perspective(<length [0,∞]> | none)
        // Distinct from the `perspective` *property* — this is a transform function
        // that applies a perspective projection matrix to the current element.
        // `none` produces an identity matrix (same as d=0 in perspective_4x4).
        "perspective" => {
            if args.try_parse(|i| i.expect_ident_matching("none")).is_ok() {
                return Ok(TransformFunction::PerspectiveFunc(0.0));
            }
            let val = parse_non_negative_length(args)?;
            let px = match val {
                CssValue::Length(v, _) => v,
                _ => 0.0,
            };
            Ok(TransformFunction::PerspectiveFunc(px))
        }
        _ => Err(ParseError::simple("unknown transform function")),
    }
}

fn parse_angle(input: &mut cssparser::Parser<'_, '_>) -> Result<f32, ParseError> {
    let token = input
        .next()
        .map_err(|_| ParseError::simple("expected angle"))?;
    match *token {
        cssparser::Token::Dimension {
            value, ref unit, ..
        } => {
            let deg = match unit.to_ascii_lowercase().as_str() {
                "deg" => value,
                "rad" => value * 180.0 / std::f32::consts::PI,
                "grad" => value * 0.9,
                "turn" => value * 360.0,
                _ => return Err(ParseError::simple("unknown angle unit")),
            };
            // Guard against overflow from extreme values (e.g. 1e38rad → Infinity deg).
            if !deg.is_finite() {
                return Err(ParseError::simple("angle value out of range"));
            }
            Ok(deg)
        }
        cssparser::Token::Number { value: 0.0, .. } => Ok(0.0),
        _ => Err(ParseError::simple("expected angle")),
    }
}

/// Parse `<length>` only (no percentage). Accepts bare `0`.
///
/// CSS Transforms L2 §12.1: `translateZ()` and the Z argument of `translate3d()`
/// accept `<length>` only — there is no reference box on the Z axis for percentages.
fn parse_length_only(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    let token = input
        .next()
        .map_err(|_| ParseError::simple("expected length value"))?;
    match *token {
        cssparser::Token::Dimension {
            value, ref unit, ..
        } => {
            let unit = elidex_plugin::css_resolve::parse_length_unit(unit);
            Ok(CssValue::Length(value, unit))
        }
        cssparser::Token::Number { value: 0.0, .. } => Ok(CssValue::Length(0.0, LengthUnit::Px)),
        _ => Err(ParseError::simple("expected length value")),
    }
}

fn parse_number(input: &mut cssparser::Parser<'_, '_>) -> Result<f32, ParseError> {
    input
        .expect_number()
        .map_err(|_| ParseError::simple("expected number"))
}

// ---------------------------------------------------------------------------
// origin parsing
// ---------------------------------------------------------------------------

/// Parse `transform-origin`: 1-3 values (X Y Z). Z is `<length>` only (default 0).
pub(crate) fn parse_transform_origin(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    let mut result = parse_origin(input)?;
    // CSS Transforms L2 §4: optional 3rd value is `<length>` (Z offset).
    if let CssValue::List(ref mut parts) = result {
        if let Ok(z) = input.try_parse(parse_length_only) {
            parts.push(z);
        }
    }
    Ok(result)
}

/// Parse 1-2 value origin (shared by perspective-origin and transform-origin base).
pub(crate) fn parse_origin(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    let (first_val, first_axis) = parse_origin_value_with_axis(input)?;
    let (second_val, second_axis) = input
        .try_parse(parse_origin_value_with_axis)
        .unwrap_or((CssValue::Percentage(50.0), OriginAxis::Either));

    // CSS Transforms L1 §4: If two keywords are given, one must be X-axis
    // and one Y-axis. Reject same-axis pairs like `left right` or `top bottom`.
    match (first_axis, second_axis) {
        (OriginAxis::X, OriginAxis::X) | (OriginAxis::Y, OriginAxis::Y) => {
            return Err(ParseError::simple(
                "transform-origin: conflicting axis keywords",
            ));
        }
        // Y keyword first → swap to (X, Y) order.
        // Covers both 2-value "top left" and 1-value "top" (where second defaults to center/Either).
        (OriginAxis::Y, OriginAxis::X | OriginAxis::Either) => {
            return Ok(CssValue::List(vec![second_val, first_val]));
        }
        _ => {}
    }
    Ok(CssValue::List(vec![first_val, second_val]))
}

/// Which axis a keyword belongs to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OriginAxis {
    X,      // left, right
    Y,      // top, bottom
    Either, // center, length, percentage
}

fn parse_origin_value_with_axis(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<(CssValue, OriginAxis), ParseError> {
    if let Ok(ident) = input.try_parse(cssparser::Parser::expect_ident_cloned) {
        let lower = ident.to_ascii_lowercase();
        return match lower.as_str() {
            "left" => Ok((CssValue::Percentage(0.0), OriginAxis::X)),
            "right" => Ok((CssValue::Percentage(100.0), OriginAxis::X)),
            "top" => Ok((CssValue::Percentage(0.0), OriginAxis::Y)),
            "bottom" => Ok((CssValue::Percentage(100.0), OriginAxis::Y)),
            "center" => Ok((CssValue::Percentage(50.0), OriginAxis::Either)),
            _ => Err(ParseError::simple("unknown origin keyword")),
        };
    }
    let val = parse_length_or_percentage(input)?;
    Ok((val, OriginAxis::Either))
}

// ---------------------------------------------------------------------------
// perspective property parsing
// ---------------------------------------------------------------------------

pub(crate) fn parse_perspective_property(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    if input.try_parse(|i| i.expect_ident_matching("none")).is_ok() {
        return Ok(CssValue::Keyword("none".to_string()));
    }
    // Positive length only
    parse_non_negative_length(input)
}

// ---------------------------------------------------------------------------
// will-change parsing
// ---------------------------------------------------------------------------

pub(crate) fn parse_will_change(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    if input.try_parse(|i| i.expect_ident_matching("auto")).is_ok() {
        return Ok(CssValue::Keyword("auto".to_string()));
    }

    let mut props = Vec::new();
    loop {
        if !props.is_empty() && input.try_parse(cssparser::Parser::expect_comma).is_err() {
            break;
        }
        let ident = input
            .expect_ident()
            .map_err(|_| ParseError::simple("expected identifier"))?;
        props.push(CssValue::Keyword(ident.to_ascii_lowercase()));
    }
    if props.is_empty() {
        return Err(ParseError::simple("expected will-change value"));
    }
    Ok(CssValue::List(props))
}
