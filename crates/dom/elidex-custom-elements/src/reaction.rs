//! Custom element reaction queue (WHATWG HTML §4.13.3).
//!
//! Reactions are batched and processed at specific checkpoints
//! (after script execution, event dispatch, and DOM mutation flush).

use std::collections::VecDeque;

use elidex_ecs::Entity;

/// A custom element lifecycle reaction to be processed at the next checkpoint.
///
/// Reactions are queued during DOM operations and drained by the JS runtime
/// at specific checkpoints per WHATWG HTML §4.13.3.
#[derive(Clone, Debug)]
pub enum CustomElementReaction {
    /// Element was inserted into a connected tree (`connectedCallback`).
    Connected(Entity),
    /// Element was removed from a connected tree (`disconnectedCallback`).
    Disconnected(Entity),
    /// Element was moved between documents (`adoptedCallback`).
    Adopted {
        entity: Entity,
        old_document: Entity,
        new_document: Entity,
    },
    /// An observed attribute was changed (`attributeChangedCallback`).
    AttributeChanged {
        entity: Entity,
        name: String,
        old_value: Option<String>,
        new_value: Option<String>,
    },
    /// Element should be upgraded (constructor invocation).
    Upgrade(Entity),
}

impl CustomElementReaction {
    /// Return the [`Entity`] this reaction targets regardless of
    /// variant shape (tuple-variant vs struct-variant). Used by
    /// [`scrub_entity_reactions`] for queue retention.
    #[must_use]
    pub fn target_entity(&self) -> Entity {
        match self {
            Self::Upgrade(e) | Self::Connected(e) | Self::Disconnected(e) => *e,
            Self::AttributeChanged { entity, .. } | Self::Adopted { entity, .. } => *entity,
        }
    }
}

/// Drop every queued reaction that targets `entity` from `queue`.
/// Called after a Failed upgrade per HTML §4.13.5 "upgrade an
/// element" step 8 ("empty element's CE reaction queue").
pub fn scrub_entity_reactions(queue: &mut VecDeque<CustomElementReaction>, entity: Entity) {
    queue.retain(|r| r.target_entity() != entity);
}
