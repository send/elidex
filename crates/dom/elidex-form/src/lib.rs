//! Form control state and input handling for elidex.
//!
//! Provides `FormControlState` ECS component for tracking the runtime state
//! of HTML form controls (`<input>`, `<button>`, `<textarea>`, `<label>`).
//! The `.value` property is distinct from the `value` HTML attribute per spec.

mod clipboard;
mod fieldset;
mod init;
mod input;
mod label;
pub mod radio;
mod select;
mod selection;
mod sizing;
mod submit;
pub mod util;
mod validation;

pub use clipboard::{clipboard_copy, clipboard_cut, clipboard_paste};
pub use fieldset::{first_legend_child, is_in_first_legend, propagate_fieldset_disabled};
pub use init::{create_form_control_state, find_autofocus_target, init_form_controls};
pub use input::{form_control_key_input, form_control_key_input_action, KeyAction};
pub use label::{find_label_target, is_label, resolve_label_for};
pub use radio::{
    find_radio_group, find_radio_group_scoped, is_radio_group_satisfied, radio_arrow_navigate,
    toggle_radio,
};
pub use select::{init_select_options, navigate_select, select_option};
pub use selection::{collapse_selection, delete_selection, extend_selection, select_all};
pub use sizing::form_intrinsic_size;
pub use submit::{
    build_form_submission, collect_form_data, encode_form_urlencoded, find_form_ancestor,
    read_form_attrs, reset_form, FormAttrs, FormDataEntry, FormSubmission,
};
pub use validation::{validate_control, ValidityState};

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

    /// Returns `true` if the control supports the selection API (HTML §4.10.5.2.10).
    ///
    /// Text/password/textarea and text-like input types (email/url/tel/search) support
    /// `selectionStart`/`selectionEnd`/`setSelectionRange()`.
    #[must_use]
    pub fn supports_selection(self) -> bool {
        self.is_text_control()
    }

    /// Returns `true` if this kind participates in form submission (HTML §4.10.15.3).
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
                | Self::File
                | Self::Hidden
        )
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
    pub value: String,
    /// Whether the control is checked (only meaningful for checkboxes/radios).
    pub checked: bool,
    /// Whether the control is disabled.
    pub disabled: bool,
    /// Placeholder text (displayed when value is empty).
    pub placeholder: String,
    /// Cursor position within the value string (byte offset).
    pub cursor_pos: usize,
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
    pub dirty_value: bool,
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
    pub selection_start: usize,
    /// Selection end (byte offset, for text controls).
    pub selection_end: usize,
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
    /// Cached character count — **must** be kept in sync with `value` via
    /// [`update_char_count()`](Self::update_char_count) after every mutation.
    /// Used by maxlength enforcement and validation (O(1) instead of O(n)).
    pub char_count: usize,
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
        }
    }
}

impl FormControlState {
    /// Update the cached `char_count` from the current value.
    ///
    /// Call this after any mutation of `self.value`.
    pub fn update_char_count(&mut self) {
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
        let raw_type = attrs.get("type").unwrap_or("text");
        let input_type = raw_type.to_ascii_lowercase();
        let kind = FormControlKind::from_type_str(&input_type);
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
        Self {
            kind,
            cursor_pos: value.len(),
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
            min: attrs.get("min").map(String::from),
            max: attrs.get("max").map(String::from),
            step: attrs.get("step").map(String::from),
            ..Self::default()
        }
    }

    /// Parse `<button>` element attributes.
    ///
    /// Per HTML §4.10.6: `<button>` reflects `name` and `value` content attributes.
    /// The submit button's name/value pair is appended to form data on submission.
    fn from_button_element(attrs: &Attributes) -> Self {
        let disabled = attrs.contains("disabled");
        let btn_type = attrs.get("type").unwrap_or("submit").to_ascii_lowercase();
        let kind = match btn_type.as_str() {
            "reset" => FormControlKind::ResetButton,
            "button" => FormControlKind::Button,
            _ => FormControlKind::SubmitButton,
        };
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
            size: attrs.get("size").and_then(|v| v.parse().ok()).unwrap_or(default_size),
            ..Self::default()
        }
    }

}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
