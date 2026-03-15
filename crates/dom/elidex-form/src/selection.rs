//! Text selection management for form controls.

use crate::util::{next_char_boundary, prev_char_boundary};
use crate::FormControlState;

/// Extend selection in the given direction.
pub fn extend_selection(state: &mut FormControlState, forward: bool) {
    if forward {
        if state.cursor_pos < state.value.len() {
            let next = next_char_boundary(&state.value, state.cursor_pos);
            state.cursor_pos = next;
            state.selection_end = next;
        }
    } else if state.cursor_pos > 0 {
        let prev = prev_char_boundary(&state.value, state.cursor_pos);
        state.cursor_pos = prev;
        state.selection_end = prev;
    }
}

/// Select all text in the control.
pub fn select_all(state: &mut FormControlState) {
    state.selection_start = 0;
    state.selection_end = state.value.len();
    state.cursor_pos = state.value.len();
}

/// Collapse selection to the cursor position.
pub fn collapse_selection(state: &mut FormControlState) {
    state.selection_start = state.cursor_pos;
    state.selection_end = state.cursor_pos;
}

/// Returns `true` if there is an active selection.
#[must_use]
pub fn has_selection(state: &FormControlState) -> bool {
    state.selection_start != state.selection_end
}

/// Get the selected text.
#[must_use]
pub fn selected_text(state: &FormControlState) -> &str {
    let (start, end) = state.safe_selection_range();
    &state.value[start..end]
}

/// Delete the current selection and return the deleted text.
pub fn delete_selection(state: &mut FormControlState) -> String {
    let (start, end) = state.safe_selection_range();
    let deleted: String = state.value.drain(start..end).collect();
    state.cursor_pos = start;
    state.selection_start = 0;
    state.selection_end = 0;
    state.update_char_count();
    deleted
}

/// Replace the current selection with the given text.
pub fn replace_selection(state: &mut FormControlState, text: &str) {
    delete_selection(state);
    state.value.insert_str(state.cursor_pos, text);
    state.cursor_pos += text.len();
    state.update_char_count();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_state(value: &str) -> FormControlState {
        FormControlState {
            value: value.to_string(),
            char_count: value.chars().count(),
            ..FormControlState::default()
        }
    }

    #[test]
    fn select_all_text() {
        let mut s = text_state("hello");
        select_all(&mut s);
        assert_eq!(s.selection_start, 0);
        assert_eq!(s.selection_end, 5);
        assert!(has_selection(&s));
    }

    #[test]
    fn selected_text_returns_range() {
        let mut s = text_state("hello");
        s.selection_start = 1;
        s.selection_end = 4;
        assert_eq!(selected_text(&s), "ell");
    }

    #[test]
    fn delete_selection_removes_text() {
        let mut s = text_state("hello");
        s.selection_start = 1;
        s.selection_end = 4;
        let deleted = delete_selection(&mut s);
        assert_eq!(deleted, "ell");
        assert_eq!(s.value, "ho");
        assert_eq!(s.cursor_pos, 1);
    }

    #[test]
    fn replace_selection_inserts_text() {
        let mut s = text_state("hello");
        s.selection_start = 1;
        s.selection_end = 4;
        replace_selection(&mut s, "XY");
        assert_eq!(s.value, "hXYo");
        assert_eq!(s.cursor_pos, 3);
    }

    #[test]
    fn extend_selection_forward() {
        let mut s = text_state("abc");
        s.cursor_pos = 1;
        s.selection_start = 1;
        s.selection_end = 1;
        extend_selection(&mut s, true);
        assert_eq!(s.selection_end, 2);
        assert_eq!(s.cursor_pos, 2);
    }

    #[test]
    fn extend_selection_backward() {
        let mut s = text_state("abc");
        s.cursor_pos = 2;
        s.selection_start = 2;
        s.selection_end = 2;
        extend_selection(&mut s, false);
        assert_eq!(s.selection_end, 1);
        assert_eq!(s.cursor_pos, 1);
    }

    #[test]
    fn collapse_resets_selection() {
        let mut s = text_state("abc");
        s.selection_start = 0;
        s.selection_end = 3;
        s.cursor_pos = 1;
        collapse_selection(&mut s);
        assert!(!has_selection(&s));
    }

    #[test]
    fn no_selection_initially() {
        let s = text_state("abc");
        assert!(!has_selection(&s));
        assert_eq!(selected_text(&s), "");
    }

    #[test]
    fn delete_empty_selection() {
        let mut s = text_state("abc");
        let deleted = delete_selection(&mut s);
        assert!(deleted.is_empty());
        assert_eq!(s.value, "abc");
    }

    #[test]
    fn reversed_selection() {
        let mut s = text_state("hello");
        s.selection_start = 4;
        s.selection_end = 1;
        assert_eq!(selected_text(&s), "ell");
    }

    #[test]
    fn delete_selection_updates_char_count() {
        let mut s = text_state("hello");
        s.selection_start = 1;
        s.selection_end = 4;
        delete_selection(&mut s);
        assert_eq!(s.char_count, 2); // "ho"
    }

    #[test]
    fn replace_selection_updates_char_count() {
        let mut s = text_state("hello");
        s.selection_start = 1;
        s.selection_end = 4;
        replace_selection(&mut s, "XY");
        assert_eq!(s.char_count, 4); // "hXYo"
    }

    #[test]
    fn extend_at_boundary() {
        let mut s = text_state("ab");
        s.cursor_pos = 2;
        s.selection_start = 2;
        s.selection_end = 2;
        extend_selection(&mut s, true);
        // At end, should not move.
        assert_eq!(s.cursor_pos, 2);
    }
}
