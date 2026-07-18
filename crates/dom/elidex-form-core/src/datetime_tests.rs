//! Tests for the date/time microsyntax foundation (`datetime.rs`).
//!
//! Edge-matrix-dense per the coupled-invariant corner: civil round-trip,
//! ISO-week week-year≠calendar-year, leap / month-end, minimal-digit
//! serialization, and per-type convert round-trips.

use super::*;
use crate::FormControlKind::{Date, DatetimeLocal, Month, Time, Week};

// ---- civil core ---------------------------------------------------

#[test]
fn leap_year_rule() {
    assert!(is_leap(2020)); // divisible by 4, not 100
    assert!(is_leap(2000)); // divisible by 400
    assert!(!is_leap(1900)); // divisible by 100, not 400
    assert!(!is_leap(2021));
}

#[test]
fn days_in_month_rule() {
    assert_eq!(days_in_month(2020, 2), 29);
    assert_eq!(days_in_month(2021, 2), 28);
    assert_eq!(days_in_month(2000, 2), 29);
    assert_eq!(days_in_month(1900, 2), 28);
    assert_eq!(days_in_month(2021, 1), 31);
    assert_eq!(days_in_month(2021, 4), 30);
}

#[test]
fn civil_round_trip() {
    // Epoch and various corners round-trip through days↔civil.
    for &(y, m, d) in &[
        (1970, 1, 1),   // epoch
        (1969, 12, 31), // day before epoch (negative)
        (2020, 2, 29),  // leap day
        (2000, 2, 29),  // century-leap day
        (1900, 3, 1),   // day after non-leap Feb
        (1, 1, 1),      // proleptic start-ish
        (9999, 12, 31), // four-digit upper bound
        (2025, 6, 14),  // arbitrary
    ] {
        let z = days_from_civil(y, m, d);
        assert_eq!(civil_from_days(z), (y, m, d), "round-trip {y}-{m}-{d}");
    }
}

#[test]
fn epoch_is_thursday() {
    // 1970-01-01 is a Thursday (weekday index 3, Mon=0).
    assert_eq!(weekday_from_days(0), 3);
}

// ---- ISO week (HTML §2.3.5.8) -------------------------------------

#[test]
fn weeks_in_year_53_vs_52() {
    assert_eq!(weeks_in_week_year(2020), 53); // Jan 1 Wednesday + leap
    assert_eq!(weeks_in_week_year(2015), 53); // Jan 1 Thursday
    assert_eq!(weeks_in_week_year(2021), 52);
    assert_eq!(weeks_in_week_year(2019), 52);
}

#[test]
fn iso_week_year_differs_from_calendar_year() {
    // 2021-01-01 (Friday) belongs to week-year 2020, week 53.
    let (wy, w) = iso_week_from_days(days_from_civil(2021, 1, 1));
    assert_eq!((wy, w), (2020, 53));

    // 2019-12-30 (Monday) belongs to week-year 2020, week 1.
    let (wy, w) = iso_week_from_days(days_from_civil(2019, 12, 30));
    assert_eq!((wy, w), (2020, 1));

    // 2020-01-01 (Wednesday) is in 2020-W01.
    let (wy, w) = iso_week_from_days(days_from_civil(2020, 1, 1));
    assert_eq!((wy, w), (2020, 1));
}

#[test]
fn iso_week_reference_epoch() {
    // HTML §2.3.5.8: Monday 1969-12-29 starts 1970-W01.
    assert_eq!(iso_week_to_days(1970, 1), days_from_civil(1969, 12, 29));
    assert_eq!(iso_week_to_days(1970, 1) * MS_PER_DAY, -259_200_000);
}

#[test]
fn iso_week_to_days_round_trip() {
    for &(wy, w) in &[(1970, 1), (2020, 1), (2020, 53), (2015, 53), (2025, 25)] {
        let days = iso_week_to_days(wy, w);
        assert_eq!(iso_week_from_days(days), (wy, w), "{wy}-W{w}");
    }
}

// ---- parse: valid / invalid enumeration ---------------------------

#[test]
fn date_convert_round_trip() {
    for s in [
        "1970-01-01",
        "2025-01-15",
        "2020-02-29",
        "9999-12-31",
        "0001-01-01",
    ] {
        let n = convert_string_to_number(Date, s).expect(s);
        assert_eq!(
            convert_number_to_string(Date, n).as_deref(),
            Some(s),
            "round-trip {s}"
        );
    }
    // Epoch is exactly 0 ms.
    assert_eq!(convert_string_to_number(Date, "1970-01-01"), Some(0.0));
}

#[test]
fn convert_number_below_year_one_has_no_valid_string() {
    // A valid date/month/week string requires year > 0 (HTML §2.3.5), so
    // stepping below 0001-… has no representable string → None (the step
    // algorithm treats this as a no-op rather than writing "0000-…").
    let before_year_one = convert_string_to_number(Date, "0001-01-01").unwrap() - 86_400_000.0;
    assert_eq!(convert_number_to_string(Date, before_year_one), None);
    let month_before = convert_string_to_number(Month, "0001-01").unwrap() - 1.0;
    assert_eq!(convert_number_to_string(Month, month_before), None);
    let week_before = convert_string_to_number(Week, "0001-W01").unwrap() - 604_800_000.0;
    assert_eq!(convert_number_to_string(Week, week_before), None);
}

#[test]
fn large_years_honored_without_overflow() {
    // Regression (Codex R2/R3): a pathological year must not overflow the
    // civil-date arithmetic (debug panic / release wrap) — but spec-valid
    // years ABOVE the ECMAScript Date range (275760) are still honored
    // (the HTML year production is unbounded).  Only years whose
    // millisecond value genuinely overflows i64 are a conversion error.

    // Accepted: above the JS Date range, and round-trips (f64-exact here).
    let n = convert_string_to_number(Date, "280000-01-01").expect("year > 275760 is valid");
    assert_eq!(
        convert_number_to_string(Date, n).as_deref(),
        Some("280000-01-01")
    );
    assert!(convert_string_to_number(Date, "275761-01-01").is_some());
    assert!(convert_string_to_number(Month, "275761-01").is_some());
    assert!(convert_string_to_number(Week, "300000-W10").is_some());
    assert!(convert_string_to_number(DatetimeLocal, "275761-01-01T00:00").is_some());

    // Rejected (no panic): millisecond value overflows i64, or the year is
    // beyond the civil-arithmetic guard (i64::MAX).
    assert_eq!(convert_string_to_number(Date, "1000000000-01-01"), None); // ms overflows i64
    assert_eq!(
        convert_string_to_number(Date, "9223372036854775807-01-01"),
        None
    ); // i64::MAX year
}

#[test]
fn month_year_above_civil_limit_is_representable() {
    // Codex #349 R2: the month-count space (year − 1970)·12 never uses
    // civil-date arithmetic, so a month string a year above CIVIL_YEAR_LIMIT
    // must parse and round-trip — the civil guard bounds only the date /
    // datetime-local / week paths (which run through `days_from_civil`).
    let s = "100000000001-06"; // CIVIL_YEAR_LIMIT + 1 year
    let n = convert_string_to_number(Month, s).expect("month above civil limit is valid");
    assert_eq!(convert_number_to_string(Month, n).as_deref(), Some(s));
    // The civil-arithmetic paths keep the bound (the same year is rejected).
    assert_eq!(convert_string_to_number(Date, "100000000001-06-15"), None);
    assert_eq!(
        convert_string_to_number(DatetimeLocal, "100000000001-06-15T00:00"),
        None
    );
    assert_eq!(convert_string_to_number(Week, "100000000001-W06"), None);
}

#[test]
fn date_invalid_strings_rejected() {
    for s in [
        "",
        "2025-02-29",  // not a leap year
        "2025-13-01",  // month out of range
        "2025-00-01",  // month zero
        "2025-01-32",  // day out of range
        "2025-01-00",  // day zero
        "0000-01-01",  // year zero
        "2025-1-01",   // one-digit month
        "2025-01-1",   // one-digit day
        "2025/01/01",  // wrong separator
        "2025-01-01x", // trailing garbage
        " 2025-01-01", // leading space
    ] {
        assert_eq!(convert_string_to_number(Date, s), None, "reject {s:?}");
    }
}

#[test]
fn month_convert_round_trip() {
    assert_eq!(convert_string_to_number(Month, "1970-01"), Some(0.0));
    assert_eq!(convert_string_to_number(Month, "1970-02"), Some(1.0));
    assert_eq!(convert_string_to_number(Month, "1969-12"), Some(-1.0));
    for s in ["1970-01", "2025-06", "1969-12", "0001-01", "9999-12"] {
        let n = convert_string_to_number(Month, s).expect(s);
        assert_eq!(
            convert_number_to_string(Month, n).as_deref(),
            Some(s),
            "round-trip {s}"
        );
    }
}

#[test]
fn month_invalid_strings_rejected() {
    for s in ["", "2025-13", "2025-00", "2025-1", "2025-06-01", "2025"] {
        assert_eq!(convert_string_to_number(Month, s), None, "reject {s:?}");
    }
}

#[test]
fn week_convert_round_trip() {
    assert_eq!(
        convert_string_to_number(Week, "1970-W01"),
        Some(-259_200_000.0)
    );
    for s in ["1970-W01", "2020-W53", "2015-W53", "2025-W25", "2021-W01"] {
        let n = convert_string_to_number(Week, s).expect(s);
        assert_eq!(
            convert_number_to_string(Week, n).as_deref(),
            Some(s),
            "round-trip {s}"
        );
    }
}

#[test]
fn week_invalid_strings_rejected() {
    for s in [
        "",
        "2021-W53", // 2021 has only 52 weeks
        "2020-W54", // beyond maxweek 53
        "2025-W00", // week zero
        "2025-w01", // lowercase w
        "2025-W1",  // one-digit week
        "0000-W01", // year zero
        "2025-W01x",
    ] {
        assert_eq!(convert_string_to_number(Week, s), None, "reject {s:?}");
    }
}

#[test]
fn time_convert_round_trip() {
    // Minimal-digit serialization (HTML §2.3.5.4 valid time string).
    let fmt = |ms: f64| convert_number_to_string(Time, ms).unwrap();
    assert_eq!(fmt(0.0), "00:00");
    assert_eq!(fmt(60_000.0), "00:01");
    assert_eq!(fmt(1_000.0), "00:00:01");
    assert_eq!(fmt(500.0), "00:00:00.5");
    assert_eq!(fmt(250.0), "00:00:00.25");
    assert_eq!(fmt(1.0), "00:00:00.001");
    assert_eq!(fmt(10.0), "00:00:00.01");
    assert_eq!(fmt(86_399_999.0), "23:59:59.999");

    for s in [
        "00:00",
        "23:59",
        "12:30",
        "12:30:45",
        "12:30:45.5",
        "12:30:45.123",
    ] {
        let n = convert_string_to_number(Time, s).expect(s);
        assert_eq!(
            convert_number_to_string(Time, n).as_deref(),
            Some(s),
            "round-trip {s}"
        );
    }
}

#[test]
fn time_invalid_strings_rejected() {
    for s in [
        "",
        "24:00",         // hour out of range
        "12:60",         // minute out of range
        "12:30:60",      // second out of range
        "12",            // no minute
        "12:3",          // one-digit minute
        "12:30:5",       // one-digit second
        "12:30:45.",     // empty fraction
        "12:30:45.12.3", // two fraction dots
        "12:30 ",        // trailing space
    ] {
        assert_eq!(convert_string_to_number(Time, s), None, "reject {s:?}");
    }
}

#[test]
fn time_permissive_path_keeps_sub_millisecond_fraction() {
    // HTML §2.3.5.4 parses the seconds component as a full decimal number, so
    // the permissive (min/max attribute) path keeps digits beyond three as a
    // sub-millisecond remainder (Codex #349 R3): `.1239` → 123.9 ms, not 123.
    let n = convert_string_to_number(Time, "12:30:45.1239").expect("4 frac digits valid");
    assert!(
        (n - (45_045_123.0 + 0.9)).abs() < 1e-9,
        "expected 45045123.9 ms, got {n}"
    );
    // `.5009` → 500.9 ms (4th digit carried, not dropped).
    let n = convert_string_to_number(Time, "00:00:00.5009").unwrap();
    assert!((n - 500.9).abs() < 1e-9, "expected 500.9 ms, got {n}");
    // datetime-local carries the sub-ms remainder too (added at the f64
    // boundary, after the checked-i64 day/time combine).
    let n = convert_string_to_number(DatetimeLocal, "1970-01-01T00:00:00.0005").unwrap();
    assert!((n - 0.5).abs() < 1e-9, "expected 0.5 ms, got {n}");
    // The written value is still ms-resolution: convert-a-number-to-a-string
    // serializes at ms granularity (a valid time string is at most 3 digits).
    assert_eq!(
        convert_number_to_string(Time, 45_045_123.9).as_deref(),
        Some("12:30:45.123")
    );
}

#[test]
fn valid_string_path_rejects_over_precision_time_fraction() {
    // Codex #349 R1: the stepUp **value** path parses only a valid time
    // string (≤3 fractional digits), modelling post-value-sanitization
    // state — so an over-precision value is the error/empty case, not a
    // truncated parse (the permissive attr path still accepts it, above).
    for s in ["12:30:45.1234", "00:00:00.5009", "2025-01-15T12:30:45.1234"] {
        let kind = if s.contains('T') { DatetimeLocal } else { Time };
        assert_eq!(
            convert_valid_string_to_number(kind, s),
            None,
            "valid-string path must reject over-precision {s:?}"
        );
        // The permissive ("convert a string to a number") path still parses
        // it — confirms the two paths diverge only on fractional precision.
        assert!(
            convert_string_to_number(kind, s).is_some(),
            "permissive path still accepts {s:?}"
        );
    }
    // ≤3 fractional digits remain valid on both paths (DatetimeLocal too).
    for s in [
        "12:30:45.123",
        "12:30:45.5",
        "12:30",
        "2025-01-15T12:30:45.999",
    ] {
        let kind = if s.contains('T') { DatetimeLocal } else { Time };
        assert!(
            convert_valid_string_to_number(kind, s).is_some(),
            "valid-string path accepts {s:?}"
        );
    }
}

#[test]
fn convert_number_to_string_rejects_out_of_i64_range() {
    // Codex R5: a huge but finite step (e.g. type=date step=1e20) reaches
    // the serializer with a value far outside the i64 ms range; the cast
    // would saturate to i64::MAX and emit an unrelated date.  Reject → None
    // (apply_step then no-ops) rather than fabricate a value.
    for kind in [Date, DatetimeLocal, Time, Week, Month] {
        assert_eq!(convert_number_to_string(kind, 1e20), None, "{kind:?} 1e20");
        assert_eq!(
            convert_number_to_string(kind, -1e20),
            None,
            "{kind:?} -1e20"
        );
        assert_eq!(convert_number_to_string(kind, f64::INFINITY), None);
        assert_eq!(convert_number_to_string(kind, f64::NAN), None);
    }
}

#[test]
fn time_wraps_for_serialization() {
    // No max applies to time; the step algorithm can produce a full-day
    // overshoot, which serializes modulo the day.
    assert_eq!(
        convert_number_to_string(Time, 86_400_000.0).as_deref(),
        Some("00:00")
    );
    assert_eq!(
        convert_number_to_string(Time, -60_000.0).as_deref(),
        Some("23:59")
    );
}

#[test]
fn datetime_local_convert_round_trip() {
    assert_eq!(
        convert_string_to_number(DatetimeLocal, "1970-01-01T00:00"),
        Some(0.0)
    );
    for s in [
        "1970-01-01T00:00",
        "2025-01-15T12:30",
        "2025-01-15T12:30:45",
        "2025-01-15T12:30:45.5",
        "2020-02-29T23:59:59.999",
    ] {
        let n = convert_string_to_number(DatetimeLocal, s).expect(s);
        assert_eq!(
            convert_number_to_string(DatetimeLocal, n).as_deref(),
            Some(s),
            "round-trip {s}"
        );
    }
    // Space separator is accepted on input, normalized to `T` on output.
    let n = convert_string_to_number(DatetimeLocal, "2025-01-15 12:30").unwrap();
    assert_eq!(
        convert_number_to_string(DatetimeLocal, n).as_deref(),
        Some("2025-01-15T12:30")
    );
}

#[test]
fn datetime_local_invalid_strings_rejected() {
    for s in [
        "",
        "2025-01-15",        // no time
        "2025-01-15T",       // empty time
        "2025-02-29T12:00",  // bad date (non-leap)
        "2025-01-15T25:00",  // bad time
        "2025-01-15X12:30",  // wrong separator
        "2025-01-15T12:30x", // trailing garbage
    ] {
        assert_eq!(
            convert_string_to_number(DatetimeLocal, s),
            None,
            "reject {s:?}"
        );
    }
}

#[test]
fn non_date_time_kinds_return_none() {
    assert_eq!(convert_string_to_number(FormControlKind::Number, "5"), None);
    assert!(!is_date_time_kind(FormControlKind::Number));
    assert!(!is_date_time_kind(FormControlKind::Range));
    for k in [Date, Month, Week, Time, DatetimeLocal] {
        assert!(is_date_time_kind(k));
    }
}

// ---- per-type scale / default-step / step-base constants ----------

#[test]
fn step_scale_and_defaults() {
    assert_eq!(step_scale_factor(Date), 86_400_000.0);
    assert_eq!(step_scale_factor(Week), 604_800_000.0);
    assert_eq!(step_scale_factor(Time), 1000.0);
    assert_eq!(step_scale_factor(DatetimeLocal), 1000.0);
    assert_eq!(step_scale_factor(Month), 1.0);

    assert_eq!(type_default_step(Date), 1.0);
    assert_eq!(type_default_step(Month), 1.0);
    assert_eq!(type_default_step(Week), 1.0);
    assert_eq!(type_default_step(Time), 60.0);
    assert_eq!(type_default_step(DatetimeLocal), 60.0);

    assert_eq!(type_default_step_base(Week), -259_200_000.0);
    assert_eq!(type_default_step_base(Date), 0.0);
    assert_eq!(type_default_step_base(Time), 0.0);
}
