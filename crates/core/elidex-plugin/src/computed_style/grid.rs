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

/// A grid track list that may contain an auto-repeat section.
///
/// CSS Grid Level 1 §7.2.3.2 allows at most one `repeat(auto-fill/auto-fit, ...)`
/// per track list. Tracks before/after the auto-repeat are explicit fixed tracks.
#[derive(Clone, Debug, PartialEq)]
pub enum GridTrackList {
    /// All tracks are explicit (no auto-repeat).
    Explicit(Vec<TrackSize>),
    /// Contains one auto-repeat section.
    AutoRepeat {
        /// Tracks before the auto-repeat.
        before: Vec<TrackSize>,
        /// The pattern to repeat.
        pattern: Vec<TrackSize>,
        /// `auto-fill` or `auto-fit`.
        mode: AutoRepeatMode,
        /// Tracks after the auto-repeat.
        after: Vec<TrackSize>,
    },
}

impl Default for GridTrackList {
    fn default() -> Self {
        Self::Explicit(Vec::new())
    }
}

impl GridTrackList {
    /// Returns `true` if the track list is empty (equivalent to `none`).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Explicit(v) if v.is_empty())
    }

    /// Returns the number of explicit tracks (without auto-repeat expansion).
    ///
    /// For `AutoRepeat`, counts `before.len() + pattern.len() + after.len()`
    /// as a minimum (one repetition).
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Explicit(v) => v.len(),
            Self::AutoRepeat {
                before,
                pattern,
                after,
                ..
            } => before.len() + pattern.len() + after.len(),
        }
    }

    /// Expand the track list into a flat `Vec<TrackSize>` given the available
    /// space on this axis. For `Explicit`, returns the tracks as-is.
    /// For `AutoRepeat`, computes the repeat count from available space.
    #[must_use]
    pub fn expand(&self, available: f32, gap: f32) -> Vec<TrackSize> {
        match self {
            Self::Explicit(v) => v.clone(),
            Self::AutoRepeat {
                before,
                pattern,
                after,
                mode: _,
            } => {
                let repeat_count =
                    compute_auto_repeat_count(before, pattern, after, available, gap);
                let mut result =
                    Vec::with_capacity(before.len() + pattern.len() * repeat_count + after.len());
                result.extend_from_slice(before);
                for _ in 0..repeat_count {
                    result.extend_from_slice(pattern);
                }
                result.extend_from_slice(after);
                result
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
                let repeat_count =
                    compute_auto_repeat_count(before, pattern, after, available, gap);
                let start = before.len();
                let end = start + pattern.len() * repeat_count;
                Some(start..end)
            }
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
