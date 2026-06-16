//! HTML §4.10.5.1.x **value sanitization algorithm** for the `<input>`
//! states — the per-`kind` transform that brings the stored
//! [`FormControlState::value`] into the shape the spec requires
//! (strip-newlines / trim / clamp / snap / empty-on-invalid).
//!
//! Split out of `input.rs` to keep that module under the 1000-line
//! convention.  The step-grid primitives (`minimum`/`maximum`/`step_base`
//! /`allowed_value_step`/`is_step_aligned`/`aligned_below`/`aligned_above`
//! and `parse_valid_floating_point`) live in [`crate::input`] and are
//! shared with [`crate::input::apply_step`] so sanitization,
//! stepUp/stepDown, and constraint validation agree (the #344
//! cancellation-aware tolerance).

use crate::input::{
    aligned_above, aligned_below, allowed_value_step, is_step_aligned, maximum, minimum,
    parse_valid_floating_point, step_base,
};
use crate::{datetime, FormControlKind, FormControlState};

/// HTML "strip newlines from a string" — remove every U+000A LF and
/// U+000D CR.  Used by the text / search / telephone / password / URL /
/// email value sanitization algorithms (§4.10.5.1.2–.6).
fn strip_newlines(s: &str) -> String {
    s.chars().filter(|&c| c != '\n' && c != '\r').collect()
}

/// HTML "strip leading and trailing ASCII whitespace".  Rust's
/// [`char::is_ascii_whitespace`] matches exactly the HTML ASCII
/// whitespace set (U+0009 TAB, U+000A LF, U+000C FF, U+000D CR,
/// U+0020 SPACE), so it is the faithful predicate here.
fn strip_ascii_whitespace_ends(s: &str) -> String {
    s.trim_matches(|c: char| c.is_ascii_whitespace())
        .to_string()
}

/// §4.10.5.1.5 Email-state value sanitization, `multiple` case: split
/// the value on U+002C COMMA, strip leading and trailing ASCII
/// whitespace from each resulting token, then concatenate the tokens
/// separated by a single comma, preserving order.
fn sanitize_email_list(s: &str) -> String {
    s.split(',')
        .map(|tok| tok.trim_matches(|c: char| c.is_ascii_whitespace()))
        .collect::<Vec<_>>()
        .join(",")
}

/// §4.10.5.1.13 Range-state value sanitization.  Returns the new value
/// string, or `None` when the value is already a valid, in-range,
/// step-aligned floating-point number (kept verbatim — the spec only
/// rewrites the value when a sanitization rule fires).
///
/// An invalid value becomes the best representation of the default value
/// (the midpoint of `[min, max]`, or `min` when `max < min`).  A valid
/// value is then clamped into `[min, max]` and, on a step mismatch,
/// rounded to the nearest in-range step (ties round toward positive
/// infinity per the spec).  The step grid math is shared with
/// [`crate::input::apply_step`] (the #344 cancellation-aware tolerance)
/// so sanitization, stepUp/stepDown, and constraint validation agree.
fn sanitize_range(state: &FormControlState) -> Option<String> {
    let min = minimum(state).unwrap_or(0.0);
    let max = maximum(state).unwrap_or(100.0);
    // Value sanitization step 1: an invalid float becomes the best
    // representation of the default value (midpoint of [min, max], or min
    // when max < min).  The underflow/overflow/step-mismatch rules below
    // then apply to the resulting value REGARDLESS of how it arrived — the
    // default itself can be off-step and must snap (spec worked example:
    // min=0 max=100 step=20 value=abc → default 50 → step-mismatch → 60).
    let parsed = parse_valid_floating_point(state.value());
    let default = if max < min {
        min
    } else {
        min + (max - min) / 2.0
    };
    let mut value = parsed.unwrap_or(default);
    // Underflow → minimum.
    if value < min {
        value = min;
    }
    // Overflow → maximum (only when the range is non-empty: max ≥ min).
    if max >= min && value > max {
        value = max;
    }
    // Step mismatch → nearest in-range step (ties → positive infinity).
    if let Some(step) = allowed_value_step(state) {
        let base = step_base(state);
        if !is_step_aligned(value, base, step) {
            let below = aligned_below(value, base, step);
            let above = aligned_above(value, base, step);
            let in_range = |c: f64| c >= min && (max < min || c <= max);
            value = match (in_range(below), in_range(above)) {
                // Nearest; an exact tie rounds up (toward +∞).
                (true, true) => {
                    if (value - below) < (above - value) {
                        below
                    } else {
                        above
                    }
                }
                (true, false) => below,
                (false, true) => above,
                // No in-range step exists (step wider than the range) —
                // leave the clamped value as-is.
                (false, false) => value,
            };
        }
    }
    // A valid input that no rule changed is kept verbatim (`None`);
    // otherwise — an invalid input (rewritten to the sanitized default) or a
    // valid input a rule clamped/snapped — emit the best floating-point
    // representation of the sanitized number.
    if parsed == Some(value) {
        None
    } else {
        Some(value.to_string())
    }
}

/// HTML §4.10.5.1.x **value sanitization algorithm**, dispatched per
/// input `kind`.  Invoked at every value-establishment site — the IDL
/// `value` / `valueAsNumber` / `defaultValue` setters, the `value`- and
/// `multiple`-attribute reconciler arms, the `type`-change step, form
/// reset, and the initial parse (`from_input_element`) — so the stored
/// `value` is always the sanitized value rather than the raw author /
/// script string.  Per-keystroke editing mutations are deliberately NOT
/// routed through here (the editing buffer is the live value).
///
/// A pure transform of (`value`, `kind`, `min`/`max`/`step`, `multiple`)
/// → `value`; it never touches `dirty_value` (each call site owns the
/// dirty policy).  When it changes `value` it re-establishes the
/// value-dependent bookkeeping — `char_count` and the cursor / selection
/// offsets, collapsed to the end of the new value (HTML §4.10.5.4
/// `value` IDL setter, step 5) — so the "selection is within the value"
/// invariant holds by construction even when sanitization shortens the value.
pub(crate) fn sanitize_value(state: &mut FormControlState) {
    let sanitized: Option<String> = match state.kind {
        // §4.10.5.1.2 Text/Search, §4.10.5.1.3 Telephone,
        // §4.10.5.1.6 Password: strip newlines.
        FormControlKind::TextInput
        | FormControlKind::Search
        | FormControlKind::Tel
        | FormControlKind::Password => Some(strip_newlines(state.value())),
        // §4.10.5.1.4 URL: strip newlines, then strip leading/trailing
        // ASCII whitespace.
        FormControlKind::Url => Some(strip_ascii_whitespace_ends(&strip_newlines(state.value()))),
        // §4.10.5.1.5 Email: single → strip newlines + trim; multiple →
        // comma-split / trim each token / rejoin.
        FormControlKind::Email => Some(if state.multiple {
            sanitize_email_list(state.value())
        } else {
            strip_ascii_whitespace_ends(&strip_newlines(state.value()))
        }),
        // §4.10.5.1.12 Number: not a valid floating-point number → empty
        // (a valid number is kept verbatim, never reserialized).
        FormControlKind::Number => parse_valid_floating_point(state.value())
            .is_none()
            .then(String::new),
        // §4.10.5.1.13 Range: invalid → default; clamp; snap to step.
        FormControlKind::Range => sanitize_range(state),
        // §4.10.5.1.7–.10 date/month/week/time: "if the value is not a
        // valid <type> string, set it to empty" — a VALID value is kept
        // VERBATIM (no normalization).  Mirror the Number arm: validity
        // check → empty on failure, else None (keep).  Do NOT round-trip
        // through convert_number_to_string here — that would reserialize a
        // valid-but-non-canonical string (e.g. `time` `"08:00:00"` → `"08:00"`).
        FormControlKind::Date
        | FormControlKind::Month
        | FormControlKind::Week
        | FormControlKind::Time => {
            datetime::convert_valid_string_to_number(state.kind, state.value())
                .is_none()
                .then(String::new)
        }
        // §4.10.5.1.11 Local Date and Time: valid → *normalized* valid
        // string (the one date/time state the spec normalizes); else empty.
        FormControlKind::DatetimeLocal => Some(
            datetime::convert_valid_string_to_number(state.kind, state.value())
                .and_then(|n| datetime::convert_number_to_string(state.kind, n))
                .unwrap_or_default(),
        ),
        // No value sanitization algorithm: hidden, checkbox, radio, file,
        // submit/reset/image/button, color (§4.10.5.1.14 color-well
        // control — deferred, `#11-input-color-well-sanitize`), and the
        // non-input kinds (textarea / select / output / meter / progress).
        FormControlKind::Hidden
        | FormControlKind::Checkbox
        | FormControlKind::Radio
        | FormControlKind::File
        | FormControlKind::SubmitButton
        | FormControlKind::ResetButton
        | FormControlKind::Button
        | FormControlKind::Color
        | FormControlKind::TextArea
        | FormControlKind::Select
        | FormControlKind::Output
        | FormControlKind::Meter
        | FormControlKind::Progress => None,
    };
    if let Some(sanitized) = sanitized {
        if sanitized != state.value {
            state.value = sanitized;
            // HTML §4.10.5.4 (`value` IDL setter) step 5: collapse the
            // selection to the end of the new value; keep char count in sync.
            let end = state.value.len();
            state.cursor_pos = end;
            state.selection_start = end;
            state.selection_end = end;
            state.update_char_count();
        }
    }
}

#[cfg(test)]
#[path = "sanitize_tests.rs"]
mod tests;
