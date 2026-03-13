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
    pub elapsed_time: f32,
    /// The `::pseudo-element` selector (empty string if not a pseudo-element).
    pub pseudo_element: String,
}

/// Initialization data for CSS animation events (CSS Animations Level 1 §4.2).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnimationEventInit {
    /// The `@keyframes` animation name.
    pub animation_name: String,
    /// The elapsed time of the animation in seconds.
    pub elapsed_time: f32,
    /// The `::pseudo-element` selector (empty string if not a pseudo-element).
    pub pseudo_element: String,
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

    #[test]
    fn transition_event_init_default() {
        let t = TransitionEventInit::default();
        assert!(t.property_name.is_empty());
        assert!((t.elapsed_time - 0.0).abs() < f32::EPSILON);
        assert!(t.pseudo_element.is_empty());
    }

    #[test]
    fn animation_event_init_default() {
        let a = AnimationEventInit::default();
        assert!(a.animation_name.is_empty());
        assert!((a.elapsed_time - 0.0).abs() < f32::EPSILON);
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
}
