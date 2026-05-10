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
//! Engine-independent.  VM `host/` is restricted to dispatching the
//! handler registered under `"<idl-attr>.enumerated.get"` — the
//! algorithm itself stays here per CLAUDE.md "Layering mandate".

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
/// Missing default = `""` (the empty-string sentinel — IDL is nullable
/// `DOMString?`, but this module uses `""` throughout to keep the
/// `&[&'static str]` table machinery uniform; the prototype-side
/// binding maps `""` → `null` at the JS boundary).  Invalid default
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
}
