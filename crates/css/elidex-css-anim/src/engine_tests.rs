use super::*;
use crate::instance::TransitionInstance;
use crate::timing::TimingFunction;
use elidex_plugin::{CssValue, LengthUnit};

#[test]
fn has_running_false_when_all_finished() {
    let mut engine = AnimationEngine::new();
    let spec = make_anim_spec(
        "test",
        0.1,
        crate::style::IterationCount::Number(1.0),
        crate::style::PlayState::Running,
    );
    let anim = AnimationInstance::new(&spec, 0.0);
    engine.add_animation(1, anim);
    assert!(engine.has_running());

    // Tick past completion.
    engine.tick(0.2);
    // has_active is true (animation retained for fill cleanup), but has_running is false.
    assert!(
        !engine.has_running(),
        "has_running should be false when all animations are finished"
    );
}

#[test]
fn has_running_true_while_active() {
    let mut engine = AnimationEngine::new();
    let trans = TransitionInstance::new(
        "opacity".into(),
        CssValue::Number(0.0),
        CssValue::Number(1.0),
        1.0,
        0.0,
        TimingFunction::Linear,
    );
    let _ = engine.add_transition(1, trans);
    assert!(engine.has_running());
}

fn make_anim_spec(
    name: &str,
    duration: f32,
    iteration_count: crate::style::IterationCount,
    play_state: crate::style::PlayState,
) -> crate::SingleAnimationSpec {
    crate::SingleAnimationSpec {
        name: name.into(),
        duration,
        timing_function: TimingFunction::Linear,
        delay: 0.0,
        iteration_count,
        direction: crate::style::AnimationDirection::Normal,
        fill_mode: crate::style::AnimationFillMode::None,
        play_state,
    }
}

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
        AnimationEvent::Transition(TransitionEventData { kind: TransitionEventKind::Run, property, .. }) if property == "opacity"
    ));
    assert!(matches!(
        &events[1].1,
        AnimationEvent::Transition(TransitionEventData { kind: TransitionEventKind::Start, property, .. }) if property == "opacity"
    ));
    assert_eq!(engine.active_transitions(1).len(), 1);

    // Tick to completion — emits only transitionend (run/start already dispatched)
    let events = engine.tick(0.2);
    assert_eq!(events.len(), 1);
    assert!(matches!(
        &events[0].1,
        AnimationEvent::Transition(TransitionEventData { kind: TransitionEventKind::End, property, .. }) if property == "opacity"
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
        AnimationEvent::Transition(TransitionEventData { kind: TransitionEventKind::Run, property, .. }) if property == "width"
    ));

    // Past delay — transitionstart fires
    let events = engine.tick(0.3);
    assert_eq!(events.len(), 1, "transitionstart when delay ends");
    assert!(matches!(
        &events[0].1,
        AnimationEvent::Transition(TransitionEventData { kind: TransitionEventKind::Start, property, .. }) if property == "width"
    ));

    // Complete — transitionend fires
    let events = engine.tick(0.4);
    assert_eq!(events.len(), 1);
    assert!(matches!(
        &events[0].1,
        AnimationEvent::Transition(TransitionEventData {
            kind: TransitionEventKind::End,
            ..
        })
    ));
}

#[test]
fn engine_animation_end() {
    let mut engine = AnimationEngine::new();
    let spec = make_anim_spec(
        "fadeIn",
        1.0,
        crate::style::IterationCount::Number(1.0),
        crate::style::PlayState::Running,
    );
    let anim = AnimationInstance::new(&spec, 0.0);
    engine.add_animation(1, anim);

    // First tick past delay: emits AnimationStart (no delay, so starts immediately).
    let events = engine.tick(0.5);
    assert_eq!(events.len(), 1);
    assert!(matches!(
        &events[0].1,
        AnimationEvent::Animation(AnimationEventData { kind: AnimationEventKind::Start, name, .. }) if name == "fadeIn"
    ));

    // Second tick completes the animation: emits AnimationEnd.
    let events = engine.tick(0.6);
    assert_eq!(events.len(), 1);
    assert!(matches!(
        &events[0].1,
        AnimationEvent::Animation(AnimationEventData { kind: AnimationEventKind::End, name, .. }) if name == "fadeIn"
    ));
}

#[test]
fn engine_infinite_animation() {
    let mut engine = AnimationEngine::new();
    let spec = make_anim_spec(
        "spin",
        1.0,
        crate::style::IterationCount::Infinite,
        crate::style::PlayState::Running,
    );
    let anim = AnimationInstance::new(&spec, 0.0);
    engine.add_animation(1, anim);

    // Should never finish; animation start + iteration events are expected but
    // no AnimationEnd should ever be emitted.
    for _ in 0..100 {
        let events = engine.tick(0.5);
        assert!(
            events.iter().all(|(_, e)| !matches!(
                e,
                AnimationEvent::Animation(AnimationEventData {
                    kind: AnimationEventKind::End,
                    ..
                })
            )),
            "infinite animation should never emit AnimationEnd"
        );
    }
    assert!(engine.has_active());
}

#[test]
fn engine_paused_animation() {
    let mut engine = AnimationEngine::new();
    let spec = make_anim_spec(
        "test",
        1.0,
        crate::style::IterationCount::Number(1.0),
        crate::style::PlayState::Paused,
    );
    let anim = AnimationInstance::new(&spec, 0.0);
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
        AnimationEvent::Transition(TransitionEventData { kind: TransitionEventKind::Cancel, property, .. }) if property == "opacity"
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
    let rule = crate::parse::parse_keyframes("fadeIn", "from { opacity: 0; } to { opacity: 1; }");
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
        let spec = make_anim_spec(
            &format!("anim{i}"),
            1.0,
            crate::style::IterationCount::Number(1.0),
            crate::style::PlayState::Running,
        );
        let anim = AnimationInstance::new(&spec, 0.0);
        engine.add_animation(1, anim);
    }
    // Should cap at MAX_ANIMATIONS_PER_ENTITY
    assert_eq!(engine.active_animations(1).len(), MAX_ANIMATIONS_PER_ENTITY);
}

#[test]
fn keyframe_values_per_keyframe_timing() {
    // Per CSS Animations L1 §3.9.1: animation-timing-function inside a
    // keyframe block applies to the interval starting at that keyframe.
    let mut engine = AnimationEngine::new();
    let rule = crate::parse::parse_keyframes(
        "test",
        "from { opacity: 0; animation-timing-function: steps(1, end); } to { opacity: 1; }",
    );
    // Verify the from keyframe has a per-keyframe timing function.
    assert!(
        rule.keyframes[0].timing_function.is_some(),
        "from keyframe should have per-keyframe timing function"
    );
    engine.register_keyframes(rule);

    // With steps(1, end), the interval [0%, 100%) maps to 0.0 and at 100% maps to 1.0.
    // At progress=0.5 (halfway), steps(1, end) should output 0.0 (step hasn't fired yet).
    let values = engine.keyframe_values("test", 0.5, Some(&TimingFunction::Linear), None);
    let opacity_val = values.iter().find(|(p, _)| p == "opacity");
    assert!(opacity_val.is_some(), "should have opacity value");
    let (_, val) = opacity_val.unwrap();
    // steps(1, end) at t=0.5 → 0.0, so interpolated opacity = 0 + (1-0)*0.0 = 0.0
    match val {
        CssValue::Number(n) => assert!(
            (*n - 0.0).abs() < 0.01,
            "expected ~0.0 with steps(1, end) at t=0.5, got {n}"
        ),
        CssValue::Length(n, _) => assert!(
            (*n - 0.0).abs() < 0.01,
            "expected ~0.0 with steps(1, end) at t=0.5, got {n}"
        ),
        other => panic!("unexpected value type: {other:?}"),
    }
}

#[test]
fn keyframe_values_fallback_to_anim_timing() {
    // When keyframe has no per-keyframe timing, the animation's timing applies.
    let mut engine = AnimationEngine::new();
    let rule = crate::parse::parse_keyframes("test", "from { opacity: 0; } to { opacity: 1; }");
    assert!(
        rule.keyframes[0].timing_function.is_none(),
        "from keyframe should have no per-keyframe timing"
    );
    engine.register_keyframes(rule);

    // With steps(1, end) as the animation-level timing, at progress=0.5 opacity should be 0.
    let values = engine.keyframe_values(
        "test",
        0.5,
        Some(&TimingFunction::Steps(
            1,
            crate::timing::StepPosition::JumpEnd,
        )),
        None,
    );
    let opacity_val = values.iter().find(|(p, _)| p == "opacity");
    assert!(opacity_val.is_some());
    let (_, val) = opacity_val.unwrap();
    match val {
        CssValue::Number(n) => assert!(
            (*n - 0.0).abs() < 0.01,
            "expected ~0.0 with steps(1, end) anim timing at t=0.5, got {n}"
        ),
        CssValue::Length(n, _) => assert!((*n - 0.0).abs() < 0.01, "expected ~0.0, got {n}"),
        other => panic!("unexpected value type: {other:?}"),
    }
}

#[test]
fn keyframe_values_no_timing_uses_linear() {
    // When no timing function is provided at all, raw progress is used (linear).
    let mut engine = AnimationEngine::new();
    let rule = crate::parse::parse_keyframes("test", "from { width: 0px; } to { width: 100px; }");
    engine.register_keyframes(rule);

    let values = engine.keyframe_values("test", 0.5, None, None);
    let width_val = values.iter().find(|(p, _)| p == "width");
    assert!(width_val.is_some());
    let (_, val) = width_val.unwrap();
    match val {
        CssValue::Length(n, _) => assert!(
            (*n - 50.0).abs() < 0.5,
            "expected ~50.0 with no timing (linear), got {n}"
        ),
        other => panic!("unexpected value type: {other:?}"),
    }
}

#[test]
fn has_running_cache_invalidated() {
    let mut engine = AnimationEngine::new();
    assert!(!engine.has_running());

    let trans = TransitionInstance::new(
        "opacity".into(),
        CssValue::Number(0.0),
        CssValue::Number(1.0),
        1.0,
        0.0,
        TimingFunction::Linear,
    );
    let _ = engine.add_transition(1, trans);
    // Cache should be invalidated by add_transition.
    assert!(engine.has_running());

    // Tick past completion.
    engine.tick(2.0);
    assert!(!engine.has_running());

    // add_animation invalidates.
    let spec = make_anim_spec(
        "test",
        0.5,
        crate::style::IterationCount::Number(1.0),
        crate::style::PlayState::Running,
    );
    engine.add_animation(2, AnimationInstance::new(&spec, 0.0));
    assert!(engine.has_running());

    // remove_entity invalidates.
    engine.remove_entity(2);
    assert!(!engine.has_running());
}
