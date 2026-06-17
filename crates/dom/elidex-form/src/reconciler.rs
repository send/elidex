//! [`FormControlReconciler`] — derived-state reconciler for
//! [`FormControlState`].
//!
//! Subscribes to [`MutationEvent::AttributeChange`] (per-attr partial
//! reconciliation of FCS fields) and [`MutationEvent::Insert`]
//! (FCS-absence-guarded attach for newly-inserted form-control
//! elements) via the D-31 `ConsumerDispatcher` typed composer (plain
//! `pub fn handle`, per sibling-consumer convention).
//!
//! ECS-native first: `FormControlState` is a derived component
//! (source-of-truth = `Attributes` content attribute). Per ECS first
//! principles, derived-state reconciliation belongs to a system
//! subscribed to mutations of the source state, NOT a side effect of
//! every IDL setter. Single reconciler path covers IDL setter /
//! `setAttribute` / parser / `innerHTML` / future Custom Element
//! attribute callback uniformly via the [`EcsDom::set_attribute`] /
//! [`EcsDom::remove_attribute`] chokepoint.
//!
//! Per-attr arms cite their respective HTML sections (§4.10.5.3.x for
//! input-common attrs, §4.10.19.x for naming/disabled/autofill).

use elidex_ecs::{EcsDom, Entity, MutationEvent, TagType};

use crate::{
    clear_focus_snapshot, compile_pattern_regex, create_form_control_state,
    sanitize_for_type_change,
};
use crate::{FormControlKind, FormControlState};

/// [`MutationEvent`] consumer maintaining [`FormControlState`] derived
/// fields against attribute mutations.
///
/// Plain unit struct (no state) — all state lives in the
/// [`FormControlState`] ECS component on form-control entities.
/// Composed as a typed field of `elidex_js::vm::consumer_dispatcher::ConsumerDispatcher`.
pub struct FormControlReconciler;

impl FormControlReconciler {
    /// Single-method dispatch entry invoked by
    /// `elidex_js::vm::consumer_dispatcher::ConsumerDispatcher`.
    pub fn handle(&mut self, event: &MutationEvent<'_>, dom: &mut EcsDom) {
        match *event {
            MutationEvent::Insert { node, .. } => handle_insert(node, dom),
            MutationEvent::AttributeChange {
                node,
                name,
                new_value,
                ..
            } => handle_attribute_change(node, name, new_value, dom),
            _ => {}
        }
    }
}

/// WHATWG DOM §4.2.3 "insert" runs the insertion steps for `node` **and
/// its shadow-inclusive descendants**, and HTML §4.10.18.3 associates
/// each form-associated element on insertion.  `MutationEvent::Insert`
/// fires once per mutation root (a single subtree append is one event),
/// so this walks the inserted subtree and attaches `FormControlState` to
/// every form-control descendant — not just the root.  Without the walk,
/// nested `<input>`/`<select>`/`<textarea>` in a dynamically-inserted
/// subtree (`innerHTML`, `appendChild` of a built fragment) never receive
/// FCS.  Mirrors the shadow-inclusive descendant walk
/// `CustomElementReactionConsumer` uses for the same reason.
///
/// Defers to [`create_form_control_state`], a no-op for non-form-control
/// tags (returns `false` after the `FormControlState::from_element` tag
/// dispatch returns `None`).
///
/// Per-entity FCS-absence-guarded: `create_form_control_state` is NOT
/// idempotent — it `insert_one`s unconditionally, overwriting any
/// existing FCS and destroying `dirty_value` / user-edit state.  The
/// guard is applied per descendant so a DocumentFragment-move (re-parent
/// of a subtree whose controls already carry FCS) preserves their
/// user-edit state.
///
/// Foreign-namespace elements are excluded centrally by
/// [`create_form_control_state`]'s HTML-namespace gate (form controls are
/// HTML elements). This matters precisely because the subtree walk now
/// reaches nested foreign content: `innerHTML = "<svg><input></svg>"` keeps
/// `input` SVG-namespaced (it is not in the SVG breakout list), so without
/// the gate the SVG node would wrongly receive `FormControlState`.
fn handle_insert(node: Entity, dom: &mut EcsDom) {
    // Two-phase: collect the subtree under the read-only walker, then
    // mutate (the walker borrows `&self`; FCS attach needs `&mut`).
    let mut subtree = Vec::new();
    dom.for_each_shadow_inclusive_descendant(node, &mut |e| subtree.push(e));
    for entity in subtree {
        if dom.world().get::<&FormControlState>(entity).is_ok() {
            continue;
        }
        let _ = create_form_control_state(dom, entity);
    }
}

/// HTML §4.10.5.1.13 continuous Range correction: when a **grid-affecting**
/// attribute changes (`min`/`max`/`step`, or the `value` content attribute
/// that serves as the step base when `min` is absent — §4.10.5.3.7), a
/// Range control "must" re-clamp/snap its value.  This is stated
/// unconditionally (NOT part of the value-sanitization trigger set, and NOT
/// gated on the dirty flag — a slider cannot represent an out-of-range /
/// off-grid value).  A no-op for every other kind: number/date/time KEEP an
/// out-of-range value for their constraint validation to report (number's
/// step rounding is only a "may").
fn recorrect_range(fcs: &mut FormControlState) {
    if fcs.kind == FormControlKind::Range {
        fcs.settle_value();
    }
}

/// WHATWG DOM §4.9 attribute change steps — partial re-derivation of
/// [`FormControlState`] fields based on attribute name.  `new_value`
/// `None` = removed (reset to default); `Some(v)` = set to `v`.
fn handle_attribute_change(node: Entity, name: &str, new_value: Option<&str>, dom: &mut EcsDom) {
    // `type` arm: HTML §4.10.5 "input type change steps" (and §4.10.6
    // for `<button>`).  In-place kind update + `sanitize_for_type_
    // change` preserves user-input state (dirty `value`, `checked`,
    // selection, etc.) per the spec — value sanitization clears only
    // non-numeric values on entry into `type=number`.  Clearing
    // checked/indeterminate on a checkable→non-checkable switch is an
    // elidex normalization beyond the spec type-change steps (the spec
    // leaves them inert, not cleared); everything else persists.  Full
    // `from_element` re-derive would clobber
    // user state (regresses
    // `elidex-js/src/vm/host/html_input_proto.rs::native_input_set_type`
    // contract preservation).
    if name == "type" {
        let new_kind = match dom.world().get::<&TagType>(node) {
            Ok(tag) => match FormControlKind::from_tag_and_type_attr(&tag.0, new_value) {
                Some(k) => k,
                None => return,
            },
            Err(_) => return,
        };
        // Immutable TagType borrow dropped at the `}` above.
        {
            let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(node) else {
                return;
            };
            let old_kind = state.kind;
            state.kind = new_kind;
            sanitize_for_type_change(&mut state, old_kind);
        }
        // HTML §4.10.5.5 change-on-blur baseline: a `type` flip to a non-text
        // kind while the control is focused must drop the focus-time text
        // snapshot, else the eventual blur consumes the stale baseline and fires
        // a spurious `change`. `record_focus_snapshot` only re-evaluates at focus
        // time, so the mid-focus flip needs this clear at the `set_attribute`
        // chokepoint (mirrors the non-text clear inside `record_focus_snapshot`).
        if !new_kind.is_text_control() {
            clear_focus_snapshot(dom, node);
        }
        return;
    }

    let Ok(mut fcs) = dom.world_mut().get::<&mut FormControlState>(node) else {
        return;
    };

    // `value` and `pattern` live outside the match because their
    // guard-conditional bodies create no-op branches that clippy's
    // match-arm-collapsing rule would flag as duplicate of the
    // wildcard.  HTML §4.10.5.4 dirty-value-flag suppression / §4.10.5.3.6
    // pattern same-value short-circuit.
    if name == "value" {
        let raw = new_value.unwrap_or("");
        // For `<input>`, `defaultValue` reflects the `value` content
        // attribute (HTML §4.10.5.4 — the IDL attribute "must reflect
        // the value content attribute"), so `default_value` must track
        // every `value` attribute mutation INCLUDING while the dirty
        // value flag suppresses the IDL `value` update — otherwise
        // anything reading the step base off this field (`input.rs`
        // `step_base`, §4.10.5.3.7) sees a stale base for a dirty input
        // whose `value` attribute was later changed.  `<textarea>` /
        // `<select>` / `<output>` take their default value from child
        // text content, NOT a `value` attribute, so their
        // `default_value` must not be driven by it (the `!dirty_value`
        // branch below preserves their pre-existing handling).
        if !fcs.dirty_value
            || !matches!(
                fcs.kind,
                FormControlKind::TextArea
                    | FormControlKind::Select
                    | FormControlKind::Output
                    | FormControlKind::Meter
                    | FormControlKind::Progress
            )
        {
            fcs.default_value.clear();
            fcs.default_value.push_str(raw);
        }
        if fcs.dirty_value {
            // Dirty: the live value is suppressed (dirty-value-flag), but
            // for a Range control the `value` content attribute is the step
            // base when `min` is absent (§4.10.5.3.7) — so changing it can
            // shift the step grid under the dirty value and leave it off-grid.
            // Range must continuously correct that (§4.10.5.1.13).
            recorrect_range(&mut fcs);
        } else {
            // HTML §4.10.5.1.18 (submit) / §4.10.5.1.19 (reset) default
            // button label substitution — matches the `from_input_element`
            // path at createElement time.
            let displayed: &str = match fcs.kind {
                FormControlKind::SubmitButton if raw.is_empty() => "Submit",
                FormControlKind::ResetButton if raw.is_empty() => "Reset",
                _ => raw,
            };
            // HTML §4.10.5: "When the value content attribute is added,
            // set, or removed, if the control's dirty value flag is false,
            // the user agent must set the value of the element to the value
            // of the value content attribute … and then run the value
            // sanitization algorithm."  This content-attribute replacement is
            // a "relevant value change" with NO explicit cursor-move policy
            // (unlike the IDL `value` setter §4.10.5.4 step 5), so HTML
            // §4.10.20 applies: the cursor / selection are only CLAMPED into
            // the (possibly shorter) replacement value — positions and
            // selection direction otherwise preserved — which is exactly
            // `settle_value`.  Inside `!dirty_value`: a `value`-attribute
            // change never re-sanitizes a dirty live value (R2).
            fcs.value.clear();
            fcs.value.push_str(displayed);
            fcs.settle_value();
        }
        return;
    }
    if name == "pattern" {
        if new_value != fcs.pattern.as_deref() {
            if let Some(p) = new_value {
                fcs.pattern = Some(p.to_string());
                fcs.cached_pattern_regex = Some(compile_pattern_regex(p));
            } else {
                fcs.pattern = None;
                fcs.cached_pattern_regex = None;
            }
        }
        return;
    }

    match name {
        // HTML §4.10.19.1 "Naming form controls: the `name` attribute".
        "name" => fcs.name = new_value.unwrap_or("").to_string(),

        // HTML §4.10.18.3 "Association of controls and forms".
        // Preserve `from_input_element` / `from_button_element`
        // semantics: any `Some(s)` (including empty) stores `Some(s)`;
        // attribute removal stores `None`.  Downstream
        // `radio::find_form_by_id` treats `Some("")` as a no-match
        // (no form has `id=""`), so empty-string vs `None` are
        // functionally equivalent for form association.
        "form" => fcs.form_owner = new_value.map(str::to_string),

        // Boolean attributes (HTML §2.5.2 — presence ⇒ true).
        // HTML §4.10.19.5 disabled / §4.10.5.3.4 required /
        // §4.10.5.3.3 readonly / §6.6.7 autofocus /
        // §4.10.5.3.5 multiple.
        "disabled" => fcs.disabled = new_value.is_some(),
        "required" => fcs.required = new_value.is_some(),
        "readonly" => fcs.readonly = new_value.is_some(),
        "autofocus" => fcs.autofocus = new_value.is_some(),
        "multiple" => {
            fcs.multiple = new_value.is_some();
            // HTML §4.10.5.1.5 (Email state): "When the multiple attribute
            // is set or removed, the user agent must run the value
            // sanitization algorithm" — the Email state switches between
            // single (strip+trim) and comma-list sanitization, so the
            // stored value must be re-sanitized.  This trigger is
            // EMAIL-SPECIFIC (it lives in the Email-state section): `multiple`
            // does not affect any other kind's sanitization, so gating on
            // Email avoids an irrelevant re-sanitize (e.g. a `multiple`
            // toggle must not clamp a range value that an earlier min/max
            // change deliberately left out of range — min/max are not
            // sanitization triggers).  Runs unconditionally on the live
            // value (NOT gated on `!dirty_value`, unlike the `value`-arm):
            // a dirty `" x , y "` must become single-mode `"x , y"` when
            // `multiple` is removed.
            if fcs.kind == FormControlKind::Email {
                fcs.settle_value();
            }
        }

        // Numeric length attributes (HTML §4.10.5.3.1 maxlength/
        // minlength).  `None` ⇒ unset, parse-failure ⇒ unset.
        "maxlength" => fcs.maxlength = new_value.and_then(|s| s.parse::<usize>().ok()),
        "minlength" => fcs.minlength = new_value.and_then(|s| s.parse::<usize>().ok()),

        // HTML §4.10.5.3.2 "The `size` attribute" (input) / §4.10.7
        // (select).  u32 field; parse-failure / removal ⇒ 0 (concrete
        // defaults are applied at `from_element` time per element
        // type; dynamic mutations use the raw parsed value).
        "size" => fcs.size = new_value.and_then(|s| s.parse::<u32>().ok()).unwrap_or(0),

        // HTML §4.10.5.3.10 "The `placeholder` attribute".
        "placeholder" => fcs.placeholder = new_value.unwrap_or("").to_string(),

        // HTML §4.10.19.7 "Autofill".
        "autocomplete" => fcs.autocomplete = new_value.unwrap_or("").to_string(),

        // HTML §4.10.5.3.7 min/max / §4.10.5.3.8 step.
        "min" | "max" | "step" => {
            let v = new_value.map(str::to_string);
            match name {
                "min" => fcs.min = v,
                "max" => fcs.max = v,
                _ => fcs.step = v,
            }
            recorrect_range(&mut fcs);
        }

        // Attribute not in FormControlState — ignore.
        _ => {}
    }
}

#[cfg(test)]
mod tests {
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
        assert!(dom.set_attribute(e, "name", "q"));
        with_fcs(&dom, e, |s| assert_eq!(s.name, "q"));
    }

    #[test]
    fn e2_set_type_attribute_full_rederives_kind() {
        // <button type="submit"> (default kind = SubmitButton).
        let (mut dom, e) = setup("button", &[]);
        with_fcs(&dom, e, |s| {
            assert_eq!(s.kind, FormControlKind::SubmitButton);
        });
        assert!(dom.set_attribute(e, "type", "button"));
        with_fcs(&dom, e, |s| assert_eq!(s.kind, FormControlKind::Button));
    }

    #[test]
    fn e3_set_form_attribute_reflects_form_owner() {
        let (mut dom, e) = setup("input", &[]);
        assert!(dom.set_attribute(e, "form", "login"));
        with_fcs(&dom, e, |s| {
            assert_eq!(s.form_owner.as_deref(), Some("login"));
        });
    }

    #[test]
    fn e4_required_boolean_presence_and_removal() {
        let (mut dom, e) = setup("input", &[]);
        // Empty-string presence per HTML §2.5.2 boolean-attribute rule.
        assert!(dom.set_attribute(e, "required", ""));
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
        assert!(dom.set_attribute(e, "value", "from-attr"));
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
        assert!(dom.set_attribute(e, "value", "from-attr"));
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
        assert!(dom.set_attribute(e, "value", "x"));
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
        assert!(dom.set_attribute(div, "data-foo", "bar"));
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
        assert!(dom.set_attribute(e, "type", "text"));
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
        assert!(dom.set_attribute(e, "type", "checkbox"));
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
        assert!(dom.set_attribute(e, "type", "search"));
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
                s.cursor_pos, 0,
                "initial cursor at the beginning (§4.10.20)"
            );
        });
        assert!(dom.set_attribute(e, "value", "short"));
        with_fcs(&dom, e, |s| {
            assert_eq!(s.value, "short");
            assert_eq!(s.default_value, "short");
            assert_eq!(s.char_count, 5);
            assert_eq!(s.cursor_pos, 0, "cursor 0 is in-bounds → clamp leaves it");
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
        assert!(dom.set_attribute(e, "value", "short"));
        with_fcs(&dom, e, |s| {
            assert_eq!(s.value, "short");
            // (10, 14) both clamp to the new length (5).
            assert_eq!(s.selection_start, 5);
            assert_eq!(s.selection_end, 5);
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
        assert!(dom.set_attribute(e, "value", ""));
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
        assert!(dom.set_attribute(e, "value", "150"));
        with_fcs(&dom, e, |s| assert_eq!(s.value, "100"));
    }

    #[test]
    fn multiple_toggle_resanitizes_email() {
        // §4.10.5.1.5: setting `multiple` re-runs value sanitization,
        // switching the Email state to the comma-list algorithm.
        let (mut dom, e) = setup("input", &[("type", "email"), ("value", " a@b , c@d ")]);
        // Parsed under single-mode (strip + trim ends).
        with_fcs(&dom, e, |s| assert_eq!(s.value, "a@b , c@d"));
        assert!(dom.set_attribute(e, "multiple", ""));
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
        assert!(dom.set_attribute(e, "multiple", ""));
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
        assert!(dom.set_attribute(e, "value", "80"));
        with_fcs(&dom, e, |s| assert_eq!(s.value, "80"));
        // `max` lowered below the value → overflow → clamp to new max.
        assert!(dom.set_attribute(e, "max", "50"));
        with_fcs(&dom, e, |s| assert_eq!(s.value, "50"));
        // `step` change makes the value off-grid → snap to nearest in-range
        // step (grid 0,30 within [0,50]; 50 is 20 from 30, 30 from 60-oor →
        // nearest in-range is 30).
        assert!(dom.set_attribute(e, "step", "30"));
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
        assert!(dom.set_attribute(e, "value", "50"));
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
        assert!(dom.set_attribute(e, "max", "50"));
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
        assert!(dom.set_attribute(e, "multiple", ""));
        with_fcs(&dom, e, |s| {
            assert_eq!(
                s.value, "a\nb",
                "multiple toggle on a non-email must not sanitize (newline kept)"
            );
        });
    }
}
