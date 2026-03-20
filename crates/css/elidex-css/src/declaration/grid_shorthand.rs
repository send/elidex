//! CSS Grid shorthand property parsers.
//!
//! Handles `grid-column/row` line shorthands, `grid-area`, `grid-template`,
//! and the `grid` shorthand.

use cssparser::Parser;
use elidex_plugin::CssValue;

use super::grid::{
    is_forbidden_grid_ident, parse_grid_line_value, parse_grid_template, parse_line_names,
    parse_track_size, TrackListParts,
};
use super::{single_decl, Declaration};

use elidex_plugin::validate_area_rectangles;

// ---------------------------------------------------------------------------
// grid-template-areas
// ---------------------------------------------------------------------------

/// Parse `grid-template-areas`: `none` | `<string>+`.
pub(super) fn parse_grid_template_areas(input: &mut Parser) -> Vec<Declaration> {
    // Try `none`
    if let Ok(()) = input.try_parse(|i| -> Result<(), ()> {
        let ident = i.expect_ident().map_err(|_| ())?;
        if ident.eq_ignore_ascii_case("none") {
            Ok(())
        } else {
            Err(())
        }
    }) {
        return single_decl("grid-template-areas", CssValue::Keyword("none".into()));
    }

    let mut rows: Vec<Vec<String>> = Vec::new();
    while let Ok(s) = input.try_parse(|i| -> Result<String, ()> {
        i.expect_string()
            .map(std::string::ToString::to_string)
            .map_err(|_| ())
    }) {
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
            return Vec::new();
        }
        // Validate names
        for cell in &cells {
            if cell != "." && is_forbidden_grid_ident(cell) {
                return Vec::new();
            }
        }
        rows.push(cells);
    }

    if rows.is_empty() {
        return Vec::new();
    }

    // Validate: all rows same length
    let cols = rows[0].len();
    if rows.iter().any(|r| r.len() != cols) {
        return Vec::new();
    }

    // Validate: areas form rectangles
    if !validate_area_rectangles(&rows) {
        return Vec::new();
    }

    // Encode as List of List of Keyword
    let value = CssValue::List(
        rows.iter()
            .map(|row| CssValue::List(row.iter().map(|s| CssValue::Keyword(s.clone())).collect()))
            .collect(),
    );
    single_decl("grid-template-areas", value)
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

/// Default value for a missing shorthand component.
///
/// CSS Grid §8.4: For `<custom-ident>`, the omitted value copies the
/// specified value. For everything else, it defaults to `auto`.
fn shorthand_default(specified: &CssValue) -> CssValue {
    match specified {
        CssValue::Keyword(_) => specified.clone(),
        _ => CssValue::Auto,
    }
}

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
                .unwrap_or_else(|()| shorthand_default(&start));

            Ok(vec![
                Declaration::new(start_prop, start),
                Declaration::new(end_prop, end),
            ])
        })
        .unwrap_or_default()
}

/// Parse `grid-area` shorthand: `<row-start> / <col-start> / <row-end> / <col-end>`.
///
/// CSS Grid §8.4: Missing `<custom-ident>` values copy from the preceding
/// specified ident. Non-ident missing values default to `auto`.
pub(super) fn parse_grid_area(input: &mut Parser) -> Vec<Declaration> {
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let row_start = parse_grid_line_value(i)?;

            let col_start = i
                .try_parse(|i2| -> Result<CssValue, ()> {
                    i2.expect_delim('/').map_err(|_| ())?;
                    parse_grid_line_value(i2)
                })
                .unwrap_or_else(|()| shorthand_default(&row_start));

            let row_end = i
                .try_parse(|i2| -> Result<CssValue, ()> {
                    i2.expect_delim('/').map_err(|_| ())?;
                    parse_grid_line_value(i2)
                })
                .unwrap_or_else(|()| shorthand_default(&row_start));

            let col_end = i
                .try_parse(|i2| -> Result<CssValue, ()> {
                    i2.expect_delim('/').map_err(|_| ())?;
                    parse_grid_line_value(i2)
                })
                .unwrap_or_else(|()| shorthand_default(&col_start));

            Ok(vec![
                Declaration::new("grid-row-start", row_start),
                Declaration::new("grid-column-start", col_start),
                Declaration::new("grid-row-end", row_end),
                Declaration::new("grid-column-end", col_end),
            ])
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// grid-template shorthand (CSS Grid §7.1)
// ---------------------------------------------------------------------------

/// Parse `grid-template` shorthand.
///
/// Patterns:
/// 1. `none` → all 3 longhands = none
/// 2. Area+row-track interleave with `/` columns:
///    `[line] "area area" track-size [line] ... / <column-track-list>`
/// 3. `<row-track-list> / <column-track-list>` (no areas)
pub(super) fn parse_grid_template_shorthand(input: &mut Parser) -> Vec<Declaration> {
    // Pattern 1: `none`
    if let Ok(()) = input.try_parse(|i| -> Result<(), ()> {
        let ident = i.expect_ident().map_err(|_| ())?;
        if ident.eq_ignore_ascii_case("none") {
            Ok(())
        } else {
            Err(())
        }
    }) {
        return vec![
            Declaration::new("grid-template-rows", CssValue::Keyword("none".into())),
            Declaration::new("grid-template-columns", CssValue::Keyword("none".into())),
            Declaration::new("grid-template-areas", CssValue::Keyword("none".into())),
        ];
    }

    // Pattern 2: Try area+row-track interleave.
    if let Ok(decls) = input.try_parse(parse_grid_template_with_areas) {
        return decls;
    }

    // Pattern 3: <row-track-list> / <column-track-list>
    input
        .try_parse(|i| -> Result<Vec<Declaration>, ()> {
            let row_decls = parse_grid_template(i, "grid-template-rows");
            if row_decls.is_empty() {
                return Err(());
            }
            i.expect_delim('/').map_err(|_| ())?;
            let col_decls = parse_grid_template(i, "grid-template-columns");
            if col_decls.is_empty() {
                return Err(());
            }
            let mut result = row_decls;
            result.extend(col_decls);
            result.push(Declaration::new(
                "grid-template-areas",
                CssValue::Keyword("none".into()),
            ));
            Ok(result)
        })
        .unwrap_or_default()
}

/// Parse grid-template with areas (pattern 2).
///
/// ```css
/// grid-template:
///   [header-start] "a a" 60px [header-end]
///   [main-start] "b c" 1fr [main-end]
///   / 200px 1fr;
/// ```
fn parse_grid_template_with_areas(input: &mut Parser) -> Result<Vec<Declaration>, ()> {
    let mut area_rows: Vec<Vec<String>> = Vec::new();
    let mut row_parts = TrackListParts::new();

    // Parse rows: each row is optional [line-names], then a string, then optional track-size, then optional [line-names]
    loop {
        // Try leading [line-names]
        if let Ok(names) = input.try_parse(parse_line_names) {
            row_parts.push_names(names);
        }

        // Try area string
        let area_string = input.try_parse(|i| -> Result<String, ()> {
            i.expect_string()
                .map(std::string::ToString::to_string)
                .map_err(|_| ())
        });
        let Ok(area_str) = area_string else {
            break;
        };

        let cells: Vec<String> = area_str
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
            return Err(());
        }
        for cell in &cells {
            if cell != "." && is_forbidden_grid_ident(cell) {
                return Err(());
            }
        }
        area_rows.push(cells);

        // Optional track size after the area string
        let track = input.try_parse(parse_track_size).unwrap_or(CssValue::Auto);
        row_parts.push_track(track);

        // Try trailing [line-names]
        if let Ok(names) = input.try_parse(parse_line_names) {
            row_parts.push_names(names);
        }
    }

    if area_rows.is_empty() {
        return Err(());
    }

    // Validate: all rows same column count
    let cols = area_rows[0].len();
    if area_rows.iter().any(|r| r.len() != cols) {
        return Err(());
    }
    if !validate_area_rectangles(&area_rows) {
        return Err(());
    }

    // Expect `/` then column track list
    input.expect_delim('/').map_err(|_| ())?;
    let col_decls = parse_grid_template(input, "grid-template-columns");
    if col_decls.is_empty() {
        return Err(());
    }

    // Build row track value
    let row_value = row_parts.to_css_value();

    // Build areas value
    let areas_value = CssValue::List(
        area_rows
            .iter()
            .map(|row| CssValue::List(row.iter().map(|s| CssValue::Keyword(s.clone())).collect()))
            .collect(),
    );

    let mut result = vec![Declaration::new("grid-template-rows", row_value)];
    result.extend(col_decls);
    result.push(Declaration::new("grid-template-areas", areas_value));
    Ok(result)
}

// ---------------------------------------------------------------------------
// grid shorthand (CSS Grid §7.4)
// ---------------------------------------------------------------------------

/// Initial values for grid properties not set by the shorthand.
fn grid_initial_auto_flow() -> CssValue {
    CssValue::Keyword("row".into())
}
fn grid_initial_auto_track() -> CssValue {
    CssValue::Auto
}

/// Parse `grid` shorthand.
///
/// Patterns:
/// 1. `auto-flow [dense] <auto-rows>? / <column-track-list>` — auto rows / explicit cols
/// 2. `<row-track-list> / auto-flow [dense] <auto-cols>?` — explicit rows / auto cols
/// 3. `<grid-template>` — same as grid-template shorthand (auto-* reset)
pub(super) fn parse_grid_shorthand(input: &mut Parser) -> Vec<Declaration> {
    // Pattern 1: auto-flow [dense] <auto-rows>? / <cols>
    if let Ok(decls) = input.try_parse(parse_grid_auto_rows_pattern) {
        return decls;
    }

    // Pattern 2: <rows> / auto-flow [dense] <auto-cols>?
    if let Ok(decls) = input.try_parse(parse_grid_auto_cols_pattern) {
        return decls;
    }

    // Pattern 3: <grid-template>
    let mut decls = parse_grid_template_shorthand(input);
    if !decls.is_empty() {
        // Reset auto-* properties to initial values
        decls.push(Declaration::new("grid-auto-flow", grid_initial_auto_flow()));
        decls.push(Declaration::new(
            "grid-auto-rows",
            grid_initial_auto_track(),
        ));
        decls.push(Declaration::new(
            "grid-auto-columns",
            grid_initial_auto_track(),
        ));
        return decls;
    }

    Vec::new()
}

/// Pattern 1: `auto-flow [dense] <auto-rows>? / <column-track-list>`
fn parse_grid_auto_rows_pattern(input: &mut Parser) -> Result<Vec<Declaration>, ()> {
    // Expect `auto-flow`
    let ident = input.expect_ident().map_err(|_| ())?;
    if !ident.eq_ignore_ascii_case("auto-flow") {
        return Err(());
    }

    // Optional `dense`
    let dense = input
        .try_parse(|i| -> Result<bool, ()> {
            let id = i.expect_ident().map_err(|_| ())?;
            if id.eq_ignore_ascii_case("dense") {
                Ok(true)
            } else {
                Err(())
            }
        })
        .unwrap_or(false);

    // Optional auto-row track size
    let auto_rows = input.try_parse(parse_track_size).unwrap_or(CssValue::Auto);

    // Expect `/`
    input.expect_delim('/').map_err(|_| ())?;

    // Parse column track list
    let col_decls = parse_grid_template(input, "grid-template-columns");
    if col_decls.is_empty() {
        return Err(());
    }

    // CSS Grid §7.4: `[auto-flow] <auto-rows>? / <columns>` sets
    // grid-auto-flow to `row` (items auto-place along the row axis).
    let flow = if dense { "row dense" } else { "row" };
    let mut result = vec![Declaration::new(
        "grid-template-rows",
        CssValue::Keyword("none".into()),
    )];
    result.extend(col_decls);
    result.push(Declaration::new(
        "grid-template-areas",
        CssValue::Keyword("none".into()),
    ));
    result.push(Declaration::new(
        "grid-auto-flow",
        CssValue::Keyword(flow.into()),
    ));
    result.push(Declaration::new("grid-auto-rows", auto_rows));
    result.push(Declaration::new(
        "grid-auto-columns",
        grid_initial_auto_track(),
    ));
    Ok(result)
}

/// Pattern 2: `<row-track-list> / auto-flow [dense] <auto-cols>?`
fn parse_grid_auto_cols_pattern(input: &mut Parser) -> Result<Vec<Declaration>, ()> {
    // Parse row track list
    let row_decls = parse_grid_template(input, "grid-template-rows");
    if row_decls.is_empty() {
        return Err(());
    }

    // Expect `/`
    input.expect_delim('/').map_err(|_| ())?;

    // Expect `auto-flow`
    let ident = input.expect_ident().map_err(|_| ())?;
    if !ident.eq_ignore_ascii_case("auto-flow") {
        return Err(());
    }

    // Optional `dense`
    let dense = input
        .try_parse(|i| -> Result<bool, ()> {
            let id = i.expect_ident().map_err(|_| ())?;
            if id.eq_ignore_ascii_case("dense") {
                Ok(true)
            } else {
                Err(())
            }
        })
        .unwrap_or(false);

    // Optional auto-column track size
    let auto_cols = input.try_parse(parse_track_size).unwrap_or(CssValue::Auto);

    // CSS Grid §7.4: `<rows> / [auto-flow] <auto-cols>?` sets
    // grid-auto-flow to `column` (items auto-place along the column axis).
    let flow = if dense { "column dense" } else { "column" };
    let mut result = row_decls;
    result.push(Declaration::new(
        "grid-template-columns",
        CssValue::Keyword("none".into()),
    ));
    result.push(Declaration::new(
        "grid-template-areas",
        CssValue::Keyword("none".into()),
    ));
    result.push(Declaration::new(
        "grid-auto-flow",
        CssValue::Keyword(flow.into()),
    ));
    result.push(Declaration::new(
        "grid-auto-rows",
        grid_initial_auto_track(),
    ));
    result.push(Declaration::new("grid-auto-columns", auto_cols));
    Ok(result)
}

#[cfg(test)]
#[path = "grid_shorthand_tests.rs"]
mod tests;
