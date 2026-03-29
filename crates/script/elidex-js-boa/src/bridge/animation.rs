//! Web Animations API bridge methods.

use super::HostBridge;

impl HostBridge {
    /// Queue a script-initiated animation for the content thread to apply.
    pub(crate) fn queue_script_animation(
        &self,
        anim: crate::globals::element::accessors::animate::ScriptAnimation,
    ) {
        self.inner.borrow_mut().pending_script_animations.push(anim);
    }

    /// Drain pending script-initiated animations.
    pub fn drain_script_animations(
        &self,
    ) -> Vec<crate::globals::element::accessors::animate::ScriptAnimation> {
        std::mem::take(&mut self.inner.borrow_mut().pending_script_animations)
    }

    /// Get the number of active animations for an entity.
    ///
    /// Currently returns 0 (pending animations not yet applied). In the full
    /// implementation, the content thread would sync animation state back.
    pub(crate) fn animation_count(&self, entity_id: u64) -> usize {
        // Count pending + active (from engine). For now, count pending only.
        self.inner
            .borrow()
            .pending_script_animations
            .iter()
            .filter(|a| a.entity_id == entity_id)
            .count()
    }

    /// Get info about an active animation for an entity.
    pub(crate) fn animation_info(
        &self,
        entity_id: u64,
        index: usize,
    ) -> Option<crate::globals::element::accessors::animate::AnimationInfo> {
        let inner = self.inner.borrow();
        let mut count = 0;
        for a in &inner.pending_script_animations {
            if a.entity_id == entity_id {
                if count == index {
                    return Some(crate::globals::element::accessors::animate::AnimationInfo {
                        id: a.options.id.clone(),
                        play_state: "running".into(),
                        current_time: 0.0,
                    });
                }
                count += 1;
            }
        }
        None
    }

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
