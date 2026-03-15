//! Animation/transition event dispatch from the engine to the DOM.

use elidex_css_anim::engine::AnimationEvent;
use elidex_ecs::Entity;
use elidex_plugin::{AnimationEventInit, EventPayload, TransitionEventInit};
use elidex_script_session::DispatchEvent;

use super::ContentState;

/// Dispatch animation/transition events from the engine to the DOM.
///
/// Converts `AnimationEvent` from `elidex-css-anim` into `DispatchEvent`s
/// with appropriate event types and payloads (CSS Transitions Level 1 §6,
/// CSS Animations Level 1 §4.2).
pub(super) fn dispatch_animation_events(
    events: &[(u64, AnimationEvent)],
    state: &mut ContentState,
) {
    use elidex_css_anim::engine::{
        AnimationEventData, AnimationEventKind, TransitionEventData, TransitionEventKind,
    };

    for &(entity_bits, ref anim_event) in events {
        let entity = Entity::from_bits(entity_bits);
        let Some(entity) = entity else { continue };
        // Element may have been removed during animation.
        if !state.pipeline.dom.contains(entity) {
            continue;
        }

        let (event_type, payload) = match anim_event {
            AnimationEvent::Transition(TransitionEventData {
                kind,
                property,
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
                name,
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
        };

        // CSS transition/animation events bubble but are not cancelable.
        let mut dispatch = DispatchEvent::new_composed(event_type, entity);
        dispatch.cancelable = false;
        dispatch.payload = payload;

        state.pipeline.runtime.dispatch_event(
            &mut dispatch,
            &mut state.pipeline.session,
            &mut state.pipeline.dom,
            state.pipeline.document,
        );
    }
}
