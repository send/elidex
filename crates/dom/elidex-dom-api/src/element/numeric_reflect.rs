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
    fn non_digit_returns_zero() {
        assert_eq!(parse_unsigned_long("garbage"), 0);
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
}
