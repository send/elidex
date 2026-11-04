//! Change-on-blur provenance snapshot (WHATWG HTML §6.6.4 focus update steps
//! step 2.1; §4.10.5.5 Common event behaviors).
//!
//! Step 2.1 fires `change` at a blurred text control only if "the user has
//! changed the element's value … while the control was focused without
//! committing that change (such that it is different to what it was when the
//! control was first focused)", and §4.10.5.5 says `input` / `change` "are not
//! fired in response to changes made to the values of form controls by
//! scripts". The snapshot therefore records two things, stored **on the
//! focused element** as an ECS component so it follows the canonical
//! `ElementState::FOCUS` bit whichever path established focus (the shell UA
//! transition or the VM's `HTMLElement.focus()`):
//!
//! - `initial` — the baseline the blur compares against;
//! - `user_changed` — the provenance mark: a user edit changed the value since
//!   focus. Only [`crate::apply_user_edit`] sets it; no non-user writer does.
//!
//! The blur's verdict is computed in one place, [`take_change_on_blur`]; every
//! focus sink calls it and only dispatches `change`.

use elidex_ecs::{EcsDom, Entity};

use crate::FormControlState;

/// The change-on-blur provenance of a focused text control. Only text controls
/// carry it; auto-cleaned on despawn. The fields are private: only this module
/// writes them (seed, mark, take).
pub struct FocusValueSnapshot {
    initial: String,
    user_changed: bool,
}

impl FocusValueSnapshot {
    /// The baseline `change` compares the value with at blur.
    #[must_use]
    pub fn initial(&self) -> &str {
        &self.initial
    }

    /// Whether a user edit changed the value since focus (the provenance mark).
    #[must_use]
    pub fn user_changed(&self) -> bool {
        self.user_changed
    }
}

/// Seed the change-on-blur snapshot on `entity` **if** it is a text control:
/// `initial` = the current value, unmarked. Call whenever a control gains focus
/// (the shell sink's §6.6.4 step 4.1 or VM `focus()`); a re-seed replaces any
/// earlier snapshot, mark included.
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
        Some(initial) => {
            let _ = dom.world_mut().insert_one(
                entity,
                FocusValueSnapshot {
                    initial,
                    user_changed: false,
                },
            );
        }
        None => {
            let _ = dom.world_mut().remove_one::<FocusValueSnapshot>(entity);
        }
    }
}

/// Mark `entity`'s snapshot: a user edit changed the value from `before`.
/// Called only by [`crate::apply_user_edit`]. Without a snapshot it does
/// nothing (never inserts one).
///
/// The **first** mark re-baselines: `initial = before`. While unmarked, the
/// value has changed only through non-user writes (a user change would have
/// marked), and each of those re-baselines (the Chromium-measured model, plan
/// §4.3), so the baseline at the first user change is the value just before
/// it. Taking it here means no non-user writer has to call anything. Later
/// changes, user or not, touch neither field.
pub(crate) fn note_user_value_change(dom: &mut EcsDom, entity: Entity, before: String) {
    if let Ok(mut snap) = dom.world_mut().get::<&mut FocusValueSnapshot>(entity) {
        if !snap.user_changed {
            snap.initial = before;
            snap.user_changed = true;
        }
    }
}

/// §6.6.4 focus update steps step 2.1 — consume (read + remove) `entity`'s
/// snapshot and return whether `change` fires: the user changed the value
/// since focus **and** it differs from the baseline (§4.10.5.5: script value
/// changes alone fire nothing). `false` without a snapshot (not a tracked text
/// control) or without a `FormControlState`. The only consumer of the
/// snapshot: every focus sink calls this and only dispatches `change`.
pub fn take_change_on_blur(dom: &mut EcsDom, entity: Entity) -> bool {
    let Ok(snap) = dom.world_mut().remove_one::<FocusValueSnapshot>(entity) else {
        return false;
    };
    snap.user_changed
        && dom
            .world()
            .get::<&FormControlState>(entity)
            .is_ok_and(|fcs| fcs.value() != snap.initial)
}

/// Drop any change-on-blur snapshot from `entity` without reading it.
///
/// `record_focus_snapshot` only re-evaluates "is this still a text control?" at
/// *focus* time, so a control whose `type` changes text → non-text *while
/// focused* keeps the stale text baseline; the eventual blur would then consume
/// it and fire a spurious `change`. The `FormControlReconciler` `type`-change arm
/// calls this at the `set_attribute` chokepoint to clear it mid-focus (the
/// counterpart to the non-text clear `record_focus_snapshot` does at focus time).
pub fn clear_focus_snapshot(dom: &mut EcsDom, entity: Entity) {
    let _ = dom.world_mut().remove_one::<FocusValueSnapshot>(entity);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{control, snap};
    use crate::{apply_user_edit, FormControlKind, FormControlState};

    fn set_value(dom: &mut EcsDom, e: Entity, value: &str) {
        dom.world_mut()
            .get::<&mut FormControlState>(e)
            .unwrap()
            .set_value(value.to_string());
    }

    /// A user edit that appends `text`.
    fn user_append(dom: &mut EcsDom, e: Entity, text: &str) {
        let edit = apply_user_edit(dom, e, |f| f.value.push_str(text)).unwrap();
        assert!(edit.changed);
    }

    /// C7 (a): a seed is unmarked with `initial` = the value.
    #[test]
    fn c7a_seed_is_unmarked_at_the_value() {
        let mut dom = EcsDom::new();
        let input = control(&mut dom, FormControlKind::TextInput, "initial");
        record_focus_snapshot(&mut dom, input);
        assert_eq!(snap(&dom, input), Some(("initial".to_string(), false)));
    }

    /// C7 (b): a user edit that changes the value marks the snapshot.
    #[test]
    fn c7b_user_value_change_marks() {
        let mut dom = EcsDom::new();
        let input = control(&mut dom, FormControlKind::TextInput, "");
        record_focus_snapshot(&mut dom, input);
        user_append(&mut dom, input, "a");
        assert_eq!(snap(&dom, input).map(|s| s.1), Some(true));
    }

    /// C7 (c): a user edit on a control without a snapshot inserts none.
    #[test]
    fn c7c_user_edit_without_snapshot_inserts_none() {
        let mut dom = EcsDom::new();
        let input = control(&mut dom, FormControlKind::TextInput, "");
        user_append(&mut dom, input, "a");
        assert_eq!(snap(&dom, input), None);
    }

    /// C7 (d): a re-seed after a mark is unmarked again.
    #[test]
    fn c7d_reseed_clears_the_mark() {
        let mut dom = EcsDom::new();
        let input = control(&mut dom, FormControlKind::TextInput, "");
        record_focus_snapshot(&mut dom, input);
        user_append(&mut dom, input, "a");
        record_focus_snapshot(&mut dom, input);
        assert_eq!(snap(&dom, input), Some(("a".to_string(), false)));
    }

    /// C7 (i): `take_change_on_blur` without a snapshot → `false`.
    #[test]
    fn c7i_take_without_snapshot_is_false() {
        let mut dom = EcsDom::new();
        let input = control(&mut dom, FormControlKind::TextInput, "x");
        assert!(!take_change_on_blur(&mut dom, input));
    }

    /// C7 (j): marked and value ≠ `initial` → `true`, consumed; a second take → `false`.
    #[test]
    fn c7j_take_marked_changed_is_true_once() {
        let mut dom = EcsDom::new();
        let input = control(&mut dom, FormControlKind::TextInput, "");
        record_focus_snapshot(&mut dom, input);
        user_append(&mut dom, input, "a");
        assert!(take_change_on_blur(&mut dom, input));
        assert_eq!(snap(&dom, input), None, "the take removes the snapshot");
        assert!(!take_change_on_blur(&mut dom, input));
    }

    /// C7 (k): unmarked with value ≠ `initial`, and marked with value =
    /// `initial` → `false`.
    #[test]
    fn c7k_take_needs_mark_and_difference() {
        let mut dom = EcsDom::new();
        let script_only = control(&mut dom, FormControlKind::TextInput, "");
        record_focus_snapshot(&mut dom, script_only);
        set_value(&mut dom, script_only, "x");
        assert!(!take_change_on_blur(&mut dom, script_only), "unmarked");

        let reverted = control(&mut dom, FormControlKind::TextInput, "");
        record_focus_snapshot(&mut dom, reverted);
        user_append(&mut dom, reverted, "a");
        set_value(&mut dom, reverted, "");
        assert!(!take_change_on_blur(&mut dom, reverted), "value = initial");
    }

    /// C7 (l): a non-user write before the first mark is absorbed into the
    /// baseline; later marks do not move it.
    #[test]
    fn c7l_first_mark_rebaselines_once() {
        let mut dom = EcsDom::new();
        let input = control(&mut dom, FormControlKind::TextInput, "");
        record_focus_snapshot(&mut dom, input);
        set_value(&mut dom, input, "x");
        user_append(&mut dom, input, "y");
        assert_eq!(snap(&dom, input), Some(("x".to_string(), true)));
        user_append(&mut dom, input, "z");
        assert_eq!(snap(&dom, input), Some(("x".to_string(), true)));
    }

    #[test]
    fn record_clears_stale_snapshot_when_no_longer_text_control() {
        // Codex R7 F4: a snapshot left from an earlier text focus must be removed
        // when the control is re-recorded as a non-text control (e.g. `type`
        // changed text → button), else a later blur consumes the stale text
        // baseline and fires a spurious `change`.
        let mut dom = EcsDom::new();
        let input = control(&mut dom, FormControlKind::TextInput, "typed");
        record_focus_snapshot(&mut dom, input);
        assert_eq!(snap(&dom, input), Some(("typed".to_string(), false)));

        // Flip to a non-text control before re-recording.
        if let Ok(mut fcs) = dom.world_mut().get::<&mut FormControlState>(input) {
            fcs.kind = FormControlKind::Button;
        }
        record_focus_snapshot(&mut dom, input);
        assert_eq!(
            snap(&dom, input),
            None,
            "re-recording a non-text control clears the stale text snapshot"
        );
    }

    #[test]
    fn no_snapshot_for_non_text_control() {
        let mut dom = EcsDom::new();
        let button = control(&mut dom, FormControlKind::Button, "");
        record_focus_snapshot(&mut dom, button);
        assert_eq!(
            snap(&dom, button),
            None,
            "non-text controls carry no change-on-blur snapshot"
        );
        set_value(&mut dom, button, "x");
        assert!(!take_change_on_blur(&mut dom, button));
    }
}
