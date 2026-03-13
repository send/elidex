//! CSS grid layout property handler plugin (grid-*).

use elidex_plugin::{
    css_resolve::{keyword_from, parse_length_unit, resolve_length},
    ComputedStyle, CssPropertyHandler, CssValue, GridAutoFlow, GridLine, LengthUnit, ParseError,
    PropertyDeclaration, ResolveContext, TrackBreadth, TrackSize,
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
            "grid-auto-columns" | "grid-auto-rows" => parse_single_track_size(input)?,
            "grid-column-start" | "grid-column-end" | "grid-row-start" | "grid-row-end" => {
                parse_grid_line(input)?
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
                style.grid_auto_columns = resolve_single_track(value, ctx);
            }
            "grid-auto-rows" => {
                style.grid_auto_rows = resolve_single_track(value, ctx);
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
            _ => {}
        }
    }

    fn initial_value(&self, name: &str) -> CssValue {
        match name {
            "grid-template-columns" | "grid-template-rows" => CssValue::Keyword("none".to_string()),
            "grid-auto-flow" => CssValue::Keyword("row".to_string()),
            "grid-auto-columns" | "grid-auto-rows" | "grid-column-start" | "grid-column-end"
            | "grid-row-start" | "grid-row-end" => CssValue::Auto,
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
            "grid-auto-columns" => track_size_to_css(&style.grid_auto_columns),
            "grid-auto-rows" => track_size_to_css(&style.grid_auto_rows),
            "grid-column-start" => grid_line_to_css(style.grid_column_start),
            "grid-column-end" => grid_line_to_css(style.grid_column_end),
            "grid-row-start" => grid_line_to_css(style.grid_row_start),
            "grid-row-end" => grid_line_to_css(style.grid_row_end),
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

/// Parse a single track size value (used for grid-auto-columns/rows and within track lists).
fn parse_single_track_size(input: &mut cssparser::Parser<'_, '_>) -> Result<CssValue, ParseError> {
    parse_single_track_size_inner(input).map_err(|()| ParseError {
        property: String::new(),
        input: String::new(),
        message: "expected track size".into(),
    })
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

/// Resolve a track list `CssValue` to `Vec<TrackSize>`.
fn resolve_track_list(value: &CssValue, ctx: &ResolveContext) -> Vec<TrackSize> {
    match value {
        CssValue::Keyword(k) if k == "none" => Vec::new(),
        CssValue::List(items) => items.iter().map(|v| resolve_single_track(v, ctx)).collect(),
        // Single track size (grid-template-columns: 100px)
        other => vec![resolve_single_track(other, ctx)],
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
        _ => TrackBreadth::Auto,
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

/// Serialize a track list back to `CssValue`.
fn track_list_to_css(tracks: &[TrackSize]) -> CssValue {
    if tracks.is_empty() {
        return CssValue::Keyword("none".to_string());
    }
    if tracks.len() == 1 {
        return track_size_to_css(&tracks[0]);
    }
    CssValue::List(tracks.iter().map(track_size_to_css).collect())
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
    }
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
        assert_eq!(names.len(), 9);
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
        assert_eq!(style.grid_template_columns.len(), 3);
        assert_eq!(style.grid_template_columns[0], TrackSize::Length(100.0));
        assert_eq!(style.grid_template_columns[1], TrackSize::Fr(2.0));
        assert_eq!(style.grid_template_columns[2], TrackSize::Auto);
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
        assert!(style.grid_template_rows.is_empty());
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
            TrackSize::MinMax(
                Box::new(TrackBreadth::MinContent),
                Box::new(TrackBreadth::MinContent),
            )
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
            grid_template_columns: vec![TrackSize::Length(50.0), TrackSize::Fr(1.0)],
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
    fn parse_percentage_track() {
        let result = parse_prop("grid-template-columns", "50%");
        assert_eq!(result[0].value, CssValue::Percentage(50.0));
    }

    #[test]
    fn parse_auto_columns_single() {
        let result = parse_prop("grid-auto-columns", "200px");
        assert_eq!(result[0].value, CssValue::Length(200.0, LengthUnit::Px));
    }
}
