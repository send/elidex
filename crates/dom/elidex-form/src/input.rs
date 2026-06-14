//! Keyboard input handling for text-based form controls.

use elidex_ecs::{Attributes, EcsDom, Entity, TagType};

use crate::util::{next_char_boundary, prev_char_boundary};
use crate::{FormControlKind, FormControlState};

/// Action returned from key input processing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyAction {
    /// The key was consumed (value modified or cursor moved).
    Consumed,
    /// Enter pressed on a text input — trigger implicit form submission.
    Submit,
    /// The key was not consumed.
    None,
}

/// Process a key press on a text form control.
///
/// Returns a `KeyAction` indicating what happened.
/// Handles `TextInput`, `Password`, and `TextArea` controls.
#[must_use]
pub fn form_control_key_input(state: &mut FormControlState, key: &str, code: &str) -> bool {
    form_control_key_input_action(state, key, code) != KeyAction::None
}

/// Process a key press with detailed action result.
#[must_use]
pub fn form_control_key_input_action(
    state: &mut FormControlState,
    key: &str,
    _code: &str,
) -> KeyAction {
    match state.kind {
        FormControlKind::TextInput
        | FormControlKind::Password
        | FormControlKind::TextArea
        | FormControlKind::Email
        | FormControlKind::Url
        | FormControlKind::Tel
        | FormControlKind::Search
        | FormControlKind::Number => {
            state.cursor_pos = state.safe_cursor_pos();

            // Clear selection on non-shift navigation keys.
            if state.selection_start != state.selection_end && !matches!(key, "Shift") {
                match key {
                    "Backspace" | "Delete" => {
                        // Delete selection.
                        let (start, end) = state.safe_selection_range();
                        state.value.drain(start..end);
                        state.cursor_pos = start;
                        state.selection_start = 0;
                        state.selection_end = 0;
                        state.dirty_value = true;
                        state.update_char_count();
                        return KeyAction::Consumed;
                    }
                    k if k.len() == 1 || (k.chars().count() == 1 && !k.starts_with("Arrow")) => {
                        // Replace selection with typed character.
                        let ch = k.chars().next().unwrap();
                        if !ch.is_control() {
                            let (start, end) = state.safe_selection_range();
                            state.value.drain(start..end);
                            state.cursor_pos = start;
                            state.value.insert(state.cursor_pos, ch);
                            state.cursor_pos += ch.len_utf8();
                            state.selection_start = 0;
                            state.selection_end = 0;
                            state.dirty_value = true;
                            state.update_char_count();
                            return KeyAction::Consumed;
                        }
                    }
                    _ => {}
                }
            }

            if state.readonly {
                return if handle_readonly_navigation(state, key) {
                    KeyAction::Consumed
                } else {
                    KeyAction::None
                };
            }
            let result = handle_text_key(state, key);
            if result == KeyAction::Consumed {
                state.dirty_value = true;
            }
            result
        }
        _ => KeyAction::None,
    }
}

/// Navigate cursor in a direction. Returns `KeyAction::Consumed` if moved.
fn navigate_cursor(state: &mut FormControlState, key: &str) -> KeyAction {
    match key {
        "ArrowLeft" => {
            if state.cursor_pos > 0 {
                state.cursor_pos = prev_char_boundary(&state.value, state.cursor_pos);
                KeyAction::Consumed
            } else {
                KeyAction::None
            }
        }
        "ArrowRight" => {
            if state.cursor_pos < state.value.len() {
                state.cursor_pos = next_char_boundary(&state.value, state.cursor_pos);
                KeyAction::Consumed
            } else {
                KeyAction::None
            }
        }
        "Home" => {
            if state.cursor_pos > 0 {
                state.cursor_pos = 0;
                KeyAction::Consumed
            } else {
                KeyAction::None
            }
        }
        "End" => {
            let end = state.value.len();
            if state.cursor_pos < end {
                state.cursor_pos = end;
                KeyAction::Consumed
            } else {
                KeyAction::None
            }
        }
        _ => KeyAction::None,
    }
}

/// Error returned by [`apply_step`].  Both variants map to a DOM
/// `InvalidStateError` per HTML §4.10.5.4 (`stepUp()`/`stepDown()`
/// steps 1 and 2); callers convert to the engine-bound exception type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepError {
    /// `state.kind` is not `Number` or `Range` — `stepUp()`/`stepDown()`
    /// do not apply (HTML §4.10.5.4 step 1).
    NotSupported,
    /// The element has no allowed value step (HTML §4.10.5.4 step 2),
    /// i.e. `step="any"` (ASCII case-insensitive).
    NoAllowedValueStep,
}

/// Multiplier (in `f64::EPSILON` units) for the "integral multiple of
/// the allowed value step" test (HTML §4.10.5.4 step 7).  The spec uses
/// exact real arithmetic; we approximate with `f64` and treat a value as
/// step-aligned when `(value − base) / step` is within a tolerance of an
/// integer.
///
/// The tolerance must bound the *actual* `f64` error in computing that
/// ratio, which is dominated by **catastrophic cancellation** in
/// `value − base` when `|base|` ≫ `|value − base|` (a realistic input:
/// `min=16 step=0.001 value=16.001` loses ~5500 ULP).  That error scales
/// with `(|value| + |base|) / |step|`, NOT with `|ratio|` — a tolerance
/// proportional to `|ratio|` alone (any fixed ULP multiple) wrongly
/// rejects such aligned values, making `stepUp()` a no-op.  See
/// [`is_step_aligned`].
const STEP_ALIGN_TOLERANCE_ULPS: f64 = 4.0 * f64::EPSILON;

/// Hard cap (in step units) on the alignment tolerance, strictly below
/// ½ a step, so a value ≈½ step off the grid always snaps regardless of
/// magnitude.
///
/// At astronomical magnitudes the cancellation-aware tolerance exceeds a
/// representable fractional offset (e.g. `step=1 value=2.8e14+0.125`),
/// so f64 cannot decide alignment as exactly as the spec's real
/// arithmetic — fully resolving that needs decimal arithmetic (à la
/// Blink's `Decimal`), tracked at defer slot
/// `#11-input-number-decimal-precision`.  The cap keeps the failure mode
/// bounded (never accept a ≈½-step-off value) rather than unbounded.
const STEP_ALIGN_MAX_TOLERANCE: f64 = 0.25;

/// HTML "rules for parsing floating-point number values"
/// (§2.3.4.3 "Floating-point numbers") — used as "convert a string to a
/// number" for the number/range input types.
///
/// Differs from Rust's `f64::from_str` in the ways that are
/// JS-observable through `stepUp()`/`stepDown()`:
///
/// * leading ASCII whitespace is skipped;
/// * a leading `+` is accepted (non-conforming, but parsed);
/// * trailing non-numeric content is ignored (`"12abc"` → `12`);
/// * `inf` / `nan` / `infinity` are **rejected** (Rust accepts them);
/// * a finite-overflow result (e.g. `"1e400"`) is rejected (the spec
///   returns an error for values that round to ±2¹⁰²⁴).
///
/// Returns `None` for an error (the spec's "return an error" outcome).
fn parse_floating_point(s: &str) -> Option<f64> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let start = i;
    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
        i += 1;
    }
    let mut saw_digit = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
        saw_digit = true;
    }
    // Fraction: a `.` is only consumed when an integer or fractional
    // digit is present (`"."` and `"+."` are errors; `".5"` and `"1."`
    // are valid).
    if i < bytes.len() && bytes[i] == b'.' {
        let mut j = i + 1;
        let mut frac_digit = false;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
            frac_digit = true;
        }
        if saw_digit || frac_digit {
            i = j;
            saw_digit = true;
        }
    }
    if !saw_digit {
        return None;
    }
    // Exponent: only consumed when `e`/`E` is followed by an optional
    // sign and at least one digit (`"1e"` parses as `1`).
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        if j < bytes.len() && (bytes[j] == b'-' || bytes[j] == b'+') {
            j += 1;
        }
        let mut exp_digit = false;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
            exp_digit = true;
        }
        if exp_digit {
            i = j;
        }
    }
    let value: f64 = s[start..i].parse().ok()?;
    value.is_finite().then_some(value)
}

/// HTML "valid floating-point number" (§2.3.4.3) — the STRICT production
/// (whole string must match: optional `-`, digits and/or `.`digits,
/// optional `e`/`E` sign digits; no leading whitespace, no leading `+`,
/// no trailing content, no `"1."` / `"1e"`).
///
/// Used to read the number/range element's **value** in the stepUp /
/// stepDown algorithm (HTML §4.10.5.4 step 5).  Although that step is
/// phrased with the permissive "convert a string to a number", it
/// operates on a value the number-state value sanitization algorithm
/// (§4.10.5.1.12) has already reduced to a valid floating-point number
/// or the empty string.  Parsing the stored value strictly enforces
/// that invariant here, so a not-yet-sanitized invalid string (e.g.
/// `"1e"`) is treated as the empty / error case rather than the
/// permissive parser's partial result.  The author-provided
/// `min`/`max`/`step` attributes keep using [`parse_floating_point`]
/// (the permissive rules, as the spec specifies for those attributes).
///
/// Returns `None` (the error case) for any string that is not a valid
/// floating-point number, or that overflows to a non-finite value.
fn parse_valid_floating_point(s: &str) -> Option<f64> {
    let bytes = s.as_bytes();
    let mut i = 0;
    if bytes.first() == Some(&b'-') {
        i += 1;
    }
    // Integer part (optional) and fraction part (`.` + 1+ digits,
    // optional) — at least one of the two must be present.
    let int_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let has_int = i > int_start;
    let mut has_frac = false;
    if i < bytes.len() && bytes[i] == b'.' {
        let frac_start = i + 1;
        let mut j = frac_start;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j > frac_start {
            has_frac = true;
            i = j;
        }
        // A `.` with no following digit (`"1."`) leaves `i` at the `.`
        // and fails the whole-string check below.
    }
    if !has_int && !has_frac {
        return None;
    }
    // Exponent (optional): `e`/`E`, optional sign, 1+ digits.
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        if j < bytes.len() && (bytes[j] == b'-' || bytes[j] == b'+') {
            j += 1;
        }
        let exp_start = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j == exp_start {
            return None; // `e` with no digits.
        }
        i = j;
    }
    if i != bytes.len() {
        return None; // leading `+`/whitespace or trailing content.
    }
    let value: f64 = s.parse().ok()?;
    value.is_finite().then_some(value)
}

/// The default step / step scale factor for the number and range
/// states are `1` / `1` (HTML §4.10.5.1.12 "Number state (type=number)"
/// and §4.10.5.1.13 "Range state (type=range)"), so the absent /
/// invalid-step fallback is `1.0` for both.
const DEFAULT_STEP: f64 = 1.0;

/// HTML "allowed value step" (§4.10.5.3.8 "The `step` attribute").
///
/// Returns `None` when there is **no** allowed value step (the
/// `step="any"` case, HTML §4.10.5.4 step 2 → `InvalidStateError`).
/// For number/range, absent / unparseable / zero / negative all fall
/// back to the default step (`1.0`).
fn allowed_value_step(state: &FormControlState) -> Option<f64> {
    match state.step.as_deref() {
        // Absent → default step × scale factor.
        None => Some(DEFAULT_STEP),
        // "any" (ASCII case-insensitive) → no allowed value step.
        Some(s) if s.eq_ignore_ascii_case("any") => None,
        Some(s) => match parse_floating_point(s) {
            // Error / zero / negative → default step.
            Some(v) if v > 0.0 => Some(v),
            _ => Some(DEFAULT_STEP),
        },
    }
}

/// HTML "step base" (§4.10.5.3.7 "The `min` and `max` attributes"):
/// `min` content attribute → `value` content attribute
/// (`default_value`) → type default step base → `0`.
///
/// Neither the number nor the range state defines a default step base,
/// so both fall through to `0`.
fn step_base(state: &FormControlState) -> f64 {
    if let Some(v) = state.min.as_deref().and_then(parse_floating_point) {
        return v;
    }
    if let Some(v) = parse_floating_point(&state.default_value) {
        return v;
    }
    0.0
}

/// HTML "minimum" (§4.10.5.3.7).  The number state has no default
/// minimum; the range state's default minimum is `0`.
fn minimum(state: &FormControlState) -> Option<f64> {
    state
        .min
        .as_deref()
        .and_then(parse_floating_point)
        .or(match state.kind {
            FormControlKind::Range => Some(0.0),
            _ => None,
        })
}

/// HTML "maximum" (§4.10.5.3.7).  The number state has no default
/// maximum; the range state's default maximum is `100`.
fn maximum(state: &FormControlState) -> Option<f64> {
    state
        .max
        .as_deref()
        .and_then(parse_floating_point)
        .or(match state.kind {
            FormControlKind::Range => Some(100.0),
            _ => None,
        })
}

/// `(value − base) / step` — the step-count of `value` relative to the
/// step grid anchored at `base`.
fn step_ratio(value: f64, base: f64, step: f64) -> f64 {
    (value - base) / step
}

/// Tolerance for the step-alignment test (HTML §4.10.5.4 step 7),
/// bounding the `f64` error in `ratio = (value − base) / step`: the
/// cancellation in `value − base` contributes `≈ ε·(|value| + |base|) /
/// |step|` and the division/rounding add `≈ ε·|ratio|`.  Bounding only
/// by `|ratio|` would reject aligned values that suffered cancellation
/// (e.g. `min=16 step=0.001 value=16.001`).  Capped by
/// [`STEP_ALIGN_MAX_TOLERANCE`] below ½ step.  `step` is always positive
/// here (the no-allowed-value-step / non-positive cases are handled by
/// [`allowed_value_step`]).
fn align_tolerance(value: f64, base: f64, step: f64) -> f64 {
    let ratio = step_ratio(value, base, step);
    let error_magnitude = (value.abs() + base.abs()) / step.abs() + ratio.abs();
    (STEP_ALIGN_TOLERANCE_ULPS * error_magnitude).min(STEP_ALIGN_MAX_TOLERANCE)
}

/// Whether `value`, when subtracted from `base`, is an integral
/// multiple of `step` (HTML §4.10.5.4 step 7), within [`align_tolerance`].
fn is_step_aligned(value: f64, base: f64, step: f64) -> bool {
    let ratio = step_ratio(value, base, step);
    (ratio - ratio.round()).abs() <= align_tolerance(value, base, step)
}

/// The step grid index of `value`, snapped to the nearest integer when
/// `value` is itself on the grid (its ratio is within [`align_tolerance`]
/// of an integer) so that float noise — e.g. a `max` that lies exactly
/// on the grid but whose ratio computes as `0.9999…` — does not push
/// `floor`/`ceil` a whole step the wrong way.  For a genuinely off-grid
/// `value` the ratio is returned unsnapped for the caller to floor/ceil.
fn grid_index(value: f64, base: f64, step: f64) -> f64 {
    let ratio = step_ratio(value, base, step);
    let nearest = ratio.round();
    if (ratio - nearest).abs() <= align_tolerance(value, base, step) {
        nearest
    } else {
        ratio
    }
}

/// Largest step-aligned value `≤ value`.
fn aligned_below(value: f64, base: f64, step: f64) -> f64 {
    base + grid_index(value, base, step).floor() * step
}

/// Smallest step-aligned value `≥ value`.
fn aligned_above(value: f64, base: f64, step: f64) -> f64 {
    base + grid_index(value, base, step).ceil() * step
}

/// Apply a `stepUp(n)` / `stepDown(n)` adjustment to a form control
/// state, implementing the HTML §4.10.5.4 "Common input element APIs"
/// 12-step algorithm for the number and range states.
///
/// `direction` is `+1.0` for `stepUp` and `-1.0` for `stepDown`.
/// Returns:
/// * `Err(StepError::NotSupported)` — step 1 (kind does not apply);
/// * `Err(StepError::NoAllowedValueStep)` — step 2 (`step="any"`);
/// * `Ok(())` with the value unchanged — the no-op returns
///   (steps 3, 4, 10);
/// * `Ok(())` with the value updated — steps 11–12.
pub fn apply_step(state: &mut FormControlState, n: f64, direction: f64) -> Result<(), StepError> {
    // Step 1: stepUp()/stepDown() must apply to the type.  Only the
    // number and range states are handled here — the date/time-family
    // step-supporting states (date, datetime-local, time, week, month)
    // need date-string "convert a/to string ↔ number" conversions, so
    // they are deferred to slot `#11-input-step-date-types` and treated
    // as not-supported for now.
    if !matches!(state.kind, FormControlKind::Number | FormControlKind::Range) {
        return Err(StepError::NotSupported);
    }
    // Step 2: the element must have an allowed value step.
    let step = allowed_value_step(state).ok_or(StepError::NoAllowedValueStep)?;

    let base = step_base(state);
    let min = minimum(state);
    let max = maximum(state);

    // Step 3: min > max → no-op.
    if let (Some(lo), Some(hi)) = (min, max) {
        if lo > hi {
            return Ok(());
        }
        // Step 4: no step-aligned value exists in [min, max] → no-op.
        if aligned_above(lo, base, step) > hi {
            return Ok(());
        }
    }

    // Step 5: convert the value to a number (error → 0).  The value is a
    // sanitized number-state value (a valid floating-point number or
    // empty, §4.10.5.1.12), so parse it strictly — `parse_valid_…`, not
    // the permissive attribute parser.
    let mut value = parse_valid_floating_point(state.value()).unwrap_or(0.0);
    // Step 6: snapshot for the reverse-direction guard.
    let value_before = value;

    // Step 7: snap an unaligned value toward `direction`, else step by
    // the allowed value step × n.
    if is_step_aligned(value, base, step) {
        let delta = step * n * direction;
        value += delta;
    } else if direction < 0.0 {
        value = aligned_below(value, base, step);
    } else {
        value = aligned_above(value, base, step);
    }

    // Step 8: clamp up to the smallest aligned value ≥ minimum.
    if let Some(lo) = min {
        if value < lo {
            value = aligned_above(lo, base, step);
        }
    }
    // Step 9: clamp down to the largest aligned value ≤ maximum.
    if let Some(hi) = max {
        if value > hi {
            value = aligned_below(hi, base, step);
        }
    }

    // Step 10: a reverse-direction overshoot is a no-op (e.g.
    // `<input type=number value=1 max=0>`.stepUp()).
    if (direction < 0.0 && value > value_before) || (direction > 0.0 && value < value_before) {
        return Ok(());
    }

    // The spec works in exact real arithmetic; with `f64` an extreme
    // `step × n` can overflow to a non-finite value (only reachable for
    // a pathologically large `step` with an unbounded maximum).  A
    // number/range value must be a valid floating-point number, so
    // never write a non-finite string — leave the value unchanged.
    if !value.is_finite() {
        return Ok(());
    }

    // Steps 11–12: convert the number to a string and set the value.
    state.set_value(value.to_string());
    Ok(())
}

/// HTML §4.10.5 type-change sanitize step.
///
/// Run after `state.kind` has been updated from `old_kind` to the
/// new value, to bring `FormControlState` back into a consistent
/// shape per the new type's invariants:
///
/// 1. **Checkable-state cleanup** (elidex normalization, beyond the
///    spec): if the old kind was `Checkbox` or `Radio` and the new
///    kind is neither, clear `checked` and `indeterminate`.  The HTML
///    §4.10.5 type-change steps only sanitize `value`/rendering and
///    never clear checkedness or indeterminateness (both merely become
///    inert on non-checkable types); elidex clears them so
///    `FormControlState` carries no stale checkable bits.
/// 2. **Number value sanitization**: if the new kind is `Number`
///    and the current value isn't a finite floating-point literal,
///    clear it (per HTML §4.10.5.1.12 number value-sanitization
///    algorithm — non-numeric values are rejected to `""`).
///
/// Other per-type sanitize algorithms (Color, URL, Email, Date,
/// Range clamp) are deferred to the next implementation pass — the
/// value-clearing rule for number is the most-frequently-tripping
/// branch in the wild and the only one with a JS-observable
/// regression today.
pub fn sanitize_for_type_change(state: &mut FormControlState, old_kind: FormControlKind) {
    if state.kind == old_kind {
        return;
    }
    let was_checkable = matches!(old_kind, FormControlKind::Checkbox | FormControlKind::Radio);
    let is_checkable = matches!(
        state.kind,
        FormControlKind::Checkbox | FormControlKind::Radio
    );
    if was_checkable && !is_checkable {
        state.checked = false;
        state.indeterminate = false;
    }
    if state.kind == FormControlKind::Number {
        let value_is_valid_number = state.value().parse::<f64>().is_ok_and(f64::is_finite);
        if !value_is_valid_number && !state.value().is_empty() {
            state.set_value(String::new());
        }
    }
}

/// Check if inserting a character would exceed maxlength.
fn would_exceed_maxlength(state: &FormControlState) -> bool {
    if let Some(max) = state.maxlength {
        state.char_count >= max
    } else {
        false
    }
}

/// Check if a character is valid for a Number input.
fn is_valid_number_char(ch: char) -> bool {
    ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == 'e' || ch == 'E' || ch == '+'
}

/// Handle a key press for a text-like control.
fn handle_text_key(state: &mut FormControlState, key: &str) -> KeyAction {
    match key {
        "Backspace" => {
            if state.cursor_pos > 0 {
                let prev = prev_char_boundary(&state.value, state.cursor_pos);
                state.value.drain(prev..state.cursor_pos);
                state.cursor_pos = prev;
                state.update_char_count();
                KeyAction::Consumed
            } else {
                KeyAction::None
            }
        }
        "Delete" => {
            if state.cursor_pos < state.value.len() {
                let next = next_char_boundary(&state.value, state.cursor_pos);
                state.value.drain(state.cursor_pos..next);
                state.update_char_count();
                KeyAction::Consumed
            } else {
                KeyAction::None
            }
        }
        "ArrowLeft" | "ArrowRight" | "Home" | "End" => navigate_cursor(state, key),
        "Enter" => {
            if state.kind == FormControlKind::TextArea {
                state.value.insert(state.cursor_pos, '\n');
                state.cursor_pos += 1;
                state.update_char_count();
                KeyAction::Consumed
            } else if state.kind.is_single_line_text() || state.kind == FormControlKind::Number {
                // Implicit form submission.
                KeyAction::Submit
            } else {
                KeyAction::None
            }
        }
        _ => {
            // Insert printable character (single-char keys only).
            if key.len() == 1 || (key.chars().count() == 1 && !key.starts_with("Arrow")) {
                let ch = key.chars().next().unwrap();
                // HTML spec: single-line inputs reject \n and \r.
                if !ch.is_control()
                    && !(state.kind.is_single_line_text() && (ch == '\n' || ch == '\r'))
                    && !(state.kind == FormControlKind::Number && (ch == '\n' || ch == '\r'))
                {
                    // Number inputs only accept numeric characters.
                    if state.kind == FormControlKind::Number && !is_valid_number_char(ch) {
                        return KeyAction::None;
                    }
                    // Enforce maxlength (HTML spec §4.10.5.2.7).
                    if would_exceed_maxlength(state) {
                        return KeyAction::None;
                    }
                    state.value.insert(state.cursor_pos, ch);
                    state.cursor_pos += ch.len_utf8();
                    state.update_char_count();
                    return KeyAction::Consumed;
                }
            }
            KeyAction::None
        }
    }
}

/// Handle navigation-only keys for readonly text controls.
///
/// Readonly controls still allow cursor movement (ArrowLeft/Right, Home, End)
/// but reject all value-modifying keys (character insert, Backspace, Delete, Enter).
fn handle_readonly_navigation(state: &mut FormControlState, key: &str) -> bool {
    navigate_cursor(state, key) == KeyAction::Consumed
}

/// Resolve `<input>.list` to its associated `<datalist>` per WHATWG HTML
/// §4.10.5.1.16: "the first element in the tree of type `HTMLDataListElement`
/// whose ID is equal to the value of the `list` attribute, if that element is
/// in the same tree as the input element".
///
/// Returns `None` for input types the `list` attribute does not apply to
/// (hidden / checkbox / radio / file / submit / image / reset / button /
/// password — see `input_list_applies_to_type` for the spec exclusion set).
///
/// Tree scope honors shadow boundaries — nested shadow subtrees within the
/// same root are correctly excluded per the spec's "same tree" wording.
/// Cross-tree (shadow-piercing) resolution is tracked at the
/// `#11-form-elements-cross-tree` defer slot.
#[must_use]
pub fn resolve_input_list(dom: &EcsDom, input_entity: Entity) -> Option<Entity> {
    if !input_list_applies_to_type(dom, input_entity) {
        return None;
    }
    let list_id: String = {
        let attrs = dom.world().get::<&Attributes>(input_entity).ok()?;
        let v = attrs.get("list")?;
        if v.is_empty() {
            return None;
        }
        v.to_owned()
    };

    // `traverse_descendants` skips `root` itself; check explicitly.
    let root = dom.find_tree_root(input_entity);
    if matches_datalist_with_id(dom, root, list_id.as_str()) {
        return Some(root);
    }
    let mut candidate = None;
    dom.traverse_descendants(root, |entity| {
        if matches_datalist_with_id(dom, entity, list_id.as_str()) {
            candidate = Some(entity);
            return false;
        }
        true
    });
    candidate
}

fn matches_datalist_with_id(dom: &EcsDom, entity: Entity, id: &str) -> bool {
    // Tag name is the cheapest discriminator (this runs on every descendant
    // of the tree walk), so check it first and reject non-`<datalist>` nodes
    // before the namespace lookup.
    let Ok(tag) = dom.world().get::<&TagType>(entity) else {
        return false;
    };
    if !tag.0.as_str().eq_ignore_ascii_case("datalist") {
        return false;
    }
    drop(tag);
    // The `list` attribute must reference an element of type
    // `HTMLDataListElement` (HTML §4.10.8), so a foreign-namespace
    // `<datalist>` look-alike (SVG / MathML) does not match.
    if !dom.is_html_namespace(entity) {
        return false;
    }
    dom.world()
        .get::<&Attributes>(entity)
        .is_ok_and(|a| a.get("id") == Some(id))
}

/// `<input>.list` applicability per HTML §4.10.5.1.16.
///
/// Reads the `type` content attribute directly (spec source of truth):
/// `setAttribute("type", X)` mutates `Attributes` synchronously while
/// any cached `FormControlState.kind` only re-syncs on a type-change
/// sanitize pass — preferring the cached kind would let stale state
/// mask a fresh `setAttribute("type", "hidden")` mutation.
///
/// Missing attribute defaults to `"text"` per HTML §4.10.5.1 missing-
/// value-default rule.
///
/// Exclusion set is matched against the spec text directly (rather than
/// routed through [`FormControlKind`]) because `from_type_str` collapses
/// `"image"` (and the unmodeled `"month"` / `"week"` / `"time"`) onto
/// `TextInput` — that fallback is harmless for the applicable types but
/// would incorrectly admit `<input type="image">` if the predicate was
/// gated on `FormControlKind::list_applies`.
fn input_list_applies_to_type(dom: &EcsDom, input_entity: Entity) -> bool {
    let Ok(attrs) = dom.world().get::<&Attributes>(input_entity) else {
        return true;
    };
    let type_str = attrs.get("type").unwrap_or("text");
    !matches!(
        type_str.to_ascii_lowercase().as_str(),
        "hidden"
            | "checkbox"
            | "radio"
            | "file"
            | "submit"
            | "image"
            | "reset"
            | "button"
            | "password"
    )
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "input_step_tests.rs"]
mod step_tests;
