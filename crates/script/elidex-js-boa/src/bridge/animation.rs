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
}
