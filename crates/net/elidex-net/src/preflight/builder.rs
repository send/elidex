//! OPTIONS preflight request construction (WHATWG Fetch §4.8
//! steps 1-9).

use bytes::Bytes;

use super::is_non_safelisted_author_header;
use crate::{CredentialsMode, RedirectMode, Request, RequestMode};

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
        .filter(|(name, value)| is_non_safelisted_author_header(name, value))
        .map(|(name, _)| name.to_ascii_lowercase())
        .collect();
    names.sort();
    names.dedup();
    names.join(",")
}

#[cfg(test)]
mod tests {
    use super::super::req_with;
    use super::super::validator::header_value;
    use super::*;

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

    /// Regression for Copilot R1 finding 1: ACRH must omit
    /// broker-injected headers like `Origin` / `Referer`,
    /// otherwise compliant servers that don't list those names in
    /// `Access-Control-Allow-Headers` would reject the preflight.
    #[test]
    fn build_preflight_acrh_omits_broker_injected_origin_and_referer() {
        let r = req_with(
            "PUT",
            "https://api.other.com/",
            "https://example.com/",
            vec![
                ("Origin".into(), "https://example.com".into()),
                ("Referer".into(), "https://example.com/page".into()),
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
}
