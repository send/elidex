//! Focus management: focusability checks, focus/blur event dispatch,
//! and change-on-blur for text controls.

use elidex_ecs::ElementState as DomElementState;
use elidex_ecs::Entity;
use elidex_form::{FormControlKind, FormControlState};
use elidex_plugin::{EventPayload, FocusEventInit};
use elidex_script_session::DispatchEvent;

use crate::app::hover::update_element_state;

use super::ContentState;

/// Move focus to the given entity, clearing focus from the previous target.
///
/// Per UI Events §5.2, dispatches focusout/focusin (bubbling) then blur/focus (non-bubbling).
/// Only focusable elements receive focus (form controls, links with href, elements with tabindex).
pub(super) fn set_focus(state: &mut ContentState, entity: Entity) {
    if state.focus_target == Some(entity) {
        return;
    }

    // N5: Only focusable elements receive focus.
    if !is_focusable(&state.pipeline.dom, entity) {
        // Clicking a non-focusable element blurs the current focus.
        blur_current(state);
        return;
    }

    let old_focus = state.focus_target;

    // UI Events §5.2.1 order: focusout → focusin → blur → focus.
    // Per UI Events §5.2: relatedTarget is the element gaining/losing focus.
    if let Some(old) = old_focus {
        if state.pipeline.dom.contains(old) {
            dispatch_focus_event_with_related(state, "focusout", old, true, Some(entity));
        }
    }
    dispatch_focus_event_with_related(state, "focusin", entity, true, old_focus);
    if let Some(old) = old_focus {
        if state.pipeline.dom.contains(old) {
            update_element_state(&mut state.pipeline.dom, old, |s| {
                s.remove(DomElementState::FOCUS);
            });
            dispatch_focus_event_with_related(state, "blur", old, false, Some(entity));
            // HTML §4.10.5.4: "change" fires during unfocusing steps (after blur).
            dispatch_change_on_blur(state, old);
        }
    }
    update_element_state(&mut state.pipeline.dom, entity, |s| {
        s.insert(DomElementState::FOCUS);
    });
    dispatch_focus_event_with_related(state, "focus", entity, false, old_focus);
    state.focus_target = Some(entity);

    // Record initial value for change event detection on blur.
    state.focus_initial_value = state
        .pipeline
        .dom
        .world()
        .get::<&FormControlState>(entity)
        .ok()
        .filter(|fcs| fcs.kind.is_text_control())
        .map(|fcs| fcs.value().to_string());
}

/// Remove focus from the current target without setting a new one.
pub(super) fn blur_current(state: &mut ContentState) {
    let Some(old) = state.focus_target.take() else {
        return;
    };
    if state.pipeline.dom.contains(old) {
        dispatch_focus_event_with_related(state, "focusout", old, true, None);
        update_element_state(&mut state.pipeline.dom, old, |s| {
            s.remove(DomElementState::FOCUS);
        });
        dispatch_focus_event_with_related(state, "blur", old, false, None);
        // HTML §4.10.5.4: "change" fires during unfocusing steps (after blur).
        dispatch_change_on_blur(state, old);
    }
    state.focus_initial_value = None;
}

/// Dispatch a focus event with optional related target.
fn dispatch_focus_event_with_related(
    state: &mut ContentState,
    event_type: &str,
    target: Entity,
    bubbles: bool,
    related_target: Option<Entity>,
) {
    let mut event = DispatchEvent::new_composed(event_type, target);
    event.cancelable = false;
    event.bubbles = bubbles;
    event.payload = EventPayload::Focus(FocusEventInit {
        related_target: related_target.map(|e| e.to_bits().get()),
    });
    state.pipeline.runtime.dispatch_event(
        &mut event,
        &mut state.pipeline.session,
        &mut state.pipeline.dom,
        state.pipeline.document,
    );
}

/// Dispatch "change" event on text control blur when value differs from initial.
fn dispatch_change_on_blur(state: &mut ContentState, entity: Entity) {
    let Some(initial) = &state.focus_initial_value else {
        return;
    };
    if !state.pipeline.dom.contains(entity) {
        return;
    }
    let changed = state
        .pipeline
        .dom
        .world()
        .get::<&FormControlState>(entity)
        .ok()
        .is_some_and(|fcs| fcs.value() != *initial);
    if changed {
        // "change" event does NOT compose (does not cross shadow boundaries).
        let mut event = DispatchEvent::new("change", entity);
        event.cancelable = false;
        state.pipeline.runtime.dispatch_event(
            &mut event,
            &mut state.pipeline.session,
            &mut state.pipeline.dom,
            state.pipeline.document,
        );
    }
}

/// Check if an element is focusable per HTML spec §6.6.
///
/// Focusable elements: form controls (not disabled), `<a>` with `href`,
/// elements with `tabindex` attribute.
pub(super) fn is_focusable(dom: &elidex_ecs::EcsDom, entity: Entity) -> bool {
    // Form controls (not disabled, not hidden) are focusable per HTML §6.6.3.
    if let Ok(fcs) = dom.world().get::<&FormControlState>(entity) {
        return !fcs.disabled && fcs.kind != FormControlKind::Hidden;
    }
    // Check for tabindex attribute, contenteditable, or <a> with href.
    let Ok(attrs) = dom.world().get::<&elidex_ecs::Attributes>(entity) else {
        return false;
    };
    if attrs.contains("tabindex") {
        return true;
    }
    // HTML §6.6.3: contenteditable elements are focusable.
    if dom.is_contenteditable(entity) {
        return true;
    }
    let Ok(tag) = dom.world().get::<&elidex_ecs::TagType>(entity) else {
        return false;
    };
    tag.0 == "a" && attrs.contains("href")
}
