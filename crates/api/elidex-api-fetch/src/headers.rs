//! HTTP header validation and guard types (WHATWG Fetch §2.2, §5.1).

/// Forbidden request header names (Fetch spec §2.2.1).
///
/// These headers cannot be set programmatically on a Request with
/// `"request"` guard.
pub const FORBIDDEN_REQUEST_HEADERS: &[&str] = &[
    "accept-charset",
    "accept-encoding",
    "access-control-request-headers",
    "access-control-request-method",
    "connection",
    "content-length",
    "cookie",
    "cookie2",
    "date",
    "dnt",
    "expect",
    "host",
    "keep-alive",
    "origin",
    "referer",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "via",
];

/// Headers guard controlling mutation rules (WHATWG Fetch §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderGuard {
    /// No restrictions on mutation.
    None,
    /// Request headers — rejects forbidden header names and `proxy-`/`sec-` prefixes.
    Request,
    /// Request (no-cors) — allows only CORS-safelisted headers.
    RequestNoCors,
    /// Response — no extra restrictions (standard response guard).
    Response,
    /// Immutable — all mutations rejected (e.g., Response headers from fetch).
    Immutable,
}

impl HeaderGuard {
    /// Check whether setting the given header name is allowed under this guard.
    ///
    /// Returns `true` if the header can be set, `false` if it is forbidden.
    #[must_use]
    pub fn allows_set(&self, name: &str) -> bool {
        match self {
            Self::Immutable => false,
            Self::Request => {
                let lower = name.to_ascii_lowercase();
                !FORBIDDEN_REQUEST_HEADERS.contains(&lower.as_str())
                    && !lower.starts_with("proxy-")
                    && !lower.starts_with("sec-")
            }
            Self::None | Self::Response | Self::RequestNoCors => true,
        }
    }
}

/// Validate a header name per Fetch spec §2.2.1 (HTTP token production).
///
/// A valid token consists of characters from the HTTP token production
/// (RFC 7230 §3.2.6) and must not be empty.
#[must_use]
pub fn is_valid_header_name(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|b| {
            matches!(b,
                b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.'
                | b'0'..=b'9' | b'A'..=b'Z' | b'^' | b'_' | b'`' | b'a'..=b'z' | b'|' | b'~'
            )
        })
}

/// Validate a header value per Fetch spec §2.2.2.
///
/// Header values must not contain NUL (`\0`), LF (`\n`), or CR (`\r`).
#[must_use]
pub fn is_valid_header_value(value: &str) -> bool {
    value.bytes().all(|b| b != 0 && b != b'\n' && b != b'\r')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_header_names() {
        assert!(is_valid_header_name("Content-Type"));
        assert!(is_valid_header_name("x-custom"));
        assert!(is_valid_header_name("Accept"));
    }

    #[test]
    fn invalid_header_names() {
        assert!(!is_valid_header_name(""));
        assert!(!is_valid_header_name("Content Type")); // space
        assert!(!is_valid_header_name("Content\tType")); // tab
        assert!(!is_valid_header_name("Content:Type")); // colon
    }

    #[test]
    fn valid_header_values() {
        assert!(is_valid_header_value("text/html"));
        assert!(is_valid_header_value(""));
        assert!(is_valid_header_value("value with spaces"));
    }

    #[test]
    fn invalid_header_values() {
        assert!(!is_valid_header_value("value\0nul"));
        assert!(!is_valid_header_value("value\nnewline"));
        assert!(!is_valid_header_value("value\rcarriage"));
    }

    #[test]
    fn guard_none_allows_all() {
        assert!(HeaderGuard::None.allows_set("Cookie"));
        assert!(HeaderGuard::None.allows_set("sec-fetch-mode"));
    }

    #[test]
    fn guard_immutable_rejects_all() {
        assert!(!HeaderGuard::Immutable.allows_set("Content-Type"));
        assert!(!HeaderGuard::Immutable.allows_set("x-custom"));
    }

    #[test]
    fn guard_request_rejects_forbidden() {
        assert!(!HeaderGuard::Request.allows_set("Cookie"));
        assert!(!HeaderGuard::Request.allows_set("Host"));
        assert!(!HeaderGuard::Request.allows_set("proxy-authorization"));
        assert!(!HeaderGuard::Request.allows_set("sec-fetch-mode"));
        assert!(HeaderGuard::Request.allows_set("Content-Type"));
        assert!(HeaderGuard::Request.allows_set("x-custom"));
    }
}
