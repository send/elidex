//! HTML §4.10.5.4 `value` IDL attribute **value mode** — the single
//! cohesive home for all value-mode logic:
//!
//! - [`ValueMode`] / [`ValueSetAction`] enums + the per-kind predicate
//!   [`FormControlKind::value_idl_mode`].
//! - The pure getter/setter dispatch helpers [`ValueMode::idl_get`] /
//!   [`ValueMode::idl_set_action`] (engine-independent spec-logic;
//!   VM-host + boa marshal around them per the Layering mandate).
//! - The §4.10.5 type-change **steps 1–3** value migration
//!   [`apply_type_change_value_migration`], driven from the reconciler
//!   `type`-arm.
//!
//! One-issue-one-way: keeping the predicate, the dispatch helpers, and
//! the type-change migration in one module (rather than scattered across
//! `lib.rs` / `reconciler.rs`) means the value-mode rules have a single
//! reading.

use elidex_ecs::{EcsDom, Entity};

use crate::{FormControlKind, FormControlState};

/// The mode of an input element's `value` IDL attribute — HTML §4.10.5.4
/// "Common input element APIs" ("The `value` IDL attribute … is in one of
/// the following modes, which define its behavior").  Returned by
/// [`FormControlKind::value_idl_mode`]; drives the IDL `value`
/// getter/setter dispatch and the §4.10.5 type-change value migration.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ValueMode {
    /// Getter returns the current value; setter runs §4.10.5.4 value-mode
    /// steps 1–5 (set live value + dirty flag + sanitize + cursor).
    Value,
    /// Getter returns the `value` content attribute or `""`; setter sets
    /// the `value` content attribute.
    Default,
    /// Getter returns the `value` content attribute or `"on"`; setter sets
    /// the `value` content attribute.
    DefaultOn,
    /// Getter returns `"C:\fakepath\"` + first selected file name (or `""`
    /// if the list is empty); setter clears the selected files on `""`,
    /// else throws `InvalidStateError`.
    Filename,
}

/// The marshalling action a host must perform for an IDL `value` setter,
/// returned by [`ValueMode::idl_set_action`].  The spec-logic (which mode
/// → which action, incl. the filename empty-vs-non-empty branch) lives
/// here in `elidex-form`; the engine host only executes the action
/// (`set_value` / `set_attribute` / clear files / raise
/// `InvalidStateError`) per the Layering mandate.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ValueSetAction {
    /// Value mode — set the live value (§4.10.5.4 value-mode steps 1–5).
    SetLiveValue,
    /// Default / default-on mode — set the `value` content attribute to
    /// the new value (the host routes this through the `set_attribute`
    /// chokepoint so the reconciler maintains derived state).
    SetContentAttr,
    /// Filename mode, empty new value — empty the list of selected files.
    ClearFiles,
    /// Filename mode, non-empty new value — throw `InvalidStateError`.
    ThrowInvalidState,
}

impl ValueMode {
    /// IDL `value` getter dispatch (HTML §4.10.5.4) — pure spec-logic.
    /// The host marshals the inputs: `live` = `FormControlState.value`,
    /// `content_attr` = the `value` content attribute (via
    /// `with_attribute`), `first_filename` = the name of the first
    /// selected file (currently always `None` — the selected-files list
    /// is not yet modeled, deferred to `#11-input-file-shell-staging`).
    #[must_use]
    pub fn idl_get(
        self,
        live: &str,
        content_attr: Option<&str>,
        first_filename: Option<&str>,
    ) -> String {
        match self {
            Self::Value => live.to_owned(),
            Self::Default => content_attr.unwrap_or("").to_owned(),
            Self::DefaultOn => content_attr.unwrap_or("on").to_owned(),
            // "C:\fakepath\" + first file name, or "" if the list is empty.
            Self::Filename => {
                first_filename.map_or_else(String::new, |name| format!("C:\\fakepath\\{name}"))
            }
        }
    }

    /// IDL `value` setter dispatch (HTML §4.10.5.4) — returns the
    /// marshalling action the host must perform.  Pure spec-logic.
    #[must_use]
    pub fn idl_set_action(self, new_value: &str) -> ValueSetAction {
        match self {
            Self::Value => ValueSetAction::SetLiveValue,
            Self::Default | Self::DefaultOn => ValueSetAction::SetContentAttr,
            Self::Filename => {
                if new_value.is_empty() {
                    ValueSetAction::ClearFiles
                } else {
                    ValueSetAction::ThrowInvalidState
                }
            }
        }
    }
}

impl FormControlKind {
    /// The mode of the `value` IDL attribute for this kind — HTML
    /// §4.10.5.4 "Common input element APIs" (the value IDL attribute is
    /// "in one of the following modes, which define its behavior").  The
    /// single canonical per-kind → mode predicate consumed by the IDL
    /// `value` getter/setter dispatch (host + boa) and the §4.10.5
    /// type-change steps 1–3 value migration.
    ///
    /// Per the per-state bookkeeping ("The `value` IDL attribute is in
    /// the X mode"): **value** for the text-like, numeric, and date/time
    /// states (text/search/tel/url/email/password/number/range/color/
    /// date/month/week/time/datetime-local); **default** for hidden/
    /// submit/reset/button/image; **default/on** for checkbox/radio;
    /// **filename** for file.
    ///
    /// ⚠ **`type=image`**: the spec puts image in **default** mode, but
    /// elidex does not model a distinct image-input kind — `from_type_str`
    /// falls `image` through to [`TextInput`](Self::TextInput) (value
    /// mode).  So `<input type=image>` currently takes the value-mode
    /// path here.  Proper image-state modeling (default value mode +
    /// coordinate submission + image rendering) is the
    /// `#11-input-image-state` defer slot.
    ///
    /// Non-`<input>` kinds (`<textarea>`/`<select>`/`<output>`/`<meter>`/
    /// `<progress>`) take the **value** mode — their live value is the
    /// authoritative value (textarea/output expose a `value` IDL whose
    /// getter returns the current value; meter/progress/select have no
    /// content-attribute value-mode bookkeeping), so reading/writing the
    /// live value is correct and avoids a spurious content-attribute
    /// round-trip.
    #[must_use]
    pub fn value_idl_mode(self) -> ValueMode {
        match self {
            Self::Hidden | Self::SubmitButton | Self::ResetButton | Self::Button => {
                ValueMode::Default
            }
            Self::Checkbox | Self::Radio => ValueMode::DefaultOn,
            Self::File => ValueMode::Filename,
            // value mode: text/search/tel/url/email/password/number/
            // range/color/date/month/week/time/datetime-local, plus the
            // non-input value-bearing kinds (textarea/select/output/
            // meter/progress).
            Self::TextInput
            | Self::Password
            | Self::Email
            | Self::Url
            | Self::Tel
            | Self::Search
            | Self::Number
            | Self::Range
            | Self::Color
            | Self::Date
            | Self::DatetimeLocal
            | Self::Time
            | Self::Week
            | Self::Month
            | Self::TextArea
            | Self::Select
            | Self::Output
            | Self::Meter
            | Self::Progress => ValueMode::Value,
        }
    }
}

/// HTML §4.10.5 "When an input element's type attribute changes state"
/// **steps 1–3** — the value-mode value migration that precedes value
/// sanitization (steps 6–9, [`sanitize_for_type_change`](crate::sanitize_for_type_change)).
/// A mutually-exclusive if / else-if / else-if chain on the previous vs
/// new [`ValueMode`] (the spec's "Otherwise, if …" structure).
///
/// Runs at the `set_attribute("type")` chokepoint (the canonical
/// type-change site) while `FormControlState.kind` is still `old_kind`;
/// the caller updates the kind and runs `sanitize_for_type_change`
/// (which settles the value under `new_kind`) immediately after, so this
/// sets **raw fields only** — no value sanitization here.
///
/// - **Step 1** (prev value mode, value ≠ "", new default | default/on):
///   set the `value` content attribute to the live value, via
///   [`EcsDom::set_attribute_without_dispatch`] — the reconciler runs
///   inside `MutationDispatcher::dispatch` and must not re-enter
///   `set_attribute` (re-entry contract).  That non-dispatching write
///   suppresses the entire `AttributeChange` consumer fan-out, so the
///   `value`-arm's `default_value` mirror (the sole effect step 1 needs)
///   is reproduced inline.
/// - **Step 2** (prev mode ≠ value, new value mode): set the live value
///   from the `value` content attribute (mirrored by `default_value`) and
///   clear the dirty value flag.
/// - **Step 3** (prev mode ≠ filename, new filename mode): empty the
///   live value.
pub(crate) fn apply_type_change_value_migration(
    old_kind: FormControlKind,
    new_kind: FormControlKind,
    dom: &mut EcsDom,
    node: Entity,
) {
    let old_mode = old_kind.value_idl_mode();
    let new_mode = new_kind.value_idl_mode();
    if old_mode == new_mode {
        return; // no value-mode transition — steps 1–3 are all no-ops
    }

    // Step 1: previous value mode → default | default/on (value ≠ "").
    if old_mode == ValueMode::Value && matches!(new_mode, ValueMode::Default | ValueMode::DefaultOn)
    {
        let Ok(live) = dom
            .world()
            .get::<&FormControlState>(node)
            .map(|s| s.value().to_owned())
        else {
            return;
        };
        if live.is_empty() {
            return; // step 1 requires a non-empty value
        }
        dom.set_attribute_without_dispatch(node, "value", &live);
        // Reproduce the suppressed value-arm `default_value` mirror.
        if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(node) {
            state.default_value = live;
        }
        return;
    }

    // Step 2: previous mode ≠ value → value mode.  (`old_mode != new_mode`
    // + the step-1 branch above exclude `old_mode == Value` here, so the
    // spec's "previous … in any mode other than the value mode" holds.)
    if new_mode == ValueMode::Value {
        if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(node) {
            state.migrate_value_from_content_attr();
        }
        return;
    }

    // Step 3: previous mode ≠ filename → filename mode.  (`old_mode !=
    // new_mode` + `new_mode == Filename` imply `old_mode != Filename`.)
    if new_mode == ValueMode::Filename {
        if let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(node) {
            state.clear_value_for_type_change();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_form_control_state, FormControlReconciler};
    use elidex_ecs::{Attributes, EcsDom, Entity, MutationDispatcher, MutationEvent};

    // -- pure predicate / helper unit tests --

    #[test]
    fn value_idl_mode_per_kind_matches_spec_bookkeeping() {
        use FormControlKind as K;
        // value mode: text-like + numeric + date/time states.
        for k in [
            K::TextInput,
            K::Password,
            K::Email,
            K::Url,
            K::Tel,
            K::Search,
            K::Number,
            K::Range,
            K::Color,
            K::Date,
            K::DatetimeLocal,
            K::Time,
            K::Week,
            K::Month,
        ] {
            assert_eq!(
                k.value_idl_mode(),
                ValueMode::Value,
                "{k:?} should be value mode"
            );
        }
        // default mode: hidden / submit / reset / button.
        for k in [K::Hidden, K::SubmitButton, K::ResetButton, K::Button] {
            assert_eq!(
                k.value_idl_mode(),
                ValueMode::Default,
                "{k:?} should be default mode"
            );
        }
        // default/on mode: checkbox / radio.
        for k in [K::Checkbox, K::Radio] {
            assert_eq!(
                k.value_idl_mode(),
                ValueMode::DefaultOn,
                "{k:?} should be default/on mode"
            );
        }
        // filename mode: file.
        assert_eq!(K::File.value_idl_mode(), ValueMode::Filename);
        // Non-input value-bearing kinds take the value mode (live value is
        // authoritative — avoids a spurious content-attribute round-trip).
        for k in [K::TextArea, K::Select, K::Output, K::Meter, K::Progress] {
            assert_eq!(
                k.value_idl_mode(),
                ValueMode::Value,
                "{k:?} should be value mode"
            );
        }
    }

    #[test]
    fn value_idl_get_dispatch() {
        // value mode → live value (ignores content attr / filename).
        assert_eq!(
            ValueMode::Value.idl_get("live", Some("attr"), Some("f.txt")),
            "live"
        );
        // default mode → content attr, fallback "".
        assert_eq!(
            ValueMode::Default.idl_get("live", Some("attr"), None),
            "attr"
        );
        assert_eq!(ValueMode::Default.idl_get("live", None, None), "");
        // default/on mode → content attr, fallback "on".
        assert_eq!(
            ValueMode::DefaultOn.idl_get("live", Some("yes"), None),
            "yes"
        );
        assert_eq!(ValueMode::DefaultOn.idl_get("live", None, None), "on");
        // filename mode → "C:\fakepath\" + first file name, or "".
        assert_eq!(
            ValueMode::Filename.idl_get("live", Some("attr"), Some("photo.png")),
            "C:\\fakepath\\photo.png"
        );
        assert_eq!(ValueMode::Filename.idl_get("live", Some("attr"), None), "");
    }

    #[test]
    fn value_idl_set_action_dispatch() {
        assert_eq!(
            ValueMode::Value.idl_set_action("x"),
            ValueSetAction::SetLiveValue
        );
        assert_eq!(
            ValueMode::Default.idl_set_action("x"),
            ValueSetAction::SetContentAttr
        );
        assert_eq!(
            ValueMode::DefaultOn.idl_set_action("x"),
            ValueSetAction::SetContentAttr
        );
        // filename: empty → clear files; non-empty → throw.
        assert_eq!(
            ValueMode::Filename.idl_set_action(""),
            ValueSetAction::ClearFiles
        );
        assert_eq!(
            ValueMode::Filename.idl_set_action("x"),
            ValueSetAction::ThrowInvalidState
        );
    }

    // -- type-change migration tests (steps 1–3), exercised end-to-end
    //    through the `set_attribute("type")` reconciler chokepoint --

    /// Minimal dispatcher wiring ONLY [`FormControlReconciler`] (the
    /// production composer lives in `elidex-js`; referencing it here would
    /// create a circular cargo dep).
    struct FormControlOnlyTestDispatcher(FormControlReconciler);
    impl MutationDispatcher for FormControlOnlyTestDispatcher {
        fn dispatch(&mut self, event: &MutationEvent<'_>, dom: &mut EcsDom) {
            self.0.handle(event, dom);
        }
    }

    fn setup(attrs: &[(&str, &str)]) -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let mut attr_map = Attributes::default();
        for (k, v) in attrs {
            attr_map.set(k.to_string(), v.to_string());
        }
        let entity = dom.create_element("input", attr_map);
        assert!(create_form_control_state(&mut dom, entity));
        let displaced = dom.set_mutation_dispatcher(Box::new(FormControlOnlyTestDispatcher(
            FormControlReconciler,
        )));
        assert!(displaced.is_none());
        (dom, entity)
    }

    fn with_fcs<R>(dom: &EcsDom, entity: Entity, f: impl FnOnce(&FormControlState) -> R) -> R {
        let state = dom.world().get::<&FormControlState>(entity).unwrap();
        f(&state)
    }

    /// Step 1: a value-mode control with a non-empty (dirty) live value
    /// becoming default/default-on writes the live value into the `value`
    /// content attribute (+ mirrors `default_value`); the live value is
    /// preserved (step 1 sets only the content attribute).
    #[test]
    fn tc1_value_to_default_writes_live_value_into_content_attr() {
        let (mut dom, e) = setup(&[]); // text (value mode)
        {
            let mut state = dom.world_mut().get::<&mut FormControlState>(e).unwrap();
            state.set_value("abc".to_string()); // dirty live value, no `value` attr
        }
        assert!(dom.set_attribute(e, "type", "hidden")); // → default mode
        assert_eq!(
            dom.with_attribute(e, "value", |v| v.map(str::to_owned)),
            Some("abc".to_string())
        );
        with_fcs(&dom, e, |s| {
            assert_eq!(s.kind, FormControlKind::Hidden);
            assert_eq!(s.default_value, "abc");
            assert_eq!(s.value, "abc");
        });
    }

    /// Step 1 no-op when the live value is empty (spec gates step 1 on a
    /// non-empty value): no `value` content attribute is created.
    #[test]
    fn tc1_value_to_default_empty_value_is_noop() {
        let (mut dom, e) = setup(&[]); // text, empty value
        assert!(dom.set_attribute(e, "type", "hidden"));
        assert_eq!(
            dom.with_attribute(e, "value", |v| v.map(str::to_owned)),
            None
        );
    }

    /// Step 2 (default → value): adopt the `value` content attribute as the
    /// live value and clear the dirty value flag.
    #[test]
    fn tc2_default_to_value_adopts_content_attr_and_clears_dirty() {
        let (mut dom, e) = setup(&[("type", "hidden"), ("value", "x")]);
        with_fcs(&dom, e, |s| {
            assert_eq!(s.kind, FormControlKind::Hidden);
            assert_eq!(s.default_value, "x");
        });
        assert!(dom.set_attribute(e, "type", "text")); // → value mode
        with_fcs(&dom, e, |s| {
            assert_eq!(s.kind, FormControlKind::TextInput);
            assert_eq!(s.value, "x");
            assert!(!s.dirty_value, "step 2 clears the dirty value flag");
        });
    }

    /// Step 2 via the filename → value entry path (distinct from default →
    /// value): `file` → `text` adopts the `value` content attribute.
    #[test]
    fn tc2_filename_to_value_adopts_content_attr() {
        let (mut dom, e) = setup(&[("type", "file"), ("value", "x")]);
        with_fcs(&dom, e, |s| {
            assert_eq!(s.kind, FormControlKind::File);
            assert_eq!(s.default_value, "x");
        });
        assert!(dom.set_attribute(e, "type", "text")); // filename → value mode
        with_fcs(&dom, e, |s| {
            assert_eq!(s.kind, FormControlKind::TextInput);
            assert_eq!(s.value, "x");
            assert!(!s.dirty_value, "step 2 clears the dirty value flag");
        });
    }

    /// Step 3: any non-filename control becoming filename mode empties the
    /// live value.
    #[test]
    fn tc3_to_filename_empties_value() {
        let (mut dom, e) = setup(&[]); // text
        {
            let mut state = dom.world_mut().get::<&mut FormControlState>(e).unwrap();
            state.set_value("abc".to_string());
        }
        assert!(dom.set_attribute(e, "type", "file")); // → filename mode
        with_fcs(&dom, e, |s| {
            assert_eq!(s.kind, FormControlKind::File);
            assert_eq!(
                s.value, "",
                "step 3 empties the value on entry to filename mode"
            );
        });
    }

    /// A value-mode → value-mode type change runs no migration (the live
    /// value is preserved, modulo the new kind's sanitization).
    #[test]
    fn tc_value_to_value_preserves_live_value() {
        let (mut dom, e) = setup(&[]); // text
        {
            let mut state = dom.world_mut().get::<&mut FormControlState>(e).unwrap();
            state.set_value("hello".to_string());
        }
        assert!(dom.set_attribute(e, "type", "search")); // value → value
        with_fcs(&dom, e, |s| {
            assert_eq!(s.kind, FormControlKind::Search);
            assert_eq!(s.value, "hello");
            assert!(
                s.dirty_value,
                "value→value migration must not touch dirty flag"
            );
        });
    }

    /// A same-value-mode type change (default → default, e.g. hidden →
    /// submit) hits the `old_mode == new_mode` early return: no migration.
    #[test]
    fn tc_same_mode_default_to_default_is_noop() {
        let (mut dom, e) = setup(&[("type", "hidden")]);
        {
            let mut state = dom.world_mut().get::<&mut FormControlState>(e).unwrap();
            state.set_value("keep".to_string()); // dirty live value
        }
        assert!(dom.set_attribute(e, "type", "submit")); // default → default
        with_fcs(&dom, e, |s| {
            assert_eq!(s.kind, FormControlKind::SubmitButton);
            assert_eq!(s.value, "keep", "same-mode transition runs no migration");
            assert!(
                s.dirty_value,
                "same-mode transition must not touch dirty flag"
            );
        });
    }
}
