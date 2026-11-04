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

/// `e`'s change-on-blur snapshot, as the three cases tests distinguish.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Snap {
    /// No snapshot.
    Absent,
    /// A snapshot with no baseline (no user value change since focus).
    Unmarked,
    /// A marked snapshot and its baseline.
    Baseline(String),
}

pub(crate) fn snap(dom: &EcsDom, e: Entity) -> Snap {
    match dom.world().get::<&FocusValueSnapshot>(e) {
        Err(_) => Snap::Absent,
        Ok(s) => s
            .baseline()
            .map_or(Snap::Unmarked, |b| Snap::Baseline(b.to_string())),
    }
}
