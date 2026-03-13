//! Animation engine — manages running animations and transitions.

use std::collections::HashMap;

use crate::instance::{AnimationInstance, TransitionInstance};
use crate::parse::KeyframesRule;
use crate::timeline::DocumentTimeline;

/// Entity identifier (mirrors `hecs::Entity` as `u64` bits).
type EntityId = u64;

/// Maximum number of concurrent animations per entity to prevent unbounded memory growth.
const MAX_ANIMATIONS_PER_ENTITY: usize = 256;

/// Maximum number of concurrent transitions per entity to prevent unbounded memory growth.
const MAX_TRANSITIONS_PER_ENTITY: usize = 256;

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
    ///
    /// Returns any `TransitionCancel` events that must be dispatched for
    /// in-progress transitions that are being replaced by this new transition.
    ///
    /// **Important**: Per CSS Transitions §5.3, when replacing an in-progress
    /// transition, the caller should use the **current animated value** (from
    /// `current_value()`) as the `from` value of the new `TransitionInstance`,
    /// not the original computed value. This ensures smooth reversal.
    pub fn add_transition(
        &mut self,
        entity: EntityId,
        transition: TransitionInstance,
    ) -> Vec<(EntityId, AnimationEvent)> {
        let transitions = self.transitions.entry(entity).or_default();
        if transitions.len() >= MAX_TRANSITIONS_PER_ENTITY {
            return Vec::new();
        }
        let mut cancel_events = Vec::new();
        // Check for an existing in-progress transition for the same property.
        // Per CSS Transitions §5.3, replacing a running transition fires
        // transitioncancel on the old transition.
        transitions.retain(|t| {
            if t.property == transition.property && !t.finished {
                cancel_events.push((
                    entity,
                    AnimationEvent::TransitionCancel {
                        property: t.property.clone(),
                        #[allow(clippy::cast_possible_truncation)]
                        elapsed_time: (t.elapsed as f32 - t.delay).max(0.0),
                    },
                ));
                false
            } else {
                t.property != transition.property
            }
        });
        transitions.push(transition);
        cancel_events
    }

    /// Add an animation instance for an entity.
    ///
    /// Silently drops the animation if the entity already has
    /// 256 animations, preventing unbounded growth.
    pub fn add_animation(&mut self, entity: EntityId, animation: AnimationInstance) {
        let anims = self.animations.entry(entity).or_default();
        if anims.len() >= MAX_ANIMATIONS_PER_ENTITY {
            return;
        }
        anims.push(animation);
    }

    /// Advance all animations/transitions by `dt` seconds.
    ///
    /// Returns a list of (entity, `event_type`) pairs for events that should
    /// be dispatched (e.g., `transitionend`, `animationend`).
    pub fn tick(&mut self, dt: f64) -> Vec<(EntityId, AnimationEvent)> {
        if !dt.is_finite() || dt < 0.0 {
            return Vec::new();
        }
        self.timeline.advance(dt);
        let mut events = Vec::new();

        Self::tick_transitions(&mut self.transitions, dt, &mut events);
        Self::tick_animations(&mut self.animations, dt, &mut events);

        // Clean up finished transitions (transitions always hold their final value
        // via the style system once complete, so they can be removed).
        self.transitions.retain(|_, v| {
            v.retain(|t| !(t.finished && t.end_event_dispatched));
            !v.is_empty()
        });
        // Clean up finished animations, but retain those with fill-mode forwards/both
        // so that progress() can continue to report the fill value.
        self.animations.retain(|_, v| {
            v.retain(|a| {
                if !a.finished || !a.end_event_dispatched {
                    return true;
                }
                // Keep animations that need to hold their fill value.
                matches!(
                    a.fill_mode(),
                    crate::style::AnimationFillMode::Forwards
                        | crate::style::AnimationFillMode::Both
                )
            });
            !v.is_empty()
        });

        events
    }

    /// Tick all transitions, emitting transition events.
    fn tick_transitions(
        transitions: &mut HashMap<EntityId, Vec<TransitionInstance>>,
        dt: f64,
        events: &mut Vec<(EntityId, AnimationEvent)>,
    ) {
        for (entity, trans_list) in transitions.iter_mut() {
            for trans in trans_list.iter_mut() {
                if trans.finished {
                    continue;
                }
                trans.elapsed += dt;

                // Dispatch transitionrun once — fired when the transition is
                // first ticked (CSS Transitions §6.1).
                if !trans.run_event_dispatched {
                    trans.run_event_dispatched = true;
                    events.push((
                        *entity,
                        AnimationEvent::TransitionRun {
                            property: trans.property.clone(),
                            elapsed_time: 0.0,
                        },
                    ));
                }

                let active_time = trans.elapsed - f64::from(trans.delay);

                // Dispatch transitionstart when the delay phase ends
                // (active_time >= 0), i.e., the transition is actually running.
                if active_time >= 0.0 && !trans.start_event_dispatched {
                    trans.start_event_dispatched = true;
                    events.push((
                        *entity,
                        AnimationEvent::TransitionStart {
                            property: trans.property.clone(),
                            elapsed_time: 0.0,
                        },
                    ));
                }

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
    }

    /// Tick all animations, emitting animation events.
    fn tick_animations(
        animations: &mut HashMap<EntityId, Vec<AnimationInstance>>,
        dt: f64,
        events: &mut Vec<(EntityId, AnimationEvent)>,
    ) {
        for (entity, anims) in animations.iter_mut() {
            for anim in anims.iter_mut() {
                if anim.finished || anim.play_state == crate::style::PlayState::Paused {
                    continue;
                }
                anim.elapsed += dt;
                let active_time = anim.elapsed - f64::from(anim.delay());
                if active_time < 0.0 {
                    continue;
                }

                // Dispatch animationstart once the delay has passed.
                if !anim.start_event_dispatched {
                    anim.start_event_dispatched = true;
                    events.push((
                        *entity,
                        AnimationEvent::AnimationStart {
                            name: anim.name().to_string(),
                            elapsed_time: {
                                let active_dur = match anim.iteration_count() {
                                    crate::style::IterationCount::Number(n) => n * anim.duration(),
                                    crate::style::IterationCount::Infinite => f32::INFINITY,
                                };
                                (-anim.delay()).clamp(0.0, active_dur)
                            },
                        },
                    ));
                }

                let total = match anim.iteration_count() {
                    crate::style::IterationCount::Number(n) => {
                        f64::from(n) * f64::from(anim.duration())
                    }
                    crate::style::IterationCount::Infinite => f64::INFINITY,
                };

                // Detect iteration changes and dispatch animationiteration.
                let dur = f64::from(anim.duration());
                if dur > 0.0 && active_time < total {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let new_iteration = (active_time / dur).floor().min(f64::from(u32::MAX)) as u32;
                    if new_iteration > anim.current_iteration {
                        // Cap iteration events per tick to prevent billions of
                        // events when dt is very large relative to duration.
                        const MAX_ITERATION_EVENTS_PER_TICK: u32 = 1000;
                        let emit_start = new_iteration
                            .saturating_sub(MAX_ITERATION_EVENTS_PER_TICK)
                            .max(anim.current_iteration + 1);
                        for iter in emit_start..=new_iteration {
                            #[allow(clippy::cast_precision_loss)]
                            events.push((
                                *entity,
                                AnimationEvent::AnimationIteration {
                                    name: anim.name().to_string(),
                                    elapsed_time: iter as f32 * anim.duration(),
                                },
                            ));
                        }
                        anim.current_iteration = new_iteration;
                    }
                }

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
    /// `transitionrun` event — fired when a transition is queued (before delay).
    TransitionRun {
        /// The property that is about to transition.
        property: String,
        /// Elapsed time at the point the event fires (always 0 when first queued).
        elapsed_time: f32,
    },
    /// `transitionstart` event — fired when the delay phase ends.
    TransitionStart {
        /// The property that started transitioning.
        property: String,
        /// Elapsed active time in seconds (0 at the start of the active phase).
        elapsed_time: f32,
    },
    /// `transitioncancel` event — fired when a transition is cancelled.
    TransitionCancel {
        /// The property whose transition was cancelled.
        property: String,
        /// Elapsed time in seconds at the point of cancellation.
        elapsed_time: f32,
    },
    /// `transitionend` event.
    TransitionEnd {
        /// The property that finished transitioning.
        property: String,
        /// Duration of the transition in seconds.
        elapsed_time: f32,
    },
    /// `animationstart` event.
    AnimationStart {
        /// The `@keyframes` name.
        name: String,
        /// Elapsed time in seconds (negative delay offset, clamped to 0).
        elapsed_time: f32,
    },
    /// `animationiteration` event.
    AnimationIteration {
        /// The `@keyframes` name.
        name: String,
        /// Elapsed time in seconds at the iteration boundary.
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
        let cancel_events = engine.add_transition(1, trans);
        assert!(cancel_events.is_empty(), "no existing transition to cancel");
        assert!(engine.has_active());

        // Tick halfway — emits transitionrun + transitionstart (no delay)
        let events = engine.tick(0.15);
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0].1,
            AnimationEvent::TransitionRun { property, .. } if property == "opacity"
        ));
        assert!(matches!(
            &events[1].1,
            AnimationEvent::TransitionStart { property, .. } if property == "opacity"
        ));
        assert_eq!(engine.active_transitions(1).len(), 1);

        // Tick to completion — emits only transitionend (run/start already dispatched)
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

        // During delay — transitionrun fires on first tick, but not transitionstart yet
        let events = engine.tick(0.1);
        assert_eq!(events.len(), 1, "only transitionrun during delay");
        assert!(matches!(
            &events[0].1,
            AnimationEvent::TransitionRun { property, .. } if property == "width"
        ));

        // Past delay — transitionstart fires
        let events = engine.tick(0.3);
        assert_eq!(events.len(), 1, "transitionstart when delay ends");
        assert!(matches!(
            &events[0].1,
            AnimationEvent::TransitionStart { property, .. } if property == "width"
        ));

        // Complete — transitionend fires
        let events = engine.tick(0.4);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0].1, AnimationEvent::TransitionEnd { .. }));
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

        // First tick past delay: emits AnimationStart (no delay, so starts immediately).
        let events = engine.tick(0.5);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0].1,
            AnimationEvent::AnimationStart { name, .. } if name == "fadeIn"
        ));

        // Second tick completes the animation: emits AnimationEnd.
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

        // Should never finish; animation start + iteration events are expected but
        // no AnimationEnd should ever be emitted.
        for _ in 0..100 {
            let events = engine.tick(0.5);
            assert!(
                events
                    .iter()
                    .all(|(_, e)| !matches!(e, AnimationEvent::AnimationEnd { .. })),
                "infinite animation should never emit AnimationEnd"
            );
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
        let cancel1 = engine.add_transition(1, t1);
        assert!(cancel1.is_empty(), "no previous transition to cancel");

        // Replace with new transition for same property — should emit TransitionCancel
        let t2 = TransitionInstance::new(
            "opacity".into(),
            CssValue::Number(0.5),
            CssValue::Number(0.0),
            0.5,
            0.0,
            TimingFunction::Linear,
        );
        let cancel2 = engine.add_transition(1, t2);
        assert_eq!(
            cancel2.len(),
            1,
            "one cancel event for the replaced transition"
        );
        assert!(matches!(
            &cancel2[0].1,
            AnimationEvent::TransitionCancel { property, .. } if property == "opacity"
        ));

        assert_eq!(engine.active_transitions(1).len(), 1);
    }

    #[test]
    fn engine_replace_finished_transition_no_cancel() {
        let mut engine = AnimationEngine::new();
        let mut t1 = TransitionInstance::new(
            "opacity".into(),
            CssValue::Number(1.0),
            CssValue::Number(0.0),
            0.1,
            0.0,
            TimingFunction::Linear,
        );
        // Mark as already finished — should not produce TransitionCancel
        t1.finished = true;
        let _ = engine.add_transition(1, t1);

        let t2 = TransitionInstance::new(
            "opacity".into(),
            CssValue::Number(0.5),
            CssValue::Number(0.0),
            0.5,
            0.0,
            TimingFunction::Linear,
        );
        let cancel = engine.add_transition(1, t2);
        assert!(
            cancel.is_empty(),
            "finished transition does not fire TransitionCancel"
        );
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
        let _ = engine.add_transition(42, trans);
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
        let _ = engine.add_transition(1, trans);
        engine.clear();
        assert!(!engine.has_active());
    }

    #[test]
    fn engine_default() {
        let engine = AnimationEngine::default();
        assert!(!engine.has_active());
    }

    #[test]
    fn engine_tick_nan_dt_is_noop() {
        let mut engine = AnimationEngine::new();
        let trans = TransitionInstance::new(
            "opacity".into(),
            CssValue::Number(1.0),
            CssValue::Number(0.0),
            1.0,
            0.0,
            TimingFunction::Linear,
        );
        let _ = engine.add_transition(1, trans);
        let events = engine.tick(f64::NAN);
        assert!(events.is_empty(), "NaN dt should produce no events");
        assert!(engine.has_active(), "transition should still be active");
    }

    #[test]
    fn engine_tick_negative_dt_is_noop() {
        let mut engine = AnimationEngine::new();
        let trans = TransitionInstance::new(
            "opacity".into(),
            CssValue::Number(1.0),
            CssValue::Number(0.0),
            1.0,
            0.0,
            TimingFunction::Linear,
        );
        let _ = engine.add_transition(1, trans);
        let events = engine.tick(-0.5);
        assert!(events.is_empty(), "negative dt should produce no events");
    }

    #[test]
    fn engine_animation_limit_enforced() {
        let mut engine = AnimationEngine::new();
        for i in 0..=MAX_ANIMATIONS_PER_ENTITY {
            let anim = AnimationInstance::new(
                format!("anim{i}"),
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
        }
        // Should cap at MAX_ANIMATIONS_PER_ENTITY
        assert_eq!(engine.active_animations(1).len(), MAX_ANIMATIONS_PER_ENTITY);
    }
}
