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
/// so a later blur can fire `change` when the value differs. No-op otherwise.
/// Call whenever a control gains focus (shell `set_focus` or VM `focus()`).
pub fn record_focus_snapshot(dom: &mut EcsDom, entity: Entity) {
    let value = dom
        .world()
        .get::<&FormControlState>(entity)
        .ok()
        .filter(|fcs| fcs.kind.is_text_control())
        .map(|fcs| fcs.value().to_string());
    if let Some(value) = value {
        let _ = dom
            .world_mut()
            .insert_one(entity, FocusValueSnapshot(value));
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
