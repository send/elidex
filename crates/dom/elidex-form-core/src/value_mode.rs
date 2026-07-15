//! HTML §4.10.5.4 `value` IDL attribute **value mode** — the per-kind
//! value-mode predicate + the pure IDL getter/setter dispatch:
//!
//! - [`ValueMode`] / [`ValueSetAction`] enums + the per-kind predicate
//!   [`FormControlKind::value_idl_mode`].
//! - The pure getter/setter dispatch helpers [`ValueMode::idl_get`] /
//!   [`ValueMode::idl_set_action`] (engine-independent spec-logic;
//!   VM-host marshals around them per the Layering mandate).
//!
//! The §4.10.5 type-change value migration
//! (`apply_type_change_value_migration`, a `&mut EcsDom` system) lives in
//! `elidex-form`, driven from the reconciler `type`-arm.

use crate::FormControlKind;

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
/// here in `elidex-form-core`; the engine host only executes the action
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
