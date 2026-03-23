//! Custom element reaction queue (WHATWG HTML §4.13.3).
//!
//! Reactions are batched and processed at specific checkpoints
//! (after script execution, event dispatch, and DOM mutation flush).

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
