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
    /// `fit-content(<length>)` — clamped max-content (CSS Grid §7.2.4).
    /// The argument is the resolved limit in pixels.
    FitContent(f32),
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
    /// `fit-content(<length>)` — resolved limit in pixels.
    FitContent(f32),
}

/// How an auto-repeat track should fill (CSS Grid Level 1 §7.2.3.2).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AutoRepeatMode {
    /// `auto-fill`: fill as many tracks as fit; empty tracks remain.
    AutoFill,
    /// `auto-fit`: fill as many tracks as fit; empty tracks collapse to 0.
    AutoFit,
}

/// A section of tracks paired with their boundary line names.
///
/// `line_names[i]` contains the names for the line *before* `tracks[i]`.
/// `line_names[tracks.len()]` contains the names for the line *after* the last track.
/// Length invariant: `line_names.len() == tracks.len() + 1` (when non-empty).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrackSection {
    /// Track sizing functions.
    pub tracks: Vec<TrackSize>,
    /// Named lines at each boundary. `line_names[i]` = names before track `i`.
    /// Empty when no named lines are used.
    pub line_names: Vec<Vec<String>>,
}

impl TrackSection {
    /// Create from tracks only (no named lines).
    #[must_use]
    pub fn from_tracks(tracks: Vec<TrackSize>) -> Self {
        let n = tracks.len();
        Self {
            tracks,
            line_names: vec![vec![]; n + 1],
        }
    }
}

/// A grid track list that may contain an auto-repeat section.
///
/// CSS Grid Level 1 §7.2.3.2 allows at most one `repeat(auto-fill/auto-fit, ...)`
/// per track list. Tracks before/after the auto-repeat are explicit fixed tracks.
#[derive(Clone, Debug, PartialEq)]
pub enum GridTrackList {
    /// All tracks are explicit (no auto-repeat).
    Explicit(TrackSection),
    /// Contains one auto-repeat section.
    AutoRepeat {
        /// Tracks before the auto-repeat.
        before: TrackSection,
        /// The pattern to repeat.
        pattern: TrackSection,
        /// `auto-fill` or `auto-fit`.
        mode: AutoRepeatMode,
        /// Tracks after the auto-repeat.
        after: TrackSection,
    },
}

impl Default for GridTrackList {
    fn default() -> Self {
        Self::Explicit(TrackSection::default())
    }
}

impl GridTrackList {
    /// Returns `true` if the track list is empty (equivalent to `none`).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Explicit(s) if s.tracks.is_empty())
    }

    /// Returns the number of explicit tracks (without auto-repeat expansion).
    ///
    /// For `AutoRepeat`, counts `before.len() + pattern.len() + after.len()`
    /// as a minimum (one repetition).
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Explicit(s) => s.tracks.len(),
            Self::AutoRepeat {
                before,
                pattern,
                after,
                ..
            } => before.tracks.len() + pattern.tracks.len() + after.tracks.len(),
        }
    }

    /// Expand the track list into a flat `Vec<TrackSize>` given the available
    /// space on this axis. For `Explicit`, returns the tracks as-is.
    /// For `AutoRepeat`, computes the repeat count from available space.
    #[must_use]
    pub fn expand(&self, available: f32, gap: f32) -> Vec<TrackSize> {
        self.expand_with_names(available, gap).tracks
    }

    /// Expand the track list into a `TrackSection` with both tracks and line names.
    ///
    /// For `AutoRepeat`, repetition boundary line names are merged:
    /// e.g. `repeat(auto-fill, [a] 200px [b])` × 3 → `[a], [b a], [b a], [b]`.
    #[must_use]
    pub fn expand_with_names(&self, available: f32, gap: f32) -> TrackSection {
        match self {
            Self::Explicit(s) => s.clone(),
            Self::AutoRepeat {
                before,
                pattern,
                after,
                mode: _,
            } => {
                let repeat_count = compute_auto_repeat_count(
                    &before.tracks,
                    &pattern.tracks,
                    &after.tracks,
                    available,
                    gap,
                );
                let total_tracks =
                    before.tracks.len() + pattern.tracks.len() * repeat_count + after.tracks.len();
                let mut tracks = Vec::with_capacity(total_tracks);
                let mut line_names: Vec<Vec<String>> = Vec::with_capacity(total_tracks + 1);

                // Before section
                append_section_names(&mut tracks, &mut line_names, before);

                // Pattern × repeat_count (merge boundary names)
                for _ in 0..repeat_count {
                    append_section_names(&mut tracks, &mut line_names, pattern);
                }

                // After section
                append_section_names(&mut tracks, &mut line_names, after);

                // Ensure trailing line_names entry
                if line_names.len() == tracks.len() {
                    line_names.push(vec![]);
                }

                TrackSection { tracks, line_names }
            }
        }
    }

    /// Whether this is an `auto-fit` repeat (empty tracks should collapse).
    #[must_use]
    pub fn is_auto_fit(&self) -> bool {
        matches!(
            self,
            Self::AutoRepeat {
                mode: AutoRepeatMode::AutoFit,
                ..
            }
        )
    }

    /// Returns the range of track indices that belong to the auto-repeat section
    /// after expansion. `None` for `Explicit`.
    #[must_use]
    pub fn auto_repeat_range(&self, available: f32, gap: f32) -> Option<std::ops::Range<usize>> {
        match self {
            Self::Explicit(_) => None,
            Self::AutoRepeat {
                before,
                pattern,
                after,
                mode: _,
            } => {
                let repeat_count = compute_auto_repeat_count(
                    &before.tracks,
                    &pattern.tracks,
                    &after.tracks,
                    available,
                    gap,
                );
                let start = before.tracks.len();
                let end = start + pattern.tracks.len() * repeat_count;
                Some(start..end)
            }
        }
    }
}

/// Append a section's tracks and merge its line names into the running lists.
///
/// When the running `line_names` already has a trailing entry (from a previous
/// section boundary) and the new section has a leading entry, they are merged.
fn append_section_names(
    tracks: &mut Vec<TrackSize>,
    line_names: &mut Vec<Vec<String>>,
    section: &TrackSection,
) {
    if section.tracks.is_empty() && section.line_names.is_empty() {
        return;
    }
    tracks.extend_from_slice(&section.tracks);

    if section.line_names.is_empty() {
        // No line names in section — just pad with empty vecs.
        while line_names.len() < tracks.len() + 1 {
            line_names.push(vec![]);
        }
        return;
    }

    for (i, names) in section.line_names.iter().enumerate() {
        if i == 0 && !line_names.is_empty() {
            // Merge with the trailing entry from previous section.
            let last = line_names.last_mut().expect("non-empty");
            last.extend(names.iter().cloned());
        } else {
            line_names.push(names.clone());
        }
    }
}

/// Compute the number of auto-repeat repetitions that fit in the available space.
///
/// CSS Grid Level 1 §7.2.3.2: The repeat count is the largest integer N such that
/// `fixed_space + N * pattern_space + (total_tracks - 1) * gap <= available`
/// where `total_tracks = before.len() + N * pattern.len() + after.len()`.
/// Minimum 1 repetition.
fn compute_auto_repeat_count(
    before: &[TrackSize],
    pattern: &[TrackSize],
    after: &[TrackSize],
    available: f32,
    gap: f32,
) -> usize {
    if pattern.is_empty() {
        return 1;
    }

    let fixed_space: f32 = before
        .iter()
        .chain(after.iter())
        .map(|ts| track_size_fixed_contribution(ts, available))
        .sum();
    let pattern_space: f32 = pattern
        .iter()
        .map(|ts| track_size_fixed_contribution(ts, available))
        .sum();

    if pattern_space <= 0.0 {
        return 1;
    }

    // Constraint: fixed_space + N * pattern_space + (B + N*P + A - 1) * gap <= available
    //   B = before.len(), P = pattern.len(), A = after.len()
    // Rearranging: N * (pattern_space + P * gap) <= available - fixed_space - (B + A - 1) * gap
    let b = before.len();
    let a = after.len();
    // (B + A - 1) can be negative (when B=A=0), which is correct: N*P tracks
    // have (N*P - 1) gaps, and per_repetition includes P gaps per repetition,
    // so the -1*gap compensates for the overcounting.
    #[allow(clippy::cast_precision_loss)]
    let base_gap = if b + a > 0 {
        (b + a - 1) as f32 * gap
    } else {
        // No fixed tracks: total gaps = (N*P - 1), handled by per_repetition
        // minus the overcounting gap.
        -gap
    };

    let remaining = available - fixed_space - base_gap;
    if remaining <= 0.0 {
        return 1;
    }

    #[allow(clippy::cast_precision_loss)]
    let per_repetition = pattern_space + pattern.len() as f32 * gap;
    if per_repetition <= 0.0 {
        return 1;
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count = (remaining / per_repetition).floor() as usize;
    count.clamp(1, 10_000)
}

/// Get the fixed pixel contribution of a track size for auto-repeat counting.
///
/// `Length` contributes its pixel value. `Percentage` resolves against the
/// available size. `MinMax` uses the max breadth if definite, else the min
/// breadth if definite (CSS Grid §7.2.3.2). `Fr` and `Auto` contribute 0.
fn track_size_fixed_contribution(ts: &TrackSize, available: f32) -> f32 {
    match ts {
        TrackSize::Length(px) | TrackSize::FitContent(px) => *px,
        TrackSize::Percentage(pct) => pct / 100.0 * available,
        TrackSize::MinMax(min, max) => {
            // Use max if definite length/percentage, else min if definite, else 0.
            match **max {
                TrackBreadth::Length(px) => px,
                TrackBreadth::Percentage(pct) => pct / 100.0 * available,
                _ => match **min {
                    TrackBreadth::Length(px) => px,
                    TrackBreadth::Percentage(pct) => pct / 100.0 * available,
                    _ => 0.0,
                },
            }
        }
        TrackSize::Fr(_) | TrackSize::Auto => 0.0,
    }
}

/// Parsed `grid-template-areas` value (CSS Grid §8.2).
///
/// Row-major 2D grid of area names. `"."` = null cell token.
/// An empty `areas` vec means `none` (no named areas).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GridTemplateAreas {
    /// Row-major 2D grid of area names.
    pub areas: Vec<Vec<String>>,
}

impl GridTemplateAreas {
    /// Number of rows defined by this template.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.areas.len()
    }

    /// Number of columns defined by this template.
    #[must_use]
    pub fn columns(&self) -> usize {
        self.areas.first().map_or(0, Vec::len)
    }

    /// Whether this is `none` (no areas defined).
    #[must_use]
    pub fn is_none(&self) -> bool {
        self.areas.is_empty()
    }
}

/// Validate that every named area in a grid-template-areas grid forms a
/// contiguous rectangle (CSS Grid §8.2).
///
/// `rows` is the row-major 2D grid of area names. `"."` cells (null) are skipped.
/// Returns `false` if any named area occupies a non-rectangular region.
#[must_use]
pub fn validate_area_rectangles(rows: &[Vec<String>]) -> bool {
    use std::collections::HashMap;

    let mut bounds: HashMap<&str, (usize, usize, usize, usize)> = HashMap::new();
    for (r, row) in rows.iter().enumerate() {
        for (c, name) in row.iter().enumerate() {
            if name == "." {
                continue;
            }
            let e = bounds.entry(name.as_str()).or_insert((r, c, r, c));
            e.0 = e.0.min(r);
            e.1 = e.1.min(c);
            e.2 = e.2.max(r);
            e.3 = e.3.max(c);
        }
    }

    for (name, &(min_r, min_c, max_r, max_c)) in &bounds {
        for row in rows.iter().take(max_r + 1).skip(min_r) {
            for cell in row.iter().take(max_c + 1).skip(min_c) {
                if cell != name {
                    return false;
                }
            }
        }
    }
    true
}

keyword_enum! {
    /// The CSS `justify-items` property (CSS Box Alignment Level 3 §6.1).
    JustifyItems {
        Stretch => "stretch",
        Start => "start",
        End => "end",
        Center => "center",
        Baseline => "baseline",
    }
}

keyword_enum! {
    /// The CSS `justify-self` property (CSS Box Alignment Level 3 §6.2).
    JustifySelf {
        Auto => "auto",
        Start => "start",
        End => "end",
        Center => "center",
        Stretch => "stretch",
        Baseline => "baseline",
    }
}

/// A grid line placement value (CSS Grid §8.1).
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub enum GridLine {
    /// `auto` — automatic placement.
    #[default]
    Auto,
    /// An explicit line number (1-based, can be negative).
    Line(i32),
    /// `span N` — span across N tracks.
    Span(u32),
    /// `<custom-ident>` — named line reference.
    Named(String),
    /// `<custom-ident> <integer>` — nth occurrence of named line.
    NamedWithIndex(String, i32),
    /// `span <custom-ident>` or `span <integer> <custom-ident>` — span to named line.
    SpanNamed(String, u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------------------
    // GridLine equality tests
    // ---------------------------------------------------------------------------

    #[test]
    fn grid_line_named_eq() {
        assert_eq!(
            GridLine::Named("header".into()),
            GridLine::Named("header".into())
        );
        assert_ne!(
            GridLine::Named("header".into()),
            GridLine::Named("footer".into())
        );
        assert_ne!(GridLine::Named("header".into()), GridLine::Auto);
    }

    #[test]
    fn grid_line_named_with_index_eq() {
        assert_eq!(
            GridLine::NamedWithIndex("a".into(), 2),
            GridLine::NamedWithIndex("a".into(), 2)
        );
        assert_ne!(
            GridLine::NamedWithIndex("a".into(), 1),
            GridLine::NamedWithIndex("a".into(), 2)
        );
        assert_ne!(
            GridLine::NamedWithIndex("a".into(), 1),
            GridLine::NamedWithIndex("b".into(), 1)
        );
    }

    #[test]
    fn grid_line_span_named_eq() {
        assert_eq!(
            GridLine::SpanNamed("sidebar".into(), 1),
            GridLine::SpanNamed("sidebar".into(), 1)
        );
        assert_ne!(
            GridLine::SpanNamed("sidebar".into(), 1),
            GridLine::SpanNamed("sidebar".into(), 2)
        );
        assert_ne!(
            GridLine::SpanNamed("a".into(), 1),
            GridLine::SpanNamed("b".into(), 1)
        );
    }

    // ---------------------------------------------------------------------------
    // TrackSection tests
    // ---------------------------------------------------------------------------

    #[test]
    fn track_section_from_tracks() {
        let tracks = vec![TrackSize::Length(100.0), TrackSize::Fr(1.0)];
        let section = TrackSection::from_tracks(tracks.clone());
        assert_eq!(section.tracks.len(), 2);
        // line_names.len() == tracks.len() + 1
        assert_eq!(
            section.line_names.len(),
            section.tracks.len() + 1,
            "line_names should have tracks.len() + 1 entries"
        );
        // All entries should be empty (no named lines)
        for names in &section.line_names {
            assert!(
                names.is_empty(),
                "line_names should be empty for from_tracks"
            );
        }
    }

    // ---------------------------------------------------------------------------
    // GridTemplateAreas tests
    // ---------------------------------------------------------------------------

    #[test]
    fn grid_template_areas_basic() {
        let areas = GridTemplateAreas {
            areas: vec![
                vec!["header".to_string(), "header".to_string()],
                vec!["main".to_string(), "sidebar".to_string()],
                vec!["footer".to_string(), "footer".to_string()],
            ],
        };
        assert_eq!(areas.rows(), 3, "should have 3 rows");
        assert_eq!(areas.columns(), 2, "should have 2 columns");
        assert!(!areas.is_none(), "should not be none");

        let empty = GridTemplateAreas::default();
        assert!(empty.is_none(), "default should be none");
        assert_eq!(empty.rows(), 0);
        assert_eq!(empty.columns(), 0);
    }

    // ---------------------------------------------------------------------------
    // GridTrackList::expand_with_names tests
    // ---------------------------------------------------------------------------

    #[test]
    fn grid_track_list_expand_with_names() {
        // Explicit track list with named lines
        let section = TrackSection {
            tracks: vec![TrackSize::Length(100.0), TrackSize::Fr(1.0)],
            line_names: vec![
                vec!["start".to_string()],
                vec!["mid".to_string()],
                vec!["end".to_string()],
            ],
        };
        let track_list = GridTrackList::Explicit(section);
        // expand_with_names with a large available space
        let expanded = track_list.expand_with_names(1000.0, 0.0);
        assert_eq!(expanded.tracks.len(), 2, "should have 2 expanded tracks");
        assert_eq!(
            expanded.line_names.len(),
            expanded.tracks.len() + 1,
            "expanded line_names should have tracks + 1 entries"
        );
        assert_eq!(expanded.line_names[0], vec!["start".to_string()]);
        assert_eq!(expanded.line_names[1], vec!["mid".to_string()]);
        assert_eq!(expanded.line_names[2], vec!["end".to_string()]);
    }

    // ---------------------------------------------------------------------------
    // validate_area_rectangles tests
    // ---------------------------------------------------------------------------

    #[test]
    fn validate_area_rectangles_valid() {
        let rows = vec![vec!["a".into(), "a".into()], vec!["b".into(), "c".into()]];
        assert!(validate_area_rectangles(&rows));
    }

    #[test]
    fn validate_area_rectangles_non_rectangular() {
        // "a" forms an L-shape → invalid
        let rows = vec![vec!["a".into(), "a".into()], vec!["a".into(), "b".into()]];
        assert!(!validate_area_rectangles(&rows));
    }

    #[test]
    fn validate_area_rectangles_null_cells() {
        let rows = vec![vec!["a".into(), ".".into()], vec![".".into(), "b".into()]];
        assert!(validate_area_rectangles(&rows));
    }

    #[test]
    fn validate_area_rectangles_empty() {
        let rows: Vec<Vec<String>> = vec![];
        assert!(validate_area_rectangles(&rows));
    }
}
