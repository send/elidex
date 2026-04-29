//! CORS preflight (WHATWG Fetch §4.8) + non-simple-request
//! detection (§4.6.5 CORS-safelisted-request-header).
//!
//! For cross-origin requests with `mode = Cors`, the broker must
//! issue an `OPTIONS` preflight to confirm the server permits the
//! actual method + author-specified non-safelisted headers before
//! dispatching the real request.
//!
//! The flow:
//!
//! 1. [`requires_preflight`] decides whether the request is "non
//!    simple" — non-`GET`/`HEAD`/`POST` method or any author
//!    header outside the §4.6.5 safelist.
//! 2. [`build_preflight_request`] constructs the OPTIONS request
//!    with `Access-Control-Request-Method` (ACRM) and
//!    `Access-Control-Request-Headers` (ACRH, sorted +
//!    lowercased).  Preflight is always credentials=Omit and
//!    redirect=Error per spec.
//! 3. [`validate_preflight_response`] parses the OPTIONS response
//!    headers (`Access-Control-Allow-Origin` /
//!    `-Allow-Credentials` / `-Allow-Methods` /
//!    `-Allow-Headers` / `-Max-Age`) and either returns a
//!    [`PreflightAllowance`] (cacheable) or a
//!    [`NetErrorKind::CorsBlocked`] error.
//! 4. [`validate_actual_against_allowance`] is the second-stage
//!    check that re-asserts the actual request's method/headers
//!    against the cached allowance (so a cache hit short-circuits
//!    the OPTIONS round-trip).
//!
//! `cors.rs` (the pre-existing module) is intentionally separate
//! — it implements only the §4.4 `Access-Control-Allow-Origin`
//! check on the actual response, not preflight.

use std::time::Duration;

use crate::error::{NetError, NetErrorKind};
use crate::{CredentialsMode, RedirectMode, Request, RequestMode, Response};
use bytes::Bytes;

/// Conservative cap on `Access-Control-Max-Age` (§4.8 step 19).
///
/// Spec allows arbitrary integers; browsers cap differently
/// (Chromium 7200s, Firefox 86400s).  We pick **7200s** to match
/// the more conservative behaviour — preflight cache entries
/// never live longer than 2 hours regardless of what the server
/// asserts.
pub const MAX_AGE_CAP_SECONDS: u64 = 7200;

/// Default `Access-Control-Max-Age` when the response omits the
/// header (§4.8 step 19 — "5 seconds" default).
pub const DEFAULT_MAX_AGE_SECONDS: u64 = 5;

/// Result of validating a preflight response.  Cached by
/// [`crate::preflight_cache::PreflightCache`] so subsequent
/// requests with the same `(origin, url, method, header-set)` key
/// can skip the OPTIONS round-trip.
#[derive(Clone, Debug)]
pub struct PreflightAllowance {
    /// `Access-Control-Allow-Methods` parsed value.  `None` means
    /// the response permitted `*` (which is only allowed for
    /// non-credentialed requests — see [`validate_preflight_response`]).
    pub allowed_methods: Option<Vec<String>>,
    /// `Access-Control-Allow-Headers` parsed value (lowercased).
    /// `None` means the response permitted `*`.
    pub allowed_headers: Option<Vec<String>>,
    /// Whether `Access-Control-Allow-Credentials: true` was
    /// present.  Required when the actual request has
    /// [`CredentialsMode::Include`].
    pub allow_credentials: bool,
    /// Cached lifetime, capped to [`MAX_AGE_CAP_SECONDS`].  When
    /// `Duration::ZERO`, callers must NOT cache the entry.
    pub max_age: Duration,
}

/// Decide whether a request requires a CORS preflight per
/// WHATWG Fetch §4.8.1 ("CORS-preflight fetch flag").
///
/// Returns `true` iff:
/// - request is `Cors` mode AND
/// - request is cross-origin AND
/// - method is **not** in `{GET, HEAD, POST}`, OR
/// - any author-specified header is **not**
///   CORS-safelisted-request-header (§4.6.5).
pub fn requires_preflight(request: &Request) -> bool {
    if request.mode != RequestMode::Cors {
        return false;
    }
    if is_same_origin(request) {
        return false;
    }
    if !is_cors_safelisted_method(&request.method) {
        return true;
    }
    request
        .headers
        .iter()
        .any(|(name, value)| !is_cors_safelisted_request_header(name, value))
}

/// Same-origin check between `request.origin` and
/// `request.url.origin()`.
///
/// `origin == None` (embedder-driven loads) returns `false` so
/// the broker conservatively never preflights an embedder load
/// — those paths set `mode = NoCors` already, so this branch is
/// only reachable from a misconfigured caller.
fn is_same_origin(request: &Request) -> bool {
    match &request.origin {
        Some(origin) => *origin == request.url.origin(),
        None => false,
    }
}

/// CORS-safelisted method check (§4.6.4): `GET`, `HEAD`, `POST`
/// (case-sensitive per spec — methods are normalised earlier).
fn is_cors_safelisted_method(method: &str) -> bool {
    matches!(method, "GET" | "HEAD" | "POST")
}

/// CORS-safelisted-request-header check (WHATWG Fetch §4.6.5).
///
/// A header is safelisted iff:
/// - name (case-insensitive) is in `{Accept, Accept-Language,
///   Content-Language, Content-Type, Range}` AND
/// - the **value** matches the per-name shape constraints below.
///
/// Special cases:
/// - `Authorization` (§4.6.5 step 4) is **always non-safelisted**
///   regardless of value — it triggers preflight.
/// - `Content-Type` value must parse to one of three MIME types
///   (`application/x-www-form-urlencoded`, `multipart/form-data`,
///   `text/plain`); other values trigger preflight.
/// - `Range` value must match `bytes=N-` or `bytes=N-M` form.
/// - `Accept` / `Accept-Language` / `Content-Language` values
///   must contain only the §4.6.5 byte set (subset of ASCII
///   excluding CORS-unsafe-request-header-byte).
pub fn is_cors_safelisted_request_header(name: &str, value: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if lower == "authorization" {
        return false;
    }
    if value.len() > 128 {
        return false;
    }
    match lower.as_str() {
        "accept" | "accept-language" | "content-language" => {
            value.bytes().all(is_cors_unsafe_request_header_byte_safe)
        }
        "content-type" => is_safelisted_content_type(value),
        "range" => is_safelisted_range(value),
        _ => false,
    }
}

/// Check that a byte is **not** a CORS-unsafe-request-header-byte
/// (§4.6.5).  The unsafe set is `0x00-0x08`, `0x10-0x19`, `"`,
/// `(`, `)`, `:`, `<`, `>`, `?`, `@`, `[`, `\`, `]`, `{`, `}`,
/// `0x7F`.
fn is_cors_unsafe_request_header_byte_safe(b: u8) -> bool {
    !matches!(
        b,
        0x00..=0x08
        | 0x10..=0x19
        | b'"'
        | b'('
        | b')'
        | b':'
        | b'<'
        | b'>'
        | b'?'
        | b'@'
        | b'['
        | b'\\'
        | b']'
        | b'{'
        | b'}'
        | 0x7F
    )
}

/// `Content-Type` safelist check.  The MIME type (before any
/// `;` parameters) must match `application/x-www-form-urlencoded`,
/// `multipart/form-data`, or `text/plain` (case-insensitive).
fn is_safelisted_content_type(value: &str) -> bool {
    let mime = value.split(';').next().unwrap_or("").trim();
    matches!(
        mime.to_ascii_lowercase().as_str(),
        "application/x-www-form-urlencoded" | "multipart/form-data" | "text/plain"
    )
}

/// `Range` safelist check (§4.6.5): only `bytes=N-` or
/// `bytes=N-M` with non-negative integers, no multi-range.
fn is_safelisted_range(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("bytes=") else {
        return false;
    };
    let mut parts = rest.splitn(2, '-');
    let Some(first) = parts.next() else {
        return false;
    };
    let Some(second) = parts.next() else {
        return false;
    };
    if first.is_empty() {
        return false;
    }
    if !first.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    if second.is_empty() {
        return true;
    }
    second.bytes().all(|b| b.is_ascii_digit())
}

/// Build the OPTIONS preflight request for an actual cross-origin
/// CORS request (WHATWG Fetch §4.8 steps 1-9).
///
/// The preflight is always:
/// - method = `OPTIONS`
/// - URL = original request's URL (fragment dropped by `url::Url`)
/// - body = empty
/// - mode = `NoCors` (the preflight is internal — never itself
///   subject to CORS gating)
/// - credentials = `Omit` (preflight is never credentialed; the
///   `Access-Control-Allow-Credentials: true` response header
///   gates the **actual** request)
/// - redirect = `Error` (3xx responses to a preflight are network
///   errors per §4.8 step 11)
/// - headers = `Access-Control-Request-Method` (ACRM) +
///   `Access-Control-Request-Headers` (ACRH, lowercased + sorted
///   non-safelisted header names) + `Origin`
pub fn build_preflight_request(orig: &Request) -> Request {
    let mut headers = vec![(
        "Access-Control-Request-Method".to_string(),
        orig.method.clone(),
    )];

    let acrh = collect_acrh_value(&orig.headers);
    if !acrh.is_empty() {
        headers.push(("Access-Control-Request-Headers".to_string(), acrh));
    }

    if let Some(origin) = &orig.origin {
        headers.push(("Origin".to_string(), origin.ascii_serialization()));
    }

    Request {
        method: "OPTIONS".to_string(),
        url: orig.url.clone(),
        headers,
        body: Bytes::new(),
        origin: orig.origin.clone(),
        redirect: RedirectMode::Error,
        credentials: CredentialsMode::Omit,
        mode: RequestMode::NoCors,
    }
}

/// Collect the lowercased + sorted comma-joined non-safelisted
/// header-name list for `Access-Control-Request-Headers` (§4.6.5
/// + §4.8 step 5).
///
/// Caller passes the actual request headers; safelisted names
/// (and `Authorization` in safelisted positions, but it's never
/// safelisted) are filtered out, the remaining names are
/// lowercased, deduplicated, sorted alphabetically, and joined
/// with `,` (no whitespace per spec).
fn collect_acrh_value(headers: &[(String, String)]) -> String {
    let mut names: Vec<String> = headers
        .iter()
        .filter(|(name, value)| !is_cors_safelisted_request_header(name, value))
        .map(|(name, _)| name.to_ascii_lowercase())
        .collect();
    names.sort();
    names.dedup();
    names.join(",")
}

/// Validate the OPTIONS preflight response per WHATWG Fetch §4.8
/// steps 11-21.  Returns a [`PreflightAllowance`] on success, or
/// `NetError(CorsBlocked)` on any spec-required failure.
pub fn validate_preflight_response(
    orig: &Request,
    resp: &Response,
) -> Result<PreflightAllowance, NetError> {
    if !(200..300).contains(&resp.status) {
        return Err(NetError::new(
            NetErrorKind::CorsBlocked,
            format!("preflight: status {} is not 2xx", resp.status),
        ));
    }

    let credentialed = orig.credentials == CredentialsMode::Include;

    // ACAO check
    let acao = header_value(&resp.headers, "access-control-allow-origin");
    let acao = acao.ok_or_else(|| {
        NetError::new(
            NetErrorKind::CorsBlocked,
            "preflight: missing Access-Control-Allow-Origin",
        )
    })?;
    let request_origin = orig
        .origin
        .as_ref()
        .map(url::Origin::ascii_serialization)
        .unwrap_or_default();
    match acao.as_str() {
        "*" if credentialed => {
            return Err(NetError::new(
                NetErrorKind::CorsBlocked,
                "preflight: Access-Control-Allow-Origin '*' rejected for credentialed request",
            ));
        }
        "*" => {}
        allowed if allowed.eq_ignore_ascii_case(&request_origin) => {}
        allowed => {
            return Err(NetError::new(
                NetErrorKind::CorsBlocked,
                format!(
                    "preflight: Access-Control-Allow-Origin '{allowed}' does not match origin '{request_origin}'"
                ),
            ));
        }
    }

    // ACAC check
    let allow_credentials_raw = header_value(&resp.headers, "access-control-allow-credentials");
    let allow_credentials = allow_credentials_raw.as_deref() == Some("true");
    if credentialed && !allow_credentials {
        return Err(NetError::new(
            NetErrorKind::CorsBlocked,
            "preflight: credentialed request requires Access-Control-Allow-Credentials: true",
        ));
    }

    // ACAM check (allowed methods)
    let allow_methods_raw = header_value(&resp.headers, "access-control-allow-methods");
    let allowed_methods = parse_method_list(allow_methods_raw.as_deref(), credentialed)?;
    method_must_be_allowed(&orig.method, allowed_methods.as_ref())?;

    // ACAH check (allowed headers)
    let allow_headers_raw = header_value(&resp.headers, "access-control-allow-headers");
    let allowed_headers = parse_header_list(allow_headers_raw.as_deref(), credentialed)?;
    headers_must_be_allowed(&orig.headers, allowed_headers.as_ref())?;

    // Max-Age
    let max_age = parse_max_age(header_value(&resp.headers, "access-control-max-age").as_deref());

    Ok(PreflightAllowance {
        allowed_methods,
        allowed_headers,
        allow_credentials,
        max_age,
    })
}

/// Re-validate the actual request against a cached
/// [`PreflightAllowance`].  Used by the cache-hit path to skip
/// the OPTIONS round-trip while still rejecting requests whose
/// method or headers are not covered by the cached allowance.
pub fn validate_actual_against_allowance(
    orig: &Request,
    allowance: &PreflightAllowance,
) -> Result<(), NetError> {
    if orig.credentials == CredentialsMode::Include && !allowance.allow_credentials {
        return Err(NetError::new(
            NetErrorKind::CorsBlocked,
            "preflight cache: credentialed request requires Access-Control-Allow-Credentials: true",
        ));
    }
    method_must_be_allowed(&orig.method, allowance.allowed_methods.as_ref())?;
    headers_must_be_allowed(&orig.headers, allowance.allowed_headers.as_ref())?;
    Ok(())
}

fn header_value(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.trim().to_string())
}

/// Parse `Access-Control-Allow-Methods`.  Returns:
/// - `Ok(None)` for `*` (allowed only for non-credentialed),
/// - `Ok(Some(list))` for explicit comma-separated methods,
/// - `Err(CorsBlocked)` for `*` with credentialed.
fn parse_method_list(
    raw: Option<&str>,
    credentialed: bool,
) -> Result<Option<Vec<String>>, NetError> {
    let Some(raw) = raw else {
        // §4.8 step 16 — no ACAM means no extra methods beyond
        // safelisted ones; an empty list rejects non-safelisted
        // methods (the typical preflight trigger).
        return Ok(Some(Vec::new()));
    };
    let trimmed = raw.trim();
    if trimmed == "*" {
        if credentialed {
            return Err(NetError::new(
                NetErrorKind::CorsBlocked,
                "preflight: Access-Control-Allow-Methods '*' rejected for credentialed request",
            ));
        }
        return Ok(None);
    }
    Ok(Some(
        trimmed
            .split(',')
            .map(|s| s.trim().to_ascii_uppercase())
            .filter(|s| !s.is_empty())
            .collect(),
    ))
}

/// Parse `Access-Control-Allow-Headers`.  Same shape as
/// [`parse_method_list`] but lowercased for case-insensitive
/// comparison against actual request header names.
fn parse_header_list(
    raw: Option<&str>,
    credentialed: bool,
) -> Result<Option<Vec<String>>, NetError> {
    let Some(raw) = raw else {
        return Ok(Some(Vec::new()));
    };
    let trimmed = raw.trim();
    if trimmed == "*" {
        if credentialed {
            return Err(NetError::new(
                NetErrorKind::CorsBlocked,
                "preflight: Access-Control-Allow-Headers '*' rejected for credentialed request",
            ));
        }
        return Ok(None);
    }
    Ok(Some(
        trimmed
            .split(',')
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect(),
    ))
}

/// Check that the actual method is in the parsed allow-methods
/// list (or the list is `None`, i.e. wildcard).  CORS-safelisted
/// methods are always allowed regardless of ACAM (spec §4.8
/// step 17).
fn method_must_be_allowed(
    method: &str,
    allowed_methods: Option<&Vec<String>>,
) -> Result<(), NetError> {
    if is_cors_safelisted_method(method) {
        return Ok(());
    }
    let Some(list) = allowed_methods else {
        return Ok(());
    };
    let upper = method.to_ascii_uppercase();
    if list.iter().any(|m| m == &upper) {
        return Ok(());
    }
    Err(NetError::new(
        NetErrorKind::CorsBlocked,
        format!("preflight: method '{method}' not in Access-Control-Allow-Methods"),
    ))
}

/// Check that every author-specified non-safelisted header is in
/// the parsed allow-headers list (or the list is `None`).
/// Safelisted headers are always allowed regardless of ACAH (spec
/// §4.8 step 18).
fn headers_must_be_allowed(
    actual_headers: &[(String, String)],
    allowed_headers: Option<&Vec<String>>,
) -> Result<(), NetError> {
    let Some(list) = allowed_headers else {
        return Ok(());
    };
    for (name, value) in actual_headers {
        if is_cors_safelisted_request_header(name, value) {
            continue;
        }
        let lower = name.to_ascii_lowercase();
        if !list.iter().any(|h| h == &lower) {
            return Err(NetError::new(
                NetErrorKind::CorsBlocked,
                format!("preflight: header '{name}' not in Access-Control-Allow-Headers"),
            ));
        }
    }
    Ok(())
}

/// Parse `Access-Control-Max-Age` per §4.8 step 19.
///
/// - Missing → [`DEFAULT_MAX_AGE_SECONDS`]
/// - Negative or non-numeric → `Duration::ZERO` (don't cache)
/// - Capped to [`MAX_AGE_CAP_SECONDS`]
fn parse_max_age(raw: Option<&str>) -> Duration {
    let Some(raw) = raw else {
        return Duration::from_secs(DEFAULT_MAX_AGE_SECONDS);
    };
    let trimmed = raw.trim();
    // Reject if not a non-negative integer (negatives + floats +
    // empty all become "don't cache").
    let secs = match trimmed.parse::<i64>() {
        Ok(n) if n > 0 => n.cast_unsigned(),
        _ => return Duration::ZERO,
    };
    Duration::from_secs(secs.min(MAX_AGE_CAP_SECONDS))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req_with(method: &str, url: &str, origin: &str, headers: Vec<(String, String)>) -> Request {
        Request {
            method: method.to_string(),
            url: url::Url::parse(url).unwrap(),
            headers,
            body: Bytes::new(),
            origin: Some(url::Url::parse(origin).unwrap().origin()),
            mode: RequestMode::Cors,
            ..Default::default()
        }
    }

    #[test]
    fn requires_preflight_simple_get_returns_false() {
        let r = req_with(
            "GET",
            "https://api.other.com/",
            "https://example.com/",
            vec![],
        );
        assert!(!requires_preflight(&r));
    }

    #[test]
    fn requires_preflight_same_origin_returns_false() {
        let r = req_with(
            "DELETE",
            "https://example.com/data",
            "https://example.com/",
            vec![],
        );
        assert!(!requires_preflight(&r));
    }

    #[test]
    fn requires_preflight_no_cors_mode_returns_false() {
        let mut r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![],
        );
        r.mode = RequestMode::NoCors;
        assert!(!requires_preflight(&r));
    }

    #[test]
    fn requires_preflight_custom_method_returns_true() {
        let r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![],
        );
        assert!(requires_preflight(&r));
    }

    #[test]
    fn requires_preflight_delete_returns_true() {
        let r = req_with(
            "DELETE",
            "https://api.other.com/",
            "https://example.com/",
            vec![],
        );
        assert!(requires_preflight(&r));
    }

    #[test]
    fn requires_preflight_custom_header_returns_true() {
        let r = req_with(
            "GET",
            "https://api.other.com/",
            "https://example.com/",
            vec![("X-Custom".into(), "1".into())],
        );
        assert!(requires_preflight(&r));
    }

    #[test]
    fn requires_preflight_authorization_header_returns_true() {
        let r = req_with(
            "GET",
            "https://api.other.com/",
            "https://example.com/",
            vec![("Authorization".into(), "Bearer token".into())],
        );
        assert!(requires_preflight(&r));
    }

    #[test]
    fn requires_preflight_safelisted_content_type_form_urlencoded() {
        let r = req_with(
            "POST",
            "https://api.other.com/",
            "https://example.com/",
            vec![(
                "Content-Type".into(),
                "application/x-www-form-urlencoded".into(),
            )],
        );
        assert!(!requires_preflight(&r));
    }

    #[test]
    fn requires_preflight_safelisted_content_type_multipart() {
        let r = req_with(
            "POST",
            "https://api.other.com/",
            "https://example.com/",
            vec![(
                "Content-Type".into(),
                "multipart/form-data; boundary=abc".into(),
            )],
        );
        assert!(!requires_preflight(&r));
    }

    #[test]
    fn requires_preflight_safelisted_content_type_text_plain() {
        let r = req_with(
            "POST",
            "https://api.other.com/",
            "https://example.com/",
            vec![("Content-Type".into(), "text/plain;charset=utf-8".into())],
        );
        assert!(!requires_preflight(&r));
    }

    #[test]
    fn requires_preflight_unsafe_content_type_application_json() {
        let r = req_with(
            "POST",
            "https://api.other.com/",
            "https://example.com/",
            vec![("Content-Type".into(), "application/json".into())],
        );
        assert!(requires_preflight(&r));
    }

    #[test]
    fn safelisted_range_simple() {
        assert!(is_cors_safelisted_request_header("Range", "bytes=0-100"));
        assert!(is_cors_safelisted_request_header("Range", "bytes=500-"));
        assert!(!is_cors_safelisted_request_header(
            "Range",
            "bytes=0-100,200-300"
        ));
        assert!(!is_cors_safelisted_request_header("Range", "items=0-100"));
    }

    #[test]
    fn safelisted_value_length_limit() {
        // 129 bytes is over the 128-byte limit
        let long = "a".repeat(129);
        assert!(!is_cors_safelisted_request_header("Accept", &long));
    }

    #[test]
    fn safelisted_accept_rejects_unsafe_byte() {
        // ':' is in the unsafe byte set
        assert!(!is_cors_safelisted_request_header(
            "Accept",
            "text/html: bad"
        ));
    }

    #[test]
    fn build_preflight_uses_options_method_and_omit_credentials() {
        let r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![("X-Custom".into(), "1".into())],
        );
        let p = build_preflight_request(&r);
        assert_eq!(p.method, "OPTIONS");
        assert_eq!(p.credentials, CredentialsMode::Omit);
        assert_eq!(p.redirect, RedirectMode::Error);
        assert_eq!(p.mode, RequestMode::NoCors);
        assert!(p.body.is_empty());
    }

    #[test]
    fn build_preflight_includes_acrm() {
        let r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![],
        );
        let p = build_preflight_request(&r);
        let acrm = header_value(&p.headers, "access-control-request-method").unwrap();
        assert_eq!(acrm, "PUT");
    }

    #[test]
    fn build_preflight_acrh_lowercase_sorted() {
        let r = req_with(
            "POST",
            "https://api.other.com/",
            "https://example.com/",
            vec![
                ("X-Zebra".into(), "1".into()),
                ("X-Alpha".into(), "1".into()),
                ("Authorization".into(), "Bearer t".into()),
            ],
        );
        let p = build_preflight_request(&r);
        let acrh = header_value(&p.headers, "access-control-request-headers").unwrap();
        assert_eq!(acrh, "authorization,x-alpha,x-zebra");
    }

    #[test]
    fn build_preflight_acrh_omits_safelisted_names() {
        let r = req_with(
            "POST",
            "https://api.other.com/",
            "https://example.com/",
            vec![
                ("Accept".into(), "text/plain".into()),
                ("Content-Type".into(), "text/plain".into()),
                ("X-Custom".into(), "1".into()),
            ],
        );
        let p = build_preflight_request(&r);
        let acrh = header_value(&p.headers, "access-control-request-headers").unwrap();
        assert_eq!(acrh, "x-custom");
    }

    #[test]
    fn build_preflight_skips_acrh_when_no_unsafe_headers() {
        let r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![("Accept".into(), "text/plain".into())],
        );
        let p = build_preflight_request(&r);
        assert!(header_value(&p.headers, "access-control-request-headers").is_none());
    }

    #[test]
    fn build_preflight_includes_origin_header() {
        let r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/page",
            vec![],
        );
        let p = build_preflight_request(&r);
        let origin = header_value(&p.headers, "origin").unwrap();
        assert_eq!(origin, "https://example.com");
    }

    fn resp(status: u16, headers: Vec<(&str, &str)>) -> Response {
        Response {
            status,
            headers: headers
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            body: Bytes::new(),
            url: url::Url::parse("https://api.other.com/").unwrap(),
            version: crate::HttpVersion::H1,
            url_list: Vec::new(),
        }
    }

    #[test]
    fn validate_response_status_must_be_2xx() {
        let r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![],
        );
        let response = resp(403, vec![]);
        let err = validate_preflight_response(&r, &response).unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    }

    #[test]
    fn validate_response_acao_match() {
        let r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![],
        );
        let response = resp(
            204,
            vec![
                ("Access-Control-Allow-Origin", "https://example.com"),
                ("Access-Control-Allow-Methods", "PUT, DELETE"),
            ],
        );
        let allowance = validate_preflight_response(&r, &response).unwrap();
        assert!(allowance.allowed_methods.is_some());
    }

    #[test]
    fn validate_response_acao_mismatch_fails() {
        let r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![],
        );
        let response = resp(
            204,
            vec![("Access-Control-Allow-Origin", "https://attacker.com")],
        );
        let err = validate_preflight_response(&r, &response).unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    }

    #[test]
    fn validate_response_acao_wildcard_credentialed_fails() {
        let mut r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![],
        );
        r.credentials = CredentialsMode::Include;
        let response = resp(
            204,
            vec![
                ("Access-Control-Allow-Origin", "*"),
                ("Access-Control-Allow-Credentials", "true"),
                ("Access-Control-Allow-Methods", "PUT"),
            ],
        );
        let err = validate_preflight_response(&r, &response).unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    }

    #[test]
    fn validate_response_credentialed_requires_acac_true() {
        let mut r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![],
        );
        r.credentials = CredentialsMode::Include;
        let response = resp(
            204,
            vec![
                ("Access-Control-Allow-Origin", "https://example.com"),
                ("Access-Control-Allow-Methods", "PUT"),
            ],
        );
        let err = validate_preflight_response(&r, &response).unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
        assert!(err.message.contains("Access-Control-Allow-Credentials"));
    }

    #[test]
    fn validate_response_method_not_allowed_fails() {
        let r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![],
        );
        let response = resp(
            204,
            vec![
                ("Access-Control-Allow-Origin", "https://example.com"),
                ("Access-Control-Allow-Methods", "DELETE"),
            ],
        );
        let err = validate_preflight_response(&r, &response).unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    }

    #[test]
    fn validate_response_method_wildcard_non_credentialed_passes() {
        let r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![],
        );
        let response = resp(
            204,
            vec![
                ("Access-Control-Allow-Origin", "https://example.com"),
                ("Access-Control-Allow-Methods", "*"),
            ],
        );
        let allowance = validate_preflight_response(&r, &response).unwrap();
        assert!(allowance.allowed_methods.is_none());
    }

    #[test]
    fn validate_response_header_not_allowed_fails() {
        let r = req_with(
            "GET",
            "https://api.other.com/",
            "https://example.com/",
            vec![("X-Custom".into(), "1".into())],
        );
        let response = resp(
            204,
            vec![
                ("Access-Control-Allow-Origin", "https://example.com"),
                ("Access-Control-Allow-Headers", "x-other"),
            ],
        );
        let err = validate_preflight_response(&r, &response).unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    }

    #[test]
    fn validate_response_header_allowed_passes() {
        let r = req_with(
            "GET",
            "https://api.other.com/",
            "https://example.com/",
            vec![("X-Custom".into(), "1".into())],
        );
        let response = resp(
            204,
            vec![
                ("Access-Control-Allow-Origin", "https://example.com"),
                ("Access-Control-Allow-Headers", "x-custom, x-other"),
            ],
        );
        assert!(validate_preflight_response(&r, &response).is_ok());
    }

    #[test]
    fn validate_response_max_age_capped() {
        let r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![],
        );
        let response = resp(
            204,
            vec![
                ("Access-Control-Allow-Origin", "https://example.com"),
                ("Access-Control-Allow-Methods", "PUT"),
                ("Access-Control-Max-Age", "999999"),
            ],
        );
        let allowance = validate_preflight_response(&r, &response).unwrap();
        assert_eq!(allowance.max_age, Duration::from_secs(MAX_AGE_CAP_SECONDS));
    }

    #[test]
    fn validate_response_max_age_negative_zero() {
        let r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![],
        );
        let response = resp(
            204,
            vec![
                ("Access-Control-Allow-Origin", "https://example.com"),
                ("Access-Control-Allow-Methods", "PUT"),
                ("Access-Control-Max-Age", "-5"),
            ],
        );
        let allowance = validate_preflight_response(&r, &response).unwrap();
        assert_eq!(allowance.max_age, Duration::ZERO);
    }

    #[test]
    fn validate_response_max_age_default_when_missing() {
        let r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![],
        );
        let response = resp(
            204,
            vec![
                ("Access-Control-Allow-Origin", "https://example.com"),
                ("Access-Control-Allow-Methods", "PUT"),
            ],
        );
        let allowance = validate_preflight_response(&r, &response).unwrap();
        assert_eq!(
            allowance.max_age,
            Duration::from_secs(DEFAULT_MAX_AGE_SECONDS)
        );
    }

    #[test]
    fn validate_actual_against_allowance_credentialed_requires_acac() {
        let mut r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![],
        );
        r.credentials = CredentialsMode::Include;
        let allowance = PreflightAllowance {
            allowed_methods: Some(vec!["PUT".into()]),
            allowed_headers: Some(Vec::new()),
            allow_credentials: false,
            max_age: Duration::from_secs(60),
        };
        assert!(validate_actual_against_allowance(&r, &allowance).is_err());
    }

    #[test]
    fn validate_actual_against_allowance_method_must_match() {
        let r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![],
        );
        let allowance = PreflightAllowance {
            allowed_methods: Some(vec!["DELETE".into()]),
            allowed_headers: Some(Vec::new()),
            allow_credentials: false,
            max_age: Duration::from_secs(60),
        };
        assert!(validate_actual_against_allowance(&r, &allowance).is_err());
    }

    #[test]
    fn validate_actual_against_allowance_safelisted_method_always_allowed() {
        let r = req_with(
            "GET",
            "https://api.other.com/",
            "https://example.com/",
            vec![("X-Custom".into(), "1".into())],
        );
        let allowance = PreflightAllowance {
            allowed_methods: Some(Vec::new()),
            allowed_headers: Some(vec!["x-custom".into()]),
            allow_credentials: false,
            max_age: Duration::from_secs(60),
        };
        assert!(validate_actual_against_allowance(&r, &allowance).is_ok());
    }
}
