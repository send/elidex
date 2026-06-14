//! Change-on-blur value snapshot (HTML §4.10.5.5 Common event behaviors).
//!
//! The text value a control had when it gained focus, stored **on the focused
//! element** as an ECS component so the snapshot follows the canonical
//! `ElementState::FOCUS` bit regardless of which path established focus — the
//! shell UA `set_focus` reconciler OR the JS VM's `HTMLElement.focus()`. A later
//! blur reads + removes it and fires `change` if the value differs. Living here
//! (not in the shell) lets both the shell and the engine `host/` seed it, so a
//! script `input.focus()` does not silently drop a user's later edit on blur.

use elidex_ecs::{EcsDom, Entity};

use crate::FormControlState;

/// The focus-time value of a text control, kept on the element for change-on-blur.
/// Only text controls carry it; auto-cleaned on despawn.
pub struct FocusValueSnapshot(pub String);

/// Record the focus-time value snapshot on `entity` **if** it is a text control,
/// so a later blur can fire `change` when the value differs. Call whenever a
/// control gains focus (shell `set_focus` or VM `focus()`).
///
/// When `entity` is **not** a text control, any pre-existing snapshot is
/// *removed* rather than left in place: the bit can otherwise be cleared without
/// the snapshot being taken (VM `blur()`, the silent §2.1.4 removal reset), so a
/// stale text baseline from before a `type` change (e.g. text → checkbox) would
/// survive and make a later blur fire a spurious `change`.
pub fn record_focus_snapshot(dom: &mut EcsDom, entity: Entity) {
    let value = dom
        .world()
        .get::<&FormControlState>(entity)
        .ok()
        .filter(|fcs| fcs.kind.is_text_control())
        .map(|fcs| fcs.value().to_string());
    match value {
        Some(value) => {
            let _ = dom
                .world_mut()
                .insert_one(entity, FocusValueSnapshot(value));
        }
        None => {
            let _ = dom.world_mut().remove_one::<FocusValueSnapshot>(entity);
        }
    }
}

/// Take (read + remove) the focus-time snapshot. `None` when absent (the element
/// was not a tracked text control) — the caller then fires no `change` event.
pub fn take_focus_snapshot(dom: &mut EcsDom, entity: Entity) -> Option<String> {
    dom.world_mut()
        .remove_one::<FocusValueSnapshot>(entity)
        .ok()
        .map(|snapshot| snapshot.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FormControlKind, FormControlState};
    use elidex_ecs::Attributes;

    #[test]
    fn record_and_take_round_trip_for_text_control() {
        let mut dom = EcsDom::new();
        let input = dom.create_element("input", Attributes::default());
        let fcs = FormControlState {
            kind: FormControlKind::TextInput,
            value: "initial".to_string(),
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(input, fcs);

        record_focus_snapshot(&mut dom, input);
        assert_eq!(
            take_focus_snapshot(&mut dom, input),
            Some("initial".to_string()),
            "a text control's focus-time value round-trips"
        );
        // Drained — a second take yields nothing.
        assert_eq!(take_focus_snapshot(&mut dom, input), None);
    }

    #[test]
    fn record_clears_stale_snapshot_when_no_longer_text_control() {
        // Codex R7 F4: a snapshot left from an earlier text focus must be removed
        // when the control is re-recorded as a non-text control (e.g. `type`
        // changed text → button), else a later blur consumes the stale text
        // baseline and fires a spurious `change`.
        let mut dom = EcsDom::new();
        let input = dom.create_element("input", Attributes::default());
        let _ = dom.world_mut().insert_one(
            input,
            FormControlState {
                kind: FormControlKind::TextInput,
                value: "typed".to_string(),
                ..Default::default()
            },
        );
        record_focus_snapshot(&mut dom, input);
        assert_eq!(
            take_focus_snapshot(&mut dom, input),
            Some("typed".to_string())
        );

        // Re-seed then flip to a non-text control before re-recording.
        record_focus_snapshot(&mut dom, input);
        if let Ok(mut fcs) = dom.world_mut().get::<&mut FormControlState>(input) {
            fcs.kind = FormControlKind::Button;
        }
        record_focus_snapshot(&mut dom, input);
        assert_eq!(
            take_focus_snapshot(&mut dom, input),
            None,
            "re-recording a non-text control clears the stale text snapshot"
        );
    }

    #[test]
    fn no_snapshot_for_non_text_control() {
        let mut dom = EcsDom::new();
        let button = dom.create_element("button", Attributes::default());
        let fcs = FormControlState {
            kind: FormControlKind::Button,
            ..Default::default()
        };
        let _ = dom.world_mut().insert_one(button, fcs);

        record_focus_snapshot(&mut dom, button);
        assert_eq!(
            take_focus_snapshot(&mut dom, button),
            None,
            "non-text controls carry no change-on-blur snapshot"
        );
    }
}
