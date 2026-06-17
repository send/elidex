//! Tests for the HTML §4.10.5.1.x value sanitization algorithm
//! ([`super::sanitize_value`]) and its wiring through the value-setter /
//! initial-parse chokepoints.

use super::*;
use elidex_ecs::Attributes;

/// Construct a state with a raw value via direct field literal (bypassing
/// `set_value`'s sanitization) so `sanitize_value` can be exercised
/// directly on the raw input.
fn raw_state(kind: FormControlKind, value: &str) -> FormControlState {
    FormControlState {
        kind,
        value: value.to_string(),
        char_count: value.chars().count(),
        ..FormControlState::default()
    }
}

#[test]
fn sanitize_text_strips_newlines() {
    // §4.10.5.1.2/.3/.6: text/search/tel/password strip CR + LF only.
    for kind in [
        FormControlKind::TextInput,
        FormControlKind::Search,
        FormControlKind::Tel,
        FormControlKind::Password,
    ] {
        let mut s = raw_state(kind, "a\r\nb\nc");
        sanitize_value(&mut s);
        assert_eq!(s.value(), "abc", "kind {kind:?}");
        // Embedded spaces/tabs are NOT stripped (only newlines).
        let mut s = raw_state(kind, " a b ");
        sanitize_value(&mut s);
        assert_eq!(s.value(), " a b ", "kind {kind:?} keeps spaces");
    }
}

#[test]
fn sanitize_url_strips_newlines_then_trims() {
    // §4.10.5.1.4: strip newlines, then strip leading/trailing ASCII ws.
    let mut s = raw_state(FormControlKind::Url, "  https://a\n.test/  ");
    sanitize_value(&mut s);
    assert_eq!(s.value(), "https://a.test/");
}

#[test]
fn sanitize_email_single_trims() {
    // §4.10.5.1.5 (no multiple): strip newlines + trim ends; internal
    // whitespace is preserved.
    let mut s = raw_state(FormControlKind::Email, "  a@b.com\n  ");
    sanitize_value(&mut s);
    assert_eq!(s.value(), "a@b.com");
}

#[test]
fn sanitize_email_multiple_splits_trims_rejoins() {
    // §4.10.5.1.5 (multiple): split on comma, trim each token, rejoin.
    let mut s = raw_state(FormControlKind::Email, " a@b , c@d ");
    s.multiple = true;
    sanitize_value(&mut s);
    assert_eq!(s.value(), "a@b,c@d");
}

#[test]
fn sanitize_number_invalid_to_empty_valid_kept() {
    // §4.10.5.1.12: non-valid-float → empty; a valid number is kept
    // verbatim (NOT reserialized).
    let mut s = raw_state(FormControlKind::Number, "1e");
    sanitize_value(&mut s);
    assert_eq!(s.value(), "");
    let mut s = raw_state(FormControlKind::Number, "1.50");
    sanitize_value(&mut s);
    assert_eq!(s.value(), "1.50", "valid number kept verbatim");
}

#[test]
fn sanitize_range_invalid_to_default() {
    // §4.10.5.1.13: invalid → best representation of the default value
    // (midpoint of [min, max]); default range is 0..100 → 50.
    let mut s = raw_state(FormControlKind::Range, "abc");
    sanitize_value(&mut s);
    assert_eq!(s.value(), "50");
}

#[test]
fn sanitize_range_clamps_underflow_overflow() {
    let mut over = raw_state(FormControlKind::Range, "150");
    sanitize_value(&mut over);
    assert_eq!(over.value(), "100");
    let mut under = raw_state(FormControlKind::Range, "-5");
    sanitize_value(&mut under);
    assert_eq!(under.value(), "0");
}

#[test]
fn sanitize_range_snaps_step_tie_rounds_up() {
    // HTML §4.10.5.1.13 worked example: min=0 max=100 step=20 value=50
    // → 60 (equidistant tie rounds toward positive infinity).
    let mut s = FormControlState {
        kind: FormControlKind::Range,
        value: "50".to_string(),
        min: Some("0".to_string()),
        max: Some("100".to_string()),
        step: Some("20".to_string()),
        ..FormControlState::default()
    };
    sanitize_value(&mut s);
    assert_eq!(s.value(), "60");
}

#[test]
fn sanitize_range_keeps_valid_on_grid_value_verbatim() {
    // A valid, in-range, on-step value is not rewritten (no rule fires).
    let mut s = raw_state(FormControlKind::Range, "40.0");
    sanitize_value(&mut s);
    assert_eq!(s.value(), "40.0");
}

#[test]
fn sanitize_date_invalid_to_empty_valid_kept() {
    // §4.10.5.1.7: not a valid date string → empty; valid kept.
    let mut bad = raw_state(FormControlKind::Date, "2025-13-40");
    sanitize_value(&mut bad);
    assert_eq!(bad.value(), "");
    let mut good = raw_state(FormControlKind::Date, "2025-06-16");
    sanitize_value(&mut good);
    assert_eq!(good.value(), "2025-06-16");
}

#[test]
fn sanitize_time_keeps_valid_verbatim_no_normalization() {
    // §4.10.5.1.10: time (like date/month/week) keeps a VALID string
    // verbatim — it must NOT be normalized (only datetime-local is).
    // A valid time with an explicit zero-seconds / fractional component
    // is kept exactly as authored.
    for v in ["08:00:00", "08:00:00.500", "08:00"] {
        let mut s = raw_state(FormControlKind::Time, v);
        sanitize_value(&mut s);
        assert_eq!(s.value(), v, "valid time {v:?} must be kept verbatim");
    }
    // Invalid → empty.
    let mut bad = raw_state(FormControlKind::Time, "25:99");
    sanitize_value(&mut bad);
    assert_eq!(bad.value(), "");
}

#[test]
fn sanitize_range_invalid_default_then_snaps() {
    // §4.10.5.1.13: an invalid value → default (midpoint), which itself
    // is then subject to the step-mismatch rule.  Spec worked example:
    // min=0 max=100 step=20 value=<invalid> → default 50 → snap → 60.
    let mut s = FormControlState {
        kind: FormControlKind::Range,
        value: "abc".to_string(),
        min: Some("0".to_string()),
        max: Some("100".to_string()),
        step: Some("20".to_string()),
        ..FormControlState::default()
    };
    sanitize_value(&mut s);
    assert_eq!(s.value(), "60");
}

#[test]
fn sanitize_datetime_local_normalizes() {
    // §4.10.5.1.11: a valid local date-time is normalized (the `:00`
    // seconds component is dropped by the canonical serialization).
    let mut s = raw_state(FormControlKind::DatetimeLocal, "2025-06-16T08:00:00");
    sanitize_value(&mut s);
    assert_eq!(s.value(), "2025-06-16T08:00");
}

#[test]
fn sanitize_noop_kinds_untouched() {
    // States with no value sanitization algorithm keep the value as-is.
    for kind in [
        FormControlKind::Hidden,
        FormControlKind::Checkbox,
        FormControlKind::Radio,
        FormControlKind::File,
        FormControlKind::SubmitButton,
        FormControlKind::Color,
    ] {
        let mut s = raw_state(kind, "x\ny 150");
        sanitize_value(&mut s);
        assert_eq!(s.value(), "x\ny 150", "kind {kind:?} must be untouched");
    }
}

#[test]
fn sanitize_resyncs_char_count_and_selection_on_shorten() {
    // G2: a value-shortening sanitize must re-sync char_count and clamp
    // the cursor/selection to the new end (JS-observable via
    // selectionStart / char_count otherwise).
    let mut s = raw_state(FormControlKind::Number, "abc");
    s.cursor_pos = 3;
    s.selection_start = 3;
    s.selection_end = 3;
    sanitize_value(&mut s);
    assert_eq!(s.value(), "");
    assert_eq!(s.char_count(), 0);
    assert_eq!(s.selection_start(), 0);
    assert_eq!(s.selection_end(), 0);
}

#[test]
fn set_value_sanitizes_without_extra_dirty_semantics() {
    // The IDL `value` setter path sanitizes (range clamps) and still
    // marks dirty.
    let mut s = FormControlState {
        kind: FormControlKind::Range,
        ..FormControlState::default()
    };
    s.set_value("150".to_string());
    assert_eq!(s.value(), "100");
    assert!(s.dirty_value);
}

#[test]
fn from_element_parse_sanitizes_input_value() {
    // F1: the struct-literal initial-parse path sanitizes.
    // `<input type=range value=150>` → stored "100".
    let mut attrs = Attributes::default();
    attrs.set("type", "range");
    attrs.set("value", "150");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert_eq!(state.value(), "100");
}

#[test]
fn from_element_parse_sanitizes_email_multiple() {
    // F1 + G1: parse reads `multiple` so the email comma-list algorithm
    // applies at element creation.
    let mut attrs = Attributes::default();
    attrs.set("type", "email");
    attrs.set("multiple", "");
    attrs.set("value", " a@b , c@d ");
    let state = FormControlState::from_element("input", &attrs).unwrap();
    assert_eq!(state.value(), "a@b,c@d");
}

#[test]
fn sanitize_date_keeps_huge_but_valid_year_verbatim() {
    // §4.10.5.1.7: a syntactically valid date is kept VERBATIM even when its
    // millisecond count overflows the internal i64 number space — date
    // value-sanitization validity is syntactic, not numeric.  `year=1e9` is
    // a valid date string but its ms count exceeds i64::MAX.
    let mut s = raw_state(FormControlKind::Date, "1000000000-01-01");
    sanitize_value(&mut s);
    assert_eq!(s.value(), "1000000000-01-01");
}

#[test]
fn sanitize_range_extreme_endpoints_no_infinity() {
    // The default-value midpoint must stay finite even for extreme finite
    // endpoints — `0.5*min + 0.5*max` avoids the `max - min` overflow to
    // infinity that would store an invalid `"inf"`.
    let mut s = FormControlState {
        kind: FormControlKind::Range,
        value: "x".to_string(),
        min: Some("-1e308".to_string()),
        max: Some("1e308".to_string()),
        ..FormControlState::default()
    };
    sanitize_value(&mut s);
    assert_eq!(s.value(), "0");
}

#[test]
fn sanitize_resets_selection_direction_on_value_change() {
    // §4.10.5.4 step 5: when sanitization changes the value, the selection
    // direction resets to "none" (alongside the cursor/selection collapse).
    let mut s = raw_state(FormControlKind::TextInput, "a\nb");
    s.selection_direction = SelectionDirection::Forward;
    sanitize_value(&mut s);
    assert_eq!(s.value(), "ab");
    assert_eq!(s.selection_direction, SelectionDirection::None);
}
