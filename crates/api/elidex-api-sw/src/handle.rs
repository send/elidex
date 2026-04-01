//! Service Worker thread handle (parent-side).
//!
//! Follows the `WorkerHandle` pattern from `elidex-api-workers`.

use std::thread::JoinHandle;
use std::time::Duration;

use elidex_plugin::LocalChannel;

use crate::registration::SwState;
use crate::types::{ContentToSw, SwToContent};

/// Default idle timeout before terminating an idle SW thread.
pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(30);

/// Handle to a running Service Worker thread.
///
/// Manages the IPC channel and thread lifecycle.
/// Sends `Shutdown` on drop if the thread is still running.
pub struct SwHandle {
    channel: LocalChannel<ContentToSw, SwToContent>,
    thread: Option<JoinHandle<()>>,
    scope: url::Url,
    script_url: url::Url,
    state: SwState,
    idle_timeout: Duration,
}

impl SwHandle {
    pub fn new(
        scope: url::Url,
        script_url: url::Url,
        channel: LocalChannel<ContentToSw, SwToContent>,
        thread: JoinHandle<()>,
    ) -> Self {
        Self {
            channel,
            thread: Some(thread),
            scope,
            script_url,
            state: SwState::Parsed,
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
        }
    }

    pub fn scope(&self) -> &url::Url {
        &self.scope
    }

    pub fn script_url(&self) -> &url::Url {
        &self.script_url
    }

    pub fn state(&self) -> SwState {
        self.state
    }

    pub fn set_state(&mut self, state: SwState) {
        self.state = state;
    }

    pub fn idle_timeout(&self) -> Duration {
        self.idle_timeout
    }

    /// Send a message to the SW thread.
    pub fn send(&self, msg: ContentToSw) {
        let _ = self.channel.send(msg);
    }

    /// Try to receive a message from the SW thread (non-blocking).
    pub fn try_recv(&self) -> Result<SwToContent, crossbeam_channel::TryRecvError> {
        self.channel.try_recv()
    }

    /// Receive with timeout.
    pub fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> Result<SwToContent, crossbeam_channel::RecvTimeoutError> {
        self.channel.recv_timeout(timeout)
    }

    /// Shut down the SW thread.
    pub fn shutdown(&mut self) {
        let _ = self.channel.send(ContentToSw::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        self.state = SwState::Redundant;
    }

    /// Check if the SW thread is still running.
    pub fn is_alive(&self) -> bool {
        self.thread
            .as_ref()
            .is_some_and(|t| !t.is_finished())
    }
}

impl Drop for SwHandle {
    fn drop(&mut self) {
        if self.thread.is_some() {
            self.shutdown();
        }
    }
}

impl std::fmt::Debug for SwHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwHandle")
            .field("scope", &self.scope.as_str())
            .field("script_url", &self.script_url.as_str())
            .field("state", &self.state)
            .field("alive", &self.is_alive())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> url::Url {
        url::Url::parse(s).unwrap()
    }

    #[test]
    fn handle_defaults() {
        let (ch1, ch2) = elidex_plugin::channel_pair();
        let thread = std::thread::spawn(move || {
            // Simulate SW thread: wait for shutdown
            loop {
                match ch2.recv_timeout(Duration::from_millis(100)) {
                    Ok(ContentToSw::Shutdown) | Err(_) => break,
                    _ => {}
                }
            }
        });

        let handle = SwHandle::new(
            url("https://example.com/"),
            url("https://example.com/sw.js"),
            ch1,
            thread,
        );

        assert_eq!(handle.state(), SwState::Parsed);
        assert_eq!(handle.scope().as_str(), "https://example.com/");
        assert_eq!(handle.idle_timeout(), DEFAULT_IDLE_TIMEOUT);
        // Drop triggers shutdown
    }
}
