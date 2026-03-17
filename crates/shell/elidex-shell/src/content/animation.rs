//! Animation/transition event dispatch from the engine to the DOM.

use elidex_css_anim::engine::AnimationEvent;

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
    crate::dispatch_anim_events(events, &mut state.pipeline);
}
