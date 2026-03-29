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
#[allow(clippy::struct_excessive_bools)] // DOM UIEvent spec requires 4 modifier key booleans.
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
#[allow(clippy::struct_excessive_bools)] // DOM UIEvent spec requires 4 modifier key booleans.
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

/// Initialization data for CSS transition events (CSS Transitions Level 1 §6).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransitionEventInit {
    /// The CSS property name that completed the transition.
    pub property_name: String,
    /// The elapsed time of the transition in seconds.
    pub elapsed_time: f64,
    /// The `::pseudo-element` selector (empty string if not a pseudo-element).
    pub pseudo_element: String,
}

/// Initialization data for CSS animation events (CSS Animations Level 1 §4.2).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnimationEventInit {
    /// The `@keyframes` animation name.
    pub animation_name: String,
    /// The elapsed time of the animation in seconds.
    pub elapsed_time: f64,
    /// The `::pseudo-element` selector (empty string if not a pseudo-element).
    pub pseudo_element: String,
}

/// Initialization data for input events (HTML §4.10.5.5).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InputEventInit {
    /// The type of input (e.g. "insertText", "deleteContentBackward").
    pub input_type: String,
    /// The data being inserted (if any).
    pub data: Option<String>,
    /// Whether an IME composition is in progress.
    pub is_composing: bool,
}

/// Initialization data for clipboard events (HTML §6.4.6).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClipboardEventInit {
    /// The clipboard data type (e.g. "text/plain").
    pub data_type: String,
    /// The clipboard data.
    pub data: String,
}

/// Initialization data for composition events (HTML §6.5).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompositionEventInit {
    /// The composition data (text being composed).
    pub data: String,
}

/// Initialization data for focus events (UI Events §5.2).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FocusEventInit {
    /// The entity that is losing/gaining focus (the "other" target).
    pub related_target: Option<u64>,
}

/// Initialization data for wheel events (UI Events §5.4).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WheelEventInit {
    /// Horizontal scroll amount.
    pub delta_x: f64,
    /// Vertical scroll amount.
    pub delta_y: f64,
    /// Delta mode: 0 = pixel, 1 = line, 2 = page (`DOM_DELTA_*`).
    pub delta_mode: u32,
}

/// Payload carried by a DOM event.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum EventPayload {
    /// Mouse event data.
    Mouse(MouseEventInit),
    /// Keyboard event data.
    Keyboard(KeyboardEventInit),
    /// CSS transition event data.
    Transition(TransitionEventInit),
    /// CSS animation event data.
    Animation(AnimationEventInit),
    /// Input event data (text editing).
    Input(InputEventInit),
    /// Clipboard event data.
    Clipboard(ClipboardEventInit),
    /// Composition event data (IME).
    Composition(CompositionEventInit),
    /// Focus event data (focus/blur/focusin/focusout).
    Focus(FocusEventInit),
    /// Wheel event data.
    Wheel(WheelEventInit),
    /// Scroll event (no additional data — target is the scroll container).
    Scroll,
    /// Cross-document message event data (WHATWG HTML §9.4.3, §9.2, §9.3).
    ///
    /// Used for `postMessage`, WebSocket `onmessage`, and SSE events.
    Message {
        /// Message data (string for text, base64 for binary).
        data: String,
        /// Serialized origin of the sender.
        origin: String,
        /// Last event ID (empty for postMessage/WebSocket, sticky for SSE).
        last_event_id: String,
    },
    /// WebSocket/SSE close event data (WHATWG HTML `CloseEvent`).
    CloseEvent(CloseEventInit),
    /// `HashChange` event data (WHATWG HTML §7.7.4).
    HashChange(HashChangeEventInit),
    /// Page transition event data (WHATWG HTML §7.8.2.4).
    PageTransition(PageTransitionEventInit),
    /// Storage event data (WHATWG HTML §11.2.1).
    Storage {
        /// The key that changed (`None` for `clear()`).
        key: Option<String>,
        /// The old value (`None` if the key was newly set or cleared).
        old_value: Option<String>,
        /// The new value (`None` if the key was removed or cleared).
        new_value: Option<String>,
        /// The URL of the document that triggered the change.
        url: String,
    },
    /// No additional data (e.g. generic events).
    #[default]
    None,
}

/// Initialization data for hashchange events (WHATWG HTML §7.7.4).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HashChangeEventInit {
    /// The previous URL (before the hash change).
    pub old_url: String,
    /// The new URL (after the hash change).
    pub new_url: String,
}

/// Initialization data for pagehide/pageshow events (WHATWG HTML §7.8.2.4).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PageTransitionEventInit {
    /// Whether the page was restored from `BFCache`.
    pub persisted: bool,
}

/// Close event initialization data (WHATWG HTML `CloseEvent`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CloseEventInit {
    /// The WebSocket connection close code (RFC 6455 §7.4).
    pub code: u16,
    /// The close reason string.
    pub reason: String,
    /// Whether the connection was closed cleanly (close handshake completed).
    pub was_clean: bool,
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

    #[test]
    fn transition_event_init_default() {
        let t = TransitionEventInit::default();
        assert!(t.property_name.is_empty());
        assert!((t.elapsed_time - 0.0).abs() < f64::EPSILON);
        assert!(t.pseudo_element.is_empty());
    }

    #[test]
    fn animation_event_init_default() {
        let a = AnimationEventInit::default();
        assert!(a.animation_name.is_empty());
        assert!((a.elapsed_time - 0.0).abs() < f64::EPSILON);
        assert!(a.pseudo_element.is_empty());
    }

    #[test]
    fn event_payload_transition_variant() {
        let p = EventPayload::Transition(TransitionEventInit {
            property_name: "opacity".into(),
            elapsed_time: 0.5,
            pseudo_element: String::new(),
        });
        assert!(matches!(p, EventPayload::Transition(_)));
    }

    #[test]
    fn event_payload_animation_variant() {
        let p = EventPayload::Animation(AnimationEventInit {
            animation_name: "fadeIn".into(),
            elapsed_time: 1.0,
            pseudo_element: String::new(),
        });
        assert!(matches!(p, EventPayload::Animation(_)));
    }

    #[test]
    fn close_event_init_default() {
        let init = CloseEventInit::default();
        assert_eq!(init.code, 0);
        assert!(init.reason.is_empty());
        assert!(!init.was_clean);
    }

    #[test]
    fn close_event_payload_variant() {
        let payload = EventPayload::CloseEvent(CloseEventInit {
            code: 1000,
            reason: "normal".to_string(),
            was_clean: true,
        });
        assert!(matches!(payload, EventPayload::CloseEvent(_)));
    }

    #[test]
    fn message_payload_with_last_event_id() {
        let payload = EventPayload::Message {
            data: "hello".to_string(),
            origin: "https://example.com".to_string(),
            last_event_id: "42".to_string(),
        };
        if let EventPayload::Message { last_event_id, .. } = payload {
            assert_eq!(last_event_id, "42");
        } else {
            panic!("expected Message");
        }
    }
}
