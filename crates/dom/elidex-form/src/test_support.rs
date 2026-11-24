//! Shared test fixtures for the focus-snapshot / user-edit tests.

use crate::{FocusValueSnapshot, FormControlKind, FormControlState};
use elidex_ecs::{Attributes, EcsDom, Entity};

/// A control state of the given kind + live value, built through `set_value`
/// so `char_count` is in sync.  (`FormControlState`'s value-model fields are
/// `elidex-form-core`-private, so this builds from `default()` rather than a
/// `..Default::default()` functional update.)
pub(crate) fn fcs(kind: FormControlKind, value: &str) -> FormControlState {
    let mut state = FormControlState::default();
    state.kind = kind;
    state.set_value(value.to_string());
    state
}

/// An `<input>` entity carrying [`fcs`]`(kind, value)`.
pub(crate) fn control(dom: &mut EcsDom, kind: FormControlKind, value: &str) -> Entity {
    let e = dom.create_element_ownerless_fixture("input", Attributes::default());
    let _ = dom.world_mut().insert_one(e, fcs(kind, value));
    e
}

/// A `tag` element (`textarea`, `select`, `button`, `output`, `meter`,
/// `progress`, …) carrying the `FormControlState` the production single-entity
/// attach ([`crate::create_form_control_state`]) gives it, as parse, insert and
/// `createElement` do. An element from `create_element_ownerless_fixture` alone
/// carries none (it runs no attach).
pub(crate) fn attached_element(dom: &mut EcsDom, tag: &str, attrs: &[(&str, &str)]) -> Entity {
    let mut attr_map = Attributes::default();
    for (k, v) in attrs {
        attr_map.set((*k).to_string(), (*v).to_string());
    }
    let e = dom.create_element_ownerless_fixture(tag, attr_map);
    assert!(
        crate::create_form_control_state(dom, e),
        "create_form_control_state must attach to <{tag}>"
    );
    e
}

/// A user edit that appends `text` at the cursor; asserts it changed the value.
#[track_caller]
pub(crate) fn user_append(dom: &mut EcsDom, e: Entity, text: &str) {
    let edit = crate::apply_user_edit(dom, e, |f| f.insert_at_cursor(text))
        .expect("user_append: the entity has no FormControlState / no focus snapshot");
    assert!(edit.changed);
}

/// `e`'s change-on-blur component, as the three cases tests distinguish.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Snap {
    /// No component.
    Absent,
    /// Seeded at first focus; no user value change since (no mark).
    Unmarked { first_focus: String },
    /// Seeded, and marked by the first user value change.
    Marked {
        first_focus: String,
        user_change_from: String,
    },
}

pub(crate) fn snap(dom: &EcsDom, e: Entity) -> Snap {
    match dom.world().get::<&FocusValueSnapshot>(e) {
        Err(_) => Snap::Absent,
        Ok(s) => {
            let first_focus = s.first_focus().to_string();
            match s.user_change_from() {
                None => Snap::Unmarked { first_focus },
                Some(p) => Snap::Marked {
                    first_focus,
                    user_change_from: p.to_string(),
                },
            }
        }
    }
}
