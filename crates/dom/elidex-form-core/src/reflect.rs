//! Content-attribute reflection/parse helpers (pattern regex, positive-with-fallback integer reflection), split out to keep `lib.rs` under the 1000-line cap.

use std::sync::Arc;

/// Maximum pattern length to prevent `ReDoS` via excessively long regex patterns.
pub const MAX_PATTERN_LENGTH: usize = 1024;

/// Compile a `pattern` attribute value into an anchored regex.
///
/// Returns `None` if the pattern exceeds [`MAX_PATTERN_LENGTH`] or is not valid regex.
/// Per HTML spec §4.10.5.3.8, invalid patterns are silently ignored.
pub(crate) fn compile_pattern_regex(p: &str) -> Option<Arc<regex::Regex>> {
    if p.len() > MAX_PATTERN_LENGTH {
        return None;
    }
    let anchored = format!("^(?:{p})$");
    regex::RegexBuilder::new(&anchored)
        .size_limit(1 << 20)
        // JS pattern attribute uses the `u` flag (WHATWG HTML §4.10.5.3.8).
        // Rust regex defaults match this: \d/\w are ASCII-only, `.` matches Unicode scalars.
        .build()
        .ok()
        .map(Arc::new)
}

/// Reflect `value` as an HTML non-negative integer under the §2.6.1
/// "limited to only positive numbers with fallback" reflection rule
/// (`#limited-to-only-non-negative-numbers-greater-than-zero-with-fallback`;
/// the anchor slug predates the dfn's rename from "non-negative numbers
/// greater than zero"): the returned value is in `[1, 2147483647]` (HTML
/// §2.6.1 getter steps: minimum 1, maximum 2147483647), otherwise (absent /
/// invalid / `0` / negative / above-maximum / overflow) return `default`.
///
/// Parsing follows HTML "rules for parsing non-negative integers"
/// (§2.3.4.2, `#rules-for-parsing-non-negative-integers`): **skip leading
/// ASCII whitespace, consume an optional leading `+`, take the leading
/// ASCII-digit run, and ignore trailing junk**. This deliberately differs
/// from [`str::parse`] (which requires the WHOLE string to be a valid
/// integer) so that `rows=" 5"` and `rows="5px"` both reflect `5` — the
/// value browsers use — instead of falling back to the default. A leading
/// `-` (or any non-digit first character) leaves an empty digit run → the
/// fallback, matching the non-negative rule that rejects negatives.
///
/// Single-sourced reflection shared by `from_textarea_element` (createElement
/// init), the `rows`/`cols` reconciler arms, and the textarea `rows`/`cols`
/// IDL getters (HTML §4.10.11 `rows`/`cols`, `ReflectPositiveWithFallback`).
#[must_use]
pub fn parse_positive_with_fallback(value: Option<&str>, default: u32) -> u32 {
    let Some(s) = value else { return default };
    // "Rules for parsing non-negative integers" (§2.3.4.2): skip leading
    // ASCII whitespace, drop an optional leading `+`, then take the leading
    // ASCII-digit run (trailing junk is ignored). `char::is_ascii_whitespace`
    // is exactly HTML's ASCII whitespace set (space, \t, \n, \x0C, \r).
    let s = s.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let s = s.strip_prefix('+').unwrap_or(s);
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    // §2.6.1 getter steps 5-6: return the parsed value only when it is in the
    // inclusive range [1, 2147483647] (minimum 1 for "…positive numbers with
    // fallback", maximum 2147483647); otherwise fall back to `default`.
    s[..end]
        .parse::<u32>()
        .ok()
        .filter(|&n| (1..=2_147_483_647).contains(&n))
        .unwrap_or(default)
}
