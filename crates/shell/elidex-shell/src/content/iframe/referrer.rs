//! Iframe referrer computation (W3C Referrer Policy §8.3 / §8.4).
//!
//! The single canonical pipeline every iframe path routes through to derive the
//! `document.referrer` a child document receives, under the iframe element's
//! `referrerpolicy` content attribute. Pure functions — no shared mutable state
//! with the load-dispatch in [`super::load`], which invokes
//! [`iframe_referrer_policy`] (to read the canonical keyword) and
//! [`compute_referrer`] (to derive the referrer) at its install sites.

use elidex_dom_api::element::enumerated_reflect::{
    canonicalize_enumerated_attr, REFERRER_POLICY_INVALID_DEFAULT, REFERRER_POLICY_MISSING_DEFAULT,
    REFERRER_POLICY_VALUES,
};
use elidex_plugin::SecurityOrigin;

/// The Fetch "local scheme" set — `about` / `blob` / `data`
/// (<https://fetch.spec.whatwg.org/#local-scheme>), reused by W3C Referrer
/// Policy §8.4 step 2. A URL with a local scheme has no referrer to disclose.
fn is_local_scheme(scheme: &str) -> bool {
    matches!(scheme, "about" | "blob" | "data")
}

/// Strip a URL "for use as a referrer" (W3C Referrer Policy §8.4 steps 3–5):
/// remove the username, password, and fragment so parent credentials
/// (`user:pass@`) and fragment secrets (`#…`) never leak into a sub-frame's
/// `document.referrer`. The step-2 "no referrer" gates (local scheme / opaque
/// origin) are applied by the sole caller [`compute_referrer`], so this is the
/// pure serialization step. Mirrors the VM's `Vm::set_navigation_referrer`
/// sanitisation (`elidex-js` `vm/vm_api.rs`) — the `Referer` header and
/// `document.referrer` share the same exposure surface.
fn strip_referrer_url(url: &url::Url) -> String {
    let mut url = url.clone();
    url.set_fragment(None);
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.to_string()
}

/// Canonicalize the iframe element's `referrerpolicy` content attribute to its
/// HTML §2.3.5 enumerated keyword, reusing the engine-independent
/// [`enumerated_reflect`](elidex_dom_api::element::enumerated_reflect) table
/// (`REFERRER_POLICY_VALUES`) shared with the `referrerPolicy` IDL getters.
///
/// Missing / invalid / unknown → `""` (the empty keyword), which
/// [`compute_referrer`] maps to the DEFAULT policy
/// `strict-origin-when-cross-origin`. `iframe_data.referrer_policy` was parsed
/// into `IframeData` at DOM-build time; this connects that previously-dead
/// field.
pub(super) fn iframe_referrer_policy(iframe_data: &elidex_ecs::IframeData) -> &'static str {
    canonicalize_enumerated_attr(
        iframe_data.referrer_policy.as_deref(),
        REFERRER_POLICY_VALUES,
        REFERRER_POLICY_MISSING_DEFAULT,
        REFERRER_POLICY_INVALID_DEFAULT,
    )
}

/// The `document.referrer` a child document receives under a given W3C Referrer
/// Policy (§8.3 "Determine request's Referrer"). This is the ONE canonical
/// pipeline every iframe path routes through (in-process same-origin, OOP
/// cross-origin, and — via the persisted bridge referrer — the navigate
/// rebuild), applied uniformly IN ORDER (§8.3 → §8.4 "Strip url for use as a
/// referrer").
///
/// - `policy`: the iframe element's canonical `referrerpolicy` keyword (from
///   [`iframe_referrer_policy`]). The empty / unknown keyword maps to the
///   DEFAULT `strict-origin-when-cross-origin` (§3, §8.3 note "if request's
///   referrer policy is the empty string"). This honours the author's explicit
///   `<iframe referrerpolicy=…>` directive — the ONLY per-request
///   ReferrerPolicy source modelled (see the deferred-slot note below).
/// - `source_url` / `source_origin`: the parent (referrer source) document URL
///   and origin.
/// - `request_origin`: the child request's origin (`from_url(loaded.url)`, the
///   ACTUAL loaded-URL origin BEFORE any sandbox opaque-ification) — NOT the
///   post-sandbox document origin and NOT the OOP-routing decision. A
///   same-origin request that is merely sandboxed-to-opaque still shares the
///   full parent URL.
///
/// **No valid referrer source** precondition (applies to EVERY policy, NOT
/// policy-overridable — even `unsafe-url`, because §8.4 step 2 strips a
/// local-scheme source to "no referrer" before the policy switch runs) →
/// `None`: `source_origin` is opaque (§8.3 step 2.2), OR `source_url` has a
/// Fetch **local scheme** (`about` / `blob` / `data`, §8.4 step 2). The
/// local-scheme test is on the **source URL scheme, not the origin** — an
/// `about:blank` parent has a local scheme but an inherited (non-opaque, tuple)
/// origin, so it is caught here on BOTH the same-origin AND cross-origin paths
/// (where an origin-opaque-only gate would leak it cross-origin).
///
/// After the precondition, the two §8.4 forms are:
///
/// - `referrerURL` = strip `source_url` (username / password / fragment
///   removed, [`strip_referrer_url`]) — the full source URL.
/// - `referrerOrigin` = the source ORIGIN-as-URL: its serialization followed by
///   U+002F SOLIDUS (`/`) — §8.4 with the origin-only flag leaves an empty path
///   that serializes as `/`.
///
/// and `downgrade` = `source_origin` is potentially-trustworthy AND
/// `request_origin` is NOT (§3.7 / §8.3). "Potentially trustworthy" — not
/// merely `https` — via [`SecurityOrigin::is_potentially_trustworthy`], so an
/// https parent embedding a loopback-`http` child (`http://localhost`,
/// `http://127.0.0.1`, `http://[::1]`) is NOT a downgrade. Same-origin is
/// necessarily same-scheme, so a downgrade only ever arises cross-origin.
///
/// The §8.3 step-8 per-policy switch:
///
/// - `no-referrer` → `None`.
/// - `no-referrer-when-downgrade` → `referrerURL`, or `None` on downgrade.
/// - `same-origin` → `referrerURL` if same-origin, else `None`.
/// - `origin` → `referrerOrigin` always.
/// - `strict-origin` → `referrerOrigin`, or `None` on downgrade.
/// - `origin-when-cross-origin` → `referrerURL` if same-origin, else
///   `referrerOrigin`.
/// - `strict-origin-when-cross-origin` (DEFAULT; empty / unset / unknown maps
///   here) → `referrerURL` if same-origin, else `None` on downgrade, else
///   `referrerOrigin`.
/// - `unsafe-url` → `referrerURL` always (still gated by the precondition).
///
/// Only the OTHER per-request ReferrerPolicy sources remain deferred → slot
/// `#11-referrer-policy` (a new carve; ledger registration is a landing
/// deliverable): the `<meta name="referrer">` element, the `Referrer-Policy`
/// response header, `rel=noreferrer` / `rel=noopener` on links, and
/// per-subresource-fetch policy inheritance — including the rule that an iframe
/// with NO `referrerpolicy` attribute inherits the parent document's referrer
/// policy (here the no-attr case falls to the fixed default, not the parent's
/// policy).
pub(super) fn compute_referrer(
    policy: &str,
    source_url: Option<&url::Url>,
    source_origin: &SecurityOrigin,
    request_origin: &SecurityOrigin,
) -> Option<String> {
    // "No valid referrer source" precondition (opaque origin OR local-scheme
    // URL) — applies to ALL policies, so it precedes the switch.
    if matches!(source_origin, SecurityOrigin::Opaque(_)) {
        return None;
    }
    let source_url = source_url?;
    if is_local_scheme(source_url.scheme()) {
        return None;
    }

    let referrer_url = || strip_referrer_url(source_url);
    let referrer_origin = || format!("{}/", source_origin.serialize());
    let same_origin = source_origin == request_origin;
    let downgrade =
        source_origin.is_potentially_trustworthy() && !request_origin.is_potentially_trustworthy();

    match policy {
        "no-referrer" => None,
        "no-referrer-when-downgrade" => (!downgrade).then(referrer_url),
        "same-origin" => same_origin.then(referrer_url),
        "origin" => Some(referrer_origin()),
        "strict-origin" => (!downgrade).then(referrer_origin),
        "origin-when-cross-origin" => Some(if same_origin {
            referrer_url()
        } else {
            referrer_origin()
        }),
        "unsafe-url" => Some(referrer_url()),
        // "strict-origin-when-cross-origin" — the DEFAULT, also reached by the
        // empty / unset / unknown keyword.
        _ => {
            if same_origin {
                Some(referrer_url())
            } else if downgrade {
                None
            } else {
                Some(referrer_origin())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{compute_referrer, strip_referrer_url, SecurityOrigin};

    fn origin(s: &str) -> SecurityOrigin {
        SecurityOrigin::from_url(&url::Url::parse(s).unwrap())
    }

    /// The DEFAULT referrer policy: the empty / unset `referrerpolicy` keyword
    /// maps to `strict-origin-when-cross-origin`. All the pre-existing referrer
    /// tests exercise this policy (they predate honouring the attribute), so
    /// they route through it here.
    const DEFAULT: &str = "";

    /// Step 5: a **same-origin** request that is merely sandboxed-to-opaque (OOP
    /// routed) still shares the FULL (stripped) source URL — the referrer keys
    /// on the request relationship (`source_origin` == `request_origin`), not the
    /// post-sandbox opaque document origin. Falsify by making the same-origin arm
    /// emit the ORIGIN-as-URL instead of the stripped full URL.
    #[test]
    fn default_referrer_same_origin_request_keeps_full_url() {
        let parent = url::Url::parse("https://parent.example/a/b?q").unwrap();
        let request = origin("https://parent.example/child");
        assert_eq!(
            compute_referrer(
                DEFAULT,
                Some(&parent),
                &origin("https://parent.example/"),
                &request
            )
            .as_deref(),
            Some("https://parent.example/a/b?q"),
            "same-origin request must expose the full parent URL, even when sandboxed"
        );
    }

    /// Step 4: a genuinely cross-origin request → source ORIGIN-as-URL (trailing
    /// slash, R3-F1).
    #[test]
    fn default_referrer_cross_origin_request_is_origin_only() {
        let parent = url::Url::parse("https://parent.example/a/b?q").unwrap();
        let request = origin("https://other.example/");
        assert_eq!(
            compute_referrer(
                DEFAULT,
                Some(&parent),
                &origin("https://parent.example/"),
                &request
            )
            .as_deref(),
            Some("https://parent.example/"),
            "cross-origin request must be trimmed to the source ORIGIN-as-URL"
        );
    }

    /// Step 2: userinfo + fragment are stripped before the source URL becomes a
    /// same-origin referrer (WHATWG Fetch "strip url for use as a referrer"), so
    /// credentials/secrets never leak via `document.referrer`. Falsify by
    /// reverting `strip_referrer_url` to a bare `to_string()`.
    #[test]
    fn strip_referrer_url_removes_userinfo_and_fragment() {
        let url = url::Url::parse("https://user:pass@parent.example/path#frag").unwrap();
        assert_eq!(strip_referrer_url(&url), "https://parent.example/path");
    }

    /// Step 1 (local scheme, same-origin path): a local-scheme URL (`about` /
    /// `blob` / `data`, Fetch "local scheme") has no referrer (W3C Referrer
    /// Policy §8.4 step 2), so its `data:`/`about:`/`blob:` URL never leaks into
    /// `document.referrer`. Each source origin here is the URL's own (opaque for
    /// data:, tuple for the blob:/about: inherited cases we use elsewhere), and
    /// the request is same-origin — the local-scheme gate must fire regardless.
    /// Falsify by removing the `is_local_scheme` gate in `default_referrer`.
    #[test]
    fn default_referrer_local_scheme_source_is_none() {
        for u in [
            "data:text/html,<p>hi",
            "about:blank",
            "blob:https://x.example/uuid",
        ] {
            let url = url::Url::parse(u).unwrap();
            // Use a tuple source origin so the opaque gate can't mask the
            // local-scheme gate for about:/blob:.
            let src = origin("https://parent.example/");
            assert_eq!(
                compute_referrer(DEFAULT, Some(&url), &src, &src),
                None,
                "local-scheme URL {u} must have no referrer"
            );
        }
    }

    /// Step 1 (opaque origin): `default_referrer` yields `None` for an
    /// opaque-origin source (data:/file:/sandboxed — W3C Referrer Policy §8.3
    /// step 2.2), so a child of such a parent never receives a leaked URL.
    /// Falsify by dropping the opaque-origin gate.
    #[test]
    fn default_referrer_opaque_source_is_none() {
        let data_parent = url::Url::parse("data:text/html,<p>hi").unwrap();
        let opaque = SecurityOrigin::opaque();
        assert_eq!(
            compute_referrer(DEFAULT, Some(&data_parent), &opaque, &opaque),
            None,
            "an opaque-origin source has no referrer to disclose"
        );
    }

    /// R3-F3 (same-origin path): an `about:blank` parent whose *inherited* origin
    /// is a non-opaque tuple still yields no referrer (its URL is a local scheme)
    /// — the local-scheme gate catches it where the opaque-origin gate does not.
    /// Falsify by reverting step 1 to an origin-opaque-only check.
    #[test]
    fn default_referrer_about_blank_tuple_origin_same_origin_is_none() {
        let about = url::Url::parse("about:blank").unwrap();
        let tuple = origin("https://parent.example/");
        assert_eq!(
            compute_referrer(DEFAULT, Some(&about), &tuple, &tuple),
            None,
            "an about:blank parent (local-scheme URL) discloses no referrer even \
             with an inherited tuple origin"
        );
    }

    /// **R4** (the leak this PR closes): an `about:blank` parent with an
    /// inherited TUPLE origin embedding a **CROSS-ORIGIN** child. The origin is a
    /// non-opaque tuple, so an origin-opaque-only gate slips past and the
    /// cross-origin arm would emit `https://parent.example/`. The step-1
    /// local-scheme test — on the SOURCE URL SCHEME, not the origin — must catch
    /// it on the cross-origin path too. Falsify by reverting step 1 to an
    /// origin-opaque-only check (the pre-R4 bug): this would return
    /// `Some("https://parent.example/")`.
    #[test]
    fn default_referrer_about_blank_tuple_origin_cross_origin_is_none() {
        let about = url::Url::parse("about:blank").unwrap();
        let tuple = origin("https://parent.example/");
        let cross = origin("https://other.example/");
        assert_eq!(
            compute_referrer(DEFAULT, Some(&about), &tuple, &cross),
            None,
            "an about:blank (local-scheme) parent leaks NO referrer cross-origin, \
             even though its inherited origin is a non-opaque tuple"
        );
    }

    /// R3-F2: an https parent embedding a cross-origin **loopback**-`http` child
    /// (`http://localhost`) is NOT a TLS downgrade — loopback http is
    /// potentially trustworthy (Secure Contexts) — so the child still receives
    /// the source ORIGIN-as-URL referrer, not `None`. Falsify by reverting the
    /// downgrade check to a bare `https` scheme test.
    #[test]
    fn default_referrer_https_to_loopback_http_is_not_downgrade() {
        let parent = url::Url::parse("https://parent.example/a/b?q").unwrap();
        let request = origin("http://localhost/");
        assert_eq!(
            compute_referrer(
                DEFAULT,
                Some(&parent),
                &origin("https://parent.example/"),
                &request
            )
            .as_deref(),
            Some("https://parent.example/"),
            "loopback-http is potentially trustworthy, so this is not a downgrade"
        );
    }

    /// Step 2 through the same-origin path (end-to-end for the OOP
    /// same-origin-sandboxed case): the stripped full URL, not the raw one.
    #[test]
    fn default_referrer_same_origin_strips_userinfo_and_fragment() {
        let parent = url::Url::parse("https://user:pass@parent.example/path#frag").unwrap();
        let request = origin("https://parent.example/child");
        assert_eq!(
            compute_referrer(
                DEFAULT,
                Some(&parent),
                &origin("https://parent.example/"),
                &request
            )
            .as_deref(),
            Some("https://parent.example/path"),
        );
    }

    /// Step 4: an opaque source origin has no usable referrer to share (`"null"`
    /// is not a referrer), so a cross-origin child gets an empty
    /// `document.referrer` (caught by step 1's opaque gate before the trim).
    #[test]
    fn default_referrer_opaque_source_cross_origin_is_none() {
        let parent = url::Url::parse("data:text/html,<p>hi").unwrap();
        assert_eq!(
            compute_referrer(
                DEFAULT,
                Some(&parent),
                &SecurityOrigin::opaque(),
                &origin("https://other.example/")
            ),
            None
        );
    }

    /// R2: a TLS downgrade — an https parent embedding a cross-origin http
    /// child — sends NO referrer (a potentially-trustworthy referrerURL with a
    /// non-potentially-trustworthy current URL, W3C Referrer Policy §3.7), not
    /// the source origin. Falsify by removing the step-3 downgrade branch (it
    /// would return `Some("https://parent.example/")`).
    #[test]
    fn default_referrer_https_to_http_downgrade_is_none() {
        let parent = url::Url::parse("https://parent.example/a/b?q").unwrap();
        let request = origin("http://other.example/");
        assert_eq!(
            compute_referrer(
                DEFAULT,
                Some(&parent),
                &origin("https://parent.example/"),
                &request
            ),
            None,
            "secure→non-secure cross-origin downgrade must omit the referrer entirely"
        );
    }

    /// R2 control: an http parent → http cross-origin child is NOT a downgrade
    /// (source was never potentially-trustworthy), so the source origin is
    /// still sent (§3.7 "referrerURL is a non-potentially trustworthy URL").
    #[test]
    fn default_referrer_http_to_http_cross_origin_is_origin() {
        let parent = url::Url::parse("http://parent.example/a/b?q").unwrap();
        let request = origin("http://other.example/");
        assert_eq!(
            compute_referrer(
                DEFAULT,
                Some(&parent),
                &origin("http://parent.example/"),
                &request
            )
            .as_deref(),
            Some("http://parent.example/"),
            "non-secure source cross-origin is not a downgrade; source origin is sent"
        );
    }

    // -----------------------------------------------------------------------
    // Per-policy switch (§8.3 step 8) — the iframe element's `referrerpolicy`
    // attribute is now HONORED (R5). Each drives `compute_referrer` with an
    // explicit canonical keyword, holding the source/request fixed.
    // -----------------------------------------------------------------------

    /// `no-referrer` → NEVER a referrer, even same-origin with a full valid
    /// source URL (§8.3 "no-referrer" → return no referrer). This is the leak
    /// this finding closes: `<iframe referrerpolicy="no-referrer">` must send
    /// NOTHING where the DEFAULT policy would send the full parent URL. Falsify
    /// by reverting to the hardcoded default policy — it would return
    /// `Some("https://parent.example/a/b?q")`.
    #[test]
    fn compute_referrer_no_referrer_policy_is_always_none() {
        let parent = url::Url::parse("https://parent.example/a/b?q").unwrap();
        let same = origin("https://parent.example/");
        assert_eq!(
            compute_referrer("no-referrer", Some(&parent), &same, &same),
            None,
            "referrerpolicy=no-referrer must disclose no referrer even same-origin"
        );
    }

    /// `unsafe-url` → the FULL stripped source URL even CROSS-ORIGIN and even on
    /// a TLS downgrade (§8.3 "unsafe-url" → return referrerURL), where the
    /// default would trim to the origin or drop it.
    #[test]
    fn compute_referrer_unsafe_url_policy_keeps_full_url_cross_origin() {
        let parent = url::Url::parse("https://parent.example/a/b?q#frag").unwrap();
        let cross_downgrade = origin("http://other.example/");
        assert_eq!(
            compute_referrer("unsafe-url", Some(&parent), &origin("https://parent.example/"), &cross_downgrade)
                .as_deref(),
            Some("https://parent.example/a/b?q"),
            "referrerpolicy=unsafe-url sends the full (stripped) URL even cross-origin on a downgrade"
        );
    }

    /// `unsafe-url` is STILL gated by the "no valid referrer source"
    /// precondition: a local-scheme source URL yields `None` regardless of the
    /// most-permissive policy (§8.4 step 2 strips it before the switch).
    #[test]
    fn compute_referrer_unsafe_url_policy_still_gated_by_local_scheme() {
        let about = url::Url::parse("about:blank").unwrap();
        let tuple = origin("https://parent.example/");
        assert_eq!(
            compute_referrer("unsafe-url", Some(&about), &tuple, &tuple),
            None,
            "the local-scheme precondition is not policy-overridable, even by unsafe-url"
        );
    }

    /// `origin` → the source ORIGIN-as-URL for BOTH same-origin and cross-origin
    /// (§8.3 "origin" → return referrerOrigin), where the default keeps the full
    /// URL same-origin.
    #[test]
    fn compute_referrer_origin_policy_is_origin_form_both_ways() {
        let parent = url::Url::parse("https://parent.example/a/b?q").unwrap();
        let src = origin("https://parent.example/");
        assert_eq!(
            compute_referrer("origin", Some(&parent), &src, &src).as_deref(),
            Some("https://parent.example/"),
            "referrerpolicy=origin trims the same-origin referrer to the origin"
        );
        assert_eq!(
            compute_referrer(
                "origin",
                Some(&parent),
                &src,
                &origin("https://other.example/")
            )
            .as_deref(),
            Some("https://parent.example/"),
            "referrerpolicy=origin is the origin form cross-origin too"
        );
    }

    /// `same-origin` → the full URL same-origin but `None` CROSS-ORIGIN (§8.3
    /// "same-origin" step 2 → no referrer), where the default would send the
    /// cross-origin ORIGIN-as-URL.
    #[test]
    fn compute_referrer_same_origin_policy_drops_cross_origin() {
        let parent = url::Url::parse("https://parent.example/a/b?q").unwrap();
        let src = origin("https://parent.example/");
        assert_eq!(
            compute_referrer("same-origin", Some(&parent), &src, &src).as_deref(),
            Some("https://parent.example/a/b?q"),
            "referrerpolicy=same-origin keeps the full URL for a same-origin request"
        );
        assert_eq!(
            compute_referrer(
                "same-origin",
                Some(&parent),
                &src,
                &origin("https://other.example/")
            ),
            None,
            "referrerpolicy=same-origin sends nothing cross-origin"
        );
    }

    /// `strict-origin` → the origin form, but `None` on a TLS downgrade (§8.3
    /// "strict-origin" step 1), and origin form (not full URL) even same-origin.
    #[test]
    fn compute_referrer_strict_origin_policy_downgrade_and_same_origin() {
        let parent = url::Url::parse("https://parent.example/a/b?q").unwrap();
        let src = origin("https://parent.example/");
        assert_eq!(
            compute_referrer(
                "strict-origin",
                Some(&parent),
                &src,
                &origin("http://other.example/")
            ),
            None,
            "referrerpolicy=strict-origin drops the referrer on an https→http downgrade"
        );
        assert_eq!(
            compute_referrer("strict-origin", Some(&parent), &src, &src).as_deref(),
            Some("https://parent.example/"),
            "referrerpolicy=strict-origin is the origin form even same-origin"
        );
    }

    /// `origin-when-cross-origin` → full URL same-origin, origin form
    /// cross-origin, and — unlike the default `strict-` variant — does NOT drop
    /// on downgrade (§8.3 "origin-when-cross-origin" has no downgrade branch).
    #[test]
    fn compute_referrer_origin_when_cross_origin_no_downgrade_drop() {
        let parent = url::Url::parse("https://parent.example/a/b?q").unwrap();
        let src = origin("https://parent.example/");
        assert_eq!(
            compute_referrer("origin-when-cross-origin", Some(&parent), &src, &src).as_deref(),
            Some("https://parent.example/a/b?q"),
            "same-origin keeps the full URL"
        );
        assert_eq!(
            compute_referrer(
                "origin-when-cross-origin",
                Some(&parent),
                &src,
                &origin("http://other.example/")
            )
            .as_deref(),
            Some("https://parent.example/"),
            "origin-when-cross-origin still sends the origin on a downgrade (no strict drop)"
        );
    }

    /// `no-referrer-when-downgrade` → full URL, dropped only on a downgrade
    /// (§8.3), so a cross-origin non-downgrade request keeps the FULL URL
    /// (contrast the default, which trims cross-origin to the origin).
    #[test]
    fn compute_referrer_no_referrer_when_downgrade_keeps_full_cross_origin() {
        let parent = url::Url::parse("https://parent.example/a/b?q").unwrap();
        let src = origin("https://parent.example/");
        assert_eq!(
            compute_referrer(
                "no-referrer-when-downgrade",
                Some(&parent),
                &src,
                &origin("https://other.example/")
            )
            .as_deref(),
            Some("https://parent.example/a/b?q"),
            "no-referrer-when-downgrade keeps the full URL cross-origin when not downgrading"
        );
        assert_eq!(
            compute_referrer(
                "no-referrer-when-downgrade",
                Some(&parent),
                &src,
                &origin("http://other.example/")
            ),
            None,
            "no-referrer-when-downgrade drops on an https→http downgrade"
        );
    }

    /// An unknown / invalid policy keyword falls to the DEFAULT
    /// `strict-origin-when-cross-origin` behaviour (the `_` match arm), the same
    /// as the empty keyword — cross-origin trims to the origin.
    #[test]
    fn compute_referrer_unknown_policy_falls_to_default() {
        let parent = url::Url::parse("https://parent.example/a/b?q").unwrap();
        let src = origin("https://parent.example/");
        assert_eq!(
            compute_referrer(
                "bogus-policy",
                Some(&parent),
                &src,
                &origin("https://other.example/")
            )
            .as_deref(),
            Some("https://parent.example/"),
            "an unrecognised keyword behaves as the default policy"
        );
    }
}
