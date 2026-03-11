//! CSS Grid keyword enums and types.

use std::fmt;

keyword_enum! {
    /// The CSS `grid-auto-flow` property.
    GridAutoFlow {
        Row => "row",
        Column => "column",
        RowDense => "row dense",
        ColumnDense => "column dense",
    }
}

/// A single track sizing function for CSS Grid.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum TrackSize {
    /// A fixed length in pixels.
    Length(f32),
    /// A percentage of the grid container's size.
    Percentage(f32),
    /// A flexible length (`fr` unit).
    Fr(f32),
    /// `auto` — sized by content.
    #[default]
    Auto,
    /// `minmax(min, max)` function.
    MinMax(Box<TrackBreadth>, Box<TrackBreadth>),
}

/// A track breadth value, used inside `minmax()`.
#[derive(Clone, Debug, PartialEq)]
pub enum TrackBreadth {
    /// A fixed length in pixels.
    Length(f32),
    /// A percentage of the grid container's size.
    Percentage(f32),
    /// A flexible length (`fr` unit).
    Fr(f32),
    /// `auto` — sized by content.
    Auto,
    /// `min-content` intrinsic size.
    MinContent,
    /// `max-content` intrinsic size.
    MaxContent,
}

/// A grid line placement value.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum GridLine {
    /// `auto` — automatic placement.
    #[default]
    Auto,
    /// An explicit line number (1-based, can be negative).
    Line(i32),
    /// `span N` — span across N tracks.
    Span(u32),
}
