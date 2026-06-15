use super::*;
use elidex_ecs::Attributes;

fn text_input(value: &str, required: bool) -> FormControlState {
    let mut attrs = Attributes::default();
    if required {
        attrs.set("required", "");
    }
    if !value.is_empty() {
        attrs.set("value", value);
    }
    FormControlState::from_element("input", &attrs).unwrap()
}

#[test]
fn valid_text_input() {
    let state = text_input("hello", false);
    let v = validate_control(&state);
    assert!(v.is_valid());
}

#[test]
fn required_empty_is_invalid() {
    let state = text_input("", true);
    let v = validate_control(&state);
    assert!(!v.is_valid());
    assert!(v.value_missing);
}

#[test]
fn required_with_value_is_valid() {
    let state = text_input("hello", true);
    let v = validate_control(&state);
    assert!(v.is_valid());
}

#[test]
fn required_checkbox_unchecked() {
    let mut attrs = Attributes::default();
    attrs.set("type", "checkbox");
    attrs.set("required", "");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    let v = validate_control(&state);
    assert!(v.value_missing);
}

#[test]
fn required_checkbox_checked() {
    let mut attrs = Attributes::default();
    attrs.set("type", "checkbox");
    attrs.set("required", "");
    attrs.set("checked", "");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    let v = validate_control(&state);
    assert!(v.is_valid());
}

#[test]
fn button_always_valid() {
    let attrs = Attributes::default();
    let state = FormControlState::from_element("button", &attrs).unwrap();
    let v = validate_control(&state);
    assert!(v.is_valid());
}

#[test]
fn password_required_empty() {
    let mut attrs = Attributes::default();
    attrs.set("type", "password");
    attrs.set("required", "");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    let v = validate_control(&state);
    assert!(v.value_missing);
}

#[test]
fn password_required_with_value() {
    let mut attrs = Attributes::default();
    attrs.set("type", "password");
    attrs.set("required", "");
    attrs.set("value", "secret");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    let v = validate_control(&state);
    assert!(v.is_valid());
}

#[test]
fn textarea_required_empty() {
    let mut attrs = Attributes::default();
    attrs.set("required", "");
    let state = FormControlState::from_element("textarea", &attrs).unwrap();
    let v = validate_control(&state);
    assert!(v.value_missing);
}

#[test]
fn non_required_empty_is_valid() {
    let state = text_input("", false);
    let v = validate_control(&state);
    assert!(v.is_valid());
}

#[test]
fn required_radio_unchecked() {
    let mut attrs = Attributes::default();
    attrs.set("type", "radio");
    attrs.set("required", "");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    let v = validate_control(&state);
    assert!(v.value_missing);
}

#[test]
fn required_select_empty() {
    let mut attrs = Attributes::default();
    attrs.set("required", "");
    let state = FormControlState::from_element("select", &attrs).unwrap();
    let v = validate_control(&state);
    assert!(v.value_missing);
}

#[test]
fn required_date_family_empty_is_value_missing() {
    // Regression (Codex R1): date/month/week/time/datetime-local with
    // `required` and an empty value must report valueMissing
    // (HTML §4.10.5.3.4).  month/week/time previously fell back to the
    // TextInput required check; modelling them as distinct kinds must not
    // drop it.
    for ty in ["date", "datetime-local", "time", "week", "month"] {
        let mut attrs = Attributes::default();
        attrs.set("type", ty);
        attrs.set("required", "");
        let state = FormControlState::from_element("input", &attrs).unwrap();
        let v = validate_control(&state);
        assert!(
            v.value_missing,
            "type={ty} empty+required should be value_missing"
        );
    }
}

#[test]
fn required_date_family_with_value_is_valid() {
    for (ty, val) in [
        ("date", "2025-01-15"),
        ("datetime-local", "2025-01-15T12:30"),
        ("time", "12:30"),
        ("week", "2025-W25"),
        ("month", "2025-06"),
    ] {
        let mut attrs = Attributes::default();
        attrs.set("type", ty);
        attrs.set("required", "");
        attrs.set("value", val);
        let state = FormControlState::from_element("input", &attrs).unwrap();
        let v = validate_control(&state);
        assert!(
            !v.value_missing,
            "type={ty} value={val} should not be value_missing"
        );
    }
}

#[test]
fn validity_state_default_is_valid() {
    let v = ValidityState::default();
    assert!(v.is_valid());
}

#[test]
fn validity_state_too_short() {
    let v = ValidityState {
        too_short: true,
        ..Default::default()
    };
    assert!(!v.is_valid());
}

#[test]
fn minlength_violation() {
    let mut attrs = Attributes::default();
    attrs.set("minlength", "5");
    attrs.set("value", "abc");
    let mut state = FormControlState::from_element("input", &attrs).unwrap();
    state.dirty_value = true; // minlength only applies to user-edited values
    let v = validate_control(&state);
    assert!(v.too_short);
}

#[test]
fn minlength_empty_skipped() {
    // minlength is not checked on empty values (that's required's job).
    let mut attrs = Attributes::default();
    attrs.set("minlength", "5");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    let v = validate_control(&state);
    assert!(!v.too_short);
}

#[test]
fn maxlength_violation() {
    let mut attrs = Attributes::default();
    attrs.set("maxlength", "3");
    attrs.set("value", "hello");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    let v = validate_control(&state);
    assert!(v.too_long);
}

#[test]
fn maxlength_ok() {
    let mut attrs = Attributes::default();
    attrs.set("maxlength", "10");
    attrs.set("value", "hello");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    let v = validate_control(&state);
    assert!(v.is_valid());
}

#[test]
fn pattern_mismatch() {
    let mut attrs = Attributes::default();
    attrs.set("pattern", "expected");
    attrs.set("value", "wrong");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    let v = validate_control(&state);
    assert!(v.pattern_mismatch);
}

#[test]
fn pattern_match() {
    let mut attrs = Attributes::default();
    attrs.set("pattern", "hello");
    attrs.set("value", "hello");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    let v = validate_control(&state);
    assert!(!v.pattern_mismatch);
}

#[test]
fn pattern_empty_skipped() {
    // Pattern is not checked on empty values.
    let mut attrs = Attributes::default();
    attrs.set("pattern", "expected");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    let v = validate_control(&state);
    assert!(!v.pattern_mismatch);
}

#[test]
fn pattern_regex_digits() {
    let mut attrs = Attributes::default();
    attrs.set("pattern", "[0-9]{3}");
    attrs.set("value", "123");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert!(!validate_control(&state).pattern_mismatch);

    let mut attrs = Attributes::default();
    attrs.set("pattern", "[0-9]{3}");
    attrs.set("value", "12");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert!(validate_control(&state).pattern_mismatch);
}

#[test]
fn pattern_regex_anchored() {
    // Pattern is implicitly anchored — partial matches should not pass.
    let mut attrs = Attributes::default();
    attrs.set("pattern", "[a-z]+");
    attrs.set("value", "abc123");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert!(validate_control(&state).pattern_mismatch);

    let mut attrs = Attributes::default();
    attrs.set("pattern", "[a-z]+");
    attrs.set("value", "abc");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert!(!validate_control(&state).pattern_mismatch);
}

#[test]
fn pattern_invalid_regex_ignored() {
    // HTML spec: invalid pattern is ignored (no mismatch).
    let mut attrs = Attributes::default();
    attrs.set("pattern", "[invalid(");
    attrs.set("value", "anything");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert!(!validate_control(&state).pattern_mismatch);
}

#[test]
fn custom_error() {
    let v = ValidityState {
        custom_error: true,
        custom_error_message: "bad".to_string(),
        ..Default::default()
    };
    assert!(!v.is_valid());
}

#[test]
fn type_mismatch_makes_invalid() {
    let v = ValidityState {
        type_mismatch: true,
        ..Default::default()
    };
    assert!(!v.is_valid());
}

#[test]
fn minlength_requires_dirty_value() {
    // minlength should only trigger when the user has edited the value.
    let mut attrs = Attributes::default();
    attrs.set("minlength", "5");
    attrs.set("value", "abc");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    // Not dirty — minlength should not fire.
    let v = validate_control(&state);
    assert!(!v.too_short);

    // Mark as dirty — now minlength should fire.
    let mut dirty_state = state;
    dirty_state.dirty_value = true;
    let v = validate_control(&dirty_state);
    assert!(v.too_short);
}

#[test]
fn email_type_mismatch() {
    let mut state = FormControlState {
        kind: FormControlKind::Email,
        value: "notanemail".to_string(),
        char_count: 10,
        ..FormControlState::default()
    };
    let v = validate_control(&state);
    assert!(v.type_mismatch);

    state.value = "user@example.com".to_string();
    state.char_count = state.value.chars().count();
    let v = validate_control(&state);
    assert!(!v.type_mismatch);
}

#[test]
fn email_rejects_multiple_at() {
    let state = FormControlState {
        kind: FormControlKind::Email,
        value: "a@b@c".to_string(),
        char_count: 5,
        ..FormControlState::default()
    };
    assert!(validate_control(&state).type_mismatch);
}

#[test]
fn email_allows_dotless_domain() {
    // WHATWG HTML §4.10.5.1.6: user@localhost is valid (dot in domain not required).
    let state = FormControlState {
        kind: FormControlKind::Email,
        value: "user@localhost".to_string(),
        char_count: 14,
        ..FormControlState::default()
    };
    assert!(!validate_control(&state).type_mismatch);
}

#[test]
fn url_type_mismatch() {
    let mut state = FormControlState {
        kind: FormControlKind::Url,
        value: "notaurl".to_string(),
        char_count: 7,
        ..FormControlState::default()
    };
    let v = validate_control(&state);
    assert!(v.type_mismatch);

    state.value = "https://example.com".to_string();
    state.char_count = state.value.chars().count();
    let v = validate_control(&state);
    assert!(!v.type_mismatch);
}

#[test]
fn url_requires_valid_absolute_url() {
    // "http://" without host is still valid per WHATWG URL Standard.
    // But a bare string with no scheme is not.
    let state = FormControlState {
        kind: FormControlKind::Url,
        value: "not-a-url".to_string(),
        char_count: 9,
        ..FormControlState::default()
    };
    assert!(validate_control(&state).type_mismatch);
}

#[test]
fn url_accepts_mailto() {
    // WHATWG: any valid absolute URL is accepted (mailto:, data:, etc.).
    let state = FormControlState {
        kind: FormControlKind::Url,
        value: "mailto:foo@bar".to_string(),
        char_count: 14,
        ..FormControlState::default()
    };
    assert!(!validate_control(&state).type_mismatch);
}

#[test]
fn number_min_max() {
    let mut state = FormControlState {
        kind: FormControlKind::Number,
        value: "3".to_string(),
        min: Some("5".to_string()),
        max: Some("10".to_string()),
        ..FormControlState::default()
    };
    let v = validate_control(&state);
    assert!(v.range_underflow);
    assert!(!v.range_overflow);

    state.value = "15".to_string();
    let v = validate_control(&state);
    assert!(!v.range_underflow);
    assert!(v.range_overflow);

    state.value = "7".to_string();
    let v = validate_control(&state);
    assert!(!v.range_underflow);
    assert!(!v.range_overflow);
}

#[test]
fn number_infinity_is_bad_input() {
    // Rust's f64::parse accepts "inf"/"-inf"/"NaN" but HTML number inputs must not.
    for val in &["inf", "-inf", "Infinity", "NaN"] {
        let state = FormControlState {
            kind: FormControlKind::Number,
            value: val.to_string(),
            ..FormControlState::default()
        };
        let v = validate_control(&state);
        assert!(v.bad_input, "expected bad_input for {val}");
    }
}

#[test]
fn select_required_placeholder() {
    let mut state = FormControlState {
        kind: FormControlKind::Select,
        required: true,
        selected_index: 0,
        value: String::new(),
        options: vec![crate::SelectOption {
            text: "-- Select --".into(),
            value: String::new(),
            disabled: false,
            group: None,
            selected: true,
        }],
        ..FormControlState::default()
    };
    let v = validate_control(&state);
    assert!(
        v.value_missing,
        "placeholder option with empty value should be value_missing"
    );

    state.options[0].value = "real_value".to_string();
    state.value = "real_value".to_string();
    let v = validate_control(&state);
    assert!(!v.value_missing);
}

#[test]
fn textarea_ignores_pattern() {
    // WHATWG §4.10.7.3.1: textarea does not support the pattern attribute.
    let mut state = FormControlState {
        kind: FormControlKind::TextArea,
        value: "wrong".to_string(),
        char_count: 5,
        pattern: Some("expected".to_string()),
        cached_pattern_regex: Some(crate::compile_pattern_regex("expected")),
        ..FormControlState::default()
    };
    state.dirty_value = true;
    let v = validate_control(&state);
    assert!(!v.pattern_mismatch, "textarea should not check pattern");
}

#[test]
fn url_too_long_is_type_mismatch() {
    let long_url = format!("https://example.com/{}", "a".repeat(2030));
    assert!(long_url.len() > MAX_URL_INPUT_LENGTH);
    let state = FormControlState {
        kind: FormControlKind::Url,
        value: long_url,
        char_count: 2050,
        ..FormControlState::default()
    };
    let v = validate_control(&state);
    assert!(
        v.type_mismatch,
        "URL exceeding MAX_URL_INPUT_LENGTH should be type_mismatch"
    );
}

#[test]
fn step_mismatch_number() {
    // <input type="number" step="5" value="12"> — 12 is not a multiple of 5.
    let state = FormControlState {
        kind: FormControlKind::Number,
        value: "12".to_string(),
        step: Some("5".to_string()),
        ..FormControlState::default()
    };
    let v = validate_control(&state);
    assert!(v.step_mismatch);
}

#[test]
fn step_mismatch_number_valid() {
    // <input type="number" step="5" value="10"> — valid.
    let state = FormControlState {
        kind: FormControlKind::Number,
        value: "10".to_string(),
        step: Some("5".to_string()),
        ..FormControlState::default()
    };
    let v = validate_control(&state);
    assert!(!v.step_mismatch);
}

#[test]
fn step_any_disables_constraint() {
    let state = FormControlState {
        kind: FormControlKind::Number,
        value: "12".to_string(),
        step: Some("any".to_string()),
        ..FormControlState::default()
    };
    let v = validate_control(&state);
    assert!(!v.step_mismatch);
}

#[test]
fn step_with_min_base() {
    // <input type="number" min="3" step="5" value="8"> — (8-3)%5 = 0, valid.
    let state = FormControlState {
        kind: FormControlKind::Number,
        value: "8".to_string(),
        step: Some("5".to_string()),
        min: Some("3".to_string()),
        ..FormControlState::default()
    };
    let v = validate_control(&state);
    assert!(!v.step_mismatch);

    // value="9" — (9-3)%5 = 1, mismatch.
    let state2 = FormControlState {
        value: "9".to_string(),
        ..state.clone()
    };
    let v2 = validate_control(&state2);
    assert!(v2.step_mismatch);
}

#[test]
fn step_decimal() {
    // <input type="number" step="0.1" value="0.3"> — should be valid.
    let state = FormControlState {
        kind: FormControlKind::Number,
        value: "0.3".to_string(),
        step: Some("0.1".to_string()),
        ..FormControlState::default()
    };
    let v = validate_control(&state);
    assert!(!v.step_mismatch);
}

#[test]
fn number_bad_input() {
    let mut state = FormControlState {
        kind: FormControlKind::Number,
        value: "abc".to_string(),
        ..FormControlState::default()
    };
    let v = validate_control(&state);
    assert!(v.bad_input);

    // Valid number should not have bad_input.
    state.value = "42.5".to_string();
    let v = validate_control(&state);
    assert!(!v.bad_input);

    // Empty is not bad_input (checked by required).
    state.value.clear();
    let v = validate_control(&state);
    assert!(!v.bad_input);
}

fn email_state(value: &str) -> FormControlState {
    FormControlState {
        kind: FormControlKind::Email,
        value: value.to_string(),
        char_count: value.chars().count(),
        ..FormControlState::default()
    }
}

#[test]
fn email_whatwg_valid_simple() {
    assert!(!validate_control(&email_state("user@example.com")).type_mismatch);
}

#[test]
fn email_whatwg_valid_plus_tag() {
    assert!(!validate_control(&email_state("user+tag@example.com")).type_mismatch);
}

#[test]
fn email_whatwg_invalid_leading_hyphen_domain() {
    // Domain label must start with [a-zA-Z0-9], not hyphen.
    assert!(validate_control(&email_state("user@-example.com")).type_mismatch);
}

#[test]
fn email_whatwg_invalid_empty_label() {
    // "example..com" contains an empty label between the dots.
    assert!(validate_control(&email_state("user@example..com")).type_mismatch);
}

#[test]
fn email_whatwg_invalid_no_local_part() {
    assert!(validate_control(&email_state("@example.com")).type_mismatch);
}

#[test]
fn email_whatwg_invalid_no_domain() {
    assert!(validate_control(&email_state("user@")).type_mismatch);
}

#[test]
fn pattern_unicode_false_digit_shorthand() {
    // Rust regex default for `\d` is ASCII `[0-9]` (matches the JS `u` flag
    // semantics that HTML §4.10.5.3.8 prescribes for the `pattern` attribute).
    let mut attrs = Attributes::default();
    attrs.set("pattern", r"\d{3}");
    attrs.set("value", "123");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert!(!validate_control(&state).pattern_mismatch);

    let mut attrs2 = Attributes::default();
    attrs2.set("pattern", r"\d{3}");
    attrs2.set("value", "12a");
    let state2 = FormControlState::from_element("input", &attrs2).unwrap();
    assert!(validate_control(&state2).pattern_mismatch);
}

#[test]
fn candidate_text_input_default() {
    let mut dom = elidex_ecs::EcsDom::new();
    let entity = dom.create_element("input", Attributes::default());
    let state = text_input("hello", false);
    assert!(is_constraint_validation_candidate(&state, entity, &dom));
}

#[test]
fn candidate_text_input_disabled_barred() {
    let mut dom = elidex_ecs::EcsDom::new();
    let entity = dom.create_element("input", Attributes::default());
    let mut state = text_input("hello", false);
    state.disabled = true;
    assert!(!is_constraint_validation_candidate(&state, entity, &dom));
}

#[test]
fn candidate_hidden_input_barred() {
    let mut dom = elidex_ecs::EcsDom::new();
    let entity = dom.create_element("input", Attributes::default());
    let mut attrs = Attributes::default();
    attrs.set("type", "hidden");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert!(!is_constraint_validation_candidate(&state, entity, &dom));
}

#[test]
fn candidate_readonly_text_input_barred() {
    // HTML §4.10.20.3: readonly bars constraint validation when
    // the attribute applies to the kind (text/textarea/etc).
    let mut dom = elidex_ecs::EcsDom::new();
    let entity = dom.create_element("input", Attributes::default());
    let mut state = text_input("hello", true);
    state.readonly = true;
    assert!(!is_constraint_validation_candidate(&state, entity, &dom));
}

#[test]
fn candidate_readonly_textarea_barred() {
    let mut dom = elidex_ecs::EcsDom::new();
    let entity = dom.create_element("textarea", Attributes::default());
    let mut attrs = Attributes::default();
    attrs.set("required", "");
    let mut state = FormControlState::from_element("textarea", &attrs).unwrap();
    state.readonly = true;
    assert!(!is_constraint_validation_candidate(&state, entity, &dom));
}

#[test]
fn candidate_readonly_checkbox_still_candidate() {
    // `readonly` does not apply to checkbox per HTML §4.10.5.1.4,
    // so setting it must not bar the control from validation.
    let mut dom = elidex_ecs::EcsDom::new();
    let entity = dom.create_element("input", Attributes::default());
    let mut attrs = Attributes::default();
    attrs.set("type", "checkbox");
    attrs.set("required", "");
    let mut state = FormControlState::from_element("input", &attrs).unwrap();
    state.readonly = true;
    assert!(is_constraint_validation_candidate(&state, entity, &dom));
}

#[test]
fn candidate_readonly_range_still_candidate() {
    // Same: `readonly` doesn't apply to `<input type=range>`.
    let mut dom = elidex_ecs::EcsDom::new();
    let entity = dom.create_element("input", Attributes::default());
    let mut attrs = Attributes::default();
    attrs.set("type", "range");
    let mut state = FormControlState::from_element("input", &attrs).unwrap();
    state.readonly = true;
    assert!(is_constraint_validation_candidate(&state, entity, &dom));
}

#[test]
fn candidate_button_not_submittable() {
    // Button kinds are not submittable, so they're never candidates
    // regardless of readonly state.
    let mut dom = elidex_ecs::EcsDom::new();
    let entity = dom.create_element("button", Attributes::default());
    let attrs = Attributes::default();
    let state = FormControlState::from_element("button", &attrs).unwrap();
    assert!(!is_constraint_validation_candidate(&state, entity, &dom));
}

// ---------------------------------------------------------------------------
// Date/time constraint validation (`#11-input-date-validity`): the five
// date/time value-mode states converge onto the SAME kind-agnostic
// `check_bad_input` / `check_range` / `check_step` helpers as number/range,
// reading the value through the canonical `convert_value_to_number` /
// `minimum` / `maximum` / `allowed_value_step` / `step_base` /
// `is_step_aligned` (HTML §4.10.21 constraint validation).
// ---------------------------------------------------------------------------

/// Build a date/time `<input>` state directly (bypassing element parsing)
/// with the given value and optional `min`/`max`/`step` content attrs.
fn datetime_state(
    kind: FormControlKind,
    value: &str,
    min: Option<&str>,
    max: Option<&str>,
    step: Option<&str>,
) -> FormControlState {
    FormControlState {
        kind,
        value: value.to_string(),
        min: min.map(str::to_string),
        max: max.map(str::to_string),
        step: step.map(str::to_string),
        ..FormControlState::default()
    }
}

#[test]
fn date_bad_input_invalid_value() {
    // Out-of-grammar date value (month 13, day 40) → strict parse fails → bad_input.
    let state = datetime_state(FormControlKind::Date, "2025-13-40", None, None, None);
    assert!(validate_control(&state).bad_input);

    // A valid date value is not bad input.
    let state = datetime_state(FormControlKind::Date, "2025-06-15", None, None, None);
    assert!(!validate_control(&state).bad_input);

    // Empty is never bad input.
    let state = datetime_state(FormControlKind::Date, "", None, None, None);
    assert!(!validate_control(&state).bad_input);
}

#[test]
fn time_bad_input_over_precision() {
    // A "valid time string" caps the seconds fraction at three digits; a
    // four-digit fraction is the strict-parse error case → bad_input (#349 R1
    // value-path parity: the stored value is read as sanitization would leave
    // it, not silently truncated).
    let state = datetime_state(FormControlKind::Time, "12:30:45.1234", None, None, None);
    assert!(validate_control(&state).bad_input);

    // Three-digit fraction is a valid time string → not bad input.
    let state = datetime_state(FormControlKind::Time, "12:30:45.123", None, None, None);
    assert!(!validate_control(&state).bad_input);
}

#[test]
fn date_range_underflow_overflow() {
    // value < min → underflow.
    let state = datetime_state(
        FormControlKind::Date,
        "2025-01-01",
        Some("2025-06-01"),
        None,
        None,
    );
    let v = validate_control(&state);
    assert!(v.range_underflow);
    assert!(!v.range_overflow);

    // value > max → overflow.
    let state = datetime_state(
        FormControlKind::Date,
        "2025-12-31",
        None,
        Some("2025-06-01"),
        None,
    );
    let v = validate_control(&state);
    assert!(!v.range_underflow);
    assert!(v.range_overflow);

    // In range → neither.
    let state = datetime_state(
        FormControlKind::Date,
        "2025-06-15",
        Some("2025-01-01"),
        Some("2025-12-31"),
        None,
    );
    let v = validate_control(&state);
    assert!(!v.range_underflow);
    assert!(!v.range_overflow);
}

#[test]
fn datetime_local_month_week_time_range_each_type() {
    // datetime-local: each type maps into its own number space via the
    // canonical `convert_value_to_number`, so the same `check_range` works.
    let v = validate_control(&datetime_state(
        FormControlKind::DatetimeLocal,
        "2025-01-01T08:00",
        Some("2025-01-01T09:00"),
        None,
        None,
    ));
    assert!(v.range_underflow, "datetime-local before min → underflow");

    // month (month-count space).
    let v = validate_control(&datetime_state(
        FormControlKind::Month,
        "2025-01",
        None,
        Some("2024-12"),
        None,
    ));
    assert!(v.range_overflow, "month after max → overflow");

    // week (week-ms space).
    let v = validate_control(&datetime_state(
        FormControlKind::Week,
        "2025-W01",
        Some("2025-W10"),
        None,
        None,
    ));
    assert!(v.range_underflow, "week before min → underflow");

    // time (ms-since-midnight space).
    let v = validate_control(&datetime_state(
        FormControlKind::Time,
        "08:00",
        None,
        Some("07:00"),
        None,
    ));
    assert!(v.range_overflow, "time after max → overflow");
}

#[test]
fn time_step_mismatch_and_alignment() {
    // step=60 (seconds) → 60 000 ms grid anchored at 0; value 30 s off grid.
    let state = datetime_state(FormControlKind::Time, "12:00:30", None, None, Some("60"));
    assert!(validate_control(&state).step_mismatch);

    // On-grid (whole minute) → no mismatch.
    let state = datetime_state(FormControlKind::Time, "12:00:00", None, None, Some("60"));
    assert!(!validate_control(&state).step_mismatch);

    // step="any" → never a step mismatch.
    let state = datetime_state(FormControlKind::Time, "12:00:30", None, None, Some("any"));
    assert!(!validate_control(&state).step_mismatch);
}

#[test]
fn month_step_mismatch_with_min_base() {
    // month step counts months; min="2025-01" sets the step base, step=2 →
    // every other month is on grid.  "2025-02" is one month off → mismatch.
    let state = datetime_state(
        FormControlKind::Month,
        "2025-02",
        Some("2025-01"),
        None,
        Some("2"),
    );
    assert!(validate_control(&state).step_mismatch);

    // "2025-03" is two months from the base → on grid.
    let state = datetime_state(
        FormControlKind::Month,
        "2025-03",
        Some("2025-01"),
        None,
        Some("2"),
    );
    assert!(!validate_control(&state).step_mismatch);
}

#[test]
fn far_future_date_step_no_false_mismatch() {
    // §9-R3: large-magnitude ms values (far-future date) must not produce a
    // false step mismatch from the cancellation term in `align_tolerance`.
    // 2999-01-01 is an integer number of days from the epoch; default step =
    // 1 day → it lies exactly on the grid.
    let state = datetime_state(FormControlKind::Date, "2999-01-01", None, None, None);
    assert!(!validate_control(&state).step_mismatch);
}

#[test]
fn step_up_result_never_step_mismatch() {
    // The convergence guard (plan §7): a value produced by `apply_step`
    // (§4.10.5.4) is NEVER `stepMismatch`, because validation snaps to the
    // SAME grid with the SAME cancellation-aware tolerance.  Exercise a few
    // types/steps/start values, including off-grid starts (which `apply_step`
    // snaps onto the grid).
    let cases: &[(FormControlKind, &str, Option<&str>, Option<&str>)] = &[
        (FormControlKind::Number, "7", None, Some("5")),
        (
            FormControlKind::Number,
            "16.0005",
            Some("16"),
            Some("0.001"),
        ),
        // Range is excluded: it is structurally barred from `stepMismatch`
        // (the UA rounds it), so it does not run `check_step` (see
        // `range_is_always_conformant`).
        (FormControlKind::Date, "2025-01-02", None, Some("3")),
        (
            FormControlKind::Month,
            "2025-02",
            Some("2025-01"),
            Some("2"),
        ),
        (FormControlKind::Week, "2025-W02", None, Some("2")),
        (FormControlKind::Time, "12:00:30", None, Some("60")),
    ];
    for &(kind, value, min, step) in cases {
        let mut state = datetime_state(kind, value, min, None, step);
        // Step up once; the result must validate without a step mismatch.
        crate::apply_step(&mut state, 1.0, 1.0).expect("step applies to these kinds");
        let v = validate_control(&state);
        assert!(
            !v.step_mismatch,
            "{kind:?} value {value:?} step {step:?}: stepUp result {:?} reported stepMismatch",
            state.value
        );
    }
}

#[test]
fn number_step_cancellation_not_mismatch() {
    // §9-R1 regression: the converged `is_step_aligned` (cancellation-aware,
    // #344 R5) must keep `min=16 step=0.001 value=16.001` aligned where the
    // old crude `% step` tolerance could have flagged it.
    let state = FormControlState {
        kind: FormControlKind::Number,
        value: "16.001".to_string(),
        min: Some("16".to_string()),
        step: Some("0.001".to_string()),
        ..FormControlState::default()
    };
    assert!(!validate_control(&state).step_mismatch);
}

#[test]
fn range_is_always_conformant() {
    // HTML §4.10.5.1.13 mandates the UA correct a range underflow→min,
    // overflow→max, and step mismatch→nearest in-range step, and its slider
    // UI cannot produce bad input — so a range control is structurally always
    // conformant.  Validation must therefore NOT report range/step/bad-input
    // bits even when the (sanitization-deferred) stored value is out of the
    // default 0..=100 / off-step (a spec UA would have clamped/rounded it).
    for (value, min, max, step) in [
        ("150", None, None, None),                 // above default max=100
        ("-20", None, None, None),                 // below default min=0
        ("23", Some("0"), Some("10"), Some("10")), // off the step grid
        ("500", Some("0"), Some("100"), None),     // above explicit max
    ] {
        let state = FormControlState {
            kind: FormControlKind::Range,
            value: value.to_string(),
            min: min.map(str::to_string),
            max: max.map(str::to_string),
            step: step.map(str::to_string),
            ..FormControlState::default()
        };
        let v = validate_control(&state);
        assert!(
            v.is_valid(),
            "range value={value} min={min:?} max={max:?} step={step:?} must be conformant, got {v:?}"
        );
    }
}

#[test]
fn time_reversed_range() {
    // HTML §4.10.5.3.7: `time` has a periodic domain, so `min` later than
    // `max` denotes a *reversed range* (an overnight interval).  Only a value
    // strictly between max and min is invalid (simultaneously under+overflow);
    // values in the wrap-around band are in range.
    let reversed = |value: &str| {
        validate_control(&datetime_state(
            FormControlKind::Time,
            value,
            Some("21:00"),
            Some("06:00"),
            None,
        ))
    };

    // Midnight is inside the allowed overnight band (≥ 21:00 wrapping to
    // ≤ 06:00) → valid, NOT underflow.
    let v = reversed("00:00");
    assert!(
        !v.range_underflow && !v.range_overflow,
        "00:00 is within the reversed range 21:00..06:00"
    );
    // 23:00 (after min) and 03:00 (before max) are also in band → valid.
    assert!(!reversed("23:00").range_underflow && !reversed("23:00").range_overflow);
    assert!(!reversed("03:00").range_underflow && !reversed("03:00").range_overflow);

    // 12:00 is strictly between max (06:00) and min (21:00) → the forbidden
    // band → simultaneously underflow AND overflow.
    let v = reversed("12:00");
    assert!(
        v.range_underflow && v.range_overflow,
        "12:00 is outside the reversed range → both under and overflow"
    );

    // A normal (non-reversed) time range still uses the plain comparison.
    let v = validate_control(&datetime_state(
        FormControlKind::Time,
        "05:00",
        Some("09:00"),
        Some("17:00"),
        None,
    ));
    assert!(v.range_underflow && !v.range_overflow, "05:00 < min 09:00");
}
