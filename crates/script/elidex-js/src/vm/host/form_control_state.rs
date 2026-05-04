//! VM-side per-element form-control state — dirty `value` slot,
//! selection range + direction (HTML §4.10.18.5 / §4.10.11.2 /
//! §4.10.5.2).
//!
//! Standalone state map living on
//! [`super::super::VmInner::form_control_entity_states`], introduced
//! in slot #11-tags-T1 Phase 6 to back HTMLTextAreaElement (and
//! shared with HTMLInputElement / HTMLSelectElement in subsequent
//! phases).
//!
//! ## Why not `elidex_form::FormControlState` directly
//!
//! [`elidex_form::FormControlState`](https://docs.rs/elidex-form)
//! already models all of this on the ECS world (added by the shell's
//! `init_form_controls`).  We **do not** depend on `elidex-form` from
//! `elidex-js` at this point — the dep landing is reserved for Phase
//! 9 alongside the `custom_validity` field extension and the `reset`
//! / `invalid` event-dispatch helpers.  Adding it earlier would
//! ripple through Cargo features without a corresponding payoff
//! since Phase 6/7/8 only need the `value` / `selection_*` slots
//! that this lightweight stand-in already carries.
//!
//! After Phase 9's dep landing, this module retires: every accessor
//! switches to `dom.world().get_mut::<FormControlState>(entity)` and
//! `form_control_entity_states` is dropped.  The migration is a
//! 1-for-1 field rename — no semantic changes — because this module
//! deliberately mirrors the elidex-form field shapes (`value` /
//! `selection_start` / `selection_end` / `selection_direction`).

#![cfg(feature = "engine")]

use elidex_ecs::Entity;

use super::super::VmInner;

/// Selection direction enum mirroring
/// `elidex_form::SelectionDirection`.
///
/// String round-trip (HTML §4.10.18.7):
/// - `"forward"` ↔ [`Self::Forward`]
/// - `"backward"` ↔ [`Self::Backward`]
/// - `"none"` ↔ [`Self::None`] (default)
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SelectionDirection {
    #[default]
    None,
    Forward,
    Backward,
}

impl SelectionDirection {
    /// Parse the IDL-side string per HTML §4.10.18.7.  Unknown
    /// values map to [`Self::None`] (matches the spec
    /// "(?selection-direction enum unknown ⇒ none)" gloss).
    pub(crate) fn parse(s: &str) -> Self {
        match s {
            "forward" => Self::Forward,
            "backward" => Self::Backward,
            _ => Self::None,
        }
    }
}

/// Per-element dirty-value + selection slots.
///
/// `dirty_value == Some(_)` means the IDL `.value` setter was
/// invoked at least once; subsequent `.value` reads return the
/// stored string regardless of intervening textContent edits.
/// `dirty_value == None` falls back to the element's current
/// defaultValue (textContent for `<textarea>`; the `value` content
/// attribute for `<input>` in Phase 8).
#[derive(Default)]
pub(crate) struct FormControlEntityState {
    /// Dirty value slot — `Some(s)` when the IDL setter has fired
    /// at least once.  Cleared when the form's `reset()` algorithm
    /// runs (Phase 9 / slot `#11c-followup-reset-form`).
    pub(crate) dirty_value: Option<String>,
    /// Selection start in UTF-16 code units (matches HTML
    /// §4.10.18.7's "API value" length convention).
    pub(crate) selection_start: u32,
    /// Selection end in UTF-16 code units (`>= selection_start`
    /// after every spec-compliant write through `set_selection`).
    pub(crate) selection_end: u32,
    /// Selection direction.
    pub(crate) selection_direction: SelectionDirection,
}

impl VmInner {
    /// Read-side helper — returns the dirty value when set,
    /// otherwise `None` (caller falls back to the element's
    /// defaultValue path).  Borrows the state map immutably.
    pub(crate) fn form_control_dirty_value(&self, entity: Entity) -> Option<&str> {
        self.form_control_entity_states
            .get(&entity)
            .and_then(|s| s.dirty_value.as_deref())
    }

    /// Mutable accessor that initialises a default state on first
    /// touch.  Used by every IDL setter for the `value` /
    /// `selection*` slots.
    pub(crate) fn form_control_state_mut(&mut self, entity: Entity) -> &mut FormControlEntityState {
        self.form_control_entity_states.entry(entity).or_default()
    }

    /// Read-side helper — returns the full state if any IDL setter
    /// has touched this entity, otherwise `None` (caller treats
    /// missing as the all-defaults state).  Borrows the state map
    /// immutably.
    pub(crate) fn form_control_state(&self, entity: Entity) -> Option<&FormControlEntityState> {
        self.form_control_entity_states.get(&entity)
    }
}

/// Read the IDL value of a form control: returns the dirty value
/// when the IDL setter has fired at least once, otherwise calls
/// `default_value` to materialise the per-element defaultValue.
/// Centralises the `dirty ?? default` pattern so every form-control
/// prototype's `value` getter shares a single dirty-check
/// implementation — when the elidex-form dep landing migrates the
/// dirty slot from this map into the ECS-resident
/// `FormControlState`, this helper is the only place that needs to
/// change.
///
/// Borrow shape: the dirty fast-path returns before invoking
/// `default_value`, so the closure runs with `&mut NativeContext`
/// uncontested.
pub(super) fn read_value<F>(
    ctx: &mut super::super::value::NativeContext<'_>,
    entity: Entity,
    default_value: F,
) -> String
where
    F: FnOnce(&mut super::super::value::NativeContext<'_>, Entity) -> String,
{
    if let Some(dirty) = ctx.vm.form_control_dirty_value(entity) {
        return dirty.to_string();
    }
    default_value(ctx, entity)
}

/// Length-only counterpart of [`read_value`] — returns the IDL
/// value's UTF-16 length without materialising a String when the
/// dirty slot is unset.  `default_value_utf16_len` is the per-element
/// fallback that counts the defaultValue's UTF-16 units (e.g.
/// `descendant_text_utf16_len` for textarea, `value` content
/// attribute length for input).
pub(super) fn value_utf16_len<F>(
    ctx: &mut super::super::value::NativeContext<'_>,
    entity: Entity,
    default_value_utf16_len: F,
) -> u32
where
    F: FnOnce(&mut super::super::value::NativeContext<'_>, Entity) -> u32,
{
    if let Some(dirty) = ctx.vm.form_control_dirty_value(entity) {
        return utf16_len(dirty);
    }
    default_value_utf16_len(ctx, entity)
}

/// Concatenate the `TextContent` data of every descendant of
/// `root` in tree order.  Mirrors HTML "child text content"
/// semantics used by HTMLTextAreaElement.defaultValue (§4.10.11.5)
/// and HTMLOptionElement.text default (§4.10.10.4 step 1).  Pulled
/// here so the two callers (`html_textarea_proto::read_default_value`
/// + `html_select_proto::option_text_content`) share one walk.
pub(super) fn descendant_text(dom: &elidex_ecs::EcsDom, root: Entity) -> String {
    let mut buf = String::new();
    dom.traverse_descendants(root, |e| {
        if let Ok(text) = dom.world().get::<&elidex_ecs::TextContent>(e) {
            buf.push_str(&text.0);
        }
        true
    });
    buf
}

/// Same descendant walk as [`descendant_text`] but counts UTF-16
/// code units without materialising an owned String.  Used by the
/// Selection-API setters (which only need the length to clamp
/// bounds, not the bytes).
pub(super) fn descendant_text_utf16_len(dom: &elidex_ecs::EcsDom, root: Entity) -> u32 {
    let mut n: u32 = 0;
    dom.traverse_descendants(root, |e| {
        if let Ok(text) = dom.world().get::<&elidex_ecs::TextContent>(e) {
            n = n.saturating_add(utf16_len(&text.0));
        }
        true
    });
    n
}

/// Count UTF-16 code units in `s` — selection range bounds use
/// "API value length" per HTML §4.10.18.7, defined in terms of
/// UTF-16 code units regardless of the engine's internal string
/// encoding.  Used wherever an IDL setter clamps its argument to
/// the length of `value`.
pub(crate) fn utf16_len(s: &str) -> u32 {
    let n: usize = s.encode_utf16().count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Convert a UTF-16 code-unit offset to a UTF-8 byte offset within
/// `s`.  Clamps `offset` to the maximum valid byte index when the
/// caller passes a value past the end (matches the spec's
/// "saturate at length" treatment for `setRangeText` / `select()`).
pub(crate) fn utf16_offset_to_utf8(s: &str, offset: u32) -> usize {
    let mut units_remaining = offset as usize;
    for (byte_idx, ch) in s.char_indices() {
        if units_remaining == 0 {
            return byte_idx;
        }
        let units = ch.len_utf16();
        if units_remaining < units {
            // Splitting mid-surrogate — saturate to the boundary
            // before this character.  Spec edge that browsers
            // resolve consistently with truncation rather than
            // panic.
            return byte_idx;
        }
        units_remaining -= units;
    }
    s.len()
}
