//! Transition detection — detects style changes that should trigger transitions.

use crate::interpolate::is_animatable;
use crate::style::{AnimStyle, TransitionProperty};
use crate::timing::TimingFunction;
use elidex_plugin::CssValue;

/// A detected transition that should be started.
#[derive(Clone, Debug)]
pub struct DetectedTransition {
    /// The property name.
    pub property: String,
    /// The old computed value.
    pub from: CssValue,
    /// The new computed value.
    pub to: CssValue,
    /// Duration in seconds.
    pub duration: f32,
    /// Delay in seconds.
    pub delay: f32,
    /// Timing function.
    pub timing_function: TimingFunction,
}

/// Detect transitions that should be triggered when computed values change.
///
/// Compares old and new values for properties listed in `transition-property`.
/// Returns a list of transitions that should be started.
#[must_use]
pub fn detect_transitions(
    anim_style: &AnimStyle,
    changed_properties: &[(String, CssValue, CssValue)],
) -> Vec<DetectedTransition> {
    if anim_style.transition_property.is_empty() {
        return Vec::new();
    }

    let mut detected = Vec::new();

    for (property, old_value, new_value) in changed_properties {
        if old_value == new_value {
            continue;
        }
        if !is_animatable(property) {
            continue;
        }
        if !property_matches_transition(&anim_style.transition_property, property) {
            continue;
        }

        // Find the index for this property in the transition lists
        let idx = property_index(&anim_style.transition_property, property);
        let duration = get_cyclic(&anim_style.transition_duration, idx).copied().unwrap_or(0.0);
        if duration <= 0.0 {
            continue;
        }
        let delay = get_cyclic(&anim_style.transition_delay, idx).copied().unwrap_or(0.0);
        let timing = get_cyclic(&anim_style.transition_timing_function, idx)
            .cloned()
            .unwrap_or_default();

        detected.push(DetectedTransition {
            property: property.clone(),
            from: old_value.clone(),
            to: new_value.clone(),
            duration,
            delay,
            timing_function: timing,
        });
    }

    detected
}

/// Check if a property matches the transition-property list.
fn property_matches_transition(props: &[TransitionProperty], property: &str) -> bool {
    props.iter().any(|p| match p {
        TransitionProperty::All => true,
        TransitionProperty::Property(name) => name == property,
        TransitionProperty::None => false,
    })
}

/// Find the index of a property in the transition-property list.
/// Returns 0 for `all`.
fn property_index(props: &[TransitionProperty], property: &str) -> usize {
    props
        .iter()
        .position(|p| match p {
            TransitionProperty::All => true,
            TransitionProperty::Property(name) => name == property,
            TransitionProperty::None => false,
        })
        .unwrap_or(0)
}

/// Get a value from a list with CSS cycling behavior.
fn get_cyclic<T>(list: &[T], index: usize) -> Option<&T> {
    if list.is_empty() {
        None
    } else {
        Some(&list[index % list.len()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::{CssColor, CssValue, LengthUnit};

    fn make_anim_style(prop: &str, duration: f32) -> AnimStyle {
        AnimStyle {
            transition_property: vec![TransitionProperty::Property(prop.into())],
            transition_duration: vec![duration],
            transition_timing_function: vec![TimingFunction::Linear],
            transition_delay: vec![0.0],
            ..AnimStyle::default()
        }
    }

    #[test]
    fn detect_opacity_transition() {
        let style = make_anim_style("opacity", 0.3);
        let changes = vec![("opacity".into(), CssValue::Number(1.0), CssValue::Number(0.5))];
        let detected = detect_transitions(&style, &changes);
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].property, "opacity");
        assert_eq!(detected[0].duration, 0.3);
    }

    #[test]
    fn detect_no_transition_same_value() {
        let style = make_anim_style("opacity", 0.3);
        let changes = vec![("opacity".into(), CssValue::Number(1.0), CssValue::Number(1.0))];
        let detected = detect_transitions(&style, &changes);
        assert!(detected.is_empty());
    }

    #[test]
    fn detect_no_transition_non_animatable() {
        let style = make_anim_style("display", 0.3);
        let changes = vec![(
            "display".into(),
            CssValue::Keyword("block".into()),
            CssValue::Keyword("none".into()),
        )];
        let detected = detect_transitions(&style, &changes);
        assert!(detected.is_empty());
    }

    #[test]
    fn detect_no_transition_zero_duration() {
        let style = make_anim_style("opacity", 0.0);
        let changes = vec![("opacity".into(), CssValue::Number(1.0), CssValue::Number(0.0))];
        let detected = detect_transitions(&style, &changes);
        assert!(detected.is_empty());
    }

    #[test]
    fn detect_transition_all() {
        let style = AnimStyle {
            transition_property: vec![TransitionProperty::All],
            transition_duration: vec![0.5],
            transition_timing_function: vec![TimingFunction::EASE],
            transition_delay: vec![0.0],
            ..AnimStyle::default()
        };
        let changes = vec![
            ("opacity".into(), CssValue::Number(1.0), CssValue::Number(0.0)),
            ("color".into(), CssValue::Color(CssColor::RED), CssValue::Color(CssColor::BLUE)),
        ];
        let detected = detect_transitions(&style, &changes);
        assert_eq!(detected.len(), 2);
    }

    #[test]
    fn detect_transition_with_delay() {
        let style = AnimStyle {
            transition_property: vec![TransitionProperty::Property("width".into())],
            transition_duration: vec![1.0],
            transition_timing_function: vec![TimingFunction::Linear],
            transition_delay: vec![0.5],
            ..AnimStyle::default()
        };
        let changes = vec![(
            "width".into(),
            CssValue::Length(100.0, LengthUnit::Px),
            CssValue::Length(200.0, LengthUnit::Px),
        )];
        let detected = detect_transitions(&style, &changes);
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].delay, 0.5);
    }

    #[test]
    fn cyclic_list_access() {
        let list = vec![0.3_f32, 0.5];
        assert_eq!(get_cyclic(&list, 0), Some(&0.3));
        assert_eq!(get_cyclic(&list, 1), Some(&0.5));
        // Wraps around
        assert_eq!(get_cyclic(&list, 2), Some(&0.3));
    }

    #[test]
    fn empty_transition_property() {
        let style = AnimStyle::default();
        let changes = vec![("opacity".into(), CssValue::Number(1.0), CssValue::Number(0.0))];
        let detected = detect_transitions(&style, &changes);
        assert!(detected.is_empty());
    }
}
