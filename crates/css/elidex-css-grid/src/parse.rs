//! Parsing helpers for CSS grid property values.

use elidex_plugin::{
    css_resolve::parse_length_unit, validate_area_rectangles, CssValue, LengthUnit, ParseError,
};

/// Maximum number of track entries in a grid-template-columns/rows list.
const MAX_TRACKS: usize = 10_000;

/// Parse a grid-template-columns / grid-template-rows value.
///
/// `none` produces `Keyword("none")`, otherwise a space-separated list of track sizes.
pub(crate) fn parse_track_list(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    if input.try_parse(|i| i.expect_ident_matching("none")).is_ok() {
        return Ok(CssValue::Keyword("none".to_string()));
    }

    // CSS Grid Level 2 §2: `subgrid [<line-name-list>]?`
    if input
        .try_parse(|i| i.expect_ident_matching("subgrid"))
        .is_ok()
    {
        let mut line_names_list = vec![CssValue::Keyword("subgrid".to_string())];
        while let Ok(names) = input.try_parse(parse_line_name_bracket) {
            line_names_list.push(CssValue::List(
                names.into_iter().map(CssValue::Keyword).collect(),
            ));
        }
        return Ok(CssValue::List(line_names_list));
    }

    let mut items = Vec::new();
    while let Ok(v) = parse_single_track_size_inner(input) {
        items.push(v);
        if items.len() >= MAX_TRACKS {
            break;
        }
    }
    if items.is_empty() {
        return Err(ParseError {
            property: String::new(),
            input: String::new(),
            message: "expected track size list".into(),
        });
    }
    // Single track: return value directly (avoid double-wrapping minmax).
    if items.len() == 1 {
        // len checked above; pop cannot fail.
        return Ok(items.pop().expect("len == 1"));
    }
    Ok(CssValue::List(items))
}

/// Parse a keyword (`auto`/`min-content`/`max-content`) or a dimension/percentage/zero token
/// into a `CssValue`. Shared by track-size and track-breadth parsing.
fn parse_track_value_token(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ()> {
    // Try keywords: auto, min-content, max-content
    if let Ok(ident) = input.try_parse(|i| i.expect_ident().map(ToString::to_string)) {
        return match ident.to_ascii_lowercase().as_str() {
            "auto" => Ok(CssValue::Auto),
            "min-content" => Ok(CssValue::Keyword("min-content".to_string())),
            "max-content" => Ok(CssValue::Keyword("max-content".to_string())),
            _ => Err(()),
        };
    }

    // Dimension (length or fr) / percentage / zero
    let token = input.next().map_err(|_| ())?;
    match *token {
        cssparser::Token::Dimension {
            value, ref unit, ..
        } => {
            let u = if unit.eq_ignore_ascii_case("fr") {
                LengthUnit::Fr
            } else {
                parse_length_unit(unit)
            };
            Ok(CssValue::Length(value, u))
        }
        cssparser::Token::Percentage { unit_value, .. } => {
            Ok(CssValue::Percentage(unit_value * 100.0))
        }
        cssparser::Token::Number { value: 0.0, .. } => Ok(CssValue::Length(0.0, LengthUnit::Px)),
        _ => Err(()),
    }
}

/// Inner helper that returns `Result<CssValue, ()>` for use with `try_parse`.
fn parse_single_track_size_inner(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ()> {
    // Try keyword or dimension
    if let Ok(v) = input.try_parse(parse_track_value_token) {
        return Ok(v);
    }

    // Try fit-content(<length-percentage>)
    if let Ok(v) = input.try_parse(|i| {
        i.expect_function_matching("fit-content").map_err(|_| ())?;
        i.parse_nested_block(|args| {
            let limit = parse_length_or_percentage_inner(args)
                .map_err(|()| args.new_custom_error::<_, ()>(()))?;
            Ok(CssValue::List(vec![
                CssValue::Keyword("fit-content".to_string()),
                limit,
            ]))
        })
        .map_err(|_: cssparser::ParseError<'_, ()>| ())
    }) {
        return Ok(v);
    }

    // Try minmax(min, max)
    input.try_parse(|i| {
        i.expect_function_matching("minmax").map_err(|_| ())?;
        i.parse_nested_block(|args| {
            let min =
                parse_track_value_token(args).map_err(|()| args.new_custom_error::<_, ()>(()))?;
            args.expect_comma()
                .map_err(|_| args.new_custom_error::<_, ()>(()))?;
            let max =
                parse_track_value_token(args).map_err(|()| args.new_custom_error::<_, ()>(()))?;
            Ok(CssValue::List(vec![
                CssValue::Keyword("minmax".to_string()),
                min,
                max,
            ]))
        })
        .map_err(|_: cssparser::ParseError<'_, ()>| ())
    })
}

/// Parse a `<length-percentage>` value inside a function argument (returns `CssValue`).
fn parse_length_or_percentage_inner(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ()> {
    let token = input.next().map_err(|_| ())?;
    match *token {
        cssparser::Token::Dimension {
            value, ref unit, ..
        } => Ok(CssValue::Length(value, parse_length_unit(unit))),
        cssparser::Token::Percentage { unit_value, .. } => {
            Ok(CssValue::Percentage(unit_value * 100.0))
        }
        cssparser::Token::Number { value: 0.0, .. } => Ok(CssValue::Length(0.0, LengthUnit::Px)),
        _ => Err(()),
    }
}

/// Parse a space-separated list of track sizes for grid-auto-columns/rows.
///
/// CSS Grid Level 2 §7.2.4: `<track-size>+` (one or more track sizes).
pub(crate) fn parse_auto_track_list(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    let mut items = Vec::new();
    while let Ok(v) = parse_single_track_size_inner(input) {
        items.push(v);
        if items.len() >= MAX_TRACKS {
            break;
        }
    }
    if items.is_empty() {
        return Err(ParseError {
            property: String::new(),
            input: String::new(),
            message: "expected track size".into(),
        });
    }
    if items.len() == 1 {
        return Ok(items.pop().expect("len == 1"));
    }
    Ok(CssValue::List(items))
}

/// Parse `grid-auto-flow`: `row`, `column`, `row dense`, `column dense`.
pub(crate) fn parse_auto_flow(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    let ident = input
        .expect_ident()
        .map(|s| s.to_ascii_lowercase())
        .map_err(|_| ParseError {
            property: "grid-auto-flow".into(),
            input: String::new(),
            message: "expected 'row' or 'column'".into(),
        })?;

    match ident.as_str() {
        "row" | "column" => {
            // Check for optional "dense"
            if input
                .try_parse(|i| i.expect_ident_matching("dense"))
                .is_ok()
            {
                Ok(CssValue::Keyword(format!("{ident} dense")))
            } else {
                Ok(CssValue::Keyword(ident))
            }
        }
        _ => Err(ParseError {
            property: "grid-auto-flow".into(),
            input: ident,
            message: "expected 'row' or 'column'".into(),
        }),
    }
}

/// Forbidden identifiers for grid line names and area names.
fn is_forbidden_grid_ident(s: &str) -> bool {
    matches!(
        s.to_ascii_lowercase().as_str(),
        "auto" | "span" | "inherit" | "initial" | "unset" | "default"
    )
}

/// Parse `grid-template-areas`: `none` or a sequence of string tokens.
pub(crate) fn parse_template_areas(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    // Try "none"
    if input.try_parse(|i| i.expect_ident_matching("none")).is_ok() {
        return Ok(CssValue::Keyword("none".to_string()));
    }

    let mut rows: Vec<Vec<String>> = Vec::new();
    while let Ok(s) = input.try_parse(|i| i.expect_string().map(std::string::ToString::to_string)) {
        let cells: Vec<String> = s
            .split_whitespace()
            .map(|t| {
                if t.chars().all(|c| c == '.') {
                    ".".to_string()
                } else {
                    t.to_string()
                }
            })
            .collect();
        if cells.is_empty() {
            return Err(ParseError {
                property: "grid-template-areas".into(),
                input: s,
                message: "empty row string".into(),
            });
        }
        // Validate area names
        for cell in &cells {
            if cell != "." && is_forbidden_grid_ident(cell) {
                return Err(ParseError {
                    property: "grid-template-areas".into(),
                    input: cell.clone(),
                    message: "forbidden area name".into(),
                });
            }
        }
        rows.push(cells);
    }

    if rows.is_empty() {
        return Err(ParseError {
            property: "grid-template-areas".into(),
            input: String::new(),
            message: "expected at least one area string".into(),
        });
    }

    // Validate: all rows same length
    let cols = rows[0].len();
    if rows.iter().any(|r| r.len() != cols) {
        return Err(ParseError {
            property: "grid-template-areas".into(),
            input: String::new(),
            message: "all rows must have the same number of columns".into(),
        });
    }

    // Validate: named areas form rectangles
    if !validate_area_rectangles(&rows) {
        return Err(ParseError {
            property: "grid-template-areas".into(),
            input: String::new(),
            message: "named areas must form rectangles".into(),
        });
    }

    // Encode as List of List of Keyword
    Ok(CssValue::List(
        rows.iter()
            .map(|row| CssValue::List(row.iter().map(|s| CssValue::Keyword(s.clone())).collect()))
            .collect(),
    ))
}

/// Parse a grid line value: `auto`, `<integer>`, `span <integer>`, `<custom-ident>`, etc.
///
/// CSS Grid §8.1 `<grid-line>` grammar:
/// ```text
/// auto | <custom-ident> |
/// [ <integer [-∞,-1]> | <integer [1,∞]> ] && <custom-ident>? |
/// [ span && [ <integer [1,∞]> || <custom-ident> ] ]
/// ```
#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
pub(crate) fn parse_grid_line(
    input: &mut cssparser::Parser<'_, '_>,
) -> Result<CssValue, ParseError> {
    // Try "auto"
    if input.try_parse(|i| i.expect_ident_matching("auto")).is_ok() {
        return Ok(CssValue::Auto);
    }

    // Try span variants: "span <integer> <ident>", "span <ident> <integer>", "span <ident>", "span <integer>"
    if let Ok(val) = input.try_parse(|i| -> Result<CssValue, ()> {
        i.expect_ident_matching("span").map_err(|_| ())?;

        // Try integer first
        let maybe_int = i.try_parse(|i2| -> Result<i32, ()> {
            let n = i2.expect_integer().map_err(|_| ())?;
            if n < 1 {
                return Err(());
            }
            Ok(n)
        });

        // Try ident
        let maybe_ident = i.try_parse(|i2| -> Result<String, ()> {
            let ident = i2.expect_ident().map_err(|_| ())?;
            let s = ident.to_string();
            if is_forbidden_grid_ident(&s) {
                return Err(());
            }
            Ok(s)
        });

        // If we got ident but no integer, try integer again (reversed order)
        let (n, ident) = match (maybe_int, maybe_ident) {
            (Ok(n), Ok(ident)) => (n, Some(ident)),
            (Ok(n), Err(())) => (n, None),
            (Err(()), Ok(ident)) => {
                // Try trailing integer
                let trailing_int = i.try_parse(|i2| -> Result<i32, ()> {
                    let n = i2.expect_integer().map_err(|_| ())?;
                    if n < 1 {
                        return Err(());
                    }
                    Ok(n)
                });
                (trailing_int.unwrap_or(1), Some(ident))
            }
            (Err(()), Err(())) => return Err(()),
        };

        if let Some(ident) = ident {
            Ok(CssValue::List(vec![
                CssValue::Keyword("span-named".to_string()),
                CssValue::Number(n as f32),
                CssValue::Keyword(ident),
            ]))
        } else {
            Ok(CssValue::List(vec![
                CssValue::Keyword("span".to_string()),
                CssValue::Number(n as f32),
            ]))
        }
    }) {
        return Ok(val);
    }

    // Try "<integer> <custom-ident>" or just "<integer>"
    if let Ok(val) = input.try_parse(|i| -> Result<CssValue, ()> {
        let n = i.expect_integer().map_err(|_| ())?;
        if n == 0 {
            return Err(());
        }
        // Try optional trailing ident
        let maybe_ident = i.try_parse(|i2| -> Result<String, ()> {
            let ident = i2.expect_ident().map_err(|_| ())?;
            let s = ident.to_string();
            if is_forbidden_grid_ident(&s) {
                return Err(());
            }
            Ok(s)
        });
        if let Ok(ident) = maybe_ident {
            Ok(CssValue::List(vec![
                CssValue::Number(n as f32),
                CssValue::Keyword(ident),
            ]))
        } else {
            Ok(CssValue::Number(n as f32))
        }
    }) {
        return Ok(val);
    }

    // Try "<custom-ident> <integer>" or just "<custom-ident>"
    if let Ok(val) = input.try_parse(|i| -> Result<CssValue, ()> {
        let ident = i.expect_ident().map_err(|_| ())?;
        let s = ident.to_string();
        if is_forbidden_grid_ident(&s) {
            return Err(());
        }
        // Try optional trailing integer
        let maybe_int = i.try_parse(|i2| -> Result<i32, ()> {
            let n = i2.expect_integer().map_err(|_| ())?;
            if n == 0 {
                return Err(());
            }
            Ok(n)
        });
        if let Ok(n) = maybe_int {
            Ok(CssValue::List(vec![
                CssValue::Keyword(s),
                CssValue::Number(n as f32),
            ]))
        } else {
            Ok(CssValue::Keyword(s))
        }
    }) {
        return Ok(val);
    }

    Err(ParseError {
        property: String::new(),
        input: String::new(),
        message: "expected grid line value".into(),
    })
}

/// Parse a bracketed line name list: `[name1 name2]`.
///
/// CSS Grid §7.2: `<line-names> = '[' <custom-ident>* ']'`
fn parse_line_name_bracket(input: &mut cssparser::Parser<'_, '_>) -> Result<Vec<String>, ()> {
    input.expect_square_bracket_block().map_err(|_| ())?;
    input
        .parse_nested_block(|args| {
            let mut names = Vec::new();
            while let Ok(ident) = args.try_parse(|i| i.expect_ident().map(ToString::to_string)) {
                if !is_forbidden_grid_ident(&ident) {
                    names.push(ident);
                }
            }
            Ok(names)
        })
        .map_err(|_: cssparser::ParseError<'_, ()>| ())
}
