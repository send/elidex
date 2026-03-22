//! Serialization helpers for CSS grid computed values.

use elidex_plugin::{
    AutoRepeatMode, CssValue, GridLine, GridTemplateAreas, GridTrackList, LengthUnit, TrackBreadth,
    TrackSection, TrackSize,
};

/// Serialize a `GridTrackList` back to `CssValue`.
pub(crate) fn track_list_to_css(list: &GridTrackList) -> CssValue {
    match list {
        GridTrackList::Explicit(section) => section_to_css(section),
        GridTrackList::Subgrid { line_names } => {
            let mut items = vec![CssValue::Keyword("subgrid".to_string())];
            for names in line_names {
                items.push(CssValue::List(
                    names.iter().map(|s| CssValue::Keyword(s.clone())).collect(),
                ));
            }
            CssValue::List(items)
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
                section_to_css_inner(before),
                section_to_css_inner(pattern),
                section_to_css_inner(after),
            ])
        }
    }
}

/// Serialize a `TrackSection` to a `CssValue`.
fn section_to_css(section: &TrackSection) -> CssValue {
    if section.tracks.is_empty() {
        return CssValue::Keyword("none".to_string());
    }
    let has_names = section.line_names.iter().any(|names| !names.is_empty());
    if !has_names {
        if section.tracks.len() == 1 {
            return track_size_to_css(&section.tracks[0]);
        }
        return CssValue::List(section.tracks.iter().map(track_size_to_css).collect());
    }
    // Encode with named-tracks marker
    let mut items = vec![CssValue::Keyword("named-tracks".to_string())];
    for (i, track) in section.tracks.iter().enumerate() {
        let names = section.line_names.get(i).cloned().unwrap_or_default();
        items.push(CssValue::List(
            names.into_iter().map(CssValue::Keyword).collect(),
        ));
        items.push(track_size_to_css(track));
    }
    // Trailing line names
    let trailing = section
        .line_names
        .get(section.tracks.len())
        .cloned()
        .unwrap_or_default();
    items.push(CssValue::List(
        trailing.into_iter().map(CssValue::Keyword).collect(),
    ));
    CssValue::List(items)
}

/// Serialize a `TrackSection` for use inside auto-repeat `CssValue` list.
fn section_to_css_inner(section: &TrackSection) -> CssValue {
    let has_names = section.line_names.iter().any(|names| !names.is_empty());
    if !has_names {
        return CssValue::List(section.tracks.iter().map(track_size_to_css).collect());
    }
    // Encode with named-tracks marker
    let mut items = vec![CssValue::Keyword("named-tracks".to_string())];
    for (i, track) in section.tracks.iter().enumerate() {
        let names = section.line_names.get(i).cloned().unwrap_or_default();
        items.push(CssValue::List(
            names.into_iter().map(CssValue::Keyword).collect(),
        ));
        items.push(track_size_to_css(track));
    }
    let trailing = section
        .line_names
        .get(section.tracks.len())
        .cloned()
        .unwrap_or_default();
    items.push(CssValue::List(
        trailing.into_iter().map(CssValue::Keyword).collect(),
    ));
    CssValue::List(items)
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

/// Serialize `GridTemplateAreas` to `CssValue`.
pub(crate) fn areas_to_css(areas: &GridTemplateAreas) -> CssValue {
    if areas.is_none() {
        return CssValue::Keyword("none".to_string());
    }
    CssValue::List(
        areas
            .areas
            .iter()
            .map(|row| CssValue::List(row.iter().map(|s| CssValue::Keyword(s.clone())).collect()))
            .collect(),
    )
}

/// Serialize a `Vec<TrackSize>` to `CssValue` for grid-auto-columns/rows.
pub(crate) fn auto_track_list_to_css(tracks: &[TrackSize]) -> CssValue {
    if tracks.len() == 1 {
        return track_size_to_css(&tracks[0]);
    }
    CssValue::List(tracks.iter().map(track_size_to_css).collect())
}

/// Serialize a `GridLine` to `CssValue`.
pub(crate) fn grid_line_to_css(gl: &GridLine) -> CssValue {
    match gl {
        GridLine::Auto => CssValue::Auto,
        #[allow(clippy::cast_precision_loss)]
        GridLine::Line(n) => CssValue::Number(*n as f32),
        #[allow(clippy::cast_precision_loss)]
        GridLine::Span(n) => CssValue::List(vec![
            CssValue::Keyword("span".to_string()),
            CssValue::Number(*n as f32),
        ]),
        GridLine::Named(ident) => CssValue::Keyword(ident.clone()),
        #[allow(clippy::cast_precision_loss)]
        GridLine::NamedWithIndex(ident, n) => CssValue::List(vec![
            CssValue::Keyword(ident.clone()),
            CssValue::Number(*n as f32),
        ]),
        #[allow(clippy::cast_precision_loss)]
        GridLine::SpanNamed(ident, n) => CssValue::List(vec![
            CssValue::Keyword("span-named".to_string()),
            CssValue::Number(*n as f32),
            CssValue::Keyword(ident.clone()),
        ]),
    }
}
