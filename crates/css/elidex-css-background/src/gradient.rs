//! Gradient parsing (linear, radial, conic).
//!
//! Parses CSS gradient functions into `CssValue::Gradient(GradientValue)`.

use elidex_plugin::{AngleOrDirection, CssColorStop, CssValue, GradientValue, ParseError};

/// Parse a CSS gradient function.
///
/// Supports `linear-gradient()`, `repeating-linear-gradient()`,
/// `radial-gradient()`, `repeating-radial-gradient()`,
/// `conic-gradient()`, `repeating-conic-gradient()`.
pub(crate) fn parse_gradient(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    let location = input.current_source_location();
    let func = input.expect_function().map_err(|_| ParseError {
        property: "background-image".into(),
        input: String::new(),
        message: "expected gradient function".into(),
    })?;
    let func_lower = func.to_ascii_lowercase();
    let func_name = func_lower.clone();

    input
        .parse_nested_block(|i| match func_name.as_str() {
            "linear-gradient" => parse_linear_inner(i, false),
            "repeating-linear-gradient" => parse_linear_inner(i, true),
            "radial-gradient" => parse_radial_inner(i, false),
            "repeating-radial-gradient" => parse_radial_inner(i, true),
            "conic-gradient" => parse_conic_inner(i, false),
            "repeating-conic-gradient" => parse_conic_inner(i, true),
            _ => Err(location.new_custom_error::<_, ()>(())),
        })
        .map_err(|_| ParseError {
            property: "background-image".into(),
            input: String::new(),
            message: format!("invalid gradient: {func_lower}"),
        })
}

fn parse_linear_inner<'i>(
    input: &mut cssparser::Parser<'i, '_>,
    repeating: bool,
) -> Result<CssValue, cssparser::ParseError<'i, ()>> {
    let direction = input
        .try_parse(
            |i| -> Result<AngleOrDirection, cssparser::ParseError<'i, ()>> {
                if let Ok(angle) = i.try_parse(parse_angle_token) {
                    i.expect_comma()?;
                    return Ok(AngleOrDirection::Angle(angle));
                }
                i.expect_ident_matching("to")?;
                let mut sides = Vec::new();
                while let Ok(ident) = i.try_parse(cssparser::Parser::expect_ident_cloned) {
                    let lower = ident.to_ascii_lowercase();
                    match lower.as_str() {
                        "top" | "bottom" | "left" | "right" => sides.push(lower),
                        _ => return Err(i.new_custom_error(())),
                    }
                    if sides.len() >= 2 {
                        break;
                    }
                }
                if sides.is_empty() {
                    return Err(i.new_custom_error(()));
                }
                i.expect_comma()?;
                Ok(AngleOrDirection::To(sides))
            },
        )
        .unwrap_or(AngleOrDirection::Angle(180.0));

    let stops = parse_color_stop_list(input)?;
    if stops.len() < 2 {
        return Err(input.new_custom_error(()));
    }

    Ok(CssValue::Gradient(Box::new(GradientValue::Linear {
        direction,
        stops,
        repeating,
    })))
}

fn parse_radial_inner<'i>(
    input: &mut cssparser::Parser<'i, '_>,
    repeating: bool,
) -> Result<CssValue, cssparser::ParseError<'i, ()>> {
    let (shape, size, position) = input
        .try_parse(|i| {
            let mut shape: Option<String> = None;
            let mut size: Option<String> = None;

            for _ in 0..2 {
                if shape.is_none() {
                    if let Ok(s) = i.try_parse(|i2| {
                        let ident = i2.expect_ident_cloned()?;
                        match ident.to_ascii_lowercase().as_str() {
                            "circle" | "ellipse" => Ok(ident.to_ascii_lowercase()),
                            _ => Err(i2.new_custom_error::<_, ()>(())),
                        }
                    }) {
                        shape = Some(s);
                        continue;
                    }
                }
                if size.is_none() {
                    if let Ok(s) = i.try_parse(|i2| {
                        let ident = i2.expect_ident_cloned()?;
                        match ident.to_ascii_lowercase().as_str() {
                            "closest-side" | "farthest-side" | "closest-corner"
                            | "farthest-corner" => Ok(ident.to_ascii_lowercase()),
                            _ => Err(i2.new_custom_error::<_, ()>(())),
                        }
                    }) {
                        size = Some(s);
                        continue;
                    }
                }
                break;
            }

            let position = i
                .try_parse(|i2| {
                    i2.expect_ident_matching("at")?;
                    let mut pos = Vec::new();
                    if let Ok(v) = i2.try_parse(parse_position_value) {
                        pos.push(v);
                    }
                    if let Ok(v) = i2.try_parse(parse_position_value) {
                        pos.push(v);
                    }
                    if pos.is_empty() {
                        Err(i2.new_custom_error::<_, ()>(()))
                    } else {
                        Ok(pos)
                    }
                })
                .ok();

            if shape.is_none() && size.is_none() && position.is_none() {
                return Err(i.new_custom_error::<_, ()>(()));
            }
            i.expect_comma()?;
            Ok((shape, size, position))
        })
        .unwrap_or((None, None, None));

    let stops = parse_color_stop_list(input)?;
    if stops.len() < 2 {
        return Err(input.new_custom_error(()));
    }

    Ok(CssValue::Gradient(Box::new(GradientValue::Radial {
        shape,
        size,
        position,
        stops,
        repeating,
    })))
}

fn parse_conic_inner<'i>(
    input: &mut cssparser::Parser<'i, '_>,
    repeating: bool,
) -> Result<CssValue, cssparser::ParseError<'i, ()>> {
    let (from_angle, position) = input
        .try_parse(|i| {
            let from_angle = i
                .try_parse(|i2| {
                    i2.expect_ident_matching("from")?;
                    parse_angle_token(i2)
                })
                .ok();

            let position = i
                .try_parse(|i2| {
                    i2.expect_ident_matching("at")?;
                    let mut pos = Vec::new();
                    if let Ok(v) = i2.try_parse(parse_position_value) {
                        pos.push(v);
                    }
                    if let Ok(v) = i2.try_parse(parse_position_value) {
                        pos.push(v);
                    }
                    if pos.is_empty() {
                        Err(i2.new_custom_error::<_, ()>(()))
                    } else {
                        Ok(pos)
                    }
                })
                .ok();

            if from_angle.is_none() && position.is_none() {
                return Err(i.new_custom_error::<_, ()>(()));
            }
            i.expect_comma()?;
            Ok((from_angle, position))
        })
        .unwrap_or((None, None));

    let stops = parse_color_stop_list(input)?;
    if stops.len() < 2 {
        return Err(input.new_custom_error(()));
    }

    Ok(CssValue::Gradient(Box::new(GradientValue::Conic {
        from_angle,
        position,
        stops,
        repeating,
    })))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_angle_token<'i>(
    input: &mut cssparser::Parser<'i, '_>,
) -> Result<f32, cssparser::ParseError<'i, ()>> {
    let token = input.next()?;
    match token {
        cssparser::Token::Dimension { value, unit, .. } => {
            let degrees = match unit.to_ascii_lowercase().as_str() {
                "deg" => *value,
                "rad" => value * 180.0 / std::f32::consts::PI,
                "grad" => value * 0.9,
                "turn" => value * 360.0,
                _ => return Err(input.new_custom_error(())),
            };
            Ok(degrees)
        }
        cssparser::Token::Number { value, .. } if *value == 0.0 => Ok(0.0),
        _ => Err(input.new_custom_error(())),
    }
}

fn parse_position_value<'i>(
    input: &mut cssparser::Parser<'i, '_>,
) -> Result<CssValue, cssparser::ParseError<'i, ()>> {
    if let Ok(ident) = input.try_parse(cssparser::Parser::expect_ident_cloned) {
        return match ident.to_ascii_lowercase().as_str() {
            "left" | "top" => Ok(CssValue::Percentage(0.0)),
            "center" => Ok(CssValue::Percentage(50.0)),
            "right" | "bottom" => Ok(CssValue::Percentage(100.0)),
            _ => Err(input.new_custom_error(())),
        };
    }
    if let Ok(pct) = input.try_parse(cssparser::Parser::expect_percentage) {
        return Ok(CssValue::Percentage(pct * 100.0));
    }
    let tok = input.next()?;
    match tok {
        cssparser::Token::Dimension { value, unit, .. } => {
            let lu = match unit.to_ascii_lowercase().as_str() {
                "em" => elidex_plugin::LengthUnit::Em,
                "rem" => elidex_plugin::LengthUnit::Rem,
                _ => elidex_plugin::LengthUnit::Px,
            };
            Ok(CssValue::Length(*value, lu))
        }
        cssparser::Token::Number { value, .. } if *value == 0.0 => {
            Ok(CssValue::Length(0.0, elidex_plugin::LengthUnit::Px))
        }
        _ => Err(input.new_custom_error(())),
    }
}

/// Parse a single stop position (percentage or length).
fn parse_stop_position<'i>(
    input: &mut cssparser::Parser<'i, '_>,
) -> Result<CssValue, cssparser::ParseError<'i, ()>> {
    if let Ok(pct) = input.try_parse(cssparser::Parser::expect_percentage) {
        return Ok(CssValue::Percentage(pct * 100.0));
    }
    let tok = input.next()?;
    match tok {
        cssparser::Token::Dimension { value, unit, .. } => {
            let lu = match unit.to_ascii_lowercase().as_str() {
                "em" => elidex_plugin::LengthUnit::Em,
                _ => elidex_plugin::LengthUnit::Px,
            };
            Ok(CssValue::Length(*value, lu))
        }
        cssparser::Token::Number { value, .. } if *value == 0.0 => {
            Ok(CssValue::Length(0.0, elidex_plugin::LengthUnit::Px))
        }
        _ => Err(input.new_custom_error(())),
    }
}

fn parse_color_stop_list<'i>(
    input: &mut cssparser::Parser<'i, '_>,
) -> Result<Vec<CssColorStop>, cssparser::ParseError<'i, ()>> {
    let mut stops = Vec::new();

    loop {
        let css_color =
            elidex_css::parse_color(input).map_err(|()| input.new_custom_error::<(), ()>(()))?;
        let color = CssValue::Color(css_color);

        // Try one or two position values
        let pos1 = input.try_parse(parse_stop_position).ok();

        let pos2 = if pos1.is_some() {
            input.try_parse(parse_stop_position).ok()
        } else {
            None
        };

        if let Some(p2) = pos2 {
            stops.push(CssColorStop {
                color: color.clone(),
                position: pos1,
            });
            stops.push(CssColorStop {
                color,
                position: Some(p2),
            });
        } else {
            stops.push(CssColorStop {
                color,
                position: pos1,
            });
        }

        if input.try_parse(cssparser::Parser::expect_comma).is_err() {
            break;
        }
    }

    Ok(stops)
}
