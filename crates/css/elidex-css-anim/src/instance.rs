//! Animation and transition instance tracking.

use crate::style::{AnimationDirection, AnimationFillMode, IterationCount, PlayState};
use crate::timing::TimingFunction;

/// A running animation instance bound to a specific element.
#[derive(Clone, Debug)]
pub struct AnimationInstance {
    /// The `@keyframes` name.
    name: String,
    /// Duration in seconds.
    duration: f32,
    /// Timing function.
    timing_function: TimingFunction,
    /// Delay in seconds (can be negative for a head start).
    delay: f32,
    /// Iteration count.
    iteration_count: IterationCount,
    /// Direction.
    direction: AnimationDirection,
    /// Fill mode.
    fill_mode: AnimationFillMode,
    /// Play state.
    pub(crate) play_state: PlayState,
    /// Elapsed time since the animation started (after delay), in seconds.
    pub(crate) elapsed: f64,
    /// The time at which this animation was started (document time).
    pub start_time: f64,
    /// Whether the animation has finished.
    pub(crate) finished: bool,
    /// Whether the `animationend` event has been dispatched.
    pub(crate) end_event_dispatched: bool,
    /// Whether the `animationstart` event has been dispatched.
    pub(crate) start_event_dispatched: bool,
    /// The last known iteration number (for `animationiteration` detection).
    pub(crate) current_iteration: u32,
}

impl AnimationInstance {
    /// Create a new animation instance starting at `start_time`.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        duration: f32,
        timing_function: TimingFunction,
        delay: f32,
        iteration_count: IterationCount,
        direction: AnimationDirection,
        fill_mode: AnimationFillMode,
        play_state: PlayState,
        start_time: f64,
    ) -> Self {
        Self {
            name,
            duration,
            timing_function,
            delay,
            iteration_count,
            direction,
            fill_mode,
            play_state,
            elapsed: 0.0,
            start_time,
            finished: false,
            end_event_dispatched: false,
            start_event_dispatched: false,
            current_iteration: 0,
        }
    }

    /// The `@keyframes` name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Duration in seconds.
    #[must_use]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Delay in seconds (can be negative for a head start).
    #[must_use]
    pub fn delay(&self) -> f32 {
        self.delay
    }

    /// Timing function.
    #[must_use]
    pub fn timing_function(&self) -> &TimingFunction {
        &self.timing_function
    }

    /// Iteration count.
    #[must_use]
    pub fn iteration_count(&self) -> IterationCount {
        self.iteration_count
    }

    /// Direction.
    #[must_use]
    pub fn direction(&self) -> AnimationDirection {
        self.direction
    }

    /// Fill mode.
    #[must_use]
    pub fn fill_mode(&self) -> AnimationFillMode {
        self.fill_mode
    }

    /// Play state.
    #[must_use]
    pub fn play_state(&self) -> PlayState {
        self.play_state
    }

    /// Whether the animation has finished.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Compute the current iteration progress (0.0..=1.0), accounting for
    /// direction and iteration count.
    ///
    /// Returns `None` if the animation is in the delay phase or finished
    /// without a fill mode that applies.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn progress(&self) -> Option<f32> {
        let active_time = self.elapsed - f64::from(self.delay);

        if active_time < 0.0 {
            // In delay phase
            return match self.fill_mode {
                AnimationFillMode::Backwards | AnimationFillMode::Both => {
                    // CSS Animations §3.9: directed progress first, then timing function.
                    let directed = self.direction_for_iteration(0, 0.0);
                    let transformed = self.timing_function.sample(directed);
                    Some(transformed)
                }
                _ => None,
            };
        }

        if self.duration <= 0.0 {
            // CSS Animations §3.9: zero-duration animations still respect
            // direction and fill-mode. The final iteration's directed progress
            // determines the output value.
            let directed = self.direction_for_iteration(self.final_iteration(), 1.0);
            let transformed = self.timing_function.sample(directed);
            return Some(transformed);
        }

        let dur = f64::from(self.duration);
        let total_duration = match self.iteration_count {
            IterationCount::Number(n) => f64::from(n) * dur,
            IterationCount::Infinite => f64::INFINITY,
        };

        if active_time >= total_duration {
            // Finished
            return match self.fill_mode {
                AnimationFillMode::Forwards | AnimationFillMode::Both => {
                    let final_iteration = self.final_iteration();
                    let raw = if n_is_whole(match self.iteration_count {
                        IterationCount::Number(n) => n,
                        IterationCount::Infinite => 1.0,
                    }) {
                        1.0
                    } else {
                        ((active_time % dur) / dur) as f32
                    };
                    // CSS Animations §3.9: apply direction first, then timing function.
                    let directed = self.direction_for_iteration(final_iteration, raw);
                    let transformed = self.timing_function.sample(directed);
                    Some(transformed)
                }
                _ => None,
            };
        }

        #[allow(clippy::cast_sign_loss)]
        let iteration = (active_time / dur).floor().min(f64::from(u32::MAX)) as u32;
        let raw_progress = ((active_time % dur) / dur) as f32;
        // Per CSS Animations Level 1 §3.9: apply direction first (step 2),
        // then apply timing function (step 3).
        let directed = self.direction_for_iteration(iteration, raw_progress);
        let transformed = self.timing_function.sample(directed);
        Some(transformed)
    }

    /// Compute the final iteration index for a finite animation.
    ///
    /// Used by `progress()` in the zero-duration and finished branches.
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation
    )]
    fn final_iteration(&self) -> u32 {
        match self.iteration_count {
            IterationCount::Number(n) => (n.ceil().min(u32::MAX as f32) as u32).saturating_sub(1),
            IterationCount::Infinite => 0,
        }
    }

    /// Compute direction-adjusted progress for a given iteration.
    fn direction_for_iteration(&self, iteration: u32, raw: f32) -> f32 {
        let reversed = match self.direction {
            AnimationDirection::Normal => false,
            AnimationDirection::Reverse => true,
            AnimationDirection::Alternate => iteration % 2 == 1,
            AnimationDirection::AlternateReverse => iteration.is_multiple_of(2),
        };
        if reversed {
            1.0 - raw
        } else {
            raw
        }
    }
}

fn n_is_whole(n: f32) -> bool {
    (n - n.round()).abs() < f32::EPSILON
}

/// A running transition instance for a single property.
#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct TransitionInstance {
    /// The property being transitioned.
    pub property: String,
    /// Start value (as CSS computed value).
    pub from: elidex_plugin::CssValue,
    /// End value (as CSS computed value).
    pub to: elidex_plugin::CssValue,
    /// Duration in seconds.
    pub duration: f32,
    /// Timing function.
    pub timing_function: TimingFunction,
    /// Delay in seconds.
    pub delay: f32,
    /// Elapsed time since transition start, in seconds.
    pub elapsed: f64,
    /// Whether the transition has completed.
    pub finished: bool,
    /// Whether the `transitionend` event has been dispatched.
    pub end_event_dispatched: bool,
    /// Whether the `transitionrun` event has been dispatched.
    pub run_event_dispatched: bool,
    /// Whether the `transitionstart` event has been dispatched.
    pub start_event_dispatched: bool,
}

impl TransitionInstance {
    /// Create a new transition instance.
    #[must_use]
    pub fn new(
        property: String,
        from: elidex_plugin::CssValue,
        to: elidex_plugin::CssValue,
        duration: f32,
        delay: f32,
        timing_function: TimingFunction,
    ) -> Self {
        Self {
            property,
            from,
            to,
            duration,
            timing_function,
            delay,
            elapsed: 0.0,
            finished: false,
            end_event_dispatched: false,
            run_event_dispatched: false,
            start_event_dispatched: false,
        }
    }

    /// Compute the current interpolated value.
    ///
    /// Returns `None` if the transition is in the delay phase.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn current_value(&self) -> Option<elidex_plugin::CssValue> {
        let active_time = self.elapsed - f64::from(self.delay);
        if active_time < 0.0 {
            return Some(self.from.clone());
        }
        let dur = f64::from(self.duration);
        if self.duration <= 0.0 || active_time >= dur {
            return Some(self.to.clone());
        }
        let raw_progress = (active_time / dur) as f32;
        let eased = self.timing_function.sample(raw_progress);
        crate::interpolate::interpolate(&self.from, &self.to, eased)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{AnimationDirection, AnimationFillMode, IterationCount, PlayState};
    use elidex_plugin::{CssValue, LengthUnit};

    fn make_anim(
        duration: f32,
        delay: f32,
        count: IterationCount,
        direction: AnimationDirection,
        fill: AnimationFillMode,
    ) -> AnimationInstance {
        AnimationInstance::new(
            "test".into(),
            duration,
            TimingFunction::Linear,
            delay,
            count,
            direction,
            fill,
            PlayState::Running,
            0.0,
        )
    }

    #[test]
    fn animation_progress_linear() {
        let mut anim = make_anim(
            1.0,
            0.0,
            IterationCount::Number(1.0),
            AnimationDirection::Normal,
            AnimationFillMode::None,
        );
        anim.elapsed = 0.5;
        let p = anim.progress().unwrap();
        assert!((p - 0.5).abs() < 0.01);
    }

    #[test]
    fn animation_progress_delay() {
        let mut anim = make_anim(
            1.0,
            0.5,
            IterationCount::Number(1.0),
            AnimationDirection::Normal,
            AnimationFillMode::None,
        );
        anim.elapsed = 0.25;
        assert!(anim.progress().is_none());
    }

    #[test]
    fn animation_progress_delay_fill_backwards() {
        let mut anim = make_anim(
            1.0,
            0.5,
            IterationCount::Number(1.0),
            AnimationDirection::Normal,
            AnimationFillMode::Backwards,
        );
        anim.elapsed = 0.25;
        assert!(anim.progress().is_some());
    }

    #[test]
    fn animation_reverse() {
        let mut anim = make_anim(
            1.0,
            0.0,
            IterationCount::Number(1.0),
            AnimationDirection::Reverse,
            AnimationFillMode::None,
        );
        anim.elapsed = 0.25;
        let p = anim.progress().unwrap();
        // Reversed: 1.0 - 0.25 = 0.75
        assert!((p - 0.75).abs() < 0.01);
    }

    #[test]
    fn animation_alternate() {
        let mut anim = make_anim(
            1.0,
            0.0,
            IterationCount::Number(3.0),
            AnimationDirection::Alternate,
            AnimationFillMode::None,
        );
        // First iteration: normal (0.5 -> 0.5)
        anim.elapsed = 0.5;
        assert!((anim.progress().unwrap() - 0.5).abs() < 0.01);

        // Second iteration: reversed (1.5 -> iteration=1, raw=0.5 -> 1-0.5=0.5)
        anim.elapsed = 1.5;
        assert!((anim.progress().unwrap() - 0.5).abs() < 0.01);
    }

    #[test]
    fn animation_zero_duration() {
        let anim = make_anim(
            0.0,
            0.0,
            IterationCount::Number(1.0),
            AnimationDirection::Normal,
            AnimationFillMode::None,
        );
        assert_eq!(anim.progress(), Some(1.0));
    }

    // S3-5: Zero-duration animation should still respect direction.
    // With direction:reverse, the final progress should be 0.0 (reversed 1.0).
    #[test]
    fn animation_zero_duration_respects_direction() {
        let anim = make_anim(
            0.0,
            0.0,
            IterationCount::Number(1.0),
            AnimationDirection::Reverse,
            AnimationFillMode::None,
        );
        // direction:reverse → 1.0 - 1.0 = 0.0
        assert_eq!(anim.progress(), Some(0.0));
    }

    #[test]
    fn animation_zero_duration_alternate_two_iterations() {
        let anim = make_anim(
            0.0,
            0.0,
            IterationCount::Number(2.0),
            AnimationDirection::Alternate,
            AnimationFillMode::None,
        );
        // 2 iterations, alternate: final iteration is 1 (reversed) → 1.0 - 1.0 = 0.0
        assert_eq!(anim.progress(), Some(0.0));
    }

    #[test]
    fn transition_current_value() {
        let mut trans = TransitionInstance::new(
            "opacity".into(),
            CssValue::Number(0.0),
            CssValue::Number(1.0),
            1.0,
            0.0,
            TimingFunction::Linear,
        );
        trans.elapsed = 0.5;
        let v = trans.current_value().unwrap();
        assert_eq!(v, CssValue::Number(0.5));
    }

    #[test]
    fn transition_in_delay() {
        let mut trans = TransitionInstance::new(
            "opacity".into(),
            CssValue::Number(0.0),
            CssValue::Number(1.0),
            1.0,
            0.5,
            TimingFunction::Linear,
        );
        trans.elapsed = 0.25;
        // In delay: returns from value
        assert_eq!(trans.current_value(), Some(CssValue::Number(0.0)));
    }

    #[test]
    fn transition_finished() {
        let mut trans = TransitionInstance::new(
            "width".into(),
            CssValue::Length(10.0, LengthUnit::Px),
            CssValue::Length(20.0, LengthUnit::Px),
            1.0,
            0.0,
            TimingFunction::Linear,
        );
        trans.elapsed = 2.0;
        assert_eq!(
            trans.current_value(),
            Some(CssValue::Length(20.0, LengthUnit::Px))
        );
    }

    // F18: fill-mode forwards with non-integer iteration count (2.5 iterations).
    //
    // CSS Animations §3.9: when the animation ends with a non-integer iteration
    // count, the final state should reflect the progress of the fractional part
    // of the last iteration rather than clamping to the end (1.0).
    // With 2.5 iterations of 1.0s duration, total active time = 2.5s.
    // The fractional part: (2.5 % 1.0) / 1.0 = 0.5, so progress = 0.5.
    #[test]
    fn fill_mode_forwards_non_integer_iteration() {
        let mut anim = make_anim(
            1.0,
            0.0,
            IterationCount::Number(2.5),
            AnimationDirection::Normal,
            AnimationFillMode::Forwards,
        );
        // Set elapsed to exactly the total duration (2.5 iterations × 1.0s = 2.5s).
        // active_time = 2.5 - 0.0 = 2.5 >= total_duration = 2.5, so we enter
        // the "finished with forwards fill" branch.
        // n_is_whole(2.5) = false, so raw = (2.5 % 1.0) / 1.0 = 0.5.
        anim.elapsed = 2.5;
        let p = anim
            .progress()
            .expect("should have progress with fill:forwards");
        // Progress should reflect the fractional position (0.5), not 1.0.
        assert!(
            (p - 0.5).abs() < 0.01,
            "expected ~0.5 for 2.5 non-integer iterations with forwards fill, got {p}"
        );
    }

    // F19: AlternateReverse direction test.
    //
    // alternate-reverse starts in the reverse direction and alternates:
    //   iteration 0 → reversed  (progress = 1.0 - raw)
    //   iteration 1 → normal    (progress = raw)
    //   iteration 2 → reversed  (progress = 1.0 - raw)
    #[test]
    fn animation_alternate_reverse() {
        let mut anim = make_anim(
            1.0,
            0.0,
            IterationCount::Number(3.0),
            AnimationDirection::AlternateReverse,
            AnimationFillMode::None,
        );

        // Iteration 0 at raw=0.25 → reversed → 1.0 - 0.25 = 0.75
        anim.elapsed = 0.25;
        let p0 = anim.progress().unwrap();
        assert!(
            (p0 - 0.75).abs() < 0.01,
            "iteration 0 alternate-reverse: expected 0.75, got {p0}"
        );

        // Iteration 1 at raw=0.25 → normal → 0.25
        anim.elapsed = 1.25;
        let p1 = anim.progress().unwrap();
        assert!(
            (p1 - 0.25).abs() < 0.01,
            "iteration 1 alternate-reverse: expected 0.25, got {p1}"
        );

        // Iteration 2 at raw=0.25 → reversed → 0.75
        anim.elapsed = 2.25;
        let p2 = anim.progress().unwrap();
        assert!(
            (p2 - 0.75).abs() < 0.01,
            "iteration 2 alternate-reverse: expected 0.75, got {p2}"
        );
    }

    // F20: Negative delay test — animation should start partway through.
    //
    // A negative delay means the animation conceptually started |delay| seconds
    // ago, so elapsed=0 should already show progress > 0.
    #[test]
    fn negative_delay_starts_partway_through() {
        let mut anim = make_anim(
            2.0,
            -0.5,
            IterationCount::Number(1.0),
            AnimationDirection::Normal,
            AnimationFillMode::None,
        );
        // elapsed=0, but delay=-0.5, so active_time = 0 - (-0.5) = 0.5 seconds
        // out of 2.0s duration → progress = 0.5 / 2.0 = 0.25
        anim.elapsed = 0.0;
        let p = anim
            .progress()
            .expect("should have active progress with negative delay");
        assert!(
            (p - 0.25).abs() < 0.01,
            "negative delay: expected ~0.25 at elapsed=0, got {p}"
        );
    }

    // F21: Mismatched type interpolation — interpolating between a length and a
    // color should use discrete fallback and return the "from" value before 50%.
    #[test]
    fn transition_mismatched_types_returns_from() {
        let mut trans = TransitionInstance::new(
            "x".into(),
            CssValue::Length(10.0, LengthUnit::Px),
            CssValue::Color(elidex_plugin::CssColor::RED),
            1.0,
            0.0,
            TimingFunction::Linear,
        );
        // At 30% (before 50%), should return the "from" value
        trans.elapsed = 0.3;
        assert_eq!(
            trans.current_value(),
            Some(CssValue::Length(10.0, LengthUnit::Px)),
            "mismatched types at t=0.3 should return from value"
        );
        // At 70% (after 50%), should return the "to" value
        trans.elapsed = 0.7;
        assert_eq!(
            trans.current_value(),
            Some(CssValue::Color(elidex_plugin::CssColor::RED)),
            "mismatched types at t=0.7 should return to value"
        );
    }
}
