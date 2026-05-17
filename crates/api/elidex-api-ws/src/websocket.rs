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

/// Normalize a WebSocket URL per WHATWG HTML §9.3.1 step 6.
///
/// Mutates `url` in place:
/// - `http://` → `ws://`
/// - `https://` → `wss://`
/// - `ws://` / `wss://` → no-op
/// - any other scheme → `Err`
///
/// Callers should invoke this BEFORE [`validate_ws_url`] so that the
/// downstream scheme check (which restricts to `ws`/`wss`) sees the
/// post-normalization scheme.  The two-step split matches the spec's
/// own ordering (step 6 normalizes, step 7+ validates fragment/SSRF).
///
/// Returns `Ok(())` after a successful normalization (or no-op for
/// already-ws/wss URLs), or an error message describing the
/// unsupported scheme.
pub fn normalize_ws_url(url: &mut url::Url) -> Result<(), String> {
    match url.scheme() {
        "ws" | "wss" => Ok(()),
        "http" => url
            .set_scheme("ws")
            .map_err(|()| "internal: failed to promote http→ws scheme".to_string()),
        "https" => url
            .set_scheme("wss")
            .map_err(|()| "internal: failed to promote https→wss scheme".to_string()),
        other => Err(format!("unsupported scheme: {other}")),
    }
}

/// Validate a WebSocket URL.
///
/// Checks:
/// - Scheme is `ws` or `wss` (callers should run [`normalize_ws_url`]
///   first to handle the WHATWG §9.3.1 step 6 http→ws / https→wss
///   promotion; this check then acts as a defensive backstop)
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
    fn normalize_promotes_http_to_ws() {
        let mut url = url::Url::parse("http://example.com/socket").unwrap();
        assert!(normalize_ws_url(&mut url).is_ok());
        assert_eq!(url.scheme(), "ws");
    }

    #[test]
    fn normalize_promotes_https_to_wss() {
        let mut url = url::Url::parse("https://example.com/socket").unwrap();
        assert!(normalize_ws_url(&mut url).is_ok());
        assert_eq!(url.scheme(), "wss");
    }

    #[test]
    fn normalize_keeps_ws_unchanged() {
        let mut url = url::Url::parse("ws://example.com/socket").unwrap();
        assert!(normalize_ws_url(&mut url).is_ok());
        assert_eq!(url.scheme(), "ws");
    }

    #[test]
    fn normalize_keeps_wss_unchanged() {
        let mut url = url::Url::parse("wss://example.com/socket").unwrap();
        assert!(normalize_ws_url(&mut url).is_ok());
        assert_eq!(url.scheme(), "wss");
    }

    #[test]
    fn normalize_rejects_unsupported_scheme() {
        let mut url = url::Url::parse("ftp://example.com/socket").unwrap();
        let err = normalize_ws_url(&mut url).unwrap_err();
        assert!(err.contains("unsupported scheme"));
        assert!(err.contains("ftp"));
        assert_eq!(url.scheme(), "ftp"); // unchanged on error
    }

    #[test]
    fn validate_after_normalize_accepts_http_input() {
        // Combined two-step: WS ctor flow.
        let mut url = url::Url::parse("http://example.com/socket").unwrap();
        normalize_ws_url(&mut url).unwrap();
        assert!(validate_ws_url(&url).is_ok());
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
