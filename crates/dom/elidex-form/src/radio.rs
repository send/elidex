//! Radio button group management.

use elidex_ecs::{Attributes, EcsDom, ElementState, Entity};

use crate::{FormControlKind, FormControlState};

/// Resolve the effective form owner for an entity.
///
/// Per HTML §4.10.18.3, the form owner is determined by:
/// 1. If the control has a `form` attribute, find the form element with that ID.
/// 2. Otherwise, the nearest ancestor `<form>` element.
///
/// Returns `None` if the control has no form owner. Used by `submit.rs`'s
/// form-owner lookup.
pub(crate) fn resolve_form_owner(dom: &EcsDom, entity: Entity) -> Option<Entity> {
    // Check for explicit `form` attribute (cross-tree association).
    let form_attr = dom
        .world()
        .get::<&FormControlState>(entity)
        .ok()
        .and_then(|fcs| fcs.form_owner.clone());

    if let Some(form_id) = form_attr {
        // Find the form element with the matching ID.
        return find_form_by_id(dom, &form_id);
    }

    // Fall back to ancestor form.
    crate::find_form_ancestor(dom, entity)
}

/// Find a `<form>` element by its `id` attribute.
///
/// Per WHATWG DOM §4.2.5 `getElementById` returns `null` for the
/// empty string (and HTML §3.2.5 "The id attribute" specifies that
/// id="" is treated as if absent); treat an empty `form="..."`
/// IDREF symmetrically by short-circuiting to `None` here rather
/// than matching forms that happen to carry an empty `id=""`
/// content attribute.
///
/// Tree order / same-tree scoping → `#11-form-owner-ecs-component`.
fn find_form_by_id(dom: &EcsDom, id: &str) -> Option<Entity> {
    if id.is_empty() {
        return None;
    }
    dom.world()
        .query::<(Entity, &elidex_ecs::TagType, &Attributes)>()
        .iter()
        .find(|(_, tag, attrs)| tag.0 == "form" && attrs.get("id") == Some(id))
        .map(|(e, _, _)| e)
}

/// Toggle a radio button: uncheck all others in the same name group, check this one.
///
/// Returns `true` if the radio was toggled (enabled, unchecked → checked).
/// Uses the radio button group per HTML §4.10.5.1.16.
pub fn toggle_radio(dom: &mut EcsDom, entity: Entity) -> bool {
    let info = dom
        .world()
        .get::<&FormControlState>(entity)
        .ok()
        .filter(|fcs| fcs.kind == FormControlKind::Radio && !fcs.disabled)
        .map(|fcs| fcs.checked);

    let Some(was_checked) = info else {
        return false;
    };

    // Clicking an already-checked radio leaves it checked (observed browser
    // behaviour; not stated as spec text).
    if was_checked {
        return false;
    }

    // Uncheck all radios in the same radio button group.
    let group = find_radio_group_scoped(dom, entity);
    for member in &group {
        if let Ok(mut fcs) = dom.world_mut().get::<&mut FormControlState>(*member) {
            fcs.checked = false;
        }
        if let Ok(mut es) = dom.world_mut().get::<&mut ElementState>(*member) {
            es.remove(ElementState::CHECKED);
        }
    }

    // Check this one.
    if let Ok(mut fcs) = dom.world_mut().get::<&mut FormControlState>(entity) {
        fcs.checked = true;
    }
    if let Ok(mut es) = dom.world_mut().get::<&mut ElementState>(entity) {
        es.insert(ElementState::CHECKED);
    }
    true
}

/// Find the radio button group of `radio` (HTML §4.10.5.1.16 "radio button
/// group"), in tree order, including `radio` itself.
///
/// Another radio `b` is in `radio`'s group iff all of the spec's conditions
/// hold: `b` is a radio, both have the same form owner (or both none), both
/// are in the same tree (same [`EcsDom::find_tree_root`]), and both have a
/// non-empty, equal `name`. An unnamed (or empty-named) radio therefore has
/// no other members: the result is just `[radio]`. Returns an empty vector
/// when `radio` is not a radio control.
///
/// Each candidate's form owner is resolved at read time (`resolve_form_owner`).
#[must_use]
pub fn find_radio_group_scoped(dom: &EcsDom, radio: Entity) -> Vec<Entity> {
    let Some(name) = dom
        .world()
        .get::<&FormControlState>(radio)
        .ok()
        .filter(|fcs| fcs.kind == FormControlKind::Radio)
        .map(|fcs| fcs.name.clone())
    else {
        return Vec::new();
    };
    if name.is_empty() {
        return vec![radio];
    }
    let form_owner = resolve_form_owner(dom, radio);
    let root = dom.find_tree_root(radio);

    // Collect candidates matching kind + name first, then filter by tree and
    // form owner.
    let candidates: Vec<Entity> = dom
        .world()
        .query::<(Entity, &FormControlState)>()
        .iter()
        .filter(|(_, fcs)| fcs.kind == FormControlKind::Radio && fcs.name == name)
        .map(|(e, _)| e)
        .collect();

    let mut group: Vec<Entity> = candidates
        .into_iter()
        .filter(|&e| {
            e == radio
                || (dom.find_tree_root(e) == root && resolve_form_owner(dom, e) == form_owner)
        })
        .collect();
    // Sort by tree order (pre-order traversal) for consistent DOM-order traversal.
    group.sort_by(|a, b| dom.tree_order_cmp(*a, *b));
    group
}

/// Check if a radio group's `required` constraint is satisfied.
///
/// Per HTML §4.10.5.3.4: a radio button's `required` constraint is satisfied
/// if any radio in the same name group is checked. Individual `check_required_checked`
/// validates per-control; use this function for group-level validation.
#[must_use]
pub fn is_radio_group_satisfied(group: &[&FormControlState]) -> bool {
    group.iter().any(|fcs| fcs.checked)
}

/// Navigate within a radio group (ArrowUp/ArrowDown).
///
/// Per HTML §4.10.5.1.16: uses the radio button group.
/// Returns the entity to focus next, if any.
#[must_use]
pub fn radio_arrow_navigate(dom: &EcsDom, current: Entity, forward: bool) -> Option<Entity> {
    let group = find_radio_group_scoped(dom, current);
    if group.len() <= 1 {
        return None;
    }
    let idx = group.iter().position(|&e| e == current)?;
    let next_idx = if forward {
        (idx + 1) % group.len()
    } else {
        (idx + group.len() - 1) % group.len()
    };
    Some(group[next_idx])
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom};

    fn make_radio(dom: &mut EcsDom, name: &str, checked: bool) -> Entity {
        let mut attrs = Attributes::default();
        attrs.set("type", "radio");
        attrs.set("name", name);
        if checked {
            attrs.set("checked", "");
        }
        let entity = dom.create_element("input", attrs.clone());
        let state = FormControlState::from_element("input", &attrs).unwrap();
        let _ = dom.world_mut().insert_one(entity, state);
        let _ = dom.world_mut().insert_one(entity, ElementState::default());
        entity
    }

    /// Create a radio with an explicit `form` attribute for cross-tree association.
    fn make_radio_with_form_attr(
        dom: &mut EcsDom,
        name: &str,
        form_id: &str,
        checked: bool,
    ) -> Entity {
        let mut attrs = Attributes::default();
        attrs.set("type", "radio");
        attrs.set("name", name);
        attrs.set("form", form_id);
        if checked {
            attrs.set("checked", "");
        }
        let entity = dom.create_element("input", attrs.clone());
        let state = FormControlState::from_element("input", &attrs).unwrap();
        let _ = dom.world_mut().insert_one(entity, state);
        let _ = dom.world_mut().insert_one(entity, ElementState::default());
        entity
    }

    #[test]
    fn toggle_radio_unchecks_others() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let r1 = make_radio(&mut dom, "color", true);
        let r2 = make_radio(&mut dom, "color", false);
        let r3 = make_radio(&mut dom, "color", false);
        for r in [r1, r2, r3] {
            let _ = dom.append_child(doc, r);
        }

        // Mark r1 as checked in state.
        if let Ok(mut fcs) = dom.world_mut().get::<&mut FormControlState>(r1) {
            fcs.checked = true;
        }

        assert!(toggle_radio(&mut dom, r2));

        assert!(!dom.world().get::<&FormControlState>(r1).unwrap().checked);
        assert!(dom.world().get::<&FormControlState>(r2).unwrap().checked);
        assert!(!dom.world().get::<&FormControlState>(r3).unwrap().checked);
    }

    #[test]
    fn toggle_already_checked_does_nothing() {
        let mut dom = EcsDom::new();
        let r1 = make_radio(&mut dom, "size", false);
        if let Ok(mut fcs) = dom.world_mut().get::<&mut FormControlState>(r1) {
            fcs.checked = true;
        }
        assert!(!toggle_radio(&mut dom, r1));
    }

    #[test]
    fn find_radio_group_same_name() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let r1 = make_radio(&mut dom, "g1", false);
        let r2 = make_radio(&mut dom, "g1", false);
        let r3 = make_radio(&mut dom, "g2", false);
        for r in [r1, r2, r3] {
            let _ = dom.append_child(doc, r);
        }

        let group = find_radio_group_scoped(&dom, r1);
        assert_eq!(group, vec![r1, r2]);
    }

    #[test]
    fn radio_arrow_navigate_wraps() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let r1 = make_radio(&mut dom, "nav", false);
        let r2 = make_radio(&mut dom, "nav", false);
        let r3 = make_radio(&mut dom, "nav", false);
        for r in [r1, r2, r3] {
            let _ = dom.append_child(doc, r);
        }

        // Forward from last wraps to first.
        assert_eq!(radio_arrow_navigate(&dom, r3, true), Some(r1));

        // Backward from first wraps to last.
        assert_eq!(radio_arrow_navigate(&dom, r1, false), Some(r3));
    }

    #[test]
    fn disabled_radio_not_toggled() {
        let mut dom = EcsDom::new();
        let mut attrs = Attributes::default();
        attrs.set("type", "radio");
        attrs.set("name", "x");
        attrs.set("disabled", "");
        let entity = dom.create_element("input", attrs.clone());
        let state = FormControlState::from_element("input", &attrs).unwrap();
        let _ = dom.world_mut().insert_one(entity, state);
        let _ = dom.world_mut().insert_one(entity, ElementState::default());

        assert!(!toggle_radio(&mut dom, entity));
    }

    /// A bare `<input type=radio name=g>` state with the given checkedness.
    /// (`FormControlState`'s value-model fields — cursor / selection /
    /// char_count — are `elidex-form-core`-private, so this builds from
    /// `default()` rather than a `..FormControlState::default()` update.)
    fn radio_state(checked: bool) -> FormControlState {
        let mut state = FormControlState::default();
        state.kind = FormControlKind::Radio;
        state.checked = checked;
        state.name = "g".to_string();
        state
    }

    #[test]
    fn radio_group_satisfied_when_one_checked() {
        let checked = radio_state(true);
        let unchecked = radio_state(false);
        assert!(is_radio_group_satisfied(&[&unchecked, &checked]));
    }

    #[test]
    fn radio_group_not_satisfied_when_none_checked() {
        let r1 = radio_state(false);
        let r2 = radio_state(false);
        assert!(!is_radio_group_satisfied(&[&r1, &r2]));
    }

    #[test]
    fn radios_in_different_forms_independent() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let form1 = dom.create_element("form", Attributes::default());
        let form2 = dom.create_element("form", Attributes::default());
        let _ = dom.append_child(doc, form1);
        let _ = dom.append_child(doc, form2);

        let r1 = make_radio(&mut dom, "color", false);
        let _ = dom.append_child(form1, r1);

        let r2 = make_radio(&mut dom, "color", false);
        let _ = dom.append_child(form2, r2);

        // Toggle r1 in form1.
        assert!(toggle_radio(&mut dom, r1));
        // Toggle r2 in form2 — should NOT uncheck r1 (different form scope).
        assert!(toggle_radio(&mut dom, r2));

        assert!(dom.world().get::<&FormControlState>(r1).unwrap().checked);
        assert!(dom.world().get::<&FormControlState>(r2).unwrap().checked);
    }

    #[test]
    fn formless_radios_share_group() {
        // Radios without a form ancestor (in the same tree) share a single
        // "no form" group.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let r1 = make_radio(&mut dom, "color", false);
        let r2 = make_radio(&mut dom, "color", false);
        let _ = dom.append_child(doc, r1);
        let _ = dom.append_child(doc, r2);

        // Both are formless — toggling r2 should uncheck r1.
        assert!(toggle_radio(&mut dom, r1));
        assert!(dom.world().get::<&FormControlState>(r1).unwrap().checked);

        assert!(toggle_radio(&mut dom, r2));
        assert!(!dom.world().get::<&FormControlState>(r1).unwrap().checked);
        assert!(dom.world().get::<&FormControlState>(r2).unwrap().checked);
    }

    #[test]
    fn formless_vs_formed_radios_independent() {
        // A radio inside a form and a radio without a form should be independent
        // even if they share the same name.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let form = dom.create_element("form", Attributes::default());
        let _ = dom.append_child(doc, form);

        let r_in_form = make_radio(&mut dom, "color", false);
        let _ = dom.append_child(form, r_in_form);

        let r_formless = make_radio(&mut dom, "color", false);
        let _ = dom.append_child(doc, r_formless);

        assert!(toggle_radio(&mut dom, r_in_form));
        assert!(toggle_radio(&mut dom, r_formless));

        // Both should remain checked (different form scopes).
        assert!(
            dom.world()
                .get::<&FormControlState>(r_in_form)
                .unwrap()
                .checked
        );
        assert!(
            dom.world()
                .get::<&FormControlState>(r_formless)
                .unwrap()
                .checked
        );
    }

    #[test]
    fn different_name_groups_independent() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let r1 = make_radio(&mut dom, "a", false);
        let r2 = make_radio(&mut dom, "b", false);
        let _ = dom.append_child(doc, r1);
        let _ = dom.append_child(doc, r2);

        assert!(toggle_radio(&mut dom, r1));
        assert!(toggle_radio(&mut dom, r2));

        assert!(dom.world().get::<&FormControlState>(r1).unwrap().checked);
        assert!(dom.world().get::<&FormControlState>(r2).unwrap().checked);
    }

    #[test]
    fn cross_tree_form_attribute_radio_group() {
        // Per HTML §4.10.18.3: a radio with form="myform" should be grouped
        // with radios inside that form, even if the radio is not a descendant.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();

        // Create a form with id="myform".
        let mut form_attrs = Attributes::default();
        form_attrs.set("id", "myform");
        let form = dom.create_element("form", form_attrs);
        let _ = dom.append_child(doc, form);

        // r1 is inside the form (ancestor association).
        let r1 = make_radio(&mut dom, "color", false);
        let _ = dom.append_child(form, r1);

        // r2 is outside the form but has form="myform" (cross-tree association).
        let r2 = make_radio_with_form_attr(&mut dom, "color", "myform", false);
        let _ = dom.append_child(doc, r2);

        // Both should be in the same radio group.
        // Toggling r1, then r2, should uncheck r1.
        assert!(toggle_radio(&mut dom, r1));
        assert!(dom.world().get::<&FormControlState>(r1).unwrap().checked);

        assert!(toggle_radio(&mut dom, r2));
        assert!(!dom.world().get::<&FormControlState>(r1).unwrap().checked);
        assert!(dom.world().get::<&FormControlState>(r2).unwrap().checked);
    }

    /// HTML §4.10.5.1.16 radio button group: "their name attributes are
    /// not empty" — an unnamed radio has no group, so two unnamed formless
    /// radios in the same document never uncheck each other.
    #[test]
    fn unnamed_formless_radios_do_not_share_group() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let mut attrs = Attributes::default();
        attrs.set("type", "radio");
        let mut unnamed = || {
            let e = dom.create_element("input", attrs.clone());
            let state = FormControlState::from_element("input", &attrs).unwrap();
            let _ = dom.world_mut().insert_one(e, state);
            let _ = dom.world_mut().insert_one(e, ElementState::default());
            e
        };
        let r1 = unnamed();
        let r2 = unnamed();
        let _ = dom.append_child(doc, r1);
        let _ = dom.append_child(doc, r2);

        assert!(toggle_radio(&mut dom, r1));
        assert!(toggle_radio(&mut dom, r2));
        assert!(dom.world().get::<&FormControlState>(r1).unwrap().checked);
        assert!(dom.world().get::<&FormControlState>(r2).unwrap().checked);
        assert_eq!(radio_arrow_navigate(&dom, r1, true), None);
    }

    /// HTML §4.10.5.1.16 radio button group: "Both a and b are in the same
    /// tree" — a detached `name=color` radio is not in the in-document
    /// formless `color` group.
    #[test]
    fn detached_radio_not_in_document_formless_group() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let r_doc = make_radio(&mut dom, "color", false);
        let _ = dom.append_child(doc, r_doc);
        let r_detached = make_radio(&mut dom, "color", false);

        assert!(toggle_radio(&mut dom, r_doc));
        assert!(toggle_radio(&mut dom, r_detached));
        assert!(dom.world().get::<&FormControlState>(r_doc).unwrap().checked);
        assert!(
            dom.world()
                .get::<&FormControlState>(r_detached)
                .unwrap()
                .checked
        );
        assert_eq!(radio_arrow_navigate(&dom, r_doc, true), None);
    }

    #[test]
    fn form_attribute_different_forms_independent() {
        // A radio with form="form1" and a radio inside form2 should be independent.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();

        let mut form1_attrs = Attributes::default();
        form1_attrs.set("id", "form1");
        let form1 = dom.create_element("form", form1_attrs);
        let _ = dom.append_child(doc, form1);

        let mut form2_attrs = Attributes::default();
        form2_attrs.set("id", "form2");
        let form2 = dom.create_element("form", form2_attrs);
        let _ = dom.append_child(doc, form2);

        // r1 has form="form1" but is outside form1 (attribute association).
        let r1 = make_radio_with_form_attr(&mut dom, "color", "form1", false);
        let _ = dom.append_child(doc, r1);

        // r2 is inside form2 (ancestor).
        let r2 = make_radio(&mut dom, "color", false);
        let _ = dom.append_child(form2, r2);

        assert!(toggle_radio(&mut dom, r1));
        assert!(toggle_radio(&mut dom, r2));

        // Both should remain checked (different form owners).
        assert!(dom.world().get::<&FormControlState>(r1).unwrap().checked);
        assert!(dom.world().get::<&FormControlState>(r2).unwrap().checked);
    }
}
