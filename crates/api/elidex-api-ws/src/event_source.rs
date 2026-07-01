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
/// marshals the readyState + `has_queued_task` + a typed-listener closure and
/// calls this).
///
/// Per §9.2.9 an `EventSource` must be kept alive while:
/// - readyState is **CONNECTING** and it has a listener for `open`/`message`/`error`; or
/// - readyState is **OPEN** and it has a listener for `message`/`error`; or
/// - there is a **task queued** by this EventSource on the **remote event task
///   source** — the **no-listener** clause, keeping the wrapper alive regardless
///   of the readyState-tier listeners.
///
/// **CLOSED is never kept** — neither the readyState tier nor the task-queued
/// clause roots a closed source. A closed source can deliver nothing: its fetch is
/// aborted (GC-while-open ⇒ abort, the seam's force-close *else* branch) and a
/// CLOSED source's buffered events are dropped by dispatch (elidex-js
/// `dispatch_sse_event` guards CLOSED, Codex R2b-B). (Elidex note: §9.2.9's
/// task-queued clause is
/// state-independent in the spec text — it has no readyState restriction — but is
/// **vacuous for a CLOSED source in elidex's delivery model**: a task queued for a
/// closed source would never fire, so rooting the wrapper for it is a pure leak.
/// This mirrors the F1 vacuity reasoning for the WebSockets §7 outbound
/// data-queued clause — see [`crate::ws_keepalive`].)
///
/// The task-queued clause **IS meaningful in elidex** (unlike WebSockets §7's
/// outbound `data-queued` clause, which is vacuous — see [`crate::ws_keepalive`]):
/// inbound SSE events buffer in the `NetworkHandle` **between**
/// `drain_fetch_responses_only` and `drain_events`, and an allocation-triggered
/// GC can fire **mid-turn** in that window. A wrapper whose only listener is a
/// **named** event (`addEventListener('foo', …)`, NOT in the readyState tier
/// `{message, error}`) would otherwise be collected in that window and its
/// buffered event **silently dropped** via a `conn_id → ObjectId` reverse-map
/// miss (Codex F3). `has_queued_task` is the GC root for that buffer window: it
/// means "an inbound event is buffered for this conn awaiting dispatch" and is
/// supplied by the engine seam (a non-draining `NetworkHandle` buffer peek).
///
/// `has_listener(event_type)` reports whether the source has a live listener
/// (an `addEventListener` registration **or** a live `on<type>` handler) for the
/// given event type — supplied by the engine seam over its listener store; this
/// rule owns only *which* types §9.2.9 keeps alive per readyState.
pub fn es_keepalive(
    state: SseReadyState,
    has_queued_task: bool,
    has_listener: impl Fn(&str) -> bool,
) -> bool {
    // CLOSED is never kept — not by the tier, not by the task-queued clause
    // (Codex R2b-A). A CLOSED source's buffered events are dropped by dispatch
    // (`dispatch_sse_event` guards CLOSED, R2b-B), so a task queued for it would
    // never fire — rooting the wrapper on `has_queued_task` would be a pure leak.
    // Must run BEFORE the `has_queued_task` short-circuit below.
    if matches!(state, SseReadyState::Closed) {
        return false;
    }
    // §9.2.9 no-listener clause (non-CLOSED): a task queued on the remote event
    // task source keeps the wrapper alive regardless of readyState-tier listeners
    // — the buffer-window root (Codex F3). A queued task on a live source always
    // needs a wrapper to dispatch to, so this short-circuits ahead of the tier
    // check.
    if has_queued_task {
        return true;
    }
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
                es_keepalive(SseReadyState::Connecting, false, |e| e == t),
                "CONNECTING + {t} listener must keep alive"
            );
        }
        // An out-of-tier type does not keep (no injection: unknown type fails).
        assert!(!es_keepalive(SseReadyState::Connecting, false, |e| e == "bogus"));
        // No listener + no queued task → collectible.
        assert!(!es_keepalive(SseReadyState::Connecting, false, |_| false));
    }

    #[test]
    fn es_keepalive_open_tier() {
        // §9.2.9: OPEN keeps for message / error only.
        for t in ["message", "error"] {
            assert!(
                es_keepalive(SseReadyState::Open, false, |e| e == t),
                "OPEN + {t} listener must keep alive"
            );
        }
        // `open` is NOT in the OPEN tier — the open event already fired, so an
        // open-only listener on an OPEN source is dead weight (proves tiered,
        // not any-listener).
        assert!(!es_keepalive(SseReadyState::Open, false, |e| e == "open"));
        assert!(!es_keepalive(SseReadyState::Open, false, |_| false));
    }

    #[test]
    fn es_keepalive_closed_never() {
        // CLOSED is never kept — even with every listener registered — provided
        // no task is queued (the queued-task short-circuit is covered separately).
        assert!(!es_keepalive(SseReadyState::Closed, false, |_| true));
    }

    #[test]
    fn es_keepalive_queued_task_no_listener_clause() {
        // §9.2.9 no-listener clause (Codex F3): a queued task on the remote event
        // task source keeps a NON-CLOSED wrapper alive regardless of readyState-
        // tier listeners — even with NO listener at all.
        for state in [SseReadyState::Connecting, SseReadyState::Open] {
            assert!(
                es_keepalive(state, true, |_| false),
                "a queued task must keep a non-CLOSED source alive regardless of listeners",
            );
            // A named-event-only wrapper (out of the {message,error} tier) is the
            // F3 case: the tier would NOT keep it, but the queued task does.
            assert!(
                es_keepalive(state, true, |e| e == "foo"),
                "a queued task keeps a named-event-only wrapper alive",
            );
        }
        // CLOSED + has_queued_task must NOT be kept (Codex R2b-A): a CLOSED
        // source's buffered events are dropped by dispatch (`dispatch_sse_event`
        // guards CLOSED, R2b-B), so rooting it on a queued task is a pure leak.
        assert!(
            !es_keepalive(SseReadyState::Closed, true, |_| false),
            "a queued task must NOT keep a CLOSED source alive (R2b-A)",
        );
        assert!(
            !es_keepalive(SseReadyState::Closed, true, |e| e == "foo"),
            "a queued task must NOT keep a CLOSED named-event-only source alive (R2b-A)",
        );
    }
}
