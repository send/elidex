//! Radio button group management.

use elidex_ecs::{EcsDom, ElementState, Entity};

use crate::{FormControlKind, FormControlState};

/// Toggle a radio button: uncheck all others in the same name group, check this one.
///
/// Returns `true` if the radio was toggled (enabled, unchecked → checked).
/// Uses form-scoped radio groups per WHATWG §4.10.5.1.11.
pub fn toggle_radio(dom: &mut EcsDom, entity: Entity) -> bool {
    let info = dom
        .world()
        .get::<&FormControlState>(entity)
        .ok()
        .filter(|fcs| fcs.kind == FormControlKind::Radio && !fcs.disabled)
        .map(|fcs| (fcs.name.clone(), fcs.checked));

    let Some((name, was_checked)) = info else {
        return false;
    };

    // HTML spec §4.10.5.1.11: clicking an already-checked radio does nothing.
    if was_checked {
        return false;
    }

    // Uncheck all radios in the same form-scoped group.
    // TODO(M4-3.7): support `form` attribute for cross-tree radio group association
    // (requires AncestorCache for efficient form owner resolution).
    let form_owner = crate::find_form_ancestor(dom, entity);
    let group = find_radio_group_scoped(dom, form_owner, &name);
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

/// Form filter for radio group search.
#[derive(Clone, Copy)]
enum FormFilter {
    /// No form scoping — match all radios with the given name.
    Any,
    /// Scope to radios sharing the same form ancestor (or no form ancestor).
    Scoped(Option<Entity>),
}

/// Internal helper: find radio buttons matching name, optionally scoped to a form owner.
///
/// Current complexity is O(n*d) where n = total radio buttons and d = max ancestor depth
/// (each radio requires `find_form_ancestor` walk when using `FormFilter::Scoped`).
/// TODO(M4-3.7): reduce to O(n) with `AncestorCache` (cached form owner per entity,
/// invalidated on DOM mutation via `MutationObserver`).
fn find_radio_group_impl(dom: &EcsDom, name: &str, filter: FormFilter) -> Vec<Entity> {
    let mut group: Vec<Entity> = dom
        .world()
        .query::<(Entity, &FormControlState)>()
        .iter()
        .filter(|(e, fcs)| {
            fcs.kind == FormControlKind::Radio
                && fcs.name == name
                && match filter {
                    FormFilter::Any => true,
                    FormFilter::Scoped(owner) => crate::find_form_ancestor(dom, *e) == owner,
                }
        })
        .map(|(e, _)| e)
        .collect();
    // Sort by entity bits to ensure consistent DOM-order traversal.
    // TODO(M4-3.7): sort by document tree order instead of entity allocation order,
    // which can diverge from DOM order after mutations.
    group.sort_by_key(|e| e.to_bits());
    group
}

/// Find all radio buttons with the given name (unscoped).
///
/// For spec-correct form-scoped grouping, use [`find_radio_group_scoped`].
#[must_use]
pub fn find_radio_group(dom: &EcsDom, name: &str) -> Vec<Entity> {
    find_radio_group_impl(dom, name, FormFilter::Any)
}

/// Find all radio buttons with the given name scoped to the same form owner.
///
/// Per HTML §4.10.5.1.11: radio buttons are grouped by name within the same
/// form owner. The caller should compute `form_owner` once via
/// [`find_form_ancestor`](crate::find_form_ancestor) and pass it to avoid
/// redundant ancestor walks.
#[must_use]
pub fn find_radio_group_scoped(
    dom: &EcsDom,
    form_owner: Option<Entity>,
    name: &str,
) -> Vec<Entity> {
    find_radio_group_impl(dom, name, FormFilter::Scoped(form_owner))
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
/// Returns the entity to focus next, if any.
#[must_use]
pub fn radio_arrow_navigate(dom: &EcsDom, current: Entity, forward: bool) -> Option<Entity> {
    let name = dom
        .world()
        .get::<&FormControlState>(current)
        .ok()
        .filter(|fcs| fcs.kind == FormControlKind::Radio)
        .map(|fcs| fcs.name.clone())?;

    let group = find_radio_group(dom, &name);
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

    #[test]
    fn toggle_radio_unchecks_others() {
        let mut dom = EcsDom::new();
        let r1 = make_radio(&mut dom, "color", true);
        let r2 = make_radio(&mut dom, "color", false);
        let r3 = make_radio(&mut dom, "color", false);

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
        let _r1 = make_radio(&mut dom, "g1", false);
        let _r2 = make_radio(&mut dom, "g1", false);
        let _r3 = make_radio(&mut dom, "g2", false);

        let group = find_radio_group(&dom, "g1");
        assert_eq!(group.len(), 2);
    }

    #[test]
    fn radio_arrow_navigate_wraps() {
        let mut dom = EcsDom::new();
        let r1 = make_radio(&mut dom, "nav", false);
        let _r2 = make_radio(&mut dom, "nav", false);
        let r3 = make_radio(&mut dom, "nav", false);

        // Forward from last wraps to first.
        let next = radio_arrow_navigate(&dom, r3, true);
        assert!(next.is_some());

        // Backward from first wraps to last.
        let prev = radio_arrow_navigate(&dom, r1, false);
        assert!(prev.is_some());
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

    #[test]
    fn radio_group_satisfied_when_one_checked() {
        let checked = FormControlState {
            kind: FormControlKind::Radio,
            checked: true,
            name: "g".to_string(),
            ..FormControlState::default()
        };
        let unchecked = FormControlState {
            kind: FormControlKind::Radio,
            checked: false,
            name: "g".to_string(),
            ..FormControlState::default()
        };
        assert!(is_radio_group_satisfied(&[&unchecked, &checked]));
    }

    #[test]
    fn radio_group_not_satisfied_when_none_checked() {
        let r1 = FormControlState {
            kind: FormControlKind::Radio,
            checked: false,
            name: "g".to_string(),
            ..FormControlState::default()
        };
        let r2 = FormControlState {
            kind: FormControlKind::Radio,
            checked: false,
            name: "g".to_string(),
            ..FormControlState::default()
        };
        assert!(!is_radio_group_satisfied(&[&r1, &r2]));
    }

    #[test]
    fn radios_in_different_forms_independent() {
        let mut dom = EcsDom::new();
        let form1 = dom.create_element("form", Attributes::default());
        let form2 = dom.create_element("form", Attributes::default());

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
        // Radios without a form ancestor share a single "no form" group.
        let mut dom = EcsDom::new();
        let r1 = make_radio(&mut dom, "color", false);
        let r2 = make_radio(&mut dom, "color", false);

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
        let form = dom.create_element("form", Attributes::default());

        let r_in_form = make_radio(&mut dom, "color", false);
        let _ = dom.append_child(form, r_in_form);

        let r_formless = make_radio(&mut dom, "color", false);

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
        let r1 = make_radio(&mut dom, "a", false);
        let r2 = make_radio(&mut dom, "b", false);

        assert!(toggle_radio(&mut dom, r1));
        assert!(toggle_radio(&mut dom, r2));

        assert!(dom.world().get::<&FormControlState>(r1).unwrap().checked);
        assert!(dom.world().get::<&FormControlState>(r2).unwrap().checked);
    }
}
