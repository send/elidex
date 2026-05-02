//! Preflight response validation (WHATWG Fetch §4.8 steps 11-21)
//! and the cache-hit re-validator
//! ([`validate_actual_against_allowance`]).

use std::time::Duration;

use super::{
    is_cors_safelisted_method, is_non_safelisted_author_header, PreflightAllowance,
    DEFAULT_MAX_AGE_SECONDS, MAX_AGE_CAP_SECONDS,
};
use crate::error::{NetError, NetErrorKind};
use crate::{CredentialsMode, Request, Response};

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

    // ACAO check (single-valued — duplicates / comma-lists fail
    // closed per Copilot R2 PR #134)
    let acao = header_value_single(&resp.headers, "access-control-allow-origin");
    let acao = acao.ok_or_else(|| {
        NetError::new(
            NetErrorKind::CorsBlocked,
            "preflight: missing or duplicate Access-Control-Allow-Origin",
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

    // ACAC check (case-insensitive ASCII comparison per WHATWG
    // Fetch §4.8 step 14 + parity with `sse/connect.rs::EventSource`
    // header parsing — Copilot R1; single-valued — Copilot R2).
    let allow_credentials_raw =
        header_value_single(&resp.headers, "access-control-allow-credentials");
    let allow_credentials = allow_credentials_raw
        .as_deref()
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));
    if credentialed && !allow_credentials {
        return Err(NetError::new(
            NetErrorKind::CorsBlocked,
            "preflight: credentialed request requires Access-Control-Allow-Credentials: true",
        ));
    }

    // ACAM check (allowed methods).  Same `extracting header
    // list values` failure semantics as Max-Age (Copilot R5
    // PR #134): treat duplicate occurrence as a network error
    // rather than silently falling through to the empty list
    // (which would let a request with a safelisted method pass
    // even when the server emitted contradictory duplicate
    // ACAM headers).
    let allow_methods_raw =
        extract_single_header_or_fail_closed(&resp.headers, "access-control-allow-methods")?;
    let allowed_methods = parse_method_list(allow_methods_raw.as_deref(), credentialed)?;
    method_must_be_allowed(&orig.method, allowed_methods.as_ref())?;

    // ACAH check (allowed headers).  Same duplicate-fail-closed
    // semantics — duplicate ACAH must not silently fall through
    // to an empty list when only safelisted-author headers are
    // present (Copilot R5 PR #134).
    let allow_headers_raw =
        extract_single_header_or_fail_closed(&resp.headers, "access-control-allow-headers")?;
    let allowed_headers = parse_header_list(allow_headers_raw.as_deref(), credentialed)?;
    headers_must_be_allowed(&orig.headers, allowed_headers.as_ref())?;

    // Max-Age (single-valued integer per WHATWG Fetch §4.8 step 19).
    // Duplicate / comma-list / malformed → network error per the
    // §4.8 step 19 "extracting header list values returning
    // failure" contract (Copilot R4 + R5 PR #134).
    let max_age_raw =
        extract_single_header_or_fail_closed(&resp.headers, "access-control-max-age")?;
    let max_age_raw = match max_age_raw {
        None => None,
        Some(raw) if raw.contains(',') => {
            return Err(NetError::new(
                NetErrorKind::CorsBlocked,
                "preflight: malformed Access-Control-Max-Age (comma-list)",
            ));
        }
        Some(raw) => Some(raw),
    };
    let max_age = parse_max_age(max_age_raw.as_deref());

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

/// Look up a header by name, returning the trimmed field-value.
///
/// Returns `None` when the header is absent **or** when more
/// than one occurrence of the name is present in the response —
/// CORS-sensitive headers must not silently merge duplicate
/// occurrences (Copilot R2).  Servers that emit ACAO/ACAC/etc.
/// twice (whether by misconfiguration or deliberate cache-key
/// confusion) must fail the §4.8 validation rather than be
/// accepted with first-match semantics.
pub(super) fn header_value(headers: &[(String, String)], name: &str) -> Option<String> {
    let mut matches = headers.iter().filter(|(k, _)| k.eq_ignore_ascii_case(name));
    let value = matches.next()?.1.trim().to_string();
    if matches.next().is_some() {
        return None;
    }
    Some(value)
}

/// Like [`header_value`] but additionally rejects comma-separated
/// values — used for the **single-valued** CORS response headers
/// (`Access-Control-Allow-Origin`, `Access-Control-Allow-Credentials`,
/// `Access-Control-Max-Age`) where comma in the field-value is
/// either a server bug or an attempt to smuggle a list into a
/// single-value slot.
fn header_value_single(headers: &[(String, String)], name: &str) -> Option<String> {
    let value = header_value(headers, name)?;
    if value.contains(',') {
        return None;
    }
    Some(value)
}

/// Distinguish "header absent" from "header present but
/// duplicate" and surface the latter as a `CorsBlocked` network
/// error per WHATWG Fetch §4.8 step 19 "extracting header list
/// values returning failure".  Returns:
///
/// - `Ok(None)` — header is absent (caller may apply spec default)
/// - `Ok(Some(value))` — exactly one occurrence; trimmed value
/// - `Err(CorsBlocked)` — header has 2+ occurrences (fail closed)
///
/// Used for ACAM / ACAH / Max-Age which all share the same
/// "duplicate is a server bug" semantics — silent fall-through
/// to `None` would let a request with safelisted method/headers
/// pass even when the server emitted contradictory duplicates
/// (Copilot R5 PR #134).
fn extract_single_header_or_fail_closed(
    headers: &[(String, String)],
    name: &str,
) -> Result<Option<String>, NetError> {
    let present = headers.iter().any(|(k, _)| k.eq_ignore_ascii_case(name));
    if !present {
        return Ok(None);
    }
    match header_value(headers, name) {
        Some(v) => Ok(Some(v)),
        None => Err(NetError::new(
            NetErrorKind::CorsBlocked,
            format!("preflight: duplicate {name}"),
        )),
    }
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
        // Skip safelisted headers (always allowed, §4.8 step 18)
        // AND broker-injected ones (`Origin` / `Referer` etc. —
        // not author-specified, so they don't participate in the
        // ACAH allow-list check).  Otherwise compliant servers
        // that don't echo `origin` / `referer` in ACAH would be
        // rejected.
        if !is_non_safelisted_author_header(name, value) {
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
    use super::super::req_with;
    use super::*;
    use bytes::Bytes;

    fn resp(status: u16, headers: Vec<(&str, &str)>) -> Response {
        let url = url::Url::parse("https://api.other.com/").unwrap();
        Response {
            status,
            headers: headers
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            body: Bytes::new(),
            url: url.clone(),
            version: crate::HttpVersion::H1,
            url_list: vec![url],
            is_redirect_tainted: false,
            credentialed_network: false,
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

    /// Regression for Copilot R1 finding 2: ACAH validation must
    /// skip broker-injected headers (`Origin` / `Referer` etc.)
    /// — compliant servers don't echo those names in ACAH and we
    /// must not reject them.
    #[test]
    fn validate_response_skips_broker_injected_in_acah_check() {
        let r = req_with(
            "GET",
            "https://api.other.com/",
            "https://example.com/",
            vec![
                ("Origin".into(), "https://example.com".into()),
                ("Referer".into(), "https://example.com/page".into()),
                ("X-Custom".into(), "1".into()),
            ],
        );
        // Server lists only "x-custom" — Origin/Referer must
        // not cause rejection.
        let response = resp(
            204,
            vec![
                ("Access-Control-Allow-Origin", "https://example.com"),
                ("Access-Control-Allow-Headers", "x-custom"),
            ],
        );
        assert!(validate_preflight_response(&r, &response).is_ok());
    }

    /// Regression for Copilot R1 finding 3: ACAC parsing must be
    /// ASCII case-insensitive — `True` / `TRUE` / `true` all
    /// satisfy the credentialed check.
    #[test]
    fn validate_response_acac_case_insensitive_true_uppercase() {
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
                ("Access-Control-Allow-Credentials", "TRUE"),
                ("Access-Control-Allow-Methods", "PUT"),
            ],
        );
        let allowance = validate_preflight_response(&r, &response).unwrap();
        assert!(allowance.allow_credentials);
    }

    #[test]
    fn validate_response_acac_case_insensitive_true_mixed() {
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
                ("Access-Control-Allow-Credentials", "True"),
                ("Access-Control-Allow-Methods", "PUT"),
            ],
        );
        assert!(validate_preflight_response(&r, &response).is_ok());
    }

    /// Regression for Copilot R2 finding 2: duplicate ACAO →
    /// fail closed (the spec defines ACAO as single-valued; a
    /// server emitting it twice is misconfigured / suspicious
    /// and must not be silently accepted via first-match
    /// semantics).
    #[test]
    fn validate_response_duplicate_acao_fails_closed() {
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
                ("Access-Control-Allow-Origin", "https://attacker.com"),
                ("Access-Control-Allow-Methods", "PUT"),
            ],
        );
        let err = validate_preflight_response(&r, &response).unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    }

    /// ACAO with comma-separated value must fail closed —
    /// single-valued header per spec.
    #[test]
    fn validate_response_acao_comma_list_fails_closed() {
        let r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![],
        );
        let response = resp(
            204,
            vec![
                (
                    "Access-Control-Allow-Origin",
                    "https://example.com,https://attacker.com",
                ),
                ("Access-Control-Allow-Methods", "PUT"),
            ],
        );
        let err = validate_preflight_response(&r, &response).unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    }

    /// Duplicate ACAC (single-valued) must fail closed for
    /// credentialed requests.
    #[test]
    fn validate_response_duplicate_acac_fails_closed() {
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
                ("Access-Control-Allow-Credentials", "true"),
                ("Access-Control-Allow-Credentials", "true"),
                ("Access-Control-Allow-Methods", "PUT"),
            ],
        );
        let err = validate_preflight_response(&r, &response).unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    }

    /// Regression for Copilot R4 finding 2: per WHATWG Fetch
    /// §4.8 step 19, "extracting header list values" returning
    /// failure (which covers duplicate occurrence of a single-
    /// valued header) must surface as a network error — NOT
    /// silently fall back to the missing-header 5s default.
    /// The earlier R3 fix only renamed the test; R4 requires
    /// the actual behaviour to fail closed.
    #[test]
    fn validate_response_duplicate_max_age_fails_closed() {
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
                ("Access-Control-Max-Age", "60"),
                ("Access-Control-Max-Age", "120"),
            ],
        );
        let err = validate_preflight_response(&r, &response).unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    }

    /// Regression for Copilot R5 finding 1: duplicate ACAM with
    /// a safelisted method (GET/HEAD/POST) must fail closed.
    /// Pre-fix the silent fall-through to an empty allow-list
    /// passed because `method_must_be_allowed(GET, empty)`
    /// returns Ok unconditionally for safelisted methods —
    /// duplicate ACAM was effectively ignored.
    #[test]
    fn validate_response_duplicate_acam_with_safelisted_method_fails_closed() {
        // GET + custom header → preflight needed.
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
                ("Access-Control-Allow-Methods", "GET"),
                ("Access-Control-Allow-Methods", "HEAD"),
                ("Access-Control-Allow-Headers", "x-custom"),
            ],
        );
        let err = validate_preflight_response(&r, &response).unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    }

    /// Duplicate ACAH with only-safelisted-author headers must
    /// fail closed.  (Trigger preflight via non-safelisted PUT
    /// method so requires_preflight=true while no actual headers
    /// need ACAH validation — pre-fix the duplicate ACAH would
    /// silently pass.)
    #[test]
    fn validate_response_duplicate_acah_with_safelisted_headers_fails_closed() {
        let r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![("Accept".into(), "text/plain".into())],
        );
        let response = resp(
            204,
            vec![
                ("Access-Control-Allow-Origin", "https://example.com"),
                ("Access-Control-Allow-Methods", "PUT"),
                ("Access-Control-Allow-Headers", "x-custom"),
                ("Access-Control-Allow-Headers", "x-other"),
            ],
        );
        let err = validate_preflight_response(&r, &response).unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    }

    /// Comma-list Max-Age (single-valued) must also fail closed.
    #[test]
    fn validate_response_max_age_comma_list_fails_closed() {
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
                ("Access-Control-Max-Age", "60,120"),
            ],
        );
        let err = validate_preflight_response(&r, &response).unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    }

    /// Duplicate ACAM (multi-valued list header) must also fail
    /// closed at the helper level (`header_value` rejects
    /// duplicates uniformly, even for headers that are normally
    /// list-valued — multiple field instances in a CORS context
    /// is still suspicious).
    #[test]
    fn validate_response_duplicate_acam_fails_closed() {
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
                ("Access-Control-Allow-Methods", "DELETE"),
            ],
        );
        let err = validate_preflight_response(&r, &response).unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
    }

    /// Sentinel: non-"true" values still fail (e.g. "false",
    /// "yes") for credentialed requests.
    #[test]
    fn validate_response_acac_non_true_rejects_credentialed() {
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
                ("Access-Control-Allow-Credentials", "yes"),
                ("Access-Control-Allow-Methods", "PUT"),
            ],
        );
        let err = validate_preflight_response(&r, &response).unwrap_err();
        assert_eq!(err.kind, NetErrorKind::CorsBlocked);
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
