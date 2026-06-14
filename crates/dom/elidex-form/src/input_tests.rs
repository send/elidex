use super::*;
use crate::FormControlKind;

fn text_state(value: &str, cursor: usize) -> FormControlState {
    FormControlState {
        kind: FormControlKind::TextInput,
        value: value.to_string(),
        cursor_pos: cursor,
        char_count: value.chars().count(),
        ..FormControlState::default()
    }
}

fn textarea_state(value: &str, cursor: usize) -> FormControlState {
    FormControlState {
        kind: FormControlKind::TextArea,
        value: value.to_string(),
        cursor_pos: cursor,
        char_count: value.chars().count(),
        rows: 2,
        cols: 20,
        ..FormControlState::default()
    }
}

#[test]
fn insert_character() {
    let mut s = text_state("ab", 1);
    assert!(form_control_key_input(&mut s, "x", "KeyX"));
    assert_eq!(s.value, "axb");
    assert_eq!(s.cursor_pos, 2);
}

#[test]
fn insert_at_end() {
    let mut s = text_state("ab", 2);
    assert!(form_control_key_input(&mut s, "c", "KeyC"));
    assert_eq!(s.value, "abc");
    assert_eq!(s.cursor_pos, 3);
}

#[test]
fn backspace_middle() {
    let mut s = text_state("abc", 2);
    assert!(form_control_key_input(&mut s, "Backspace", "Backspace"));
    assert_eq!(s.value, "ac");
    assert_eq!(s.cursor_pos, 1);
}

#[test]
fn backspace_at_start() {
    let mut s = text_state("abc", 0);
    assert!(!form_control_key_input(&mut s, "Backspace", "Backspace"));
    assert_eq!(s.value, "abc");
}

#[test]
fn delete_middle() {
    let mut s = text_state("abc", 1);
    assert!(form_control_key_input(&mut s, "Delete", "Delete"));
    assert_eq!(s.value, "ac");
    assert_eq!(s.cursor_pos, 1);
}

#[test]
fn delete_at_end() {
    let mut s = text_state("abc", 3);
    assert!(!form_control_key_input(&mut s, "Delete", "Delete"));
}

#[test]
fn arrow_left_right() {
    let mut s = text_state("abc", 2);
    assert!(form_control_key_input(&mut s, "ArrowLeft", "ArrowLeft"));
    assert_eq!(s.cursor_pos, 1);
    assert!(form_control_key_input(&mut s, "ArrowRight", "ArrowRight"));
    assert_eq!(s.cursor_pos, 2);
}

#[test]
fn home_end() {
    let mut s = text_state("abc", 1);
    assert!(form_control_key_input(&mut s, "Home", "Home"));
    assert_eq!(s.cursor_pos, 0);
    assert!(form_control_key_input(&mut s, "End", "End"));
    assert_eq!(s.cursor_pos, 3);
}

#[test]
fn enter_in_textarea() {
    let mut s = textarea_state("ab", 1);
    assert!(form_control_key_input(&mut s, "Enter", "Enter"));
    assert_eq!(s.value, "a\nb");
    assert_eq!(s.cursor_pos, 2);
}

#[test]
fn enter_in_text_input_returns_submit() {
    let mut s = text_state("ab", 1);
    // Enter on text input triggers implicit form submission.
    assert_eq!(
        form_control_key_input_action(&mut s, "Enter", "Enter"),
        KeyAction::Submit
    );
    // form_control_key_input returns true (Submit != None).
    let mut s2 = text_state("ab", 1);
    assert!(form_control_key_input(&mut s2, "Enter", "Enter"));
}

#[test]
fn multibyte_character() {
    let mut s = text_state("", 0);
    assert!(form_control_key_input(&mut s, "あ", ""));
    assert_eq!(s.value, "あ");
    assert_eq!(s.cursor_pos, 3); // UTF-8 3 bytes
}

#[test]
fn backspace_multibyte() {
    let mut s = text_state("あい", 3);
    assert!(form_control_key_input(&mut s, "Backspace", "Backspace"));
    assert_eq!(s.value, "い");
    assert_eq!(s.cursor_pos, 0);
}

#[test]
fn cursor_pos_clamped_to_value_len() {
    // cursor_pos beyond value length should be clamped, not panic.
    let mut s = text_state("abc", 100);
    assert!(form_control_key_input(&mut s, "x", "KeyX"));
    assert_eq!(s.value, "abcx");
    assert_eq!(s.cursor_pos, 4);
}

#[test]
fn cursor_pos_clamped_to_char_boundary() {
    // cursor_pos in the middle of a multibyte char should be corrected.
    let mut s = text_state("あい", 1); // byte 1 is not a char boundary
    assert!(form_control_key_input(&mut s, "x", "KeyX"));
    // Should have been clamped to byte 0 (prev char boundary)
    assert_eq!(s.value, "xあい");
    assert_eq!(s.cursor_pos, 1);
}

#[test]
fn readonly_rejects_editing() {
    let mut s = FormControlState {
        value: "abc".to_string(),
        cursor_pos: 1,
        readonly: true,
        ..FormControlState::default()
    };
    // Typing should be rejected.
    assert!(!form_control_key_input(&mut s, "x", "KeyX"));
    assert_eq!(s.value, "abc");
    // Backspace/Delete should be rejected.
    assert!(!form_control_key_input(&mut s, "Backspace", "Backspace"));
    assert_eq!(s.value, "abc");
    assert!(!form_control_key_input(&mut s, "Delete", "Delete"));
    assert_eq!(s.value, "abc");
    // Navigation should still work.
    assert!(form_control_key_input(&mut s, "ArrowRight", "ArrowRight"));
    assert_eq!(s.cursor_pos, 2);
    assert!(form_control_key_input(&mut s, "Home", "Home"));
    assert_eq!(s.cursor_pos, 0);
    assert!(form_control_key_input(&mut s, "End", "End"));
    assert_eq!(s.cursor_pos, 3);
}

#[test]
fn checkbox_ignores_keys() {
    let mut s = FormControlState {
        kind: FormControlKind::Checkbox,
        ..FormControlState::default()
    };
    assert!(!form_control_key_input(&mut s, "a", "KeyA"));
}

#[test]
fn newline_rejected_in_text_input() {
    // HTML spec: single-line inputs reject \n and \r.
    let mut s = text_state("ab", 2);
    // \n is a control character that should be rejected anyway,
    // but we explicitly guard against it.
    assert!(!form_control_key_input(&mut s, "\n", "Enter"));
    assert_eq!(s.value, "ab");
}

#[test]
fn maxlength_blocks_insertion() {
    let mut s = FormControlState {
        kind: FormControlKind::TextInput,
        value: "abcd".to_string(),
        cursor_pos: 4,
        char_count: 4,
        maxlength: Some(4),
        ..FormControlState::default()
    };
    assert!(!form_control_key_input(&mut s, "x", "KeyX"));
    assert_eq!(s.value, "abcd");
}

#[test]
fn number_rejects_letters() {
    let mut s = FormControlState {
        kind: FormControlKind::Number,
        value: "12".to_string(),
        cursor_pos: 2,
        ..FormControlState::default()
    };
    assert!(!form_control_key_input(&mut s, "a", "KeyA"));
    assert_eq!(s.value, "12");
    // Digits should be accepted.
    assert!(form_control_key_input(&mut s, "3", "Digit3"));
    assert_eq!(s.value, "123");
    // Dot/minus/e should be accepted.
    assert!(form_control_key_input(&mut s, ".", "Period"));
    assert_eq!(s.value, "123.");
}

#[test]
fn supports_selection_types() {
    assert!(FormControlKind::TextInput.supports_selection());
    assert!(FormControlKind::Password.supports_selection());
    assert!(FormControlKind::TextArea.supports_selection());
    assert!(!FormControlKind::Checkbox.supports_selection());
    assert!(!FormControlKind::Select.supports_selection());
}

// -- apply_step tests (D-2 hoist target) -----------------------

fn make_state(kind: FormControlKind, value: &str, step: Option<&str>) -> FormControlState {
    make_state_mm(kind, value, step, None, None)
}

/// `make_state` with `min`/`max` content attributes (HTML §4.10.5.3.7).
/// The `value` here is the IDL value (dirty), so `default_value` (the
/// `value` content attribute / step base fallback) stays empty unless a
/// test sets it explicitly via `set_value_initial`.
fn make_state_mm(
    kind: FormControlKind,
    value: &str,
    step: Option<&str>,
    min: Option<&str>,
    max: Option<&str>,
) -> FormControlState {
    let mut s = FormControlState {
        kind,
        ..Default::default()
    };
    s.set_value(value.to_string());
    s.step = step.map(String::from);
    s.min = min.map(String::from);
    s.max = max.map(String::from);
    s
}

#[test]
fn apply_step_number_default_step_one() {
    let mut s = make_state(FormControlKind::Number, "5", None);
    assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
    assert_eq!(s.value(), "6");
}

#[test]
fn apply_step_range_descending() {
    let mut s = make_state(FormControlKind::Range, "10", Some("2"));
    assert!(apply_step(&mut s, 3.0, -1.0).is_ok());
    assert_eq!(s.value(), "4");
}

#[test]
fn apply_step_unsupported_kind_returns_not_supported() {
    let mut s = make_state(FormControlKind::TextInput, "abc", None);
    assert_eq!(apply_step(&mut s, 1.0, 1.0), Err(StepError::NotSupported));
    // Value untouched.
    assert_eq!(s.value(), "abc");
}

#[test]
fn apply_step_invalid_step_falls_back_to_one() {
    let mut s = make_state(FormControlKind::Number, "0", Some("not-a-number"));
    assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
    assert_eq!(s.value(), "1");
}

#[test]
fn apply_step_empty_value_treated_as_zero() {
    let mut s = make_state(FormControlKind::Number, "", Some("2"));
    assert!(apply_step(&mut s, 5.0, 1.0).is_ok());
    assert_eq!(s.value(), "10");
}

#[test]
fn apply_step_fractional_step() {
    let mut s = make_state(FormControlKind::Number, "1", Some("0.5"));
    assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
    // f64 1.5 prints as "1.5" via to_string.
    assert_eq!(s.value(), "1.5");
}

// -- §4.10.5.4 step 1/2: applicability + allowed value step ----

#[test]
fn apply_step_any_returns_no_allowed_value_step() {
    // step="any" (ASCII case-insensitive) → no allowed value step
    // → InvalidStateError (HTML §4.10.5.4 step 2).
    for raw in ["any", "ANY", "Any"] {
        let mut s = make_state(FormControlKind::Number, "5", Some(raw));
        assert_eq!(
            apply_step(&mut s, 1.0, 1.0),
            Err(StepError::NoAllowedValueStep),
            "step={raw:?}"
        );
        // Value untouched on error.
        assert_eq!(s.value(), "5");
    }
}

#[test]
fn apply_step_any_on_range_returns_no_allowed_value_step() {
    let mut s = make_state(FormControlKind::Range, "5", Some("any"));
    assert_eq!(
        apply_step(&mut s, 1.0, 1.0),
        Err(StepError::NoAllowedValueStep)
    );
}

#[test]
fn apply_step_zero_and_negative_step_fall_back_to_default() {
    // step ≤ 0 falls back to the default step (1), NOT "any"
    // (HTML §4.10.5.3.8 rule 4).
    for raw in ["0", "-1", "-2.5"] {
        let mut s = make_state(FormControlKind::Number, "5", Some(raw));
        assert!(apply_step(&mut s, 1.0, 1.0).is_ok(), "step={raw:?}");
        assert_eq!(s.value(), "6", "step={raw:?}");
    }
}

// -- §4.10.5.4 step 7: round-to-step-base snapping -------------

#[test]
fn apply_step_unaligned_snaps_up() {
    // value=5 step=10 base=0: 5 is not on the grid {0,10,20,…};
    // stepUp snaps to the nearest aligned value above (10), not 15.
    let mut s = make_state_mm(FormControlKind::Number, "5", Some("10"), Some("0"), None);
    assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
    assert_eq!(s.value(), "10");
}

#[test]
fn apply_step_unaligned_snaps_down() {
    // Same grid; stepDown snaps to the nearest aligned value below (0).
    let mut s = make_state_mm(FormControlKind::Number, "5", Some("10"), Some("0"), None);
    assert!(apply_step(&mut s, 1.0, -1.0).is_ok());
    assert_eq!(s.value(), "0");
}

// -- §4.10.5.4 step 7 step base sourcing -----------------------

#[test]
fn apply_step_step_base_from_min_attr() {
    // base = min attr (3); grid {3,13,23,…}; value=3 aligned →
    // stepUp adds one step → 13.
    let mut s = make_state_mm(FormControlKind::Number, "3", Some("10"), Some("3"), None);
    assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
    assert_eq!(s.value(), "13");
}

#[test]
fn apply_step_step_base_from_value_content_attr() {
    // No min attr → base = value content attribute (default_value=2);
    // grid {2,12,22,…}; value=2 aligned → stepUp → 12.  If the base
    // were 0 the value would snap instead, so 12 proves the source.
    let mut s = make_state(FormControlKind::Number, "", Some("10"));
    s.set_value_initial("2".to_string());
    assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
    assert_eq!(s.value(), "12");
}

// -- §4.10.5.4 steps 8/9: min/max clamping ---------------------

#[test]
fn apply_step_clamps_up_to_minimum() {
    // base=min=5, grid {5,15,25,…}; value=15 stepDown by 2 → -5,
    // below min → clamp up to the smallest aligned value ≥ 5 (= 5).
    let mut s = make_state_mm(
        FormControlKind::Number,
        "15",
        Some("10"),
        Some("5"),
        Some("100"),
    );
    assert!(apply_step(&mut s, 2.0, -1.0).is_ok());
    assert_eq!(s.value(), "5");
}

#[test]
fn apply_step_clamps_down_to_maximum() {
    // base=0, grid {0,10,20,…}; value=20 stepUp by 10 → 120, above max
    // → clamp down to the largest aligned value ≤ 95 (= 90).  90 is
    // still above the starting value (20), so step 10 does not nullify.
    let mut s = make_state_mm(
        FormControlKind::Number,
        "20",
        Some("10"),
        Some("0"),
        Some("95"),
    );
    assert!(apply_step(&mut s, 10.0, 1.0).is_ok());
    assert_eq!(s.value(), "90");
}

// -- §4.10.5.4 step 3/4/10: no-op returns ----------------------

#[test]
fn apply_step_min_greater_than_max_is_noop() {
    // Step 3: min > max → return, value unchanged.
    let mut s = make_state_mm(
        FormControlKind::Number,
        "5",
        Some("1"),
        Some("10"),
        Some("0"),
    );
    assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
    assert_eq!(s.value(), "5");
}

#[test]
fn apply_step_no_aligned_value_in_range_is_noop() {
    // Step 4: base=0, step=10, min=1, max=9 → no multiple of 10 in
    // [1,9] → return, value unchanged.
    let mut s = make_state_mm(
        FormControlKind::Number,
        "3",
        Some("10"),
        Some("1"),
        Some("9"),
    );
    assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
    assert_eq!(s.value(), "3");
}

#[test]
fn apply_step_reverse_overshoot_is_noop() {
    // Spec example: <input type=number value=1 max=0>.stepUp() does
    // not change the value (step 10 — value would drop below the
    // starting point).
    let mut s = make_state_mm(FormControlKind::Number, "1", None, None, Some("0"));
    assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
    assert_eq!(s.value(), "1");
}

// -- range type defaults (HTML §4.10.5.1.13) -------------------

#[test]
fn apply_step_range_default_max_clamps_to_100() {
    // Range has a default maximum of 100; value=100 stepUp clamps
    // back down to 100 (no min/max attrs present).
    let mut s = make_state(FormControlKind::Range, "100", Some("10"));
    assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
    assert_eq!(s.value(), "100");
}

#[test]
fn apply_step_range_default_min_clamps_to_0() {
    // Range has a default minimum of 0; value=0 stepDown clamps to 0.
    let mut s = make_state(FormControlKind::Range, "0", Some("10"));
    assert!(apply_step(&mut s, 1.0, -1.0).is_ok());
    assert_eq!(s.value(), "0");
}

#[test]
fn apply_step_number_has_no_default_min_max() {
    // Unlike range, number has no default min/max, so a large stepUp
    // is not clamped.
    let mut s = make_state(FormControlKind::Number, "0", Some("10"));
    assert!(apply_step(&mut s, 50.0, 1.0).is_ok());
    assert_eq!(s.value(), "500");
}

#[test]
fn apply_step_large_magnitude_unaligned_still_snaps() {
    // The step-alignment tolerance must stay far below ½ a step even
    // for large ratios: `value=5e8+0.5` on a step-1 grid is off-grid and
    // stepUp must snap to 500000001, not add a full step to 500000001.5.
    // (Regression for an unbounded relative tolerance — Codex PR#344.)
    let mut s = make_state(FormControlKind::Number, "500000000.5", Some("1"));
    assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
    assert_eq!(s.value(), "500000001");
}

#[test]
fn apply_step_very_large_magnitude_unaligned_still_snaps() {
    // The alignment tolerance is capped below ½ a step, so even at a
    // magnitude where the relative term would exceed ½ (here ~1e14, still
    // f64-representable with a 0.5 fractional offset) an off-grid value
    // snaps instead of gaining a full step.  (Codex PR#344 round 3.)
    let mut s = make_state(FormControlKind::Number, "100000000000000.5", Some("1"));
    assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
    assert_eq!(s.value(), "100000000000001");
}

#[test]
fn apply_step_quarter_step_offset_at_large_magnitude_snaps() {
    // The relative tolerance is tight (4 ULP), so even a value only an
    // eighth of a step off the grid at a large magnitude is unaligned
    // and snaps to the next grid point instead of gaining a full step.
    // (Codex PR#344 round 4 — a quarter-step cap alone accepted it.)
    let mut s = make_state(FormControlKind::Number, "100000000000000.125", Some("1"));
    assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
    assert_eq!(s.value(), "100000000000001");
}

#[test]
fn apply_step_invalid_value_string_treated_as_empty() {
    // The number-state value is sanitized to a valid floating-point
    // number or empty (HTML §4.10.5.1.12); a non-valid stored value
    // (e.g. "1e", which the permissive attribute parser would read as 1)
    // must be the error/empty case → 0, so stepUp yields 1, not 2.
    // (Codex PR#344 round 4.)
    let mut s = make_state(FormControlKind::Number, "1e", Some("1"));
    assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
    assert_eq!(s.value(), "1");
}

#[test]
fn apply_step_non_finite_result_is_noop() {
    // f64 overflow guard: a pathologically large step with no maximum
    // makes step×n overflow to infinity; the value must NOT become
    // "inf" — the step is a no-op instead.
    let mut s = make_state(FormControlKind::Number, "0", Some("1e308"));
    assert!(apply_step(&mut s, 10.0, 1.0).is_ok());
    assert_eq!(s.value(), "0");
}

// -- convert-number-to-string boundary (steps 11-12) -----------

#[test]
fn apply_step_fractional_round_trips_cleanly() {
    // Guard against f64 noise leaking into the value string: 0.3 on a
    // 0.1 grid steps up to 0.4 and serializes as "0.4", not
    // "0.4000000000000001".
    let mut s = make_state(FormControlKind::Number, "0.3", Some("0.1"));
    assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
    assert_eq!(s.value(), "0.4");
}

// -- parse_floating_point: HTML float parsing rules ------------

#[test]
fn parse_floating_point_rejects_non_numeric_specials() {
    // Rust's f64::from_str accepts these; the HTML parse rules reject
    // them (the core observable gap behind step="any" mis-handling).
    for raw in ["inf", "-inf", "infinity", "nan", "NaN", "", "   ", "abc"] {
        assert_eq!(parse_floating_point(raw), None, "input={raw:?}");
    }
}

#[test]
fn parse_floating_point_accepts_spec_forms() {
    assert_eq!(parse_floating_point("5"), Some(5.0));
    assert_eq!(parse_floating_point("+3"), Some(3.0));
    assert_eq!(parse_floating_point("-2.5"), Some(-2.5));
    assert_eq!(parse_floating_point(".5"), Some(0.5));
    assert_eq!(parse_floating_point("1."), Some(1.0));
    assert_eq!(parse_floating_point("1e3"), Some(1000.0));
    // Leading whitespace skipped; trailing content ignored.
    assert_eq!(parse_floating_point("  5  "), Some(5.0));
    assert_eq!(parse_floating_point("12abc"), Some(12.0));
    // Dangling exponent: the "e" is not consumed.
    assert_eq!(parse_floating_point("1e"), Some(1.0));
}

#[test]
fn parse_floating_point_rejects_finite_overflow() {
    // A value that rounds to ±2^1024 is an error per the parse rules.
    assert_eq!(parse_floating_point("1e400"), None);
}

#[test]
fn parse_valid_floating_point_strict_production() {
    // Accepts the "valid floating-point number" production.
    assert_eq!(parse_valid_floating_point("5"), Some(5.0));
    assert_eq!(parse_valid_floating_point("-2.5"), Some(-2.5));
    assert_eq!(parse_valid_floating_point(".5"), Some(0.5));
    assert_eq!(parse_valid_floating_point("1.5e3"), Some(1500.0));
    assert_eq!(parse_valid_floating_point("1e-3"), Some(0.001));
    // Rejects everything the permissive parser would over-accept — these
    // are exactly the strings number value sanitization clears to "".
    for raw in [
        "1e", "1.", "+5", " 5", "5 ", "12abc", ".", "", "inf", "nan", "1e400",
    ] {
        assert_eq!(parse_valid_floating_point(raw), None, "input={raw:?}");
    }
}

// -------------------------------------------------------------------
// sanitize_for_type_change (HTML §4.10.5.6)
// -------------------------------------------------------------------

#[test]
fn sanitize_clears_checked_when_leaving_checkbox() {
    let mut s = FormControlState {
        kind: FormControlKind::TextInput,
        checked: true,
        ..FormControlState::default()
    };
    sanitize_for_type_change(&mut s, FormControlKind::Checkbox);
    assert!(!s.checked);
}

#[test]
fn sanitize_clears_indeterminate_when_leaving_checkbox() {
    let mut s = FormControlState {
        kind: FormControlKind::TextInput,
        indeterminate: true,
        ..FormControlState::default()
    };
    sanitize_for_type_change(&mut s, FormControlKind::Checkbox);
    assert!(!s.indeterminate);
}

#[test]
fn sanitize_clears_checked_when_leaving_radio() {
    let mut s = FormControlState {
        kind: FormControlKind::TextInput,
        checked: true,
        ..FormControlState::default()
    };
    sanitize_for_type_change(&mut s, FormControlKind::Radio);
    assert!(!s.checked);
}

#[test]
fn sanitize_keeps_checked_when_staying_checkable() {
    let mut s = FormControlState {
        kind: FormControlKind::Radio,
        checked: true,
        ..FormControlState::default()
    };
    sanitize_for_type_change(&mut s, FormControlKind::Checkbox);
    assert!(s.checked);
}

#[test]
fn sanitize_clears_value_when_entering_number_with_non_numeric() {
    let mut s = FormControlState {
        kind: FormControlKind::Number,
        ..FormControlState::default()
    };
    s.set_value("abc".to_string());
    sanitize_for_type_change(&mut s, FormControlKind::TextInput);
    assert_eq!(s.value(), "");
}

#[test]
fn sanitize_keeps_value_when_entering_number_with_numeric() {
    let mut s = FormControlState {
        kind: FormControlKind::Number,
        ..FormControlState::default()
    };
    s.set_value("3.14".to_string());
    sanitize_for_type_change(&mut s, FormControlKind::TextInput);
    assert_eq!(s.value(), "3.14");
}

#[test]
fn sanitize_no_op_when_kind_unchanged() {
    let mut s = FormControlState {
        kind: FormControlKind::Number,
        ..FormControlState::default()
    };
    s.set_value("not-a-number".to_string());
    sanitize_for_type_change(&mut s, FormControlKind::Number);
    // Same-kind transition: no sanitize runs (caller already had
    // this value, and same-kind means content didn't change).
    assert_eq!(s.value(), "not-a-number");
}

// -----------------------------------------------------------------
// resolve_input_list — HTML §4.10.5.1.16 `<input>.list` IDREF
// -----------------------------------------------------------------

fn input_with_list(dom: &mut EcsDom, list_value: &str) -> Entity {
    let mut attrs = Attributes::default();
    attrs.set("list", list_value);
    dom.create_element("input", attrs)
}

fn datalist_with_id(dom: &mut EcsDom, id_value: &str) -> Entity {
    let mut attrs = Attributes::default();
    attrs.set("id", id_value);
    dom.create_element("datalist", attrs)
}

#[test]
fn resolve_input_list_returns_none_when_no_attribute() {
    let mut dom = EcsDom::new();
    let input = dom.create_element("input", Attributes::default());
    assert_eq!(resolve_input_list(&dom, input), None);
}

#[test]
fn resolve_input_list_returns_none_when_attribute_empty() {
    let mut dom = EcsDom::new();
    let input = input_with_list(&mut dom, "");
    assert_eq!(resolve_input_list(&dom, input), None);
}

#[test]
fn resolve_input_list_returns_none_when_id_does_not_exist() {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    let input = input_with_list(&mut dom, "missing");
    let _ = dom.append_child(container, input);
    let other = datalist_with_id(&mut dom, "other");
    let _ = dom.append_child(container, other);
    assert_eq!(resolve_input_list(&dom, input), None);
}

#[test]
fn resolve_input_list_returns_none_when_target_is_not_datalist() {
    // `<div id="opts">` matches the IDREF but is not a `<datalist>`.
    // Spec wording "of type HTMLDataListElement" rejects non-datalist.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    let input = input_with_list(&mut dom, "opts");
    let _ = dom.append_child(container, input);
    let mut div_attrs = Attributes::default();
    div_attrs.set("id", "opts");
    let div = dom.create_element("div", div_attrs);
    let _ = dom.append_child(container, div);
    assert_eq!(resolve_input_list(&dom, input), None);
}

#[test]
fn resolve_input_list_rejects_foreign_namespace_datalist() {
    // A foreign-namespace (SVG) `<datalist>` matching the IDREF is NOT an
    // `HTMLDataListElement` (HTML §4.10.8), so the `list` attribute does
    // not resolve to it — exercises the `is_html_namespace` guard.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    let input = input_with_list(&mut dom, "opts");
    let _ = dom.append_child(container, input);
    let mut attrs = Attributes::default();
    attrs.set("id", "opts");
    let svg_datalist = dom.create_element_ns("datalist", elidex_ecs::Namespace::Svg, attrs, None);
    let _ = dom.append_child(container, svg_datalist);
    assert_eq!(resolve_input_list(&dom, input), None);
}

#[test]
fn resolve_input_list_returns_datalist_when_id_matches() {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    let input = input_with_list(&mut dom, "opts");
    let _ = dom.append_child(container, input);
    let datalist = datalist_with_id(&mut dom, "opts");
    let _ = dom.append_child(container, datalist);
    assert_eq!(resolve_input_list(&dom, input), Some(datalist));
}

#[test]
fn resolve_input_list_picks_tree_order_first_when_duplicate_id() {
    // Malformed (duplicate ids), but spec says "the first such
    // element ... of type HTMLDataListElement".  Pre-order DFS
    // wins.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    let input = input_with_list(&mut dom, "opts");
    let _ = dom.append_child(container, input);
    let earlier = datalist_with_id(&mut dom, "opts");
    let _ = dom.append_child(container, earlier);
    let later = datalist_with_id(&mut dom, "opts");
    let _ = dom.append_child(container, later);
    assert_eq!(resolve_input_list(&dom, input), Some(earlier));
}

#[test]
fn resolve_input_list_id_match_is_case_sensitive() {
    // HTML §6.13.2: id attribute is case-sensitive.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    let input = input_with_list(&mut dom, "FOO");
    let _ = dom.append_child(container, input);
    let datalist = datalist_with_id(&mut dom, "foo");
    let _ = dom.append_child(container, datalist);
    assert_eq!(resolve_input_list(&dom, input), None);
}

#[test]
fn resolve_input_list_skips_non_datalist_match_then_finds_datalist() {
    // Filter-during-walk lock: a `<div>` with matching id appearing
    // earlier in tree order must NOT poison the lookup — the spec
    // requires the FIRST `<datalist>` match, not the first id match.
    // Mirrors `resolve_label_for_skips_non_labelable_when_id_collision`.
    let mut dom = EcsDom::new();
    let mut div_attrs = Attributes::default();
    div_attrs.set("id", "opts");
    let div_root = dom.create_element("div", div_attrs);
    let input = input_with_list(&mut dom, "opts");
    let _ = dom.append_child(div_root, input);
    let datalist = datalist_with_id(&mut dom, "opts");
    let _ = dom.append_child(div_root, datalist);
    assert_eq!(resolve_input_list(&dom, input), Some(datalist));
}

#[test]
fn resolve_input_list_returns_none_when_input_detached() {
    // Detached input → `find_tree_root` returns the input itself;
    // descendant walk is empty.  Tree-scope contract: no datalist
    // outside the input's tree is ever matched.
    let mut dom = EcsDom::new();
    let input = input_with_list(&mut dom, "opts");
    // Datalist exists in the world but in a different tree.
    let other_root = dom.create_element("div", Attributes::default());
    let datalist = datalist_with_id(&mut dom, "opts");
    let _ = dom.append_child(other_root, datalist);
    assert_eq!(resolve_input_list(&dom, input), None);
}

#[test]
fn resolve_input_list_returns_none_when_input_self_references() {
    // `<input id="x" list="x">` detached.  `find_tree_root` returns
    // the input itself; the explicit root check is gated by the
    // tag-first filter in `matches_datalist_with_id`, so the input
    // (not a `<datalist>`) is rejected.
    let mut dom = EcsDom::new();
    let mut attrs = Attributes::default();
    attrs.set("id", "x");
    attrs.set("list", "x");
    let input = dom.create_element("input", attrs);
    assert_eq!(resolve_input_list(&dom, input), None);
}

#[test]
fn resolve_input_list_does_not_trim_whitespace() {
    // Spec is silent on trimming; Chrome / Firefox both do
    // exact-compare (e.g. `list=" foo "` does NOT match
    // `id="foo"`).
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    let input = input_with_list(&mut dom, " foo ");
    let _ = dom.append_child(container, input);
    let datalist = datalist_with_id(&mut dom, "foo");
    let _ = dom.append_child(container, datalist);
    assert_eq!(resolve_input_list(&dom, input), None);
}

#[test]
fn resolve_input_list_skips_datalist_with_empty_id_then_matches_next() {
    // An earlier `<datalist id="">` does not match `list="opts"`
    // (empty id ≠ "opts"); the walk continues and finds the next
    // datalist with the matching id.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    let input = input_with_list(&mut dom, "opts");
    let _ = dom.append_child(container, input);
    let empty_id = datalist_with_id(&mut dom, "");
    let _ = dom.append_child(container, empty_id);
    let target = datalist_with_id(&mut dom, "opts");
    let _ = dom.append_child(container, target);
    assert_eq!(resolve_input_list(&dom, input), Some(target));
}

/// Helper: build an `<input type=T list="opts">` (attr path — no
/// `FormControlState` attached) and a matching `<datalist id="opts">`
/// in a shared tree; return `(input, datalist)`.
fn input_with_type_and_list_plus_datalist(dom: &mut EcsDom, input_type: &str) -> (Entity, Entity) {
    let container = dom.create_element("div", Attributes::default());
    let mut input_attrs = Attributes::default();
    input_attrs.set("type", input_type);
    input_attrs.set("list", "opts");
    let input = dom.create_element("input", input_attrs);
    let _ = dom.append_child(container, input);
    let datalist = datalist_with_id(dom, "opts");
    let _ = dom.append_child(container, datalist);
    (input, datalist)
}

#[test]
fn resolve_input_list_returns_none_for_hidden_type() {
    // HTML §4.10.5.1.16: `list` does not apply to `<input type=hidden>`.
    let mut dom = EcsDom::new();
    let (input, _datalist) = input_with_type_and_list_plus_datalist(&mut dom, "hidden");
    assert_eq!(resolve_input_list(&dom, input), None);
}

#[test]
fn resolve_input_list_returns_none_for_checkbox_type() {
    let mut dom = EcsDom::new();
    let (input, _datalist) = input_with_type_and_list_plus_datalist(&mut dom, "checkbox");
    assert_eq!(resolve_input_list(&dom, input), None);
}

#[test]
fn resolve_input_list_returns_none_for_radio_type() {
    let mut dom = EcsDom::new();
    let (input, _datalist) = input_with_type_and_list_plus_datalist(&mut dom, "radio");
    assert_eq!(resolve_input_list(&dom, input), None);
}

#[test]
fn resolve_input_list_returns_none_for_file_type() {
    let mut dom = EcsDom::new();
    let (input, _datalist) = input_with_type_and_list_plus_datalist(&mut dom, "file");
    assert_eq!(resolve_input_list(&dom, input), None);
}

#[test]
fn resolve_input_list_returns_none_for_password_type() {
    let mut dom = EcsDom::new();
    let (input, _datalist) = input_with_type_and_list_plus_datalist(&mut dom, "password");
    assert_eq!(resolve_input_list(&dom, input), None);
}

#[test]
fn resolve_input_list_returns_none_for_button_types() {
    // image is grouped with the button-typed exclusions per spec
    // ("button"-state inputs in the HTML §4.10.5 type-state table).
    for ty in ["submit", "reset", "button", "image"] {
        let mut dom = EcsDom::new();
        let (input, _datalist) = input_with_type_and_list_plus_datalist(&mut dom, ty);
        assert_eq!(
            resolve_input_list(&dom, input),
            None,
            "list should not apply to <input type={ty}>"
        );
    }
}

#[test]
fn resolve_input_list_applies_to_search_url_email_tel() {
    // Allow-list spot check: types that are NOT TextInput-fallback
    // but DO carry their own variants resolve correctly.
    for ty in [
        "search", "url", "email", "tel", "number", "date", "color", "range",
    ] {
        let mut dom = EcsDom::new();
        let (input, datalist) = input_with_type_and_list_plus_datalist(&mut dom, ty);
        assert_eq!(
            resolve_input_list(&dom, input),
            Some(datalist),
            "list should apply to <input type={ty}>"
        );
    }
}

#[test]
fn resolve_input_list_ignores_stale_form_control_state_when_attr_excludes() {
    // Stale-state guard: the IDL accessor must reflect the current
    // `type` content attribute (HTML §4.10.5.1.16) — a cached
    // `FormControlState.kind` that disagrees with the current
    // attribute must not override.  This locks against the
    // Copilot R2 regression where preferring `state.kind` over
    // the attribute let a fresh `setAttribute('type', 'hidden')`
    // mutation incorrectly resolve a datalist.
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    let mut input_attrs = Attributes::default();
    input_attrs.set("type", "hidden");
    input_attrs.set("list", "opts");
    let input = dom.create_element("input", input_attrs.clone());
    // Attach a state whose `kind` DISAGREES with the attribute
    // (TextInput would be the createElement default before any
    // type-change sync) to prove the attribute wins.
    let stale_attrs = Attributes::default();
    let stale_state = FormControlState::from_element("input", &stale_attrs).unwrap();
    assert_eq!(stale_state.kind, FormControlKind::TextInput);
    let _ = dom.world_mut().insert_one(input, stale_state);
    let _ = dom.append_child(container, input);
    let datalist = datalist_with_id(&mut dom, "opts");
    let _ = dom.append_child(container, datalist);
    assert_eq!(resolve_input_list(&dom, input), None);
}

#[test]
fn resolve_input_list_returns_none_for_image_type() {
    // R2 regression: `image` is excluded per HTML §4.10.5.1.16 even
    // though `FormControlKind::from_type_str("image")` falls back
    // to `TextInput` (pre-existing FormControlKind coverage gap).
    // The attribute-direct check correctly excludes it.
    let mut dom = EcsDom::new();
    let (input, _datalist) = input_with_type_and_list_plus_datalist(&mut dom, "image");
    assert_eq!(resolve_input_list(&dom, input), None);
}

#[test]
fn resolve_input_list_type_match_is_case_insensitive_via_attr() {
    // HTML §3.2.6.5: enumerated attributes are ASCII case-insensitive.
    // `<input type="HIDDEN" list="opts">` should still be excluded.
    let mut dom = EcsDom::new();
    let (input, _datalist) = input_with_type_and_list_plus_datalist(&mut dom, "HIDDEN");
    assert_eq!(resolve_input_list(&dom, input), None);
}
