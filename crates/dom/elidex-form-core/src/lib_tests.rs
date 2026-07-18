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
    }
    // The text-selection APIs (selectionStart/End/Direction, setRangeText,
    // setSelectionRange) apply to Text/Search/Tel/URL/Password (+textarea)
    // but NOT to Email — HTML §4.10.5.1.5 lists them under "do not apply"
    // for the Email state (only `select()` applies).  `supports_selection`
    // is the canonical "setRangeText() applies" predicate, so it excludes
    // Email even though `is_text_control` includes it for editing.
    for kind in [
        FormControlKind::Url,
        FormControlKind::Tel,
        FormControlKind::Search,
        FormControlKind::TextInput,
        FormControlKind::Password,
        FormControlKind::TextArea,
    ] {
        assert!(
            kind.supports_selection(),
            "{kind:?} should support selection"
        );
    }
    assert!(
        !FormControlKind::Email.supports_selection(),
        "Email must NOT support the text-selection APIs (HTML §4.10.5.1.5)"
    );
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
    // HTML §4.10.20: selection API is only for text/password/textarea + text-like types.
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

// ---- Editing method tests (R2 #33) ----

#[test]
fn set_value_moves_cursor_to_end() {
    let mut state = FormControlState::default();
    state.set_value("hello".to_string());
    assert_eq!(state.value(), "hello");
    assert_eq!(state.cursor_pos(), 5);
    assert_eq!(state.selection_start(), 5);
    assert_eq!(state.selection_end(), 5);
    assert!(state.is_dirty());
    assert_eq!(state.char_count(), 5);
}

#[test]
fn set_value_unchanged_preserves_cursor_and_selection() {
    // HTML §4.10.5.4 `value`-setter step 5 (R4-F10): a set whose
    // post-sanitization value EQUALS the old value must NOT move the
    // cursor, collapse the selection, or reset the direction.
    let mut state = FormControlState::default(); // TextInput → supports selection
    state.set_value("hello".to_string());
    state.cursor_pos = 3;
    state.selection_start = 1;
    state.selection_end = 3;
    state.selection_direction = SelectionDirection::Forward;
    state.set_value("hello".to_string()); // same value
    assert_eq!(
        state.cursor_pos(),
        3,
        "el.value=el.value must not move cursor"
    );
    assert_eq!(state.selection_start(), 1);
    assert_eq!(state.selection_end(), 3);
    assert_eq!(state.selection_direction, SelectionDirection::Forward);
}

#[test]
fn set_value_sanitize_back_to_old_does_not_move_cursor() {
    // R4-F10: a set that SANITIZES back to the current value (TextInput
    // strips newlines) is "unchanged" for step 5 → no cursor move.
    let mut state = FormControlState::default();
    state.set_value("ab".to_string());
    state.cursor_pos = 1;
    state.selection_start = 1;
    state.selection_end = 1;
    state.set_value("a\nb".to_string()); // sanitizes (newline strip) → "ab" == old
    assert_eq!(state.value(), "ab");
    assert_eq!(
        state.cursor_pos(),
        1,
        "sanitize-back-to-old must not move cursor"
    );
}

#[test]
fn set_value_changed_collapses_to_end_and_resets_direction() {
    // HTML §4.10.5.4 step 5 (R2-F5): a set that CHANGES the value moves the
    // cursor to the end, unselects, and resets selection direction to None.
    let mut state = FormControlState::default();
    state.set_value("hello".to_string());
    state.cursor_pos = 1;
    state.selection_start = 1;
    state.selection_end = 3;
    state.selection_direction = SelectionDirection::Backward;
    state.set_value("worldwide".to_string());
    assert_eq!(state.cursor_pos(), 9);
    assert_eq!(state.selection_start(), 9);
    assert_eq!(state.selection_end(), 9);
    assert_eq!(state.selection_direction, SelectionDirection::None);
}

#[test]
fn set_value_on_editable_text_kind_moves_cursor_to_end() {
    // Step 5's "has a text entry cursor position" is the editable-text set
    // (`has_selectable_text`), which is BROADER than `supports_selection`:
    // email and number have an editing cursor (key input maintains
    // `cursor_pos`) even though their `selectionStart` getter does not
    // apply.  So `el.value = ...` must move the cursor to the end for them,
    // or the next typed character lands at a stale position.
    for kind in [FormControlKind::Email, FormControlKind::Number] {
        let mut state = FormControlState {
            kind,
            ..FormControlState::default()
        };
        state.cursor_pos = 0;
        state.selection_start = 0;
        state.selection_end = 0;
        let v = if kind == FormControlKind::Number {
            "42"
        } else {
            "a@b.c"
        };
        state.set_value(v.to_string());
        assert_eq!(state.value(), v);
        assert_eq!(state.cursor_pos(), v.len(), "{kind:?}: cursor to end");
        assert_eq!(state.selection_start(), v.len());
        assert_eq!(state.selection_end(), v.len());
    }
}

#[test]
fn set_value_on_non_text_kind_does_not_move_cursor() {
    // A control with NO text entry cursor (`has_selectable_text` false —
    // range/checkbox/date pickers) gets no step-5 move; the cursor fields
    // stay put (they are inert for such kinds).
    for kind in [
        FormControlKind::Range,
        FormControlKind::Checkbox,
        FormControlKind::Date,
    ] {
        let mut state = FormControlState {
            kind,
            ..FormControlState::default()
        };
        state.cursor_pos = 0;
        state.selection_start = 0;
        state.selection_end = 0;
        state.set_value("5".to_string());
        assert_eq!(
            state.selection_start(),
            0,
            "{kind:?}: no text entry cursor → no step-5 move"
        );
        assert_eq!(state.selection_end(), 0);
    }
}

#[test]
fn type_change_newly_selectable_moves_cursor_to_beginning() {
    // HTML §4.10.5 type-change step 9 (R4-F8): a control that was NOT
    // selectable (hidden) and IS now (text) gets its cursor at the
    // beginning + selection direction "none".
    let mut state = FormControlState::default();
    state.set_value("hello".to_string()); // TextInput, cursor at end (5)
    state.selection_direction = SelectionDirection::Forward;
    state.kind = FormControlKind::TextInput; // new kind
    sanitize_for_type_change(&mut state, FormControlKind::Hidden);
    assert_eq!(
        state.cursor_pos(),
        0,
        "newly-selectable → cursor to beginning"
    );
    assert_eq!(state.selection_start(), 0);
    assert_eq!(state.selection_end(), 0);
    assert_eq!(state.selection_direction, SelectionDirection::None);
}

#[test]
fn type_change_both_selectable_preserves_cursor() {
    // text → search (both selectable) → step 9 does NOT fire; the cursor
    // is preserved (not reset to the beginning).
    let mut state = FormControlState::default();
    state.set_value("hello".to_string());
    state.cursor_pos = 3;
    state.selection_start = 3;
    state.selection_end = 3;
    state.kind = FormControlKind::Search;
    sanitize_for_type_change(&mut state, FormControlKind::TextInput);
    assert_eq!(state.cursor_pos(), 3, "both selectable → cursor preserved");
}

#[test]
fn type_change_to_email_does_not_move_cursor() {
    // Email is NOT selectable (HTML §4.10.5.1.5), so a hidden→email or
    // text→email change never fires step 9.
    let mut state = FormControlState::default();
    state.set_value("a@b.example".to_string());
    state.cursor_pos = 4;
    state.selection_start = 4;
    state.selection_end = 4;
    state.kind = FormControlKind::Email;
    sanitize_for_type_change(&mut state, FormControlKind::Hidden);
    assert_eq!(
        state.cursor_pos(),
        4,
        "email not selectable → no step-9 move"
    );
}

#[test]
fn set_value_initial_does_not_mark_dirty() {
    let mut state = FormControlState::default();
    state.set_value_initial("world".to_string());
    assert_eq!(state.value(), "world");
    assert!(!state.is_dirty());
    // §4.10.20: initial value establishment is clamp-only (no collapse to the
    // end); the default cursor (0) is in-bounds, so it stays at the beginning.
    assert_eq!(state.cursor_pos(), 0);
    assert_eq!(state.selection_start(), 0);
    assert_eq!(state.selection_end(), 0);
}

#[test]
fn relevant_value_change_clamps_without_resetting_direction() {
    // §4.10.20 (R4 regression): a non-setter relevant-value change (here a
    // shorter default value via reset) clamps an out-of-bounds selection but
    // preserves the selection direction — it must NOT reset it to "none"
    // (that reset belongs to the §4.10.5.4 value setter / type-change steps).
    let mut state = FormControlState::default();
    state.set_value("a long value".to_string()); // text input → IDL setter
    state.default_value = "hi".to_string();
    state.set_selection(5, 9);
    state.selection_direction = SelectionDirection::Backward;
    state.reset_value(); // value → "hi" (len 2), a relevant-value change
    assert_eq!(state.value(), "hi");
    assert_eq!(state.selection_start(), 2, "clamped to new length");
    assert_eq!(state.selection_end(), 2);
    assert_eq!(
        state.selection_direction,
        SelectionDirection::Backward,
        "direction preserved on a non-setter relevant-value change"
    );
}

#[test]
fn reset_value_restores_default() {
    let mut state = FormControlState::default();
    state.set_value_initial("original".to_string());
    state.set_value("modified".to_string());
    assert!(state.is_dirty());
    state.reset_value();
    assert_eq!(state.value(), "original");
    assert!(!state.is_dirty());
}

/// §4.10.5 reset "empty the list of selected files": a file control resets to
/// the empty string, NOT its `default_value` (the value content attribute
/// does not apply in filename mode).  Guards the resurrection path Codex
/// flagged — `file.value = ""` / type change clears `value`, but a stale
/// `default_value` (e.g. a `value` attr at creation) must not be restored on
/// form reset.
#[test]
fn reset_value_empties_file_control_not_default() {
    let mut attrs = Attributes::default();
    attrs.set("type", "file");
    attrs.set("value", "stale");
    let mut state = FormControlState::from_element("input", &attrs).unwrap();
    assert_eq!(state.kind, FormControlKind::File);
    // Premise: creation mirrors the `value` attr into the reset backing, so a
    // non-file-aware reset would resurrect it (this is what the fix guards).
    assert_eq!(state.default_value, "stale");
    // The file setter / type change empties the live backing.
    state.clear_file_value();
    assert_eq!(state.value(), "");
    // Form reset must NOT resurrect the stale value content attribute.
    state.reset_value();
    assert_eq!(
        state.value(),
        "",
        "file control reset empties the selected files, not restore default_value"
    );
}

#[test]
fn insert_at_cursor_basic() {
    let mut state = FormControlState::default();
    state.set_value_initial("ac".to_string());
    state.set_cursor(1);
    state.insert_at_cursor("b");
    assert_eq!(state.value(), "abc");
    assert_eq!(state.cursor_pos(), 2);
    assert!(state.is_dirty());
    assert_eq!(state.char_count(), 3);
}

#[test]
fn insert_at_cursor_multibyte() {
    let mut state = FormControlState::default();
    state.set_value_initial("あう".to_string());
    state.set_cursor(3); // after 'あ'
    state.insert_at_cursor("い");
    assert_eq!(state.value(), "あいう");
    assert_eq!(state.char_count(), 3);
}

#[test]
fn delete_backward_basic() {
    let mut state = FormControlState::default();
    state.set_value("abc".to_string());
    state.set_cursor(2);
    assert!(state.delete_backward());
    assert_eq!(state.value(), "ac");
    assert_eq!(state.cursor_pos(), 1);
}

#[test]
fn delete_backward_at_start() {
    let mut state = FormControlState::default();
    state.set_value("abc".to_string());
    state.set_cursor(0);
    assert!(!state.delete_backward());
    assert_eq!(state.value(), "abc");
}

#[test]
fn delete_forward_basic() {
    let mut state = FormControlState::default();
    state.set_value("abc".to_string());
    state.set_cursor(1);
    assert!(state.delete_forward());
    assert_eq!(state.value(), "ac");
    assert_eq!(state.cursor_pos(), 1);
}

#[test]
fn delete_forward_at_end() {
    let mut state = FormControlState::default();
    state.set_value("abc".to_string());
    // cursor is at end after set_value
    assert!(!state.delete_forward());
    assert_eq!(state.value(), "abc");
}

#[test]
fn replace_selection_with_text() {
    let mut state = FormControlState::default();
    state.set_value("hello world".to_string());
    state.set_selection(5, 11);
    state.replace_selection("!");
    assert_eq!(state.value(), "hello!");
    assert_eq!(state.cursor_pos(), 6);
    assert_eq!(state.char_count(), 6);
}

#[test]
fn replace_selection_empty_inserts_at_cursor() {
    let mut state = FormControlState::default();
    state.set_value("ab".to_string());
    state.set_cursor(1);
    state.set_selection(1, 1);
    state.replace_selection("X");
    assert_eq!(state.value(), "aXb");
}

#[test]
fn delete_backward_marks_dirty() {
    let mut state = FormControlState::default();
    state.set_value_initial("x".to_string());
    assert!(!state.is_dirty());
    state.set_cursor(1);
    state.delete_backward();
    assert!(state.is_dirty());
}

#[test]
fn delete_forward_marks_dirty() {
    let mut state = FormControlState::default();
    state.set_value_initial("x".to_string());
    assert!(!state.is_dirty());
    state.set_cursor(0);
    state.delete_forward();
    assert!(state.is_dirty());
}

// -- Slice 0b: shared positive-with-fallback reflection helper -----------

#[test]
fn parse_positive_with_fallback_takes_positive_else_default() {
    // §2.6.1 "limited to only positive numbers with fallback": absent /
    // non-numeric / `0` / negative all fall back to the default; a valid
    // `> 0` integer is taken as-is.
    assert_eq!(parse_positive_with_fallback(None, 2), 2);
    assert_eq!(parse_positive_with_fallback(Some("0"), 2), 2);
    assert_eq!(parse_positive_with_fallback(Some("-5"), 2), 2);
    assert_eq!(parse_positive_with_fallback(Some("abc"), 2), 2);
    assert_eq!(parse_positive_with_fallback(Some("10"), 2), 10);
    assert_eq!(parse_positive_with_fallback(Some("100000"), 20), 100_000);

    // HTML "rules for parsing non-negative integers" (§2.3.4.2): skip leading
    // ASCII whitespace, consume an optional leading `+`, take the leading
    // ASCII-digit run, and IGNORE trailing junk. Unlike `str::parse` (whole
    // string), these all parse the leading digit run:
    assert_eq!(parse_positive_with_fallback(Some("3.5"), 2), 3); // leading run "3"
    assert_eq!(parse_positive_with_fallback(Some(" 5"), 2), 5); // leading space skipped
    assert_eq!(parse_positive_with_fallback(Some("\t5"), 2), 5); // leading tab skipped
    assert_eq!(parse_positive_with_fallback(Some("5px"), 2), 5); // trailing junk ignored
    assert_eq!(parse_positive_with_fallback(Some("5 "), 2), 5); // trailing space ignored
    assert_eq!(parse_positive_with_fallback(Some("+5"), 2), 5); // leading `+` consumed
    assert_eq!(parse_positive_with_fallback(Some("  12  "), 2), 12); // ws both sides
    assert_eq!(parse_positive_with_fallback(Some(" "), 2), 2); // whitespace only → default

    // §2.6.1 getter steps 5-6: the reflection is bounded by maximum
    // 2147483647 (inclusive). The max itself is accepted; anything above it
    // (still ≤ u32::MAX, or overflowing u32) falls back to the default.
    assert_eq!(
        parse_positive_with_fallback(Some("2147483647"), 2),
        2_147_483_647
    ); // max, accepted
    assert_eq!(parse_positive_with_fallback(Some("2147483648"), 2), 2); // max + 1 → default
    assert_eq!(parse_positive_with_fallback(Some("3000000000"), 2), 2); // > max, ≤ u32::MAX → default
    assert_eq!(parse_positive_with_fallback(Some("4294967295"), 2), 2); // u32::MAX → default
}

#[test]
fn textarea_rows_cols_zero_fall_back_to_default() {
    // Latent init bug fixed by the shared helper: the previous plain `u32`
    // parse made `rows="0"` → 0; §2.6.1 requires the fallback (default 2 rows
    // / 20 cols).  Ties `from_textarea_element` to the same reflection the
    // reconciler arm uses.
    let mut attrs = Attributes::default();
    attrs.set("rows", "0");
    attrs.set("cols", "0");
    let state = FormControlState::from_element("textarea", &attrs).unwrap();
    assert_eq!(state.rows, 2);
    assert_eq!(state.cols, 20);
}
