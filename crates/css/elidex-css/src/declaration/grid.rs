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

/// Result from parsing a `repeat()` function.
enum RepeatResult {
    /// Integer repeat: already expanded tracks.
    Expanded(Vec<CssValue>),
    /// Auto-fill or auto-fit: the mode keyword and the pattern tracks.
    AutoRepeat(String, Vec<CssValue>),
}

/// Parse `grid-template-columns` or `grid-template-rows`.
///
/// Accepts: `none` | `<track-size>+` | `repeat(N, <track-size>+)`.
/// For `repeat(auto-fill/auto-fit, ...)`, emits a special `CssValue::List`
/// with marker `Keyword("auto-repeat")`.
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
    // CSS spec allows at most one auto-repeat per track list.
    let mut before = Vec::new();
    let mut auto_repeat: Option<(String, Vec<CssValue>)> = None;
    let mut after = Vec::new();

    while !input.is_exhausted() {
        // Try repeat() function.
        if let Ok(result) = input.try_parse(parse_repeat) {
            match result {
                RepeatResult::Expanded(expanded) => {
                    if auto_repeat.is_some() {
                        after.extend(expanded);
                    } else {
                        before.extend(expanded);
                    }
                }
                RepeatResult::AutoRepeat(mode, pattern) => {
                    if auto_repeat.is_some() {
                        // CSS spec: at most one auto-repeat; ignore second.
                        break;
                    }
                    auto_repeat = Some((mode, pattern));
                }
            }
            continue;
        }

        // Try a single track-size.
        if let Ok(ts) = input.try_parse(parse_track_size) {
            if auto_repeat.is_some() {
                after.push(ts);
            } else {
                before.push(ts);
            }
            continue;
        }

        break;
    }

    if let Some((mode, pattern)) = auto_repeat {
        // Emit: List([Keyword("auto-repeat"), Keyword(mode), List(before), List(pattern), List(after)])
        let value = CssValue::List(vec![
            CssValue::Keyword("auto-repeat".into()),
            CssValue::Keyword(mode),
            CssValue::List(before),
            CssValue::List(pattern),
            CssValue::List(after),
        ]);
        return single_decl(name, value);
    }

    if before.is_empty() {
        return Vec::new();
    }

    single_decl(name, CssValue::List(before))
}

/// Maximum repeat count to prevent OOM from malicious CSS (e.g. `repeat(999999999, 1fr)`).
const MAX_REPEAT_COUNT: u32 = 10_000;

/// Parse `repeat(N, <track-size>+)` or `repeat(auto-fill/auto-fit, <track-size>+)`.
#[allow(clippy::cast_sign_loss)] // CSS repeat count is always >= 1
fn parse_repeat(input: &mut Parser) -> Result<RepeatResult, ()> {
    input.expect_function_matching("repeat").map_err(|_| ())?;
    input
        .parse_nested_block(
            |args| -> Result<RepeatResult, cssparser::ParseError<'_, ()>> {
                // Try integer count first.
                let count_or_mode = if let Ok(n) = args.try_parse(|i| -> Result<u32, ()> {
                    let tok = i.next().map_err(|_| ())?;
                    match *tok {
                        Token::Number {
                            int_value: Some(n), ..
                        } if n >= 1 => Ok(n as u32),
                        _ => Err(()),
                    }
                }) {
                    Ok(n)
                } else {
                    // auto-fill / auto-fit
                    let ident = args.expect_ident().map_err(cssparser::ParseError::from)?;
                    let lower = ident.to_ascii_lowercase();
                    if lower == "auto-fill" || lower == "auto-fit" {
                        Err(lower)
                    } else {
                        return Err(args.new_custom_error(()));
                    }
                };

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

                match count_or_mode {
                    Ok(count) => {
                        let count = count.min(MAX_REPEAT_COUNT);
                        let mut result = Vec::new();
                        for _ in 0..count {
                            result.extend(pattern.clone());
                        }
                        Ok(RepeatResult::Expanded(result))
                    }
                    Err(mode) => {
                        // CSS Grid §7.2.3.2: auto-repeat tracks must all be
                        // fixed-size (Length, Percentage, or minmax with fixed
                        // bounds). Reject patterns containing fr, auto,
                        // min-content, or max-content.
                        if !pattern.iter().all(is_fixed_track_value) {
                            return Err(args.new_custom_error(()));
                        }
                        Ok(RepeatResult::AutoRepeat(mode, pattern))
                    }
                }
            },
        )
        .map_err(|_: cssparser::ParseError<'_, ()>| ())
}

/// Check whether a parsed track-size `CssValue` is a `<fixed-size>` per
/// CSS Grid §7.2.3.2. Valid forms:
///   - `<fixed-breadth>` (length or percentage, not `fr`)
///   - `minmax(<fixed-breadth>, <track-breadth>)` — any max OK if min is fixed
///   - `minmax(<inflexible-breadth>, <fixed-breadth>)` — any non-fr min OK if max is fixed
fn is_fixed_track_value(v: &CssValue) -> bool {
    match v {
        CssValue::Length(_, unit) => *unit != LengthUnit::Fr,
        CssValue::Percentage(_) => true,
        CssValue::List(items)
            if items.first() == Some(&CssValue::Keyword("minmax".into())) && items.len() == 3 =>
        {
            let min_fixed = is_fixed_breadth_value(&items[1]);
            let max_fixed = is_fixed_breadth_value(&items[2]);
            if min_fixed {
                // minmax(<fixed-breadth>, <track-breadth>) — any max is OK
                true
            } else {
                // minmax(<inflexible-breadth>, <fixed-breadth>) — min must not be fr
                is_inflexible_breadth_value(&items[1]) && max_fixed
            }
        }
        _ => false,
    }
}

/// Check whether a track-breadth `CssValue` is a fixed (definite) size.
/// `fr` units are not fixed.
fn is_fixed_breadth_value(v: &CssValue) -> bool {
    match v {
        CssValue::Length(_, unit) => *unit != LengthUnit::Fr,
        CssValue::Percentage(_) => true,
        _ => false,
    }
}

/// Check whether a track-breadth is inflexible (anything except `fr`).
fn is_inflexible_breadth_value(v: &CssValue) -> bool {
    !matches!(v, CssValue::Length(_, unit) if *unit == LengthUnit::Fr)
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

    #[test]
    fn auto_fill_emits_auto_repeat_marker() {
        let decls = parse_template("repeat(auto-fill, 200px)");
        assert_eq!(decls.len(), 1);
        if let CssValue::List(items) = &decls[0].value {
            assert_eq!(items[0], CssValue::Keyword("auto-repeat".into()));
            assert_eq!(items[1], CssValue::Keyword("auto-fill".into()));
            // before: empty
            assert_eq!(items[2], CssValue::List(vec![]));
            // pattern: [Length(200, Px)]
            if let CssValue::List(pattern) = &items[3] {
                assert_eq!(pattern.len(), 1);
            } else {
                panic!("expected pattern List");
            }
            // after: empty
            assert_eq!(items[4], CssValue::List(vec![]));
        } else {
            panic!("expected List with auto-repeat marker");
        }
    }

    #[test]
    fn auto_fit_emits_auto_repeat_marker() {
        let decls = parse_template("repeat(auto-fit, 100px 200px)");
        assert_eq!(decls.len(), 1);
        if let CssValue::List(items) = &decls[0].value {
            assert_eq!(items[0], CssValue::Keyword("auto-repeat".into()));
            assert_eq!(items[1], CssValue::Keyword("auto-fit".into()));
            if let CssValue::List(pattern) = &items[3] {
                assert_eq!(pattern.len(), 2);
            } else {
                panic!("expected pattern List");
            }
        } else {
            panic!("expected List with auto-repeat marker");
        }
    }

    #[test]
    fn auto_repeat_rejects_non_fixed_tracks() {
        // repeat(auto-fill, 1fr) should be rejected — fr is not a fixed size.
        let decls = parse_template("repeat(auto-fill, 1fr)");
        assert!(
            decls.is_empty(),
            "repeat(auto-fill, 1fr) should be rejected, got {decls:?}"
        );

        // repeat(auto-fit, auto) should also be rejected.
        let decls = parse_template("repeat(auto-fit, auto)");
        assert!(
            decls.is_empty(),
            "repeat(auto-fit, auto) should be rejected, got {decls:?}"
        );

        // repeat(auto-fill, minmax(100px, 1fr)) is valid: min is <fixed-breadth>.
        let decls = parse_template("repeat(auto-fill, minmax(100px, 1fr))");
        assert_eq!(
            decls.len(),
            1,
            "repeat(auto-fill, minmax(100px, 1fr)) should be accepted (fixed min)"
        );

        // repeat(auto-fill, minmax(min-content, 200px)) is valid: max is <fixed-breadth>.
        let decls = parse_template("repeat(auto-fill, minmax(min-content, 200px))");
        assert_eq!(
            decls.len(),
            1,
            "repeat(auto-fill, minmax(min-content, 200px)) should be accepted"
        );

        // repeat(auto-fill, minmax(1fr, 200px)) is invalid: min is fr (flexible).
        let decls = parse_template("repeat(auto-fill, minmax(1fr, 200px))");
        assert!(
            decls.is_empty(),
            "repeat(auto-fill, minmax(1fr, 200px)) should be rejected (fr min), got {decls:?}"
        );

        // repeat(auto-fill, 100px) should still work.
        let decls = parse_template("repeat(auto-fill, 100px)");
        assert_eq!(
            decls.len(),
            1,
            "repeat(auto-fill, 100px) should be accepted"
        );

        // repeat(auto-fit, minmax(100px, 200px)) should work (both bounds fixed).
        let decls = parse_template("repeat(auto-fit, minmax(100px, 200px))");
        assert_eq!(
            decls.len(),
            1,
            "repeat(auto-fit, minmax(100px, 200px)) should be accepted"
        );
    }

    #[test]
    fn auto_fill_with_fixed_tracks() {
        // 100px repeat(auto-fill, 200px) 50px
        let decls = parse_template("100px repeat(auto-fill, 200px) 50px");
        assert_eq!(decls.len(), 1);
        if let CssValue::List(items) = &decls[0].value {
            assert_eq!(items[0], CssValue::Keyword("auto-repeat".into()));
            assert_eq!(items[1], CssValue::Keyword("auto-fill".into()));
            // before: [Length(100, Px)]
            if let CssValue::List(before) = &items[2] {
                assert_eq!(before.len(), 1);
            } else {
                panic!("expected before List");
            }
            // after: [Length(50, Px)]
            if let CssValue::List(after) = &items[4] {
                assert_eq!(after.len(), 1);
            } else {
                panic!("expected after List");
            }
        } else {
            panic!("expected List with auto-repeat marker");
        }
    }
}
