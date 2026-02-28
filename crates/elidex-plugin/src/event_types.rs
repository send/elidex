//! DOM event types shared across the elidex engine.
//!
//! These types are engine-independent — they carry event data without
//! referencing any JS engine.

/// DOM event propagation phase.
///
/// Corresponds to `Event.eventPhase` values in the DOM spec.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[repr(u8)]
#[non_exhaustive]
pub enum EventPhase {
    /// No event is being processed.
    #[default]
    None = 0,
    /// The event is propagating down to the target (capture phase).
    Capturing = 1,
    /// The event has reached the target element.
    AtTarget = 2,
    /// The event is propagating back up from the target (bubble phase).
    Bubbling = 3,
}

/// Initialization data for mouse events.
#[derive(Clone, Debug, Default, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct MouseEventInit {
    /// Horizontal coordinate relative to the viewport.
    pub client_x: f64,
    /// Vertical coordinate relative to the viewport.
    pub client_y: f64,
    /// The button that triggered the event (0=left, 1=middle, 2=right).
    pub button: i16,
    /// Bitmask of currently pressed buttons.
    pub buttons: u16,
    /// `true` if the Alt key was held.
    pub alt_key: bool,
    /// `true` if the Ctrl key was held.
    pub ctrl_key: bool,
    /// `true` if the Meta key was held.
    pub meta_key: bool,
    /// `true` if the Shift key was held.
    pub shift_key: bool,
}

/// Initialization data for keyboard events.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct KeyboardEventInit {
    /// The `key` attribute value (e.g. `"Enter"`, `"a"`).
    pub key: String,
    /// The `code` attribute value (e.g. `"Enter"`, `"KeyA"`).
    pub code: String,
    /// `true` if the Alt key was held.
    pub alt_key: bool,
    /// `true` if the Ctrl key was held.
    pub ctrl_key: bool,
    /// `true` if the Meta key was held.
    pub meta_key: bool,
    /// `true` if the Shift key was held.
    pub shift_key: bool,
    /// `true` if the key is being held down (repeat event).
    pub repeat: bool,
}

/// Payload carried by a DOM event.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum EventPayload {
    /// Mouse event data.
    Mouse(MouseEventInit),
    /// Keyboard event data.
    Keyboard(KeyboardEventInit),
    /// No additional data (e.g. generic events).
    #[default]
    None,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_phase_default_is_none() {
        assert_eq!(EventPhase::default(), EventPhase::None);
    }

    #[test]
    fn event_phase_discriminant_values() {
        assert_eq!(EventPhase::None as u8, 0);
        assert_eq!(EventPhase::Capturing as u8, 1);
        assert_eq!(EventPhase::AtTarget as u8, 2);
        assert_eq!(EventPhase::Bubbling as u8, 3);
    }

    #[test]
    fn mouse_event_init_default() {
        let m = MouseEventInit::default();
        assert_eq!(m.client_x, 0.0);
        assert_eq!(m.client_y, 0.0);
        assert_eq!(m.button, 0);
        assert_eq!(m.buttons, 0);
        assert!(!m.alt_key);
        assert!(!m.ctrl_key);
        assert!(!m.meta_key);
        assert!(!m.shift_key);
    }

    #[test]
    fn keyboard_event_init_default() {
        let k = KeyboardEventInit::default();
        assert!(k.key.is_empty());
        assert!(k.code.is_empty());
        assert!(!k.alt_key);
        assert!(!k.repeat);
    }

    #[test]
    fn event_payload_clone_eq() {
        let p1 = EventPayload::Mouse(MouseEventInit {
            client_x: 10.0,
            client_y: 20.0,
            button: 0,
            ..Default::default()
        });
        let p2 = p1.clone();
        assert_eq!(p1, p2);

        let p3 = EventPayload::Keyboard(KeyboardEventInit {
            key: "Enter".into(),
            code: "Enter".into(),
            ..Default::default()
        });
        assert_ne!(p1, p3);
    }
}
