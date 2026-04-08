//! Animation and transition detection, synchronization, and application.

use std::collections::HashMap;

use elidex_css::Stylesheet;
use elidex_css_anim::engine::{AnimationEngine, AnimationEvent};
use elidex_css_anim::style::AnimStyle;
use elidex_ecs::Entity;
use elidex_plugin::{AnimationEventInit, ComputedStyle, EventPayload, TransitionEventInit};
use elidex_script_session::DispatchEvent;

use crate::PipelineResult;

/// Register `@keyframes` rules from parsed stylesheets into the animation engine.
pub(crate) fn register_keyframes_from_stylesheets(
    stylesheets: &[Stylesheet],
    engine: &mut AnimationEngine,
) {
    for ss in stylesheets {
        for (name, body) in &ss.keyframes_raw {
            let rule = elidex_css_anim::parse::parse_keyframes(name, body);
            engine.register_keyframes(rule);
        }
    }
}

/// Create and initialize an `AnimationEngine` with `@keyframes` from stylesheets.
pub(crate) fn create_animation_engine(stylesheets: &[Stylesheet]) -> AnimationEngine {
    let mut engine = AnimationEngine::new();
    register_keyframes_from_stylesheets(stylesheets, &mut engine);
    engine
}

/// Collect old computed styles for entities that have an `AnimStyle` component.
///
/// Returns a list of `(entity_bits, AnimStyle, ComputedStyle)` tuples for
/// entities with transition properties, used by transition detection.
pub(crate) fn collect_old_anim_styles(
    dom: &elidex_ecs::EcsDom,
) -> Vec<(u64, AnimStyle, ComputedStyle)> {
    let mut result = Vec::new();
    for (entity, (anim_style, computed)) in &mut dom
        .world()
        .query::<(Entity, (&AnimStyle, &ComputedStyle))>()
    {
        if !anim_style.transition_property.is_empty() || !anim_style.animation_name.is_empty() {
            result.push((entity.to_bits().get(), anim_style.clone(), computed.clone()));
        }
    }
    result
}

/// Collect computed styles for entities that have `ComputedStyle` but no `AnimStyle`.
///
/// Used as baseline for entities that gain `AnimStyle` (transition-* properties)
/// during style re-resolution, so their first transition has a correct "from" value.
pub(crate) fn collect_computed_without_anim(
    dom: &elidex_ecs::EcsDom,
) -> HashMap<u64, ComputedStyle> {
    let has_anim: std::collections::HashSet<u64> = dom
        .world()
        .query::<(Entity, &AnimStyle)>()
        .iter()
        .map(|(e, _)| e.to_bits().get())
        .collect();

    let mut result = HashMap::new();
    for (entity, computed) in &mut dom.world().query::<(Entity, &ComputedStyle)>() {
        let bits = entity.to_bits().get();
        if !has_anim.contains(&bits) {
            result.insert(bits, computed.clone());
        }
    }
    result
}

/// Compare old vs new computed values to detect transitions, then add them to the engine.
///
/// Only compares properties listed in the entity's `transition-property` (or all
/// animatable properties when `transition-property: all`).
pub(crate) fn detect_and_start_transitions(
    result: &mut PipelineResult,
    old_styles: &[(u64, AnimStyle, ComputedStyle)],
    old_computed_no_anim: &HashMap<u64, ComputedStyle>,
) {
    use elidex_css_anim::style::TransitionProperty;

    let registry = &result.registry;
    let mut cancel_dispatches = Vec::new();

    for &(entity_bits, ref old_anim_style, ref old_computed) in old_styles {
        let entity = Entity::from_bits(entity_bits);
        let Some(entity) = entity else { continue };
        let new_computed = {
            let Ok(r) = result.dom.world().get::<&ComputedStyle>(entity) else {
                continue;
            };
            ComputedStyle::clone(&r)
        };

        let new_anim_style = result
            .dom
            .world()
            .get::<&AnimStyle>(entity)
            .ok()
            .map(|a| AnimStyle::clone(&a));

        // Union of old and new transition-property lists for change detection.
        let comparison = new_anim_style.as_ref().unwrap_or(old_anim_style);
        let prop_sources: &[&[TransitionProperty]] = &[
            &old_anim_style.transition_property,
            &comparison.transition_property,
        ];
        let params_style = new_anim_style.as_ref().unwrap_or(old_anim_style);

        detect_changed_and_add(
            entity_bits,
            old_computed,
            &new_computed,
            prop_sources,
            params_style,
            registry,
            &mut result.animation_engine,
            &mut cancel_dispatches,
        );
    }

    // Handle entities that newly gained AnimStyle (were not in old_styles).
    let old_bits: std::collections::HashSet<u64> =
        old_styles.iter().map(|&(bits, _, _)| bits).collect();
    for (entity, new_anim_style) in &mut result.dom.world().query::<(Entity, &AnimStyle)>() {
        let bits = entity.to_bits().get();
        if old_bits.contains(&bits) || new_anim_style.transition_property.is_empty() {
            continue;
        }
        let Some(old_computed) = old_computed_no_anim.get(&bits) else {
            continue;
        };
        let new_computed = {
            let Ok(c) = result.dom.world().get::<&ComputedStyle>(entity) else {
                continue;
            };
            ComputedStyle::clone(&c)
        };
        let prop_sources: &[&[TransitionProperty]] = &[&new_anim_style.transition_property];

        detect_changed_and_add(
            bits,
            old_computed,
            &new_computed,
            prop_sources,
            new_anim_style,
            registry,
            &mut result.animation_engine,
            &mut cancel_dispatches,
        );
    }

    dispatch_anim_events(&cancel_dispatches, result);
}

/// Synchronize CSS animations from `AnimStyle.animation_name` to the engine.
///
/// CSS Animations Level 1 4.2: when `animation-name` on an element changes,
/// animations for removed names are cancelled (emitting `animationcancel`) and
/// animations for new names are started.
///
/// `old_styles` contains the pre-re-resolve `AnimStyle` snapshot. For the initial
/// render pass an empty slice (all names are considered new).
pub(crate) fn sync_css_animations(
    result: &mut PipelineResult,
    old_styles: &[(u64, AnimStyle, ComputedStyle)],
) {
    use elidex_css_anim::instance::AnimationInstance;

    // Build lookup from entity_bits -> old animation names (ordered Vec).
    let old_names_map: HashMap<u64, Vec<String>> = old_styles
        .iter()
        .map(|(bits, anim_style, _)| (*bits, anim_style.animation_name.clone()))
        .collect();

    // Collect current AnimStyle data for all entities.
    let current: Vec<(u64, AnimStyle)> = result
        .dom
        .world()
        .query::<(Entity, &AnimStyle)>()
        .iter()
        .filter(|(_, anim_style)| !anim_style.animation_name.is_empty())
        .map(|(entity, anim_style)| (entity.to_bits().get(), anim_style.clone()))
        .collect();

    let mut cancel_events = Vec::new();
    let timeline_time = result.animation_engine.timeline().current_time();

    for (entity_bits, anim_style) in &current {
        let old_names_vec: Vec<&str> = old_names_map
            .get(entity_bits)
            .map_or_else(Vec::new, |names| names.iter().map(String::as_str).collect());
        let new_names_vec: Vec<&str> = anim_style
            .animation_name
            .iter()
            .map(String::as_str)
            .collect();

        // CSS Animations L1 4.2: "The same @keyframes rule name may be repeated...
        // each one is independently started." If the name list changed at all,
        // cancel all old animations and restart from scratch.
        if old_names_vec != new_names_vec {
            let events = result.animation_engine.cancel_animations(*entity_bits);
            cancel_events.extend(events);

            // Start all new animations.
            for (i, name) in anim_style.animation_name.iter().enumerate() {
                if name == "none" {
                    continue;
                }
                if result.animation_engine.get_keyframes(name).is_none() {
                    continue;
                }
                let spec = build_animation_spec(name, i, anim_style);
                let instance = AnimationInstance::new(&spec, timeline_time);
                result
                    .animation_engine
                    .add_animation(*entity_bits, instance);
            }
        }
    }

    // Cancel animations for entities whose animation-name was cleared entirely
    // (no longer in `current` because filter requires non-empty animation_name).
    let current_set: std::collections::HashSet<u64> =
        current.iter().map(|(bits, _)| *bits).collect();
    for (entity_bits, old_names) in &old_names_map {
        if !old_names.is_empty() && !current_set.contains(entity_bits) {
            let events = result.animation_engine.cancel_animations(*entity_bits);
            cancel_events.extend(events);
        }
    }

    if !cancel_events.is_empty() {
        dispatch_anim_events(&cancel_events, result);
    }
}

/// Build a `SingleAnimationSpec` from `AnimStyle` lists with CSS cycling.
///
/// CSS Animations L1 5.2: when animation property lists have different lengths,
/// values cycle (index mod `list.len()`).
fn build_animation_spec(
    name: &str,
    index: usize,
    anim_style: &AnimStyle,
) -> elidex_css_anim::SingleAnimationSpec {
    use elidex_css_anim::style::{
        AnimationDirection, AnimationFillMode, IterationCount, PlayState,
    };
    use elidex_css_anim::timing::TimingFunction;

    let cycle = |list_len: usize| -> usize {
        if list_len == 0 {
            0
        } else {
            index % list_len
        }
    };

    elidex_css_anim::SingleAnimationSpec {
        name: name.to_string(),
        duration: anim_style
            .animation_duration
            .get(cycle(anim_style.animation_duration.len()))
            .copied()
            .unwrap_or(0.0),
        timing_function: anim_style
            .animation_timing_function
            .get(cycle(anim_style.animation_timing_function.len()))
            .cloned()
            .unwrap_or(TimingFunction::EASE),
        delay: anim_style
            .animation_delay
            .get(cycle(anim_style.animation_delay.len()))
            .copied()
            .unwrap_or(0.0),
        iteration_count: anim_style
            .animation_iteration_count
            .get(cycle(anim_style.animation_iteration_count.len()))
            .copied()
            .unwrap_or(IterationCount::default()),
        direction: anim_style
            .animation_direction
            .get(cycle(anim_style.animation_direction.len()))
            .copied()
            .unwrap_or(AnimationDirection::default()),
        fill_mode: anim_style
            .animation_fill_mode
            .get(cycle(anim_style.animation_fill_mode.len()))
            .copied()
            .unwrap_or(AnimationFillMode::default()),
        play_state: anim_style
            .animation_play_state
            .get(cycle(anim_style.animation_play_state.len()))
            .copied()
            .unwrap_or(PlayState::default()),
    }
}

/// Compare old vs new computed values for the given transition-property lists,
/// detect transitions, and add them to the engine.
#[allow(clippy::too_many_arguments)]
fn detect_changed_and_add(
    entity_bits: u64,
    old_computed: &ComputedStyle,
    new_computed: &ComputedStyle,
    prop_sources: &[&[elidex_css_anim::style::TransitionProperty]],
    params_style: &AnimStyle,
    registry: &elidex_plugin::CssPropertyRegistry,
    engine: &mut AnimationEngine,
    cancel_dispatches: &mut Vec<(u64, AnimationEvent)>,
) {
    use elidex_css_anim::detection::detect_transitions;
    use elidex_css_anim::instance::TransitionInstance;
    use elidex_css_anim::style::TransitionProperty;

    let has_all = prop_sources
        .iter()
        .flat_map(|s| s.iter())
        .any(|p| matches!(p, TransitionProperty::All));

    let named_props: std::collections::HashSet<&str> = if has_all {
        std::collections::HashSet::new()
    } else {
        prop_sources
            .iter()
            .flat_map(|s| s.iter())
            .filter_map(|p| match p {
                TransitionProperty::Property(name) => Some(name.as_str()),
                _ => None,
            })
            .collect()
    };

    let mut changed = Vec::new();
    for prop_name in elidex_css_anim::interpolate::ANIMATABLE_PROPERTIES {
        if !has_all && !named_props.contains(prop_name) {
            continue;
        }
        let old_val = elidex_style::get_computed_with_registry(prop_name, old_computed, registry);
        let new_val = elidex_style::get_computed_with_registry(prop_name, new_computed, registry);
        if old_val != new_val {
            changed.push((prop_name.to_string(), old_val, new_val));
        }
    }

    if changed.is_empty() {
        return;
    }

    let detected = detect_transitions(params_style, &changed);
    for dt in detected {
        let trans = TransitionInstance::new(
            dt.property,
            dt.from,
            dt.to,
            dt.duration,
            dt.delay,
            dt.timing_function,
        );
        let events = engine.add_transition(entity_bits, trans);
        cancel_dispatches.extend(events);
    }
}

/// Dispatch animation/transition events to the DOM via `PipelineResult`.
///
/// Shared by `re_render` (cancel events during transition detection) and
/// `content/animation.rs` (all event kinds from engine tick).
pub(crate) fn dispatch_anim_events(events: &[(u64, AnimationEvent)], result: &mut PipelineResult) {
    for &(eid, ref event) in events {
        let entity = Entity::from_bits(eid);
        let Some(entity) = entity else { continue };
        if !result.dom.world().contains(entity) {
            continue;
        }
        let (event_type, payload) = anim_event_to_payload(event);
        let mut dispatch = DispatchEvent::new_composed(event_type, entity);
        dispatch.cancelable = false;
        dispatch.payload = payload;
        result.dispatch_event(&mut dispatch);
    }
}

/// Convert an `AnimationEvent` to its DOM event type string and payload.
pub(crate) fn anim_event_to_payload(event: &AnimationEvent) -> (&'static str, EventPayload) {
    use elidex_css_anim::engine::{
        AnimationEventData, AnimationEventKind, TransitionEventData, TransitionEventKind,
    };

    match event {
        AnimationEvent::Transition(TransitionEventData {
            kind,
            ref property,
            elapsed_time,
        }) => {
            let name = match kind {
                TransitionEventKind::Run => "transitionrun",
                TransitionEventKind::Start => "transitionstart",
                TransitionEventKind::End => "transitionend",
                TransitionEventKind::Cancel => "transitioncancel",
            };
            (
                name,
                EventPayload::Transition(TransitionEventInit {
                    property_name: property.clone(),
                    elapsed_time: *elapsed_time,
                    pseudo_element: String::new(),
                }),
            )
        }
        AnimationEvent::Animation(AnimationEventData {
            kind,
            ref name,
            elapsed_time,
        }) => {
            let event_name = match kind {
                AnimationEventKind::Start => "animationstart",
                AnimationEventKind::End => "animationend",
                AnimationEventKind::Iteration => "animationiteration",
                AnimationEventKind::Cancel => "animationcancel",
            };
            (
                event_name,
                EventPayload::Animation(AnimationEventInit {
                    animation_name: name.clone(),
                    elapsed_time: *elapsed_time,
                    pseudo_element: String::new(),
                }),
            )
        }
    }
}

/// Apply animated values from active transitions and animations to `ComputedStyle`.
///
/// Iterates over all entities with active transitions and animations in the engine,
/// reads the current interpolated values, and overwrites the corresponding
/// `ComputedStyle` fields.
pub(crate) fn apply_active_animations(result: &mut PipelineResult) {
    use elidex_css_anim::apply::apply_animated_value;

    // Collect entity IDs that have active transitions/animations (to avoid borrow conflict).
    // Animations are applied after transitions so they take priority when both affect
    // the same property (CSS Animations L1 4.1). Duplicates are safe (last write wins).
    let entity_ids: Vec<u64> = result.animation_engine.active_entity_ids().collect();

    for entity_bits in entity_ids {
        let entity = Entity::from_bits(entity_bits);
        let Some(entity) = entity else { continue };

        // Collect animated values from transitions.
        let transitions = result.animation_engine.active_transitions(entity_bits);
        let mut animated_values: Vec<(String, elidex_plugin::CssValue)> = transitions
            .iter()
            .filter_map(|t| t.current_value().map(|v| (t.property.clone(), v)))
            .collect();

        // Collect animated values from keyframe animations.
        // CSS Animations L1 5: read underlying computed values before applying
        // animation, so before-only keyframe properties interpolate correctly.
        let underlying_values: Option<HashMap<String, elidex_plugin::CssValue>> = result
            .dom
            .world()
            .get::<&ComputedStyle>(entity)
            .ok()
            .map(|style| {
                let mut map = HashMap::new();
                for prop in elidex_css_anim::interpolate::ANIMATABLE_PROPERTIES {
                    let v = elidex_style::get_computed(prop, &style);
                    if !matches!(v, elidex_plugin::CssValue::Keyword(ref k) if k == "initial") {
                        map.insert((*prop).to_string(), v);
                    }
                }
                map
            });

        let animations = result.animation_engine.active_animations(entity_bits);
        for anim in animations {
            if let Some(progress) = anim.progress() {
                let kf_values = result.animation_engine.keyframe_values(
                    anim.name(),
                    f64::from(progress),
                    Some(anim.timing_function()),
                    underlying_values.as_ref(),
                );
                animated_values.extend(kf_values);
            }
        }

        // Apply to ComputedStyle.
        if !animated_values.is_empty() {
            if let Ok(mut style) = result.dom.world_mut().get::<&mut ComputedStyle>(entity) {
                for (property, value) in &animated_values {
                    apply_animated_value(&mut style, property, value);
                }
            }
        }
    }
}
