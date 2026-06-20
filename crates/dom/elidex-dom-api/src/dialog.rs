//! Engine-independent `<dialog>` algorithms (HTML §4.11.4).
//!
//! Today this is the state-mutation portion of the **"close the
//! dialog"** algorithm, shared by the two callers that close a dialog:
//! the `HTMLDialogElement.close()` IDL method (VM host) and
//! `<form method=dialog>` submission (shell, HTML §4.10.22.3 step 11).
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", the spec **algorithm** lives in
//! this engine-independent crate; the VM host / shell are thin callers.
//! Following the `elidex_form::reset_form` precedent (caller fires the
//! DOM event, the algorithm mutates), [`close_the_dialog`] performs only
//! the ECS state mutation and does **not** fire the `close` DOM event —
//! DOM-event dispatch needs the full event-dispatch machinery, which
//! each caller reaches through a different entry point (VM
//! `dispatch_simple_event` / shell `pipeline.dispatch_event`). Keeping
//! the event at the caller makes close-the-dialog consistent with every
//! other elidex algorithm that mutates and fires an event
//! (`reset`/`submit`/`input`/`change`).

use elidex_ecs::{DialogReturnValue, EcsDom, Entity, IsModalDialog};

/// HTML §4.11.4 "close the dialog" — state-mutation portion.
///
/// Closes the dialog `subject`:
/// - **step 1**: if `subject` has no `open` attribute, return early
///   (returns `false` — nothing closed, so the caller must NOT fire
///   `close`).
/// - **step 9**: if `result` is `Some`, set `subject`'s `returnValue`
///   to it (a `None` result leaves `returnValue` unchanged — this is the
///   spec's null/non-null distinction, e.g. `dialog.close()` with no arg
///   or a `<form method=dialog>` submit with no submitter value).
/// - **step 8**: set `is modal` false (remove the [`IsModalDialog`]
///   marker; a no-op for a non-modal dialog).
/// - **step 5**: remove the `open` attribute via the canonical
///   [`EcsDom::remove_attribute`] chokepoint, which bumps `rev_version`
///   and dispatches `MutationEvent::AttributeChange` (so a JS
///   MutationObserver observes the close).
///
/// Returns `true` if the dialog was open (and thus closed). The caller
/// fires the `close` event (close-the-dialog step 13) iff this returns
/// `true`.
///
/// Steps 2/4 (`beforetoggle` + the dialog toggle task), step 6 (top
/// layer), and step 12 (focus restoration) are pre-existing omissions in
/// elidex's dialog implementation (`#11-dialog-top-layer`,
/// `#11-canonical-focus-update-steps`), preserved here. step 13's `close`
/// is fired synchronously by the caller rather than via the queued
/// element task.
pub fn close_the_dialog(dom: &mut EcsDom, subject: Entity, result: Option<&str>) -> bool {
    // step 1: no `open` attribute → nothing to close.
    if !dom.has_attribute(subject, "open") {
        return false;
    }
    // step 9: result non-null → set returnValue (insert-or-update the
    // ECS component on the dialog entity).
    if let Some(value) = result {
        // Scope the `&mut DialogReturnValue` borrow so it is dropped
        // before the insert fallback (the `Result` temporary would
        // otherwise keep `world` borrowed across `insert_one`).
        let updated = {
            if let Ok(mut existing) = dom.world_mut().get::<&mut DialogReturnValue>(subject) {
                existing.0 = value.to_string();
                true
            } else {
                false
            }
        };
        if !updated {
            let _ = dom
                .world_mut()
                .insert_one(subject, DialogReturnValue(value.to_string()));
        }
    }
    // step 8: set is-modal false (no-op when not modal).
    let _ = dom.world_mut().remove_one::<IsModalDialog>(subject);
    // step 5: remove `open` via the canonical chokepoint (fires
    // MutationEvent::AttributeChange).
    dom.remove_attribute(subject, "open");
    true
}

#[cfg(test)]
#[path = "dialog_tests.rs"]
mod tests;
