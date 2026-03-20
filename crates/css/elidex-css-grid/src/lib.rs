//! CSS grid layout property handler plugin (grid-*).

use elidex_plugin::{
    css_resolve::{keyword_from, parse_length_unit, resolve_length},
    AutoRepeatMode, ComputedStyle, CssPropertyHandler, CssValue, GridAutoFlow, GridLine,
    GridTrackList, JustifyItems, JustifySelf, LengthUnit, ParseError, PropertyDeclaration,
    ResolveContext, TrackBreadth, TrackSize,
};

/// CSS grid property handler.
#[derive(Clone)]
pub struct GridHandler;

impl GridHandler {
    /// Register this handler in a CSS property registry.
    pub fn register(registry: &mut elidex_plugin::CssPropertyRegistry) {
        elidex_plugin::register_css_handler(registry, Self);
    }
}

const GRID_PROPERTIES: &[&str] = &[
    "grid-template-columns",
    "grid-template-rows",
    "grid-auto-flow",
    "grid-auto-columns",
    "grid-auto-rows",
    "grid-column-start",
    "grid-column-end",
    "grid-row-start",
    "grid-row-end",
    "justify-items",
    "justify-self",
];

impl CssPropertyHandler for GridHandler {
    fn property_names(&self) -> &[&str] {
        GRID_PROPERTIES
    }

    fn parse(
        &self,
        name: &str,
        input: &mut cssparser::Parser<'_, '_>,
    ) -> Result<Vec<PropertyDeclaration>, ParseError> {
        let value = match name {
            "grid-template-columns" | "grid-template-rows" => parse_track_list(input)?,
            "grid-auto-flow" => parse_auto_flow(input)?,
            "grid-auto-columns" | "grid-auto-rows" => parse_auto_track_list(input)?,
            "grid-column-start" | "grid-column-end" | "grid-row-start" | "grid-row-end" => {
                parse_grid_line(input)?
            }
            "justify-items" | "justify-self" => {
                let ident = input
                    .expect_ident()
                    .map(|s| s.to_ascii_lowercase())
                    .map_err(|_| ParseError {
                        property: name.into(),
                        input: String::new(),
                        message: "expected alignment keyword".into(),
                    })?;
                match ident.as_str() {
                    "stretch" | "start" | "end" | "center" | "baseline" => CssValue::Keyword(ident),
                    "auto" if name == "justify-self" => CssValue::Keyword(ident),
                    _ => {
                        return Err(ParseError {
                            property: name.into(),
                            input: ident,
                            message: "invalid alignment keyword".into(),
                        })
                    }
                }
            }
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
            "grid-template-columns" => {
                style.grid_template_columns = resolve_track_list(value, ctx);
            }
            "grid-template-rows" => {
                style.grid_template_rows = resolve_track_list(value, ctx);
            }
            "grid-auto-flow" => {
                elidex_plugin::resolve_keyword!(value, style.grid_auto_flow, GridAutoFlow);
            }
            "grid-auto-columns" => {
                style.grid_auto_columns = resolve_auto_track_list(value, ctx);
            }
            "grid-auto-rows" => {
                style.grid_auto_rows = resolve_auto_track_list(value, ctx);
            }
            "grid-column-start" => {
                style.grid_column_start = resolve_grid_line(value);
            }
            "grid-column-end" => {
                style.grid_column_end = resolve_grid_line(value);
            }
            "grid-row-start" => {
                style.grid_row_start = resolve_grid_line(value);
            }
            "grid-row-end" => {
                style.grid_row_end = resolve_grid_line(value);
            }
            "justify-items" => {
                elidex_plugin::resolve_keyword!(value, style.justify_items, JustifyItems);
            }
            "justify-self" => {
                elidex_plugin::resolve_keyword!(value, style.justify_self, JustifySelf);
            }
            _ => {}
        }
    }

    fn initial_value(&self, name: &str) -> CssValue {
        match name {
            "grid-template-columns" | "grid-template-rows" => CssValue::Keyword("none".to_string()),
            "grid-auto-flow" => CssValue::Keyword("row".to_string()),
            "grid-auto-columns" | "grid-auto-rows" | "grid-column-start" | "grid-column-end"
            | "grid-row-start" | "grid-row-end" => CssValue::Auto,
            "justify-items" => CssValue::Keyword("stretch".to_string()),
            "justify-self" => CssValue::Keyword("auto".to_string()),
            _ => CssValue::Initial,
        }
    }

    fn is_inherited(&self, _name: &str) -> bool {
        false
    }

    fn affects_layout(&self, _name: &str) -> bool {
        true
    }

    fn get_computed(&self, name: &str, style: &ComputedStyle) -> CssValue {
        match name {
            "grid-template-columns" => track_list_to_css(&style.grid_template_columns),
            "grid-template-rows" => track_list_to_css(&style.grid_template_rows),
            "grid-auto-flow" => keyword_from(&style.grid_auto_flow),
            "grid-auto-columns" => auto_track_list_to_css(&style.grid_auto_columns),
            "grid-auto-rows" => auto_track_list_to_css(&style.grid_auto_rows),
            "grid-column-start" => grid_line_to_css(style.grid_column_start),
            "grid-column-end" => grid_line_to_css(style.grid_column_end),
            "grid-row-start" => grid_line_to_css(style.grid_row_start),
            "grid-row-end" => grid_line_to_css(style.grid_row_end),
            "justify-items" => keyword_from(&style.justify_items),
            "justify-self" => keyword_from(&style.justify_self),
            _ => CssValue::Initial,
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Maximum number of track entries in a grid-template-columns/rows list.
const MAX_TRACKS: usize = 10_000;

/// Parse a grid-template-columns / grid-template-rows value.
///
/// `none` produces `Keyword("none")`, otherwise a space-separated list of track sizes.
fn parse_track_list(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    if input.try_parse(|i| i.expect_ident_matching("none")).is_ok() {
        return Ok(CssValue::Keyword("none".to_string()));
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
fn parse_auto_track_list(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
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
fn parse_auto_flow(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
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

/// Parse a grid line value: `auto`, `<integer>`, `span <integer>`.
fn parse_grid_line(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    // Try "auto"
    if input.try_parse(|i| i.expect_ident_matching("auto")).is_ok() {
        return Ok(CssValue::Auto);
    }

    // Try "span <integer>"
    if let Ok(val) = input.try_parse(|i| {
        i.expect_ident_matching("span").map_err(|_| ())?;
        let n = i.expect_integer().map_err(|_| ())?;
        if n <= 0 {
            return Err(());
        }
        Ok(CssValue::List(vec![
            CssValue::Keyword("span".to_string()),
            #[allow(clippy::cast_precision_loss)]
            CssValue::Number(n as f32),
        ]))
    }) {
        return Ok(val);
    }

    // Try integer
    let n = input.expect_integer().map_err(|_| ParseError {
        property: String::new(),
        input: String::new(),
        message: "expected 'auto', integer, or 'span <integer>'".into(),
    })?;
    #[allow(clippy::cast_precision_loss)]
    Ok(CssValue::Number(n as f32))
}

// ---------------------------------------------------------------------------
// Resolution helpers
// ---------------------------------------------------------------------------

/// Resolve a track list `CssValue` to `GridTrackList`.
fn resolve_track_list(value: &CssValue, ctx: &ResolveContext) -> GridTrackList {
    match value {
        CssValue::Keyword(k) if k == "none" => GridTrackList::default(),
        CssValue::List(items)
            if items.first() == Some(&CssValue::Keyword("auto-repeat".to_string())) =>
        {
            // Auto-repeat marker: [Keyword("auto-repeat"), Keyword(mode), List(before), List(pattern), List(after)]
            let mode = match items.get(1).and_then(|v| v.as_keyword()) {
                Some("auto-fill") => AutoRepeatMode::AutoFill,
                Some("auto-fit") => AutoRepeatMode::AutoFit,
                _ => return GridTrackList::default(),
            };
            let resolve_list = |v: &CssValue| -> Vec<TrackSize> {
                match v {
                    CssValue::List(l) => l.iter().map(|i| resolve_single_track(i, ctx)).collect(),
                    _ => Vec::new(),
                }
            };
            let before = items.get(2).map_or_else(Vec::new, resolve_list);
            let pattern = items.get(3).map_or_else(Vec::new, resolve_list);
            let after = items.get(4).map_or_else(Vec::new, resolve_list);
            GridTrackList::AutoRepeat {
                before,
                pattern,
                mode,
                after,
            }
        }
        // Single fit-content() or minmax() — these are CssValue::List but represent one track.
        CssValue::List(items)
            if items
                .first()
                .and_then(|v| v.as_keyword())
                .is_some_and(|k| k == "fit-content" || k == "minmax") =>
        {
            GridTrackList::Explicit(vec![resolve_single_track(value, ctx)])
        }
        CssValue::List(items) => {
            GridTrackList::Explicit(items.iter().map(|v| resolve_single_track(v, ctx)).collect())
        }
        // Single track size (grid-template-columns: 100px)
        other => GridTrackList::Explicit(vec![resolve_single_track(other, ctx)]),
    }
}

/// Resolve a single `CssValue` to `TrackSize`.
fn resolve_single_track(value: &CssValue, ctx: &ResolveContext) -> TrackSize {
    match value {
        CssValue::Length(v, LengthUnit::Fr) => TrackSize::Fr(*v),
        CssValue::Length(v, unit) => TrackSize::Length(resolve_length(*v, *unit, ctx)),
        CssValue::Percentage(p) => TrackSize::Percentage(*p),
        CssValue::Keyword(k) => match k.as_str() {
            "min-content" => TrackSize::MinMax(
                Box::new(TrackBreadth::MinContent),
                Box::new(TrackBreadth::MinContent),
            ),
            "max-content" => TrackSize::MinMax(
                Box::new(TrackBreadth::MaxContent),
                Box::new(TrackBreadth::MaxContent),
            ),
            _ => TrackSize::Auto,
        },
        CssValue::List(items)
            if items.first() == Some(&CssValue::Keyword("fit-content".to_string())) =>
        {
            // fit-content(<length-percentage>): resolve the limit argument.
            let limit = items.get(1).map_or(0.0, |v| match v {
                CssValue::Length(px, unit) => resolve_length(*px, *unit, ctx),
                CssValue::Percentage(pct) => {
                    // Percentage resolved against viewport width as approximation.
                    // Correct resolution against the grid container's available space
                    // happens in track sizing (CSS Grid §7.2.4).
                    ctx.viewport_width * pct / 100.0
                }
                _ => 0.0,
            });
            TrackSize::FitContent(limit)
        }
        CssValue::List(items)
            if items.first() == Some(&CssValue::Keyword("minmax".to_string())) =>
        {
            let min = items
                .get(1)
                .map_or(TrackBreadth::Auto, |v| resolve_breadth(v, ctx));
            let max = items
                .get(2)
                .map_or(TrackBreadth::Auto, |v| resolve_breadth(v, ctx));
            TrackSize::MinMax(Box::new(min), Box::new(max))
        }
        _ => TrackSize::Auto,
    }
}

/// Resolve a breadth value inside `minmax()`.
fn resolve_breadth(value: &CssValue, ctx: &ResolveContext) -> TrackBreadth {
    match value {
        CssValue::Length(v, LengthUnit::Fr) => TrackBreadth::Fr(*v),
        CssValue::Length(v, unit) => TrackBreadth::Length(resolve_length(*v, *unit, ctx)),
        CssValue::Percentage(p) => TrackBreadth::Percentage(*p),
        CssValue::Keyword(k) => match k.as_str() {
            "min-content" => TrackBreadth::MinContent,
            "max-content" => TrackBreadth::MaxContent,
            _ => TrackBreadth::Auto,
        },
        CssValue::List(items)
            if items.first() == Some(&CssValue::Keyword("fit-content".to_string())) =>
        {
            let limit = items.get(1).map_or(0.0, |v| match v {
                CssValue::Length(px, unit) => resolve_length(*px, *unit, ctx),
                CssValue::Percentage(pct) => ctx.viewport_width * pct / 100.0,
                _ => 0.0,
            });
            TrackBreadth::FitContent(limit)
        }
        _ => TrackBreadth::Auto,
    }
}

/// Resolve a `CssValue` to a `Vec<TrackSize>` for grid-auto-columns/rows.
fn resolve_auto_track_list(value: &CssValue, ctx: &ResolveContext) -> Vec<TrackSize> {
    match value {
        CssValue::List(items) => items.iter().map(|v| resolve_single_track(v, ctx)).collect(),
        _ => vec![resolve_single_track(value, ctx)],
    }
}

/// Resolve a grid line value.
fn resolve_grid_line(value: &CssValue) -> GridLine {
    match value {
        CssValue::Number(n) => {
            #[allow(clippy::cast_possible_truncation)]
            let i = *n as i32;
            if i == 0 {
                GridLine::Auto
            } else {
                GridLine::Line(i.clamp(-10000, 10000))
            }
        }
        CssValue::List(items) => {
            // span <integer>
            if items.first() == Some(&CssValue::Keyword("span".to_string())) {
                if let Some(CssValue::Number(n)) = items.get(1) {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let span = (*n as u32).max(1);
                    return GridLine::Span(span);
                }
            }
            GridLine::Auto
        }
        _ => GridLine::Auto,
    }
}

// ---------------------------------------------------------------------------
// Serialization helpers
// ---------------------------------------------------------------------------

/// Serialize a `GridTrackList` back to `CssValue`.
fn track_list_to_css(list: &GridTrackList) -> CssValue {
    match list {
        GridTrackList::Explicit(tracks) => {
            if tracks.is_empty() {
                return CssValue::Keyword("none".to_string());
            }
            if tracks.len() == 1 {
                return track_size_to_css(&tracks[0]);
            }
            CssValue::List(tracks.iter().map(track_size_to_css).collect())
        }
        GridTrackList::AutoRepeat {
            before,
            pattern,
            mode,
            after,
        } => {
            let mode_str = match mode {
                AutoRepeatMode::AutoFill => "auto-fill",
                AutoRepeatMode::AutoFit => "auto-fit",
            };
            CssValue::List(vec![
                CssValue::Keyword("auto-repeat".to_string()),
                CssValue::Keyword(mode_str.to_string()),
                CssValue::List(before.iter().map(track_size_to_css).collect()),
                CssValue::List(pattern.iter().map(track_size_to_css).collect()),
                CssValue::List(after.iter().map(track_size_to_css).collect()),
            ])
        }
    }
}

/// Serialize a single `TrackSize` to `CssValue`.
fn track_size_to_css(ts: &TrackSize) -> CssValue {
    match ts {
        TrackSize::Length(px) => CssValue::Length(*px, LengthUnit::Px),
        TrackSize::Fr(f) => CssValue::Length(*f, LengthUnit::Fr),
        TrackSize::Percentage(p) => CssValue::Percentage(*p),
        TrackSize::Auto => CssValue::Auto,
        TrackSize::MinMax(min, max) => CssValue::List(vec![
            CssValue::Keyword("minmax".to_string()),
            breadth_to_css(min),
            breadth_to_css(max),
        ]),
        TrackSize::FitContent(px) => CssValue::List(vec![
            CssValue::Keyword("fit-content".to_string()),
            CssValue::Length(*px, LengthUnit::Px),
        ]),
    }
}

/// Serialize a `TrackBreadth` to `CssValue`.
fn breadth_to_css(b: &TrackBreadth) -> CssValue {
    match b {
        TrackBreadth::Length(px) => CssValue::Length(*px, LengthUnit::Px),
        TrackBreadth::Fr(f) => CssValue::Length(*f, LengthUnit::Fr),
        TrackBreadth::Percentage(p) => CssValue::Percentage(*p),
        TrackBreadth::Auto => CssValue::Auto,
        TrackBreadth::MinContent => CssValue::Keyword("min-content".to_string()),
        TrackBreadth::MaxContent => CssValue::Keyword("max-content".to_string()),
        TrackBreadth::FitContent(px) => CssValue::List(vec![
            CssValue::Keyword("fit-content".to_string()),
            CssValue::Length(*px, LengthUnit::Px),
        ]),
    }
}

/// Serialize a `Vec<TrackSize>` to `CssValue` for grid-auto-columns/rows.
fn auto_track_list_to_css(tracks: &[TrackSize]) -> CssValue {
    if tracks.len() == 1 {
        return track_size_to_css(&tracks[0]);
    }
    CssValue::List(tracks.iter().map(track_size_to_css).collect())
}

/// Serialize a `GridLine` to `CssValue`.
fn grid_line_to_css(gl: GridLine) -> CssValue {
    match gl {
        GridLine::Auto => CssValue::Auto,
        #[allow(clippy::cast_precision_loss)]
        GridLine::Line(n) => CssValue::Number(n as f32),
        #[allow(clippy::cast_precision_loss)]
        GridLine::Span(n) => CssValue::List(vec![
            CssValue::Keyword("span".to_string()),
            CssValue::Number(n as f32),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_prop(name: &str, input_str: &str) -> Vec<PropertyDeclaration> {
        let handler = GridHandler;
        let mut pi = cssparser::ParserInput::new(input_str);
        let mut parser = cssparser::Parser::new(&mut pi);
        handler.parse(name, &mut parser).unwrap()
    }

    #[test]
    fn property_names_complete() {
        let handler = GridHandler;
        let names = handler.property_names();
        assert_eq!(names.len(), 11);
        assert!(names.contains(&"grid-template-columns"));
        assert!(names.contains(&"grid-row-end"));
    }

    #[test]
    fn parse_template_none() {
        let result = parse_prop("grid-template-columns", "none");
        assert_eq!(result[0].value, CssValue::Keyword("none".to_string()));
    }

    #[test]
    fn parse_template_track_list() {
        let result = parse_prop("grid-template-columns", "100px 1fr auto");
        assert_eq!(result[0].property, "grid-template-columns");
        if let CssValue::List(ref items) = result[0].value {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], CssValue::Length(100.0, LengthUnit::Px));
            assert_eq!(items[1], CssValue::Length(1.0, LengthUnit::Fr));
            assert_eq!(items[2], CssValue::Auto);
        } else {
            panic!("expected List");
        }
    }

    #[test]
    fn parse_template_minmax() {
        let result = parse_prop("grid-template-columns", "minmax(100px, 1fr)");
        if let CssValue::List(ref items) = result[0].value {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], CssValue::Keyword("minmax".to_string()));
            assert_eq!(items[1], CssValue::Length(100.0, LengthUnit::Px));
            assert_eq!(items[2], CssValue::Length(1.0, LengthUnit::Fr));
        } else {
            panic!("expected List");
        }
    }

    #[test]
    fn parse_auto_flow_row() {
        let result = parse_prop("grid-auto-flow", "row");
        assert_eq!(result[0].value, CssValue::Keyword("row".to_string()));
    }

    #[test]
    fn parse_auto_flow_column_dense() {
        let result = parse_prop("grid-auto-flow", "column dense");
        assert_eq!(
            result[0].value,
            CssValue::Keyword("column dense".to_string())
        );
    }

    #[test]
    fn parse_grid_line_auto() {
        let result = parse_prop("grid-column-start", "auto");
        assert_eq!(result[0].value, CssValue::Auto);
    }

    #[test]
    fn parse_grid_line_integer() {
        let result = parse_prop("grid-column-start", "3");
        assert_eq!(result[0].value, CssValue::Number(3.0));
    }

    #[test]
    fn parse_grid_line_span() {
        let result = parse_prop("grid-row-end", "span 2");
        assert_eq!(
            result[0].value,
            CssValue::List(vec![
                CssValue::Keyword("span".to_string()),
                CssValue::Number(2.0),
            ])
        );
    }

    #[test]
    fn resolve_template_columns() {
        let handler = GridHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        let value = CssValue::List(vec![
            CssValue::Length(100.0, LengthUnit::Px),
            CssValue::Length(2.0, LengthUnit::Fr),
            CssValue::Auto,
        ]);
        handler.resolve("grid-template-columns", &value, &ctx, &mut style);
        assert_eq!(
            style.grid_template_columns,
            GridTrackList::Explicit(vec![
                TrackSize::Length(100.0),
                TrackSize::Fr(2.0),
                TrackSize::Auto,
            ])
        );
    }

    #[test]
    fn resolve_template_none() {
        let handler = GridHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        handler.resolve(
            "grid-template-rows",
            &CssValue::Keyword("none".into()),
            &ctx,
            &mut style,
        );
        assert_eq!(style.grid_template_rows, GridTrackList::default());
    }

    #[test]
    fn resolve_auto_flow() {
        let handler = GridHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        handler.resolve(
            "grid-auto-flow",
            &CssValue::Keyword("column dense".into()),
            &ctx,
            &mut style,
        );
        assert_eq!(style.grid_auto_flow, GridAutoFlow::ColumnDense);
    }

    #[test]
    fn resolve_grid_line_clamp() {
        let handler = GridHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        handler.resolve(
            "grid-column-start",
            &CssValue::Number(99999.0),
            &ctx,
            &mut style,
        );
        assert_eq!(style.grid_column_start, GridLine::Line(10000));
    }

    #[test]
    fn resolve_grid_line_zero_is_auto() {
        let handler = GridHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        handler.resolve("grid-row-start", &CssValue::Number(0.0), &ctx, &mut style);
        assert_eq!(style.grid_row_start, GridLine::Auto);
    }

    #[test]
    fn resolve_min_content_keyword() {
        let handler = GridHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        handler.resolve(
            "grid-auto-columns",
            &CssValue::Keyword("min-content".into()),
            &ctx,
            &mut style,
        );
        assert_eq!(
            style.grid_auto_columns,
            vec![TrackSize::MinMax(
                Box::new(TrackBreadth::MinContent),
                Box::new(TrackBreadth::MinContent),
            )]
        );
    }

    #[test]
    fn initial_values() {
        let handler = GridHandler;
        assert_eq!(
            handler.initial_value("grid-template-columns"),
            CssValue::Keyword("none".to_string())
        );
        assert_eq!(
            handler.initial_value("grid-auto-flow"),
            CssValue::Keyword("row".to_string())
        );
        assert_eq!(handler.initial_value("grid-auto-columns"), CssValue::Auto);
        assert_eq!(handler.initial_value("grid-column-start"), CssValue::Auto);
    }

    #[test]
    fn not_inherited() {
        let handler = GridHandler;
        for name in handler.property_names() {
            assert!(!handler.is_inherited(name));
        }
    }

    #[test]
    fn get_computed_roundtrip() {
        let handler = GridHandler;
        let style = ComputedStyle {
            grid_template_columns: GridTrackList::Explicit(vec![
                TrackSize::Length(50.0),
                TrackSize::Fr(1.0),
            ]),
            grid_auto_flow: GridAutoFlow::RowDense,
            grid_column_start: GridLine::Span(3),
            grid_row_end: GridLine::Line(-2),
            ..ComputedStyle::default()
        };
        // template columns
        let v = handler.get_computed("grid-template-columns", &style);
        assert_eq!(
            v,
            CssValue::List(vec![
                CssValue::Length(50.0, LengthUnit::Px),
                CssValue::Length(1.0, LengthUnit::Fr),
            ])
        );
        // auto-flow
        assert_eq!(
            handler.get_computed("grid-auto-flow", &style),
            CssValue::Keyword("row dense".to_string())
        );
        // span
        assert_eq!(
            handler.get_computed("grid-column-start", &style),
            CssValue::List(vec![
                CssValue::Keyword("span".to_string()),
                CssValue::Number(3.0),
            ])
        );
        // negative line
        assert_eq!(
            handler.get_computed("grid-row-end", &style),
            CssValue::Number(-2.0)
        );
    }

    #[test]
    fn parse_fit_content() {
        let result = parse_prop("grid-template-columns", "fit-content(200px)");
        assert_eq!(
            result[0].value,
            CssValue::List(vec![
                CssValue::Keyword("fit-content".to_string()),
                CssValue::Length(200.0, LengthUnit::Px),
            ])
        );
    }

    #[test]
    fn resolve_fit_content_track() {
        let handler = GridHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        let value = CssValue::List(vec![
            CssValue::Keyword("fit-content".to_string()),
            CssValue::Length(150.0, LengthUnit::Px),
        ]);
        handler.resolve("grid-template-columns", &value, &ctx, &mut style);
        assert_eq!(
            style.grid_template_columns,
            GridTrackList::Explicit(vec![TrackSize::FitContent(150.0)])
        );
    }

    #[test]
    fn get_computed_fit_content_roundtrip() {
        let handler = GridHandler;
        let style = ComputedStyle {
            grid_template_columns: GridTrackList::Explicit(vec![TrackSize::FitContent(100.0)]),
            ..ComputedStyle::default()
        };
        let v = handler.get_computed("grid-template-columns", &style);
        assert_eq!(
            v,
            CssValue::List(vec![
                CssValue::Keyword("fit-content".to_string()),
                CssValue::Length(100.0, LengthUnit::Px),
            ])
        );
    }

    #[test]
    fn parse_minmax_min_content_fit_content() {
        // fit-content inside minmax is not standard syntax, but ensure the parser
        // at least handles individual fit-content tracks correctly alongside minmax.
        let result = parse_prop("grid-template-columns", "fit-content(100px) 1fr");
        if let CssValue::List(ref items) = result[0].value {
            assert_eq!(items.len(), 2);
            // First track: fit-content(100px)
            assert_eq!(
                items[0],
                CssValue::List(vec![
                    CssValue::Keyword("fit-content".to_string()),
                    CssValue::Length(100.0, LengthUnit::Px),
                ])
            );
            // Second track: 1fr
            assert_eq!(items[1], CssValue::Length(1.0, LengthUnit::Fr));
        } else {
            panic!("expected List");
        }
    }

    #[test]
    fn parse_percentage_track() {
        let result = parse_prop("grid-template-columns", "50%");
        assert_eq!(result[0].value, CssValue::Percentage(50.0));
    }

    #[test]
    fn parse_auto_columns_single() {
        let result = parse_prop("grid-auto-columns", "200px");
        assert_eq!(result[0].value, CssValue::Length(200.0, LengthUnit::Px));
    }

    #[test]
    fn parse_justify_items() {
        for kw in ["stretch", "start", "end", "center", "baseline"] {
            let result = parse_prop("justify-items", kw);
            assert_eq!(result[0].value, CssValue::Keyword(kw.to_string()), "{kw}");
        }
    }

    #[test]
    fn parse_justify_self() {
        for kw in ["auto", "start", "end", "center", "stretch", "baseline"] {
            let result = parse_prop("justify-self", kw);
            assert_eq!(result[0].value, CssValue::Keyword(kw.to_string()), "{kw}");
        }
    }

    #[test]
    fn resolve_justify_items() {
        let handler = GridHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        handler.resolve(
            "justify-items",
            &CssValue::Keyword("center".into()),
            &ctx,
            &mut style,
        );
        assert_eq!(style.justify_items, JustifyItems::Center);
    }

    #[test]
    fn resolve_justify_self() {
        let handler = GridHandler;
        let ctx = ResolveContext::default();
        let mut style = ComputedStyle::default();
        handler.resolve(
            "justify-self",
            &CssValue::Keyword("end".into()),
            &ctx,
            &mut style,
        );
        assert_eq!(style.justify_self, JustifySelf::End);
    }
}
