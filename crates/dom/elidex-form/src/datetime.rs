//! Date/time microsyntax foundation (WHATWG HTML §2.3.5) and per-type
//! number conversion for the date/time `<input>` states
//! (HTML §4.10.5.1.7–.11).
//!
//! This is the single canonical date/time conversion layer that every
//! date-type form-control algorithm depends on — `stepUp()`/`stepDown()`
//! (the first consumer, via [`crate::input::apply_step`]) plus the
//! deferred consumers value-sanitization (date types under the existing
//! `#11-input-type-sanitize-extended` slot), `valueAsNumber`/`valueAsDate`
//! (`#11-input-value-as-date`), and constraint validation
//! (`#11-input-date-validity`).  No stepping-only
//! shim: parsers/serializers are written once here and reused, never
//! embedded in the step path.
//!
//! ## Number spaces (per HTML §4.10.5.1.x)
//!
//! * date / week / datetime-local: **milliseconds** since
//!   1970-01-01T00:00:00Z (week anchored at the Monday of the parsed week).
//! * time: **milliseconds** since midnight (time-of-day).
//! * month: **month count** since January 1970 (no millisecond scaling).
//!
//! All arithmetic is performed in `i64` and cast to `f64` only at the
//! boundary; realistic values (year ≈ ±285 000) stay below 2⁵³ so the
//! cast is exact.  Out-of-range / non-Gregorian inputs are rejected as a
//! parse error rather than silently coerced.

use crate::FormControlKind;

/// Milliseconds in one day (the date / week / datetime-local step scale).
const MS_PER_DAY: i64 = 86_400_000;
/// Milliseconds in one week (the week step scale).
const MS_PER_WEEK: i64 = 604_800_000;
/// Milliseconds in one second (the time / datetime-local step scale).
const MS_PER_SECOND: i64 = 1_000;

/// Largest representable year.  The date/time number spaces convert to a
/// `Date` object (`valueAsDate`), whose range is the ECMAScript maximum
/// time value ±8.64×10¹⁵ ms, i.e. up to 275760-09-13 UTC.  Years beyond
/// this are rejected as a parse / conversion error rather than overflowing
/// the `i64` civil-date arithmetic (a huge year like `9223372036854775807`
/// would otherwise wrap or panic in `days_from_civil`).  This also keeps
/// every reachable ms value below 2⁵³, so the cast to `f64` stays exact.
const MAX_YEAR: i64 = 275_760;

// ===================================================================
// Civil-date core (proleptic Gregorian, Howard Hinnant algorithms)
// ===================================================================

/// Whether `year` is a leap year in the proleptic Gregorian calendar.
pub(crate) fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Number of days in `month` (1–12) of `year` (HTML "number of days in
/// month month of year year").
pub(crate) fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Days from the civil date `(year, month, day)` to 1970-01-01
/// (negative before the epoch).  Howard Hinnant's `days_from_civil`,
/// exact for the entire `i64` range.  `month ∈ [1,12]`, `day ∈ [1,31]`.
pub(crate) fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let m = i64::from(month);
    let d = i64::from(day);
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// Civil date `(year, month, day)` for a day count since 1970-01-01.
/// Howard Hinnant's `civil_from_days`, the exact inverse of
/// [`days_from_civil`].
pub(crate) fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

/// ISO weekday of a day count, `0 = Monday … 6 = Sunday`.  Day 0
/// (1970-01-01) is a Thursday (`3`).
fn weekday_from_days(z: i64) -> i64 {
    (z + 3).rem_euclid(7)
}

// ===================================================================
// ISO-8601 week date (HTML §2.3.5.8 Weeks)
// ===================================================================

/// Day count of the Monday of week 1 of `week_year`.  Week 1 is the week
/// containing the first Thursday of the Gregorian year (equivalently, the
/// week containing January 4th).
fn iso_week1_monday(week_year: i64) -> i64 {
    let jan4 = days_from_civil(week_year, 1, 4);
    jan4 - weekday_from_days(jan4)
}

/// Number of weeks in `week_year` (52 or 53) — HTML §2.3.5.8: 53 weeks if
/// January 1st is a Thursday, or a Wednesday in a leap year; else 52.
/// This is "maxweek", the week number of the last day of the week-year.
pub(crate) fn weeks_in_week_year(week_year: i64) -> u32 {
    let jan1_wd = weekday_from_days(days_from_civil(week_year, 1, 1));
    if jan1_wd == 3 || (jan1_wd == 2 && is_leap(week_year)) {
        53
    } else {
        52
    }
}

/// The `(week_year, week)` that a day count falls in.  The week-year is
/// the Gregorian year of the Thursday of the day's week.
fn iso_week_from_days(z: i64) -> (i64, u32) {
    let monday_this_week = z - weekday_from_days(z);
    let thursday = monday_this_week + 3;
    let (week_year, _, _) = civil_from_days(thursday);
    let week = (monday_this_week - iso_week1_monday(week_year)) / 7 + 1;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    (week_year, week as u32)
}

/// Day count of the Monday of `(week_year, week)`.
fn iso_week_to_days(week_year: i64, week: u32) -> i64 {
    iso_week1_monday(week_year) + (i64::from(week) - 1) * 7
}

// ===================================================================
// Cursor-based parsing primitives (HTML "collect a sequence of code
// points that are ASCII digits")
// ===================================================================

/// Collect a run of ASCII digits starting at `*pos`, advancing `*pos`.
/// Returns the run as a string slice (possibly empty).
fn collect_digits<'a>(input: &'a [u8], pos: &mut usize) -> &'a [u8] {
    let start = *pos;
    while *pos < input.len() && input[*pos].is_ascii_digit() {
        *pos += 1;
    }
    &input[start..*pos]
}

/// Parse a run of ASCII digits as a non-negative `i64`.  Returns `None`
/// on overflow (realistically unreachable for the bounded fields here).
fn digits_to_i64(d: &[u8]) -> Option<i64> {
    // ASCII digits only, validated by the caller via `collect_digits`.
    std::str::from_utf8(d).ok()?.parse::<i64>().ok()
}

// ===================================================================
// Parsed value types
// ===================================================================

/// A parsed calendar date (proleptic Gregorian).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CivilDate {
    year: i64,
    month: u32,
    day: u32,
}

/// A parsed time-of-day, held as milliseconds since midnight `[0, 86_400_000)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TimeOfDay {
    ms: i64,
}

// ===================================================================
// Microsyntax parsers (HTML §2.3.5)
// ===================================================================

/// HTML "parse a month component" (§2.3.5.1): `YYYY(+)-MM`, `year > 0`,
/// `1 ≤ month ≤ 12`.  Advances `pos`; returns `(year, month)`.
fn parse_month_component(input: &[u8], pos: &mut usize) -> Option<(i64, u32)> {
    let year_digits = collect_digits(input, pos);
    if year_digits.len() < 4 {
        return None;
    }
    let year = digits_to_i64(year_digits)?;
    if !(1..=MAX_YEAR).contains(&year) {
        return None;
    }
    if input.get(*pos) != Some(&b'-') {
        return None;
    }
    *pos += 1;
    let month_digits = collect_digits(input, pos);
    if month_digits.len() != 2 {
        return None;
    }
    let month = digits_to_i64(month_digits)?;
    if !(1..=12).contains(&month) {
        return None;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Some((year, month as u32))
}

/// HTML "parse a date component" (§2.3.5.2): a month component then
/// `-DD` with `1 ≤ day ≤ days_in_month`.
fn parse_date_component(input: &[u8], pos: &mut usize) -> Option<CivilDate> {
    let (year, month) = parse_month_component(input, pos)?;
    let maxday = days_in_month(year, month);
    if input.get(*pos) != Some(&b'-') {
        return None;
    }
    *pos += 1;
    let day_digits = collect_digits(input, pos);
    if day_digits.len() != 2 {
        return None;
    }
    let day = digits_to_i64(day_digits)?;
    if day < 1 || day > i64::from(maxday) {
        return None;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Some(CivilDate {
        year,
        month,
        day: day as u32,
    })
}

/// HTML "parse a time component" (§2.3.5.4): `HH:MM` then optional
/// `:SS[.sss]`, `0 ≤ hour ≤ 23`, `0 ≤ minute ≤ 59`, `0 ≤ second < 60`,
/// 1–3 fractional digits.  Returns milliseconds since midnight.
fn parse_time_component(input: &[u8], pos: &mut usize) -> Option<TimeOfDay> {
    let hour_digits = collect_digits(input, pos);
    if hour_digits.len() != 2 {
        return None;
    }
    let hour = digits_to_i64(hour_digits)?;
    if !(0..=23).contains(&hour) {
        return None;
    }
    if input.get(*pos) != Some(&b':') {
        return None;
    }
    *pos += 1;
    let min_digits = collect_digits(input, pos);
    if min_digits.len() != 2 {
        return None;
    }
    let minute = digits_to_i64(min_digits)?;
    if !(0..=59).contains(&minute) {
        return None;
    }

    let mut second = 0_i64;
    let mut milli = 0_i64;
    if input.get(*pos) == Some(&b':') {
        *pos += 1;
        let sec_digits = collect_digits(input, pos);
        if sec_digits.len() != 2 {
            return None;
        }
        second = digits_to_i64(sec_digits)?;
        if !(0..=59).contains(&second) {
            return None;
        }
        // Optional fraction: U+002E then 1–3 digits.
        if input.get(*pos) == Some(&b'.') {
            *pos += 1;
            let frac_digits = collect_digits(input, pos);
            if frac_digits.is_empty() || frac_digits.len() > 3 {
                return None;
            }
            let frac = digits_to_i64(frac_digits)?;
            // Scale to milliseconds: ".5" → 500, ".05" → 50, ".001" → 1.
            milli = frac * 10_i64.pow(3 - u32::try_from(frac_digits.len()).ok()?);
        }
    }

    Some(TimeOfDay {
        ms: ((hour * 60 + minute) * 60 + second) * MS_PER_SECOND + milli,
    })
}

/// HTML "parse a date string" (§2.3.5.2): a date component consuming the
/// whole string.
fn parse_date(s: &str) -> Option<CivilDate> {
    let input = s.as_bytes();
    let mut pos = 0;
    let date = parse_date_component(input, &mut pos)?;
    (pos == input.len()).then_some(date)
}

/// HTML "parse a month string" (§2.3.5.1).  Returns `(year, month)`.
fn parse_month(s: &str) -> Option<(i64, u32)> {
    let input = s.as_bytes();
    let mut pos = 0;
    let ym = parse_month_component(input, &mut pos)?;
    (pos == input.len()).then_some(ym)
}

/// HTML "parse a week string" (§2.3.5.8): `YYYY(+)-Www`, validated
/// against `maxweek = weeks_in_week_year`.  Returns `(week_year, week)`.
fn parse_week(s: &str) -> Option<(i64, u32)> {
    let input = s.as_bytes();
    let mut pos = 0;
    let year_digits = collect_digits(input, &mut pos);
    if year_digits.len() < 4 {
        return None;
    }
    let week_year = digits_to_i64(year_digits)?;
    if !(1..=MAX_YEAR).contains(&week_year) {
        return None;
    }
    if input.get(pos) != Some(&b'-') {
        return None;
    }
    pos += 1;
    if input.get(pos) != Some(&b'W') {
        return None;
    }
    pos += 1;
    let week_digits = collect_digits(input, &mut pos);
    if week_digits.len() != 2 {
        return None;
    }
    let week = digits_to_i64(week_digits)?;
    let maxweek = i64::from(weeks_in_week_year(week_year));
    if week < 1 || week > maxweek {
        return None;
    }
    if pos != input.len() {
        return None;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Some((week_year, week as u32))
}

/// HTML "parse a time string" (§2.3.5.4).  Returns milliseconds since
/// midnight.
fn parse_time(s: &str) -> Option<TimeOfDay> {
    let input = s.as_bytes();
    let mut pos = 0;
    let time = parse_time_component(input, &mut pos)?;
    (pos == input.len()).then_some(time)
}

/// HTML "parse a local date and time string" (§2.3.5.5): a date
/// component, then `T` or U+0020 SPACE, then a time component.
fn parse_local_date_time(s: &str) -> Option<(CivilDate, TimeOfDay)> {
    let input = s.as_bytes();
    let mut pos = 0;
    let date = parse_date_component(input, &mut pos)?;
    match input.get(pos) {
        Some(&b'T' | &b' ') => pos += 1,
        _ => return None,
    }
    let time = parse_time_component(input, &mut pos)?;
    (pos == input.len()).then_some((date, time))
}

// ===================================================================
// Serializers (HTML "valid X string" productions, minimal forms)
// ===================================================================

/// Format a year as four-or-more ASCII digits (zero-padded to four;
/// years > 9999 use their natural width).  Valid strings always have
/// `year > 0`, so the padding branch is the only reachable one for
/// in-range values.
fn format_year(year: i64) -> String {
    if (0..=9999).contains(&year) {
        format!("{year:04}")
    } else {
        year.to_string()
    }
}

/// Serialize a civil date as a valid date string `YYYY-MM-DD`.
fn format_date(date: CivilDate) -> String {
    format!(
        "{}-{:02}-{:02}",
        format_year(date.year),
        date.month,
        date.day
    )
}

/// Serialize milliseconds-since-midnight as a valid time string: `HH:MM`,
/// extended to `HH:MM:SS` when the seconds component is nonzero and to
/// `HH:MM:SS.sss` (1–3 minimal fractional digits) when the millisecond
/// component is nonzero.
fn format_time(ms: i64) -> String {
    let ms = ms.rem_euclid(MS_PER_DAY);
    let hour = ms / 3_600_000;
    let minute = (ms % 3_600_000) / 60_000;
    let second = (ms % 60_000) / 1000;
    let milli = ms % 1000;
    if second == 0 && milli == 0 {
        format!("{hour:02}:{minute:02}")
    } else if milli == 0 {
        format!("{hour:02}:{minute:02}:{second:02}")
    } else {
        // Minimal fractional digits: 500 → "5", 250 → "25", 1 → "001".
        let frac = format!("{milli:03}");
        format!(
            "{hour:02}:{minute:02}:{second:02}.{}",
            frac.trim_end_matches('0')
        )
    }
}

// ===================================================================
// Per-type number conversion (HTML §4.10.5.1.7–.11)
// ===================================================================

/// HTML "convert a string to a number" for the date/time input states.
/// Returns `None` for the spec's "return an error" outcome (parse
/// failure or a non-date-time `kind`).  The number is milliseconds
/// (months for the Month state); see the module docs for each space.
pub(crate) fn convert_string_to_number(kind: FormControlKind, s: &str) -> Option<f64> {
    #[allow(clippy::cast_precision_loss)]
    match kind {
        FormControlKind::Date => {
            let d = parse_date(s)?;
            Some((days_from_civil(d.year, d.month, d.day) * MS_PER_DAY) as f64)
        }
        FormControlKind::Month => {
            let (year, month) = parse_month(s)?;
            Some(((year - 1970) * 12 + i64::from(month) - 1) as f64)
        }
        FormControlKind::Week => {
            let (wy, week) = parse_week(s)?;
            Some((iso_week_to_days(wy, week) * MS_PER_DAY) as f64)
        }
        FormControlKind::Time => {
            let t = parse_time(s)?;
            Some(t.ms as f64)
        }
        FormControlKind::DatetimeLocal => {
            let (d, t) = parse_local_date_time(s)?;
            Some((days_from_civil(d.year, d.month, d.day) * MS_PER_DAY + t.ms) as f64)
        }
        _ => None,
    }
}

/// HTML "convert a number to a string" for the date/time input states
/// (the inverse of [`convert_string_to_number`]).  Returns the type's
/// valid X string, or `None` when the number lies outside the range for
/// which a valid string exists — a calendar/week year < 1, for which no
/// valid date/month/week/local-date-time string exists (HTML §2.3.5
/// requires `year > 0`) — or for a non-date-time `kind`.  Callers (e.g.
/// [`crate::input::apply_step`]) treat `None` as "no value to set",
/// leaving the control's value unchanged rather than writing a string
/// the parsers would reject.
pub(crate) fn convert_number_to_string(kind: FormControlKind, n: f64) -> Option<String> {
    // The number is an exact integer for every reachable input (integer
    // ms / month-count produced by the step algorithm), so truncating
    // the f64 to i64 is lossless.
    #[allow(clippy::cast_possible_truncation)]
    let n = n as i64;
    match kind {
        FormControlKind::Date => {
            let date: CivilDate = civil_from_days(n.div_euclid(MS_PER_DAY)).into();
            (1..=MAX_YEAR)
                .contains(&date.year)
                .then(|| format_date(date))
        }
        FormControlKind::Month => {
            let year = 1970 + n.div_euclid(12);
            let month = n.rem_euclid(12) + 1;
            (1..=MAX_YEAR)
                .contains(&year)
                .then(|| format!("{}-{:02}", format_year(year), month))
        }
        FormControlKind::Week => {
            let (wy, week) = iso_week_from_days(n.div_euclid(MS_PER_DAY));
            (1..=MAX_YEAR)
                .contains(&wy)
                .then(|| format!("{}-W{:02}", format_year(wy), week))
        }
        FormControlKind::Time => Some(format_time(n)),
        FormControlKind::DatetimeLocal => {
            let date: CivilDate = civil_from_days(n.div_euclid(MS_PER_DAY)).into();
            // Guard the year bound (below) before formatting.
            // `format_time` extracts the time-of-day from the full ms
            // value itself (it normalizes modulo the day), so pass `n`
            // directly — same as the Time arm.
            (1..=MAX_YEAR)
                .contains(&date.year)
                .then(|| format!("{}T{}", format_date(date), format_time(n)))
        }
        _ => None,
    }
}

impl From<(i64, u32, u32)> for CivilDate {
    fn from((year, month, day): (i64, u32, u32)) -> Self {
        Self { year, month, day }
    }
}

/// Whether `kind` is a date/time input state handled by this module
/// (date, month, week, time, datetime-local) — as opposed to the
/// numeric states (number, range) handled by floating-point parsing.
pub(crate) fn is_date_time_kind(kind: FormControlKind) -> bool {
    matches!(
        kind,
        FormControlKind::Date
            | FormControlKind::Month
            | FormControlKind::Week
            | FormControlKind::Time
            | FormControlKind::DatetimeLocal
    )
}

/// HTML "step scale factor" (§4.10.5.1.7–.11): the multiplier from the
/// `step` attribute's unit to the number space.  Date/week/time/
/// datetime-local convert their unit to milliseconds; month uses month
/// counts directly (scale 1).
pub(crate) fn step_scale_factor(kind: FormControlKind) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    match kind {
        FormControlKind::Date => MS_PER_DAY as f64,
        FormControlKind::Week => MS_PER_WEEK as f64,
        FormControlKind::Time | FormControlKind::DatetimeLocal => MS_PER_SECOND as f64,
        // Month (scale 1) and the numeric states.
        _ => 1.0,
    }
}

/// HTML "default step" (§4.10.5.1.7–.11), expressed in the `step`
/// attribute's unit (before scaling): 1 for date/month/week, 60 seconds
/// for time/datetime-local.
pub(crate) fn type_default_step(kind: FormControlKind) -> f64 {
    match kind {
        FormControlKind::Time | FormControlKind::DatetimeLocal => 60.0,
        // Date / Week / Month default step is 1 unit; numeric states 1.
        _ => 1.0,
    }
}

/// HTML "default step base" (§4.10.5.1.9): the Week state anchors its
/// grid at −259 200 000 ms (the Monday starting 1970-W01); all other
/// states default to 0.
pub(crate) fn type_default_step_base(kind: FormControlKind) -> f64 {
    match kind {
        // −259 200 000 ms = the Monday (1969-12-29) that starts 1970-W01
        // (= −3 × MS_PER_DAY).
        FormControlKind::Week => -259_200_000.0,
        _ => 0.0,
    }
}

#[cfg(test)]
#[path = "datetime_tests.rs"]
mod tests;
