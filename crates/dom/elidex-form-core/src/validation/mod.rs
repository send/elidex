//! Form validation (HTML §4.10.15).

use std::sync::OnceLock;

use elidex_ecs::{EcsDom, Entity};

use crate::input::{
    allowed_value_step, convert_value_to_number, is_step_aligned, maximum, minimum, step_base,
};
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
        // The numeric Number state and the five date/time value-mode states
        // (date, datetime-local, time, week, month) share ONE
        // constraint-validation algorithm — they differ only in the per-type
        // "convert a string to a number" hidden inside the canonical helpers,
        // so they converge onto a single arm (One-issue-one-way) rather than
        // two arms kept in sync.  Each check reads the value through the
        // canonical `convert_value_to_number` / `minimum` / `maximum` /
        // `allowed_value_step` / `step_base` / `is_step_aligned` helpers —
        // the same grid `apply_step` (§4.10.5.4) snaps to, so a value
        // `stepUp` produces can never report `stepMismatch` (one tolerance,
        // one grid).  Bits: `valueMissing` §4.10.5.3.4, `badInput`
        // §4.10.21.1, `rangeUnderflow`/`rangeOverflow` §4.10.5.3.7,
        // `stepMismatch` §4.10.5.3.8.
        FormControlKind::Number
        | FormControlKind::Date
        | FormControlKind::DatetimeLocal
        | FormControlKind::Time
        | FormControlKind::Week
        | FormControlKind::Month => {
            check_required(&mut validity, state);
            check_bad_input(&mut validity, state);
            check_range(&mut validity, state);
            check_step(&mut validity, state);
        }
        FormControlKind::Checkbox | FormControlKind::Radio => {
            check_required_checked(&mut validity, state);
        }
        FormControlKind::Range => {
            // Range has no `required` (it always has a value, defaulting to the
            // midpoint) and no `badInput` (its slider UI cannot describe a
            // non-float — HTML §4.10.5.1.13).  But `rangeUnderflow` /
            // `rangeOverflow` / `stepMismatch` ARE evaluated on the stored
            // value: the spec's structural conformance for range comes from the
            // UA *correcting* the value (clamp to [min,max], snap to step) via
            // value sanitization, now wired at the write layer
            // ([`crate::input::sanitize_value`]).  Validity is still computed
            // *honestly on the actual stored value* rather than assuming the
            // clamp — a stored out-of-range value no longer occurs through the
            // sanitized write paths, but assuming it can't (reporting `valid`
            // for a hypothetical stored `value=150`) would be a read of
            // unmodelled lifecycle state, and inconsistent with how the numeric
            // and date/time states surface a non-conforming value.
            check_range(&mut validity, state);
            check_step(&mut validity, state);
        }
        // Kinds with no value-based constraint validation (only a central
        // `customError` can apply): Color always has a value (default
        // #000000) → never value-missing; File's required check is over
        // selected files, not the value string (deferred); the buttons /
        // Output / Meter / Progress / Hidden have no validity.
        FormControlKind::SubmitButton
        | FormControlKind::ResetButton
        | FormControlKind::Button
        | FormControlKind::Color
        | FormControlKind::File
        | FormControlKind::Hidden
        | FormControlKind::Output
        | FormControlKind::Meter
        | FormControlKind::Progress => {}
    }

    // Custom validity, per HTML §4.10.21.3 (the constraint validation
    // API): when
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

/// Suffering from bad input (HTML §4.10.21.1): the value is non-empty
/// but the user agent cannot convert it to a number.  Kind-agnostic —
/// it reuses the canonical `convert_value_to_number`, which is the
/// strict valid-floating-point parse for the numeric states (rejecting
/// `"inf"`/`"NaN"`/trailing junk, which Rust's `f64::parse` would
/// otherwise accept) and the strict valid-`<type>`-string parse for the
/// date/time states (an out-of-grammar / over-precision stored value is
/// the error case).  An empty value is never bad input.
fn check_bad_input(validity: &mut ValidityState, state: &FormControlState) {
    if !state.value.is_empty() && convert_value_to_number(state).is_none() {
        tracing::debug!(kind = ?state.kind, name = %state.name, "validation: bad_input");
        validity.bad_input = true;
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

/// Check the `min`/`max` range constraints (HTML §4.10.5.3.7),
/// kind-agnostic across number/date/time.  The value is read through
/// `convert_value_to_number` (so each date/time type maps into its own
/// number space) and compared against the canonical `minimum` /
/// `maximum`.  An empty or unparseable value yields no range verdict
/// (the error case is handled by `check_bad_input`).
///
/// A type with a **periodic domain** (`time`) where `max < min` has a
/// *reversed range*: a value strictly between `max` and `min` is
/// *simultaneously* suffering an underflow and an overflow, while values
/// outside that band (the wrap-around interval) are in range.  For every
/// other case the determination is the plain `val < min` / `val > max`
/// (which, for a non-periodic type configured with `max < min`, already
/// flags every value as at least one of the two, per the spec note).
fn check_range(validity: &mut ValidityState, state: &FormControlState) {
    let Some(val) = convert_value_to_number(state) else {
        return;
    };
    let min = minimum(state);
    let max = maximum(state);

    if crate::datetime::is_periodic_domain(state.kind) {
        if let (Some(lo), Some(hi)) = (min, max) {
            if hi < lo {
                // Reversed range: only the (max, min) band is invalid.
                if val > hi && val < lo {
                    validity.range_underflow = true;
                    validity.range_overflow = true;
                }
                return;
            }
        }
    }

    if let Some(lo) = min {
        if val < lo {
            validity.range_underflow = true;
        }
    }
    if let Some(hi) = max {
        if val > hi {
            validity.range_overflow = true;
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

/// Check the `step` constraint (HTML §4.10.5.3.8), kind-agnostic across
/// number/range/date/time.
///
/// The value suffers from a step mismatch when, having an allowed value
/// step, it is not an integral number of steps from the step base.  This
/// reuses the canonical `allowed_value_step` (which folds in `step="any"`
/// → no allowed value step → never a mismatch, the per-type step scale,
/// and the default step), `step_base` (min → `value` attr → type default
/// → 0), and the cancellation-aware `is_step_aligned` tolerance — the
/// SAME grid `apply_step` (§4.10.5.4) snaps to, so a value `stepUp`
/// produces can never report `stepMismatch`.
fn check_step(validity: &mut ValidityState, state: &FormControlState) {
    // No allowed value step (`step="any"`) → never a step mismatch.
    let Some(step) = allowed_value_step(state) else {
        return;
    };
    // Empty / unparseable value → no step verdict (bad input handled by
    // `check_bad_input`).
    let Some(val) = convert_value_to_number(state) else {
        return;
    };
    if !is_step_aligned(val, step_base(state), step) {
        tracing::debug!(kind = ?state.kind, name = %state.name, "validation: step_mismatch");
        validity.step_mismatch = true;
    }
}

/// Check required for checkbox/radio: per-control check.
///
/// For radio buttons, this checks the individual control. For group-level
/// validation (any radio in the group is checked), use
/// `is_radio_group_satisfied` (in `elidex-form`).
fn check_required_checked(validity: &mut ValidityState, state: &FormControlState) {
    if state.required && !state.checked {
        validity.value_missing = true;
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod datetime_validation_tests;
