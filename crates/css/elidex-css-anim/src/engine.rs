//! Animation engine — manages running animations and transitions.

use std::collections::HashMap;

use crate::instance::{AnimationInstance, TransitionInstance};
use crate::parse::KeyframesRule;
use crate::timeline::DocumentTimeline;

/// Entity identifier (mirrors `hecs::Entity` as `u64` bits).
type EntityId = u64;

/// The animation engine ticks all running animations and transitions,
/// producing interpolated values for the style system.
#[derive(Debug)]
pub struct AnimationEngine {
    /// Document timeline.
    timeline: DocumentTimeline,
    /// Running transitions per entity.
    transitions: HashMap<EntityId, Vec<TransitionInstance>>,
    /// Running animation instances per entity.
    animations: HashMap<EntityId, Vec<AnimationInstance>>,
    /// Registered `@keyframes` rules by name.
    keyframes: HashMap<String, KeyframesRule>,
}

impl AnimationEngine {
    /// Create a new animation engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            timeline: DocumentTimeline::new(),
            transitions: HashMap::new(),
            animations: HashMap::new(),
            keyframes: HashMap::new(),
        }
    }

    /// Access the document timeline.
    #[must_use]
    pub fn timeline(&self) -> &DocumentTimeline {
        &self.timeline
    }

    /// Register a `@keyframes` rule.
    pub fn register_keyframes(&mut self, rule: KeyframesRule) {
        self.keyframes.insert(rule.name.clone(), rule);
    }

    /// Look up a `@keyframes` rule by name.
    #[must_use]
    pub fn get_keyframes(&self, name: &str) -> Option<&KeyframesRule> {
        self.keyframes.get(name)
    }

    /// Add a transition for an entity.
    pub fn add_transition(&mut self, entity: EntityId, transition: TransitionInstance) {
        let transitions = self.transitions.entry(entity).or_default();
        // Replace any existing transition for the same property
        transitions.retain(|t| t.property != transition.property);
        transitions.push(transition);
    }

    /// Add an animation instance for an entity.
    pub fn add_animation(&mut self, entity: EntityId, animation: AnimationInstance) {
        self.animations.entry(entity).or_default().push(animation);
    }

    /// Advance all animations/transitions by `dt` seconds.
    ///
    /// Returns a list of (entity, `event_type`) pairs for events that should
    /// be dispatched (e.g., `transitionend`, `animationend`).
    pub fn tick(&mut self, dt: f64) -> Vec<(EntityId, AnimationEvent)> {
        self.timeline.advance(dt);
        let mut events = Vec::new();

        // Tick transitions
        for (entity, transitions) in &mut self.transitions {
            for trans in transitions.iter_mut() {
                if trans.finished {
                    continue;
                }
                trans.elapsed += dt;
                let active_time = trans.elapsed - f64::from(trans.delay);
                if active_time >= f64::from(trans.duration) {
                    trans.finished = true;
                    if !trans.end_event_dispatched {
                        trans.end_event_dispatched = true;
                        events.push((
                            *entity,
                            AnimationEvent::TransitionEnd {
                                property: trans.property.clone(),
                                elapsed_time: trans.duration,
                            },
                        ));
                    }
                }
            }
        }

        // Tick animations
        for (entity, anims) in &mut self.animations {
            for anim in anims.iter_mut() {
                if anim.finished || anim.play_state == crate::style::PlayState::Paused {
                    continue;
                }
                anim.elapsed += dt;
                let active_time = anim.elapsed - f64::from(anim.delay());
                if active_time < 0.0 {
                    continue;
                }
                let total = match anim.iteration_count() {
                    crate::style::IterationCount::Number(n) => {
                        f64::from(n) * f64::from(anim.duration())
                    }
                    crate::style::IterationCount::Infinite => f64::INFINITY,
                };
                if active_time >= total && total.is_finite() {
                    anim.finished = true;
                    if !anim.end_event_dispatched {
                        anim.end_event_dispatched = true;
                        #[allow(clippy::cast_possible_truncation)]
                        events.push((
                            *entity,
                            AnimationEvent::AnimationEnd {
                                name: anim.name().to_string(),
                                elapsed_time: total as f32,
                            },
                        ));
                    }
                }
            }
        }

        // Clean up finished transitions/animations
        self.transitions.retain(|_, v| {
            v.retain(|t| !t.finished);
            !v.is_empty()
        });
        self.animations.retain(|_, v| {
            v.retain(|a| !a.finished);
            !v.is_empty()
        });

        events
    }

    /// Get all currently active transitions for an entity.
    #[must_use]
    pub fn active_transitions(&self, entity: EntityId) -> &[TransitionInstance] {
        self.transitions.get(&entity).map_or(&[], Vec::as_slice)
    }

    /// Get all currently active animations for an entity.
    #[must_use]
    pub fn active_animations(&self, entity: EntityId) -> &[AnimationInstance] {
        self.animations.get(&entity).map_or(&[], Vec::as_slice)
    }

    /// Returns `true` if any animations or transitions are running.
    #[must_use]
    pub fn has_active(&self) -> bool {
        !self.transitions.is_empty() || !self.animations.is_empty()
    }

    /// Remove all animations and transitions for an entity.
    pub fn remove_entity(&mut self, entity: EntityId) {
        self.transitions.remove(&entity);
        self.animations.remove(&entity);
    }

    /// Clear all state.
    pub fn clear(&mut self) {
        self.transitions.clear();
        self.animations.clear();
    }
}

impl Default for AnimationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// An animation/transition event to be dispatched to the DOM.
#[derive(Clone, Debug, PartialEq)]
pub enum AnimationEvent {
    /// `transitionend` event.
    TransitionEnd {
        /// The property that finished transitioning.
        property: String,
        /// Duration of the transition in seconds.
        elapsed_time: f32,
    },
    /// `animationend` event.
    AnimationEnd {
        /// The `@keyframes` name.
        name: String,
        /// Total active duration in seconds.
        elapsed_time: f32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::TransitionInstance;
    use crate::timing::TimingFunction;
    use elidex_plugin::{CssValue, LengthUnit};

    #[test]
    fn engine_add_and_tick_transition() {
        let mut engine = AnimationEngine::new();
        let trans = TransitionInstance::new(
            "opacity".into(),
            CssValue::Number(1.0),
            CssValue::Number(0.0),
            0.3,
            0.0,
            TimingFunction::Linear,
        );
        engine.add_transition(1, trans);
        assert!(engine.has_active());

        // Tick halfway
        let events = engine.tick(0.15);
        assert!(events.is_empty());
        assert_eq!(engine.active_transitions(1).len(), 1);

        // Tick to completion
        let events = engine.tick(0.2);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0].1,
            AnimationEvent::TransitionEnd { property, .. } if property == "opacity"
        ));

        // Transition removed after completion
        assert!(!engine.has_active());
    }

    #[test]
    fn engine_transition_with_delay() {
        let mut engine = AnimationEngine::new();
        let trans = TransitionInstance::new(
            "width".into(),
            CssValue::Length(100.0, LengthUnit::Px),
            CssValue::Length(200.0, LengthUnit::Px),
            0.5,
            0.2,
            TimingFunction::Linear,
        );
        engine.add_transition(1, trans);

        // During delay — no completion
        let events = engine.tick(0.1);
        assert!(events.is_empty());

        // Past delay, transition active
        let events = engine.tick(0.3);
        assert!(events.is_empty());

        // Complete
        let events = engine.tick(0.4);
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn engine_animation_end() {
        let mut engine = AnimationEngine::new();
        let anim = AnimationInstance::new(
            "fadeIn".into(),
            1.0,
            TimingFunction::Linear,
            0.0,
            crate::style::IterationCount::Number(1.0),
            crate::style::AnimationDirection::Normal,
            crate::style::AnimationFillMode::None,
            crate::style::PlayState::Running,
            0.0,
        );
        engine.add_animation(1, anim);

        let events = engine.tick(0.5);
        assert!(events.is_empty());

        let events = engine.tick(0.6);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0].1,
            AnimationEvent::AnimationEnd { name, .. } if name == "fadeIn"
        ));
    }

    #[test]
    fn engine_infinite_animation() {
        let mut engine = AnimationEngine::new();
        let anim = AnimationInstance::new(
            "spin".into(),
            1.0,
            TimingFunction::Linear,
            0.0,
            crate::style::IterationCount::Infinite,
            crate::style::AnimationDirection::Normal,
            crate::style::AnimationFillMode::None,
            crate::style::PlayState::Running,
            0.0,
        );
        engine.add_animation(1, anim);

        // Should never finish
        for _ in 0..100 {
            let events = engine.tick(0.5);
            assert!(events.is_empty());
        }
        assert!(engine.has_active());
    }

    #[test]
    fn engine_paused_animation() {
        let mut engine = AnimationEngine::new();
        let anim = AnimationInstance::new(
            "test".into(),
            1.0,
            TimingFunction::Linear,
            0.0,
            crate::style::IterationCount::Number(1.0),
            crate::style::AnimationDirection::Normal,
            crate::style::AnimationFillMode::None,
            crate::style::PlayState::Paused,
            0.0,
        );
        engine.add_animation(1, anim);

        // Paused: should not advance
        let events = engine.tick(2.0);
        assert!(events.is_empty());
        assert!(engine.has_active());
    }

    #[test]
    fn engine_replace_transition_same_property() {
        let mut engine = AnimationEngine::new();
        let t1 = TransitionInstance::new(
            "opacity".into(),
            CssValue::Number(1.0),
            CssValue::Number(0.0),
            1.0,
            0.0,
            TimingFunction::Linear,
        );
        engine.add_transition(1, t1);

        // Replace with new transition for same property
        let t2 = TransitionInstance::new(
            "opacity".into(),
            CssValue::Number(0.5),
            CssValue::Number(0.0),
            0.5,
            0.0,
            TimingFunction::Linear,
        );
        engine.add_transition(1, t2);

        assert_eq!(engine.active_transitions(1).len(), 1);
    }

    #[test]
    fn engine_remove_entity() {
        let mut engine = AnimationEngine::new();
        let trans = TransitionInstance::new(
            "opacity".into(),
            CssValue::Number(1.0),
            CssValue::Number(0.0),
            1.0,
            0.0,
            TimingFunction::Linear,
        );
        engine.add_transition(42, trans);
        assert!(engine.has_active());

        engine.remove_entity(42);
        assert!(!engine.has_active());
    }

    #[test]
    fn engine_register_keyframes() {
        let mut engine = AnimationEngine::new();
        let rule =
            crate::parse::parse_keyframes("fadeIn", "from { opacity: 0; } to { opacity: 1; }");
        engine.register_keyframes(rule);
        assert!(engine.get_keyframes("fadeIn").is_some());
        assert!(engine.get_keyframes("nonexistent").is_none());
    }

    #[test]
    fn engine_clear() {
        let mut engine = AnimationEngine::new();
        let trans = TransitionInstance::new(
            "opacity".into(),
            CssValue::Number(1.0),
            CssValue::Number(0.0),
            1.0,
            0.0,
            TimingFunction::Linear,
        );
        engine.add_transition(1, trans);
        engine.clear();
        assert!(!engine.has_active());
    }

    #[test]
    fn engine_default() {
        let engine = AnimationEngine::default();
        assert!(!engine.has_active());
    }
}
