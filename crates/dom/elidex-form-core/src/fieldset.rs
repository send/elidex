//! Fieldset disabled detection — legend lookup + disabled-ancestry pull
//! predicate (HTML §4.10.19.2).
//!
//! Pure read-only derivations (no `&mut EcsDom`). The push-propagation
//! system (`propagate_fieldset_disabled` / `disable_descendants`) lives in
//! `elidex-form`, which calls [`first_legend_child`] cross-crate (UP→DOWN).

use elidex_ecs::{Attributes, EcsDom, Entity, TagType, MAX_ANCESTOR_DEPTH};

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

/// Check if an entity is inside a disabled `<fieldset>` ancestor.
///
/// Per HTML §4.10.19.2, a form control is disabled if it is a descendant of a
/// disabled `<fieldset>` element AND is not a descendant of that fieldset's
/// first `<legend>` child.
#[must_use]
pub fn is_fieldset_disabled(entity: Entity, dom: &EcsDom) -> bool {
    let mut current = dom.get_parent(entity);
    for _ in 0..MAX_ANCESTOR_DEPTH {
        let Some(ancestor) = current else {
            return false;
        };
        let is_disabled_fieldset = dom
            .world()
            .get::<&TagType>(ancestor)
            .is_ok_and(|t| t.0 == "fieldset")
            && dom
                .world()
                .get::<&Attributes>(ancestor)
                .is_ok_and(|a| a.contains("disabled"));
        if is_disabled_fieldset {
            // Check first-legend exemption: if the entity is inside the first
            // <legend> child of this fieldset, it is NOT disabled.
            let first_legend = first_legend_child(dom, ancestor);
            if let Some(legend) = first_legend {
                if is_in_first_legend(dom, entity, legend) {
                    // Exempt — but keep walking; an outer fieldset may still disable it.
                    current = dom.get_parent(ancestor);
                    continue;
                }
            }
            return true;
        }
        current = dom.get_parent(ancestor);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::{Attributes, EcsDom};

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
}
