//! Change-on-blur provenance snapshot (WHATWG HTML §6.6.4 focus update steps
//! step 2.1; §4.10.5.5 Common event behaviors).
//!
//! Step 2.1 fires `change` at a blurred `<input>` only if "the change event
//! applies to the element, and the element does not have a defined activation
//! behavior", and "the user has changed the element's value … while the
//! control was focused without committing that change (such that it is
//! different to what it was when the control was first focused)". The spec is
//! silent for `<textarea>`; it follows the same rule (re-derivation
//! `docs/plans/2026-11-change-on-blur-spec-rederivation.md` D-4). §4.10.5.5
//! says `input` / `change` "are not fired in response to changes made to the
//! values of form controls by scripts". The snapshot is stored **on the focused
//! element** as an ECS component so it follows the canonical
//! `ElementState::FOCUS` bit whichever path established focus (the shell UA
//! transition or the VM's `HTMLElement.focus()`).
//!
//! The Population (the elements that carry the component) is chosen once, by
//! the element, at the seed: see [`record_focus_snapshot`]. The kind (the `type`
//! state) is read only at focus loss, by [`take_change_on_blur`] through
//! `change_on_blur_kind_applies`, so no write chooses by kind. The component holds one thing: the baseline the
//! blur compares against, `None` until the first user value change since focus,
//! which stores the value just before it. `Some` is therefore also the
//! provenance mark. Only [`crate::apply_user_edit`] sets it; no non-user writer
//! does.
//!
//! "The plan" below is `docs/plans/2026-11-focus-fixup-change-on-blur-baseline.md`
//! (its B-2, QB-1..3, O-B1 and §4.4); "the re-derivation" is
//! `docs/plans/2026-11-change-on-blur-spec-rederivation.md` (D-4, R-2, PE-2).
//!
//! The blur's verdict is computed in one place, [`take_change_on_blur`]; every
//! focus sink calls it and only dispatches `change`.

use elidex_ecs::{EcsDom, Entity};

use crate::{FormControlKind, FormControlState};

/// The change-on-blur provenance of a focused `input` / `textarea` element.
/// Which elements carry it is [`record_focus_snapshot`]'s rule (B-2 of the plan); auto-cleaned on despawn. The field is private: only this module
/// writes it (seed, mark, take).
///
/// No focus-time copy of the value is kept: an unmarked baseline would never
/// be read, so the first mark stores the pre-edit value instead (D-1 of `docs/plans/2026-11-change-on-blur-user-provenance.md`,
/// PM 393).
pub struct FocusValueSnapshot(Option<String>);

impl FocusValueSnapshot {
    /// The baseline `change` compares the value with at blur: `Some` once a user
    /// edit changed the value since focus (the provenance mark), else `None`.
    #[must_use]
    pub fn baseline(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

/// Seed the change-on-blur snapshot on `entity`: unmarked (no baseline), iff it
/// is an HTML-namespace `input` or `textarea` element carrying a
/// [`FormControlState`]. Any other element gets no write at all (no insert, no
/// remove). Call whenever an element gains focus (the shell sink's seed, after
/// §6.6.4 focus update steps step 4.4, or VM `focus()`); a re-seed replaces any
/// earlier snapshot, mark included.
///
/// The test is the element, never `FormControlState.kind`: the kind cannot
/// express "is an `input` element" (a `<button>` shares kinds with `<input>`)
/// and a `type` change must not change who carries the component. Whether a
/// seeded component yields `change` is decided at focus loss, by the kind then
/// (see [`take_change_on_blur`]).
pub fn record_focus_snapshot(dom: &mut EcsDom, entity: Entity) {
    // Cheapest exclusion first: most focused elements are not form controls.
    if dom.world().get::<&FormControlState>(entity).is_ok()
        && dom.is_html_namespace(entity)
        && (dom.has_tag(entity, "input") || dom.has_tag(entity, "textarea"))
    {
        let _ = dom.world_mut().insert_one(entity, FocusValueSnapshot(None));
    }
}

/// Mark `entity`'s snapshot: a user edit changed the value from `before`.
/// Called only by [`crate::apply_user_edit`]. Without a snapshot it does
/// nothing (never inserts one).
///
/// The **first** mark sets the baseline to `before`. While unmarked, the
/// value has changed only through non-user writes (a user change would have
/// marked), and each of those re-baselines (the Chromium-measured model, plan
/// §4.3), so the baseline at the first user change is the value just before
/// it. Taking it here means no non-user writer has to call anything. Later
/// changes, user or not, leave the baseline.
pub(crate) fn note_user_value_change(dom: &mut EcsDom, entity: Entity, before: String) {
    if let Ok(mut snap) = dom.world_mut().get::<&mut FocusValueSnapshot>(entity) {
        snap.0.get_or_insert(before);
    }
}

/// Whether step 2.1's kind conjuncts hold for an element whose kind at focus
/// loss is `kind`: HTML §6.6.4 *focus update steps* step 2.1, "the change event
/// applies to the element, and the element does not have a defined activation
/// behavior" (under R-2 of the re-derivation,
/// `docs/plans/2026-11-change-on-blur-spec-rederivation.md` §0.5). One
/// exhaustive `match` with no wildcard arm: a new [`FormControlKind`] must be
/// decided here. Private on purpose: its only caller is
/// [`take_change_on_blur`], so `change` is decided in one place (a public
/// classifier would invite a second caller).
///
/// Per group, the state's "The input and change events apply" (QB-2) / "do not
/// apply" (QB-1) line of HTML §4.10.5.1.x, and the activation half (R-2).
// The groups share a result but not a ground: each is the spec's own state line
// (QB-1 / QB-2, R-2, D-4), kept as its own documented arm.
#[allow(clippy::match_same_arms)]
fn change_on_blur_kind_applies(kind: FormControlKind) -> bool {
    match kind {
        // Text/Search (§4.10.5.1.2), Telephone, URL, Email, Password, Date,
        // Month, Week, Time, Local Date and Time, Number, Range: "The input and
        // change events apply" (QB-2), and none defines an input activation
        // behavior (R-2). An `<input type=image>` also lands on this arm:
        // `FormControlKind::from_type_str` maps `image` to `TextInput` (there is
        // no Image Button state), although HTML says of the Image Button state
        // "The input and change events do not apply" — slot
        // `#11-input-type-keyword-state-mapping` (O-B1 of the plan).
        FormControlKind::TextInput
        | FormControlKind::Search
        | FormControlKind::Tel
        | FormControlKind::Url
        | FormControlKind::Email
        | FormControlKind::Password
        | FormControlKind::Date
        | FormControlKind::Month
        | FormControlKind::Week
        | FormControlKind::Time
        | FormControlKind::DatetimeLocal
        | FormControlKind::Number
        | FormControlKind::Range => true,
        // The `<textarea>` element: the spec states no `change` rule for it
        // (re-derivation PE-2); it follows the text states (re-derivation D-4). Also the
        // kind an `<input type=textarea>` maps to (Text state by QB-3): same
        // answer.
        FormControlKind::TextArea => true,
        // Checkbox (§4.10.5.1.15), Radio Button, File Upload, Color: the events
        // apply (QB-2) but the state defines an input activation behavior (R-2),
        // which step 2.1 excludes.
        FormControlKind::Checkbox
        | FormControlKind::Radio
        | FormControlKind::File
        | FormControlKind::Color => false,
        // Hidden (§4.10.5.1.1), Submit Button, Reset Button, Button: "The input
        // and change events do not apply." (QB-1)
        FormControlKind::Hidden
        | FormControlKind::SubmitButton
        | FormControlKind::ResetButton
        | FormControlKind::Button => false,
        // Not `input` states: no step 2.1 applies. An `<input>` reaches `Select`
        // only through `FormControlKind::from_type_str`'s mapping of the
        // `select-one` keyword, which is not that keyword's state — slot
        // `#11-input-type-keyword-state-mapping` (O-B1).
        FormControlKind::Select
        | FormControlKind::Output
        | FormControlKind::Meter
        | FormControlKind::Progress => false,
    }
}

/// §6.6.4 focus update steps step 2.1 — consume (read + remove) `entity`'s
/// snapshot and return whether `change` fires: the kind at focus loss passes
/// step 2.1's kind conjuncts (`change_on_blur_kind_applies`), the user changed
/// the value since focus, **and** it differs from the baseline (§4.10.5.5:
/// script value changes alone fire nothing). `false` without a snapshot (an
/// element the seed did not write) or without a `FormControlState`. The only
/// consumer of the snapshot: every focus sink calls this and only dispatches
/// `change`.
pub fn take_change_on_blur(dom: &mut EcsDom, entity: Entity) -> bool {
    let Ok(FocusValueSnapshot(baseline)) = dom.world_mut().remove_one::<FocusValueSnapshot>(entity)
    else {
        return false;
    };
    dom.world()
        .get::<&FormControlState>(entity)
        .is_ok_and(|fcs| {
            change_on_blur_kind_applies(fcs.kind)
                && matches!(baseline.as_deref(), Some(b) if fcs.value() != b)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{attached_element, control, snap, user_append, Snap};

    fn set_value(dom: &mut EcsDom, e: Entity, value: &str) {
        dom.world_mut()
            .get::<&mut FormControlState>(e)
            .unwrap()
            .set_value(value.to_string());
    }

    /// C7 (a): a seed is unmarked (no baseline).
    #[test]
    fn c7a_seed_is_unmarked() {
        let mut dom = EcsDom::new();
        let input = control(&mut dom, FormControlKind::TextInput, "initial");
        record_focus_snapshot(&mut dom, input);
        assert_eq!(snap(&dom, input), Snap::Unmarked);
    }

    /// C7 (b): a user edit that changes the value marks the snapshot.
    #[test]
    fn c7b_user_value_change_marks() {
        let mut dom = EcsDom::new();
        let input = control(&mut dom, FormControlKind::TextInput, "");
        record_focus_snapshot(&mut dom, input);
        user_append(&mut dom, input, "a");
        assert_eq!(snap(&dom, input), Snap::Baseline(String::new()));
    }

    /// C7 (c): a user edit on a control without a snapshot inserts none.
    #[test]
    fn c7c_user_edit_without_snapshot_inserts_none() {
        let mut dom = EcsDom::new();
        let input = control(&mut dom, FormControlKind::TextInput, "");
        user_append(&mut dom, input, "a");
        assert_eq!(snap(&dom, input), Snap::Absent);
    }

    /// C7 (d): a re-seed after a mark is unmarked again.
    #[test]
    fn c7d_reseed_clears_the_mark() {
        let mut dom = EcsDom::new();
        let input = control(&mut dom, FormControlKind::TextInput, "");
        record_focus_snapshot(&mut dom, input);
        user_append(&mut dom, input, "a");
        record_focus_snapshot(&mut dom, input);
        assert_eq!(snap(&dom, input), Snap::Unmarked);
    }

    /// C7 (i): `take_change_on_blur` without a snapshot → `false`.
    #[test]
    fn c7i_take_without_snapshot_is_false() {
        let mut dom = EcsDom::new();
        let input = control(&mut dom, FormControlKind::TextInput, "x");
        assert!(!take_change_on_blur(&mut dom, input));
    }

    /// C7 (j): marked and value ≠ baseline → `true`, consumed; a second take → `false`.
    #[test]
    fn c7j_take_marked_changed_is_true_once() {
        let mut dom = EcsDom::new();
        let input = control(&mut dom, FormControlKind::TextInput, "");
        record_focus_snapshot(&mut dom, input);
        user_append(&mut dom, input, "a");
        assert!(take_change_on_blur(&mut dom, input));
        assert_eq!(
            snap(&dom, input),
            Snap::Absent,
            "the take removes the snapshot"
        );
        assert!(!take_change_on_blur(&mut dom, input));
    }

    /// C7 (k): unmarked with the value changed, and marked with value =
    /// baseline → `false`.
    #[test]
    fn c7k_take_needs_mark_and_difference() {
        let mut dom = EcsDom::new();
        let script_only = control(&mut dom, FormControlKind::TextInput, "");
        record_focus_snapshot(&mut dom, script_only);
        set_value(&mut dom, script_only, "x");
        assert!(!take_change_on_blur(&mut dom, script_only), "unmarked");

        let reverted = control(&mut dom, FormControlKind::TextInput, "");
        record_focus_snapshot(&mut dom, reverted);
        user_append(&mut dom, reverted, "a");
        set_value(&mut dom, reverted, "");
        assert!(!take_change_on_blur(&mut dom, reverted), "value = baseline");
    }

    /// C7 (l): a non-user write before the first mark is absorbed into the
    /// baseline; later marks do not move it.
    #[test]
    fn c7l_first_mark_rebaselines_once() {
        let mut dom = EcsDom::new();
        let input = control(&mut dom, FormControlKind::TextInput, "");
        record_focus_snapshot(&mut dom, input);
        set_value(&mut dom, input, "x");
        user_append(&mut dom, input, "y");
        assert_eq!(snap(&dom, input), Snap::Baseline("x".to_string()));
        user_append(&mut dom, input, "z");
        assert_eq!(snap(&dom, input), Snap::Baseline("x".to_string()));
    }

    /// The plan's §4.4 truth table: one row per [`FormControlKind`], with the
    /// spec ground of the expected verdict, stated independently of
    /// `change_on_blur_kind_applies`. What is enforced: [`row`] (next to this
    /// table) is an exhaustive `match` with no wildcard, so a new variant fails
    /// to compile there and its table row is added in the same edit;
    /// `truth_table_rows_are_indexed_by_row` checks every existing row's index
    /// agrees with `row`; `TRUTH_TABLE`'s length is the `26` in its type. What
    /// stays manual: writing the row itself (that the table has a row for the
    /// variant is not a compile-time fact).
    const TRUTH_TABLE: [(FormControlKind, bool); 26] = {
        use FormControlKind as K;
        [
            // QB-2 + R-2: the events apply, no input activation behavior.
            (K::TextInput, true),
            (K::Search, true),
            (K::Tel, true),
            (K::Url, true),
            (K::Email, true),
            (K::Password, true),
            (K::Date, true),
            (K::Month, true),
            (K::Week, true),
            (K::Time, true),
            (K::DatetimeLocal, true),
            (K::Number, true),
            (K::Range, true),
            // D-4: `<textarea>` follows the text states.
            (K::TextArea, true),
            // QB-2 + R-2: the events apply, but an input activation behavior is
            // defined.
            (K::Checkbox, false),
            (K::Radio, false),
            (K::File, false),
            (K::Color, false),
            // QB-1: the events do not apply.
            (K::Hidden, false),
            (K::SubmitButton, false),
            (K::ResetButton, false),
            (K::Button, false),
            // O-B1: not `input` states.
            (K::Select, false),
            (K::Output, false),
            (K::Meter, false),
            (K::Progress, false),
        ]
    };

    /// `kind`'s row in [`TRUTH_TABLE`]. Exhaustive, no wildcard.
    const fn row(kind: FormControlKind) -> usize {
        use FormControlKind as K;
        match kind {
            K::TextInput => 0,
            K::Search => 1,
            K::Tel => 2,
            K::Url => 3,
            K::Email => 4,
            K::Password => 5,
            K::Date => 6,
            K::Month => 7,
            K::Week => 8,
            K::Time => 9,
            K::DatetimeLocal => 10,
            K::Number => 11,
            K::Range => 12,
            K::TextArea => 13,
            K::Checkbox => 14,
            K::Radio => 15,
            K::File => 16,
            K::Color => 17,
            K::Hidden => 18,
            K::SubmitButton => 19,
            K::ResetButton => 20,
            K::Button => 21,
            K::Select => 22,
            K::Output => 23,
            K::Meter => 24,
            K::Progress => 25,
        }
    }

    /// Every table row sits at its kind's [`row`] index; with `row` exhaustive
    /// and injective over the table length, each variant has exactly one row.
    #[test]
    fn truth_table_rows_are_indexed_by_row() {
        for (i, (kind, _)) in TRUTH_TABLE.iter().enumerate() {
            assert_eq!(row(*kind), i, "{kind:?} is not at its row index");
        }
    }

    /// PB-1α: the seed writes an unmarked component on an `input` element of
    /// every kind (the population is the element, not the kind).
    #[test]
    fn seed_writes_every_input_kind() {
        for (kind, _) in TRUTH_TABLE {
            let mut dom = EcsDom::new();
            let input = control(&mut dom, kind, "v");
            record_focus_snapshot(&mut dom, input);
            assert_eq!(snap(&dom, input), Snap::Unmarked, "{kind:?}");
        }
    }

    /// PB-2: a `textarea` element is seeded.
    #[test]
    fn seed_writes_textarea() {
        let mut dom = EcsDom::new();
        let ta = attached_element(&mut dom, "textarea", &[]);
        record_focus_snapshot(&mut dom, ta);
        assert_eq!(snap(&dom, ta), Snap::Unmarked);
    }

    /// PB-3: other `FormControlState` elements get no write at all: no insert,
    /// and a component the test pre-inserted is untouched (no remove, no
    /// overwrite).
    #[test]
    fn seed_writes_nothing_for_non_population() {
        for tag in ["select", "button", "output", "meter", "progress"] {
            let mut dom = EcsDom::new();
            let fresh = attached_element(&mut dom, tag, &[]);
            record_focus_snapshot(&mut dom, fresh);
            assert_eq!(snap(&dom, fresh), Snap::Absent, "<{tag}>: no insert");

            let marked = attached_element(&mut dom, tag, &[]);
            dom.world_mut()
                .insert_one(marked, FocusValueSnapshot(Some("kept".to_string())))
                .unwrap();
            record_focus_snapshot(&mut dom, marked);
            assert_eq!(
                snap(&dom, marked),
                Snap::Baseline("kept".to_string()),
                "<{tag}>: a pre-existing component is untouched"
            );
        }
    }

    /// PB-4: `change_on_blur_kind_applies` over every kind (§4.4).
    #[test]
    fn kind_predicate_truth_table() {
        for (kind, expected) in TRUTH_TABLE {
            assert_eq!(change_on_blur_kind_applies(kind), expected, "{kind:?}");
        }
    }
}
