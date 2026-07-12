//! Iframe context management for multi-document support (WHATWG HTML §4.8.5).
//!
//! Manages same-origin (in-process) and cross-origin (out-of-process) iframes
//! within a content thread.
//!
//! ## Module structure
//!
//! - `types` — IPC messages, handle types, metadata, load context
//! - `load` — URL resolution, security checks, pipeline construction
//! - `referrer` — W3C Referrer Policy computation for child documents
//! - `thread` — Cross-origin iframe thread event loop
//! - `lifecycle` — Mutation detection, lazy loading, unloading, DOM scanning
//! - `render` — Display list management for parent compositing

mod lifecycle;
mod load;
mod referrer;
mod render;
pub(super) mod thread;
mod types;

use std::collections::HashMap;

use elidex_ecs::Entity;
use elidex_script_session::{HostDriver, ParentMessage};

pub(super) use lifecycle::{
    check_lazy_iframes, find_iframe_by_name, navigate_iframe, rescan_iframes_by_diff,
    scan_initial_iframes,
};
// Exposed within `content` for the OOP-path ordering tests
// (`content_iframe_security_tests`), which drive the OOP entry directly with a
// synthesized `LoadedDocument`.
#[cfg(test)]
pub(super) use load::make_out_of_process_entry;
pub(super) use render::{re_render_all_iframes, tick_iframe_timers};
pub(super) use thread::{click_event_types, mouse_event_init_from_click};
pub use types::*;

use lifecycle::unload_iframe_handle;

// ---------------------------------------------------------------------------
// IframeRegistry
// ---------------------------------------------------------------------------

/// Registry of all iframes owned by a content thread.
///
/// Keyed by the `<iframe>` element entity in the parent DOM.
/// Also tracks lazy-loading pending entities.
#[derive(Default)]
pub struct IframeRegistry {
    entries: HashMap<Entity, IframeEntry>,
    /// Entities awaiting lazy load (`loading="lazy"` iframes not yet in viewport).
    lazy_pending: Vec<Entity>,
}

impl IframeRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a new iframe entry.
    pub fn insert(&mut self, entity: Entity, entry: IframeEntry) {
        self.entries.insert(entity, entry);
    }

    /// Remove an iframe entry, returning it if present.
    pub fn remove(&mut self, entity: Entity) -> Option<IframeEntry> {
        self.entries.remove(&entity)
    }

    /// Get a reference to an iframe entry.
    #[must_use]
    pub fn get(&self, entity: Entity) -> Option<&IframeEntry> {
        self.entries.get(&entity)
    }

    /// Get a mutable reference to an iframe entry.
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut IframeEntry> {
        self.entries.get_mut(&entity)
    }

    /// Iterate over all iframe entries.
    pub fn iter(&self) -> impl Iterator<Item = (&Entity, &IframeEntry)> {
        self.entries.iter()
    }

    /// Iterate over all iframe entries mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Entity, &mut IframeEntry)> {
        self.entries.iter_mut()
    }

    /// Number of registered iframes.
    #[must_use]
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Drain iframe→parent `postMessage`s from ALL iframes, normalising both
    /// transports onto `Vec<ParentMessage>` so the parent's single §9.3.3 gate
    /// (`parent_message_allowed`) sees one input shape (One-issue-one-way):
    /// - OOP: `try_recv` the iframe thread's `IframeToBrowser::PostMessage` (the
    ///   display-list update is a side effect of the same channel drain);
    /// - in-process: `take_pending_parent_messages()` off the child pipeline's
    ///   VM FIFO (§6.1 gap-close — pre-VM this same-origin `parent.postMessage`
    ///   path was never drained, so the message never reached the parent).
    ///
    /// Both feed the same `Vec<ParentMessage>` → the same gate at the parent
    /// event-loop chokepoint. Returns an owned `Vec` so the `self.entries`
    /// borrow is released before the parent dispatches (borrow-discipline).
    pub fn drain_parent_messages(&mut self) -> Vec<ParentMessage> {
        let mut parent_messages = Vec::new();
        for entry in self.entries.values_mut() {
            match &mut entry.handle {
                IframeHandle::OutOfProcess(oop) => {
                    while let Ok(msg) = oop.channel.try_recv() {
                        match msg {
                            IframeToBrowser::DisplayListReady(dl) => {
                                oop.display_list = dl;
                            }
                            IframeToBrowser::PostMessage {
                                data,
                                origin,
                                target_origin,
                            } => {
                                parent_messages.push(ParentMessage {
                                    data,
                                    origin,
                                    target_origin,
                                });
                            }
                        }
                    }
                }
                IframeHandle::InProcess(ip) => {
                    parent_messages.extend(ip.pipeline.runtime.take_pending_parent_messages());
                }
            }
        }
        parent_messages
    }

    /// Shut down all iframes gracefully (WHATWG HTML §7.1.3).
    pub fn shutdown_all(&mut self) {
        for (_, entry) in self.entries.drain() {
            unload_iframe_handle(entry.handle);
        }
    }

    // --- Lazy loading management ---

    /// Add an entity to the lazy-pending list.
    pub fn add_lazy_pending(&mut self, entity: Entity) {
        if !self.lazy_pending.contains(&entity) {
            self.lazy_pending.push(entity);
        }
    }

    /// Remove a single entity from the lazy-pending list.
    pub fn remove_lazy_pending(&mut self, entity: Entity) {
        self.lazy_pending.retain(|&e| e != entity);
    }

    /// Remove entities from the lazy-pending list by a provided list.
    pub fn remove_lazy_pending_list(&mut self, entities: &[Entity]) {
        self.lazy_pending.retain(|e| !entities.contains(e));
    }

    /// Whether there are any lazy-pending entities.
    #[must_use]
    pub fn has_lazy_pending(&self) -> bool {
        !self.lazy_pending.is_empty()
    }

    /// Iterate over lazy-pending entities.
    pub fn lazy_pending_iter(&self) -> std::slice::Iter<'_, Entity> {
        self.lazy_pending.iter()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::ScrollState;
    use elidex_navigation::NavigationController;

    fn make_test_entry() -> (Entity, IframeEntry) {
        let pipeline = crate::build_pipeline_interactive("", "");
        let entity = pipeline.document;
        let handle = IframeHandle::InProcess(Box::new(InProcessIframe {
            pipeline,
            nav_controller: NavigationController::new(),
            scroll_state: ScrollState::default(),
            needs_render: false,
            cached_display_list: None,
        }));
        (
            entity,
            IframeEntry {
                handle,
                loaded_src: None,
                loaded_srcdoc: None,
            },
        )
    }

    #[test]
    fn iframe_registry_insert_remove() {
        let mut registry = IframeRegistry::new();
        assert!(registry.is_empty());

        let (entity, entry) = make_test_entry();
        registry.insert(entity, entry);

        assert_eq!(registry.len(), 1);
        assert!(registry.get(entity).is_some());

        let removed = registry.remove(entity);
        assert!(removed.is_some());
        assert!(registry.is_empty());
    }

    #[test]
    fn iframe_registry_drain_empty() {
        let mut registry = IframeRegistry::new();
        let messages = registry.drain_parent_messages();
        assert!(messages.is_empty());
    }

    #[test]
    fn iframe_registry_shutdown_empty() {
        let mut registry = IframeRegistry::new();
        registry.shutdown_all();
    }

    #[test]
    fn iframe_registry_iter() {
        let mut registry = IframeRegistry::new();
        let (entity, entry) = make_test_entry();
        registry.insert(entity, entry);

        let count = registry.iter().count();
        assert_eq!(count, 1);
    }

    #[test]
    fn lazy_pending_management() {
        let mut registry = IframeRegistry::new();
        let (entity, _) = make_test_entry();

        assert!(!registry.has_lazy_pending());
        registry.add_lazy_pending(entity);
        assert!(registry.has_lazy_pending());

        // Duplicate add is idempotent.
        registry.add_lazy_pending(entity);
        assert_eq!(registry.lazy_pending_iter().count(), 1);

        registry.remove_lazy_pending(entity);
        assert!(!registry.has_lazy_pending());
    }
}
