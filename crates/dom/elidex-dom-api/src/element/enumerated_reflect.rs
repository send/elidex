//! HTML §"reflect" enumerated-attribute canonical-value mapping
//! (slot `#11-tags-T2a-url-bearing`).
//!
//! WHATWG HTML §2.3.5 ("enumerated attributes") defines per-attribute
//! tables where the IDL getter returns:
//!
//! 1. The exact canonical keyword for any case-insensitive ASCII
//!    match of the content attribute against a known keyword.
//! 2. The **invalid-value default** when the attribute is present but
//!    the value matches no keyword.
//! 3. The **missing-value default** when the attribute is absent
//!    (passed as `None`).
//!
//! This module hosts the shared canonicalisation routine plus the
//! per-IDL-attribute tables consumed by HTMLImageElement /
//! HTMLAnchorElement / HTMLAreaElement / HTMLScriptElement /
//! HTMLLinkElement getters in the T2a slot, and by `<area>.shape`.
//!
//! ## Layering
//!
//! Engine-independent.  Tables (`*_VALUES`, `*_MISSING_DEFAULT`,
//! `*_INVALID_DEFAULT` consts) and the canonicalisation algorithm
//! ([`canonicalize_enumerated_attr`]) live here; VM `host/` consumes
//! them through getter helpers (`get_enumerated_reflect` /
//! `get_enumerated_reflect_nullable` in
//! `vm/host/html_hyperlink_mixin.rs`) and **does not** route through
//! a dom-api registry handler — the algorithm is pure data + a `match`
//! against the keyword table, so the binding inlines the call rather
//! than going through `invoke_dom_api`.  Per CLAUDE.md "Layering
//! mandate" the algorithm itself stays in this engine-independent
//! crate.

/// Canonicalise an enumerated-attribute raw value against a known keyword
/// table.  Performs ASCII case-insensitive match per HTML §2.3.5.
///
/// - `raw = None` → returns `missing_default`
/// - `raw = Some(value)` matching any `known` (ASCII-CI) → returns
///   the canonical keyword
/// - otherwise → returns `invalid_default`
///
/// All defaults and keywords are `&'static str` so the cost is a
/// pointer copy except for the borrowed return.
pub fn canonicalize_enumerated_attr(
    raw: Option<&str>,
    known: &[&'static str],
    missing_default: &'static str,
    invalid_default: &'static str,
) -> &'static str {
    let Some(value) = raw else {
        return missing_default;
    };
    for &candidate in known {
        if value.eq_ignore_ascii_case(candidate) {
            return candidate;
        }
    }
    invalid_default
}

// ---------------------------------------------------------------------------
// Canonical-value tables (HTML §"reflect" enumerated-attribute set)
// ---------------------------------------------------------------------------

/// `referrerPolicy` IDL attribute (HTML §6.6.5 referrer policy).
///
/// Missing default = `""` (empty), invalid default = `""` (empty).
/// Used by HTMLAnchorElement / HTMLAreaElement / HTMLImageElement /
/// HTMLScriptElement / HTMLLinkElement / HTMLIFrameElement.
pub const REFERRER_POLICY_VALUES: &[&str] = &[
    "",
    "no-referrer",
    "no-referrer-when-downgrade",
    "origin",
    "origin-when-cross-origin",
    "same-origin",
    "strict-origin",
    "strict-origin-when-cross-origin",
    "unsafe-url",
];
pub const REFERRER_POLICY_MISSING_DEFAULT: &str = "";
pub const REFERRER_POLICY_INVALID_DEFAULT: &str = "";

/// `crossOrigin` IDL attribute (HTML §3.2.7.5 CORS settings).
///
/// IDL type `DOMString?` (nullable).  The prototype-side binding
/// goes through `get_enumerated_reflect_nullable` (in
/// `vm/host/html_hyperlink_mixin.rs`) which returns `JsValue::Null`
/// directly when the content attribute is absent, so
/// `CROSS_ORIGIN_MISSING_DEFAULT` is **not consumed by the binding**;
/// it is retained only for the engine-independent canonicalisation
/// table machinery + the unit tests in this module.  Invalid default
/// = `"anonymous"` per HTML.
pub const CROSS_ORIGIN_VALUES: &[&str] = &["", "anonymous", "use-credentials"];
pub const CROSS_ORIGIN_MISSING_DEFAULT: &str = "";
pub const CROSS_ORIGIN_INVALID_DEFAULT: &str = "anonymous";

/// `loading` IDL attribute (HTML §4.8.4 lazy loading).
///
/// Missing default = `"eager"`, invalid default = `"eager"`.
pub const LOADING_VALUES: &[&str] = &["eager", "lazy"];
pub const LOADING_MISSING_DEFAULT: &str = "eager";
pub const LOADING_INVALID_DEFAULT: &str = "eager";

/// `decoding` IDL attribute (HTML §4.8.4 image decoding hint).
///
/// Missing default = `"auto"`, invalid default = `"auto"`.
pub const DECODING_VALUES: &[&str] = &["sync", "async", "auto"];
pub const DECODING_MISSING_DEFAULT: &str = "auto";
pub const DECODING_INVALID_DEFAULT: &str = "auto";

/// `fetchpriority` IDL attribute (HTML §4.8.4 / Fetch Priority API).
///
/// Missing default = `"auto"`, invalid default = `"auto"`.
pub const FETCH_PRIORITY_VALUES: &[&str] = &["high", "low", "auto"];
pub const FETCH_PRIORITY_MISSING_DEFAULT: &str = "auto";
pub const FETCH_PRIORITY_INVALID_DEFAULT: &str = "auto";

/// `<area>.shape` IDL attribute (HTML §4.6.6 image map area shape).
///
/// Missing default = `"rect"`, invalid default = `"rect"`.
pub const AREA_SHAPE_VALUES: &[&str] = &["circle", "default", "poly", "rect"];
pub const AREA_SHAPE_MISSING_DEFAULT: &str = "rect";
pub const AREA_SHAPE_INVALID_DEFAULT: &str = "rect";

// ---------------------------------------------------------------------------
// "Limited to only known values" — case-sensitive variant
// ---------------------------------------------------------------------------

/// Canonicalise per HTML §6.5.5 "limited to only known values" — used
/// by enumerated reflects whose canonical keywords differ only in
/// ASCII case (so the regular ASCII-CI [`canonicalize_enumerated_attr`]
/// would collapse them).
///
/// Per HTML §6.5.5 the IDL getter for a "limited to only known values"
/// reflect:
/// 1. Reads the content attribute value.
/// 2. If the value is not in canonical form (here: an exact
///    case-sensitive match against one of `known`), returns the empty
///    string.
/// 3. Otherwise returns the matching canonical keyword.
///
/// `raw = None` → `""` (no value to canonicalise).
///
/// Distinct from [`canonicalize_enumerated_attr`] which performs ASCII
/// case-insensitive matching and returns separate missing/invalid
/// defaults.  `<ol>.type` is the only T2b consumer (keywords `1` /
/// `a` / `A` / `i` / `I` — `a` and `A` are intentionally distinct).
pub fn canonicalize_limited_to_known_values(
    raw: Option<&str>,
    known: &[&'static str],
) -> &'static str {
    let Some(value) = raw else {
        return "";
    };
    for &candidate in known {
        if value == candidate {
            return candidate;
        }
    }
    ""
}

/// `<ol>.type` IDL attribute (HTML §4.4.5 ordered-list type).
///
/// Keywords are case-sensitive (`a` and `A` are distinct, etc.) so
/// canonicalisation must go through
/// [`canonicalize_limited_to_known_values`] (not the ASCII-CI
/// [`canonicalize_enumerated_attr`]).  Missing / invalid → `""`
/// per the "limited to only known values" pattern.
pub const OL_TYPE_VALUES: &[&str] = &["1", "a", "A", "i", "I"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn referrer_policy_canonicalises_match() {
        assert_eq!(
            canonicalize_enumerated_attr(
                Some("origin"),
                REFERRER_POLICY_VALUES,
                REFERRER_POLICY_MISSING_DEFAULT,
                REFERRER_POLICY_INVALID_DEFAULT
            ),
            "origin"
        );
    }

    #[test]
    fn referrer_policy_ascii_ci_match() {
        assert_eq!(
            canonicalize_enumerated_attr(
                Some("ORIGIN"),
                REFERRER_POLICY_VALUES,
                REFERRER_POLICY_MISSING_DEFAULT,
                REFERRER_POLICY_INVALID_DEFAULT
            ),
            "origin"
        );
    }

    #[test]
    fn referrer_policy_invalid_returns_default() {
        assert_eq!(
            canonicalize_enumerated_attr(
                Some("bogus"),
                REFERRER_POLICY_VALUES,
                REFERRER_POLICY_MISSING_DEFAULT,
                REFERRER_POLICY_INVALID_DEFAULT
            ),
            ""
        );
    }

    #[test]
    fn referrer_policy_missing_returns_default() {
        assert_eq!(
            canonicalize_enumerated_attr(
                None,
                REFERRER_POLICY_VALUES,
                REFERRER_POLICY_MISSING_DEFAULT,
                REFERRER_POLICY_INVALID_DEFAULT
            ),
            ""
        );
    }

    #[test]
    fn area_shape_invalid_default_rect() {
        assert_eq!(
            canonicalize_enumerated_attr(
                Some("triangle"),
                AREA_SHAPE_VALUES,
                AREA_SHAPE_MISSING_DEFAULT,
                AREA_SHAPE_INVALID_DEFAULT
            ),
            "rect"
        );
    }

    #[test]
    fn area_shape_missing_default_rect() {
        assert_eq!(
            canonicalize_enumerated_attr(
                None,
                AREA_SHAPE_VALUES,
                AREA_SHAPE_MISSING_DEFAULT,
                AREA_SHAPE_INVALID_DEFAULT
            ),
            "rect"
        );
    }

    #[test]
    fn area_shape_canonicalises() {
        assert_eq!(
            canonicalize_enumerated_attr(
                Some("CIRCLE"),
                AREA_SHAPE_VALUES,
                AREA_SHAPE_MISSING_DEFAULT,
                AREA_SHAPE_INVALID_DEFAULT
            ),
            "circle"
        );
    }

    #[test]
    fn loading_invalid_default_eager() {
        assert_eq!(
            canonicalize_enumerated_attr(
                Some("bogus"),
                LOADING_VALUES,
                LOADING_MISSING_DEFAULT,
                LOADING_INVALID_DEFAULT
            ),
            "eager"
        );
    }

    #[test]
    fn cross_origin_invalid_default_anonymous() {
        assert_eq!(
            canonicalize_enumerated_attr(
                Some("bogus"),
                CROSS_ORIGIN_VALUES,
                CROSS_ORIGIN_MISSING_DEFAULT,
                CROSS_ORIGIN_INVALID_DEFAULT
            ),
            "anonymous"
        );
    }

    #[test]
    fn cross_origin_missing_default_empty() {
        assert_eq!(
            canonicalize_enumerated_attr(
                None,
                CROSS_ORIGIN_VALUES,
                CROSS_ORIGIN_MISSING_DEFAULT,
                CROSS_ORIGIN_INVALID_DEFAULT
            ),
            ""
        );
    }

    #[test]
    fn fetch_priority_canonicalises() {
        assert_eq!(
            canonicalize_enumerated_attr(
                Some("HIGH"),
                FETCH_PRIORITY_VALUES,
                FETCH_PRIORITY_MISSING_DEFAULT,
                FETCH_PRIORITY_INVALID_DEFAULT
            ),
            "high"
        );
    }

    #[test]
    fn decoding_missing_default_auto() {
        assert_eq!(
            canonicalize_enumerated_attr(
                None,
                DECODING_VALUES,
                DECODING_MISSING_DEFAULT,
                DECODING_INVALID_DEFAULT
            ),
            "auto"
        );
    }

    // -- canonicalize_limited_to_known_values (case-sensitive) --------------

    #[test]
    fn ol_type_distinct_lowercase_uppercase() {
        assert_eq!(
            canonicalize_limited_to_known_values(Some("a"), OL_TYPE_VALUES),
            "a"
        );
        assert_eq!(
            canonicalize_limited_to_known_values(Some("A"), OL_TYPE_VALUES),
            "A"
        );
        assert_eq!(
            canonicalize_limited_to_known_values(Some("i"), OL_TYPE_VALUES),
            "i"
        );
        assert_eq!(
            canonicalize_limited_to_known_values(Some("I"), OL_TYPE_VALUES),
            "I"
        );
        assert_eq!(
            canonicalize_limited_to_known_values(Some("1"), OL_TYPE_VALUES),
            "1"
        );
    }

    #[test]
    fn ol_type_invalid_returns_empty() {
        assert_eq!(
            canonicalize_limited_to_known_values(Some("X"), OL_TYPE_VALUES),
            ""
        );
        assert_eq!(
            canonicalize_limited_to_known_values(Some(""), OL_TYPE_VALUES),
            ""
        );
    }

    #[test]
    fn ol_type_missing_returns_empty() {
        assert_eq!(
            canonicalize_limited_to_known_values(None, OL_TYPE_VALUES),
            ""
        );
    }

    #[test]
    fn ol_type_case_sensitive_no_collapse() {
        // "B" doesn't match "a"/"A"/"1"/"i"/"I" — the case-insensitive
        // canonicalize_enumerated_attr would treat "A" as matching "a"
        // (collapsing the two), but the limited-to-known-values variant
        // must keep them distinct.
        assert_eq!(
            canonicalize_limited_to_known_values(Some("B"), OL_TYPE_VALUES),
            ""
        );
    }
}
