//! Grid track sizing algorithm.
//!
//! Resolves track sizes from `TrackSize` definitions (explicit or implicit),
//! including `fr` unit distribution, `minmax()`, and percentage resolution.
//! Implements CSS Grid §12.3-12.6 full track sizing algorithm with multi-span
//! item support.

use elidex_layout_block::total_gap;
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
    /// Whether this track has an intrinsic sizing function (Auto/MinContent/MaxContent/FitContent).
    intrinsic: bool,
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
// TrackContribution
// ---------------------------------------------------------------------------

/// A grid item's intrinsic size contribution to the track sizing algorithm.
///
/// Each item contributes min-content and max-content sizes to the tracks it spans.
/// Items are processed by span size (span=1 first, then increasing span).
#[derive(Clone, Debug)]
pub(crate) struct TrackContribution {
    /// Starting track index (0-based).
    pub(crate) start: usize,
    /// Number of tracks spanned.
    pub(crate) span: usize,
    /// Min-content contribution (narrow-probe layout).
    pub(crate) min_content: f32,
    /// Max-content contribution (full-width layout).
    pub(crate) max_content: f32,
}

// ---------------------------------------------------------------------------
// Track resolution
// ---------------------------------------------------------------------------

/// Resolve a list of `TrackSize` definitions to pixel sizes.
///
/// `available` is the total available space on this axis (content width or height).
/// `gap` is the gap between tracks. `contributions` are per-item intrinsic size
/// contributions (CSS Grid §12.3-12.6).
/// `stretch` enables Phase 4 (§12.6) auto track stretching when
/// `align-content`/`justify-content` is `normal` or `stretch`.
pub(crate) fn resolve_tracks(
    definitions: &[TrackSize],
    available: f32,
    gap: f32,
    contributions: &[TrackContribution],
    stretch: bool,
) -> Vec<ResolvedTrack> {
    if definitions.is_empty() {
        return Vec::new();
    }

    // Phase 1: Initialize tracks (§12.3).
    let mut tracks: Vec<ResolvedTrack> = definitions
        .iter()
        .map(|def| initialize_track(def, available))
        .collect();

    // Phase 2: Resolve intrinsic track sizes (§12.4-12.5).
    resolve_intrinsic_sizes(&mut tracks, contributions);

    // Apply FitContent cap: growth limit cannot exceed the fit-content limit.
    for (i, track) in tracks.iter_mut().enumerate() {
        if let Some(TrackSize::FitContent(cap)) = definitions.get(i) {
            track.limit = track.limit.min(*cap).max(track.base);
        }
    }

    // Phase 3: Maximize tracks (§12.5) — grow non-fr tracks up to their limit.
    maximize_tracks(&mut tracks, available, gap);

    // Phase 4: Stretch auto tracks (§12.6).
    if stretch {
        stretch_auto_tracks(&mut tracks, available, gap);
    }

    // Distribute fr units (§12.7).
    distribute_fr(&mut tracks, available, gap);

    // Finalize: non-fr tracks get their effective size.
    for track in &mut tracks {
        if track.fr == 0.0 {
            track.size = track.effective_non_fr_size();
        }
        track.size = track.size.max(0.0);
    }

    tracks
}

/// Phase 1: Initialize a single track from its definition.
///
/// Sets base size to the track minimum and growth limit to the track maximum.
/// Intrinsic tracks start at 0 base / infinity limit (before item contributions).
fn initialize_track(def: &TrackSize, available: f32) -> ResolvedTrack {
    match def {
        TrackSize::Length(px) => ResolvedTrack {
            base: *px,
            limit: *px,
            fr: 0.0,
            size: *px,
            collapsed: false,
            intrinsic: false,
        },
        TrackSize::Percentage(pct) => {
            let resolved = available * pct / 100.0;
            ResolvedTrack {
                base: resolved,
                limit: resolved,
                fr: 0.0,
                size: resolved,
                collapsed: false,
                intrinsic: false,
            }
        }
        TrackSize::Fr(f) => ResolvedTrack {
            base: 0.0,
            limit: f32::INFINITY,
            fr: *f,
            size: 0.0,
            collapsed: false,
            intrinsic: false,
        },
        TrackSize::Auto => ResolvedTrack {
            base: 0.0,
            limit: f32::INFINITY,
            fr: 0.0,
            size: 0.0,
            collapsed: false,
            intrinsic: true,
        },
        TrackSize::FitContent(_limit) => ResolvedTrack {
            base: 0.0,
            limit: f32::INFINITY,
            fr: 0.0,
            size: 0.0,
            collapsed: false,
            intrinsic: true,
        },
        TrackSize::MinMax(min_breadth, max_breadth) => {
            let min_val = resolve_breadth_as_min(min_breadth, available, 0.0, 0.0);
            let max_val = resolve_breadth_as_max(max_breadth, available, 0.0, 0.0);
            let fr_val = match **max_breadth {
                elidex_plugin::TrackBreadth::Fr(f) => f,
                _ => 0.0,
            };
            let intrinsic = is_intrinsic_breadth(min_breadth) || is_intrinsic_breadth(max_breadth);
            ResolvedTrack {
                base: min_val,
                limit: if fr_val > 0.0 { f32::INFINITY } else { max_val },
                fr: fr_val,
                size: 0.0,
                collapsed: false,
                intrinsic,
            }
        }
    }
}

/// Check if a track breadth is intrinsic (Auto/MinContent/MaxContent/FitContent).
fn is_intrinsic_breadth(breadth: &elidex_plugin::TrackBreadth) -> bool {
    use elidex_plugin::TrackBreadth;
    matches!(
        breadth,
        TrackBreadth::Auto
            | TrackBreadth::MinContent
            | TrackBreadth::MaxContent
            | TrackBreadth::FitContent(_)
    )
}

/// Phase 2: Resolve intrinsic track sizes from item contributions (§12.4).
///
/// Processes items in order of increasing span:
/// 1. span=1 items set base/limit directly from min/max-content contributions
/// 2. span>1 items distribute extra space across spanned intrinsic tracks
fn resolve_intrinsic_sizes(tracks: &mut [ResolvedTrack], contributions: &[TrackContribution]) {
    if contributions.is_empty() {
        return;
    }

    // Separate span=1 and span>1 contributions.
    let mut multi_span: Vec<&TrackContribution> = Vec::new();

    // Process span=1 items first.
    for contrib in contributions {
        if contrib.span == 1 {
            if contrib.start < tracks.len() {
                let track = &mut tracks[contrib.start];
                if track.intrinsic {
                    track.base = track.base.max(contrib.min_content);
                    // Set limit from max-content. For infinity limits (uninitialized),
                    // directly replace; for finite limits, take the max.
                    if track.limit == f32::INFINITY {
                        track.limit = contrib.max_content;
                    } else {
                        track.limit = track.limit.max(contrib.max_content);
                    }
                }
            }
        } else {
            multi_span.push(contrib);
        }
    }

    // §12.4 step 2.3: If a track's growth limit is infinity after span=1,
    // set it equal to its base size.
    for track in tracks.iter_mut() {
        if track.intrinsic && track.limit == f32::INFINITY && track.fr == 0.0 {
            track.limit = track.base;
        }
    }

    // Sort multi-span items by span (ascending) per §12.5.
    multi_span.sort_by_key(|c| c.span);

    // Process multi-span items.
    for contrib in &multi_span {
        let end = (contrib.start + contrib.span).min(tracks.len());
        if contrib.start >= tracks.len() || end <= contrib.start {
            continue;
        }

        // Distribute min-content contribution to base sizes.
        distribute_extra_to_bases(tracks, contrib.start, end, contrib.min_content);

        // Distribute max-content contribution to growth limits.
        distribute_extra_to_limits(tracks, contrib.start, end, contrib.max_content);
    }
}

/// Distribute extra min-content space across spanned track bases (§12.5).
///
/// Extra space = item contribution - sum of spanned track bases.
/// Distributes to intrinsic tracks only; if all tracks are fixed, skip
/// (CSS Grid §12.5: only intrinsic tracks participate in distribution).
fn distribute_extra_to_bases(
    tracks: &mut [ResolvedTrack],
    start: usize,
    end: usize,
    contribution: f32,
) {
    let current_sum: f32 = tracks[start..end].iter().map(|t| t.base).sum();
    let extra = contribution - current_sum;
    if extra <= 0.0 {
        return;
    }

    let intrinsic_count = tracks[start..end].iter().filter(|t| t.intrinsic).count();

    if intrinsic_count > 0 {
        distribute_evenly_with_freeze(tracks, start, end, extra, true, true);
    }
    // If all spanned tracks are fixed, do nothing — fixed tracks don't grow.
}

/// Distribute extra max-content space across spanned track limits (§12.5).
fn distribute_extra_to_limits(
    tracks: &mut [ResolvedTrack],
    start: usize,
    end: usize,
    contribution: f32,
) {
    let current_sum: f32 = tracks[start..end]
        .iter()
        .map(|t| {
            if t.limit < f32::INFINITY {
                t.limit
            } else {
                t.base
            }
        })
        .sum();
    let extra = contribution - current_sum;
    if extra <= 0.0 {
        return;
    }

    let intrinsic_count = tracks[start..end].iter().filter(|t| t.intrinsic).count();

    if intrinsic_count > 0 {
        distribute_evenly_with_freeze(tracks, start, end, extra, true, false);
    }
    // If all spanned tracks are fixed, do nothing — fixed tracks don't grow.
}

/// Distribute `extra` space evenly among eligible tracks with a freeze loop.
///
/// When `intrinsic_only` is true, only intrinsic tracks receive space.
/// When `to_base` is true, distributes to base; otherwise to limit.
/// Tracks that would exceed their limit (for base distribution) are frozen.
fn distribute_evenly_with_freeze(
    tracks: &mut [ResolvedTrack],
    start: usize,
    end: usize,
    extra: f32,
    intrinsic_only: bool,
    to_base: bool,
) {
    let count = end - start;
    let mut frozen = vec![false; count];
    let mut remaining = extra;

    // Each iteration freezes at least one track, so at most `count` iterations.
    for _ in 0..count {
        let eligible: usize = (0..count)
            .filter(|&i| !frozen[i] && (!intrinsic_only || tracks[start + i].intrinsic))
            .count();
        if eligible == 0 || remaining <= 0.0 {
            break;
        }

        #[allow(clippy::cast_precision_loss)]
        let share = remaining / eligible as f32;
        let mut newly_frozen = false;

        for i in 0..count {
            if frozen[i] || (intrinsic_only && !tracks[start + i].intrinsic) {
                continue;
            }
            let track = &tracks[start + i];
            if to_base && track.limit < f32::INFINITY {
                let new_base = track.base + share;
                if new_base > track.limit {
                    // Freeze at limit.
                    let added = track.limit - track.base;
                    tracks[start + i].base = track.limit;
                    remaining -= added;
                    frozen[i] = true;
                    newly_frozen = true;
                }
            }
        }

        if !newly_frozen {
            // No freezing needed — distribute evenly.
            for i in 0..count {
                if frozen[i] || (intrinsic_only && !tracks[start + i].intrinsic) {
                    continue;
                }
                if to_base {
                    tracks[start + i].base += share;
                } else if tracks[start + i].limit == f32::INFINITY {
                    tracks[start + i].limit = tracks[start + i].base + share;
                } else {
                    tracks[start + i].limit += share;
                }
            }
            break;
        }
    }
}

/// Phase 3: Maximize tracks (§12.5).
///
/// If free space is positive, grow non-fr track bases up to their growth limits.
#[allow(clippy::cast_precision_loss)]
fn maximize_tracks(tracks: &mut [ResolvedTrack], available: f32, gap: f32) {
    let total_gap = total_gap(tracks.len(), gap);

    let used: f32 = tracks.iter().map(|t| t.base).sum();
    let mut free = available - used - total_gap;
    if free <= 0.0 {
        return;
    }

    // Grow non-fr tracks that have room between base and limit.
    // Process tracks with finite limits first.
    for track in tracks.iter_mut() {
        if track.fr > 0.0 || free <= 0.0 {
            continue;
        }
        if track.limit < f32::INFINITY && track.base < track.limit {
            let grow = (track.limit - track.base).min(free);
            track.base += grow;
            free -= grow;
        }
    }

    // If there's still free space, grow tracks with infinite limits.
    if free > 0.0 {
        let infinite_count = tracks
            .iter()
            .filter(|t| t.fr == 0.0 && t.limit == f32::INFINITY)
            .count();
        if infinite_count > 0 {
            let share = free / infinite_count as f32;
            for track in tracks.iter_mut() {
                if track.fr == 0.0 && track.limit == f32::INFINITY {
                    track.base += share;
                }
            }
        }
    }
}

/// Phase 4: Stretch auto tracks (§12.6).
///
/// When `align-content`/`justify-content` is `normal` or `stretch`,
/// auto tracks are expanded equally to fill remaining space.
#[allow(clippy::cast_precision_loss)]
fn stretch_auto_tracks(tracks: &mut [ResolvedTrack], available: f32, gap: f32) {
    let total_gap = total_gap(tracks.len(), gap);

    let used: f32 = tracks
        .iter()
        .map(|t| {
            if t.fr == 0.0 {
                t.effective_non_fr_size()
            } else {
                0.0
            }
        })
        .sum();
    let free = available - used - total_gap;
    if free <= 0.0 {
        return;
    }

    let auto_count = tracks.iter().filter(|t| t.intrinsic && t.fr == 0.0).count();
    if auto_count == 0 {
        return;
    }

    let share = free / auto_count as f32;
    for track in tracks.iter_mut() {
        if track.intrinsic && track.fr == 0.0 {
            let current = track.effective_non_fr_size();
            let new_size = current + share;
            track.base = new_size;
            track.limit = new_size;
        }
    }
}

// ---------------------------------------------------------------------------
// Breadth resolution helpers
// ---------------------------------------------------------------------------

/// Resolve a `TrackBreadth` as a minimum value (CSS Grid §12.3).
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
        TrackBreadth::Auto
        | TrackBreadth::MinContent
        | TrackBreadth::Fr(_)
        | TrackBreadth::FitContent(_) => min_content,
        TrackBreadth::MaxContent => max_content,
    }
}

/// Resolve a `TrackBreadth` as a maximum value (CSS Grid §12.3).
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
            if *f > 0.0 {
                f32::INFINITY
            } else {
                0.0
            }
        }
        // CSS Grid §7.2.4: fit-content(limit) as max → min(limit, max-content).
        TrackBreadth::FitContent(limit) => limit.min(max_content).max(0.0),
    }
}

// ---------------------------------------------------------------------------
// Fr distribution
// ---------------------------------------------------------------------------

/// Distribute remaining space among `fr` tracks.
///
/// Implements CSS Grid §12.7.1 "Finding the Size of an fr":
/// 1. Sum non-fr track sizes + gaps.
/// 2. Remaining = available - `non_fr_sum`.
/// 3. Sum all fr values. If sum < 1, clamp to 1 (§12.7.1 step 2).
/// 4. Each fr track gets: (remaining / `effective_fr`) * `fr_value`.
/// 5. If a track's fr-size < base, freeze it at base and redistribute.
fn distribute_fr(tracks: &mut [ResolvedTrack], available: f32, gap: f32) {
    let total_gap = total_gap(tracks.len(), gap);

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

    let effective_fr = total_fr.max(1.0);

    let mut frozen = vec![false; tracks.len()];
    let mut remaining_space = remaining;
    let mut remaining_fr = effective_fr;

    // O(n) bound: each iteration freezes at least one track, so at most n iterations.
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

// ---------------------------------------------------------------------------
// Track positions and totals
// ---------------------------------------------------------------------------

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
    let gap_count = tracks
        .windows(2)
        .filter(|pair| !pair[0].collapsed && !pair[1].collapsed)
        .count();
    #[allow(clippy::cast_precision_loss)]
    let gap_sum = gap * gap_count as f32;
    track_sum + gap_sum
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn contrib(start: usize, span: usize, min: f32, max: f32) -> TrackContribution {
        TrackContribution {
            start,
            span,
            min_content: min,
            max_content: max,
        }
    }

    #[test]
    fn single_span_unchanged() {
        // 2 auto tracks, span=1 items
        let defs = vec![TrackSize::Auto, TrackSize::Auto];
        let contribs = vec![contrib(0, 1, 50.0, 100.0), contrib(1, 1, 30.0, 80.0)];
        let tracks = resolve_tracks(&defs, 500.0, 0.0, &contribs, false);
        assert_eq!(tracks.len(), 2);
        // After maximize, auto tracks should grow to fill available space.
        // Track 0 has base=50, limit=100. Track 1 has base=30, limit=80.
        // Maximize: free=500-80=420. Track 0 grows to 100 (free=370), Track 1 grows to 80 (free=290).
        // Then infinite limit tracks get remaining (none here).
        assert!(tracks[0].size >= 100.0);
        assert!(tracks[1].size >= 80.0);
    }

    #[test]
    fn multi_span_equal_distribution() {
        // 2 auto tracks, one span=2 item
        let defs = vec![TrackSize::Auto, TrackSize::Auto];
        let contribs = vec![contrib(0, 2, 100.0, 200.0)];
        let tracks = resolve_tracks(&defs, 500.0, 0.0, &contribs, false);
        // Each track gets 50 min-content, 100 max-content initially.
        // After maximize, they should grow.
        assert!(tracks[0].size >= 50.0);
        assert!(tracks[1].size >= 50.0);
        let total = tracks[0].size + tracks[1].size;
        assert!(total >= 100.0);
    }

    #[test]
    fn multi_span_intrinsic_only_distribution() {
        // 1 fixed track (100px) + 1 auto track, span=2 item
        let defs = vec![TrackSize::Length(100.0), TrackSize::Auto];
        let contribs = vec![contrib(0, 2, 200.0, 300.0)];
        let tracks = resolve_tracks(&defs, 500.0, 0.0, &contribs, false);
        // Fixed track stays at 100. Auto track gets extra: 200-100=100 for base.
        assert_eq!(tracks[0].size, 100.0);
        assert!(tracks[1].size >= 100.0);
    }

    #[test]
    fn multi_span_freeze_loop() {
        // 3 auto tracks with span=1 items setting limits, plus a span=3 item
        let defs = vec![TrackSize::Auto, TrackSize::Auto, TrackSize::Auto];
        let contribs = vec![
            contrib(0, 1, 10.0, 20.0),
            contrib(1, 1, 10.0, 20.0),
            contrib(2, 1, 10.0, 20.0),
            contrib(0, 3, 90.0, 150.0),
        ];
        let tracks = resolve_tracks(&defs, 500.0, 0.0, &contribs, false);
        // span=1: each track base=10, limit=20
        // span=3 min=90: extra=90-30=60, each gets 20 → base=30 each
        // But limit was set to 20 by §12.4 step 2.3, so freeze loop applies
        assert_eq!(tracks.len(), 3);
        let total_base: f32 = tracks.iter().map(|t| t.size).sum();
        assert!(total_base >= 90.0);
    }

    #[test]
    fn growth_limit_infinity_init() {
        // Auto track with no span=1 items — limit should be set to base after span=1 phase
        let defs = vec![TrackSize::Auto, TrackSize::Auto];
        let contribs = vec![contrib(0, 2, 100.0, 200.0)];
        let tracks = resolve_tracks(&defs, 500.0, 0.0, &contribs, false);
        // After span=1 (none), limits go from infinity → 0 (base).
        // Then span=2 distributes. Each should get at least 50.
        assert!(tracks[0].size >= 50.0);
        assert!(tracks[1].size >= 50.0);
    }

    #[test]
    fn maximize_phase() {
        // Single auto track, small content, large available
        let defs = vec![TrackSize::Auto];
        let contribs = vec![contrib(0, 1, 50.0, 100.0)];
        let tracks = resolve_tracks(&defs, 500.0, 0.0, &contribs, false);
        // After maximize, track should grow up to limit then beyond (infinite limit → 500).
        // limit was set to 100 by span=1, then §12.4 step 2.3 doesn't apply (limit is finite).
        // maximize grows base from 50 to 100 (limit). Then no infinite limits.
        // effective_non_fr_size = max(100, 100) = 100.
        assert_eq!(tracks[0].size, 100.0);
    }

    #[test]
    fn stretch_auto_tracks_test() {
        let defs = vec![TrackSize::Auto, TrackSize::Length(100.0)];
        let contribs = vec![contrib(0, 1, 50.0, 80.0)];
        let tracks = resolve_tracks(&defs, 500.0, 0.0, &contribs, true);
        // Without stretch: auto track = 80 (limit after maximize).
        // With stretch: free = 500 - 80 - 100 = 320. Auto gets +320.
        assert_eq!(tracks[0].size, 400.0);
        assert_eq!(tracks[1].size, 100.0);
    }

    #[test]
    fn fit_content_spanning() {
        let defs = vec![TrackSize::FitContent(150.0), TrackSize::Auto];
        let contribs = vec![contrib(0, 1, 30.0, 200.0), contrib(1, 1, 20.0, 60.0)];
        let tracks = resolve_tracks(&defs, 500.0, 0.0, &contribs, false);
        // FitContent(150) is intrinsic: base=30 (min-content), limit capped at min(150, 200)=150.
        // After §12.4 step 2.3: limit is finite (150), no change.
        // Maximize: base grows from 30 to 150 (limit).
        assert!(tracks[0].size <= 150.0);
        assert!(tracks[0].size >= 30.0);
    }

    #[test]
    fn fr_and_intrinsic_mixed() {
        let defs = vec![TrackSize::Fr(1.0), TrackSize::Auto];
        let contribs = vec![contrib(1, 1, 50.0, 100.0)];
        let tracks = resolve_tracks(&defs, 500.0, 0.0, &contribs, false);
        // Auto track: base=50, limit=100, after maximize grows to 100.
        // Fr track: remaining = 500 - 100 = 400.
        assert_eq!(tracks[1].size, 100.0);
        assert!((tracks[0].size - 400.0).abs() < 1.0);
    }

    #[test]
    fn empty_grid() {
        let defs: Vec<TrackSize> = vec![];
        let tracks = resolve_tracks(&defs, 500.0, 0.0, &[], false);
        assert!(tracks.is_empty());
    }

    #[test]
    fn negative_free_space() {
        // Items larger than available space — no overflow panic
        let defs = vec![TrackSize::Auto, TrackSize::Auto];
        let contribs = vec![contrib(0, 1, 300.0, 400.0), contrib(1, 1, 300.0, 400.0)];
        let tracks = resolve_tracks(&defs, 500.0, 0.0, &contribs, false);
        // Both tracks want 400, available is 500. Maximize grows to limit (400 each).
        // effective size = max(400, 400) = 400 each. Total = 800 > 500 but that's OK.
        assert_eq!(tracks[0].size, 400.0);
        assert_eq!(tracks[1].size, 400.0);
    }

    #[test]
    fn span_sort_order() {
        // Ensure span=2 is processed before span=3
        let defs = vec![TrackSize::Auto, TrackSize::Auto, TrackSize::Auto];
        let contribs = vec![contrib(0, 3, 300.0, 600.0), contrib(0, 2, 200.0, 400.0)];
        let tracks = resolve_tracks(&defs, 1000.0, 0.0, &contribs, false);
        // span=2 processed first: tracks[0..2] get 100 each (200/2).
        // span=3: sum bases = 300 (100+100+100 after span=2). 300-300=0, no extra.
        // Wait: span=2 only covers tracks 0..2. Track 2 base is 0.
        // span=2 first: extra=200-0=200, each of [0,1] gets 100 → base=100 each.
        // span=3: sum bases = 100+100+0=200. extra=300-200=100. Distributed to all 3.
        // Each gets ~33.3 → bases: 133.3, 133.3, 33.3
        let total: f32 = tracks.iter().map(|t| t.base).sum();
        assert!(total >= 300.0);
        assert_eq!(tracks.len(), 3);
    }
}
