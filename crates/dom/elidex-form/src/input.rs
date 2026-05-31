//! Keyboard input handling for text-based form controls.

use elidex_ecs::{Attributes, EcsDom, Entity, TagType};

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

/// Resolve `<input>.list` to its associated `<datalist>` per WHATWG HTML
/// §4.10.5.1.16: "the first element in the tree of type `HTMLDataListElement`
/// whose ID is equal to the value of the `list` attribute, if that element is
/// in the same tree as the input element".
///
/// Returns `None` for input types the `list` attribute does not apply to
/// (hidden / checkbox / radio / file / submit / image / reset / button /
/// password — see `input_list_applies_to_type` for the spec exclusion set).
///
/// Tree scope honors shadow boundaries — nested shadow subtrees within the
/// same root are correctly excluded per the spec's "same tree" wording.
/// Cross-tree (shadow-piercing) resolution is tracked at the
/// `#11-form-elements-cross-tree` defer slot.
#[must_use]
pub fn resolve_input_list(dom: &EcsDom, input_entity: Entity) -> Option<Entity> {
    if !input_list_applies_to_type(dom, input_entity) {
        return None;
    }
    let list_id: String = {
        let attrs = dom.world().get::<&Attributes>(input_entity).ok()?;
        let v = attrs.get("list")?;
        if v.is_empty() {
            return None;
        }
        v.to_owned()
    };

    // `traverse_descendants` skips `root` itself; check explicitly.
    let root = dom.find_tree_root(input_entity);
    if matches_datalist_with_id(dom, root, list_id.as_str()) {
        return Some(root);
    }
    let mut candidate = None;
    dom.traverse_descendants(root, |entity| {
        if matches_datalist_with_id(dom, entity, list_id.as_str()) {
            candidate = Some(entity);
            return false;
        }
        true
    });
    candidate
}

fn matches_datalist_with_id(dom: &EcsDom, entity: Entity, id: &str) -> bool {
    // Tag name is the cheapest discriminator (this runs on every descendant
    // of the tree walk), so check it first and reject non-`<datalist>` nodes
    // before the namespace lookup.
    let Ok(tag) = dom.world().get::<&TagType>(entity) else {
        return false;
    };
    if !tag.0.as_str().eq_ignore_ascii_case("datalist") {
        return false;
    }
    drop(tag);
    // The `list` attribute must reference an element of type
    // `HTMLDataListElement` (HTML §4.10.8), so a foreign-namespace
    // `<datalist>` look-alike (SVG / MathML) does not match.
    if !dom.is_html_namespace(entity) {
        return false;
    }
    dom.world()
        .get::<&Attributes>(entity)
        .is_ok_and(|a| a.get("id") == Some(id))
}

/// `<input>.list` applicability per HTML §4.10.5.1.16.
///
/// Reads the `type` content attribute directly (spec source of truth):
/// `setAttribute("type", X)` mutates `Attributes` synchronously while
/// any cached `FormControlState.kind` only re-syncs on a type-change
/// sanitize pass — preferring the cached kind would let stale state
/// mask a fresh `setAttribute("type", "hidden")` mutation.
///
/// Missing attribute defaults to `"text"` per HTML §4.10.5.1 missing-
/// value-default rule.
///
/// Exclusion set is matched against the spec text directly (rather than
/// routed through [`FormControlKind`]) because `from_type_str` collapses
/// `"image"` (and the unmodeled `"month"` / `"week"` / `"time"`) onto
/// `TextInput` — that fallback is harmless for the applicable types but
/// would incorrectly admit `<input type="image">` if the predicate was
/// gated on `FormControlKind::list_applies`.
fn input_list_applies_to_type(dom: &EcsDom, input_entity: Entity) -> bool {
    let Ok(attrs) = dom.world().get::<&Attributes>(input_entity) else {
        return true;
    };
    let type_str = attrs.get("type").unwrap_or("text");
    !matches!(
        type_str.to_ascii_lowercase().as_str(),
        "hidden"
            | "checkbox"
            | "radio"
            | "file"
            | "submit"
            | "image"
            | "reset"
            | "button"
            | "password"
    )
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
