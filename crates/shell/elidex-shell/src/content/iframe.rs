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
// Iframe loading / navigation
// ---------------------------------------------------------------------------

/// Load an iframe document from a `src` URL or `srcdoc` content.
///
/// 1. Resolves the iframe's origin from its URL (or parent origin for srcdoc/about:blank)
/// 2. Checks CSP frame-ancestors and X-Frame-Options headers
/// 3. Creates a `PipelineResult` (DOM, JS runtime, styles, layout)
/// 4. Wraps it in an `InProcessIframe` (same-origin) or `OutOfProcessIframe` (cross-origin)
///
/// Returns `None` if framing is blocked by security headers.
#[allow(clippy::cast_precision_loss)] // u32 width/height to f32 is acceptable for CSS pixels.
#[allow(clippy::too_many_arguments)] // Grouped context params; struct extraction deferred.
pub fn load_iframe(
    iframe_entity: Entity,
    iframe_data: &elidex_ecs::IframeData,
    parent_origin: &SecurityOrigin,
    parent_url: Option<&url::Url>,
    font_db: &std::sync::Arc<elidex_text::FontDatabase>,
    fetch_handle: &std::rc::Rc<elidex_net::FetchHandle>,
    _registry: &elidex_plugin::CssPropertyRegistry,
    depth: usize,
) -> IframeEntry {
    // Guard against excessive iframe nesting (DoS prevention).
    if depth > elidex_plugin::MAX_IFRAME_DEPTH {
        eprintln!("iframe nesting exceeds MAX_IFRAME_DEPTH ({depth})");
        let pipeline = crate::build_pipeline_interactive("", "");
        return make_iframe_entry(
            iframe_entity,
            pipeline,
            SecurityOrigin::opaque(),
            iframe_data,
        );
    }

    // Determine content source and origin.
    let (pipeline, iframe_origin) = if let Some(srcdoc) = &iframe_data.srcdoc {
        // srcdoc: parse inline HTML, inherit parent origin (WHATWG HTML §4.8.5).
        // Sandbox + credentialless override handled by apply_sandbox_origin.
        let pipeline = crate::build_pipeline_interactive(srcdoc, "");
        let origin = apply_sandbox_origin(parent_origin.clone(), iframe_data);
        (pipeline, origin)
    } else if let Some(src) = &iframe_data.src {
        if src.is_empty() || src == "about:blank" {
            // about:blank: empty document with parent origin.
            let pipeline = crate::build_pipeline_interactive("", "");
            (pipeline, parent_origin.clone())
        } else {
            // URL: resolve relative to parent, fetch and parse.
            let base = parent_url.cloned().unwrap_or_else(|| {
                url::Url::parse("about:blank").expect("about:blank is a valid URL")
            });
            let Ok(resolved) = base.join(src) else {
                eprintln!("iframe: invalid src URL: {src}");
                let pipeline = crate::build_pipeline_interactive("", "");
                return make_iframe_entry(
                    iframe_entity,
                    pipeline,
                    parent_origin.clone(),
                    iframe_data,
                );
            };

            // Use a credentialless FetchHandle when the iframe has the
            // `credentialless` attribute (WHATWG HTML §4.8.5).
            let effective_handle: std::rc::Rc<elidex_net::FetchHandle> =
                if iframe_data.credentialless {
                    std::rc::Rc::new(elidex_net::FetchHandle::new(
                        elidex_net::NetClient::new_credentialless(),
                    ))
                } else {
                    fetch_handle.clone()
                };

            match elidex_navigation::load_document(&resolved, &effective_handle, None) {
                Ok(loaded) => {
                    // Check security headers before allowing framing.
                    let doc_origin = SecurityOrigin::from_url(&loaded.url);
                    if !check_framing_allowed(&loaded.response_headers, parent_origin, &doc_origin)
                    {
                        eprintln!(
                            "iframe blocked by frame-ancestors/X-Frame-Options: {}",
                            loaded.url
                        );
                        // Show blank document instead.
                        let pipeline = crate::build_pipeline_interactive("", "");
                        return make_iframe_entry(
                            iframe_entity,
                            pipeline,
                            SecurityOrigin::opaque(),
                            iframe_data,
                        );
                    }

                    let pipeline = crate::build_pipeline_from_loaded(
                        loaded,
                        effective_handle,
                        font_db.clone(),
                    );
                    let origin = apply_sandbox_origin(
                        SecurityOrigin::from_url(pipeline.url.as_ref().unwrap_or(&resolved)),
                        iframe_data,
                    );
                    (pipeline, origin)
                }
                Err(e) => {
                    eprintln!("iframe load error: {e}");
                    let pipeline = crate::build_pipeline_interactive("", "");
                    (pipeline, parent_origin.clone())
                }
            }
        }
    } else {
        // No src or srcdoc: about:blank with parent origin.
        let pipeline = crate::build_pipeline_interactive("", "");
        (pipeline, parent_origin.clone())
    };

    make_iframe_entry(iframe_entity, pipeline, iframe_origin, iframe_data)
}

/// Check framing permission from response headers.
///
/// CSP `frame-ancestors` takes priority over `X-Frame-Options` (W3C CSP L3).
fn check_framing_allowed(
    headers: &std::collections::HashMap<String, String>,
    parent_origin: &SecurityOrigin,
    doc_origin: &SecurityOrigin,
) -> bool {
    // CSP frame-ancestors check (takes priority).
    if let Some(csp) = headers.get("content-security-policy") {
        if let Some(policy) = elidex_plugin::parse_frame_ancestors(csp) {
            return elidex_plugin::is_framing_allowed(&policy, parent_origin, doc_origin);
        }
    }
    // X-Frame-Options fallback (only if no CSP frame-ancestors).
    if let Some(xfo) = headers.get("x-frame-options") {
        return elidex_plugin::check_x_frame_options(xfo, parent_origin, doc_origin);
    }
    // No restrictions → allow framing.
    true
}

/// Apply sandbox origin override.
///
/// If sandbox is present without `allow-same-origin`, force opaque origin.
fn apply_sandbox_origin(
    origin: SecurityOrigin,
    iframe_data: &elidex_ecs::IframeData,
) -> SecurityOrigin {
    if let Some(ref sandbox_str) = iframe_data.sandbox {
        let flags = elidex_plugin::parse_sandbox_attribute(sandbox_str);
        if !flags.contains(IframeSandboxFlags::ALLOW_SAME_ORIGIN) {
            return SecurityOrigin::opaque();
        }
    }
    if iframe_data.credentialless {
        return SecurityOrigin::opaque();
    }
    origin
}

/// Create an `IframeEntry` from a pipeline and origin.
///
/// Same-origin iframes use `InProcess` (direct access); cross-origin iframes
/// use `InProcess` as well in the current implementation (true `OutOfProcess`
/// thread spawning requires async iframe loading, deferred to Phase 5).
#[allow(clippy::cast_precision_loss)] // u32 width/height to f32 is acceptable for CSS pixels.
fn make_iframe_entry(
    iframe_entity: Entity,
    pipeline: crate::PipelineResult,
    origin: SecurityOrigin,
    iframe_data: &elidex_ecs::IframeData,
) -> IframeEntry {
    let sandbox_flags = iframe_data
        .sandbox
        .as_deref()
        .map(elidex_plugin::parse_sandbox_attribute);

    // Set sandbox flags on the iframe's JS bridge for runtime enforcement.
    pipeline.runtime.bridge().set_sandbox_flags(sandbox_flags);
    // Set origin on the iframe's JS bridge.
    pipeline.runtime.bridge().set_origin(origin.clone());

    let viewport = Size::new(iframe_data.width as f32, iframe_data.height as f32);

    // Note: All iframes currently use InProcess. Cross-origin thread isolation
    // (OutOfProcessIframe) requires async iframe loading to avoid blocking the
    // parent content thread during synchronous HTTP fetch. This is deferred to
    // Phase 5 when async resource loading is implemented. The same-origin policy
    // is still enforced via JS-level access control (contentDocument returns null
    // for cross-origin, sandbox flags block script execution, etc.).
    IframeEntry {
        handle: IframeHandle::InProcess(Box::new(InProcessIframe {
            pipeline,
            nav_controller: NavigationController::new(),
            focus_target: None,
            scroll_state: ScrollState::default(),
            needs_render: false,
        })),
        meta: IframeMeta {
            origin,
            sandbox_flags,
            parent_entity: iframe_entity,
            viewport,
        },
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
