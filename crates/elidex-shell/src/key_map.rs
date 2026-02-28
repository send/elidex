//! Mapping winit keyboard events to DOM `key` and `code` strings.
//!
//! Covers the most common keys used in web applications.

use winit::keyboard::{Key, NamedKey, PhysicalKey};

/// Convert winit key information to DOM `(key, code)` strings.
///
/// The `key` value corresponds to `KeyboardEvent.key` in the DOM spec.
/// The `code` value corresponds to `KeyboardEvent.code` (physical key position).
#[must_use]
pub fn winit_key_to_dom(logical: &Key, physical: &PhysicalKey) -> (String, String) {
    let key = match logical {
        Key::Named(named) => named_key_to_dom(*named).to_string(),
        Key::Character(c) => c.to_string(),
        _ => "Unidentified".to_string(),
    };

    let code = match physical {
        PhysicalKey::Code(kc) => physical_key_to_dom(*kc).to_string(),
        PhysicalKey::Unidentified(_) => "Unidentified".to_string(),
    };

    (key, code)
}

fn named_key_to_dom(key: NamedKey) -> &'static str {
    match key {
        NamedKey::Enter => "Enter",
        NamedKey::Space => " ",
        NamedKey::Tab => "Tab",
        NamedKey::Escape => "Escape",
        NamedKey::Backspace => "Backspace",
        NamedKey::Delete => "Delete",
        NamedKey::ArrowUp => "ArrowUp",
        NamedKey::ArrowDown => "ArrowDown",
        NamedKey::ArrowLeft => "ArrowLeft",
        NamedKey::ArrowRight => "ArrowRight",
        NamedKey::Home => "Home",
        NamedKey::End => "End",
        NamedKey::PageUp => "PageUp",
        NamedKey::PageDown => "PageDown",
        NamedKey::Shift => "Shift",
        NamedKey::Control => "Control",
        NamedKey::Alt => "Alt",
        NamedKey::Super => "Meta",
        NamedKey::CapsLock => "CapsLock",
        NamedKey::F1 => "F1",
        NamedKey::F2 => "F2",
        NamedKey::F3 => "F3",
        NamedKey::F4 => "F4",
        NamedKey::F5 => "F5",
        NamedKey::F6 => "F6",
        NamedKey::F7 => "F7",
        NamedKey::F8 => "F8",
        NamedKey::F9 => "F9",
        NamedKey::F10 => "F10",
        NamedKey::F11 => "F11",
        NamedKey::F12 => "F12",
        NamedKey::Insert => "Insert",
        _ => "Unidentified",
    }
}

fn physical_key_to_dom(code: winit::keyboard::KeyCode) -> &'static str {
    use winit::keyboard::KeyCode;

    match code {
        // Letters
        KeyCode::KeyA => "KeyA",
        KeyCode::KeyB => "KeyB",
        KeyCode::KeyC => "KeyC",
        KeyCode::KeyD => "KeyD",
        KeyCode::KeyE => "KeyE",
        KeyCode::KeyF => "KeyF",
        KeyCode::KeyG => "KeyG",
        KeyCode::KeyH => "KeyH",
        KeyCode::KeyI => "KeyI",
        KeyCode::KeyJ => "KeyJ",
        KeyCode::KeyK => "KeyK",
        KeyCode::KeyL => "KeyL",
        KeyCode::KeyM => "KeyM",
        KeyCode::KeyN => "KeyN",
        KeyCode::KeyO => "KeyO",
        KeyCode::KeyP => "KeyP",
        KeyCode::KeyQ => "KeyQ",
        KeyCode::KeyR => "KeyR",
        KeyCode::KeyS => "KeyS",
        KeyCode::KeyT => "KeyT",
        KeyCode::KeyU => "KeyU",
        KeyCode::KeyV => "KeyV",
        KeyCode::KeyW => "KeyW",
        KeyCode::KeyX => "KeyX",
        KeyCode::KeyY => "KeyY",
        KeyCode::KeyZ => "KeyZ",
        // Digits
        KeyCode::Digit0 => "Digit0",
        KeyCode::Digit1 => "Digit1",
        KeyCode::Digit2 => "Digit2",
        KeyCode::Digit3 => "Digit3",
        KeyCode::Digit4 => "Digit4",
        KeyCode::Digit5 => "Digit5",
        KeyCode::Digit6 => "Digit6",
        KeyCode::Digit7 => "Digit7",
        KeyCode::Digit8 => "Digit8",
        KeyCode::Digit9 => "Digit9",
        // Special keys
        KeyCode::Enter => "Enter",
        KeyCode::Space => "Space",
        KeyCode::Tab => "Tab",
        KeyCode::Escape => "Escape",
        KeyCode::Backspace => "Backspace",
        KeyCode::Delete => "Delete",
        KeyCode::ArrowUp => "ArrowUp",
        KeyCode::ArrowDown => "ArrowDown",
        KeyCode::ArrowLeft => "ArrowLeft",
        KeyCode::ArrowRight => "ArrowRight",
        KeyCode::Home => "Home",
        KeyCode::End => "End",
        KeyCode::PageUp => "PageUp",
        KeyCode::PageDown => "PageDown",
        KeyCode::ShiftLeft => "ShiftLeft",
        KeyCode::ShiftRight => "ShiftRight",
        KeyCode::ControlLeft => "ControlLeft",
        KeyCode::ControlRight => "ControlRight",
        KeyCode::AltLeft => "AltLeft",
        KeyCode::AltRight => "AltRight",
        KeyCode::SuperLeft => "MetaLeft",
        KeyCode::SuperRight => "MetaRight",
        KeyCode::CapsLock => "CapsLock",
        _ => "Unidentified",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::{KeyCode, SmolStr};

    #[test]
    fn enter_key_mapping() {
        let logical = Key::Named(NamedKey::Enter);
        let physical = PhysicalKey::Code(KeyCode::Enter);
        let (key, code) = winit_key_to_dom(&logical, &physical);
        assert_eq!(key, "Enter");
        assert_eq!(code, "Enter");
    }

    #[test]
    fn character_key_mapping() {
        let logical = Key::Character(SmolStr::new("a"));
        let physical = PhysicalKey::Code(KeyCode::KeyA);
        let (key, code) = winit_key_to_dom(&logical, &physical);
        assert_eq!(key, "a");
        assert_eq!(code, "KeyA");
    }

    #[test]
    fn arrow_key_mapping() {
        let logical = Key::Named(NamedKey::ArrowUp);
        let physical = PhysicalKey::Code(KeyCode::ArrowUp);
        let (key, code) = winit_key_to_dom(&logical, &physical);
        assert_eq!(key, "ArrowUp");
        assert_eq!(code, "ArrowUp");
    }

    #[test]
    fn space_key_mapping() {
        let logical = Key::Named(NamedKey::Space);
        let physical = PhysicalKey::Code(KeyCode::Space);
        let (key, code) = winit_key_to_dom(&logical, &physical);
        assert_eq!(key, " ");
        assert_eq!(code, "Space");
    }

    #[test]
    fn unidentified_key() {
        let logical = Key::Dead(None);
        let physical = PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified);
        let (key, code) = winit_key_to_dom(&logical, &physical);
        assert_eq!(key, "Unidentified");
        assert_eq!(code, "Unidentified");
    }
}
