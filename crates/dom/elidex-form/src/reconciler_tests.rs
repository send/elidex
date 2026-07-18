//! Tests for `reconciler.rs` — the `FormControlState` derived-state
//! reconciler (attribute-arm coverage + insertion attach). Split out to
//! keep `reconciler.rs` under the 1000-line review threshold
//! (sibling-module pattern, cf. `submit_tests.rs` / `lib_tests.rs`).

use super::*;
use crate::SelectionDirection;
use elidex_ecs::{Attributes, MutationDispatcher};

/// Minimal test dispatcher that wires ONLY [`FormControlReconciler`].
/// Used to validate end-to-end attribute mutation → reconciliation
/// without depending on the full production composer (which lives
/// in `elidex-js` and would create a circular cargo dep if
/// referenced from this crate's tests).
struct FormControlOnlyTestDispatcher(FormControlReconciler);
impl MutationDispatcher for FormControlOnlyTestDispatcher {
    fn dispatch(&mut self, event: &MutationEvent<'_>, dom: &mut EcsDom) {
        self.0.handle(event, dom);
    }
}

/// Build an EcsDom with a single form-control element, FCS
/// pre-attached, and the form-control reconciler installed.
fn setup(tag: &str, attrs: &[(&str, &str)]) -> (EcsDom, Entity) {
    let mut dom = EcsDom::new();
    let mut attr_map = Attributes::default();
    for (k, v) in attrs {
        attr_map.set(k.to_string(), v.to_string());
    }
    let entity = dom.create_element(tag, attr_map);
    assert!(
        create_form_control_state(&mut dom, entity),
        "create_form_control_state must succeed for tag {tag}"
    );
    let displaced = dom.set_mutation_dispatcher(Box::new(FormControlOnlyTestDispatcher(
        FormControlReconciler,
    )));
    assert!(displaced.is_none(), "no prior dispatcher installed");
    (dom, entity)
}

/// Read FCS field via callback (avoids returning a hecs::Ref
/// which would require declaring hecs as a direct dep of
/// elidex-form).
fn with_fcs<R>(dom: &EcsDom, entity: Entity, f: impl FnOnce(&FormControlState) -> R) -> R {
    let state = dom.world().get::<&FormControlState>(entity).unwrap();
    f(&state)
}

#[test]
fn e1_set_name_attribute_reflects_to_fcs() {
    let (mut dom, e) = setup("input", &[]);
    assert!(dom.set_attribute(e, "name", "q").did_set);
    with_fcs(&dom, e, |s| assert_eq!(s.name, "q"));
}

#[test]
fn e2_set_type_attribute_full_rederives_kind() {
    // <button type="submit"> (default kind = SubmitButton).
    let (mut dom, e) = setup("button", &[]);
    with_fcs(&dom, e, |s| {
        assert_eq!(s.kind, FormControlKind::SubmitButton);
    });
    assert!(dom.set_attribute(e, "type", "button").did_set);
    with_fcs(&dom, e, |s| assert_eq!(s.kind, FormControlKind::Button));
}

#[test]
fn e3_set_form_attribute_reflects_form_owner() {
    let (mut dom, e) = setup("input", &[]);
    assert!(dom.set_attribute(e, "form", "login").did_set);
    with_fcs(&dom, e, |s| {
        assert_eq!(s.form_owner.as_deref(), Some("login"));
    });
}

#[test]
fn e4_required_boolean_presence_and_removal() {
    let (mut dom, e) = setup("input", &[]);
    // Empty-string presence per HTML §2.5.2 boolean-attribute rule.
    assert!(dom.set_attribute(e, "required", "").did_set);
    with_fcs(&dom, e, |s| assert!(s.required));
    // Removal resets to default (false).
    dom.remove_attribute(e, "required");
    with_fcs(&dom, e, |s| assert!(!s.required));
}

#[test]
fn e5_value_attribute_suppressed_when_dirty_value_flag_set() {
    let (mut dom, e) = setup("input", &[("value", "initial")]);
    // Simulate user input by marking dirty + changing value.
    {
        let mut state = dom.world_mut().get::<&mut FormControlState>(e).unwrap();
        state.set_value("user-typed".to_string());
        assert!(state.dirty_value, "set_value must set dirty_value");
    }
    // Content-attribute write should NOT change FCS.value
    // (HTML §4.10.5.4 dirty value flag suppression).
    assert!(dom.set_attribute(e, "value", "from-attr").did_set);
    with_fcs(&dom, e, |s| assert_eq!(s.value, "user-typed"));
}

#[test]
fn e5e_value_attribute_write_updates_default_value_even_when_dirty() {
    // `defaultValue` reflects the `value` content attribute even
    // while the dirty value flag suppresses the IDL `value` update
    // (HTML §4.10.5.4).  The step base (§4.10.5.3.7) reads this
    // field, so it must stay fresh for a dirty input.
    let (mut dom, e) = setup("input", &[("value", "initial")]);
    {
        let mut state = dom.world_mut().get::<&mut FormControlState>(e).unwrap();
        state.set_value("user-typed".to_string());
        assert!(state.dirty_value);
    }
    assert!(dom.set_attribute(e, "value", "from-attr").did_set);
    with_fcs(&dom, e, |s| {
        // IDL value suppressed (dirty), but defaultValue reflects.
        assert_eq!(s.value, "user-typed");
        assert_eq!(s.default_value, "from-attr");
    });
}

#[test]
fn e5f_value_attribute_write_does_not_corrupt_dirty_textarea_default() {
    // A `<textarea>` takes its default value from child text content,
    // NOT a `value` content attribute, so a `value` attribute write
    // on a dirty textarea must not overwrite `default_value` (which
    // form reset restores).  Guards the input-only scoping of the
    // dirty-bypass defaultValue reflection.
    let (mut dom, e) = setup("textarea", &[]);
    {
        let mut s = dom.world_mut().get::<&mut FormControlState>(e).unwrap();
        s.set_value_initial("orig".to_string());
        s.set_value("typed".to_string());
        assert!(s.dirty_value);
    }
    assert!(dom.set_attribute(e, "value", "x").did_set);
    with_fcs(&dom, e, |s| {
        assert_eq!(s.value, "typed");
        assert_eq!(s.default_value, "orig");
    });
}

#[test]
fn e6_insert_form_control_without_fcs_attaches_fcs() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let mut attrs = Attributes::default();
    attrs.set("type".to_string(), "text".to_string());
    attrs.set("name".to_string(), "q".to_string());
    let input = dom.create_element("input", attrs);
    // Install dispatcher BEFORE append_child so the Insert event
    // is observed by the reconciler.
    let _ = dom.set_mutation_dispatcher(Box::new(FormControlOnlyTestDispatcher(
        FormControlReconciler,
    )));
    // Input has no FCS yet — reconciler's Insert arm should attach.
    assert!(dom.world().get::<&FormControlState>(input).is_err());
    assert!(dom.append_child(parent, input));
    with_fcs(&dom, input, |s| {
        assert_eq!(s.kind, FormControlKind::TextInput);
        assert_eq!(s.name, "q");
    });
}

#[test]
fn e7_insert_non_form_control_is_noop() {
    let mut dom = EcsDom::new();
    let parent = dom.create_element("div", Attributes::default());
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.set_mutation_dispatcher(Box::new(FormControlOnlyTestDispatcher(
        FormControlReconciler,
    )));
    assert!(dom.append_child(parent, div));
    // No FCS attached to <div>.
    assert!(dom.world().get::<&FormControlState>(div).is_err());
}

#[test]
fn e8_attribute_change_on_non_fcs_element_is_noop() {
    let mut dom = EcsDom::new();
    let div = dom.create_element("div", Attributes::default());
    let _ = dom.set_mutation_dispatcher(Box::new(FormControlOnlyTestDispatcher(
        FormControlReconciler,
    )));
    // Must not panic, must not spuriously attach FCS.
    assert!(dom.set_attribute(div, "data-foo", "bar").did_set);
    assert!(dom.world().get::<&FormControlState>(div).is_err());
}

#[test]
fn e2b_type_change_checkable_to_text_clears_checked_via_sanitize() {
    // <input type="checkbox" checked> → switch type to "text"
    // should clear FCS.checked.  This is an elidex normalization
    // (beyond the HTML §4.10.5 type-change steps, which leave
    // checkedness inert rather than clearing it) applied by
    // `sanitize_for_type_change`, invoked from the reconciler's
    // type-arm.
    let (mut dom, e) = setup("input", &[("type", "checkbox"), ("checked", "")]);
    {
        let mut state = dom.world_mut().get::<&mut FormControlState>(e).unwrap();
        state.checked = true;
    }
    assert!(dom.set_attribute(e, "type", "text").did_set);
    with_fcs(&dom, e, |s| {
        assert_eq!(s.kind, FormControlKind::TextInput);
        assert!(!s.checked, "sanitize_for_type_change must clear checked");
    });
}

#[test]
fn e2c_type_flip_to_non_text_while_focused_clears_focus_snapshot() {
    // Codex S2 F7: a focused text control whose `type` flips to a non-text
    // kind (text → checkbox) must drop the focus-time change-on-blur
    // snapshot at the `set_attribute` chokepoint, else a later blur consumes
    // the stale text baseline and fires a spurious `change`.
    // `record_focus_snapshot` only re-evaluates at focus time, so the
    // reconciler's type arm clears it mid-focus.
    use crate::{record_focus_snapshot, take_focus_snapshot};
    let (mut dom, e) = setup("input", &[("type", "text")]);
    {
        let mut state = dom.world_mut().get::<&mut FormControlState>(e).unwrap();
        state.set_value("typed".to_string());
    }
    record_focus_snapshot(&mut dom, e);
    assert!(dom.set_attribute(e, "type", "checkbox").did_set);
    with_fcs(&dom, e, |s| assert_eq!(s.kind, FormControlKind::Checkbox));
    assert_eq!(
        take_focus_snapshot(&mut dom, e),
        None,
        "type flip to a non-text kind must clear the stale focus snapshot"
    );
}

#[test]
fn e2d_type_flip_between_text_kinds_preserves_focus_snapshot() {
    // The complement of e2c: flipping between two text kinds (text → search)
    // keeps the control a text control, so the focus-time baseline must
    // survive — only a non-text destination clears it.
    use crate::{record_focus_snapshot, take_focus_snapshot};
    let (mut dom, e) = setup("input", &[("type", "text")]);
    {
        let mut state = dom.world_mut().get::<&mut FormControlState>(e).unwrap();
        state.set_value("typed".to_string());
    }
    record_focus_snapshot(&mut dom, e);
    assert!(dom.set_attribute(e, "type", "search").did_set);
    with_fcs(&dom, e, |s| assert_eq!(s.kind, FormControlKind::Search));
    assert_eq!(
        take_focus_snapshot(&mut dom, e),
        Some("typed".to_string()),
        "a text → text flip keeps the change-on-blur baseline"
    );
}

#[test]
fn e5b_non_dirty_value_write_syncs_cursor_and_default_value() {
    // Non-dirty `setAttribute("value", new)` must update FCS
    // value + char_count + default_value, and keep `cursor_pos` a valid
    // in-bounds offset so subsequent text-editing keypresses index
    // correctly.  Per HTML §4.10.20 the cursor is CLAMPED (not collapsed
    // to the end): a freshly parsed control starts with the cursor at the
    // beginning (0), and a relevant-value change only clamps if the
    // cursor is now past the end.
    let (mut dom, e) = setup("input", &[("value", "longer-initial")]);
    with_fcs(&dom, e, |s| {
        assert_eq!(
            s.cursor_pos(),
            0,
            "initial cursor at the beginning (§4.10.20)"
        );
    });
    assert!(dom.set_attribute(e, "value", "short").did_set);
    with_fcs(&dom, e, |s| {
        assert_eq!(s.value, "short");
        assert_eq!(s.default_value, "short");
        assert_eq!(s.char_count(), 5);
        assert_eq!(s.cursor_pos(), 0, "cursor 0 is in-bounds → clamp leaves it");
        assert!(!s.dirty_value);
    });
}

#[test]
fn e5d_non_dirty_value_write_clamps_stale_selection() {
    // HTML §4.10.20 relevant-value-change steps: when the value is
    // programmatically replaced (a non-dirty `value` content-attribute
    // write), a stale selection whose endpoints are now past the end of
    // the (shorter) value is CLAMPED to the end — but the selection
    // direction is NOT reset (that reset is specific to the §4.10.5.4
    // value-setter / type-change steps, which do not apply here).  This
    // keeps the "selection is within the value" invariant without
    // imposing a cursor-move policy the spec does not specify here.
    let (mut dom, e) = setup("input", &[("value", "longer-initial")]);
    // Simulate a `setSelectionRange(10, 14)` from JS.
    {
        let mut state = dom.world_mut().get::<&mut FormControlState>(e).unwrap();
        state.set_selection(10, 14);
        state.selection_direction = SelectionDirection::Forward;
    }
    // Non-dirty content-attribute write replaces the value.
    assert!(dom.set_attribute(e, "value", "short").did_set);
    with_fcs(&dom, e, |s| {
        assert_eq!(s.value, "short");
        // (10, 14) both clamp to the new length (5).
        assert_eq!(s.selection_start(), 5);
        assert_eq!(s.selection_end(), 5);
        // Direction is PRESERVED — §4.10.20 clamps positions only.
        assert_eq!(s.selection_direction, SelectionDirection::Forward);
    });
}

#[test]
fn e5c_submit_button_default_label_substitution_on_empty_value() {
    // <input type="submit"> with empty `value` attribute must
    // store FCS.value="Submit" (HTML §4.10.5.1.18 default button
    // label) to match `from_input_element` createElement-time
    // semantics; `default_value` keeps the raw attribute value.
    let (mut dom, e) = setup("input", &[("type", "submit")]);
    with_fcs(&dom, e, |s| {
        assert_eq!(s.kind, FormControlKind::SubmitButton);
        assert_eq!(s.value, "Submit");
    });
    // Explicit empty re-write keeps the substitution.
    assert!(dom.set_attribute(e, "value", "").did_set);
    with_fcs(&dom, e, |s| {
        assert_eq!(s.value, "Submit");
        assert_eq!(s.default_value, "");
    });
}

#[test]
fn e8b_insert_form_control_with_existing_fcs_preserves_dirty_value() {
    // DocumentFragment-move scenario: a form-control entity with
    // FCS already attached + dirty_value=true is re-parented.
    // The reconciler's FCS-absence guard must skip re-attach so
    // user-typed value + dirty_value flag are preserved.
    let (mut dom, input) = setup("input", &[("value", "initial")]);
    {
        let mut state = dom.world_mut().get::<&mut FormControlState>(input).unwrap();
        state.set_value("user-typed".to_string());
        assert!(state.dirty_value);
    }
    // Re-parent under a new <div> — fires Insert event again.
    let parent = dom.create_element("div", Attributes::default());
    assert!(dom.append_child(parent, input));
    // FCS + dirty_value preserved across move.
    with_fcs(&dom, input, |s| {
        assert_eq!(s.value, "user-typed");
        assert!(s.dirty_value);
    });
}

#[test]
fn e9_insert_subtree_attaches_fcs_to_nested_control() {
    // One `Insert` fires per mutation root, so a subtree append (e.g.
    // `innerHTML = "<div><span><input></span></div>"`) emits a single
    // event on the root. The reconciler must walk the subtree and
    // attach FCS to the nested control, not just the root.
    let mut dom = EcsDom::new();
    // Build the subtree detached, BEFORE installing the dispatcher, so
    // the only dispatched event is the final root append below.
    let outer = dom.create_element("div", Attributes::default());
    let span = dom.create_element("span", Attributes::default());
    let mut attrs = Attributes::default();
    attrs.set("name".to_string(), "q".to_string());
    let input = dom.create_element("input", attrs);
    assert!(dom.append_child(span, input));
    assert!(dom.append_child(outer, span));
    let root = dom.create_element("body", Attributes::default());

    let _ = dom.set_mutation_dispatcher(Box::new(FormControlOnlyTestDispatcher(
        FormControlReconciler,
    )));
    // Nested input has no FCS yet.
    assert!(dom.world().get::<&FormControlState>(input).is_err());
    // Single Insert on `outer` — reconciler walks to the nested input.
    assert!(dom.append_child(root, outer));
    with_fcs(&dom, input, |s| {
        assert_eq!(s.kind, FormControlKind::TextInput);
        assert_eq!(s.name, "q");
    });
}

#[test]
fn e9b_subtree_move_preserves_nested_dirty_value() {
    // Subtree-walk analogue of e8b: a re-parented subtree whose nested
    // control already carries FCS + dirty_value must keep it (the
    // per-entity FCS-absence guard skips re-attach).
    let mut dom = EcsDom::new();
    let outer = dom.create_element("div", Attributes::default());
    let input = dom.create_element("input", {
        let mut a = Attributes::default();
        a.set("value".to_string(), "initial".to_string());
        a
    });
    assert!(dom.append_child(outer, input));
    let root = dom.create_element("body", Attributes::default());

    let _ = dom.set_mutation_dispatcher(Box::new(FormControlOnlyTestDispatcher(
        FormControlReconciler,
    )));
    // First insertion attaches FCS to the nested input.
    assert!(dom.append_child(root, outer));
    with_fcs(&dom, input, |s| {
        assert_eq!(s.kind, FormControlKind::TextInput);
    });
    // Simulate user input → dirty_value.
    {
        let mut state = dom.world_mut().get::<&mut FormControlState>(input).unwrap();
        state.set_value("user-typed".to_string());
        assert!(state.dirty_value);
    }
    // Re-parent the whole subtree — Insert fires again on `outer`; the
    // per-entity guard must skip the nested input's re-attach.
    let section = dom.create_element("section", Attributes::default());
    assert!(dom.append_child(root, section));
    assert!(dom.append_child(section, outer));
    with_fcs(&dom, input, |s| {
        assert_eq!(s.value, "user-typed");
        assert!(s.dirty_value);
    });
}

#[test]
fn e9c_subtree_walk_skips_foreign_namespace_control() {
    // Codex #329 R4 (P2): the subtree walk now reaches nested foreign
    // content. An SVG-namespaced <input> (e.g. innerHTML
    // "<svg><input></svg>") must NOT receive FormControlState — the
    // central HTML-namespace gate in `create_form_control_state` excludes
    // it.
    use elidex_ecs::Namespace;
    let mut dom = EcsDom::new();
    let outer = dom.create_element("div", Attributes::default());
    let svg = dom.create_element_ns("svg", Namespace::Svg, Attributes::default(), None);
    let svg_input = dom.create_element_ns("input", Namespace::Svg, Attributes::default(), None);
    assert!(dom.append_child(svg, svg_input));
    assert!(dom.append_child(outer, svg));
    let root = dom.create_element("body", Attributes::default());

    let _ = dom.set_mutation_dispatcher(Box::new(FormControlOnlyTestDispatcher(
        FormControlReconciler,
    )));
    assert!(dom.append_child(root, outer));
    assert!(
        dom.world().get::<&FormControlState>(svg_input).is_err(),
        "SVG-namespaced <input> reached by the subtree walk must not get FCS"
    );
}

#[test]
fn e9d_nested_control_subtree_move_preserves_dirty_value() {
    // Combines e9 (nested control reached by the subtree walk) with e8b
    // (dirty-value preserved across re-parent): a NESTED control whose FCS
    // carries user input must keep it when the whole subtree is moved
    // (the per-entity absence guard skips re-attach during the walk).
    let mut dom = EcsDom::new();
    let outer = dom.create_element("div", Attributes::default());
    let span = dom.create_element("span", Attributes::default());
    let input = dom.create_element("input", {
        let mut a = Attributes::default();
        a.set("value".to_string(), "initial".to_string());
        a
    });
    assert!(dom.append_child(span, input));
    assert!(dom.append_child(outer, span));
    let root = dom.create_element("body", Attributes::default());

    let _ = dom.set_mutation_dispatcher(Box::new(FormControlOnlyTestDispatcher(
        FormControlReconciler,
    )));
    // First insertion attaches FCS to the nested input.
    assert!(dom.append_child(root, outer));
    with_fcs(&dom, input, |s| {
        assert_eq!(s.kind, FormControlKind::TextInput);
    });
    // User input on the nested control.
    {
        let mut state = dom.world_mut().get::<&mut FormControlState>(input).unwrap();
        state.set_value("user-typed".to_string());
        assert!(state.dirty_value);
    }
    // Move the whole subtree — Insert fires on `outer`; the walk reaches
    // the nested input but the per-entity guard skips re-attach.
    let section = dom.create_element("section", Attributes::default());
    assert!(dom.append_child(root, section));
    assert!(dom.append_child(section, outer));
    with_fcs(&dom, input, |s| {
        assert_eq!(s.value, "user-typed");
        assert!(s.dirty_value);
    });
}

#[test]
fn value_attr_arm_sanitizes_range() {
    // A `value` content-attribute write on a non-dirty range control
    // runs §4.10.5.1.13 value sanitization (clamp to max).
    let (mut dom, e) = setup("input", &[("type", "range"), ("max", "100")]);
    assert!(dom.set_attribute(e, "value", "150").did_set);
    with_fcs(&dom, e, |s| assert_eq!(s.value, "100"));
}

#[test]
fn multiple_toggle_resanitizes_email() {
    // §4.10.5.1.5: setting `multiple` re-runs value sanitization,
    // switching the Email state to the comma-list algorithm.
    let (mut dom, e) = setup("input", &[("type", "email"), ("value", " a@b , c@d ")]);
    // Parsed under single-mode (strip + trim ends).
    with_fcs(&dom, e, |s| assert_eq!(s.value, "a@b , c@d"));
    assert!(dom.set_attribute(e, "multiple", "").did_set);
    with_fcs(&dom, e, |s| assert_eq!(s.value, "a@b,c@d"));
}

#[test]
fn multiple_toggle_sanitizes_even_when_dirty() {
    // The `multiple`-toggle trigger runs on the live value
    // unconditionally (NOT gated on `!dirty_value`, unlike the
    // `value`-attribute arm): a dirty single-mode value re-sanitizes
    // under the comma-list algorithm when `multiple` is set.
    let (mut dom, e) = setup("input", &[("type", "email")]);
    {
        let mut state = dom.world_mut().get::<&mut FormControlState>(e).unwrap();
        state.set_value(" a , b ".to_string()); // single-mode → "a , b", dirty
        assert_eq!(state.value, "a , b");
        assert!(state.dirty_value);
    }
    assert!(dom.set_attribute(e, "multiple", "").did_set);
    with_fcs(&dom, e, |s| {
        assert_eq!(
            s.value, "a,b",
            "multiple toggle must re-sanitize a dirty value"
        );
        assert!(
            s.dirty_value,
            "re-sanitization must not clear the dirty flag"
        );
    });
}

#[test]
fn range_reclamps_on_min_max_step_change() {
    // HTML §4.10.5.1.13: a Range control must continuously correct an
    // over/underflow / step mismatch as its constraints change.
    let (mut dom, e) = setup("input", &[("type", "range"), ("min", "0"), ("max", "100")]);
    assert!(dom.set_attribute(e, "value", "80").did_set);
    with_fcs(&dom, e, |s| assert_eq!(s.value, "80"));
    // `max` lowered below the value → overflow → clamp to new max.
    assert!(dom.set_attribute(e, "max", "50").did_set);
    with_fcs(&dom, e, |s| assert_eq!(s.value, "50"));
    // `step` change makes the value off-grid → snap to nearest in-range
    // step (grid 0,30 within [0,50]; 50 is 20 from 30, 30 from 60-oor →
    // nearest in-range is 30).
    assert!(dom.set_attribute(e, "step", "30").did_set);
    with_fcs(&dom, e, |s| assert_eq!(s.value, "30"));
}

#[test]
fn dirty_range_recorrects_on_value_attr_step_base_change() {
    // §4.10.5.1.13 + §4.10.5.3.7: with no `min`, the `value` content
    // attribute is the Range step base.  Changing it shifts the grid, so
    // a dirty live value that was on the old grid must be re-corrected.
    let (mut dom, e) = setup(
        "input",
        &[("type", "range"), ("step", "20"), ("value", "40")],
    );
    // Dirty the live value to 60 (on the old grid, base 40: (60-40)/20=1).
    {
        let mut state = dom.world_mut().get::<&mut FormControlState>(e).unwrap();
        state.set_value("60".to_string());
        assert_eq!(state.value, "60");
        assert!(state.dirty_value);
    }
    // `value` attr → new step base 50; live 60 is off the new grid
    // (50,70,…) → snap to nearest in-range (tie 50/70 → +∞ → 70).
    assert!(dom.set_attribute(e, "value", "50").did_set);
    with_fcs(&dom, e, |s| {
        assert_eq!(s.value, "70", "dirty range re-corrected to the new grid");
        assert!(s.dirty_value, "re-correction must keep the dirty flag");
    });
}

#[test]
fn number_does_not_reclamp_on_max_change() {
    // Number (unlike Range) KEEPS an out-of-range value — its
    // constraint validation reports the overflow; min/max changes are
    // not a re-correction trigger (the spec's number step rounding is
    // only a "may").
    let (mut dom, e) = setup("input", &[("type", "number"), ("value", "80")]);
    with_fcs(&dom, e, |s| assert_eq!(s.value, "80"));
    assert!(dom.set_attribute(e, "max", "50").did_set);
    with_fcs(&dom, e, |s| {
        assert_eq!(s.value, "80", "number keeps out-of-range value");
    });
}

#[test]
fn multiple_toggle_does_not_resanitize_non_email() {
    // The `multiple`-toggle sanitization trigger is Email-specific
    // (§4.10.5.1.5) — toggling `multiple` on a non-email control must
    // not run value sanitization.  Seed a raw (unsanitized) newline
    // directly so that, if the gate were missing, the toggle would
    // strip it; the newline surviving proves sanitize was not called.
    let (mut dom, e) = setup("input", &[("type", "text")]);
    {
        let mut state = dom.world_mut().get::<&mut FormControlState>(e).unwrap();
        state.value = "a\nb".to_string(); // direct set bypasses sanitize
    }
    assert!(dom.set_attribute(e, "multiple", "").did_set);
    with_fcs(&dom, e, |s| {
        assert_eq!(
            s.value, "a\nb",
            "multiple toggle on a non-email must not sanitize (newline kept)"
        );
    });
}

// -- Slice 0b: `checked` arm (default-checkedness only) --------------

#[test]
fn checked_arm_sets_and_clears_default_checked() {
    // HTML §4.10.5 `#attr-input-checked`: the `checked` content attribute
    // gives the DEFAULT checkedness.  Arm A maintains `default_checked`
    // from the attribute (previously fell through `_ => {}`).
    let (mut dom, e) = setup("input", &[("type", "checkbox")]);
    assert!(dom.set_attribute(e, "checked", "").did_set);
    with_fcs(&dom, e, |s| assert!(s.default_checked));
    dom.remove_attribute(e, "checked");
    with_fcs(&dom, e, |s| assert!(!s.default_checked));
}

#[test]
fn checked_arm_reset_restores_default_checked() {
    // Umbrella §1.2 reset bug: `setAttribute("checked")` must update
    // `default_checked` so `reset_value` (restores `checked =
    // default_checked`) yields the correct checkedness.  Before arm A this
    // fell through `_ => {}` → reset restored a stale (false) checkedness.
    let (mut dom, e) = setup("input", &[("type", "checkbox")]);
    assert!(dom.set_attribute(e, "checked", "").did_set);
    {
        let mut state = dom.world_mut().get::<&mut FormControlState>(e).unwrap();
        state.reset_value();
    }
    with_fcs(&dom, e, |s| {
        assert!(s.checked, "reset restores checked from default_checked");
    });
}

#[test]
fn checked_arm_does_not_touch_live_checked() {
    // Non-regression / Slice-4 deferral guard: arm A maintains ONLY
    // `default_checked`.  Live checkedness (`FCS.checked`) is gated on the
    // unmodeled dirty checkedness flag (HTML §4.10.5
    // `#concept-input-checked-dirty-flag` → Slice 4), so the arm must NOT
    // touch it.  Seed a user-toggled live `checked = true`, then flip the
    // content attribute both ways: live `checked` stays `true` while
    // `default_checked` tracks the attribute.  A future Slice-4 change that
    // starts writing live checkedness from the arm trips the removal branch
    // (a spurious `checked = is_some()` would clobber it to false).
    let (mut dom, e) = setup("input", &[("type", "checkbox")]);
    {
        let mut state = dom.world_mut().get::<&mut FormControlState>(e).unwrap();
        state.checked = true; // user-toggled live checkedness
    }
    assert!(dom.set_attribute(e, "checked", "").did_set);
    with_fcs(&dom, e, |s| {
        assert!(s.default_checked, "default half tracks the attribute");
        assert!(s.checked, "live checkedness untouched (Slice-4 deferral)");
    });
    dom.remove_attribute(e, "checked");
    with_fcs(&dom, e, |s| {
        assert!(!s.default_checked, "default half cleared on removal");
        assert!(
            s.checked,
            "live checkedness STILL untouched by removal (would clobber to false if the arm wrote it)"
        );
    });
}

// -- Slice 0b: `rows`/`cols` arms (§2.6.1 positive-with-fallback) -----

#[test]
fn rows_cols_arms_apply_positive_fallback() {
    // HTML §4.10.11 rows/cols + §2.6.1 reflection: a valid `> 0` integer
    // is taken; `0` / non-numeric / absent fall back to the default
    // (2 / 20).
    let (mut dom, e) = setup("textarea", &[]);
    assert!(dom.set_attribute(e, "rows", "10").did_set);
    with_fcs(&dom, e, |s| assert_eq!(s.rows, 10));
    // `0` is not "> 0" → default.
    assert!(dom.set_attribute(e, "rows", "0").did_set);
    with_fcs(&dom, e, |s| assert_eq!(s.rows, 2));
    // Non-numeric → default.
    assert!(dom.set_attribute(e, "cols", "abc").did_set);
    with_fcs(&dom, e, |s| assert_eq!(s.cols, 20));
    // Valid cols.
    assert!(dom.set_attribute(e, "cols", "40").did_set);
    with_fcs(&dom, e, |s| assert_eq!(s.cols, 40));
    // Removal → default.
    dom.remove_attribute(e, "rows");
    with_fcs(&dom, e, |s| assert_eq!(s.rows, 2));
}

#[test]
fn rows_cols_reconcile_to_intrinsic_size() {
    // Integration: a runtime `rows`/`cols` attribute change now flows to
    // `form_intrinsic_size` (`sizing.rs`), which previously used the stale
    // init value (the umbrella §1.2 textarea layout bug).
    let (mut dom, e) = setup("textarea", &[]);
    assert!(dom.set_attribute(e, "rows", "5").did_set);
    assert!(dom.set_attribute(e, "cols", "40").did_set);
    let size = with_fcs(&dom, e, crate::form_intrinsic_size);
    // sizing.rs: w = cols * 8.0, h = rows * 18.0.
    assert_eq!(size.width, 40.0 * 8.0);
    assert_eq!(size.height, 5.0 * 18.0);
}

#[test]
fn rows_init_and_arm_agree_on_fallback() {
    // J-C single-maintainer parity: init (`from_textarea_element`) and the
    // reconciler arm both route through `parse_positive_with_fallback`, so
    // `rows="0"` yields the SAME `FCS.rows` (default 2) whether established
    // at createElement time or by a runtime attribute change.
    let (dom_init, e_init) = setup("textarea", &[("rows", "0")]);
    let init_rows = with_fcs(&dom_init, e_init, |s| s.rows);
    let (mut dom_arm, e_arm) = setup("textarea", &[]);
    assert!(dom_arm.set_attribute(e_arm, "rows", "0").did_set);
    let arm_rows = with_fcs(&dom_arm, e_arm, |s| s.rows);
    assert_eq!(
        init_rows, 2,
        "init `rows=0` → default 2 (§2.6.1, latent init bug fixed)"
    );
    assert_eq!(
        arm_rows, init_rows,
        "arm and init produce identical FCS.rows"
    );
}
