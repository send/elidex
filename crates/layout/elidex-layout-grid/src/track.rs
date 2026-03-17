//! Grid track sizing algorithm.
//!
//! Resolves track sizes from `TrackSize` definitions (explicit or implicit),
//! including `fr` unit distribution, `minmax()`, and percentage resolution.

use elidex_plugin::TrackSize;

// ---------------------------------------------------------------------------
// ResolvedTrack
// ---------------------------------------------------------------------------

/// A resolved track with its final pixel size.
#[derive(Clone, Debug)]
pub(crate) struct ResolvedTrack {
    /// Base size — the minimum the track should be.
    pub(crate) base: f32,
    /// Growth limit — the maximum the track can grow to.
    pub(crate) limit: f32,
    /// Whether this track uses an `fr` unit (for later flex distribution).
    pub(crate) fr: f32,
    /// Final resolved size in pixels.
    pub(crate) size: f32,
    /// Whether this track was collapsed by auto-fit (CSS Grid §7.2.3.2).
    /// Collapsed tracks have size 0 and their adjacent gutters are also collapsed.
    pub(crate) collapsed: bool,
}

impl ResolvedTrack {
    /// Effective size for a non-fr track: `max(base, limit)` when limit is
    /// finite, otherwise just `base`.
    fn effective_non_fr_size(&self) -> f32 {
        if self.limit < f32::INFINITY {
            self.base.max(self.limit)
        } else {
            self.base
        }
    }
}

// ---------------------------------------------------------------------------
// Track resolution
// ---------------------------------------------------------------------------

/// Resolve a list of `TrackSize` definitions to pixel sizes.
///
/// `available` is the total available space on this axis (content width or height).
/// `gap` is the gap between tracks. `min_sizes` and `max_sizes` are per-track
/// min-content and max-content intrinsic sizes respectively (CSS Grid §12.3).
pub(crate) fn resolve_tracks(
    definitions: &[TrackSize],
    available: f32,
    gap: f32,
    min_sizes: &[f32],
    max_sizes: &[f32],
) -> Vec<ResolvedTrack> {
    if definitions.is_empty() {
        return Vec::new();
    }

    let mut tracks: Vec<ResolvedTrack> = definitions
        .iter()
        .enumerate()
        .map(|(i, def)| {
            resolve_single_track(
                def,
                available,
                min_sizes.get(i).copied().unwrap_or(0.0),
                max_sizes.get(i).copied().unwrap_or(0.0),
            )
        })
        .collect();

    // Distribute fr units.
    distribute_fr(&mut tracks, available, gap);

    // Finalize: non-fr tracks get their effective size.
    // Clamp to 0.0 to prevent negative sizes from malformed CSS values.
    for track in &mut tracks {
        if track.fr == 0.0 {
            track.size = track.effective_non_fr_size();
        }
        // fr tracks already sized by distribute_fr.
        track.size = track.size.max(0.0);
    }

    tracks
}

/// Resolve a single `TrackSize` to base/limit/fr values.
///
/// `min_content` is the min-content intrinsic size (from narrow-probe layout).
/// `max_content` is the max-content intrinsic size (from full-width layout).
fn resolve_single_track(
    def: &TrackSize,
    available: f32,
    min_content: f32,
    max_content: f32,
) -> ResolvedTrack {
    match def {
        TrackSize::Length(px) => ResolvedTrack {
            base: *px,
            limit: *px,
            fr: 0.0,
            size: *px,
            collapsed: false,
        },
        TrackSize::Percentage(pct) => {
            let resolved = available * pct / 100.0;
            ResolvedTrack {
                base: resolved,
                limit: resolved,
                fr: 0.0,
                size: resolved,
                collapsed: false,
            }
        }
        TrackSize::Fr(f) => ResolvedTrack {
            base: 0.0,
            limit: f32::INFINITY,
            fr: *f,
            size: 0.0, // Will be set by distribute_fr.
            collapsed: false,
        },
        TrackSize::Auto => ResolvedTrack {
            base: min_content,
            limit: max_content.max(0.0),
            fr: 0.0,
            size: max_content,
            collapsed: false,
        },
        TrackSize::MinMax(min_breadth, max_breadth) => {
            let min_val = resolve_breadth_as_min(min_breadth, available, min_content, max_content);
            let max_val = resolve_breadth_as_max(max_breadth, available, min_content, max_content);
            let fr_val = match **max_breadth {
                elidex_plugin::TrackBreadth::Fr(f) => f,
                _ => 0.0,
            };
            ResolvedTrack {
                base: min_val,
                limit: max_val,
                fr: fr_val,
                size: if fr_val > 0.0 {
                    0.0
                } else {
                    min_val.max(max_val.min(min_val.max(max_content)))
                },
                collapsed: false,
            }
        }
    }
}

/// Resolve a `TrackBreadth` as a minimum value (CSS Grid §12.3).
///
/// `min_content` is the min-content intrinsic size.
/// `max_content` is the max-content intrinsic size.
fn resolve_breadth_as_min(
    breadth: &elidex_plugin::TrackBreadth,
    available: f32,
    min_content: f32,
    max_content: f32,
) -> f32 {
    use elidex_plugin::TrackBreadth;
    match breadth {
        TrackBreadth::Length(px) => *px,
        TrackBreadth::Percentage(pct) => available * pct / 100.0,
        // CSS Grid §7.2.1: `<flex>` in the min position of `minmax()` is treated
        // as `auto` (resolved to min-content).
        TrackBreadth::Auto | TrackBreadth::MinContent | TrackBreadth::Fr(_) => min_content,
        TrackBreadth::MaxContent => max_content,
    }
}

/// Resolve a `TrackBreadth` as a maximum value (CSS Grid §12.3).
///
/// `min_content` is the min-content intrinsic size.
/// `max_content` is the max-content intrinsic size.
fn resolve_breadth_as_max(
    breadth: &elidex_plugin::TrackBreadth,
    available: f32,
    min_content: f32,
    max_content: f32,
) -> f32 {
    use elidex_plugin::TrackBreadth;
    match breadth {
        TrackBreadth::Length(px) => *px,
        TrackBreadth::Percentage(pct) => available * pct / 100.0,
        TrackBreadth::Auto | TrackBreadth::MaxContent => max_content.max(0.0),
        TrackBreadth::MinContent => min_content.max(0.0),
        TrackBreadth::Fr(f) => {
            // fr as max — size is determined by fr distribution.
            // Return infinity to signal this.
            if *f > 0.0 {
                f32::INFINITY
            } else {
                0.0
            }
        }
    }
}

/// Distribute remaining space among `fr` tracks.
///
/// Implements CSS Grid §12.7.1 "Finding the Size of an fr":
/// 1. Sum non-fr track sizes + gaps.
/// 2. Remaining = available - `non_fr_sum`.
/// 3. Sum all fr values. If sum < 1, clamp to 1 (§12.7.1 step 2).
/// 4. Each fr track gets: (remaining / `effective_fr`) * `fr_value`.
/// 5. If a track's fr-size < base, freeze it at base and redistribute.
#[allow(clippy::cast_precision_loss)]
fn distribute_fr(tracks: &mut [ResolvedTrack], available: f32, gap: f32) {
    let total_gap = if tracks.len() > 1 {
        gap * (tracks.len() - 1) as f32
    } else {
        0.0
    };

    // Calculate space used by non-fr tracks.
    let non_fr_sum: f32 = tracks
        .iter()
        .filter(|t| t.fr == 0.0)
        .map(ResolvedTrack::effective_non_fr_size)
        .sum();

    let remaining = (available - non_fr_sum - total_gap).max(0.0);

    let total_fr: f32 = tracks.iter().map(|t| t.fr).sum();
    if total_fr <= 0.0 {
        return;
    }

    // CSS Grid §12.7.1 step 2: if flex factor sum < 1, clamp to 1.
    // This prevents fractional fr values from consuming all available space.
    let effective_fr = total_fr.max(1.0);

    // CSS Grid §12.7.1: iterative freeze loop.
    // Tracks whose hypothetical size would be less than their base size are
    // frozen at their base; remaining space is redistributed among unfrozen
    // tracks until no more tracks need freezing.
    let mut frozen = vec![false; tracks.len()];
    let mut remaining_space = remaining;
    let mut remaining_fr = effective_fr;

    loop {
        let mut newly_frozen = false;
        for (i, track) in tracks.iter().enumerate() {
            if frozen[i] || track.fr <= 0.0 {
                continue;
            }
            let hypothetical = remaining_space * track.fr / remaining_fr;
            if hypothetical < track.base {
                frozen[i] = true;
                remaining_space -= track.base;
                remaining_fr -= track.fr;
                newly_frozen = true;
            }
        }
        if !newly_frozen {
            break;
        }
    }

    remaining_space = remaining_space.max(0.0);

    // Distribute remaining space among unfrozen tracks.
    for (i, track) in tracks.iter_mut().enumerate() {
        if track.fr > 0.0 {
            if frozen[i] {
                track.size = track.base;
            } else if remaining_fr > 0.0 {
                track.size = remaining_space * track.fr / remaining_fr;
                track.size = track.size.max(track.base);
            } else {
                track.size = track.base;
            }
        }
    }
}

/// Compute track positions (cumulative offsets from the content edge).
///
/// Returns a vector of starting positions for each track.
/// Gaps adjacent to collapsed tracks (from auto-fit) are skipped per CSS Grid §7.2.3.2.
pub(crate) fn compute_track_positions(tracks: &[ResolvedTrack], gap: f32) -> Vec<f32> {
    let mut positions = Vec::with_capacity(tracks.len());
    let mut offset = 0.0;
    for (i, track) in tracks.iter().enumerate() {
        positions.push(offset);
        offset += track.size;
        // Only add a gap between two non-collapsed adjacent tracks.
        if i + 1 < tracks.len() && !track.collapsed && !tracks[i + 1].collapsed {
            offset += gap;
        }
    }
    positions
}

/// Get the total size of all tracks plus gaps.
///
/// Gaps adjacent to collapsed tracks are excluded per CSS Grid §7.2.3.2.
pub(crate) fn total_track_size(tracks: &[ResolvedTrack], gap: f32) -> f32 {
    if tracks.is_empty() {
        return 0.0;
    }
    let track_sum: f32 = tracks.iter().map(|t| t.size).sum();
    // Count gaps only between pairs of adjacent non-collapsed tracks.
    let gap_count = tracks
        .windows(2)
        .filter(|pair| !pair[0].collapsed && !pair[1].collapsed)
        .count();
    #[allow(clippy::cast_precision_loss)]
    let gap_sum = gap * gap_count as f32;
    track_sum + gap_sum
}
