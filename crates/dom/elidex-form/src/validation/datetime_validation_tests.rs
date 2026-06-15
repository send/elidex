//! Date/time `<input>` constraint-validation tests (split out of
//! `tests.rs` to keep each file under the 1000-line guideline).

use super::*;

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
        (FormControlKind::Range, "23", Some("0"), Some("10")),
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
fn range_validation_on_unsanitized_value() {
    // Range has no `required`/`badInput`, but `rangeUnderflow`/`Overflow`/
    // `stepMismatch` ARE evaluated on the stored value.  The spec's structural
    // conformance for range relies on the UA clamping/snapping the value
    // (§4.10.5.1.13), which is the deferred value-sanitization slot — so until
    // then validity is computed honestly on the actual value rather than
    // assuming the un-wired clamp (which would report a stored `value=150` as
    // valid).  An in-range, on-grid value is conformant; an out-of-range /
    // off-step stored value surfaces the corresponding bit.
    let range = |value: &str, min: Option<&str>, max: Option<&str>, step: Option<&str>| {
        validate_control(&FormControlState {
            kind: FormControlKind::Range,
            value: value.to_string(),
            min: min.map(str::to_string),
            max: max.map(str::to_string),
            step: step.map(str::to_string),
            ..FormControlState::default()
        })
    };

    // In range and on the default grid (min=0,max=100,step=1) → valid.
    assert!(range("50", None, None, None).is_valid());
    // Above the default max=100 / below default min=0 → overflow / underflow.
    assert!(range("150", None, None, None).range_overflow);
    assert!(range("-20", None, None, None).range_underflow);
    // Above an explicit max → overflow.
    assert!(range("500", Some("0"), Some("100"), None).range_overflow);
    // Off the step grid → step mismatch.
    assert!(range("23", Some("0"), Some("100"), Some("10")).step_mismatch);
    // Range never reports bad input (slider UI cannot describe a non-float),
    // even for a (hypothetically) non-float stored value.
    assert!(!range("not-a-number", None, None, None).bad_input);
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
