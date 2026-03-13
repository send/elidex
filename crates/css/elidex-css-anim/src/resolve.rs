//! Resolution of animation/transition property values into `AnimStyle`.

use crate::style::{
    AnimStyle, AnimationDirection, AnimationFillMode, IterationCount, PlayState, TransitionProperty,
};
use crate::timing::TimingFunction;
use elidex_plugin::CssValue;

/// Maximum number of items in a comma-separated CSS list value to prevent
/// unbounded memory growth from malicious or malformed stylesheets.
const MAX_LIST_ITEMS: usize = 1024;

/// Resolve a parsed transition-property string into `TransitionProperty` values.
#[must_use]
pub fn resolve_transition_property(value: &CssValue) -> Vec<TransitionProperty> {
    let s = match value {
        CssValue::String(s) => s.as_str(),
        CssValue::Keyword(k) => k.as_str(),
        _ => return vec![TransitionProperty::All],
    };
    s.split(',')
        .take(MAX_LIST_ITEMS)
        .map(|p| {
            let p = p.trim();
            match p {
                "none" => TransitionProperty::None,
                "all" => TransitionProperty::All,
                _ => TransitionProperty::Property(p.to_string()),
            }
        })
        .collect()
}

/// Resolve a parsed time list (single Time or List of Time) into seconds.
#[must_use]
pub fn resolve_time_list(value: &CssValue) -> Vec<f32> {
    match value {
        CssValue::Time(t) => vec![*t],
        CssValue::Number(n) => vec![*n],
        CssValue::List(items) => items
            .iter()
            .map(|v| match v {
                CssValue::Time(t) => *t,
                CssValue::Number(n) => *n,
                _ => 0.0,
            })
            .collect(),
        _ => vec![0.0],
    }
}

/// Resolve a timing function list string into `TimingFunction` values.
#[must_use]
pub fn resolve_timing_function_list(value: &CssValue) -> Vec<TimingFunction> {
    let s = match value {
        CssValue::String(s) => s.as_str(),
        _ => return vec![TimingFunction::default()],
    };
    // Each entry could be a keyword or a function — re-parse
    s.split(',')
        .take(MAX_LIST_ITEMS)
        .map(|part| {
            let part = part.trim();
            let mut pi = cssparser::ParserInput::new(part);
            let mut parser = cssparser::Parser::new(&mut pi);
            crate::parse::parse_timing_function(&mut parser).unwrap_or_default()
        })
        .collect()
}

/// Resolve a comma-separated string into animation name list.
#[must_use]
pub fn resolve_animation_names(value: &CssValue) -> Vec<String> {
    let s = match value {
        CssValue::String(s) => s.as_str(),
        CssValue::Keyword(k) => k.as_str(),
        _ => return Vec::new(),
    };
    s.split(',')
        .take(MAX_LIST_ITEMS)
        .map(|s| s.trim().to_string())
        .collect()
}

/// Resolve a comma-separated string into iteration count list.
#[must_use]
pub fn resolve_iteration_counts(value: &CssValue) -> Vec<IterationCount> {
    let s = match value {
        CssValue::String(s) => s.as_str(),
        _ => return vec![IterationCount::default()],
    };
    s.split(',')
        .take(MAX_LIST_ITEMS)
        .map(|part| {
            let part = part.trim();
            if part.eq_ignore_ascii_case("infinite") {
                IterationCount::Infinite
            } else {
                part.parse::<f32>()
                    .map(IterationCount::Number)
                    .unwrap_or_default()
            }
        })
        .collect()
}

/// Resolve a comma-separated string into animation direction list.
#[must_use]
pub fn resolve_animation_directions(value: &CssValue) -> Vec<AnimationDirection> {
    resolve_keyword_list(value, |kw| match kw {
        "reverse" => AnimationDirection::Reverse,
        "alternate" => AnimationDirection::Alternate,
        "alternate-reverse" => AnimationDirection::AlternateReverse,
        // "normal" and unknown keywords default to Normal
        _ => AnimationDirection::Normal,
    })
}

/// Resolve a comma-separated string into animation fill mode list.
#[must_use]
pub fn resolve_fill_modes(value: &CssValue) -> Vec<AnimationFillMode> {
    resolve_keyword_list(value, |kw| match kw {
        "forwards" => AnimationFillMode::Forwards,
        "backwards" => AnimationFillMode::Backwards,
        "both" => AnimationFillMode::Both,
        // "none" and unknown keywords default to None
        _ => AnimationFillMode::None,
    })
}

/// Resolve a comma-separated string into play state list.
#[must_use]
pub fn resolve_play_states(value: &CssValue) -> Vec<PlayState> {
    resolve_keyword_list(value, |kw| match kw {
        "paused" => PlayState::Paused,
        // "running" and unknown keywords default to Running
        _ => PlayState::Running,
    })
}

fn resolve_keyword_list<T: Default>(value: &CssValue, f: impl Fn(&str) -> T) -> Vec<T> {
    let s = match value {
        CssValue::String(s) => s.as_str(),
        CssValue::Keyword(k) => k.as_str(),
        _ => return vec![T::default()],
    };
    s.split(',')
        .take(MAX_LIST_ITEMS)
        .map(|part| f(part.trim()))
        .collect()
}

/// Build an `AnimStyle` from resolved property values.
///
/// Called by the handler's `resolve()` to populate the ECS component.
pub fn resolve_anim_property(name: &str, value: &CssValue, style: &mut AnimStyle) {
    match name {
        "transition-property" => style.transition_property = resolve_transition_property(value),
        "transition-duration" => style.transition_duration = resolve_time_list(value),
        "transition-timing-function" => {
            style.transition_timing_function = resolve_timing_function_list(value);
        }
        "transition-delay" => style.transition_delay = resolve_time_list(value),
        "animation-name" => style.animation_name = resolve_animation_names(value),
        "animation-duration" => style.animation_duration = resolve_time_list(value),
        "animation-timing-function" => {
            style.animation_timing_function = resolve_timing_function_list(value);
        }
        "animation-delay" => style.animation_delay = resolve_time_list(value),
        "animation-iteration-count" => {
            style.animation_iteration_count = resolve_iteration_counts(value);
        }
        "animation-direction" => {
            style.animation_direction = resolve_animation_directions(value);
        }
        "animation-fill-mode" => style.animation_fill_mode = resolve_fill_modes(value),
        "animation-play-state" => style.animation_play_state = resolve_play_states(value),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_transition_property_all() {
        let props = resolve_transition_property(&CssValue::String("all".into()));
        assert_eq!(props, vec![TransitionProperty::All]);
    }

    #[test]
    fn resolve_transition_property_multiple() {
        let props = resolve_transition_property(&CssValue::String("opacity, width, color".into()));
        assert_eq!(props.len(), 3);
        assert_eq!(props[0], TransitionProperty::Property("opacity".into()));
        assert_eq!(props[1], TransitionProperty::Property("width".into()));
        assert_eq!(props[2], TransitionProperty::Property("color".into()));
    }

    #[test]
    fn resolve_time_list_single() {
        let times = resolve_time_list(&CssValue::Time(0.3));
        assert_eq!(times, vec![0.3]);
    }

    #[test]
    fn resolve_time_list_multiple() {
        let times = resolve_time_list(&CssValue::List(vec![
            CssValue::Time(0.3),
            CssValue::Time(0.5),
        ]));
        assert_eq!(times, vec![0.3, 0.5]);
    }

    #[test]
    fn resolve_timing_function_list_ease() {
        let fns = resolve_timing_function_list(&CssValue::String("ease".into()));
        assert_eq!(fns, vec![TimingFunction::EASE]);
    }

    #[test]
    fn resolve_animation_names_multiple() {
        let names = resolve_animation_names(&CssValue::String("fadeIn, slideUp".into()));
        assert_eq!(names, vec!["fadeIn", "slideUp"]);
    }

    #[test]
    fn resolve_iteration_counts_mixed() {
        let counts = resolve_iteration_counts(&CssValue::String("3, infinite, 1.5".into()));
        assert_eq!(counts.len(), 3);
        assert_eq!(counts[0], IterationCount::Number(3.0));
        assert_eq!(counts[1], IterationCount::Infinite);
        assert_eq!(counts[2], IterationCount::Number(1.5));
    }

    #[test]
    fn resolve_directions() {
        let dirs = resolve_animation_directions(&CssValue::String("normal, reverse".into()));
        assert_eq!(
            dirs,
            vec![AnimationDirection::Normal, AnimationDirection::Reverse]
        );
    }

    #[test]
    fn resolve_fill_modes_both() {
        let modes = resolve_fill_modes(&CssValue::String("both".into()));
        assert_eq!(modes, vec![AnimationFillMode::Both]);
    }

    #[test]
    fn resolve_play_states_paused() {
        let states = resolve_play_states(&CssValue::String("paused".into()));
        assert_eq!(states, vec![PlayState::Paused]);
    }

    #[test]
    fn resolve_anim_property_transition_duration() {
        let mut style = AnimStyle::default();
        resolve_anim_property("transition-duration", &CssValue::Time(0.5), &mut style);
        assert_eq!(style.transition_duration, vec![0.5]);
    }

    #[test]
    fn resolve_anim_property_animation_name() {
        let mut style = AnimStyle::default();
        resolve_anim_property(
            "animation-name",
            &CssValue::String("fadeIn".into()),
            &mut style,
        );
        assert_eq!(style.animation_name, vec!["fadeIn"]);
    }
}
