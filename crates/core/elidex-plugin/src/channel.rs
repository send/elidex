//! Bidirectional channel abstraction built on `crossbeam_channel`.
//!
//! Provides [`LocalChannel`] and [`channel_pair()`], shared by the browser ↔
//! content IPC layer and the parent ↔ worker messaging layer.

use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, TryRecvError};

/// A bidirectional channel endpoint.
///
/// Each endpoint can send messages of type `S` and receive messages of type `R`.
pub struct LocalChannel<S, R> {
    tx: Sender<S>,
    rx: Receiver<R>,
}

impl<S, R> LocalChannel<S, R> {
    /// Send a message. Returns `Err` if the other end is disconnected.
    pub fn send(&self, msg: S) -> Result<(), crossbeam_channel::SendError<S>> {
        self.tx.send(msg)
    }

    /// Try to receive a message without blocking.
    pub fn try_recv(&self) -> Result<R, TryRecvError> {
        self.rx.try_recv()
    }

    /// Receive a message with a timeout.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<R, RecvTimeoutError> {
        self.rx.recv_timeout(timeout)
    }
}

/// Create a pair of connected bidirectional channels.
///
/// Returns `(a_end, b_end)` where:
/// - `a_end` sends `A` and receives `B`
/// - `b_end` sends `B` and receives `A`
///
/// Uses unbounded channels: the browser sends at most one message per input event,
/// and the content thread drains messages on each loop iteration, so the queue
/// depth is naturally bounded by input rate. Bounded channels risk deadlock when
/// both sides send simultaneously (each blocks waiting for the other to recv).
pub fn channel_pair<A: Send, B: Send>() -> (LocalChannel<A, B>, LocalChannel<B, A>) {
    let (tx_a, rx_a) = crossbeam_channel::unbounded();
    let (tx_b, rx_b) = crossbeam_channel::unbounded();
    (
        LocalChannel { tx: tx_a, rx: rx_b },
        LocalChannel { tx: tx_b, rx: rx_a },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_pair_roundtrip() {
        let (a, b) = channel_pair::<String, u32>();
        a.send("hello".to_string()).unwrap();
        b.send(42).unwrap();

        assert_eq!(b.try_recv().unwrap(), "hello");
        assert_eq!(a.try_recv().unwrap(), 42);
    }

    #[test]
    fn try_recv_empty() {
        let (a, _b) = channel_pair::<String, u32>();
        assert!(a.try_recv().is_err());
    }

    #[test]
    fn recv_timeout_empty() {
        let (a, _b) = channel_pair::<String, u32>();
        let result = a.recv_timeout(Duration::from_millis(1));
        assert!(matches!(result, Err(RecvTimeoutError::Timeout)));
    }

    #[test]
    fn disconnect_detected() {
        let (a, b) = channel_pair::<String, u32>();
        drop(b);
        assert!(a.send("test".to_string()).is_err());
    }

    #[test]
    fn multiple_messages_ordered() {
        let (a, b) = channel_pair::<u32, u32>();
        for i in 0..5 {
            a.send(i).unwrap();
        }
        for i in 0..5 {
            assert_eq!(b.try_recv().unwrap(), i);
        }
    }
}
