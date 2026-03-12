//! `AnimStyle` ECS component for animation/transition computed values.

use crate::timing::TimingFunction;

/// Computed animation/transition style attached as an ECS component.
///
/// This component is only inserted on elements that have explicit
/// `transition-*` or `animation-*` properties set.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnimStyle {
    // --- Transition properties ---
    /// `transition-property` — list of animatable property names.
    pub transition_property: Vec<TransitionProperty>,
    /// `transition-duration` — list of durations in seconds.
    pub transition_duration: Vec<f32>,
    /// `transition-timing-function` — list of timing functions.
    pub transition_timing_function: Vec<TimingFunction>,
    /// `transition-delay` — list of delays in seconds.
    pub transition_delay: Vec<f32>,

    // --- Animation properties ---
    /// `animation-name` — list of `@keyframes` names.
    pub animation_name: Vec<String>,
    /// `animation-duration` — list of durations in seconds.
    pub animation_duration: Vec<f32>,
    /// `animation-timing-function` — list of timing functions.
    pub animation_timing_function: Vec<TimingFunction>,
    /// `animation-delay` — list of delays in seconds.
    pub animation_delay: Vec<f32>,
    /// `animation-iteration-count` — list of iteration counts.
    pub animation_iteration_count: Vec<IterationCount>,
    /// `animation-direction` — list of directions.
    pub animation_direction: Vec<AnimationDirection>,
    /// `animation-fill-mode` — list of fill modes.
    pub animation_fill_mode: Vec<AnimationFillMode>,
    /// `animation-play-state` — list of play states.
    pub animation_play_state: Vec<PlayState>,
}

/// Which properties to transition.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TransitionProperty {
    /// `none` — no property transitions.
    None,
    /// `all` — all animatable properties.
    All,
    /// A specific property name.
    Property(String),
}

use std::fmt;

/// `animation-iteration-count` value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IterationCount {
    /// A finite number of iterations.
    Number(f32),
    /// `infinite`.
    Infinite,
}

impl Default for IterationCount {
    fn default() -> Self {
        Self::Number(1.0)
    }
}

impl fmt::Display for IterationCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{n}"),
            Self::Infinite => f.write_str("infinite"),
        }
    }
}

/// `animation-direction` value.
///
/// Note: `elidex_plugin::keyword_enum!` is not `#[macro_export]`, so these
/// enums retain manual `Display` impls instead of using the macro.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum AnimationDirection {
    /// `normal`.
    #[default]
    Normal,
    /// `reverse`.
    Reverse,
    /// `alternate`.
    Alternate,
    /// `alternate-reverse`.
    AlternateReverse,
}

impl fmt::Display for AnimationDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => f.write_str("normal"),
            Self::Reverse => f.write_str("reverse"),
            Self::Alternate => f.write_str("alternate"),
            Self::AlternateReverse => f.write_str("alternate-reverse"),
        }
    }
}

/// `animation-fill-mode` value.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum AnimationFillMode {
    /// `none` (default).
    #[default]
    None,
    /// `forwards`.
    Forwards,
    /// `backwards`.
    Backwards,
    /// `both`.
    Both,
}

impl fmt::Display for AnimationFillMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => f.write_str("none"),
            Self::Forwards => f.write_str("forwards"),
            Self::Backwards => f.write_str("backwards"),
            Self::Both => f.write_str("both"),
        }
    }
}

/// `animation-play-state` value.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum PlayState {
    /// `running` (default).
    #[default]
    Running,
    /// `paused`.
    Paused,
}

impl fmt::Display for PlayState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Running => f.write_str("running"),
            Self::Paused => f.write_str("paused"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anim_style_default() {
        let style = AnimStyle::default();
        assert!(style.transition_property.is_empty());
        assert!(style.animation_name.is_empty());
    }

    #[test]
    fn iteration_count_default() {
        assert_eq!(IterationCount::default(), IterationCount::Number(1.0));
    }

    #[test]
    fn animation_direction_default() {
        assert_eq!(AnimationDirection::default(), AnimationDirection::Normal);
    }

    #[test]
    fn fill_mode_default() {
        assert_eq!(AnimationFillMode::default(), AnimationFillMode::None);
    }

    #[test]
    fn play_state_default() {
        assert_eq!(PlayState::default(), PlayState::Running);
    }

    #[test]
    fn transition_property_variants() {
        let none = TransitionProperty::None;
        let all = TransitionProperty::All;
        let prop = TransitionProperty::Property("opacity".into());
        assert_ne!(none, all);
        assert_ne!(all, prop);
    }
}
