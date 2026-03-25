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
    /// Cached `Arc<DisplayList>` to avoid re-cloning on every parent render.
    /// Updated only when `needs_render` is true and re-render completes.
    pub cached_display_list: Option<std::sync::Arc<elidex_render::DisplayList>>,
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

    /// Shut down all iframes gracefully (WHATWG HTML §7.1.3).
    ///
    /// Dispatches `beforeunload`/`unload` on in-process iframe documents,
    /// sends `Shutdown` to out-of-process iframes and joins their threads.
    pub fn shutdown_all(&mut self) {
        for (_, entry) in self.entries.drain() {
            match entry.handle {
                IframeHandle::InProcess(mut ip) => {
                    crate::pipeline::dispatch_unload_events(
                        &mut ip.pipeline.runtime,
                        &mut ip.pipeline.session,
                        &mut ip.pipeline.dom,
                        ip.pipeline.document,
                    );
                }
                IframeHandle::OutOfProcess(mut oop) => {
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
/// Context from the parent document needed to load an iframe.
pub struct IframeLoadContext<'a> {
    /// Security origin of the parent document.
    pub parent_origin: &'a SecurityOrigin,
    /// URL of the parent document (for relative URL resolution).
    pub parent_url: Option<&'a url::Url>,
    /// Shared font database.
    pub font_db: &'a std::sync::Arc<elidex_text::FontDatabase>,
    /// Shared fetch handle (for network requests).
    pub fetch_handle: &'a std::rc::Rc<elidex_net::FetchHandle>,
    /// Iframe nesting depth (for `MAX_IFRAME_DEPTH` enforcement).
    pub depth: usize,
}

/// Always returns an `IframeEntry`. If framing is blocked by security headers,
/// returns a blank document with an opaque origin.
#[allow(clippy::cast_precision_loss)] // u32 width/height to f32 is acceptable for CSS pixels.
pub fn load_iframe(
    iframe_entity: Entity,
    iframe_data: &elidex_ecs::IframeData,
    ctx: &IframeLoadContext<'_>,
) -> IframeEntry {
    // Guard against excessive iframe nesting (DoS prevention).
    if ctx.depth >= elidex_plugin::MAX_IFRAME_DEPTH {
        eprintln!("iframe nesting exceeds MAX_IFRAME_DEPTH ({})", ctx.depth);
        return make_blank_entry(iframe_entity, SecurityOrigin::opaque(), iframe_data, ctx);
    }

    // Determine content source and origin.
    let (pipeline, iframe_origin) = if let Some(srcdoc) = &iframe_data.srcdoc {
        // srcdoc: parse inline HTML, inherit parent origin (WHATWG HTML §4.8.5).
        // Sandbox + credentialless override handled by apply_sandbox_origin.
        let mut pipeline = crate::build_pipeline_interactive(srcdoc, "");
        // Use the parent's font database and fetch handle instead of creating
        // fresh instances, so that srcdoc iframes share cached fonts and cookies.
        pipeline.font_db = ctx.font_db.clone();
        pipeline.fetch_handle = ctx.fetch_handle.clone();
        let origin = apply_sandbox_origin(ctx.parent_origin.clone(), iframe_data);
        (pipeline, origin)
    } else if let Some(src) = &iframe_data.src {
        if src.is_empty() || src == "about:blank" {
            // about:blank: empty document with parent origin.
            let mut pipeline = crate::build_pipeline_interactive("", "");
            // Share the parent's font database and fetch handle.
            pipeline.font_db = ctx.font_db.clone();
            pipeline.fetch_handle = ctx.fetch_handle.clone();
            (
                pipeline,
                apply_sandbox_origin(ctx.parent_origin.clone(), iframe_data),
            )
        } else {
            // URL: resolve relative to parent, fetch and parse.
            let base = ctx.parent_url.cloned().unwrap_or_else(|| {
                url::Url::parse("about:blank").expect("about:blank is a valid URL")
            });
            let Ok(resolved) = base.join(src) else {
                eprintln!("iframe: invalid src URL: {src}");
                return make_blank_entry(
                    iframe_entity,
                    apply_sandbox_origin(ctx.parent_origin.clone(), iframe_data),
                    iframe_data,
                    ctx,
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
                    ctx.fetch_handle.clone()
                };

            match elidex_navigation::load_document(&resolved, &effective_handle, None) {
                Ok(loaded) => {
                    // Check security headers before allowing framing.
                    let doc_origin = SecurityOrigin::from_url(&loaded.url);
                    if !check_framing_allowed(
                        &loaded.response_headers,
                        ctx.parent_origin,
                        &doc_origin,
                    ) {
                        eprintln!(
                            "iframe blocked by frame-ancestors/X-Frame-Options: {}",
                            loaded.url
                        );
                        return make_blank_entry(
                            iframe_entity,
                            SecurityOrigin::opaque(),
                            iframe_data,
                            ctx,
                        );
                    }

                    let pipeline = crate::build_pipeline_from_loaded(
                        loaded,
                        effective_handle,
                        ctx.font_db.clone(),
                    );
                    let origin = apply_sandbox_origin(
                        SecurityOrigin::from_url(pipeline.url.as_ref().unwrap_or(&resolved)),
                        iframe_data,
                    );
                    (pipeline, origin)
                }
                Err(e) => {
                    eprintln!("iframe load error: {e}");
                    return make_blank_entry(
                        iframe_entity,
                        apply_sandbox_origin(ctx.parent_origin.clone(), iframe_data),
                        iframe_data,
                        ctx,
                    );
                }
            }
        }
    } else {
        // No src or srcdoc: about:blank with parent origin.
        let mut pipeline = crate::build_pipeline_interactive("", "");
        // Share the parent's font database and fetch handle.
        pipeline.font_db = ctx.font_db.clone();
        pipeline.fetch_handle = ctx.fetch_handle.clone();
        (
            pipeline,
            apply_sandbox_origin(ctx.parent_origin.clone(), iframe_data),
        )
    };

    let entry = make_iframe_entry(iframe_entity, pipeline, iframe_origin, iframe_data);
    // Set the referrer to the parent document's URL (WHATWG HTML §4.8.5).
    if let IframeHandle::InProcess(ref ip) = entry.handle {
        ip.pipeline
            .runtime
            .bridge()
            .set_referrer(ctx.parent_url.map(url::Url::to_string));
    }
    entry
}

/// Check framing permission from response headers.
///
/// CSP `frame-ancestors` takes priority over `X-Frame-Options` (W3C CSP L3).
/// For CSP, any header that blocks framing wins (most restrictive).
/// For XFO, the most restrictive value across all header values is used.
fn check_framing_allowed(
    headers: &std::collections::HashMap<String, Vec<String>>,
    parent_origin: &SecurityOrigin,
    doc_origin: &SecurityOrigin,
) -> bool {
    // CSP frame-ancestors check (takes priority).
    if let Some(csp_values) = headers.get("content-security-policy") {
        let mut has_frame_ancestors = false;
        for csp in csp_values {
            if let Some(policy) = elidex_plugin::parse_frame_ancestors(csp) {
                has_frame_ancestors = true;
                // Any CSP header that blocks framing → blocked.
                if !elidex_plugin::is_framing_allowed(&policy, parent_origin, doc_origin) {
                    return false;
                }
            }
        }
        if has_frame_ancestors {
            return true;
        }
    }
    // X-Frame-Options fallback (only if no CSP frame-ancestors).
    // Use most restrictive value: if any header blocks, framing is blocked.
    if let Some(xfo_values) = headers.get("x-frame-options") {
        for xfo in xfo_values {
            if !elidex_plugin::check_x_frame_options(xfo, parent_origin, doc_origin) {
                return false;
            }
        }
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

/// Create a blank `IframeEntry` (empty document) for error/fallback cases.
///
/// Used when iframe loading fails, is blocked by security headers,
/// or exceeds the nesting depth limit.
fn make_blank_entry(
    iframe_entity: Entity,
    origin: SecurityOrigin,
    iframe_data: &elidex_ecs::IframeData,
    ctx: &IframeLoadContext<'_>,
) -> IframeEntry {
    let mut pipeline = crate::build_pipeline_interactive("", "");
    // Share parent's font database and fetch handle instead of creating fresh ones.
    pipeline.font_db = ctx.font_db.clone();
    pipeline.fetch_handle = ctx.fetch_handle.clone();
    make_iframe_entry(iframe_entity, pipeline, origin, iframe_data)
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
            cached_display_list: None,
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
// Iframe helper functions (moved from content/mod.rs)
// ---------------------------------------------------------------------------

/// Detect iframe additions/removals from mutation records.
///
/// Scans `MutationRecord` added/removed nodes for entities with `IframeData`
/// components, and triggers iframe loading/unloading accordingly.
///
/// Also detects `src` attribute changes on existing `<iframe>` elements
/// to trigger re-navigation.
pub(super) fn detect_iframe_mutations(
    records: &[elidex_script_session::MutationRecord],
    state: &mut super::ContentState,
) {
    use elidex_script_session::MutationKind;

    for record in records {
        match record.kind {
            MutationKind::ChildList => {
                // Check added nodes (and their subtrees) for <iframe> elements.
                // innerHTML inserts may contain nested iframes that wouldn't be
                // found by checking only the direct added_nodes.
                for &entity in &record.added_nodes {
                    let mut nested = Vec::new();
                    collect_iframe_entities(&state.pipeline.dom, entity, &mut nested, 0);
                    for iframe_entity in nested {
                        if state.iframes.get(iframe_entity).is_some() {
                            continue;
                        }
                        try_load_iframe_entity(state, iframe_entity, false);
                    }
                }
                // Check removed nodes (and their subtrees) for <iframe> elements.
                // Subtree iframes must also be unloaded when a parent is removed.
                let mut removed_set = std::collections::HashSet::new();
                for &entity in &record.removed_nodes {
                    let mut nested = Vec::new();
                    collect_iframe_entities(&state.pipeline.dom, entity, &mut nested, 0);
                    for iframe_entity in nested {
                        if let Some(removed_entry) = state.iframes.remove(iframe_entity) {
                            // Dispatch beforeunload/unload on the iframe's document
                            // before dropping it (WHATWG HTML §7.1.3).
                            if let IframeHandle::InProcess(mut ip) = removed_entry.handle {
                                crate::pipeline::dispatch_unload_events(
                                    &mut ip.pipeline.runtime,
                                    &mut ip.pipeline.session,
                                    &mut ip.pipeline.dom,
                                    ip.pipeline.document,
                                );
                            }
                            if state.focused_iframe == Some(iframe_entity) {
                                state.focused_iframe = None;
                            }
                        }
                        removed_set.insert(iframe_entity);
                    }
                }
                // Batch clean up lazy_iframe_pending for all removed iframes.
                if !removed_set.is_empty() {
                    state
                        .lazy_iframe_pending
                        .retain(|e| !removed_set.contains(e));
                }
            }
            MutationKind::Attribute => {
                // src attribute change on <iframe> → re-navigate.
                if record
                    .attribute_name
                    .as_deref()
                    .is_some_and(|name| name == "src")
                {
                    let target = record.target;
                    // Dispatch unload events on the old iframe before removing it
                    // (WHATWG HTML §7.1.3).
                    if let Some(mut removed_entry) = state.iframes.remove(target) {
                        if let IframeHandle::InProcess(ref mut ip) = removed_entry.handle {
                            crate::pipeline::dispatch_unload_events(
                                &mut ip.pipeline.runtime,
                                &mut ip.pipeline.session,
                                &mut ip.pipeline.dom,
                                ip.pipeline.document,
                            );
                        }
                    }
                    // force=true: src attribute change is an explicit navigation,
                    // should not be deferred by loading="lazy".
                    try_load_iframe_entity(state, target, true);
                }
            }
            _ => {}
        }
    }
}

/// Register a loaded iframe: store its display list on the parent DOM,
/// insert into the registry, and dispatch the `load` event.
fn register_iframe_entry(state: &mut super::ContentState, entity: Entity, mut entry: IframeEntry) {
    if let IframeHandle::InProcess(ref mut ip) = entry.handle {
        let arc_dl = std::sync::Arc::new(ip.pipeline.display_list.clone());
        ip.cached_display_list = Some(std::sync::Arc::clone(&arc_dl));
        // Remove then insert: hecs insert_one fails if component already exists
        // (e.g., iframe reload after src mutation).
        let _ = state
            .pipeline
            .dom
            .world_mut()
            .remove_one::<elidex_render::IframeDisplayList>(entity);
        let _ = state
            .pipeline
            .dom
            .world_mut()
            .insert_one(entity, elidex_render::IframeDisplayList(arc_dl));
    }
    state.iframes.insert(entity, entry);
    dispatch_iframe_load_event(state, entity);
}

/// Count the iframe nesting depth of an entity by walking its DOM ancestors.
///
/// Returns the number of ancestor elements that have `IframeData` components.
/// Used for `MAX_IFRAME_DEPTH` enforcement to prevent runaway nesting.
fn count_iframe_ancestor_depth(dom: &elidex_ecs::EcsDom, entity: Entity) -> usize {
    let mut depth = 0;
    let mut current = dom.get_parent(entity);
    let mut steps = 0;
    while let Some(parent) = current {
        steps += 1;
        if steps > elidex_ecs::MAX_ANCESTOR_DEPTH {
            break;
        }
        if dom.world().get::<&elidex_ecs::IframeData>(parent).is_ok() {
            depth += 1;
        }
        current = dom.get_parent(parent);
    }
    depth
}

/// Check lazy iframes and load those near the viewport.
///
/// Uses `LayoutBox` position to determine if a lazy iframe is within 200px
/// of the viewport bounds. Once loaded, the entity is removed from the
/// pending list. Iframes without a `LayoutBox` (e.g., inside a `display:none`
/// parent) remain in the pending list until layout is computed.
pub(super) fn check_lazy_iframes(state: &mut super::ContentState) {
    if state.lazy_iframe_pending.is_empty() {
        return;
    }

    let vp_width = state.pipeline.viewport.width;
    let vp_height = state.pipeline.viewport.height;
    let scroll_x = state.viewport_scroll.scroll_offset.x;
    let scroll_y = state.viewport_scroll.scroll_offset.y;
    let margin = 200.0_f32; // Load iframes within 200px of viewport edge.

    let visible_left = scroll_x - margin;
    let visible_right = scroll_x + vp_width + margin;
    let visible_top = scroll_y - margin;
    let visible_bottom = scroll_y + vp_height + margin;

    // Collect entities to load (to avoid borrow conflict with state).
    let to_load: Vec<Entity> = state
        .lazy_iframe_pending
        .iter()
        .copied()
        .filter(|&entity| {
            state
                .pipeline
                .dom
                .world()
                .get::<&elidex_plugin::LayoutBox>(entity)
                .ok()
                .is_some_and(|lb| {
                    let left = lb.content.origin.x;
                    let right = left + lb.content.size.width;
                    let top = lb.content.origin.y;
                    let bottom = top + lb.content.size.height;
                    // Iframe overlaps the extended viewport (2D check).
                    right >= visible_left
                        && left <= visible_right
                        && bottom >= visible_top
                        && top <= visible_bottom
                })
        })
        .collect();

    if to_load.is_empty() {
        return;
    }

    // Remove loaded entities from pending list (use HashSet to avoid O(n^2)).
    let to_load_set: std::collections::HashSet<Entity> = to_load.iter().copied().collect();
    state
        .lazy_iframe_pending
        .retain(|e| !to_load_set.contains(e));

    // Load each visible lazy iframe.
    for entity in to_load {
        // Re-read IframeData since we're loading now.
        let iframe_data = state
            .pipeline
            .dom
            .world()
            .get::<&elidex_ecs::IframeData>(entity)
            .ok()
            .map(|d| (*d).clone());
        if let Some(data) = iframe_data {
            let parent_origin = state.pipeline.runtime.bridge().origin();
            let depth = count_iframe_ancestor_depth(&state.pipeline.dom, entity);
            let ctx = IframeLoadContext {
                parent_origin: &parent_origin,
                parent_url: state.pipeline.url.as_ref(),
                font_db: &state.pipeline.font_db,
                fetch_handle: &state.pipeline.fetch_handle,
                depth,
            };
            let entry = load_iframe(entity, &data, &ctx);
            register_iframe_entry(state, entity, entry);
        }
    }
}

/// Dispatch a "load" event on an iframe element entity in the parent document.
///
/// Per WHATWG HTML §4.8.5: when an iframe's content is loaded, a "load"
/// event fires on the `<iframe>` element (not on the iframe's document).
fn dispatch_iframe_load_event(state: &mut super::ContentState, iframe_entity: Entity) {
    let mut event = elidex_script_session::DispatchEvent::new("load", iframe_entity);
    event.bubbles = false;
    event.cancelable = false;
    state.pipeline.runtime.dispatch_event(
        &mut event,
        &mut state.pipeline.session,
        &mut state.pipeline.dom,
        state.pipeline.document,
    );
}

/// Try to load an iframe for `entity` if it has `IframeData`.
///
/// Respects `loading="lazy"`: lazy iframes are skipped here and will be
/// loaded when `check_lazy_iframes` detects their `LayoutBox` is near the viewport.
///
/// When `force` is `true`, the lazy check is bypassed (e.g., for explicit
/// navigation via `iframe.src = ...` which should load immediately regardless
/// of the `loading` attribute).
pub(super) fn try_load_iframe_entity(state: &mut super::ContentState, entity: Entity, force: bool) {
    let iframe_data = state
        .pipeline
        .dom
        .world()
        .get::<&elidex_ecs::IframeData>(entity)
        .ok()
        .map(|d| (*d).clone());
    if let Some(data) = iframe_data {
        // loading="lazy": defer loading until near viewport (WHATWG HTML §4.8.5).
        // Registers the entity in the pending list; the event loop checks
        // LayoutBox positions each frame to detect viewport proximity.
        // Explicit navigation (force=true) bypasses the lazy check.
        if !force && data.loading == elidex_ecs::LoadingAttribute::Lazy {
            if !state.lazy_iframe_pending.contains(&entity) {
                state.lazy_iframe_pending.push(entity);
            }
            return;
        }
        let parent_origin = state.pipeline.runtime.bridge().origin();
        let depth = count_iframe_ancestor_depth(&state.pipeline.dom, entity);
        let ctx = IframeLoadContext {
            parent_origin: &parent_origin,
            parent_url: state.pipeline.url.as_ref(),
            font_db: &state.pipeline.font_db,
            fetch_handle: &state.pipeline.fetch_handle,
            depth,
        };
        let entry = load_iframe(entity, &data, &ctx);
        register_iframe_entry(state, entity, entry);
    }
}

/// Walk the DOM tree and load any `<iframe>` elements found during initial parse.
///
/// Mutation-based detection (`detect_iframe_mutations`) only catches iframes
/// added via JS. This function handles iframes present in the initial HTML.
pub(super) fn scan_initial_iframes(state: &mut super::ContentState) {
    let mut iframes_to_load = Vec::new();
    collect_iframe_entities(
        &state.pipeline.dom,
        state.pipeline.document,
        &mut iframes_to_load,
        0,
    );
    for entity in iframes_to_load {
        try_load_iframe_entity(state, entity, false);
    }
}

/// Recursively collect entities with `IframeData` components.
fn collect_iframe_entities(
    dom: &elidex_ecs::EcsDom,
    entity: Entity,
    result: &mut Vec<Entity>,
    depth: usize,
) {
    if depth > elidex_ecs::MAX_ANCESTOR_DEPTH {
        return;
    }
    if dom.world().get::<&elidex_ecs::IframeData>(entity).is_ok() {
        result.push(entity);
    }
    let mut child = dom.get_first_child(entity);
    while let Some(c) = child {
        collect_iframe_entities(dom, c, result, depth + 1);
        child = dom.get_next_sibling(c);
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
            cached_display_list: None,
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
