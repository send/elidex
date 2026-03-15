//! Fieldset disabled propagation and legend detection.

use elidex_ecs::{Attributes, EcsDom, ElementState, Entity, TagType, MAX_ANCESTOR_DEPTH};

use crate::FormControlState;

/// Propagate `<fieldset disabled>` to descendant form controls.
///
/// Per HTML §4.10.19.2, controls inside a disabled fieldset are disabled,
/// except those inside the first `<legend>` child.
pub fn propagate_fieldset_disabled(dom: &mut EcsDom) {
    let fieldsets: Vec<(Entity, bool)> = dom
        .world()
        .query::<(Entity, &TagType, &Attributes)>()
        .iter()
        .filter(|(_, tag, _)| tag.0 == "fieldset")
        .map(|(e, _, attrs)| (e, attrs.contains("disabled")))
        .collect();

    for (fs_entity, disabled) in fieldsets {
        if !disabled {
            continue;
        }
        let first_legend = first_legend_child(dom, fs_entity);
        disable_descendants(dom, fs_entity, first_legend, 0);
    }
}

/// Find the first `<legend>` child of a fieldset.
#[must_use]
pub fn first_legend_child(dom: &EcsDom, fieldset: Entity) -> Option<Entity> {
    let mut child = dom.get_first_child(fieldset)?;
    loop {
        let is_legend = dom
            .world()
            .get::<&TagType>(child)
            .is_ok_and(|t| t.0 == "legend");
        if is_legend {
            return Some(child);
        }
        child = dom.get_next_sibling(child)?;
    }
}

/// Check if an entity is inside the first legend of a fieldset.
#[must_use]
pub fn is_in_first_legend(dom: &EcsDom, entity: Entity, first_legend: Entity) -> bool {
    let mut current = Some(entity);
    for _ in 0..MAX_ANCESTOR_DEPTH {
        let Some(e) = current else { return false };
        if e == first_legend {
            return true;
        }
        current = dom.get_parent(e);
    }
    false
}

fn disable_descendants(
    dom: &mut EcsDom,
    entity: Entity,
    first_legend: Option<Entity>,
    depth: usize,
) {
    if depth > MAX_ANCESTOR_DEPTH {
        return;
    }
    let children: Vec<Entity> = {
        let mut v = Vec::new();
        let mut child = dom.get_first_child(entity);
        while let Some(c) = child {
            v.push(c);
            child = dom.get_next_sibling(c);
        }
        v
    };

    for child in children {
        // Skip the first legend and its descendants.
        if first_legend == Some(child) {
            continue;
        }
        // If the child is a form control, disable it.
        let has_fcs = dom.world().get::<&FormControlState>(child).is_ok();
        if has_fcs {
            if let Ok(mut fcs) = dom.world_mut().get::<&mut FormControlState>(child) {
                fcs.disabled = true;
            }
            if let Ok(mut es) = dom.world_mut().get::<&mut ElementState>(child) {
                es.insert(ElementState::DISABLED);
            }
        }
        disable_descendants(dom, child, None, depth + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom};

    #[test]
    fn fieldset_disables_descendants() {
        let mut dom = EcsDom::new();
        let mut fs_attrs = Attributes::default();
        fs_attrs.set("disabled", "");
        let fs = dom.create_element("fieldset", fs_attrs);
        let input = dom.create_element("input", Attributes::default());
        let fcs = FormControlState::from_element("input", &Attributes::default()).unwrap();
        let _ = dom.world_mut().insert_one(input, fcs);
        let _ = dom.world_mut().insert_one(input, ElementState::default());
        let _ = dom.append_child(fs, input);

        propagate_fieldset_disabled(&mut dom);

        assert!(
            dom.world()
                .get::<&FormControlState>(input)
                .unwrap()
                .disabled
        );
        assert!(dom
            .world()
            .get::<&ElementState>(input)
            .unwrap()
            .contains(ElementState::DISABLED));
    }

    #[test]
    fn first_legend_exempted() {
        let mut dom = EcsDom::new();
        let mut fs_attrs = Attributes::default();
        fs_attrs.set("disabled", "");
        let fs = dom.create_element("fieldset", fs_attrs);
        let legend = dom.create_element("legend", Attributes::default());
        let input_in_legend = dom.create_element("input", Attributes::default());
        let fcs = FormControlState::from_element("input", &Attributes::default()).unwrap();
        let _ = dom.world_mut().insert_one(input_in_legend, fcs);
        let _ = dom
            .world_mut()
            .insert_one(input_in_legend, ElementState::default());
        let _ = dom.append_child(legend, input_in_legend);
        let _ = dom.append_child(fs, legend);

        let input_outside = dom.create_element("input", Attributes::default());
        let fcs2 = FormControlState::from_element("input", &Attributes::default()).unwrap();
        let _ = dom.world_mut().insert_one(input_outside, fcs2);
        let _ = dom
            .world_mut()
            .insert_one(input_outside, ElementState::default());
        let _ = dom.append_child(fs, input_outside);

        propagate_fieldset_disabled(&mut dom);

        // Input in legend should NOT be disabled.
        assert!(
            !dom.world()
                .get::<&FormControlState>(input_in_legend)
                .unwrap()
                .disabled
        );
        // Input outside legend should be disabled.
        assert!(
            dom.world()
                .get::<&FormControlState>(input_outside)
                .unwrap()
                .disabled
        );
    }

    #[test]
    fn non_disabled_fieldset_no_effect() {
        let mut dom = EcsDom::new();
        let fs = dom.create_element("fieldset", Attributes::default());
        let input = dom.create_element("input", Attributes::default());
        let fcs = FormControlState::from_element("input", &Attributes::default()).unwrap();
        let _ = dom.world_mut().insert_one(input, fcs);
        let _ = dom.world_mut().insert_one(input, ElementState::default());
        let _ = dom.append_child(fs, input);

        propagate_fieldset_disabled(&mut dom);
        assert!(
            !dom.world()
                .get::<&FormControlState>(input)
                .unwrap()
                .disabled
        );
    }

    #[test]
    fn first_legend_child_found() {
        let mut dom = EcsDom::new();
        let fs = dom.create_element("fieldset", Attributes::default());
        let legend = dom.create_element("legend", Attributes::default());
        let _ = dom.append_child(fs, legend);
        assert_eq!(first_legend_child(&dom, fs), Some(legend));
    }

    #[test]
    fn no_legend_returns_none() {
        let mut dom = EcsDom::new();
        let fs = dom.create_element("fieldset", Attributes::default());
        let div = dom.create_element("div", Attributes::default());
        let _ = dom.append_child(fs, div);
        assert_eq!(first_legend_child(&dom, fs), None);
    }

    #[test]
    fn is_in_first_legend_true() {
        let mut dom = EcsDom::new();
        let legend = dom.create_element("legend", Attributes::default());
        let input = dom.create_element("input", Attributes::default());
        let _ = dom.append_child(legend, input);
        assert!(is_in_first_legend(&dom, input, legend));
    }

    #[test]
    fn is_in_first_legend_false() {
        let mut dom = EcsDom::new();
        let legend = dom.create_element("legend", Attributes::default());
        let other = dom.create_element("div", Attributes::default());
        assert!(!is_in_first_legend(&dom, other, legend));
    }

    #[test]
    fn nested_fieldset_disabled() {
        let mut dom = EcsDom::new();
        let mut fs_attrs = Attributes::default();
        fs_attrs.set("disabled", "");
        let fs = dom.create_element("fieldset", fs_attrs);
        let div = dom.create_element("div", Attributes::default());
        let input = dom.create_element("input", Attributes::default());
        let fcs = FormControlState::from_element("input", &Attributes::default()).unwrap();
        let _ = dom.world_mut().insert_one(input, fcs);
        let _ = dom.world_mut().insert_one(input, ElementState::default());
        let _ = dom.append_child(div, input);
        let _ = dom.append_child(fs, div);

        propagate_fieldset_disabled(&mut dom);
        assert!(
            dom.world()
                .get::<&FormControlState>(input)
                .unwrap()
                .disabled
        );
    }
}
