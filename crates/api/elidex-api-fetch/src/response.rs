//! Response protocol types (WHATWG Fetch §3.1.4, §5.4).

/// Valid redirect status codes (WHATWG Fetch §3.1.4).
pub const REDIRECT_STATUSES: &[u16] = &[301, 302, 303, 307, 308];

/// Engine-independent representation of a Response's data.
///
/// Used to pass response information between the network layer and
/// script engine bindings without depending on any JS engine types.
#[derive(Debug, Clone)]
pub struct ResponseParts {
    /// HTTP status code.
    pub status: u16,
    /// HTTP reason phrase.
    pub status_text: String,
    /// Header name-value pairs.
    pub headers: Vec<(String, String)>,
    /// Response body as UTF-8 string.
    pub body: String,
    /// Final URL after redirects.
    pub url: String,
    /// Response type (basic, cors, default, error, opaque, opaqueredirect).
    pub response_type: String,
    /// Whether the response was redirected.
    pub redirected: bool,
}

impl ResponseParts {
    /// Whether the status is in the 200-299 range.
    #[must_use]
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Whether this is a redirect status.
    #[must_use]
    pub fn is_redirect(&self) -> bool {
        REDIRECT_STATUSES.contains(&self.status)
    }
}

/// Map an HTTP status code to its standard reason phrase.
#[must_use]
pub fn status_text_for(status: u16) -> &'static str {
    match status {
        100 => "Continue",
        101 => "Switching Protocols",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        206 => "Partial Content",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_text_known() {
        assert_eq!(status_text_for(200), "OK");
        assert_eq!(status_text_for(404), "Not Found");
        assert_eq!(status_text_for(500), "Internal Server Error");
    }

    #[test]
    fn status_text_unknown() {
        assert_eq!(status_text_for(999), "");
    }

    #[test]
    fn response_parts_ok() {
        let parts = ResponseParts {
            status: 200,
            status_text: "OK".into(),
            headers: vec![],
            body: String::new(),
            url: String::new(),
            response_type: "basic".into(),
            redirected: false,
        };
        assert!(parts.ok());
        assert!(!parts.is_redirect());
    }

    #[test]
    fn response_parts_redirect() {
        let parts = ResponseParts {
            status: 302,
            status_text: "Found".into(),
            headers: vec![],
            body: String::new(),
            url: String::new(),
            response_type: "default".into(),
            redirected: false,
        };
        assert!(!parts.ok());
        assert!(parts.is_redirect());
    }

    #[test]
    fn redirect_statuses_valid() {
        for &status in REDIRECT_STATUSES {
            assert!(matches!(status, 301 | 302 | 303 | 307 | 308));
        }
    }
}
