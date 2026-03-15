//! Form validation (HTML §4.10.15).

use crate::{FormControlKind, FormControlState};

/// Validity state for a form control (HTML §4.10.14.2).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // HTML spec defines validity as individual boolean constraints.
pub struct ValidityState {
    /// The value is missing but the control is required.
    pub value_missing: bool,
    /// The value doesn't match the `type` (e.g., email/url format).
    pub type_mismatch: bool,
    /// The value doesn't match the `pattern` attribute.
    pub pattern_mismatch: bool,
    /// The value is shorter than `minlength`.
    pub too_short: bool,
    /// The value is longer than `maxlength`.
    pub too_long: bool,
    /// The value is less than `min`.
    pub range_underflow: bool,
    /// The value is greater than `max`.
    pub range_overflow: bool,
    /// The value does not match the `step` constraint.
    pub step_mismatch: bool,
    /// The value is not in a format the user agent can parse (e.g., bad number).
    pub bad_input: bool,
    /// A custom validity message has been set via `setCustomValidity()`.
    pub custom_error: bool,
    /// Custom error message (set via `setCustomValidity`).
    pub custom_error_message: String,
}

impl ValidityState {
    /// Returns `true` if the control is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.value_missing
            && !self.type_mismatch
            && !self.pattern_mismatch
            && !self.too_short
            && !self.too_long
            && !self.range_underflow
            && !self.range_overflow
            && !self.step_mismatch
            && !self.bad_input
            && !self.custom_error
    }
}

/// Validate a form control and return its validity state.
#[must_use]
pub fn validate_control(state: &FormControlState) -> ValidityState {
    let mut validity = ValidityState::default();

    // Buttons don't have validation constraints.
    match state.kind {
        FormControlKind::TextInput
        | FormControlKind::Password
        | FormControlKind::TextArea
        | FormControlKind::Tel
        | FormControlKind::Search => {
            check_text_constraints(&mut validity, state);
        }
        FormControlKind::Select => {
            check_required_select(&mut validity, state);
        }
        FormControlKind::Email => {
            check_text_constraints(&mut validity, state);
            check_email_type(&mut validity, state);
        }
        FormControlKind::Url => {
            check_text_constraints(&mut validity, state);
            check_url_type(&mut validity, state);
        }
        FormControlKind::Number => {
            check_required(&mut validity, state);
            check_number_bad_input(&mut validity, state);
            check_number_range(&mut validity, state);
            check_step(&mut validity, state);
        }
        FormControlKind::Checkbox | FormControlKind::Radio => {
            check_required_checked(&mut validity, state);
        }
        FormControlKind::Range => {
            // Range always has a value (defaults to midpoint), so no required check.
            check_number_range(&mut validity, state);
            check_step(&mut validity, state);
        }
        FormControlKind::SubmitButton
        | FormControlKind::ResetButton
        | FormControlKind::Button
        | FormControlKind::Color
        | FormControlKind::Date
        | FormControlKind::DatetimeLocal
        | FormControlKind::File
        | FormControlKind::Hidden
        | FormControlKind::Output
        | FormControlKind::Meter
        | FormControlKind::Progress => {}
    }

    validity
}

/// Common validation for text-like controls: required + length + pattern.
fn check_text_constraints(validity: &mut ValidityState, state: &FormControlState) {
    check_required(validity, state);
    check_length(validity, state);
    // WHATWG §4.10.7.3.1: textarea does not support the pattern attribute.
    if state.kind != FormControlKind::TextArea {
        check_pattern(validity, state);
    }
}

fn check_required(validity: &mut ValidityState, state: &FormControlState) {
    if state.required && state.value.is_empty() {
        tracing::debug!(kind = ?state.kind, name = %state.name, "validation: value_missing (required)");
        validity.value_missing = true;
    }
}

fn check_length(validity: &mut ValidityState, state: &FormControlState) {
    // Use cached char_count (O(1) instead of O(n)).
    let char_count = state.char_count;
    if let Some(min) = state.minlength {
        // HTML spec §4.10.15.2: minlength only applies when the value has been
        // edited by the user (dirty_value flag) and is non-empty.
        if state.dirty_value && !state.value.is_empty() && char_count < min {
            tracing::debug!(kind = ?state.kind, name = %state.name, minlength = min, char_count, "validation: too_short");
            validity.too_short = true;
        }
    }
    if let Some(max) = state.maxlength {
        if char_count > max {
            validity.too_long = true;
        }
    }
}

fn check_pattern(validity: &mut ValidityState, state: &FormControlState) {
    // HTML spec §4.10.5.3.8: pattern is a JavaScript regex anchored to match the entire value.
    // We anchor with ^(?:...)$ to match the entire string, per spec.
    // Note: HTML spec says patterns should use JS regex semantics; Rust `regex` differs
    // in some areas (e.g., `\b` is Unicode-aware, no backreferences). This is acceptable
    // for Phase 4; full JS regex integration deferred to Phase 5.
    // Per HTML spec: if the pattern is not a valid regex, it is ignored (no mismatch).
    if state.value.is_empty() {
        return;
    }

    match &state.cached_pattern_regex {
        // No pattern attribute, or pattern set but regex compilation failed — ignore per HTML spec.
        None | Some(None) => {}
        // Valid cached regex.
        Some(Some(re)) => {
            if !re.is_match(&state.value) {
                tracing::debug!(kind = ?state.kind, name = %state.name, pattern = ?state.pattern, value_len = state.value.len(), "validation: pattern_mismatch");
                validity.pattern_mismatch = true;
            }
        }
    }
}

fn check_number_bad_input(validity: &mut ValidityState, state: &FormControlState) {
    // HTML spec §4.10.5.1.12: if the value is non-empty and cannot be parsed
    // as a valid finite floating-point number, it's a bad input.
    // Rust's f64::parse accepts "inf"/"-inf"/"NaN" which are not valid HTML numbers.
    if !state.value.is_empty() {
        match state.value.parse::<f64>() {
            Ok(v) if !v.is_finite() => validity.bad_input = true,
            Err(_) => validity.bad_input = true,
            _ => {}
        }
    }
}

/// Check email `type_mismatch` (simplified per HTML §4.10.5.1.6).
///
/// Full RFC 5322 validation deferred to Phase 5. Current check:
/// exactly one `@`, non-empty local and domain parts.
/// Note: `user@localhost` is valid per WHATWG (dot in domain not required).
/// L2: WHATWG email regex places no restriction on the local-part characters,
/// which this implementation already handles by only checking for `@` split.
fn check_email_type(validity: &mut ValidityState, state: &FormControlState) {
    if state.value.is_empty() {
        return;
    }
    let parts: Vec<&str> = state.value.splitn(3, '@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        tracing::debug!(kind = ?state.kind, name = %state.name, value_len = state.value.len(), "validation: email type_mismatch");
        validity.type_mismatch = true;
    }
}

/// Check URL `type_mismatch` (WHATWG §4.10.5.1.7).
///
/// Per WHATWG: `<input type="url">` accepts any valid absolute URL.
/// Uses `url::Url::parse` which implements the WHATWG URL Standard.
/// Maximum URL length accepted for `<input type="url">` validation.
///
/// Prevents excessive memory usage in `url::Url::parse()` for extremely long inputs.
const MAX_URL_INPUT_LENGTH: usize = 2048;

fn check_url_type(validity: &mut ValidityState, state: &FormControlState) {
    if state.value.is_empty() {
        return;
    }
    if state.value.len() > MAX_URL_INPUT_LENGTH {
        tracing::debug!(kind = ?state.kind, name = %state.name, value_len = state.value.len(), "validation: url type_mismatch (too long)");
        validity.type_mismatch = true;
        return;
    }
    if url::Url::parse(&state.value).is_err() {
        tracing::debug!(kind = ?state.kind, name = %state.name, value_len = state.value.len(), "validation: url type_mismatch");
        validity.type_mismatch = true;
    }
}

/// Check number min/max range constraints.
fn check_number_range(validity: &mut ValidityState, state: &FormControlState) {
    if state.value.is_empty() {
        return;
    }
    let Ok(val) = state.value.parse::<f64>() else {
        return; // bad_input handled separately.
    };
    if let Some(ref min_str) = state.min {
        if let Ok(min_val) = min_str.parse::<f64>() {
            if val < min_val {
                validity.range_underflow = true;
            }
        }
    }
    if let Some(ref max_str) = state.max {
        if let Ok(max_val) = max_str.parse::<f64>() {
            if val > max_val {
                validity.range_overflow = true;
            }
        }
    }
}

/// Check required for `<select>`: considers placeholder option rule.
///
/// HTML §4.10.5.3.11: A select is `value_missing` if required and the value
/// is empty, or the first option is a placeholder (value="") and is selected.
fn check_required_select(validity: &mut ValidityState, state: &FormControlState) {
    if !state.required {
        return;
    }
    if state.value.is_empty() {
        validity.value_missing = true;
        return;
    }
    // Placeholder option rule: if the first option has value="" and is selected.
    if state.selected_index == 0 {
        if let Some(first) = state.options.first() {
            if first.value.is_empty() {
                validity.value_missing = true;
            }
        }
    }
}

/// Check step constraint (HTML §4.10.5.1.12).
///
/// A value `v` satisfies the step constraint when `(v - step_base) % step == 0`.
/// `step_base` is `min` if set, otherwise 0. `step="any"` disables the constraint.
fn check_step(validity: &mut ValidityState, state: &FormControlState) {
    if state.value.is_empty() {
        return;
    }
    let Some(ref step_str) = state.step else {
        return; // No step attribute — use default step (1 for number/range), always valid for integers.
    };
    // "any" disables the constraint.
    if step_str.eq_ignore_ascii_case("any") {
        return;
    }
    let Ok(step_val) = step_str.parse::<f64>() else {
        return; // Invalid step attribute — ignored per spec.
    };
    if !step_val.is_finite() || step_val <= 0.0 {
        return; // Non-positive or non-finite step — ignored per spec.
    }
    let Ok(val) = state.value.parse::<f64>() else {
        return; // bad_input handled separately.
    };
    if !val.is_finite() {
        return;
    }
    // step_base is min if set, otherwise 0.
    let step_base = state
        .min
        .as_deref()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .unwrap_or(0.0);

    let remainder = (val - step_base) % step_val;
    // Use epsilon-relative tolerance for floating-point comparison.
    let epsilon = step_val * 1e-10;
    if remainder.abs() > epsilon && (step_val - remainder.abs()).abs() > epsilon {
        validity.step_mismatch = true;
    }
}

/// Check required for checkbox/radio: per-control check.
///
/// For radio buttons, this checks the individual control. For group-level
/// validation (any radio in the group is checked), use
/// [`is_radio_group_satisfied`](crate::radio::is_radio_group_satisfied).
fn check_required_checked(validity: &mut ValidityState, state: &FormControlState) {
    if state.required && !state.checked {
        validity.value_missing = true;
    }
}

#[cfg(test)]
mod tests {
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
}
