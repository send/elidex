//! Date string → time value parsing (`Date.parse` / `new Date(string)`).
//!
//! Two tiers, tried in order (ECMA-262 §21.4.3.2):
//! 1. **Date Time String Format** (§21.4.1.32) — the spec-mandated ISO 8601
//!    profile `YYYY-MM-DDTHH:mm:ss.sssZ` incl. date-only and expanded-year
//!    forms. This is complete and authoritative.
//! 2. **Legacy round-trip** — a bounded, best-effort parser for the strings
//!    this engine's own `toString` / `toUTCString` / `toDateString` emit (so
//!    `new Date(d.toString())` reconstructs `d`). General implementation-defined
//!    formats (RFC 2822, locale strings, …) are a follow-up
//!    (`#11-vm-date-parse-nonstandard-formats`).
//!
//! **UTC-baseline**: a date-time form with no timezone is spec-defined as local
//! time; with the identity `LocalTime`/`UTC` (no tz database) it is treated as
//! UTC. An explicit `Z` / `±HH:mm` offset is always honoured.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]

use super::algorithms::{
    date_from_time, make_date, make_day, make_time, month_from_time, time_clip, year_from_time,
    MS_PER_MINUTE,
};

/// `Date.parse(string)` — returns a time value, or `NaN` if unrecognizable.
pub(super) fn parse(s: &str) -> f64 {
    let trimmed = s.trim();
    if let Some(tv) = parse_iso(trimmed) {
        return tv;
    }
    if let Some(tv) = parse_legacy(trimmed) {
        return tv;
    }
    f64::NAN
}

// ---------------------------------------------------------------------------
// Tier 1 — Date Time String Format (§21.4.1.32)
// ---------------------------------------------------------------------------

struct Cursor<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Cursor<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, i: 0 }
    }
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }
    fn eof(&self) -> bool {
        self.i >= self.b.len()
    }
    fn eat(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    /// Read exactly `count` ASCII digits as a non-negative integer.
    fn digits(&mut self, count: usize) -> Option<i64> {
        let mut v = 0i64;
        for _ in 0..count {
            let c = self.peek()?;
            if !c.is_ascii_digit() {
                return None;
            }
            v = v * 10 + i64::from(c - b'0');
            self.i += 1;
        }
        Some(v)
    }
}

fn parse_iso(s: &str) -> Option<f64> {
    let mut c = Cursor::new(s.as_bytes());

    // Year: 4 digits, or an expanded `±YYYYYY`.
    let year = match c.peek() {
        Some(b'+') => {
            c.i += 1;
            c.digits(6)?
        }
        Some(b'-') => {
            c.i += 1;
            let y = c.digits(6)?;
            if y == 0 {
                return None; // "-000000" is explicitly invalid (§21.4.1.32.1).
            }
            -y
        }
        _ => c.digits(4)?,
    };

    let mut month = 1i64;
    let mut day = 1i64;
    if c.eat(b'-') {
        month = c.digits(2)?;
        if c.eat(b'-') {
            day = c.digits(2)?;
        }
    }

    let mut hour = 0i64;
    let mut min = 0i64;
    let mut sec = 0i64;
    let mut ms = 0i64;
    // `None` = no offset in the string (date-only → UTC; date-time → local,
    // which is UTC under the UTC-baseline). `Some(mins)` = explicit offset.
    let mut offset_min: Option<i64> = None;

    if c.eat(b'T') {
        hour = c.digits(2)?;
        if !c.eat(b':') {
            return None;
        }
        min = c.digits(2)?;
        if c.eat(b':') {
            sec = c.digits(2)?;
            if c.eat(b'.') {
                ms = c.digits(3)?;
            }
        }
        if c.eat(b'Z') {
            offset_min = Some(0);
        } else if matches!(c.peek(), Some(b'+' | b'-')) {
            let sign = if c.peek() == Some(b'-') { -1 } else { 1 };
            c.i += 1;
            let oh = c.digits(2)?;
            if !c.eat(b':') {
                return None;
            }
            let om = c.digits(2)?;
            if oh > 23 || om > 59 {
                return None;
            }
            offset_min = Some(sign * (oh * 60 + om));
        }
    }

    if !c.eof() {
        return None; // trailing garbage
    }

    // Field range validation (§21.4.1.32: out-of-bounds → invalid). Hour 24 is
    // permitted only as the "end of day" midnight (min = sec = ms = 0, Note 1).
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    if hour > 24 || (hour == 24 && (min != 0 || sec != 0 || ms != 0)) {
        return None;
    }
    if min > 59 || sec > 59 {
        return None;
    }

    build_time_value(
        year,
        month,
        day,
        hour,
        min,
        sec,
        ms,
        offset_min.unwrap_or(0),
    )
}

/// Assemble a UTC time value from calendar/clock fields already known to be in
/// range, rejecting non-existent calendar dates (e.g. `2021-02-30`) and
/// out-of-range expanded years. `offset_min` is subtracted to move the
/// parsed local instant to UTC.
#[allow(clippy::too_many_arguments)]
fn build_time_value(
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    min: i64,
    sec: i64,
    ms: i64,
    offset_min: i64,
) -> Option<f64> {
    let day_num = make_day(year as f64, (month - 1) as f64, day as f64);
    // Reject calendar dates that rolled over (Feb 30, Apr 31, …): the day-only
    // instant must round-trip to the same y/m/d. `hour == 24` legitimately
    // spills into the next day, so validate on the date alone (time = 0).
    let date_only = make_date(day_num, 0.0);
    if year_from_time(date_only) != year as f64
        || month_from_time(date_only) != (month - 1) as f64
        || date_from_time(date_only) != day as f64
    {
        return None;
    }
    let time = make_time(hour as f64, min as f64, sec as f64, ms as f64);
    let tv = make_date(day_num, time) - (offset_min as f64) * MS_PER_MINUTE;
    let clipped = time_clip(tv);
    if clipped.is_nan() {
        None
    } else {
        Some(clipped)
    }
}

// ---------------------------------------------------------------------------
// Tier 2 — legacy round-trip (this engine's own `toString` family)
// ---------------------------------------------------------------------------

fn month_index(tok: &str) -> Option<i64> {
    let names = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    if tok.len() < 3 {
        return None;
    }
    let key = tok[..3].to_ascii_lowercase();
    names.iter().position(|&m| m == key).map(|i| i as i64)
}

fn is_weekday(tok: &str) -> bool {
    let names = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];
    tok.len() >= 3 && names.contains(&tok[..3].to_ascii_lowercase().as_str())
}

/// Parse an offset token: `"GMT"`, `"UTC"`, `"Z"`, `"GMT+0900"`, `"+0000"`,
/// `"-0800"` → minutes east of UTC.
fn parse_offset_token(tok: &str) -> Option<i64> {
    let (had_prefix, rest) = if let Some(r) = tok.strip_prefix("GMT") {
        (true, r)
    } else if let Some(r) = tok.strip_prefix("UTC") {
        (true, r)
    } else {
        (false, tok)
    };
    if had_prefix && (rest.is_empty() || rest.eq_ignore_ascii_case("Z")) {
        return Some(0);
    }
    let bytes = rest.as_bytes();
    let sign = match bytes.first() {
        Some(b'+') => 1,
        Some(b'-') => -1,
        _ => return None,
    };
    let digits = &rest[1..];
    // Accept "HH:mm" bare, or "HHMM" only behind a GMT/UTC prefix. A bare
    // "±HHMM" is ambiguous with a signed year (`-0001`) and is left to the
    // integer branch to classify.
    let (hh, mm) = if let Some((h, m)) = digits.split_once(':') {
        (h, m)
    } else if had_prefix && digits.len() == 4 {
        (&digits[..2], &digits[2..])
    } else {
        return None;
    };
    let oh: i64 = hh.parse().ok()?;
    let om: i64 = mm.parse().ok()?;
    if oh > 23 || om > 59 {
        return None;
    }
    Some(sign * (oh * 60 + om))
}

fn parse_hms(tok: &str) -> Option<(i64, i64, i64)> {
    let mut it = tok.split(':');
    let h: i64 = it.next()?.parse().ok()?;
    let m: i64 = it.next()?.parse().ok()?;
    let s: i64 = match it.next() {
        Some(sec) => sec.parse().ok()?,
        None => 0,
    };
    if it.next().is_some() || h > 24 || m > 59 || s > 59 {
        return None;
    }
    Some((h, m, s))
}

/// Bounded parser for the shapes this engine emits:
/// - `toString`     — `"Www Mmm DD YYYY HH:mm:ss GMT+0000 (…)"`
/// - `toUTCString`  — `"Www, DD Mmm YYYY HH:mm:ss GMT"`
/// - `toDateString` — `"Www Mmm DD YYYY"`
///
/// Tokenizes on whitespace/commas and classifies each token, so the two
/// day/month orderings both parse. Any parenthesized tz-name tail is ignored.
fn parse_legacy(s: &str) -> Option<f64> {
    let mut month: Option<i64> = None;
    let mut day: Option<i64> = None;
    let mut year: Option<i64> = None;
    let mut hms: Option<(i64, i64, i64)> = None;
    let mut offset_min = 0i64;

    let mut in_paren = false;
    for tok in s.split(|c: char| c.is_whitespace() || c == ',') {
        if tok.is_empty() {
            continue;
        }
        // Skip a "(Coordinated Universal Time)" style trailer entirely.
        if in_paren || tok.starts_with('(') {
            in_paren = !tok.ends_with(')');
            continue;
        }
        if is_weekday(tok) {
            continue;
        }
        if let Some(m) = month_index(tok) {
            month = Some(m);
            continue;
        }
        if tok.contains(':') {
            hms = Some(parse_hms(tok)?);
            continue;
        }
        if let Some(off) = parse_offset_token(tok) {
            offset_min = off;
            continue;
        }
        // Pure integer: a 1-2 digit unsigned value is the day-of-month; any
        // longer or signed value is the year.
        if let Ok(num) = tok.parse::<i64>() {
            if day.is_none() && tok.len() <= 2 && (1..=31).contains(&num) {
                day = Some(num);
            } else {
                year = Some(num);
            }
            continue;
        }
        return None; // unrecognized token
    }

    let (h, mi, s) = hms.unwrap_or((0, 0, 0));
    let (year, month, day) = (year?, month? + 1, day?);
    if !(1..=12).contains(&month) {
        return None;
    }
    build_time_value(year, month, day, h, mi, s, 0, offset_min)
}

#[cfg(test)]
mod tests {
    use super::super::format;
    use super::*;

    #[test]
    fn iso_basic() {
        assert_eq!(parse("1970-01-01T00:00:00.000Z"), 0.0);
        assert_eq!(parse("1970-01-01"), 0.0);
        assert_eq!(parse("1970"), 0.0);
        assert_eq!(parse("2020-01-01T00:00:00Z"), 1_577_836_800_000.0);
    }

    #[test]
    fn iso_timezone_offset() {
        // 2020-01-01T00:00:00+09:00 == 2019-12-31T15:00:00Z.
        assert_eq!(parse("2020-01-01T09:00:00+09:00"), 1_577_836_800_000.0);
        assert_eq!(parse("2020-01-01T00:00:00-01:00"), 1_577_840_400_000.0);
    }

    #[test]
    fn iso_no_tz_is_utc_baseline() {
        // No offset on a date-time form → UTC under the UTC-baseline.
        assert_eq!(parse("2020-01-01T00:00:00"), 1_577_836_800_000.0);
    }

    #[test]
    fn iso_invalid() {
        assert!(parse("2021-02-30").is_nan()); // Feb 30 doesn't exist
        assert!(parse("2021-13-01").is_nan()); // month 13
        assert!(parse("2021-00-01").is_nan()); // month 0
        assert!(parse("not a date").is_nan());
        assert!(parse("2021-01-01T25:00:00").is_nan()); // hour 25
        assert!(parse("-000000-01-01").is_nan()); // -000000 invalid
    }

    #[test]
    fn hour_24_is_end_of_day() {
        // 1995-02-04T24:00 == 1995-02-05T00:00.
        assert_eq!(parse("1995-02-04T24:00Z"), parse("1995-02-05T00:00Z"));
    }

    #[test]
    fn legacy_round_trip() {
        // Second-aligned instant (toString drops ms).
        let t = 1_577_836_800_000.0;
        assert_eq!(parse(&format::to_string(t)), t);
        assert_eq!(parse(&format::utc_string(t)), t);
        // toDateString → midnight of that day.
        assert_eq!(parse(&format::to_date_string(t)), t);
    }

    #[test]
    fn legacy_round_trip_negative_year() {
        let t = super::make_date(super::make_day(-1.0, 5.0, 15.0), 0.0);
        assert_eq!(parse(&format::to_string(t)), t);
    }
}
