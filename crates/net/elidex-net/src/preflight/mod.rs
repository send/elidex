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
//!    [`crate::error::NetErrorKind::CorsBlocked`] error.
//! 4. [`validate_actual_against_allowance`] is the second-stage
//!    check that re-asserts the actual request's method/headers
//!    against the cached allowance (so a cache hit short-circuits
//!    the OPTIONS round-trip).
//!
//! `cors.rs` (the pre-existing module) is intentionally separate
//! — it implements only the §4.4 `Access-Control-Allow-Origin`
//! check on the actual response, not preflight.
//!
//! ## File layout
//!
//! Originally a single `preflight.rs` (~1758 LoC); split out into
//! the project's standard 1000-line file convention as part of
//! M4-12 PR-file-split-b (slot #10.5b):
//!
//! - [`builder`] — [`build_preflight_request`] + ACRH collection.
//! - [`validator`] — [`validate_preflight_response`] +
//!   [`validate_actual_against_allowance`] + private parsing helpers.
//! - [`cache`] — [`PreflightCache`] and [`PreflightCacheKey`]
//!   (was the sibling `preflight_cache.rs` before the split;
//!   merged into this module so the cache lives next to its
//!   keys/values).

pub mod builder;
pub mod cache;
pub mod validator;

pub use builder::build_preflight_request;
pub use cache::{PreflightCache, PreflightCacheKey};
pub use validator::{validate_actual_against_allowance, validate_preflight_response};

use std::time::Duration;

use crate::error::{NetError, NetErrorKind};
use crate::transport::HttpTransport;
use crate::{Request, RequestMode};

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
/// [`PreflightCache`] so subsequent requests with the same
/// `(origin, url, method, header-set)` key can skip the OPTIONS
/// round-trip.
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
    /// [`crate::CredentialsMode::Include`].
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
        .any(|(name, value)| is_non_safelisted_author_header(name, value))
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
pub(super) fn is_cors_safelisted_method(method: &str) -> bool {
    matches!(method, "GET" | "HEAD" | "POST")
}

/// Header names that the broker / VM-side fetch path auto-injects
/// (NOT author-controllable per WHATWG Fetch §4.6 forbidden-request-
/// header list + the broker's Origin / Referer attachments in
/// `crates/script/elidex-js/src/vm/host/fetch.rs::attach_default_origin`
/// / `::attach_default_referer`).
///
/// These headers MUST be excluded from:
/// - the §4.8.1 preflight detection ([`requires_preflight`])
/// - the `Access-Control-Request-Headers` enumeration
///   (the per-request `collect_acrh_value` helper inside [`builder`])
/// - the `Access-Control-Allow-Headers` validation
///   (the `headers_must_be_allowed` helper inside [`validator`])
/// - the [`PreflightCacheKey`] header set
///
/// Otherwise a normal cross-origin `fetch()` would be classified
/// as non-simple (because `request.headers` carries `Origin` /
/// `Referer`), force a preflight, and require servers to list
/// `origin` / `referer` in `Access-Control-Allow-Headers` —
/// neither of which is spec-compliant.
pub fn is_broker_injected_header(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        // §4.6 forbidden + broker auto-injected.
        "origin"
            | "referer"
            | "host"
            | "content-length"
            | "connection"
            | "transfer-encoding"
            | "upgrade"
            | "user-agent"
            | "cookie"
    )
}

/// "Author-specified non-safelisted-request-header" predicate: the
/// header participates in §4.8 preflight decisions iff it is both
/// (a) author-controlled (not [`is_broker_injected_header`]) AND
/// (b) not [`is_cors_safelisted_request_header`].
pub(super) fn is_non_safelisted_author_header(name: &str, value: &str) -> bool {
    !is_broker_injected_header(name) && !is_cors_safelisted_request_header(name, value)
}

/// CORS-safelisted-request-header check (WHATWG Fetch §4.6.5).
///
/// A header is safelisted iff:
/// - name (case-insensitive) is in `{Accept, Accept-Language,
///   Content-Language, Content-Type, Range, Save-Data}` AND
/// - the **value** matches the per-name shape constraints below.
///
/// Special cases:
/// - `Authorization` (§4.6.5 step 4) is **always non-safelisted**
///   regardless of value — it triggers preflight.
/// - `Content-Type` value must (a) contain only safe bytes (no
///   CORS-unsafe-request-header-byte such as `:` outside MIME
///   delimiters — Copilot R6) AND (b) parse to one of three
///   MIME types (`application/x-www-form-urlencoded`,
///   `multipart/form-data`, `text/plain`); other values trigger
///   preflight.
/// - `Range` value must match `bytes=N-` or `bytes=N-M` form.
/// - `Accept` / `Accept-Language` / `Content-Language` /
///   `Save-Data` values must contain only the §4.6.5 byte set
///   (subset of ASCII excluding CORS-unsafe-request-header-byte).
///
/// **Note**: this function answers the per-header §4.6.5 question
/// in isolation; callers that need the "is this header an
/// author-specified non-safelisted header (so it counts toward
/// preflight)" question should compose with [`is_broker_injected_header`]
/// (e.g. `!is_broker_injected_header(name) && !is_cors_safelisted_request_header(name, value)`)
/// to additionally filter out auto-injected headers like `Origin` /
/// `Referer`.
pub fn is_cors_safelisted_request_header(name: &str, value: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if lower == "authorization" {
        return false;
    }
    if value.len() > 128 {
        return false;
    }
    match lower.as_str() {
        "accept" | "accept-language" | "content-language" | "save-data" => {
            value.bytes().all(is_cors_unsafe_request_header_byte_safe)
        }
        "content-type" => {
            value.bytes().all(is_cors_unsafe_request_header_byte_safe)
                && is_safelisted_content_type(value)
        }
        "range" => is_safelisted_range(value),
        _ => false,
    }
}

/// Check that a byte is **not** a CORS-unsafe-request-header-byte
/// (WHATWG Fetch §4.6.5).
///
/// A byte is **unsafe** iff:
/// - it is less than `0x20` AND not `0x09` (HT), OR
/// - it is one of `"`, `(`, `)`, `:`, `<`, `>`, `?`, `@`, `[`,
///   `\`, `]`, `{`, `}`, `0x7F` (DEL).
///
/// Therefore newline (`0x0A`), carriage return (`0x0D`), and the
/// rest of the C0 control range (`0x00..=0x08` + `0x0B` + `0x0C` +
/// `0x0E..=0x1F`) are all unsafe — they could enable header
/// injection if accepted into safelisted-header values
/// (Copilot R4 PR #134).
fn is_cors_unsafe_request_header_byte_safe(b: u8) -> bool {
    if b < 0x20 && b != 0x09 {
        return false;
    }
    !matches!(
        b,
        b'"' | b'('
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

/// Run the CORS preflight stage for `request` against `transport`,
/// using `cache` to short-circuit the OPTIONS round-trip on a hit.
/// Either:
///
/// - hits the cache (no network round-trip; just re-validate the
///   actual request against the cached allowance), or
/// - dispatches an OPTIONS preflight, validates the response, and
///   stores the parsed allowance for subsequent same-key requests.
///
/// On any spec-required failure (preflight non-2xx, ACAO mismatch,
/// method / header not allowed, ACAC mismatch for credentialed)
/// returns [`NetErrorKind::CorsBlocked`].  On a successful return
/// the caller is free to dispatch the actual request.
///
/// Shared by [`crate::NetClient::send`] (initial preflight) and
/// the broker's redirect loop (re-issued preflight on a
/// cross-origin redirect target — WHATWG Fetch §4.4 step 14).
/// The free-function form lets both call sites share the cache +
/// validation logic without coupling to `NetClient`.
pub async fn run_preflight(
    transport: &HttpTransport,
    cache: &PreflightCache,
    request: &Request,
    cancel: Option<&crate::CancelHandle>,
) -> Result<(), NetError> {
    let Some(key) = PreflightCacheKey::from_request(request) else {
        // `run_preflight` is only entered after `requires_preflight`
        // returned true, which already requires `mode = Cors` plus
        // a populated `request.origin`.  Reaching here without an
        // origin means the cors-mode entry guard upstream let a
        // misconfigured request through — fail closed rather than
        // silently bypass §4.8 (Copilot R2 PR #134).
        return Err(NetError::new(
            NetErrorKind::CorsBlocked,
            "preflight: cors-mode request reached preflight stage without origin context",
        ));
    };
    if let Some(allowance) = cache.lookup(&key) {
        return validate_actual_against_allowance(request, &allowance);
    }
    let preflight_req = build_preflight_request(request);
    let preflight_resp = transport.send(&preflight_req, cancel).await?;
    let allowance = validate_preflight_response(request, &preflight_resp)?;
    // Re-validate the actual request before storing — the cache
    // should never hold an entry that the actual request itself
    // can't satisfy.
    validate_actual_against_allowance(request, &allowance)?;
    cache.store(key, allowance);
    Ok(())
}

#[cfg(test)]
pub(super) fn req_with(
    method: &str,
    url: &str,
    origin: &str,
    headers: Vec<(String, String)>,
) -> Request {
    Request {
        method: method.to_string(),
        url: url::Url::parse(url).unwrap(),
        headers,
        body: bytes::Bytes::new(),
        origin: Some(url::Url::parse(origin).unwrap().origin()),
        mode: RequestMode::Cors,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// Regression for Copilot R1 finding 1 + 4: broker-injected
    /// `Origin` / `Referer` (auto-injected by `attach_default_origin`
    /// / `attach_default_referer` on the VM-side fetch path) must
    /// NOT count toward the §4.8.1 preflight detection — they are
    /// not author-controllable headers.  Without the filter, a
    /// normal cross-origin `fetch()` would force a preflight just
    /// because the broker put `Origin` into `request.headers`.
    #[test]
    fn requires_preflight_skips_broker_injected_origin() {
        let r = req_with(
            "GET",
            "https://api.other.com/",
            "https://example.com/",
            vec![("Origin".into(), "https://example.com".into())],
        );
        assert!(!requires_preflight(&r));
    }

    #[test]
    fn requires_preflight_skips_broker_injected_referer() {
        let r = req_with(
            "GET",
            "https://api.other.com/",
            "https://example.com/",
            vec![("Referer".into(), "https://example.com/page".into())],
        );
        assert!(!requires_preflight(&r));
    }

    #[test]
    fn requires_preflight_skips_broker_injected_origin_and_referer_combined() {
        // Both auto-injected headers + safelisted Accept → still
        // simple, no preflight.
        let r = req_with(
            "GET",
            "https://api.other.com/",
            "https://example.com/",
            vec![
                ("Origin".into(), "https://example.com".into()),
                ("Referer".into(), "https://example.com/page".into()),
                ("Accept".into(), "text/plain".into()),
            ],
        );
        assert!(!requires_preflight(&r));
    }

    /// Sentinel: an unsafe header alongside broker-injected ones
    /// still triggers preflight.
    #[test]
    fn requires_preflight_unsafe_header_with_broker_injected_present() {
        let r = req_with(
            "GET",
            "https://api.other.com/",
            "https://example.com/",
            vec![
                ("Origin".into(), "https://example.com".into()),
                ("X-Custom".into(), "1".into()),
            ],
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

    /// Regression for Copilot R4 finding 1: §4.6.5
    /// CORS-unsafe-request-header-byte set covers the **full**
    /// `0x00..=0x1F` range minus `0x09` (HT).  Previously the
    /// implementation only flagged `0x00..=0x08` and
    /// `0x10..=0x19` as unsafe, which meant `\n` (0x0A), `\r`
    /// (0x0D), and ESC (0x1B) etc. would be considered safe in
    /// safelisted-header values — a header-injection footgun.
    #[test]
    fn safelisted_accept_rejects_newline() {
        assert!(!is_cors_safelisted_request_header(
            "Accept",
            "text/html\nX-Injected: y"
        ));
    }

    #[test]
    fn safelisted_accept_rejects_carriage_return() {
        assert!(!is_cors_safelisted_request_header(
            "Accept",
            "text/html\rX-Injected: y"
        ));
    }

    #[test]
    fn safelisted_accept_rejects_esc_control() {
        // ESC (0x1B) — was in the previous "safe" 0x1A..=0x1F gap.
        assert!(!is_cors_safelisted_request_header(
            "Accept",
            "text/html\x1bsneaky"
        ));
    }

    /// Sentinel: HT (`0x09`) is the **only** byte below `0x20`
    /// that's safelisted (§4.6.5 explicitly excludes it).
    #[test]
    fn safelisted_accept_allows_horizontal_tab() {
        assert!(is_cors_safelisted_request_header(
            "Accept",
            "text/html\ttext/plain"
        ));
    }

    /// Regression for Copilot R6 finding 1: `Content-Type` value
    /// must pass the byte-check **before** the MIME-prefix
    /// match.  Pre-fix `text/plain; x=y:z` (contains unsafe `:`
    /// in the parameters) was silently classified as safelisted.
    #[test]
    fn safelisted_content_type_rejects_unsafe_byte_in_parameters() {
        assert!(!is_cors_safelisted_request_header(
            "Content-Type",
            "text/plain; x=y:z"
        ));
    }

    /// Sentinel: a Content-Type with safe parameter syntax
    /// (e.g. `; charset=utf-8`) still safelists.
    #[test]
    fn safelisted_content_type_with_safe_params_passes() {
        assert!(is_cors_safelisted_request_header(
            "Content-Type",
            "text/plain; charset=utf-8"
        ));
    }

    /// Regression for Copilot R6 finding 2: `Save-Data` is in
    /// the §4.6.5 safelist (formerly deferred as SP-CORS-4 in
    /// PR-spec-polish; closed inline during R6).  Values must
    /// pass the same byte-check as `Accept` etc.
    #[test]
    fn safelisted_save_data_on() {
        assert!(is_cors_safelisted_request_header("Save-Data", "on"));
    }

    #[test]
    fn safelisted_save_data_rejects_unsafe_byte() {
        assert!(!is_cors_safelisted_request_header(
            "Save-Data",
            "on:malicious"
        ));
    }

    /// Sentinel: a Save-Data header alone does NOT trigger
    /// preflight on a cross-origin simple request.
    #[test]
    fn requires_preflight_save_data_only_returns_false() {
        let r = req_with(
            "GET",
            "https://api.other.com/",
            "https://example.com/",
            vec![("Save-Data".into(), "on".into())],
        );
        assert!(!requires_preflight(&r));
    }
}
