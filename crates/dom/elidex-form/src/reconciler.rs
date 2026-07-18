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

use crate::value_mode::apply_type_change_value_migration;
use crate::{
    clear_focus_snapshot, create_form_control_state, parse_positive_with_fallback,
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
        // Read the old kind before any mutation (short borrow, dropped
        // before the migration's `&mut EcsDom` calls).
        let Ok(old_kind) = dom.world().get::<&FormControlState>(node).map(|s| s.kind) else {
            return;
        };
        // HTML §4.10.5 type-change steps 1–3: value-mode value migration.
        // MUST run BEFORE the kind update + sanitization (steps 6–9).
        // Step 1 writes the `value` content attribute via the
        // non-dispatching primitive (re-entry contract — see fn docs).
        apply_type_change_value_migration(old_kind, new_kind, dom, node);
        // Steps 4–9: kind update + value sanitization + selectability.
        {
            let Ok(mut state) = dom.world_mut().get::<&mut FormControlState>(node) else {
                return;
            };
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
            fcs.update_pattern(new_value);
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

        // HTML §4.10.5 `#attr-input-checked` / `#concept-input-checked-dirty-flag`.
        // The `checked` content attribute gives the DEFAULT checkedness; setting
        // LIVE checkedness is gated on the (unmodeled → Slice 4) dirty checkedness
        // flag, so 0b maintains only the default half — the half `reset_value`
        // (elidex-form-core `lib.rs:684`) consumes.
        "checked" => fcs.default_checked = new_value.is_some(),
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

        // HTML §4.10.11 `#attr-textarea-rows` / `#attr-textarea-cols`; reflection
        // rule §2.6.1 (non-negative, > 0, with fallback). `ReflectDefault` 2 / 20.
        // Shared `parse_positive_with_fallback` = same maintainer as init.
        "rows" => fcs.rows = parse_positive_with_fallback(new_value, 2),
        "cols" => fcs.cols = parse_positive_with_fallback(new_value, 20),

        // Attribute not in FormControlState — ignore.
        _ => {}
    }
}

#[cfg(test)]
#[path = "reconciler_tests.rs"]
mod tests;
