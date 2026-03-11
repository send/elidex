//! CSS Grid property parsers.
//!
//! Handles `grid-template-columns/rows`, `grid-auto-flow`, `grid-auto-columns/rows`,
//! `grid-column/row-start/end`, `grid-column/row` shorthands, and `grid-area`.

use cssparser::{Parser, Token};
use elidex_plugin::{CssValue, LengthUnit};

use crate::values::{parse_length_or_percentage, parse_non_negative_length_or_percentage};

use super::{single_decl, Declaration};

// ---------------------------------------------------------------------------
// Track list parsing (grid-template-columns / grid-template-rows)
// ---------------------------------------------------------------------------

/// Parse a single `<track-size>` value.
///
/// Returns `CssValue` encoding:
/// - `Length(v, Px)` for px values
/// - `Percentage(v)` for percentages
/// - `Length(v, Fr)` for fr values
/// - `Auto` for `auto`
/// - `Keyword("min-content")` / `Keyword("max-content")`
/// - `List([Keyword("minmax"), min, max])` for `minmax()`
fn parse_track_size(input: &mut Parser) -> Result<CssValue, ()> {
    // Try minmax() function.
    if let Ok(val) = input.try_parse(parse_minmax) {
        return Ok(val);
    }

    // Try keywords: auto, min-content, max-content.
    if let Ok(val) = input.try_parse(|i| -> Result<CssValue, ()> {
        let ident = i.expect_ident().map_err(|_| ())?;
        match ident.to_ascii_lowercase().as_str() {
            "auto" => Ok(CssValue::Auto),
            "min-content" => Ok(CssValue::Keyword("min-content".into())),
            "max-content" => Ok(CssValue::Keyword("max-content".into())),
            _ => Err(()),
        }
    }) {
        return Ok(val);
    }

    // Try fr unit.
    if let Ok(val) = input.try_parse(parse_fr) {
        return Ok(val);
    }

    // Fall back to length/percentage.
    parse_non_negative_length_or_percentage(input)
}

/// Parse an `<fr>` dimension value.
fn parse_fr(input: &mut Parser) -> Result<CssValue, ()> {
    let tok = input.next().map_err(|_| ())?;
    match tok {
        Token::Dimension {
            value, ref unit, ..
        } if unit.eq_ignore_ascii_case("fr") && *value >= 0.0 => {
            Ok(CssValue::Length(*value, LengthUnit::Fr))
        }
        _ => Err(()),
    }
}

/// Parse `minmax(min, max)`.
fn parse_minmax(input: &mut Parser) -> Result<CssValue, ()> {
    input.expect_function_matching("minmax").map_err(|_| ())?;
    input
        .parse_nested_block(|args| -> Result<CssValue, cssparser::ParseError<'_, ()>> {
            let min = parse_track_breadth(args).map_err(|()| args.new_custom_error(()))?;
            args.expect_comma().map_err(cssparser::ParseError::from)?;
            let max = parse_track_breadth(args).map_err(|()| args.new_custom_error(()))?;
            Ok(CssValue::List(vec![
                CssValue::Keyword("minmax".into()),
                min,
                max,
            ]))
        })
        .map_err(|_: cssparser::ParseError<'_, ()>| ())
}

/// Parse a `<track-breadth>` value (used inside minmax).
fn parse_track_breadth(input: &mut Parser) -> Result<CssValue, ()> {
    // Try keywords first.
    if let Ok(val) = input.try_parse(|i| -> Result<CssValue, ()> {
        let ident = i.expect_ident().map_err(|_| ())?;
        match ident.to_ascii_lowercase().as_str() {
            "auto" => Ok(CssValue::Auto),
            "min-content" => Ok(CssValue::Keyword("min-content".into())),
            "max-content" => Ok(CssValue::Keyword("max-content".into())),
            _ => Err(()),
        }
    }) {
        return Ok(val);
    }

    // Try fr.
    if let Ok(val) = input.try_parse(parse_fr) {
        return Ok(val);
    }

    // Fall back to length/percentage.
    parse_length_or_percentage(input)
}

/// Parse `grid-template-columns` or `grid-template-rows`.
///
/// Accepts: `none` | `<track-size>+` | `repeat(N, <track-size>+)`.
pub(super) fn parse_grid_template(input: &mut Parser, name: &str) -> Vec<Declaration> {
    // Try `none` keyword.
    if let Ok(()) = input.try_parse(|i| -> Result<(), ()> {
        let ident = i.expect_ident().map_err(|_| ())?;
        if ident.eq_ignore_ascii_case("none") {
            Ok(())
        } else {
            Err(())
        }
    }) {
        return single_decl(name, CssValue::Keyword("none".into()));
    }

    // Parse track list (one or more track-size values, possibly with repeat()).
    let mut tracks = Vec::new();
    while !input.is_exhausted() {
        // Try repeat() function.
        if let Ok(expanded) = input.try_parse(parse_repeat) {
            tracks.extend(expanded);
            continue;
        }

        // Try a single track-size.
        if let Ok(ts) = input.try_parse(parse_track_size) {
            tracks.push(ts);
            continue;
        }

        break;
    }

    if tracks.is_empty() {
        return Vec::new();
    }

    single_decl(name, CssValue::List(tracks))
}

/// Maximum repeat count to prevent OOM from malicious CSS (e.g. `repeat(999999999, 1fr)`).
const MAX_REPEAT_COUNT: u32 = 10_000;

/// Parse `repeat(N, <track-size>+)`.
#[allow(clippy::cast_sign_loss)] // CSS repeat count is always >= 1
///
/// `auto-fill`/`auto-fit` are treated as `repeat(1, ...)` (Phase 3.5 simplification).
fn parse_repeat(input: &mut Parser) -> Result<Vec<CssValue>, ()> {
    input.expect_function_matching("repeat").map_err(|_| ())?;
    input
        .parse_nested_block(
            |args| -> Result<Vec<CssValue>, cssparser::ParseError<'_, ()>> {
                // Parse repeat count: integer or auto-fill/auto-fit → 1.
                let count = if let Ok(n) = args.try_parse(|i| -> Result<u32, ()> {
                    let tok = i.next().map_err(|_| ())?;
                    match *tok {
                        Token::Number {
                            int_value: Some(n), ..
                        } if n >= 1 => Ok(n as u32),
                        _ => Err(()),
                    }
                }) {
                    n
                } else {
                    // auto-fill / auto-fit → 1
                    let ident = args.expect_ident().map_err(cssparser::ParseError::from)?;
                    let lower = ident.to_ascii_lowercase();
                    if lower == "auto-fill" || lower == "auto-fit" {
                        1
                    } else {
                        return Err(args.new_custom_error(()));
                    }
                };

                let count = count.min(MAX_REPEAT_COUNT);

                args.expect_comma().map_err(cssparser::ParseError::from)?;

                // Parse track list inside repeat.
                let mut pattern = Vec::new();
                while !args.is_exhausted() {
                    let ts = parse_track_size(args).map_err(|()| args.new_custom_error(()))?;
                    pattern.push(ts);
                }
                if pattern.is_empty() {
                    return Err(args.new_custom_error(()));
                }

                let mut result = Vec::new();
                for _ in 0..count {
                    result.extend(pattern.clone());
                }
                Ok(result)
            },
        )
        .map_err(|_: cssparser::ParseError<'_, ()>| ())
}

// ---------------------------------------------------------------------------
// Grid auto track size
// ---------------------------------------------------------------------------

/// Parse `grid-auto-columns` or `grid-auto-rows`: a single `<track-size>`.
pub(super) fn parse_grid_auto_track(input: &mut Parser, name: &str) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let val = parse_track_size(i)?;
            Ok(single_decl(name, val))
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Grid auto flow
// ---------------------------------------------------------------------------

/// Parse `grid-auto-flow`: `row` | `column` | `row dense` | `column dense`.
pub(super) fn parse_grid_auto_flow(input: &mut Parser) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let ident = i.expect_ident().map_err(|_| ())?;
            let lower = ident.to_ascii_lowercase();
            let direction = match lower.as_str() {
                "row" | "column" => lower.clone(),
                "dense" => {
                    // `dense` alone = `row dense`
                    return Ok(single_decl(
                        "grid-auto-flow",
                        CssValue::Keyword("row dense".into()),
                    ));
                }
                _ => return Err(()),
            };

            // Try optional `dense` keyword.
            let dense = i
                .try_parse(|i2| -> Result<bool, ()> {
                    let ident2 = i2.expect_ident().map_err(|_| ())?;
                    if ident2.eq_ignore_ascii_case("dense") {
                        Ok(true)
                    } else {
                        Err(())
                    }
                })
                .unwrap_or(false);

            let kw = if dense {
                format!("{direction} dense")
            } else {
                direction
            };
            Ok(single_decl("grid-auto-flow", CssValue::Keyword(kw)))
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Grid line placement
// ---------------------------------------------------------------------------

/// Parse a single `<grid-line>` value: `auto` | `<integer>` | `span <integer>`.
#[allow(clippy::cast_precision_loss)] // CSS grid line numbers are small integers
fn parse_grid_line_value(input: &mut Parser) -> Result<CssValue, ()> {
    // Try `auto`.
    if let Ok(()) = input.try_parse(|i| -> Result<(), ()> {
        let ident = i.expect_ident().map_err(|_| ())?;
        if ident.eq_ignore_ascii_case("auto") {
            Ok(())
        } else {
            Err(())
        }
    }) {
        return Ok(CssValue::Auto);
    }

    // Try `span <integer>`.
    if let Ok(val) = input.try_parse(|i| -> Result<CssValue, ()> {
        let ident = i.expect_ident().map_err(|_| ())?;
        if !ident.eq_ignore_ascii_case("span") {
            return Err(());
        }
        let tok = i.next().map_err(|_| ())?;
        match *tok {
            Token::Number {
                int_value: Some(n), ..
            } if n >= 1 => Ok(CssValue::List(vec![
                CssValue::Keyword("span".into()),
                CssValue::Number(n as f32),
            ])),
            _ => Err(()),
        }
    }) {
        return Ok(val);
    }

    // Try plain integer.
    let tok = input.next().map_err(|_| ())?;
    match *tok {
        Token::Number {
            int_value: Some(n), ..
        } if n != 0 => Ok(CssValue::Number(n as f32)),
        _ => Err(()),
    }
}

/// Parse `grid-column-start`, `grid-column-end`, `grid-row-start`, `grid-row-end`.
pub(super) fn parse_grid_line(input: &mut Parser, name: &str) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let val = parse_grid_line_value(i)?;
            Ok(single_decl(name, val))
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Grid line shorthands
// ---------------------------------------------------------------------------

/// Parse `grid-column` or `grid-row` shorthand: `<start> / <end>` or `<start>`.
pub(super) fn parse_grid_line_shorthand(input: &mut Parser, name: &str) -> Vec<Declaration> {
    let (start_prop, end_prop) = match name {
        "grid-column" => ("grid-column-start", "grid-column-end"),
        "grid-row" => ("grid-row-start", "grid-row-end"),
        _ => return Vec::new(),
    };

    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let start = parse_grid_line_value(i)?;

            // Try optional slash + end.
            let end = i
                .try_parse(|i2| -> Result<CssValue, ()> {
                    i2.expect_delim('/').map_err(|_| ())?;
                    parse_grid_line_value(i2)
                })
                .unwrap_or(CssValue::Auto);

            Ok(vec![
                Declaration::new(start_prop, start),
                Declaration::new(end_prop, end),
            ])
        })
        .unwrap_or_default()
}

/// Parse `grid-area` shorthand: `<row-start> / <col-start> / <row-end> / <col-end>`.
///
/// Missing values default to `auto`.
pub(super) fn parse_grid_area(input: &mut Parser) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let row_start = parse_grid_line_value(i)?;

            let col_start = i
                .try_parse(|i2| -> Result<CssValue, ()> {
                    i2.expect_delim('/').map_err(|_| ())?;
                    parse_grid_line_value(i2)
                })
                .unwrap_or(CssValue::Auto);

            let row_end = i
                .try_parse(|i2| -> Result<CssValue, ()> {
                    i2.expect_delim('/').map_err(|_| ())?;
                    parse_grid_line_value(i2)
                })
                .unwrap_or(CssValue::Auto);

            let col_end = i
                .try_parse(|i2| -> Result<CssValue, ()> {
                    i2.expect_delim('/').map_err(|_| ())?;
                    parse_grid_line_value(i2)
                })
                .unwrap_or(CssValue::Auto);

            Ok(vec![
                Declaration::new("grid-row-start", row_start),
                Declaration::new("grid-column-start", col_start),
                Declaration::new("grid-row-end", row_end),
                Declaration::new("grid-column-end", col_end),
            ])
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cssparser::ParserInput;

    fn parse_template(css: &str) -> Vec<Declaration> {
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        parse_grid_template(&mut parser, "grid-template-columns")
    }

    #[test]
    fn repeat_count_capped_at_max() {
        // A huge repeat count should be capped at MAX_REPEAT_COUNT.
        let decls = parse_template("repeat(99999, 1fr)");
        assert_eq!(decls.len(), 1);
        if let CssValue::List(tracks) = &decls[0].value {
            assert_eq!(tracks.len(), MAX_REPEAT_COUNT as usize);
        } else {
            panic!("expected List");
        }
    }

    #[test]
    fn repeat_small_count_works() {
        let decls = parse_template("repeat(3, 100px)");
        assert_eq!(decls.len(), 1);
        if let CssValue::List(tracks) = &decls[0].value {
            assert_eq!(tracks.len(), 3);
        } else {
            panic!("expected List");
        }
    }
}
