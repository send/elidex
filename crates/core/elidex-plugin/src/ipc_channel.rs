//! Process-transparent IPC channel abstraction.
//!
//! Defines the [`IpcChannel`] trait (design doc 05-process-async.md §5.3.1)
//! that abstracts over in-process and cross-process communication.
//!
//! [`LocalChannel`](crate::LocalChannel) implements this trait using
//! `crossbeam_channel` for zero-copy in-process messaging. A future
//! `ProcessChannel` implementation will use `ipc-channel` + `postcard`
//! for cross-process messaging with serialization.

use std::fmt;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned when sending a message fails because the receiver is
/// disconnected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcSendError;

impl fmt::Display for IpcSendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("IPC channel disconnected")
    }
}

impl std::error::Error for IpcSendError {}

/// Error returned by [`IpcChannel::try_recv`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcTryRecvError {
    /// No message is available right now.
    Empty,
    /// The sender has disconnected.
    Disconnected,
}

impl fmt::Display for IpcTryRecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("channel empty"),
            Self::Disconnected => f.write_str("IPC channel disconnected"),
        }
    }
}

impl std::error::Error for IpcTryRecvError {}

/// Error returned by [`IpcChannel::recv_timeout`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcRecvTimeoutError {
    /// No message arrived within the timeout.
    Timeout,
    /// The sender has disconnected.
    Disconnected,
}

impl fmt::Display for IpcRecvTimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => f.write_str("IPC recv timed out"),
            Self::Disconnected => f.write_str("IPC channel disconnected"),
        }
    }
}

impl std::error::Error for IpcRecvTimeoutError {}

// ---------------------------------------------------------------------------
// IpcChannel trait
// ---------------------------------------------------------------------------

/// Process-transparent IPC channel (design doc §5.3.1).
///
/// Implementations:
/// - [`LocalChannel`](crate::LocalChannel): in-process, zero-copy (crossbeam).
/// - `ProcessChannel` (future): cross-process, serialized (ipc-channel + postcard).
///
/// Content threads and broker threads program against this trait so that the
/// same code works regardless of whether the target is in-process or in a
/// separate OS process.
pub trait IpcChannel<S, R>: Send + Sync {
    /// Send a message. Returns `Err` if the receiver has disconnected.
    fn send(&self, message: S) -> Result<(), IpcSendError>;

    /// Try to receive a message without blocking.
    fn try_recv(&self) -> Result<R, IpcTryRecvError>;

    /// Receive a message, blocking up to `timeout`.
    fn recv_timeout(&self, timeout: Duration) -> Result<R, IpcRecvTimeoutError>;
}

// ---------------------------------------------------------------------------
// LocalChannel impl
// ---------------------------------------------------------------------------

impl<S: Send, R: Send> IpcChannel<S, R> for crate::LocalChannel<S, R> {
    fn send(&self, message: S) -> Result<(), IpcSendError> {
        self.send(message).map_err(|_| IpcSendError)
    }

    fn try_recv(&self) -> Result<R, IpcTryRecvError> {
        self.try_recv().map_err(|e| match e {
            crossbeam_channel::TryRecvError::Empty => IpcTryRecvError::Empty,
            crossbeam_channel::TryRecvError::Disconnected => IpcTryRecvError::Disconnected,
        })
    }

    fn recv_timeout(&self, timeout: Duration) -> Result<R, IpcRecvTimeoutError> {
        self.recv_timeout(timeout).map_err(|e| match e {
            crossbeam_channel::RecvTimeoutError::Timeout => IpcRecvTimeoutError::Timeout,
            crossbeam_channel::RecvTimeoutError::Disconnected => IpcRecvTimeoutError::Disconnected,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel_pair;

    #[test]
    fn ipc_channel_send_recv() {
        let (a, b) = channel_pair::<String, u32>();
        let a: &dyn IpcChannel<String, u32> = &a;
        let b: &dyn IpcChannel<u32, String> = &b;

        a.send("hello".to_string()).unwrap();
        b.send(42).unwrap();

        assert_eq!(IpcChannel::try_recv(b), Ok("hello".to_string()));
        assert_eq!(IpcChannel::try_recv(a), Ok(42));
    }

    #[test]
    fn ipc_channel_try_recv_empty() {
        let (a, _b) = channel_pair::<String, u32>();
        let a: &dyn IpcChannel<String, u32> = &a;
        assert_eq!(IpcChannel::try_recv(a), Err(IpcTryRecvError::Empty));
    }

    #[test]
    fn ipc_channel_recv_timeout() {
        let (a, _b) = channel_pair::<String, u32>();
        let a: &dyn IpcChannel<String, u32> = &a;
        let result = a.recv_timeout(Duration::from_millis(1));
        assert_eq!(result, Err(IpcRecvTimeoutError::Timeout));
    }

    #[test]
    fn ipc_channel_disconnect() {
        let (a, b) = channel_pair::<String, u32>();
        drop(b);
        let a: &dyn IpcChannel<String, u32> = &a;
        assert!(a.send("test".to_string()).is_err());
        assert_eq!(IpcChannel::try_recv(a), Err(IpcTryRecvError::Disconnected));
    }
}
