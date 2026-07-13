//! ECMA-262 §21.4.1 Date abstract operations — pure numeric time-value math.
//!
//! Every operation here works on **time values** (milliseconds since the Unix
//! epoch, `1970-01-01T00:00:00Z`) as `f64`, matching the spec's use of Number.
//! Because a valid time value satisfies `abs(t) <= 8.64e15 < 2^53`, all the
//! `floor` / integer arithmetic below is exact in `f64`.
//!
//! **UTC-baseline**: `LocalTime(t)` and `UTC(t)` (§21.4.1.25 / §21.4.1.26) are
//! the identity here — the engine carries no timezone database
//! (`intl-icu-deferral`), so local and UTC components coincide and
//! `getTimezoneOffset()` is always `+0`. Full local-timezone support is a
//! follow-up (`#11-vm-date-local-timezone`).
//!
//! Month/weekday indices are cast `f64 → usize`; every source is a
//! decomposition AO whose range is `0..=11` / `0..=6`, so the sign-loss and
//! truncation lints are provably inapplicable here.
#![allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]

// The canonical §7.1.5 ToIntegerOrInfinity (One issue, one way) — shared with
// natives_string_ext / typed-array / relative-index helpers. It uses `trunc`
// and does NOT normalize `-0`; the Date-specific `-0 → +0` happens once, in
// `time_clip`, the single funnel every time value passes through.
use super::super::coerce::to_integer_or_infinity;

/// §21.4.1.2 — milliseconds per second.
pub(super) const MS_PER_SECOND: f64 = 1000.0;
/// Milliseconds per minute.
pub(super) const MS_PER_MINUTE: f64 = 60_000.0;
/// Milliseconds per hour.
pub(super) const MS_PER_HOUR: f64 = 3_600_000.0;
/// §21.4.1.2 — milliseconds per day.
pub(super) const MS_PER_DAY: f64 = 86_400_000.0;
/// §21.4.1.1 — the maximum magnitude of a valid time value.
pub(super) const MAX_TIME_VALUE: f64 = 8.64e15;

/// Cumulative day-of-year at the first of each month in a non-leap year
/// (January = index 0). March onward gains `InLeapYear(t)` in a leap year.
const MONTH_START_DAY: [f64; 12] = [
    0.0, 31.0, 59.0, 90.0, 120.0, 151.0, 181.0, 212.0, 243.0, 273.0, 304.0, 334.0,
];

// ECMA-262 numeric `x modulo y` (result has the sign of `y`) is exactly
// `f64::rem_euclid` for the positive divisors every Date AO uses — the
// repo-wide idiom (coerce.rs, natives_string.rs). No bespoke `modulo` helper.

/// §21.4.1.3 Day(t) — the day number of the day in which `t` falls.
pub(super) fn day(t: f64) -> f64 {
    (t / MS_PER_DAY).floor()
}

/// §21.4.1.4 TimeWithinDay(t).
pub(super) fn time_within_day(t: f64) -> f64 {
    t.rem_euclid(MS_PER_DAY)
}

/// §21.4.1.5 DaysInYear(y) — 365 or 366.
fn days_in_year(y: f64) -> f64 {
    if y.rem_euclid(4.0) != 0.0 {
        365.0
    } else if y.rem_euclid(100.0) != 0.0 {
        366.0
    } else if y.rem_euclid(400.0) != 0.0 {
        365.0
    } else {
        366.0
    }
}

/// §21.4.1.6 DayFromYear(y) — day number of the first day of year `y`.
fn day_from_year(y: f64) -> f64 {
    365.0 * (y - 1970.0) + ((y - 1969.0) / 4.0).floor() - ((y - 1901.0) / 100.0).floor()
        + ((y - 1601.0) / 400.0).floor()
}

/// §21.4.1.10 InLeapYear(t) — `1` in a leap year, else `0`.
fn in_leap_year(t: f64) -> f64 {
    if days_in_year(year_from_time(t)) == 366.0 {
        1.0
    } else {
        0.0
    }
}

/// §21.4.1.8 YearFromTime(t) — the largest year `y` with
/// `TimeFromYear(y) <= t`, i.e. `DayFromYear(y) <= Day(t)`.
pub(super) fn year_from_time(t: f64) -> f64 {
    let d = day(t);
    // Mean Gregorian year length is 365.2425 days; start close, then correct.
    let mut y = (1970.0 + d / 365.2425).floor();
    if day_from_year(y) > d {
        while day_from_year(y) > d {
            y -= 1.0;
        }
    } else {
        while day_from_year(y + 1.0) <= d {
            y += 1.0;
        }
    }
    y
}

/// §21.4.1.9 DayWithinYear(t).
fn day_within_year(t: f64) -> f64 {
    day(t) - day_from_year(year_from_time(t))
}

/// §21.4.1.11 MonthFromTime(t) — `0` (January) .. `11` (December).
pub(super) fn month_from_time(t: f64) -> f64 {
    // Defensive NaN passthrough (matching `year_from_time`): without this the
    // `d < …` comparisons are all false for NaN and the final `else` would
    // wrongly return 11 (December). Callers currently guard `is_nan` first, so
    // this closes a latent trap rather than a live bug.
    if !t.is_finite() {
        return f64::NAN;
    }
    let d = day_within_year(t);
    let leap = in_leap_year(t);
    if d < 31.0 {
        0.0
    } else if d < 59.0 + leap {
        1.0
    } else if d < 90.0 + leap {
        2.0
    } else if d < 120.0 + leap {
        3.0
    } else if d < 151.0 + leap {
        4.0
    } else if d < 181.0 + leap {
        5.0
    } else if d < 212.0 + leap {
        6.0
    } else if d < 243.0 + leap {
        7.0
    } else if d < 273.0 + leap {
        8.0
    } else if d < 304.0 + leap {
        9.0
    } else if d < 334.0 + leap {
        10.0
    } else {
        11.0
    }
}

/// §21.4.1.12 DateFromTime(t) — day of the month, `1` .. `31`.
pub(super) fn date_from_time(t: f64) -> f64 {
    let d = day_within_year(t);
    let month = month_from_time(t) as usize;
    let leap = in_leap_year(t);
    let offset = MONTH_START_DAY[month] + if month >= 2 { leap } else { 0.0 };
    d - offset + 1.0
}

/// §21.4.1.13 WeekDay(t) — `0` (Sunday) .. `6` (Saturday).
pub(super) fn week_day(t: f64) -> f64 {
    (day(t) + 4.0).rem_euclid(7.0)
}

/// §21.4.1.14 HourFromTime(t).
pub(super) fn hour_from_time(t: f64) -> f64 {
    (t / MS_PER_HOUR).floor().rem_euclid(24.0)
}

/// §21.4.1.15 MinFromTime(t).
pub(super) fn min_from_time(t: f64) -> f64 {
    (t / MS_PER_MINUTE).floor().rem_euclid(60.0)
}

/// §21.4.1.16 SecFromTime(t).
pub(super) fn sec_from_time(t: f64) -> f64 {
    (t / MS_PER_SECOND).floor().rem_euclid(60.0)
}

/// §21.4.1.17 msFromTime(t).
pub(super) fn ms_from_time(t: f64) -> f64 {
    t.rem_euclid(MS_PER_SECOND)
}

/// §21.4.1.27 MakeTime(hour, min, sec, ms). Result is **not** range-clamped —
/// the caller runs it through [`make_date`] + [`time_clip`].
pub(super) fn make_time(hour: f64, min: f64, sec: f64, ms: f64) -> f64 {
    if !hour.is_finite() || !min.is_finite() || !sec.is_finite() || !ms.is_finite() {
        return f64::NAN;
    }
    let h = to_integer_or_infinity(hour);
    let m = to_integer_or_infinity(min);
    let s = to_integer_or_infinity(sec);
    let milli = to_integer_or_infinity(ms);
    h * MS_PER_HOUR + m * MS_PER_MINUTE + s * MS_PER_SECOND + milli
}

/// §21.4.1.28 MakeDay(year, month, date) — a day number, or `NaN`.
pub(super) fn make_day(year: f64, month: f64, date: f64) -> f64 {
    if !year.is_finite() || !month.is_finite() || !date.is_finite() {
        return f64::NAN;
    }
    let y = to_integer_or_infinity(year);
    let m = to_integer_or_infinity(month);
    let dt = to_integer_or_infinity(date);
    let ym = y + (m / 12.0).floor();
    if !ym.is_finite() {
        return f64::NAN;
    }
    let mn = m.rem_euclid(12.0);
    // Day number of `(ym, mn, 1)`: the spec's "find tv such that
    // YearFromTime(tv)=ym, MonthFromTime(tv)=mn, DateFromTime(tv)=1".
    let leap = if days_in_year(ym) == 366.0 { 1.0 } else { 0.0 };
    let month_offset = MONTH_START_DAY[mn as usize] + if mn >= 2.0 { leap } else { 0.0 };
    let tv_day = day_from_year(ym) + month_offset;
    // §21.4.1.28 step 9: Return Day(tv) + date - 1.
    tv_day + dt - 1.0
}

/// §21.4.1.29 MakeDate(day, time).
pub(super) fn make_date(day_num: f64, time: f64) -> f64 {
    // The trailing `is_finite` check is load-bearing (it maps an
    // overflow-to-infinity to NaN so `year_from_time`'s correction loop never
    // spins on it), so both non-finite inputs and non-finite sums funnel
    // through it — no separate leading guard needed.
    let tv = day_num * MS_PER_DAY + time;
    if tv.is_finite() {
        tv
    } else {
        f64::NAN
    }
}

/// §21.4.1.31 TimeClip(time).
pub(super) fn time_clip(time: f64) -> f64 {
    if !time.is_finite() || time.abs() > MAX_TIME_VALUE {
        return f64::NAN;
    }
    // `coerce::to_integer_or_infinity` uses `trunc`, which preserves `-0`;
    // §21.4.1.31 returns `𝔽(!ToIntegerOrInfinity(time))` and
    // ToIntegerOrInfinity(-0.4) is `+0`, so normalize `-0 → +0` here — the one
    // funnel every produced time value passes through.
    let clipped = to_integer_or_infinity(time);
    if clipped == 0.0 {
        0.0
    } else {
        clipped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_decomposition() {
        // 1970-01-01T00:00:00Z, a Thursday.
        assert_eq!(year_from_time(0.0), 1970.0);
        assert_eq!(month_from_time(0.0), 0.0);
        assert_eq!(date_from_time(0.0), 1.0);
        assert_eq!(week_day(0.0), 4.0); // Thursday
        assert_eq!(hour_from_time(0.0), 0.0);
    }

    #[test]
    fn leap_day_2000() {
        // 2000-02-29 is day-from-year(2000)+59.
        let t = make_date(make_day(2000.0, 1.0, 29.0), 0.0);
        assert_eq!(year_from_time(t), 2000.0);
        assert_eq!(month_from_time(t), 1.0);
        assert_eq!(date_from_time(t), 29.0);
    }

    #[test]
    fn month_overflow_rolls_year() {
        // Month 12 (zero-based) rolls to January of the next year.
        let t = make_date(make_day(2020.0, 12.0, 1.0), 0.0);
        assert_eq!(year_from_time(t), 2021.0);
        assert_eq!(month_from_time(t), 0.0);
        assert_eq!(date_from_time(t), 1.0);
    }

    #[test]
    fn negative_time_before_epoch() {
        // 1969-12-31T23:59:59Z = -1000 ms.
        assert_eq!(year_from_time(-1000.0), 1969.0);
        assert_eq!(month_from_time(-1000.0), 11.0);
        assert_eq!(date_from_time(-1000.0), 31.0);
        assert_eq!(hour_from_time(-1000.0), 23.0);
        assert_eq!(sec_from_time(-1000.0), 59.0);
    }

    #[test]
    fn time_clip_range() {
        assert!(time_clip(MAX_TIME_VALUE + 1.0).is_nan());
        assert!(time_clip(-MAX_TIME_VALUE - 1.0).is_nan());
        assert_eq!(time_clip(MAX_TIME_VALUE), MAX_TIME_VALUE);
        assert_eq!(time_clip(1.5), 1.0);
        assert!(time_clip(f64::INFINITY).is_nan());
    }

    #[test]
    fn time_clip_normalizes_negative_zero() {
        // §21.4.1.31 / ToIntegerOrInfinity(-0.4) = +0 (observable via getTime).
        let t = time_clip(-0.4);
        assert_eq!(t, 0.0);
        assert!(t.is_sign_positive(), "time_clip(-0.4) must be +0, not -0");
    }

    #[test]
    fn nan_passthrough() {
        assert!(month_from_time(f64::NAN).is_nan());
        assert!(year_from_time(f64::NAN).is_nan());
        assert!(make_day(f64::NAN, 0.0, 1.0).is_nan());
        assert!(make_time(f64::NAN, 0.0, 0.0, 0.0).is_nan());
    }
}
