//! Iframe context management for multi-document support (WHATWG HTML §4.8.5).
//!
//! Manages same-origin (in-process) and cross-origin (out-of-process) iframes
//! within a content thread.

// Infrastructure for upcoming iframe loading/lifecycle steps.
// Most types here will be used when iframe loading is implemented in later steps.
#![allow(dead_code)]

use std::collections::HashMap;
use std::thread::JoinHandle;

use elidex_ecs::{Entity, ScrollState};
use elidex_navigation::NavigationController;
use elidex_plugin::{IframeSandboxFlags, SecurityOrigin, Size};
use elidex_render::DisplayList;

use crate::ipc::LocalChannel;
use crate::PipelineResult;

// ---------------------------------------------------------------------------
// IPC message types for cross-origin iframe communication
// ---------------------------------------------------------------------------

/// Messages sent from the parent content thread to a cross-origin iframe thread.
#[derive(Debug)]
#[allow(dead_code)] // Used when cross-origin iframe IPC is implemented.
pub enum BrowserToIframe {
    /// Navigate the iframe to a new URL.
    Navigate(url::Url),
    /// Mouse click at iframe-local coordinates.
    MouseClick(crate::ipc::MouseClickEvent),
    /// Key pressed.
    KeyDown {
        /// DOM key value.
        key: String,
        /// DOM code value.
        code: String,
        /// Whether this is a repeat event.
        repeat: bool,
        /// Modifier keys.
        mods: crate::ipc::ModifierState,
    },
    /// Viewport size changed.
    SetViewport {
        /// New width in logical pixels.
        width: f32,
        /// New height in logical pixels.
        height: f32,
    },
    /// Cross-document postMessage (WHATWG HTML §9.4.3).
    PostMessage {
        /// JSON-serialized message data.
        data: String,
        /// Sender's serialized origin.
        origin: String,
    },
    /// Shut down the iframe thread.
    Shutdown,
}

/// Messages sent from a cross-origin iframe thread to the parent content thread.
#[derive(Debug)]
#[allow(dead_code)] // Used when cross-origin iframe IPC is implemented.
pub enum IframeToBrowser {
    /// A new display list is ready for compositing into the parent.
    DisplayListReady(DisplayList),
    /// Cross-document postMessage from iframe to parent (WHATWG HTML §9.4.3).
    PostMessage {
        /// JSON-serialized message data.
        data: String,
        /// Sender's serialized origin.
        origin: String,
    },
}

// ---------------------------------------------------------------------------
// Iframe handle types
// ---------------------------------------------------------------------------

/// Same-origin iframe: runs in the parent content thread with direct access.
pub struct InProcessIframe {
    /// Full rendering pipeline (DOM, JS, styles, layout, display list).
    pub pipeline: PipelineResult,
    /// Independent navigation history for this iframe.
    pub nav_controller: NavigationController,
    /// Currently focused entity within this iframe's document.
    pub focus_target: Option<Entity>,
    /// Independent scroll state for this iframe's viewport.
    pub scroll_state: ScrollState,
    /// Whether this iframe needs a re-render on the next frame.
    pub needs_render: bool,
}

/// Cross-origin iframe: runs in a separate thread, communicates via IPC.
pub struct OutOfProcessIframe {
    /// IPC channel to the iframe thread.
    pub channel: LocalChannel<BrowserToIframe, IframeToBrowser>,
    /// Latest display list received from the iframe thread.
    ///
    /// Updated atomically when `IframeToBrowser::DisplayListReady` is received.
    /// The parent thread always renders the most recent snapshot; stale frames
    /// are acceptable and will be replaced on the next update.
    pub display_list: DisplayList,
    /// Handle to the iframe's content thread.
    pub thread: Option<JoinHandle<()>>,
}

/// Iframe handle: dispatches to in-process or out-of-process implementation
/// based on the origin relationship with the parent document.
pub enum IframeHandle {
    /// Same-origin iframe: parent thread owns the `PipelineResult` directly.
    /// Boxed to avoid large size difference between variants (`PipelineResult` is ~1.7KB).
    InProcess(Box<InProcessIframe>),
    /// Cross-origin iframe: separate thread with IPC communication.
    OutOfProcess(OutOfProcessIframe),
}

/// Metadata shared by all iframe types (origin, sandbox, geometry).
pub struct IframeMeta {
    /// Security origin of the iframe document.
    pub origin: SecurityOrigin,
    /// Sandbox flags (if `<iframe sandbox>` attribute is present).
    pub sandbox_flags: Option<IframeSandboxFlags>,
    /// The `<iframe>` element entity in the parent DOM.
    pub parent_entity: Entity,
    /// Iframe viewport dimensions (from width/height attributes or CSS).
    pub viewport: Size,
}

/// Combined iframe entry stored in `ContentState.iframes`.
pub struct IframeEntry {
    /// Handle to the iframe's pipeline (in-process or out-of-process).
    pub handle: IframeHandle,
    /// Metadata shared by all iframe types.
    pub meta: IframeMeta,
}

/// Registry of all iframes owned by a content thread.
///
/// Keyed by the `<iframe>` element entity in the parent DOM.
#[derive(Default)]
pub struct IframeRegistry {
    entries: HashMap<Entity, IframeEntry>,
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
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Drain incoming messages from all out-of-process iframes.
    ///
    /// Processes `DisplayListReady` messages by updating the cached display list.
    /// Returns any `PostMessage` messages that need to be delivered to the parent.
    pub fn drain_oop_messages(&mut self) -> Vec<(Entity, String, String)> {
        let mut post_messages = Vec::new();
        for (entity, entry) in &mut self.entries {
            if let IframeHandle::OutOfProcess(oop) = &mut entry.handle {
                while let Ok(msg) = oop.channel.try_recv() {
                    match msg {
                        IframeToBrowser::DisplayListReady(dl) => {
                            oop.display_list = dl;
                        }
                        IframeToBrowser::PostMessage { data, origin } => {
                            post_messages.push((*entity, data, origin));
                        }
                    }
                }
            }
        }
        post_messages
    }

    /// Shut down all iframes gracefully.
    ///
    /// Sends `Shutdown` to all out-of-process iframes and joins their threads.
    /// In-process iframes are dropped directly.
    pub fn shutdown_all(&mut self) {
        for (_, entry) in self.entries.drain() {
            if let IframeHandle::OutOfProcess(mut oop) = entry.handle {
                let _ = oop.channel.send(BrowserToIframe::Shutdown);
                if let Some(thread) = oop.thread.take() {
                    if let Err(e) = thread.join() {
                        eprintln!("iframe thread panicked: {e:?}");
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create an in-process iframe entry using the pipeline's document entity.
    fn make_test_entry() -> (Entity, IframeEntry) {
        let pipeline = crate::build_pipeline_interactive("", "");
        let entity = pipeline.document;
        let meta = IframeMeta {
            origin: SecurityOrigin::opaque(),
            sandbox_flags: None,
            parent_entity: entity,
            viewport: Size::new(300.0, 150.0),
        };
        let handle = IframeHandle::InProcess(Box::new(InProcessIframe {
            pipeline,
            nav_controller: NavigationController::new(),
            focus_target: None,
            scroll_state: ScrollState::default(),
            needs_render: false,
        }));
        (entity, IframeEntry { handle, meta })
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
        let messages = registry.drain_oop_messages();
        assert!(messages.is_empty());
    }

    #[test]
    fn iframe_registry_shutdown_empty() {
        let mut registry = IframeRegistry::new();
        registry.shutdown_all(); // Should not panic.
    }

    #[test]
    fn iframe_registry_iter() {
        let mut registry = IframeRegistry::new();
        let (entity, entry) = make_test_entry();
        registry.insert(entity, entry);

        let count = registry.iter().count();
        assert_eq!(count, 1);
    }
}
