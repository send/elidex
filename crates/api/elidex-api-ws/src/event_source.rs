//! EventSource (SSE) protocol types (WHATWG HTML §9.2).

/// EventSource readyState constants.
pub const SSE_READYSTATE_CONSTANTS: [(&str, i32); 3] =
    [("CONNECTING", 0), ("OPEN", 1), ("CLOSED", 2)];

/// EventSource connection readyState (WHATWG HTML §9.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum SseReadyState {
    Connecting = 0,
    Open = 1,
    Closed = 2,
}

impl SseReadyState {
    /// Create from integer value.
    #[must_use]
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Connecting),
            1 => Some(Self::Open),
            2 => Some(Self::Closed),
            _ => None,
        }
    }
}

/// The spec-faithful EventSource **GC keepalive** rule (WHATWG HTML §9.2.9
/// "Garbage collection") — the engine-independent half of the keepalive seam's
/// `EventSource` arm (the VM-side seam in `elidex-js` `vm/gc/keepalive.rs`
/// marshals the readyState + a typed-listener closure and calls this).
///
/// Per §9.2.9 an `EventSource` must be kept alive while:
/// - readyState is **CONNECTING** and it has a listener for `open`/`message`/`error`; or
/// - readyState is **OPEN** and it has a listener for `message`/`error`.
///
/// (CLOSED is never kept — a closed source can deliver nothing; GC-while-open ⇒
/// abort the fetch, which is the seam's force-close *else* branch.)
///
/// The spec's third clause — "while there is a task queued … on the **remote
/// event task source** … strong reference" — is the **no-listener** keepalive.
/// It has **no elidex analogue**: elidex drains broker→renderer SSE messages and
/// dispatches them **inline** (no queued-but-unrun task window that could span a
/// GC), so there is never a "no listener yet a task is queued" state to protect.
/// Hence this rule has no in-flight axis (unlike [`crate::ws_keepalive`], whose
/// `buffered_amount` data-queued clause IS real). A no-listener OPEN source is
/// therefore collectible — spec-correct for elidex's delivery model.
///
/// `has_listener(event_type)` reports whether the source has a live listener
/// (an `addEventListener` registration **or** a live `on<type>` handler) for the
/// given event type — supplied by the engine seam over its listener store; this
/// rule owns only *which* types §9.2.9 keeps alive per readyState.
pub fn es_keepalive(state: SseReadyState, has_listener: impl Fn(&str) -> bool) -> bool {
    match state {
        SseReadyState::Connecting => ["open", "message", "error"]
            .iter()
            .any(|&t| has_listener(t)),
        SseReadyState::Open => ["message", "error"].iter().any(|&t| has_listener(t)),
        SseReadyState::Closed => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readystate_from_i32() {
        assert_eq!(SseReadyState::from_i32(0), Some(SseReadyState::Connecting));
        assert_eq!(SseReadyState::from_i32(1), Some(SseReadyState::Open));
        assert_eq!(SseReadyState::from_i32(2), Some(SseReadyState::Closed));
        assert_eq!(SseReadyState::from_i32(3), None);
    }

    #[test]
    fn constants_match_enum() {
        assert_eq!(
            SSE_READYSTATE_CONSTANTS[0].1,
            SseReadyState::Connecting as i32
        );
        assert_eq!(SSE_READYSTATE_CONSTANTS[1].1, SseReadyState::Open as i32);
        assert_eq!(SSE_READYSTATE_CONSTANTS[2].1, SseReadyState::Closed as i32);
    }

    #[test]
    fn es_keepalive_connecting_tier() {
        // §9.2.9: CONNECTING keeps for open / message / error.
        for t in ["open", "message", "error"] {
            assert!(
                es_keepalive(SseReadyState::Connecting, |e| e == t),
                "CONNECTING + {t} listener must keep alive"
            );
        }
        // An out-of-tier type does not keep (no injection: unknown type fails).
        assert!(!es_keepalive(SseReadyState::Connecting, |e| e == "bogus"));
        // No listener at all → collectible (no in-flight axis for SSE).
        assert!(!es_keepalive(SseReadyState::Connecting, |_| false));
    }

    #[test]
    fn es_keepalive_open_tier() {
        // §9.2.9: OPEN keeps for message / error only.
        for t in ["message", "error"] {
            assert!(
                es_keepalive(SseReadyState::Open, |e| e == t),
                "OPEN + {t} listener must keep alive"
            );
        }
        // `open` is NOT in the OPEN tier — the open event already fired, so an
        // open-only listener on an OPEN source is dead weight (proves tiered,
        // not any-listener).
        assert!(!es_keepalive(SseReadyState::Open, |e| e == "open"));
        assert!(!es_keepalive(SseReadyState::Open, |_| false));
    }

    #[test]
    fn es_keepalive_closed_never() {
        // CLOSED is never kept — even with every listener registered.
        assert!(!es_keepalive(SseReadyState::Closed, |_| true));
    }
}
