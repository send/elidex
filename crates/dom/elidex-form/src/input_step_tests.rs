//! Tests for the HTML §4.10.5.4 stepUp/stepDown algorithm and the
//! floating-point parsers it uses (split from `input_tests.rs` to keep
//! both test files under the 1000-line convention).

use super::*;
use crate::FormControlKind;

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
fn apply_step_aligned_value_with_min_and_small_step_cancellation() {
    // Catastrophic cancellation in `value - base` must NOT make an
    // aligned value look off-grid: `min=4 step=0.1 value=4.1` is on the
    // grid, so stepUp ADVANCES one step (~4.2) — a ratio-only tolerance
    // wrongly treated it as unaligned and snapped back to 4.1 (no-op).
    // The serialized string carries f64 noise (`4.1 + 0.1` is not exactly
    // 4.2); exact decimal output is the deferred number-to-string concern
    // (slot `#11-input-number-decimal-precision`), so assert the numeric
    // advance, not the exact string.
    let mut s = make_state_mm(FormControlKind::Number, "4.1", Some("0.1"), Some("4"), None);
    assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
    let got: f64 = s.value().parse().unwrap();
    assert!((got - 4.2).abs() < 1e-9, "expected ~4.2, got {}", s.value());
}

#[test]
fn apply_step_aligned_value_with_large_base_small_step() {
    // Worse cancellation (~5500 ULP): `min=16 step=0.001 value=16.001`
    // is aligned, so stepUp advances ~one step to ~16.002 (not a no-op).
    let mut s = make_state_mm(
        FormControlKind::Number,
        "16.001",
        Some("0.001"),
        Some("16"),
        None,
    );
    assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
    let got: f64 = s.value().parse().unwrap();
    assert!(
        (got - 16.002).abs() < 1e-9,
        "expected ~16.002, got {}",
        s.value()
    );
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
fn apply_step_clamps_to_on_grid_max_under_float_noise() {
    // The max clamp (step 9) must round to a `max` that is on the grid
    // even when its f64 ratio computes as `0.9999…`: `min=0.2 max=0.3
    // step=0.1`, a stepUp overshooting max clamps to 0.3, not 0.2 (a raw
    // `floor()` under-clamped a whole step).  (Codex PR#344 round 6.)
    let mut s = make_state_mm(
        FormControlKind::Number,
        "0.2",
        Some("0.1"),
        Some("0.2"),
        Some("0.3"),
    );
    assert!(apply_step(&mut s, 5.0, 1.0).is_ok());
    let got: f64 = s.value().parse().unwrap();
    assert!((got - 0.3).abs() < 1e-9, "expected ~0.3, got {}", s.value());
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
