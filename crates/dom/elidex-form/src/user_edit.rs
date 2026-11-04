//! The user-edit funnel (WHATWG HTML §4.10.5.5 Common event behaviors,
//! §4.10.11 The textarea element): the one place that decides "the user caused
//! the control's value to change".
//!
//! §4.10.5.5: "any time the user causes the element's value to change … fire an
//! event named `input`", and §6.6.4 focus update steps step 2.1 fires `change`
//! on blur only for a value "the user has changed". Both consumers read the same
//! predicate, computed here: the value after the edit differs from the value
//! before it.

use elidex_ecs::{EcsDom, Entity};

use crate::focus_snapshot::note_user_value_change;
use crate::FormControlState;

/// The outcome of [`apply_user_edit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserEdit<R> {
    /// What the edit closure returned.
    pub result: R,
    /// Whether the edit changed the control's value. The caller fires `input`
    /// iff this is `true`.
    pub changed: bool,
}

/// Run a **user** edit `edit` on `entity`'s [`FormControlState`] and report
/// whether it changed the value. If it did, the change-on-blur snapshot (if
/// any) is marked as user-changed, before the caller builds its `input` event,
/// so a blur caused by an `input` listener sees the mark. Returns `None` when
/// `entity` has no `FormControlState`.
///
/// Every path by which the **user** writes a text control's value must call
/// this and fire `input` iff [`UserEdit::changed`] — today the shell's key
/// edits, cut, paste and IME commit. A future user edit path (drag-and-drop
/// insert, undo / redo, number spin buttons, IME commit on blur) must call it
/// too; a path that skips it fires neither `input` nor `change`, so the miss is
/// visible. Script and other non-user writers (the `value` / `defaultValue`
/// setters, attribute reconciliation, `setRangeText`, form reset) must **not**
/// call it: §4.10.5.5's events "are not fired in response to changes made to
/// the values of form controls by scripts". A user writing-direction change
/// fires `input` without a value change (§4.10.11); that `input` is not decided
/// here.
pub fn apply_user_edit<R>(
    dom: &mut EcsDom,
    entity: Entity,
    edit: impl FnOnce(&mut FormControlState) -> R,
) -> Option<UserEdit<R>> {
    let (result, before, changed) = {
        let mut fcs = dom.world_mut().get::<&mut FormControlState>(entity).ok()?;
        let before = fcs.value().to_owned();
        let result = edit(&mut fcs);
        let changed = fcs.value() != before;
        (result, before, changed)
    };
    if changed {
        note_user_value_change(dom, entity, before);
    }
    Some(UserEdit { result, changed })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{clipboard_cut, clipboard_paste, FocusValueSnapshot, FormControlKind};
    use elidex_ecs::Attributes;

    fn focused_control(dom: &mut EcsDom, value: &str) -> Entity {
        let e = dom.create_element("input", Attributes::default());
        let mut state = FormControlState::default();
        state.kind = FormControlKind::TextInput;
        state.set_value(value.to_string());
        let _ = dom.world_mut().insert_one(e, state);
        crate::record_focus_snapshot(dom, e);
        e
    }

    fn marked(dom: &EcsDom, e: Entity) -> bool {
        dom.world()
            .get::<&FocusValueSnapshot>(e)
            .is_ok_and(|s| s.user_changed())
    }

    /// C7 (e): an edit that leaves the value equal → unchanged, unmarked.
    #[test]
    fn c7e_value_preserving_edit_is_unchanged() {
        let mut dom = EcsDom::new();
        let e = focused_control(&mut dom, "ab");
        let edit = apply_user_edit(&mut dom, e, |f| {
            f.value.push('x');
            f.value.pop();
        })
        .unwrap();
        assert!(!edit.changed);
        assert!(!marked(&dom, e));
    }

    /// C7 (f): paste into a readonly control and into a full-`maxlength` control
    /// leaves the value → unchanged.
    #[test]
    fn c7f_refused_paste_is_unchanged() {
        let mut dom = EcsDom::new();
        let readonly = focused_control(&mut dom, "ab");
        dom.world_mut()
            .get::<&mut FormControlState>(readonly)
            .unwrap()
            .readonly = true;
        let edit = apply_user_edit(&mut dom, readonly, |f| clipboard_paste(f, "q")).unwrap();
        assert!(!edit.changed, "readonly paste");
        assert!(!marked(&dom, readonly));

        let full = focused_control(&mut dom, "z");
        dom.world_mut()
            .get::<&mut FormControlState>(full)
            .unwrap()
            .maxlength = Some(1);
        let edit = apply_user_edit(&mut dom, full, |f| clipboard_paste(f, "q")).unwrap();
        assert!(!edit.changed, "full-maxlength paste");
        assert!(!marked(&dom, full));
    }

    /// C7 (g): a cut with a selection → changed, `result` = the cut text.
    #[test]
    fn c7g_cut_with_selection_is_changed() {
        let mut dom = EcsDom::new();
        let e = focused_control(&mut dom, "ab");
        dom.world_mut()
            .get::<&mut FormControlState>(e)
            .unwrap()
            .set_selection(0, 1);
        let edit = apply_user_edit(&mut dom, e, clipboard_cut).unwrap();
        assert!(edit.changed);
        assert_eq!(edit.result, "a");
        assert!(marked(&dom, e));
    }

    /// C7 (h): no `FormControlState` → `None`.
    #[test]
    fn c7h_no_form_control_state_is_none() {
        let mut dom = EcsDom::new();
        let e = dom.create_element("div", Attributes::default());
        assert!(apply_user_edit(&mut dom, e, |_| ()).is_none());
    }
}
