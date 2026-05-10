//! HTML §"reflect" non-negative-integer parser
//! (slot `#11-tags-T2a-url-bearing`).
//!
//! WHATWG HTML §2.4.4.2 ("Rules for parsing non-negative integers")
//! specifies the algorithm used by IDL `unsigned long` reflect
//! attributes — most notably `<img>.width` / `<img>.height`.  The
//! algorithm is:
//!
//! 1. Skip ASCII whitespace.
//! 2. Optional leading `+`.
//! 3. Take the maximal *leading* run of ASCII digits.  Trailing
//!    non-digit garbage (e.g. `"100px"` → `100`) is ignored — it does
//!    not fail the parse, matching browser behaviour for legacy CSS
//!    pixel-suffixed reflect attributes.
//! 4. On overflow, saturate at `u32::MAX`.
//! 5. If no digits were collected (empty after trim, leading non-digit
//!    such as `-` or a letter), return `0` — the IDL default for
//!    unsigned-long reflects.
//!
//! ## Layering
//!
//! Engine-independent.  Used by `<img>.width` / `<img>.height` getters
//! in the T2a slot; setters serialise via `u32::to_string()` and write
//! back through `EcsDom::set_attribute`.

/// Parse a content-attribute string per HTML's "rules for parsing
/// non-negative integers" (HTML §2.4.4.2).  Takes the maximal leading
/// run of ASCII digits (so `"100px"` → `100`); returns `0` when no
/// digits are present, and saturates at `u32::MAX` on overflow.
pub fn parse_unsigned_long(raw: &str) -> u32 {
    // Skip leading ASCII whitespace.
    let trimmed = raw.trim_start_matches(|c: char| c.is_ascii_whitespace());
    // Optional leading `+`.
    let after_sign = trimmed.strip_prefix('+').unwrap_or(trimmed);
    // Collect leading digits.
    let digit_end = after_sign
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit())
        .map_or(after_sign.len(), |(idx, _)| idx);
    let digits = &after_sign[..digit_end];
    if digits.is_empty() {
        return 0;
    }
    // u32 saturating parse.
    digits.parse::<u64>().map_or(u32::MAX, |n| {
        if n > u64::from(u32::MAX) {
            u32::MAX
        } else {
            #[allow(clippy::cast_possible_truncation)]
            // Bounds-checked above: n ≤ u32::MAX.
            (n as u32)
        }
    })
}

/// `i32::MAX` as `u64` — bound for the positive-magnitude saturation
/// in [`parse_long_or_default`] and the i32 cast guard in
/// [`js_number_to_i32_saturating`].
const I32_MAX_AS_U64: u64 = i32::MAX as u64;

/// Absolute value of `i32::MIN` as `u64` (`2^31`).  Distinct from
/// [`I32_MAX_AS_U64`] (`2^31 - 1`) by exactly one.
const I32_MIN_MAGNITUDE: u64 = I32_MAX_AS_U64 + 1;

/// Parse a content-attribute value per HTML's "rules for parsing
/// integers" (HTML §2.4.4.1) — the signed-`long` IDL counterpart to
/// [`parse_unsigned_long`].  Used by `<ol>.start` (default `1`) and
/// `<li>.value` (default `0`); both reflect a `long` IDL attribute
/// that is implementation-clamped to the i32 range.
///
/// Algorithm:
/// 1. `raw = None` → return `default` (missing-default for the IDL
///    `long` reflect).
/// 2. Skip ASCII whitespace.
/// 3. Optional leading sign (`+` or `-`).
/// 4. Take the maximal *leading* run of ASCII digits.  Trailing
///    non-digit garbage is ignored (matching browser behaviour for
///    `"100px"` style values, and consistent with `parse_unsigned_long`).
/// 5. If no digits were collected, return `default`.
/// 6. On overflow (`> i32::MAX` or `< i32::MIN`) saturate at the
///    matching i32 bound (HTML §6.5.5 "limited to only non-negative
///    numbers" applies to the unsigned-long variant; for the plain
///    `long` reflect we mirror Chromium / Firefox saturation).
pub fn parse_long_or_default(raw: Option<&str>, default: i32) -> i32 {
    let Some(input) = raw else {
        return default;
    };
    let trimmed = input.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let (negative, after_sign) = if let Some(rest) = trimmed.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = trimmed.strip_prefix('+') {
        (false, rest)
    } else {
        (false, trimmed)
    };
    let digit_end = after_sign
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit())
        .map_or(after_sign.len(), |(idx, _)| idx);
    let digits = &after_sign[..digit_end];
    if digits.is_empty() {
        return default;
    }
    // Parse magnitude as u64 to allow detecting overflow against
    // both i32 bounds without sign-flip-on-i32::MIN UB.
    let Ok(magnitude) = digits.parse::<u64>() else {
        // Magnitude exceeds u64::MAX — saturate to the matching
        // i32 bound.
        return if negative { i32::MIN } else { i32::MAX };
    };
    if negative {
        if magnitude >= I32_MIN_MAGNITUDE {
            i32::MIN
        } else {
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            // Bounds-checked above: magnitude < 2^31, so the i64 → i32
            // cast is lossless and the negation does not overflow i32.
            -(magnitude as i64 as i32)
        }
    } else if magnitude > I32_MAX_AS_U64 {
        i32::MAX
    } else {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        // Bounds-checked above: magnitude ≤ i32::MAX.
        (magnitude as i32)
    }
}

/// Convert an `f64` (the natural ECMAScript Number IDL form) to an
/// i32 with browser-compatible saturation semantics: NaN / ±Inf
/// collapse to 0; values outside the i32 range saturate to the
/// matching bound.  Used by VM-side IDL `long` setters that need to
/// round-trip a JS Number through the serialiser/parser pair so the
/// stored content-attribute value matches what
/// [`parse_long_or_default`] would parse back.
///
/// Distinct from a strict ECMAScript ToInt32 (which wraps modulo
/// 2^32); this saturating variant matches Chromium / Firefox
/// observable behaviour for `<ol>.start = 1e20` (= `i32::MAX`).
#[allow(clippy::cast_possible_truncation)]
pub fn js_number_to_i32_saturating(n: f64) -> i32 {
    if n.is_nan() || n.is_infinite() {
        return 0;
    }
    if n >= f64::from(i32::MAX) {
        i32::MAX
    } else if n <= f64::from(i32::MIN) {
        i32::MIN
    } else {
        // Bounds-checked above: f64 lossless to i32 in this range.
        n as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple() {
        assert_eq!(parse_unsigned_long("100"), 100);
    }

    #[test]
    fn skips_leading_whitespace() {
        assert_eq!(parse_unsigned_long("  100"), 100);
        assert_eq!(parse_unsigned_long("\t100"), 100);
        assert_eq!(parse_unsigned_long("\n100"), 100);
    }

    #[test]
    fn allows_leading_plus() {
        assert_eq!(parse_unsigned_long("+100"), 100);
        assert_eq!(parse_unsigned_long("  +100"), 100);
    }

    #[test]
    fn rejects_leading_minus() {
        assert_eq!(parse_unsigned_long("-100"), 0);
    }

    #[test]
    fn empty_returns_zero() {
        assert_eq!(parse_unsigned_long(""), 0);
        assert_eq!(parse_unsigned_long("   "), 0);
    }

    #[test]
    fn leading_non_digit_returns_zero() {
        assert_eq!(parse_unsigned_long("garbage"), 0);
    }

    #[test]
    fn trailing_non_digit_ignored() {
        assert_eq!(parse_unsigned_long("100px"), 100);
    }

    #[test]
    fn overflow_saturates() {
        assert_eq!(parse_unsigned_long("99999999999999"), u32::MAX);
    }

    #[test]
    fn zero() {
        assert_eq!(parse_unsigned_long("0"), 0);
    }

    #[test]
    fn trailing_whitespace_kept_as_terminator() {
        // Trailing whitespace is not a digit, so digit collection
        // stops there; leading digits parse fine.
        assert_eq!(parse_unsigned_long("100   "), 100);
    }

    // -- parse_long_or_default ----------------------------------------------

    #[test]
    fn long_missing_returns_default() {
        assert_eq!(parse_long_or_default(None, 1), 1);
        assert_eq!(parse_long_or_default(None, 0), 0);
    }

    #[test]
    fn long_simple_positive() {
        assert_eq!(parse_long_or_default(Some("42"), 1), 42);
    }

    #[test]
    fn long_simple_negative() {
        assert_eq!(parse_long_or_default(Some("-42"), 1), -42);
    }

    #[test]
    fn long_leading_plus() {
        assert_eq!(parse_long_or_default(Some("+42"), 1), 42);
    }

    #[test]
    fn long_skips_leading_whitespace() {
        assert_eq!(parse_long_or_default(Some("  -7"), 0), -7);
    }

    #[test]
    fn long_invalid_returns_default() {
        assert_eq!(parse_long_or_default(Some("garbage"), 1), 1);
        assert_eq!(parse_long_or_default(Some(""), 1), 1);
    }

    #[test]
    fn long_trailing_garbage_ignored() {
        assert_eq!(parse_long_or_default(Some("100px"), 1), 100);
    }

    #[test]
    fn long_overflow_saturates_positive() {
        assert_eq!(parse_long_or_default(Some("99999999999999"), 1), i32::MAX);
    }

    #[test]
    fn long_overflow_saturates_negative() {
        assert_eq!(parse_long_or_default(Some("-99999999999999"), 1), i32::MIN);
    }

    #[test]
    fn long_i32_min_boundary() {
        // i32::MIN = -2147483648; the magnitude is 2^31 which is one
        // beyond i32::MAX as u64.  Saturate path triggers here.
        assert_eq!(parse_long_or_default(Some("-2147483648"), 1), i32::MIN);
    }

    #[test]
    fn long_i32_max_boundary() {
        assert_eq!(parse_long_or_default(Some("2147483647"), 1), i32::MAX);
    }

    // -- js_number_to_i32_saturating ----------------------------------------

    #[test]
    fn number_to_i32_nan_zero() {
        assert_eq!(js_number_to_i32_saturating(f64::NAN), 0);
    }

    #[test]
    fn number_to_i32_inf_collapses_to_zero() {
        // ECMAScript ToInt32 spec: NaN / ±Inf → 0.  Distinct from
        // saturation, which only triggers for finite values outside
        // the i32 range (see `number_to_i32_above_max_saturates`).
        assert_eq!(js_number_to_i32_saturating(f64::INFINITY), 0);
        assert_eq!(js_number_to_i32_saturating(f64::NEG_INFINITY), 0);
    }

    #[test]
    fn number_to_i32_in_range() {
        assert_eq!(js_number_to_i32_saturating(42.0), 42);
        assert_eq!(js_number_to_i32_saturating(-42.0), -42);
        assert_eq!(js_number_to_i32_saturating(0.0), 0);
    }

    #[test]
    fn number_to_i32_above_max_saturates() {
        assert_eq!(js_number_to_i32_saturating(1e20), i32::MAX);
    }

    #[test]
    fn number_to_i32_below_min_saturates() {
        assert_eq!(js_number_to_i32_saturating(-1e20), i32::MIN);
    }
}
