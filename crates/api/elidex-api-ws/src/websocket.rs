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

/// Normalize a WebSocket URL's scheme per WHATWG WebSockets §9.3.1
/// (the `http`→`ws` / `https`→`wss` promotion the spec performs before
/// the scheme/fragment validation steps).
///
/// Mutates `url` in place:
/// - `http://` → `ws://`
/// - `https://` → `wss://`
/// - `ws://` / `wss://` → no-op
/// - any other scheme → `Err`
///
/// Callers should invoke this BEFORE [`validate_ws_url`] so that the
/// downstream scheme check (which restricts to `ws`/`wss`) sees the
/// post-normalization scheme. The pair mirrors the spec's own split:
/// normalize first, then validate scheme and fragment.
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

/// Validate a WebSocket URL per WHATWG WebSockets §9.3.1.
///
/// Checks (run in spec order, with the SSRF extension last):
/// - Scheme is `ws` or `wss`. Callers should run [`normalize_ws_url`]
///   first to handle the `http`→`ws` / `https`→`wss` promotion; this
///   check then acts as a defensive backstop.
/// - No fragment component (matches the spec's `SyntaxError` rule).
/// - SSRF protection via `elidex_plugin::url_security::validate_url`
///   (engine-local extension, not in spec; converts `ws`/`wss` to
///   `http`/`https` for the shared policy). Runs last so the
///   spec-defined `SyntaxError` precedence for scheme/fragment is
///   preserved regardless of host-policy outcome.
///
/// Returns `Ok(())` if the URL is valid, or an error message if it is not.
pub fn validate_ws_url(url: &url::Url) -> Result<(), String> {
    // 1. Scheme check.
    match url.scheme() {
        "ws" | "wss" => {}
        scheme => return Err(format!("unsupported scheme: {scheme}")),
    }

    // 2. Fragment check. Runs before the SSRF extension so the
    //    spec-mandated SyntaxError precedence is preserved (e.g.
    //    `ws://localhost/#frag` reports the fragment violation, not the
    //    SSRF block, matching browser behaviour).
    if url.fragment().is_some() {
        return Err("URL must not contain a fragment".to_string());
    }

    // 3. SSRF check (engine-local extension): convert ws/wss to http/https
    //    so the shared `validate_url` policy can evaluate the host.
    let http_scheme = if url.scheme() == "wss" {
        "https"
    } else {
        "http"
    };
    let mut check_url = url.clone();
    check_url.set_scheme(http_scheme).map_err(|()| {
        format!(
            "internal: failed to swap {} → {http_scheme} for SSRF check",
            url.scheme()
        )
    })?;
    elidex_plugin::url_security::validate_url(&check_url)
        .map_err(|e| format!("URL blocked: {e}"))?;

    Ok(())
}

/// Whether a `ws://` connection is **mixed content** that must be blocked
/// (W3C Mixed Content §4.4 "Should fetching request be blocked as mixed
/// content?", via §4.3 "Does settings prohibit mixed security contexts?"): true
/// iff the client is a **secure context** — i.e. its origin is *potentially
/// trustworthy* (Secure Contexts §3.1 "Is origin potentially trustworthy?") —
/// AND the target is the insecure `ws:` scheme (`wss:` is never mixed content).
/// Gating on origin trustworthiness rather than the raw page-URL scheme is what
/// makes an **opaque-origin** document (a sandboxed iframe, `data:`/`file:` doc)
/// — which is never a secure context — correctly *exempt* from the block, while
/// a same-`https`-URL **tuple** origin is still blocked.
#[must_use]
pub fn is_mixed_content(client_origin: &elidex_plugin::SecurityOrigin, ws_url: &url::Url) -> bool {
    client_origin.is_potentially_trustworthy() && ws_url.scheme() == "ws"
}

/// The spec-faithful WebSocket **GC keepalive** rule (WHATWG WebSockets §7
/// "Garbage collection") — the engine-independent half of the keepalive seam's
/// `WebSocket` arm (the VM-side seam in `elidex-js` `vm/gc/keepalive.rs` marshals
/// the readyState + a typed-listener closure and calls this).
///
/// Per §7 a `WebSocket` must be kept alive while:
/// - readyState is **CONNECTING** and it has a listener for
///   `open`/`message`/`error`/`close`; or
/// - readyState is **OPEN** and it has a listener for `message`/`error`/`close`; or
/// - readyState is **CLOSING** and it has a listener for `error`/`close`.
///
/// This is a **pure readyState-tier check**. §7's fourth (no-listener) clause —
/// "an established connection that has **data queued to be transmitted** to the
/// network must not be garbage collected" — is an **OUTBOUND** clause and is
/// **OMITTED as vacuous in elidex**: once `send()` emits, the outbound bytes are
/// **broker-owned FIFO** (they transmit ahead of any GC-emitted `WebSocketClose`
/// regardless of whether the wrapper survives — WebSockets §3.1 `send(data)`,
/// `#dom-websocket-send`), so keeping the wrapper alive on a `buffered_amount`
/// input would protect nothing. Worse, `buffered_amount` is incremented
/// **unconditionally** (including CLOSING/CLOSED sends that never transmit and
/// never clear), so keying keepalive on it would **over-root a listener-less
/// CLOSING socket into an indefinite leak** (Codex F1). Hence no `has_queued_data`
/// parameter and no in-flight axis.
///
/// CLOSED is never kept (a closed socket can deliver nothing). GC-while-open ⇒
/// start the closing handshake — the seam's force-close *else* branch for a
/// connection this rule does NOT keep.
///
/// `has_listener(event_type)` reports whether the socket has a live listener (an
/// `addEventListener` registration **or** a live `on<type>` handler) for the
/// given event type — supplied by the engine seam over its listener store; this
/// rule owns only *which* types §7 keeps alive per readyState.
pub fn ws_keepalive(state: WsReadyState, has_listener: impl Fn(&str) -> bool) -> bool {
    match state {
        WsReadyState::Connecting => ["open", "message", "error", "close"]
            .iter()
            .any(|&t| has_listener(t)),
        WsReadyState::Open => ["message", "error", "close"]
            .iter()
            .any(|&t| has_listener(t)),
        WsReadyState::Closing => ["error", "close"].iter().any(|&t| has_listener(t)),
        WsReadyState::Closed => false,
    }
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
    fn fragment_check_precedes_ssrf_check() {
        // The spec-mandated fragment SyntaxError precedes our engine-local
        // SSRF extension, so a fragment on an otherwise-blocked host must
        // still surface the fragment error rather than the SSRF block.
        let url = url::Url::parse("ws://localhost/socket#frag").unwrap();
        let err = validate_ws_url(&url).unwrap_err();
        assert!(
            err.contains("fragment"),
            "expected fragment error to take precedence over SSRF block, got: {err}"
        );
        assert!(!err.contains("URL blocked"), "got SSRF error: {err}");
    }

    #[test]
    fn readystate_from_i32() {
        assert_eq!(WsReadyState::from_i32(0), Some(WsReadyState::Connecting));
        assert_eq!(WsReadyState::from_i32(3), Some(WsReadyState::Closed));
        assert_eq!(WsReadyState::from_i32(4), None);
    }

    #[test]
    fn mixed_content_detection() {
        use elidex_plugin::SecurityOrigin;
        let ws = url::Url::parse("ws://example.com").unwrap();
        let wss = url::Url::parse("wss://example.com").unwrap();
        let secure = SecurityOrigin::from_url(&url::Url::parse("https://page.example/").unwrap());
        let insecure = SecurityOrigin::from_url(&url::Url::parse("http://page.example/").unwrap());
        let opaque = SecurityOrigin::opaque();
        // Secure (https tuple) client + ws:// → blocked; wss:// → allowed.
        assert!(is_mixed_content(&secure, &ws));
        assert!(!is_mixed_content(&secure, &wss));
        // Insecure (public http) client is not a secure context → not mixed.
        assert!(!is_mixed_content(&insecure, &ws));
        // Opaque (sandboxed) origin is never a secure context → not mixed
        // (the S5-6b flip-parity regression this fix closes).
        assert!(!is_mixed_content(&opaque, &ws));
    }

    #[test]
    fn ws_keepalive_connecting_tier() {
        // §7: CONNECTING keeps for open / message / error / close.
        for t in ["open", "message", "error", "close"] {
            assert!(
                ws_keepalive(WsReadyState::Connecting, |e| e == t),
                "CONNECTING + {t} listener must keep alive"
            );
        }
        assert!(!ws_keepalive(WsReadyState::Connecting, |e| e == "bogus"));
        assert!(!ws_keepalive(WsReadyState::Connecting, |_| false));
    }

    #[test]
    fn ws_keepalive_open_tier() {
        // §7: OPEN keeps for message / error / close only.
        for t in ["message", "error", "close"] {
            assert!(
                ws_keepalive(WsReadyState::Open, |e| e == t),
                "OPEN + {t} listener must keep alive"
            );
        }
        // `open` is NOT in the OPEN tier (proves tiered, not any-listener).
        assert!(!ws_keepalive(WsReadyState::Open, |e| e == "open"));
        assert!(!ws_keepalive(WsReadyState::Open, |_| false));
    }

    #[test]
    fn ws_keepalive_closing_tier() {
        // §7: CLOSING keeps for error / close only.
        for t in ["error", "close"] {
            assert!(
                ws_keepalive(WsReadyState::Closing, |e| e == t),
                "CLOSING + {t} listener must keep alive"
            );
        }
        // `open` / `message` are NOT in the CLOSING tier.
        assert!(!ws_keepalive(WsReadyState::Closing, |e| e == "open"));
        assert!(!ws_keepalive(WsReadyState::Closing, |e| e == "message"));
        // A listener-less CLOSING socket is collectible — the §7 data-queued
        // clause is OMITTED (no `buffered_amount` over-root, the F1 guard at the
        // unit level).
        assert!(!ws_keepalive(WsReadyState::Closing, |_| false));
    }

    #[test]
    fn ws_keepalive_closed_never() {
        // CLOSED is never kept — not by any listener.
        assert!(!ws_keepalive(WsReadyState::Closed, |_| true));
    }
}
