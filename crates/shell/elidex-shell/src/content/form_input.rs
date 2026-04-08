//! Form control interaction: checkbox toggle, label click, and event dispatch.

use elidex_ecs::ElementState as DomElementState;
use elidex_ecs::Entity;
use elidex_form::{FormControlKind, FormControlState};
use elidex_plugin::{EventPayload, MouseEventInit};
use elidex_script_session::DispatchEvent;

use crate::app::hover::update_element_state;

use super::focus::set_focus;
use super::ContentState;

/// Toggle a checkbox's checked state on click.
///
/// Returns `true` if the checkbox was actually toggled (enabled checkbox).
/// HTML spec §4.10.5.4: disabled controls do not respond to user interaction.
pub(super) fn toggle_checkbox_if_needed(dom: &mut elidex_ecs::EcsDom, entity: Entity) -> bool {
    let is_enabled_checkbox = dom
        .world()
        .get::<&FormControlState>(entity)
        .ok()
        .is_some_and(|fcs| fcs.kind == FormControlKind::Checkbox && !fcs.disabled);

    if !is_enabled_checkbox {
        return false;
    }

    let new_checked = if let Ok(mut fcs) = dom.world_mut().get::<&mut FormControlState>(entity) {
        fcs.checked = !fcs.checked;
        fcs.checked
    } else {
        return false;
    };

    // Sync ElementState CHECKED flag for CSS :checked matching.
    update_element_state(dom, entity, |s| {
        s.set(DomElementState::CHECKED, new_checked);
    });
    true
}

/// Handle label click: dispatch a synthetic click on the associated form control.
///
/// Per HTML spec §4.10.4, activating a label dispatches a click event on the
/// target control. This simplified implementation dispatches a synthetic click
/// and handles focus + checkbox toggle as side effects.
pub(super) fn handle_label_click(
    state: &mut ContentState,
    clicked_entity: Entity,
    already_toggled: bool,
) {
    // Walk up to find a <label> ancestor (or the clicked entity itself).
    let label_entity = find_label_ancestor(&state.pipeline.dom, clicked_entity);
    let Some(label_entity) = label_entity else {
        return;
    };

    let Some(target) = elidex_form::find_label_target(&state.pipeline.dom, label_entity) else {
        return;
    };

    set_focus(state, target);

    // Dispatch a synthetic click event on the target control.
    let mut synthetic_click = DispatchEvent::new_composed("click", target);
    synthetic_click.payload = EventPayload::Mouse(MouseEventInit::default());
    let prevented = state.pipeline.dispatch_event(&mut synthetic_click);

    // If the target is a checkbox, toggle it (unless already toggled by direct click
    // or the synthetic click was prevented).
    if !already_toggled && !prevented && toggle_checkbox_if_needed(&mut state.pipeline.dom, target)
    {
        dispatch_state_change_events(state, target);
    }
}

/// Dispatch "input" then "change" events for a checkbox/radio state change.
///
/// HTML spec §4.10.5.4: after state change, fire input then change.
pub(super) fn dispatch_state_change_events(state: &mut ContentState, target: Entity) {
    dispatch_input_event(state, target);
    // DOM spec: "change" event is NOT composed (does not cross shadow boundaries).
    let mut change_event = DispatchEvent::new("change", target);
    change_event.cancelable = false;
    state.pipeline.dispatch_event(&mut change_event);
}

/// Dispatch an "input" event on the given entity (HTML spec §4.10.5.5).
///
/// Per UI Events spec, `InputEvent` is always composed (bubbles across
/// shadow boundaries). This applies to both checkbox/radio toggles and
/// text control edits.
pub(super) fn dispatch_input_event(state: &mut ContentState, target: Entity) {
    dispatch_input_event_typed(state, target, "", None);
}

/// Dispatch an "input" event with `inputType` and data (`InputEvent` interface).
pub(super) fn dispatch_input_event_typed(
    state: &mut ContentState,
    target: Entity,
    input_type: &str,
    data: Option<&str>,
) {
    let mut event = DispatchEvent::new_composed("input", target);
    event.cancelable = false;
    // Per Input Events Level 2: always set InputEvent payload.
    // For checkbox/radio, inputType is "" and data is null.
    event.payload = EventPayload::Input(elidex_plugin::InputEventInit {
        input_type: input_type.to_string(),
        data: data.map(String::from),
        is_composing: false,
    });
    state.pipeline.dispatch_event(&mut event);
}

/// Handle implicit form submission (Enter on text input/password).
///
/// Per HTML §4.10.15.3: implicit submission finds the form ancestor,
/// dispatches a "submit" event, and if not prevented, logs the form data.
///
/// Sandbox `allow-forms` enforcement: if this document is inside a sandboxed
/// iframe without `allow-forms`, form submission is silently blocked
/// (WHATWG HTML §4.8.5).
pub(super) fn handle_form_submit(state: &mut ContentState, target: Entity) {
    // Sandbox allow-forms check: block form submission in sandboxed iframes
    // that do not have the allow-forms flag.
    if !state.pipeline.runtime.bridge().forms_allowed() {
        return;
    }
    let Some(form_entity) = elidex_form::find_form_ancestor(&state.pipeline.dom, target) else {
        return;
    };

    // Dispatch the "submit" event on the form element.
    let mut submit_event = DispatchEvent::new("submit", form_entity);
    submit_event.cancelable = true;
    let prevented = state.pipeline.dispatch_event(&mut submit_event);

    if !prevented {
        let mut submission =
            elidex_form::build_form_submission(&state.pipeline.dom, form_entity, Some(target));
        // WHATWG §4.10.15.3 step 5: submitter formaction/formmethod override.
        if let Ok(attrs) = state
            .pipeline
            .dom
            .world()
            .get::<&elidex_ecs::Attributes>(target)
        {
            if let Some(fa) = attrs.get("formaction") {
                if !fa.is_empty() {
                    submission.action = fa.to_string();
                }
            }
            if let Some(fm) = attrs.get("formmethod") {
                let fm_lower = fm.to_ascii_lowercase();
                if fm_lower == "get" || fm_lower == "post" {
                    submission.method = fm_lower;
                }
            }
        }
        // WHATWG §4.10.15.3 step 7: empty action → current document URL.
        let action = if submission.action.is_empty() {
            state
                .pipeline
                .url
                .as_ref()
                .map(std::string::ToString::to_string)
                .unwrap_or_default()
        } else {
            submission.action.clone()
        };
        if !action.is_empty() {
            if let Some(target_url) = build_submission_url(state, &submission, &action) {
                execute_submission(state, &submission, &target_url);
            }
        }
    }
}

/// Resolve and build the submission target URL.
///
/// For GET: replaces query with form-encoded data, preserves fragment from action URL
/// (WHATWG §4.10.15.3 step 11).
/// For POST: preserves existing query, strips fragment.
fn build_submission_url(
    state: &ContentState,
    submission: &elidex_form::FormSubmission,
    action: &str,
) -> Option<url::Url> {
    let resolved = crate::app::navigation::resolve_nav_url(state.pipeline.url.as_ref(), action)?;
    let encoded = elidex_form::encode_form_urlencoded(&submission.data);

    if submission.method == "get" {
        let mut target_url = resolved;
        // WHATWG §4.10.15.3: GET replaces the query entirely (no appending).
        target_url.set_query(if encoded.is_empty() {
            None
        } else {
            Some(&encoded)
        });
        // WHATWG §4.10.15.3 step 11: fragment is preserved from the action URL.
        Some(target_url)
    } else {
        let mut target_url = resolved;
        target_url.set_fragment(None);
        Some(target_url)
    }
}

/// Execute the form submission (navigate with GET or POST).
fn execute_submission(
    state: &mut ContentState,
    submission: &elidex_form::FormSubmission,
    target_url: &url::Url,
) {
    if submission.method == "get" {
        super::navigation::handle_navigate(state, target_url, false, None);
    } else if submission.method == "post" {
        if submission.enctype == "multipart/form-data" {
            tracing::warn!(
                action = %submission.action,
                "multipart/form-data enctype not yet supported, falling back to urlencoded"
            );
        }
        let encoded = elidex_form::encode_form_urlencoded(&submission.data);
        tracing::debug!(
            action = %submission.action,
            entries = submission.data.len(),
            "POST form submission"
        );
        let request = elidex_net::Request {
            method: "POST".to_string(),
            url: target_url.clone(),
            headers: vec![(
                "Content-Type".to_string(),
                "application/x-www-form-urlencoded".to_string(),
            )],
            body: bytes::Bytes::from(encoded),
        };
        super::navigation::handle_navigate(state, target_url, false, Some(request));
    }
}

/// Handle form reset (reset button click).
///
/// Per HTML §4.10.15.4: dispatches a "reset" event, then restores default values.
pub(super) fn handle_form_reset(state: &mut ContentState, target: Entity) {
    let Some(form_entity) = elidex_form::find_form_ancestor(&state.pipeline.dom, target) else {
        return;
    };

    let mut reset_event = DispatchEvent::new("reset", form_entity);
    reset_event.cancelable = true;
    let prevented = state.pipeline.dispatch_event(&mut reset_event);

    if !prevented {
        elidex_form::reset_form(&mut state.pipeline.dom, form_entity);
    }
}

/// Walk up ancestors to find a `<label>` element.
fn find_label_ancestor(dom: &elidex_ecs::EcsDom, entity: Entity) -> Option<Entity> {
    if elidex_form::is_label(dom, entity) {
        return Some(entity);
    }
    let mut current = dom.get_parent(entity);
    for _ in 0..elidex_ecs::MAX_ANCESTOR_DEPTH {
        let e = current?;
        if elidex_form::is_label(dom, e) {
            return Some(e);
        }
        current = dom.get_parent(e);
    }
    None
}
