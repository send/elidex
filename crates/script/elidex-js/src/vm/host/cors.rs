//! WHATWG Fetch CORS (Cross-Origin Resource Sharing) classifier.
//!
//! Lives between the broker (which is mode-agnostic — it just
//! delivers an HTTP exchange) and the JS-facing Response object
//! (which surfaces a `.type` IDL attribute and gates header
//! visibility by the chosen filter).
//!
//! ## Spec mapping
//!
//! WHATWG Fetch §3.1.4-§3.1.7 describe the four filtered-response
//! kinds; this module supplies the per-fetch decision that selects
//! one:
//!
//! - `Basic` — same-origin response.
//! - `Cors` — cross-origin response that satisfied the CORS check
//!   (`Access-Control-Allow-Origin` matched the request's origin
//!   or was `*`).
//! - `Opaque` — `mode: "no-cors"` cross-origin response (always
//!   succeeds at the network level but body / headers / url
//!   are stripped from JS).
//! - `OpaqueRedirect` — `mode: "manual"` redirect response (a 3xx
//!   surfaced as if it had no body and no headers).
//! - Network error — `mode: "cors"` cross-origin response without
//!   ACAO; the JS Promise rejects with `TypeError`.

#![cfg(feature = "engine")]

use url::Url;

use super::request_response::{RedirectMode, RequestCredentials, RequestMode, ResponseType};

/// Per-pending-fetch metadata captured at dispatch time so the
/// `tick_network` settlement step can run the CORS classifier
/// without re-deriving any of these values from the broker reply.
/// Stored in [`super::super::VmInner::pending_fetch_cors`] keyed
/// by `FetchId`; same drain lifecycle as
/// [`super::super::VmInner::pending_fetches`].
#[derive(Debug, Clone)]
pub(crate) struct FetchCorsMeta {
    /// Original request URL — used for the same-origin check and
    /// for `response.url` rewriting under opaque shapes.
    pub(crate) request_url: Url,
    /// Document origin that initiated the fetch.  `None` for
    /// embedder-driven loads with no JS-script-origin context
    /// or for opaque initiator origins; the classifier short-
    /// circuits to `Basic` in that case.  Stored as
    /// [`url::Origin`] so the classifier compares origin-to-
    /// origin without a `.origin()` round-trip and so the
    /// classifier never sees the initiator's path / query /
    /// fragment (Copilot R1, PR #133).
    pub(crate) request_origin: Option<url::Origin>,
    /// `init.mode` (or the source `Request`'s mode for the
    /// Request-input path).
    pub(crate) request_mode: RequestMode,
    /// `init.credentials` — gates the strict ACAO/ACAC checks
    /// in [`cors_check_passes`].  Cross-origin requests with
    /// `Include` credentials reject `Access-Control-Allow-
    /// Origin: *` and require `Access-Control-Allow-Credentials:
    /// true` per WHATWG Fetch §3.2.5 (Copilot R3 finding 5).
    pub(crate) request_credentials: RequestCredentials,
    /// `init.redirect` — `Manual` triggers the OpaqueRedirect
    /// classification when the response status is 3xx.
    pub(crate) redirect_mode: RedirectMode,
}

/// Classification + filter outcome: which `ResponseType` the JS
/// Response should expose, plus a flag signalling that the
/// response should rewrite to opaque-shape (empty headers, body
/// dropped, url empty, status 0).  The flag is `true` for both
/// `Opaque` and `OpaqueRedirect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CorsClassification {
    pub(crate) response_type: ResponseType,
    pub(crate) opaque_shape: bool,
}

/// Outcome of [`classify_response_type`]: either a successful
/// classification, or a network error.  The latter rejects the
/// Promise with `TypeError("Failed to fetch")`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CorsOutcome {
    Ok(CorsClassification),
    NetworkError,
}

/// Classify a broker response per WHATWG Fetch §3.2.5 / §4.4.
///
/// - **Same-origin → `Basic`** (only when **both** the original
///   request URL **and** the final response URL match the
///   initiator origin).  WHATWG Fetch's actual algorithm tracks
///   a *tainted-origin flag* across redirects so a chain that
///   crossed origin can still get Basic when the final URL is
///   same-origin; we don't propagate that flag through the
///   broker yet, so the defensive `&&` here fails closed: a
///   chain where any URL crossed origin runs the cors path
///   (which would NetworkError unless the same-origin server
///   explicitly opts in via ACAO — typically not the case).
///   Tracking the tainted-origin flag belongs with PR5-cors-
///   preflight broker-state work (Copilot R3 finding 4
///   acknowledged; defensive logic kept).
/// - `mode = "no-cors"` cross-origin → `Opaque` (body / headers
///   / url stripped).
/// - `redirect = "manual"` returning a 3xx (regardless of
///   origin) → `OpaqueRedirect`.
/// - `mode = "cors"` cross-origin:
///   - For non-credentialed requests:
///     `Access-Control-Allow-Origin` matches request origin or
///     is `*` → `Cors`; else `NetworkError`.
///   - For credentialed requests (`credentials: "include"`):
///     `*` is rejected (spec §3.2.5); ACAO must match the
///     request origin **exactly** AND `Access-Control-Allow-
///     Credentials: true` must be present (Copilot R3 finding
///     5).
/// - `mode = "navigate"` is internal and cannot be reached from
///   JS-facing fetch (parser rejects it).
///
/// `request_origin` is the document / worker origin that
/// initiated the fetch (the value populated by
/// [`super::fetch::origin_for_request`]).  Script-initiated
/// fetches always carry `Some(origin)` — including opaque
/// origins from `data:` / `about:blank` initiators (the
/// classifier compares the opaque origin against the response
/// origin and proceeds through the cors path because opaque !=
/// any tuple origin).  `None` is reserved for **embedder-driven
/// callers** that bypass the VM fetch path entirely (initial
/// document load, favicon prefetch); those genuinely have no
/// script-origin context against which to compute "cross-
/// origin" so the classifier falls through to `Basic`.
///
/// Copilot R3 (finding 3): before this PR, `origin_for_request`
/// returned `None` for non-HTTP(S) initiators (`data:` /
/// `about:blank`), which made the classifier short-circuit to
/// `Basic` and bypass CORS for opaque-origin scripts.  Fixed
/// upstream in `origin_for_request`; this function's `None`
/// path is now unreachable from VM-side fetch and only serves
/// the embedder fallback contract.
pub(crate) fn classify_response_type(
    request_origin: Option<&url::Origin>,
    request_url: &Url,
    request_mode: RequestMode,
    request_credentials: RequestCredentials,
    redirect_mode: RedirectMode,
    response_url: &Url,
    response_status: u16,
    response_headers: &[(String, String)],
) -> CorsOutcome {
    // `manual` redirect mode + 3xx response → OpaqueRedirect
    // regardless of cross-origin status (spec §4.4 "main fetch"
    // step 13.2).
    if matches!(redirect_mode, RedirectMode::Manual) && (300..400).contains(&response_status) {
        return CorsOutcome::Ok(CorsClassification {
            response_type: ResponseType::OpaqueRedirect,
            opaque_shape: true,
        });
    }

    let Some(source) = request_origin else {
        // No origin context — treat as Basic.  The cookie /
        // referrer plumbing already handles this case (no
        // attach), and there's no origin to compare against
        // here.
        return CorsOutcome::Ok(CorsClassification {
            response_type: ResponseType::Basic,
            opaque_shape: false,
        });
    };

    let same_origin = *source == response_url.origin() && *source == request_url.origin();
    if same_origin {
        return CorsOutcome::Ok(CorsClassification {
            response_type: ResponseType::Basic,
            opaque_shape: false,
        });
    }

    match request_mode {
        RequestMode::SameOrigin => {
            // The earlier same-origin reject in `build_net_request`
            // makes this branch unreachable for normal flows, but
            // a redirect chain can land on a different origin
            // mid-flight.  Fail closed.
            CorsOutcome::NetworkError
        }
        RequestMode::NoCors => CorsOutcome::Ok(CorsClassification {
            response_type: ResponseType::Opaque,
            opaque_shape: true,
        }),
        RequestMode::Cors | RequestMode::Navigate => {
            // WHATWG Fetch §3.2.5 "CORS check" — the response must
            // carry an `Access-Control-Allow-Origin` whose value
            // is `*` or matches the request's origin, and for
            // credentialed cross-origin requests the additional
            // `Access-Control-Allow-Credentials: true` rule
            // applies (`*` rejected).
            //
            // The credentialed-network condition fires only when
            // `init.credentials = "include"` because `SameOrigin`
            // strips Cookie at `should_attach_cookies` for
            // cross-origin paths — so the network response was
            // not credentialed, and the relaxed `*` rule applies.
            let credentialed_network = matches!(request_credentials, RequestCredentials::Include);
            if cors_check_passes(source, response_headers, credentialed_network) {
                CorsOutcome::Ok(CorsClassification {
                    response_type: ResponseType::Cors,
                    opaque_shape: false,
                })
            } else {
                CorsOutcome::NetworkError
            }
        }
    }
}

/// WHATWG Fetch §3.2.5 "CORS check" — verify the response's
/// `Access-Control-Allow-Origin` header against the request
/// origin.  When the request was sent with credentials
/// (`credentialed_network = true`), the spec mandates stricter
/// rules:
///
/// - `Access-Control-Allow-Origin: *` is **rejected** (the
///   wildcard is incompatible with credentials).
/// - The ACAO value must match the request origin **exactly**
///   (case-insensitive serialised form).
/// - `Access-Control-Allow-Credentials: true` must be present.
///
/// For non-credentialed requests, `*` is accepted as before.
fn cors_check_passes(
    source: &url::Origin,
    response_headers: &[(String, String)],
    credentialed_network: bool,
) -> bool {
    let allowed = response_headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("access-control-allow-origin"))
        .map(|(_, v)| v.trim());
    match allowed {
        None => false,
        Some("*") => {
            // Credentialed requests cannot use the `*` wildcard
            // (Copilot R3 finding 5).  Without this gate, a
            // cross-origin server returning `ACAO: *` would let
            // a `credentials: 'include'` response slip through
            // and expose the response to script.
            !credentialed_network
        }
        Some(value) => {
            let serialised = source.ascii_serialization();
            if !value.eq_ignore_ascii_case(&serialised) {
                return false;
            }
            // Origin matches; for credentialed requests the spec
            // additionally requires `Access-Control-Allow-
            // Credentials: true` on the response.
            if credentialed_network {
                response_headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("access-control-allow-credentials"))
                    .is_some_and(|(_, v)| v.trim().eq_ignore_ascii_case("true"))
            } else {
                true
            }
        }
    }
}

/// Apply the CORS-mode header filter (WHATWG Fetch §3.1.5 +
/// §3.2.6).  Cors-typed responses expose only the
/// CORS-safelisted-response-header-names plus any names listed
/// in `Access-Control-Expose-Headers` — every other entry is
/// dropped before the headers are handed to the Response's
/// companion `Headers` object.  Header names may be provided
/// in any casing; this function lowercases names internally
/// for matching and returns a new vector with the filter
/// applied, preserving each retained header's original
/// name/value pair.
///
/// Forbidden-response-header names (§2.2.6 — `Set-Cookie` /
/// `Set-Cookie2`) are **always dropped**, regardless of
/// `Access-Control-Expose-Headers` content (Copilot R2 — without
/// this guard, a server that explicitly listed `Set-Cookie` in
/// expose-headers, or used the `*` wildcard, could leak HttpOnly
/// cookies into cross-origin script).
pub(crate) fn filter_headers_for_cors_response(
    headers: Vec<(String, String)>,
) -> Vec<(String, String)> {
    let exposed: std::collections::HashSet<String> = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("access-control-expose-headers"))
        .flat_map(|(_, value)| value.split(','))
        .map(|tok| tok.trim().to_ascii_lowercase())
        .filter(|tok| !tok.is_empty())
        .collect();
    let wildcard_expose = exposed.contains("*");
    headers
        .into_iter()
        .filter(|(name, _)| {
            let lower = name.to_ascii_lowercase();
            // Hard exclusion: forbidden-response-header names
            // (`Set-Cookie` / `Set-Cookie2`) are never visible to
            // cross-origin script even when the server explicitly
            // opts them into expose-headers (the spec leaves no
            // opt-in path for these so HttpOnly cookies stay
            // protected).
            if is_forbidden_response_header(&lower) {
                return false;
            }
            if is_cors_safelisted_response_header(&lower) {
                return true;
            }
            if wildcard_expose {
                // `Access-Control-Expose-Headers: *` exposes every
                // header except `Authorization` (spec §3.2.6).
                return lower != "authorization";
            }
            exposed.contains(&lower)
        })
        .collect()
}

/// WHATWG Fetch §2.2.6 — names that must never be exposed to
/// cross-origin script.  `Set-Cookie` / `Set-Cookie2` carry
/// HttpOnly cookies whose exposure would defeat the HttpOnly
/// guarantee.  The CORS filter at
/// [`filter_headers_for_cors_response`] drops these
/// unconditionally so a misconfigured `Access-Control-Expose-
/// Headers` value cannot leak them.
fn is_forbidden_response_header(name_lowercase: &str) -> bool {
    matches!(name_lowercase, "set-cookie" | "set-cookie2")
}

/// WHATWG Fetch §3.2.6 — names always visible on a Cors / Basic
/// filtered response.  Used by [`filter_headers_for_cors_response`]
/// to retain the spec-mandated minimum set even when the server
/// did not list them in `Access-Control-Expose-Headers`.
fn is_cors_safelisted_response_header(name_lowercase: &str) -> bool {
    matches!(
        name_lowercase,
        "cache-control"
            | "content-language"
            | "content-length"
            | "content-type"
            | "expires"
            | "last-modified"
            | "pragma"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).expect("valid url")
    }

    /// Helper: build a [`url::Origin`] from a URL string for
    /// the classifier's `request_origin` parameter.
    fn origin(s: &str) -> url::Origin {
        url(s).origin()
    }

    #[test]
    fn same_origin_classifies_as_basic() {
        let source = origin("http://example.com/page");
        let target = url("http://example.com/api");
        let out = classify_response_type(
            Some(&source),
            &target,
            RequestMode::Cors,
            RequestCredentials::SameOrigin,
            RedirectMode::Follow,
            &target,
            200,
            &[],
        );
        assert!(matches!(
            out,
            CorsOutcome::Ok(CorsClassification {
                response_type: ResponseType::Basic,
                opaque_shape: false
            })
        ));
    }

    #[test]
    fn no_cors_cross_origin_is_opaque() {
        let source = origin("http://example.com/page");
        let target = url("http://other.com/api");
        let out = classify_response_type(
            Some(&source),
            &target,
            RequestMode::NoCors,
            RequestCredentials::SameOrigin,
            RedirectMode::Follow,
            &target,
            200,
            &[],
        );
        assert!(matches!(
            out,
            CorsOutcome::Ok(CorsClassification {
                response_type: ResponseType::Opaque,
                opaque_shape: true
            })
        ));
    }

    #[test]
    fn cors_cross_origin_with_acao_passes() {
        let source = origin("http://example.com/page");
        let target = url("http://other.com/api");
        let headers = vec![(
            "Access-Control-Allow-Origin".to_string(),
            "http://example.com".to_string(),
        )];
        let out = classify_response_type(
            Some(&source),
            &target,
            RequestMode::Cors,
            RequestCredentials::SameOrigin,
            RedirectMode::Follow,
            &target,
            200,
            &headers,
        );
        assert!(matches!(
            out,
            CorsOutcome::Ok(CorsClassification {
                response_type: ResponseType::Cors,
                opaque_shape: false
            })
        ));
    }

    #[test]
    fn cors_cross_origin_wildcard_passes() {
        let source = origin("http://example.com/page");
        let target = url("http://other.com/api");
        let headers = vec![("access-control-allow-origin".to_string(), "*".to_string())];
        let out = classify_response_type(
            Some(&source),
            &target,
            RequestMode::Cors,
            RequestCredentials::SameOrigin,
            RedirectMode::Follow,
            &target,
            200,
            &headers,
        );
        assert!(matches!(
            out,
            CorsOutcome::Ok(CorsClassification {
                response_type: ResponseType::Cors,
                opaque_shape: false
            })
        ));
    }

    #[test]
    fn cors_cross_origin_without_acao_is_network_error() {
        let source = origin("http://example.com/page");
        let target = url("http://other.com/api");
        let out = classify_response_type(
            Some(&source),
            &target,
            RequestMode::Cors,
            RequestCredentials::SameOrigin,
            RedirectMode::Follow,
            &target,
            200,
            &[],
        );
        assert!(matches!(out, CorsOutcome::NetworkError));
    }

    #[test]
    fn cors_cross_origin_wrong_acao_is_network_error() {
        let source = origin("http://example.com/page");
        let target = url("http://other.com/api");
        let headers = vec![(
            "Access-Control-Allow-Origin".to_string(),
            "http://attacker.com".to_string(),
        )];
        let out = classify_response_type(
            Some(&source),
            &target,
            RequestMode::Cors,
            RequestCredentials::SameOrigin,
            RedirectMode::Follow,
            &target,
            200,
            &headers,
        );
        assert!(matches!(out, CorsOutcome::NetworkError));
    }

    #[test]
    fn manual_redirect_3xx_classifies_as_opaque_redirect() {
        let source = origin("http://example.com/page");
        let target = url("http://example.com/api");
        let out = classify_response_type(
            Some(&source),
            &target,
            RequestMode::Cors,
            RequestCredentials::SameOrigin,
            RedirectMode::Manual,
            &target,
            302,
            &[],
        );
        assert!(matches!(
            out,
            CorsOutcome::Ok(CorsClassification {
                response_type: ResponseType::OpaqueRedirect,
                opaque_shape: true
            })
        ));
    }

    #[test]
    fn manual_redirect_non_3xx_falls_through_to_normal_classification() {
        let source = origin("http://example.com/page");
        let target = url("http://example.com/api");
        let out = classify_response_type(
            Some(&source),
            &target,
            RequestMode::Cors,
            RequestCredentials::SameOrigin,
            RedirectMode::Manual,
            &target,
            200,
            &[],
        );
        assert!(matches!(
            out,
            CorsOutcome::Ok(CorsClassification {
                response_type: ResponseType::Basic,
                opaque_shape: false
            })
        ));
    }

    #[test]
    fn no_origin_falls_through_to_basic() {
        let target = url("http://example.com/api");
        let out = classify_response_type(
            None,
            &target,
            RequestMode::Cors,
            RequestCredentials::SameOrigin,
            RedirectMode::Follow,
            &target,
            200,
            &[],
        );
        assert!(matches!(
            out,
            CorsOutcome::Ok(CorsClassification {
                response_type: ResponseType::Basic,
                opaque_shape: false
            })
        ));
    }

    #[test]
    fn filter_drops_non_safelisted_when_no_expose_header() {
        let headers = vec![
            ("content-type".to_string(), "application/json".to_string()),
            ("x-custom".to_string(), "secret".to_string()),
        ];
        let filtered = filter_headers_for_cors_response(headers);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "content-type");
    }

    #[test]
    fn filter_keeps_explicitly_exposed_headers() {
        let headers = vec![
            (
                "Access-Control-Expose-Headers".to_string(),
                "X-Custom, X-Another".to_string(),
            ),
            ("x-custom".to_string(), "ok".to_string()),
            ("x-other".to_string(), "drop".to_string()),
        ];
        let filtered = filter_headers_for_cors_response(headers);
        let names: Vec<_> = filtered.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"x-custom"));
        assert!(!names.contains(&"x-other"));
    }

    #[test]
    fn filter_wildcard_expose_keeps_all_except_authorization() {
        let headers = vec![
            ("Access-Control-Expose-Headers".to_string(), "*".to_string()),
            ("x-custom".to_string(), "ok".to_string()),
            ("authorization".to_string(), "Bearer t".to_string()),
        ];
        let filtered = filter_headers_for_cors_response(headers);
        let names: Vec<_> = filtered.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"x-custom"));
        assert!(!names.contains(&"authorization"));
    }

    /// Copilot R2 regression: `Set-Cookie` / `Set-Cookie2` must be
    /// dropped from a Cors-filtered response even when the server
    /// explicitly lists them in `Access-Control-Expose-Headers`.
    /// Without this guard, a misconfigured CORS server could leak
    /// HttpOnly cookies into cross-origin script.
    #[test]
    fn filter_drops_set_cookie_even_when_explicitly_exposed() {
        let headers = vec![
            (
                "Access-Control-Expose-Headers".to_string(),
                "Set-Cookie, Set-Cookie2, X-Custom".to_string(),
            ),
            (
                "set-cookie".to_string(),
                "session=abc; HttpOnly".to_string(),
            ),
            ("set-cookie2".to_string(), "legacy=def".to_string()),
            ("x-custom".to_string(), "visible".to_string()),
        ];
        let filtered = filter_headers_for_cors_response(headers);
        let names: Vec<_> = filtered.iter().map(|(n, _)| n.as_str()).collect();
        assert!(!names.contains(&"set-cookie"), "Set-Cookie must be dropped");
        assert!(
            !names.contains(&"set-cookie2"),
            "Set-Cookie2 must be dropped"
        );
        assert!(names.contains(&"x-custom"), "non-forbidden expose passes");
    }

    /// Copilot R3 regression (finding 5): credentialed CORS
    /// requests must reject `Access-Control-Allow-Origin: *`
    /// (spec §3.2.5).
    #[test]
    fn credentialed_cors_rejects_wildcard_acao() {
        let source = origin("http://example.com/page");
        let target = url("http://other.com/api");
        let headers = vec![("access-control-allow-origin".to_string(), "*".to_string())];
        let out = classify_response_type(
            Some(&source),
            &target,
            RequestMode::Cors,
            RequestCredentials::Include,
            RedirectMode::Follow,
            &target,
            200,
            &headers,
        );
        assert!(matches!(out, CorsOutcome::NetworkError));
    }

    /// Copilot R3 regression (finding 5): credentialed CORS
    /// requests with matching origin still require
    /// `Access-Control-Allow-Credentials: true`.
    #[test]
    fn credentialed_cors_requires_allow_credentials_true() {
        let source = origin("http://example.com/page");
        let target = url("http://other.com/api");
        // Origin matches but ACAC missing → NetworkError.
        let headers_no_acac = vec![(
            "Access-Control-Allow-Origin".to_string(),
            "http://example.com".to_string(),
        )];
        let out = classify_response_type(
            Some(&source),
            &target,
            RequestMode::Cors,
            RequestCredentials::Include,
            RedirectMode::Follow,
            &target,
            200,
            &headers_no_acac,
        );
        assert!(matches!(out, CorsOutcome::NetworkError));

        // Origin matches AND ACAC: true → passes.
        let headers_with_acac = vec![
            (
                "Access-Control-Allow-Origin".to_string(),
                "http://example.com".to_string(),
            ),
            (
                "Access-Control-Allow-Credentials".to_string(),
                "true".to_string(),
            ),
        ];
        let out = classify_response_type(
            Some(&source),
            &target,
            RequestMode::Cors,
            RequestCredentials::Include,
            RedirectMode::Follow,
            &target,
            200,
            &headers_with_acac,
        );
        assert!(matches!(
            out,
            CorsOutcome::Ok(CorsClassification {
                response_type: ResponseType::Cors,
                ..
            })
        ));
    }

    /// Sentinel: non-credentialed (SameOrigin / Omit) cross-
    /// origin CORS requests still accept wildcard `*` (Copilot
    /// R3 finding 5 only restricts credentialed requests).
    #[test]
    fn non_credentialed_cors_still_accepts_wildcard() {
        let source = origin("http://example.com/page");
        let target = url("http://other.com/api");
        let headers = vec![("access-control-allow-origin".to_string(), "*".to_string())];
        let out = classify_response_type(
            Some(&source),
            &target,
            RequestMode::Cors,
            RequestCredentials::SameOrigin,
            RedirectMode::Follow,
            &target,
            200,
            &headers,
        );
        assert!(matches!(
            out,
            CorsOutcome::Ok(CorsClassification {
                response_type: ResponseType::Cors,
                ..
            })
        ));
    }

    /// Copilot R2 regression: wildcard `Access-Control-Expose-
    /// Headers: *` must NOT expose `Set-Cookie` / `Set-Cookie2`
    /// either — the forbidden-response-header guard takes
    /// precedence over the wildcard expose path.
    #[test]
    fn filter_wildcard_expose_still_drops_set_cookie() {
        let headers = vec![
            ("Access-Control-Expose-Headers".to_string(), "*".to_string()),
            (
                "set-cookie".to_string(),
                "session=abc; HttpOnly".to_string(),
            ),
            ("x-custom".to_string(), "visible".to_string()),
        ];
        let filtered = filter_headers_for_cors_response(headers);
        let names: Vec<_> = filtered.iter().map(|(n, _)| n.as_str()).collect();
        assert!(!names.contains(&"set-cookie"));
        assert!(names.contains(&"x-custom"));
    }
}
