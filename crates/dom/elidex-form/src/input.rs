//! Keyboard input handling for text-based form controls.

use crate::util::{next_char_boundary, prev_char_boundary};
use crate::{FormControlKind, FormControlState};

/// Action returned from key input processing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyAction {
    /// The key was consumed (value modified or cursor moved).
    Consumed,
    /// Enter pressed on a text input — trigger implicit form submission.
    Submit,
    /// The key was not consumed.
    None,
}

/// Process a key press on a text form control.
///
/// Returns a `KeyAction` indicating what happened.
/// Handles `TextInput`, `Password`, and `TextArea` controls.
#[must_use]
pub fn form_control_key_input(state: &mut FormControlState, key: &str, code: &str) -> bool {
    form_control_key_input_action(state, key, code) != KeyAction::None
}

/// Process a key press with detailed action result.
#[must_use]
pub fn form_control_key_input_action(
    state: &mut FormControlState,
    key: &str,
    _code: &str,
) -> KeyAction {
    match state.kind {
        FormControlKind::TextInput
        | FormControlKind::Password
        | FormControlKind::TextArea
        | FormControlKind::Email
        | FormControlKind::Url
        | FormControlKind::Tel
        | FormControlKind::Search
        | FormControlKind::Number => {
            state.cursor_pos = state.safe_cursor_pos();

            // Clear selection on non-shift navigation keys.
            if state.selection_start != state.selection_end && !matches!(key, "Shift") {
                match key {
                    "Backspace" | "Delete" => {
                        // Delete selection.
                        let (start, end) = state.safe_selection_range();
                        state.value.drain(start..end);
                        state.cursor_pos = start;
                        state.selection_start = 0;
                        state.selection_end = 0;
                        state.dirty_value = true;
                        state.update_char_count();
                        return KeyAction::Consumed;
                    }
                    k if k.len() == 1 || (k.chars().count() == 1 && !k.starts_with("Arrow")) => {
                        // Replace selection with typed character.
                        let ch = k.chars().next().unwrap();
                        if !ch.is_control() {
                            let (start, end) = state.safe_selection_range();
                            state.value.drain(start..end);
                            state.cursor_pos = start;
                            state.value.insert(state.cursor_pos, ch);
                            state.cursor_pos += ch.len_utf8();
                            state.selection_start = 0;
                            state.selection_end = 0;
                            state.dirty_value = true;
                            state.update_char_count();
                            return KeyAction::Consumed;
                        }
                    }
                    _ => {}
                }
            }

            if state.readonly {
                return if handle_readonly_navigation(state, key) {
                    KeyAction::Consumed
                } else {
                    KeyAction::None
                };
            }
            let result = handle_text_key(state, key);
            if result == KeyAction::Consumed {
                state.dirty_value = true;
            }
            result
        }
        _ => KeyAction::None,
    }
}

/// Navigate cursor in a direction. Returns `KeyAction::Consumed` if moved.
fn navigate_cursor(state: &mut FormControlState, key: &str) -> KeyAction {
    match key {
        "ArrowLeft" => {
            if state.cursor_pos > 0 {
                state.cursor_pos = prev_char_boundary(&state.value, state.cursor_pos);
                KeyAction::Consumed
            } else {
                KeyAction::None
            }
        }
        "ArrowRight" => {
            if state.cursor_pos < state.value.len() {
                state.cursor_pos = next_char_boundary(&state.value, state.cursor_pos);
                KeyAction::Consumed
            } else {
                KeyAction::None
            }
        }
        "Home" => {
            if state.cursor_pos > 0 {
                state.cursor_pos = 0;
                KeyAction::Consumed
            } else {
                KeyAction::None
            }
        }
        "End" => {
            let end = state.value.len();
            if state.cursor_pos < end {
                state.cursor_pos = end;
                KeyAction::Consumed
            } else {
                KeyAction::None
            }
        }
        _ => KeyAction::None,
    }
}

/// Error returned by [`apply_step`] when the form control's kind
/// does not support stepping (HTML §4.10.5.4 mandates `InvalidStateError`
/// for these inputs).  Callers convert this to the engine-bound
/// exception type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepError {
    /// `state.kind` is not `Number` or `Range`.
    NotSupported,
}

/// Apply a `stepUp(n)` / `stepDown(n)` adjustment to a form control
/// state (HTML §4.10.5.4).
///
/// `direction` is `+1.0` for `stepUp` and `-1.0` for `stepDown`. The
/// adjustment is computed as `current + direction * n * step`, where
/// `step` is parsed from `state.step` (default `1.0` when missing or
/// unparseable) and `current` is parsed from `state.value()`
/// (default `0.0` when empty or unparseable).
///
/// TODO(spec-compliance): full §4.10.5.4 algorithm requires
/// round-to-base / clamp min-max / `"any"` rejection (`InvalidStateError`).
/// The current implementation is the historical VM behaviour; spec
/// fixes are deferred to slot `#11-input-step-spec-compliance`.
pub fn apply_step(state: &mut FormControlState, n: f64, direction: f64) -> Result<(), StepError> {
    if !matches!(state.kind, FormControlKind::Number | FormControlKind::Range) {
        return Err(StepError::NotSupported);
    }
    let step = state
        .step
        .as_deref()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(1.0);
    let cur = state.value().parse::<f64>().unwrap_or(0.0);
    let new = cur + direction * n * step;
    state.set_value(new.to_string());
    Ok(())
}

/// HTML §4.10.5.6 type-change sanitize step.
///
/// Run after `state.kind` has been updated from `old_kind` to the
/// new value, to bring `FormControlState` back into a consistent
/// shape per the new type's invariants:
///
/// 1. **Checkable-state cleanup**: if the old kind was `Checkbox`
///    or `Radio` and the new kind is neither, clear `checked` and
///    `indeterminate` (HTML §4.10.5.6 step 3.1).  These bits are
///    semantically meaningless on non-checkable types.
/// 2. **Number value sanitization**: if the new kind is `Number`
///    and the current value isn't a finite floating-point literal,
///    clear it (per HTML §4.10.5.4 number value-sanitization
///    algorithm — non-numeric values are rejected to `""`).
///
/// Other per-type sanitize algorithms (Color, URL, Email, Date,
/// Range clamp) are deferred to the next implementation pass — the
/// value-clearing rule for number is the most-frequently-tripping
/// branch in the wild and the only one with a JS-observable
/// regression today.
pub fn sanitize_for_type_change(state: &mut FormControlState, old_kind: FormControlKind) {
    if state.kind == old_kind {
        return;
    }
    let was_checkable = matches!(old_kind, FormControlKind::Checkbox | FormControlKind::Radio);
    let is_checkable = matches!(
        state.kind,
        FormControlKind::Checkbox | FormControlKind::Radio
    );
    if was_checkable && !is_checkable {
        state.checked = false;
        state.indeterminate = false;
    }
    if state.kind == FormControlKind::Number {
        let value_is_valid_number = state.value().parse::<f64>().is_ok_and(f64::is_finite);
        if !value_is_valid_number && !state.value().is_empty() {
            state.set_value(String::new());
        }
    }
}

/// Check if inserting a character would exceed maxlength.
fn would_exceed_maxlength(state: &FormControlState) -> bool {
    if let Some(max) = state.maxlength {
        state.char_count >= max
    } else {
        false
    }
}

/// Check if a character is valid for a Number input.
fn is_valid_number_char(ch: char) -> bool {
    ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == 'e' || ch == 'E' || ch == '+'
}

/// Handle a key press for a text-like control.
fn handle_text_key(state: &mut FormControlState, key: &str) -> KeyAction {
    match key {
        "Backspace" => {
            if state.cursor_pos > 0 {
                let prev = prev_char_boundary(&state.value, state.cursor_pos);
                state.value.drain(prev..state.cursor_pos);
                state.cursor_pos = prev;
                state.update_char_count();
                KeyAction::Consumed
            } else {
                KeyAction::None
            }
        }
        "Delete" => {
            if state.cursor_pos < state.value.len() {
                let next = next_char_boundary(&state.value, state.cursor_pos);
                state.value.drain(state.cursor_pos..next);
                state.update_char_count();
                KeyAction::Consumed
            } else {
                KeyAction::None
            }
        }
        "ArrowLeft" | "ArrowRight" | "Home" | "End" => navigate_cursor(state, key),
        "Enter" => {
            if state.kind == FormControlKind::TextArea {
                state.value.insert(state.cursor_pos, '\n');
                state.cursor_pos += 1;
                state.update_char_count();
                KeyAction::Consumed
            } else if state.kind.is_single_line_text() || state.kind == FormControlKind::Number {
                // Implicit form submission.
                KeyAction::Submit
            } else {
                KeyAction::None
            }
        }
        _ => {
            // Insert printable character (single-char keys only).
            if key.len() == 1 || (key.chars().count() == 1 && !key.starts_with("Arrow")) {
                let ch = key.chars().next().unwrap();
                // HTML spec: single-line inputs reject \n and \r.
                if !ch.is_control()
                    && !(state.kind.is_single_line_text() && (ch == '\n' || ch == '\r'))
                    && !(state.kind == FormControlKind::Number && (ch == '\n' || ch == '\r'))
                {
                    // Number inputs only accept numeric characters.
                    if state.kind == FormControlKind::Number && !is_valid_number_char(ch) {
                        return KeyAction::None;
                    }
                    // Enforce maxlength (HTML spec §4.10.5.2.7).
                    if would_exceed_maxlength(state) {
                        return KeyAction::None;
                    }
                    state.value.insert(state.cursor_pos, ch);
                    state.cursor_pos += ch.len_utf8();
                    state.update_char_count();
                    return KeyAction::Consumed;
                }
            }
            KeyAction::None
        }
    }
}

/// Handle navigation-only keys for readonly text controls.
///
/// Readonly controls still allow cursor movement (ArrowLeft/Right, Home, End)
/// but reject all value-modifying keys (character insert, Backspace, Delete, Enter).
fn handle_readonly_navigation(state: &mut FormControlState, key: &str) -> bool {
    navigate_cursor(state, key) == KeyAction::Consumed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FormControlKind;

    fn text_state(value: &str, cursor: usize) -> FormControlState {
        FormControlState {
            kind: FormControlKind::TextInput,
            value: value.to_string(),
            cursor_pos: cursor,
            char_count: value.chars().count(),
            ..FormControlState::default()
        }
    }

    fn textarea_state(value: &str, cursor: usize) -> FormControlState {
        FormControlState {
            kind: FormControlKind::TextArea,
            value: value.to_string(),
            cursor_pos: cursor,
            char_count: value.chars().count(),
            rows: 2,
            cols: 20,
            ..FormControlState::default()
        }
    }

    #[test]
    fn insert_character() {
        let mut s = text_state("ab", 1);
        assert!(form_control_key_input(&mut s, "x", "KeyX"));
        assert_eq!(s.value, "axb");
        assert_eq!(s.cursor_pos, 2);
    }

    #[test]
    fn insert_at_end() {
        let mut s = text_state("ab", 2);
        assert!(form_control_key_input(&mut s, "c", "KeyC"));
        assert_eq!(s.value, "abc");
        assert_eq!(s.cursor_pos, 3);
    }

    #[test]
    fn backspace_middle() {
        let mut s = text_state("abc", 2);
        assert!(form_control_key_input(&mut s, "Backspace", "Backspace"));
        assert_eq!(s.value, "ac");
        assert_eq!(s.cursor_pos, 1);
    }

    #[test]
    fn backspace_at_start() {
        let mut s = text_state("abc", 0);
        assert!(!form_control_key_input(&mut s, "Backspace", "Backspace"));
        assert_eq!(s.value, "abc");
    }

    #[test]
    fn delete_middle() {
        let mut s = text_state("abc", 1);
        assert!(form_control_key_input(&mut s, "Delete", "Delete"));
        assert_eq!(s.value, "ac");
        assert_eq!(s.cursor_pos, 1);
    }

    #[test]
    fn delete_at_end() {
        let mut s = text_state("abc", 3);
        assert!(!form_control_key_input(&mut s, "Delete", "Delete"));
    }

    #[test]
    fn arrow_left_right() {
        let mut s = text_state("abc", 2);
        assert!(form_control_key_input(&mut s, "ArrowLeft", "ArrowLeft"));
        assert_eq!(s.cursor_pos, 1);
        assert!(form_control_key_input(&mut s, "ArrowRight", "ArrowRight"));
        assert_eq!(s.cursor_pos, 2);
    }

    #[test]
    fn home_end() {
        let mut s = text_state("abc", 1);
        assert!(form_control_key_input(&mut s, "Home", "Home"));
        assert_eq!(s.cursor_pos, 0);
        assert!(form_control_key_input(&mut s, "End", "End"));
        assert_eq!(s.cursor_pos, 3);
    }

    #[test]
    fn enter_in_textarea() {
        let mut s = textarea_state("ab", 1);
        assert!(form_control_key_input(&mut s, "Enter", "Enter"));
        assert_eq!(s.value, "a\nb");
        assert_eq!(s.cursor_pos, 2);
    }

    #[test]
    fn enter_in_text_input_returns_submit() {
        let mut s = text_state("ab", 1);
        // Enter on text input triggers implicit form submission.
        assert_eq!(
            form_control_key_input_action(&mut s, "Enter", "Enter"),
            KeyAction::Submit
        );
        // form_control_key_input returns true (Submit != None).
        let mut s2 = text_state("ab", 1);
        assert!(form_control_key_input(&mut s2, "Enter", "Enter"));
    }

    #[test]
    fn multibyte_character() {
        let mut s = text_state("", 0);
        assert!(form_control_key_input(&mut s, "あ", ""));
        assert_eq!(s.value, "あ");
        assert_eq!(s.cursor_pos, 3); // UTF-8 3 bytes
    }

    #[test]
    fn backspace_multibyte() {
        let mut s = text_state("あい", 3);
        assert!(form_control_key_input(&mut s, "Backspace", "Backspace"));
        assert_eq!(s.value, "い");
        assert_eq!(s.cursor_pos, 0);
    }

    #[test]
    fn cursor_pos_clamped_to_value_len() {
        // cursor_pos beyond value length should be clamped, not panic.
        let mut s = text_state("abc", 100);
        assert!(form_control_key_input(&mut s, "x", "KeyX"));
        assert_eq!(s.value, "abcx");
        assert_eq!(s.cursor_pos, 4);
    }

    #[test]
    fn cursor_pos_clamped_to_char_boundary() {
        // cursor_pos in the middle of a multibyte char should be corrected.
        let mut s = text_state("あい", 1); // byte 1 is not a char boundary
        assert!(form_control_key_input(&mut s, "x", "KeyX"));
        // Should have been clamped to byte 0 (prev char boundary)
        assert_eq!(s.value, "xあい");
        assert_eq!(s.cursor_pos, 1);
    }

    #[test]
    fn readonly_rejects_editing() {
        let mut s = FormControlState {
            value: "abc".to_string(),
            cursor_pos: 1,
            readonly: true,
            ..FormControlState::default()
        };
        // Typing should be rejected.
        assert!(!form_control_key_input(&mut s, "x", "KeyX"));
        assert_eq!(s.value, "abc");
        // Backspace/Delete should be rejected.
        assert!(!form_control_key_input(&mut s, "Backspace", "Backspace"));
        assert_eq!(s.value, "abc");
        assert!(!form_control_key_input(&mut s, "Delete", "Delete"));
        assert_eq!(s.value, "abc");
        // Navigation should still work.
        assert!(form_control_key_input(&mut s, "ArrowRight", "ArrowRight"));
        assert_eq!(s.cursor_pos, 2);
        assert!(form_control_key_input(&mut s, "Home", "Home"));
        assert_eq!(s.cursor_pos, 0);
        assert!(form_control_key_input(&mut s, "End", "End"));
        assert_eq!(s.cursor_pos, 3);
    }

    #[test]
    fn checkbox_ignores_keys() {
        let mut s = FormControlState {
            kind: FormControlKind::Checkbox,
            ..FormControlState::default()
        };
        assert!(!form_control_key_input(&mut s, "a", "KeyA"));
    }

    #[test]
    fn newline_rejected_in_text_input() {
        // HTML spec: single-line inputs reject \n and \r.
        let mut s = text_state("ab", 2);
        // \n is a control character that should be rejected anyway,
        // but we explicitly guard against it.
        assert!(!form_control_key_input(&mut s, "\n", "Enter"));
        assert_eq!(s.value, "ab");
    }

    #[test]
    fn maxlength_blocks_insertion() {
        let mut s = FormControlState {
            kind: FormControlKind::TextInput,
            value: "abcd".to_string(),
            cursor_pos: 4,
            char_count: 4,
            maxlength: Some(4),
            ..FormControlState::default()
        };
        assert!(!form_control_key_input(&mut s, "x", "KeyX"));
        assert_eq!(s.value, "abcd");
    }

    #[test]
    fn number_rejects_letters() {
        let mut s = FormControlState {
            kind: FormControlKind::Number,
            value: "12".to_string(),
            cursor_pos: 2,
            ..FormControlState::default()
        };
        assert!(!form_control_key_input(&mut s, "a", "KeyA"));
        assert_eq!(s.value, "12");
        // Digits should be accepted.
        assert!(form_control_key_input(&mut s, "3", "Digit3"));
        assert_eq!(s.value, "123");
        // Dot/minus/e should be accepted.
        assert!(form_control_key_input(&mut s, ".", "Period"));
        assert_eq!(s.value, "123.");
    }

    #[test]
    fn supports_selection_types() {
        assert!(FormControlKind::TextInput.supports_selection());
        assert!(FormControlKind::Password.supports_selection());
        assert!(FormControlKind::TextArea.supports_selection());
        assert!(!FormControlKind::Checkbox.supports_selection());
        assert!(!FormControlKind::Select.supports_selection());
    }

    // -- apply_step tests (D-2 hoist target) -----------------------

    fn make_state(kind: FormControlKind, value: &str, step: Option<&str>) -> FormControlState {
        let mut s = FormControlState {
            kind,
            ..Default::default()
        };
        s.set_value(value.to_string());
        s.step = step.map(String::from);
        s
    }

    #[test]
    fn apply_step_number_default_step_one() {
        let mut s = make_state(FormControlKind::Number, "5", None);
        assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
        assert_eq!(s.value(), "6");
    }

    #[test]
    fn apply_step_range_descending() {
        let mut s = make_state(FormControlKind::Range, "10", Some("2"));
        assert!(apply_step(&mut s, 3.0, -1.0).is_ok());
        assert_eq!(s.value(), "4");
    }

    #[test]
    fn apply_step_unsupported_kind_returns_not_supported() {
        let mut s = make_state(FormControlKind::TextInput, "abc", None);
        assert_eq!(apply_step(&mut s, 1.0, 1.0), Err(StepError::NotSupported));
        // Value untouched.
        assert_eq!(s.value(), "abc");
    }

    #[test]
    fn apply_step_invalid_step_falls_back_to_one() {
        let mut s = make_state(FormControlKind::Number, "0", Some("not-a-number"));
        assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
        assert_eq!(s.value(), "1");
    }

    #[test]
    fn apply_step_empty_value_treated_as_zero() {
        let mut s = make_state(FormControlKind::Number, "", Some("2"));
        assert!(apply_step(&mut s, 5.0, 1.0).is_ok());
        assert_eq!(s.value(), "10");
    }

    #[test]
    fn apply_step_fractional_step() {
        let mut s = make_state(FormControlKind::Number, "1", Some("0.5"));
        assert!(apply_step(&mut s, 1.0, 1.0).is_ok());
        // f64 1.5 prints as "1.5" via to_string.
        assert_eq!(s.value(), "1.5");
    }

    // -------------------------------------------------------------------
    // sanitize_for_type_change (HTML §4.10.5.6)
    // -------------------------------------------------------------------

    #[test]
    fn sanitize_clears_checked_when_leaving_checkbox() {
        let mut s = FormControlState {
            kind: FormControlKind::TextInput,
            checked: true,
            ..FormControlState::default()
        };
        sanitize_for_type_change(&mut s, FormControlKind::Checkbox);
        assert!(!s.checked);
    }

    #[test]
    fn sanitize_clears_indeterminate_when_leaving_checkbox() {
        let mut s = FormControlState {
            kind: FormControlKind::TextInput,
            indeterminate: true,
            ..FormControlState::default()
        };
        sanitize_for_type_change(&mut s, FormControlKind::Checkbox);
        assert!(!s.indeterminate);
    }

    #[test]
    fn sanitize_clears_checked_when_leaving_radio() {
        let mut s = FormControlState {
            kind: FormControlKind::TextInput,
            checked: true,
            ..FormControlState::default()
        };
        sanitize_for_type_change(&mut s, FormControlKind::Radio);
        assert!(!s.checked);
    }

    #[test]
    fn sanitize_keeps_checked_when_staying_checkable() {
        let mut s = FormControlState {
            kind: FormControlKind::Radio,
            checked: true,
            ..FormControlState::default()
        };
        sanitize_for_type_change(&mut s, FormControlKind::Checkbox);
        assert!(s.checked);
    }

    #[test]
    fn sanitize_clears_value_when_entering_number_with_non_numeric() {
        let mut s = FormControlState {
            kind: FormControlKind::Number,
            ..FormControlState::default()
        };
        s.set_value("abc".to_string());
        sanitize_for_type_change(&mut s, FormControlKind::TextInput);
        assert_eq!(s.value(), "");
    }

    #[test]
    fn sanitize_keeps_value_when_entering_number_with_numeric() {
        let mut s = FormControlState {
            kind: FormControlKind::Number,
            ..FormControlState::default()
        };
        s.set_value("3.14".to_string());
        sanitize_for_type_change(&mut s, FormControlKind::TextInput);
        assert_eq!(s.value(), "3.14");
    }

    #[test]
    fn sanitize_no_op_when_kind_unchanged() {
        let mut s = FormControlState {
            kind: FormControlKind::Number,
            ..FormControlState::default()
        };
        s.set_value("not-a-number".to_string());
        sanitize_for_type_change(&mut s, FormControlKind::Number);
        // Same-kind transition: no sanitize runs (caller already had
        // this value, and same-kind means content didn't change).
        assert_eq!(s.value(), "not-a-number");
    }
}
