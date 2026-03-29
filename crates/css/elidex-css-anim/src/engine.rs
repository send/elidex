//! Animation engine — manages running animations and transitions.

use std::collections::HashMap;

use crate::instance::{AnimationInstance, TransitionInstance};
use crate::parse::KeyframesRule;
use crate::timeline::DocumentTimeline;

use crate::EntityId;

/// Maximum number of concurrent animations per entity to prevent unbounded memory growth.
const MAX_ANIMATIONS_PER_ENTITY: usize = 256;

/// Maximum number of concurrent transitions per entity to prevent unbounded memory growth.
const MAX_TRANSITIONS_PER_ENTITY: usize = 256;

/// Maximum total events emitted per `tick()` call across all entities.
const MAX_EVENTS_PER_TICK: usize = 10_000;

/// Cap iteration events per tick to prevent billions of events when dt is
/// very large relative to duration.
const MAX_ITERATION_EVENTS_PER_TICK: u32 = 1000;

/// Epsilon for comparing keyframe offsets (floating-point tolerance).
const KEYFRAME_OFFSET_EPSILON: f32 = 1e-6;

/// The animation engine ticks all running animations and transitions,
/// producing interpolated values for the style system.
///
/// **Important**: Callers must call [`remove_entity()`](Self::remove_entity)
/// when an element is destroyed to prevent memory leaks. Animations with
/// `fill-mode: forwards` or `both` are intentionally retained after finishing
/// (to hold the fill value), so they will accumulate unless explicitly removed.
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
    /// Cached result of `has_running()`. `None` means invalidated (needs recomputation).
    has_running_cache: Option<bool>,
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
            has_running_cache: None,
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
    /// **Caller responsibility**: the returned cancel events contain entity IDs
    /// that must be validated against the live DOM before dispatch (the entity
    /// may have been destroyed between transition start and replacement).
    #[must_use]
    pub fn add_transition(
        &mut self,
        entity: EntityId,
        transition: TransitionInstance,
    ) -> Vec<(EntityId, AnimationEvent)> {
        self.has_running_cache = None;
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
                    AnimationEvent::Transition(TransitionEventData {
                        kind: TransitionEventKind::Cancel,
                        property: t.property.clone(),
                        elapsed_time: (t.elapsed - f64::from(t.delay))
                            .max(0.0)
                            .min(f64::from(t.duration)),
                    }),
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
    /// Drops the animation with a warning if the entity already has
    /// 256 animations, preventing unbounded growth.
    pub fn add_animation(&mut self, entity: EntityId, animation: AnimationInstance) {
        self.has_running_cache = None;
        let anims = self.animations.entry(entity).or_default();
        if anims.len() >= MAX_ANIMATIONS_PER_ENTITY {
            #[cfg(debug_assertions)]
            eprintln!(
                "elidex-css-anim: animation limit ({MAX_ANIMATIONS_PER_ENTITY}) reached for entity {entity}, dropping new animation"
            );
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
        self.has_running_cache = None;
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
            v.retain(should_retain_animation);
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
        'outer: for (entity, trans_list) in transitions.iter_mut() {
            for trans in trans_list.iter_mut() {
                if events.len() >= MAX_EVENTS_PER_TICK {
                    break 'outer;
                }
                if trans.finished {
                    continue;
                }
                trans.elapsed += dt;

                // CSS Transitions L2 §6.1: elapsedTime = max(min(-delay, duration), 0).
                let delay_elapsed = f64::from(-trans.delay)
                    .min(f64::from(trans.duration))
                    .max(0.0);

                // Dispatch transitionrun once — fired when the transition is
                // first ticked (CSS Transitions §6.1).
                if !trans.run_event_dispatched {
                    trans.run_event_dispatched = true;
                    push_transition_event(
                        events,
                        *entity,
                        TransitionEventKind::Run,
                        &trans.property,
                        delay_elapsed,
                    );
                }

                let active_time = trans.elapsed - f64::from(trans.delay);

                // Dispatch transitionstart when the delay phase ends
                // (active_time >= 0), i.e., the transition is actually running.
                if active_time >= 0.0 && !trans.start_event_dispatched {
                    trans.start_event_dispatched = true;
                    push_transition_event(
                        events,
                        *entity,
                        TransitionEventKind::Start,
                        &trans.property,
                        delay_elapsed,
                    );
                }

                if active_time >= f64::from(trans.duration) {
                    trans.finished = true;
                    if !trans.end_event_dispatched {
                        trans.end_event_dispatched = true;
                        push_transition_event(
                            events,
                            *entity,
                            TransitionEventKind::End,
                            &trans.property,
                            f64::from(trans.duration),
                        );
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
        'outer: for (entity, anims) in animations.iter_mut() {
            for anim in anims.iter_mut() {
                if events.len() >= MAX_EVENTS_PER_TICK {
                    break 'outer;
                }
                if anim.finished || anim.play_state == crate::style::PlayState::Paused {
                    continue;
                }
                anim.elapsed += dt;
                let active_time = anim.elapsed - f64::from(anim.delay());
                if active_time < 0.0 {
                    continue;
                }

                Self::emit_animation_start(*entity, anim, events);

                let total = Self::total_active_duration(anim);

                Self::emit_iteration_events(*entity, anim, active_time, total, events);

                Self::check_animation_end(*entity, anim, active_time, total, events);
            }
        }
    }

    /// Emit `animationstart` when the delay phase ends.
    /// For negative delays, the animation starts partway into the active phase,
    /// so `elapsedTime = min(|delay|, active_duration)`. Per CSS Animations L1 §4.3.2.
    fn emit_animation_start(
        entity: EntityId,
        anim: &mut AnimationInstance,
        events: &mut Vec<(EntityId, AnimationEvent)>,
    ) {
        if anim.start_event_dispatched {
            return;
        }
        anim.start_event_dispatched = true;
        // CSS Animations L1 §4.2: elapsedTime = max(min(-delay, active_duration), 0).
        let active_duration = Self::total_active_duration(anim);
        let start_elapsed = f64::from(-anim.delay()).min(active_duration).max(0.0);
        events.push((
            entity,
            AnimationEvent::Animation(AnimationEventData {
                kind: AnimationEventKind::Start,
                name: anim.name().to_string(),
                elapsed_time: start_elapsed,
            }),
        ));
    }

    /// Detect iteration changes and emit `animationiteration` events.
    fn emit_iteration_events(
        entity: EntityId,
        anim: &mut AnimationInstance,
        active_time: f64,
        total: f64,
        events: &mut Vec<(EntityId, AnimationEvent)>,
    ) {
        let dur = f64::from(anim.duration());
        if dur <= 0.0 || active_time >= total {
            return;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let new_iteration = (active_time / dur).floor().min(f64::from(u32::MAX)) as u32;
        if new_iteration <= anim.current_iteration {
            return;
        }
        // Cap iteration events per tick to prevent DoS from large dt values.
        // Per CSS Animations L1 §4.6, all iterations should fire events;
        // this is a pragmatic limit for safety.
        let emit_start = new_iteration
            .saturating_sub(MAX_ITERATION_EVENTS_PER_TICK)
            .max(anim.current_iteration + 1);
        let emit_end = new_iteration.min(emit_start.saturating_add(MAX_ITERATION_EVENTS_PER_TICK));
        if emit_start > emit_end {
            return;
        }
        for iter in emit_start..=emit_end {
            let elapsed = f64::from(iter) * f64::from(anim.duration());
            let elapsed_time = if elapsed.is_finite() {
                elapsed
            } else {
                f64::MAX
            };
            events.push((
                entity,
                AnimationEvent::Animation(AnimationEventData {
                    kind: AnimationEventKind::Iteration,
                    name: anim.name().to_string(),
                    elapsed_time,
                }),
            ));
        }
        anim.current_iteration = new_iteration;
    }

    /// Check if the animation has ended and emit `animationend` if so.
    fn check_animation_end(
        entity: EntityId,
        anim: &mut AnimationInstance,
        active_time: f64,
        total: f64,
        events: &mut Vec<(EntityId, AnimationEvent)>,
    ) {
        if active_time < total || !total.is_finite() {
            return;
        }
        anim.finished = true;
        if anim.end_event_dispatched {
            return;
        }
        anim.end_event_dispatched = true;
        events.push((
            entity,
            AnimationEvent::Animation(AnimationEventData {
                kind: AnimationEventKind::End,
                name: anim.name().to_string(),
                elapsed_time: total.max(0.0),
            }),
        ));
    }

    /// Compute the total active duration for an animation.
    fn total_active_duration(anim: &AnimationInstance) -> f64 {
        match anim.iteration_count() {
            crate::style::IterationCount::Number(n) => f64::from(n) * f64::from(anim.duration()),
            crate::style::IterationCount::Infinite => f64::INFINITY,
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

    /// Returns `true` if any animations or transitions exist (including finished with fill).
    #[must_use]
    pub fn has_active(&self) -> bool {
        !self.transitions.is_empty() || !self.animations.is_empty()
    }

    /// Returns `true` if any animations or transitions are still running (not finished).
    ///
    /// Unlike [`has_active`](Self::has_active), this returns `false` when all remaining
    /// entries are finished (e.g. fill-mode:forwards holding final values), preventing
    /// the frame loop from spinning at 60fps indefinitely.
    ///
    /// Uses an internal cache that is invalidated by mutating methods (`tick`,
    /// `add_transition`, `add_animation`, `remove_entity`, `cancel_animations`,
    /// `clear`) for amortized O(1) lookups.
    #[must_use]
    pub fn has_running(&mut self) -> bool {
        if let Some(cached) = self.has_running_cache {
            return cached;
        }
        let result = self
            .transitions
            .values()
            .any(|v| v.iter().any(|t| !t.finished))
            || self
                .animations
                .values()
                .any(|v| v.iter().any(|a| !a.finished));
        self.has_running_cache = Some(result);
        result
    }

    /// Returns an iterator over all entity IDs that have active transitions or animations.
    ///
    /// May contain duplicates; callers that apply values idempotently (like
    /// `apply_animated_value`) do not need deduplication.
    pub fn active_entity_ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.transitions
            .keys()
            .chain(self.animations.keys())
            .copied()
    }

    /// Look up keyframe values for a named animation at a given progress.
    ///
    /// Finds the surrounding keyframes and interpolates each declared property.
    /// Returns an empty `Vec` if the animation name is not registered.
    ///
    /// `anim_timing` is the animation-level timing function, used as fallback
    /// when a keyframe does not declare its own `animation-timing-function`.
    /// Per CSS Animations L1 §3.9.1, per-keyframe timing functions apply to
    /// the interval starting at the `before` keyframe.
    ///
    /// `underlying_values`, when provided, supplies the element's non-animated
    /// computed values. Properties declared only in the `before` keyframe will
    /// interpolate toward the underlying value per CSS Animations L1 §5.
    #[must_use]
    pub fn keyframe_values(
        &self,
        name: &str,
        progress: f64,
        anim_timing: Option<&crate::timing::TimingFunction>,
        underlying_values: Option<&std::collections::HashMap<String, elidex_plugin::CssValue>>,
    ) -> Vec<(String, elidex_plugin::CssValue)> {
        let Some(rule) = self.keyframes.get(name) else {
            return Vec::new();
        };
        if rule.keyframes.is_empty() {
            return Vec::new();
        }

        #[allow(clippy::cast_possible_truncation)]
        let p = progress.clamp(0.0, 1.0) as f32;

        // Find surrounding keyframes.
        let (before, after) = find_surrounding_keyframes(&rule.keyframes, p);

        // If same keyframe, just return its declarations directly.
        if (before.offset - after.offset).abs() < KEYFRAME_OFFSET_EPSILON {
            return before
                .declarations
                .iter()
                .map(|d| (d.property.clone(), d.value.clone()))
                .collect();
        }

        // Interpolate between surrounding keyframes.
        let range = after.offset - before.offset;
        let raw_local_t = if range.abs() < KEYFRAME_OFFSET_EPSILON {
            1.0
        } else {
            let t = (p - before.offset) / range;
            // Guard against NaN/Infinity from near-zero division.
            if t.is_finite() {
                t.clamp(0.0, 1.0)
            } else {
                1.0
            }
        };

        // CSS Animations L1 §3.9.1: apply per-keyframe timing function if present,
        // otherwise fall back to the animation's overall timing function.
        let interval_timing = before.timing_function.as_ref().or(anim_timing);
        let local_t = if let Some(tf) = interval_timing {
            tf.sample(raw_local_t)
        } else {
            raw_local_t
        };

        // Build a lookup from `before` declarations for O(1) access.
        let before_map: std::collections::HashMap<&str, &elidex_plugin::CssValue> = before
            .declarations
            .iter()
            .map(|d| (d.property.as_str(), &d.value))
            .collect();

        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();
        // For each property declared in `after`, try to find matching `from` in `before`.
        for decl in &after.declarations {
            seen.insert(decl.property.as_str());
            if let Some(from) = before_map.get(decl.property.as_str()) {
                if let Some(interp) =
                    crate::interpolate::interpolate(from, &decl.value, local_t, &decl.property)
                {
                    result.push((decl.property.clone(), interp));
                }
            } else {
                result.push((decl.property.clone(), decl.value.clone()));
            }
        }
        // CSS Animations L1 §5: properties only in `before` interpolate
        // toward the underlying (non-animated) computed value.
        for decl in &before.declarations {
            if !seen.contains(decl.property.as_str()) {
                if let Some(underlying) = underlying_values.and_then(|uv| uv.get(&decl.property)) {
                    if let Some(interp) = crate::interpolate::interpolate(
                        &decl.value,
                        underlying,
                        local_t,
                        &decl.property,
                    ) {
                        result.push((decl.property.clone(), interp));
                    } else {
                        result.push((decl.property.clone(), decl.value.clone()));
                    }
                } else {
                    result.push((decl.property.clone(), decl.value.clone()));
                }
            }
        }

        // Deduplicate by property name (last wins), in case the same property
        // appears multiple times within a single keyframe's declarations.
        deduplicate_by_property(&mut result);

        result
    }

    /// Remove all animations and transitions for an entity.
    ///
    /// Must be called when an element is destroyed to prevent memory leaks.
    /// Animations with `fill-mode: forwards/both` are retained after finishing
    /// and will not be cleaned up by `tick()`, so this is the only way to
    /// reclaim their memory.
    pub fn remove_entity(&mut self, entity: EntityId) {
        self.has_running_cache = None;
        self.transitions.remove(&entity);
        self.animations.remove(&entity);
    }

    /// Remove all animation/transition state for entities that no longer exist.
    ///
    /// Call this after DOM mutations (e.g. `session.flush()`) to prevent memory
    /// leaks from destroyed entities whose animations/transitions would
    /// otherwise persist indefinitely.
    pub fn prune_dead_entities(&mut self, alive: &dyn Fn(EntityId) -> bool) {
        self.has_running_cache = None;
        self.transitions.retain(|id, _| alive(*id));
        self.animations.retain(|id, _| alive(*id));
    }

    /// Remove keyframes rules that are no longer referenced by any active animation.
    ///
    /// Call periodically to prevent unbounded growth of the keyframes store
    /// from script-initiated `element.animate()` calls.
    pub fn prune_unused_keyframes(&mut self) {
        let referenced: std::collections::HashSet<&str> = self
            .animations
            .values()
            .flat_map(|anims| anims.iter().map(AnimationInstance::name))
            .collect();
        self.keyframes
            .retain(|name, _| referenced.contains(name.as_str()));
    }

    /// Cancel all running animations for an entity, emitting `animationcancel` events.
    ///
    /// Returns cancel events for all non-finished animations. Used when
    /// `display: none` is set or `animation-name` changes.
    #[must_use]
    pub fn cancel_animations(&mut self, entity: EntityId) -> Vec<(EntityId, AnimationEvent)> {
        self.has_running_cache = None;
        let mut events = Vec::new();
        if let Some(anims) = self.animations.remove(&entity) {
            for anim in &anims {
                if anim.finished {
                    continue;
                }
                let active_duration = Self::total_active_duration(anim);
                let active_time = anim.elapsed - f64::from(anim.delay());
                let elapsed_time = if active_time.is_finite() {
                    active_time.min(active_duration).max(0.0)
                } else {
                    0.0
                };
                events.push((
                    entity,
                    AnimationEvent::Animation(AnimationEventData {
                        kind: AnimationEventKind::Cancel,
                        name: anim.name().to_string(),
                        elapsed_time,
                    }),
                ));
            }
        }
        events
    }

    /// Cancel animations by name for an entity, emitting `animationcancel` events.
    ///
    /// Only cancels animations whose name is in `names_to_cancel`. Returns cancel
    /// events. Animations not in the set are retained.
    #[must_use]
    pub fn cancel_animations_by_name(
        &mut self,
        entity: EntityId,
        names_to_cancel: &std::collections::HashSet<&str>,
    ) -> Vec<(EntityId, AnimationEvent)> {
        self.has_running_cache = None;
        let mut events = Vec::new();
        if let Some(anims) = self.animations.get_mut(&entity) {
            let mut i = 0;
            while i < anims.len() {
                if names_to_cancel.contains(anims[i].name()) {
                    let anim = anims.remove(i);
                    // Only emit animationcancel for non-finished animations.
                    // Finished animations completed naturally (animationend already fired).
                    if !anim.finished {
                        let active_duration = Self::total_active_duration(&anim);
                        let active_time = anim.elapsed - f64::from(anim.delay());
                        let elapsed_time = if active_time.is_finite() {
                            active_time.min(active_duration).max(0.0)
                        } else {
                            0.0
                        };
                        events.push((
                            entity,
                            AnimationEvent::Animation(AnimationEventData {
                                kind: AnimationEventKind::Cancel,
                                name: anim.name().to_string(),
                                elapsed_time,
                            }),
                        ));
                    }
                } else {
                    i += 1;
                }
            }
            if anims.is_empty() {
                self.animations.remove(&entity);
            }
        }
        events
    }

    /// Clear all state.
    pub fn clear(&mut self) {
        self.has_running_cache = None;
        self.transitions.clear();
        self.animations.clear();
    }
}

/// Deduplicate property values by name, keeping the last occurrence (last wins).
fn deduplicate_by_property(values: &mut Vec<(String, elidex_plugin::CssValue)>) {
    if values.len() <= 1 {
        return;
    }
    let mut seen = std::collections::HashSet::new();
    // Walk from the end so the last occurrence is kept.
    let mut i = values.len();
    while i > 0 {
        i -= 1;
        if !seen.insert(values[i].0.clone()) {
            values.remove(i);
        }
    }
}

/// Find the surrounding keyframes for a given progress value.
///
/// Returns `(before, after)` where `before.offset <= progress <= after.offset`.
/// Assumes keyframes are sorted by offset.
fn find_surrounding_keyframes(
    keyframes: &[crate::parse::Keyframe],
    progress: f32,
) -> (&crate::parse::Keyframe, &crate::parse::Keyframe) {
    debug_assert!(
        !keyframes.is_empty(),
        "find_surrounding_keyframes called with empty keyframes"
    );
    let last = keyframes.len().saturating_sub(1);
    // Find the first keyframe with offset > progress.
    let after_idx = keyframes
        .iter()
        .position(|k| k.offset > progress + KEYFRAME_OFFSET_EPSILON)
        .unwrap_or(last);
    let before_idx = if after_idx > 0 { after_idx - 1 } else { 0 };
    (&keyframes[before_idx], &keyframes[after_idx.min(last)])
}

/// Push a transition event to the events list.
fn push_transition_event(
    events: &mut Vec<(EntityId, AnimationEvent)>,
    entity: EntityId,
    kind: TransitionEventKind,
    property: &str,
    elapsed_time: f64,
) {
    events.push((
        entity,
        AnimationEvent::Transition(TransitionEventData {
            kind,
            property: property.to_string(),
            elapsed_time,
        }),
    ));
}

/// Returns `true` if the animation should be kept in the active list.
fn should_retain_animation(a: &AnimationInstance) -> bool {
    if !a.finished {
        return true;
    }
    if !a.end_event_dispatched {
        return true;
    }
    // Keep animations that need to hold their fill value.
    matches!(
        a.fill_mode(),
        crate::style::AnimationFillMode::Forwards | crate::style::AnimationFillMode::Both
    )
}

impl Default for AnimationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// An animation/transition event to be dispatched to the DOM.
#[derive(Clone, Debug, PartialEq)]
pub enum AnimationEvent {
    /// A CSS transition event.
    Transition(TransitionEventData),
    /// A CSS animation event.
    Animation(AnimationEventData),
}

/// Data for a CSS transition event.
#[derive(Clone, Debug, PartialEq)]
pub struct TransitionEventData {
    /// The kind of transition event.
    pub kind: TransitionEventKind,
    /// The property being transitioned.
    pub property: String,
    /// Elapsed time in seconds at the point the event fires.
    pub elapsed_time: f64,
}

/// The kind of a CSS transition event.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TransitionEventKind {
    /// `transitionrun` — fired when a transition is queued (before delay).
    Run,
    /// `transitionstart` — fired when the delay phase ends.
    Start,
    /// `transitionend` — fired when the transition completes.
    End,
    /// `transitioncancel` — fired when a transition is cancelled.
    Cancel,
}

/// Data for a CSS animation event.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimationEventData {
    /// The kind of animation event.
    pub kind: AnimationEventKind,
    /// The `@keyframes` name.
    pub name: String,
    /// Elapsed time in seconds at the point the event fires.
    pub elapsed_time: f64,
}

/// The kind of a CSS animation event.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AnimationEventKind {
    /// `animationstart` — fired when the animation starts (after delay).
    Start,
    /// `animationend` — fired when the animation completes.
    End,
    /// `animationiteration` — fired at iteration boundaries.
    Iteration,
    /// `animationcancel` — fired when an animation is aborted
    /// (e.g., display:none, animation-name change).
    Cancel,
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
