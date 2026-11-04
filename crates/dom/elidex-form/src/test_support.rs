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
    let e = dom.create_element("input", Attributes::default());
    let _ = dom.world_mut().insert_one(e, fcs(kind, value));
    e
}

/// `(initial, user_changed)` of `e`'s snapshot, `None` when absent.
pub(crate) fn snap(dom: &EcsDom, e: Entity) -> Option<(String, bool)> {
    dom.world()
        .get::<&FocusValueSnapshot>(e)
        .ok()
        .map(|s| (s.initial().to_string(), s.user_changed()))
}
