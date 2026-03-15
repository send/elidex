//! IME (Input Method Editor) event handling.

use elidex_form::FormControlState;
use elidex_plugin::{CompositionEventInit, EventPayload};
use elidex_script_session::DispatchEvent;

use crate::ipc::ImeKind;

use super::form_input::dispatch_input_event_typed;
use super::ContentState;

/// Check if the focused entity is an editable text control.
fn is_editable_text(state: &ContentState, target: elidex_ecs::Entity) -> bool {
    state
        .pipeline
        .dom
        .world()
        .get::<&FormControlState>(target)
        .ok()
        .is_some_and(|fcs| fcs.kind.is_text_control() && !fcs.disabled && !fcs.readonly)
}

/// Handle an IME event from the browser thread.
#[allow(clippy::too_many_lines)]
pub(super) fn handle_ime(state: &mut ContentState, kind: ImeKind) {
    let Some(target) = state.focus_target else {
        return;
    };
    if !state.pipeline.dom.contains(target) {
        return;
    }

    match kind {
        ImeKind::Preedit(text) => {
            // Preedit only requires the control to be a text control (not readonly check
            // since composition display is read-only visual feedback).
            let (is_text_control, was_composing) = state
                .pipeline
                .dom
                .world()
                .get::<&FormControlState>(target)
                .ok()
                .map_or((false, false), |fcs| (
                    fcs.kind.is_text_control() && !fcs.disabled,
                    fcs.composition_text.is_some(),
                ));

            if is_text_control {

                if let Ok(mut fcs) = state
                    .pipeline
                    .dom
                    .world_mut()
                    .get::<&mut FormControlState>(target)
                {
                    const MAX_COMPOSITION_LEN: usize = 10_000;
                    fcs.composition_text = if text.is_empty() {
                        None
                    } else if text.len() > MAX_COMPOSITION_LEN {
                        Some(text.chars().take(MAX_COMPOSITION_LEN).collect())
                    } else {
                        Some(text.clone())
                    };
                }

                // Dispatch compositionstart when entering composition (M-5).
                // Per UI Events §5.7.2: compositionstart is cancelable.
                // The data property contains the currently selected text (S7).
                if !was_composing && !text.is_empty() {
                    let selected_text = state
                        .pipeline
                        .dom
                        .world()
                        .get::<&FormControlState>(target)
                        .ok()
                        .filter(|fcs| fcs.selection_start != fcs.selection_end)
                        .map(|fcs| {
                            let (s, e) = fcs.safe_selection_range();
                            fcs.value[s..e].to_string()
                        })
                        .unwrap_or_default();
                    let mut start_event = DispatchEvent::new_composed("compositionstart", target);
                    start_event.cancelable = true;
                    start_event.payload = EventPayload::Composition(CompositionEventInit {
                        data: selected_text,
                    });
                    state.pipeline.runtime.dispatch_event(
                        &mut start_event,
                        &mut state.pipeline.session,
                        &mut state.pipeline.dom,
                        state.pipeline.document,
                    );
                }

                // Dispatch compositionupdate event.
                let mut event = DispatchEvent::new_composed("compositionupdate", target);
                event.cancelable = false;
                event.payload = EventPayload::Composition(CompositionEventInit {
                    data: text,
                });
                state.pipeline.runtime.dispatch_event(
                    &mut event,
                    &mut state.pipeline.session,
                    &mut state.pipeline.dom,
                    state.pipeline.document,
                );

                // Re-render to show preedit text visually (P12).
                crate::re_render(&mut state.pipeline);
                state.send_display_list();
            }
        }
        ImeKind::Commit(text) => {
            if !is_editable_text(state, target) {
                return;
            }

            {
                // Clear composition and insert the committed text.
                // If there is an active selection, delete it first (F-09).
                if let Ok(mut fcs) = state
                    .pipeline
                    .dom
                    .world_mut()
                    .get::<&mut FormControlState>(target)
                {
                    fcs.composition_text = None;
                    if fcs.selection_start != fcs.selection_end {
                        elidex_form::delete_selection(&mut fcs);
                    }
                    // Enforce maxlength on committed text (P13).
                    let insert_text = if let Some(max) = fcs.maxlength {
                        let available = max.saturating_sub(fcs.char_count);
                        if available == 0 {
                            String::new()
                        } else {
                            text.chars().take(available).collect()
                        }
                    } else {
                        text.clone()
                    };
                    if !insert_text.is_empty() {
                        let pos = fcs.safe_cursor_pos();
                        fcs.value.insert_str(pos, &insert_text);
                        fcs.cursor_pos = pos + insert_text.len();
                        fcs.dirty_value = true;
                        fcs.update_char_count();
                    }
                }

                // Dispatch compositionend event.
                let mut end_event = DispatchEvent::new_composed("compositionend", target);
                end_event.cancelable = false;
                end_event.payload = EventPayload::Composition(CompositionEventInit {
                    data: text.clone(),
                });
                state.pipeline.runtime.dispatch_event(
                    &mut end_event,
                    &mut state.pipeline.session,
                    &mut state.pipeline.dom,
                    state.pipeline.document,
                );

                // Dispatch input event.
                dispatch_input_event_typed(state, target, "insertFromComposition", Some(&text));
            }

            state.reset_caret_blink();
            crate::re_render(&mut state.pipeline);
            state.send_display_list();
        }
        ImeKind::Enabled | ImeKind::Disabled => {
            // No-op for now.
        }
    }
}
