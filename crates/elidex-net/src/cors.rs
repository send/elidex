//! CORS (Cross-Origin Resource Sharing) validation.
//!
//! Validates that responses include appropriate CORS headers when the
//! request is cross-origin.

use crate::error::{NetError, NetErrorKind};

/// CORS request origin information.
#[derive(Clone, Debug)]
pub struct CorsContext {
    /// The origin of the requesting page (e.g. `https://example.com`).
    pub origin: Option<String>,
    /// Whether the request includes credentials (cookies, authorization).
    pub with_credentials: bool,
}

/// Validate CORS headers on a response.
///
/// If the request has an `Origin` that differs from the response URL's origin,
/// the response must include a matching `Access-Control-Allow-Origin` header.
///
/// Per the Fetch specification, `Access-Control-Allow-Origin: *` is not valid
/// when the request includes credentials.
///
/// # Errors
///
/// Returns `NetError` with `CorsBlocked` if CORS validation fails.
pub fn validate_cors(
    context: &CorsContext,
    response_url: &url::Url,
    response_headers: &[(String, String)],
) -> Result<(), NetError> {
    let Some(ref origin) = context.origin else {
        // No origin context — not a cross-origin request
        return Ok(());
    };

    // Check if same-origin.
    // Normalize both sides to "scheme://host:port" to handle default port
    // ambiguity (e.g. "https://example.com" vs "https://example.com:443")
    // and IPv6 bracket format.
    let response_origin = build_origin(response_url);
    let normalized_origin = normalize_origin(origin);

    if normalized_origin.eq_ignore_ascii_case(&response_origin) {
        return Ok(()); // Same-origin, no CORS check needed
    }

    // Cross-origin: check Access-Control-Allow-Origin
    let allowed_origin = response_headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("access-control-allow-origin"))
        .map(|(_, v)| v.as_str());

    match allowed_origin {
        Some("*") if context.with_credentials => Err(NetError::new(
            NetErrorKind::CorsBlocked,
            format!("CORS: wildcard '*' not allowed with credentials for origin '{origin}'"),
        )),
        Some("*") => Ok(()),
        Some(allowed) if normalize_origin(allowed).eq_ignore_ascii_case(&normalized_origin) => {
            Ok(())
        }
        _ => Err(NetError::new(
            NetErrorKind::CorsBlocked,
            format!(
                "CORS: origin '{origin}' not allowed by '{}'",
                allowed_origin.unwrap_or("(no ACAO header)")
            ),
        )),
    }
}

/// Build a normalized origin string from a URL.
///
/// Format: `scheme://host:port` with port always included (using known
/// defaults for http/https), and IPv6 addresses wrapped in brackets.
fn build_origin(url: &url::Url) -> String {
    let scheme = url.scheme();
    let host = url.host_str().unwrap_or("");
    let port = url.port_or_known_default().unwrap_or(0);
    // IPv6 addresses (contain ':') need brackets in origin format
    if host.contains(':') {
        format!("{scheme}://[{host}]:{port}")
    } else {
        format!("{scheme}://{host}:{port}")
    }
}

/// Normalize an origin string for comparison.
///
/// If the string is a valid URL, normalize via `build_origin`.
/// Otherwise return it as-is (the comparison will fail gracefully).
fn normalize_origin(origin: &str) -> String {
    url::Url::parse(origin).map_or_else(|_| origin.to_string(), |u| build_origin(&u))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_origin_passes() {
        let ctx = CorsContext {
            origin: Some("https://example.com".into()),
            with_credentials: false,
        };
        let url = url::Url::parse("https://example.com/api").unwrap();
        assert!(validate_cors(&ctx, &url, &[]).is_ok());
    }

    #[test]
    fn no_origin_passes() {
        let ctx = CorsContext {
            origin: None,
            with_credentials: false,
        };
        let url = url::Url::parse("https://api.example.com/data").unwrap();
        assert!(validate_cors(&ctx, &url, &[]).is_ok());
    }

    #[test]
    fn cors_wildcard_allowed() {
        let ctx = CorsContext {
            origin: Some("https://example.com".into()),
            with_credentials: false,
        };
        let url = url::Url::parse("https://api.other.com/data").unwrap();
        let headers = vec![("Access-Control-Allow-Origin".to_string(), "*".to_string())];
        assert!(validate_cors(&ctx, &url, &headers).is_ok());
    }

    #[test]
    fn cors_wildcard_blocked_with_credentials() {
        let ctx = CorsContext {
            origin: Some("https://example.com".into()),
            with_credentials: true,
        };
        let url = url::Url::parse("https://api.other.com/data").unwrap();
        let headers = vec![("Access-Control-Allow-Origin".to_string(), "*".to_string())];
        let result = validate_cors(&ctx, &url, &headers);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, NetErrorKind::CorsBlocked);
    }

    #[test]
    fn cors_specific_origin_allowed() {
        let ctx = CorsContext {
            origin: Some("https://example.com".into()),
            with_credentials: false,
        };
        let url = url::Url::parse("https://api.other.com/data").unwrap();
        let headers = vec![(
            "access-control-allow-origin".to_string(),
            "https://example.com".to_string(),
        )];
        assert!(validate_cors(&ctx, &url, &headers).is_ok());
    }

    #[test]
    fn cors_specific_origin_with_credentials_allowed() {
        let ctx = CorsContext {
            origin: Some("https://example.com".into()),
            with_credentials: true,
        };
        let url = url::Url::parse("https://api.other.com/data").unwrap();
        let headers = vec![(
            "access-control-allow-origin".to_string(),
            "https://example.com".to_string(),
        )];
        assert!(validate_cors(&ctx, &url, &headers).is_ok());
    }

    #[test]
    fn cors_blocked_no_header() {
        let ctx = CorsContext {
            origin: Some("https://example.com".into()),
            with_credentials: false,
        };
        let url = url::Url::parse("https://api.other.com/data").unwrap();
        let result = validate_cors(&ctx, &url, &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, NetErrorKind::CorsBlocked);
    }

    #[test]
    fn same_origin_explicit_default_port() {
        // Origin with explicit :443 should still match https URL without port
        let ctx = CorsContext {
            origin: Some("https://example.com:443".into()),
            with_credentials: false,
        };
        let url = url::Url::parse("https://example.com/api").unwrap();
        assert!(validate_cors(&ctx, &url, &[]).is_ok());
    }

    #[test]
    fn same_origin_ipv6() {
        let ctx = CorsContext {
            origin: Some("http://[::1]:8080".into()),
            with_credentials: false,
        };
        let url = url::Url::parse("http://[::1]:8080/data").unwrap();
        assert!(validate_cors(&ctx, &url, &[]).is_ok());
    }

    #[test]
    fn cors_blocked_wrong_origin() {
        let ctx = CorsContext {
            origin: Some("https://example.com".into()),
            with_credentials: false,
        };
        let url = url::Url::parse("https://api.other.com/data").unwrap();
        let headers = vec![(
            "access-control-allow-origin".to_string(),
            "https://attacker.com".to_string(),
        )];
        let result = validate_cors(&ctx, &url, &headers);
        assert!(result.is_err());
    }
}
