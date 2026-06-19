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
    aligned_above, aligned_below, allowed_value_step, is_step_aligned,
    is_valid_floating_point_string, maximum, minimum, step_base,
};
use crate::{datetime, FormControlKind, FormControlState};

/// HTML "strip newlines from a string" — remove every U+000A LF and
/// U+000D CR.  Used by the text / search / telephone / password / URL /
/// email value sanitization algorithms (§4.10.5.1.2–.6).
fn strip_newlines(s: &str) -> String {
    s.chars().filter(|&c| c != '\n' && c != '\r').collect()
}

/// HTML "normalize newlines in a string" — replace every U+000D U+000A
/// CRLF pair with a single U+000A LF, then every remaining lone U+000D CR
/// with U+000A LF (order-significant).  This derives a `<textarea>`'s
/// **API value** from its raw value (HTML §4.10.11): the API value is what
/// the `value` / `textLength` IDL attributes and `maxlength` / `minlength`
/// observe.  Storing the normalized value at every value-establishment site
/// keeps those observers (and the shared selection conversion) spec-correct
/// uniformly — see [`FormControlState::settle_value`].
///
/// Scope: this covers value-*establishment* (the IDL `value` setter, parser
/// child-text init, form reset).  Folding newlines on the incremental
/// *editing* paths (`setRangeText` / paste / IME) — together with maxlength
/// counted on the API value and the `InputEvent` data — is the coupled
/// follow-up (`#11-textarea-edit-path-newline-normalization`).
fn normalize_newlines(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
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
    // A grammar-valid value (HTML §2.3.4.3) is interpreted as a number even
    // if its magnitude overflows `f64` (`"1e309"` → +∞, which clamps to max
    // below) — validity is the GRAMMAR, not numeric representability.  A
    // grammar-invalid value (`"abc"`, `"1e"`) → best representation of the
    // default (midpoint).
    let parsed = is_valid_floating_point_string(state.value())
        .then(|| state.value().parse::<f64>().ok())
        .flatten();
    let default = if max < min {
        min
    } else {
        // Midpoint as `0.5*min + 0.5*max`, NOT `min + (max-min)/2`: the
        // latter overflows the `max - min` intermediate to infinity for
        // extreme finite endpoints (e.g. `min=-1e308 max=1e308`), which
        // would store `"inf"` — not a valid floating-point number.
        0.5 * min + 0.5 * max
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
/// → `value`; it never touches `dirty_value`, and it is a **pure value
/// transform**: it imposes no cursor-collapse / selection-direction POLICY
/// (that is the value-mutation algorithm's job and differs per call site) and
/// does not itself re-sync `char_count` or clamp the cursor / selection.  The
/// "selection is within the value" invariant and the `char_count` re-sync are
/// owned by [`FormControlState::settle_value`], the single establishment
/// primitive every caller routes through.
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
        // (a valid number is kept verbatim, never reserialized).  Validity
        // is the GRAMMAR (§2.3.4.3), not f64 representability — a
        // grammatically valid magnitude that overflows f64 (`"1e309"`) is a
        // valid floating-point number string and is kept verbatim.
        FormControlKind::Number => {
            (!is_valid_floating_point_string(state.value())).then(String::new)
        }
        // §4.10.5.1.13 Range: invalid → default; clamp; snap to step.
        FormControlKind::Range => sanitize_range(state),
        // §4.10.5.1.7–.10 date/month/week/time: "if the value is not a
        // valid <type> string, set it to empty" — a VALID value is kept
        // VERBATIM (no normalization).  The validity test is *syntactic*
        // (`is_valid_datetime_string`), NOT `convert_valid_string_to_number`:
        // the latter also returns `None` when a syntactically valid but
        // astronomically large year overflows the i64 ms space, which would
        // wrongly empty a valid string.  Do NOT round-trip through
        // convert_number_to_string here — that would reserialize a
        // valid-but-non-canonical string (e.g. `time` `"08:00:00"` → `"08:00"`).
        FormControlKind::Date
        | FormControlKind::Month
        | FormControlKind::Week
        | FormControlKind::Time => {
            (!datetime::is_valid_datetime_string(state.kind, state.value())).then(String::new)
        }
        // §4.10.5.1.11 Local Date and Time: valid → *normalized* valid
        // string (the one date/time state the spec normalizes); else empty.
        // Normalizes from the parsed components (NOT the combined-ms
        // round-trip) so a syntactically valid huge year is normalized and
        // kept rather than emptied by an i64 ms overflow (mirrors the
        // date/month/week/time syntactic-validity fix above).
        FormControlKind::DatetimeLocal => {
            Some(datetime::normalize_local_date_time(state.value()).unwrap_or_default())
        }
        // §4.10.11 `<textarea>`: the stored value is the element's API
        // value = its raw value with newlines normalized (CRLF→LF, lone
        // CR→LF).  Normalizing at every value-*establishment* site (the IDL
        // `value` setter, parser child-text init, form reset — all via
        // `settle_value`) makes `value` / `textLength` / `maxlength` and the
        // shared selection conversion observe the API value for values set
        // that way.  The incremental edit paths (`setRangeText` / paste / IME)
        // do NOT pass through here and still leave raw CRs in the value —
        // folding them, with the coupled maxlength / `InputEvent` handling, is
        // the follow-up `#11-textarea-edit-path-newline-normalization`.
        FormControlKind::TextArea => Some(normalize_newlines(state.value())),
        // No value sanitization algorithm: hidden, checkbox, radio, file,
        // submit/reset/image/button, color (§4.10.5.1.14 color-well
        // control — deferred, `#11-input-color-well-sanitize`), and the
        // remaining non-input kinds (select / output / meter / progress).
        FormControlKind::Hidden
        | FormControlKind::Checkbox
        | FormControlKind::Radio
        | FormControlKind::File
        | FormControlKind::SubmitButton
        | FormControlKind::ResetButton
        | FormControlKind::Button
        | FormControlKind::Color
        | FormControlKind::Select
        | FormControlKind::Output
        | FormControlKind::Meter
        | FormControlKind::Progress => None,
    };
    if let Some(sanitized) = sanitized {
        state.value = sanitized;
    }
}

#[cfg(test)]
#[path = "sanitize_tests.rs"]
mod tests;
