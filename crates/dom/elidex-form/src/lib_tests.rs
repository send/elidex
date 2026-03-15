use super::*;

#[test]
fn text_input_from_element() {
    let attrs = Attributes::default();
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert_eq!(state.kind, FormControlKind::TextInput);
    assert!(!state.checked);
    assert!(!state.disabled);
    assert!(state.value.is_empty());
}

#[test]
fn checkbox_from_element() {
    let mut attrs = Attributes::default();
    attrs.set("type", "checkbox");
    attrs.set("checked", "");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert_eq!(state.kind, FormControlKind::Checkbox);
    assert!(state.checked);
}

#[test]
fn submit_button_default_label() {
    let mut attrs = Attributes::default();
    attrs.set("type", "submit");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert_eq!(state.kind, FormControlKind::SubmitButton);
    assert_eq!(state.value, "Submit");
}

#[test]
fn submit_button_custom_label() {
    let mut attrs = Attributes::default();
    attrs.set("type", "submit");
    attrs.set("value", "Go");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert_eq!(state.value, "Go");
}

#[test]
fn button_element() {
    let attrs = Attributes::default();
    let state = FormControlState::from_element("button", &attrs).unwrap();
    assert_eq!(state.kind, FormControlKind::SubmitButton);
}

#[test]
fn textarea_element() {
    let mut attrs = Attributes::default();
    attrs.set("placeholder", "Enter text...");
    attrs.set("disabled", "");
    let state = FormControlState::from_element("textarea", &attrs).unwrap();
    assert_eq!(state.kind, FormControlKind::TextArea);
    assert!(state.disabled);
    assert_eq!(state.placeholder, "Enter text...");
}

#[test]
fn hidden_input_returns_some() {
    let mut attrs = Attributes::default();
    attrs.set("type", "hidden");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert_eq!(state.kind, FormControlKind::Hidden);
}

#[test]
fn input_type_case_insensitive() {
    let mut attrs = Attributes::default();
    attrs.set("type", "Checkbox");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert_eq!(state.kind, FormControlKind::Checkbox);

    let mut attrs = Attributes::default();
    attrs.set("type", "SUBMIT");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert_eq!(state.kind, FormControlKind::SubmitButton);
}

#[test]
fn non_form_element_returns_none() {
    let attrs = Attributes::default();
    assert!(FormControlState::from_element("div", &attrs).is_none());
    assert!(FormControlState::from_element("span", &attrs).is_none());
}

#[test]
fn disabled_attribute() {
    let mut attrs = Attributes::default();
    attrs.set("disabled", "");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert!(state.disabled);
}

#[test]
fn reset_button_default_label() {
    let mut attrs = Attributes::default();
    attrs.set("type", "reset");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert_eq!(state.kind, FormControlKind::ResetButton);
    assert_eq!(state.value, "Reset");
}

#[test]
fn reset_button_custom_label() {
    let mut attrs = Attributes::default();
    attrs.set("type", "reset");
    attrs.set("value", "Clear");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert_eq!(state.kind, FormControlKind::ResetButton);
    assert_eq!(state.value, "Clear");
}

#[test]
fn textarea_rows_cols() {
    let mut attrs = Attributes::default();
    attrs.set("rows", "5");
    attrs.set("cols", "40");
    let state = FormControlState::from_element("textarea", &attrs).unwrap();
    assert_eq!(state.rows, 5);
    assert_eq!(state.cols, 40);
}

#[test]
fn textarea_default_rows_cols() {
    let attrs = Attributes::default();
    let state = FormControlState::from_element("textarea", &attrs).unwrap();
    assert_eq!(state.rows, 2);
    assert_eq!(state.cols, 20);
}

#[test]
fn readonly_attribute() {
    let mut attrs = Attributes::default();
    attrs.set("readonly", "");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert!(state.readonly);
}

#[test]
fn safe_selection_range_basic() {
    let mut attrs = Attributes::default();
    attrs.set("value", "hello");
    let mut state = FormControlState::from_element("input", &attrs).unwrap();
    state.selection_start = 1;
    state.selection_end = 4;
    assert_eq!(state.safe_selection_range(), (1, 4));
}

#[test]
fn safe_selection_range_reversed() {
    let mut attrs = Attributes::default();
    attrs.set("value", "hello");
    let mut state = FormControlState::from_element("input", &attrs).unwrap();
    state.selection_start = 4;
    state.selection_end = 1;
    assert_eq!(state.safe_selection_range(), (1, 4));
}

#[test]
fn safe_selection_range_beyond_len() {
    let mut attrs = Attributes::default();
    attrs.set("value", "hi");
    let mut state = FormControlState::from_element("input", &attrs).unwrap();
    state.selection_start = 0;
    state.selection_end = 100;
    let (start, end) = state.safe_selection_range();
    assert_eq!(start, 0);
    assert_eq!(end, 2);
}

#[test]
fn default_checked_preserved_on_reset() {
    let mut attrs = Attributes::default();
    attrs.set("type", "checkbox");
    attrs.set("checked", "");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert!(state.default_checked);
    assert!(state.checked);
}

#[test]
fn button_type_reset_is_reset_button() {
    let mut attrs = Attributes::default();
    attrs.set("type", "reset");
    let state = FormControlState::from_element("button", &attrs).unwrap();
    assert_eq!(state.kind, FormControlKind::ResetButton);
}

#[test]
fn safe_selection_range_multibyte_boundary() {
    let attrs = Attributes::default();
    let mut state = FormControlState::from_element("input", &attrs).unwrap();
    state.value = "あい".to_string(); // 6 bytes
                                      // Set selection_start to mid-char boundary (byte 1 of 'あ')
    state.selection_start = 1;
    state.selection_end = 4; // byte 4 is mid-char of 'い'
    let (start, end) = state.safe_selection_range();
    assert!(state.value.is_char_boundary(start));
    assert!(state.value.is_char_boundary(end));
    assert_eq!(start, 0); // snapped back to 0
    assert_eq!(end, 3); // snapped back to 3
}

#[test]
fn new_input_types_from_element() {
    let cases = [
        ("email", FormControlKind::Email),
        ("url", FormControlKind::Url),
        ("tel", FormControlKind::Tel),
        ("search", FormControlKind::Search),
        ("number", FormControlKind::Number),
        ("range", FormControlKind::Range),
        ("color", FormControlKind::Color),
        ("date", FormControlKind::Date),
        ("datetime-local", FormControlKind::DatetimeLocal),
        ("file", FormControlKind::File),
    ];
    for (type_str, expected_kind) in cases {
        let mut attrs = Attributes::default();
        attrs.set("type", type_str);
        let state = FormControlState::from_element("input", &attrs)
            .unwrap_or_else(|| panic!("Expected Some for type={type_str}"));
        assert_eq!(
            state.kind, expected_kind,
            "type={type_str} should map to {expected_kind:?}"
        );
    }
}

#[test]
fn hidden_input_participates_in_form_data() {
    let mut attrs = Attributes::default();
    attrs.set("type", "hidden");
    attrs.set("name", "csrf");
    attrs.set("value", "token123");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert_eq!(state.kind, FormControlKind::Hidden);
    assert_eq!(state.name, "csrf");
    assert_eq!(state.value, "token123");
}

#[test]
fn text_like_types_are_text_controls() {
    for kind in [
        FormControlKind::Email,
        FormControlKind::Url,
        FormControlKind::Tel,
        FormControlKind::Search,
    ] {
        assert!(kind.is_text_control(), "{kind:?} should be a text control");
        assert!(
            kind.supports_selection(),
            "{kind:?} should support selection"
        );
    }
}

#[test]
fn non_text_like_types_not_text_controls() {
    for kind in [
        FormControlKind::Number,
        FormControlKind::Range,
        FormControlKind::Color,
        FormControlKind::Date,
        FormControlKind::DatetimeLocal,
        FormControlKind::File,
        FormControlKind::Hidden,
    ] {
        assert!(
            !kind.is_text_control(),
            "{kind:?} should not be a text control"
        );
    }
}

#[test]
fn is_single_line_text() {
    assert!(FormControlKind::TextInput.is_single_line_text());
    assert!(FormControlKind::Password.is_single_line_text());
    assert!(FormControlKind::Email.is_single_line_text());
    assert!(!FormControlKind::TextArea.is_single_line_text());
    assert!(!FormControlKind::Checkbox.is_single_line_text());
}

#[test]
fn supports_selection_false_for_non_text() {
    // HTML §4.10.5.2.10: selection API is only for text/password/textarea + text-like types.
    for kind in [
        FormControlKind::Checkbox,
        FormControlKind::Radio,
        FormControlKind::Hidden,
        FormControlKind::Number,
        FormControlKind::Range,
        FormControlKind::Color,
        FormControlKind::Date,
        FormControlKind::DatetimeLocal,
        FormControlKind::File,
        FormControlKind::SubmitButton,
        FormControlKind::Button,
        FormControlKind::Select,
        FormControlKind::Output,
        FormControlKind::Meter,
        FormControlKind::Progress,
    ] {
        assert!(
            !kind.supports_selection(),
            "{kind:?} should not support selection"
        );
    }
}

#[test]
fn as_str_round_trip() {
    assert_eq!(FormControlKind::TextInput.as_str(), "text");
    assert_eq!(FormControlKind::Password.as_str(), "password");
    assert_eq!(FormControlKind::Select.as_str(), "select-one");
    assert_eq!(FormControlKind::Hidden.as_str(), "hidden");
    assert_eq!(FormControlKind::DatetimeLocal.as_str(), "datetime-local");
}

#[test]
fn from_type_str_round_trip() {
    // All kinds that have a string representation should round-trip
    // through as_str → from_type_str.
    let kinds = [
        FormControlKind::TextInput,
        FormControlKind::Password,
        FormControlKind::Checkbox,
        FormControlKind::Radio,
        FormControlKind::SubmitButton,
        FormControlKind::ResetButton,
        FormControlKind::Button,
        FormControlKind::TextArea,
        FormControlKind::Select,
        FormControlKind::Email,
        FormControlKind::Url,
        FormControlKind::Tel,
        FormControlKind::Search,
        FormControlKind::Number,
        FormControlKind::Range,
        FormControlKind::Color,
        FormControlKind::Date,
        FormControlKind::DatetimeLocal,
        FormControlKind::File,
        FormControlKind::Hidden,
    ];
    for kind in kinds {
        let s = kind.as_str();
        let round_tripped = FormControlKind::from_type_str(s);
        assert_eq!(
            round_tripped, kind,
            "from_type_str({s:?}) should return {kind:?}"
        );
    }
}

#[test]
fn char_count_tracks_value() {
    let mut attrs = Attributes::default();
    attrs.set("value", "hello");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert_eq!(state.char_count, 5);

    let mut state = FormControlState {
        value: "あいう".to_string(),
        ..FormControlState::default()
    };
    state.update_char_count();
    assert_eq!(state.char_count, 3);
}

#[test]
fn update_pattern_rebuilds_cache() {
    let mut attrs = Attributes::default();
    attrs.set("pattern", "[0-9]+");
    attrs.set("value", "abc");
    let mut state = FormControlState::from_element("input", &attrs).unwrap();
    // Original pattern: digits only → "abc" should mismatch.
    assert!(crate::validate_control(&state).pattern_mismatch);

    // Update pattern to accept letters.
    state.update_pattern(Some("[a-z]+"));
    assert!(!crate::validate_control(&state).pattern_mismatch);

    // Remove pattern.
    state.update_pattern(None);
    assert!(!crate::validate_control(&state).pattern_mismatch);
}

#[test]
fn is_submittable_kinds() {
    assert!(FormControlKind::TextInput.is_submittable());
    assert!(FormControlKind::Hidden.is_submittable());
    assert!(FormControlKind::Select.is_submittable());
    assert!(!FormControlKind::SubmitButton.is_submittable());
    assert!(!FormControlKind::ResetButton.is_submittable());
    assert!(!FormControlKind::Button.is_submittable());
    assert!(!FormControlKind::Output.is_submittable());
    assert!(!FormControlKind::Meter.is_submittable());
}

#[test]
fn min_max_step_parsed() {
    let mut attrs = Attributes::default();
    attrs.set("type", "number");
    attrs.set("min", "0");
    attrs.set("max", "100");
    attrs.set("step", "5");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert_eq!(state.min.as_deref(), Some("0"));
    assert_eq!(state.max.as_deref(), Some("100"));
    assert_eq!(state.step.as_deref(), Some("5"));
}

#[test]
fn output_meter_progress_kinds() {
    let output = FormControlState::from_element("output", &Attributes::default()).unwrap();
    assert_eq!(output.kind, FormControlKind::Output);
    let meter = FormControlState::from_element("meter", &Attributes::default()).unwrap();
    assert_eq!(meter.kind, FormControlKind::Meter);
    let progress = FormControlState::from_element("progress", &Attributes::default()).unwrap();
    assert_eq!(progress.kind, FormControlKind::Progress);
}

#[test]
fn button_name_and_value() {
    let mut attrs = Attributes::default();
    attrs.set("name", "action");
    attrs.set("value", "save");
    let state = FormControlState::from_element("button", &attrs).unwrap();
    assert_eq!(state.name, "action");
    assert_eq!(state.value, "save");
    assert_eq!(state.kind, FormControlKind::SubmitButton);
}

#[test]
fn button_without_name_or_value() {
    let state = FormControlState::from_element("button", &Attributes::default()).unwrap();
    assert!(state.name.is_empty());
    assert!(state.value.is_empty());
}

#[test]
fn textarea_minlength_maxlength() {
    let mut attrs = Attributes::default();
    attrs.set("minlength", "5");
    attrs.set("maxlength", "100");
    let state = FormControlState::from_element("textarea", &attrs).unwrap();
    assert_eq!(state.minlength, Some(5));
    assert_eq!(state.maxlength, Some(100));
}

#[test]
fn textarea_without_minlength_maxlength() {
    let state = FormControlState::from_element("textarea", &Attributes::default()).unwrap();
    assert_eq!(state.minlength, None);
    assert_eq!(state.maxlength, None);
}
