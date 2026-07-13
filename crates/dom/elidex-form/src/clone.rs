//! Form-control **cloning steps** (WHATWG HTML §4.10.5 `<input>` /
//! §4.10.11 `<textarea>`) — the DOM §4.4 "clone a node" step-3 hook.
//!
//! `cloneNode` (and every other DOM §4.4 clone caller) must propagate a form
//! control's *live* state — not just its attributes — onto the clone:
//! `input.value = 'x'; input.cloneNode().value` must be `'x'`, not the
//! `value`-attribute default. The ECS cloner copies only the clone-policy
//! component set, which deliberately excludes [`FormControlState`], so a fresh
//! clone has no form state until the reconciler re-derives an attribute
//! *default* on insertion — losing any user-typed / JS-set value, checkedness,
//! or indeterminateness.
//!
//! This module closes that gap. The `elidex-dom-api` cloner tags each `<input>`
//! / `<textarea>` clone with a transient [`ClonedFrom`] marker (it cannot read
//! `FormControlState` — the crate dependency runs `elidex-form → elidex-dom-api`,
//! never the reverse); this consumer, invoked synchronously at clone time from
//! the VM `cloneNode` shim, resolves each marker and materializes the copy
//! **before any insertion**, so a detached clone reads correctly too.

use elidex_dom_api::ClonedFrom;
use elidex_ecs::{EcsDom, ElementState, Entity};

use crate::{create_form_control_state, FormControlKind, FormControlState};

/// Run the form-control cloning steps over the freshly cloned subtree rooted at
/// `clone_root` (WHATWG HTML §4.10.5 / §4.10.11).
///
/// Walks the clone's shadow-inclusive descendants **plus** every `<template>`'s
/// content fragment (`collect_template_inclusive_descendants`) and, for every
/// entity the cloner tagged with a [`ClonedFrom`] marker, copies the source's
/// cloning-step form state onto the clone, then removes the marker.
///
/// It creates the clone's `FormControlState` **at clone time**, so the
/// insert-time reconciler's absence guard then *skips* the clone (it does not
/// overwrite the copied state), and a never-inserted detached clone already
/// reads correctly.
///
/// The walk must include template contents: the cloner marks `<input>` /
/// `<textarea>` clones inside a deep-cloned `<template>`'s content too, and a
/// JS-created control moved into `template.content` keeps its `FormControlState`
/// (`for_each_shadow_inclusive_descendant` alone does not reach the detached
/// content fragment) — so a template-only walk would both drop that live value
/// and leave the marker unswept.
pub fn apply_clone_form_state(dom: &mut EcsDom, clone_root: Entity) {
    // Two-phase (the walker borrows `&self`, the copy below needs `&mut`):
    // collect first, mutate after. The frontier includes each `<template>`'s
    // content fragment so the coverage matches the cloner's marker attach (no
    // marker left unswept, no encapsulated control's value dropped).
    let subtree = crate::init::collect_template_inclusive_descendants(dom, clone_root);

    for dst in subtree {
        let Some(source) = dom
            .world()
            .get::<&ClonedFrom>(dst)
            .ok()
            .map(|marker| marker.source)
        else {
            continue;
        };
        // Sweep the transient marker unconditionally — even a no-op copy must
        // not leave clone provenance (holding a source entity ref) behind.
        let _ = dom.world_mut().remove_one::<ClonedFrom>(dst);
        copy_form_state(dom, source, dst);
    }
}

/// Copy the cloning-step fields from `source`'s form state onto `dst` (creating
/// `dst`'s `FormControlState`).
///
/// No-op when `source` carries no `FormControlState` — a foreign-namespace
/// `<input>` (the reconciler's HTML-namespace gate withheld one) or a
/// never-materialized control whose only state is its attribute default, which
/// the reconciler re-derives identically on the clone.
fn copy_form_state(dom: &mut EcsDom, source: Entity, dst: Entity) {
    let Some((kind, value, dirty_value, checked, indeterminate)) =
        dom.world().get::<&FormControlState>(source).ok().map(|s| {
            (
                s.kind,
                s.value.clone(),
                s.dirty_value,
                s.checked,
                s.indeterminate,
            )
        })
    else {
        return;
    };

    // Build the clone's default (attribute-consistent) state via the canonical
    // single-entity init — the SAME path the reconciler runs on insertion, so
    // the clone also gets `finalize_control`'s ElementState flags, `<textarea>`
    // child-text value, and fieldset-disabled derivation. The cloning-step
    // overlay below then replaces only the propagated fields.
    //
    // Reusing this path (rather than a bare `FormControlState::from_element`) is
    // load-bearing: `ElementState` is a clone-policy *non-copy*, so it is
    // re-derived by whoever creates the control's form state. By creating that
    // state here at clone time we trip the insert-time absence guard, meaning
    // the reconciler never gets its second chance — so `:checked` / `:disabled`
    // / `:required` / `:read-only` on the inserted clone would regress unless we
    // set `ElementState` on this path.
    //
    // Caveat — fieldset-disabled: this creates the FCS while the clone is a
    // detached orphan, so its `disabled` reflects its own attributes only (no
    // ancestor `<fieldset disabled>` to inherit from). Materializing it at clone
    // time is forced by the cloning steps' detached-read contract (a detached
    // `.value` must be correct). Inheriting a disabled ancestor is then NOT
    // re-derived when the clone is later inserted under a disabled `<fieldset>` —
    // but that is the *pre-existing* insert-time limitation `handle_insert`
    // already has for a `createElement`'d control or a moved control (reconciler
    // `e8b`), not one this slice introduces: it aligns the clone path with those.
    // The proper fix — dynamic fieldset-disabled propagation on insert/move —
    // is deferred (`#11-fieldset-disabled-dynamic-insert`).
    if !create_form_control_state(dom, dst) {
        return;
    }

    {
        let Ok(mut d) = dom.world_mut().get::<&mut FormControlState>(dst) else {
            return;
        };
        // §4.10.11 textarea cloning steps: raw value + dirty value flag.
        // §4.10.5 input cloning steps: + checkedness + indeterminateness. (The
        // dirty *checkedness* flag is unmodeled in `FormControlState` — deferred,
        // `#11-input-dirty-checkedness-flag`; a cloned control's observable
        // value / checkedness / indeterminateness are correct regardless.)
        d.value = value;
        d.dirty_value = dirty_value;
        if kind != FormControlKind::TextArea {
            d.checked = checked;
            d.indeterminate = indeterminate;
        }
        d.update_char_count();
    }

    // Re-sync the `:checked` and `:indeterminate` ElementState flags to the
    // *overlaid* state (inputs only — a textarea is never checked/indeterminate),
    // so the selector engine's `:checked` / `:indeterminate` reads stay consistent
    // with the copied `FormControlState`. `create_form_control_state` derived
    // `:checked` from the `checked` content attribute and `:indeterminate` as
    // `false` (it has no content attribute), but the cloning steps propagate the
    // live checkedness (differs for a dirtied checkbox) and the live
    // indeterminateness (a JS-only property), so both bits must be re-set here or
    // a cloned control reads e.g. `.indeterminate === true` yet
    // `matches(':indeterminate') === false` (Codex PR466 R5).
    if kind != FormControlKind::TextArea {
        if let Ok(mut es) = dom.world_mut().get::<&mut ElementState>(dst) {
            es.set(ElementState::CHECKED, checked);
            es.set(ElementState::INDETERMINATE, indeterminate);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom};

    fn attrs_map(attrs: &[(&str, &str)]) -> Attributes {
        let mut map = Attributes::default();
        for (k, v) in attrs {
            map.set((*k).to_string(), (*v).to_string());
        }
        map
    }

    /// Build an entity with the given tag + attributes and an attached
    /// `FormControlState` (as the reconciler / parse-time walk would).
    fn control(dom: &mut EcsDom, tag: &str, attrs: &[(&str, &str)]) -> Entity {
        let e = dom.create_element(tag, attrs_map(attrs));
        assert!(create_form_control_state(dom, e), "expected a form control");
        e
    }

    /// Simulate the ECS clone of `src`: a fresh entity with the same tag +
    /// attributes (the clone-policy copies `Attributes`) and a `ClonedFrom`
    /// marker — but *no* `FormControlState` yet (the clone-policy non-copy).
    fn cloned_from(dom: &mut EcsDom, src: Entity, tag: &str, attrs: &[(&str, &str)]) -> Entity {
        let dst = dom.create_element(tag, attrs_map(attrs));
        let _ = dom.world_mut().insert_one(dst, ClonedFrom { source: src });
        dst
    }

    #[test]
    fn input_value_and_dirty_flag_copied() {
        let mut dom = EcsDom::new();
        let src = control(&mut dom, "input", &[]);
        {
            let mut s = dom.world_mut().get::<&mut FormControlState>(src).unwrap();
            s.value = "typed".to_string();
            s.dirty_value = true;
            s.update_char_count();
        }
        let dst = cloned_from(&mut dom, src, "input", &[]);
        apply_clone_form_state(&mut dom, dst);

        let d = dom.world().get::<&FormControlState>(dst).unwrap();
        assert_eq!(d.value, "typed");
        assert!(d.dirty_value);
        // Marker swept.
        assert!(dom.world().get::<&ClonedFrom>(dst).is_err());
    }

    #[test]
    fn input_checked_and_indeterminate_copied_no_checked_attribute() {
        let mut dom = EcsDom::new();
        // Checkbox with NO `checked` attribute, live-toggled on (dirty).
        let src = control(&mut dom, "input", &[("type", "checkbox")]);
        {
            let mut s = dom.world_mut().get::<&mut FormControlState>(src).unwrap();
            s.checked = true;
            s.indeterminate = true;
        }
        let dst = cloned_from(&mut dom, src, "input", &[("type", "checkbox")]);
        apply_clone_form_state(&mut dom, dst);

        let d = dom.world().get::<&FormControlState>(dst).unwrap();
        assert!(d.checked);
        assert!(d.indeterminate);
        // :checked / :indeterminate ElementState flags re-synced to the live
        // state even though the clone has no `checked` content attribute and
        // `indeterminate` has no content attribute at all — so the selector
        // engine's `:checked` / `:indeterminate` reads match the copied FCS.
        let es = dom.world().get::<&ElementState>(dst).unwrap();
        assert!(es.contains(ElementState::CHECKED));
        assert!(
            es.contains(ElementState::INDETERMINATE),
            ":indeterminate must mirror the copied indeterminateness (Codex PR466 R5)"
        );
    }

    #[test]
    fn input_unchecked_live_clears_checked_element_state() {
        let mut dom = EcsDom::new();
        // Checkbox WITH `checked` attribute, live-toggled off (dirty).
        let src = control(&mut dom, "input", &[("type", "checkbox"), ("checked", "")]);
        {
            let mut s = dom.world_mut().get::<&mut FormControlState>(src).unwrap();
            s.checked = false;
        }
        let dst = cloned_from(
            &mut dom,
            src,
            "input",
            &[("type", "checkbox"), ("checked", "")],
        );
        apply_clone_form_state(&mut dom, dst);

        let d = dom.world().get::<&FormControlState>(dst).unwrap();
        assert!(!d.checked, "live-unchecked checkedness propagates");
        let es = dom.world().get::<&ElementState>(dst).unwrap();
        assert!(
            !es.contains(ElementState::CHECKED),
            ":checked must clear when the live checkedness is false despite the attribute"
        );
    }

    #[test]
    fn textarea_raw_value_and_dirty_copied() {
        let mut dom = EcsDom::new();
        let src = control(&mut dom, "textarea", &[]);
        {
            let mut s = dom.world_mut().get::<&mut FormControlState>(src).unwrap();
            s.value = "edited raw".to_string();
            s.dirty_value = true;
            s.update_char_count();
        }
        let dst = cloned_from(&mut dom, src, "textarea", &[]);
        apply_clone_form_state(&mut dom, dst);

        let d = dom.world().get::<&FormControlState>(dst).unwrap();
        assert_eq!(d.value, "edited raw");
        assert!(d.dirty_value);
        assert_eq!(d.kind, FormControlKind::TextArea);
    }

    #[test]
    fn non_form_control_pair_gets_no_state() {
        let mut dom = EcsDom::new();
        // A `<div>` carrying a (hypothetical) marker: the copy must not spawn a
        // spurious FormControlState, and the marker is swept.
        let src = dom.create_element("div", Attributes::default());
        let dst = dom.create_element("div", Attributes::default());
        let _ = dom.world_mut().insert_one(dst, ClonedFrom { source: src });
        apply_clone_form_state(&mut dom, dst);

        assert!(dom.world().get::<&FormControlState>(dst).is_err());
        assert!(dom.world().get::<&ClonedFrom>(dst).is_err());
    }

    #[test]
    fn source_without_form_state_yields_no_clone_state() {
        let mut dom = EcsDom::new();
        // Source `<input>` that never materialized a FormControlState.
        let src = dom.create_element("input", Attributes::default());
        let dst = cloned_from(&mut dom, src, "input", &[]);
        apply_clone_form_state(&mut dom, dst);

        // No source FCS to propagate → the clone stays FCS-less (the reconciler
        // re-derives the attribute default on eventual insertion, identically).
        assert!(dom.world().get::<&FormControlState>(dst).is_err());
        assert!(dom.world().get::<&ClonedFrom>(dst).is_err());
    }
}
