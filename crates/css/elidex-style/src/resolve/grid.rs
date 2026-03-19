//! CSS Grid property resolution.

use elidex_plugin::{
    AutoRepeatMode, ComputedStyle, CssValue, GridAutoFlow, GridLine, GridTrackList, JustifyItems,
    JustifySelf, LengthUnit, TrackBreadth, TrackSize,
};

use super::helpers::resolve_keyword_enum_prop;
use super::helpers::{resolve_length, resolve_prop, PropertyMap};
use super::ResolveContext;

/// Resolve grid container and item properties.
pub(super) fn resolve_grid_properties(
    style: &mut ComputedStyle,
    winners: &PropertyMap<'_>,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) {
    // grid-auto-flow keyword enum
    resolve_keyword_enum_prop!(
        "grid-auto-flow",
        winners,
        parent_style,
        style.grid_auto_flow,
        GridAutoFlow::from_keyword
    );

    // grid-template-columns / grid-template-rows
    resolve_prop(
        "grid-template-columns",
        winners,
        parent_style,
        |v| resolve_track_list(v, ctx),
        |tracks| style.grid_template_columns = tracks,
    );
    resolve_prop(
        "grid-template-rows",
        winners,
        parent_style,
        |v| resolve_track_list(v, ctx),
        |tracks| style.grid_template_rows = tracks,
    );

    // grid-auto-columns / grid-auto-rows (Vec<TrackSize>)
    resolve_prop(
        "grid-auto-columns",
        winners,
        parent_style,
        |v| resolve_auto_track_list(v, ctx),
        |ts| style.grid_auto_columns = ts,
    );
    resolve_prop(
        "grid-auto-rows",
        winners,
        parent_style,
        |v| resolve_auto_track_list(v, ctx),
        |ts| style.grid_auto_rows = ts,
    );

    // Grid line placement properties
    resolve_prop(
        "grid-column-start",
        winners,
        parent_style,
        resolve_grid_line,
        |gl| style.grid_column_start = gl,
    );
    resolve_prop(
        "grid-column-end",
        winners,
        parent_style,
        resolve_grid_line,
        |gl| style.grid_column_end = gl,
    );
    resolve_prop(
        "grid-row-start",
        winners,
        parent_style,
        resolve_grid_line,
        |gl| style.grid_row_start = gl,
    );
    resolve_prop(
        "grid-row-end",
        winners,
        parent_style,
        resolve_grid_line,
        |gl| style.grid_row_end = gl,
    );

    // justify-items / justify-self keyword enums
    resolve_keyword_enum_prop!(
        "justify-items",
        winners,
        parent_style,
        style.justify_items,
        JustifyItems::from_keyword
    );
    resolve_keyword_enum_prop!(
        "justify-self",
        winners,
        parent_style,
        style.justify_self,
        JustifySelf::from_keyword
    );
}

/// Resolve a `CssValue` track list to `GridTrackList`.
fn resolve_track_list(value: &CssValue, ctx: &ResolveContext) -> GridTrackList {
    match value {
        CssValue::List(items)
            if items.first() == Some(&CssValue::Keyword("auto-repeat".to_string())) =>
        {
            // Auto-repeat marker: [Keyword("auto-repeat"), Keyword(mode), List(before), List(pattern), List(after)]
            let mode = match items.get(1).and_then(|v| v.as_keyword()) {
                Some("auto-fill") => AutoRepeatMode::AutoFill,
                Some("auto-fit") => AutoRepeatMode::AutoFit,
                _ => return GridTrackList::default(),
            };
            let before = items
                .get(2)
                .and_then(|v| match v {
                    CssValue::List(l) => {
                        Some(l.iter().map(|i| resolve_track_size(i, ctx)).collect())
                    }
                    _ => None,
                })
                .unwrap_or_default();
            let pattern = items
                .get(3)
                .and_then(|v| match v {
                    CssValue::List(l) => {
                        Some(l.iter().map(|i| resolve_track_size(i, ctx)).collect())
                    }
                    _ => None,
                })
                .unwrap_or_default();
            let after = items
                .get(4)
                .and_then(|v| match v {
                    CssValue::List(l) => {
                        Some(l.iter().map(|i| resolve_track_size(i, ctx)).collect())
                    }
                    _ => None,
                })
                .unwrap_or_default();
            GridTrackList::AutoRepeat {
                before,
                pattern,
                mode,
                after,
            }
        }
        CssValue::List(items) => GridTrackList::Explicit(
            items
                .iter()
                .map(|item| resolve_track_size(item, ctx))
                .collect(),
        ),
        CssValue::Keyword(k) if k == "none" => GridTrackList::default(),
        _ => GridTrackList::default(),
    }
}

/// Resolve a `CssValue` to a `Vec<TrackSize>` for grid-auto-columns/rows.
fn resolve_auto_track_list(value: &CssValue, ctx: &ResolveContext) -> Vec<TrackSize> {
    match value {
        CssValue::List(items) => items.iter().map(|v| resolve_track_size(v, ctx)).collect(),
        _ => vec![resolve_track_size(value, ctx)],
    }
}

/// Resolve a single `CssValue` to a `TrackSize`.
fn resolve_track_size(value: &CssValue, ctx: &ResolveContext) -> TrackSize {
    match value {
        CssValue::Length(v, LengthUnit::Fr) => TrackSize::Fr(*v),
        CssValue::Length(v, unit) => TrackSize::Length(resolve_length(*v, *unit, ctx)),
        CssValue::Percentage(v) => TrackSize::Percentage(*v),
        CssValue::Keyword(k) if k == "min-content" => TrackSize::MinMax(
            Box::new(TrackBreadth::MinContent),
            Box::new(TrackBreadth::MinContent),
        ),
        CssValue::Keyword(k) if k == "max-content" => TrackSize::MinMax(
            Box::new(TrackBreadth::MaxContent),
            Box::new(TrackBreadth::MaxContent),
        ),
        CssValue::List(items) if items.len() == 3 && items[0].as_keyword() == Some("minmax") => {
            TrackSize::MinMax(
                Box::new(resolve_track_breadth(&items[1], ctx)),
                Box::new(resolve_track_breadth(&items[2], ctx)),
            )
        }
        _ => TrackSize::Auto,
    }
}

/// Resolve a single `CssValue` to a `TrackBreadth`.
fn resolve_track_breadth(value: &CssValue, ctx: &ResolveContext) -> TrackBreadth {
    match value {
        CssValue::Length(v, LengthUnit::Fr) => TrackBreadth::Fr(*v),
        CssValue::Length(v, unit) => TrackBreadth::Length(resolve_length(*v, *unit, ctx)),
        CssValue::Percentage(v) => TrackBreadth::Percentage(*v),
        CssValue::Keyword(k) if k == "min-content" => TrackBreadth::MinContent,
        CssValue::Keyword(k) if k == "max-content" => TrackBreadth::MaxContent,
        _ => TrackBreadth::Auto,
    }
}

/// CSS Grid spec maximum line number magnitude (CSS Grid Level 1 §6.2).
const MAX_GRID_LINE: i32 = 10_000;

/// Resolve a `CssValue` to a `GridLine`.
///
/// Line numbers are clamped to [-10000, 10000] per CSS Grid spec.
/// Non-finite values (NaN/Infinity) resolve to `Auto`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // CSS grid line numbers are small
fn resolve_grid_line(value: &CssValue) -> GridLine {
    match value {
        CssValue::Number(n) if n.is_finite() => {
            let line = (*n as i32).clamp(-MAX_GRID_LINE, MAX_GRID_LINE);
            if line == 0 {
                GridLine::Auto
            } else {
                GridLine::Line(line)
            }
        }
        CssValue::List(items) if items.len() == 2 && items[0].as_keyword() == Some("span") => {
            if let Some(n) = items[1].as_number() {
                if n.is_finite() {
                    #[allow(clippy::cast_sign_loss)]
                    let span = (n as u32).min(MAX_GRID_LINE as u32);
                    if span >= 1 {
                        GridLine::Span(span)
                    } else {
                        GridLine::Auto
                    }
                } else {
                    GridLine::Auto
                }
            } else {
                GridLine::Auto
            }
        }
        _ => GridLine::Auto,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use elidex_plugin::{
        ComputedStyle, CssValue, GridAutoFlow, GridLine, GridTrackList, LengthUnit, TrackBreadth,
        TrackSize,
    };

    use crate::resolve::helpers::PropertyMap;
    use crate::resolve::{build_computed_style, ResolveContext};

    fn default_ctx() -> ResolveContext {
        ResolveContext {
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            em_base: 16.0,
            root_font_size: 16.0,
        }
    }

    // 6a: Table-driven grid-auto-flow keyword resolution
    #[test]
    fn grid_auto_flow_resolved() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        for (keyword, expected) in [
            ("column", GridAutoFlow::Column),
            ("column dense", GridAutoFlow::ColumnDense),
            ("row", GridAutoFlow::Row),
            ("row dense", GridAutoFlow::RowDense),
        ] {
            let kw = CssValue::Keyword(keyword.into());
            let mut winners: PropertyMap = HashMap::new();
            winners.insert("grid-auto-flow", &kw);
            let style = build_computed_style(&winners, &parent, &ctx);
            assert_eq!(style.grid_auto_flow, expected, "grid-auto-flow: {keyword}");
        }
    }

    #[test]
    fn grid_template_columns_resolved() {
        let parent = ComputedStyle::default();
        let tracks = CssValue::List(vec![
            CssValue::Length(100.0, LengthUnit::Px),
            CssValue::Length(1.0, LengthUnit::Fr),
        ]);
        let mut winners: PropertyMap = HashMap::new();
        winners.insert("grid-template-columns", &tracks);
        let ctx = default_ctx();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(
            style.grid_template_columns,
            GridTrackList::Explicit(vec![TrackSize::Length(100.0), TrackSize::Fr(1.0)])
        );
    }

    #[test]
    fn grid_template_minmax_resolved() {
        let parent = ComputedStyle::default();
        let tracks = CssValue::List(vec![CssValue::List(vec![
            CssValue::Keyword("minmax".to_string()),
            CssValue::Length(100.0, LengthUnit::Px),
            CssValue::Length(1.0, LengthUnit::Fr),
        ])]);
        let mut winners: PropertyMap = HashMap::new();
        winners.insert("grid-template-rows", &tracks);
        let ctx = default_ctx();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(
            style.grid_template_rows,
            GridTrackList::Explicit(vec![TrackSize::MinMax(
                Box::new(TrackBreadth::Length(100.0)),
                Box::new(TrackBreadth::Fr(1.0)),
            )])
        );
    }

    // 6c: Table-driven grid line resolution (number, zero→auto, span)
    #[test]
    fn grid_line_resolution() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let cases: Vec<(&str, CssValue, GridLine)> = vec![
            (
                "grid-column-start",
                CssValue::Number(3.0),
                GridLine::Line(3),
            ),
            ("grid-column-start", CssValue::Number(0.0), GridLine::Auto),
            (
                "grid-row-end",
                CssValue::List(vec![
                    CssValue::Keyword("span".to_string()),
                    CssValue::Number(2.0),
                ]),
                GridLine::Span(2),
            ),
            ("grid-column-end", CssValue::Number(5.0), GridLine::Line(5)),
            (
                "grid-row-start",
                CssValue::List(vec![
                    CssValue::Keyword("span".into()),
                    CssValue::Number(3.0),
                ]),
                GridLine::Span(3),
            ),
        ];
        for (prop, value, expected) in &cases {
            let mut winners: PropertyMap = HashMap::new();
            winners.insert(*prop, value);
            let style = build_computed_style(&winners, &parent, &ctx);
            let actual = match *prop {
                "grid-column-start" => style.grid_column_start,
                "grid-column-end" => style.grid_column_end,
                "grid-row-start" => style.grid_row_start,
                "grid-row-end" => style.grid_row_end,
                _ => unreachable!(),
            };
            assert_eq!(actual, *expected, "{prop}: {value:?}");
        }
    }

    #[test]
    fn resolve_grid_auto_columns() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let val = CssValue::Length(50.0, LengthUnit::Px);
        let mut winners: PropertyMap = HashMap::new();
        winners.insert("grid-auto-columns", &val);
        let style = build_computed_style(&winners, &parent, &ctx);
        assert_eq!(style.grid_auto_columns, vec![TrackSize::Length(50.0)]);
    }

    #[test]
    fn grid_defaults_without_winners() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let winners: PropertyMap = HashMap::new();
        let style = build_computed_style(&winners, &parent, &ctx);
        assert!(style.grid_template_columns.is_empty());
        assert!(style.grid_template_rows.is_empty());
        assert_eq!(style.grid_auto_flow, GridAutoFlow::Row);
        assert_eq!(style.grid_auto_columns, vec![TrackSize::Auto]);
        assert_eq!(style.grid_auto_rows, vec![TrackSize::Auto]);
        assert_eq!(style.grid_column_start, GridLine::Auto);
        assert_eq!(style.grid_column_end, GridLine::Auto);
        assert_eq!(style.grid_row_start, GridLine::Auto);
        assert_eq!(style.grid_row_end, GridLine::Auto);
    }

    #[test]
    fn grid_computed_value_roundtrip() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        let tracks = CssValue::List(vec![
            CssValue::Length(100.0, LengthUnit::Px),
            CssValue::Length(2.0, LengthUnit::Fr),
        ]);
        let line_val = CssValue::Number(3.0);
        let span_val = CssValue::List(vec![
            CssValue::Keyword("span".into()),
            CssValue::Number(2.0),
        ]);
        let mut winners: PropertyMap = HashMap::new();
        winners.insert("grid-template-columns", &tracks);
        winners.insert("grid-row-start", &line_val);
        winners.insert("grid-row-end", &span_val);
        let style = build_computed_style(&winners, &parent, &ctx);

        let cols = crate::get_computed("grid-template-columns", &style);
        assert_eq!(
            cols,
            CssValue::List(vec![
                CssValue::Length(100.0, LengthUnit::Px),
                CssValue::Length(2.0, LengthUnit::Fr),
            ])
        );
        let rs = crate::get_computed("grid-row-start", &style);
        assert_eq!(rs, CssValue::Number(3.0));
        let re = crate::get_computed("grid-row-end", &style);
        assert_eq!(
            re,
            CssValue::List(vec![
                CssValue::Keyword("span".into()),
                CssValue::Number(2.0),
            ])
        );
    }

    // 6d: Table-driven grid line safety (clamping, NaN)
    #[test]
    fn grid_line_safety() {
        let parent = ComputedStyle::default();
        let ctx = default_ctx();
        for (value, prop, expected) in [
            (
                CssValue::Number(99999.0),
                "grid-column-start",
                GridLine::Line(10_000),
            ),
            (
                CssValue::Number(-99999.0),
                "grid-row-start",
                GridLine::Line(-10_000),
            ),
            (
                CssValue::Number(f32::NAN),
                "grid-column-start",
                GridLine::Auto,
            ),
        ] {
            let mut winners: PropertyMap = HashMap::new();
            winners.insert(prop, &value);
            let style = build_computed_style(&winners, &parent, &ctx);
            let actual = match prop {
                "grid-column-start" => style.grid_column_start,
                "grid-row-start" => style.grid_row_start,
                _ => unreachable!(),
            };
            assert_eq!(actual, expected, "{prop}: {value:?}");
        }
    }
}
