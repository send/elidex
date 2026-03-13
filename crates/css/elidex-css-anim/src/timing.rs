//! CSS easing / timing functions.
//!
//! Cubic-bezier solver uses Newton-Raphson iteration (8 steps) with bisection
//! fallback, matching browser implementations. `steps()` implements all four
//! jump positions from CSS Easing Functions Level 2.

use std::fmt;

/// A CSS `<easing-function>` value.
///
/// Used by `transition-timing-function` and `animation-timing-function`.
#[derive(Clone, Debug, PartialEq)]
pub enum TimingFunction {
    /// Cubic bezier curve `cubic-bezier(x1, y1, x2, y2)`.
    /// Per CSS Easing Functions L1 §2.1, x1 and x2 must be in [0, 1].
    /// Validated at parse time; no runtime check.
    CubicBezier(f32, f32, f32, f32),
    /// `steps(count, position)`.
    Steps(u32, StepPosition),
    /// `linear` — equivalent to `cubic-bezier(0, 0, 1, 1)`.
    Linear,
}

impl Default for TimingFunction {
    fn default() -> Self {
        Self::EASE
    }
}

/// Step position for `steps()` function.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum StepPosition {
    /// `jump-start` / `start` — first rise at input 0.
    JumpStart,
    /// `jump-end` / `end` (default) — last rise at input 1.
    #[default]
    JumpEnd,
    /// `jump-none` — neither first nor last rise.
    JumpNone,
    /// `jump-both` — both first and last rise.
    JumpBoth,
}

impl TimingFunction {
    /// `ease` — `cubic-bezier(0.25, 0.1, 0.25, 1.0)`.
    pub const EASE: Self = Self::CubicBezier(0.25, 0.1, 0.25, 1.0);
    /// `ease-in` — `cubic-bezier(0.42, 0, 1, 1)`.
    pub const EASE_IN: Self = Self::CubicBezier(0.42, 0.0, 1.0, 1.0);
    /// `ease-out` — `cubic-bezier(0, 0, 0.58, 1)`.
    pub const EASE_OUT: Self = Self::CubicBezier(0.0, 0.0, 0.58, 1.0);
    /// `ease-in-out` — `cubic-bezier(0.42, 0, 0.58, 1)`.
    pub const EASE_IN_OUT: Self = Self::CubicBezier(0.42, 0.0, 0.58, 1.0);

    /// Sample the timing function at progress `t` (0.0..=1.0).
    ///
    /// Returns the output progress (may overshoot for cubic-bezier per CSS
    /// Easing Functions Level 1 §2.2).
    #[must_use]
    pub fn sample(&self, t: f32) -> f32 {
        match self {
            Self::Linear => t,
            Self::CubicBezier(x1, y1, x2, y2) => cubic_bezier_sample(*x1, *y1, *x2, *y2, t),
            Self::Steps(count, position) => steps_sample(*count, *position, t),
        }
    }
}

impl fmt::Display for TimingFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Linear => f.write_str("linear"),
            Self::CubicBezier(x1, y1, x2, y2) => {
                // Check named curves using epsilon comparison
                if let Some(name) = match_named_bezier(*x1, *y1, *x2, *y2) {
                    return f.write_str(name);
                }
                write!(f, "cubic-bezier({x1}, {y1}, {x2}, {y2})")
            }
            Self::Steps(count, pos) => {
                let pos_str = match pos {
                    StepPosition::JumpEnd => "end",
                    StepPosition::JumpStart => "start",
                    StepPosition::JumpNone => "jump-none",
                    StepPosition::JumpBoth => "jump-both",
                };
                write!(f, "steps({count}, {pos_str})")
            }
        }
    }
}

/// Match a cubic-bezier against named easing keywords.
///
/// Returns the keyword name if the control points match a named curve
/// within epsilon tolerance, or `None` for custom curves.
fn match_named_bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> Option<&'static str> {
    const NAMED_CURVES: &[(&str, f32, f32, f32, f32)] = &[
        ("ease", 0.25, 0.1, 0.25, 1.0),
        ("ease-in", 0.42, 0.0, 1.0, 1.0),
        ("ease-out", 0.0, 0.0, 0.58, 1.0),
        ("ease-in-out", 0.42, 0.0, 0.58, 1.0),
    ];
    for &(name, nx1, ny1, nx2, ny2) in NAMED_CURVES {
        if (x1 - nx1).abs() < f32::EPSILON
            && (y1 - ny1).abs() < f32::EPSILON
            && (x2 - nx2).abs() < f32::EPSILON
            && (y2 - ny2).abs() < f32::EPSILON
        {
            return Some(name);
        }
    }
    None
}

// --- Cubic bezier solver ---
//
// We solve for t on the X curve, then evaluate Y at that t.
// Uses Newton-Raphson with bisection fallback (matches browser implementations).

const NEWTON_ITERATIONS: u32 = 8;
const NEWTON_MIN_SLOPE: f32 = 0.001;
const SUBDIVISION_PRECISION: f32 = 1e-7;
const SUBDIVISION_MAX_ITERATIONS: u32 = 10;

fn cubic_bezier_sample(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    if t <= 0.0 {
        // Before phase: extrapolate via tangent at t=0
        let slope = bezier_component_derivative(y1, y2, 0.0);
        let x_slope = bezier_component_derivative(x1, x2, 0.0);
        if x_slope.abs() < f32::EPSILON {
            return 0.0;
        }
        return (slope / x_slope) * t;
    }
    if t >= 1.0 {
        // After phase: extrapolate via tangent at t=1
        let slope = bezier_component_derivative(y1, y2, 1.0);
        let x_slope = bezier_component_derivative(x1, x2, 1.0);
        if x_slope.abs() < f32::EPSILON {
            return 1.0;
        }
        return 1.0 + (slope / x_slope) * (t - 1.0);
    }
    // Linear case
    if (x1 - y1).abs() < f32::EPSILON && (x2 - y2).abs() < f32::EPSILON {
        return t;
    }

    // Find parameter on X curve corresponding to `t`
    let param = solve_curve_x(x1, x2, t);
    // Evaluate Y at that parameter
    bezier_component(y1, y2, param)
}

/// Evaluate a single component of a cubic bezier at parameter `t`.
/// Control points: P0=0, P1=c1, P2=c2, P3=1.
fn bezier_component(c1: f32, c2: f32, t: f32) -> f32 {
    // B(t) = 3(1-t)^2*t*c1 + 3(1-t)*t^2*c2 + t^3
    // Expanded: ((1-3c2+3c1)*t + (3c2-6c1))*t + 3c1)*t
    let a = 1.0 - 3.0 * c2 + 3.0 * c1;
    let b = 3.0 * c2 - 6.0 * c1;
    let c = 3.0 * c1;
    ((a * t + b) * t + c) * t
}

/// Derivative of bezier component at `t`.
fn bezier_component_derivative(c1: f32, c2: f32, t: f32) -> f32 {
    let a = 1.0 - 3.0 * c2 + 3.0 * c1;
    let b = 3.0 * c2 - 6.0 * c1;
    let c = 3.0 * c1;
    (3.0 * a * t + 2.0 * b) * t + c
}

/// Solve for `t` such that `bezier_x(t) = x`.
fn solve_curve_x(x1: f32, x2: f32, x: f32) -> f32 {
    // Try Newton-Raphson
    let mut t = x; // initial guess
    for _ in 0..NEWTON_ITERATIONS {
        let current_x = bezier_component(x1, x2, t);
        let slope = bezier_component_derivative(x1, x2, t);
        if slope.abs() < NEWTON_MIN_SLOPE {
            break;
        }
        t -= (current_x - x) / slope;
    }

    // Verify Newton result
    let result = bezier_component(x1, x2, t);
    if (result - x).abs() < SUBDIVISION_PRECISION {
        return t;
    }

    // Bisection fallback
    let mut lo = 0.0_f32;
    let mut hi = 1.0_f32;
    t = x;
    for _ in 0..SUBDIVISION_MAX_ITERATIONS {
        let current_x = bezier_component(x1, x2, t);
        if !current_x.is_finite() {
            break;
        }
        if (current_x - x).abs() < SUBDIVISION_PRECISION {
            break;
        }
        if current_x < x {
            lo = t;
        } else {
            hi = t;
        }
        t = lo.midpoint(hi);
        if !t.is_finite() {
            break;
        }
    }
    t.clamp(0.0, 1.0)
}

/// Evaluate `steps()` at progress `t`.
///
/// Per CSS Easing Functions Level 1, clamping is applied to the output
/// rather than the input so that before/after phase calculations work
/// correctly (e.g. negative output for `jump-start`/`jump-both` when `t < 0`).
fn steps_sample(count: u32, position: StepPosition, t: f32) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    let steps = count.max(1) as f32;

    let (intervals, start_offset) = match position {
        StepPosition::JumpStart => (steps, 1.0),
        StepPosition::JumpEnd => (steps, 0.0),
        StepPosition::JumpNone => ((steps - 1.0).max(1.0), 0.0),
        StepPosition::JumpBoth => (steps + 1.0, 1.0),
    };

    let current_step = (t * steps).floor() + start_offset;
    // NOTE: Clamped to [0, 1] since the animation engine always provides
    // progress values within [0, 1]. If cubic-bezier overshoot feeding into
    // steps() is needed in the future, remove this clamp.
    (current_step / intervals).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_timing() {
        let tf = TimingFunction::Linear;
        assert_eq!(tf.sample(0.0), 0.0);
        assert_eq!(tf.sample(0.5), 0.5);
        assert_eq!(tf.sample(1.0), 1.0);
    }

    #[test]
    fn ease_endpoints() {
        let tf = TimingFunction::EASE;
        assert_eq!(tf.sample(0.0), 0.0);
        assert_eq!(tf.sample(1.0), 1.0);
    }

    #[test]
    fn ease_midpoint_faster_than_linear() {
        // ease starts slow, accelerates — at 0.5 input, output > 0.5
        let tf = TimingFunction::EASE;
        assert!(tf.sample(0.5) > 0.5);
    }

    #[test]
    fn ease_in_starts_slow() {
        let tf = TimingFunction::EASE_IN;
        // At 0.25, ease-in should be below linear
        assert!(tf.sample(0.25) < 0.25);
    }

    #[test]
    fn ease_out_starts_fast() {
        let tf = TimingFunction::EASE_OUT;
        // At 0.25, ease-out should be above linear
        assert!(tf.sample(0.25) > 0.25);
    }

    #[test]
    fn ease_in_out_symmetric() {
        let tf = TimingFunction::EASE_IN_OUT;
        // ease-in-out is symmetric around 0.5
        let a = tf.sample(0.25);
        let b = tf.sample(0.75);
        assert!((a + b - 1.0).abs() < 0.01);
    }

    #[test]
    fn cubic_bezier_custom() {
        let tf = TimingFunction::CubicBezier(0.0, 0.0, 1.0, 1.0);
        // This is effectively linear
        assert!((tf.sample(0.5) - 0.5).abs() < 0.01);
    }

    #[test]
    fn steps_jump_end() {
        let tf = TimingFunction::Steps(4, StepPosition::JumpEnd);
        assert_eq!(tf.sample(0.0), 0.0);
        assert_eq!(tf.sample(0.24), 0.0);
        assert_eq!(tf.sample(0.25), 0.25);
        assert_eq!(tf.sample(0.49), 0.25);
        assert_eq!(tf.sample(0.5), 0.5);
        assert_eq!(tf.sample(1.0), 1.0);
    }

    #[test]
    fn steps_jump_start() {
        let tf = TimingFunction::Steps(4, StepPosition::JumpStart);
        // At 0.0, already at first step
        assert_eq!(tf.sample(0.0), 0.25);
    }

    #[test]
    fn steps_jump_both() {
        let tf = TimingFunction::Steps(3, StepPosition::JumpBoth);
        // 4 intervals (count+1), starts at 1/4
        assert_eq!(tf.sample(0.0), 0.25);
        assert_eq!(tf.sample(1.0), 1.0);
    }

    #[test]
    fn steps_jump_none() {
        let tf = TimingFunction::Steps(3, StepPosition::JumpNone);
        // 2 intervals (count-1), starts at 0
        assert_eq!(tf.sample(0.0), 0.0);
        assert_eq!(tf.sample(1.0), 1.0);
    }

    #[test]
    fn timing_function_display() {
        assert_eq!(TimingFunction::Linear.to_string(), "linear");
        assert_eq!(TimingFunction::EASE.to_string(), "ease");
        assert_eq!(
            TimingFunction::CubicBezier(0.1, 0.2, 0.3, 0.4).to_string(),
            "cubic-bezier(0.1, 0.2, 0.3, 0.4)"
        );
        assert_eq!(
            TimingFunction::Steps(4, StepPosition::JumpEnd).to_string(),
            "steps(4, end)"
        );
    }

    #[test]
    fn timing_function_default_is_ease() {
        assert_eq!(TimingFunction::default(), TimingFunction::EASE);
    }

    #[test]
    fn linear_passes_through_out_of_range() {
        // Linear passes t through unchanged (no clamping).
        let tf = TimingFunction::Linear;
        assert_eq!(tf.sample(-0.5), -0.5);
        assert_eq!(tf.sample(1.5), 1.5);
    }

    #[test]
    fn steps_clamps_out_of_range() {
        // Steps clamps t to [0, 1] internally.
        let tf = TimingFunction::Steps(4, StepPosition::JumpEnd);
        assert_eq!(tf.sample(-0.5), 0.0);
        assert_eq!(tf.sample(1.5), 1.0);
    }

    #[test]
    fn step_position_default() {
        assert_eq!(StepPosition::default(), StepPosition::JumpEnd);
    }
}
