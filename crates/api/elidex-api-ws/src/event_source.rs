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
}
