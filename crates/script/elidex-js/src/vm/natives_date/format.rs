//! Time value → String formatting (ECMA-262 §21.4.4 `toString` family +
//! `toISOString` / `toUTCString`). **UTC-baseline**: `LocalTime` is the
//! identity and the timezone offset is always `+0000` (`getTimezoneOffset`
//! returns `+0`); see [`super::algorithms`].
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use super::algorithms::{
    date_from_time, hour_from_time, min_from_time, month_from_time, ms_from_time, sec_from_time,
    week_day, year_from_time,
};

/// Table 61 — day-of-week names, `WeekDay(t)` = 0 (Sunday) .. 6 (Saturday).
const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
/// Table 62 — month names, `MonthFromTime(t)` = 0 (January) .. 11 (December).
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `(yearSign, abs(year))` for the zero-padded-to-4 rendering shared by
/// `DateString` (§21.4.4.41.2 steps 5-6) and `toUTCString`.
fn year_parts(t: f64) -> (&'static str, i64) {
    let yv = year_from_time(t) as i64;
    if yv >= 0 {
        ("", yv)
    } else {
        ("-", -yv)
    }
}

/// §21.4.4.41.2 DateString(t) — `"Www Mmm DD YYYY"`.
fn date_string(t: f64) -> String {
    let weekday = WEEKDAYS[week_day(t) as usize];
    let month = MONTHS[month_from_time(t) as usize];
    let day = date_from_time(t) as i64;
    let (sign, ay) = year_parts(t);
    format!("{weekday} {month} {day:02} {sign}{ay:04}")
}

/// §21.4.4.41.1 TimeString(t) — `"HH:mm:ss GMT"`.
fn time_string(t: f64) -> String {
    format!(
        "{:02}:{:02}:{:02} GMT",
        hour_from_time(t) as i64,
        min_from_time(t) as i64,
        sec_from_time(t) as i64
    )
}

/// §21.4.4.41.3 TimeZoneString(t) — UTC-baseline: always `"+0000
/// (Coordinated Universal Time)"`.
fn time_zone_string() -> &'static str {
    "+0000 (Coordinated Universal Time)"
}

/// §21.4.4.41.4 ToDateString(t) — `Date.prototype.toString`.
/// `"Www Mmm DD YYYY HH:mm:ss GMT+0000 (Coordinated Universal Time)"`.
pub(super) fn to_string(t: f64) -> String {
    if t.is_nan() {
        return "Invalid Date".to_string();
    }
    // DateString ⌴ TimeString TimeZoneString — TimeString ends "GMT",
    // TimeZoneString begins "+0000", so no space between them.
    format!(
        "{} {}{}",
        date_string(t),
        time_string(t),
        time_zone_string()
    )
}

/// §21.4.4.35 `Date.prototype.toDateString` — `"Www Mmm DD YYYY"`.
pub(super) fn to_date_string(t: f64) -> String {
    if t.is_nan() {
        return "Invalid Date".to_string();
    }
    date_string(t)
}

/// §21.4.4.42 `Date.prototype.toTimeString` —
/// `"HH:mm:ss GMT+0000 (Coordinated Universal Time)"`.
pub(super) fn to_time_string(t: f64) -> String {
    if t.is_nan() {
        return "Invalid Date".to_string();
    }
    format!("{}{}", time_string(t), time_zone_string())
}

/// §21.4.4.36 `Date.prototype.toISOString` body —
/// `"YYYY-MM-DDTHH:mm:ss.sssZ"` (expanded year `±YYYYYY` outside 0000-9999).
/// Caller guarantees `t` is finite (throws `RangeError` otherwise).
pub(super) fn iso_string(t: f64) -> String {
    let y = year_from_time(t) as i64;
    let year = if (0..=9999).contains(&y) {
        format!("{y:04}")
    } else if y > 9999 {
        format!("+{y:06}")
    } else {
        format!("-{:06}", -y)
    };
    format!(
        "{year}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        month_from_time(t) as i64 + 1,
        date_from_time(t) as i64,
        hour_from_time(t) as i64,
        min_from_time(t) as i64,
        sec_from_time(t) as i64,
        ms_from_time(t) as i64,
    )
}

/// §21.4.4.43 `Date.prototype.toUTCString` —
/// `"Www, DD Mmm YYYY HH:mm:ss GMT"`.
pub(super) fn utc_string(t: f64) -> String {
    if t.is_nan() {
        return "Invalid Date".to_string();
    }
    let weekday = WEEKDAYS[week_day(t) as usize];
    let month = MONTHS[month_from_time(t) as usize];
    let day = date_from_time(t) as i64;
    let (sign, ay) = year_parts(t);
    format!(
        "{weekday}, {day:02} {month} {sign}{ay:04} {:02}:{:02}:{:02} GMT",
        hour_from_time(t) as i64,
        min_from_time(t) as i64,
        sec_from_time(t) as i64,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_renderings() {
        assert_eq!(
            to_string(0.0),
            "Thu Jan 01 1970 00:00:00 GMT+0000 (Coordinated Universal Time)"
        );
        assert_eq!(iso_string(0.0), "1970-01-01T00:00:00.000Z");
        assert_eq!(utc_string(0.0), "Thu, 01 Jan 1970 00:00:00 GMT");
        assert_eq!(to_date_string(0.0), "Thu Jan 01 1970");
        assert_eq!(
            to_time_string(0.0),
            "00:00:00 GMT+0000 (Coordinated Universal Time)"
        );
    }

    #[test]
    fn invalid_date_renderings() {
        assert_eq!(to_string(f64::NAN), "Invalid Date");
        assert_eq!(to_date_string(f64::NAN), "Invalid Date");
        assert_eq!(utc_string(f64::NAN), "Invalid Date");
    }

    #[test]
    fn expanded_year() {
        // Year 275760 (near the max time value) → +275760.
        let t = 8_640_000_000_000_000.0;
        assert!(iso_string(t).starts_with("+275760-"));
        // Negative expanded year.
        let neg = -8_640_000_000_000_000.0;
        assert!(iso_string(neg).starts_with("-271821-"));
    }
}
