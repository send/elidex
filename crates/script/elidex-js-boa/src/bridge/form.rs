//! Form data collection from ECS DOM.

use super::HostBridge;

impl HostBridge {
    /// Collect form control name/value pairs from a form entity.
    ///
    /// Walks child elements of the given entity, collecting submittable
    /// controls (input, select, textarea) with a name attribute.
    pub(crate) fn collect_form_data(&self, entity_bits: u64) -> Vec<(String, String)> {
        let inner = self.inner.borrow();
        #[allow(unsafe_code)]
        let Some(dom) = (unsafe { inner.dom_ptr.as_ref() }) else {
            return Vec::new();
        };
        let Some(entity) = elidex_ecs::Entity::from_bits(entity_bits) else {
            return Vec::new();
        };

        let mut pairs = Vec::new();
        collect_form_data_recursive(dom, entity, &mut pairs);
        pairs
    }
}

/// Recursively walk children of a form entity, collecting submittable name/value pairs.
fn collect_form_data_recursive(
    dom: &elidex_ecs::EcsDom,
    parent: elidex_ecs::Entity,
    pairs: &mut Vec<(String, String)>,
) {
    let mut child_opt = dom.get_first_child(parent);
    while let Some(child) = child_opt {
        // Check if this child has a FormControlState.
        if let Ok(fcs) = dom.world().get::<&elidex_form::FormControlState>(child) {
            // Skip disabled controls and controls without a name.
            if !fcs.disabled && !fcs.name.is_empty() {
                // For checkbox/radio, only include if checked.
                if fcs.kind == elidex_form::FormControlKind::Checkbox
                    || fcs.kind == elidex_form::FormControlKind::Radio
                {
                    if fcs.checked {
                        let value = if fcs.value().is_empty() {
                            "on".to_string()
                        } else {
                            fcs.value().to_string()
                        };
                        pairs.push((fcs.name.clone(), value));
                    }
                } else if fcs.kind.is_submittable() {
                    pairs.push((fcs.name.clone(), fcs.value().to_string()));
                }
            }
        }
        // Recurse into children (fieldset, div, etc. can contain form controls).
        collect_form_data_recursive(dom, child, pairs);
        child_opt = dom.get_next_sibling(child);
    }
}
