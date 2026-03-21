//! Resolution helpers for CSS grid properties.

use elidex_plugin::{
    css_resolve::resolve_length, CssValue, GridLine, GridTemplateAreas, GridTrackList,
    ResolveContext, TrackBreadth, TrackSection, TrackSize,
};
use elidex_plugin::{AutoRepeatMode, LengthUnit};

/// Resolve a track list `CssValue` to `GridTrackList`.
pub(crate) fn resolve_track_list(value: &CssValue, ctx: &ResolveContext) -> GridTrackList {
    match value {
        CssValue::Keyword(k) if k == "none" => GridTrackList::default(),
        // CSS Grid Level 2 §2: subgrid [<line-name-list>]*
        CssValue::List(items)
            if items.first() == Some(&CssValue::Keyword("subgrid".to_string())) =>
        {
            let line_names: Vec<Vec<String>> = items[1..]
                .iter()
                .map(|v| match v {
                    CssValue::List(names) => names
                        .iter()
                        .filter_map(|n| n.as_keyword().map(String::from))
                        .collect(),
                    _ => vec![],
                })
                .collect();
            GridTrackList::Subgrid { line_names }
        }
        CssValue::List(items)
            if items.first() == Some(&CssValue::Keyword("auto-repeat".to_string())) =>
        {
            // Auto-repeat marker: [Keyword("auto-repeat"), Keyword(mode), List(before), List(pattern), List(after)]
            let mode = match items.get(1).and_then(|v| v.as_keyword()) {
                Some("auto-fill") => AutoRepeatMode::AutoFill,
                Some("auto-fit") => AutoRepeatMode::AutoFit,
                _ => return GridTrackList::default(),
            };
            let resolve_section = |v: &CssValue| -> TrackSection { resolve_track_section(v, ctx) };
            let before = items
                .get(2)
                .map_or_else(TrackSection::default, resolve_section);
            let pattern = items
                .get(3)
                .map_or_else(TrackSection::default, resolve_section);
            let after = items
                .get(4)
                .map_or_else(TrackSection::default, resolve_section);
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
            GridTrackList::Explicit(TrackSection::from_tracks(vec![resolve_single_track(
                value, ctx,
            )]))
        }
        // Check for named-tracks marker
        CssValue::List(items)
            if items.first() == Some(&CssValue::Keyword("named-tracks".to_string())) =>
        {
            GridTrackList::Explicit(resolve_named_track_section(items, ctx))
        }
        CssValue::List(items) => GridTrackList::Explicit(TrackSection::from_tracks(
            items.iter().map(|v| resolve_single_track(v, ctx)).collect(),
        )),
        // Single track size (grid-template-columns: 100px)
        other => GridTrackList::Explicit(TrackSection::from_tracks(vec![resolve_single_track(
            other, ctx,
        )])),
    }
}

/// Resolve a `CssValue` section (possibly with named-tracks marker) into a `TrackSection`.
fn resolve_track_section(value: &CssValue, ctx: &ResolveContext) -> TrackSection {
    match value {
        CssValue::List(items)
            if items.first() == Some(&CssValue::Keyword("named-tracks".to_string())) =>
        {
            resolve_named_track_section(items, ctx)
        }
        CssValue::List(items) => {
            TrackSection::from_tracks(items.iter().map(|v| resolve_single_track(v, ctx)).collect())
        }
        _ => TrackSection::default(),
    }
}

/// Resolve a named-tracks encoded list into a `TrackSection`.
///
/// Format: `[Keyword("named-tracks"), List(names_0), track0, List(names_1), track1, ..., List(names_n)]`
fn resolve_named_track_section(items: &[CssValue], ctx: &ResolveContext) -> TrackSection {
    // Skip the "named-tracks" marker
    let rest = &items[1..];
    let mut tracks = Vec::new();
    let mut line_names: Vec<Vec<String>> = Vec::new();

    let mut i = 0;
    while i < rest.len() {
        // Check if this is a line-names list
        if let CssValue::List(names) = &rest[i] {
            let name_strs: Vec<String> = names
                .iter()
                .filter_map(|v| v.as_keyword().map(String::from))
                .collect();
            line_names.push(name_strs);
            i += 1;
        } else {
            // It's a track value — ensure we have a line_names entry before it
            if line_names.len() == tracks.len() {
                line_names.push(vec![]);
            }
            tracks.push(resolve_single_track(&rest[i], ctx));
            i += 1;
        }
    }
    // Ensure trailing line_names
    while line_names.len() <= tracks.len() {
        line_names.push(vec![]);
    }

    TrackSection { tracks, line_names }
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
pub(crate) fn resolve_auto_track_list(value: &CssValue, ctx: &ResolveContext) -> Vec<TrackSize> {
    match value {
        CssValue::List(items) => items.iter().map(|v| resolve_single_track(v, ctx)).collect(),
        _ => vec![resolve_single_track(value, ctx)],
    }
}

/// Resolve a grid line value.
///
/// Non-finite values (NaN/Infinity) resolve to `Auto`.
/// Line numbers are clamped to [-10000, 10000].
pub(crate) fn resolve_grid_line(value: &CssValue) -> GridLine {
    match value {
        CssValue::Number(n) if n.is_finite() => {
            #[allow(clippy::cast_possible_truncation)]
            let i = (*n as i32).clamp(-10000, 10000);
            if i == 0 {
                GridLine::Auto
            } else {
                GridLine::Line(i)
            }
        }
        // Named ident: Keyword(ident)
        CssValue::Keyword(ident) if ident != "none" && ident != "auto" => {
            GridLine::Named(ident.clone())
        }
        CssValue::List(items) => {
            // span <integer>
            if items.first() == Some(&CssValue::Keyword("span".to_string())) {
                if let Some(CssValue::Number(n)) = items.get(1) {
                    if n.is_finite() && *n >= 1.0 {
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        let span = (*n as u32).min(10000);
                        return GridLine::Span(span);
                    }
                    return GridLine::Auto;
                }
            }
            // span-named: [Keyword("span-named"), Number(n), Keyword(ident)]
            if items.first() == Some(&CssValue::Keyword("span-named".to_string())) {
                if let (Some(CssValue::Number(n)), Some(CssValue::Keyword(ident))) =
                    (items.get(1), items.get(2))
                {
                    if n.is_finite() && *n >= 1.0 {
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        let span = (*n as u32).min(10000);
                        return GridLine::SpanNamed(ident.clone(), span);
                    }
                    return GridLine::Auto;
                }
            }
            // Named with index: [Number(n), Keyword(ident)] or [Keyword(ident), Number(n)]
            if items.len() == 2 {
                match (&items[0], &items[1]) {
                    (CssValue::Number(n), CssValue::Keyword(ident))
                    | (CssValue::Keyword(ident), CssValue::Number(n))
                        if n.is_finite() =>
                    {
                        #[allow(clippy::cast_possible_truncation)]
                        let idx = (*n as i32).clamp(-10000, 10000);
                        if idx != 0 {
                            return GridLine::NamedWithIndex(ident.clone(), idx);
                        }
                    }
                    _ => {}
                }
            }
            GridLine::Auto
        }
        _ => GridLine::Auto,
    }
}

/// Resolve a `CssValue` to `GridTemplateAreas`.
pub(crate) fn resolve_template_areas(value: &CssValue) -> GridTemplateAreas {
    match value {
        CssValue::Keyword(k) if k == "none" => GridTemplateAreas::default(),
        CssValue::List(rows) => {
            let areas: Vec<Vec<String>> = rows
                .iter()
                .filter_map(|row| match row {
                    CssValue::List(cells) => Some(
                        cells
                            .iter()
                            .filter_map(|c| c.as_keyword().map(String::from))
                            .collect(),
                    ),
                    _ => None,
                })
                .collect();
            GridTemplateAreas { areas }
        }
        _ => GridTemplateAreas::default(),
    }
}
