//! Flex property parsers: flex-grow/shrink, order, flex-basis, flex/flex-flow shorthands.

use cssparser::{Parser, Token};
use elidex_plugin::{CssValue, LengthUnit};

use crate::values::parse_length_or_percentage;

use super::{parse_value_property, single_decl, Declaration};

/// Parse a non-negative number property (flex-grow, flex-shrink).
pub(super) fn parse_non_negative_number(input: &mut Parser, name: &str) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let tok = i.next().map_err(|_| ())?;
            match *tok {
                Token::Number { value, .. } if value >= 0.0 => {
                    Ok(single_decl(name, CssValue::Number(value)))
                }
                _ => Err(()),
            }
        })
        .unwrap_or_default()
}

/// Parse an integer property (order).
pub(super) fn parse_integer_property(input: &mut Parser, name: &str) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let tok = i.next().map_err(|_| ())?;
            match *tok {
                Token::Number {
                    int_value: Some(n), ..
                } =>
                {
                    #[allow(clippy::cast_precision_loss)]
                    Ok(single_decl(name, CssValue::Number(n as f32)))
                }
                _ => Err(()),
            }
        })
        .unwrap_or_default()
}

/// Parse flex-basis: `auto` | `content` | length/percentage.
pub(super) fn parse_flex_basis(input: &mut Parser) -> Vec<Declaration> {
    // Try keyword first.
    if let Ok(decls) = input.try_parse(|i| -> Result<Vec<Declaration>, ()> {
        let ident = i.expect_ident().map_err(|_| ())?;
        let lower = ident.to_ascii_lowercase();
        match lower.as_str() {
            // `content` treated as `auto` in Phase 2.
            "auto" | "content" => Ok(single_decl("flex-basis", CssValue::Auto)),
            _ => Err(()),
        }
    }) {
        return decls;
    }
    // Fall back to length/percentage.
    parse_value_property(input, "flex-basis", parse_length_or_percentage)
}

/// Parse the `flex` shorthand.
///
/// - `flex: none` → `0 0 auto`
/// - `flex: auto` → `1 1 auto`
/// - `flex: <number>` → `<n> 1 0`  (CSS spec: unitless 0 flex-basis)
/// - `flex: <grow> <shrink>` → `<grow> <shrink> 0`
/// - `flex: <grow> <shrink> <basis>` → full form
pub(super) fn parse_flex_shorthand(input: &mut Parser) -> Vec<Declaration> {
    // Try keyword first.
    if let Ok(decls) = input.try_parse(|i| -> Result<Vec<Declaration>, ()> {
        let ident = i.expect_ident().map_err(|_| ())?;
        let lower = ident.to_ascii_lowercase();
        match lower.as_str() {
            "none" => Ok(vec![
                Declaration {
                    property: "flex-grow".into(),
                    value: CssValue::Number(0.0),
                    important: false,
                },
                Declaration {
                    property: "flex-shrink".into(),
                    value: CssValue::Number(0.0),
                    important: false,
                },
                Declaration {
                    property: "flex-basis".into(),
                    value: CssValue::Auto,
                    important: false,
                },
            ]),
            "auto" => Ok(vec![
                Declaration {
                    property: "flex-grow".into(),
                    value: CssValue::Number(1.0),
                    important: false,
                },
                Declaration {
                    property: "flex-shrink".into(),
                    value: CssValue::Number(1.0),
                    important: false,
                },
                Declaration {
                    property: "flex-basis".into(),
                    value: CssValue::Auto,
                    important: false,
                },
            ]),
            _ => Err(()),
        }
    }) {
        return decls;
    }

    // Try numeric form: <grow> [<shrink> [<basis>]]
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let tok = i.next().map_err(|_| ())?;
            let grow = match *tok {
                Token::Number { value, .. } if value >= 0.0 => value,
                _ => return Err(()),
            };

            // Try optional shrink.
            let shrink = i.try_parse(|i2| -> Result<f32, ()> {
                let tok2 = i2.next().map_err(|_| ())?;
                match *tok2 {
                    Token::Number { value, .. } if value >= 0.0 => Ok(value),
                    _ => Err(()),
                }
            });

            let (shrink_val, basis_val) = match shrink {
                Ok(s) => {
                    // Try optional basis.
                    let basis = i.try_parse(|i3| -> Result<CssValue, ()> {
                        // Try auto/content keyword.
                        if let Ok(val) = i3.try_parse(|i4| -> Result<CssValue, ()> {
                            let ident = i4.expect_ident().map_err(|_| ())?;
                            match ident.to_ascii_lowercase().as_str() {
                                "auto" | "content" => Ok(CssValue::Auto),
                                _ => Err(()),
                            }
                        }) {
                            return Ok(val);
                        }
                        parse_length_or_percentage(i3)
                    });
                    // CSS spec: when flex-basis is omitted but grow/shrink are present, basis = 0
                    (s, basis.unwrap_or(CssValue::Length(0.0, LengthUnit::Px)))
                }
                Err(()) => {
                    // Single number: <n> 1 0
                    (1.0, CssValue::Length(0.0, LengthUnit::Px))
                }
            };

            Ok(vec![
                Declaration {
                    property: "flex-grow".into(),
                    value: CssValue::Number(grow),
                    important: false,
                },
                Declaration {
                    property: "flex-shrink".into(),
                    value: CssValue::Number(shrink_val),
                    important: false,
                },
                Declaration {
                    property: "flex-basis".into(),
                    value: basis_val,
                    important: false,
                },
            ])
        })
        .unwrap_or_default()
}

/// Parse the `flex-flow` shorthand: `<direction> || <wrap>`.
pub(super) fn parse_flex_flow_shorthand(input: &mut Parser) -> Vec<Declaration> {
    let direction_keywords = ["row", "row-reverse", "column", "column-reverse"];
    let wrap_keywords = ["nowrap", "wrap", "wrap-reverse"];

    let mut direction: Option<CssValue> = None;
    let mut wrap: Option<CssValue> = None;

    for _ in 0..2 {
        if input.is_exhausted() {
            break;
        }
        if let Ok(kw) = input.try_parse(|i| -> Result<(String, bool), ()> {
            let ident = i.expect_ident().map_err(|_| ())?;
            let lower = ident.to_ascii_lowercase();
            if direction.is_none() && direction_keywords.iter().any(|&k| k == lower) {
                Ok((lower, true))
            } else if wrap.is_none() && wrap_keywords.iter().any(|&k| k == lower) {
                Ok((lower, false))
            } else {
                Err(())
            }
        }) {
            if kw.1 {
                direction = Some(CssValue::Keyword(kw.0));
            } else {
                wrap = Some(CssValue::Keyword(kw.0));
            }
        } else {
            break;
        }
    }

    if direction.is_none() && wrap.is_none() {
        return Vec::new();
    }

    vec![
        Declaration {
            property: "flex-direction".into(),
            value: direction.unwrap_or(CssValue::Keyword("row".into())),
            important: false,
        },
        Declaration {
            property: "flex-wrap".into(),
            value: wrap.unwrap_or(CssValue::Keyword("nowrap".into())),
            important: false,
        },
    ]
}
