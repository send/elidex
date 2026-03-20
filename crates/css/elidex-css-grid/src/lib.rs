//! CSS grid layout property handler plugin (grid-*).

mod parse;
mod resolve;
mod serialize;

use elidex_plugin::{
    css_resolve::keyword_from, ComputedStyle, CssPropertyHandler, CssValue, GridAutoFlow,
    JustifyItems, JustifySelf, ParseError, PropertyDeclaration, ResolveContext,
};
#[cfg(test)]
use elidex_plugin::{GridLine, GridTrackList, LengthUnit, TrackBreadth, TrackSection, TrackSize};
use parse::{
    parse_auto_flow, parse_auto_track_list, parse_grid_line, parse_template_areas, parse_track_list,
};
use resolve::{
    resolve_auto_track_list, resolve_grid_line, resolve_template_areas, resolve_track_list,
};
use serialize::{areas_to_css, auto_track_list_to_css, grid_line_to_css, track_list_to_css};

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
    "grid-template-areas",
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
            "grid-template-areas" => parse_template_areas(input)?,
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
            "grid-template-areas" => {
                style.grid_template_areas = resolve_template_areas(value);
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
            "grid-template-columns" | "grid-template-rows" | "grid-template-areas" => {
                CssValue::Keyword("none".to_string())
            }
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
            "grid-template-areas" => areas_to_css(&style.grid_template_areas),
            "grid-auto-flow" => keyword_from(&style.grid_auto_flow),
            "grid-auto-columns" => auto_track_list_to_css(&style.grid_auto_columns),
            "grid-auto-rows" => auto_track_list_to_css(&style.grid_auto_rows),
            "grid-column-start" => grid_line_to_css(&style.grid_column_start),
            "grid-column-end" => grid_line_to_css(&style.grid_column_end),
            "grid-row-start" => grid_line_to_css(&style.grid_row_start),
            "grid-row-end" => grid_line_to_css(&style.grid_row_end),
            "justify-items" => keyword_from(&style.justify_items),
            "justify-self" => keyword_from(&style.justify_self),
            _ => CssValue::Initial,
        }
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
        assert_eq!(names.len(), 12);
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
            GridTrackList::Explicit(TrackSection::from_tracks(vec![
                TrackSize::Length(100.0),
                TrackSize::Fr(2.0),
                TrackSize::Auto,
            ]))
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
            grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                TrackSize::Length(50.0),
                TrackSize::Fr(1.0),
            ])),
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
            GridTrackList::Explicit(TrackSection::from_tracks(vec![TrackSize::FitContent(
                150.0
            )]))
        );
    }

    #[test]
    fn get_computed_fit_content_roundtrip() {
        let handler = GridHandler;
        let style = ComputedStyle {
            grid_template_columns: GridTrackList::Explicit(TrackSection::from_tracks(vec![
                TrackSize::FitContent(100.0),
            ])),
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
