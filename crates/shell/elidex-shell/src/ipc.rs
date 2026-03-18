//! Inter-thread communication types for browser ↔ content thread messaging.
//!
//! Defines the message protocol and a bidirectional channel abstraction
//! used between the browser thread (window, events, rendering) and the
//! content thread (DOM, JS, style, layout).

use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, TryRecvError};

use elidex_render::DisplayList;

/// Winit-independent modifier key state.
#[derive(Clone, Copy, Debug, Default)]
#[allow(clippy::struct_excessive_bools)] // Matches DOM UIEvent modifier key set (alt/ctrl/meta/shift).
pub struct ModifierState {
    /// Alt/Option key.
    pub alt: bool,
    /// Control key.
    pub ctrl: bool,
    /// Meta/Command/Windows key.
    pub meta: bool,
    /// Shift key.
    pub shift: bool,
}

/// Mouse click event data.
///
/// Bundles content-relative coordinates, viewport coordinates, button number,
/// and modifier key state for a mouse click.
#[derive(Clone, Debug)]
pub struct MouseClickEvent {
    /// `(x, y)` position in content area (for hit testing).
    pub point: (f32, f32),
    /// `(x, y)` position in viewport (for DOM event clientX/clientY).
    pub client_point: (f64, f64),
    /// Mouse button number (DOM spec: 0=primary, 1=aux, 2=secondary).
    pub button: u8,
    /// Modifier keys held during click.
    pub mods: ModifierState,
}

/// Messages sent from the browser thread to the content thread.
#[derive(Debug)]
pub enum BrowserToContent {
    /// Navigate to a URL.
    Navigate(url::Url),
    /// Mouse button pressed at content-relative coordinates.
    MouseClick(MouseClickEvent),
    /// Mouse button released.
    ///
    /// Per UI Events spec, `:active` pseudo-class applies from mousedown
    /// to mouseup. This message signals the content thread to clear ACTIVE.
    MouseRelease {
        /// Mouse button number.
        button: u8,
    },
    /// Mouse moved to content-relative coordinates.
    MouseMove {
        /// `(x, y)` position in content area (for hit testing).
        point: (f32, f32),
        /// `(x, y)` position in viewport (for DOM event clientX/clientY).
        client_point: (f64, f64),
    },
    /// Cursor left the content area.
    CursorLeft,
    /// Key pressed.
    KeyDown {
        /// DOM key value.
        key: String,
        /// DOM code value.
        code: String,
        /// Whether this is a repeat event.
        repeat: bool,
        /// Modifier keys.
        mods: ModifierState,
    },
    /// Key released.
    KeyUp {
        /// DOM key value.
        key: String,
        /// DOM code value.
        code: String,
        /// Whether this is a repeat event (always false for keyup).
        repeat: bool,
        /// Modifier keys.
        mods: ModifierState,
    },
    /// Viewport size changed.
    SetViewport {
        /// New width in logical pixels.
        width: f32,
        /// New height in logical pixels.
        height: f32,
    },
    /// Navigate back in history.
    GoBack,
    /// Navigate forward in history.
    GoForward,
    /// Reload the current page.
    Reload,
    /// Mouse wheel scrolled in the content area.
    MouseWheel {
        /// `(horizontal, vertical)` scroll delta in CSS pixels (positive = scroll right/down).
        delta: (f64, f64),
        /// `(x, y)` content-relative coordinates for scroll target hit testing.
        point: (f32, f32),
    },
    /// IME event.
    Ime {
        /// The IME event kind.
        kind: ImeKind,
    },
    /// Shut down the content thread.
    Shutdown,
}

/// IME event kinds.
#[derive(Clone, Debug)]
pub enum ImeKind {
    /// IME composition started or text updated.
    Preedit(String),
    /// IME composition committed.
    Commit(String),
    /// IME enabled.
    Enabled,
    /// IME disabled.
    Disabled,
}

/// Messages sent from the content thread to the browser thread.
#[derive(Debug)]
pub enum ContentToBrowser {
    /// A new display list is ready for rendering.
    DisplayListReady(DisplayList),
    /// The page title changed.
    TitleChanged(String),
    /// Navigation state changed (for chrome back/forward button states).
    NavigationState {
        /// Whether back navigation is available.
        can_go_back: bool,
        /// Whether forward navigation is available.
        can_go_forward: bool,
    },
    /// The current URL changed (for chrome address bar).
    UrlChanged(url::Url),
    /// A navigation request failed.
    NavigationFailed {
        /// The URL that failed to load.
        url: url::Url,
        /// Human-readable error description.
        error: String,
    },
}

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
/// Returns `(browser_end, content_end)` where:
/// - `browser_end` sends `A` and receives `B`
/// - `content_end` sends `B` and receives `A`
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

    #[test]
    fn modifier_state_default() {
        let m = ModifierState::default();
        assert!(!m.alt);
        assert!(!m.ctrl);
        assert!(!m.meta);
        assert!(!m.shift);
    }
}
