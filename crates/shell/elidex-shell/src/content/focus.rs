//! Focus management: focusability checks, focus/blur event dispatch,
//! and change-on-blur for text controls.
//!
//! Focus *state* is the canonical `ElementState::FOCUS` ECS component — read
//! via [`elidex_dom_api::focus::current_focus`] (with its connectedness
//! filter), written via [`elidex_dom_api::focus::set_focus_bit`] (clear-all
//! then set, so single-focus holds by construction). This module runs the
//! UA-input focusing / unfocusing steps (WHATWG HTML §6.6.4) — the event
//! dispatch the engine-independent helpers leave to the caller. The same
//! reconciler serves every UA input path (content thread, the legacy
//! single-thread `App`, and in-process iframes) by operating on a
//! `&mut PipelineResult` (each owns its own document `EcsDom`).

use elidex_dom_api::focus::{current_focus, set_focus_bit};
use elidex_ecs::Entity;
use elidex_form::{record_focus_snapshot, take_focus_snapshot, FormControlKind, FormControlState};
use elidex_plugin::{EventPayload, FocusEventInit};
use elidex_script_session::DispatchEvent;

use crate::PipelineResult;

/// Move focus to the given entity, clearing focus from the previous target.
///
/// Per UI Events §5.2, dispatches focusout/focusin (bubbling) then blur/focus
/// (non-bubbling). Only focusable elements receive focus (form controls, links
/// with href, elements with tabindex / contenteditable).
pub(crate) fn set_focus(pipeline: &mut PipelineResult, entity: Entity) {
    let old = current_focus(&pipeline.dom, pipeline.document);
    if old == Some(entity) {
        return;
    }

    // N5: Only focusable elements receive focus. Clicking a non-focusable
    // element blurs the current focus.
    if !is_focusable(&pipeline.dom, entity) {
        blur_current(pipeline);
        return;
    }

    // UI Events §5.2.1 order: focusout → focusin → blur → focus.
    // Per UI Events §5.2: relatedTarget is the element gaining/losing focus.
    // `current_focus` already filtered connectedness, so `old` is connected.
    if let Some(old) = old {
        dispatch_focus_event_with_related(pipeline, "focusout", old, true, Some(entity));
    }
    dispatch_focus_event_with_related(pipeline, "focusin", entity, true, old);
    if let Some(old) = old {
        // Clear the prior focus before the blur event so `activeElement`
        // reports `<body>` during blur (matching the prior field model).
        set_focus_bit(&mut pipeline.dom, None);
        dispatch_focus_event_with_related(pipeline, "blur", old, false, Some(entity));
        // HTML §4.10.5.5: "change" fires during the unfocusing steps (after blur).
        dispatch_change_on_blur(pipeline, old);
    }
    set_focus_bit(&mut pipeline.dom, Some(entity));
    dispatch_focus_event_with_related(pipeline, "focus", entity, false, old);

    // Record the focus-time value for change-on-blur (the engine-indep
    // `elidex_form` helper, shared with the VM `focus()` path so a script
    // `input.focus()` also seeds the snapshot).
    record_focus_snapshot(&mut pipeline.dom, entity);
}

/// Remove focus from the current target without setting a new one.
pub(crate) fn blur_current(pipeline: &mut PipelineResult) {
    let Some(old) = current_focus(&pipeline.dom, pipeline.document) else {
        // No focused element. A removed holder's `FOCUS` bit is already cleared
        // at removal (`EcsDom::fire_after_remove`, WHATWG HTML §2.1.4 removing
        // steps), and `focus()` cannot set it on a disconnected element (the
        // `is_focusable` connectedness gate), so the bit is connected by
        // construction — `current_focus` never misses a stale-detached holder,
        // and there is nothing to sweep here.
        return;
    };
    dispatch_focus_event_with_related(pipeline, "focusout", old, true, None);
    set_focus_bit(&mut pipeline.dom, None);
    dispatch_focus_event_with_related(pipeline, "blur", old, false, None);
    // HTML §4.10.5.5: "change" fires during the unfocusing steps (after blur).
    dispatch_change_on_blur(pipeline, old);
}

/// Dispatch a focus event with optional related target.
fn dispatch_focus_event_with_related(
    pipeline: &mut PipelineResult,
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
    pipeline.dispatch_event(&mut event);
}

/// Dispatch "change" on text-control blur when the value differs from the
/// snapshot taken at focus (HTML §4.10.5.5). Consumes (reads + removes) the
/// `FocusValueSnapshot`; absence ⇒ not a tracked text control ⇒ no change event.
fn dispatch_change_on_blur(pipeline: &mut PipelineResult, entity: Entity) {
    let Some(initial) = take_focus_snapshot(&mut pipeline.dom, entity) else {
        return;
    };
    let changed = pipeline
        .dom
        .world()
        .get::<&FormControlState>(entity)
        .ok()
        .is_some_and(|fcs| fcs.value() != initial);
    if changed {
        // "change" does NOT compose (does not cross shadow boundaries).
        let mut event = DispatchEvent::new("change", entity);
        event.cancelable = false;
        pipeline.dispatch_event(&mut event);
    }
}

/// Check if an element is focusable per HTML §6.6.2.
///
/// Form controls use the authoritative `FormControlState` (which captures
/// fieldset-inherited disabled — slot `#11-focusable-area-fieldset-inherited-disabled`
/// tracks bringing that to the engine-indep predicate for the VM path);
/// everything else delegates to the engine-independent focusable-area predicate.
pub(crate) fn is_focusable(dom: &elidex_ecs::EcsDom, entity: Entity) -> bool {
    // Form controls (not disabled, not hidden) are focusable per HTML §6.6.2.
    if let Ok(fcs) = dom.world().get::<&FormControlState>(entity) {
        // §6.6.2: a focusable area must be connected ("being rendered"). Gate
        // it HERE for the form-control branch; the non-form branch inherits the
        // same gate from the dom-api `is_focusable` below — so both branches
        // require connectedness without double-walking the connectedness chain
        // on the non-form path (one-issue-one-way: the predicate must not
        // diverge on connectedness across its branches).
        return dom.is_connected(entity) && !fcs.disabled && fcs.kind != FormControlKind::Hidden;
    }
    // Non-form-control: the engine-independent focusable-area predicate
    // (connectedness + tabindex / contenteditable / `<a href>`), one home
    // shared with the VM.
    elidex_dom_api::focus::is_focusable(dom, entity)
}
