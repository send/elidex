//! Intrinsic sizing for form controls.

use crate::{FormControlKind, FormControlState};

/// Default intrinsic width for text inputs (CSS pixels).
const TEXT_INPUT_WIDTH: f32 = 173.0;
/// Default intrinsic height for text inputs (CSS pixels).
const TEXT_INPUT_HEIGHT: f32 = 21.0;

/// Default intrinsic width for checkboxes (CSS pixels).
const CHECKBOX_SIZE: f32 = 13.0;

/// Default intrinsic width for submit/button inputs (CSS pixels).
const BUTTON_MIN_WIDTH: f32 = 54.0;
/// Default intrinsic height for buttons (CSS pixels).
const BUTTON_HEIGHT: f32 = 21.0;

/// Line height used for textarea row calculation (CSS pixels).
const TEXTAREA_LINE_HEIGHT: f32 = 18.0;
/// Average character width used for textarea column calculation (CSS pixels).
/// Approximation of 8px average char width at 13px font-size (monospace ~0.6em).
const TEXTAREA_CHAR_WIDTH: f32 = 8.0;

/// Default intrinsic width for select elements (CSS pixels).
const SELECT_WIDTH: f32 = 173.0;
/// Default intrinsic height for select elements (CSS pixels).
const SELECT_HEIGHT: f32 = 21.0;

/// Approximate character width for select option text measurement.
const SELECT_CHAR_WIDTH: f32 = 7.0;
/// Padding for select dropdown arrow.
const SELECT_ARROW_WIDTH: f32 = 20.0;

/// Returns the intrinsic (width, height) for a form control.
///
/// Used by the layout engine as a replaced-element fallback when no
/// explicit CSS width/height is set.
#[must_use]
pub fn form_intrinsic_size(state: &FormControlState) -> (f32, f32) {
    match state.kind {
        FormControlKind::Checkbox | FormControlKind::Radio => (CHECKBOX_SIZE, CHECKBOX_SIZE),
        FormControlKind::SubmitButton | FormControlKind::ResetButton | FormControlKind::Button => {
            (BUTTON_MIN_WIDTH, BUTTON_HEIGHT)
        }
        FormControlKind::TextArea => {
            let rows = state.rows.max(1);
            let cols = state.cols.max(1);
            #[allow(clippy::cast_precision_loss)]
            let w = (cols as f32) * TEXTAREA_CHAR_WIDTH;
            #[allow(clippy::cast_precision_loss)]
            let h = (rows as f32) * TEXTAREA_LINE_HEIGHT;
            (w, h)
        }
        FormControlKind::Select => {
            // Size based on longest option text.
            if state.options.is_empty() {
                return (SELECT_WIDTH, SELECT_HEIGHT);
            }
            let max_len = state.options.iter().map(|o| o.text.chars().count()).max().unwrap_or(0);
            #[allow(clippy::cast_precision_loss)]
            let w = ((max_len as f32) * SELECT_CHAR_WIDTH + SELECT_ARROW_WIDTH)
                .max(SELECT_WIDTH);
            (w, SELECT_HEIGHT)
        }
        // Hidden inputs have no visual footprint.
        FormControlKind::Hidden => (0.0, 0.0),
        // Text-like inputs, specialised types, and output elements use text input size.
        FormControlKind::TextInput
        | FormControlKind::Password
        | FormControlKind::Email
        | FormControlKind::Url
        | FormControlKind::Tel
        | FormControlKind::Search
        | FormControlKind::Number
        | FormControlKind::Range
        | FormControlKind::Color
        | FormControlKind::Date
        | FormControlKind::DatetimeLocal
        | FormControlKind::File
        | FormControlKind::Output
        | FormControlKind::Meter
        | FormControlKind::Progress => (TEXT_INPUT_WIDTH, TEXT_INPUT_HEIGHT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(kind: FormControlKind, rows: u32, cols: u32) -> FormControlState {
        FormControlState {
            kind,
            rows,
            cols,
            ..FormControlState::default()
        }
    }

    #[test]
    fn text_input_size() {
        let state = make_state(FormControlKind::TextInput, 0, 0);
        let (w, h) = form_intrinsic_size(&state);
        assert!(w > 0.0);
        assert!(h > 0.0);
    }

    #[test]
    fn textarea_size_from_rows_cols() {
        let state = make_state(FormControlKind::TextArea, 5, 40);
        let (w, h) = form_intrinsic_size(&state);
        assert_eq!(w, 40.0 * 8.0);
        assert_eq!(h, 5.0 * 18.0);
    }

    #[test]
    fn textarea_default_rows_cols() {
        let state = make_state(FormControlKind::TextArea, 2, 20);
        let (w, h) = form_intrinsic_size(&state);
        assert_eq!(w, 20.0 * 8.0);
        assert_eq!(h, 2.0 * 18.0);
    }

    #[test]
    fn checkbox_is_square() {
        let state = make_state(FormControlKind::Checkbox, 0, 0);
        let (w, h) = form_intrinsic_size(&state);
        assert_eq!(w, h);
    }
}
