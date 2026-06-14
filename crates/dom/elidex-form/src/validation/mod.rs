//! Form validation (HTML §4.10.15).

use std::sync::OnceLock;

use elidex_ecs::{EcsDom, Entity};

use crate::{FormControlKind, FormControlState};

/// HTML §4.10.20.3 "candidate for constraint validation" predicate
/// — a form control is considered a candidate when it is
/// submittable, not disabled, not `<input type=hidden>`, not
/// readonly (when `readonly` applies to the control kind), and not
/// a descendant of a disabled `<fieldset>`.  Used by ValidityState
/// accessors and `checkValidity()` to bypass the validation
/// algorithm for barred controls (whose stored bits stay at their
/// initialised all-false values per the spec).
#[must_use]
pub fn is_constraint_validation_candidate(
    state: &FormControlState,
    entity: Entity,
    dom: &EcsDom,
) -> bool {
    if !state.kind.is_submittable() {
        return false;
    }
    if state.disabled {
        return false;
    }
    if matches!(state.kind, FormControlKind::Hidden) {
        return false;
    }
    // Readonly bars constraint validation only for kinds where the
    // attribute applies (text-editable controls + number/date/
    // datetime-local) — checkbox/radio/range/file/etc still validate
    // even with `readonly` set, because the attribute has no effect
    // on them.
    if state.readonly && state.kind.readonly_applies() {
        return false;
    }
    !crate::is_fieldset_disabled(entity, dom)
}

/// WHATWG HTML §4.10.5.1.6 email validation regex, compiled once.
fn email_regex() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(
            r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$",
        )
        .expect("email regex is valid")
    })
}

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
        FormControlKind::Date
        | FormControlKind::DatetimeLocal
        | FormControlKind::Time
        | FormControlKind::Week
        | FormControlKind::Month => {
            // `required` applies to the date/time value-mode states, so an
            // empty value with `required` set suffers from being missing
            // (HTML §4.10.5.3.4).  The remaining date/time constraint
            // validation (rangeUnderflow/Overflow/stepMismatch/badInput) is
            // the separate `#11-input-date-validity` slot.
            check_required(&mut validity, state);
        }
        FormControlKind::SubmitButton
        | FormControlKind::ResetButton
        | FormControlKind::Button
        // Color always has a value (default #000000), so it can never be
        // value-missing; File's required check is over selected files, not
        // the value string (deferred); the rest have no validity.
        | FormControlKind::Color
        | FormControlKind::File
        | FormControlKind::Hidden
        | FormControlKind::Output
        | FormControlKind::Meter
        | FormControlKind::Progress => {}
    }

    // Custom validity, per HTML §4.10.20.2: when
    // `setCustomValidity()` was called with a non-empty string, the
    // `customError` bit is set and the message becomes the
    // `validationMessage`.  `customError` does NOT replace the other
    // anchor bits — `valid` is the OR over all 10 flags, so any
    // anchor (`valueMissing`, `tooLong`, …) keeps the control invalid
    // even if the custom message is cleared.
    if let Some(msg) = state.custom_validity_message.as_deref() {
        if !msg.is_empty() {
            validity.custom_error = true;
            validity.custom_error_message = msg.to_string();
        }
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

/// Check email `type_mismatch` per WHATWG HTML §4.10.5.1.6.
///
/// Uses the spec-defined regex which validates local-part and domain labels
/// (each label: starts/ends with alphanumeric, up to 63 chars, hyphens allowed internally).
///
/// Note: `user@localhost` is valid per WHATWG (dot in domain not required).
fn check_email_type(validity: &mut ValidityState, state: &FormControlState) {
    if state.value.is_empty() {
        return;
    }
    if !email_regex().is_match(&state.value) {
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
mod tests;
