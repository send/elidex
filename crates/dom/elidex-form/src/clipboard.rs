//! Clipboard operations for form controls.

use crate::selection;
use crate::FormControlState;

/// Copy selected text (returns the text to be placed on clipboard).
///
/// Returns empty string for password fields (security: prevent clipboard leak).
#[must_use]
pub fn clipboard_copy(state: &FormControlState) -> String {
    if state.kind == crate::FormControlKind::Password {
        return String::new();
    }
    selection::selected_text(state).to_string()
}

/// Cut selected text (removes from value, returns the text).
///
/// Returns empty string for password fields (security: prevent clipboard leak).
pub fn clipboard_cut(state: &mut FormControlState) -> String {
    if !selection::has_selection(state) || state.readonly {
        return String::new();
    }
    // Security: prevent clipboard leak from password fields.
    if state.kind == crate::FormControlKind::Password {
        return String::new();
    }
    selection::delete_selection(state)
}

/// Maximum paste size in characters when no `maxlength` is set (1MB of chars).
const MAX_PASTE_CHARS: usize = 1_000_000;

/// Paste text at cursor position (replaces selection if any).
///
/// Truncates pasted text to fit within `maxlength` constraint if set.
/// When no `maxlength` is set, applies a safety limit of 1M characters.
pub fn clipboard_paste(state: &mut FormControlState, text: &str) {
    if state.readonly {
        return;
    }
    // Ensure cursor_pos is valid before insertion.
    state.cursor_pos = state.safe_cursor_pos();

    // Compute available chars: maxlength or safety limit.
    let selection_chars = if selection::has_selection(state) {
        let (s, e) = state.safe_selection_range();
        state.value[s..e].chars().count()
    } else {
        0
    };
    let limit = state.maxlength.unwrap_or(MAX_PASTE_CHARS);
    let available = limit.saturating_sub(state.char_count.saturating_sub(selection_chars));
    if available == 0 {
        return;
    }
    // Allocation bounded by min(maxlength, MAX_PASTE_CHARS) via `available`.
    let truncated: String = text.chars().take(available).collect();
    if selection::has_selection(state) {
        selection::replace_selection(state, &truncated);
    } else {
        state.value.insert_str(state.cursor_pos, &truncated);
        state.cursor_pos += truncated.len();
    }
    state.dirty_value = true;
    state.update_char_count();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FormControlKind;

    fn text_state(value: &str) -> FormControlState {
        FormControlState {
            value: value.to_string(),
            cursor_pos: value.len(),
            char_count: value.chars().count(),
            ..FormControlState::default()
        }
    }

    #[test]
    fn copy_with_selection() {
        let mut s = text_state("hello");
        s.selection_start = 1;
        s.selection_end = 4;
        assert_eq!(clipboard_copy(&s), "ell");
    }

    #[test]
    fn copy_no_selection() {
        let s = text_state("hello");
        assert_eq!(clipboard_copy(&s), "");
    }

    #[test]
    fn cut_with_selection() {
        let mut s = text_state("hello");
        s.selection_start = 1;
        s.selection_end = 4;
        let cut = clipboard_cut(&mut s);
        assert_eq!(cut, "ell");
        assert_eq!(s.value, "ho");
    }

    #[test]
    fn cut_readonly() {
        let mut s = text_state("hello");
        s.readonly = true;
        s.selection_start = 0;
        s.selection_end = 5;
        let cut = clipboard_cut(&mut s);
        assert!(cut.is_empty());
        assert_eq!(s.value, "hello");
    }

    #[test]
    fn paste_at_cursor() {
        let mut s = text_state("ac");
        s.cursor_pos = 1;
        clipboard_paste(&mut s, "b");
        assert_eq!(s.value, "abc");
        assert!(s.dirty_value);
    }

    #[test]
    fn paste_replaces_selection() {
        let mut s = text_state("hello");
        s.selection_start = 1;
        s.selection_end = 4;
        clipboard_paste(&mut s, "XY");
        assert_eq!(s.value, "hXYo");
    }

    #[test]
    fn paste_readonly() {
        let mut s = text_state("hello");
        s.readonly = true;
        clipboard_paste(&mut s, "world");
        assert_eq!(s.value, "hello");
    }

    #[test]
    fn cut_no_selection() {
        let mut s = text_state("hello");
        let cut = clipboard_cut(&mut s);
        assert!(cut.is_empty());
    }

    #[test]
    fn password_copy_returns_empty() {
        let mut s = text_state("secret");
        s.kind = FormControlKind::Password;
        s.selection_start = 0;
        s.selection_end = 6;
        assert_eq!(clipboard_copy(&s), "");
    }

    #[test]
    fn password_cut_returns_empty() {
        let mut s = text_state("secret");
        s.kind = FormControlKind::Password;
        s.selection_start = 0;
        s.selection_end = 6;
        let cut = clipboard_cut(&mut s);
        assert!(cut.is_empty());
        // Value should not be modified.
        assert_eq!(s.value, "secret");
    }

    #[test]
    fn paste_enforces_maxlength() {
        let mut s = text_state("ab");
        s.maxlength = Some(4);
        s.cursor_pos = 2;
        clipboard_paste(&mut s, "xyz");
        // Only "xy" should be pasted (4 - 2 = 2 available).
        assert_eq!(s.value, "abxy");
        assert!(s.dirty_value);
    }

    #[test]
    fn paste_maxlength_at_limit() {
        let mut s = text_state("abcd");
        s.maxlength = Some(4);
        s.cursor_pos = 4;
        clipboard_paste(&mut s, "x");
        // No room, nothing pasted.
        assert_eq!(s.value, "abcd");
    }
}
