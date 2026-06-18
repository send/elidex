//! Form control state and input handling for elidex.
//!
//! Provides `FormControlState` ECS component for tracking the runtime state
//! of HTML form controls (`<input>`, `<button>`, `<textarea>`, `<label>`).
//! The `.value` property is distinct from the `value` HTML attribute per spec.

pub mod ancestor_cache;
mod clipboard;
mod datetime;
mod fieldset;
pub mod focus_snapshot;
mod init;
mod input;
mod label;
pub mod radio;
mod reconciler;
mod sanitize;
mod select;
mod selection;
mod sizing;
mod submit;
pub mod util;
mod validation;
mod value_mode;

pub use ancestor_cache::AncestorCache;
pub use clipboard::{clipboard_copy, clipboard_cut, clipboard_paste};
pub use fieldset::{
    first_legend_child, is_fieldset_disabled, is_in_first_legend, propagate_fieldset_disabled,
};
pub use focus_snapshot::{
    clear_focus_snapshot, record_focus_snapshot, take_focus_snapshot, FocusValueSnapshot,
};
pub use init::{create_form_control_state, find_autofocus_target, init_form_controls};
pub use input::{
    apply_step, form_control_key_input, form_control_key_input_action, resolve_input_list,
    sanitize_for_type_change, KeyAction, StepError,
};
pub use label::{find_label_target, is_label, is_labelable_element, resolve_label_for};
pub use radio::{
    find_radio_group, find_radio_group_scoped, is_radio_group_satisfied, radio_arrow_navigate,
    toggle_radio,
};
pub use reconciler::FormControlReconciler;
pub use select::{
    find_option_index_in_tree, find_option_select, init_select_options, is_option_disabled,
    navigate_select, option_value_string, select_get_value, select_option, select_selected_index,
    select_set_selected_index, select_set_value, select_uses_implicit_default,
};
pub use selection::{collapse_selection, delete_selection, extend_selection, select_all};
pub use sizing::form_intrinsic_size;
pub use submit::{
    build_form_submission, collect_form_data, encode_form_urlencoded, find_form_ancestor,
    is_form_owner, is_submit_button, read_form_attrs, reset_form, FormAttrs, FormDataEntry,
    FormSubmission,
};
pub use validation::{is_constraint_validation_candidate, validate_control, ValidityState};
pub use value_mode::{ValueMode, ValueSetAction};

use elidex_ecs::Attributes;
use std::sync::Arc;

/// Maximum pattern length to prevent `ReDoS` via excessively long regex patterns.
pub const MAX_PATTERN_LENGTH: usize = 1024;

/// Compile a `pattern` attribute value into an anchored regex.
///
/// Returns `None` if the pattern exceeds [`MAX_PATTERN_LENGTH`] or is not valid regex.
/// Per HTML spec §4.10.5.3.8, invalid patterns are silently ignored.
pub(crate) fn compile_pattern_regex(p: &str) -> Option<Arc<regex::Regex>> {
    if p.len() > MAX_PATTERN_LENGTH {
        return None;
    }
    let anchored = format!("^(?:{p})$");
    regex::RegexBuilder::new(&anchored)
        .size_limit(1 << 20)
        // JS pattern attribute uses the `u` flag (WHATWG HTML §4.10.5.3.8).
        // Rust regex defaults match this: \d/\w are ASCII-only, `.` matches Unicode scalars.
        .build()
        .ok()
        .map(Arc::new)
}

/// The kind of form control represented by an element.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FormControlKind {
    TextInput,
    Password,
    Checkbox,
    Radio,
    SubmitButton,
    ResetButton,
    Button,
    TextArea,
    Select,
    /// `<input type="email">`
    Email,
    /// `<input type="url">`
    Url,
    /// `<input type="tel">`
    Tel,
    /// `<input type="search">`
    Search,
    /// `<input type="number">`
    Number,
    /// `<input type="range">`
    Range,
    /// `<input type="color">`
    Color,
    /// `<input type="date">`
    Date,
    /// `<input type="datetime-local">`
    DatetimeLocal,
    /// `<input type="time">`
    Time,
    /// `<input type="week">`
    Week,
    /// `<input type="month">`
    Month,
    /// `<input type="file">`
    File,
    /// `<input type="hidden">` — not rendered, but participates in form data.
    Hidden,
    /// `<output>` element — displays calculation results.
    Output,
    /// `<meter>` element — scalar measurement within a known range.
    Meter,
    /// `<progress>` element — completion progress indicator.
    Progress,
}

impl FormControlKind {
    /// Returns `true` for text-editable control kinds (`TextInput`, `Password`, `TextArea`, and
    /// text-like input types: `Email`, `Url`, `Tel`, `Search`).
    #[must_use]
    pub fn is_text_control(self) -> bool {
        matches!(
            self,
            Self::TextInput
                | Self::Password
                | Self::TextArea
                | Self::Email
                | Self::Url
                | Self::Tel
                | Self::Search
        )
    }

    /// Returns `true` for single-line text input kinds (all text controls except `TextArea`).
    #[must_use]
    pub fn is_single_line_text(self) -> bool {
        self.is_text_control() && self != Self::TextArea
    }

    /// Returns `true` if the control supports the text-selection APIs
    /// (`selectionStart`/`selectionEnd`/`selectionDirection`,
    /// `setRangeText()`, `setSelectionRange()`).  Per the input-state
    /// apply-lists these apply only to Text (§4.10.5.1.2), Search,
    /// Telephone (§4.10.5.1.3), URL (§4.10.5.1.4), and Password
    /// (§4.10.5.1.6), plus `<textarea>` — they do **not** apply to the
    /// **Email** state (§4.10.5.1.5 lists `selectionStart`/`End`/
    /// `Direction`/`setRangeText`/`setSelectionRange` under "do not
    /// apply"; only `select()` applies), nor to number or the date/time
    /// states.  This is the canonical "setRangeText() applies" predicate —
    /// it also gates the §4.10.5.4 step-5 "has a text entry cursor
    /// position" move and the §4.10.5 type-change step-9 selectability
    /// transition.  (`Email` is therefore EXCLUDED here even though
    /// [`Self::is_text_control`] includes it for editing purposes.)
    #[must_use]
    pub fn supports_selection(self) -> bool {
        matches!(
            self,
            Self::TextInput
                | Self::Search
                | Self::Tel
                | Self::Url
                | Self::Password
                | Self::TextArea
        )
    }

    /// Returns `true` if the control has **selectable text** for the
    /// `select()` method to act on — the text-like states plus the number
    /// state, which elidex renders as editable text fields.
    ///
    /// `select()` applies to a broader set per the input-state apply-lists
    /// (it also lists the date/time states), but HTML "select() method"
    /// step 1 makes it a **no-op** for a control "that has no selectable
    /// text".  elidex renders the date/time states as pickers
    /// (`emit_button`), so they have no selectable text and `select()`
    /// must not record a selection for them — otherwise a stale range is
    /// observable via `selectionEnd` after a type change.  `select()`
    /// itself never throws (the no-op branch covers every other kind).
    #[must_use]
    pub fn has_selectable_text(self) -> bool {
        self.is_text_control() || matches!(self, Self::Number)
    }

    /// Returns `true` if the `readonly` attribute applies to this kind
    /// (HTML §4.10.5.3.3 "The readonly attribute").  `readonly` is
    /// meaningful for text-editable controls (`text`, `password`,
    /// `textarea`, `email`, `url`, `tel`, `search`) and the
    /// type-specific input subtypes that accept user editing (`number`,
    /// `date`, `datetime-local`, `time`, `week`, `month`).  For
    /// non-applicable kinds (`checkbox`, `radio`, `range`, `color`,
    /// `file`, `hidden`, button-typed) the attribute exists but has
    /// no effect — including for the constraint-validation barring
    /// rule of §4.10.20.3.
    #[must_use]
    pub fn readonly_applies(self) -> bool {
        matches!(
            self,
            Self::TextInput
                | Self::Password
                | Self::TextArea
                | Self::Email
                | Self::Url
                | Self::Tel
                | Self::Search
                | Self::Number
                | Self::Date
                | Self::DatetimeLocal
                | Self::Time
                | Self::Week
                | Self::Month
        )
    }

    /// Returns `true` if this kind participates in form submission
    /// (submittable element — HTML §4.10.2 Categories).
    #[must_use]
    pub fn is_submittable(self) -> bool {
        matches!(
            self,
            Self::TextInput
                | Self::Password
                | Self::Checkbox
                | Self::Radio
                | Self::TextArea
                | Self::Select
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
                | Self::File
                | Self::Hidden
        )
    }

    /// Map a `type` content-attribute value to a [`FormControlKind`]
    /// for the given tag.  Returns `None` for tags whose `type`
    /// attribute does not select a kind (e.g. `textarea` / `select`).
    /// Spec defaults: HTML §4.10.5.1 (input → "text"), §4.10.6 (button
    /// → "submit").
    #[must_use]
    pub fn from_tag_and_type_attr(tag: &str, raw_value: Option<&str>) -> Option<Self> {
        match tag {
            "input" => {
                let v = raw_value.unwrap_or("text").to_ascii_lowercase();
                Some(Self::from_type_str(&v))
            }
            "button" => Some(
                match raw_value.unwrap_or("submit").to_ascii_lowercase().as_str() {
                    "reset" => Self::ResetButton,
                    "button" => Self::Button,
                    _ => Self::SubmitButton,
                },
            ),
            _ => None,
        }
    }

    /// Parse an input type string to a `FormControlKind`.
    ///
    /// Unrecognized types fall back to `TextInput` per HTML spec §4.10.5.1.
    #[must_use]
    pub fn from_type_str(s: &str) -> Self {
        match s {
            "checkbox" => Self::Checkbox,
            "radio" => Self::Radio,
            "password" => Self::Password,
            "submit" => Self::SubmitButton,
            "reset" => Self::ResetButton,
            "button" => Self::Button,
            "email" => Self::Email,
            "url" => Self::Url,
            "tel" => Self::Tel,
            "search" => Self::Search,
            "number" => Self::Number,
            "range" => Self::Range,
            "color" => Self::Color,
            "date" => Self::Date,
            "datetime-local" => Self::DatetimeLocal,
            "time" => Self::Time,
            "week" => Self::Week,
            "month" => Self::Month,
            "file" => Self::File,
            "hidden" => Self::Hidden,
            "select-one" => Self::Select,
            "textarea" => Self::TextArea,
            _ => Self::TextInput,
        }
    }

    /// Returns the HTML type attribute string for this form control kind.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TextInput => "text",
            Self::Password => "password",
            Self::Checkbox => "checkbox",
            Self::Radio => "radio",
            Self::SubmitButton => "submit",
            Self::ResetButton => "reset",
            Self::Button => "button",
            Self::TextArea => "textarea",
            Self::Select => "select-one",
            Self::Email => "email",
            Self::Url => "url",
            Self::Tel => "tel",
            Self::Search => "search",
            Self::Number => "number",
            Self::Range => "range",
            Self::Color => "color",
            Self::Date => "date",
            Self::DatetimeLocal => "datetime-local",
            Self::Time => "time",
            Self::Week => "week",
            Self::Month => "month",
            Self::File => "file",
            Self::Hidden => "hidden",
            Self::Output => "output",
            Self::Meter => "meter",
            Self::Progress => "progress",
        }
    }
}

/// An option within a `<select>` element.
#[derive(Clone, Debug)]
pub struct SelectOption {
    /// The display text for this option.
    pub text: String,
    /// The value attribute (defaults to text if not set).
    pub value: String,
    /// Whether the option is disabled.
    pub disabled: bool,
    /// The optgroup label, if this option belongs to an optgroup.
    pub group: Option<String>,
    /// Whether this option is currently selected (for `<select multiple>`).
    pub selected: bool,
}

/// Selection direction for text selection (HTML spec §4.10.5.2.10).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SelectionDirection {
    /// Selection direction is not specified.
    #[default]
    None,
    /// Selection extends forward (left to right).
    Forward,
    /// Selection extends backward (right to left).
    Backward,
}

/// Runtime state for a form control element.
///
/// Separate from `Attributes` — the `.value` IDL property and the `value`
/// content attribute are distinct per HTML spec (§4.10.5.4).
#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)] // HTML form controls inherently have many boolean states.
pub struct FormControlState {
    /// The kind of form control.
    pub kind: FormControlKind,
    /// Current value (text content for inputs/textareas, label for buttons).
    pub(crate) value: String,
    /// Whether the control is checked (only meaningful for checkboxes/radios).
    pub checked: bool,
    /// Whether the control is disabled.
    pub disabled: bool,
    /// Placeholder text (displayed when value is empty).
    pub placeholder: String,
    /// Cursor position within the value string (byte offset).
    pub(crate) cursor_pos: usize,
    /// Whether the control is read-only (text controls only).
    pub readonly: bool,
    /// Number of visible rows (textarea only, default 2 per HTML spec §4.10.7).
    pub rows: u32,
    /// Number of visible columns (textarea only, default 20 per HTML spec §4.10.7).
    pub cols: u32,
    /// The `name` attribute value (used for radio group association and form data).
    pub name: String,
    /// Default value (for form reset).
    pub default_value: String,
    /// Whether the user has modified the value (dirty flag).
    pub(crate) dirty_value: bool,
    /// Default checked state (for form reset).
    pub default_checked: bool,
    /// Whether the control is required.
    pub required: bool,
    /// Minimum length constraint (`minlength` attribute).
    pub minlength: Option<usize>,
    /// Maximum length constraint (`maxlength` attribute).
    pub maxlength: Option<usize>,
    /// Pattern constraint (`pattern` attribute, regex string).
    pub pattern: Option<String>,
    /// The `form` attribute value (associates control with a form by ID).
    pub form_owner: Option<String>,
    /// Whether the control has the `autofocus` attribute.
    pub autofocus: bool,
    /// Horizontal scroll offset for text controls (pixels).
    pub scroll_offset_x: f32,
    /// Whether multiple selection is allowed (`<select multiple>`).
    pub multiple: bool,
    /// Visible size (`<select size>` or `<input size>`).
    pub size: u32,
    /// Autocomplete hint (`autocomplete` attribute).
    pub autocomplete: String,
    /// Selection start (byte offset, for text controls).
    pub(crate) selection_start: usize,
    /// Selection end (byte offset, for text controls).
    pub(crate) selection_end: usize,
    /// Selection direction.
    pub selection_direction: SelectionDirection,
    /// Composition text from IME (if active).
    pub composition_text: Option<String>,
    /// Selected index for `<select>` controls (-1 = no selection).
    pub selected_index: i32,
    /// Options list for `<select>` controls.
    pub options: Vec<SelectOption>,
    /// Whether the dropdown is open (for `<select>` controls).
    pub dropdown_open: bool,
    /// Cached character count (O(1)).  Kept in sync with `value` by editing methods.
    pub(crate) char_count: usize,
    /// Minimum value constraint (`min` attribute, for number/range/date types).
    pub min: Option<String>,
    /// Maximum value constraint (`max` attribute, for number/range/date types).
    pub max: Option<String>,
    /// Step constraint (`step` attribute, for number/range/date types).
    pub step: Option<String>,
    /// Cached compiled regex for the `pattern` attribute (avoids recompilation on each validation).
    ///
    /// - `None` — no `pattern` attribute set.
    /// - `Some(None)` — pattern set but regex compilation failed (invalid or too long).
    /// - `Some(Some(re))` — valid compiled regex.
    pub cached_pattern_regex: Option<Option<Arc<regex::Regex>>>,
    /// Custom validity message set via `setCustomValidity(message)`
    /// (HTML §4.10.20.2).  `None` means no custom error;
    /// `Some(empty)` is also "no custom error" per spec — both
    /// trigger `customError = false`.
    pub custom_validity_message: Option<String>,
    /// IDL-only `indeterminate` bit for `<input type=checkbox>`
    /// (HTML §4.10.5.1.16).  Independent of `checked`; observable
    /// via the `:indeterminate` CSS pseudo-class once styling lands.
    pub indeterminate: bool,
}

impl Default for FormControlState {
    fn default() -> Self {
        Self {
            kind: FormControlKind::TextInput,
            value: String::new(),
            checked: false,
            disabled: false,
            placeholder: String::new(),
            cursor_pos: 0,
            readonly: false,
            rows: 0,
            cols: 0,
            name: String::new(),
            default_value: String::new(),
            dirty_value: false,
            default_checked: false,
            required: false,
            minlength: None,
            maxlength: None,
            pattern: None,
            form_owner: None,
            autofocus: false,
            scroll_offset_x: 0.0,
            multiple: false,
            size: 0,
            autocomplete: String::new(),
            selection_start: 0,
            selection_end: 0,
            selection_direction: SelectionDirection::None,
            composition_text: None,
            selected_index: -1,
            options: Vec::new(),
            dropdown_open: false,
            char_count: 0,
            min: None,
            max: None,
            step: None,
            cached_pattern_regex: None,
            custom_validity_message: None,
            indeterminate: false,
        }
    }
}

impl FormControlState {
    /// Update the cached `char_count` from the current value.
    pub(crate) fn update_char_count(&mut self) {
        self.char_count = self.value.chars().count();
    }

    /// Update the `pattern` attribute and rebuild the cached regex.
    ///
    /// Pass `None` to remove the pattern constraint.
    pub fn update_pattern(&mut self, pattern: Option<&str>) {
        self.pattern = pattern.map(String::from);
        self.cached_pattern_regex = pattern.map(compile_pattern_regex);
    }

    /// Return `cursor_pos` clamped to `value.len()` and aligned to a char boundary.
    #[must_use]
    pub fn safe_cursor_pos(&self) -> usize {
        util::snap_to_char_boundary(&self.value, self.cursor_pos)
    }

    /// Return ordered `(start, end)` from selection, clamped and char-boundary-aligned.
    #[must_use]
    pub fn safe_selection_range(&self) -> (usize, usize) {
        let start = self.selection_start.min(self.selection_end);
        let end = self.selection_start.max(self.selection_end);
        let start = util::snap_to_char_boundary(&self.value, start);
        let end = util::snap_to_char_boundary(&self.value, end);
        (start, end)
    }

    // ---- Read accessors (public API for external crates) ----

    /// Returns the current value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the cached character count.
    #[must_use]
    pub fn char_count(&self) -> usize {
        self.char_count
    }

    /// Returns whether the value has been modified by the user.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty_value
    }

    /// Returns the cursor position (byte offset).
    #[must_use]
    pub fn cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    /// Returns the selection start (byte offset).
    #[must_use]
    pub fn selection_start(&self) -> usize {
        self.selection_start
    }

    /// Returns the selection end (byte offset).
    #[must_use]
    pub fn selection_end(&self) -> usize {
        self.selection_end
    }

    // ---- High-level editing methods ----

    /// Settle a freshly-established `self.value`: run the §4.10.5.1.x
    /// per-type value sanitization, re-sync the cached `char_count`, and
    /// CLAMP the text-entry cursor / selection into the (possibly
    /// shorter) value so the "selection is within the value" invariant
    /// holds by construction.
    ///
    /// **Policy-free** — it never moves the cursor to a spec-defined
    /// position; that is the caller's job at the two normative sites
    /// (HTML §4.10.5.4 `value`-setter step 5 → end, §4.10.5 type-change
    /// step 9 → beginning) via [`Self::move_text_cursor_to`].  The clamp
    /// runs UNCONDITIONALLY (not only when sanitization changes the
    /// value), because a value-establishment that shortens `value`
    /// without sanitization further changing it would otherwise leave a
    /// stale out-of-bounds selection.  Every value-establishment site
    /// (`set_value`, `set_value_initial`, `reset_value`,
    /// `from_input_element`, the reconciler `value`-arm,
    /// `sanitize_for_type_change`) sets `self.value` then calls this —
    /// no site is exempt.
    ///
    /// The clamp operates on the crate's **byte** offsets, whereas HTML
    /// §4.10.20 preserves/clamps the relevant-value-change selection in
    /// **UTF-16 code-unit** offsets.  For ASCII values the two coincide; for
    /// a multibyte value change an in-range byte offset can correspond to a
    /// different code-unit offset than the spec preserves.  The whole
    /// selection subsystem is byte-internal (converting to UTF-16 only at
    /// the IDL boundary), so code-unit-correct clamping is the deferred
    /// `#11-selection-offset-utf16-units` migration, not a per-call-site fix.
    pub(crate) fn settle_value(&mut self) {
        sanitize::sanitize_value(self);
        self.update_char_count();
        self.cursor_pos = util::snap_to_char_boundary(&self.value, self.cursor_pos);
        self.selection_start = util::snap_to_char_boundary(&self.value, self.selection_start);
        self.selection_end = util::snap_to_char_boundary(&self.value, self.selection_end);
    }

    /// Move the text entry cursor to `pos`, unselect any selection, and
    /// reset the selection direction to "none".
    ///
    /// Implements the cursor move shared by HTML §4.10.5.4 `value`-setter
    /// step 5 (`pos = value.len()`, the end) and the §4.10.5 type-change
    /// step 9 (`pos = 0`, the beginning).  `pos` is assumed in-bounds and
    /// char-aligned (callers pass `value.len()` or `0`).
    pub(crate) fn move_text_cursor_to(&mut self, pos: usize) {
        self.cursor_pos = pos;
        self.selection_start = pos;
        self.selection_end = pos;
        self.selection_direction = SelectionDirection::None;
    }

    /// Set the value via the IDL `value` setter (marks the control dirty).
    ///
    /// Implements HTML §4.10.5.4 (`value` IDL attribute, mode "value"):
    /// step 1 capture `oldValue`; step 2 set the value; step 3 set the
    /// dirty value flag; step 4 invoke value sanitization; step 5 if the
    /// value (after sanitization) differs from `oldValue` AND the control
    /// has a text entry cursor position, move the cursor to the end,
    /// unselect, and reset the selection direction to "none".  A set that
    /// sanitizes back to (or equals) the old value leaves the cursor /
    /// selection unchanged.
    pub fn set_value(&mut self, text: String) {
        let old = self.value.clone(); // step 1: oldValue
        self.value = text; // step 2
        self.dirty_value = true; // step 3
        self.settle_value(); // step 4 (sanitize) + bounds clamp
                             // step 5: the move fires only when the post-sanitization value
                             // changed AND the control "has a text entry cursor position".  That
                             // is the editable-text set (`has_selectable_text` = the kinds whose
                             // key input maintains `cursor_pos` via the text handler: text /
                             // search / tel / url / password / textarea / email / number) — it is
                             // BROADER than `supports_selection` (the "setRangeText() applies"
                             // set, which excludes email/number): those controls have an editing
                             // cursor even though their `selectionStart` getter does not apply, so
                             // a stale `cursor_pos` would mis-place the next typed character.
        if self.value != old && self.kind.has_selectable_text() {
            self.move_text_cursor_to(self.value.len());
        }
    }

    /// Set the value during initialization (`dirty_value` stays false).
    ///
    /// Also sets `default_value` for form reset.  This is a "relevant value
    /// change" with **no** explicit cursor-move policy (unlike the IDL
    /// `value` setter step 5), so per HTML §4.10.20 the cursor / selection
    /// are only CLAMPED into the new value — not collapsed to the end and
    /// not direction-reset.  `settle_value` performs exactly that clamp.
    pub fn set_value_initial(&mut self, text: String) {
        self.default_value.clone_from(&text);
        self.value = text;
        self.settle_value();
    }

    /// Reset to default value (form reset behavior).
    ///
    /// Restores `default_value`, clears the dirty flag, and restores
    /// checked / indeterminate.
    ///
    /// Per HTML §4.10.5 reset algorithm for `<input>`: set the dirty value
    /// flag back to false, set the value to the `value` content attribute
    /// (or empty), restore checkedness from the `checked` content
    /// attribute, set indeterminate back to false, and invoke value
    /// sanitization.  The reset algorithm carries **no** cursor-move policy,
    /// so the relevant-value-change rule (HTML §4.10.20) applies: the cursor
    /// / selection are only CLAMPED into the restored value (positions and
    /// selection direction otherwise preserved), via `settle_value`.
    pub fn reset_value(&mut self) {
        self.value = self.default_value.clone();
        self.dirty_value = false;
        self.checked = self.default_checked;
        self.indeterminate = false;
        self.settle_value();
    }

    /// HTML §4.10.5 type-change **step 2** (previous mode ≠ value, new
    /// value mode): set the live value to the `value` content attribute
    /// (or `""`), then clear the dirty value flag.  The caller passes the
    /// CURRENT `value` content attribute (`content`) read straight from
    /// `Attributes`.
    ///
    /// Both the live `value` and the `default_value` mirror are set from the
    /// single `content` read, so `value == default_value == the value content
    /// attribute` holds **by construction** after this call.  Setting only
    /// `value` (reading the mirror would have been the alternative) would let
    /// the two diverge whenever the mirror is stale — a non-dispatching
    /// buffered `SetAttribute` flush (`SessionCore::flush` →
    /// `apply_set_attribute`) writes `Attributes` without running the
    /// `FormControlReconciler` `value`-arm that maintains the mirror — and a
    /// later [`reset_value`](Self::reset_value) or step-base calculation
    /// (which read `default_value`) would then resurrect the stale value.
    /// Re-deriving the mirror here is convergent (in the common, dispatched
    /// case it already equals `content`, so this is a no-op), not a competing
    /// maintainer: the reconciler maintains the mirror on `value`-attribute
    /// changes, while this re-establishes it on the value-MODE change.
    ///
    /// **No sanitize / cursor move here** — the type-change algorithm
    /// sanitizes at step 6
    /// ([`sanitize_for_type_change`](crate::sanitize_for_type_change), which
    /// settles under the new kind), so this sets the raw value only.
    pub(crate) fn set_value_from_content_attr(&mut self, content: String) {
        self.default_value.clone_from(&content);
        self.value = content;
        self.dirty_value = false;
        self.update_char_count();
    }

    /// HTML §4.10.5 type-change **step 3** (previous mode ≠ filename, new
    /// filename mode): set the live value to the empty string.  **No
    /// sanitize here** — step 6 settles under the new kind.
    pub(crate) fn clear_value_for_type_change(&mut self) {
        self.value.clear();
        self.update_char_count();
    }

    /// HTML §4.10.5.4 filename-mode `value` setter, empty-string branch
    /// ("empty the list of selected files").  The selected-files list is not
    /// yet modeled (`#11-input-file-shell-staging`), but a file input can
    /// still carry a stale live backing value (e.g. a `value` content
    /// attribute present at creation); clear it so `file.value = ""` does not
    /// leave that value observable to form submission (§4.10.22.4).
    pub fn clear_file_value(&mut self) {
        self.value.clear();
        self.update_char_count();
    }

    /// Insert text at the current cursor position (marks as dirty).
    pub fn insert_at_cursor(&mut self, text: &str) {
        let pos = self.safe_cursor_pos();
        self.value.insert_str(pos, text);
        self.cursor_pos = pos + text.len();
        self.dirty_value = true;
        self.update_char_count();
    }

    /// Delete the character before the cursor (Backspace). Returns `true` if deleted.
    pub fn delete_backward(&mut self) -> bool {
        let pos = self.safe_cursor_pos();
        if pos > 0 {
            let prev = util::prev_char_boundary(&self.value, pos);
            self.value.drain(prev..pos);
            self.cursor_pos = prev;
            self.dirty_value = true;
            self.update_char_count();
            true
        } else {
            false
        }
    }

    /// Delete the character after the cursor (Delete key). Returns `true` if deleted.
    pub fn delete_forward(&mut self) -> bool {
        let pos = self.safe_cursor_pos();
        if pos < self.value.len() {
            let next = util::next_char_boundary(&self.value, pos);
            self.value.drain(pos..next);
            self.dirty_value = true;
            self.update_char_count();
            true
        } else {
            false
        }
    }

    /// Replace the current selection with the given text (marks as dirty).
    ///
    /// If there is no selection, inserts at the cursor position.
    pub fn replace_selection(&mut self, text: &str) {
        let (start, end) = self.safe_selection_range();
        if start != end {
            self.value.drain(start..end);
        }
        self.value.insert_str(start, text);
        self.cursor_pos = start + text.len();
        self.selection_start = self.cursor_pos;
        self.selection_end = self.cursor_pos;
        self.dirty_value = true;
        self.update_char_count();
    }

    /// Set the cursor position (snapped to char boundary).
    pub fn set_cursor(&mut self, pos: usize) {
        self.cursor_pos = util::snap_to_char_boundary(&self.value, pos);
    }

    /// Set the selection range (snapped to char boundaries).
    pub fn set_selection(&mut self, start: usize, end: usize) {
        self.selection_start = util::snap_to_char_boundary(&self.value, start);
        self.selection_end = util::snap_to_char_boundary(&self.value, end);
    }

    /// Set the selection start (snapped to char boundary).
    pub fn set_selection_start(&mut self, pos: usize) {
        self.selection_start = util::snap_to_char_boundary(&self.value, pos);
    }

    /// Set the selection end (snapped to char boundary).
    pub fn set_selection_end(&mut self, pos: usize) {
        self.selection_end = util::snap_to_char_boundary(&self.value, pos);
    }

    /// Create a `FormControlState` from an element's tag name and attributes.
    ///
    /// Returns `None` if the element is not a recognized form control.
    #[must_use]
    pub fn from_element(tag: &str, attrs: &Attributes) -> Option<Self> {
        match tag {
            "input" => Some(Self::from_input_element(attrs)),
            "button" => Some(Self::from_button_element(attrs)),
            "textarea" => Some(Self::from_textarea_element(attrs)),
            "select" => Some(Self::from_select_element(attrs)),
            "output" => Some(Self {
                kind: FormControlKind::Output,
                name: attrs.get("name").unwrap_or("").to_string(),
                ..Self::default()
            }),
            "meter" => Some(Self {
                kind: FormControlKind::Meter,
                ..Self::default()
            }),
            "progress" => Some(Self {
                kind: FormControlKind::Progress,
                ..Self::default()
            }),
            _ => None,
        }
    }

    /// Parse `<input>` element attributes into form control state.
    fn from_input_element(attrs: &Attributes) -> Self {
        // HTML spec §2.5.2: enumerated attributes are ASCII case-insensitive.
        let kind = FormControlKind::from_tag_and_type_attr("input", attrs.get("type"))
            .unwrap_or(FormControlKind::TextInput);
        let raw_value = attrs.get("value").unwrap_or("").to_string();
        let value = if kind == FormControlKind::SubmitButton && raw_value.is_empty() {
            "Submit".to_string()
        } else if kind == FormControlKind::ResetButton && raw_value.is_empty() {
            "Reset".to_string()
        } else {
            raw_value.clone()
        };
        let checked = attrs.contains("checked");
        let char_count = value.chars().count();
        let mut state = Self {
            kind,
            // HTML §4.10.20: "The initial state must consist of a text entry
            // cursor at the beginning of the control." `cursor_pos` therefore
            // defaults to 0 (not the value length) at element creation;
            // `settle_value` below clamps it (a no-op at 0).
            default_value: raw_value,
            char_count,
            value,
            checked,
            default_checked: checked,
            disabled: attrs.contains("disabled"),
            readonly: attrs.contains("readonly"),
            placeholder: attrs.get("placeholder").unwrap_or("").to_string(),
            name: attrs.get("name").unwrap_or("").to_string(),
            required: attrs.contains("required"),
            minlength: attrs.get("minlength").and_then(|v| v.parse().ok()),
            maxlength: attrs.get("maxlength").and_then(|v| v.parse().ok()),
            pattern: attrs.get("pattern").map(String::from),
            cached_pattern_regex: attrs.get("pattern").map(compile_pattern_regex),
            form_owner: attrs.get("form").map(String::from),
            autofocus: attrs.contains("autofocus"),
            autocomplete: attrs.get("autocomplete").unwrap_or("").to_string(),
            // `multiple` drives the Email-state value sanitization mode
            // (§4.10.5.1.5) as well as `<select multiple>`, so it must be
            // read at parse time for the sanitize call below to pick the
            // comma-list algorithm for `<input type=email multiple>`.
            multiple: attrs.contains("multiple"),
            min: attrs.get("min").map(String::from),
            max: attrs.get("max").map(String::from),
            step: attrs.get("step").map(String::from),
            ..Self::default()
        };
        // HTML value sanitization (§4.10.5.1.x) at element creation: the
        // struct-literal parse is a value-establishment site, so it is
        // settled through the same canonical primitive as every other site
        // (a raw `<input type=range value=150>` becomes the clamped value,
        // etc.).  Element creation carries no cursor-move policy, so this is
        // the clamp-only path: per HTML §4.10.20 the initial text entry cursor
        // is at the beginning (`cursor_pos` defaults to 0 above), and
        // `settle_value` only clamps it into bounds (a no-op at 0).
        state.settle_value();
        state
    }

    /// Parse `<button>` element attributes.
    ///
    /// Per HTML §4.10.6: `<button>` reflects `name` and `value` content attributes.
    /// The submit button's name/value pair is appended to form data on submission.
    fn from_button_element(attrs: &Attributes) -> Self {
        let disabled = attrs.contains("disabled");
        let kind = FormControlKind::from_tag_and_type_attr("button", attrs.get("type"))
            .unwrap_or(FormControlKind::SubmitButton);
        Self {
            kind,
            value: attrs.get("value").unwrap_or("").to_string(),
            disabled,
            name: attrs.get("name").unwrap_or("").to_string(),
            form_owner: attrs.get("form").map(String::from),
            autofocus: attrs.contains("autofocus"),
            ..Self::default()
        }
    }

    /// Parse `<textarea>` element attributes.
    fn from_textarea_element(attrs: &Attributes) -> Self {
        Self {
            kind: FormControlKind::TextArea,
            disabled: attrs.contains("disabled"),
            readonly: attrs.contains("readonly"),
            placeholder: attrs.get("placeholder").unwrap_or("").to_string(),
            rows: attrs.get("rows").and_then(|v| v.parse().ok()).unwrap_or(2),
            cols: attrs.get("cols").and_then(|v| v.parse().ok()).unwrap_or(20),
            name: attrs.get("name").unwrap_or("").to_string(),
            required: attrs.contains("required"),
            minlength: attrs.get("minlength").and_then(|v| v.parse().ok()),
            maxlength: attrs.get("maxlength").and_then(|v| v.parse().ok()),
            ..Self::default()
        }
    }

    /// Parse `<select>` element attributes.
    fn from_select_element(attrs: &Attributes) -> Self {
        let multiple = attrs.contains("multiple");
        // HTML spec §4.10.5: default size is 4 for multiple, 1 for single.
        let default_size = if multiple { 4 } else { 1 };
        Self {
            kind: FormControlKind::Select,
            disabled: attrs.contains("disabled"),
            name: attrs.get("name").unwrap_or("").to_string(),
            required: attrs.contains("required"),
            multiple,
            size: attrs
                .get("size")
                .and_then(|v| v.parse().ok())
                .unwrap_or(default_size),
            ..Self::default()
        }
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
