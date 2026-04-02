//! WebSocket protocol types (WHATWG HTML §9.3).

/// WebSocket readyState constants.
pub const WS_READYSTATE_CONSTANTS: [(&str, i32); 4] = [
    ("CONNECTING", 0),
    ("OPEN", 1),
    ("CLOSING", 2),
    ("CLOSED", 3),
];

/// WebSocket connection readyState (WHATWG HTML §9.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum WsReadyState {
    Connecting = 0,
    Open = 1,
    Closing = 2,
    Closed = 3,
}

impl WsReadyState {
    /// Create from integer value.
    #[must_use]
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Connecting),
            1 => Some(Self::Open),
            2 => Some(Self::Closing),
            3 => Some(Self::Closed),
            _ => None,
        }
    }
}

/// Validate a WebSocket URL.
///
/// Checks:
/// - Scheme is `ws` or `wss`
/// - No fragment component
/// - SSRF protection via `elidex_plugin::url_security::validate_url`
///   (converts ws/wss to http/https for validation)
///
/// Returns `Ok(())` if the URL is valid, or an error message if it is not.
pub fn validate_ws_url(url: &url::Url) -> Result<(), String> {
    // 1. Scheme check.
    match url.scheme() {
        "ws" | "wss" => {}
        scheme => return Err(format!("unsupported scheme: {scheme}")),
    }

    // 2. SSRF check: convert ws/wss to http/https for validate_url.
    let http_scheme = if url.scheme() == "wss" {
        "https"
    } else {
        "http"
    };
    let mut check_url = url.clone();
    check_url.set_scheme(http_scheme).ok();
    elidex_plugin::url_security::validate_url(&check_url)
        .map_err(|e| format!("URL blocked: {e}"))?;

    // 3. Fragment check.
    if url.fragment().is_some() {
        return Err("URL must not contain a fragment".to_string());
    }

    Ok(())
}

/// Check for mixed content: secure origin trying to use insecure ws://.
///
/// Returns `true` if the connection should be blocked.
#[must_use]
pub fn is_mixed_content(origin_scheme: &str, ws_url: &url::Url) -> bool {
    origin_scheme == "https" && ws_url.scheme() == "ws"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_ws_url() {
        let url = url::Url::parse("ws://example.com/socket").unwrap();
        assert!(validate_ws_url(&url).is_ok());
    }

    #[test]
    fn valid_wss_url() {
        let url = url::Url::parse("wss://example.com/socket").unwrap();
        assert!(validate_ws_url(&url).is_ok());
    }

    #[test]
    fn rejects_http_scheme() {
        let url = url::Url::parse("http://example.com/socket").unwrap();
        assert!(validate_ws_url(&url).is_err());
    }

    #[test]
    fn rejects_fragment() {
        let url = url::Url::parse("ws://example.com/socket#frag").unwrap();
        let err = validate_ws_url(&url).unwrap_err();
        assert!(err.contains("fragment"));
    }

    #[test]
    fn readystate_from_i32() {
        assert_eq!(WsReadyState::from_i32(0), Some(WsReadyState::Connecting));
        assert_eq!(WsReadyState::from_i32(3), Some(WsReadyState::Closed));
        assert_eq!(WsReadyState::from_i32(4), None);
    }

    #[test]
    fn mixed_content_detection() {
        let ws = url::Url::parse("ws://example.com").unwrap();
        let wss = url::Url::parse("wss://example.com").unwrap();
        assert!(is_mixed_content("https", &ws));
        assert!(!is_mixed_content("https", &wss));
        assert!(!is_mixed_content("http", &ws));
    }
}
