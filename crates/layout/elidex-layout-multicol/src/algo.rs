//! Column geometry computation (CSS Multi-column Layout Level 1 §3.4).

use elidex_plugin::Dimension;

/// Resolved column geometry for a multicol container.
#[derive(Clone, Debug)]
pub struct ColumnGeometry {
    /// Actual number of columns.
    pub count: u32,
    /// Actual column width (content-box inline extent per column).
    pub width: f32,
    /// Resolved column gap.
    pub gap: f32,
}

/// Compute actual column count and width per CSS Multi-column L1 §3.4.
///
/// `available_inline_size`: the container's available inline extent (physical
/// width for horizontal-tb, physical height for vertical-rl/lr).
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn compute_column_geometry(
    available_inline_size: f32,
    column_count: Option<u32>,
    column_width: Dimension,
    column_gap: f32,
) -> ColumnGeometry {
    let gap = column_gap.max(0.0);

    let resolved_width = match column_width {
        Dimension::Length(px) if px.is_finite() && px > 0.0 => Some(px),
        _ => None,
    };

    let n = match (resolved_width, column_count) {
        // column-width only: N = max(1, floor((available + gap) / (width + gap)))
        (Some(w), None) => {
            let denom = w + gap;
            if denom > 0.0 {
                ((available_inline_size + gap) / denom).floor().max(1.0) as u32
            } else {
                1
            }
        }
        // column-count only
        (None, Some(count)) => count.max(1),
        // Both specified: N = min(count, floor-based)
        (Some(w), Some(count)) => {
            let denom = w + gap;
            let floor_n = if denom > 0.0 {
                ((available_inline_size + gap) / denom).floor().max(1.0) as u32
            } else {
                1
            };
            count.max(1).min(floor_n)
        }
        // Neither specified (shouldn't happen for multicol, but safe fallback)
        (None, None) => 1,
    };

    // W = max(0, (available - (N-1) * gap) / N)
    let w = if n > 0 {
        let total_gaps = gap * (n - 1) as f32;
        ((available_inline_size - total_gaps) / n as f32).max(0.0)
    } else {
        available_inline_size
    };

    ColumnGeometry {
        count: n,
        width: w,
        gap,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_count_only() {
        let g = compute_column_geometry(600.0, Some(3), Dimension::Auto, 20.0);
        assert_eq!(g.count, 3);
        // (600 - 2*20) / 3 = 186.666...
        assert!((g.width - 186.666_66).abs() < 0.01);
        assert_eq!(g.gap, 20.0);
    }

    #[test]
    fn column_width_only() {
        // available=600, width=200, gap=0 → N = floor((600+0)/(200+0)) = 3
        let g = compute_column_geometry(600.0, None, Dimension::Length(200.0), 0.0);
        assert_eq!(g.count, 3);
        assert_eq!(g.width, 200.0);
    }

    #[test]
    fn both_count_and_width() {
        // count=2, width=100, gap=20, available=600
        // floor((600+20)/(100+20)) = floor(5.166) = 5
        // N = min(2, 5) = 2
        let g = compute_column_geometry(600.0, Some(2), Dimension::Length(100.0), 20.0);
        assert_eq!(g.count, 2);
        // W = (600 - 20) / 2 = 290
        assert_eq!(g.width, 290.0);
    }

    #[test]
    fn single_column() {
        let g = compute_column_geometry(600.0, Some(1), Dimension::Auto, 20.0);
        assert_eq!(g.count, 1);
        assert_eq!(g.width, 600.0);
        assert_eq!(g.gap, 20.0);
    }

    #[test]
    fn gap_subtracted() {
        // 3 columns with 10px gap: W = (300 - 2*10) / 3 = 93.333...
        let g = compute_column_geometry(300.0, Some(3), Dimension::Auto, 10.0);
        assert_eq!(g.count, 3);
        assert!((g.width - 93.333_33).abs() < 0.01);
    }

    #[test]
    fn narrow_available_with_width() {
        // available=50, width=200 → N = max(1, floor((50+0)/200)) = 1
        let g = compute_column_geometry(50.0, None, Dimension::Length(200.0), 0.0);
        assert_eq!(g.count, 1);
        assert_eq!(g.width, 50.0);
    }

    #[test]
    fn zero_gap() {
        let g = compute_column_geometry(400.0, Some(4), Dimension::Auto, 0.0);
        assert_eq!(g.count, 4);
        assert_eq!(g.width, 100.0);
    }

    #[test]
    fn negative_gap_clamped() {
        let g = compute_column_geometry(400.0, Some(2), Dimension::Auto, -10.0);
        assert_eq!(g.gap, 0.0);
        assert_eq!(g.width, 200.0);
    }
}
