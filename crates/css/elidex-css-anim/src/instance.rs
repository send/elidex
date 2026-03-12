//! Animation and transition instance tracking.

use crate::style::{AnimationDirection, AnimationFillMode, IterationCount, PlayState};
use crate::timing::TimingFunction;

/// A running animation instance bound to a specific element.
#[derive(Clone, Debug)]
pub struct AnimationInstance {
    /// The `@keyframes` name.
    pub name: String,
    /// Duration in seconds.
    pub duration: f32,
    /// Timing function.
    pub timing_function: TimingFunction,
    /// Delay in seconds (can be negative for a head start).
    pub delay: f32,
    /// Iteration count.
    pub iteration_count: IterationCount,
    /// Direction.
    pub direction: AnimationDirection,
    /// Fill mode.
    pub fill_mode: AnimationFillMode,
    /// Play state.
    pub play_state: PlayState,
    /// Elapsed time since the animation started (after delay), in seconds.
    pub elapsed: f32,
    /// The time at which this animation was started (document time).
    pub start_time: f64,
    /// Whether the animation has finished.
    pub finished: bool,
    /// Whether the `animationend` event has been dispatched.
    pub end_event_dispatched: bool,
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
        }
    }

    /// Compute the current iteration progress (0.0..=1.0), accounting for
    /// direction and iteration count.
    ///
    /// Returns `None` if the animation is in the delay phase or finished
    /// without a fill mode that applies.
    #[must_use]
    pub fn progress(&self) -> Option<f32> {
        let active_time = self.elapsed - self.delay;

        if active_time < 0.0 {
            // In delay phase
            return match self.fill_mode {
                AnimationFillMode::Backwards | AnimationFillMode::Both => {
                    Some(self.direction_adjusted_progress(0.0))
                }
                _ => None,
            };
        }

        if self.duration <= 0.0 {
            return Some(1.0);
        }

        let total_duration = match self.iteration_count {
            IterationCount::Number(n) => n * self.duration,
            IterationCount::Infinite => f32::INFINITY,
        };

        if active_time >= total_duration {
            // Finished
            return match self.fill_mode {
                AnimationFillMode::Forwards | AnimationFillMode::Both => {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let final_iteration = match self.iteration_count {
                        IterationCount::Number(n) => (n.ceil() as u32).saturating_sub(1),
                        IterationCount::Infinite => 0,
                    };
                    let raw = if n_is_whole(match self.iteration_count {
                        IterationCount::Number(n) => n,
                        IterationCount::Infinite => 1.0,
                    }) {
                        1.0
                    } else {
                        (active_time % self.duration) / self.duration
                    };
                    Some(self.direction_for_iteration(final_iteration, raw))
                }
                _ => None,
            };
        }

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let iteration = (active_time / self.duration).floor() as u32;
        let raw_progress = (active_time % self.duration) / self.duration;
        let directed = self.direction_for_iteration(iteration, raw_progress);
        Some(self.timing_function.sample(directed))
    }

    /// Apply direction adjustment to a raw progress value.
    fn direction_adjusted_progress(&self, raw: f32) -> f32 {
        self.direction_for_iteration(0, raw)
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
    pub elapsed: f32,
    /// Whether the transition has completed.
    pub finished: bool,
    /// Whether the `transitionend` event has been dispatched.
    pub end_event_dispatched: bool,
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
        }
    }

    /// Compute the current interpolated value.
    ///
    /// Returns `None` if the transition is in the delay phase.
    #[must_use]
    pub fn current_value(&self) -> Option<elidex_plugin::CssValue> {
        let active_time = self.elapsed - self.delay;
        if active_time < 0.0 {
            return Some(self.from.clone());
        }
        if self.duration <= 0.0 || active_time >= self.duration {
            return Some(self.to.clone());
        }
        let raw_progress = active_time / self.duration;
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
        let mut anim = make_anim(1.0, 0.0, IterationCount::Number(1.0), AnimationDirection::Normal, AnimationFillMode::None);
        anim.elapsed = 0.5;
        let p = anim.progress().unwrap();
        assert!((p - 0.5).abs() < 0.01);
    }

    #[test]
    fn animation_progress_delay() {
        let mut anim = make_anim(1.0, 0.5, IterationCount::Number(1.0), AnimationDirection::Normal, AnimationFillMode::None);
        anim.elapsed = 0.25;
        assert!(anim.progress().is_none());
    }

    #[test]
    fn animation_progress_delay_fill_backwards() {
        let mut anim = make_anim(1.0, 0.5, IterationCount::Number(1.0), AnimationDirection::Normal, AnimationFillMode::Backwards);
        anim.elapsed = 0.25;
        assert!(anim.progress().is_some());
    }

    #[test]
    fn animation_reverse() {
        let mut anim = make_anim(1.0, 0.0, IterationCount::Number(1.0), AnimationDirection::Reverse, AnimationFillMode::None);
        anim.elapsed = 0.25;
        let p = anim.progress().unwrap();
        // Reversed: 1.0 - 0.25 = 0.75
        assert!((p - 0.75).abs() < 0.01);
    }

    #[test]
    fn animation_alternate() {
        let mut anim = make_anim(1.0, 0.0, IterationCount::Number(3.0), AnimationDirection::Alternate, AnimationFillMode::None);
        // First iteration: normal (0.5 -> 0.5)
        anim.elapsed = 0.5;
        assert!((anim.progress().unwrap() - 0.5).abs() < 0.01);

        // Second iteration: reversed (1.5 -> iteration=1, raw=0.5 -> 1-0.5=0.5)
        anim.elapsed = 1.5;
        assert!((anim.progress().unwrap() - 0.5).abs() < 0.01);
    }

    #[test]
    fn animation_zero_duration() {
        let anim = make_anim(0.0, 0.0, IterationCount::Number(1.0), AnimationDirection::Normal, AnimationFillMode::None);
        assert_eq!(anim.progress(), Some(1.0));
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
}
